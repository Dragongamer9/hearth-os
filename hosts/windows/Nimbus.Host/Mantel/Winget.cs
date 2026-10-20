using System.Diagnostics;
using System.Text;
using System.Text.RegularExpressions;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Apps (mantel-apps.html), Mantel's app store, backed by Windows' own package manager, winget, the way
/// AppStore.qml on Hearth OS is backed by Flathub (hearth-apps): browse, search, an app's page, install, update
/// (one or everything) and remove. winget has no machine-readable output for search / list / upgrade, so its
/// tables are read by column position (the header's word starts), which works whatever language winget prints
/// in; ids winget shortened with "…" are looked up again on their own (one row is never shortened).
/// Progress lines ("12.0 MB / 50.0 MB", "45%") become progress events. One install / update / removal at a time.
/// </summary>
internal static class Winget
{
    public sealed record App(string Id, string Name, string Version, string Available, string Source, bool Installed, string? Summary = null, string? Icon = null);

    /// <summary>A Discover list of popular apps (winget has no "popular" feed), and what each category searches for (winget tags).</summary>
    private static readonly (string Id, string Name, string Summary)[] Popular =
    {
        ("Valve.Steam", "Steam", "Valve's game store and launcher"),
        ("Discord.Discord", "Discord", "Voice, video and text chat"),
        ("Mozilla.Firefox", "Firefox", "Fast, private web browser"),
        ("Google.Chrome", "Google Chrome", "Google's web browser"),
        ("VideoLAN.VLC", "VLC media player", "Plays nearly every video and music file"),
        ("OBSProject.OBSStudio", "OBS Studio", "Record and stream your screen"),
        ("Spotify.Spotify", "Spotify", "Music and podcasts"),
        ("EpicGames.EpicGamesLauncher", "Epic Games Launcher", "Epic's game store and launcher"),
        ("GOG.Galaxy", "GOG GALAXY", "GOG's game launcher"),
        ("HeroicGamesLauncher.HeroicGamesLauncher", "Heroic Games Launcher", "Epic, GOG and Amazon games in one launcher"),
        ("Playnite.Playnite", "Playnite", "All your game libraries in one place"),
        ("7zip.7zip", "7-Zip", "Opens and makes .zip, .7z and .rar archives"),
        ("Notepad++.Notepad++", "Notepad++", "Text and code editor"),
        ("Microsoft.VisualStudioCode", "Visual Studio Code", "Code editor"),
        ("GIMP.GIMP", "GIMP", "Edit photos and pictures"),
        ("KDE.Krita", "Krita", "Digital painting"),
        ("BlenderFoundation.Blender", "Blender", "3D modelling and animation"),
        ("TheDocumentFoundation.LibreOffice", "LibreOffice", "Documents, spreadsheets and presentations"),
        ("Audacity.Audacity", "Audacity", "Record and edit audio"),
        ("Valve.SteamVR", "SteamVR", "VR for Steam games"),
        ("RARLab.WinRAR", "WinRAR", "Archive manager"),
        ("Telegram.TelegramDesktop", "Telegram", "Messaging"),
        ("Bitwarden.Bitwarden", "Bitwarden", "Password manager"),
        ("qBittorrent.qBittorrent", "qBittorrent", "Download torrents"),
    };

    private static readonly Dictionary<string, string> CategoryTags = new()
    {
        ["Game"] = "game", ["AudioVideo"] = "video", ["Graphics"] = "graphics", ["Network"] = "browser", ["Office"] = "office",
        ["Development"] = "development", ["Education"] = "education", ["Science"] = "science", ["System"] = "system", ["Utility"] = "utility",
    };

    private static readonly SemaphoreSlim Work = new(1);
    private static List<App>? _installed;
    private static DateTime _installedAt;

    public static async Task<object> StatusAsync()
    {
        try
        {
            var (code, outp) = await RunAsync("--version", TimeSpan.FromSeconds(20));
            return new { winget = code == 0, version = code == 0 ? outp.Trim() : "" };
        }
        catch { return new { winget = false, version = "" }; }
    }

    // ------------------------------------------------------------------ lists

    public static async Task<object> BrowseAsync(string what)
    {
        var installed = await InstalledCachedAsync();
        if (what == "featured")
            return new { apps = Popular.Select(p => new App(p.Id, p.Name, "", "", "winget", IsInstalled(installed, p.Id), p.Summary, IconFor(p.Name))).ToList() };
        if (!CategoryTags.TryGetValue(what, out var tag)) throw new ArgumentException("Unknown category");
        var rows = await TableAsync($"search --tag {tag} --source winget --count 60");
        return new { apps = Mark(rows, installed) };
    }

