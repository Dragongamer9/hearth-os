using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using Nimbus.Host.Library;

namespace Nimbus.Host.Services;

/// <summary>
/// Show demo (docs/DEMO.md) on Ember: Quad Mech, Hearth's multi-seat demo game, downloaded from Hearth OS's release
/// (quadmech.json → QuadMech-windows-x64.zip: this repository's releases only, https, size cap, SHA-256) and unpacked
/// (names checked: nothing outside its folder, no links) into %LOCALAPPDATA%\Hearth\demo\quadmech\&lt;version&gt;.
/// Windows can't split one PC into seats, so it plays on this screen: split-screen for everyone with a controller
/// (<c>-- --local --players=auto</c>), or the self-playing attract mode when no controller is connected. It starts
/// through the library like any game (Quick Resume, closing). The same answers as hearth-core's demo.rs.
/// </summary>
public sealed class DemoService
{
    public const string ManifestUrl = "https://github.com/Dragongamer9/hearth-os/releases/latest/download/quadmech.json";
    private const string Releases = "https://github.com/Dragongamer9/hearth-os/releases/";
    public const string WindowsZip = "QuadMech-windows-x64.zip";
    public const string Exe = "QuadMech.exe";
    public const string GameId = "demo:quadmech";
    public const string Title = "Quad Mech (demo)";
    public const long MaxZip = 400L * 1024 * 1024;
    private const long MaxUnpacked = 1024L * 1024 * 1024;
    private const int MaxEntries = 64;
    public const int AttractSeconds = 3600;

    public static string Root { get; } = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Hearth", "demo", "quadmech");

    private static readonly HttpClient Http = CreateHttp();
    private static HttpClient CreateHttp()
    {
        var h = new HttpClient(new HttpClientHandler { AllowAutoRedirect = true, MaxAutomaticRedirections = 5 }) { Timeout = TimeSpan.FromMinutes(30) };
        h.DefaultRequestHeaders.UserAgent.ParseAdd("Ember/" + (typeof(DemoService).Assembly.GetName().Version?.ToString(3) ?? "0"));
        return h;
    }

    private readonly Action<string, object> _event;
    private readonly object _lock = new();
    private bool _downloading;
    private double _pct;
    private string _text = "";
    private string? _error;
    private (string Version, long Size)? _latest;

    public DemoService(Action<string, object> ev) { _event = ev; }

    // ------------------------------------------------------------------ checks (the same as demo.rs)
    public static bool ValidVersion(string? v) =>
        !string.IsNullOrEmpty(v) && v.Length <= 64 && char.IsAsciiLetterOrDigit(v[0]) && v.All(c => char.IsAsciiLetterOrDigit(c) || c is '.' or '_' or '-');
    public static bool ValidSha256(string? s) => s is { Length: 64 } && s.All(c => char.IsAsciiDigit(c) || c is >= 'a' and <= 'f');

    /// <summary>Only this repository's release downloads of exactly that file.</summary>
    public static bool ValidAssetUrl(string? url, string file)
    {
        if (url is null || !url.StartsWith(Releases, StringComparison.Ordinal)) return false;
        var rest = url[Releases.Length..];
        if (rest.StartsWith("latest/download/", StringComparison.Ordinal)) return rest["latest/download/".Length..] == file;
        if (!rest.StartsWith("download/", StringComparison.Ordinal)) return false;
        var r = rest["download/".Length..];
        var slash = r.IndexOf('/');
        if (slash < 2 || slash > 40) return false;
        var tag = r[..slash];
        return r[(slash + 1)..] == file && tag[0] == 'v' && tag[1..].All(c => char.IsAsciiLetterOrDigit(c) || c is '.' or '_' or '-');
    }

    public sealed record Asset(string Url, string Sha256, long Size);
    public sealed record Manifest(string Version, Asset Windows);

