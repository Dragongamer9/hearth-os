using System.Text.Json.Nodes;

namespace HearthSteam.Open;

/// <summary>What the Open engine's sign-in needs from Steam (SteamKit2's authentication in the service, a fake in the
/// tests). Failures Steam reports are thrown as <see cref="SignInFailure"/> with a message for the player.</summary>
public interface IAuthBackend
{
    /// <summary>Connected to Steam (sign-in needs it), and why not when it isn't.</summary>
    (bool Connected, string Why) Connection();
    /// <summary>Name and password: runs Steam's sign-in to the end. <paramref name="askCode"/> is called for a Steam
    /// Guard code ("device" or "email"; true when the last one was wrong).</summary>
    Task<AuthResult> Credentials(string user, string password, bool remember, Func<string, bool, Task<string>> askCode, CancellationToken ct);
    /// <summary>QR: <paramref name="onUrl"/> gets the code's URL, and again whenever Steam replaces it.</summary>
    Task<AuthResult> Qr(Action<string> onUrl, CancellationToken ct);
    /// <summary>Signs the session in with the token; the answer comes back through <see cref="SignInFlow.LoggedOn"/>.</summary>
    void LogOn(string account, string refreshToken);
}

public sealed record AuthResult(string AccountName, string RefreshToken);

public sealed class SignInFailure : Exception { public SignInFailure(string message) : base(message) { } }

/// <summary>The Open engine's sign-in as Hearth's sign-in screen sees it (signin.password / signin.code / signin.qr /
/// signin.cancel and Status' "signIn"). Nothing typed is kept: the password goes to the backend and is dropped; only
/// the refresh token Steam gives back is kept, in the secret store, and only when "remember" is on.</summary>
public sealed class SignInFlow
{
    readonly IAuthBackend backend;
    readonly ISecretStore secrets;
    readonly Func<string, string> qrSvg;
    readonly object gate = new();
    JsonObject state = new() { ["state"] = "idle" };
    TaskCompletionSource<JsonObject>? step;
    TaskCompletionSource<string>? code;
    CancellationTokenSource? running;
    public TimeSpan StepWait { get; init; } = TimeSpan.FromSeconds(40);
    public TimeSpan QrWait { get; init; } = TimeSpan.FromSeconds(30);
    public string? Account { get; private set; }
    public Action<string>? Log { get; init; }

    public SignInFlow(IAuthBackend backend, ISecretStore secrets, Func<string, string> qrSvg)
    { this.backend = backend; this.secrets = secrets; this.qrSvg = qrSvg; }

    public JsonObject State() { lock (gate) return (JsonObject)state.DeepClone(); }
    void Set(JsonObject s) { lock (gate) state = s; }

    void NeedConnection()
    {
        var (ok, why) = backend.Connection();
        if (!ok) throw new EngineException(Codes.NotReady, why.Length > 0 ? why : "Not connected to Steam yet. Try again in a moment.");
    }

    async Task<JsonObject> NextStep(TaskCompletionSource<JsonObject> s)
    {
        var done = await Task.WhenAny(s.Task, Task.Delay(StepWait));
        if (done != s.Task) return new JsonObject { ["needs"] = null, ["pending"] = true };
        return await s.Task;
    }

    Task<string> AskCode(string kind, bool wrong)
    {
        var c = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        lock (gate) code = c;
        Set(new JsonObject { ["state"] = "needs", ["needs"] = kind, ["wrong"] = wrong });
        step?.TrySetResult(new JsonObject { ["needs"] = kind, ["wrong"] = wrong });
        return c.Task;
    }

    void Fail(string message)
    {
        Set(new JsonObject { ["state"] = "failed", ["error"] = message });
        step?.TrySetException(new EngineException(Codes.SteamError, message));
    }

    void Run(Func<CancellationToken, Task<AuthResult>> auth, bool remember)
    {
        running?.Cancel();
        var ct = (running = new CancellationTokenSource()).Token;
        _ = Task.Run(async () =>
        {
            try
            {
                var r = await auth(ct);
                ct.ThrowIfCancellationRequested();
                Account = r.AccountName;
                if (remember) { try { secrets.Save(r.AccountName, r.RefreshToken); } catch (Exception) { Log?.Invoke("couldn't keep the sign-in (keyring and file both failed)"); } }
                Set(new JsonObject { ["state"] = "signing-in" });
                backend.LogOn(r.AccountName, r.RefreshToken);
            }
            catch (OperationCanceledException) { Set(new JsonObject { ["state"] = "idle" }); step?.TrySetException(new EngineException(Codes.SteamError, "Cancelled.")); }
            catch (SignInFailure e) { Fail(e.Message); }
            catch (Exception e) { Log?.Invoke("sign-in failed: " + e.GetType().Name); Fail("Steam didn't sign you in. Try again, or use the QR code."); }
        });
    }

    public async Task<JsonObject> Password(string user, string password, bool remember)
    {
        if (string.IsNullOrWhiteSpace(user) || string.IsNullOrEmpty(password)) throw new EngineException(Codes.BadRequest, "Type your account name and password.");
        NeedConnection();
        var s = step = new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
        Set(new JsonObject { ["state"] = "waiting" });
        Run(ct => backend.Credentials(user.Trim(), password, remember, AskCode, ct), remember);
        return await NextStep(s);
    }

    public async Task<JsonObject> Code(string value)
    {
        TaskCompletionSource<string>? c;
        lock (gate) { c = code; code = null; }
        if (c == null || c.Task.IsCompleted) throw new EngineException(Codes.BadRequest, "No sign-in is waiting for a code.");
        var s = step = new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
        Set(new JsonObject { ["state"] = "waiting" });
        c.TrySetResult(value.Trim());
        var r = await NextStep(s);
        if (r["wrong"]?.GetValue<bool>() == true) throw new EngineException(Codes.SteamError, "That code wasn't right. Try again.");
        return r;
    }

    public async Task<JsonObject> Qr()
    {
        NeedConnection();
        var s = step = new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
        var first = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        Run(ct => backend.Qr(url => { Set(new JsonObject { ["state"] = "qr", ["qrSvg"] = qrSvg(url) }); first.TrySetResult(url); }, ct), true);
        var done = await Task.WhenAny(first.Task, s.Task, Task.Delay(QrWait));
        if (done != first.Task)
        {
            if (s.Task.IsFaulted) await s.Task;
            throw new EngineException(Codes.SteamError, "Steam didn't give a QR code. Try again.");
        }
        var url = await first.Task;
        return new JsonObject { ["url"] = url, ["qrSvg"] = qrSvg(url) };
    }

    public void Cancel()
    {
        running?.Cancel();
        TaskCompletionSource<string>? c;
        lock (gate) { c = code; code = null; }
        c?.TrySetCanceled();
        Set(new JsonObject { ["state"] = "idle" });
    }

    /// <summary>Steam's answer to <see cref="IAuthBackend.LogOn"/>: null when signed in, else the message for the
    /// player (the saved sign-in is forgotten when <paramref name="forget"/>: wrong, expired or revoked).</summary>
    public void LoggedOn(string? error, bool forget = false, string? account = null)
    {
        if (error == null)
        {
            if (account != null) Account = account;
            Set(new JsonObject { ["state"] = "idle" });
            step?.TrySetResult(new JsonObject { ["needs"] = null });
            return;
        }
        if (forget && (account ?? Account) is { } name) { try { secrets.Delete(name); } catch (Exception) { } }
        Fail(error);
    }
}
