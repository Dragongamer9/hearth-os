using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using HearthSteam;
using Xunit;

public class FakeEngine : IEngine
{
    public bool Ready = true, SignedIn = true;
    public string? LastPassword;
    public List<string> Calls = new();
    void Need() { if (!Ready) throw new EngineException(Codes.NotReady, "starting"); if (!SignedIn) throw new EngineException(Codes.SignedOut, "signed out"); }
    public JsonObject Status() => new() { ["engine"] = Ready ? "ready" : "starting", ["signedIn"] = SignedIn };
    public Task<JsonObject> SignInPassword(string user, string password, bool remember) { LastPassword = password; return Task.FromResult(new JsonObject { ["needs"] = "device" }); }
    public Task<JsonObject> SignInCode(string code, string kind) => Task.FromResult(new JsonObject());
    public Task<JsonObject> SignInQr() => Task.FromResult(new JsonObject { ["url"] = "https://s.team/q/1/2" });
    public void SignInCancel() => Calls.Add("cancel");
    public Task<JsonObject> SignOut() => Task.FromResult(new JsonObject());
    public JsonObject Library() { Need(); return new() { ["apps"] = new JsonArray(new JsonObject { ["appid"] = 10, ["name"] = "CS", ["installed"] = true }) }; }
    public JsonObject Install(uint appId, string? library) { Need(); Calls.Add($"install {appId} {library}"); return new(); }
    public JsonObject Uninstall(uint appId) { Need(); Calls.Add($"uninstall {appId}"); return new(); }
    public JsonObject Update(uint appId) { Need(); return new(); }
    public JsonObject Downloads() => new() { ["downloads"] = new JsonArray() };
    public Task<JsonObject> Launch(uint appId) { Need(); if (appId == 730) throw new EngineException(Codes.Vac, "VAC"); Calls.Add($"launch {appId}"); return Task.FromResult(new JsonObject()); }
    public JsonObject Running() => new() { ["apps"] = new JsonArray() };
    public void Release() => Calls.Add("release");
}

public class ServerTests
{
    static JsonObject Ask(Server s, string line) => (JsonObject)JsonNode.Parse(s.Handle(line).Result)!;

    [Fact]
    public void AnswersWithTheRequestId()
    {
        var r = Ask(new Server(new FakeEngine()), "{\"id\":7,\"cmd\":\"status\"}");
        Assert.True(r["ok"]!.GetValue<bool>());
        Assert.Equal(7, r["id"]!.GetValue<int>());
        Assert.Equal("ready", r["engine"]!.GetValue<string>());
    }

    [Fact]
    public void ErrorsCarryACode()
    {
        var e = new FakeEngine { Ready = false };
        var r = Ask(new Server(e), "{\"id\":1,\"cmd\":\"launch\",\"appid\":440}");
        Assert.False(r["ok"]!.GetValue<bool>());
        Assert.Equal(Codes.NotReady, r["code"]!.GetValue<string>());
        Assert.Equal(Codes.Vac, Ask(new Server(new FakeEngine()), "{\"cmd\":\"launch\",\"appid\":730}")["code"]!.GetValue<string>());
        Assert.Equal(Codes.SignedOut, Ask(new Server(new FakeEngine { SignedIn = false }), "{\"cmd\":\"library\"}")["code"]!.GetValue<string>());
    }

    [Fact]
    public void BadRequests()
    {
        var s = new Server(new FakeEngine());
        Assert.Equal(Codes.BadRequest, Ask(s, "not json")["code"]!.GetValue<string>());
        Assert.Equal(Codes.BadRequest, Ask(s, "{\"cmd\":\"launch\"}")["code"]!.GetValue<string>());
        Assert.Equal(Codes.BadRequest, Ask(s, "{\"cmd\":\"launch\",\"appid\":\"x\"}")["code"]!.GetValue<string>());
        Assert.Equal(Codes.Unsupported, Ask(s, "{\"cmd\":\"nope\"}")["code"]!.GetValue<string>());
        Assert.Equal(Codes.BadRequest, Ask(s, "{\"cmd\":\"" + new string('a', Server.MaxLine) + "\"}")["code"]!.GetValue<string>());
    }

