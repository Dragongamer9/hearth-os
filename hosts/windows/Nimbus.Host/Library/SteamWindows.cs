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
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] private static extern bool AllowSetForegroundWindow(int pid);
    [StructLayout(LayoutKind.Sequential)] private struct RECT { public int L, T, R, B; }
    private const int SW_HIDE = 0;
    private const int ASFW_ANY = -1;

    // Dialogs we've already brought to the front (so we don't keep stealing focus back).
    private static readonly HashSet<IntPtr> Focused = new();

    /// <summary>Lets Steam put its own windows in front (called just before sending it a steam:// request).</summary>
    public static void AllowSteamToFocus() => AllowSetForegroundWindow(ASFW_ANY);

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

    /// <summary>Stops hiding Steam's main window (when you asked to open Steam).</summary>
    public static void StopHiding() { lock (Gate) _watchUntil = DateTime.MinValue; }

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
            var t = title.ToString();
            if (t == "Steam" || t.Equals("Friends List", StringComparison.OrdinalIgnoreCase)) { ShowWindow(h, SW_HIDE); hidden++; }
            else if (IsQuestion(t) && GetWindowRect(h, out var r) && r.R - r.L > 250 && r.B - r.T > 150)
            {
                // A real Steam dialog (e.g. "Install - <game>", a cloud-save conflict, Steam's sign-in): bring it to
                // the front once so the controller bridge's keys reach it.
                lock (Focused) if (Focused.Add(h)) SetForegroundWindow(h);
            }
            return true;
        }, IntPtr.Zero);
        return hidden;
    }

    /// <summary>A Steam window that may need you while a game starts (a cloud-save conflict, an error, an EULA,
    /// Install), not Steam's own main, friends, Big Picture or "launching" windows, which stay out of the way and are
    /// never pulled in front of the game. Same rule as Hearth's gamescope::steam_question.</summary>
    public static bool IsQuestion(string title)
    {
        var t = title.Trim().ToLowerInvariant();
        bool steamsOwn = t.Length == 0 || t is "steam" or "steamwebhelper" or "steam big picture mode" or "friends list"
            || t.StartsWith("launching") || t.StartsWith("preparing") || t.StartsWith("updating") || t.StartsWith("steam - updat");
        return !steamsOwn;
    }
}
