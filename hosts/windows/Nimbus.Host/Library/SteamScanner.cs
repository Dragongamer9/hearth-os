using Microsoft.Win32;

namespace Nimbus.Host.Library;

/// <summary>
/// Installed Steam games: libraryfolders.vdf → appmanifest_*.acf in every library.
/// Play time and last played come from the most recent user's localconfig.vdf.
/// Art comes from custom grid images first, then Steam's local library cache.
/// </summary>
public static class SteamScanner
{
    private const long SteamId64Base = 76561197960265728;

    // Redistributables and tools that show up as "apps" but aren't games.
    private static readonly HashSet<string> SkipAppIds = new() { "228980", "250820", "1070560", "1391110", "1628350", "1493710" };
    private static readonly string[] SkipPrefixes = { "Steamworks", "Proton", "Steam Linux Runtime", "SteamVR" };

    public static string? SteamPath()
    {
        var p = Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam", "SteamPath", null) as string
             ?? Registry.GetValue(@"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath", null) as string;
        if (string.IsNullOrEmpty(p)) return null;
        p = p.Replace('/', '\\');
        return Directory.Exists(p) ? p : null;
    }

    // appmanifest StateFlags (EAppState)
    private const int FullyInstalled = 4, UpdateRunning = 256, UpdatePaused = 512, UpdateStarted = 1024,
                      Preallocating = 524288, Downloading = 1048576, Staging = 2097152, Committing = 4194304;
    private const int Busy = UpdateRunning | UpdateStarted | Preallocating | Downloading | Staging | Committing;

    /// <summary>Steam's public image CDN, used when the art isn't cached locally (e.g. uninstalled games).</summary>
    private const string Cdn = "https://cdn.cloudflare.steamstatic.com/steam/apps/";
    private const string AssetCdn = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/";

    /// <summary>Installed games plus games Steam is installing, downloading or updating.</summary>
    public static List<Game> Scan()
    {
        var games = new List<Game>();
        var steam = SteamPath();
        if (steam is null) return games;

        var (gridDir, stats) = UserData(steam);
        var cache = Path.Combine(steam, "appcache", "librarycache");

        foreach (var lib in Libraries(steam))
        {
            var apps = Path.Combine(lib, "steamapps");
            if (!Directory.Exists(apps)) continue;

            foreach (var manifest in Directory.EnumerateFiles(apps, "appmanifest_*.acf"))
            {
                var acf = Vdf.Obj(Vdf.Load(manifest), "AppState");
                var appId = Vdf.Str(acf, "appid");
                var name = Vdf.Str(acf, "name");
                var installDir = Vdf.Str(acf, "installdir");
                if (appId is null || string.IsNullOrWhiteSpace(name) || installDir is null) continue;
                if (SkipAppIds.Contains(appId) || SkipPrefixes.Any(pre => name.StartsWith(pre, StringComparison.OrdinalIgnoreCase))) continue;

                var dir = Path.Combine(apps, "common", installDir);
                long.TryParse(Vdf.Str(acf, "SizeOnDisk"), out var size);
                stats.TryGetValue(appId, out var st);

                int.TryParse(Vdf.Str(acf, "StateFlags"), out var flags);
                long.TryParse(Vdf.Str(acf, "BytesToDownload"), out var toDl);
                long.TryParse(Vdf.Str(acf, "BytesDownloaded"), out var dl);
                bool installed = (flags & FullyInstalled) != 0;
                string state = installed
                    ? ((flags & Busy) != 0 ? "updating" : "installed")
                    : (flags & UpdatePaused) != 0 ? "paused"
                    : (flags & Busy) != 0 ? "downloading"
                    : "queued";

                var g = new Game
                {
                    Id = "steam:" + appId,
                    Title = name,
                    Source = "steam",
                    InstallPath = dir,
                    SizeBytes = size > 0 ? size : null,
                    Added = Time.Ms(Directory.Exists(dir) ? Directory.GetCreationTimeUtc(dir) : File.GetCreationTimeUtc(manifest)),
                    StoreMinutes = st.Minutes,
                    StoreLastPlayed = st.LastPlayed,
                    LaunchUri = "steam://rungameid/" + appId,
                    Installed = installed,
                    State = state,
                    BytesDone = toDl > 0 ? dl : null,
                    BytesTotal = toDl > 0 ? toDl : null,
                    Progress = toDl > 0 ? Math.Clamp((double)dl / toDl, 0, 1) : null,
                    InstallUri = "steam://install/" + appId,
                };
                g.ProcessDirs.Add(dir);
                FindArt(g, appId, gridDir, cache);
                games.Add(g);
            }
        }
        return games;
    }