    public static async Task<object> SearchAsync(string query)
    {
        query = query.Trim();
        if (query.Length == 0) return new { apps = Array.Empty<App>() };
        if (query.Length > 100 || query.Contains('"')) throw new ArgumentException("Search for a shorter name");
        var rows = await TableAsync($"search \"{query}\" --count 60");
        return new { apps = Mark(rows, await InstalledCachedAsync()) };
    }

    public static async Task<object> InstalledAsync()
    {
        var list = await InstalledFreshAsync();
        return new { apps = list.Where(a => !a.Id.StartsWith("ARP\\") || a.Source.Length > 0).OrderBy(a => a.Name, StringComparer.CurrentCultureIgnoreCase).ToList() };
    }

    public static async Task<object> UpdatesAsync()
    {
        var rows = await TableAsync("upgrade");
        return new { apps = rows.Where(r => r.Available.Length > 0).Select(r => r with { Installed = true, Icon = IconFor(r.Name) }).ToList() };
    }

    private static List<App> Mark(List<App> rows, List<App> installed) =>
        rows.Select(r => r with { Installed = IsInstalled(installed, r.Id), Icon = IconFor(r.Name) }).ToList();

    private static bool IsInstalled(List<App> installed, string id) => installed.Any(i => string.Equals(i.Id, id, StringComparison.OrdinalIgnoreCase));

    private static async Task<List<App>> InstalledCachedAsync()
    {
        if (_installed is not null && DateTime.UtcNow - _installedAt < TimeSpan.FromMinutes(5)) return _installed;
        try { return await InstalledFreshAsync(); } catch { return _installed ?? new(); }
    }

    private static async Task<List<App>> InstalledFreshAsync()
    {
        var rows = await TableAsync("list");
        _installed = rows.Select(r => r with { Installed = true, Icon = IconFor(r.Name) }).ToList();
        _installedAt = DateTime.UtcNow;
        return _installed;
    }

    /// <summary>The icon for a list row: winget's catalogue has none, so an installed app's Start menu icon (by name), else none (the page draws its letter).</summary>
    private static string? IconFor(string name)
    {
        var apps = Apps.StartAppsCached;
        if (apps is null || name.Length < 2) return null;
        var n = Norm(name);
        var a = apps.FirstOrDefault(x => Norm(x.Name) == n) ?? apps.FirstOrDefault(x => Norm(x.Name).StartsWith(n) || n.StartsWith(Norm(x.Name)) && Norm(x.Name).Length > 3);
        return a?.Icon;
    }
    private static string Norm(string s) => Regex.Replace(s.ToLowerInvariant(), "[^a-z0-9]", "");

    // ------------------------------------------------------------------ an app's page

    public static async Task<object> AppAsync(string id)
    {
        CheckId(id);
        if (id.EndsWith('…')) id = await FullIdAsync(id);
        var (code, outp) = await RunAsync($"show --id \"{id}\" --exact --accept-source-agreements --disable-interactivity", TimeSpan.FromSeconds(60));
        var installed = await InstalledCachedAsync();
        var mine = installed.FirstOrDefault(i => string.Equals(i.Id, id, StringComparison.OrdinalIgnoreCase));
        var lines = Clean(outp);
        var found = lines.Select(l => Regex.Match(l, @"^\S.*?\s(.+?)\s\[(.+)\]\s*$")).FirstOrDefault(m => m.Success);
        if (code != 0 || found is null)
        {
            if (mine is not null)   // installed outside winget's catalogue: just what `list` knows
                return new { id, name = mine.Name, summary = "", description = "", installed = true, installedVersion = mine.Version, version = mine.Version, catalogued = false, icon = IconFor(mine.Name) };
            throw new InvalidOperationException(lines.LastOrDefault(l => l.Trim().Length > 0)?.Trim() ?? "winget didn't find it");
        }
        var f = Fields(lines);
        string F(params string[] keys) { foreach (var k in keys) if (f.TryGetValue(k, out var v) && v.Length > 0) return v; return ""; }
        var desc = F("Description");
        var name = found.Groups[1].Value.Trim();
        return new
        {
            id = found.Groups[2].Value.Trim(),
            name,
            summary = desc.Split('\n')[0].Length <= 140 ? desc.Split('\n')[0] : desc[..140] + "…",
            description = desc,
            developer = F("Publisher", "Author"),
            version = F("Version"),
            homepage = F("Homepage", "Publisher Url"),
            license = F("License"),
            installed = mine is not null,
            installedVersion = mine?.Version ?? "",
            catalogued = true,
            icon = IconFor(name),
            screenshots = Array.Empty<string>(),
        };
    }

