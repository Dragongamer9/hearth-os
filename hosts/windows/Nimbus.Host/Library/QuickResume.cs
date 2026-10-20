using System.Runtime.InteropServices;
using System.Text.Json;

namespace Nimbus.Host.Library;

/// <summary>
/// Quick Resume, like Xbox's: starting another game puts the one you're playing aside, frozen where you left it
/// (no processor or graphics use), and picking it later carries on. Up to 5 games; putting aside one more closes
/// the oldest, and so does Windows running low on memory. Kept in a file so they're still listed if Ember restarts.
/// Port of the Hearth (Rust) host's Quick Resume.
/// </summary>
public sealed class QuickResume : IDisposable
{
    public const int Max = 5;
    public sealed record Entry(string Id, string[] Dirs, long FrozenAt);

    private readonly object _lock = new();
    private readonly List<Entry> _frozen = new();   // newest first
    private readonly System.Threading.Timer _timer;
    private static string FilePath => Path.Combine(Stores.HelperTools.DataDir, "quick-resume.json");

    /// <summary>The list changed by itself (a game was closed some other way, or to make room): (message or null).</summary>
    public event Action<string?>? Changed;

    public QuickResume()
    {
        try
        {
            if (File.Exists(FilePath))
                foreach (var e in JsonSerializer.Deserialize<List<Entry>>(File.ReadAllText(FilePath)) ?? new())
                    if (GameMonitor.ProcessIds(e.Dirs).Count > 0) _frozen.Add(e);   // still there (not after a restart of Windows)
        }
        catch { }
        _timer = new System.Threading.Timer(_ => Tick(), null, TimeSpan.FromSeconds(5), TimeSpan.FromSeconds(5));
    }

    public List<Entry> List() { lock (_lock) return _frozen.ToList(); }
    public bool Has(string id) { lock (_lock) return _frozen.Any(e => e.Id == id); }

    private void Save() { try { File.WriteAllText(FilePath, JsonSerializer.Serialize(_frozen)); } catch { } }

    // Anti-cheat that runs with a game (in its folder, or as its service): freezing a game it protects can look like
    // tampering (a kick, or worse), and its protected processes refuse to be frozen anyway. Lowercase name starts.
    private static readonly string[] AntiCheatInGame = { "easyanticheat", "eac_launcher", "start_protected_game", "beservice", "battleye", "bedaisy",
                                                          "eaanticheat", "anticheatexpert", "ace-base", "xigncode", "gameguard", "nprotect", "mhyprot", "vanguard" };
    // Only the services that run while a protected game runs (not ones that are always on, like Vanguard or FACEIT,
    // which would stop Quick Resume for every game).
    private static readonly (string Process, string Name)[] AntiCheatServices =
    {
        ("EasyAntiCheat", "Easy Anti-Cheat"), ("EasyAntiCheat_EOS", "Easy Anti-Cheat"), ("BEService", "BattlEye"), ("BEService_x64", "BattlEye"),
        ("EAAntiCheat.GameService", "EA javelin anti-cheat"),
    };

    /// <summary>The anti-cheat the game seems to run with (by its processes' names), or null.</summary>
    public static string? AntiCheatOf(string[] dirs)
    {
        foreach (var n in GameMonitor.ProcessNames(dirs))
        {
            var l = n.ToLowerInvariant();
            if (AntiCheatInGame.FirstOrDefault(a => l.StartsWith(a)) is { } hit)
                return hit switch { "beservice" or "battleye" or "bedaisy" => "BattlEye", "eaanticheat" => "EA javelin anti-cheat", "vanguard" => "Riot Vanguard", "xigncode" => "XIGNCODE", "gameguard" or "nprotect" => "GameGuard", _ => "Easy Anti-Cheat" };
        }
        foreach (var (proc, name) in AntiCheatServices)
        {
            var ps = System.Diagnostics.Process.GetProcessesByName(proc);
            bool found = ps.Length > 0;
            foreach (var p in ps) p.Dispose();
            if (found) return name;
        }
        return null;
    }

