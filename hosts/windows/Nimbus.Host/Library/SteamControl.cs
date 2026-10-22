using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace Nimbus.Host.Library;

/// <summary>
/// Steam run by Ember, hidden, with the real Steam client doing all the work. The same design as Hearth OS's
/// hearth-core steam_ctl.rs and steam_owned.rs (docs/STEAM.md):
/// - games you own but haven't installed, from your public Steam Community games page (no key, no password);
/// - installs without Steam's dialog: Ember writes the install record Steam itself writes when you press Install
///   (appmanifest_&lt;id&gt;.acf, StateFlags 1026) into the library you chose and lets Steam read it (Steam reads its
///   libraries when it starts); if Steam doesn't take it up within two minutes, Steam's own install window instead;
/// - progress from those records and Steam's logs\content_log.txt;
/// - uninstalls without a dialog, only when Steam isn't working on the game and it isn't running, only inside the
///   game's Steam library;
/// - "Start Steam only when needed": Steam starts hidden (-silent) when something needs it and is closed properly
///   (-shutdown, never killed) when nothing has needed it for a while, never during a download, update, sync or game;
/// - signing in: Steam's own window, in front; Ember only reads Steam's files to notice when it's done. Ember never
///   signs in itself and never sees or keeps a password or token.
/// </summary>
public static class SteamControl
{
    // Steam's StateFlags bits (EAppState).
    public const long UpdateRequired = 2, FullyInstalled = 4, AppRunning = 64, UpdateRunning = 256, UpdatePaused = 512, UpdateStarted = 1024,
        Uninstalling = 2048, BackupRunning = 4096, Reconfiguring = 65536, Validating = 131072, AddingFiles = 262144, Preallocating = 524288,
        Downloading = 1048576, Staging = 2097152, Committing = 4194304, UpdateStopping = 8388608;
    /// <summary>What Steam writes for an install it has queued: update required + update started.</summary>
    public const long NewInstall = UpdateRequired | UpdateStarted;
    private const long WritingFlags = AppRunning | UpdateRunning | Uninstalling | BackupRunning | Reconfiguring | Validating | AddingFiles
        | Preallocating | Downloading | Staging | Committing | UpdateStopping;

    /// <summary>Steam is (or will be, as soon as it can) working on this app: a download, update or check that isn't paused.</summary>
    public static bool SteamWorking(long flags) => (flags & UpdatePaused) == 0 && (flags & (WritingFlags | UpdateStarted)) != 0;
    /// <summary>Steam is touching the game's files right now.</summary>
    public static bool Writing(long flags) => (flags & WritingFlags) != 0;

    public const long IdleCloseMs = 10 * 60 * 1000;
    public const long PickupMs = 120_000;
    private const long SteamId64Base = 76561197960265728;

    /// <summary>Things for the page: ("steam.window", {text}), ("steam.signedIn", {name}), ("rescan", null), ("accounts", null).</summary>
    public static event Action<string, object?>? Event;
    private static void Emit(string name, object? data = null) { try { Event?.Invoke(name, data); } catch { } }
    private static void Log(string t) => SystemBridge.Log("steam: " + t);
    private static long Now => Time.NowMs;

    // ------------------------------------------------------------------ what Ember knows about the client

    public sealed record Pending(uint AppId, string Title, string Manifest, string Text, long Written, long Kicked);

    private static readonly object Gate = new();
    private static bool _startedByEmber, _busy;
    private static long _lastUse, _upSince, _lastIdleCheck;
    private static readonly List<Pending> _pending = new();
    private static (long Started, SignState Before)? _signin;
    private static System.Threading.Timer? _timer;
    private static Func<bool> _onDemand = () => true;
    private static Func<bool> _emberSteamGame = () => false;

    public static void Touch() { lock (Gate) _lastUse = Now; }
    public static void StartedByEmber() { lock (Gate) { _startedByEmber = true; _lastUse = Now; } }
    public static uint[] PendingIds() { lock (Gate) return _pending.Select(p => p.AppId).ToArray(); }
    public static bool SigningIn { get { lock (Gate) return _signin is not null; } }

    /// <summary>Starts the background check (every 4 seconds). `onDemand`: the setting; `steamGame`: Ember runs a Steam game.</summary>
    public static void Start(Func<bool> onDemand, Func<bool> steamGame)
    {
        _onDemand = onDemand; _emberSteamGame = steamGame;
        _timer ??= new System.Threading.Timer(_ => { try { Tick(); } catch (Exception ex) { Log("tick: " + ex.Message); } }, null, 4000, 4000);
    }

    /// <summary>Starts Steam hidden if it isn't running (and remembers Ember started it).</summary>
    public static async Task EnsureRunningAsync()
    {
        Touch();
        if (SteamScanner.IsRunning() || SteamScanner.SteamPath() is null) return;
        Log("starting it in the background (-silent)");
        SteamWindows.HideMainFor(60);
        StartedByEmber();
        await SteamScanner.EnsureRunningSilentAsync();
    }

