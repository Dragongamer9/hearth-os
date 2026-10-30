using System.Text.Json.Nodes;
using HearthSteam.Open;
using QRCoder;
using SteamKit2;
using SteamKit2.Authentication;

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
    string? account;
    ulong steamId;
    JsonObject signIn = new() { ["state"] = "idle" };
    TaskCompletionSource<JsonObject>? step;
    CancellationTokenSource? authCts;
    BridgeAuthenticator? authenticator;
    List<SteamApps.LicenseListCallback.License> licenses = new();
    readonly Dictionary<uint, AppInfo> owned = new();
    HashSet<uint> ownedApps = new();
    bool libraryLoading;
    readonly List<Job> queue = new();
    Job? current;
    readonly Dictionary<uint, Job> finished = new();
    Timer? watchdog;
    int reconnectDelay = 5;
    public event Action? Released;

    sealed class Job
    {
        public uint AppId; public string Library = ""; public bool Verify;
        public string Name = "", State = "queued", Error = "";
        public long Done, Total, Speed;
        public CancellationTokenSource Cts = new();
    }

    public OpenEngine(string home)
    {
        this.home = home;
        var st = Environment.GetEnvironmentVariable("XDG_STATE_HOME");
        stateDir = Path.Combine(string.IsNullOrEmpty(st) ? Path.Combine(home, ".local/state") : st, "hearth-steam");
        Secrets.EnsurePrivateDir(stateDir);
        secrets = Secrets.Pick(stateDir);
        store = new StateFile(Path.Combine(stateDir, "open-state.json"));
        client = new SteamClient(SteamConfiguration.Create(b => b.WithConnectionTimeout(TimeSpan.FromSeconds(15))));
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
    }

    static void Log(string s) => Console.Error.WriteLine("hearth-steam (open): " + s);

    public void Start()
    {
        if (SteamFiles.ClassicRunning()) { lock (gate) { state = "unavailable"; reason = "The normal Steam client is running."; } return; }
        new Thread(() => { while (!cts.IsCancellationRequested) callbacks.RunWaitCallbacks(TimeSpan.FromSeconds(1)); }) { IsBackground = true, Name = "steamkit-callbacks" }.Start();
        client.Connect();
        // The normal Steam client started anyway: stop writing into the libraries and let go, as the Lightweight engine does.
        watchdog = new Timer(_ => { if (SteamFiles.ClassicRunning()) { Log("the normal Steam client started; letting go"); Release(); } }, null, 5000, 5000);
        _ = Task.Run(Worker);
    }

    // ---------------------------------------------------------------- connection and sign-in

    void OnConnected()
    {
        lock (gate) { connected = true; state = "ready"; reason = ""; reconnectDelay = 5; }
        Log("connected to Steam");
        var last = store.Data["account"]?.GetValue<string>();
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
        var d = reconnectDelay; reconnectDelay = Math.Min(reconnectDelay * 2, 300);
        Log($"disconnected; trying again in {d}s");
        Task.Delay(TimeSpan.FromSeconds(d), cts.Token).ContinueWith(t => { if (!t.IsCanceled) client.Connect(); });
    }

    void OnLoggedOn(SteamUser.LoggedOnCallback e)
    {
        if (e.Result != EResult.OK)
        {
            Log($"sign-in refused ({e.Result})");
            if (e.Result is EResult.InvalidPassword or EResult.AccessDenied or EResult.Expired or EResult.Revoked or EResult.InvalidSignature)
            {
                var name = store.Data["account"]?.GetValue<string>();
                if (name != null) { try { secrets.Delete(name); } catch (Exception) { } }
            }
            lock (gate) { signedIn = false; signIn = new JsonObject { ["state"] = "failed", ["error"] = SignInError(e.Result) }; }
            step?.TrySetException(new EngineException(Codes.SteamError, SignInError(e.Result)));
            return;
        }
        lock (gate)
        {
            signedIn = true;
            steamId = e.ClientSteamID?.ConvertToUInt64() ?? 0;
            signIn = new JsonObject { ["state"] = "idle" };
        }
        store.Update(d => { d["account"] = account; d["steamId"] = steamId; });
        Log("signed in");
        step?.TrySetResult(new JsonObject { ["needs"] = null });
        _ = Task.Run(ResumeQueue);
    }

    static string SignInError(EResult r) => r switch
    {
        EResult.InvalidPassword => "Wrong account name or password.",
        EResult.RateLimitExceeded or EResult.AccountLoginDeniedThrottle => "Too many tries. Wait a while, then try again.",
        EResult.Expired or EResult.AccessDenied or EResult.Revoked => "That sign-in has run out. Sign in again.",
        EResult.TwoFactorCodeMismatch or EResult.InvalidLoginAuthCode => "That code wasn't right.",
        _ => $"Steam didn't sign you in ({r}).",
    };

    /// <summary>Hands Steam Guard's questions to Hearth's sign-in screen: the next signin.code answers them.</summary>
    sealed class BridgeAuthenticator : IAuthenticator
    {
        readonly OpenEngine e;
        public TaskCompletionSource<string>? Code;
        public BridgeAuthenticator(OpenEngine e) { this.e = e; }
        Task<string> Ask(string kind, bool wrong)
        {
            Code = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
            lock (e.gate) e.signIn = new JsonObject { ["state"] = "needs", ["needs"] = kind, ["wrong"] = wrong };
            e.step?.TrySetResult(new JsonObject { ["needs"] = kind, ["wrong"] = wrong });
            return Code.Task;
        }
        public Task<string> GetDeviceCodeAsync(bool previousCodeWasIncorrect) => Ask("device", previousCodeWasIncorrect);
        public Task<string> GetEmailCodeAsync(string email, bool previousCodeWasIncorrect) => Ask("email", previousCodeWasIncorrect);
        // Typing the code works with a controller (Hearth's keyboard); approving in the app is the QR sign-in.
        public Task<bool> AcceptDeviceConfirmationAsync() => Task.FromResult(false);
    }

    void NeedConnection()
    {
        lock (gate) if (!connected) throw new EngineException(Codes.NotReady, reason.Length > 0 ? reason : "Not connected to Steam yet.");
    }

    async Task<JsonObject> NextStep()
    {
        var s = step!;
        var done = await Task.WhenAny(s.Task, Task.Delay(TimeSpan.FromSeconds(40)));
        if (done != s.Task) return new JsonObject { ["needs"] = null, ["pending"] = true };
        return await s.Task;
    }

    /// <summary>Runs one sign-in (credentials or QR) to the end in the background.</summary>
    void RunAuth(Func<CancellationToken, Task<AuthSession>> begin, bool remember)
    {
        authCts?.Cancel();
        var ac = authCts = CancellationTokenSource.CreateLinkedTokenSource(cts.Token);
        _ = Task.Run(async () =>
        {
            try
            {
                var session = await begin(ac.Token);
                var result = await session.PollingWaitForResultAsync(ac.Token);
                account = result.AccountName;
                if (remember) { try { secrets.Save(result.AccountName, result.RefreshToken); } catch (Exception) { Log("couldn't keep the sign-in (keyring and file both failed)"); } }
                lock (gate) signIn = new JsonObject { ["state"] = "signing-in" };
                LogOn(result.AccountName, result.RefreshToken);
            }
            catch (OperationCanceledException) { lock (gate) signIn = new JsonObject { ["state"] = "idle" }; step?.TrySetException(new EngineException(Codes.SteamError, "Cancelled.")); }
            catch (AuthenticationException e)
            {
                lock (gate) signIn = new JsonObject { ["state"] = "failed", ["error"] = SignInError(e.Result) };
                step?.TrySetException(new EngineException(Codes.SteamError, SignInError(e.Result)));
            }
            catch (Exception e)
            {
                Log("sign-in failed: " + e.GetType().Name);
                var msg = e is InvalidOperationException ? "Steam wants this sign-in approved in the Steam app: use the QR code instead." : "Steam didn't sign you in.";
                lock (gate) signIn = new JsonObject { ["state"] = "failed", ["error"] = msg };
                step?.TrySetException(new EngineException(Codes.SteamError, msg));
            }
        });
    }

    public async Task<JsonObject> SignInPassword(string userName, string password, bool remember)
    {
        NeedConnection();
        step = new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
        authenticator = new BridgeAuthenticator(this);
        lock (gate) signIn = new JsonObject { ["state"] = "waiting" };
        var auth = authenticator;
        // The password goes straight into SteamKit2's request (encrypted with Steam's key) and is dropped with it.
        RunAuth(async _ => await client.Authentication.BeginAuthSessionViaCredentialsAsync(new AuthSessionDetails
        {
            Username = userName, Password = password, IsPersistentSession = remember, Authenticator = auth, DeviceFriendlyName = "Hearth OS",
        }), remember);
        return await NextStep();
    }

    public async Task<JsonObject> SignInCode(string code, string kind)
    {
        var a = authenticator?.Code ?? throw new EngineException(Codes.BadRequest, "No sign-in is waiting for a code.");
        step = new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
        lock (gate) signIn = new JsonObject { ["state"] = "waiting" };
        a.TrySetResult(code.Trim());
        var r = await NextStep();
        if (r["wrong"]?.GetValue<bool>() == true) throw new EngineException(Codes.SteamError, "That code wasn't right. Try again.");
        return r;
    }

    public async Task<JsonObject> SignInQr()
    {
        NeedConnection();
        step = new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
        var first = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        RunAuth(async _ =>
        {
            var s = await client.Authentication.BeginAuthSessionViaQRAsync(new AuthSessionDetails { IsPersistentSession = true, DeviceFriendlyName = "Hearth OS" });
            void Show() { lock (gate) signIn = new JsonObject { ["state"] = "qr", ["qrSvg"] = Svg(s.ChallengeURL) }; }
            s.ChallengeURLChanged = Show;
            Show();
            first.TrySetResult(s.ChallengeURL);
            return s;
        }, true);
        var done = await Task.WhenAny(first.Task, step.Task, Task.Delay(TimeSpan.FromSeconds(30)));
        if (done != first.Task) { if (step.Task.IsFaulted) await step.Task; throw new EngineException(Codes.SteamError, "Steam didn't give a QR code."); }
        var url = await first.Task;
        return new JsonObject { ["url"] = url, ["qrSvg"] = Svg(url) };
    }

    static string Svg(string url)
    {
        using var gen = new QRCodeGenerator();
        return new SvgQRCode(gen.CreateQrCode(url, QRCodeGenerator.ECCLevel.M)).GetGraphic(4);
    }

    public void SignInCancel()
    {
        authCts?.Cancel();
        authenticator?.Code?.TrySetCanceled();
        lock (gate) signIn = new JsonObject { ["state"] = "idle" };
    }

    public Task<JsonObject> SignOut()
    {
        var name = store.Data["account"]?.GetValue<string>();
        if (name != null) { try { secrets.Delete(name); } catch (Exception) { } }
        store.Update(d => { d.Remove("account"); d.Remove("steamId"); });
        foreach (var j in AllJobs()) j.Cts.Cancel();
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
                ["signIn"] = signIn.DeepClone(), ["secretStore"] = secrets.Kind, ["libraryApps"] = owned.Count,
                ["libraryLoading"] = libraryLoading,
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

    async Task<Dictionary<uint, AppInfo>> AppInfos(IEnumerable<uint> ids)
    {
        var result = new Dictionary<uint, AppInfo>();
        foreach (var batch in ids.Chunk(200))
        {
            var tokens = await apps.PICSGetAccessTokens(batch, Enumerable.Empty<uint>());
            var reqs = batch.Select(a => new SteamApps.PICSRequest(a, tokens.AppTokens.TryGetValue(a, out var t) ? t : 0)).ToList();
            var r = await apps.PICSGetProductInfo(reqs, Enumerable.Empty<SteamApps.PICSRequest>());
            foreach (var cb in r.Results ?? new List<SteamApps.PICSProductInfoCallback>().AsReadOnly())
                foreach (var (id, i) in cb.Apps) result[id] = AppInfo.Parse(id, Conv(i.KeyValues));
        }
        return result;
    }

    async Task<AppInfo?> FreshInfo(uint appId)
    {
        var r = await AppInfos(new[] { appId });
        if (r.TryGetValue(appId, out var i)) { lock (gate) if (i.IsGame) owned[appId] = i; return i; }
        return null;
    }

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

    IEnumerable<Job> AllJobs() { lock (gate) return (current != null ? queue.Prepend(current) : queue).ToList(); }

    void SaveQueue() => store.Update(d =>
    {
        var a = new JsonArray();
        foreach (var j in AllJobs()) a.Add(new JsonObject { ["appid"] = j.AppId, ["library"] = j.Library, ["verify"] = j.Verify });
        d["queue"] = a;
    });

    JsonObject Enqueue(uint appId, string? library, bool verify)
    {
        NeedSignIn();
        if (SteamFiles.ClassicRunning()) throw new EngineException(Codes.ClassicRunning, "The normal Steam client is running.");
        AppInfo? info;
        try { info = FreshInfo(appId).WaitAsync(TimeSpan.FromSeconds(12)).GetAwaiter().GetResult(); }
        catch (TimeoutException) { throw new EngineException(Codes.Busy, "Steam didn't answer in time."); }
        bool ownsApp(uint a) { lock (gate) return ownedApps.Contains(a); }
        var os = info?.ChooseOs(UserPickedProton(appId)) ?? "windows";
        var picked = info?.PickDepots(os, "english", ownsApp) ?? new List<DepotInfo>();
        if (OpenRules.CanInstall(info, picked, false, true) is { } no) throw new EngineException(no.code, no.why);
        var libs = Libraries();
        var lib = OpenRules.PickLibrary(libs, l => File.Exists(AppManifest.Path(l, appId)), library);
        lock (gate)
        {
            if (current?.AppId == appId || queue.Any(j => j.AppId == appId)) return new JsonObject { ["queued"] = true, ["library"] = lib };
            queue.Add(new Job { AppId = appId, Library = lib, Verify = verify, Name = info!.Name });
            finished.Remove(appId);
        }
        SaveQueue();
        return new JsonObject { ["queued"] = true, ["library"] = lib, ["os"] = os };
    }

    public JsonObject Install(uint appId, string? library) => Enqueue(appId, library, false);
    public JsonObject Update(uint appId) => Enqueue(appId, null, false);
    public JsonObject Verify(uint appId) => Enqueue(appId, null, true);

    public JsonObject Cancel(uint appId)
    {
        lock (gate)
        {
            queue.RemoveAll(j => j.AppId == appId);
            if (current?.AppId == appId) current.Cts.Cancel();
        }
        SaveQueue();
        return new JsonObject { ["cancelled"] = true };
    }

    public JsonObject Uninstall(uint appId)
    {
        NeedSignIn();
        if (SteamFiles.ClassicRunning()) throw new EngineException(Codes.ClassicRunning, "The normal Steam client is running.");
        Cancel(appId);
        lock (gate) if (current?.AppId == appId) throw new EngineException(Codes.Busy, "Still stopping its download.");
        var inst = Installed(appId) ?? throw new EngineException(Codes.Unsupported, "Not installed.");
        OpenRules.Uninstall(inst.lib, appId);
        lock (gate) finished.Remove(appId);
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
        if (store.Data["queue"] is not JsonArray a) return;
        foreach (var e in a.ToList())
        {
            if (e == null) continue;
            var id = e["appid"]!.GetValue<uint>();
            lock (gate) if (current?.AppId == id || queue.Any(j => j.AppId == id)) continue;
            // Valve's client may have finished it meanwhile.
            var inst = Installed(id);
            if (inst?.acf.StateFlags == AppManifest.FullyInstalled && e["verify"]?.GetValue<bool>() != true) continue;
            lock (gate) queue.Add(new Job { AppId = id, Library = e["library"]?.GetValue<string>() ?? inst?.lib ?? "", Verify = e["verify"]?.GetValue<bool>() == true, Name = inst?.acf.Name ?? "" });
        }
    }

    async Task Worker()
    {
        while (!cts.IsCancellationRequested)
        {
            Job? j;
            lock (gate) { j = signedIn && queue.Count > 0 ? queue[0] : null; if (j != null) { queue.RemoveAt(0); current = j; } }
            if (j == null) { try { await Task.Delay(1000, cts.Token); } catch (OperationCanceledException) { } continue; }
            using var linked = CancellationTokenSource.CreateLinkedTokenSource(cts.Token, j.Cts.Token);
            try
            {
                await RunJob(j, linked.Token);
                j.State = "done";
                Log($"app {j.AppId}: {(j.Verify ? "verified" : "installed")}");
            }
            catch (OperationCanceledException) { j.State = cts.IsCancellationRequested ? "paused" : "cancelled"; }
            catch (Exception e)
            {
                j.State = "failed";
                j.Error = e is EngineException ? e.Message : $"{e.GetType().Name}: {e.Message}";
                Log($"app {j.AppId}: failed ({e.GetType().Name}); Valve's client can finish it from its record");
            }
            lock (gate) { current = null; finished[j.AppId] = j; }
            if (!cts.IsCancellationRequested) SaveQueue();
        }
    }

    async Task<List<SteamKit2.CDN.Server>> Servers()
    {
        var all = await content.GetServersForSteamPipe();
        var list = all.Where(s => s.Type is "SteamCache" or "CDN").OrderBy(s => s.WeightedLoad).ToList();
        return list.Count > 0 ? list : all.ToList();
    }

    static List<DepotFile> Files(DepotManifest m) => (m.Files ?? new List<DepotManifest.FileData>()).Select(f => new DepotFile(
        f.FileName, f.TotalSize, (int)f.Flags, f.FileHash, f.Chunks.Select(c => new ChunkRef(c.ChunkID ?? Array.Empty<byte>(), c.Offset, c.UncompressedLength, c.CompressedLength, c.Checksum)).ToList(), f.LinkTarget)).ToList();

    async Task<DepotManifest> GetManifest(uint appId, uint depot, ulong gid, byte[] key, List<SteamKit2.CDN.Server> servers, CancellationToken ct)
    {
        var code = await content.GetManifestRequestCode(depot, appId, gid, "public");
        if (code == 0) throw new EngineException(Codes.SteamError, $"Steam didn't give a manifest code for depot {depot}.");
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

    async Task RunJob(Job j, CancellationToken ct)
    {
        j.State = "preparing";
        var info = await FreshInfo(j.AppId) ?? throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
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
            var m = await GetManifest(j.AppId, d.Id, d.Manifest, k.DepotKey, servers, ct);
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
        acf.SizeOnDisk = (ulong)acf.Depots.Values.Sum(v => (decimal)v.Size);
        acf.BytesDownloaded = acf.BytesToDownload;
        acf.LastUpdated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        acf.Save(lib);
        try { Directory.Delete(OpenPaths.Staging(lib, j.AppId), true); } catch (IOException) { }
    }

    public JsonObject Downloads()
    {
        var a = new JsonArray();
        lock (gate)
        {
            IEnumerable<Job> all = (current != null ? new[] { current } : Array.Empty<Job>()).Concat(queue).Concat(finished.Values);
            foreach (var j in all)
                a.Add(new JsonObject { ["appid"] = j.AppId, ["name"] = j.Name, ["state"] = j.State, ["done"] = j.Done, ["total"] = j.Total, ["speed"] = j.Speed, ["error"] = j.Error.Length > 0 ? j.Error : null, ["verify"] = j.Verify });
        }
        return new JsonObject { ["downloads"] = a };
    }

    // ---------------------------------------------------------------- launching

    /// <summary>DRM-free games: how Hearth starts it itself ({direct, exe, args, cwd, os, prefix}). Others answer
    /// unsupported, and Hearth starts them through Valve's client (legitimate licence checks).</summary>
    public async Task<JsonObject> Launch(uint appId)
    {
        NeedSignIn();
        var inst = Installed(appId) ?? throw new EngineException(Codes.Unsupported, "Not installed.");
        if (!inst.acf.Installed || (inst.acf.StateFlags & AppManifest.UpdateRequired) != 0) throw new EngineException(Codes.Busy, "It's being installed or updated.");
        AppInfo? info; lock (gate) owned.TryGetValue(appId, out info);
        info ??= await FreshInfo(appId).WaitAsync(TimeSpan.FromSeconds(10));
        if (info == null) throw new EngineException(Codes.Unsupported, "Steam didn't give this app's details.");
        var gameDir = Path.Combine(inst.lib, "steamapps", "common", inst.acf.InstallDir);
        if (!AppManifest.SafeDirName(inst.acf.InstallDir) || !Directory.Exists(gameDir)) throw new EngineException(Codes.Unsupported, "Its folder is missing.");
        // Which build is installed: Linux when its depots are (the record's depots), else Windows.
        var linuxDepots = info.Depots.Where(d => d.OsList.Contains("linux", StringComparison.OrdinalIgnoreCase)).Select(d => d.Id).ToHashSet();
        var os = inst.acf.Depots.Keys.Any(linuxDepots.Contains) ? "linux" : info.Depots.Any(d => d.OsList.Contains("windows", StringComparison.OrdinalIgnoreCase)) ? "windows" : info.ChooseOs(false);
        var files = Directory.EnumerateFiles(gameDir, "*", new EnumerationOptions { RecurseSubdirectories = true, MaxRecursionDepth = 8, IgnoreInaccessible = true, AttributesToSkip = FileAttributes.ReparsePoint })
            .Take(200_000).Select(f => Path.GetRelativePath(gameDir, f));
        var decision = LaunchPolicy.Decide(info, os, files, exe =>
        {
            try { using var fs = File.OpenRead(Depot.Inside(gameDir, exe)); return LaunchPolicy.HasDrmWrapper(fs); } catch (Exception) { return true; }
        }, store.Launch(appId));
        Log($"app {appId}: {decision.Reason}");
        if (!decision.Direct || decision.Entry == null) throw new EngineException(Codes.Unsupported, decision.Reason);
        var exePath = Depot.Inside(gameDir, decision.Entry.Exe);
        if (!File.Exists(exePath)) throw new EngineException(Codes.Unsupported, "Its program is missing.");
        var cwd = decision.Entry.WorkingDir.Length > 0 && Depot.SafePath(decision.Entry.WorkingDir) != null ? Depot.Inside(gameDir, decision.Entry.WorkingDir + "/x")[..^2] : Path.GetDirectoryName(exePath)!;
        return new JsonObject
        {
            ["direct"] = true, ["exe"] = exePath, ["args"] = decision.Entry.Args, ["cwd"] = cwd, ["os"] = decision.Os,
            ["firstRun"] = decision.FirstRun, ["reason"] = decision.Reason, ["dir"] = gameDir,
            // Valve's client's Proton prefix for the game, so saves are shared with it.
            ["prefix"] = Path.Combine(inst.lib, "steamapps", "compatdata", appId.ToString(), "pfx"),
        };
    }

    public JsonObject LaunchResult(uint appId, bool ok)
    {
        store.RememberLaunch(appId, ok ? "direct" : "client");
        Log($"app {appId}: direct start {(ok ? "works" : "failed; it starts through Valve's client from now on")}");
        return new JsonObject { ["remembered"] = ok ? "direct" : "client" };
    }

    public JsonObject Running() => new() { ["apps"] = new JsonArray() };

    public void Release()
    {
        if (released) return;
        released = true;
        watchdog?.Dispose();
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
