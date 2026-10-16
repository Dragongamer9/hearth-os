using System.Diagnostics;
using Windows.Management.Deployment;

namespace Nimbus.Host.Library;

/// <summary>What the UI receives for each game.</summary>
public sealed record GameDto(
    string Id, string Title, string Source, bool Fav, bool Hidden,
    int Minutes, long LastPlayed, long Added, long? SizeBytes,
    Dictionary<string, string> Art, string? Path, bool CanRemove,
    bool Installed, string State, double? Progress, long? BytesDone, long? BytesTotal, long? BytesPerSec = null);

public sealed record LibraryDto(List<GameDto> Games, Dictionary<string, bool> Sources, Dictionary<string, int> Counts,
                                string? RunningId, bool SteamBackground);

public sealed record DownloadsDto(List<GameDto> Active, bool Changed);

/// <summary>Scans all stores, merges Nimbus's own data, launches games and tracks them.</summary>
public sealed class LibraryService : IDisposable
{
    public const string ArtHost = "nimbus-art.local";

    private readonly LibraryStore _store = new();
    private readonly GameMonitor _monitor = new();
    private Dictionary<string, Game> _games = new();
    private Dictionary<string, int> _counts = new();
    private readonly SemaphoreSlim _scanLock = new(1, 1);

    public event Action<string>? GameStarted;
    public event Action<GameDto>? GameExited;
    public event Action<string>? GameStartTimedOut;

    public LibraryService()
    {
        _monitor.Started += id => GameStarted?.Invoke(id);
        _monitor.StartTimedOut += id => GameStartTimedOut?.Invoke(id);
        _monitor.Exited += (id, minutes) =>
        {
            _store.Update(d =>
            {
                if (!d.Stats.TryGetValue(id, out var s)) d.Stats[id] = s = new GameStats();
                s.LastPlayed = Time.NowMs;
                s.Minutes += minutes;
            });
            if (_games.TryGetValue(id, out var g)) GameExited?.Invoke(ToDto(g));
        };
    }

    // ------------------------------------------------------------------ scanning

