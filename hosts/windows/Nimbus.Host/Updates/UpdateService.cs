using System.Diagnostics;
using System.IO.Compression;
using System.Net.Http.Headers;
using System.Reflection;
using System.Security.Cryptography;
using System.Text.Json;

namespace Nimbus.Host.Updates;

public sealed record UpdateInfo(string Version, string Tag, string? Notes, string ZipUrl, string? ShaUrl, long Size, DateTimeOffset? Published);

public sealed record UpdateStatus(
    string Current, string State, UpdateInfo? Latest, double? Progress, string? Error,
    bool CanInstallHere, string InstallFolder, bool AutoCheck, long LastChecked);

/// <summary>
/// Updates from GitHub Releases. A release carries Nimbus-win-x64.zip (the published app) and
/// Nimbus-win-x64.zip.sha256. Nimbus downloads and verifies it, unpacks it next to the app,
/// then a small script swaps folders after Nimbus exits and starts the new version.
/// The previous version is kept as app.old and restored automatically if the new one keeps crashing.
/// </summary>
public sealed class UpdateService
{
    public const string Repo = "Dragongamer9/hearth-os";
    private const string Asset = "Nimbus-win-x64.zip";
    /// <summary>Exit code meaning "closing to install an update" (the watchdog must not restart).</summary>
    public const int UpdateExitCode = 3;

    private static readonly HttpClient Http = CreateHttp();
    private readonly Func<bool> _autoCheck;

    public string State { get; private set; } = "idle";   // idle | checking | upToDate | available | downloading | ready | error
    public UpdateInfo? Latest { get; private set; }
    public double? Progress { get; private set; }
    public string? Error { get; private set; }
    public long LastChecked { get; private set; }
    private string? _stagedDir;

    /// <summary>Raised (any thread) when State or Progress changes.</summary>
    public event Action? Changed;

    public UpdateService(Func<bool> autoCheck) => _autoCheck = autoCheck;

    public static string Root => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus");
    public static string InstallFolder => Path.Combine(Root, "app");
    private static string UpdatesDir => Path.Combine(Root, "updates");

    public static string CurrentVersion
    {
        get
        {
            var v = Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "0.0.0";
            int plus = v.IndexOf('+');            // strip "+commit" build metadata
            return plus > 0 ? v[..plus] : v;
        }
    }

    /// <summary>Updates replace the installed copy only (not a development build in a project folder).</summary>
    public static bool RunningFromInstall =>
        string.Equals(Path.GetFullPath(AppContext.BaseDirectory).TrimEnd('\\'), Path.GetFullPath(InstallFolder).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase);

    public UpdateStatus Status() => new(CurrentVersion, State, Latest, Progress, Error, RunningFromInstall, InstallFolder, _autoCheck(), LastChecked);

    private void Set(string state, double? progress = null, string? error = null)
    {
        State = state; Progress = progress; Error = error;
        Changed?.Invoke();
    }

    // ------------------------------------------------------------------ check