    private static string? Exe() => SteamScanner.SteamPath() is { } s && File.Exists(Path.Combine(s, "steam.exe")) ? Path.Combine(s, "steam.exe") : null;

    /// <summary>Closes Steam properly (steam -shutdown; Steam finishes its own cloud sync) and waits up to 45 s. Never kills it.</summary>
    public static async Task ShutdownAsync()
    {
        if (!SteamScanner.IsRunning()) return;
        var exe = Exe() ?? throw new InvalidOperationException("Steam isn't installed on this PC.");
        Log("closing it (-shutdown)");
        Process.Start(new ProcessStartInfo(exe, "-shutdown") { UseShellExecute = false })?.Dispose();
        for (int i = 0; i < 90 && SteamScanner.IsRunning(); i++) await Task.Delay(500);
        if (SteamScanner.IsRunning()) throw new InvalidOperationException("Steam didn't close. It may be finishing something; try again in a minute.");
        lock (Gate) { _startedByEmber = false; _upSince = 0; }
    }

    public static async Task RestartHiddenAsync()
    {
        await ShutdownAsync();
        await Task.Delay(2000);
        await EnsureRunningAsync();
    }

    /// <summary>Starts Steam hidden first if needed, then opens a steam:// address with Steam allowed in front.</summary>
    public static async Task OpenUrlAsync(string url)
    {
        await EnsureRunningAsync();
        SteamWindows.StopHiding();
        SteamWindows.AllowSteamToFocus();
        Process.Start(new ProcessStartInfo(url) { UseShellExecute = true })?.Dispose();
    }

    // ------------------------------------------------------------------ owned games (the Community games page)

    public const string PrivateNote = "Only installed games show: your Steam profile's game details are private. In Steam, set Privacy › Game details to Public to see all your games here.";
    public sealed record OwnedGame(uint AppId, string Title, int Minutes, long LastPlayed);
    private static bool _ownedBusy;
    public static bool OwnedBusy { get { lock (Gate) return _ownedBusy; } }

    /// <summary>The signed-in account's SteamID64 (the most recent user in loginusers.vdf).</summary>
    public static long? CurrentId64()
    {
        var steam = SteamScanner.SteamPath();
        if (steam is null) return null;
        var users = Vdf.Obj(Vdf.Load(Path.Combine(steam, "config", "loginusers.vdf")), "users");
        if (users is null) return null;
        foreach (var (id, v) in users)
            if (v is Dictionary<string, object> u && Vdf.Str(u, "MostRecent") == "1" && long.TryParse(id, out var n) && n > SteamId64Base) return n;
        return null;
    }

    private static string Unescape(string s) => s.Replace("&quot;", "\"").Replace("&#39;", "'").Replace("&#039;", "'").Replace("&apos;", "'")
        .Replace("&lt;", "<").Replace("&gt;", ">").Replace("&amp;", "&");

    private static string? Tag(string xml, string t)
    {
        int a = xml.IndexOf("<" + t + ">", StringComparison.Ordinal);
        if (a < 0) return null;
        a += t.Length + 2;
        int b = xml.IndexOf("</" + t + ">", a, StringComparison.Ordinal);
        if (b < 0) return null;
        var v = xml[a..b].Trim();
        if (v.StartsWith("<![CDATA[")) v = v[9..];
        if (v.EndsWith("]]>")) v = v[..^3];
        v = v.Trim();
        return v.Length == 0 ? null : v;
    }

    /// <summary>The games XML (games?tab=all&amp;xml=1). null list + private: the profile's game details are private.</summary>
    public static (List<OwnedGame>? List, bool Private) ParseXml(string xml)
    {
        if (!xml.Contains("<gamesList")) return (null, xml.Contains("<error>"));
        if (xml.Contains("<error>") && !xml.Contains("<game>")) return (null, true);
        var list = new List<OwnedGame>();
        foreach (Match m in Regex.Matches(xml, "<game>(.*?)</game>", RegexOptions.Singleline))
        {
            var block = m.Groups[1].Value;
            if (!uint.TryParse(Tag(block, "appID"), out var id)) continue;
            var title = Tag(block, "name") is { } n ? Unescape(n) : "";
            if (title.Length == 0) continue;
            double.TryParse((Tag(block, "hoursOnRecord") ?? "0").Replace(",", ""), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var hours);
            list.Add(new OwnedGame(id, title, (int)Math.Round(hours * 60), 0));
        }
        if (list.Count == 0 && !xml.Contains("<games>")) return (null, true);
        return (list, false);
    }

