using System.Text.Json;
using Nimbus.Host.Services;
using Windows.Devices.Radios;

namespace Nimbus.Host;

/// <summary>
/// Request/response bridge for system settings.
/// UI sends   {type:"call", id, method, args}
/// Host sends {type:"result", id, ok, value | error}
///        and {type:"event", name, data} for pushes (status, bt.found, bt.scanDone, bt.pin).
/// </summary>
public sealed class SystemBridge : IDisposable
{
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);

    private readonly Control _ui;
    private readonly Action<string> _postJson;
    private readonly RadioService _radios = new();
    private readonly WifiService _wifi = new();
    private readonly BluetoothService _bt = new();
    private readonly AudioService _audio = new();
    private readonly DisplayService _display = new();
    private readonly PowerService _power = new();
    private readonly PerfService _perf = new();
    /// <summary>Other pages (the in-game overlay) that also receive events.</summary>
    private readonly List<Action<string>> _listeners = new();
    public void AddListener(Action<string> post) => _listeners.Add(post);
    public Nimbus.Host.Library.LibraryService Games { get; } = new();
    public Nimbus.Host.Updates.UpdateService Updater { get; }
    /// <summary>Set by the window once the controller bridge is running.</summary>
    public Input.ControllerBridge? Pad { get; set; }
    private readonly System.Windows.Forms.Timer _updateTimer = new() { Interval = 6 * 60 * 60 * 1000 };
    private readonly System.Windows.Forms.Timer _statusDebounce = new() { Interval = 400 };
    private readonly System.Windows.Forms.Timer _batteryPoll = new() { Interval = 30_000 };
    private readonly System.Windows.Forms.Timer _accountsDebounce = new() { Interval = 300 };

    // Windows has no public airplane-mode API, so we emulate it: remember the radios, switch them off.
    private bool _airplane;
    private (bool Wifi, bool Bt)? _beforeAirplane;

    public SystemBridge(Control ui, Action<string> postJson)
    {
        _ui = ui;
        _postJson = postJson;

        _statusDebounce.Tick += async (_, _) => { _statusDebounce.Stop(); await PushStatusAsync(); };
        _batteryPoll.Tick += async (_, _) => await PushStatusAsync();
        _batteryPoll.Start();

        _radios.Changed += StatusChangedSoon;
        _wifi.Changed += StatusChangedSoon;
        _bt.Found += d => Event("bt.found", d);
        _bt.ScanDone += () => Event("bt.scanDone", new { });
        _bt.Pin += (pin, confirm) => Event("bt.pin", new { pin, confirm });

        Updater = new Nimbus.Host.Updates.UpdateService(() => Games.AutoUpdateCheck);
        Updater.Changed += () => Event("update", Updater.Status());
        _updateTimer.Tick += async (_, _) => { if (Games.AutoUpdateCheck) await Updater.CheckAsync(); };
        _updateTimer.Start();

        // Store accounts: tell the UI (debounced) when an account or its game list changes.
        _accountsDebounce.Tick += (_, _) => { _accountsDebounce.Stop(); Event("accounts.changed", new { }); };
        Games.Accounts.Changed += () =>
        {
            if (!_ui.IsDisposed) _ui.BeginInvoke((MethodInvoker)(() => { _accountsDebounce.Stop(); _accountsDebounce.Start(); }));
        };
        Games.InstallFinished += (id, title, error) => Event("install.done", new { id, title, error });

        Games.GameStarted += id => { Log($"game started {id}"); Event("game.started", new { id }); };
        Games.GameStartTimedOut += id => { Log($"game start timed out {id}"); Event("game.timeout", new { id }); };
        Games.GameExited += g =>
        {
            Log($"game exited {g.Id}");
            Event("game.exited", g);
            if (!_ui.IsDisposed) _ui.BeginInvoke((MethodInvoker)BringShellToFront);
        };
    }

    /// <summary>Starts things that should run alongside the shell (Steam in the background).</summary>
    public async void StartBackgroundServices()
    {
        try { await Games.StartBackgroundServicesAsync(); Log("background services started"); }
        catch (Exception ex) { Log("background services FAILED " + ex.Message); }

        // First update check shortly after startup (not right away, so it doesn't slow down sign-in).
        await Task.Delay(TimeSpan.FromSeconds(20));
        if (Games.AutoUpdateCheck) { var s = await Updater.CheckAsync(); Log($"update check: {s.State} (current {s.Current}, latest {s.Latest?.Version ?? "-"})"); }
    }

    /// <summary>Windows only lets the foreground app steal focus, so briefly go topmost to get back in front.</summary>
    private void BringShellToFront()
    {
        if (_ui.FindForm() is not { } form) return;
        if (form.WindowState == FormWindowState.Minimized) form.WindowState = FormWindowState.Normal;
        form.TopMost = true;
        form.TopMost = false;
        form.Activate();
    }

    // ------------------------------------------------------------- plumbing

    /// <summary>Handles a call from a page. <paramref name="reply"/>: where the result goes (default: the main UI).</summary>
    public async void Handle(JsonElement m, Action<string>? reply = null)
    {
        int id = m.GetProperty("id").GetInt32();
        string method = m.GetProperty("method").GetString() ?? "";
        JsonElement args = m.TryGetProperty("args", out var a) ? a : default;
        var sw = System.Diagnostics.Stopwatch.StartNew();
        try
        {
            var value = await DispatchAsync(method, args);
            Log($"{method} ok {sw.ElapsedMilliseconds}ms {Summarize(value)}");
            Post(new { type = "result", id, ok = true, value }, reply);
        }
        catch (Exception ex)
        {
            Log($"{method} FAILED {sw.ElapsedMilliseconds}ms {ex.GetType().Name}: {ex.Message}");
            Post(new { type = "result", id, ok = false, error = ex.Message }, reply);
        }
    }

    // bridge.log next to the exe: one line per call (method, time, result size or error). No passwords.
    private static readonly object LogLock = new();
    private static readonly string LogPath = Path.Combine(AppContext.BaseDirectory, "bridge.log");
    public static void Log(string line)
    {
        try { lock (LogLock) File.AppendAllText(LogPath, $"{DateTime.Now:HH:mm:ss.fff} {line}{Environment.NewLine}"); }
        catch { /* logging is best effort */ }
    }
    private static string Summarize(object? v) => v switch
    {
        null => "",
        System.Collections.ICollection c => $"[{c.Count} items]",
        _ => v.GetType().Name,
    };

    private void Post(object msg, Action<string>? to = null)
    {
        var json = JsonSerializer.Serialize(msg, Json);
        if (_ui.IsDisposed) return;
        var target = to ?? _postJson;
        if (_ui.InvokeRequired) _ui.BeginInvoke((MethodInvoker)(() => target(json)));
        else target(json);
    }

    /// <summary>Events go to the main UI and every other page (the overlay).</summary>
    public void Event(string name, object data)
    {
        var json = JsonSerializer.Serialize(new { type = "event", name, data }, Json);
        void Send() { _postJson(json); foreach (var l in _listeners) { try { l(json); } catch { } } }
        if (_ui.IsDisposed) return;
        if (_ui.InvokeRequired) _ui.BeginInvoke((MethodInvoker)Send); else Send();
    }

    private void StatusChangedSoon()
    {
        if (_ui.IsDisposed) return;
        _ui.BeginInvoke((MethodInvoker)(() => { _statusDebounce.Stop(); _statusDebounce.Start(); }));
    }

    private async Task PushStatusAsync()
    {
        try { Event("status", await StatusAsync()); } catch { /* best effort */ }
    }

    // ------------------------------------------------------------- methods

    private async Task<object?> DispatchAsync(string method, JsonElement a)
    {
        switch (method)
        {
            case "host.info":
                return new
                {
                    platform = "windows",
                    name = "Windows 11 PC",
                    host = "Ember (WebView2)",
                    version = typeof(SystemBridge).Assembly.GetName().Version?.ToString(3),
                    session = Program.IsShell,
                    user = Environment.UserName,
                };
            case "sys.status": return await StatusAsync();

            case "radios.set":
                await _radios.SetAsync(Req(a, "kind") == "wifi" ? RadioKind.WiFi : RadioKind.Bluetooth, Bool(a, "on"));
                if (Bool(a, "on")) _airplane = false;
                return await StatusAsync();
            case "airplane.set":
                await SetAirplaneAsync(Bool(a, "on"));
                return await StatusAsync();

            case "wifi.scan": return await _wifi.ScanAsync();
            case "wifi.connect":
            {
                var (ok, error) = await _wifi.ConnectAsync(Req(a, "ssid"), Str(a, "password"));
                return new { ok, error };
            }
            case "wifi.disconnect": await _wifi.DisconnectAsync(); return null;
            case "wifi.forget": await _wifi.ForgetAsync(Req(a, "ssid")); return null;

            case "bt.paired": return await _bt.PairedAsync();
            case "bt.scanStart":
                if (await _radios.GetAsync(RadioKind.Bluetooth) != true) throw new InvalidOperationException("Bluetooth is off");
                _bt.StartScan(Int(a, "seconds", 15));
                return null;
            case "bt.scanStop": _bt.StopScan(); return null;
            case "bt.pair":
            {
                var (ok, error) = await _bt.PairAsync(Req(a, "id"));
                return new { ok, error };
            }
            case "bt.confirmPin": _bt.ConfirmPin(Bool(a, "accept")); return null;
            case "bt.unpair":
            {
                var (ok, error) = await _bt.UnpairAsync(Req(a, "id"));
                return new { ok, error };
            }

            case "audio.get": return _audio.Get();
            case "audio.setVolume": _audio.SetVolume(Int(a, "v")); return null;
            case "audio.setMute": _audio.SetMute(Bool(a, "m")); return null;
            case "audio.setDefault": _audio.SetDefault(Req(a, "id")); return _audio.Get();

            case "display.get": return await _display.GetAsync(_ui);
            case "display.setBrightness": await _display.SetBrightnessAsync(Int(a, "v")); return null;
            case "display.setMode":
                await _display.SetModeAsync(_ui, Int(a, "w"), Int(a, "h"), Int(a, "hz"));
                await Task.Delay(800);   // let Windows and the window settle before reading back
                return await _display.GetAsync(_ui);
            case "display.setScale":
                await _display.SetScaleAsync(_ui, Int(a, "v"));
                await Task.Delay(500);
                return await _display.GetAsync(_ui);

            case "power.get": return _power.Get();
            case "power.setMode": _power.SetMode(Req(a, "mode")); return _power.Get();

            case "library.get": return await Games.GetAsync(Bool(a, "rescan"));
            case "library.setFav": Games.SetFavorite(Req(a, "id"), Bool(a, "on")); return null;
            case "library.setHidden": Games.SetHidden(Req(a, "id"), Bool(a, "on")); return null;
            case "library.setSources":
            {
                var src = new Dictionary<string, bool>();
                foreach (var k in new[] { "steam", "epic", "gog", "xbox" })
                    if (a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v)) src[k] = v.ValueKind == JsonValueKind.True;
                await Games.SetSourcesAsync(src);
                return await Games.GetAsync(false);
            }
            case "library.addManual":
            {
                using var dlg = new OpenFileDialog
                {
                    Title = "Add a game to Ember",
                    Filter = "Games and shortcuts (*.exe;*.lnk;*.url)|*.exe;*.lnk;*.url|All files (*.*)|*.*",
                    CheckFileExists = true,
                    DereferenceLinks = false,
                };
                if (dlg.ShowDialog(_ui.FindForm()) != DialogResult.OK) return null;
                return await Games.AddManualAsync(dlg.FileName);
            }
            case "library.remove": await Games.RemoveManualAsync(Req(a, "id")); return null;
            case "game.launch": return new { tracked = await Games.LaunchAsync(Req(a, "id")) };
            case "game.install": await Games.InstallAsync(Req(a, "id")); return null;
            case "library.downloads": return await Games.DownloadsAsync();
            case "library.setSteamBackground": await Games.SetSteamBackgroundAsync(Bool(a, "on")); return null;
            case "game.close": return new { ok = await Games.CloseRunningAsync() };
            case "game.cancelInstall": return new { ok = Games.CancelInstall(Req(a, "id")) };
            case "game.uninstall": await Games.UninstallAsync(Req(a, "id")); return await Games.GetAsync(false);

            case "accounts.status": return new { accounts = Games.AccountsStatus(), installRoot = Games.Accounts.InstallRoot };
            case "accounts.signIn":
            {
                var store = Req(a, "store");
                if (store == "steam")
                {
                    // Steam signs in through its own window.
                    Library.SteamWindows.StopHiding();
                    Library.SteamWindows.AllowSteamToFocus();
                    System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("steam://open/main") { UseShellExecute = true })?.Dispose();
                    return new { ok = true };
                }
                return new { ok = await Games.Accounts.SignInAsync(store, _ui.FindForm()) };
            }
            case "accounts.signOut": await Games.Accounts.SignOutAsync(Req(a, "store")); return null;
            case "accounts.refresh": await Games.Accounts.RefreshAsync(Req(a, "store")); return null;
            case "accounts.pickFolder":
            {
                using var dlg = new FolderBrowserDialog
                {
                    Description = "Choose where Ember installs Epic and GOG games",
                    UseDescriptionForTitle = true,
                    SelectedPath = Games.Accounts.InstallRoot,
                    ShowNewFolderButton = true,
                };
                if (dlg.ShowDialog(_ui.FindForm()) == DialogResult.OK) Games.Accounts.InstallRoot = dlg.SelectedPath;
                return new { installRoot = Games.Accounts.InstallRoot };
            }

            case "profile.get": return Games.Profile();
            case "profile.set": return Games.SetProfile(Str(a, "name"), Str(a, "status"), Str(a, "avatar"), Str(a, "accent"), Str(a, "background"));
            case "profile.pickImage":
            {
                var kind = Req(a, "kind");
                using var dlg = new OpenFileDialog
                {
                    Title = kind == "avatar" ? "Choose a profile picture" : "Choose a background picture",
                    Filter = "Pictures (*.png;*.jpg;*.jpeg;*.webp;*.gif)|*.png;*.jpg;*.jpeg;*.webp;*.gif",
                    CheckFileExists = true,
                };
                if (dlg.ShowDialog(_ui.FindForm()) != DialogResult.OK) return null;
                return Games.SetProfileImage(kind, dlg.FileName);
            }

            case "update.status": return Updater.Status();
            case "update.check": return await Updater.CheckAsync();
            case "update.download": _ = Updater.DownloadAsync(); return Updater.Status();
            case "update.install":
                _ui.BeginInvoke((MethodInvoker)(() =>
                {
                    try { (_ui as ShellForm)?.ExitForUpdate(Updater); }
                    catch (Exception ex) { Event("update", Updater.Status() with { Error = ex.Message }); }
                }));
                return null;
            case "update.setAuto": Games.AutoUpdateCheck = Bool(a, "on"); return Updater.Status();

            case "perf.stats":
            {
                var p = await Task.Run(_perf.Sample);
                var since = Games.RunningSince;
                return new
                {
                    cpu = p.Cpu, gpu = p.Gpu, ramUsed = p.RamUsedGb, ramTotal = p.RamTotalGb, vramUsed = p.VramUsedGb,
                    battery = _power.Battery(),
                    runningId = Games.RunningId,
                    runningSince = since is { } t ? new DateTimeOffset(DateTime.SpecifyKind(t, DateTimeKind.Utc)).ToUnixTimeMilliseconds() : (long?)null,
                };
            }

            case "overlay.settings": return OverlaySettings();
            case "overlay.setHoldView":
                Games.OverlayHoldView = Bool(a, "on");
                if (Pad is not null) Pad.HoldViewOpensOverlay = Games.OverlayHoldView;
                return OverlaySettings();
            case "overlay.setGameBarButton":
                Overlay.GameBarButton.Set(Bool(a, "on"));
                return OverlaySettings();

            case "pad.toggleMouse": return new { mouseOn = Pad?.ToggleMouse() ?? false };
            case "pad.status":
                return new { connected = Pad?.Connected ?? 0, mouseMode = Games.ControllerMouse, mouseOn = Pad?.MouseActive ?? false };
            case "pad.setMouseMode":
            {
                var mode = Req(a, "mode");
                if (mode is not ("off" or "steam" or "always")) throw new ArgumentException("Unknown mode");
                Games.ControllerMouse = mode;
                Pad?.SetMouseMode(mode);
                return new { connected = Pad?.Connected ?? 0, mouseMode = mode, mouseOn = Pad?.MouseActive ?? false };
            }

            case "shell.status":
                return new
                {
                    running = Program.IsShell,
                    registered = ShellRegistration.IsNimbus(),
                    value = ShellRegistration.Current(),
                    exe = Environment.ProcessPath,
                };
            case "shell.register":
                ShellRegistration.Register(Environment.ProcessPath!);
                return new { registered = ShellRegistration.IsNimbus() };
            case "shell.unregister":
                ShellRegistration.Unregister();
                return new { registered = ShellRegistration.IsNimbus() };
            case "shell.returnToDesktop":
                _ui.BeginInvoke((MethodInvoker)(() => (_ui as ShellForm)?.ReturnToDesktop()));
                return null;

            default: throw new NotSupportedException($"Unknown method: {method}");
        }
    }

    private object OverlaySettings() => new
    {
        holdView = Games.OverlayHoldView,
        gameBarButton = Overlay.GameBarButton.Get(),
        hotkey = "Ctrl+Alt+Q",
    };

    private async Task SetAirplaneAsync(bool on)
    {
        if (on)
        {
            bool wifi = await _radios.GetAsync(RadioKind.WiFi) == true;
            bool bt = await _radios.GetAsync(RadioKind.Bluetooth) == true;
            _beforeAirplane = (wifi, bt);
            if (wifi) await _radios.SetAsync(RadioKind.WiFi, false);
            if (bt) await _radios.SetAsync(RadioKind.Bluetooth, false);
            _bt.StopScan();
        }
        else
        {
            var before = _beforeAirplane ?? (true, true);
            if (before.Wifi && await _radios.GetAsync(RadioKind.WiFi) is not null) await _radios.SetAsync(RadioKind.WiFi, true);
            if (before.Bt && await _radios.GetAsync(RadioKind.Bluetooth) is not null) await _radios.SetAsync(RadioKind.Bluetooth, true);
        }
        _airplane = on;
    }

    private async Task<object> StatusAsync()
    {
        bool? wifiOn = null, btOn = null;
        try { wifiOn = await _radios.GetAsync(RadioKind.WiFi); } catch { }
        try { btOn = await _radios.GetAsync(RadioKind.Bluetooth); } catch { }
        if (wifiOn is null && await _wifi.HasAdapterAsync()) wifiOn = true; // radio API unavailable but adapter works

        var (ssid, bars) = wifiOn == true ? await _wifi.CurrentAsync() : (null, 0);
        if (_airplane && (wifiOn == true || btOn == true)) _airplane = false; // turned back on elsewhere

        object? audio = null;
        try { var s = _audio.Get(); audio = new { volume = s.Volume, muted = s.Muted }; } catch { }

        int? brightness = null;
        try { brightness = await _display.GetBrightnessAsync(); } catch { }

        return new
        {
            wifi = new { available = wifiOn is not null, on = wifiOn == true, ssid, bars },
            bt = new { available = btOn is not null, on = btOn == true },
            airplane = _airplane,
            battery = _power.Battery(),
            audio,
            display = new { supported = brightness is not null, brightness = brightness ?? 0 },
            powerMode = _power.Mode(),
        };
    }

    // ------------------------------------------------------------- args

    private static string? Str(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

    private static string Req(JsonElement a, string k) =>
        Str(a, k) ?? throw new ArgumentException($"Missing '{k}'");

    private static bool Bool(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.True;

    private static int Int(JsonElement a, string k, int fallback = 0) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number
            ? (int)Math.Round(v.GetDouble()) : fallback;

    public void Dispose()
    {
        _statusDebounce.Dispose();
        _batteryPoll.Dispose();
        _accountsDebounce.Dispose();
        _bt.Dispose();
        Games.Dispose();
        _updateTimer.Dispose();
    }
}