    public async Task<UpdateStatus> CheckAsync()
    {
        if (State is "downloading") return Status();
        // A downloaded update stays ready to install through a check (the timer checks every few hours).
        bool wasReady = State == "ready" && _stagedDir is not null && Directory.Exists(_stagedDir);
        Set("checking");
        try
        {
            // All recent releases, newest Windows build first (see LegacyTag for the old previews that are skipped).
            using var res = await Http.GetAsync($"https://api.github.com/repos/{Repo}/releases?per_page=30");
            LastChecked = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            res.EnsureSuccessStatusCode();

            using var doc = JsonDocument.Parse(await res.Content.ReadAsStringAsync());
            JsonElement? best = null;
            string bestTag = "";
            foreach (var rel in doc.RootElement.EnumerateArray())
            {
                if (rel.TryGetProperty("draft", out var dr) && dr.ValueKind == JsonValueKind.True) continue;
                if (rel.TryGetProperty("prerelease", out var pr) && pr.ValueKind == JsonValueKind.True) continue;
                var t = rel.GetProperty("tag_name").GetString() ?? "";
                if (LegacyTag.IsMatch(t)) continue;
                if (!rel.GetProperty("assets").EnumerateArray().Any(a => a.GetProperty("name").GetString() == Asset)) continue;
                if (best is null || IsNewer(t.TrimStart('v', 'V'), bestTag.TrimStart('v', 'V'))) { best = rel.Clone(); bestTag = t; }
            }
            if (best is not { } r) { Latest = null; Set("upToDate"); return Status(); }
            var tag = bestTag;
            string? zip = null, sha = null; long size = 0;
            foreach (var a in r.GetProperty("assets").EnumerateArray())
            {
                var name = a.GetProperty("name").GetString();
                var url = a.GetProperty("browser_download_url").GetString();
                if (name == Asset) { zip = url; size = a.GetProperty("size").GetInt64(); }
                if (name == Asset + ".sha256") sha = url;
            }
            if (zip is null) { Latest = null; Set("upToDate"); return Status(); }

            DateTimeOffset? published = r.TryGetProperty("published_at", out var p) && p.ValueKind == JsonValueKind.String
                ? DateTimeOffset.Parse(p.GetString()!, System.Globalization.CultureInfo.InvariantCulture, System.Globalization.DateTimeStyles.AssumeUniversal) : null;
            Latest = new UpdateInfo(tag.TrimStart('v', 'V'), tag, r.TryGetProperty("body", out var b) ? b.GetString() : null, zip, sha, size, published);

            if (wasReady) Set("ready", 1);
            else Set(IsNewer(Latest.Version, CurrentVersion) ? "available" : "upToDate");
        }
        catch (Exception ex)
        {
            if (wasReady) Set("ready", 1);
            else Set("error", error: "Couldn't check for updates: " + Short(ex));
        }
        return Status();
    }

    /// <summary>
    /// Nimbus was renumbered to 0.0.x at 0.0.5. Linux previews from before that (v0.3.x to v0.5.x with -linux.N)
    /// look newer than they are, so they're skipped. Plain v0.3.0 and later are today's releases (skipping them
    /// kept Ember from ever being offered 0.3.0 to 0.5.1).
    /// </summary>
    private static readonly System.Text.RegularExpressions.Regex LegacyTag = new(@"^v?0\.[3-5]\.\d+-linux\.");

    public static bool IsNewer(string candidate, string current)
    {
        static Version Parse(string s)
        {
            var core = s.Split('-', '+')[0];
            return Version.TryParse(core.Count(c => c == '.') == 0 ? core + ".0" : core, out var v) ? v : new Version(0, 0);
        }
        return Parse(candidate) > Parse(current);
    }

    // ------------------------------------------------------------------ download

    public async Task DownloadAsync()
    {
        if (Latest is null || State is "downloading" or "ready") return;
        // Started without waiting (update.download), so a problem is shown through the state, never thrown.
        if (!RunningFromInstall) { Set("error", error: "This copy of Ember is a development build. Updates install to the copy set up by install-shell.cmd."); return; }
        var info = Latest;
        Set("downloading", 0);
        try
        {
            Directory.CreateDirectory(UpdatesDir);
            var zipPath = Path.Combine(UpdatesDir, $"Nimbus-{info.Version}.zip");

            using (var res = await Http.GetAsync(info.ZipUrl, HttpCompletionOption.ResponseHeadersRead))
            {
                res.EnsureSuccessStatusCode();
                long total = res.Content.Headers.ContentLength ?? info.Size;
                await using var src = await res.Content.ReadAsStreamAsync();
                await using var dst = File.Create(zipPath);
                var buf = new byte[1 << 16];
                long done = 0; int n; var lastReport = DateTime.MinValue;
                while ((n = await src.ReadAsync(buf)) > 0)
                {
                    await dst.WriteAsync(buf.AsMemory(0, n));
                    done += n;
                    if (total > 0 && DateTime.UtcNow - lastReport > TimeSpan.FromMilliseconds(250))
                    {
                        lastReport = DateTime.UtcNow;
                        Set("downloading", Math.Min(0.99, (double)done / total));
                    }
                }
            }

            // Integrity check against the published SHA-256.
            if (info.ShaUrl is not null)
            {
                var expected = (await Http.GetStringAsync(info.ShaUrl)).Trim().Split(' ', '\t', '\r', '\n')[0].ToLowerInvariant();
                string actual;
                await using (var fs = File.OpenRead(zipPath)) actual = Convert.ToHexString(await SHA256.HashDataAsync(fs)).ToLowerInvariant();
                if (actual != expected) { File.Delete(zipPath); throw new InvalidDataException("The download was damaged (checksum mismatch). Try again."); }
            }

            var stage = Path.Combine(UpdatesDir, "stage-" + info.Version);
            if (Directory.Exists(stage)) Directory.Delete(stage, true);
            ZipFile.ExtractToDirectory(zipPath, stage);
            if (!File.Exists(Path.Combine(stage, "Nimbus.exe")) || !File.Exists(Path.Combine(stage, "wwwroot", "index.html")))
                throw new InvalidDataException("The update package is incomplete.");
            File.Delete(zipPath);

            _stagedDir = stage;
            Set("ready", 1);
        }
        catch (Exception ex)
        {
            Set("error", error: "Update download failed: " + Short(ex));
        }
    }

