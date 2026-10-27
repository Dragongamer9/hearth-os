using System.Diagnostics;
using Windows.Management.Deployment;

namespace Nimbus.Host.Library;

/// <summary>What the UI receives for each game.</summary>
public sealed record GameDto(
    string Id, string Title, string Source, bool Fav, bool Hidden,
    int Minutes, long LastPlayed, long Added, long? SizeBytes,
    Dictionary<string, string> Art, string? Path, bool CanRemove,
    bool Installed, string State, double? Progress, long? BytesDone, long? BytesTotal, long? BytesPerSec = null,
    bool CanUninstall = false, string? Note = null, string? InstallKind = null,
    List<string>? Cols = null, object? Beacon = null, bool Frozen = false, string? Stage = null);

public sealed record LibraryDto(List<GameDto> Games, Dictionary<string, bool> Sources, Dictionary<string, int> Counts,
                                string? RunningId, bool SteamBackground, object? QuickResume = null, List<object>? Collections = null, bool SteamOnDemand = true);

public sealed record DownloadsDto(List<GameDto> Active, bool Changed);

/// <summary>Scans all stores, merges Nimbus's own data, launches games and tracks them.</summary>
public sealed class LibraryService : IDisposable
{
    public const string ArtHost = "nimbus-art.local";

    private readonly LibraryStore _store = new();
    private readonly GameMonitor _monitor = new();
    private readonly QuickResume _quick = new();
    /// <summary>Quick Resume's list changed: (message for a toast, or null).</summary>
    public event Action<string?>? QuickChanged;
    public Stores.StoreAccounts Accounts { get; }
    private Dictionary<string, Game> _games = new();
    private Dictionary<string, int> _counts = new();
    private readonly SemaphoreSlim _scanLock = new(1, 1);

    public event Action<string>? GameStarted;
    public event Action<GameDto>? GameExited;
    public event Action<string>? GameStartTimedOut;
    /// <summary>An Epic/GOG install Nimbus ran finished: (game id, title, error or null; "cancelled" if you cancelled).</summary>
    public event Action<string, string, string?>? InstallFinished;

    public LibraryService()
    {
        Accounts = new Stores.StoreAccounts(_store);
        Accounts.Installs.Finished += (id, error) =>
        {
            var title = _games.TryGetValue(id, out var g) ? g.Title : id;
            InstallFinished?.Invoke(id, title, error);
        };
        _monitor.Started += id => { SaveRunning(); GameStarted?.Invoke(id); };
        _quick.Changed += closedForRoom => QuickChanged?.Invoke(closedForRoom is { } id && _games.TryGetValue(id, out var cg)
            ? $"Closed {cg.Title} to make room in memory (Quick Resume)" : null);
        _monitor.StartTimedOut += id => GameStartTimedOut?.Invoke(id);
        _monitor.Exited += (id, minutes) =>
        {
            if (_pausedId == id) _pausedId = null;
            SaveRunning();
            _store.Update(d =>
            {
                if (!d.Stats.TryGetValue(id, out var s)) d.Stats[id] = s = new GameStats();
                s.LastPlayed = Time.NowMs;
                s.Minutes += minutes;
            });
            if (_games.TryGetValue(id, out var g)) GameExited?.Invoke(ToDto(g));
            bool cloud = Options(id).CloudSaves;
            // GOG cloud saves go up once the game has closed.
            if (cloud && GogDir(id) is { } gd)
                _ = Task.Run(async () => { try { await Accounts.Gog.SyncSavesAsync(id["gog:".Length..], gd, "up"); } catch (Exception ex) { SystemBridge.Log("gog saves: " + ex.Message); } });
            // Epic cloud saves too, for games Ember installed (legendary knows their save folders).
            if (cloud && g is { LaunchVia: "legendary" })
                _ = Task.Run(async () => { try { await Accounts.Epic.SyncSavesAsync(id["epic:".Length..], "up"); } catch (Exception ex) { SystemBridge.Log("epic saves: " + ex.Message); } });
        };
        RestoreRunning();
    }

    // ------------------------------------------------------------------ the running game, across restarts
    //
    // Like Hearth's restore_quick: if Ember restarts (an update, a crash, switching back from Desktop Mode) while
    // a game plays, it carries on watching it instead of forgetting it. Quick Resume's own list is kept by QuickResume.

    internal sealed record RunningRecord(string Id, string[] Dirs, long StartedAt);
    private static string RunningFile => Path.Combine(Stores.HelperTools.DataDir, "running-game.json");

