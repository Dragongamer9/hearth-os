using System.Diagnostics;
using System.Text.Json;
using Nimbus.Host.Library;

namespace Nimbus.Host.Stores;

/// <summary>
/// Epic Games without the Epic Games Launcher, through legendary (github.com/legendary-gl/legendary).
/// legendary keeps its sign-in and install records in %LOCALAPPDATA%\Nimbus\legendary.
/// </summary>
public sealed class EpicStore
{
    public const string ClientId = "34a02cf8f4414e29b15921876da36f9a";
    private static readonly string RedirectUrl = $"https://www.epicgames.com/id/api/redirect?clientId={ClientId}&responseType=code";
    public static readonly string LoginUrl = "https://www.epicgames.com/id/login?redirectUrl=" + Uri.EscapeDataString(RedirectUrl);

    private static string ConfigDir => Path.Combine(HelperTools.DataDir, "legendary");
    private static string CachePath => Path.Combine(HelperTools.DataDir, "cache", "epic-owned.json");
    private static Dictionary<string, string> Env => new() { ["LEGENDARY_CONFIG_PATH"] = ConfigDir };

    public OwnedCache Owned { get; private set; } = OwnedCache.Load(CachePath);

    public bool SignedIn => File.Exists(Path.Combine(ConfigDir, "user.json"));