    /// <summary>The games page (games/?tab=all): data-profile-gameslist="{..rgGames..}" or `var rgGames = [..];`.</summary>
    public static (List<OwnedGame>? List, bool Private) ParseHtml(string html)
    {
        JsonNode? games = null;
        try
        {
            var m = Regex.Match(html, "data-profile-gameslist=\"([^\"]*)\"");
            if (m.Success) games = JsonNode.Parse(Unescape(m.Groups[1].Value))?["rgGames"];
            else
            {
                var m2 = Regex.Match(html, @"var rgGames\s*=\s*(\[.*?\]);", RegexOptions.Singleline);
                if (m2.Success) games = JsonNode.Parse(m2.Groups[1].Value);
            }
        }
        catch { games = null; }
        if (games is not JsonArray arr)
            return (null, html.Contains("profile_private_info") || html.Contains("This profile is private"));
        var list = new List<OwnedGame>();
        foreach (var g in arr)
        {
            if (g is null) continue;
            uint id;
            try { id = g["appid"]?.GetValueKind() == JsonValueKind.String ? uint.Parse((string)g["appid"]!) : (uint)g["appid"]!; } catch { continue; }
            var title = g["name"]?.ToString();
            if (string.IsNullOrEmpty(title)) continue;
            int minutes = 0; long last = 0;
            try { minutes = (int?)g["playtime_forever"] ?? 0; } catch { }
            try { last = ((long?)g["rtime_last_played"] ?? 0) * 1000; } catch { }
            list.Add(new OwnedGame(id, Unescape(title), minutes, last));
        }
        return list.Count == 0 ? (null, true) : (list, false);
    }

    private static string OwnedFile(long id64) => Path.Combine(Stores.HelperTools.DataDir, "cache", $"steam-owned-{id64}.json");

    /// <summary>The cached list for the signed-in account: {fetched, games:[{id,title,minutes,lastPlayed}], problem}.</summary>
    public static JsonObject LoadOwned()
    {
        try { if (CurrentId64() is { } id && File.Exists(OwnedFile(id)) && JsonNode.Parse(File.ReadAllText(OwnedFile(id))) is JsonObject o) return o; } catch { }
        return new JsonObject { ["fetched"] = 0L, ["games"] = new JsonArray(), ["problem"] = null };
    }

    /// <summary>A number from a JSON value, whatever it was stored as.</summary>
    public static long Num(JsonNode? n)
    {
        if (n is not JsonValue v) return 0;
        if (v.TryGetValue<long>(out var l)) return l;
        if (v.TryGetValue<int>(out var i)) return i;
        if (v.TryGetValue<double>(out var d)) return (long)d;
        if (v.TryGetValue<JsonElement>(out var e) && e.TryGetInt64(out var x)) return x;
        return 0;
    }

    /// <summary>Fetches the signed-in account's list in the background (when older than 12 hours, or `force`).</summary>
    public static void RefreshOwned(bool force)
    {
        if (CurrentId64() is not { } id64) return;
        var old = LoadOwned();
        long fetched = Num(old["fetched"]);
        if (!force && Now - fetched < 12 * 3600 * 1000) return;
        lock (Gate) { if (_ownedBusy) return; _ownedBusy = true; }
        Emit("accounts");
        _ = Task.Run(async () =>
        {
            try
            {
                (List<OwnedGame>? list, bool priv, string? problem) r = (null, false, null);
                try
                {
                    var x = await Stores.HelperTools.Http.GetStringAsync($"https://steamcommunity.com/profiles/{id64}/games?tab=all&xml=1");
                    var (l, p) = ParseXml(x); r = (l, p, null);
                }
                catch (Exception ex) { r.problem = ex.Message; }
                if (r.list is null)
                {
                    try
                    {
                        var h = await Stores.HelperTools.Http.GetStringAsync($"https://steamcommunity.com/profiles/{id64}/games/?tab=all");
                        var (l, p) = ParseHtml(h); r = (l, r.priv || p, r.problem);
                    }
                    catch (Exception ex) { r.problem ??= ex.Message; }
                }
                JsonObject entry;
                if (r.list is not null)
                {
                    Dictionary<uint, SteamCache.AppInfo> info;
                    try { info = SteamCache.ReadAppInfo(Path.Combine(SteamScanner.SteamPath() ?? "", "appcache", "appinfo.vdf")); } catch { info = new(); }
                    var arr = new JsonArray();
                    foreach (var g in r.list)
                    {
                        if (info.TryGetValue(g.AppId, out var ai) && NotAGame(ai.Type)) continue;
                        arr.Add(new JsonObject { ["id"] = g.AppId.ToString(), ["title"] = g.Title, ["minutes"] = g.Minutes, ["lastPlayed"] = g.LastPlayed });
                    }
                    entry = new JsonObject { ["fetched"] = Now, ["games"] = arr, ["problem"] = null };
                }
                else if (r.priv) entry = new JsonObject { ["fetched"] = Now, ["games"] = new JsonArray(), ["problem"] = "private" };
                else entry = new JsonObject { ["fetched"] = fetched, ["games"] = old["games"]?.DeepClone() ?? new JsonArray(), ["problem"] = r.problem ?? "No answer" };
                Directory.CreateDirectory(Path.GetDirectoryName(OwnedFile(id64))!);
                File.WriteAllText(OwnedFile(id64), entry.ToJsonString());
                Log($"owned games list: {(entry["games"] as JsonArray)?.Count ?? 0} games{(entry["problem"] is { } pr ? $" ({pr})" : "")}");
            }
            catch (Exception ex) { Log("owned games list FAILED " + ex.Message); }
            finally { lock (Gate) _ownedBusy = false; }
            Emit("accounts");
            Emit("rescan");
        });
    }

