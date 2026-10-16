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

    public GameMonitor() => _timer = new System.Threading.Timer(_ => Poll(), null, Timeout.Infinite, Timeout.Infinite);

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