    public string? DisplayName()
    {
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(Path.Combine(ConfigDir, "user.json")));
            return doc.RootElement.Str("displayName");
        }
        catch { return null; }
    }

    /// <summary>Shows Epic's sign-in page; returns false if you closed it.</summary>
    public async Task<bool> SignInAsync(IWin32Window? owner)
    {
        var tool = HelperTools.EnsureAsync(HelperTools.Legendary);   // download while you sign in
        var code = await SignInWindow.RunAsync(owner, "Sign in to Epic Games", LoginUrl, onLoaded: async (url, core) =>
        {
            if (!url.StartsWith("https://www.epicgames.com/id/api/redirect", StringComparison.OrdinalIgnoreCase)) return null;
            using var doc = await SignInWindow.ReadJsonAsync(core);
            var c = doc?.RootElement.Str("authorizationCode");
            if (!string.IsNullOrEmpty(c)) return c;
            core.Navigate(LoginUrl);   // not signed in yet
            return null;
        });
        if (code is null) return false;
        await tool;
        var r = await HelperTools.RunAsync(HelperTools.Legendary, new[] { "auth", "--code", code }, Env, timeout: TimeSpan.FromMinutes(2));
        if (!SignedIn) throw new InvalidOperationException("Epic sign-in didn't finish: " + r.Problem());
        return true;
    }

    public async Task SignOutAsync()
    {
        if (HelperTools.IsReady(HelperTools.Legendary))
            await HelperTools.RunAsync(HelperTools.Legendary, new[] { "auth", "--delete" }, Env, timeout: TimeSpan.FromMinutes(1));
        try { File.Delete(Path.Combine(ConfigDir, "user.json")); } catch { }
        Owned = new OwnedCache();
        try { File.Delete(CachePath); } catch { }
    }

    /// <summary>Downloads the list of games on your Epic account.</summary>
    public async Task RefreshOwnedAsync()
    {
        var r = await HelperTools.RunAsync(HelperTools.Legendary, new[] { "list", "--json" }, Env, timeout: TimeSpan.FromMinutes(4));
        var json = r.Out.Trim();
        int start = json.IndexOf('[');
        if (r.ExitCode != 0 || start < 0) throw new InvalidOperationException("Couldn't get your Epic library: " + r.Problem());
        using var doc = JsonDocument.Parse(json[start..]);
        var cache = new OwnedCache { Fetched = Time.NowMs };
        foreach (var g in doc.RootElement.EnumerateArray())
        {
            var app = g.Str("app_name"); var title = g.Str("app_title");
            if (app is null || title is null) continue;
            var meta = g.Obj("metadata");
            var o = new OwnedGame { Id = app, Title = title.Trim(), Added = J.Ms(meta.Str("creationDate")) };
            foreach (var img in meta.Arr("keyImages"))
            {
                var url = img.Str("url"); if (url is null) continue;
                switch (img.Str("type"))
                {
                    case "DieselGameBoxTall": o.Art["cover"] = url; break;
                    case "DieselGameBox": o.Art["hero"] = url; o.Art.TryAdd("wide", url); break;
                    case "DieselGameBoxWide": o.Art["wide"] = url; break;
                    case "DieselGameBoxLogo": o.Art["logo"] = url; break;
                    case "Thumbnail": o.Art.TryAdd("cover", url); break;
                }
            }
            var custom = meta.Obj("customAttributes");
            var third = custom.Obj("ThirdPartyManagedApp").Str("value") ?? custom.Obj("ThirdPartyManagedProvider").Str("value");
            if (!string.IsNullOrEmpty(third)) o.Note = $"Installs through {third}";
            cache.Games.Add(o);
        }
        cache.Save(CachePath);
        Owned = cache;
    }

    /// <summary>Games installed with legendary (by Nimbus), from legendary's own records.</summary>
    public List<Game> Installed()
    {
        var games = new List<Game>();
        var file = Path.Combine(ConfigDir, "installed.json");
        if (!File.Exists(file)) return games;
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(file));
            foreach (var p in doc.RootElement.EnumerateObject())
            {
                var e = p.Value;
                var path = e.Str("install_path"); var title = e.Str("title");
                if (path is null || title is null || !Directory.Exists(path)) continue;
                if (e.True("is_dlc")) continue;
                long size = e.TryGetProperty("install_size", out var s) && s.ValueKind == JsonValueKind.Number && s.TryGetInt64(out var n) ? n : 0;
                var g = new Game
                {
                    Id = "epic:" + p.Name,
                    Title = title,
                    Source = "epic",
                    InstallPath = path,
                    SizeBytes = size > 0 ? size : null,
                    Added = Time.Ms(Directory.GetCreationTimeUtc(path)),
                    LaunchVia = "legendary",
                    CanUninstall = true,
                };
                g.ProcessDirs.Add(path);
                if (Owned.Games.FirstOrDefault(o => o.Id == p.Name) is { } owned)
                    foreach (var (k, v) in owned.Art) g.Art[k] = v;
                games.Add(g);
            }
        }
        catch (Exception ex) { SystemBridge.Log("epic installed.json: " + ex.Message); }
        return games;
    }

    /// <summary>Installs a game into &lt;root&gt;\Epic. Runs inside the install queue.</summary>
    public async Task InstallAsync(InstallJob job, string appName, string root, CancellationToken ct)
    {
        var basePath = Path.Combine(root, "Epic");
        Directory.CreateDirectory(basePath);
        var r = await HelperTools.RunAsync(HelperTools.Legendary,
            new[] { "-y", "install", appName, "--base-path", basePath, "--skip-sdl", "--skip-dlcs" }, Env,
            onLine: line => job.ReadLine(line, bytesInLine: false), ct: ct);
        if (!Installed().Any(g => g.Id == "epic:" + appName))
            throw new InvalidOperationException("Epic install failed: " + r.Problem());
        job.State = "installing";
        await RunPrerequisitesAsync(appName);
    }

    /// <summary>Some games list a one-time installer (Visual C++ runtime etc.) that the Epic launcher would run.</summary>
    private static async Task RunPrerequisitesAsync(string appName)
    {
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(Path.Combine(ConfigDir, "installed.json")));
            if (!doc.RootElement.TryGetProperty(appName, out var e)) return;
            var pre = e.Obj("prereq_info");
            var rel = pre.Str("path"); var dir = e.Str("install_path");
            if (rel is null || dir is null || pre.True("installed")) return;
            var exe = Path.Combine(dir, rel);
            if (!File.Exists(exe)) return;
            SystemBridge.Log($"epic {appName}: running prerequisites {rel}");
            using var p = Process.Start(new ProcessStartInfo(exe, pre.Str("args") ?? "") { UseShellExecute = true, WorkingDirectory = Path.GetDirectoryName(exe)! });
            if (p is not null) await p.WaitForExitAsync().WaitAsync(TimeSpan.FromMinutes(10));
        }
        catch (Exception ex) { SystemBridge.Log("epic prerequisites: " + ex.Message); }
    }

    public async Task UninstallAsync(string appName)
    {
        var r = await HelperTools.RunAsync(HelperTools.Legendary, new[] { "-y", "uninstall", appName }, Env, timeout: TimeSpan.FromMinutes(10));
        if (Installed().Any(g => g.Id == "epic:" + appName)) throw new InvalidOperationException("Couldn't uninstall: " + r.Problem());
    }

    /// <summary>Epic cloud saves for a game Ember installed: "down" before playing, "up" after (like hearth-core's
    /// epic::sync_saves). On Windows legendary finds the save folder itself from the game's metadata; games without
    /// cloud saves are skipped by legendary. Returns whether legendary said it worked.</summary>
    public async Task<bool> SyncSavesAsync(string appName, string direction)
    {
        if (!SignedIn) return false;
        var flag = direction == "down" ? "--skip-upload" : "--skip-download";
        var r = await HelperTools.RunAsync(HelperTools.Legendary, new[] { "-y", "sync-saves", appName, flag }, Env, timeout: TimeSpan.FromMinutes(2));
        SystemBridge.Log($"epic saves: {appName} {direction} {(r.ExitCode == 0 ? "ok" : r.Problem())}");
        return r.ExitCode == 0;
    }

    /// <summary>Asks legendary how to start the game (with a fresh Epic login token) and starts it directly, so Nimbus can watch it.</summary>
    public async Task LaunchAsync(string appName)
    {
        var r = await HelperTools.RunAsync(HelperTools.Legendary, new[] { "launch", appName, "--json", "--skip-version-check" }, Env, timeout: TimeSpan.FromMinutes(1));
        int start = r.Out.IndexOf('{');
        if (start < 0)
        {
            // Offline or Epic is down: games that allow it can start without the online token.
            r = await HelperTools.RunAsync(HelperTools.Legendary, new[] { "launch", appName, "--json", "--offline", "--skip-version-check" }, Env, timeout: TimeSpan.FromMinutes(1));
            start = r.Out.IndexOf('{');
            if (start < 0) throw new InvalidOperationException("Couldn't start the game: " + r.Problem());
        }
        using var doc = JsonDocument.Parse(r.Out[start..]);
        var p = doc.RootElement;
        var dir = p.Str("game_directory") ?? "";
        var exe = Path.Combine(dir, p.Str("game_executable") ?? "");
        if (!File.Exists(exe)) throw new InvalidOperationException($"The game's program is missing ({exe}). Try reinstalling it.");
        var psi = new ProcessStartInfo(exe) { UseShellExecute = false, WorkingDirectory = p.Str("working_directory") is { Length: > 0 } wd ? wd : dir };
        foreach (var key in new[] { "game_parameters", "user_parameters", "egl_parameters" })
            foreach (var a in p.Arr(key)) if (a.GetString() is { } s) psi.ArgumentList.Add(s);
        var env = p.Obj("environment");
        if (env.ValueKind == JsonValueKind.Object)
            foreach (var e in env.EnumerateObject()) psi.Environment[e.Name] = e.Value.GetString() ?? "";
        try { Process.Start(psi)?.Dispose(); }
        catch (System.ComponentModel.Win32Exception ex) when (ex.NativeErrorCode == 740)
        {
            // The game asks for administrator rights: start it through the shell so Windows can ask you.
            var args = string.Join(' ', psi.ArgumentList.Select(a => a.Contains(' ') || a.Length == 0 ? $"\"{a}\"" : a));
            Process.Start(new ProcessStartInfo(exe, args) { UseShellExecute = true, WorkingDirectory = psi.WorkingDirectory })?.Dispose();
        }
    }
}
