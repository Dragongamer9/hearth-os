using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using HearthSteam;
using HearthSteam.Open;
using Xunit;

// The 0.6.59 sign-in failures: the Lightweight engine's restart call couldn't be marshalled (so the engine never got
// ready and no sign-in was possible), the exit handler crashed after Main, and the Open engine's sign-in flow (name +
// password, Steam Guard codes, QR) end to end against a fake of SteamKit2's authentication.

static class BadInterop
{
    // What 0.6.59 had: .NET refuses to build this stub ("String[] must be paired with an ArraySubType of LPStr...").
    [DllImport("libc", EntryPoint = "execv")]
    public static extern int BadExecv([MarshalAs(UnmanagedType.LPUTF8Str)] string file, [MarshalAs(UnmanagedType.LPArray, ArraySubType = UnmanagedType.LPUTF8Str)] string?[] args);
}

public class InteropTests
{
    [Fact]
    public void EveryNativeCallOfTheCoreMarshals()
    {
        var (n, problems, _) = NativeExec.CheckInterop(typeof(Server).Assembly);
        Assert.True(n >= 2, $"only {n} native calls found");
        Assert.Empty(problems);
    }

    [Fact]
    public void TheCheckCatchesTheOldRestartCall()
    {
        var (_, problems, _) = NativeExec.CheckInterop(typeof(BadInterop).Assembly);
        Assert.NotEmpty(problems);
        Assert.All(problems, p => Assert.Contains("BadExecv", p));
    }

    [Fact]
    public void ArgumentListsAreUtf8CStrings()
    {
        var block = NativeExec.Alloc(new[] { "/usr/lib/hearth-steam/hearth-steam", "--engine", "lightweight", "Spiel ü ✓" });
        try
        {
            Assert.Equal(5, block.Length);
            Assert.Equal(IntPtr.Zero, block[^1]);
            Assert.Equal("Spiel ü ✓", Marshal.PtrToStringUTF8(block[3]));
            Assert.Equal("--engine", Marshal.PtrToStringUTF8(block[1]));
        }
        finally { NativeExec.Free(block); }
    }

    [Fact]
    public void TheRestartCarriesDotnetsEnvironment()
    {
        Environment.SetEnvironmentVariable("HEARTH_STEAM_TEST_REEXEC", "1");
        try
        {
            var env = NativeExec.Environ();
            Assert.Contains("HEARTH_STEAM_TEST_REEXEC=1", env);
            Assert.All(env, e => Assert.Contains('=', e));
        }
        finally { Environment.SetEnvironmentVariable("HEARTH_STEAM_TEST_REEXEC", null); }
    }

    [Fact]
    public void StoppingIsSafeAtAnyTime()
    {
        var s = new ShutdownSignal();
        Assert.False(s.Token.IsCancellationRequested);
        s.Stop();
        Assert.True(s.Token.IsCancellationRequested && s.Stopped);
        s.Stop();   // ProcessExit after Main: no exception
        var t = new Thread(() => s.Stop());
        t.Start(); t.Join();
    }
}

/// <summary>A fake of SteamKit2's authentication: scripted Steam Guard codes, QR URLs, failures, and the session
/// sign-in's answer.</summary>
public sealed class FakeAuth : IAuthBackend
{
    public bool Connected = true;
    public string Why = "";
    public string? GoodCode;            // null: no Steam Guard
    public string Kind = "device";
    public string? FailWith;            // Steam refuses the password
    public string? LogOnError;          // the session sign-in is refused
    public List<string> QrUrls = new() { "https://s.team/q/1/A", "https://s.team/q/1/B" };
    public TaskCompletionSource<bool> QrApproved = new(TaskCreationOptions.RunContinuationsAsynchronously);
    public List<string> LogOns = new();
    public string? PasswordSeen;
    public SignInFlow? Flow;

    public (bool Connected, string Why) Connection() => (Connected, Why);

