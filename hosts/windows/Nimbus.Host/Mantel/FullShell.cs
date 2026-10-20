using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Nimbus.Host.Mantel;

/// <summary>
/// What Explorer normally does for the desktop, done by full Mantel when Explorer isn't running
/// (the wallpaper itself is Mantel's desktop window, see MantelHost):
/// - the space the taskbar keeps (the "work area", so maximized windows stop above Mantel's taskbar),
/// - minimized windows go away instead of stacking up as little title bars in the corner,
/// - the Windows key opens Mantel's launcher (alone; Win+E, Win+R and other shortcuts still go to Windows),
/// - the volume keys work (Explorer is what normally handles them) and show Mantel's volume bar,
/// - Windows+Left/Right snap the window in front to half the screen and Windows+Up/Down maximize, restore or
///   minimize it (Explorer handles these shortcuts too; dragging a window to an edge still snaps by itself).
/// Everything is put back when Mantel stops.
/// </summary>
internal sealed class FullShell : IDisposable
{
    private Native.RECT? _savedWorkArea;
    private MINIMIZEDMETRICS? _savedMinimized;
    private IntPtr _hook;
    private HookProc? _hookProc;
    private bool _winDown, _winCombo;
    public event Action? WinKey;
    /// <summary>+1 / -1 for the volume keys, 0 for mute.</summary>
    public event Action<int>? VolumeKey;
    /// <summary>Ctrl+Shift+Esc (Explorer's shortcut for Task Manager): Hearth Task Manager.</summary>
    public event Action? TaskManagerKey;

    public void Start(int barSize)
    {
        SetWorkArea(barSize);
        HideMinimized();
        _hookProc = KeyboardHook;
        _hook = SetWindowsHookEx(13 /* WH_KEYBOARD_LL */, _hookProc, GetModuleHandle(null), 0);
    }

    public void Stop()
    {
        if (_hook != IntPtr.Zero) { UnhookWindowsHookEx(_hook); _hook = IntPtr.Zero; }
        if (_savedMinimized is MINIMIZEDMETRICS m) { var mm = m; SystemParametersInfo(0x002C /* SPI_SETMINIMIZEDMETRICS */, (uint)mm.cbSize, ref mm, 1 | 2); _savedMinimized = null; }
        if (_savedWorkArea is Native.RECT r) { var rr = r; Native.SystemParametersInfo(0x002F /* SPI_SETWORKAREA */, 0, ref rr, 1 | 2); _savedWorkArea = null; }
    }

    public void Dispose() => Stop();

