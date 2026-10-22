using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace Nimbus.Host.Library;

/// <summary>
/// Watches for the launched game's processes (any process whose exe lives under the game's folders).
/// Store launchers hand off to other processes, so this is how we know a game really started and ended.
/// </summary>
public sealed class GameMonitor : IDisposable
{
    private static readonly TimeSpan PollEvery = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan StartTimeout = TimeSpan.FromMinutes(3); // Steam may update first
    private const int MissesBeforeExit = 4;                                // ~8s grace: launchers restart into the game

    private readonly object _lock = new();
    private readonly System.Threading.Timer _timer;
    private string? _id;
    private string[] _dirs = Array.Empty<string>();
    private DateTime _launchedAt;
    private DateTime? _startedAt;
    private int _misses;

    // Raised on a background thread.
    public event Action<string>? Started;
    public event Action<string, int>? Exited;     // id, minutes played this session
    public event Action<string>? StartTimedOut;

    public string? RunningId { get { lock (_lock) return _startedAt is null ? null : _id; } }
    /// <summary>When the running game started (UTC), or null.</summary>
    public DateTime? StartedAt { get { lock (_lock) return _startedAt; } }

    // On a timer thread an error would end Ember (saving play time can fail while antivirus holds the file): logged instead.
    public GameMonitor() => _timer = new System.Threading.Timer(_ => { try { Poll(); } catch (Exception ex) { SystemBridge.Log("game monitor: " + ex); } }, null, Timeout.Infinite, Timeout.Infinite);

    public void Track(string id, IEnumerable<string> dirs)
    {
        lock (_lock)
        {
            _id = id;
            _dirs = dirs.Where(d => !string.IsNullOrWhiteSpace(d))
                        .Select(d => Path.GetFullPath(d).TrimEnd('\\') + "\\")
                        .ToArray();
            _launchedAt = DateTime.UtcNow;
            _startedAt = null;
            _misses = 0;
        }
        _timer.Change(PollEvery, PollEvery);
    }

    /// <summary>Quick Resume: stops watching the running game without counting it as ended. Returns what was
    /// watched (id, folders, when it started), or null if nothing was running.</summary>
    public (string Id, string[] Dirs, DateTime Started)? Detach()
    {
        lock (_lock)
        {
            if (_id is null || _startedAt is null) return null;
            var r = (_id, _dirs, _startedAt.Value);
            _id = null; _startedAt = null;
            _timer.Change(Timeout.Infinite, Timeout.Infinite);
            return r;
        }
    }

    /// <summary>Quick Resume: watches a game that's already running again (it was put aside), counting from now.</summary>
    public void Resume(string id, string[] dirs) => Adopt(id, dirs, DateTime.UtcNow);

    /// <summary>Watches a game that's already running, started at <paramref name="startedUtc"/> (after Ember
    /// restarted while it played, or back from Quick Resume).</summary>
    public void Adopt(string id, string[] dirs, DateTime startedUtc)
    {
        lock (_lock) { _id = id; _dirs = dirs; _launchedAt = DateTime.UtcNow; _startedAt = startedUtc; _misses = 0; }
        _timer.Change(PollEvery, PollEvery);
    }

    /// <summary>The running game's folders (empty when nothing runs).</summary>
    public string[] RunningDirs { get { lock (_lock) return _startedAt is null ? Array.Empty<string>() : _dirs; } }