    public static Manifest ParseManifest(JsonElement m)
    {
        string S(JsonElement e, string k) => e.ValueKind == JsonValueKind.Object && e.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "";
        var version = S(m, "version");
        if (!ValidVersion(version)) throw new InvalidOperationException("The demo's version isn't right.");
        var files = m.ValueKind == JsonValueKind.Object && m.TryGetProperty("files", out var f) ? f : default;
        var w = files.ValueKind == JsonValueKind.Object && files.TryGetProperty("windows", out var ww) ? ww : default;
        var url = S(w, "url");
        if (!ValidAssetUrl(url, WindowsZip)) throw new InvalidOperationException("The demo's download isn't from Hearth's releases, so it wasn't used.");
        var sha = S(w, "sha256").ToLowerInvariant();
        if (!ValidSha256(sha)) throw new InvalidOperationException("The demo's checksum is missing.");
        long size = w.ValueKind == JsonValueKind.Object && w.TryGetProperty("size", out var sz) && sz.TryGetInt64(out var n) ? n : 0;
        if (size <= 0 || size > MaxZip) throw new InvalidOperationException("The demo's download size isn't right.");
        return new Manifest(version, new Asset(url, sha, size));
    }

    /// <summary>A name that can only land inside its folder: relative, no "." or ".." parts, plain characters.</summary>
    public static bool SafeName(string name)
    {
        if (name.Length is 0 or > 200 || name.StartsWith('/')) return false;
        if (!name.All(c => char.IsAsciiLetterOrDigit(c) || c is '.' or '_' or '-' or '/')) return false;
        var t = name.EndsWith('/') ? name[..^1] : name;
        return t.Length > 0 && t.Split('/').All(p => p.Length > 0 && p != "." && p != "..");
    }

    /// <summary>The game's arguments (after Godot's "--"): everyone with a controller plays split-screen on this
    /// screen, or the mechs play by themselves (attract mode).</summary>
    public static string BuildArgs(bool attract) => attract ? $"-- --local --players=auto --selftest={AttractSeconds} --quality=low" : "-- --local --players=auto";

    // ------------------------------------------------------------------ installed copy
    public static (string Version, string Dir)? Installed()
    {
        try
        {
            var v = File.ReadAllText(Path.Combine(Root, "current")).Trim();
            if (!ValidVersion(v)) return null;
            var dir = Path.Combine(Root, v);
            return File.Exists(Path.Combine(dir, Exe)) ? (v, dir) : null;
        }
        catch { return null; }
    }

    /// <summary>The library's entry once installed (LibraryService's scan).</summary>
    public static List<Game> LibraryGames()
    {
        var list = new List<Game>();
        if (Installed() is not { } inst) return list;
        long added = 0;
        try { added = Time.Ms(File.GetLastWriteTimeUtc(Path.Combine(Root, "current"))); } catch { }
        var g = new Game { Id = GameId, Title = Title, Source = "demo", InstallPath = inst.Dir, Added = added, Exe = Path.Combine(inst.Dir, Exe), Args = "", WorkDir = inst.Dir };
        g.ProcessDirs.Add(inst.Dir);
        foreach (var kind in new[] { "cover", "wide", "hero" })
        {
            var f = Path.Combine(inst.Dir, kind + ".png");
            if (File.Exists(f)) g.Art[kind] = f;
        }
        list.Add(g);
        return list;
    }

    public object Status(int controllers)
    {
        var inst = Installed();
        lock (_lock)
            return new
            {
                platform = "windows", installed = inst is not null, version = inst?.Version, downloading = _downloading, pct = _pct, text = _text, error = _error,
                busy = (string?)null, size = _latest?.Size, latest = _latest?.Version, seatsAvailable = false, seatsHow = (string?)null,
                seatsReason = "Windows can't split one PC into seats: the demo plays split-screen on this screen.", seatsConfigured = false,
                controllers, extraSeat = false, active = false, attractSeconds = AttractSeconds,
            };
    }

