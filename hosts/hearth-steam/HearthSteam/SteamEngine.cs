using System.Text.Json.Nodes;
using HearthSteam.Vendor;
using OpenSteamClient.Logging;
using OpenSteamworks;
using OpenSteamworks.Data;
using OpenSteamworks.Data.Enums;
using OpenSteamworks.Data.Structs;
using OpenSteamworks.Helpers;
using OpenSteamworks.Native;
using QRCoder;

namespace HearthSteam;

/// <summary>Valve's steamclient.so, loaded through OpenSteamworks with no Steam UI. Everything that touches
/// licences, DRM, downloads, cloud saves and Proton is steamclient's own doing; this only asks it.</summary>
public sealed class SteamEngine : IEngine, IDisposable
{
    readonly string home, dataDir, stateDir;
    readonly ILoggerFactory logs = new ConsoleLoggerFactory();
    readonly object gate = new();
    readonly CancellationTokenSource cts = new();
    ISteamClient? client;
    string state = "starting", reason = "", userSteam = "", clientDir = "";
    long pinned;
    (string text, int percent)? progress;
    AuthSession? auth;
    JsonObject signIn = new() { ["state"] = "idle" };
    DownloadsHelper.DownloadChangedEventArgs? lastDownload;
    Timer? watchdog;
    public event Action? Released;

    public SteamEngine(string home)
    {
        this.home = home;
        var data = Environment.GetEnvironmentVariable("XDG_DATA_HOME");
        var st = Environment.GetEnvironmentVariable("XDG_STATE_HOME");
        dataDir = Path.Combine(string.IsNullOrEmpty(data) ? Path.Combine(home, ".local/share") : data, "hearth-steam");
        stateDir = Path.Combine(string.IsNullOrEmpty(st) ? Path.Combine(home, ".local/state") : st, "hearth-steam");
    }

    static void Log(string s) => Console.Error.WriteLine("hearth-steam: " + s);

    void Unavailable(string code, string why) { lock (gate) { state = "unavailable"; reason = why; } Log($"unavailable ({code}): {why}"); unavailableCode = code; }
    string unavailableCode = Codes.NotReady;

    /// <summary>The Steam client build this OpenSteamworks was made for (its embedded manifest).</summary>
    static string PinnedManifest()
    {
        var asm = typeof(ISteamClient).Assembly;
        var name = asm.GetManifestResourceNames().FirstOrDefault(n => n.EndsWith(SteamFiles.ManifestName + ".vdf", StringComparison.OrdinalIgnoreCase))
                   ?? throw new InvalidDataException("OpenSteamworks has no client manifest.");
        using var s = asm.GetManifestResourceStream(name)!;
        return new StreamReader(s).ReadToEnd();
    }

    public void Start() => Task.Run(async () =>
    {
        try { await StartAsync(); }
        catch (Exception e) { Unavailable(Codes.NotReady, "Steam's engine didn't start: " + e.Message); }
    });