    [Fact]
    public void PasswordsAreNeverLogged()
    {
        var log = new List<string>();
        var e = new FakeEngine();
        var s = new Server(e, log.Add);
        var r = Ask(s, "{\"cmd\":\"signin.password\",\"user\":\"sam\",\"password\":\"hunter2\"}");
        Assert.Equal("device", r["needs"]!.GetValue<string>());
        Assert.Equal("hunter2", e.LastPassword);
        Assert.DoesNotContain(log, l => l.Contains("hunter2") || l.Contains("sam"));
        Ask(s, "{\"cmd\":\"signin.code\",\"code\":\"AB12C\"}");
        Assert.DoesNotContain(log, l => l.Contains("AB12C"));
        var red = Server.Redacted((JsonObject)JsonNode.Parse("{\"cmd\":\"x\",\"password\":\"p\",\"code\":\"c\",\"token\":\"t\"}")!);
        Assert.DoesNotContain("\"p\"", red); Assert.DoesNotContain("\"c\"", red); Assert.DoesNotContain("\"t\"", red);
    }

    [Fact]
    public void ReleaseMarksTheServerDone()
    {
        var e = new FakeEngine();
        var s = new Server(e);
        Ask(s, "{\"cmd\":\"release\"}");
        Assert.True(s.Released);
        Assert.Contains("release", e.Calls);
    }

    [Fact]
    public async Task ServesOverAUnixSocket()
    {
        var dir = Directory.CreateTempSubdirectory("hs-test");
        var path = Path.Combine(dir.FullName, "s.sock");
        var e = new FakeEngine();
        var s = new Server(e);
        using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(20));
        var run = s.Run(path, cts.Token);
        for (int i = 0; i < 100 && !File.Exists(path); i++) await Task.Delay(50);
        Assert.Equal(UnixFileMode.UserRead | UnixFileMode.UserWrite, File.GetUnixFileMode(path));
        using var c = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
        await c.ConnectAsync(new UnixDomainSocketEndPoint(path));
        using var ns = new NetworkStream(c);
        using var w = new StreamWriter(ns, new UTF8Encoding(false)) { AutoFlush = true, NewLine = "\n" };
        using var rd = new StreamReader(ns);
        await w.WriteLineAsync("{\"id\":1,\"cmd\":\"install\",\"appid\":570,\"library\":\"/mnt/games\"}");
        var a = JsonNode.Parse((await rd.ReadLineAsync())!)!;
        Assert.True(a["ok"]!.GetValue<bool>());
        Assert.Contains("install 570 /mnt/games", e.Calls);
        await w.WriteLineAsync("{\"id\":2,\"cmd\":\"launch\",\"appid\":440}");
        Assert.True(JsonNode.Parse((await rd.ReadLineAsync())!)!["ok"]!.GetValue<bool>());
        cts.Cancel();
        await run;
        Assert.False(File.Exists(path));
    }
}

public class SteamFilesTests
{
    const string Manifest = "\"ubuntu12\"\n{\n\t\"version\"\t\t\"1745623383\"\n\t\"bins_ubuntu12\"\n\t{\n\t\t\"file\"\t\t\"bins.zip\"\n\t}\n}\n";

    [Fact]
    public void ReadsTheClientBuild() => Assert.Equal(1745623383L, SteamFiles.ManifestVersion(Manifest));

