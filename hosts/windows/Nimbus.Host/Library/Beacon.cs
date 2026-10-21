using System.Diagnostics;
using System.Text;
using System.Text.Json;

namespace Nimbus.Host.Library;

/// <summary>
/// Beacon on Windows: the launcher for The Battle for Middle-earth games, Age of the Ring, LOTR: Conquest and the
/// community versions of Universe at War, Armies of Exigo, Rise of Legends and Galactic Battlegrounds. Same contract as
/// Hearth's hosts/hearth-core/src/beacon.rs (docs/BEACON.md): Ember never reads Beacon's files or starts the games
/// itself; it runs Beacon's own command-line tool with an argument list (never a shell) and reads what it prints.
///
///   beacon.exe status --json                         every game Beacon knows, with its state
///   beacon.exe mods &lt;game&gt;                           a game's mods: "* &lt;id&gt; &lt;mod|launcher&gt; &lt;path&gt;", * = chosen
///   beacon.exe select-mod &lt;game&gt; &lt;mod id|none&gt;      the mod a game starts with (Beacon keeps it)
///   beacon.exe launch &lt;game&gt; [--mod=&lt;id&gt;|--vanilla] --wait
///   beacon-app.exe [--fullscreen]                    Beacon's window
///
/// Where they're looked for: an absolute folder on PATH, then %LOCALAPPDATA%\Beacon (Beacon's installer),
/// %LOCALAPPDATA%\Programs\Beacon (and its bin folder),
/// then %ProgramFiles%\Beacon. Windows file names ignore case, so the window can't also be called beacon.exe next to
/// the tool: the Windows Beacon is expected to ship the window as beacon-app.exe (or the tool in bin\). Nothing is
/// listed when beacon.exe isn't found.
/// </summary>
public static class Beacon
{
    public static readonly string[] Games = { "bfme1", "bfme2", "rotwk", "aotr", "conquest", "uaw", "exigo", "rol", "swgb" };
    private static readonly Dictionary<string, int> SteamIds = new() { ["uaw"] = 10430, ["swgb"] = 356500 };
    private const int MaxMods = 200;
    private const string Cdn = "https://cdn.cloudflare.steamstatic.com/steam/apps/";

    public static bool KnownGame(string id) => Games.Contains(id);

    /// <summary>A mod id as Beacon makes them, safe on a command line: letters, digits and . _ : - only.</summary>
    public static bool ValidModId(string s) =>
        s.Length is > 0 and <= 128 && s[0] is not ('-' or '.' or ':') && s.All(c => c < 128 && (char.IsLetterOrDigit(c) || c is '.' or '_' or ':' or '-'));

    /// <summary>Games that start with Hearth Input's "Strategy (RTS)" layout: all of Beacon's but LOTR: Conquest.</summary>
    public static bool RtsByDefault(string libraryId) =>
        libraryId.StartsWith("beacon:") && KnownGame(libraryId[7..]) && libraryId[7..] != "conquest";

    // ------------------------------------------------------------------ finding Beacon