    async Task StartAsync()
    {
        Bootstrap.RestoreDatalinks(home, stateDir);
        if (SteamFiles.ClassicRunning()) { Unavailable(Codes.ClassicRunning, "The normal Steam client is running."); return; }
        var user = SteamFiles.UserSteam(home);
        if (user == null) { Unavailable(Codes.NotReady, "Steam isn't installed. Open Steam once first."); return; }
        userSteam = user;
        var manifest = PinnedManifest();
        pinned = SteamFiles.ManifestVersion(manifest) ?? throw new InvalidDataException("Bad pinned manifest.");
        var own = Path.Combine(dataDir, "client");
        var found = new List<(string, long?)>();
        if (!user.Contains("/.var/app/")) found.Add((user, SteamFiles.ClientVersion(user)));
        found.Add((own, SteamFiles.ClientVersion(own)));
        var (dir, why) = SteamFiles.PickClient(pinned, found);
        if (dir == null)
        {
            if (Environment.GetEnvironmentVariable("HEARTH_STEAM_NO_BOOTSTRAP") == "1") { Unavailable(Codes.NotReady, why); return; }
            Log($"{why}; getting Steam's own build {pinned} from Valve");
            lock (gate) state = "bootstrapping";
            await Bootstrap.Run(manifest, own, user, (t, p) => { lock (gate) progress = (t, p); }, cts.Token);
            lock (gate) progress = null;
            dir = own;
        }
        if (dir == own) Bootstrap.LinkUserSteam(own, user);
        clientDir = dir;
        Bootstrap.ReExecWithEnvs(dir);   // returns only once Steam's libraries are on the path
        if (SteamFiles.ClassicRunning()) { Unavailable(Codes.ClassicRunning, "The normal Steam client is running."); return; }

        Bootstrap.SetEnvsForSteamLoad(dir);
        Directory.SetCurrentDirectory(dir);
        Bootstrap.TakeDatalinks(home, dir, stateDir);
        client = new SteamClientBuilder().WithNativeBackend(new NativeSteamClientCreateOptions
        {
            ClientDLLPath = Path.Combine(dir, "linux64", "steamclient.so"),
            ConnectionTypes = ConnectionType.NewClient,
            LoggingSettings = new LoggingSettings(logs),
            HookLoggingFunctions = false,
            EnableConCommandSystem = false,
            AutomaticCallbackPump = true,
            IsUIProcess = true,
        }).Build();
        var args = Environment.GetCommandLineArgs();
        client.IClientEngine.SetClientCommandLine(args.Length, args);
        client.DownloadsHelper.DownloadChanged += (_, e) => lastDownload = e;
        lock (gate) { state = "ready"; reason = ""; }
        Log($"ready (Steam client build {pinned} from {dir})");
        // The normal Steam client started anyway: let go at once, two clients on one library can damage it.
        watchdog = new Timer(_ => { if (SteamFiles.ClassicRunning()) { Log("the normal Steam client started; letting go"); Release(); } }, null, 5000, 5000);
        await TryCachedLogin();
    }

    // ---------------------------------------------------------------- sign-in

    string LastUserFile => Path.Combine(stateDir, "last-user.json");

    /// <summary>Signs in again with what steamclient itself remembered. Hearth keeps only the account name and
    /// SteamID (not secrets) to know whom to ask steamclient for.</summary>
    async Task TryCachedLogin()
    {
        try
        {
            if (client == null || !File.Exists(LastUserFile)) return;
            var o = JsonNode.Parse(File.ReadAllText(LastUserFile))!;
            var account = o["account"]!.GetValue<string>();
            if (!client.UserHelper.BHasCachedCredentials(account)) return;
            await client.UserHelper.LogOnWithCached(new CSteamID(o["steamId"]!.GetValue<ulong>()), account);
            AfterLogin();
        }
        catch (Exception e) { Log("couldn't sign in again by itself: " + e.GetType().Name); }
    }

    void AfterLogin()
    {
        if (client == null) return;
        Directory.CreateDirectory(stateDir);
        File.WriteAllText(LastUserFile, new JsonObject { ["account"] = client.UserHelper.LoggedInAccountName, ["steamId"] = client.UserHelper.SteamID.SteamID64 }.ToJsonString());
        ApplyCompat();
    }

    /// <summary>The user's Proton choices from their normal Steam, handed to steamclient.</summary>
    void ApplyCompat()
    {
        try
        {
            var cfg = Path.Combine(userSteam, "config", "config.vdf");
            if (client == null || !File.Exists(cfg)) return;
            foreach (var (app, tool) in SteamFiles.CompatMapping(File.ReadAllText(cfg)))
                client.IClientCompat.SpecifyCompatTool(app, tool, "", app == 0 ? ECompatToolPriority.Wildcard : ECompatToolPriority.AppForced);
        }
        catch (Exception e) { Log("couldn't copy Proton choices: " + e.Message); }
    }

    ISteamClient Ready(bool needUser = true)
    {
        lock (gate)
        {
            if (state == "unavailable") throw new EngineException(unavailableCode, reason);
            if (state != "ready" || client == null) throw new EngineException(Codes.NotReady, "Steam's engine is still starting.");
        }
        if (needUser && !client.UserHelper.LoggedIn) throw new EngineException(Codes.SignedOut, "Not signed in to Steam.");
        return client;
    }

    AuthSession NewAuth()
    {
        var c = Ready(false);
        if (c.UserHelper.LoggedIn) throw new EngineException(Codes.Busy, "Already signed in.");
        auth?.Dispose();
        var a = new AuthSession(c, logs);
        a.ChallengeUrl += url => SetSignIn("waiting", qr: url);
        a.Failed += r => SetSignIn("failed", error: $"Steam said no ({r}).");
        a.Token += async (account, token) =>
        {
            try { await c.UserHelper.LogOnWithToken(account, token); AfterLogin(); SetSignIn("done"); }
            catch (Exception e) { SetSignIn("failed", error: "Couldn't sign in: " + e.Message); }
        };
        auth = a;
        return a;
    }

