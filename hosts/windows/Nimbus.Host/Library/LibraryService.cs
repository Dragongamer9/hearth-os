using System.Diagnostics;
using Windows.Management.Deployment;

namespace Nimbus.Host.Library;

/// <summary>What the UI receives for each game.</summary>
public sealed record GameDto(
    string Id, string Title, string Source, bool Fav, bool Hidden,
    int Minutes, long LastPlayed, long Added, long? SizeBytes,
    Dictionary<string, string> Art, string? Path, bool CanRemove,
    bool Installed, string State, double? Progress, long? BytesDone, long? BytesTotal, long? BytesPerSec = null,
    bool CanUninstall = false, string? Note = null, string? InstallKind = null);

public sealed record LibraryDto(List<GameDto> Games, Dictionary<string, bool> Sources, Dictionary<string, int> Counts,
                                string? RunningId, bool SteamBackground);

public sealed record DownloadsDto(List<GameDto> Active, bool Changed);

/// <summary>Scans all stores, merges Nimbus's own data, launches games and tracks them.</summary>
public sealed class LibraryService : IDisposable
{
    public const string ArtHost = "nimbus-art.local";

    private readonly LibraryStore _store = new();
    private readonly GameMonitor _monitor = new();
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
        if (rescan) Accounts.RefreshStale();
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
                // Games Nimbus installed itself come first, then the stores' own launchers.
                ["epic"] = On("epic") ? Task.Run(() => Accounts.Installed("epic").Concat(EpicScanner.Scan()).ToList()) : Task.FromResult(new List<Game>()),
                ["gog"] = On("gog") ? Task.Run(() => Accounts.Installed("gog").Concat(GogScanner.Scan()).ToList()) : Task.FromResult(new List<Game>()),
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
        var (fav, hidden, stats) = _store.Read(d => (d.Favorites.Contains(g.Id), d.Hidden.Contains(g.Id),
                                                     d.Stats.TryGetValue(g.Id, out var s) ? s : null));
        // Steam counts its own play time; for other stores Nimbus's numbers are the only ones.
        int minutes = Math.Max(g.StoreMinutes, stats?.Minutes ?? 0);
        long last = Math.Max(g.StoreLastPlayed, stats?.LastPlayed ?? 0);
        // Local files are served through the art host; web URLs (Steam CDN) go to the UI as-is.
        var art = g.Art.ToDictionary(kv => kv.Key, kv => kv.Value.StartsWith("https://", StringComparison.OrdinalIgnoreCase)
            ? kv.Value : $"https://{ArtHost}/{Uri.EscapeDataString(g.Id)}/{kv.Key}");
        var installKind = g.InstallUri is null ? null
            : g.InstallUri.StartsWith("nimbus-") ? "nimbus"
            : g.InstallUri.StartsWith("steam:") ? "steam" : "store";
        var dto = new GameDto(g.Id, g.Title, g.Source, fav, hidden, minutes, last, g.Added, g.SizeBytes, art,
                              g.Installed ? g.InstallPath : null, g.Source == "manual",
                              g.Installed, g.State, g.Progress, g.BytesDone, g.BytesTotal,
                              CanUninstall: g.CanUninstall, Note: g.Note, InstallKind: installKind);
        // An install Nimbus is running shows as a download.
        if (Accounts.Installs.Get(g.Id) is { } job)
            dto = dto with { State = job.State, Progress = job.Progress, BytesDone = job.BytesDone, BytesTotal = job.BytesTotal, BytesPerSec = job.Speed };
        return dto;
    }

    /// <summary>Local file for https://nimbus-art.local/{id}/{kind}, or null.</summary>
    public string? ArtFile(string id, string kind)
    {
        if (id == "profile") return ProfileFile(kind);
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

        var sig = CurrentSignature();
        bool changed = sig != _downloadSignature;
        _downloadSignature = sig;
        var active = _games.Values.Where(g => IsActive(g.State)).Select(ToDto).Select(Estimate).ToList();
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
        Accounts.RefreshStale();   // Epic / GOG / Xbox game lists, if they're old
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

        if (g.LaunchVia == "legendary")
        {
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

    // ------------------------------------------------------------------ Nimbus-run installs (Epic, GOG)

    public bool CancelInstall(string id) => Accounts.Installs.Cancel(id);

    public async Task UninstallAsync(string id)
    {
        if (!_games.TryGetValue(id, out var g)) throw new InvalidOperationException("That game isn't in the library anymore.");
        if (_monitor.RunningId == id) throw new InvalidOperationException($"Exit {g.Title} first.");
        await Accounts.UninstallAsync(g);
        await ScanAsync();
    }

    public List<Stores.AccountDto> AccountsStatus()
    {
        var installed = _games.Values.Where(g => g.Installed).GroupBy(g => g.Source).ToDictionary(x => x.Key, x => x.Count());
        return Accounts.Status(installed, _games.Values.Count(g => g.Source == "steam"));
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
    public string? RunningTitle => _monitor.RunningId is { } id && _games.TryGetValue(id, out var g) ? g.Title : null;
    public DateTime? RunningSince => _monitor.StartedAt;
    public bool OverlayHoldView { get => _store.Read(d => d.OverlayHoldView); set => _store.Update(d => d.OverlayHoldView = value); }
    public string StartMode { get => _store.Read(d => d.StartMode) ?? "game"; set => _store.Update(d => d.StartMode = value); }
    public string DesktopStyle { get => _store.Read(d => d.DesktopStyle) ?? "mantel"; set => _store.Update(d => d.DesktopStyle = value); }
    public string MantelWallpaper { get => _store.Read(d => d.MantelWallpaper) ?? "hearth-cabin"; set => _store.Update(d => d.MantelWallpaper = value); }

    public void Dispose() => _monitor.Dispose();
}
