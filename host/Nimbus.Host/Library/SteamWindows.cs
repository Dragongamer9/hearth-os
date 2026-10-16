using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace Nimbus.Host.Library;

/// <summary>
/// Keeps Steam's main window out of the way while Steam runs in the background.
/// Steam brings its main window forward when it starts or receives a steam:// request; Nimbus hides
/// that one window (the same as Steam being "in the tray"). Other Steam windows, such as the
/// Install dialog, are left alone. The tray icon still restores Steam normally.
/// </summary>
public static class SteamWindows
{
    private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr hWnd, int cmd);
    private const int SW_HIDE = 0;

    private static readonly object Gate = new();
    private static DateTime _watchUntil = DateTime.MinValue;
    private static bool _watching;

    /// <summary>Hides Steam's main window whenever it appears during the next <paramref name="seconds"/>.</summary>
    public static void HideMainFor(int seconds)
    {
        lock (Gate)
        {
            var until = DateTime.UtcNow.AddSeconds(seconds);
            if (until > _watchUntil) _watchUntil = until;
            if (_watching) return;
            _watching = true;
        }
        Task.Run(async () =>
        {
            try
            {
                while (true)
                {
                    lock (Gate) if (DateTime.UtcNow > _watchUntil) return;
                    HideMainOnce();
                    await Task.Delay(250);
                }
            }
            finally { lock (Gate) _watching = false; }
        });
    }

    /// <summary>Hides the visible top-level window titled exactly "Steam" owned by Steam. Returns how many were hidden.</summary>
    public static int HideMainOnce()
    {
        var steamPids = new HashSet<uint>();
        foreach (var name in new[] { "steam", "steamwebhelper" })
            foreach (var p in Process.GetProcessesByName(name)) { steamPids.Add((uint)p.Id); p.Dispose(); }
        if (steamPids.Count == 0) return 0;

        int hidden = 0;
        var title = new StringBuilder(64);
        EnumWindows((h, _) =>
        {
            if (!IsWindowVisible(h)) return true;
            GetWindowThreadProcessId(h, out var pid);
            if (!steamPids.Contains(pid)) return true;
            title.Clear();
            GetWindowText(h, title, title.Capacity);
            if (title.ToString() == "Steam") { ShowWindow(h, SW_HIDE); hidden++; }
            return true;
        }, IntPtr.Zero);
        return hidden;
    }
}
