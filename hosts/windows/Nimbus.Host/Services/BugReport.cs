using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace Nimbus.Host.Services;

/// <summary>
/// Settings › System › Save logs / Send logs to the developer, on Ember: everything useful for a bug report in one
/// text file in the same "Hearth logs" format as Hearth's hearth-core logs.rs (a header line, then "===== Section ====="
/// blocks, the first "About this device" as JSON), with personal details taken out. Saved in your user folder (and
/// on any USB drive or SD card plugged in); Send also posts it to Hearth OS's bug-report service (the player
/// counter's server), which files it for the developer and answers with a short code.
/// </summary>
public static class BugReport
{
    public const string Service = "https://hearth-counter.game305-cl.workers.dev";
    private const int MaxSend = 600_000;

    private static string Tail(string text, int n)
    {
        var lines = text.Replace("\r\n", "\n").Split('\n');
        int skip = Math.Max(0, lines.Length - n);
        var head = skip > 0 ? $"[… {skip} older lines left out …]\n" : "";
        return head + string.Join("\n", lines.Skip(skip));
    }

    private static readonly (Regex Re, string To)[] Rules =
    {
        (new Regex("(?i)(\"?(?:access_token|refresh_token|id_token|token|password|passwd|psk|secret|cookie|authorization|session_?id|code)\"?\\s*[:=]\\s*\"?)[^\\s\",&}]{4,}"), "$1[removed]"),
        (new Regex("(?i)(bearer\\s+)[A-Za-z0-9._~+/=-]{8,}"), "$1[removed]"),
        (new Regex("[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}"), "[email]"),
        (new Regex("\\b7656119\\d{10}\\b"), "[steam-id]"),
        (new Regex("(?i)([?&](?:code|token|key|sig|signature)=)[^&\\s]+"), "$1[removed]"),
        (new Regex("(?i)(ssid[\\s:=]+)\\S+"), "$1[wifi]"),
    };

    /// <summary>Personal details out: the user name and user folder, sign-in tokens, e-mail addresses, Steam
    /// account ids, Wi-Fi names. The same rules as hearth-core's logs::redact.</summary>
    public static string Redact(string text, string user, string home)
    {
        var t = text;
        if (home.Length > 3) t = t.Replace(home, "~", StringComparison.OrdinalIgnoreCase);
        foreach (var (re, to) in Rules) t = re.Replace(t, to);
        if (user.Length >= 3) t = Regex.Replace(t, $@"\b{Regex.Escape(user)}\b", "[user]", RegexOptions.IgnoreCase);
        return t;
    }