    /// <summary>Freezes the game and puts it first in the list. Returns entries pushed out (to close). Refused (with
    /// the reason) for a game protected by anti-cheat, and for one whose processes Windows won't let Ember freeze
    /// (running as administrator): those are left running as they were.</summary>
    public List<Entry> Add(string id, string[] dirs, string? title = null)
    {
        var name = title ?? "The game";
        if (AntiCheatOf(dirs) is { } ac)
        {
            SystemBridge.Log($"quick resume: {id} not put aside: {ac} runs");
            throw new InvalidOperationException($"{name} runs with {ac}, which can treat a frozen game as cheating, so Ember doesn't put it aside. Exit it first (Quick Access › Now playing).");
        }
        var (done, failed) = GameMonitor.SuspendAll(dirs, true);
        if (done == 0 || failed > 0)
        {
            // Half frozen is worse than not at all: everything runs again.
            if (done > 0) GameMonitor.SuspendAll(dirs, false);
            SystemBridge.Log($"quick resume: {id} not put aside: froze {done}, {failed} refused");
            throw new InvalidOperationException(done == 0 && failed == 0
                ? $"Couldn't put {name} aside: its program isn't running anymore."
                : $"Windows didn't let Ember freeze {name} (it runs as administrator, or it's protected), so it's still running. Exit it first (Quick Access › Now playing).");
        }
        SystemBridge.Log($"quick resume: {id} put aside ({done} processes frozen)");
        lock (_lock)
        {
            _frozen.RemoveAll(e => e.Id == id);
            _frozen.Insert(0, new Entry(id, dirs, Time.NowMs));
            var over = _frozen.Skip(Max).ToList();
            _frozen.RemoveRange(Math.Min(Max, _frozen.Count), over.Count);
            Save();
            return over;
        }
    }

    /// <summary>Takes a game out of the list and unfreezes it.</summary>
    public Entry Take(string id)
    {
        Entry e;
        lock (_lock)
        {
            e = _frozen.FirstOrDefault(x => x.Id == id) ?? throw new InvalidOperationException("That game isn't in Quick Resume anymore.");
            _frozen.Remove(e);
            Save();
        }
        GameMonitor.Suspend(e.Dirs, false);
        return e;
    }

    /// <summary>Takes a game out of the list and closes it.</summary>
    public async Task CloseAsync(string id)
    {
        Entry? e;
        lock (_lock) { e = _frozen.FirstOrDefault(x => x.Id == id); if (e is not null) { _frozen.Remove(e); Save(); } }
        if (e is null) throw new InvalidOperationException("That game isn't in Quick Resume anymore.");
        await GameMonitor.CloseDirsAsync(e.Dirs);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MemoryStatus { public uint Length, Load; public ulong TotalPhys, AvailPhys, TotalPage, AvailPage, TotalVirtual, AvailVirtual, AvailExtended; }
    [DllImport("kernel32.dll")] private static extern bool GlobalMemoryStatusEx(ref MemoryStatus m);

    private async void Tick()
    {
        List<Entry> snapshot;
        lock (_lock) snapshot = _frozen.ToList();
        if (snapshot.Count == 0) return;
        bool changed = false;
        foreach (var e in snapshot.Where(e => GameMonitor.ProcessIds(e.Dirs).Count == 0))
        {
            lock (_lock) { _frozen.Remove(e); Save(); }
            changed = true;
        }
        string? msg = null;
        var m = new MemoryStatus { Length = (uint)Marshal.SizeOf<MemoryStatus>() };
        if (GlobalMemoryStatusEx(ref m) && m.AvailPhys < Math.Max(1UL << 30, m.TotalPhys / 14))
        {
            Entry? old;
            lock (_lock) old = _frozen.LastOrDefault();
            if (old is not null) { try { await CloseAsync(old.Id); } catch { } msg = old.Id; changed = true; }
        }
        if (changed) Changed?.Invoke(msg);
    }

    public void Dispose() => _timer.Dispose();
}