    public async Task<AuthResult> Credentials(string user, string password, bool remember, Func<string, bool, Task<string>> askCode, CancellationToken ct)
    {
        PasswordSeen = password;
        await Task.Yield();
        if (FailWith != null) throw new SignInFailure(FailWith);
        if (GoodCode != null)
        {
            var wrong = false;
            while (true)
            {
                var c = await askCode(Kind, wrong).WaitAsync(ct);
                if (c == GoodCode) break;
                wrong = true;
            }
        }
        return new AuthResult(user, "refresh-" + user);
    }

    public async Task<AuthResult> Qr(Action<string> onUrl, CancellationToken ct)
    {
        onUrl(QrUrls[0]);
        for (int i = 1; i < QrUrls.Count; i++) { await Task.Delay(20, ct); onUrl(QrUrls[i]); }
        await QrApproved.Task.WaitAsync(ct);
        return new AuthResult("qruser", "refresh-qr");
    }

    public void LogOn(string account, string refreshToken)
    {
        LogOns.Add(account);
        // SteamUser.LoggedOnCallback, as the engine passes it on.
        Task.Run(() => Flow!.LoggedOn(LogOnError, LogOnError != null, account));
    }
}

public sealed class MemorySecrets : ISecretStore
{
    public Dictionary<string, string> Saved = new();
    public string Kind => "memory";
    public string? Load(string account) => Saved.TryGetValue(account, out var t) ? t : null;
    public void Save(string account, string token) => Saved[account] = token;
    public void Delete(string account) => Saved.Remove(account);
}

public class SignInTests
{
    static (SignInFlow flow, FakeAuth auth, MemorySecrets secrets) Make(Action<FakeAuth>? setup = null)
    {
        var auth = new FakeAuth();
        setup?.Invoke(auth);
        var secrets = new MemorySecrets();
        var flow = new SignInFlow(auth, secrets, url => "<svg>" + url + "</svg>") { StepWait = TimeSpan.FromSeconds(10), QrWait = TimeSpan.FromSeconds(5) };
        auth.Flow = flow;
        return (flow, auth, secrets);
    }

    static async Task Until(Func<bool> ok)
    {
        for (int i = 0; i < 200 && !ok(); i++) await Task.Delay(10);
        Assert.True(ok());
    }

    [Fact]
    public async Task PasswordWithoutSteamGuard()
    {
        var (flow, auth, secrets) = Make();
        var r = await flow.Password(" gabe ", "hunter2", true);
        Assert.Null(r["needs"]);
        Assert.Equal(new[] { "gabe" }, auth.LogOns);
        Assert.Equal("refresh-gabe", secrets.Saved["gabe"]);
        Assert.Equal("idle", flow.State()["state"]!.GetValue<string>());
        Assert.Equal("gabe", flow.Account);
        Assert.DoesNotContain("hunter2", flow.State().ToJsonString());
    }

    [Fact]
    public async Task SteamGuardCodeFromTheApp()
    {
        var (flow, auth, secrets) = Make(a => a.GoodCode = "F7K2Q");
        var r = await flow.Password("gabe", "pw", true);
        Assert.Equal("device", r["needs"]!.GetValue<string>());
        Assert.Equal("needs", flow.State()["state"]!.GetValue<string>());
        // A wrong code: told so, and another code can be typed.
        var e = await Assert.ThrowsAsync<EngineException>(() => flow.Code("AAAAA"));
        Assert.Equal(Codes.SteamError, e.Code);
        Assert.True(flow.State()["wrong"]!.GetValue<bool>());
        var ok = await flow.Code(" F7K2Q ");
        Assert.Null(ok["needs"]);
        Assert.Single(auth.LogOns);
        Assert.True(secrets.Saved.ContainsKey("gabe"));
    }

