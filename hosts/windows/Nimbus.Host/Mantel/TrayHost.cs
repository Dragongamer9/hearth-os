using System.Runtime.InteropServices;

namespace Nimbus.Host.Mantel;

/// <summary>
/// The notification area (tray) for full Mantel, when Explorer isn't running. Apps add their tray icons with
/// Shell_NotifyIcon, which hands them to whichever window has the class "Shell_TrayWnd" (normally Explorer's
/// taskbar) as a WM_COPYDATA message. This creates that window, keeps the icons, and passes clicks back to
/// the apps the way Explorer does. Then it tells every app the taskbar (re)started ("TaskbarCreated"), so
/// apps that were already running add their icons again.
/// The same approach as the open-source ManagedShell library (Cairo Desktop, RetroBar).
/// </summary>
internal sealed class TrayHost : IDisposable
{
    public sealed class TrayIcon
    {
        public uint HWnd;
        public uint Id;
        public Guid Guid;
        public uint CallbackMessage;
        public uint Version;
        public string Tip = "";
        public string? Image;       // PNG data URL
        public bool Hidden;
        public uint Pid;
        public string Key => Guid != Guid.Empty ? Guid.ToString("N") : $"{HWnd:x}-{Id}";
    }

    private readonly Dictionary<string, TrayIcon> _icons = new();
    private IntPtr _hwnd;
    private WndProc? _proc;   // kept alive: Windows calls it
    private ushort _atom;
    public event Action? Changed;
    /// <summary>A tray icon's balloon notification (NIF_INFO): app name, title, text. Explorer turns these into
    /// Windows' toasts; with no Explorer, Mantel keeps them in the clock panel's history.</summary>
    public event Action<string, string, string>? Balloon;

    public IReadOnlyList<TrayIcon> Icons { get { lock (_icons) return _icons.Values.Where(i => !i.Hidden).ToList(); } }

    /// <summary>Creates the tray window. Only when no other taskbar (Explorer) is running.</summary>
    public bool Start()
    {
        if (_hwnd != IntPtr.Zero) return true;
        if (FindWindow("Shell_TrayWnd", null) != IntPtr.Zero) return false;
        _proc = WindowProc;
        var wc = new WNDCLASSEX
        {
            cbSize = Marshal.SizeOf<WNDCLASSEX>(),
            lpfnWndProc = Marshal.GetFunctionPointerForDelegate(_proc),
            hInstance = GetModuleHandle(null),
            lpszClassName = "Shell_TrayWnd",
        };
        _atom = RegisterClassEx(ref wc);
        if (_atom == 0) { SystemBridge.Log("mantel tray: couldn't register Shell_TrayWnd (" + Marshal.GetLastWin32Error() + ")"); return false; }
        _hwnd = CreateWindowEx(0x80 /* WS_EX_TOOLWINDOW */ | 0x8 /* WS_EX_TOPMOST */, "Shell_TrayWnd", "", 0x80000000 /* WS_POPUP */,
            0, 0, 0, 0, IntPtr.Zero, IntPtr.Zero, wc.hInstance, IntPtr.Zero);
        if (_hwnd == IntPtr.Zero) { SystemBridge.Log("mantel tray: couldn't create the tray window"); return false; }
        // Some apps only accept the tray if Windows lets them talk to it across privilege levels.
        ChangeWindowMessageFilterEx(_hwnd, 0x4A /* WM_COPYDATA */, 1 /* MSGFLT_ALLOW */, IntPtr.Zero);
        // "The taskbar started": running apps add their icons again.
        uint created = RegisterWindowMessage("TaskbarCreated");
        SendNotifyMessage(new IntPtr(0xFFFF), created, IntPtr.Zero, IntPtr.Zero);
        SystemBridge.Log("mantel tray: hosting tray icons");
        return true;
    }

    public void Stop()
    {
        if (_hwnd != IntPtr.Zero) { DestroyWindow(_hwnd); _hwnd = IntPtr.Zero; }
        if (_atom != 0) { UnregisterClass("Shell_TrayWnd", GetModuleHandle(null)); _atom = 0; }
        lock (_icons) _icons.Clear();
    }

    public void Dispose() => Stop();

    // ------------------------------------------------------------------ messages from apps

