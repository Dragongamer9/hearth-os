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

    /// <summary>Freezes the game and puts it first in the list. Returns entries pushed out (to close).</summary>
    public List<Entry> Add(string id, string[] dirs)
    {
        if (GameMonitor.Suspend(dirs, true) == 0) throw new InvalidOperationException("Couldn't put the game aside. It may be running as administrator.");
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