    [Fact]
    public async Task EmailCodeAndNotRemembered()
    {
        var (flow, _, secrets) = Make(a => { a.GoodCode = "12345"; a.Kind = "email"; });
        var r = await flow.Password("gabe", "pw", false);
        Assert.Equal("email", r["needs"]!.GetValue<string>());
        await flow.Code("12345");
        Assert.Empty(secrets.Saved);   // "remember" off: no token kept
    }

    [Fact]
    public async Task WrongPasswordIsAClearError()
    {
        var (flow, auth, _) = Make(a => a.FailWith = "Wrong account name or password.");
        var e = await Assert.ThrowsAsync<EngineException>(() => flow.Password("gabe", "nope", true));
        Assert.Equal("Wrong account name or password.", e.Message);
        Assert.Equal("failed", flow.State()["state"]!.GetValue<string>());
        Assert.Empty(auth.LogOns);
    }

    [Fact]
    public async Task NotConnectedSaysWhyInsteadOfNothing()
    {
        var (flow, _, _) = Make(a => { a.Connected = false; a.Why = "Steam's engine didn't start: something. Sign-in isn't possible right now."; });
        var e = await Assert.ThrowsAsync<EngineException>(() => flow.Password("gabe", "pw", true));
        Assert.Equal(Codes.NotReady, e.Code);
        Assert.Contains("didn't start", e.Message);
        var q = await Assert.ThrowsAsync<EngineException>(() => flow.Qr());
        Assert.Equal(Codes.NotReady, q.Code);
        var (flow2, _, _) = Make(a => a.Connected = false);
        Assert.Contains("Try again", (await Assert.ThrowsAsync<EngineException>(() => flow2.Qr())).Message);
        await Assert.ThrowsAsync<EngineException>(() => flow2.Password("", "", true));
    }

    [Fact]
    public async Task QrSignIn()
    {
        var (flow, auth, secrets) = Make();
        var r = await flow.Qr();
        Assert.Equal("https://s.team/q/1/A", r["url"]!.GetValue<string>());
        Assert.Contains("q/1/A", r["qrSvg"]!.GetValue<string>());
        // Steam replaces the code: the status carries the new one.
        await Until(() => flow.State()["qrSvg"]?.GetValue<string>().Contains("q/1/B") == true);
        auth.QrApproved.SetResult(true);
        await Until(() => auth.LogOns.Count == 1 && flow.State()["state"]!.GetValue<string>() == "idle");
        Assert.Equal("refresh-qr", secrets.Saved["qruser"]);
    }

    [Fact]
    public async Task CancelLeavesNothingWaiting()
    {
        var (flow, auth, _) = Make(a => a.GoodCode = "1");
        await flow.Password("gabe", "pw", true);
        flow.Cancel();
        Assert.Equal("idle", flow.State()["state"]!.GetValue<string>());
        var e = await Assert.ThrowsAsync<EngineException>(() => flow.Code("1"));
        Assert.Equal(Codes.BadRequest, e.Code);
        Assert.Empty(auth.LogOns);
    }

    [Fact]
    public async Task RefusedSessionForgetsTheSavedSignIn()
    {
        var (flow, _, secrets) = Make(a => a.LogOnError = "That sign-in has run out. Sign in again.");
        var e = await Assert.ThrowsAsync<EngineException>(() => flow.Password("gabe", "pw", true));
        Assert.Contains("run out", e.Message);
        Assert.Empty(secrets.Saved);
        Assert.Equal("failed", flow.State()["state"]!.GetValue<string>());
    }

    [Fact]
    public void SignInGoesThroughTheSocketWithoutEchoingSecrets()
    {
        var logged = new List<string>();
        var s = new Server(new FakeEngine(), logged.Add);
        var r = (JsonObject)JsonNode.Parse(s.Handle("{\"cmd\":\"signin.password\",\"user\":\"gabe\",\"password\":\"hunter2\"}").Result)!;
        Assert.True(r["ok"]!.GetValue<bool>());
        Assert.DoesNotContain(logged, l => l.Contains("hunter2"));
        Assert.DoesNotContain("hunter2", r.ToJsonString());
    }
}