    /// <summary>demo.status: the download size comes from quadmech.json the first time (not installed yet).</summary>
    public async Task<object> StatusAsync(int controllers)
    {
        bool need;
        lock (_lock) need = _latest is null && !_downloading;
        if (need && Installed() is null)
        {
            try
            {
                using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(8));
                var m = await FetchManifestAsync(cts.Token);
                lock (_lock) _latest = (m.Version, m.Windows.Size);
            }
            catch { /* offline: no size shown */ }
        }
        return Status(controllers);
    }

    private static async Task<Manifest> FetchManifestAsync(CancellationToken ct)
    {
        using var resp = await Http.GetAsync(ManifestUrl, ct);
        if ((int)resp.StatusCode == 404) throw new InvalidOperationException("GitHub doesn't have the demo yet (it comes with each Hearth OS release).");
        resp.EnsureSuccessStatusCode();
        var text = await resp.Content.ReadAsStringAsync(ct);
        if (text.Length > 64 * 1024) throw new InvalidOperationException("The demo's description is too big.");
        using var doc = JsonDocument.Parse(text);
        return ParseManifest(doc.RootElement);
    }

    private void Progress(double f, string text)
    {
        var pct = Math.Round(Math.Clamp(f, 0, 1) * 100);
        lock (_lock) { _pct = pct; _text = text; }
        _event("demo.progress", new { action = "install", pct, text });
    }

    /// <summary>demo.install: download, check and unpack in the background; demo.progress, then demo.done.</summary>
    public object Install(int controllers, Action onInstalled)
    {
        lock (_lock)
        {
            if (_downloading) return Status(controllers);
            _downloading = true; _pct = 0; _text = "Asking GitHub for the demo"; _error = null;
        }
        _ = Task.Run(async () =>
        {
            try
            {
                using var cts = new CancellationTokenSource(TimeSpan.FromMinutes(35));
                var m = await FetchManifestAsync(cts.Token);
                if (Installed()?.Version != m.Version)
                {
                    lock (_lock) _latest = (m.Version, m.Windows.Size);
                    Directory.CreateDirectory(Root);
                    var tmp = Path.Combine(Root, ".download.zip");
                    try
                    {
                        await DownloadAsync(m.Windows, tmp, cts.Token);
                        InstallZip(tmp, m.Version, m.Windows.Sha256, m.Windows.Size);
                    }
                    finally { try { File.Delete(tmp); } catch { } }
                }
                lock (_lock) { _downloading = false; _pct = 100; _text = ""; }
                onInstalled();
                _event("demo.done", new { action = "install", ok = true, version = m.Version });
            }
            catch (Exception ex)
            {
                var msg = ex is HttpRequestException or TaskCanceledException ? "The download didn't work: " + ex.Message : ex.Message;
                SystemBridge.Log("demo: install FAILED " + msg);
                lock (_lock) { _downloading = false; _error = msg; }
                _event("demo.done", new { action = "install", ok = false, error = msg });
            }
        });
        return Status(controllers);
    }

    private async Task DownloadAsync(Asset a, string to, CancellationToken ct)
    {
        using var resp = await Http.GetAsync(a.Url, HttpCompletionOption.ResponseHeadersRead, ct);
        if (resp.RequestMessage?.RequestUri is { } final && final.Scheme != Uri.UriSchemeHttps) throw new InvalidOperationException("The download left https, so it was stopped.");
        resp.EnsureSuccessStatusCode();
        if (resp.Content.Headers.ContentLength is long len && len != a.Size) throw new InvalidOperationException("The demo's download is the wrong size.");
        await using var src = await resp.Content.ReadAsStreamAsync(ct);
        await using (var dst = File.Create(to))
        {
            var buf = new byte[1 << 16];
            long done = 0; int n; var last = DateTime.MinValue;
            while ((n = await src.ReadAsync(buf, ct)) > 0)
            {
                done += n;
                if (done > a.Size) throw new InvalidOperationException("The demo's download is bigger than it should be, so it was stopped.");
                await dst.WriteAsync(buf.AsMemory(0, n), ct);
                if ((DateTime.UtcNow - last).TotalMilliseconds > 400)
                {
                    last = DateTime.UtcNow;
                    Progress(0.02 + 0.88 * done / a.Size, $"Downloading the demo: {done / 1048576} of {a.Size / 1048576} MB");
                }
            }
        }
    }

    private void InstallZip(string zip, string version, string sha, long size)
    {
        if (new FileInfo(zip).Length != size) throw new InvalidOperationException("The demo's download is the wrong size.");
        Progress(0.92, "Checking the download");
        string got;
        using (var fs = File.OpenRead(zip)) got = Convert.ToHexString(SHA256.HashData(fs)).ToLowerInvariant();
        if (got != sha) throw new InvalidOperationException("The demo's download didn't match its checksum, so it wasn't used.");
        Progress(0.96, "Unpacking");
        var dest = Path.Combine(Root, version);
        var staging = Path.Combine(Root, ".unpack-" + version);
        if (Directory.Exists(staging)) Directory.Delete(staging, true);
        Directory.CreateDirectory(staging);
        try
        {
            var stagingFull = Path.GetFullPath(staging) + Path.DirectorySeparatorChar;
            using (var z = ZipFile.OpenRead(zip))
            {
                if (z.Entries.Count is 0 or > MaxEntries) throw new InvalidOperationException("The demo's download has an unusual number of files.");
                long total = 0; bool hasExe = false;
                var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
                foreach (var e in z.Entries)
                {
                    if (!SafeName(e.FullName)) throw new InvalidOperationException("The demo's download has a file name that could land outside its folder.");
                    // Made on Unix: the top of the attributes is the file's mode; only plain files and folders.
                    int kind = (e.ExternalAttributes >> 16) & 0xF000;
                    if (kind != 0 && kind != 0x8000 && kind != 0x4000) throw new InvalidOperationException("The demo's download has a link or special file in it.");
                    if (e.FullName.EndsWith('/')) continue;
                    if (!names.Add(e.FullName)) throw new InvalidOperationException("The demo's download has a file twice.");
                    total += e.Length;
                    if (total > MaxUnpacked) throw new InvalidOperationException("The demo is too big unpacked.");
                    if (e.FullName == Exe) hasExe = true;
                }
                if (!hasExe) throw new InvalidOperationException("The demo's download doesn't have the game in it.");
                foreach (var e in z.Entries)
                {
                    var target = Path.GetFullPath(Path.Combine(staging, e.FullName.Replace('/', Path.DirectorySeparatorChar)));
                    if (!target.StartsWith(stagingFull, StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("The demo's download has a file name that could land outside its folder.");
                    if (e.FullName.EndsWith('/')) { Directory.CreateDirectory(target); continue; }
                    Directory.CreateDirectory(Path.GetDirectoryName(target)!);
                    e.ExtractToFile(target, overwrite: false);
                }
            }
            if (Directory.Exists(dest)) Directory.Delete(dest, true);
            Directory.Move(staging, dest);
        }
        catch { try { Directory.Delete(staging, true); } catch { } throw; }
        File.WriteAllText(Path.Combine(Root, "current"), version);
        foreach (var d in Directory.GetDirectories(Root))
        {
            var n = Path.GetFileName(d);
            if (n != version && !n.StartsWith('.')) try { Directory.Delete(d, true); } catch { }
        }
        SystemBridge.Log($"demo: installed {version}");
    }

    /// <summary>demo.start: attract mode when asked, or when no controller is connected and nothing was asked.</summary>
    public static (bool Attract, string Mode) Pick(string? asked, int controllers)
    {
        if (asked is not (null or "" or "seats" or "local" or "attract")) throw new ArgumentException("Unknown demo mode");
        bool attract = asked == "attract" || (string.IsNullOrEmpty(asked) && controllers == 0);
        return (attract, attract ? "attract" : "local");
    }
}
