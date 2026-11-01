using System.Text.Json.Nodes;
using HearthSteam.Open;
using QRCoder;
using SteamKit2;
using SteamKit2.Authentication;
using SteamKit2.Discovery;

namespace HearthSteam;

/// <summary>The Open engine (experimental): Hearth's own Steam client, clean-room, on SteamKit2 (LGPL-2.1, the
/// community's documented Steam protocol library; see LICENSE.SteamKit2). It signs in, reads the owned library and app
/// info, and installs, updates and verifies games with depot downloads into the user's own Steam libraries, writing
/// normal appmanifest_*.acf records. It never loads steamclient, never stands in for steam_api: Steamworks games
/// start through Valve's own client.</summary>
public sealed class OpenEngine : IOpenEngine, IDisposable
{
    const uint LoginId = 0x48537470;   // this engine's own session id, so it never replaces another Steam session

    readonly string home, stateDir;
    readonly object gate = new();
    readonly CancellationTokenSource cts = new();
    readonly SteamClient client;
    readonly CallbackManager callbacks;
    readonly SteamUser user;
    readonly SteamApps apps;
    readonly SteamContent content;
    readonly SteamKit2.CDN.Client cdn;
    readonly ISecretStore secrets;
    readonly StateFile store;
    string state = "starting", reason = "";
    bool connected, signedIn, released;
    /// <summary>Valve's client is running: the Open engine stays signed in (sign-in and the library still work) but
    /// writes nothing into the libraries; its downloads wait and carry on once Valve's client has closed.</summary>
    bool standby;
    string? account;
    ulong steamId;
    readonly SignInFlow flow;
    readonly CloudSync cloudSync;
    readonly SteamWorkshopApi workshop;
    static readonly HttpClient Http = new() { Timeout = TimeSpan.FromMinutes(30) };
    DateTime lastWorkshopCheck = DateTime.MinValue;
    List<SteamApps.LicenseListCallback.License> licenses = new();
    readonly Dictionary<uint, AppInfo> owned = new();
    HashSet<uint> ownedApps = new();
    bool libraryLoading;
    readonly DownloadQueue dl = new();
    CancellationTokenSource? runCts;
    readonly RateLimiter limiter = new();
    string autoPolicy = AutoUpdate.Idle;
    bool gameRunning;
    DateTime lastUpdateCheck = DateTime.MinValue;
    Timer? yielder;
    Timer? watchdog;
    int reconnectDelay = 5;
    // Steam's "TryAnotherCM" (that server can't sign you in): the servers that said it, and whether the next
    // connection must go to another one (switching between TCP and WebSocket each time).
    readonly HashSet<string> badCms = new();
    bool otherCm;
    ProtocolTypes lastProtocol = ProtocolTypes.Tcp;
    public event Action? Released;

    public OpenEngine(string home)
    {
        this.home = home;
        var st = Environment.GetEnvironmentVariable("XDG_STATE_HOME");
        stateDir = Path.Combine(string.IsNullOrEmpty(st) ? Path.Combine(home, ".local/state") : st, "hearth-steam");
        Secrets.EnsurePrivateDir(stateDir);
        secrets = Secrets.Pick(stateDir);
        store = new StateFile(Path.Combine(stateDir, "open-state.json"));
        client = new SteamClient(SteamConfiguration.Create(b => b.WithConnectionTimeout(TimeSpan.FromSeconds(15))
            .WithProtocolTypes(ProtocolTypes.Tcp | ProtocolTypes.WebSocket)));
        callbacks = new CallbackManager(client);
        user = client.GetHandler<SteamUser>()!;
        apps = client.GetHandler<SteamApps>()!;
        content = client.GetHandler<SteamContent>()!;
        cdn = new SteamKit2.CDN.Client(client);
        callbacks.Subscribe<SteamClient.ConnectedCallback>(_ => OnConnected());
        callbacks.Subscribe<SteamClient.DisconnectedCallback>(OnDisconnected);
        callbacks.Subscribe<SteamUser.LoggedOnCallback>(OnLoggedOn);
        callbacks.Subscribe<SteamUser.LoggedOffCallback>(e => { Log($"signed off by Steam ({e.Result})"); lock (gate) signedIn = false; });
        callbacks.Subscribe<SteamApps.LicenseListCallback>(OnLicenses);
        flow = new SignInFlow(new SteamAuth(this), secrets, Svg) { Log = Log };
        var data = Environment.GetEnvironmentVariable("XDG_DATA_HOME");
        var copies = Path.Combine(string.IsNullOrEmpty(data) ? Path.Combine(home, ".local/share") : data, "Hearth", "Steam Cloud copies");
        cloudSync = new CloudSync(new SteamCloudApi(client), stateDir, copies, Log);
        workshop = new SteamWorkshopApi(client);
        autoPolicy = AutoUpdate.Normal(store.Data["autoUpdate"]?.GetValue<string>());
        limiter.BytesPerSecond = (store.Data["limitKBps"]?.GetValue<long>() ?? 0) * 1024;
    }

    static void Log(string s) => Console.Error.WriteLine("hearth-steam (open): " + s);

    public void Start()
    {
        if (SteamFiles.ClassicRunning()) EnterStandby("the normal Steam client is running");
        new Thread(() => { while (!cts.IsCancellationRequested) callbacks.RunWaitCallbacks(TimeSpan.FromSeconds(1)); }) { IsBackground = true, Name = "steamkit-callbacks" }.Start();
        client.Connect();
        // Valve's client started (a game that needs it): stop writing into the libraries until it has closed again.
        watchdog = new Timer(_ =>
        {
            bool classic = SteamFiles.ClassicRunning(), was;
            lock (gate) was = standby;
            if (classic && !was) EnterStandby("the normal Steam client started");
            else if (!classic && was) LeaveStandby();
        }, null, 5000, 5000);
        // A paused, moved-down or cancelled running job (or standby, or a game starting during an automatic update)
        // stops within a second; what it downloaded is kept.
        yielder = new Timer(_ => { if (dl.ShouldYield(Hold(), Volatile.Read(ref gameRunning), AutoUpdate.DuringGame(autoPolicy))) StopRun(); }, null, 1000, 1000);
        _ = Task.Run(Worker);
        _ = Task.Run(AutoUpdates);
    }

    bool Hold() { lock (gate) return !signedIn || standby; }

    /// <summary>Stops the running job (it may be finishing at the same moment: never throws).</summary>
    void StopRun() { try { runCts?.Cancel(); } catch (ObjectDisposedException) { } }

    // ---------------------------------------------------------------- connection and sign-in

    void OnConnected()
    {
        lock (gate) { connected = true; state = "ready"; reason = ""; reconnectDelay = 5; }
        Log($"connected to Steam ({client.CurrentEndPoint}, {lastProtocol})");
        var last = account ?? store.Data["account"]?.GetValue<string>();   // the account just signed in to comes first
        string? token = null;
        if (last != null) { try { token = secrets.Load(last); } catch (Exception) { } }
        if (last != null && token != null) { account = last; LogOn(last, token); }
    }

    void LogOn(string name, string refreshToken) => user.LogOn(new SteamUser.LogOnDetails
    {
        Username = name, AccessToken = refreshToken, ShouldRememberPassword = true, LoginID = LoginId, MachineName = "Hearth OS",
    });