    public static bool NotAGame(string kind) => kind.ToLowerInvariant() is "tool" or "config" or "dlc" or "music" or "video" or "series" or "episode" or "hardware" or "beta";

    // ------------------------------------------------------------------ install records

    private static string Quote(string s) => s.Replace("\\", "\\\\").Replace("\"", "\\\"");

    /// <summary>A new install record in Steam's own layout, as Steam writes it when an install is queued.</summary>
    public static string ManifestText(uint appId, string name, string installDir, long owner) =>
        "\"AppState\"\n{\n" +
        $"\t\"appid\"\t\t\"{appId}\"\n\t\"Universe\"\t\t\"1\"\n\t\"name\"\t\t\"{Quote(name)}\"\n\t\"StateFlags\"\t\t\"{NewInstall}\"\n\t\"installdir\"\t\t\"{Quote(installDir)}\"\n" +
        "\t\"LastUpdated\"\t\t\"0\"\n\t\"SizeOnDisk\"\t\t\"0\"\n\t\"StagingSize\"\t\t\"0\"\n\t\"buildid\"\t\t\"0\"\n" +
        $"\t\"LastOwner\"\t\t\"{owner}\"\n\t\"UpdateResult\"\t\t\"0\"\n\t\"BytesToDownload\"\t\t\"0\"\n\t\"BytesDownloaded\"\t\t\"0\"\n\t\"BytesToStage\"\t\t\"0\"\n" +
        "\t\"BytesStaged\"\t\t\"0\"\n\t\"TargetBuildID\"\t\t\"0\"\n\t\"AutoUpdateBehavior\"\t\t\"0\"\n\t\"AllowOtherDownloadsWhileRunning\"\t\t\"0\"\n\t\"ScheduledAutoUpdate\"\t\t\"0\"\n" +
        "\t\"InstalledDepots\"\n\t{\n\t}\n\t\"UserConfig\"\n\t{\n\t}\n\t\"MountedConfig\"\n\t{\n\t}\n}\n";

    /// <summary>A folder name from a title (when appinfo.vdf doesn't say): no path or reserved characters.</summary>
    public static string FolderName(string title, uint appId)
    {
        var s = new string(title.Where(c => !"/\\:*?\"<>|".Contains(c) && !char.IsControl(c)).ToArray()).Trim().Trim('.').Trim();
        return s.Length == 0 ? "app_" + appId : s;
    }

    /// <summary>A folder name from a record that's safe to delete inside steamapps\common: one plain path part.</summary>
    public static bool SafeDirName(string? d) => !string.IsNullOrEmpty(d) && d != "." && d != ".." && d.IndexOfAny(new[] { '/', '\\', ':', '\0' }) < 0 && d.Trim() == d;

    public static (string Lib, string File, Dictionary<string, object>? Acf)? FindManifest(string steam, uint appId)
    {
        foreach (var lib in SteamScanner.Libraries(steam))
        {
            var f = Path.Combine(lib, "steamapps", $"appmanifest_{appId}.acf");
            if (File.Exists(f)) return (lib, f, Vdf.Obj(Vdf.Load(f), "AppState"));
        }
        return null;
    }

    private static long Flags(Dictionary<string, object>? acf) => long.TryParse(Vdf.Str(acf, "StateFlags"), out var f) ? f : 0;

    /// <summary>Where Steam games go: your choice (if it's still a library), else one on the drive of Ember's install
    /// folder, else Steam's own folder.</summary>
    public static string? ChooseLibrary(IReadOnlyList<string> libs, string? chosen, string? installRoot)
    {
        if (chosen is not null && libs.FirstOrDefault(l => string.Equals(l.TrimEnd('\\'), chosen.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase)) is { } c) return c;
        var drive = string.IsNullOrEmpty(installRoot) ? null : Path.GetPathRoot(installRoot);
        if (drive is not null && libs.Skip(1).FirstOrDefault(l => string.Equals(Path.GetPathRoot(l), drive, StringComparison.OrdinalIgnoreCase)) is { } d) return d;
        return libs.FirstOrDefault();
    }

