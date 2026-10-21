using System.Text.Json;
using Nimbus.Host.Library;
using Nimbus.Host.Services;

namespace Nimbus.Host;

/// <summary>
/// Bridge calls that bring Ember's Game Mode level with Hearth OS's (docs/EMBER-PARITY.md): bug reports, collections,
/// per-game settings and power limit, Hearth Input (controller layouts), Beacon, Pause, screenshots, Steam friends'
/// actions. SystemBridge hands these over (see <see cref="Handles"/>); answers have the same shape as hearth-core's.
/// Called on the UI thread: slow work goes to the thread pool.
/// </summary>
internal sealed class GameModeBridge
{
    private static readonly HashSet<string> Methods = new()
    {
        "logs.save", "logs.send", "logs.sendToDeveloper", "app.exportLogs", "app.restart",
        "library.collection.create", "library.collection.rename", "library.collection.remove", "library.collection.set",
        "game.options.get", "game.options.set", "perf.setTdp", "perf.gamePowerForget",
        "beacon.selectMod", "beacon.open", "beacon.install", "game.pause", "game.unpause",
        "steam.downloads", "downloads.clearRecent", "screenshots.list", "screenshots.delete", "friends.action",
        "game.pauseInstall", "game.resumeInstall", "game.installFirst",
    };
    public static bool Handles(string method) => Methods.Contains(method) || method.StartsWith("input.");

    private readonly SystemBridge _bridge;
    private readonly Control _ui;
    private LibraryService Games => _bridge.Games;
    public TdpService Tdp { get; }
    /// <summary>Hearth Input's runtime (ControllerBridge feeds it).</summary>
    public Nimbus.Host.Input.HearthInput Input { get; } = new();

    public GameModeBridge(SystemBridge bridge, Control ui)
    {
        _bridge = bridge;
        _ui = ui;
        Tdp = new TdpService(() => Games.TdpWatts, w => Games.TdpWatts = w);
        Input.RunningId = () => Games.RunningId;
        Input.RunningDirs = () => Games.RunningDirs;
        Input.Pads = () => _bridge.Pad?.Connected ?? 0;
        Input.OsdText += text => { if (!_ui.IsDisposed) _ui.BeginInvoke((MethodInvoker)(() => Nimbus.Host.Input.Osd.Show(text))); };
    }

    private static string Version => typeof(SystemBridge).Assembly.GetName().Version?.ToString(3) ?? "0";