    void OnDisconnected(SteamClient.DisconnectedCallback e)
    {
        lock (gate) { connected = false; signedIn = false; if (state == "ready") { state = "starting"; reason = "Reconnecting to Steam"; } }
        if (released || cts.IsCancellationRequested || e.UserInitiated) return;
        bool other; lock (gate) { other = otherCm; otherCm = false; }
        if (other)
        {
            // Straight to another server (Steam's own list), not the same one again.
            _ = Task.Run(async () =>
            {
                var next = await NextCm();
                if (cts.IsCancellationRequested || released) return;
                if (next != null) lastProtocol = next.ProtocolTypes;
                Log(next != null ? $"trying another Steam server: {next.EndPoint} ({next.ProtocolTypes})" : "trying again (no other Steam server known)");
                client.Connect(next);
            });
            return;
        }
        var d = reconnectDelay; reconnectDelay = Math.Min(reconnectDelay * 2, 300);
        Log($"disconnected; trying again in {d}s");
        Task.Delay(TimeSpan.FromSeconds(d), cts.Token).ContinueWith(t => { if (!t.IsCanceled) client.Connect(); });
    }

    void OnLoggedOn(SteamUser.LoggedOnCallback e)
    {
        if (e.Result != EResult.OK)
        {
            Log($"sign-in refused ({e.Result}) by {client.CurrentEndPoint}");
            if (e.Result is EResult.TryAnotherCM or EResult.ServiceUnavailable)
            {
                // Not about your account: that server can't sign anyone in right now. Another one is tried next.
                var ep = client.CurrentEndPoint?.ToString();
                lock (gate) { if (ep != null) badCms.Add(ep); otherCm = true; }
                return;
            }
            bool forget = e.Result is EResult.InvalidPassword or EResult.AccessDenied or EResult.Expired or EResult.Revoked or EResult.InvalidSignature;
            lock (gate) signedIn = false;
            flow.LoggedOn(SignInError(e.Result), forget, store.Data["account"]?.GetValue<string>() ?? account);
            return;
        }
        lock (gate)
        {
            signedIn = true;
            steamId = e.ClientSteamID?.ConvertToUInt64() ?? 0;
        }
        store.Update(d => { d["account"] = account; d["steamId"] = steamId; });
        Log("signed in");
        flow.LoggedOn(null, false, account);
        _ = Task.Run(ResumeQueue);
    }

    /// <summary>A Steam server that hasn't said TryAnotherCM, using the other connection kind (TCP or WebSocket) than
    /// the last one when it can; null when Steam's list can't be had (SteamKit2 then picks one itself).</summary>
    async Task<ServerRecord?> NextCm()
    {
        try
        {
            var all = await SteamDirectory.LoadAsync(client.Configuration, cts.Token);
            string[] bad; lock (gate) bad = badCms.ToArray();
            var fresh = all.Where(r => !bad.Contains(r.EndPoint.ToString())).ToList();
            if (fresh.Count == 0) { lock (gate) badCms.Clear(); fresh = all.ToList(); }
            var want = lastProtocol.HasFlag(ProtocolTypes.WebSocket) ? ProtocolTypes.Tcp : ProtocolTypes.WebSocket;
            return fresh.FirstOrDefault(r => r.ProtocolTypes.HasFlag(want)) ?? fresh.FirstOrDefault();
        }
        catch (Exception x) when (x is not OperationCanceledException) { Log($"Steam's server list: {x.Message}"); return null; }
    }

    static string SignInError(EResult r) => r switch
    {
        EResult.InvalidPassword => "Wrong account name or password.",
        EResult.RateLimitExceeded or EResult.AccountLoginDeniedThrottle => "Too many tries. Wait a while, then try again.",
        EResult.Expired or EResult.AccessDenied or EResult.Revoked => "That sign-in has run out. Sign in again.",
        EResult.TwoFactorCodeMismatch or EResult.InvalidLoginAuthCode => "That code wasn't right.",
        _ => $"Steam didn't sign you in ({r}).",
    };

    /// <summary>SteamKit2's authentication for the sign-in flow (SignInFlow in HearthSteam.Core, tested with a fake).</summary>
    sealed class SteamAuth : IAuthBackend
    {
        readonly OpenEngine e;
        public SteamAuth(OpenEngine e) { this.e = e; }

        public (bool Connected, string Why) Connection()
        {
            lock (e.gate)
            {
                if (e.connected) return (true, "");
                if (e.state == "unavailable") return (false, e.reason.Length > 0 ? e.reason + " Sign-in isn't possible right now." : "Steam isn't reachable right now.");
                return (false, e.reason.Length > 0 ? e.reason + ". Try again in a moment." : "Still connecting to Steam. Try again in a moment.");
            }
        }

        /// <summary>Hands Steam Guard's questions to Hearth's sign-in screen: the next signin.code answers them.</summary>
        sealed class Guard : IAuthenticator
        {
            readonly Func<string, bool, Task<string>> ask;
            public Guard(Func<string, bool, Task<string>> ask) { this.ask = ask; }
            public Task<string> GetDeviceCodeAsync(bool previousCodeWasIncorrect) => ask("device", previousCodeWasIncorrect);
            public Task<string> GetEmailCodeAsync(string email, bool previousCodeWasIncorrect) => ask("email", previousCodeWasIncorrect);
            // Typing the code works with a controller (Hearth's keyboard); approving in the app is the QR sign-in.
            public Task<bool> AcceptDeviceConfirmationAsync() => Task.FromResult(false);
        }

        public async Task<AuthResult> Credentials(string user, string password, bool remember, Func<string, bool, Task<string>> askCode, CancellationToken ct)
        {
            try
            {
                // The password goes straight into SteamKit2's request (encrypted with Steam's key) and is dropped with it.
                var session = await e.client.Authentication.BeginAuthSessionViaCredentialsAsync(new AuthSessionDetails
                {
                    Username = user, Password = password, IsPersistentSession = remember, Authenticator = new Guard(askCode), DeviceFriendlyName = "Hearth OS",
                });
                var r = await session.PollingWaitForResultAsync(ct);
                return new AuthResult(r.AccountName, r.RefreshToken);
            }
            catch (AuthenticationException x) { throw new SignInFailure(SignInError(x.Result)); }
            catch (InvalidOperationException x) when (x is not ObjectDisposedException) { throw new SignInFailure("Steam wants this sign-in approved in the Steam app: use the QR code instead."); }
        }

        public async Task<AuthResult> Qr(Action<string> onUrl, CancellationToken ct)
        {
            try
            {
                var s = await e.client.Authentication.BeginAuthSessionViaQRAsync(new AuthSessionDetails { IsPersistentSession = true, DeviceFriendlyName = "Hearth OS" });
                s.ChallengeURLChanged = () => onUrl(s.ChallengeURL);
                onUrl(s.ChallengeURL);
                var r = await s.PollingWaitForResultAsync(ct);
                return new AuthResult(r.AccountName, r.RefreshToken);
            }
            catch (AuthenticationException x) { throw new SignInFailure(SignInError(x.Result)); }
        }

        public void LogOn(string account, string refreshToken) { e.account = account; e.LogOn(account, refreshToken); }
    }

    public Task<JsonObject> SignInPassword(string userName, string password, bool remember) => flow.Password(userName, password, remember);
    public Task<JsonObject> SignInCode(string code, string kind) => flow.Code(code);
    public Task<JsonObject> SignInQr() => flow.Qr();
    public void SignInCancel() => flow.Cancel();