    public static long? FreeBytes(string path) { try { return new DriveInfo(Path.GetPathRoot(path)!).AvailableFreeSpace; } catch { return null; } }

    /// <summary>Starts a Steam install without Steam's dialog. Returns (waiting: Steam is busy and takes it up afterwards).</summary>
    public static async Task<object> InstallAsync(uint appId, string title, string library)
    {
        var steam = SteamScanner.SteamPath() ?? throw new InvalidOperationException("Steam isn't installed on this PC.");
        if (FindManifest(steam, appId) is { } m)
        {
            if ((Flags(m.Acf) & FullyInstalled) != 0) throw new InvalidOperationException($"{title} is already installed.");
            return new { silent = true, waiting = false, already = true };
        }
        var owner = CurrentId64() ?? throw new InvalidOperationException("Sign in to Steam first (Library › Stores).");
        if (!SteamScanner.Libraries(steam).Any(l => string.Equals(l, library, StringComparison.OrdinalIgnoreCase)))
            throw new InvalidOperationException("That folder isn't a Steam library anymore. Pick another one in Library › Stores.");
        SteamCache.AppInfo? info = null;
        try { info = SteamCache.ReadAppInfo(Path.Combine(steam, "appcache", "appinfo.vdf")).GetValueOrDefault(appId); } catch { }
        var name = string.IsNullOrWhiteSpace(info?.Name) ? title : info!.Name;
        var dir = SafeDirName(info?.InstallDir) ? info!.InstallDir! : FolderName(name, appId);
        var apps = Path.Combine(library, "steamapps");
        Directory.CreateDirectory(Path.Combine(apps, "common"));
        var file = Path.Combine(apps, $"appmanifest_{appId}.acf");
        var text = ManifestText(appId, name, dir, owner);
        var tmp = file + ".tmp";
        File.WriteAllText(tmp, text);
        File.Move(tmp, file, overwrite: false);
        Log($"install {appId} ({name}): record written to {file}");
        lock (Gate) _pending.Add(new Pending(appId, name, file, text, Now, 0));
        Touch();
        return new { silent = true, waiting = await KickAsync() };
    }

    /// <summary>Gets Steam to read new records: starts it, or restarts it when idle. True: Steam is busy, records wait.</summary>
    private static async Task<bool> KickAsync()
    {
        if (SteamScanner.IsRunning() && !IdleForRestart()) return true;
        lock (Gate) { if (_busy) return false; _busy = true; }
        lock (Gate) for (int i = 0; i < _pending.Count; i++) if (_pending[i].Kicked == 0) _pending[i] = _pending[i] with { Kicked = Now };
        _ = Task.Run(async () =>
        {
            try { if (SteamScanner.IsRunning()) await RestartHiddenAsync(); else await EnsureRunningAsync(); }
            catch (Exception ex) { Log("couldn't restart for an install: " + ex.Message); }
            finally { lock (Gate) { _busy = false; for (int i = 0; i < _pending.Count; i++) _pending[i] = _pending[i] with { Kicked = Now }; } }
        });
        await Task.CompletedTask;
        return false;
    }

    /// <summary>A game Steam started is running: Steam keeps its app id in RunningAppID while it plays.</summary>
    public static bool SteamGameRunning(uint? appId = null)
    {
        var id = Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam", "RunningAppID", 0) as int? ?? 0;
        return appId is null ? id != 0 : id == (int)appId.Value;
    }

    private static bool AnyWorking(string steam, uint[] except)
    {
        foreach (var lib in SteamScanner.Libraries(steam))
        {
            var apps = Path.Combine(lib, "steamapps");
            if (!Directory.Exists(apps)) continue;
            foreach (var f in Directory.EnumerateFiles(apps, "appmanifest_*.acf"))
            {
                var n = Path.GetFileNameWithoutExtension(f)["appmanifest_".Length..];
                if (uint.TryParse(n, out var id) && except.Contains(id)) continue;
                if (SteamWorking(Flags(Vdf.Obj(Vdf.Load(f), "AppState")))) return true;
            }
        }
        return false;
    }

    private static bool RecentlyWritten(string f, long ms) => File.Exists(f) && (DateTime.UtcNow - File.GetLastWriteTimeUtc(f)).TotalMilliseconds < ms;

    private static bool IdleForRestart()
    {
        var steam = SteamScanner.SteamPath();
        if (steam is null) return true;
        return !SteamGameRunning() && !_emberSteamGame() && !AnyWorking(steam, PendingIds()) && !RecentlyWritten(Path.Combine(steam, "logs", "cloud_log.txt"), 120_000);
    }

    public enum Pickup { Waiting, Taken, Gone, TimedOut }

    public static Pickup PickupOf(string written, string? textNow, bool steamUp, long kicked, long now)
    {
        if (textNow is null) return kicked > 0 && steamUp ? Pickup.Gone : Pickup.Waiting;
        if (textNow != written) return Pickup.Taken;
        return kicked > 0 && steamUp && now - kicked > PickupMs ? Pickup.TimedOut : Pickup.Waiting;
    }

