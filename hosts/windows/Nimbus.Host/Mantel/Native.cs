using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;

namespace Nimbus.Host.Mantel;

/// <summary>Windows APIs Mantel uses: windows, app bars, icons, wallpaper.</summary>
internal static class Native
{
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool LockWorkStation();
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] private static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    /// <summary>Win+PrintScreen: Windows saves a screenshot to Pictures\Screenshots (and flashes the screen).</summary>
    public static void Screenshot()
    {
        keybd_event(0x5B, 0, 0, UIntPtr.Zero); keybd_event(0x2C, 0, 0, UIntPtr.Zero);
        keybd_event(0x2C, 0, 2, UIntPtr.Zero); keybd_event(0x5B, 0, 2, UIntPtr.Zero);
    }
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr hWnd, uint cmd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder s, int max);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder s, int max);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr hWnd, int index);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string? cls, string? title);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string? cls, string? title);
    [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint msg, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW")] public static extern IntPtr GetClassLongPtr(IntPtr hWnd, int index);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool AllowSetForegroundWindow(int pid);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attr, out int value, int size);
    [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] public static extern bool QueryFullProcessImageName(IntPtr h, int flags, StringBuilder s, ref int size);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool SystemParametersInfo(uint action, uint param, string? pv, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool SystemParametersInfo(uint action, uint param, StringBuilder pv, uint flags);
    [DllImport("user32.dll")] public static extern bool SystemParametersInfo(uint action, uint param, ref RECT pv, uint flags);

    public const int GWL_EXSTYLE = -20;
    public const int WS_EX_TOOLWINDOW = 0x80, WS_EX_APPWINDOW = 0x40000, WS_EX_NOACTIVATE = 0x08000000;
    public const uint GW_OWNER = 4;
    public const int SW_HIDE = 0, SW_SHOWNA = 8, SW_RESTORE = 9, SW_MINIMIZE = 6, SW_SHOW = 5;
    public const uint WM_GETICON = 0x7F, WM_CLOSE = 0x10, SMTO_ABORTIFHUNG = 2;
    public const int GCLP_HICON = -14, GCLP_HICONSM = -34;
    public const int DWMWA_CLOAKED = 14;
    public const uint SPI_SETDESKWALLPAPER = 0x14, SPI_GETDESKWALLPAPER = 0x73, SPIF_UPDATEINIFILE = 1, SPIF_SENDCHANGE = 2;

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    // -------------------------------------------------------------- app bar (the space the taskbar keeps)
    [StructLayout(LayoutKind.Sequential)]
    public struct APPBARDATA
    {
        public int cbSize;
        public IntPtr hWnd;
        public uint uCallbackMessage;
        public uint uEdge;
        public RECT rc;
        public IntPtr lParam;
    }
    [DllImport("shell32.dll")] public static extern UIntPtr SHAppBarMessage(uint msg, ref APPBARDATA data);
    public const uint ABM_NEW = 0, ABM_REMOVE = 1, ABM_QUERYPOS = 2, ABM_SETPOS = 3, ABM_GETSTATE = 4, ABM_SETSTATE = 10;
    public const uint ABE_BOTTOM = 3;
    public const int ABS_AUTOHIDE = 1, ABS_ALWAYSONTOP = 2;

    // -------------------------------------------------------------- shell icons (any app, including Store apps)
    [ComImport, Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IShellItemImageFactory
    {
        [PreserveSig] int GetImage(SIZE size, int flags, out IntPtr hbitmap);
    }
    [StructLayout(LayoutKind.Sequential)] public struct SIZE { public int cx, cy; }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    public static extern void SHCreateItemFromParsingName(string path, IntPtr bc, [MarshalAs(UnmanagedType.LPStruct)] Guid riid,
        [MarshalAs(UnmanagedType.Interface)] out IShellItemImageFactory item);
    [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr h);

    /// <summary>A shell item's icon (an app id like "shell:AppsFolder\...", or a file) as a PNG data URL.</summary>
    public static string? ShellIcon(string parsingName, int px = 48)
    {
        try
        {
            SHCreateItemFromParsingName(parsingName, IntPtr.Zero, typeof(IShellItemImageFactory).GUID, out var f);
            if (f.GetImage(new SIZE { cx = px, cy = px }, 0 /* SIIGBF_RESIZETOFIT */, out var hbm) != 0 || hbm == IntPtr.Zero) return null;
            try { return ToDataUrl(BitmapWithAlpha(hbm)); }
            finally { DeleteObject(hbm); }
        }
        catch { return null; }
    }

    /// <summary>Image.FromHbitmap drops transparency: copy the 32-bit pixels across instead.</summary>
    private static Bitmap BitmapWithAlpha(IntPtr hbm)
    {
        using var src = Image.FromHbitmap(hbm);
        var rect = new Rectangle(0, 0, src.Width, src.Height);
        var data = src.LockBits(rect, ImageLockMode.ReadOnly, src.PixelFormat);
        try
        {
            var dst = new Bitmap(data.Width, data.Height, data.Stride, PixelFormat.Format32bppArgb, data.Scan0);
            var copy = new Bitmap(dst);
            dst.Dispose();
            return copy;
        }
        finally { src.UnlockBits(data); }
    }

    public static string ToDataUrl(Image img)
    {
        using (img)
        using (var ms = new MemoryStream())
        {
            img.Save(ms, ImageFormat.Png);
            return "data:image/png;base64," + Convert.ToBase64String(ms.ToArray());
        }
    }

    public static string? ProcessPath(uint pid)
    {
        var h = OpenProcess(0x1000 /* PROCESS_QUERY_LIMITED_INFORMATION */, false, pid);
        if (h == IntPtr.Zero) return null;
        try
        {
            var sb = new StringBuilder(1024);
            int size = sb.Capacity;
            return QueryFullProcessImageName(h, 0, sb, ref size) ? sb.ToString() : null;
        }
        finally { CloseHandle(h); }
    }
}