    static string Svg(string url)
    {
        using var gen = new QRCodeGenerator();
        return new SvgQRCode(gen.CreateQrCode(url, QRCodeGenerator.ECCLevel.M)).GetGraphic(4);
    }

    public Task<JsonObject> SignOut()
    {
        var name = store.Data["account"]?.GetValue<string>();
        if (name != null) { try { secrets.Delete(name); } catch (Exception) { } }
        store.Update(d => { d.Remove("account"); d.Remove("steamId"); });
        StopRun();
        try { user.LogOff(); } catch (Exception) { }
        lock (gate) { signedIn = false; account = null; steamId = 0; owned.Clear(); }
        return Task.FromResult(new JsonObject());
    }

    public JsonObject Status()
    {
        lock (gate)
        {
            var o = new JsonObject
            {
                ["kind"] = "open", ["engine"] = state, ["reason"] = reason, ["signedIn"] = signedIn,
                ["account"] = signedIn ? account ?? store.Data["account"]?.GetValue<string>() : null,
                ["steamId"] = signedIn ? steamId.ToString() : null,
                ["signIn"] = flow.State(), ["secretStore"] = secrets.Kind, ["libraryApps"] = owned.Count,
                ["libraryLoading"] = libraryLoading, ["classicRunning"] = standby, ["standby"] = standby,
            };
            if (state == "unavailable") o["code"] = Codes.ClassicRunning;
            return o;
        }
    }

    void NeedSignIn()
    {
        lock (gate)
        {
            if (state != "ready") throw new EngineException(Codes.NotReady, reason.Length > 0 ? reason : "Not ready.");
            if (!signedIn) throw new EngineException(Codes.SignedOut, "Not signed in to Steam.");
        }
    }

    // ---------------------------------------------------------------- library

    void OnLicenses(SteamApps.LicenseListCallback e)
    {
        if (e.Result != EResult.OK) return;
        licenses = e.LicenseList.ToList();
        _ = Task.Run(LoadLibrary);
    }

    static VdfNode Conv(KeyValue kv)
    {
        var n = new VdfNode { Value = kv.Children.Count == 0 ? kv.Value : null };
        foreach (var c in kv.Children) n.Children[c.Name ?? ""] = Conv(c);
        return n;
    }

    async Task LoadLibrary()
    {
        lock (gate) libraryLoading = true;
        try
        {
            var pkgs = licenses.Select(l => new SteamApps.PICSRequest(l.PackageID, l.AccessToken)).ToList();
            var appIds = new HashSet<uint>();
            foreach (var batch in pkgs.Chunk(200))
            {
                var r = await apps.PICSGetProductInfo(Enumerable.Empty<SteamApps.PICSRequest>(), batch);
                foreach (var cb in r.Results ?? new List<SteamApps.PICSProductInfoCallback>().AsReadOnly())
                    foreach (var (_, p) in cb.Packages) foreach (var a in PackageInfo.Apps(Conv(p.KeyValues))) appIds.Add(a);
            }
            lock (gate) ownedApps = appIds;
            var infos = await AppInfos(appIds);
            lock (gate) { owned.Clear(); foreach (var (id, i) in infos) if (i.IsGame) owned[id] = i; }
            Log($"library: {owned.Count} games from {licenses.Count} licences");
        }
        catch (Exception e) { Log("library: " + e.GetType().Name + ": " + e.Message); }
        finally { lock (gate) libraryLoading = false; }
    }

    async Task<Dictionary<uint, AppInfo>> AppInfos(IEnumerable<uint> ids, string branch = "public")
    {
        var result = new Dictionary<uint, AppInfo>();
        foreach (var batch in ids.Chunk(200))
        {
            var tokens = await apps.PICSGetAccessTokens(batch, Enumerable.Empty<uint>());
            var reqs = batch.Select(a => new SteamApps.PICSRequest(a, tokens.AppTokens.TryGetValue(a, out var t) ? t : 0)).ToList();
            var r = await apps.PICSGetProductInfo(reqs, Enumerable.Empty<SteamApps.PICSRequest>());
            foreach (var cb in r.Results ?? new List<SteamApps.PICSProductInfoCallback>().AsReadOnly())
                foreach (var (id, i) in cb.Apps) result[id] = AppInfo.Parse(id, Conv(i.KeyValues), branch);
        }
        return result;
    }

    async Task<AppInfo?> FreshInfo(uint appId, string branch = "public")
    {
        var r = await AppInfos(new[] { appId }, branch);
        if (r.TryGetValue(appId, out var i)) { if (branch == "public") lock (gate) if (i.IsGame) owned[appId] = i; return i; }
        return null;
    }

    /// <summary>The app info for a branch ("" the default), with a password-protected beta's manifests unlocked when
    /// its key is known.</summary>
    async Task<AppInfo?> InfoFor(uint appId, string branch)
    {
        var info = await FreshInfo(appId, branch.Length == 0 ? "public" : branch);
        return info == null ? null : Betas.WithKey(info, BetaKey(appId, branch));
    }

    byte[]? BetaKey(uint appId, string branch)
    {
        if (branch.Length == 0 || branch == "public") return null;
        try { return secrets.Load($"beta:{appId}:{branch}") is { Length: > 0 } hex ? Convert.FromHexString(hex) : null; }
        catch (Exception) { return null; }
    }

    /// <summary>The branch a game is on (or was picked for it): Hearth's choice, else the installed record's.</summary>
    string BranchOf(uint appId) => store.Data["betas"]?[appId.ToString()]?.GetValue<string>() ?? Installed(appId)?.acf.BetaKey ?? "";

    List<string> Libraries() => SteamFiles.UserSteam(home) is { } s ? SteamFiles.Libraries(s) : new List<string>();

    (string lib, AppManifest acf)? Installed(uint appId)
    {
        foreach (var l in Libraries()) if (AppManifest.Load(l, appId) is { } m) return (l, m);
        return null;
    }

    public JsonObject Library()
    {
        NeedSignIn();
        var list = new JsonArray();
        List<AppInfo> games;
        lock (gate) games = owned.Values.ToList();
        foreach (var g in games.OrderBy(g => g.Name, StringComparer.OrdinalIgnoreCase))
        {
            var inst = Installed(g.AppId);
            list.Add(new JsonObject
            {
                ["appid"] = g.AppId, ["name"] = g.Name, ["type"] = g.Type, ["installdir"] = g.InstallDir, ["vac"] = g.Vac,
                ["installed"] = inst?.acf.Installed == true, ["buildid"] = inst?.acf.BuildId, ["latest"] = g.BuildId,
                ["update"] = inst?.acf.Installed == true && g.BuildId != 0 && inst.Value.acf.BuildId != g.BuildId,
                ["library"] = inst?.lib,
            });
        }
        return new JsonObject { ["apps"] = list, ["loading"] = libraryLoading, ["steamId"] = steamId.ToString() };
    }

    // ---------------------------------------------------------------- installing, updating, verifying

    void SaveQueue() => store.Update(d => d["queue"] = dl.Save());