    private void SaveRunning()
    {
        try
        {
            var id = _monitor.RunningId;
            var dirs = _monitor.RunningDirs;
            if (id is null || dirs.Length == 0 || _monitor.StartedAt is not { } t) { File.Delete(RunningFile); return; }
            File.WriteAllText(RunningFile, System.Text.Json.JsonSerializer.Serialize(new RunningRecord(id, dirs, Time.Ms(t))));
        }
        catch { }
    }

    private void RestoreRunning()
    {
        try
        {
            if (!File.Exists(RunningFile)) return;
            var r = System.Text.Json.JsonSerializer.Deserialize<RunningRecord>(File.ReadAllText(RunningFile));
            if (r is null || r.Dirs.Length == 0 || _quick.Has(r.Id) || GameMonitor.ProcessIds(r.Dirs).Count == 0) { File.Delete(RunningFile); return; }
            _monitor.Adopt(r.Id, r.Dirs, DateTimeOffset.FromUnixTimeMilliseconds(r.StartedAt).UtcDateTime);
            SystemBridge.Log($"game {r.Id} still running after Ember restarted: watching it again");
        }
        catch (Exception ex) { SystemBridge.Log("running game: " + ex.Message); }
    }

    // ------------------------------------------------------------------ scanning

    public async Task<LibraryDto> GetAsync(bool rescan)
    {
        if (rescan || _games.Count == 0) await ScanAsync();
        if (rescan) Accounts.RefreshStale();
        return new LibraryDto(
            _games.Values.Select(ToDto).OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase).ToList(),
            _store.Read(d => new Dictionary<string, bool>(d.Sources)),
            _counts,
            _monitor.RunningId,
            _store.Read(d => d.SteamBackground),
            QuickState(),
            _store.Read(d => d.Collections.Select(c => (object)new { id = c.Id, name = c.Name, games = c.Games.ToList() }).ToList()),
            SteamOnDemand);
    }

    /// <summary>Reads every store again (after Steam took up an install, a sign-in, a new owned-games list).</summary>
    public Task RescanAsync() => ScanAsync();

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
                // Games Nimbus installed itself come first, then the stores' own launchers.
                ["epic"] = On("epic") ? Task.Run(() => Accounts.Installed("epic").Concat(EpicScanner.Scan()).ToList()) : Task.FromResult(new List<Game>()),
                ["gog"] = On("gog") ? Task.Run(() => Accounts.Installed("gog").Concat(GogScanner.Scan()).ToList()) : Task.FromResult(new List<Game>()),
                ["xbox"] = On("xbox") ? Task.Run(XboxScanner.Scan) : Task.FromResult(new List<Game>()),
                ["manual"] = Task.Run(ManualGames),
                // Beacon (BFME and friends), when its Windows version is installed: through beacon.exe.
                ["beacon"] = On("beacon") ? Task.Run(Beacon.Scan) : Task.FromResult(new List<Game>()),
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
                // And the ones on your public Steam Community games page (SteamControl), when Steam's caches don't list them.
                try { foreach (var g in SteamScanner.ScanCommunity(all.Keys.ToHashSet())) all.TryAdd(g.Id, g); } catch { }
                counts["steamOwned"] = all.Values.Count(g => g.Source == "steam");
                SteamStages(all.Values, null);
            }

            // Games on your Epic / GOG / Xbox accounts that aren't installed (when you're signed in).
            foreach (var store in new[] { "epic", "gog", "xbox" })
            {
                if (!On(store)) continue;
                foreach (var g in all.Values.Where(g => g.Source == store)) Accounts.AddArt(g);
                var installedIds = all.Keys.ToHashSet();
                foreach (var g in Accounts.NotInstalled(store, installedIds)) all.TryAdd(g.Id, g);
                counts[store + "Owned"] = all.Values.Count(g => g.Source == store);
            }
            _games = all;
            _counts = counts;
            _downloadSignature = CurrentSignature();
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
        var (fav, hidden, stats, cols) = _store.Read(d => (d.Favorites.Contains(g.Id), d.Hidden.Contains(g.Id),
                                                     d.Stats.TryGetValue(g.Id, out var s) ? s : null,
                                                     d.Collections.Where(c => c.Games.Contains(g.Id)).Select(c => c.Id).ToList()));
        // Steam counts its own play time; for other stores Nimbus's numbers are the only ones.
        int minutes = Math.Max(g.StoreMinutes, stats?.Minutes ?? 0);
        long last = Math.Max(g.StoreLastPlayed, stats?.LastPlayed ?? 0);
        // Local files are served through the art host; web URLs (Steam CDN) go to the UI as-is.
        var art = g.Art.ToDictionary(kv => kv.Key, kv => kv.Value.StartsWith("https://", StringComparison.OrdinalIgnoreCase)
            ? kv.Value : $"https://{ArtHost}/{Uri.EscapeDataString(g.Id)}/{kv.Key}");
        var installKind = g.InstallUri is null ? null
            : g.InstallUri.StartsWith("nimbus-") ? "nimbus"
            : g.InstallUri.StartsWith("steam:") ? "steam"
            : g.InstallUri.StartsWith("beacon:") ? "beacon" : "store";
        var dto = new GameDto(g.Id, g.Title, g.Source, fav, hidden, minutes, last, g.Added, g.SizeBytes, art,
                              g.Installed ? g.InstallPath : null, g.Source == "manual",
                              g.Installed, g.State, g.Progress, g.BytesDone, g.BytesTotal,
                              CanUninstall: g.CanUninstall, Note: g.Note, InstallKind: installKind,
                              Cols: cols, Beacon: g.BeaconInfo, Frozen: _quick.Has(g.Id), Stage: g.Stage);
        // An install Nimbus is running shows as a download.
        if (Accounts.Installs.Get(g.Id) is { } job)
            dto = dto with { State = job.State, Progress = job.Progress, BytesDone = job.BytesDone, BytesTotal = job.BytesTotal, BytesPerSec = job.Speed };
        return dto;
    }

    /// <summary>Local file for https://nimbus-art.local/{id}/{kind}, or null.</summary>
    public string? ArtFile(string id, string kind)
    {
        if (id == "profile") return ProfileFile(kind);
        if (id == "shot") return Screenshots.File(kind);   // Library › Screenshots
        return _games.TryGetValue(id, out var g) && g.Art.TryGetValue(kind, out var f) && !f.StartsWith("https://", StringComparison.OrdinalIgnoreCase) ? f : null;
    }

    // ------------------------------------------------------------------ installs and downloads

    private string _downloadSignature = "";

    private static bool IsActive(string state) => state is not ("installed" or "notInstalled");

    /// <summary>What's downloading (Steam's own downloads plus Nimbus's Epic/GOG installs), as one string to spot changes.</summary>
    private string CurrentSignature()
    {
        var steam = _games.Values.Where(g => IsActive(g.State)).Select(g => g.Id + "=" + g.State);
        var jobs = Accounts.Installs.Active().Select(j => j.GameId + "=" + (j.State == "queued" ? "queued" : "active"));
        return string.Join(";", steam.Concat(jobs).OrderBy(x => x));
    }

    /// <summary>
    /// Cheap refresh of Steam's install records only. Returns what's downloading now, and whether the set of
    /// downloads changed (something started, finished or was cancelled), in which case the UI reloads the library.
    /// </summary>
    public async Task<DownloadsDto> DownloadsAsync()
    {
        if (_store.Read(d => !d.Sources.TryGetValue("steam", out var on) || on))
        {
            var manifests = await Task.Run(SteamScanner.Scan);
            var games = new Dictionary<string, Game>(_games);
            foreach (var g in manifests) games[g.Id] = g;            // newer state wins
            // A download that was cancelled removes its manifest: show the game as not installed again.
            foreach (var old in _games.Values.Where(x => x.Source == "steam" && IsActive(x.State)))
                if (!manifests.Any(m => m.Id == old.Id)) games.Remove(old.Id);
            _games = games;
        }
        var (clog, fresh) = SteamControl.ReadContentLog();
        SteamStages(_games.Values, clog);

        var sig = CurrentSignature();
        bool changed = sig != _downloadSignature;
        _downloadSignature = sig;
        var active = _games.Values.Where(g => IsActive(g.State)).Select(ToDto).Select(Estimate).ToList();
        // Steam's own log says its download speed while it writes it.
        if (fresh && clog.Rate is { } rate && rate > 0)
            for (int i = 0; i < active.Count; i++)
                if (active[i].Source == "steam" && active[i].State == "downloading") { active[i] = active[i] with { BytesPerSec = rate }; break; }
        foreach (var job in Accounts.Installs.Active())
            if (_games.TryGetValue(job.GameId, out var g) && !active.Any(a => a.Id == g.Id)) active.Add(ToDto(g));
        return new DownloadsDto(active, changed);
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
        if (d.Source != "steam" || d.State != "downloading" || d.BytesTotal is not { } total || total <= 0) return d;
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

    /// <summary>Steam downloads: what Steam is doing (its content log), and installs waiting for Steam to take them up.</summary>
    private static void SteamStages(IEnumerable<Game> games, SteamControl.ContentLog? log)
    {
        log ??= SteamControl.ReadContentLog().Log;
        var pending = SteamControl.PendingIds();
        foreach (var g in games.Where(g => g.Source == "steam"))
        {
            g.Stage = null;
            if (!uint.TryParse(g.Id["steam:".Length..], out var app)) continue;
            if (pending.Contains(app) && !g.Installed) { g.State = "queued"; g.Stage = "Waiting for Steam"; continue; }
            if (g.State is "installed" or "notInstalled") continue;
            if (log.Apps.TryGetValue(app, out var e)) g.Stage = e.Reason is { } r ? "Stopped: " + r : e.Stage.Length > 0 ? e.Stage : null;
        }
    }

    // ------------------------------------------------------------------ Steam (SteamControl)

    /// <summary>"Start Steam only when needed" (on by default).</summary>
    public bool SteamOnDemand => _store.Read(d => d.SteamOnDemand);
    /// <summary>Steam starts hidden for games and installs (always, or when needed).</summary>
    private bool SteamHidden => SteamOnDemand || SteamBackground;

    public async Task SetSteamOnDemandAsync(bool on)
    {
        _store.Update(d => { d.SteamOnDemand = on; if (!on) d.SteamBackground = true; });
        if (!on) await SteamControl.EnsureRunningAsync();
    }

    /// <summary>The Steam library new Steam games go to.</summary>
    public string? SteamLibrary()
    {
        var steam = SteamScanner.SteamPath();
        if (steam is null) return null;
        return SteamControl.ChooseLibrary(SteamScanner.Libraries(steam).ToList(), _store.Read(d => d.SteamLibrary), Accounts.InstallRoot);
    }
    public object SteamLibraries()
    {
        var steam = SteamScanner.SteamPath() ?? throw new InvalidOperationException("Steam isn't installed on this PC.");
        var chosen = SteamLibrary();
        var libs = SteamScanner.Libraries(steam).ToList();
        return new { list = libs.Select((l, i) => new { path = l, free = SteamControl.FreeBytes(l), main = i == 0, chosen = string.Equals(l, chosen, StringComparison.OrdinalIgnoreCase) }).ToList(), chosen };
    }
    public void SetSteamLibrary(string path)
    {
        var steam = SteamScanner.SteamPath() ?? throw new InvalidOperationException("Steam isn't installed on this PC.");
        if (!SteamScanner.Libraries(steam).Any(l => string.Equals(l, path, StringComparison.OrdinalIgnoreCase))) throw new InvalidOperationException("That folder isn't a Steam library.");
        _store.Update(d => d.SteamLibrary = path);
    }

    /// <summary>Installs a Steam game without Steam's dialog (SteamControl.InstallAsync): {silent, waiting}.</summary>
    public async Task<object> InstallSteamAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g)) throw new InvalidOperationException("That game isn't in the library anymore.");
        if (g.Installed) throw new InvalidOperationException($"{g.Title} is already installed.");
        if (!uint.TryParse(id["steam:".Length..], out var app)) throw new InvalidOperationException("That isn't a Steam game.");
        var lib = SteamLibrary() ?? throw new InvalidOperationException("Steam isn't installed on this PC.");
        var r = await SteamControl.InstallAsync(app, g.Title, lib);
        g.State = "queued"; g.Stage = "Waiting for Steam";
        return r;
    }

    /// <summary>Deletes a Steam game without Steam's dialog (SteamControl.UninstallAsync), then reads the library again.</summary>
    public async Task UninstallSteamAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g)) throw new InvalidOperationException("That game isn't in the library anymore.");
        if (_monitor.RunningId == id || _quick.Has(id)) throw new InvalidOperationException($"Exit {g.Title} first.");
        if (!uint.TryParse(id["steam:".Length..], out var app)) throw new InvalidOperationException("That isn't a Steam game.");
        bool wasRunning = await SteamControl.UninstallAsync(app, g.Title);
        if (wasRunning && !SteamOnDemand) await SteamControl.EnsureRunningAsync();
        await ScanAsync();
    }

    /// <summary>The Steam card on the Stores page, with what SteamControl knows.</summary>
    private Stores.AccountDto SteamCard(Stores.AccountDto a)
    {
        var owned = SteamControl.LoadOwned();
        var problem = owned["problem"]?.ToString();
        return a with
        {
            Fetched = SteamControl.Num(owned["fetched"]), Busy = SteamControl.OwnedBusy, Signing = SteamControl.SigningIn, Library = SteamLibrary(),
            Hint = problem == "private" ? SteamControl.PrivateNote : null,
            Error = problem is { Length: > 0 } p && p != "private" ? "Couldn't get your Steam games list: " + p : null,
            Note = a.Note ?? (a.SignedIn ? null : "Not signed in"),
        };
    }

    /// <summary>Asks the store to install the game. For Steam this opens Steam's install dialog.</summary>
    public async Task InstallAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g) || g.InstallUri is null)
            throw new InvalidOperationException(_games.TryGetValue(id, out var x) && x.Note is not null
                ? $"{x.Note}. Ember can't install it." : "Ember can't install this game. Install it from its store app.");
        if (g.InstallUri.StartsWith("nimbus-"))
        {
            Accounts.StartInstall(g);   // Epic / GOG: Nimbus downloads it itself
            return;
        }
        // Beacon's games are set up in Beacon (its folder, CD key, community files).
        if (g.InstallUri == "beacon:open") { Beacon.OpenApp(); return; }
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

    /// <summary>Hearth Task Manager's "Steam starts with Ember": the opposite of "Start Steam only when needed".</summary>
    public async Task SetSteamBackgroundAsync(bool on)
    {
        _store.Update(d => { d.SteamBackground = on; d.SteamOnDemand = !on; });
        if (on) await SteamControl.EnsureRunningAsync();
    }
    public bool SteamStartsWithEmber => !SteamOnDemand && SteamBackground;

    /// <summary>Called when Nimbus starts: bring Steam up hidden so games and installs start without its window.</summary>
    public async Task StartBackgroundServicesAsync()
    {
        Accounts.RefreshStale();   // Epic / GOG / Xbox game lists, if they're old
        if (!_store.Read(d => !d.Sources.TryGetValue("steam", out var on) || on)) return;
        // SteamControl: installs Steam should take up, and closing Steam when nothing needs it (with the setting on).
        SteamControl.Start(() => SteamOnDemand, () => _monitor.RunningId?.StartsWith("steam:") == true || _quick.List().Any(e => e.Id.StartsWith("steam:")));
        _ = Task.Run(async () => { await Task.Delay(20_000); SteamControl.RefreshOwned(false); });
        if (SteamOnDemand || !SteamBackground) return;
        // Steam can show its window while it starts or updates even with -silent; hide it for the first minute.
        SteamWindows.HideMainFor(60);
        await SteamControl.EnsureRunningAsync();
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
    public async Task<bool> LaunchAsync(string id, string? beaconMod = null)
    {
        if (!_games.TryGetValue(id, out var g)) throw new InvalidOperationException("That game isn't in the library anymore. Try rescanning.");
        // Put aside with Quick Resume: carry on where it was.
        if (_quick.Has(id)) { await QuickResumeAsync(id); return true; }
        if (_monitor.RunningId == id) { GameMonitor.FocusWindow(g.ProcessDirs.ToArray()); return true; }
        if (!g.Installed) throw new InvalidOperationException($"{g.Title} isn't installed yet.");
        if (_monitor.RunningId is { } other)
        {
            var ot = _games.TryGetValue(other, out var og) ? og.Title : "the game that's running";
            if (!QuickResumeOn) throw new InvalidOperationException($"Exit {ot} first (Quick Access → Now playing)");
            await PutAsideAsync();
        }
        if (g.Source == "steam" && SteamHidden)
        {
            await SteamControl.EnsureRunningAsync();
            SteamWindows.HideMainFor(20);
        }
        if (g.Source == "steam") SteamControl.Touch();

        if (g.BeaconGame is { } bg)
        {
            // Beacon's games: through beacon.exe, with the mod picked (or Beacon's saved choice).
            Beacon.Launch(bg, beaconMod);
        }
        else if (g.LaunchVia == "legendary")
        {
            // Epic cloud saves come down first (games that have them; Game settings can turn it off).
            if (Options(g.Id).CloudSaves)
                try { await Accounts.Epic.SyncSavesAsync(g.Id["epic:".Length..], "down"); } catch (Exception ex) { SystemBridge.Log("epic saves: " + ex.Message); }
            await Accounts.Epic.LaunchAsync(g.Id["epic:".Length..]);
        }
        else if (g.XboxFamily is not null)
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
            // GOG cloud saves come down first, for GOG games Ember installed.
            if (GogDir(g.Id) is { } gd && Options(g.Id).CloudSaves)
                try { await Accounts.Gog.SyncSavesAsync(g.Id["gog:".Length..], gd, "down"); } catch (Exception ex) { SystemBridge.Log("gog saves: " + ex.Message); }
            Process.Start(new ProcessStartInfo(g.Exe, g.Args ?? "")
            {
                UseShellExecute = true,                                 // allows .lnk/.url and games that need admin
                WorkingDirectory = g.WorkDir ?? Path.GetDirectoryName(g.Exe) ?? "",
            });
        }
        else throw new InvalidOperationException("Ember doesn't know how to start this game.");

        if (g.ProcessDirs.Count == 0) return false;
        _monitor.Track(g.Id, g.ProcessDirs);
        return true;
    }

    public Task<bool> CloseRunningAsync() => _monitor.CloseAsync();

    private string? GogDir(string id) => id.StartsWith("gog:") && _store.Read(d => d.GogInstalls.TryGetValue(id["gog:".Length..], out var p) ? p : null) is { } dir ? dir : null;

    // ------------------------------------------------------------------ Quick Resume

    public bool QuickResumeOn { get => _store.Read(d => d.QuickResume); set => _store.Update(d => d.QuickResume = value); }

    public object QuickState() => new
    {
        runningId = _monitor.RunningId,
        on = QuickResumeOn,
        list = _quick.List().Select(e => new { id = e.Id, title = _games.TryGetValue(e.Id, out var g) ? g.Title : e.Id, frozenAt = e.FrozenAt }).ToList(),
    };

    /// <summary>The running game goes aside: frozen where it is. Play time so far is saved now.</summary>
    public async Task PutAsideAsync()
    {
        var title = RunningTitle;
        var d = _monitor.Detach() ?? throw new InvalidOperationException("No game is running.");
        // Paused (Quick Access › Pause): unpaused first. Windows counts freezes, so Quick Resume's own freeze on top
        // would leave it frozen after carrying on with it.
        bool paused = _pausedId == d.Id;
        if (paused) { GameMonitor.SuspendAll(d.Dirs, false); _pausedId = null; }
        List<QuickResume.Entry> over;
        try { over = _quick.Add(d.Id, d.Dirs, title); }
        catch
        {
            _monitor.Adopt(d.Id, d.Dirs, d.Started);   // not frozen: still the running game, same play time
            if (paused && GameMonitor.SuspendAll(d.Dirs, true).Done > 0) _pausedId = d.Id;
            throw;
        }
        SaveRunning();
        foreach (var e in over) await GameMonitor.CloseDirsAsync(e.Dirs);
        var minutes = (int)Math.Round((DateTime.UtcNow - d.Started).TotalMinutes);
        _store.Update(x =>
        {
            if (!x.Stats.TryGetValue(d.Id, out var s)) x.Stats[d.Id] = s = new GameStats();
            s.LastPlayed = Time.NowMs;
            s.Minutes += Math.Max(0, minutes);
        });
    }

    /// <summary>Back to a game that was put aside (the running one goes aside in its place) and in front.</summary>
    public async Task QuickResumeAsync(string id)
    {
        if (!_quick.Has(id)) throw new InvalidOperationException("That game isn't in Quick Resume anymore.");
        if (_monitor.RunningId is not null) await PutAsideAsync();
        var e = _quick.Take(id);
        _monitor.Resume(e.Id, e.Dirs);
        SaveRunning();
        GameMonitor.FocusWindow(e.Dirs);
    }

    public Task QuickCloseAsync(string id) => _quick.CloseAsync(id);
    public bool IsFrozen(string id) => _quick.Has(id);

    /// <summary>Quick Access › Pause: the running game frozen in place (still the game in front, unlike Quick
    /// Resume); Unpause carries on. Refused for games with anti-cheat, like Quick Resume.</summary>
    public bool Pause(bool on)
    {
        var dirs = _monitor.RunningDirs;
        if (dirs.Length == 0) throw new InvalidOperationException("No game is running.");
        if (!on) { GameMonitor.SuspendAll(dirs, false); _pausedId = null; return false; }
        // Windows counts freezes: pausing twice (from the overlay and from Ember's Quick Access) would need two unpauses.
        if (_pausedId is { } p && p == _monitor.RunningId) return true;
        if (QuickResume.AntiCheatOf(dirs) is { } ac) throw new InvalidOperationException($"{RunningTitle ?? "This game"} runs with {ac}, which can treat a frozen game as cheating, so Ember doesn't pause it.");
        var (done, failed) = GameMonitor.SuspendAll(dirs, true);
        if (done == 0 || failed > 0)
        {
            if (done > 0) GameMonitor.SuspendAll(dirs, false);
            throw new InvalidOperationException("Windows didn't let Ember pause this game (it runs as administrator, or it's protected).");
        }
        _pausedId = _monitor.RunningId;
        return true;
    }

    /// <summary>The running game paused with Quick Access › Pause (frozen in place), or null.</summary>
    private string? _pausedId;

    // ------------------------------------------------------------------ collections (Library tabs of your own)

    private const int MaxCollections = 30;
    private static string CleanName(string name) { var t = (name ?? "").Trim(); return t.Length > 40 ? t[..40] : t; }

    public object CollectionCreate(string name)
    {
        var clean = CleanName(name);
        if (clean.Length == 0) throw new InvalidOperationException("Give the collection a name");
        CollectionData? made = null;
        _store.Update(d =>
        {
            if (d.Collections.Count >= MaxCollections) throw new InvalidOperationException($"Ember has room for {MaxCollections} collections");
            if (d.Collections.Any(c => string.Equals(c.Name, clean, StringComparison.CurrentCultureIgnoreCase))) throw new InvalidOperationException($"There's already a collection called {clean}");
            made = new CollectionData { Id = "c" + (DateTime.UtcNow.Ticks % 0xffff_ffff_ffffL).ToString("x"), Name = clean };
            d.Collections.Add(made);
        });
        return new { id = made!.Id, name = made.Name, games = new List<string>() };
    }

    public void CollectionRename(string id, string name)
    {
        var clean = CleanName(name);
        if (clean.Length == 0) throw new InvalidOperationException("Give the collection a name");
        _store.Update(d => (d.Collections.FirstOrDefault(c => c.Id == id) ?? throw new InvalidOperationException("No such collection")).Name = clean);
    }

    public void CollectionRemove(string id) => _store.Update(d => d.Collections.RemoveAll(c => c.Id == id));

    public void CollectionSet(string id, string game, bool on) => _store.Update(d =>
    {
        var c = d.Collections.FirstOrDefault(x => x.Id == id) ?? throw new InvalidOperationException("No such collection");
        c.Games.RemoveAll(x => x == game);
        if (on && game.Length > 0) c.Games.Add(game);
    });

    // ------------------------------------------------------------------ per-game settings

    public GameOptionsData Options(string id) => _store.Read(d => d.GameOptions.TryGetValue(id, out var o)
        ? new GameOptionsData { Tdp = o.Tdp, CloudSaves = o.CloudSaves } : new GameOptionsData());

    public void SetOptions(string id, int? tdp, bool? cloudSaves) => _store.Update(d =>
    {
        if (!d.GameOptions.TryGetValue(id, out var o)) d.GameOptions[id] = o = new GameOptionsData();
        if (tdp is { } t) o.Tdp = Math.Clamp(t, 0, 120);
        if (cloudSaves is { } c) o.CloudSaves = c;
    });

    public Game? Find(string id) => _games.TryGetValue(id, out var g) ? g : null;
    public int TdpWatts { get => _store.Read(d => d.TdpWatts); set => _store.Update(d => d.TdpWatts = value); }

    // ------------------------------------------------------------------ Beacon

    /// <summary>Game settings › Mod: the mod a Beacon game starts with (kept in Beacon), then the list again.</summary>
    public async Task<GameDto> BeaconSelectModAsync(string id, string? mod)
    {
        var g = Find(id) ?? throw new InvalidOperationException("That game isn't in the library anymore.");
        if (g.BeaconGame is not { } bg) throw new InvalidOperationException("That isn't one of Beacon's games.");
        await Task.Run(() => Beacon.SelectMod(bg, mod));
        await ScanAsync();
        return Find(id) is { } now ? ToDto(now) : throw new InvalidOperationException("That game isn't in the library anymore.");
    }

    public IEnumerable<Game> AllGames => _games.Values;

    // ------------------------------------------------------------------ Nimbus-run installs (Epic, GOG)

    public bool CancelInstall(string id) => Accounts.Installs.Cancel(id);

    public async Task UninstallAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g)) throw new InvalidOperationException("That game isn't in the library anymore.");
        // A game put aside with Quick Resume still runs (frozen) and holds its files: deleting its folder now would
        // leave half of it behind.
        if (_monitor.RunningId == id || _quick.Has(id)) throw new InvalidOperationException($"Exit {g.Title} first.");
        await Accounts.UninstallAsync(g);
        await ScanAsync();
    }

    public List<Stores.AccountDto> AccountsStatus()
    {
        var installed = _games.Values.Where(g => g.Installed).GroupBy(g => g.Source).ToDictionary(x => x.Key, x => x.Count());
        return Accounts.Status(installed, _games.Values.Count(g => g.Source == "steam")).Select(a => a.Store == "steam" ? SteamCard(a) : a).ToList();
    }

    // ------------------------------------------------------------------ profile

    private static string ProfileDir => Path.Combine(Stores.HelperTools.DataDir, "profile");

    private static string? ProfileImage(string kind) =>
        Directory.Exists(ProfileDir) ? Directory.EnumerateFiles(ProfileDir, kind + ".*").FirstOrDefault() : null;

    private string? ProfileFile(string kind) => kind switch
    {
        "avatar" or "background" => ProfileImage(kind),
        "steam" => SteamScanner.Account().AvatarFile,
        _ => null,
    };

    public object Profile()
    {
        var p = _store.Read(d => d.Profile);
        string? Url(string kind) => ProfileFile(kind) is null ? null : $"https://{ArtHost}/profile/{kind}?v={p.ImageStamp}";
        var gog = Accounts.Gog.Profile();
        return new
        {
            name = string.IsNullOrWhiteSpace(p.Name) ? Environment.UserName : p.Name,
            status = p.Status,
            avatar = p.Avatar,
            accent = p.Accent,
            background = p.Background,
            customAvatar = Url("avatar"),
            customBackground = Url("background"),
            storeAvatars = new { steam = Url("steam"), xbox = Accounts.Xbox.Gamerpic, gog = gog.Avatar },
        };
    }

    public object SetProfile(string? name, string? status, string? avatar, string? accent, string? background)
    {
        _store.Update(d =>
        {
            var p = d.Profile;
            if (name is not null) p.Name = name.Trim().Length == 0 ? null : name.Trim()[..Math.Min(name.Trim().Length, 32)];
            if (status is not null) p.Status = status.Trim().Length == 0 ? null : status.Trim()[..Math.Min(status.Trim().Length, 60)];
            if (avatar is not null) p.Avatar = avatar;
            if (accent is not null && System.Text.RegularExpressions.Regex.IsMatch(accent, "^#[0-9a-fA-F]{6}$")) p.Accent = accent;
            if (background is not null) p.Background = background;
        });
        return Profile();
    }

    /// <summary>Copies a picture you chose into Nimbus's profile folder and uses it.</summary>
    public object SetProfileImage(string kind, string file)
    {
        if (kind is not ("avatar" or "background")) throw new ArgumentException("Unknown picture");
        var ext = Path.GetExtension(file).ToLowerInvariant();
        if (ext is not (".png" or ".jpg" or ".jpeg" or ".webp" or ".gif")) throw new InvalidOperationException("Pick a PNG, JPG, WebP or GIF picture.");
        if (new FileInfo(file).Length > 20_000_000) throw new InvalidOperationException("That picture is too big (over 20 MB).");
        Directory.CreateDirectory(ProfileDir);
        if (ProfileImage(kind) is { } old) File.Delete(old);
        File.Copy(file, Path.Combine(ProfileDir, kind + ext), overwrite: true);
        _store.Update(d =>
        {
            d.Profile.ImageStamp = Time.NowMs;
            if (kind == "avatar") d.Profile.Avatar = "custom"; else d.Profile.Background = "custom";
        });
        return Profile();
    }
    public string? RunningId => _monitor.RunningId;
    /// <summary>The running game's folders (Hearth Input checks the window in front against them).</summary>
    public string[] RunningDirs => _monitor.RunningDirs;
    public string? RunningTitle => _monitor.RunningId is { } id && _games.TryGetValue(id, out var g) ? g.Title : null;
    public DateTime? RunningSince => _monitor.StartedAt;
    public bool OverlayHoldView { get => _store.Read(d => d.OverlayHoldView); set => _store.Update(d => d.OverlayHoldView = value); }
    public string? OverlayQaStyle { get => _store.Read(d => d.OverlayQaStyle); set => _store.Update(d => d.OverlayQaStyle = value); }
    public string StartMode { get => _store.Read(d => d.StartMode) ?? "game"; set => _store.Update(d => d.StartMode = value); }
    public string DesktopStyle { get => _store.Read(d => d.DesktopStyle) ?? "mantel"; set => _store.Update(d => d.DesktopStyle = value); }
    public string MantelWallpaper { get => _store.Read(d => d.MantelWallpaper) ?? "anim:fireside"; set => _store.Update(d => d.MantelWallpaper = value); }
    public bool MantelNightLight { get => _store.Read(d => d.MantelNightLight); set => _store.Update(d => d.MantelNightLight = value); }
    public bool MantelReduceMotion { get => _store.Read(d => d.MantelReduceMotion); set => _store.Update(d => d.MantelReduceMotion = value); }
    public double PointerSpeed
    {
        get { var v = _store.Read(d => d.PointerSpeed); return v is >= 0.4 and <= 2.0 ? v : 1.0; }
        set => _store.Update(d => d.PointerSpeed = Math.Clamp(value, 0.4, 2.0));
    }

    public void Dispose() { _monitor.Dispose(); _quick.Dispose(); }
}