    // ------------------------------------------------------------------ install

    /// <summary>
    /// Writes and starts the swap script, then returns: the caller must exit Nimbus with UpdateExitCode.
    /// The script waits for every Nimbus process to end, moves app → app.old and the staged folder → app,
    /// marks it as just updated, and starts Nimbus again (as the shell if it was the shell).
    /// </summary>
    public void BeginInstall(bool asShell, bool windowed)
    {
        if (State != "ready" || _stagedDir is null) throw new InvalidOperationException("No update is ready to install.");
        var app = InstallFolder;
        var args = asShell ? "--shell" : windowed ? "--windowed" : "";
        var script = Path.Combine(UpdatesDir, "apply-update.cmd");
        var log = Path.Combine(UpdatesDir, "apply-update.log");
        string NUL = ">" + "nul";
        // The folders reach the script as environment variables, not written into it: cmd reads a script in the
        // console's code page, so a user name with letters outside it (José, Łukasz) would come out garbled, and
        // characters like & % ^ would be taken as commands. Expanded variables are used as they are.
        File.WriteAllText(script, string.Join("\r\n", new[]
        {
            "@echo off",
            "rem Written by Nimbus to install an update. Safe to delete.",
            "set \"APP=%EMBER_UPDATE_APP%\"",
            "set \"NEW=%EMBER_UPDATE_NEW%\"",
            "set \"LOG=%EMBER_UPDATE_LOG%\"",
            "echo %DATE% %TIME% applying update > \"%LOG%\"",
            // Wait up to 30 s for every Nimbus process (UI and watchdog) to exit.
            "for /l %%i in (1,1,30) do (",
            $"  tasklist /fi \"imagename eq Nimbus.exe\" | find /i \"Nimbus.exe\" {NUL} || goto gone",
            $"  timeout /t 1 {NUL}",
            ")",
            $"taskkill /im Nimbus.exe /f {NUL} 2>&1",
            ":gone",
            // Hearth's other programs in the same folder (Terminal, Disks, hearthfetch) would keep it locked.
            $"taskkill /im hearth-terminal.exe /f {NUL} 2>&1",
            $"taskkill /im hearth-disks.exe /f {NUL} 2>&1",
            $"taskkill /im hearthfetch.exe /f {NUL} 2>&1",
            $"if exist \"%APP%.old\" rmdir /s /q \"%APP%.old\"",
            // Files can stay locked for a moment after exit; retry the move.
            "for /l %%i in (1,1,10) do (",
            "  move \"%APP%\" \"%APP%.old\" >> \"%LOG%\" 2>&1",
            "  if not exist \"%APP%\" goto moved",
            $"  timeout /t 1 {NUL}",
            ")",
            ":moved",
            // Still locked: keep the version that's there (moving the new one now would put it inside it).
            "if exist \"%APP%\" (",
            "  echo couldn't move the old version, keeping it >> \"%LOG%\"",
            "  goto launch",
            ")",
            $"move \"%NEW%\" \"%APP%\" >> \"%LOG%\" 2>&1",
            "if not exist \"%APP%\\Nimbus.exe\" (",
            "  echo new version missing, rolling back >> \"%LOG%\"",
            $"  if exist \"%APP%\" rmdir /s /q \"%APP%\"",
            $"  move \"%APP%.old\" \"%APP%\" >> \"%LOG%\" 2>&1",
            ") else (",
            $"  echo updated > \"%APP%\\just-updated\"",
            ")",
            ":launch",
            "echo done >> \"%LOG%\"",
            $"start \"\" \"%APP%\\Nimbus.exe\" {args}",
            "",
        }));
        var psi = Script(script);
        psi.Environment["EMBER_UPDATE_APP"] = app;
        psi.Environment["EMBER_UPDATE_NEW"] = _stagedDir;
        psi.Environment["EMBER_UPDATE_LOG"] = log;
        Process.Start(psi)?.Dispose();
    }