    JsonObject Enqueue(uint appId, string? library, string kind, string branch = "")
    {
        NeedSignIn();
        if (SteamFiles.ClassicRunning()) throw new EngineException(Codes.ClassicRunning, "The normal Steam client is running.");
        if (branch.Length == 0) branch = BranchOf(appId);
        AppInfo? info;
        try { info = InfoFor(appId, branch).WaitAsync(TimeSpan.FromSeconds(12)).GetAwaiter().GetResult(); }
        catch (TimeoutException) { throw new EngineException(Codes.Busy, "Steam didn't answer in time."); }
        bool ownsApp(uint a) { lock (gate) return ownedApps.Contains(a); }
        var os = info?.ChooseOs(UserPickedProton(appId)) ?? "windows";
        var picked = info?.PickDepots(os, "english", ownsApp) ?? new List<DepotInfo>();
        if (OpenRules.CanInstall(info, picked, false, true) is { } no) throw new EngineException(no.code, no.why);
        var libs = Libraries();
        var lib = OpenRules.PickLibrary(libs, l => File.Exists(AppManifest.Path(l, appId)), library);
        dl.Add(new QueueItem { AppId = appId, Library = lib, Kind = kind, Name = info!.Name, Branch = branch });
        SaveQueue();
        return new JsonObject { ["queued"] = true, ["library"] = lib, ["os"] = os };
    }

    public JsonObject Install(uint appId, string? library) => Enqueue(appId, library, "install");
    public JsonObject Update(uint appId) => Enqueue(appId, null, "update");
    public JsonObject Verify(uint appId) => Enqueue(appId, null, "verify");

    public JsonObject Cancel(uint appId)
    {
        bool running = dl.Current?.AppId == appId;
        var had = dl.Cancel(appId);
        if (running) StopRun();
        SaveQueue();
        return new JsonObject { ["cancelled"] = had };
    }

    public JsonObject Pause(uint? appId)
    {
        int n = appId is uint a ? (dl.Pause(a) ? 1 : 0) : dl.PauseAll();
        SaveQueue();
        return new JsonObject { ["paused"] = n };
    }

    public JsonObject Resume(uint? appId)
    {
        int n = appId is uint a ? (dl.Resume(a) ? 1 : 0) : dl.ResumeAll();
        SaveQueue();
        return new JsonObject { ["resumed"] = n };
    }

    public JsonObject Move(uint appId, string to)
    {
        var moved = dl.Move(appId, to);
        if (moved) SaveQueue();
        return new JsonObject { ["moved"] = moved };
    }

    /// <summary>Hearth's settings for the engine (Settings › Library) and whether a game is running, sent by Hearth
    /// whenever they change.</summary>
    public JsonObject Config(string? autoUpdate, long? limitKBps, bool? running)
    {
        if (autoUpdate != null) autoPolicy = AutoUpdate.Normal(autoUpdate);
        if (limitKBps is long l) limiter.BytesPerSecond = Math.Max(0, l) * 1024;
        if (running is bool r) Volatile.Write(ref gameRunning, r);
        store.Update(d => { d["autoUpdate"] = autoPolicy; d["limitKBps"] = limiter.BytesPerSecond / 1024; });
        return new JsonObject { ["autoUpdate"] = autoPolicy, ["limitKBps"] = limiter.BytesPerSecond / 1024, ["gameRunning"] = gameRunning };
    }

    public JsonObject Uninstall(uint appId)
    {
        NeedSignIn();
        if (SteamFiles.ClassicRunning()) throw new EngineException(Codes.ClassicRunning, "The normal Steam client is running.");
        Cancel(appId);
        for (int i = 0; i < 30 && dl.Current?.AppId == appId; i++) Thread.Sleep(100);
        if (dl.Current?.AppId == appId) throw new EngineException(Codes.Busy, "Still stopping its download.");
        var inst = Installed(appId) ?? throw new EngineException(Codes.Unsupported, "Not installed.");
        OpenRules.Uninstall(inst.lib, appId);
        return new JsonObject { ["removed"] = true };
    }

    bool UserPickedProton(uint appId)
    {
        var s = SteamFiles.UserSteam(home);
        var f = s == null ? null : Path.Combine(s, "config", "config.vdf");
        try { return f != null && File.Exists(f) && SteamFiles.CompatMapping(File.ReadAllText(f)).ContainsKey(appId); } catch (IOException) { return false; }
    }

    void ResumeQueue()
    {
        // Valve's client may have finished an install meanwhile (an automatic update or a verify still runs).
        dl.Load(store.Data["queue"] as JsonArray, i => i.Kind is "install" or "update" && Installed(i.AppId)?.acf.StateFlags == AppManifest.FullyInstalled);
    }

    async Task Worker()
    {
        while (!cts.IsCancellationRequested)
        {
            var j = dl.Next(Hold(), Volatile.Read(ref gameRunning), AutoUpdate.DuringGame(autoPolicy));
            if (j == null) { try { await Task.Delay(1000, cts.Token); } catch (OperationCanceledException) { } continue; }
            dl.Started(j);
            var run = runCts = CancellationTokenSource.CreateLinkedTokenSource(cts.Token);
            try
            {
                await RunJob(j, run.Token);
                dl.Ended(j, "done");
                Log($"app {j.AppId}: {(j.Verify ? "verified" : j.Auto ? "updated automatically" : "installed")}");
            }
            catch (OperationCanceledException)
            {
                // Paused, moved, standby or Hearth stopping: it waits in the queue. Cancelled: it's gone from it.
                dl.Ended(j, dl.Has(j.AppId) ? "waiting" : "cancelled");
            }
            catch (Exception) when (run.IsCancellationRequested)
            {
                // Stopped mid-request (a download error from the stop itself): the same as above.
                dl.Ended(j, dl.Has(j.AppId) ? "waiting" : "cancelled");
            }
            catch (Exception e)
            {
                dl.Ended(j, "failed", e is EngineException ? e.Message : $"{e.GetType().Name}: {e.Message}");
                Log($"app {j.AppId}: failed ({e.GetType().Name}); Valve's client can finish it from its record");
            }
            finally { runCts = null; run.Dispose(); }
            if (!cts.IsCancellationRequested) SaveQueue();
        }
    }

    /// <summary>Automatic updates (Settings › Library: off / when idle / always): Steam's PICS change numbers are
    /// checked every 20 minutes while signed in (not during a game unless "always"); installed games whose build
    /// changed get an "auto" job. Never while Valve's client runs (standby).</summary>
    async Task AutoUpdates()
    {
        try { await Task.Delay(TimeSpan.FromSeconds(90), cts.Token); } catch (OperationCanceledException) { return; }
        while (!cts.IsCancellationRequested)
        {
            try
            {
                if (!Hold() && AutoUpdate.CheckDue(autoPolicy, Volatile.Read(ref gameRunning), lastUpdateCheck, DateTime.UtcNow))
                {
                    lastUpdateCheck = DateTime.UtcNow;
                    await CheckUpdates();
                }
            }
            catch (Exception e) when (e is not OperationCanceledException) { Log("update check: " + e.GetType().Name); }
            try { await Task.Delay(TimeSpan.FromSeconds(60), cts.Token); } catch (OperationCanceledException) { return; }
        }
    }

