using System.Runtime.InteropServices;
using Microsoft.Win32;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Windows' virtual desktops for Mantel's taskbar (DeskSwitcher.qml on Hearth OS), using only what's documented
/// or long-stable, so it keeps working across Windows 11 builds:
/// - the list and the current one: Explorer's own registry values (VirtualDesktopIDs, CurrentVirtualDesktop, and
///   each desktop's Name), read-only;
/// - which desktop a window is on, and moving a window: IVirtualDesktopManager (public COM API);
/// - switching: Windows' own shortcut, Win+Ctrl+Left/Right, typed with SendInput (there's no public API).
/// Virtual desktops belong to Explorer: with no Explorer (full Mantel) there are none, and the switcher hides.
/// Anything failing makes <see cref="Get"/> say there's nothing to show, never throws.
/// </summary>
internal static class VirtualDesktops
{
    public sealed record Desk(string Id, string Name, bool Active);

    private const string Key = @"Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops";

    /// <summary>The desktops in order, or an empty list when there's only one or Windows doesn't say.</summary>
    public static List<Desk> Get()
    {
        try
        {
            if (!DesktopMode.ExplorerShellRunning()) return new();
            using var k = Registry.CurrentUser.OpenSubKey(Key);
            if (k?.GetValue("VirtualDesktopIDs") is not byte[] ids || ids.Length < 32 || ids.Length % 16 != 0) return new();
            var list = new List<Guid>();
            for (int i = 0; i < ids.Length; i += 16) list.Add(new Guid(ids.AsSpan(i, 16)));
            var cur = Current(k);
            var outp = new List<Desk>();
            for (int i = 0; i < list.Count; i++)
            {
                string? name = null;
                try { using var d = k.OpenSubKey(@"Desktops\" + list[i].ToString("B").ToUpperInvariant()); name = d?.GetValue("Name") as string; } catch { }
                outp.Add(new Desk(list[i].ToString("D"), string.IsNullOrWhiteSpace(name) ? "Desktop " + (i + 1) : name!, list[i] == cur));
            }
            return outp;
        }
        catch (Exception ex) { SystemBridge.Log("mantel: virtual desktops: " + ex.Message); return new(); }
    }

    /// <summary>Windows 11 keeps the current desktop next to the list; Windows 10 per session. If neither says, the
    /// desktop of the window in front.</summary>
    private static Guid Current(RegistryKey k)
    {
        if (k.GetValue("CurrentVirtualDesktop") is byte[] c && c.Length == 16) return new Guid(c);
        try
        {
            int session = System.Diagnostics.Process.GetCurrentProcess().SessionId;
            using var s = Registry.CurrentUser.OpenSubKey($@"Software\Microsoft\Windows\CurrentVersion\Explorer\SessionInfo\{session}\VirtualDesktops");
            if (s?.GetValue("CurrentVirtualDesktop") is byte[] c2 && c2.Length == 16) return new Guid(c2);
        }
        catch { }
        var m = Manager();
        var fg = Native.GetForegroundWindow();
        if (m is not null && fg != IntPtr.Zero && m.GetWindowDesktopId(fg, out var g) == 0) return g;
        return Guid.Empty;
    }

    /// <summary>Switches to desktop <paramref name="index"/> (0-based) by typing Win+Ctrl+Left/Right as many times as needed.</summary>
    public static bool Switch(int index)
    {
        var all = Get();
        int cur = all.FindIndex(d => d.Active);
        if (all.Count < 2 || cur < 0 || index < 0 || index >= all.Count || index == cur) return false;
        int steps = Math.Abs(index - cur);
        ushort arrow = (ushort)(index > cur ? 0x27 /* VK_RIGHT */ : 0x25 /* VK_LEFT */);
        var keys = new List<INPUT> { Key_(0x5B, false), Key_(0x11, false) };
        for (int i = 0; i < steps; i++) { keys.Add(Key_(arrow, false, true)); keys.Add(Key_(arrow, true, true)); }
        keys.Add(Key_(0x11, true)); keys.Add(Key_(0x5B, true));
        var arr = keys.ToArray();
        return SendInput((uint)arr.Length, arr, Marshal.SizeOf<INPUT>()) == arr.Length;
    }

    /// <summary>Moves a window to another desktop. Windows lets an app move only its own windows this way; for
    /// other apps' windows it says no, and the caller tells you to use Task View (Win+Tab) instead.</summary>
    public static bool MoveWindow(IntPtr hwnd, string deskId, out string? error)
    {
        error = null;
        if (!Guid.TryParse(deskId, out var g)) { error = "That desktop isn't there any more"; return false; }
        var m = Manager();
        if (m is null) { error = "Windows' virtual desktops aren't available"; return false; }
        int hr = m.MoveWindowToDesktop(hwnd, ref g);
        if (hr == 0) return true;
        error = hr == unchecked((int)0x80070005)
            ? "Windows only lets each app move its own windows between desktops. Drag it in Task View (Windows+Tab) instead"
            : $"Windows couldn't move it (0x{hr:X8})";
        return false;
    }

    private static IVirtualDesktopManager? _manager;
    private static IVirtualDesktopManager? Manager()
    {
        if (_manager is not null) return _manager;
        try
        {
            var t = Type.GetTypeFromCLSID(new Guid("aa509086-5ca9-4c25-8f95-589d3c07b48a"));
            if (t is null) return null;
            _manager = Activator.CreateInstance(t) as IVirtualDesktopManager;
        }
        catch (Exception ex) { SystemBridge.Log("mantel: IVirtualDesktopManager: " + ex.Message); }
        return _manager;
    }

    [ComImport, Guid("a5cd92ff-29be-454c-8d04-d82879fb3f1b"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IVirtualDesktopManager
    {
        [PreserveSig] int IsWindowOnCurrentVirtualDesktop(IntPtr topLevelWindow, [MarshalAs(UnmanagedType.Bool)] out bool onCurrent);
        [PreserveSig] int GetWindowDesktopId(IntPtr topLevelWindow, out Guid desktopId);
        [PreserveSig] int MoveWindowToDesktop(IntPtr topLevelWindow, ref Guid desktopId);
    }

    // ------------------------------------------------------------------ SendInput
    private static INPUT Key_(ushort vk, bool up, bool extended = false) => new()
    {
        type = 1,
        U = new InputUnion { ki = new KEYBDINPUT { wVk = vk, dwFlags = (up ? 2u : 0u) | (extended ? 1u : 0u) } },
    };
    [StructLayout(LayoutKind.Sequential)] private struct INPUT { public uint type; public InputUnion U; }
    [StructLayout(LayoutKind.Explicit)] private struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)] private struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)] private struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo; }
    [DllImport("user32.dll", SetLastError = true)] private static extern uint SendInput(uint n, INPUT[] inputs, int size);
}
