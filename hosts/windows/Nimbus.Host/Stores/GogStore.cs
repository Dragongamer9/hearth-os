using System.Net.Http.Headers;
using System.Text.Json;
using System.Text.Json.Nodes;
using Nimbus.Host.Library;

namespace Nimbus.Host.Stores;

/// <summary>
/// GOG without GOG Galaxy. Sign-in and the game list use GOG's web API directly; downloads use gogdl
/// (from the Heroic launcher). The sign-in is kept in %LOCALAPPDATA%\Nimbus\gog\auth.json, which gogdl also reads.
/// </summary>
public sealed class GogStore
{
    private const string ClientId = "46899977096215655";
    private const string ClientSecret = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
    private const string Redirect = "https://embed.gog.com/on_login_success?origin=client";
    public static readonly string LoginUrl =
        $"https://auth.gog.com/auth?client_id={ClientId}&redirect_uri={Uri.EscapeDataString(Redirect)}&response_type=code&layout=client2";

    private static string Dir => Path.Combine(HelperTools.DataDir, "gog");
    private static string AuthPath => Path.Combine(Dir, "auth.json");
    private static string CachePath => Path.Combine(HelperTools.DataDir, "cache", "gog-owned.json");
    private static string ProfilePath => Path.Combine(Dir, "profile.json");
    private static Dictionary<string, string> Env => new() { ["GOGDL_CONFIG_PATH"] = Dir };

    private readonly SemaphoreSlim _tokenLock = new(1, 1);

    public OwnedCache Owned { get; private set; } = OwnedCache.Load(CachePath);
    public bool SignedIn => File.Exists(AuthPath);