    [Fact]
    public void PicksOnlyTheExactBuild()
    {
        var root = Directory.CreateTempSubdirectory("hs-client").FullName;
        string Make(string name, long v)
        {
            var d = Path.Combine(root, name);
            Directory.CreateDirectory(Path.Combine(d, "linux64"));
            Directory.CreateDirectory(Path.Combine(d, "package"));
            File.WriteAllText(Path.Combine(d, "linux64", "steamclient.so"), "");
            File.WriteAllText(Path.Combine(d, "package", "steam_client_ubuntu12.manifest"), Manifest.Replace("1745623383", v.ToString()));
            return d;
        }
        var user = Make("user", 1790000000);
        var own = Make("own", 1745623383);
        var (dir, why) = SteamFiles.PickClient(1745623383, new[] { (user, SteamFiles.ClientVersion(user)), (own, SteamFiles.ClientVersion(own)) });
        Assert.Equal(own, dir);
        Assert.Equal("ok", why);
        var (none, why2) = SteamFiles.PickClient(1745623383, new[] { (user, SteamFiles.ClientVersion(user)) });
        Assert.Null(none);
        Assert.Contains("1790000000", why2);
        Assert.Null(SteamFiles.PickClient(1, Array.Empty<(string, long?)>()).dir);
    }

    [Fact]
    public void ReadsCompatToolChoices()
    {
        var cfg = "\"InstallConfigStore\"\n{\n\"Software\"\n{\n\"valve\"\n{\n\"Steam\"\n{\n\"CompatToolMapping\"\n{\n"
                + "\"0\"\n{\n\"name\"\t\"proton_experimental\"\n\"config\"\t\"\"\n\"priority\"\t\"75\"\n}\n"
                + "\"1091500\"\n{\n\"name\"\t\"GE-Proton9-20\"\n}\n\"440\"\n{\n\"name\"\t\"\"\n}\n}\n}\n}\n}\n}\n";
        var m = SteamFiles.CompatMapping(cfg);
        Assert.Equal("proton_experimental", m[0]);
        Assert.Equal("GE-Proton9-20", m[1091500]);
        Assert.False(m.ContainsKey(440));
    }

    [Fact]
    public void ReadsLibraries()
    {
        var d = Directory.CreateTempSubdirectory("hs-steam").FullName;
        Directory.CreateDirectory(Path.Combine(d, "steamapps"));
        File.WriteAllText(Path.Combine(d, "steamapps", "libraryfolders.vdf"),
            $"\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{d}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"/mnt/sd/SteamLibrary\"\n\t\t\"apps\" {{ \"10\" \"123\" }}\n\t}}\n}}\n");
        Assert.Equal(new[] { d, "/mnt/sd/SteamLibrary" }, SteamFiles.Libraries(d));
    }

    [Fact]
    public void KnowsTheClassicClient()
    {
        Assert.True(SteamFiles.IsClassicCmdline(Encoding.UTF8.GetBytes("/home/u/.local/share/Steam/ubuntu12_32/steam\0-silent\0")));
        Assert.False(SteamFiles.IsClassicCmdline(Encoding.UTF8.GetBytes("/usr/lib/hearth-steam/hearth-steam\0")));
        var proc = Directory.CreateTempSubdirectory("hs-proc").FullName;
        Directory.CreateDirectory(Path.Combine(proc, "4242"));
        File.WriteAllText(Path.Combine(proc, "4242", "comm"), "bash\n");
        File.WriteAllText(Path.Combine(proc, "4242", "cmdline"), "bash\0");
        Assert.False(SteamFiles.ClassicRunning(proc));
        File.WriteAllText(Path.Combine(proc, "4242", "comm"), "steam\n");
        Assert.True(SteamFiles.ClassicRunning(proc));
    }

    [Fact]
    public void VdfHandlesCommentsAndEscapes()
    {
        var n = Vdf.Parse("// c\n\"a\" { \"b\" \"x\\\"y\" \"C\" { k v } }");
        Assert.Equal("x\"y", n.Str("a", "b"));
        Assert.Equal("v", n.Str("A", "c", "K"));
    }
}