    /// <summary>"Key: value" lines of `winget show` (continuation lines indented); the first word of each key.</summary>
    private static Dictionary<string, string> Fields(List<string> lines)
    {
        var d = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        string? key = null;
        foreach (var l in lines)
        {
            var m = Regex.Match(l, @"^([A-Za-z][A-Za-z ]{1,40}):\s?(.*)$");
            if (m.Success && !l.StartsWith(' ')) { key = m.Groups[1].Value.Trim(); d[key] = m.Groups[2].Value.Trim(); }
            else if (key is not null && l.StartsWith("  ")) d[key] = (d[key].Length > 0 ? d[key] + "\n" : "") + l.Trim();
        }
        return d;
    }

    // ------------------------------------------------------------------ install / update / remove

    public static Task InstallAsync(string id, Action<double, string> progress) =>
        WorkAsync($"install --id \"{Checked(id)}\" --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity", progress);

    public static Task UpdateAsync(string id, Action<double, string> progress) => id == "--all"
        ? WorkAsync("upgrade --all --silent --accept-package-agreements --accept-source-agreements --disable-interactivity", progress)
        : WorkAsync($"upgrade --id \"{Checked(id)}\" --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity", progress);

    public static Task RemoveAsync(string id, Action<double, string> progress) =>
        WorkAsync($"uninstall --id \"{Checked(id)}\" --exact --silent --accept-source-agreements --disable-interactivity", progress);

    private static string Checked(string id) { CheckId(id); return id; }
    private static void CheckId(string id)
    {
        if (string.IsNullOrWhiteSpace(id) || id.Length > 200 || !Regex.IsMatch(id, @"^[\w.+\-\\ {}…]+$")) throw new ArgumentException("That isn't an app id");
    }

    private static async Task WorkAsync(string args, Action<double, string> progress)
    {
        if (!await Work.WaitAsync(0)) throw new InvalidOperationException("Wait for the other install or update to finish.");
        try
        {
            var lines = new List<string>();
            int code = await StreamAsync(args, line =>
            {
                lines.Add(line);
                var m = Regex.Match(line, @"([\d.,]+)\s*(KB|MB|GB)\s*/\s*([\d.,]+)\s*(KB|MB|GB)");
                if (m.Success && Size(m.Groups[3].Value, m.Groups[4].Value) is double total && total > 0)
                    progress(Math.Clamp((Size(m.Groups[1].Value, m.Groups[2].Value) ?? 0) / total, 0, 1), "Downloading");
                else if (Regex.Match(line, @"(\d{1,3})\s*%") is { Success: true } p) progress(Math.Clamp(int.Parse(p.Groups[1].Value) / 100.0, 0, 1), "");
                else if (line.Trim().Length > 3 && !Regex.IsMatch(line, @"^[\s\-\\|/█▒]+$")) progress(-1, line.Trim());
            });
            _installed = null;   // look again next time
            if (code != 0)
            {
                var why = lines.Select(l => l.Trim()).LastOrDefault(l => l.Length > 3 && !Regex.IsMatch(l, @"^[\-\\|/█▒\s\d.%KMGB/]+$"));
                throw new InvalidOperationException(why ?? $"winget stopped with code 0x{code:X8}");
            }
        }
        finally { Work.Release(); }
    }

    private static double? Size(string n, string unit)
    {
        if (!double.TryParse(n.Replace(',', '.'), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var v)) return null;
        return v * unit switch { "GB" => 1024 * 1024, "MB" => 1024, _ => 1 };
    }