    /// <summary>Is this process one of the game's (its exe is under one of the folders)?</summary>
    public static bool IsUnder(int pid, string[] dirs)
    {
        if (dirs.Length == 0 || pid <= 4) return false;
        var path = ImagePath(pid);
        return path is not null && dirs.Any(d => path.StartsWith(d, StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>The exe file names of a game's processes (for spotting anti-cheat before freezing it).</summary>
    public static List<string> ProcessNames(string[] dirs)
    {
        var l = new List<string>();
        foreach (var p in MatchingProcesses(dirs)) { try { l.Add(Path.GetFileName(ImagePath(p.Id) ?? "")); } catch { } p.Dispose(); }
        return l;
    }

    private void Poll()
    {
        string? id; string[] dirs; DateTime? started; DateTime launched;
        lock (_lock) { id = _id; dirs = _dirs; started = _startedAt; launched = _launchedAt; }
        if (id is null) return;

        bool running = MatchingProcesses(dirs).Any();
        if (started is null)
        {
            if (running)
            {
                lock (_lock) _startedAt = DateTime.UtcNow;
                Started?.Invoke(id);
            }
            else if (DateTime.UtcNow - launched > StartTimeout)
            {
                Stop();
                StartTimedOut?.Invoke(id);
            }
            return;
        }

        if (running) { lock (_lock) _misses = 0; return; }
        int misses;
        lock (_lock) misses = ++_misses;
        if (misses < MissesBeforeExit) return;

        int minutes = (int)Math.Round((DateTime.UtcNow - started.Value).TotalMinutes - MissesBeforeExit * PollEvery.TotalMinutes);
        Stop();
        Exited?.Invoke(id, Math.Max(0, minutes));
    }

    private void Stop()
    {
        _timer.Change(Timeout.Infinite, Timeout.Infinite);
        lock (_lock) { _id = null; _startedAt = null; }
    }

    /// <summary>Asks the game to close, then force-closes whatever is left after a few seconds.</summary>
    public async Task<bool> CloseAsync()
    {
        string[] dirs;
        lock (_lock) dirs = _dirs;
        if (dirs.Length == 0) return false;

        foreach (var p in MatchingProcesses(dirs)) { try { p.CloseMainWindow(); } catch { } }
        for (int i = 0; i < 12 && MatchingProcesses(dirs).Any(); i++) await Task.Delay(500);

        bool allGone = true;
        foreach (var p in MatchingProcesses(dirs))
        {
            try { p.Kill(entireProcessTree: true); }
            catch { allGone = false; } // e.g. the game runs as administrator
        }
        return allGone;
    }

    /// <summary>Process ids of a game's processes (any exe under its folders).</summary>
    public static List<int> ProcessIds(string[] dirs) { var l = new List<int>(); foreach (var p in MatchingProcesses(dirs)) { l.Add(p.Id); p.Dispose(); } return l; }

    // Quick Resume: the whole game frozen (no processor or graphics use) and carried on later.
    private const uint PROCESS_SUSPEND_RESUME = 0x0800;
    [DllImport("ntdll.dll")] private static extern int NtSuspendProcess(IntPtr h);
    [DllImport("ntdll.dll")] private static extern int NtResumeProcess(IntPtr h);
    /// <summary>Freezes (or unfreezes) every process of the game. Returns how many it could.</summary>
    public static int Suspend(string[] dirs, bool freeze) => SuspendAll(dirs, freeze).Done;

    /// <summary>Freezes (or unfreezes) every process of the game: (how many it could, how many it couldn't: a
    /// process running as administrator, or one protected by anti-cheat, can't be opened or refuses).</summary>
    public static (int Done, int Failed) SuspendAll(string[] dirs, bool freeze)
    {
        int n = 0, failed = 0;
        foreach (var pid in ProcessIds(dirs))
        {
            var h = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid);
            if (h == IntPtr.Zero) { failed++; continue; }
            try { if ((freeze ? NtSuspendProcess(h) : NtResumeProcess(h)) == 0) n++; else failed++; } finally { CloseHandle(h); }
        }
        return (n, failed);
    }

    /// <summary>Closes a game that's frozen: it runs again for a moment so it can close properly.</summary>
    public static async Task<bool> CloseDirsAsync(string[] dirs)
    {
        Suspend(dirs, false);
        foreach (var p in MatchingProcesses(dirs)) { try { p.CloseMainWindow(); } catch { } p.Dispose(); }
        for (int i = 0; i < 12 && MatchingProcesses(dirs).Any(); i++) await Task.Delay(500);
        bool allGone = true;
        foreach (var p in MatchingProcesses(dirs)) { try { p.Kill(entireProcessTree: true); } catch { allGone = false; } p.Dispose(); }
        return allGone;
    }

    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] private static extern bool IsIconic(IntPtr h);
    /// <summary>Puts the game's window in front (after it's carried on).</summary>
    public static void FocusWindow(string[] dirs)
    {
        foreach (var p in MatchingProcesses(dirs))
        {
            var h = p.MainWindowHandle; p.Dispose();
            if (h == IntPtr.Zero) continue;
            if (IsIconic(h)) ShowWindow(h, 9);   // SW_RESTORE
            SetForegroundWindow(h);
            return;
        }
    }

    private static IEnumerable<Process> MatchingProcesses(string[] dirs)
    {
        if (dirs.Length == 0) yield break;
        int self = Environment.ProcessId;
        foreach (var p in Process.GetProcesses())
        {
            if (p.Id == self || p.Id <= 4) { p.Dispose(); continue; }
            var path = ImagePath(p.Id);
            if (path is not null && dirs.Any(d => path.StartsWith(d, StringComparison.OrdinalIgnoreCase))) yield return p;
            else p.Dispose();
        }
    }

    // QueryFullProcessImageName works for most processes without admin, unlike Process.MainModule.
    private const uint PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool QueryFullProcessImageName(IntPtr h, int flags, StringBuilder name, ref int size);

    private static string? ImagePath(int pid)
    {
        var h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if (h == IntPtr.Zero) return null;
        try
        {
            var sb = new StringBuilder(1024);
            int size = sb.Capacity;
            return QueryFullProcessImageName(h, 0, sb, ref size) ? sb.ToString(0, size) : null;
        }
        finally { CloseHandle(h); }
    }

    public void Dispose() => _timer.Dispose();
}