    void SetSignIn(string st, string? needs = null, string? qr = null, string? error = null)
    {
        var o = new JsonObject { ["state"] = st };
        if (needs != null) o["needs"] = needs;
        if (error != null) o["error"] = error;
        if (qr != null)
        {
            o["qrUrl"] = qr;
            using var gen = new QRCodeGenerator();
            o["qrSvg"] = new SvgQRCode(gen.CreateQrCode(qr, QRCodeGenerator.ECCLevel.M)).GetGraphic(4);
        }
        lock (gate) signIn = o;
    }

    public async Task<JsonObject> SignInPassword(string user, string password, bool remember)
    {
        var a = NewAuth();
        var (r, needs) = await a.StartCredentials(user, password, remember);
        if (r != EResult.OK) { SetSignIn("failed", error: r == EResult.InvalidPassword ? "Wrong name or password." : $"Steam said no ({r})."); throw new EngineException(Codes.SteamError, r == EResult.InvalidPassword ? "Wrong name or password." : $"Steam said no ({r})."); }
        SetSignIn(needs == null ? "waiting" : "needs", needs: needs);
        return new JsonObject { ["needs"] = needs };
    }

    public async Task<JsonObject> SignInCode(string code, string kind)
    {
        if (auth == null) throw new EngineException(Codes.BadRequest, "No sign-in is waiting for a code.");
        var r = await auth.SubmitCode(code.Trim(), kind);
        if (r != EResult.OK) throw new EngineException(Codes.SteamError, r == EResult.InvalidLoginAuthCode || r == EResult.TwoFactorCodeMismatch ? "That code didn't work." : $"Steam said no ({r}).");
        SetSignIn("waiting");
        return new JsonObject();
    }

    public async Task<JsonObject> SignInQr()
    {
        var url = await NewAuth().StartQr();
        SetSignIn("waiting", qr: url);
        lock (gate) return (JsonObject)signIn.DeepClone();
    }

    public void SignInCancel() { auth?.Dispose(); auth = null; SetSignIn("idle"); }

    public Task<JsonObject> SignOut()
    {
        var c = Ready(false);
        if (c.UserHelper.LoggedIn) c.IClientUser.LogOff();
        try { File.Delete(LastUserFile); } catch (IOException) { }
        SetSignIn("idle");
        return Task.FromResult(new JsonObject());
    }

    // ---------------------------------------------------------------- status, library, downloads

    public JsonObject Status()
    {
        var o = new JsonObject();
        lock (gate)
        {
            o["engine"] = state;
            o["reason"] = reason;
            if (state == "unavailable") o["code"] = unavailableCode;
            o["build"] = pinned;
            if (progress is { } p) o["progress"] = new JsonObject { ["text"] = p.text, ["percent"] = p.percent };
            o["signIn"] = signIn.DeepClone();
        }
        var c = client;
        bool signed = false;
        try { signed = state == "ready" && c != null && c.UserHelper.LoggedIn; } catch (Exception) { }
        o["signedIn"] = signed;
        if (signed) { o["account"] = c!.UserHelper.LoggedInAccountName; o["steamId"] = c.UserHelper.SteamID.SteamID64.ToString(); }
        return o;
    }

    public JsonObject Library()
    {
        var c = Ready();
        var apps = new JsonArray();
        foreach (var app in c.UserHelper.GetSubscribedApps())
        {
            try
            {
                if (c.AppsHelper.GetAppType(app) != EAppType.Game) continue;
                apps.Add(new JsonObject
                {
                    ["appid"] = (uint)app,
                    ["name"] = c.AppsHelper.GetAppLocalizedName(app),
                    ["installed"] = c.AppManagerHelper.IsAppInstalled(app),
                    ["state"] = (uint)c.AppManagerHelper.GetAppState(app),
                });
            }
            catch (Exception) { }
        }
        return new JsonObject { ["apps"] = apps };
    }

    LibraryFolder_t FolderFor(ISteamClient c, string? path)
    {
        if (string.IsNullOrEmpty(path)) return 0;
        int n = c.IClientAppManager.GetNumLibraryFolders();
        for (int i = 0; i < n; i++)
            if (string.Equals(c.AppManagerHelper.GetLibraryFolderPath(i).TrimEnd('/'), path.TrimEnd('/'), StringComparison.Ordinal)) return i;
        return c.AppManagerHelper.AddLibraryFolder(path);
    }