    /// <summary>An id winget shortened ("Microsoft.VisualStudio.2022.Commu…"): searched for on its own, where it fits.</summary>
    private static async Task<string> FullIdAsync(string shortened)
    {
        var prefix = shortened.TrimEnd('…');
        var rows = await TableAsync($"search --id \"{prefix}\" --count 20");
        return rows.FirstOrDefault(r => r.Id.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) && !r.Id.EndsWith('…'))?.Id
            ?? throw new InvalidOperationException("winget shortened that app's id and it couldn't be found again");
    }

    // ------------------------------------------------------------------ reading winget's tables

    /// <summary>Runs a winget command that prints a table and reads it: Name, Id, Version, then (upgrade) Available,
    /// and Source when it's there.</summary>
    private static async Task<List<App>> TableAsync(string args)
    {
        var (code, outp) = await RunAsync(args + " --accept-source-agreements --disable-interactivity", TimeSpan.FromSeconds(90));
        var lines = Clean(outp);
        int dash = lines.FindIndex(l => l.Length > 10 && l.Trim().All(c => c == '-'));
        if (dash < 1)
        {
            // "No package found matching input criteria." and the like: an empty list, not an error.
            if (code == 0 || lines.Any(l => l.Contains("No ", StringComparison.OrdinalIgnoreCase))) return new();
            throw new InvalidOperationException(lines.LastOrDefault(l => l.Trim().Length > 0)?.Trim() ?? "winget didn't answer");
        }
        var header = lines[dash - 1];
        var starts = new List<int>();
        for (int i = 0; i < header.Length; i++) if (header[i] != ' ' && (i == 0 || header[i - 1] == ' ')) starts.Add(i);
        var rows = new List<App>();
        foreach (var l in lines.Skip(dash + 1))
        {
            if (l.Trim().Length == 0 || l.Length < (starts.Count > 1 ? starts[1] : 1)) continue;
            if (l.Trim().All(c => c == '-')) continue;
            string Col(int c)
            {
                if (c >= starts.Count || starts[c] >= l.Length) return "";
                int end = c + 1 < starts.Count ? Math.Min(starts[c + 1], l.Length) : l.Length;
                return l[starts[c]..end].Trim();
            }
            var cols = Enumerable.Range(0, starts.Count).Select(Col).ToList();
            if (cols.Count < 3 || cols[1].Length == 0) continue;
            // The last column is Source when its values are a source's name; "Available" / "Match" sit before it.
            string source = "", available = "";
            int last = cols.Count - 1;
            bool lastIsSource = cols[last] is "winget" or "msstore" or "" && cols.Count > 3;
            if (lastIsSource) source = cols[last];
            bool upgrade = args.StartsWith("upgrade") || args.StartsWith("list");
            if (upgrade && cols.Count >= 4 && (lastIsSource ? cols.Count >= 5 : true)) available = cols[3];
            if (!upgrade) available = "";
            rows.Add(new App(cols[1], cols[0], cols[2], available, source, false));
        }
        return rows;
    }

    /// <summary>winget's output as lines, each the last thing written on it (progress spinners and bars rewrite the line with \r).</summary>
    private static List<string> Clean(string outp) =>
        outp.Replace("\r\n", "\n").Split('\n').Select(l => { int r = l.LastIndexOf('\r'); return (r >= 0 ? l[(r + 1)..] : l).TrimEnd(); })
            .Select(l => Regex.Replace(l, @"\x1B\[[0-9;?]*[A-Za-z]", "")).ToList();

    private static async Task<(int Code, string Output)> RunAsync(string args, TimeSpan timeout)
    {
        var sb = new StringBuilder();
        int code = await StreamAsync(args, l => sb.Append(l).Append('\n'), timeout, raw: true);
        return (code, sb.ToString());
    }

    /// <summary>Runs winget, handing each line (split at \n and \r) to <paramref name="onLine"/>. raw: lines as written (with \r kept for Clean).</summary>
    private static async Task<int> StreamAsync(string args, Action<string> onLine, TimeSpan? timeout = null, bool raw = false)
    {
        var psi = new ProcessStartInfo("winget.exe", args)
        {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8, StandardErrorEncoding = Encoding.UTF8,
        };
        Process p;
        try { p = Process.Start(psi) ?? throw new InvalidOperationException("winget didn't start"); }
        catch (System.ComponentModel.Win32Exception)
        {
            throw new InvalidOperationException("winget isn't installed. Get \"App Installer\" from the Microsoft Store, then try again.");
        }
        using (p)
        {
            var errTask = p.StandardError.ReadToEndAsync();
            var buf = new char[4096];
            var line = new StringBuilder();
            using var cts = new CancellationTokenSource(timeout ?? TimeSpan.FromHours(2));
            try
            {
                int n;
                while ((n = await p.StandardOutput.ReadAsync(buf.AsMemory(), cts.Token)) > 0)
                {
                    for (int i = 0; i < n; i++)
                    {
                        char c = buf[i];
                        if (c == '\n' || (!raw && c == '\r'))
                        {
                            if (line.Length > 0 || c == '\n') onLine(line.ToString());
                            line.Clear();
                        }
                        else line.Append(c);
                    }
                }
                if (line.Length > 0) onLine(line.ToString());
                await p.WaitForExitAsync(cts.Token);
            }
            catch (OperationCanceledException)
            {
                try { p.Kill(true); } catch { }
                throw new TimeoutException("winget took too long");
            }
            var err = (await errTask).Trim();
            if (err.Length > 0) foreach (var l in err.Split('\n')) onLine(l.TrimEnd('\r'));
            return p.ExitCode;
        }
    }
}