    /// <summary>Removes a record Ember wrote that Steam hasn't taken up. False when Steam has it already.</summary>
    public static bool CancelPending(uint appId)
    {
        Pending? p;
        lock (Gate) { p = _pending.FirstOrDefault(x => x.AppId == appId); if (p is null) return false; _pending.Remove(p); }
        try { if (File.ReadAllText(p.Manifest) == p.Text) { File.Delete(p.Manifest); Log($"install {appId}: cancelled before Steam took it up"); return true; } } catch { }
        return false;
    }

    // ------------------------------------------------------------------ content_log.txt

    public sealed class ContentLog
    {
        public long? Rate;
        public Dictionary<uint, (string Stage, string? Reason)> Apps = new();
    }

    public static ContentLog ParseContentLog(string text)
    {
        var o = new ContentLog();
        foreach (var raw in text.Split('\n'))
        {
            var line = raw.Trim();
            int b = line.IndexOf("] ", StringComparison.Ordinal);
            var body = b >= 0 ? line[(b + 2)..].Trim() : line;
            if (body.StartsWith("Current download rate:"))
            {
                var parts = body["Current download rate:".Length..].Split(' ', StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length > 0 && double.TryParse(parts[0], System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var n))
                {
                    double mult = (parts.Length > 1 ? parts[1] : "Mbps") switch { "Kbps" => 1e3, "Gbps" => 1e9, _ => 1e6 };
                    o.Rate = (long)Math.Round(n * mult / 8);
                }
                continue;
            }
            if (!body.StartsWith("AppID ")) continue;
            var rest = body[6..];
            int sp = rest.IndexOf(' ');
            if (sp < 0 || !uint.TryParse(rest[..sp], out var id)) continue;
            var what = rest[(sp + 1)..];
            int colon = what.IndexOf(" : ", StringComparison.Ordinal);
            if (colon < 0) continue;
            var kind = what[..colon].Trim();
            var val = what[(colon + 3)..];
            string? reason = null;
            int open = val.LastIndexOf('(');
            if (open >= 0 && val.EndsWith(")")) reason = val[(open + 1)..^1].Trim();
            var words = (open >= 0 ? val[..open] : val).Split(',').Select(w => w.Trim()).Where(w => w.Length > 0).ToList();
            var e = o.Apps.TryGetValue(id, out var cur) ? cur : ("", null);
            if (kind == "update changed") e.Stage = StageOf(words);
            else if (kind == "state changed")
            {
                e.Reason = reason is { Length: > 0 } r && !r.StartsWith("Update delayed", StringComparison.OrdinalIgnoreCase) && !r.Contains("scheduled", StringComparison.OrdinalIgnoreCase) ? r : null;
                if (words.Contains("Fully Installed") && !words.Any(w => w.StartsWith("Update"))) e.Stage = "";
            }
            else continue;
            o.Apps[id] = e;
        }
        return o;
    }

    private static string StageOf(List<string> w)
    {
        bool Has(string x) => w.Any(y => y.Equals(x, StringComparison.OrdinalIgnoreCase));
        if (Has("None")) return "";
        if (Has("Validating") || Has("Verifying")) return "Verifying";
        if (Has("Committing") || Has("Staging")) return "Installing";
        if (Has("Preallocating")) return "Making room";
        if (Has("Downloading")) return "Downloading";
        if (Has("Reconfiguring")) return "Getting ready";
        if (Has("Stopping")) return "Stopping";
        return "";
    }