    // ------------------------------------------------------------------ rollback support

    private static string Marker => Path.Combine(InstallFolder, "just-updated");

    /// <summary>Called once the new version has run fine for a while: it's no longer "just updated".</summary>
    public static void ConfirmHealthy()
    {
        try { if (File.Exists(Marker)) File.Delete(Marker); } catch { }
    }

    /// <summary>
    /// For the watchdog: if the current version was just installed and keeps crashing, swap app.old back.
    /// Returns true if a rollback was started (Nimbus will be restarted by the script).
    /// </summary>
    public static bool TryRollback(bool asShell)
    {
        var app = InstallFolder;
        if (!File.Exists(Marker) || !Directory.Exists(app + ".old")) return false;
        var script = Path.Combine(Root, "rollback-update.cmd");
        string NUL = ">" + "nul";
        File.WriteAllText(script, string.Join("\r\n", new[]
        {
            "@echo off",
            $"timeout /t 2 {NUL}",
            $"taskkill /im Nimbus.exe /f {NUL} 2>&1",
            $"taskkill /im hearth-terminal.exe /f {NUL} 2>&1",
            $"taskkill /im hearth-disks.exe /f {NUL} 2>&1",
            $"taskkill /im hearthfetch.exe /f {NUL} 2>&1",
            "set \"APP=%EMBER_UPDATE_APP%\"",
            "if exist \"%APP%.bad\" rmdir /s /q \"%APP%.bad\"",
            // Files can stay locked for a moment after exit; retry the move.
            "for /l %%i in (1,1,10) do (",
            "  move \"%APP%\" \"%APP%.bad\"",
            "  if not exist \"%APP%\" goto moved",
            $"  timeout /t 1 {NUL}",
            ")",
            ":moved",
            // Still locked: moving the old version now would put it inside the current one (and lose it).
            "if not exist \"%APP%\" move \"%APP%.old\" \"%APP%\"",
            $"start \"\" \"%APP%\\Nimbus.exe\" {(asShell ? "--shell" : "")}",
            "",
        }));
        var psi = Script(script);
        psi.Environment["EMBER_UPDATE_APP"] = app;
        Process.Start(psi)?.Dispose();
        return true;
    }

    // ------------------------------------------------------------------ helpers

    /// <summary>Runs a script with cmd. The doubled quotes matter: cmd /c takes off the outer pair, so the path stays
    /// quoted even when it has spaces or characters like &amp; in it (a user name).</summary>
    private static ProcessStartInfo Script(string script) =>
        new("cmd.exe", $"/d /c \"\"{script}\"\"") { UseShellExecute = false, CreateNoWindow = true, WorkingDirectory = Root };

    private static HttpClient CreateHttp()
    {
        var h = new HttpClient { Timeout = TimeSpan.FromMinutes(10) };
        h.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("Nimbus", CurrentVersion));
        h.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/vnd.github+json"));
        return h;
    }

    private static string Short(Exception ex) => ex is HttpRequestException ? "no connection to GitHub" : ex.Message;
}
