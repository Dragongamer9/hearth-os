using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HearthSteam;

/// <summary>hearth-steam's local API: one JSON object per line on a Unix socket only this user can open
/// ($XDG_RUNTIME_DIR/hearth-steam.sock, mode 600). Request {"id":1,"cmd":"status",...}; answer
/// {"id":1,"ok":true,...} or {"id":1,"ok":false,"code":"...","error":"..."}.</summary>
public sealed class Server
{
    public const int MaxLine = 64 * 1024;
    static readonly string[] Secret = { "password", "code", "token", "refreshToken" };

    readonly IEngine engine;
    readonly Action<string> log;
    public bool Released { get; private set; }

    public Server(IEngine engine, Action<string>? log = null) { this.engine = engine; this.log = log ?? (_ => { }); }

    public static string DefaultPath()
    {
        var run = Environment.GetEnvironmentVariable("XDG_RUNTIME_DIR");
        if (string.IsNullOrEmpty(run)) run = $"/run/user/{Environment.GetEnvironmentVariable("UID") ?? "0"}";
        return Path.Combine(run, "hearth-steam.sock");
    }

    /// <summary>A request as it may be written to a log: secrets replaced. Only used for debugging; the server
    /// itself logs command names only.</summary>
    public static string Redacted(JsonObject req)
    {
        var copy = (JsonObject)req.DeepClone();
        foreach (var k in Secret) if (copy.ContainsKey(k)) copy[k] = "***";
        return copy.ToJsonString();
    }

    public async Task<string> Handle(string line)
    {
        JsonNode? id = null;
        try
        {
            if (line.Length > MaxLine) throw new EngineException(Codes.BadRequest, "Request too long.");
            var req = JsonNode.Parse(line) as JsonObject ?? throw new EngineException(Codes.BadRequest, "Not a JSON object.");
            id = req["id"]?.DeepClone();
            var cmd = req["cmd"]?.GetValue<string>() ?? throw new EngineException(Codes.BadRequest, "No cmd.");
            log($"request: {cmd}");
            var res = await Dispatch(cmd, req);
            res["ok"] = true;
            if (id != null) res["id"] = id;
            return res.ToJsonString();
        }
        catch (Exception e)
        {
            var (code, msg) = e switch
            {
                EngineException ee => (ee.Code, ee.Message),
                JsonException or InvalidOperationException or FormatException => (Codes.BadRequest, "Bad request."),
                _ => (Codes.SteamError, e.Message),
            };
            if (e is not EngineException) log($"error: {e.GetType().Name}");
            var o = new JsonObject { ["ok"] = false, ["code"] = code, ["error"] = msg };
            if (id != null) o["id"] = id;
            return o.ToJsonString();
        }
    }

    static uint App(JsonObject r) => r["appid"]?.GetValue<uint>() is uint a and > 0 ? a : throw new EngineException(Codes.BadRequest, "No appid.");
    static string Str(JsonObject r, string k) => r[k]?.GetValue<string>() ?? throw new EngineException(Codes.BadRequest, $"No {k}.");

    IOpenEngine Open() => engine as IOpenEngine ?? throw new EngineException(Codes.Unsupported, "Only the Open engine does that.");

    async Task<JsonObject> Dispatch(string cmd, JsonObject r)
    {
        switch (cmd)
        {
            case "ping": return new JsonObject { ["pong"] = true };
            case "status": return engine.Status();
            case "signin.password": return await engine.SignInPassword(Str(r, "user"), Str(r, "password"), r["remember"]?.GetValue<bool>() ?? true);
            case "signin.code": return await engine.SignInCode(Str(r, "code"), r["kind"]?.GetValue<string>() ?? "device");
            case "signin.qr": return await engine.SignInQr();
            case "signin.cancel": engine.SignInCancel(); return new JsonObject();
            case "signout": return await engine.SignOut();
            case "library": return engine.Library();
            case "install": return engine.Install(App(r), r["library"]?.GetValue<string>());
            case "uninstall": return engine.Uninstall(App(r));
            case "update": return engine.Update(App(r));
            case "downloads": return engine.Downloads();
            case "launch": return await engine.Launch(App(r));
            case "running": return engine.Running();
            case "launch.result": return Open().LaunchResult(App(r), r["ok"]?.GetValue<bool>() ?? false);
            case "verify": return Open().Verify(App(r));
            case "cancel": return Open().Cancel(App(r));
            case "release": Released = true; engine.Release(); return new JsonObject { ["released"] = true };
            default: throw new EngineException(Codes.Unsupported, $"Unknown command {cmd}.");
        }
    }

    /// <summary>Serves until cancelled or released.</summary>
    public async Task Run(string path, CancellationToken ct)
    {
        if (File.Exists(path)) File.Delete(path);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        using var sock = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
        var old = UmaskOwnerOnly();
        try { sock.Bind(new UnixDomainSocketEndPoint(path)); }
        finally { RestoreUmask(old); }
        if (!OperatingSystem.IsWindows()) File.SetUnixFileMode(path, UnixFileMode.UserRead | UnixFileMode.UserWrite);
        sock.Listen(8);
        log($"listening on {path}");
        try
        {
            while (!ct.IsCancellationRequested && !Released)
            {
                var c = await sock.AcceptAsync(ct);
                _ = Task.Run(() => Serve(c, ct), ct);
            }
        }
        catch (OperationCanceledException) { }
        finally { try { File.Delete(path); } catch (IOException) { } }
    }

    async Task Serve(Socket c, CancellationToken ct)
    {
        using var _ = c;
        using var stream = new NetworkStream(c, ownsSocket: false);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        using var writer = new StreamWriter(stream, new UTF8Encoding(false)) { AutoFlush = true, NewLine = "\n" };
        try
        {
            while (!ct.IsCancellationRequested)
            {
                var line = await reader.ReadLineAsync(ct);
                if (line == null) break;
                if (line.Length == 0) continue;
                await writer.WriteLineAsync(await Handle(line));
                if (Released) break;
            }
        }
        catch (Exception e) when (e is IOException or OperationCanceledException or SocketException) { }
    }

    [System.Runtime.InteropServices.DllImport("libc", EntryPoint = "umask")]
    static extern uint umask(uint mask);
    static uint UmaskOwnerOnly() { try { return umask(0x3F); } catch (Exception) { return 0x12; } }   // 077
    static void RestoreUmask(uint old) { try { umask(old); } catch (Exception) { } }
}