    /// <summary>The end of content_log.txt (64 KB), and whether it was written in the last 30 seconds.</summary>
    public static (ContentLog Log, bool Fresh) ReadContentLog()
    {
        var steam = SteamScanner.SteamPath();
        if (steam is null) return (new ContentLog(), false);
        var f = Path.Combine(steam, "logs", "content_log.txt");
        try
        {
            using var fs = new FileStream(f, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
            fs.Seek(Math.Max(0, fs.Length - 65536), SeekOrigin.Begin);
            using var sr = new StreamReader(fs);
            return (ParseContentLog(sr.ReadToEnd()), RecentlyWritten(f, 30_000));
        }
        catch { return (new ContentLog(), false); }
    }

    // ------------------------------------------------------------------ uninstalling

    /// <summary>Removes a Steam game without Steam's dialog (same rules as Hearth OS). Returns whether Steam was running
    /// (and was closed for it).</summary>
    public static async Task<bool> UninstallAsync(uint appId, string title)
    {
        var steam = SteamScanner.SteamPath() ?? throw new InvalidOperationException("Steam isn't installed on this PC.");
        var m = FindManifest(steam, appId) ?? throw new InvalidOperationException($"Steam doesn't have {title} installed anymore.");
        if (Writing(Flags(m.Acf))) throw new InvalidOperationException($"Steam is working on {title} (downloading, updating or checking it). Try again when it's done.");
        if (SteamGameRunning(appId)) throw new InvalidOperationException($"Exit {title} first.");
        bool wasRunning = SteamScanner.IsRunning();
        if (wasRunning)
        {
            if (SteamGameRunning()) throw new InvalidOperationException("Exit the Steam game that's running first: Steam has to close to delete a game.");
            if (AnyWorking(steam, new[] { appId })) throw new InvalidOperationException("Steam is downloading or updating another game. Try again when it's done.");
            lock (Gate) { if (_busy) throw new InvalidOperationException("Steam is starting or closing. Try again in a moment."); _busy = true; }
            try { await ShutdownAsync(); } finally { lock (Gate) _busy = false; }
        }
        // Read again: Steam writes its records as it closes.
        m = FindManifest(steam, appId) ?? throw new InvalidOperationException($"Steam doesn't have {title} installed anymore.");
        if (Writing(Flags(m.Acf))) throw new InvalidOperationException($"Steam is working on {title}. Try again when it's done.");
        RemoveFiles(m.Lib, m.File, appId, Vdf.Str(m.Acf, "installdir"));
        Log($"uninstalled {appId} ({title}) from {m.Lib}");
        return wasRunning;
    }

    private static bool IsLink(FileSystemInfo i) => i.Attributes.HasFlag(FileAttributes.ReparsePoint) || i.LinkTarget is not null;

    /// <summary>Deletes a game's files inside one Steam library (its folder, shader cache, unfinished download, Workshop
    /// items, then its record, set aside first). A link is removed, never followed.</summary>
    public static void RemoveFiles(string lib, string manifest, uint appId, string? installDir)
    {
        var apps = Path.GetFullPath(Path.Combine(lib, "steamapps"));
        if (!string.Equals(Path.GetDirectoryName(Path.GetFullPath(manifest)), apps, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("That record isn't in a Steam library.");
        var aside = manifest + ".hearth-removing";
        File.Move(manifest, aside, overwrite: true);
        var targets = new List<string>
        {
            Path.Combine(apps, "shadercache", appId.ToString()), Path.Combine(apps, "downloading", appId.ToString()), Path.Combine(apps, "temp", appId.ToString()),
            Path.Combine(apps, "workshop", "content", appId.ToString()), Path.Combine(apps, "workshop", $"appworkshop_{appId}.acf"),
        };
        if (SafeDirName(installDir)) targets.Insert(0, Path.Combine(apps, "common", installDir!));
        foreach (var t in targets)
        {
            var full = Path.GetFullPath(t);
            // Never outside the library.
            if (!full.StartsWith(apps + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase)) { Log($"left {t} alone: not inside the library"); continue; }
            try
            {
                if (File.Exists(full)) { File.Delete(full); continue; }
                var di = new DirectoryInfo(full);
                if (!di.Exists) continue;
                if (IsLink(di)) di.Delete(false);   // the link only
                else di.Delete(true);               // .NET doesn't follow links inside while deleting
            }
            catch (Exception ex)
            {
                try { File.Move(aside, manifest, overwrite: false); } catch { }
                throw new InvalidOperationException($"Couldn't delete {full} ({ex.Message}).");
            }
        }
        try { File.Delete(aside); } catch { }
    }

    // ------------------------------------------------------------------ closing Steam when nothing needs it

    public sealed record IdleCheck(bool OnDemand, bool Running, bool StartedByEmber, bool Busy, bool GameRunning, bool Working,
                                   bool ContentRecent, bool CloudRecent, bool Pending, long SinceUse, long SinceUp);

    public static bool ShouldClose(IdleCheck i) =>
        i.OnDemand && i.Running && i.StartedByEmber && !i.Busy && !i.GameRunning && !i.Working && !i.ContentRecent && !i.CloudRecent && !i.Pending
        && i.SinceUse >= IdleCloseMs && i.SinceUp >= IdleCloseMs;

    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

    private static bool SteamInFront()
    {
        try
        {
            GetWindowThreadProcessId(GetForegroundWindow(), out var pid);
            if (pid == 0) return false;
            using var p = Process.GetProcessById((int)pid);
            return p.ProcessName.Equals("steam", StringComparison.OrdinalIgnoreCase) || p.ProcessName.Equals("steamwebhelper", StringComparison.OrdinalIgnoreCase);
        }
        catch { return false; }
    }

    private static void Tick()
    {
        long t = Now;
        bool running = SteamScanner.IsRunning();
        lock (Gate)
        {
            if (running && _upSince == 0) _upSince = t;
            if (!running) { _upSince = 0; if (!_busy && t - _lastUse > 60_000) _startedByEmber = false; }
        }
        // Install records Steam should take up.
        Pending[] list; bool busy;
        lock (Gate) { list = _pending.ToArray(); busy = _busy; }
        bool needKick = false;
        foreach (var p in list)
        {
            string? now = null;
            try { if (File.Exists(p.Manifest)) now = File.ReadAllText(p.Manifest); } catch { now = p.Text; }
            switch (PickupOf(p.Text, now, running && !busy, p.Kicked, t))
            {
                case Pickup.Waiting: if (p.Kicked == 0) needKick = true; break;
                case Pickup.Taken:
                    Log($"install {p.AppId}: Steam took it up");
                    lock (Gate) _pending.RemoveAll(x => x.AppId == p.AppId);
                    Emit("rescan");
                    break;
                default:
                    if (now is not null) { try { File.Delete(p.Manifest); } catch { } }
                    lock (Gate) _pending.RemoveAll(x => x.AppId == p.AppId);
                    Log($"install {p.AppId}: Steam {(now is null ? "removed" : "didn't take up")} the record; opening Steam's own install window");
                    SteamWindows.StopHiding();
                    SteamWindows.AllowSteamToFocus();
                    try { Process.Start(new ProcessStartInfo("steam://install/" + p.AppId) { UseShellExecute = true })?.Dispose(); } catch { }
                    SteamWindows.HideMainFor(30);   // brings the Install window in front, keeps Steam's main window away
                    Emit("steam.window", new { why = "install", id = "steam:" + p.AppId, text = $"Steam didn't start {p.Title} by itself: confirm the install in Steam's window." });
                    Emit("rescan");
                    break;
            }
        }
        if (needKick && !busy) _ = KickAsync();

        // Closing an unneeded Steam (every 30 seconds).
        if (t - _lastIdleCheck < 30_000) return;
        _lastIdleCheck = t;
        if (!running) return;
        if (SteamInFront()) Touch();
        if (!_onDemand()) return;
        var steam = SteamScanner.SteamPath();
        if (steam is null) return;
        IdleCheck i;
        lock (Gate) i = new IdleCheck(true, true, _startedByEmber, _busy, false, false, false, false, _pending.Count > 0 || _signin is not null, t - _lastUse, t - _upSince);
        if (!ShouldClose(i)) return;
        i = i with
        {
            GameRunning = SteamGameRunning() || _emberSteamGame(), Working = AnyWorking(steam, Array.Empty<uint>()),
            ContentRecent = RecentlyWritten(Path.Combine(steam, "logs", "content_log.txt"), 90_000),
            CloudRecent = RecentlyWritten(Path.Combine(steam, "logs", "cloud_log.txt"), 120_000),
        };
        if (!ShouldClose(i)) return;
        Log("nothing has needed it for a while: closing it");
        lock (Gate) { if (_busy) return; _busy = true; }
        _ = Task.Run(async () => { try { await ShutdownAsync(); } catch (Exception ex) { Log("couldn't close it: " + ex.Message); } finally { lock (Gate) _busy = false; } });
    }

    // ------------------------------------------------------------------ signing in (Steam's own window)

    public sealed record SignState(long? Recent, long UsersChanged, string? Auto);

    public static SignState ReadSignState()
    {
        var steam = SteamScanner.SteamPath();
        var users = steam is null ? null : Path.Combine(steam, "config", "loginusers.vdf");
        long changed = users is not null && File.Exists(users) ? Time.Ms(File.GetLastWriteTimeUtc(users)) : 0;
        var auto = Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam", "AutoLoginUser", null) as string;
        return new SignState(CurrentId64(), changed, string.IsNullOrEmpty(auto) ? null : auto);
    }

    public static bool SignedInSince(SignState before, SignState now) =>
        now.Recent is not null && (now.UsersChanged > before.UsersChanged || now.Recent != before.Recent || (now.Auto is not null && now.Auto != before.Auto));

    /// <summary>Waits (up to 15 minutes) for a sign-in in Steam's window; raises "steam.signedIn" when Steam's files say it's done.</summary>
    public static void WatchSignIn()
    {
        var start = ReadSignState();
        long t = Now;
        lock (Gate) _signin = (t, start);
        Touch();
        _ = Task.Run(async () =>
        {
            while (Now - t < 15 * 60 * 1000)
            {
                await Task.Delay(2000);
                lock (Gate) if (_signin?.Started != t) return;
                if (SignedInSince(start, ReadSignState()))
                {
                    lock (Gate) _signin = null;
                    Log("signed in (Steam wrote its account list)");
                    RefreshOwned(true);
                    Emit("steam.signedIn", new { name = SteamScanner.Account().Name });
                    Emit("accounts");
                    Emit("rescan");
                    return;
                }
            }
            lock (Gate) if (_signin?.Started == t) _signin = null;
        });
    }
    public static void CancelSignIn() { lock (Gate) _signin = null; }
}