    public async Task<LibraryDto> GetAsync(bool rescan)
    {
        if (rescan || _games.Count == 0) await ScanAsync();
        return new LibraryDto(
            _games.Values.Select(ToDto).OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase).ToList(),
            _store.Read(d => new Dictionary<string, bool>(d.Sources)),
            _counts,
            _monitor.RunningId,
            _store.Read(d => d.SteamBackground));
    }

    private async Task ScanAsync()
    {
        await _scanLock.WaitAsync();
        try
        {
            var sources = _store.Read(d => new Dictionary<string, bool>(d.Sources));
            bool On(string s) => !sources.TryGetValue(s, out var on) || on;

            var tasks = new Dictionary<string, Task<List<Game>>>
            {
                ["steam"] = On("steam") ? Task.Run(SteamScanner.Scan) : Task.FromResult(new List<Game>()),
                ["epic"] = On("epic") ? Task.Run(EpicScanner.Scan) : Task.FromResult(new List<Game>()),
                ["gog"] = On("gog") ? Task.Run(GogScanner.Scan) : Task.FromResult(new List<Game>()),
                ["xbox"] = On("xbox") ? Task.Run(XboxScanner.Scan) : Task.FromResult(new List<Game>()),
                ["manual"] = Task.Run(ManualGames),
            };
            var counts = new Dictionary<string, int>();
            var all = new Dictionary<string, Game>();
            foreach (var (src, task) in tasks)
            {
                List<Game> list;
                try { list = await task; } catch { list = new(); } // one broken store shouldn't break the library
                counts[src] = list.Count(g => g.Installed);
                foreach (var g in list) all.TryAdd(g.Id, g);
            }

            // Owned-but-not-installed Steam games come from Steam's binary caches (slower, so after the rest).
            if (On("steam"))
            {
                var installedIds = all.Values.Where(g => g.Source == "steam").Select(g => g.Id["steam:".Length..]).ToHashSet();
                List<Game> owned;
                try { owned = await Task.Run(() => SteamScanner.ScanOwned(installedIds)); }
                catch { owned = new(); }
                foreach (var g in owned) all.TryAdd(g.Id, g);
                counts["steamOwned"] = all.Values.Count(g => g.Source == "steam");
            }
            _games = all;
            _counts = counts;
            _downloadSignature = Signature(all.Values);
        }
        finally { _scanLock.Release(); }
    }

    private List<Game> ManualGames() => _store.Read(d => d.Manual.Select(m =>
    {
        var g = new Game
        {
            Id = m.Id,
            Title = m.Title,
            Source = "manual",
            InstallPath = m.Path,
            Added = m.Added,
            Exe = m.Path,
            Args = m.Args,
            WorkDir = Path.GetDirectoryName(m.Path),
        };
        // Only real executables can be tracked by folder; shortcuts (.lnk/.url) launch untracked.
        if (m.Path.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) && Path.GetDirectoryName(m.Path) is { } dir)
            g.ProcessDirs.Add(dir);
        return g;
    }).ToList());

    private GameDto ToDto(Game g)
    {
        var (fav, hidden, stats) = _store.Read(d => (d.Favorites.Contains(g.Id), d.Hidden.Contains(g.Id),
                                                     d.Stats.TryGetValue(g.Id, out var s) ? s : null));
        // Steam counts its own play time; for other stores Nimbus's numbers are the only ones.
        int minutes = Math.Max(g.StoreMinutes, stats?.Minutes ?? 0);
        long last = Math.Max(g.StoreLastPlayed, stats?.LastPlayed ?? 0);
        // Local files are served through the art host; web URLs (Steam CDN) go to the UI as-is.
        var art = g.Art.ToDictionary(kv => kv.Key, kv => kv.Value.StartsWith("https://", StringComparison.OrdinalIgnoreCase)
            ? kv.Value : $"https://{ArtHost}/{Uri.EscapeDataString(g.Id)}/{kv.Key}");
        return new GameDto(g.Id, g.Title, g.Source, fav, hidden, minutes, last, g.Added, g.SizeBytes, art,
                           g.Installed ? g.InstallPath : null, g.Source == "manual",
                           g.Installed, g.State, g.Progress, g.BytesDone, g.BytesTotal);
    }

    /// <summary>Local file for https://nimbus-art.local/{id}/{kind}, or null.</summary>
    public string? ArtFile(string id, string kind) =>
        _games.TryGetValue(id, out var g) && g.Art.TryGetValue(kind, out var f) && !f.StartsWith("https://", StringComparison.OrdinalIgnoreCase) ? f : null;

    // ------------------------------------------------------------------ installs and downloads

    private string _downloadSignature = "";

    private static string Signature(IEnumerable<Game> games) =>
        string.Join(";", games.Where(g => g.State != "installed" && g.State != "notInstalled")
                              .OrderBy(g => g.Id).Select(g => g.Id + "=" + g.State));

    /// <summary>
    /// Cheap refresh of Steam's install records only. Returns what's downloading now, and whether the set of
    /// downloads changed (something started, finished or was cancelled), in which case the UI reloads the library.
    /// </summary>
    public async Task<DownloadsDto> DownloadsAsync()
    {
        if (!_store.Read(d => !d.Sources.TryGetValue("steam", out var on) || on))
            return new DownloadsDto(new(), false);

        var manifests = await Task.Run(SteamScanner.Scan);
        var games = new Dictionary<string, Game>(_games);
        foreach (var g in manifests) games[g.Id] = g;            // newer state wins
        // A download that was cancelled removes its manifest: show the game as not installed again.
        foreach (var old in _games.Values.Where(x => x.Source == "steam" && x.State != "notInstalled" && x.State != "installed"))
            if (!manifests.Any(m => m.Id == old.Id)) games.Remove(old.Id);
        _games = games;

        var sig = Signature(games.Values);
        bool changed = sig != _downloadSignature;
        _downloadSignature = sig;
        var active = games.Values.Where(g => g.State is not ("installed" or "notInstalled")).Select(ToDto).ToList();
        return new DownloadsDto(active.Select(Estimate).ToList(), changed);
    }

    // ------------------------------------------------------------------ live download estimate
    //
    // Steam only writes BytesDownloaded to the app manifest at checkpoints, so between them the
    // progress would sit still. While a single Steam download is running we add the bytes this PC has
    // received since the last checkpoint (Steam is normally the only big download), capped below the
    // total. The next checkpoint corrects any drift.

    private string? _estId;
    private long _estCheckpoint, _estNetAtCheckpoint, _lastNet;
    private DateTime _lastNetAt;
    private long _speed;

    private static long NetReceived()
    {
        long total = 0;
        foreach (var nic in System.Net.NetworkInformation.NetworkInterface.GetAllNetworkInterfaces())
        {
            if (nic.OperationalStatus != System.Net.NetworkInformation.OperationalStatus.Up) continue;
            if (nic.NetworkInterfaceType is System.Net.NetworkInformation.NetworkInterfaceType.Loopback
                or System.Net.NetworkInformation.NetworkInterfaceType.Tunnel) continue;
            try { total += nic.GetIPStatistics().BytesReceived; } catch { }
        }
        return total;
    }

    private GameDto Estimate(GameDto d)
    {
        if (d.State != "downloading" || d.BytesTotal is not { } total || total <= 0) return d;
        long net = NetReceived();
        var now = DateTime.UtcNow;
        long done = d.BytesDone ?? 0;

        if (_estId != d.Id || done != _estCheckpoint)
        {
            _estId = d.Id; _estCheckpoint = done; _estNetAtCheckpoint = net;
        }
        if (_lastNetAt != default && now > _lastNetAt)
        {
            double secs = (now - _lastNetAt).TotalSeconds;
            long rate = (long)Math.Max(0, (net - _lastNet) / secs);
            _speed = _speed == 0 ? rate : (long)(_speed * 0.6 + rate * 0.4); // smoothed
        }
        _lastNet = net; _lastNetAt = now;

        long estimate = Math.Clamp(done + (net - _estNetAtCheckpoint), done, (long)(total * 0.99));
        return d with { BytesDone = estimate, Progress = (double)estimate / total, BytesPerSec = _speed };
    }

    /// <summary>Asks the store to install the game. For Steam this opens Steam's install dialog.</summary>
    public async Task InstallAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g) || g.InstallUri is null)
            throw new InvalidOperationException("Nimbus can't install this game. Install it from its store app.");
        if (g.Source == "steam" && SteamBackground)
        {
            await SteamScanner.EnsureRunningSilentAsync();
            SteamWindows.HideMainFor(20);
        }
        SteamWindows.AllowSteamToFocus();
        Process.Start(new ProcessStartInfo(g.InstallUri) { UseShellExecute = true });
        // Steam brings its main window forward for steam:// requests; keep only the Install dialog.
        if (g.Source == "steam" && SteamBackground) SteamWindows.HideMainFor(20);
        else if (g.Source == "steam") SteamWindows.HideMainFor(0);
    }

    // ------------------------------------------------------------------ Steam in the background

    public bool SteamBackground => _store.Read(d => d.SteamBackground);
    public bool AutoUpdateCheck { get => _store.Read(d => d.AutoUpdateCheck); set => _store.Update(d => d.AutoUpdateCheck = value); }
    public string ControllerMouse { get => _store.Read(d => d.ControllerMouse); set => _store.Update(d => d.ControllerMouse = value); }

    public async Task SetSteamBackgroundAsync(bool on)
    {
        _store.Update(d => d.SteamBackground = on);
        if (on) await SteamScanner.EnsureRunningSilentAsync();
    }

    /// <summary>Called when Nimbus starts: bring Steam up hidden so games and installs start without its window.</summary>
    public async Task StartBackgroundServicesAsync()
    {
        if (!SteamBackground || !_store.Read(d => !d.Sources.TryGetValue("steam", out var on) || on)) return;
        // Steam can show its window while it starts or updates even with -silent; hide it for the first minute.
        SteamWindows.HideMainFor(60);
        await SteamScanner.EnsureRunningSilentAsync();
    }

    // ------------------------------------------------------------------ edits

    public void SetFavorite(string id, bool on) => _store.Update(d => { if (on) d.Favorites.Add(id); else d.Favorites.Remove(id); });
    public void SetHidden(string id, bool on) => _store.Update(d => { if (on) d.Hidden.Add(id); else d.Hidden.Remove(id); });

    public async Task SetSourcesAsync(Dictionary<string, bool> sources)
    {
        _store.Update(d => { foreach (var (k, v) in sources) d.Sources[k] = v; });
        await ScanAsync();
    }

    public async Task<GameDto> AddManualAsync(string path)
    {
        string title;
        try
        {
            var info = FileVersionInfo.GetVersionInfo(path);
            title = !string.IsNullOrWhiteSpace(info.ProductName) ? info.ProductName!.Trim() : Path.GetFileNameWithoutExtension(path);
        }
        catch { title = Path.GetFileNameWithoutExtension(path); }

        var m = new ManualGame { Id = "manual:" + Guid.NewGuid().ToString("N")[..12], Title = title, Path = path, Added = Time.NowMs };
        _store.Update(d => d.Manual.Add(m));
        await ScanAsync();
        return ToDto(_games[m.Id]);
    }

    public async Task RemoveManualAsync(string id)
    {
        _store.Update(d =>
        {
            d.Manual.RemoveAll(m => m.Id == id);
            d.Favorites.Remove(id); d.Hidden.Remove(id); d.Stats.Remove(id);
        });
        await ScanAsync();
    }

    // ------------------------------------------------------------------ launching

    /// <summary>Starts the game. Returns false when Nimbus can't watch its process (it then won't show as running).</summary>
    public async Task<bool> LaunchAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g)) throw new InvalidOperationException("That game isn't in the library anymore. Try rescanning.");
        if (!g.Installed) throw new InvalidOperationException($"{g.Title} isn't installed yet.");
        if (g.Source == "steam" && SteamBackground)
        {
            await SteamScanner.EnsureRunningSilentAsync();
            SteamWindows.HideMainFor(20);
        }

        if (g.XboxFamily is not null)
        {
            var pkg = new PackageManager().FindPackagesForUser(string.Empty, g.XboxFamily).FirstOrDefault()
                      ?? throw new InvalidOperationException("The Xbox app can't find this game. Is it still installed?");
            var entries = await pkg.GetAppListEntriesAsync();
            if (entries.Count == 0 || !await entries[0].LaunchAsync())
                throw new InvalidOperationException("Windows couldn't start this game.");
        }
        else if (g.LaunchUri is not null)
        {
            // ShellExecute handles steam:// and com.epicgames.launcher:// without explorer.exe running.
            Process.Start(new ProcessStartInfo(g.LaunchUri) { UseShellExecute = true });
        }
        else if (g.Exe is not null)
        {
            if (!File.Exists(g.Exe)) throw new InvalidOperationException($"File not found: {g.Exe}");
            Process.Start(new ProcessStartInfo(g.Exe, g.Args ?? "")
            {
                UseShellExecute = true,                                 // allows .lnk/.url and games that need admin
                WorkingDirectory = g.WorkDir ?? Path.GetDirectoryName(g.Exe) ?? "",
            });
        }
        else throw new InvalidOperationException("Nimbus doesn't know how to start this game.");

        if (g.ProcessDirs.Count == 0) return false;
        _monitor.Track(g.Id, g.ProcessDirs);
        return true;
    }

    public Task<bool> CloseRunningAsync() => _monitor.CloseAsync();
    public string? RunningId => _monitor.RunningId;

    public void Dispose() => _monitor.Dispose();
}