    /// <summary>Everything but the bar: along the bottom, or down the left side on an upright screen.</summary>
    public void SetWorkArea(int barSize)
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        var cur = new Native.RECT();
        Native.SystemParametersInfo(0x0030 /* SPI_GETWORKAREA */, 0, ref cur, 0);
        _savedWorkArea ??= cur;
        var wa = MantelWindow.Portrait
            ? new Native.RECT { Left = scr.Left + barSize, Top = scr.Top, Right = scr.Right, Bottom = scr.Bottom }
            : new Native.RECT { Left = scr.Left, Top = scr.Top, Right = scr.Right, Bottom = scr.Bottom - barSize };
        Native.SystemParametersInfo(0x002F /* SPI_SETWORKAREA */, 0, ref wa, 1 | 2);
    }

    private void HideMinimized()
    {
        var m = new MINIMIZEDMETRICS { cbSize = Marshal.SizeOf<MINIMIZEDMETRICS>() };
        SystemParametersInfo(0x002B /* SPI_GETMINIMIZEDMETRICS */, (uint)m.cbSize, ref m, 0);
        _savedMinimized ??= m;
        m.iArrange |= 8;   // ARW_HIDE
        SystemParametersInfo(0x002C, (uint)m.cbSize, ref m, 1 | 2);
    }

    // The Windows key on its own (pressed and let go with nothing else) toggles the launcher.
    private IntPtr KeyboardHook(int code, IntPtr wParam, IntPtr lParam)
    {
        if (code >= 0)
        {
            int vk = Marshal.ReadInt32(lParam);
            int msg = wParam.ToInt32();
            bool down = msg is 0x100 or 0x104, up = msg is 0x101 or 0x105;
            if (vk is 0x5B or 0x5C)
            {
                if (down) { if (!_winDown) _winCombo = false; _winDown = true; }
                if (up)
                {
                    _winDown = false;
                    if (!_winCombo) { WinKey?.Invoke(); return (IntPtr)1; }
                }
            }
            else if (down && _winDown)
            {
                _winCombo = true;
                // Windows+arrows (alone, without Ctrl or Shift): snap like Explorer would.
                if (vk is >= 0x25 and <= 0x28 && (GetAsyncKeyState(0x11) & 0x8000) == 0 && (GetAsyncKeyState(0x10) & 0x8000) == 0)
                {
                    var fg = Native.GetForegroundWindow();
                    ThreadPool.QueueUserWorkItem(_ => Snap(fg, vk));
                    return (IntPtr)1;
                }
            }
            if (vk == 0x1B && down && (GetAsyncKeyState(0x11) & 0x8000) != 0 && (GetAsyncKeyState(0x10) & 0x8000) != 0)
            {
                TaskManagerKey?.Invoke();
                return (IntPtr)1;
            }
            if (vk is 0xAF or 0xAE or 0xAD)   // volume up / down / mute
            {
                if (down) VolumeKey?.Invoke(vk == 0xAF ? 1 : vk == 0xAE ? -1 : 0);
                return (IntPtr)1;
            }
        }
        return CallNextHookEx(_hook, code, wParam, lParam);
    }

    /// <summary>Windows+Left/Right: the left or right half of the work area (beside Mantel's taskbar); Up: maximize;
    /// Down: restore a maximized window, else minimize it. Windows' invisible resize borders are allowed for.</summary>
    private static void Snap(IntPtr h, int vk)
    {
        try
        {
            if (h == IntPtr.Zero || (Native.GetWindowLong(h, Native.GWL_EXSTYLE) & Native.WS_EX_TOOLWINDOW) != 0) return;
            if (Native.GetWindowLong(h, -16 /* GWL_STYLE */) is int style && (style & 0x00040000 /* WS_THICKFRAME */) == 0 && vk is 0x25 or 0x27) return;
            switch (vk)
            {
                case 0x26: Native.ShowWindow(h, 3 /* SW_MAXIMIZE */); return;
                case 0x28: Native.ShowWindow(h, IsZoomed(h) ? Native.SW_RESTORE : Native.SW_MINIMIZE); return;
            }
            if (IsZoomed(h)) Native.ShowWindow(h, Native.SW_RESTORE);
            var wa = Screen.FromHandle(h).WorkingArea;
            var half = vk == 0x25 ? new Rectangle(wa.Left, wa.Top, wa.Width / 2, wa.Height) : new Rectangle(wa.Left + wa.Width / 2, wa.Top, wa.Width - wa.Width / 2, wa.Height);
            // GetWindowRect includes the invisible borders; the visible frame is DWM's extended frame bounds.
            GetWindowRect(h, out var outer);
            int l = 0, t = 0, r = 0, b = 0;
            if (DwmGetWindowAttribute(h, 9 /* DWMWA_EXTENDED_FRAME_BOUNDS */, out var frame, Marshal.SizeOf<Native.RECT>()) == 0)
            { l = frame.Left - outer.Left; t = frame.Top - outer.Top; r = outer.Right - frame.Right; b = outer.Bottom - frame.Bottom; }
            SetWindowPos(h, IntPtr.Zero, half.Left - l, half.Top - t, half.Width + l + r, half.Height + t + b, 0x0004 /* NOZORDER */ | 0x0010 /* NOACTIVATE */ | 0x4000 /* ASYNCWINDOWPOS */);
        }
        catch { }
    }

    [DllImport("user32.dll")] private static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr h, out Native.RECT r);
    [DllImport("user32.dll")] private static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("dwmapi.dll")] private static extern int DwmGetWindowAttribute(IntPtr h, int attr, out Native.RECT value, int size);

    [StructLayout(LayoutKind.Sequential)]
    private struct MINIMIZEDMETRICS { public int cbSize, iWidth, iHorzGap, iVertGap, iArrange; }
    private delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool SystemParametersInfo(uint action, uint param, ref MINIMIZEDMETRICS pv, uint flags);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr SetWindowsHookEx(int id, HookProc fn, IntPtr mod, uint thread);
    [DllImport("user32.dll")] private static extern bool UnhookWindowsHookEx(IntPtr hook);
    [DllImport("user32.dll")] private static extern short GetAsyncKeyState(int vk);
    [DllImport("user32.dll")] private static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr wParam, IntPtr lParam);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr GetModuleHandle(string? name);
}