    private IntPtr WindowProc(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam)
    {
        if (msg == 0x4A /* WM_COPYDATA */ && lParam != IntPtr.Zero)
        {
            try
            {
                var cds = Marshal.PtrToStructure<COPYDATASTRUCT>(lParam);
                if (cds.dwData == (IntPtr)1 && cds.lpData != IntPtr.Zero && cds.cbData >= 8 + 24)
                {
                    // Older apps send a shorter structure: copy what's there into a zeroed full-size one.
                    int size = Marshal.SizeOf<SHELLTRAYDATA>();
                    var buf = Marshal.AllocHGlobal(size);
                    try
                    {
                        var bytes = new byte[size];
                        Marshal.Copy(cds.lpData, bytes, 0, Math.Min(size, cds.cbData));
                        Marshal.Copy(bytes, 0, buf, size);
                        var data = Marshal.PtrToStructure<SHELLTRAYDATA>(buf);
                        if (data.dwSignature != 0x34753423) return IntPtr.Zero;
                        return (IntPtr)(Handle(data.dwMessage, data.nid) ? 1 : 0);
                    }
                    finally { Marshal.FreeHGlobal(buf); }
                }
            }
            catch (Exception ex) { SystemBridge.Log("mantel tray: " + ex.Message); }
            return IntPtr.Zero;
        }
        return DefWindowProc(hwnd, msg, wParam, lParam);
    }

    private const uint NIM_ADD = 0, NIM_MODIFY = 1, NIM_DELETE = 2, NIM_SETVERSION = 4;
    private const uint NIF_MESSAGE = 1, NIF_ICON = 2, NIF_TIP = 4, NIF_STATE = 8, NIF_GUID = 0x20;

    private bool Handle(uint message, NOTIFYICONDATA n)
    {
        var guid = (n.uFlags & NIF_GUID) != 0 ? n.guidItem : Guid.Empty;
        string key = guid != Guid.Empty ? guid.ToString("N") : $"{n.hWnd:x}-{n.uID}";
        lock (_icons)
        {
            switch (message)
            {
                case NIM_ADD:
                case NIM_MODIFY:
                    if (!_icons.TryGetValue(key, out var icon))
                    {
                        if (message == NIM_MODIFY) return false;
                        icon = new TrayIcon { HWnd = n.hWnd, Id = n.uID, Guid = guid };
                        GetWindowThreadProcessId(new IntPtr(n.hWnd), out icon.Pid);
                        _icons[key] = icon;
                    }
                    if ((n.uFlags & NIF_MESSAGE) != 0) icon.CallbackMessage = n.uCallbackMessage;
                    if ((n.uFlags & NIF_TIP) != 0) icon.Tip = n.szTip ?? "";
                    if ((n.uFlags & NIF_STATE) != 0 && (n.dwStateMask & 1) != 0) icon.Hidden = (n.dwState & 1) != 0;
                    if ((n.uFlags & NIF_ICON) != 0 && n.hIcon != 0) icon.Image = IconPng(new IntPtr(n.hIcon));
                    if ((n.uFlags & 0x10 /* NIF_INFO */) != 0 && !string.IsNullOrWhiteSpace(n.szInfo))
                    {
                        var (app, title, text) = (AppName(icon.Pid, icon.Tip), n.szInfoTitle ?? "", n.szInfo ?? "");
                        ThreadPool.QueueUserWorkItem(_ => Balloon?.Invoke(app, title, text));
                    }
                    break;
                case NIM_DELETE:
                    if (!_icons.Remove(key)) return false;
                    break;
                case NIM_SETVERSION:
                    if (_icons.TryGetValue(key, out var v)) v.Version = n.uVersion;
                    break;
                default:
                    return true;
            }
        }
        Changed?.Invoke();
        return true;
    }

    /// <summary>The app behind a tray icon, for its notifications: its program's description, else its tooltip.</summary>
    private static string AppName(uint pid, string tip)
    {
        try
        {
            var exe = Native.ProcessPath(pid);
            if (exe is not null)
            {
                var d = System.Diagnostics.FileVersionInfo.GetVersionInfo(exe).FileDescription;
                return string.IsNullOrWhiteSpace(d) ? Path.GetFileNameWithoutExtension(exe) : d.Trim();
            }
        }
        catch { }
        return tip.Split('\n')[0];
    }