    public async Task<object?> DispatchAsync(string method, JsonElement a)
    {
        if (method.StartsWith("input."))
        {
            var running = Games.RunningId;
            return await Task.Run(() => Nimbus.Host.Input.InputLayouts.Call(method, a, running, Input));
        }
        switch (method)
        {
            // ------------------------------------------------ Settings › System › Troubleshooting
            case "logs.save":
            case "app.exportLogs":
            {
                var sections = ReportSections();
                var (path, copies, _) = await Task.Run(() => BugReport.Save(Version, sections));
                return new { path, copies };
            }
            case "logs.send":
            {
                var sections = ReportSections();
                var (path, copies, _) = await Task.Run(() => BugReport.Save(Version, sections));
                return new { path, copies };
            }
            case "logs.sendToDeveloper":
            {
                var sections = ReportSections();
                var note = Str(a, "note") ?? "";
                if (note.Length > 1000) note = note[..1000];
                return await Task.Run(() => BugReport.SendToDeveloperAsync(Version, note, sections));
            }
            case "app.restart":
                if (Games.RunningId is not null) SystemBridge.Log("app.restart with a game running: it keeps running");
                _ui.BeginInvoke((MethodInvoker)(() => (_ui as ShellForm)?.RestartForProfile()));
                return null;

            // ------------------------------------------------ collections
            case "library.collection.create": return Games.CollectionCreate(Str(a, "name") ?? "");
            case "library.collection.rename": Games.CollectionRename(Req(a, "id"), Str(a, "name") ?? ""); return null;
            case "library.collection.remove": Games.CollectionRemove(Req(a, "id")); return null;
            case "library.collection.set": Games.CollectionSet(Req(a, "id"), Str(a, "game") ?? "", Bool(a, "on")); return null;

            // ------------------------------------------------ Game settings
            case "game.options.get": return GameOptions(Req(a, "id"));
            case "game.options.set":
            {
                var id = Req(a, "id");
                int? tdp = null; bool? cloud = null;
                if (a.TryGetProperty("options", out var o) && o.ValueKind == JsonValueKind.Object)
                {
                    if (o.TryGetProperty("tdp", out var t) && t.ValueKind == JsonValueKind.Number) tdp = (int)Math.Round(t.GetDouble());
                    if (o.TryGetProperty("cloudSaves", out var c) && c.ValueKind is JsonValueKind.True or JsonValueKind.False) cloud = c.ValueKind == JsonValueKind.True;
                }
                Games.SetOptions(id, tdp, cloud);
                // The game in front gets its new limit now (or the usual one back).
                if (tdp is not null && Games.RunningId == id) _ = PowerFrontAsync(id, force: true);
                return GameOptions(id);
            }

            // ------------------------------------------------ Quick Access › Performance: power limit
            case "perf.setTdp":
            {
                int w = Int(a, "watts");
                if (w <= 0) throw new ArgumentException("Pick a power limit");
                var st = await Tdp.SetAsync(w, "Quick Access");
                // While a game with its own limit plays, the limit picked now is the one to go back to after it.
                if (_powerFor is not null) _powerBack = w;
                return st;
            }
            case "perf.gamePowerForget":
            {
                var id = Req(a, "id");
                Games.SetOptions(id, 0, null);
                if (Games.RunningId == id) await PowerRestoreAsync();
                return new { running = false };
            }

            // ------------------------------------------------ Beacon
            case "beacon.selectMod":
            {
                var mod = Str(a, "mod");
                return await Games.BeaconSelectModAsync(Req(a, "id"), mod is null or "@none" ? null : mod);
            }
            case "beacon.open": await Task.Run(Beacon.OpenApp); return new { ok = true };
            case "beacon.install": await Beacon.InstallAsync(); await Games.GetAsync(true); return new { ok = true };

            // ------------------------------------------------ Quick Access › Now playing: Pause
            case "game.pause":
            {
                var paused = Games.Pause(true);
                if (paused) _ = WatchPauseAsync(Games.RunningId);
                return new { paused };
            }
            case "game.unpause": _pausedId = null; Games.Pause(false); return new { paused = false };

            // ------------------------------------------------ Downloads, Screenshots, Friends
            case "steam.downloads":
                SteamWindows.StopHiding();
                SteamWindows.AllowSteamToFocus();
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("steam://open/downloads") { UseShellExecute = true })?.Dispose();
                return new { ok = true };
            case "downloads.clearRecent": return new { ok = true };   // Ember keeps no list of finished downloads
            // Downloads Ember runs itself (Epic, GOG): pause, resume, start now.
            case "game.pauseInstall": return new { ok = Games.Accounts.Installs.Pause(Req(a, "id")) };
            case "game.resumeInstall": return new { ok = Games.Accounts.Installs.Resume(Req(a, "id")) };
            case "game.installFirst": return new { ok = Games.Accounts.Installs.First(Req(a, "id")) };
            case "screenshots.list": return new { dir = Screenshots.Dir, list = await Task.Run(Screenshots.List) };
            case "screenshots.delete":
                await Task.Run(() => Screenshots.Delete(Req(a, "file")));
                return new { list = await Task.Run(Screenshots.List) };
            case "friends.action":
            {
                // Like hearth-core's friends::action: Steam's own chat, join or profile page opens in front.
                var (platform, id, what, server) = (Str(a, "platform") ?? "", Str(a, "id") ?? "", Str(a, "what") ?? "", Str(a, "server"));
                if (platform != "steam") throw new InvalidOperationException("That isn't possible for this friend yet.");
                string url = what switch
                {
                    "message" when id.Length > 0 && id.All(char.IsAsciiDigit) => "steam://friends/message/" + id,
                    "join" when server is { Length: > 0 } s && s.All(c => char.IsAsciiDigit(c) || c is '.' or ':') => "steam://connect/" + server,
                    "join" => throw new InvalidOperationException("Join them from Steam's chat: open Message."),
                    "profile" when id.Length > 0 && id.All(char.IsAsciiDigit) => "steam://url/SteamIDPage/" + id,
                    _ => throw new InvalidOperationException("That isn't possible for this friend yet."),
                };
                SteamWindows.StopHiding();
                SteamWindows.AllowSteamToFocus();
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(url) { UseShellExecute = true })?.Dispose();
                return new { open = url };
            }
        }
        throw new NotSupportedException($"Unknown method: {method}");
    }

    /// <summary>game.options.get: {id, kind, options, ...} like Hearth's; kind "windows" (Steam's, the stores' and
    /// your own games: the controller layout, power limit and cloud saves are Ember's) or "beacon".</summary>
    private object GameOptions(string id)
    {
        var g = Games.Find(id) ?? throw new InvalidOperationException("That game isn't in the library anymore.");
        var o = Games.Options(id);
        bool cloudSaves = g.CanUninstall && (g.Source == "epic" || g.Source == "gog");
        return new
        {
            id,
            kind = g.Source == "beacon" ? "beacon" : "windows",
            options = new { tdp = o.Tdp, powerLabel = "", cloudSaves = o.CloudSaves, fpsLimit = 0, proton = "", args = "", env = "", mangohud = false, gamemode = false, fsr = false, hhdPower = new { } },
            protonChoices = Array.Empty<object>(),
            gamemode = false,
            canResetPrefix = false,
            tdp = Tdp.Supported,
            cloudSaves,
            source = g.Source,
        };
    }

    // ------------------------------------------------------------------ Pause

    private string? _pausedId;

    /// <summary>A paused game carries on by itself once its window is back in front (the overlay closed, or you
    /// switched to it): a frozen window in front would only look stuck.</summary>
    private async Task WatchPauseAsync(string? id)
    {
        if (id is null) return;
        _pausedId = id;
        while (_pausedId == id && Games.RunningId == id)
        {
            await Task.Delay(500);
            if (_pausedId != id || Games.RunningId != id) break;
            if (Input.IsGameWindow(GetForegroundWindow()))
            {
                _pausedId = null;
                try { Games.Pause(false); } catch { }
                SystemBridge.Log($"game {id} back in front: unpaused");
                _bridge.Event("game.paused", new { paused = false });
                return;
            }
        }
        if (_pausedId == id) _pausedId = null;
    }
    [System.Runtime.InteropServices.DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();

    // ------------------------------------------------------------------ per-game power
    //
    // Like Hearth's game_power_front: a game's own limit goes on when it comes to the front (started, or back from
    // Quick Resume) and the usual one comes back when it leaves (closed, or put aside).

    private string? _powerFor;
    private int _powerBack;

    /// <summary>The game coming to the front gets its own power limit, if it has one.</summary>
    public async Task PowerFrontAsync(string id, bool force = false)
    {
        try
        {
            var w = Games.Options(id).Tdp;
            if (w <= 0) { if (_powerFor is not null && _powerFor != id) await PowerRestoreAsync(); else if (force) await PowerRestoreAsync(); return; }
            if (!Tdp.Supported) return;
            if (_powerFor == id && !force) return;
            if (_powerFor is null) _powerBack = Games.TdpWatts;   // 0: Ember never set one (nothing to put back)
            await Tdp.SetAsync(w, "game " + id);
            _powerFor = id;
        }
        catch (Exception ex)
        {
            _bridge.Event("osd", new { kind = "msg", icon = "bolt", text = "Couldn't set this game's power limit: " + ex.Message, ms = 6000 });
        }
    }

    /// <summary>The usual power limit back (when a game with its own left the front).</summary>
    public async Task PowerRestoreAsync()
    {
        if (_powerFor is null) return;
        var back = _powerBack;
        _powerFor = null; _powerBack = 0;
        try
        {
            if (back > 0) await Tdp.SetAsync(back, "usual");
            else SystemBridge.Log("power: no usual limit to put back (Ember hadn't set one)");
        }
        catch (Exception ex) { SystemBridge.Log("power: couldn't put the usual limit back: " + ex.Message); }
    }

    /// <summary>For Quick Access › Performance (perf.stats): the game in front and whether it keeps its own limit.</summary>
    public object? GamePower()
    {
        var id = Games.RunningId;
        if (id is null) return null;
        var w = Games.Options(id).Tdp;
        return new { id, saved = w > 0, label = w > 0 ? $"{w} W" : "", tdp = w };
    }

    // ------------------------------------------------------------------ the bug report's Ember sections

    private List<(string, string)> ReportSections()
    {
        var json = new JsonSerializerOptions(JsonSerializerDefaults.Web) { WriteIndented = true };
        string J(object? o) { try { return JsonSerializer.Serialize(o, json); } catch (Exception ex) { return "(" + ex.Message + ")"; } }
        var list = new List<(string, string)>();
        list.Add(("Ember settings", J(new
        {
            shell = Program.IsShell, startMode = Games.StartMode, desktopStyle = Games.DesktopStyle, steamBackground = Games.SteamBackground,
            quickResume = Games.QuickResumeOn, controllerMouse = Games.ControllerMouse, autoUpdate = Games.AutoUpdateCheck, overlayHoldView = Games.OverlayHoldView,
        })));
        list.Add(("Quick Resume and the running game", J(new { state = Games.QuickState(), runningTitle = Games.RunningTitle, since = Games.RunningSince })));
        list.Add(("Controllers and Hearth Input", J(new { xinput = _bridge.Pad?.Connected ?? 0, mouseOn = _bridge.Pad?.MouseActive ?? false, input = Input.Status() })));
        list.Add(("Power", J(new { tdp = Tdp.Status(), elevated = TdpService.IsElevated() })));
        string? steam = null;
        try { steam = SteamScanner.SteamPath(); } catch { }
        list.Add(("Other programs", J(new
        {
            steam, steamRunning = SafeBool(SteamScanner.IsRunning), beacon = Beacon.Cli(), beaconApp = Beacon.App(),
            hidhide = Nimbus.Host.Input.HearthInput.HidHideInstalled(),
            handheldCompanion = System.Diagnostics.Process.GetProcessesByName("HandheldCompanion").Length > 0,
            webView2 = SafeStr(() => Microsoft.Web.WebView2.Core.CoreWebView2Environment.GetAvailableBrowserVersionString()),
        })));
        return list;
    }
    private static bool SafeBool(Func<bool> f) { try { return f(); } catch { return false; } }
    private static string? SafeStr(Func<string?> f) { try { return f(); } catch { return null; } }

    // ------------------------------------------------------------------ args

    private static string? Str(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;
    private static string Req(JsonElement a, string k) => Str(a, k) is { Length: > 0 } s ? s : throw new ArgumentException($"Missing '{k}'");
    private static bool Bool(JsonElement a, string k) => a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.True;
    private static int Int(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? (int)Math.Round(v.GetDouble()) : 0;
}
