using System.Diagnostics;
using System.Text.Json.Nodes;

namespace HearthSteam.Open;

/// <summary>Where the Open engine keeps the one secret it has: the Steam refresh token from signing in (so it can sign in
/// again without the password). In the user's keyring (Secret Service, through secret-tool) when there is one,
/// otherwise in a file only the user can read (0600, in a 0700 folder). Never logged, never in an answer.</summary>
public interface ISecretStore
{
    string? Load(string account);
    void Save(string account, string token);
    void Delete(string account);
    string Kind { get; }
}

public static class Secrets
{
    public static void EnsurePrivateDir(string dir)
    {
        Directory.CreateDirectory(dir);
        if (!OperatingSystem.IsWindows()) File.SetUnixFileMode(dir, UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute);
    }

    /// <summary>The keyring when secret-tool works, else the private file.</summary>
    public static ISecretStore Pick(string stateDir)
    {
        var file = new FileSecretStore(Path.Combine(stateDir, "open-token.json"));
        var kr = KeyringSecretStore.Find();
        return kr != null ? new FallbackStore(kr, file) : file;
    }

    /// <summary>The keyring first; when it fails (locked, no Secret Service running), the file.</summary>
    sealed class FallbackStore : ISecretStore
    {
        readonly ISecretStore a, b;
        public FallbackStore(ISecretStore a, ISecretStore b) { this.a = a; this.b = b; }
        public string Kind => a.Kind + "+" + b.Kind;
        public string? Load(string account) { try { return a.Load(account) ?? b.Load(account); } catch (Exception) { return b.Load(account); } }
        public void Save(string account, string token)
        {
            try { a.Save(account, token); b.Delete(account); }
            catch (Exception) { b.Save(account, token); }
        }
        public void Delete(string account) { try { a.Delete(account); } catch (Exception) { } b.Delete(account); }
    }
}

public sealed class FileSecretStore : ISecretStore
{
    readonly string path;
    public FileSecretStore(string path) { this.path = path; }
    public string Kind => "file";

    JsonObject Read()
    {
        try { return File.Exists(path) ? JsonNode.Parse(File.ReadAllText(path)) as JsonObject ?? new() : new(); }
        catch (Exception e) when (e is IOException or System.Text.Json.JsonException) { return new(); }
    }

    void Write(JsonObject o)
    {
        Secrets.EnsurePrivateDir(Path.GetDirectoryName(path)!);
        if (o.Count == 0) { if (File.Exists(path)) File.Delete(path); return; }
        var tmp = path + ".tmp";
        // Created 0600 before anything is written to it.
        using (var fs = new FileStream(tmp, new FileStreamOptions { Mode = FileMode.Create, Access = FileAccess.Write, UnixCreateMode = OperatingSystem.IsWindows() ? null : UnixFileMode.UserRead | UnixFileMode.UserWrite }))
        using (var w = new StreamWriter(fs)) w.Write(o.ToJsonString());
        if (!OperatingSystem.IsWindows()) File.SetUnixFileMode(tmp, UnixFileMode.UserRead | UnixFileMode.UserWrite);
        File.Move(tmp, path, true);
    }

    public string? Load(string account) => Read()[account]?.GetValue<string>();
    public void Save(string account, string token) { var o = Read(); o[account] = token; Write(o); }
    public void Delete(string account) { var o = Read(); if (o.Remove(account)) Write(o); }
}

/// <summary>The user's keyring through secret-tool (libsecret). The token goes over stdin, never on a command line.</summary>
public sealed class KeyringSecretStore : ISecretStore
{
    readonly string tool;
    KeyringSecretStore(string tool) { this.tool = tool; }
    public string Kind => "keyring";

    public static KeyringSecretStore? Find()
    {
        if (string.IsNullOrEmpty(Environment.GetEnvironmentVariable("DBUS_SESSION_BUS_ADDRESS")) && string.IsNullOrEmpty(Environment.GetEnvironmentVariable("XDG_RUNTIME_DIR"))) return null;
        foreach (var d in new[] { "/usr/bin", "/usr/local/bin", "/bin" })
            if (File.Exists(Path.Combine(d, "secret-tool"))) return new KeyringSecretStore(Path.Combine(d, "secret-tool"));
        return null;
    }

    (int code, string output) Run(string? stdin, params string[] args)
    {
        var psi = new ProcessStartInfo(tool) { RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true, UseShellExecute = false };
        foreach (var a in args) psi.ArgumentList.Add(a);
        using var p = Process.Start(psi) ?? throw new IOException("secret-tool didn't start");
        if (stdin != null) p.StandardInput.Write(stdin);
        p.StandardInput.Close();
        var output = p.StandardOutput.ReadToEnd();
        if (!p.WaitForExit(10_000)) { try { p.Kill(); } catch (Exception) { } throw new IOException("secret-tool timed out"); }
        return (p.ExitCode, output);
    }

    public string? Load(string account)
    {
        var (code, output) = Run(null, "lookup", "service", "hearth-steam", "account", account);
        return code == 0 && output.Length > 0 ? output.TrimEnd('\n') : null;
    }

    public void Save(string account, string token)
    {
        var (code, _) = Run(token, "store", "--label=Hearth Steam sign-in", "service", "hearth-steam", "account", account);
        if (code != 0) throw new IOException("secret-tool store failed");
    }

    public void Delete(string account) => Run(null, "clear", "service", "hearth-steam", "account", account);
}