    async Task CheckUpdates()
    {
        uint since = store.Data["changeNumber"]?.GetValue<uint>() ?? 0;
        var changes = await apps.PICSGetChangesSince(since, true, false);
        var changed = new HashSet<uint>(changes.AppChanges.Keys);
        bool full = since == 0 || changes.RequiresFullUpdate || changes.RequiresFullAppUpdate;
        var installed = new Dictionary<uint, (uint Build, string Branch, bool Installed)>();
        var where = new Dictionary<uint, string>();
        HashSet<uint> mine; lock (gate) mine = ownedApps.ToHashSet();
        foreach (var lib in Libraries())
        foreach (var f in SafeEnum(Path.Combine(lib, "steamapps"), "appmanifest_*.acf"))
        {
            if (!uint.TryParse(Path.GetFileNameWithoutExtension(f)["appmanifest_".Length..], out var app) || !mine.Contains(app)) continue;
            var m = AppManifest.Load(lib, app);
            if (m == null || m.StateFlags != AppManifest.FullyInstalled) continue;
            installed[app] = (m.BuildId, m.BetaKey, true);
            where[app] = lib;
        }
        var want = installed.Keys.Where(a => full || changed.Contains(a)).ToList();
        var infos = want.Count > 0 ? await AppInfos(want) : new Dictionary<uint, AppInfo>();
        var need = AutoUpdate.Needing(installed, changed, full, (a, branch) => infos.TryGetValue(a, out var i) ? i.BuildFor(branch) : 0);
        int added = 0;
        foreach (var a in need)
            if (dl.Add(new QueueItem { AppId = a, Library = where[a], Kind = "auto", Name = infos[a].Name, Branch = installed[a].Branch })) added++;
        store.Update(d => d["changeNumber"] = changes.CurrentChangeNumber);
        // Workshop items of installed games, every 6 hours: a Workshop job each for games that have some.
        if (DateTime.UtcNow - lastWorkshopCheck >= TimeSpan.FromHours(6))
        {
            lastWorkshopCheck = DateTime.UtcNow;
            foreach (var (a, lib) in where)
                if (WorkshopManifest.Load(lib, a).Items.Count > 0 && dl.Add(new QueueItem { AppId = a, Library = lib, Kind = "workshop", Name = infos.TryGetValue(a, out var wi) ? wi.Name : "" })) added++;
        }
        if (added > 0) SaveQueue();
        Log($"update check: {installed.Count} installed, {want.Count} changed, {added} to update");
    }