    /// <summary>
    /// Games the signed-in account owns but hasn't installed, from Steam's local app and license caches.
    /// Only Windows games that have been released are included.
    /// </summary>
    public static List<Game> ScanOwned(IReadOnlySet<string> installedAppIds)
    {
        var games = new List<Game>();
        var steam = SteamPath();
        if (steam is null) return games;

        var appinfo = SteamCache.ReadAppInfo(Path.Combine(steam, "appcache", "appinfo.vdf"));
        var owned = SteamCache.ReadOwnedAppIds(Path.Combine(steam, "appcache", "packageinfo.vdf"));
        var (gridDir, stats) = UserData(steam);
        var cache = Path.Combine(steam, "appcache", "librarycache");

        foreach (var id in owned)
        {
            if (!appinfo.TryGetValue(id, out var app)) continue;
            var appId = id.ToString();
            if (installedAppIds.Contains(appId) || SkipAppIds.Contains(appId)) continue;
            if (!app.Type.Equals("game", StringComparison.OrdinalIgnoreCase)) continue;
            if (app.OsList is { Length: > 0 } os && !os.Contains("windows", StringComparison.OrdinalIgnoreCase)) continue;
            if (app.ReleaseState is { Length: > 0 } rs && !rs.Equals("released", StringComparison.OrdinalIgnoreCase)) continue;
            if (SkipPrefixes.Any(pre => app.Name.StartsWith(pre, StringComparison.OrdinalIgnoreCase))) continue;

            stats.TryGetValue(appId, out var st);
            var g = new Game
            {
                Id = "steam:" + appId,
                Title = app.Name,
                Source = "steam",
                Added = 0,
                StoreMinutes = st.Minutes,
                StoreLastPlayed = st.LastPlayed,
                LaunchUri = "steam://rungameid/" + appId,
                InstallUri = "steam://install/" + appId,
                Installed = false,
                State = "notInstalled",
            };
            // Exact CDN paths from Steam's own metadata beat the guessed ones FindArt falls back to.
            foreach (var (kind, rel) in app.Assets)
                if (!g.Art.ContainsKey(kind) || g.Art[kind].StartsWith("http")) g.Art[kind] = AssetCdn + appId + "/" + rel;
            FindArt(g, appId, gridDir, cache, cdnFallback: app.Assets.Count == 0);
            games.Add(g);
        }
        return games;
    }

    private static IEnumerable<string> Libraries(string steam)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { steam };
        yield return steam;