    private static IEnumerable<string> Folders()
    {
        foreach (var d in (Environment.GetEnvironmentVariable("PATH") ?? "").Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
            if (Path.IsPathFullyQualified(d)) yield return d;
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        // Beacon's own Windows installer (NSIS, per user) puts it in %LOCALAPPDATA%\Beacon; its MSI in Program Files.
        yield return Path.Combine(local, "Beacon");
        yield return Path.Combine(local, "Programs", "Beacon");
        yield return Path.Combine(local, "Programs", "Beacon", "bin");
        yield return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), "Beacon");
    }

    private static string? Find(string exe)
    {
        foreach (var d in Folders())
        {
            try { var p = Path.Combine(d, exe); if (File.Exists(p)) return p; } catch { }
        }
        return null;
    }

    public static string? Cli() => Find("beacon.exe");
    public static string? App() => Find("beacon-app.exe");

    // ------------------------------------------------------------------ running it

    private static string Run(string exe, IEnumerable<string> args, TimeSpan limit)
    {
        var psi = new ProcessStartInfo(exe)
        {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8, StandardErrorEncoding = Encoding.UTF8,
        };
        foreach (var a in args) psi.ArgumentList.Add(a);
        using var p = Process.Start(psi) ?? throw new InvalidOperationException("Couldn't start Beacon");
        var outTask = p.StandardOutput.ReadToEndAsync();
        var errTask = p.StandardError.ReadToEndAsync();
        if (!p.WaitForExit((int)limit.TotalMilliseconds))
        {
            try { p.Kill(entireProcessTree: true); } catch { }
            throw new TimeoutException("Beacon took too long to answer");
        }
        var o = outTask.GetAwaiter().GetResult();
        var e = errTask.GetAwaiter().GetResult();
        if (p.ExitCode != 0) throw new InvalidOperationException((e.Trim().Split('\n').LastOrDefault() ?? "").Trim() is { Length: > 0 } msg ? msg : $"Beacon exited with {p.ExitCode}");
        return o;
    }

    // ------------------------------------------------------------------ what Beacon says

    public sealed record Mod(string Id, string Name, string Kind, string Path);
    public sealed record Status(string Id, string Family, string Title, string Short, string? Path, bool Installed, bool Ready,
                                List<(string Level, string Message)> Issues, string? SelectedMod, string? Edition, int? SteamAppId, bool ViaSteam);

    private static string? Text(JsonElement v, int max)
    {
        if (v.ValueKind != JsonValueKind.String) return null;
        var s = new string((v.GetString() ?? "").Where(c => !char.IsControl(c)).Take(max).ToArray()).Trim();
        return s.Length == 0 ? null : s;
    }
    private static string DefaultFamily(string id) => id switch { "conquest" => "lotr", "uaw" or "exigo" or "rol" or "swgb" => "other", _ => "bfme" };
    private static JsonElement Get(JsonElement o, string k) => o.ValueKind == JsonValueKind.Object && o.TryGetProperty(k, out var v) ? v : default;

    /// <summary>`beacon status --json`, checked: unknown games are left out, every field the right kind and length.</summary>
    public static List<Status> ParseStatus(string json)
    {
        using var doc = JsonDocument.Parse(json);
        if (doc.RootElement.ValueKind != JsonValueKind.Array) throw new InvalidOperationException("Beacon's game list isn't a list");
        var out_ = new List<Status>();
        foreach (var g in doc.RootElement.EnumerateArray().Take(64))
        {
            if (Get(g, "id").ValueKind != JsonValueKind.String) continue;
            var id = Get(g, "id").GetString() ?? "";
            if (!KnownGame(id)) { SystemBridge.Log($"beacon: left out a game Ember doesn't know: {new string(id.Take(40).ToArray())}"); continue; }
            if (out_.Any(x => x.Id == id)) continue;
            var fam = Get(g, "family").ValueKind == JsonValueKind.String && Get(g, "family").GetString() is "bfme" or "lotr" or "other" ? Get(g, "family").GetString()! : DefaultFamily(id);
            var issues = new List<(string, string)>();
            if (Get(g, "issues").ValueKind == JsonValueKind.Array)
                foreach (var i in Get(g, "issues").EnumerateArray().Take(20))
                    if (Text(Get(i, "message"), 400) is { } m) issues.Add((Get(i, "level").ValueKind == JsonValueKind.String && Get(i, "level").GetString() == "error" ? "error" : "warn", m));
            string? sel = Get(g, "selected_mod").ValueKind == JsonValueKind.String ? Get(g, "selected_mod").GetString() : null;
            if (sel is not null && !ValidModId(sel)) { SystemBridge.Log($"beacon: {id}: ignored a mod id that isn't safe to use"); sel = null; }
            string? path = Get(g, "path").ValueKind == JsonValueKind.String ? Get(g, "path").GetString() : null;
            if (path is not null && (path.Length >= 4096 || path.Contains('\0') || !Path.IsPathFullyQualified(path))) path = null;
            int? appid = Get(g, "steam_appid").ValueKind == JsonValueKind.Number && Get(g, "steam_appid").TryGetInt32(out var n) && n > 0 ? n : null;
            out_.Add(new Status(id, fam, Text(Get(g, "title"), 120) ?? id.ToUpperInvariant(), Text(Get(g, "short"), 40) ?? id.ToUpperInvariant(), path,
                Get(g, "installed").ValueKind == JsonValueKind.True, Get(g, "ready").ValueKind == JsonValueKind.True, issues, sel,
                Text(Get(g, "edition"), 80), appid, Get(g, "via_steam").ValueKind == JsonValueKind.True));
        }
        return out_;
    }

    /// <summary>"rotwk:age-of-the-ring" → "Age of the Ring".</summary>
    public static string ModName(string id)
    {
        var slug = id.Split(':').Last();
        var small = new[] { "of", "the", "and", "a", "in", "for" };
        return string.Join(" ", slug.Split(new[] { '-', '_', '.' }, StringSplitOptions.RemoveEmptyEntries)
            .Select((w, i) => i > 0 && small.Contains(w) ? w : char.ToUpperInvariant(w[0]) + w[1..]));
    }

    /// <summary>`beacon mods &lt;game&gt;`: "&lt;* or space&gt; &lt;id&gt; &lt;mod|launcher&gt; &lt;path&gt;" lines; anything else is skipped,
    /// as are unsafe ids and paths that aren't absolute. Returns the mods and the one marked chosen.</summary>
    public static (List<Mod> Mods, string? Chosen) ParseMods(string text)
    {
        var mods = new List<Mod>();
        string? chosen = null;
        foreach (var raw in text.Split('\n'))
        {
            var line = raw.TrimEnd('\r');
            if (line.Length == 0 || line[0] is not ('*' or ' ')) continue;
            var parts = line[1..].TrimStart().Split((char[]?)null, 3, StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length < 3) continue;
            var (id, kind, path) = (parts[0], parts[1], parts[2].Trim());
            if (kind is not ("mod" or "launcher") || !Path.IsPathFullyQualified(path)) continue;
            if (!ValidModId(id)) { SystemBridge.Log($"beacon: left out a mod whose id isn't safe to use: {new string(id.Take(40).ToArray())}"); continue; }
            if (mods.Any(m => m.Id == id) || mods.Count >= MaxMods) continue;
            if (line[0] == '*') chosen = id;
            mods.Add(new Mod(id, ModName(id), kind, path.Length > 4096 ? path[..4096] : path));
        }
        return (mods, chosen);
    }

    // ------------------------------------------------------------------ the library

    /// <summary>Beacon's games for the library (none when beacon.exe isn't installed).</summary>
    public static List<Game> Scan()
    {
        var list = new List<Game>();
        var cli = Cli();
        if (cli is null) return list;
        List<Status> games;
        try { games = ParseStatus(Run(cli, new[] { "status", "--json" }, TimeSpan.FromSeconds(20))); }
        catch (Exception ex) { SystemBridge.Log("beacon: status: " + ex.Message); return list; }
        foreach (var b in games)
        {
            var mods = new List<Mod>(); string? sel = b.SelectedMod;
            if (b.Installed)
            {
                try { var (m, chosen) = ParseMods(Run(cli, new[] { "mods", b.Id }, TimeSpan.FromSeconds(15))); mods = m; sel ??= chosen; }
                catch (Exception ex) { SystemBridge.Log($"beacon: mods {b.Id}: {ex.Message}"); }
            }
            var info = new
            {
                family = b.Family, @short = b.Short, ready = b.Ready, blocker = b.Issues.FirstOrDefault(i => i.Level == "error").Message,
                edition = b.Edition, viaSteam = b.ViaSteam,
                issues = b.Issues.Select(i => new { level = i.Level, message = i.Message }).ToList(),
                mod = sel, mods = mods.Select(m => new { id = m.Id, name = m.Name, kind = m.Kind }).ToList(),
            };
            var g = new Game
            {
                Id = "beacon:" + b.Id, Title = b.Title, Source = "beacon", InstallPath = b.Path, Installed = b.Installed,
                State = b.Installed ? "installed" : "notInstalled", InstallUri = b.Installed ? null : "beacon:open",
                BeaconInfo = info, BeaconGame = b.Id,
            };
            if (b.Installed && b.Path is { } p && Directory.Exists(p)) g.ProcessDirs.Add(p);
            int? app = b.SteamAppId ?? (SteamIds.TryGetValue(b.Id, out var s) ? s : null);
            if (app is { } a)
            {
                g.Art["cover"] = Cdn + a + "/library_600x900.jpg"; g.Art["hero"] = Cdn + a + "/library_hero.jpg";
                g.Art["logo"] = Cdn + a + "/logo.png"; g.Art["wide"] = Cdn + a + "/header.jpg";
            }
            list.Add(g);
        }
        SystemBridge.Log($"beacon: {list.Count} games ({list.Count(x => x.Installed)} set up)");
        return list;
    }

    /// <summary>Starts a game through `beacon launch … --wait`. mod: an id (that mod this time), "@none" or "" (no
    /// mod), null (Beacon's saved choice).</summary>
    public static void Launch(string game, string? mod)
    {
        var cli = Cli() ?? throw new InvalidOperationException("Beacon isn't installed anymore.");
        if (!KnownGame(game)) throw new InvalidOperationException("Beacon doesn't know that game");
        var psi = new ProcessStartInfo(cli) { UseShellExecute = false, CreateNoWindow = true };
        psi.ArgumentList.Add("launch"); psi.ArgumentList.Add(game);
        if (mod is "" or "@none") psi.ArgumentList.Add("--vanilla");
        else if (mod is not null) { if (!ValidModId(mod)) throw new InvalidOperationException("That mod's name can't be used"); psi.ArgumentList.Add("--mod=" + mod); }
        psi.ArgumentList.Add("--wait");
        SystemBridge.Log($"beacon: launch {game} ({mod ?? "its chosen mod"})");
        Process.Start(psi)?.Dispose();
    }

    /// <summary>Saves the mod a game starts with, in Beacon. mod null, "" or "@none": the base game.</summary>
    public static void SelectMod(string game, string? mod)
    {
        var cli = Cli() ?? throw new InvalidOperationException("Beacon isn't installed anymore.");
        if (!KnownGame(game)) throw new InvalidOperationException("Beacon doesn't know that game");
        var m = mod is null or "" or "@none" ? "none" : ValidModId(mod) ? mod : throw new InvalidOperationException("That mod's name can't be used");
        Run(cli, new[] { "select-mod", game, m }, TimeSpan.FromSeconds(15));
    }

    /// <summary>Opens Beacon's window (full screen: Ember is Game Mode).</summary>
    public static void OpenApp()
    {
        var app = App() ?? throw new InvalidOperationException("Beacon's window (beacon-app.exe) isn't installed.");
        var psi = new ProcessStartInfo(app) { UseShellExecute = false };
        psi.ArgumentList.Add("--fullscreen");
        SystemBridge.Log("beacon: opening Beacon");
        Process.Start(psi)?.Dispose();
    }

    /// <summary>For Library › Stores (the "beacon" card), like hearth-core's accounts_status.</summary>
    public static object StatusCard(IEnumerable<Game> library)
    {
        var games = library.Where(g => g.Source == "beacon").ToList();
        return new { installed = Cli() is not null, app = App() is not null, via = Cli() is null ? null : "native", games = games.Count, setUp = games.Count(g => g.Installed), error = (string?)null };
    }
}