    public (string? Name, string? Avatar) Profile()
    {
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(ProfilePath));
            return (doc.RootElement.Str("name"), doc.RootElement.Str("avatar"));
        }
        catch { return (null, null); }
    }

    public async Task<bool> SignInAsync(IWin32Window? owner)
    {
        var tool = HelperTools.EnsureAsync(HelperTools.Gogdl);
        var code = await SignInWindow.RunAsync(owner, "Sign in to GOG", LoginUrl, onStarting: url =>
            url.StartsWith("https://embed.gog.com/on_login_success", StringComparison.OrdinalIgnoreCase) ? SignInWindow.Query(url, "code") ?? "" : null);
        if (string.IsNullOrEmpty(code)) return false;

        var url = $"https://auth.gog.com/token?client_id={ClientId}&client_secret={ClientSecret}&grant_type=authorization_code" +
                  $"&redirect_uri={Uri.EscapeDataString(Redirect)}&code={Uri.EscapeDataString(code)}";
        await SaveTokenAsync(await HelperTools.Http.GetStringAsync(url));
        await LoadProfileAsync();
        await tool;
        return true;
    }

    public void SignOut()
    {
        try { File.Delete(AuthPath); File.Delete(ProfilePath); File.Delete(CachePath); } catch { }
        Owned = new OwnedCache();
    }

    // gogdl's format: { "<client id>": { access_token, refresh_token, expires_in, user_id, loginTime, ... } }
    private static async Task SaveTokenAsync(string json)
    {
        var token = JsonNode.Parse(json)!.AsObject();
        if (token["access_token"] is null) throw new InvalidOperationException("GOG didn't accept the sign-in. Try again.");
        token["loginTime"] = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() / 1000.0;
        var root = new JsonObject { [ClientId] = token };
        Directory.CreateDirectory(Dir);
        await File.WriteAllTextAsync(AuthPath + ".tmp", root.ToJsonString());
        File.Move(AuthPath + ".tmp", AuthPath, overwrite: true);
    }

    /// <summary>A valid access token, refreshed if it has expired.</summary>
    private async Task<string> TokenAsync()
    {
        await _tokenLock.WaitAsync();
        try
        {
            if (!File.Exists(AuthPath)) throw new InvalidOperationException("Sign in to GOG first.");
            var t = JsonNode.Parse(await File.ReadAllTextAsync(AuthPath))![ClientId]!.AsObject();
            double login = t["loginTime"]?.GetValue<double>() ?? 0, expires = t["expires_in"]?.GetValue<double>() ?? 0;
            if (DateTimeOffset.UtcNow.ToUnixTimeSeconds() < login + expires - 120) return t["access_token"]!.GetValue<string>();

            var url = $"https://auth.gog.com/token?client_id={ClientId}&client_secret={ClientSecret}&grant_type=refresh_token" +
                      $"&refresh_token={Uri.EscapeDataString(t["refresh_token"]!.GetValue<string>())}";
            using var resp = await HelperTools.Http.GetAsync(url);
            if (!resp.IsSuccessStatusCode) throw new InvalidOperationException("Your GOG sign-in expired. Sign in again under Library › Stores.");
            var body = await resp.Content.ReadAsStringAsync();
            await SaveTokenAsync(body);
            return JsonNode.Parse(body)!["access_token"]!.GetValue<string>();
        }
        finally { _tokenLock.Release(); }
    }

    private async Task<JsonDocument> GetAsync(string url)
    {
        using var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", await TokenAsync());
        using var resp = await HelperTools.Http.SendAsync(req);
        resp.EnsureSuccessStatusCode();
        return JsonDocument.Parse(await resp.Content.ReadAsStreamAsync());
    }

    private async Task LoadProfileAsync()
    {
        try
        {
            using var doc = await GetAsync("https://embed.gog.com/userData.json");
            var u = doc.RootElement;
            string? avatar = null;
            var av = u.Obj("avatars");
            if (av.ValueKind == JsonValueKind.Object)
                avatar = av.Str("menu_user_av_big2") ?? av.Str("menu_user_av_big") ?? av.EnumerateObject().Select(p => p.Value.GetString()).LastOrDefault();
            await File.WriteAllTextAsync(ProfilePath, JsonSerializer.Serialize(new { name = u.Str("username"), avatar = avatar is null ? null : J.Https(avatar) }));
        }
        catch (Exception ex) { SystemBridge.Log("gog profile: " + ex.Message); }
    }

    /// <summary>Downloads the list of Windows games on your GOG account, with cover art.</summary>
    public async Task RefreshOwnedAsync()
    {
        if (Profile().Name is null) await LoadProfileAsync();
        var list = new List<OwnedGame>();
        for (int page = 1, pages = 1; page <= pages && page <= 100; page++)
        {
            using var doc = await GetAsync($"https://embed.gog.com/account/getFilteredProducts?mediaType=1&sortBy=title&page={page}");
            var r = doc.RootElement;
            pages = r.TryGetProperty("totalPages", out var tp) && tp.ValueKind == JsonValueKind.Number && tp.TryGetInt32(out var n) ? n : 1;
            foreach (var p in r.Arr("products"))
            {
                if (!p.Obj("worksOn").True("Windows")) continue;
                if (!p.TryGetProperty("id", out var idEl)) continue;
                var id = idEl.ToString();
                var o = new OwnedGame { Id = id, Title = p.Str("title") ?? id };
                if (p.Str("image") is { } img) { o.Art["wide"] = J.Https(img) + "_product_card_v2_mobile_slider_639.jpg"; o.Art["hero"] = o.Art["wide"]; }
                list.Add(o);
            }
        }

        // Better art (vertical covers, backgrounds) from GOG's game database; kept between refreshes.
        var old = Owned.Games.ToDictionary(g => g.Id);
        using var gate = new SemaphoreSlim(6);
        await Task.WhenAll(list.Select(async o =>
        {
            if (old.TryGetValue(o.Id, out var prev) && prev.Art.ContainsKey("cover")) { o.Art = prev.Art; return; }
            await gate.WaitAsync();
            try
            {
                using var resp = await HelperTools.Http.GetAsync($"https://gamesdb.gog.com/platforms/gog/external_releases/{o.Id}");
                if (!resp.IsSuccessStatusCode) return;
                using var doc = JsonDocument.Parse(await resp.Content.ReadAsStreamAsync());
                var game = doc.RootElement.Obj("game");
                string? F(string k, string ext) => game.Obj(k).Str("url_format")?.Replace("{formatter}", "").Replace("{ext}", ext);
                if (F("vertical_cover", "jpg") is { } c) o.Art["cover"] = c;
                if (F("background", "jpg") is { } b) o.Art["hero"] = b;
                if (F("logo", "jpg") is { } w) o.Art["wide"] = w;
            }
            catch { /* art is optional */ }
            finally { gate.Release(); }
        }));

        var cache = new OwnedCache { Fetched = Time.NowMs, Games = list };
        cache.Save(CachePath);
        Owned = cache;
    }

    /// <summary>GOG games Nimbus installed (records kept in library.json: id → folder).</summary>
    public List<Game> Installed(Dictionary<string, string> records)
    {
        var games = new List<Game>();
        foreach (var (id, dir) in records)
        {
            try
            {
                var info = Path.Combine(dir, $"goggame-{id}.info");
                if (!File.Exists(info)) continue;
                using var doc = JsonDocument.Parse(File.ReadAllText(info));
                var r = doc.RootElement;
                var task = r.Arr("playTasks").FirstOrDefault(t => t.True("isPrimary"));
                if (task.ValueKind != JsonValueKind.Object) task = r.Arr("playTasks").FirstOrDefault(t => t.Str("type") == "FileTask");
                var rel = task.ValueKind == JsonValueKind.Object ? task.Str("path") : null;
                if (rel is null) continue;
                var exe = Path.Combine(dir, rel);
                var work = task.Str("workingDir");
                var g = new Game
                {
                    Id = "gog:" + id,
                    Title = r.Str("name") ?? Owned.Games.FirstOrDefault(o => o.Id == id)?.Title ?? id,
                    Source = "gog",
                    InstallPath = dir,
                    Added = Time.Ms(Directory.GetCreationTimeUtc(dir)),
                    Exe = exe,
                    Args = task.Str("arguments"),
                    WorkDir = string.IsNullOrEmpty(work) ? Path.GetDirectoryName(exe) : Path.Combine(dir, work),
                    CanUninstall = true,
                };
                g.ProcessDirs.Add(dir);
                if (Owned.Games.FirstOrDefault(o => o.Id == id) is { } owned)
                    foreach (var (k, v) in owned.Art) g.Art[k] = v;
                games.Add(g);
            }
            catch (Exception ex) { SystemBridge.Log($"gog installed {id}: {ex.Message}"); }
        }
        return games;
    }

    /// <summary>Downloads a game into &lt;root&gt;\GOG\&lt;its folder&gt;. Returns the folder.</summary>
    // ------------------------------------------------------------------ cloud saves

    private static string SavesPath => Path.Combine(Dir, "saves.json");

    /// <summary>GOG's folder placeholders (&lt;?INSTALL?&gt;, %AppData%, %Documents%...) as real folders.</summary>
    public static string? Resolve(string loc, string install)
    {
        string F(Environment.SpecialFolder f) => Environment.GetFolderPath(f);
        var user = F(Environment.SpecialFolder.UserProfile);
        var map = new (string, string)[]
        {
            ("<?INSTALL?>", install), ("%INSTALL%", install),
            ("%LocalAppData%", F(Environment.SpecialFolder.LocalApplicationData)), ("%AppData%", F(Environment.SpecialFolder.ApplicationData)),
            ("%LocalAppDataLow%", Path.Combine(user, "AppData", "LocalLow")), ("%Documents%", F(Environment.SpecialFolder.MyDocuments)),
            ("%UserProfile%", user), ("%SavedGames%", Path.Combine(user, "Saved Games")), ("%Public%", Environment.GetEnvironmentVariable("PUBLIC") ?? @"C:\Users\Public"),
        };
        var o = loc.Replace('/', '\\');
        foreach (var (k, v) in map)
        {
            int i = o.IndexOf(k, StringComparison.OrdinalIgnoreCase);
            if (i >= 0) o = o[..i] + v + o[(i + k.Length)..];
        }
        return o.Contains('%') || o.Contains("<?") ? null : o;
    }

    /// <summary>Where the game keeps its saves, from GOG's own settings for it (as GOG Galaxy uses).</summary>
    public static async Task<List<(string Name, string Path)>> SaveLocationsAsync(string id, string install)
    {
        var info = JsonNode.Parse(await File.ReadAllTextAsync(Path.Combine(install, $"goggame-{id}.info")));
        var client = (string?)info?["clientId"] ?? throw new InvalidOperationException("no GOG client id for this game");
        var cfg = JsonNode.Parse(await HelperTools.Http.GetStringAsync($"https://remote-config.gog.com/components/galaxy_client/clients/{client}?component_version=2.0.45"));
        var cloud = cfg?["content"]?["Windows"]?["cloudStorage"];
        var list = new List<(string, string)>();
        if (cloud?["enabled"]?.GetValue<bool>() != true) return list;
        var locs = cloud["locations"] as JsonArray ?? new JsonArray();
        if (locs.Count == 0 && Resolve($"%LocalAppData%/GOG.com/Galaxy/Applications/{client}/Storage/Shared/Files", install) is { } d) list.Add(("__default", d));
        foreach (var l in locs)
            if ((string?)l?["name"] is { } n && (string?)l?["location"] is { } loc && Resolve(loc, install) is { } p) list.Add((n, p));
        return list;
    }

    /// <summary>GOG cloud saves: "down" before playing, "up" after. Returns how many places were synced.</summary>
    public async Task<int> SyncSavesAsync(string id, string install, string direction)
    {
        if (!SignedIn) return 0;
        await TokenAsync();
        var locs = await SaveLocationsAsync(id, install);
        if (locs.Count == 0) return 0;
        JsonObject stamps;
        try { stamps = JsonNode.Parse(File.ReadAllText(SavesPath)) as JsonObject ?? new(); } catch { stamps = new(); }
        int n = 0;
        foreach (var (name, path) in locs)
        {
            Directory.CreateDirectory(path);
            var ts = (stamps[id]?[name]?.GetValue<double>() ?? 0).ToString(System.Globalization.CultureInfo.InvariantCulture);
            var r = await HelperTools.RunAsync(HelperTools.Gogdl, new[] { "--auth-config-path", AuthPath, "save-sync", path, id, "--os", "windows", "--ts", ts, "--name", name,
                direction == "down" ? "--skip-upload" : "--skip-download" }, Env, timeout: TimeSpan.FromMinutes(3));
            if (r.ExitCode != 0) { Nimbus.Host.SystemBridge.Log($"gog saves: {id} {name} {direction} failed"); continue; }
            var last = r.Out.Split('\n').Select(x => x.Trim()).LastOrDefault(x => double.TryParse(x, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out _));
            if (last is not null)
            {
                if (stamps[id] is not JsonObject) stamps[id] = new JsonObject();
                stamps[id]![name] = double.Parse(last, System.Globalization.CultureInfo.InvariantCulture);
            }
            n++;
        }
        File.WriteAllText(SavesPath, stamps.ToJsonString());
        Nimbus.Host.SystemBridge.Log($"gog saves: {id} {direction}: {n} place(s)");
        return n;
    }

    public async Task<string> InstallAsync(InstallJob job, string id, string root, CancellationToken ct)
    {
        await TokenAsync();   // refresh now so gogdl finds a valid token
        var basePath = Path.Combine(root, "GOG");
        Directory.CreateDirectory(basePath);
        var r = await HelperTools.RunAsync(HelperTools.Gogdl,
            new[] { "--auth-config-path", AuthPath, "download", id, "--platform", "windows", "--path", basePath, "--skip-dlcs", "--lang", "en-US" },
            Env, onLine: line => job.ReadLine(line, bytesInLine: true), ct: ct);
        var dir = Directory.EnumerateDirectories(basePath).FirstOrDefault(d => File.Exists(Path.Combine(d, $"goggame-{id}.info")));
        if (dir is null) throw new InvalidOperationException("GOG install failed: " + r.Problem());
        return dir;
    }
}
