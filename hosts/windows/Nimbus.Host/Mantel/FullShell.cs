using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Nimbus.Host.Mantel;

/// <summary>
/// What Explorer normally does for the desktop, done by full Mantel when Explorer isn't running
/// (the wallpaper itself is Mantel's desktop window, see MantelHost):
/// - the space the taskbar keeps (the "work area", so maximized windows stop above Mantel's taskbar),
/// - minimized windows go away instead of stacking up as little title bars in the corner,
/// - the Windows key opens Mantel's launcher (alone; Win+E, Win+R and other shortcuts still go to Windows),
/// - the volume keys work (Explorer is what normally handles them) and show Mantel's volume bar.
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

    public void Start(int barHeight)
    {
        SetWorkArea(barHeight);
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

    private void SetWorkArea(int barHeight)
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        var cur = new Native.RECT();
        Native.SystemParametersInfo(0x0030 /* SPI_GETWORKAREA */, 0, ref cur, 0);
        _savedWorkArea ??= cur;
        var wa = new Native.RECT { Left = scr.Left, Top = scr.Top, Right = scr.Right, Bottom = scr.Bottom - barHeight };
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
            else if (down && _winDown) _winCombo = true;
            if (vk is 0xAF or 0xAE or 0xAD)   // volume up / down / mute
            {
                if (down) VolumeKey?.Invoke(vk == 0xAF ? 1 : vk == 0xAE ? -1 : 0);
                return (IntPtr)1;
            }
        }
        return CallNextHookEx(_hook, code, wParam, lParam);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MINIMIZEDMETRICS { public int cbSize, iWidth, iHorzGap, iVertGap, iArrange; }
    private delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool SystemParametersInfo(uint action, uint param, ref MINIMIZEDMETRICS pv, uint flags);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr SetWindowsHookEx(int id, HookProc fn, IntPtr mod, uint thread);
    [DllImport("user32.dll")] private static extern bool UnhookWindowsHookEx(IntPtr hook);
    [DllImport("user32.dll")] private static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr wParam, IntPtr lParam);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr GetModuleHandle(string? name);
}