    private static string? IconPng(IntPtr hIcon)
    {
        try
        {
            using var ico = Icon.FromHandle(hIcon);
            using var bmp = ico.ToBitmap();
            return Native.ToDataUrl(new Bitmap(bmp, new Size(32, 32)));
        }
        catch { return null; }
    }

    // ------------------------------------------------------------------ clicks back to apps

    /// <summary>A click on an icon: what Explorer would send to the app (x, y: where on the screen).</summary>
    public void Click(string key, string button, int x, int y)
    {
        TrayIcon? i;
        lock (_icons) _icons.TryGetValue(key, out i);
        if (i is null || i.CallbackMessage == 0) return;
        var hwnd = new IntPtr(i.HWnd);
        if (!IsWindow(hwnd)) { lock (_icons) _icons.Remove(key); Changed?.Invoke(); return; }
        AllowSetForegroundWindow((int)i.Pid);   // lets its menu or window come to the front
        uint down, up, dbl;
        (down, up, dbl) = button == "right" ? (0x204u, 0x205u, 0x206u) : button == "middle" ? (0x207u, 0x208u, 0x209u) : (0x201u, 0x202u, 0x203u);
        if (i.Version >= 4)
        {
            var wp = (IntPtr)((y << 16) | (x & 0xFFFF));
            IntPtr L(uint m) => (IntPtr)(((int)i.Id << 16) | (int)(m & 0xFFFF));
            if (button == "double") { PostMessage(hwnd, i.CallbackMessage, wp, L(0x203)); return; }
            PostMessage(hwnd, i.CallbackMessage, wp, L(down));
            PostMessage(hwnd, i.CallbackMessage, wp, L(up));
            if (button == "left") PostMessage(hwnd, i.CallbackMessage, wp, L(0x400 /* NIN_SELECT */));
            if (button == "right") PostMessage(hwnd, i.CallbackMessage, wp, L(0x7B /* WM_CONTEXTMENU */));
        }
        else
        {
            if (button == "double") { PostMessage(hwnd, i.CallbackMessage, (IntPtr)i.Id, (IntPtr)0x203); return; }
            PostMessage(hwnd, i.CallbackMessage, (IntPtr)i.Id, (IntPtr)down);
            PostMessage(hwnd, i.CallbackMessage, (IntPtr)i.Id, (IntPtr)up);
            if (button == "right") PostMessage(hwnd, i.CallbackMessage, (IntPtr)i.Id, (IntPtr)0x7B);
        }
    }

    // ------------------------------------------------------------------ Windows API

    private delegate IntPtr WndProc(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WNDCLASSEX
    {
        public int cbSize; public uint style; public IntPtr lpfnWndProc; public int cbClsExtra; public int cbWndExtra;
        public IntPtr hInstance; public IntPtr hIcon; public IntPtr hCursor; public IntPtr hbrBackground;
        public string? lpszMenuName; public string lpszClassName; public IntPtr hIconSm;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct COPYDATASTRUCT { public IntPtr dwData; public int cbData; public IntPtr lpData; }

    // Handles in this structure are 32-bit values, whatever the process's bitness (so 32-bit apps can use it).
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct NOTIFYICONDATA
    {
        public int cbSize; public uint hWnd; public uint uID; public uint uFlags; public uint uCallbackMessage; public uint hIcon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szTip;
        public uint dwState; public uint dwStateMask;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string szInfo;
        public uint uVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string szInfoTitle;
        public uint dwInfoFlags; public Guid guidItem; public uint hBalloonIcon;
    }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SHELLTRAYDATA { public int dwSignature; public uint dwMessage; public NOTIFYICONDATA nid; }

    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)] private static extern ushort RegisterClassEx(ref WNDCLASSEX wc);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool UnregisterClass(string cls, IntPtr hInstance);
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr CreateWindowEx(uint exStyle, string cls, string name, uint style, int x, int y, int w, int h, IntPtr parent, IntPtr menu, IntPtr inst, IntPtr param);
    [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern IntPtr DefWindowProc(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr GetModuleHandle(string? name);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr FindWindow(string? cls, string? title);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern uint RegisterWindowMessage(string name);
    [DllImport("user32.dll")] private static extern bool SendNotifyMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] private static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] private static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern bool AllowSetForegroundWindow(int pid);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] private static extern bool ChangeWindowMessageFilterEx(IntPtr hwnd, uint msg, uint action, IntPtr info);
}