    public JsonObject Install(uint appId, string? library)
    {
        var c = Ready();
        var err = c.AppManagerHelper.InstallApp(appId, FolderFor(c, library));
        if (err != EAppError.NoError) throw new EngineException(Codes.SteamError, $"Steam couldn't start the install ({err}).");
        return new JsonObject();
    }

    public JsonObject Uninstall(uint appId)
    {
        var c = Ready();
        var err = c.AppManagerHelper.UninstallApp(appId);
        if (err != EAppError.NoError) throw new EngineException(Codes.SteamError, $"Steam couldn't uninstall it ({err}).");
        return new JsonObject();
    }

    public JsonObject Update(uint appId)
    {
        var c = Ready();
        if (!c.AppManagerHelper.UpdateApp(appId)) throw new EngineException(Codes.SteamError, "Steam couldn't start the update.");
        return new JsonObject();
    }

    const EAppState Busy = EAppState.UpdateRunning | EAppState.UpdateQueued | EAppState.UpdateStarted | EAppState.UpdatePaused | EAppState.UpdateRequired;

    public JsonObject Downloads()
    {
        var c = Ready();
        var list = new JsonArray();
        foreach (var app in c.AppManagerHelper.GetInstalledApps())
        {
            var st = c.AppManagerHelper.GetAppState(app);
            if ((st & Busy) == 0) continue;
            var o = new JsonObject { ["appid"] = (uint)app, ["state"] = (uint)st, ["paused"] = (st & EAppState.UpdatePaused) != 0 };
            if (c.IClientAppManager.GetUpdateInfo(app, out var info))
            {
                o["downloaded"] = info.m_unBytesDownloaded; o["toDownload"] = info.m_unBytesToDownload;
                o["processed"] = info.m_unBytesProcessed; o["toProcess"] = info.m_unBytesToProcess;
            }
            list.Add(o);
        }
        var d = lastDownload;
        return new JsonObject
        {
            ["downloads"] = list,
            ["current"] = d == null || d.DownloadingAppID == 0 ? null : new JsonObject { ["appid"] = (uint)d.DownloadingAppID, ["rate"] = d.DownloadRate, ["paused"] = d.Paused, ["downloaded"] = d.TotalDownloaded, ["toDownload"] = d.TotalToDownload },
        };
    }

    // ---------------------------------------------------------------- playing

    public async Task<JsonObject> Launch(uint appId)
    {
        var c = Ready();
        // VAC-secured games always go through the normal client. Unknown app info counts as "can't tell": no.
        try { if (c.AppsHelper.GetAppData(appId, "common/category/category_8") == "1") throw new EngineException(Codes.Vac, "VAC-secured games start with the normal Steam client."); }
        catch (KeyNotFoundException) { }
        catch (EngineException) { throw; }
        catch (Exception) { throw new EngineException(Codes.Unsupported, "Steam has no details for this game yet."); }
        if (!c.AppManagerHelper.IsAppInstalled(appId)) throw new EngineException(Codes.BadRequest, "It isn't installed.");
        var options = c.AppsHelper.GetAvailableLaunchOptions(appId);
        try { await c.AppManagerHelper.LaunchApp(appId, options.Length > 0 ? (uint)options[0] : 0u, ELaunchSource.None); }
        catch (Exception e) { throw new EngineException(Codes.SteamError, e.Message); }
        return new JsonObject { ["launched"] = appId };
    }

    public JsonObject Running()
    {
        var c = Ready(false);
        var apps = new JsonArray();
        int n = c.IClientUser.NumGamesRunning();
        for (int i = 0; i < n; i++)
            apps.Add(new JsonObject { ["appid"] = (uint)c.IClientUser.GetRunningGameID(i).AppID, ["pid"] = c.IClientUser.GetRunningGamePID(i) });
        return new JsonObject { ["apps"] = apps };
    }

    public void Release()
    {
        watchdog?.Dispose();
        Released?.Invoke();
    }

    public void Dispose()
    {
        watchdog?.Dispose();
        cts.Cancel();
        auth?.Dispose();
        try { client?.Dispose(); } catch (Exception e) { Log("closing steamclient: " + e.Message); }
        client = null;
        Bootstrap.RestoreDatalinks(home, stateDir);
    }
}