    /// <summary>A note for an HTTP header: printable ASCII kept, everything else percent-encoded (UTF-8), at most
    /// 1000 bytes of the note. Same as hearth-core's logs::header_note.</summary>
    public static string HeaderNote(string note)
    {
        var n = (note ?? "").Trim();
        var bytes = Encoding.UTF8.GetBytes(n);
        int len = Math.Min(bytes.Length, 1000);
        while (len > 0 && len < bytes.Length && (bytes[len] & 0xC0) == 0x80) len--;   // not in the middle of a character
        var sb = new StringBuilder();
        for (int i = 0; i < len; i++)
        {
            byte b = bytes[i];
            if ((b >= '0' && b <= '9') || (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') || " -_.~!*'()".IndexOf((char)b) >= 0) sb.Append((char)b);
            else sb.Append('%').Append(b.ToString("X2"));
        }
        return sb.ToString();
    }

    private static string Run(string file, string args, int seconds = 15)
    {
        try
        {
            var psi = new ProcessStartInfo(file, args) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
            using var p = Process.Start(psi);
            if (p is null) return "(couldn't run " + file + ")";
            var o = p.StandardOutput.ReadToEndAsync();
            var e = p.StandardError.ReadToEndAsync();
            if (!p.WaitForExit(seconds * 1000)) { try { p.Kill(true); } catch { } return "(took too long)"; }
            return (o.GetAwaiter().GetResult() + e.GetAwaiter().GetResult()).Trim();
        }
        catch (Exception ex) { return "(" + ex.Message + ")"; }
    }

    private static string Processes()
    {
        var list = new List<(long Ws, string Name, int Pid)>();
        foreach (var p in Process.GetProcesses())
        {
            try { list.Add((p.WorkingSet64, p.ProcessName, p.Id)); } catch { }
            p.Dispose();
        }
        var sb = new StringBuilder();
        var mem = GC.GetGCMemoryInfo();
        sb.AppendLine($"Memory: {mem.TotalAvailableMemoryBytes / 1048576} MB in all, {mem.MemoryLoadBytes / 1048576} MB in use");
        sb.AppendLine();
        foreach (var x in list.OrderByDescending(x => x.Ws).Take(30)) sb.AppendLine($"{x.Ws / 1048576,7} MB  {x.Name} ({x.Pid})");
        return sb.ToString();
    }

    /// <summary>The report. <paramref name="sections"/>: Ember's own state the bridge adds (settings, Quick Resume,
    /// controllers, power), each a title and its text.</summary>
    public static string Report(string version, IEnumerable<(string Title, string Body)> sections)
    {
        var sb = new StringBuilder();
        sb.Append($"Hearth logs · {DateTime.Now:yyyy-MM-dd HH:mm} · Ember {version}\n\n");
        void Section(string title, string body) => sb.Append($"\n===== {title} =====\n{body.TrimEnd()}\n");

        JsonNode info;
        try { info = JsonSerializer.SerializeToNode(SystemInfo.Get(), new JsonSerializerOptions(JsonSerializerDefaults.Web)) ?? new JsonObject(); }
        catch { info = new JsonObject(); }
        if (info is JsonObject io) { io["version"] = version; io["host"] = "ember"; io["kind"] = "windows"; io["shell"] = Program.IsShell; }
        Section("About this device", info.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        foreach (var (t, b) in sections) Section(t, b);

        // Ember's log: the controller input lines first (the jumping-selection reports), then the newest lines, then
        // the errors on their own. The UI's frequent polls are left out.
        var log = SystemBridge.LogFile;
        try
        {
            if (File.Exists(log))
            {
                string text;
                using (var f = new FileStream(log, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
                using (var r = new StreamReader(f)) text = r.ReadToEnd();
                var spam = new Regex(@"^\d\d:\d\d:\d\d\.\d+ (library\.downloads|perf\.stats|sys\.status|pad\.status|input\.status|tasks\.(list|perf|alive)) ok \d+ms");
                var lines = text.Replace("\r\n", "\n").Split('\n').Where(l => !spam.IsMatch(l)).ToList();
                string Keep(List<string> v, int n) { int skip = Math.Max(0, v.Count - n); return (skip > 0 ? $"[… {skip} older lines left out …]\n" : "") + string.Join("\n", v.Skip(skip)); }
                var input = lines.Where(l => l.Contains(" input ")).ToList();
                if (input.Count > 0) Section("Controller input (newest last)", Keep(input, 400));
                var errors = lines.Where(l => l.Contains("FAILED") || l.Contains("UNHANDLED") || l.Contains(" failed") || l.Contains("error", StringComparison.OrdinalIgnoreCase)).ToList();
                if (errors.Count > 0) Section("Recent errors (newest last)", Keep(errors, 200));
                Section("Ember log", Keep(lines, 1500));
            }
            var old = log + ".old";
            if (File.Exists(old)) Section("Ember log (before that)", Tail(File.ReadAllText(old), 300));
        }
        catch (Exception ex) { Section("Ember log", "(couldn't read it: " + ex.Message + ")"); }

        var upd = Path.Combine(Stores.HelperTools.DataDir, "updates", "apply-update.log");
        try { if (File.Exists(upd)) Section("Last update", Tail(File.ReadAllText(upd), 80)); } catch { }
        Section("Memory and processes", Processes());
        Section("Windows: errors in the Application log", Run("wevtutil.exe", "qe Application /c:60 /rd:true /f:text /q:\"*[System[(Level=1 or Level=2)]]\""));

        var user = Environment.UserName;
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        return Redact(sb.ToString(), user, home);
    }

    /// <summary>Saves the report in your user folder: Hearth-logs-&lt;date&gt;.txt, and a copy on each USB drive or SD
    /// card plugged in. {path, copies}.</summary>
    public static (string Path, List<string> Copies, string Text) Save(string version, IEnumerable<(string, string)> sections)
    {
        var text = Report(version, sections);
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var name = $"Hearth-logs-{DateTime.Now:yyyy-MM-dd-HH-mm}.txt";
        var file = Path.Combine(home, name);
        try { File.WriteAllText(file, text, new UTF8Encoding(false)); }
        catch (Exception ex) { throw new InvalidOperationException("Couldn't save the logs: " + ex.Message); }
        var copies = new List<string>();
        foreach (var d in DriveInfo.GetDrives())
        {
            try
            {
                if (d.DriveType != DriveType.Removable || !d.IsReady) continue;
                var to = Path.Combine(d.RootDirectory.FullName, name);
                File.Copy(file, to, true);
                copies.Add(to);
            }
            catch { }
        }
        return (file, copies, text);
    }

    /// <summary>Saves (as above) and sends it with the person's note ("What happened?"): {path, copies, code, url}.
    /// Too-long reports lose the Windows event log first, then their oldest lines.</summary>
    public static async Task<object> SendToDeveloperAsync(string version, string note, IEnumerable<(string, string)> sections)
    {
        var (path, copies, text) = Save(version, sections);
        if (Encoding.UTF8.GetByteCount(text) > MaxSend)
        {
            int i = text.IndexOf("\n===== Windows: errors", StringComparison.Ordinal);
            if (i > 0) text = text[..i] + "\n[the Windows event log left out: too long]\n";
        }
        int over = Encoding.UTF8.GetByteCount(text) - MaxSend;
        if (over > 0)
        {
            int head = Math.Max(0, text.IndexOf("\n===== Ember log", StringComparison.Ordinal));
            int cut = Math.Min(text.Length, head + over + 64);
            text = text[..head] + "\n[… older lines left out …]\n" + text[cut..];
        }
        using var req = new HttpRequestMessage(HttpMethod.Post, Service + "/v1/logs")
        {
            Content = new StringContent(text, new UTF8Encoding(false), "text/plain"),
        };
        req.Headers.TryAddWithoutValidation("X-Hearth-Version", version);
        req.Headers.TryAddWithoutValidation("X-Hearth-Kind", "windows");
        if (!string.IsNullOrWhiteSpace(note)) req.Headers.TryAddWithoutValidation("X-Hearth-Note", HeaderNote(note));
        string? code = null, err = null;
        try
        {
            using var resp = await Stores.HelperTools.Http.SendAsync(req);
            var body = await resp.Content.ReadAsStringAsync();
            try
            {
                using var doc = JsonDocument.Parse(body);
                if (doc.RootElement.ValueKind == JsonValueKind.Object)
                {
                    if (doc.RootElement.TryGetProperty("code", out var c) && c.ValueKind == JsonValueKind.String) code = c.GetString();
                    if (doc.RootElement.TryGetProperty("error", out var e) && e.ValueKind == JsonValueKind.String) err = e.GetString();
                }
            }
            catch (JsonException) { err = $"HTTP {(int)resp.StatusCode}"; }
        }
        catch (Exception ex) { err = ex.Message; }
        SystemBridge.Log($"logs: sent to the developer: {(code is null ? "FAILED " + err : "code " + code)}");
        if (string.IsNullOrEmpty(code))
            throw new InvalidOperationException($"Saved to {path}, but sending didn't work{(err is null ? "" : $" ({err})")}. Check the internet connection and try again.");
        return new { path, copies, code, url = $"{Service}/v1/logs/{code}" };
    }
}