        var vdf = Vdf.Obj(Vdf.Load(Path.Combine(steam, "steamapps", "libraryfolders.vdf")), "libraryfolders");
        if (vdf is null) yield break;
        foreach (var entry in vdf.Values)
        {
            // New format: "0" { "path" "D:\\SteamLibrary" ... }   Old format: "1" "D:\\SteamLibrary"
            var path = entry is Dictionary<string, object> o ? Vdf.Str(o, "path") : entry as string;
            if (string.IsNullOrEmpty(path)) continue;
            path = path.Replace("\\\\", "\\");
            if (Directory.Exists(path) && seen.Add(path)) yield return path;
        }
    }

    /// <summary>The most recently signed-in Steam account: its name and its cached avatar picture, if any.</summary>
    public static (string? Name, string? AvatarFile) Account()
    {
        var steam = SteamPath();
        if (steam is null) return (null, null);
        var users = Vdf.Obj(Vdf.Load(Path.Combine(steam, "config", "loginusers.vdf")), "users");
        if (users is null) return (null, null);
        foreach (var (id64, v) in users)
        {
            if (v is not Dictionary<string, object> u || Vdf.Str(u, "MostRecent") != "1") continue;
            var avatar = Path.Combine(steam, "config", "avatarcache", id64 + ".png");
            return (Vdf.Str(u, "PersonaName") ?? Vdf.Str(u, "AccountName"), File.Exists(avatar) ? avatar : null);
        }
        return (null, null);
    }

    /// <summary>The signed-in (most recent) Steam user's folder, userdata\&lt;account id&gt;, or null.</summary>
    public static string? AccountDir()
    {
        var steam = SteamPath();
        if (steam is null) return null;
        var userdata = Path.Combine(steam, "userdata");
        if (!Directory.Exists(userdata)) return null;
        var users = Vdf.Obj(Vdf.Load(Path.Combine(steam, "config", "loginusers.vdf")), "users");
        if (users is not null)
            foreach (var (id64, v) in users)
                if (v is Dictionary<string, object> u && Vdf.Str(u, "MostRecent") == "1" && long.TryParse(id64, out var n))
                {
                    var dir = Path.Combine(userdata, (n - SteamId64Base).ToString());
                    if (Directory.Exists(dir)) return dir;
                }
        return Directory.EnumerateDirectories(userdata)
            .Where(d => File.Exists(Path.Combine(d, "config", "localconfig.vdf")))
            .OrderByDescending(d => File.GetLastWriteTimeUtc(Path.Combine(d, "config", "localconfig.vdf")))
            .FirstOrDefault();
    }
    public const long Id64Base = SteamId64Base;

    /// <summary>Custom grid folder and per-app stats for the most recently signed-in Steam user.</summary>
    private static (string? GridDir, Dictionary<string, (int Minutes, long LastPlayed)> Stats) UserData(string steam)
    {
        var stats = new Dictionary<string, (int, long)>();
        var userdata = Path.Combine(steam, "userdata");
        if (!Directory.Exists(userdata)) return (null, stats);

        string? accountDir = null;
        var users = Vdf.Obj(Vdf.Load(Path.Combine(steam, "config", "loginusers.vdf")), "users");
        if (users is not null)
        {
            foreach (var (id64, v) in users)
            {
                if (v is Dictionary<string, object> u && Vdf.Str(u, "MostRecent") == "1" && long.TryParse(id64, out var n))
                {
                    var dir = Path.Combine(userdata, (n - SteamId64Base).ToString());
                    if (Directory.Exists(dir)) accountDir = dir;
                }
            }
        }
        accountDir ??= Directory.EnumerateDirectories(userdata)
            .Where(d => File.Exists(Path.Combine(d, "config", "localconfig.vdf")))
            .OrderByDescending(d => File.GetLastWriteTimeUtc(Path.Combine(d, "config", "localconfig.vdf")))
            .FirstOrDefault();
        if (accountDir is null) return (null, stats);

        var local = Vdf.Load(Path.Combine(accountDir, "config", "localconfig.vdf"));
        var apps = Vdf.Obj(local, "UserLocalConfigStore", "Software", "Valve", "Steam", "apps");
        if (apps is not null)
        {
            foreach (var (appId, v) in apps)
            {
                if (v is not Dictionary<string, object> a) continue;
                int.TryParse(Vdf.Str(a, "Playtime"), out var minutes);
                long.TryParse(Vdf.Str(a, "LastPlayed"), out var lastSec);
                stats[appId] = (minutes, lastSec > 0 ? lastSec * 1000 : 0);
            }
        }
        var grid = Path.Combine(accountDir, "config", "grid");
        return (Directory.Exists(grid) ? grid : null, stats);
    }

    private static void FindArt(Game g, string appId, string? grid, string cache, bool cdnFallback = true)
    {
        // Custom art the user set in Steam (grid folder) wins over Steam's defaults.
        void Pick(string kind, string[] gridNames, string legacyName, string cachePattern)
        {
            if (grid is not null)
            {
                foreach (var n in gridNames)
                {
                    foreach (var ext in new[] { ".png", ".jpg", ".jpeg", ".webp" })
                    {
                        var f = Path.Combine(grid, n + ext);
                        if (File.Exists(f)) { g.Art[kind] = f; return; }
                    }
                }
            }
            var legacy = Path.Combine(cache, legacyName);
            if (File.Exists(legacy)) { g.Art[kind] = legacy; return; }

            // Newer Steam clients: librarycache/<appid>/<hash>/library_600x900.jpg etc.
            var dir = Path.Combine(cache, appId);
            if (!Directory.Exists(dir)) return;
            try
            {
                foreach (var pattern in cachePattern.Split('|'))
                {
                    var f = Directory.EnumerateFiles(dir, pattern, SearchOption.AllDirectories).FirstOrDefault();
                    if (f is not null) { g.Art[kind] = f; break; }
                }
            }
            catch (IOException) { }
            catch (UnauthorizedAccessException) { }
        }

        Pick("cover", new[] { appId + "p" }, $"{appId}_library_600x900.jpg", "library_600x900*.jpg|library_capsule*.jpg");
        Pick("hero", new[] { appId + "_hero" }, $"{appId}_library_hero.jpg", "library_hero*.jpg");
        Pick("logo", new[] { appId + "_logo" }, $"{appId}_logo.png", "logo*.png");
        Pick("wide", new[] { appId }, $"{appId}_header.jpg", "header*.jpg|library_header*.jpg");

        // Anything still missing comes from Steam's public CDN (needs internet; the UI falls back to a gradient).
        if (!cdnFallback) return;
        g.Art.TryAdd("cover", Cdn + appId + "/library_600x900.jpg");
        g.Art.TryAdd("hero", Cdn + appId + "/library_hero.jpg");
        g.Art.TryAdd("logo", Cdn + appId + "/logo.png");
        g.Art.TryAdd("wide", Cdn + appId + "/header.jpg");
    }

    // ------------------------------------------------------------------ background mode

    public static bool IsRunning()
    {
        // Steam records its pid here while it runs.
        var pid = Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam\ActiveProcess", "pid", 0) as int? ?? 0;
        if (pid != 0)
        {
            try { using var p = System.Diagnostics.Process.GetProcessById(pid); return !p.HasExited; }
            catch { /* stale pid */ }
        }
        return System.Diagnostics.Process.GetProcessesByName("steam").Length > 0;
    }

    /// <summary>
    /// Starts Steam with -silent (no window; it lives in the background) if it isn't running.
    /// Returns once Steam is up, so steam:// links don't pop up the full client.
    /// </summary>
    public static async Task EnsureRunningSilentAsync()
    {
        var steam = SteamPath();
        if (steam is null || IsRunning()) return;
        var exe = Path.Combine(steam, "steam.exe");
        if (!File.Exists(exe)) return;

        System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(exe, "-silent") { UseShellExecute = false, WorkingDirectory = steam });
        // Wait for the client to register itself (it may update first), then give it a moment to finish loading.
        for (int i = 0; i < 60; i++)
        {
            await Task.Delay(500);
            var pid = Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam\ActiveProcess", "pid", 0) as int? ?? 0;
            if (pid != 0) { await Task.Delay(3000); return; }
        }
    }
}