    static IEnumerable<string> SafeEnum(string dir, string pattern)
    {
        try { return Directory.Exists(dir) ? Directory.EnumerateFiles(dir, pattern).ToList() : new List<string>(); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { return new List<string>(); }
    }

    async Task<List<SteamKit2.CDN.Server>> Servers()
    {
        var all = await content.GetServersForSteamPipe();
        var list = all.Where(s => s.Type is "SteamCache" or "CDN").OrderBy(s => s.WeightedLoad).ToList();
        return list.Count > 0 ? list : all.ToList();
    }

    static List<DepotFile> Files(DepotManifest m) => (m.Files ?? new List<DepotManifest.FileData>()).Select(f => new DepotFile(
        f.FileName, f.TotalSize, (int)f.Flags, f.FileHash, f.Chunks.Select(c => new ChunkRef(c.ChunkID ?? Array.Empty<byte>(), c.Offset, c.UncompressedLength, c.CompressedLength, c.Checksum)).ToList(), f.LinkTarget)).ToList();

    async Task<DepotManifest> GetManifest(uint appId, uint depot, ulong gid, byte[] key, List<SteamKit2.CDN.Server> servers, CancellationToken ct, string branch = "public", bool ugc = false)
    {
        var code = await content.GetManifestRequestCode(depot, appId, gid, branch.Length == 0 ? "public" : branch);
        if (code == 0 && !ugc) throw new EngineException(Codes.SteamError, $"Steam didn't give a manifest code for depot {depot}.");
        Exception? last = null;
        foreach (var s in servers.Take(5))
        {
            ct.ThrowIfCancellationRequested();
            try { return await cdn.DownloadManifestAsync(depot, gid, code, s, key); }
            catch (Exception e) when (e is HttpRequestException or TaskCanceledException) { last = e; }
        }
        throw new EngineException(Codes.SteamError, $"Couldn't download the manifest of depot {depot}: {last?.Message}");
    }

    /// <summary>The manifest a depot was at before (for a delta update): the Open engine's own copy, then Valve's
    /// client's depotcache, then Steam's CDN. Null: unknown, so the disk is checked chunk by chunk.</summary>
    async Task<List<DepotFile>?> OldFiles(string lib, uint appId, uint depot, ulong oldGid, byte[] key, List<SteamKit2.CDN.Server> servers, CancellationToken ct)
    {
        if (oldGid == 0) return null;
        try
        {
            var mine = OpenPaths.Manifest(lib, appId, depot, oldGid);
            DepotManifest? m = DepotManifest.LoadFromFile(mine);
            if (m == null && SteamFiles.UserSteam(home) is { } s)
            {
                m = DepotManifest.LoadFromFile(Path.Combine(s, "depotcache", $"{depot}_{oldGid}.manifest"));
                if (m != null && m.FilenamesEncrypted && !m.DecryptFilenames(key)) m = null;
            }
            m ??= await GetManifest(appId, depot, oldGid, key, servers, ct);
            return Files(m);
        }
        catch (Exception e) when (e is not OperationCanceledException) { Log($"depot {depot}: old manifest unknown ({e.GetType().Name}); checking files on disk"); return null; }
    }

    async Task RunJob(QueueItem j, CancellationToken ct)
    {
        j.State = "preparing";
        if (j.Kind == "workshop") { await RunWorkshop(j, ct); return; }
        var info = await InfoFor(j.AppId, j.Branch) ?? throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
        j.Name = info.Name;
        bool ownsApp(uint a) { lock (gate) return ownedApps.Contains(a); }
        var os = info.ChooseOs(UserPickedProton(j.AppId));
        var depots = info.PickDepots(os, "english", ownsApp);
        if (OpenRules.CanInstall(info, depots, SteamFiles.ClassicRunning(), signedIn) is { } no) throw new EngineException(no.code, no.why);
        var lib = j.Library;
        var acf = AppManifest.Load(lib, j.AppId) ?? new AppManifest { AppId = j.AppId, StateFlags = AppManifest.NewInstall };
        acf.Name = info.Name;
        acf.InstallDir = AppManifest.FolderName(acf.InstallDir.Length > 0 ? acf.InstallDir : info.InstallDir, info.Name, j.AppId);
        acf.LastOwner = steamId;
        acf.TargetBuildId = info.BuildId;
        acf.StateFlags = acf.Installed ? AppManifest.Updating : AppManifest.NewInstall;
        acf.UpdateResult = 0;
        acf.Save(lib);
        var gameDir = Path.Combine(lib, "steamapps", "common", acf.InstallDir);
        var servers = await Servers();
        if (servers.Count == 0) throw new EngineException(Codes.SteamError, "No Steam content servers.");

        // Manifests and plans for every depot first, so the total is known before downloading.
        var work = new List<(DepotInfo d, byte[] key, DepotManifest m, Plan plan)>();
        foreach (var d in depots)
        {
            ct.ThrowIfCancellationRequested();
            var k = await apps.GetDepotDecryptionKey(d.Id, j.AppId);
            if (k.Result != EResult.OK) { if (d.DlcAppId != 0) continue; throw new EngineException(Codes.SteamError, $"No access to depot {d.Id} ({k.Result})."); }
            var m = await GetManifest(j.AppId, d.Id, d.Manifest, k.DepotKey, servers, ct, j.Branch);
            List<DepotFile>? old = null;
            if (!j.Verify && acf.Depots.TryGetValue(d.Id, out var had)) old = had.Manifest == d.Manifest ? Files(m) : await OldFiles(lib, j.AppId, d.Id, had.Manifest, k.DepotKey, servers, ct);
            var plan = Depot.Build(Files(m), old, p => { try { var fi = new FileInfo(Depot.Inside(gameDir, p)); return fi.Exists ? (ulong)fi.Length : null; } catch (InvalidDataException) { return null; } }, j.Verify);
            work.Add((d, k.DepotKey, m, plan));
        }
        j.Total = (long)work.Sum(w => (decimal)w.plan.Bytes);
        acf.BytesToDownload = (ulong)j.Total; acf.BytesDownloaded = 0; acf.Save(lib);
        j.State = j.Verify ? "verifying" : "downloading";

        long before = 0;
        var serverIndex = new int[1];
        foreach (var (d, key, m, plan) in work)
        {
            var writer = new DepotWriter(gameDir, OpenPaths.Staging(lib, j.AppId), async (c, t) =>
            {
                for (int attempt = 0; ; attempt++)
                {
                    var s = servers[(Volatile.Read(ref serverIndex[0]) + attempt) % servers.Count];
                    try
                    {
                        await limiter.Wait(c.CompressedLength, t);
                        var buf = new byte[c.Length];
                        var n = await cdn.DownloadDepotChunkAsync(d.Id, new DepotManifest.ChunkData(c.Sha, c.Checksum, c.Offset, c.CompressedLength, c.Length), s, buf, key);
                        return n == buf.Length ? buf : buf[..n];
                    }
                    catch (Exception e) when (attempt < 6 && e is HttpRequestException or TaskCanceledException or IOException && !t.IsCancellationRequested)
                    {
                        Interlocked.Increment(ref serverIndex[0]);
                        await Task.Delay(500 * (attempt + 1), t);
                    }
                }
            })
            {
                Progress = done => j.Done = before + done,
            };
            var start = DateTime.UtcNow;
            using var ticker = new Timer(_ =>
            {
                var secs = Math.Max(1, (DateTime.UtcNow - start).TotalSeconds);
                j.Speed = (long)(Interlocked.Read(ref writer.Downloaded) / secs);
                try { acf.BytesDownloaded = (ulong)Math.Max(0, j.Done); acf.Save(lib); } catch (IOException) { }
            }, null, 2000, 2000);
            await writer.Apply(plan, ct);
            before += (long)plan.Bytes;
            // Kept for the next update's delta (Valve's client ignores this folder).
            try { Directory.CreateDirectory(OpenPaths.AppDir(lib, j.AppId)); m.SaveToFile(OpenPaths.Manifest(lib, j.AppId, d.Id, d.Manifest)); }
            catch (IOException) { }
            if (acf.Depots.TryGetValue(d.Id, out var oldD) && oldD.Manifest != d.Manifest) try { File.Delete(OpenPaths.Manifest(lib, j.AppId, d.Id, oldD.Manifest)); } catch (IOException) { }
            acf.Depots[d.Id] = (d.Manifest, m.TotalUncompressedSize, d.DlcAppId);
        }
        acf.StateFlags = AppManifest.FullyInstalled;
        acf.BuildId = info.BuildId;
        acf.BetaKey = j.Branch == "public" ? "" : j.Branch;   // UserConfig / MountedConfig BetaKey, as Steam records the branch
        acf.SizeOnDisk = (ulong)acf.Depots.Values.Sum(v => (decimal)v.Size);
        acf.BytesDownloaded = acf.BytesToDownload;
        acf.LastUpdated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        acf.Save(lib);
        try { Directory.Delete(OpenPaths.Staging(lib, j.AppId), true); } catch (IOException) { }
    }

    public JsonObject Downloads()
    {
        var o = dl.Answer();
        o["autoUpdate"] = autoPolicy; o["limitKBps"] = limiter.BytesPerSecond / 1024;
        lock (gate) o["standby"] = standby;
        return o;
    }

    // ---------------------------------------------------------------- launching

    /// <summary>DRM-free games: how Hearth starts it itself ({direct, exe, args, cwd, os, prefix}). Others answer
    /// unsupported, and Hearth starts them through Valve's client (legitimate licence checks).</summary>
    /// <summary>An installed game's app info, library, folder and installed build (linux / windows).</summary>
    async Task<(AppInfo info, string lib, AppManifest acf, string gameDir, string os)> Context(uint appId)
    {
        var inst = Installed(appId) ?? throw new EngineException(Codes.Unsupported, "Not installed.");
        AppInfo? info; lock (gate) owned.TryGetValue(appId, out info);
        info ??= await FreshInfo(appId).WaitAsync(TimeSpan.FromSeconds(10));
        if (info == null) throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
        var gameDir = Path.Combine(inst.lib, "steamapps", "common", inst.acf.InstallDir);
        if (!AppManifest.SafeDirName(inst.acf.InstallDir) || !Directory.Exists(gameDir)) throw new EngineException(Codes.Unsupported, "Its folder is missing.");
        // Which build is installed: Linux when its depots are (the record's depots), else Windows.
        var linuxDepots = info.Depots.Where(d => d.OsList.Contains("linux", StringComparison.OrdinalIgnoreCase)).Select(d => d.Id).ToHashSet();
        var os = inst.acf.Depots.Keys.Any(linuxDepots.Contains) ? "linux" : info.Depots.Any(d => d.OsList.Contains("windows", StringComparison.OrdinalIgnoreCase)) ? "windows" : info.ChooseOs(false);
        return (info, inst.lib, inst.acf, gameDir, os);
    }

    static string Prefix(string lib, uint appId) => Path.Combine(lib, "steamapps", "compatdata", appId.ToString(), "pfx");

    CloudPaths Paths(AppInfo info, string lib, string gameDir, string os, uint appId) => new(info.Ufs, gameDir, os, Prefix(lib, appId), home, steamId);

    /// <summary>DRM-free games: how Hearth starts it itself ({direct, exe, args, cwd, os, prefix, cloud}). Others answer
    /// unsupported, and Hearth starts them through Valve's client (legitimate licence checks). A game with Steam Cloud
    /// saves Hearth can't place also starts through Valve's client, which syncs them.</summary>
    public async Task<JsonObject> Launch(uint appId)
    {
        NeedSignIn();
        var inst = Installed(appId) ?? throw new EngineException(Codes.Unsupported, "Not installed.");
        if (!inst.acf.Installed || (inst.acf.StateFlags & AppManifest.UpdateRequired) != 0) throw new EngineException(Codes.Busy, "It's being installed or updated.");
        var (info, lib, _, gameDir, os) = await Context(appId);
        var files = Directory.EnumerateFiles(gameDir, "*", new EnumerationOptions { RecurseSubdirectories = true, MaxRecursionDepth = 8, IgnoreInaccessible = true, AttributesToSkip = FileAttributes.ReparsePoint })
            .Take(200_000).Select(f => Path.GetRelativePath(gameDir, f));
        var decision = LaunchPolicy.Decide(info, os, files, exe =>
        {
            try { using var fs = File.OpenRead(Depot.Inside(gameDir, exe)); return LaunchPolicy.HasDrmWrapper(fs); } catch (Exception) { return true; }
        }, store.Launch(appId));
        Log($"app {appId}: {decision.Reason}");
        if (!decision.Direct || decision.Entry == null) throw new EngineException(Codes.Unsupported, decision.Reason);
        var (cloudOk, cloudWhy) = Paths(info, lib, gameDir, decision.Os, appId).Supported();
        if (!cloudOk)
        {
            Log($"app {appId}: Steam Cloud: {cloudWhy}; Valve's client starts it");
            throw new EngineException(Codes.Unsupported, $"Steam Cloud saves Hearth can't place ({cloudWhy}): Valve's client starts it and syncs them.");
        }
        var exePath = Depot.Inside(gameDir, decision.Entry.Exe);
        if (!File.Exists(exePath)) throw new EngineException(Codes.Unsupported, "Its program is missing.");
        var cwd = decision.Entry.WorkingDir.Length > 0 && Depot.SafePath(decision.Entry.WorkingDir) != null ? Depot.Inside(gameDir, decision.Entry.WorkingDir + "/x")[..^2] : Path.GetDirectoryName(exePath)!;
        return new JsonObject
        {
            ["direct"] = true, ["exe"] = exePath, ["args"] = decision.Entry.Args, ["cwd"] = cwd, ["os"] = decision.Os,
            ["firstRun"] = decision.FirstRun, ["reason"] = decision.Reason, ["dir"] = gameDir,
            // Valve's client's Proton prefix for the game, so saves are shared with it.
            ["prefix"] = Prefix(lib, appId),
            // Steam Cloud: Hearth syncs it before the start (cloud.before) and after the game exits (cloud.after).
            ["cloud"] = info.Ufs.HasCloud,
        };
    }

    async Task<CloudPaths> CloudFor(uint appId)
    {
        NeedSignIn();
        var (info, lib, _, gameDir, os) = await Context(appId);
        return Paths(info, lib, gameDir, os, appId);   // the same build and places as Launch
    }

    public async Task<JsonObject> CloudBefore(uint appId) => await cloudSync.Before(appId, await CloudFor(appId));
    public async Task<JsonObject> CloudAfter(uint appId) => await cloudSync.After(appId, await CloudFor(appId));
    public async Task<JsonObject> CloudResolve(uint appId, string choice) => await cloudSync.Resolve(appId, choice, await CloudFor(appId));

    // ---------------------------------------------------------------- beta branches

    public async Task<JsonObject> BetaList(uint appId)
    {
        NeedSignIn();
        var info = await FreshInfo(appId) ?? throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
        var list = new JsonArray();
        foreach (var b in Betas.Offer(info))
            list.Add(new JsonObject
            {
                ["name"] = b.Name, ["buildid"] = b.BuildId, ["password"] = b.PasswordRequired, ["unlocked"] = b.PasswordRequired && BetaKey(appId, b.Name) != null,
                ["description"] = b.Description, ["updated"] = b.TimeUpdated,
            });
        var inst = Installed(appId);
        return new JsonObject { ["branches"] = list, ["chosen"] = BranchOf(appId), ["installed"] = inst?.acf.Installed == true ? inst.Value.acf.BetaKey : null };
    }

    /// <summary>Steam's beta password check (SteamKit2's CheckAppBetaPassword): the branches it unlocks, their keys kept
    /// in the secret store. The password itself is never kept or logged.</summary>
    public async Task<JsonObject> BetaPassword(uint appId, string password)
    {
        NeedSignIn();
        var r = await apps.CheckAppBetaPassword(appId, password);
        if (r.Result != EResult.OK || r.BetaPasswords.Count == 0) throw new EngineException(Codes.SteamError, "That password didn't unlock a beta.");
        var unlocked = new JsonArray();
        foreach (var (branch, key) in r.BetaPasswords)
        {
            try { secrets.Save($"beta:{appId}:{branch}", Convert.ToHexString(key)); } catch (Exception) { Log("couldn't keep a beta's key"); }
            unlocked.Add(branch);
        }
        Log($"app {appId}: {unlocked.Count} beta(s) unlocked");
        return new JsonObject { ["unlocked"] = unlocked };
    }

    /// <summary>Picks a game's branch ("" or "public": the default) and, when it's installed, queues the switch (that
    /// branch's manifests, recorded as BetaKey in the appmanifest like Steam does).</summary>
    public async Task<JsonObject> BetaSet(uint appId, string branch)
    {
        NeedSignIn();
        var info = await FreshInfo(appId) ?? throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
        var name = Betas.Pick(info, branch);
        var b = info.Branches.FirstOrDefault(x => x.Name == name);
        if (b != null && b.PasswordRequired && BetaKey(appId, name) == null) throw new EngineException(Codes.BadRequest, "That beta needs its password first.");
        store.Update(d => { if (d["betas"] is not JsonObject o) d["betas"] = o = new JsonObject(); o[appId.ToString()] = name; });
        var inst = Installed(appId);
        bool queued = false;
        if (inst != null && inst.Value.acf.BetaKey != name) { Enqueue(appId, inst.Value.lib, "update", name.Length == 0 ? "public" : name); queued = true; }
        return new JsonObject { ["branch"] = name, ["queued"] = queued };
    }

    // ---------------------------------------------------------------- Workshop

    public async Task<JsonObject> WorkshopList(uint appId)
    {
        NeedSignIn();
        var subs = await workshop.Subscribed(appId, steamId);
        var inst = Installed(appId);
        var m = inst == null ? new WorkshopManifest { AppId = appId } : WorkshopManifest.Load(inst.Value.lib, appId);
        var a = new JsonArray();
        foreach (var i in subs ?? new List<WorkshopItem>())
        {
            bool have = m.Items.TryGetValue(i.Id, out var h);
            a.Add(new JsonObject
            {
                ["id"] = i.Id.ToString(), ["title"] = i.Title, ["size"] = i.Size, ["updated"] = i.TimeUpdated, ["installed"] = have,
                ["needsUpdate"] = have && (h.TimeUpdated < i.TimeUpdated || (i.Manifest != 0 && h.Manifest != i.Manifest)),
            });
        }
        return new JsonObject { ["items"] = a, ["complete"] = subs != null, ["gameInstalled"] = inst != null };
    }

    public JsonObject WorkshopSync(uint appId)
    {
        NeedSignIn();
        if (SteamFiles.ClassicRunning()) throw new EngineException(Codes.ClassicRunning, "The normal Steam client is running.");
        var inst = Installed(appId) ?? throw new EngineException(Codes.Unsupported, "Install the game first.");
        bool added = dl.Add(new QueueItem { AppId = appId, Library = inst.lib, Kind = "workshop", Name = inst.acf.Name });
        SaveQueue();
        return new JsonObject { ["queued"] = added };
    }

    public async Task<JsonObject> WorkshopUnsubscribe(uint appId, ulong item)
    {
        NeedSignIn();
        if (dl.Current is { } c && c.AppId == appId && c.Kind == "workshop") throw new EngineException(Codes.Busy, "Its Workshop items are downloading; try again when that's done.");
        await workshop.Unsubscribe(appId, item);
        if (Installed(appId) is { } inst)
        {
            var m = WorkshopManifest.Load(inst.lib, appId);
            if (m.Items.ContainsKey(item)) { WorkshopRules.Remove(inst.lib, m, item); m.TimeLastUpdated = DateTimeOffset.UtcNow.ToUnixTimeSeconds(); m.Save(inst.lib); }
        }
        return new JsonObject { ["unsubscribed"] = true };
    }

    /// <summary>A Workshop job: subscribed items that are new or updated come down into
    /// steamapps/workshop/content/&lt;app&gt;/&lt;id&gt; (UGC manifests through the app's workshop depot, or an old item's single
    /// file), unsubscribed ones are removed, and appworkshop_&lt;app&gt;.acf says what's there.</summary>
    async Task RunWorkshop(QueueItem j, CancellationToken ct)
    {
        var inst = Installed(j.AppId) ?? throw new EngineException(Codes.Unsupported, "Install the game first.");
        var lib = inst.lib;
        var info = await FreshInfo(j.AppId) ?? throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
        j.Name = info.Name;
        var subs = await workshop.Subscribed(j.AppId, steamId);
        var m = WorkshopManifest.Load(lib, j.AppId);
        m.SubscribedBy = (uint)(steamId & 0xFFFFFFFF);
        m.LastBuildId = inst.acf.BuildId;
        var (get, remove) = WorkshopRules.Plan(subs, m);
        foreach (var id in remove) WorkshopRules.Remove(lib, m, id);
        m.TimeLastUpdated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        m.Save(lib);
        Log($"app {j.AppId}: Workshop: {get.Count} to download, {remove.Count} removed");
        if (get.Count == 0) return;
        var depot = info.WorkshopDepot != 0 ? info.WorkshopDepot : j.AppId;
        byte[]? key = null;
        var servers = await Servers();
        if (servers.Count == 0) throw new EngineException(Codes.SteamError, "No Steam content servers.");
        j.Total = (long)get.Sum(i => (decimal)i.Size); j.Done = 0; j.State = "downloading";
        long before = 0;
        var serverIndex = new int[1];
        foreach (var item in get)
        {
            ct.ThrowIfCancellationRequested();
            var dir = WorkshopManifest.ContentDir(lib, j.AppId, item.Id);
            ulong size;
            if (item.Manifest != 0)
            {
                if (key == null)
                {
                    var k = await apps.GetDepotDecryptionKey(depot, j.AppId);
                    if (k.Result != EResult.OK) throw new EngineException(Codes.SteamError, $"No access to the Workshop depot ({k.Result}).");
                    key = k.DepotKey;
                }
                var man = await GetManifest(j.AppId, depot, item.Manifest, key, servers, ct, "public", ugc: true);
                var keep = Path.Combine(OpenPaths.AppDir(lib, j.AppId), "workshop");
                List<DepotFile>? old = null;
                if (m.Items.TryGetValue(item.Id, out var had) && had.Manifest != 0 && DepotManifest.LoadFromFile(Path.Combine(keep, $"{item.Id}_{had.Manifest}.manifest")) is { } om) old = Files(om);
                var plan = Depot.Build(Files(man), old, p => { try { var fi = new FileInfo(Depot.Inside(dir, p)); return fi.Exists ? (ulong)fi.Length : null; } catch (InvalidDataException) { return null; } });
                var writer = new DepotWriter(dir, Path.Combine(OpenPaths.AppDir(lib, j.AppId), "workshop-staging", item.Id.ToString()), Fetcher(depot, key, servers, serverIndex))
                {
                    Progress = done => j.Done = before + done,
                };
                await writer.Apply(plan, ct);
                try
                {
                    Directory.CreateDirectory(keep);
                    man.SaveToFile(Path.Combine(keep, $"{item.Id}_{item.Manifest}.manifest"));
                    if (had.Manifest != 0 && had.Manifest != item.Manifest) File.Delete(Path.Combine(keep, $"{item.Id}_{had.Manifest}.manifest"));
                }
                catch (IOException) { }
                size = man.TotalUncompressedSize;
            }
            else
            {
                Directory.CreateDirectory(dir);
                var file = Depot.Inside(dir, WorkshopRules.FileName(item.FileName, item.Id));
                await limiter.Wait((long)item.Size, ct);
                using (var resp = await Http.GetAsync(item.FileUrl, HttpCompletionOption.ResponseHeadersRead, ct))
                {
                    resp.EnsureSuccessStatusCode();
                    await using var fs = File.Create(file + ".hearth-tmp");
                    await resp.Content.CopyToAsync(fs, ct);
                }
                File.Move(file + ".hearth-tmp", file, true);
                size = (ulong)new FileInfo(file).Length;
            }
            before += (long)item.Size;
            j.Done = before;
            m.Items[item.Id] = (size, item.TimeUpdated, item.Manifest);
            m.Save(lib);
        }
    }

    /// <summary>Downloads one chunk from Steam's CDN (servers in turn on errors, the speed limit first).</summary>
    Func<ChunkRef, CancellationToken, Task<byte[]>> Fetcher(uint depot, byte[] key, List<SteamKit2.CDN.Server> servers, int[] serverIndex) => async (c, t) =>
    {
        for (int attempt = 0; ; attempt++)
        {
            var s = servers[(Volatile.Read(ref serverIndex[0]) + attempt) % servers.Count];
            try
            {
                await limiter.Wait(c.CompressedLength, t);
                var buf = new byte[c.Length];
                var n = await cdn.DownloadDepotChunkAsync(depot, new DepotManifest.ChunkData(c.Sha, c.Checksum, c.Offset, c.CompressedLength, c.Length), s, buf, key);
                return n == buf.Length ? buf : buf[..n];
            }
            catch (Exception e) when (attempt < 6 && e is HttpRequestException or TaskCanceledException or IOException && !t.IsCancellationRequested)
            {
                Interlocked.Increment(ref serverIndex[0]);
                await Task.Delay(500 * (attempt + 1), t);
            }
        }
    };

    public JsonObject LaunchResult(uint appId, bool ok)
    {
        store.RememberLaunch(appId, ok ? "direct" : "client");
        Log($"app {appId}: direct start {(ok ? "works" : "failed; it starts through Valve's client from now on")}");
        return new JsonObject { ["remembered"] = ok ? "direct" : "client" };
    }

    public JsonObject Running() => new() { ["apps"] = new JsonArray() };

    /// <summary>Valve's client runs (or is about to): downloads stop where they are (kept, resumed later) and nothing
    /// is written into the libraries; sign-in and the library keep working.</summary>
    void EnterStandby(string why)
    {
        lock (gate) { if (standby) return; standby = true; }
        StopRun();
        Log($"standing by: {why}");
    }

    void LeaveStandby()
    {
        lock (gate) { if (!standby) return; standby = false; }
        Log("the normal Steam client has closed; carrying on");
        _ = Task.Run(ResumeQueue);
    }

    public JsonObject Standby()
    {
        EnterStandby("asked to (Valve's client is starting)");
        // Wait briefly for a running download to stop, so Valve's client never writes the same files at the same time.
        for (int i = 0; i < 50 && dl.Current != null; i++) Thread.Sleep(100);
        bool idle = dl.Current == null;
        return new JsonObject { ["standby"] = true, ["stopped"] = idle };
    }

    public void Release()
    {
        if (released) return;
        released = true;
        watchdog?.Dispose();
        yielder?.Dispose();
        cts.Cancel();
        try { client.Disconnect(); } catch (Exception) { }
        Released?.Invoke();
    }

    public void Dispose()
    {
        watchdog?.Dispose();
        cts.Cancel();
        try { client.Disconnect(); } catch (Exception) { }
        cdn.Dispose();
    }
}
