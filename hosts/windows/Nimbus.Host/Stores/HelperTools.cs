using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;

namespace Nimbus.Host.Stores;

/// <summary>
/// The open-source command-line tools Nimbus uses to talk to Epic and GOG without their launchers:
/// legendary (Epic) and gogdl (GOG, from the Heroic launcher). They are downloaded from their GitHub
/// releases the first time you sign in, checked against a pinned SHA-256, and kept in %LOCALAPPDATA%\Nimbus\tools.
/// </summary>
public static class HelperTools
{
    public sealed record Tool(string Name, string File, string Url, string Sha256, long Size);

    public static readonly Tool Legendary = new("legendary", "legendary.exe",
        "https://github.com/legendary-gl/legendary/releases/download/0.21.1/legendary_windows_x64.exe",
        "6be77857dc0a6dd33ea7442ce8b5291944ec6da53757f81ca95b57ce794baf5b", 17_654_404);

    public static readonly Tool Gogdl = new("gogdl", "gogdl.exe",
        "https://github.com/Heroic-Games-Launcher/heroic-gogdl/releases/download/v1.3.0/gogdl_windows_x86_64.exe",
        "69ea54467371803f681d6c39805992e3a4b8ddccb44ac8a1de7b1e3c80deaeec", 12_304_645);

    public static string DataDir { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus");

    private static string ToolsDir => Path.Combine(DataDir, "tools");

    public static string PathOf(Tool t) => Path.Combine(ToolsDir, t.File);
    public static bool IsReady(Tool t) => File.Exists(PathOf(t));

    private static readonly SemaphoreSlim DownloadLock = new(1, 1);
    internal static readonly HttpClient Http = CreateHttp();

    private static HttpClient CreateHttp()
    {
        var h = new HttpClient { Timeout = TimeSpan.FromSeconds(60) };
        h.DefaultRequestHeaders.UserAgent.ParseAdd("Nimbus/" + (typeof(HelperTools).Assembly.GetName().Version?.ToString(3) ?? "0"));
        return h;
    }

    /// <summary>Makes sure the tool is downloaded and intact. Returns its path.</summary>
    public static async Task<string> EnsureAsync(Tool t, Action<double>? progress = null)
    {
        var path = PathOf(t);
        if (File.Exists(path)) return path;
        await DownloadLock.WaitAsync();
        try
        {
            if (File.Exists(path)) return path;
            Directory.CreateDirectory(ToolsDir);
            var tmp = path + ".download";
            using (var resp = await Http.GetAsync(t.Url, HttpCompletionOption.ResponseHeadersRead))
            {
                resp.EnsureSuccessStatusCode();
                long total = resp.Content.Headers.ContentLength ?? t.Size;
                await using var src = await resp.Content.ReadAsStreamAsync();
                await using var dst = File.Create(tmp);
                var buf = new byte[81920];
                long done = 0; int n;
                while ((n = await src.ReadAsync(buf)) > 0)
                {
                    await dst.WriteAsync(buf.AsMemory(0, n));
                    done += n;
                    progress?.Invoke(total > 0 ? Math.Min(1, (double)done / total) : 0);
                }
            }
            string hash;
            await using (var f = File.OpenRead(tmp)) hash = Convert.ToHexString(await SHA256.HashDataAsync(f)).ToLowerInvariant();
            if (hash != t.Sha256)
            {
                File.Delete(tmp);
                throw new InvalidOperationException($"The download of {t.Name} didn't match its checksum, so Ember didn't use it. Try again later.");
            }
            File.Move(tmp, path, overwrite: true);
            SystemBridge.Log($"tools: downloaded {t.Name}");
            return path;
        }
        finally { DownloadLock.Release(); }
    }

    public sealed record RunResult(int ExitCode, string Out, string Err)
    {
        /// <summary>The last meaningful error line, for showing to the user.</summary>
        public string Problem()
        {
            var lines = (Err + "\n" + Out).Split('\n').Select(l => l.Trim()).Where(l => l.Length > 0).ToList();
            var err = lines.LastOrDefault(l => l.Contains("ERROR", StringComparison.Ordinal) || l.Contains("CRITICAL", StringComparison.Ordinal))
                      ?? lines.LastOrDefault() ?? $"exit code {ExitCode}";
            // "[cli] ERROR: Login failed" → "Login failed"
            int i = err.IndexOf(": ", StringComparison.Ordinal);
            return i > 0 && err.StartsWith('[') ? err[(i + 2)..] : err;
        }
    }

    /// <summary>
    /// Runs a tool hidden (no console window) and collects its output. <paramref name="onLine"/> sees
    /// every line from stdout and stderr as it arrives (both tools log progress to stderr).
    /// </summary>
    public static async Task<RunResult> RunAsync(Tool t, IEnumerable<string> args, IDictionary<string, string>? env = null,
                                                 Action<string>? onLine = null, Action<Process>? started = null,
                                                 CancellationToken ct = default, TimeSpan? timeout = null)
    {
        var exe = await EnsureAsync(t);
        var psi = new ProcessStartInfo(exe)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            RedirectStandardInput = true,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
            WorkingDirectory = ToolsDir,
        };
        foreach (var a in args) psi.ArgumentList.Add(a);
        psi.Environment["PYTHONIOENCODING"] = "utf-8";
        psi.Environment["PYTHONUNBUFFERED"] = "1";
        if (env is not null) foreach (var (k, v) in env) psi.Environment[k] = v;

        using var p = new Process { StartInfo = psi, EnableRaisingEvents = true };
        var outSb = new StringBuilder(); var errSb = new StringBuilder();
        var outDone = new TaskCompletionSource(); var errDone = new TaskCompletionSource();
        // These run on a thread-pool thread: an error in onLine (an odd progress line it can't read) would end Ember.
        void Line(string l) { try { onLine?.Invoke(l); } catch (Exception ex) { SystemBridge.Log($"{t.Name}: line not read: {ex.Message}"); } }
        p.OutputDataReceived += (_, e) => { if (e.Data is null) outDone.TrySetResult(); else { lock (outSb) outSb.AppendLine(e.Data); Line(e.Data); } };
        p.ErrorDataReceived += (_, e) => { if (e.Data is null) errDone.TrySetResult(); else { lock (errSb) errSb.AppendLine(e.Data); Line(e.Data); } };
        p.Start();
        p.StandardInput.Close();   // nothing will ever answer a prompt: make it fail instead of hang
        p.BeginOutputReadLine();
        p.BeginErrorReadLine();
        started?.Invoke(p);

        using var timeoutCts = timeout is { } to ? new CancellationTokenSource(to) : new CancellationTokenSource();
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(ct, timeoutCts.Token);
        try { await p.WaitForExitAsync(linked.Token); }
        catch (OperationCanceledException)
        {
            try { p.Kill(entireProcessTree: true); } catch { }
            if (ct.IsCancellationRequested) throw;
            throw new TimeoutException($"{t.Name} took too long to respond.");
        }
        await Task.WhenAny(Task.WhenAll(outDone.Task, errDone.Task), Task.Delay(2000));
        string o, e2;
        lock (outSb) o = outSb.ToString();
        lock (errSb) e2 = errSb.ToString();
        return new RunResult(p.ExitCode, o, e2);
    }
}
