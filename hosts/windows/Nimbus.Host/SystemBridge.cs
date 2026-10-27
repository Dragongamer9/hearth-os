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
    /// <summary>The player counter's id for this PC: the same on every update and reinstall of Ember (a hash of
    /// Windows' MachineGuid, so the guid itself never leaves the PC). Empty if Windows doesn't have one.</summary>
    private static string CounterDeviceId()
    {
        try
        {
            using var key = Microsoft.Win32.RegistryKey.OpenBaseKey(Microsoft.Win32.RegistryHive.LocalMachine, Microsoft.Win32.RegistryView.Registry64)
                .OpenSubKey(@"SOFTWARE\Microsoft\Cryptography");
            var guid = key?.GetValue("MachineGuid") as string;
            if (string.IsNullOrWhiteSpace(guid)) return "";
            var hash = System.Security.Cryptography.SHA256.HashData(System.Text.Encoding.UTF8.GetBytes("hearth-counter:" + guid.Trim().ToLowerInvariant()));
            return Convert.ToHexString(hash, 0, 16).ToLowerInvariant();
        }
        catch { return ""; }
    }

    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);

    private readonly Control _ui;
    /// <summary>Who's playing: their own sign-ins, favorites and play time; parental controls.</summary>
    public Profiles Profiles { get; } = new(Stores.HelperTools.DataDir);
    private static readonly bool Picked = ConsumePicked();
    private static bool ConsumePicked() { try { if (File.Exists(Profiles.PickedFile)) { File.Delete(Profiles.PickedFile); return true; } } catch { } return false; }
    private DateTime? _playSince;
    private bool _playWarned;
    private readonly System.Windows.Forms.Timer _parentalTimer = new() { Interval = 20000 };
    private readonly Action<string> _postJson;
    private readonly RadioService _radios = new();
    private readonly WifiService _wifi = new();
    private readonly BluetoothService _bt = new();
    private readonly AudioService _audio = new();
    private readonly DisplayService _display = new();
    private readonly PowerService _power = new();
    private readonly PerfService _perf = new();
    /// <summary>Hearth Task Manager (mantel-tasks.html).</summary>
    private readonly TaskService _tasks = new();
    /// <summary>Hearth Files (mantel-files.html).</summary>
    public FileService Files { get; }
    /// <summary>Other pages (the in-game overlay) that also receive events.</summary>
    private readonly List<Action<string>> _listeners = new();
    public void AddListener(Action<string> post) => _listeners.Add(post);
    public Nimbus.Host.Library.LibraryService Games { get; } = new();
    public Nimbus.Host.Updates.UpdateService Updater { get; }
    /// <summary>Settings › Switch to Hearth OS.</summary>
    public Nimbus.Host.Switch.SwitchToHearth Switch { get; }
    /// <summary>Set by the window once the controller bridge is running (Hearth Input rides on it).</summary>
    public Input.ControllerBridge? Pad
    {
        get => _pad;
        set { _pad = value; if (value is not null) value.Mapper = _gm.Input; }
    }
    private Input.ControllerBridge? _pad;
    /// <summary>Game Mode's newer parts (bug reports, collections, per-game settings and power, Hearth Input, Beacon).</summary>
    private readonly GameModeBridge _gm;
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
        Files = new FileService((n, d) => Event(n, d));
        _gm = new GameModeBridge(this, ui);

        Updater = new Nimbus.Host.Updates.UpdateService(() => Games.AutoUpdateCheck);
        Switch = new Nimbus.Host.Switch.SwitchToHearth((n, d) => Event(n, d), Games);
        Updater.Changed += () => Event("update", Updater.Status());
        _updateTimer.Tick += async (_, _) => { if (Games.AutoUpdateCheck) await Updater.CheckAsync(); };
        _updateTimer.Start();

        // Store accounts: tell the UI (debounced) when an account or its game list changes.
        _accountsDebounce.Tick += (_, _) => { _accountsDebounce.Stop(); Event("accounts.changed", new { }); };
        Games.Accounts.Changed += () =>
        {
            OnUi((MethodInvoker)(() => { _accountsDebounce.Stop(); _accountsDebounce.Start(); }));
        };
        Games.InstallFinished += (id, title, error) => Event("install.done", new { id, title, error });
        _parentalTimer.Tick += (_, _) => ParentalTick();
        Library.Friends.Changed += () => Event("friends", new { });
        // Steam run hidden by Ember (Library/SteamControl.cs, docs/STEAM.md).
        Library.SteamControl.Event += (name, data) =>
        {
            switch (name)
            {
                case "rescan": _ = Task.Run(async () => { try { await Games.RescanAsync(); } catch { } Event("library.changed", new { }); }); break;
                case "accounts": Event("accounts.changed", new { }); break;
                case "steam.signedIn":
                    // Steam's own sign-in window finished: Ember comes back in front.
                    if (!_ui.IsDisposed) _ui.BeginInvoke((MethodInvoker)BringShellToFront);
                    Event(name, data ?? new { });
                    break;
                default: Event(name, data ?? new { }); break;
            }
        };

        Games.GameStarted += id => { Log($"game started {id}"); Event("game.started", new { id }); StartPlayClock(); };
        Games.QuickChanged += msg =>
        {
            Event("quickResume", Games.QuickState());
            if (msg is not null) Event("osd", new { kind = "msg", icon = "clock", text = msg, ms = 5000 });
        };
        Games.GameStartTimedOut += id => { Log($"game start timed out {id}"); Event("game.timeout", new { id }); };
        Games.GameExited += g =>
        {
            Log($"game exited {g.Id}");
            Event("game.exited", g);
            StopPlayClock();
            OnUi((MethodInvoker)(async () => await _gm.PowerRestoreAsync()));
            OnUi((MethodInvoker)BringShellToFront);
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

    /// <summary>Is a page message from Ember's own pages (https://nimbus.local/)? Calls can delete files, end
    /// processes and change the Windows shell, so a page from anywhere else (one that somehow got into Ember's
    /// windows) is never listened to.</summary>
    public static bool FromOwnPage(string? source) =>
        Uri.TryCreate(source, UriKind.Absolute, out var u) && u.Scheme == Uri.UriSchemeHttps
        && string.Equals(u.Host, "nimbus.local", StringComparison.OrdinalIgnoreCase);

    /// <summary>Handles a call from a page. <paramref name="reply"/>: where the result goes (default: the main UI).</summary>
    public async void Handle(JsonElement m, Action<string>? reply = null)
    {
        // async void: a malformed call must not throw from here (that ends Ember when it isn't the shell).
        int id; string method; JsonElement args;
        try
        {
            id = m.GetProperty("id").GetInt32();
            method = m.GetProperty("method").GetString() ?? "";
            args = m.TryGetProperty("args", out var a) ? a : default;
        }
        catch (Exception ex) { Log("bad call from a page: " + ex.Message); return; }
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
    /// <summary>Where bridge.log is (bug reports read it).</summary>
    public static string LogFile => LogPath;
    private static int _logWrites;
    public static void Log(string line)
    {
        try
        {
            lock (LogLock)
            {
                // Kept to a few MB, like Hearth's: the old one becomes bridge.log.old (checked now and then).
                if (_logWrites++ % 500 == 0 && File.Exists(LogPath) && new FileInfo(LogPath).Length > 4_000_000)
                    File.Move(LogPath, LogPath + ".old", overwrite: true);
                File.AppendAllText(LogPath, $"{DateTime.Now:HH:mm:ss.fff} {line}{Environment.NewLine}");
            }
        }
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
        if (_ui.InvokeRequired) OnUi(() => target(json));
        else target(json);
    }

    /// <summary>The volume keys in full Mantel (Explorer normally handles them): +1 / -1 steps of 5 %, 0 = mute.
    /// Returns what the volume bar shows.</summary>
    /// <summary>The volume now (0–100) and whether it's muted.</summary>
    public (int Volume, bool Muted) AudioState() { var s = _audio.Get(); return (s.Volume, s.Muted); }

    public string VolumeStep(int d)
    {
        var s = _audio.Get();
        if (d == 0) _audio.SetMute(!s.Muted);
        else { _audio.SetVolume(Math.Clamp(s.Volume + d * 5, 0, 100)); if (s.Muted) _audio.SetMute(false); }
        var n = _audio.Get();
        StatusChangedSoon();
        return n.Muted || n.Volume == 0 ? "🔇  Muted" : $"🔊  {n.Volume}%";
    }

    /// <summary>Events go to the main UI and every other page (the overlay).</summary>
    // ---------------------------------------------------------------- parental controls: play time a day
    private double SessionMinutes => _playSince is { } t ? (DateTime.UtcNow - t).TotalMinutes : 0;
    private void StartPlayClock()
    {
        if (_ui.IsDisposed) return;
        OnUi((() =>
        {
            // Another game was playing (put aside with Quick Resume to start this one): its time counts too.
            if (_playSince is not null) Profiles.AddPlayed(Profiles.Active, SessionMinutes);
            _playSince = DateTime.UtcNow; _playWarned = false;
            if (Profiles.Restricted(Profiles.Active)) { _parentalTimer.Start(); ParentalTick(); }
        }));
    }
    private void StopPlayClock()
    {
        if (_ui.IsDisposed) return;
        OnUi((() =>
        {
            _parentalTimer.Stop();
            if (_playSince is not null) Profiles.AddPlayed(Profiles.Active, SessionMinutes);
            _playSince = null;
        }));
    }
    private async void ParentalTick()
    {
        var (ok, _, left) = Profiles.CanPlay(Games.RunningId, SessionMinutes);
        if (left is null) return;
        if (!ok)
        {
            Log("parental: time is up");
            _parentalTimer.Stop();
            Event("parental.timeup", new { });
            Event("osd", new { kind = "msg", icon = "clock", text = "Play time's up for today. The game is closing.", ms = 6000 });
            await Task.Delay(5000);
            try { await Games.CloseRunningAsync(); }
            catch (Exception ex) { Log("parental: couldn't close the game: " + ex.Message); }   // async void: never throw
        }
        else if (left <= 5 && !_playWarned)
        {
            _playWarned = true;
            Event("osd", new { kind = "msg", icon = "clock", text = $"{left} minute{(left == 1 ? "" : "s")} of play time left today. Time to find a save point!", ms = 7000 });
        }
    }
    private void CheckCanPlay(string id)
    {
        var (ok, reason, _) = Profiles.CanPlay(id, SessionMinutes);
        if (!ok) throw new InvalidOperationException(reason ?? "Not now");
    }

    /// <summary>Gets Ember out of the way so the game picked from Quick Resume is in front.</summary>
    private void MinimizeShell() { OnUi((MethodInvoker)(() => { if (_ui.FindForm() is { } f) f.WindowState = FormWindowState.Minimized; })); }

    public void Event(string name, object data)
    {
        var json = JsonSerializer.Serialize(new { type = "event", name, data }, Json);
        void Send() { _postJson(json); foreach (var l in _listeners) { try { l(json); } catch { } } }
        if (_ui.IsDisposed) return;
        if (_ui.InvokeRequired) OnUi(Send); else Send();
    }

    /// <summary>Runs on the window's thread. Events come from background threads (radios, the network, games); while
    /// Ember closes the window can go away between the check and the call, which would end Ember from that thread.</summary>
    private void OnUi(MethodInvoker run)
    {
        try { if (!_ui.IsDisposed) _ui.BeginInvoke(run); }
        catch (Exception ex) when (ex is InvalidOperationException or ObjectDisposedException) { }
    }

    private void StatusChangedSoon()
    {
        OnUi(() => { _statusDebounce.Stop(); _statusDebounce.Start(); });
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
                    // Game Mode parts Ember has (docs/EMBER-PARITY.md). Not "coop", "seats" (Windows can't split one
                    // PC between players without third-party tools) or the Linux-only ones (remap through
                    // InputPlumber, Handheld Daemon, xodus, Nvidia drawing).
                    features = new[] { "beaconInstall", "desktopMode", "mantel", "quickResume", "profiles", "friends", "restartApp", "sendLogs", "exportLogs",
                                       "screenshots", "gameSettings", "hearthInput", "collections", "pause" },
                    user = Environment.UserName,
                    deviceId = CounterDeviceId(),
                };
            case "system.info": return SystemInfo.Get();
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
            // Hearth Settings › Sound › Microphone
            case "audio.input": return _audio.GetInput();
            case "audio.setInputVolume": _audio.SetInputVolume(Int(a, "v")); return null;

            // Hearth Settings (Mantel): Appearance › Night light / Reduce motion, Controller › Pointer speed,
            // Sign-in › Ask for my password after sleep, Power › Power button. Null values: Windows can't tell.
            case "desktop.get": return await Task.Run(DesktopSettings);
            case "desktop.set":
            {
                var key = Req(a, "key");
                JsonElement v = a.TryGetProperty("value", out var vv) ? vv : default;
                bool On() => v.ValueKind == JsonValueKind.True;
                var mantel = (_ui as ShellForm)?.Mantel;
                switch (key)
                {
                    case "nightLight":
                        Games.MantelNightLight = On(); Mantel.MantelHost.NightLightOn = On();
                        if (!_ui.IsDisposed) _ui.Invoke((MethodInvoker)(() => mantel?.ApplyNightLight()));
                        break;
                    case "reduceMotion":
                        Games.MantelReduceMotion = On(); Mantel.MantelHost.ReduceMotion = On();
                        OnUi((MethodInvoker)(() => mantel?.ApplyReduceMotion()));
                        break;
                    case "pointerSpeed":
                        if (v.ValueKind != JsonValueKind.Number) throw new ArgumentException("Pointer speed is a number");
                        Games.PointerSpeed = v.GetDouble();
                        if (Pad is not null) Pad.PointerSpeed = Games.PointerSpeed;
                        break;
                    case "lockOnWake": await Task.Run(() => Mantel.WinSettings.SetLockOnWake(On())); break;
                    case "powerPress": await Task.Run(() => Mantel.WinSettings.SetPowerPress(v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "")); break;
                    default: throw new ArgumentException("Unknown desktop setting");
                }
                return await Task.Run(DesktopSettings);
            }
            case "browser.status": return await Task.Run(Mantel.WinSettings.BrowserStatus);
            // Hearth Browser (Services/HearthBrowser.cs): open (a page in) it, setting it up first if needed.
            case "browser.hearth":
            {
                Services.HearthBrowser.Changed ??= () => Event("browser", Mantel.WinSettings.BrowserStatus());
                var url = Str(a, "url");
                if (url is not null && !Services.HearthBrowser.ValidUrl(url)) throw new ArgumentException("That link can't be opened.");
                bool game = !(Pad?.DesktopMode() ?? false);
                switch (Str(a, "action"))
                {
                    case "open" when Services.HearthBrowser.Exe() is not null:
                        await Task.Run(() => Services.HearthBrowser.Open(url, game));
                        break;
                    case "open":
                        _ = Services.HearthBrowser.SetupAsync().ContinueWith(_ => { try { if (Services.HearthBrowser.Exe() is not null) Services.HearthBrowser.Open(url, game); } catch (Exception ex) { Log("hearth browser: " + ex.Message); } });
                        break;
                    case "setup": _ = Services.HearthBrowser.SetupAsync(); break;
                }
                return await Task.Run(Mantel.WinSettings.BrowserStatus);
            }

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
            case "game.launch":
            {
                var id = Req(a, "id");
                CheckCanPlay(id);
                bool resumed = Games.IsFrozen(id);
                // Beacon's games: "mod" plays with that mod this time ("@none": without mods).
                var tracked = await Games.LaunchAsync(id, Str(a, "mod"));
                if (resumed) StartPlayClock();   // back from Quick Resume: no "started" event, the clock runs again
                Event("quickResume", Games.QuickState());
                _ = _gm.PowerFrontAsync(id);   // its own power limit, if it has one (may ask Windows' permission)
                return new { tracked, resumed };
            }
            // Quick Resume: games put aside stay frozen and carry on where they were
            case "game.quickResume":
                CheckCanPlay(Req(a, "id"));
                await Games.QuickResumeAsync(Req(a, "id"));
                StartPlayClock();
                MinimizeShell();
                Event("quickResume", Games.QuickState());
                _ = _gm.PowerFrontAsync(Req(a, "id"));
                return Games.QuickState();
            case "game.putAside":
                await Games.PutAsideAsync();
                StopPlayClock();   // frozen: no play time (and no closing it when time's up) until it's back
                _ = _gm.PowerRestoreAsync();
                OnUi((MethodInvoker)BringShellToFront);
                Event("quickResume", Games.QuickState());
                return Games.QuickState();
            case "game.quickClose":
                await Games.QuickCloseAsync(Req(a, "id"));
                Event("quickResume", Games.QuickState());
                return new { state = Games.QuickState() };
            case "quickResume.set": Games.QuickResumeOn = !(a.TryGetProperty("on", out var qo) && qo.ValueKind == JsonValueKind.False); return Games.QuickState();
            // Steam friends: who's online and what they're playing
            case "steam.friends": return await Task.Run(Library.Friends.List);
            // Profiles: who's playing (switching restarts Ember with their files and Steam account)
            case "profiles.get": return Profiles.Info(Picked);
            case "profiles.create": return Profiles.Create(Str(a, "name") ?? "", Str(a, "color"));
            case "profiles.update":
                return Profiles.Update(Req(a, "id"), Str(a, "name"), Str(a, "color"), Str(a, "steam"), a.TryGetProperty("steam", out _));
            case "profiles.remove": Profiles.Remove(Req(a, "id")); return null;
            case "profiles.setAsk": Profiles.SetAsk(Bool(a, "on")); return null;
            case "profiles.unlock": return Profiles.Unlock(Str(a, "pin") ?? "");
            case "profiles.setPin": Profiles.SetPin(Str(a, "pin") ?? ""); return Profiles.Info(Picked);
            case "profiles.setLimits": return Profiles.SetLimits(Req(a, "id"), a);
            case "profiles.switch":
            {
                var id = Req(a, "id");
                if (Games.RunningId is not null) throw new InvalidOperationException("Close the game first");
                Event("profiles.switching", new { name = Profiles.NameOf(id) });
                await Task.Run(() => Profiles.SwitchTo(id, false));
                OnUi((MethodInvoker)(() => (_ui as ShellForm)?.RestartForProfile()));
                return true;
            }
            case "profiles.addSteam":
            {
                var id = Req(a, "id");
                Profiles.NeedPin();
                var before = Profiles.SteamAccounts().Select(x => x.account.ToLowerInvariant()).ToHashSet();
                await Task.Run(() => { Profiles.StopSteam(); Profiles.SetSteamAccount(""); });
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("steam://open/main") { UseShellExecute = true });
                OnUi((MethodInvoker)(() => _ui.FindForm()!.WindowState = FormWindowState.Minimized));
                // The account that signs in (within 10 minutes) is linked to the profile.
                _ = Task.Run(async () =>
                {
                    for (int i = 0; i < 300; i++)
                    {
                        await Task.Delay(2000);
                        var now = Profiles.SteamAccounts().Select(x => x.account).FirstOrDefault(x => x.Length > 0 && !before.Contains(x.ToLowerInvariant()));
                        if (now is null) continue;
                        try { Profiles.Update(id, null, null, now, true); } catch { }
                        Log($"profiles: linked Steam account {now}");
                        Event("profiles.changed", new { });
                        OnUi((MethodInvoker)BringShellToFront);
                        return;
                    }
                });
                return true;
            }
            // Turned off with the games kept: Windows' hibernation, when it's turned on.
            case "power.hibernateStatus": return Hibernation.Status();
            case "game.install":
            {
                var id = Req(a, "id");
                // Steam games install without Steam's dialog (SteamControl): {silent, waiting}.
                if (id.StartsWith("steam:") && Library.SteamScanner.SteamPath() is not null) return await Games.InstallSteamAsync(id);
                await Games.InstallAsync(id); return null;
            }
            case "library.setSteamOnDemand": await Games.SetSteamOnDemandAsync(Bool(a, "on")); return null;
            // Where Steam games install, and Steam's window in front again (during a sign-in).
            case "steam.libraries": return Games.SteamLibraries();
            case "steam.setLibrary": { var p = Req(a, "path"); Games.SetSteamLibrary(p); return new { chosen = p }; }
            case "steam.showWindow": await Library.SteamControl.OpenUrlAsync("steam://open/main"); Library.SteamWindows.HideMainFor(10); return new { ok = true };
            case "steam.signInCancel":
                Library.SteamControl.CancelSignIn();
                if (!_ui.IsDisposed) _ui.BeginInvoke((MethodInvoker)BringShellToFront);
                return new { ok = true };
            case "library.downloads": return await Games.DownloadsAsync();
            case "library.setSteamBackground": await Games.SetSteamBackgroundAsync(Bool(a, "on")); return null;
            case "game.close": return new { ok = await Games.CloseRunningAsync() };
            case "game.cancelInstall":
            {
                var id = Req(a, "id");
                if (id.StartsWith("steam:") && uint.TryParse(id["steam:".Length..], out var app))
                {
                    if (!Library.SteamControl.CancelPending(app)) throw new InvalidOperationException("Steam is downloading it already: cancel it in Steam (Manage in Steam).");
                    await Games.RescanAsync();
                    Event("library.changed", new { });
                    return new { ok = true };
                }
                return new { ok = Games.CancelInstall(id) };
            }
            case "game.uninstall":
            {
                var id = Req(a, "id");
                // Steam games: deleted without Steam's dialog once Ember's own confirmation was given (SteamControl:
                // never while Steam works on the game or it runs, only inside its Steam library; Steam closes first).
                if (id.StartsWith("steam:"))
                {
                    await Games.UninstallSteamAsync(id);
                    return new { steam = true, silent = true };
                }
                await Games.UninstallAsync(id);
                return await Games.GetAsync(false);
            }

            case "accounts.status": return new { accounts = Games.AccountsStatus(), installRoot = Games.Accounts.InstallRoot, beacon = await Task.Run(() => Library.Beacon.StatusCard(Games.AllGames.ToList())) };
            case "accounts.signIn":
            {
                var store = Req(a, "store");
                if (store == "steam")
                {
                    // Steam signs in through its own window, in front; Ember only reads Steam's files to notice when it's
                    // done (SteamControl.WatchSignIn). Ember never signs in itself.
                    if (Library.SteamScanner.SteamPath() is null) throw new InvalidOperationException("Steam isn't installed on this PC.");
                    bool signedIn = Library.SteamControl.CurrentId64() is not null;
                    Library.SteamControl.WatchSignIn();
                    await Library.SteamControl.OpenUrlAsync("steam://open/main");
                    // Not signed in: Steam's sign-in window comes in front (Steam's main window stays away).
                    if (!signedIn) Library.SteamWindows.HideMainFor(60);
                    return new { ok = true, watching = true };
                }
                return new { ok = await Games.Accounts.SignInAsync(store, _ui.FindForm()) };
            }
            case "accounts.signOut": await Games.Accounts.SignOutAsync(Req(a, "store")); return null;
            case "accounts.refresh":
            {
                var store = Req(a, "store");
                if (store == "steam") { Library.SteamControl.RefreshOwned(true); return new { ok = true }; }
                await Games.Accounts.RefreshAsync(store); return null;
            }
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
            case "update.download":
                _ = Updater.DownloadAsync().ContinueWith(t => { if (t.Exception is { } ex) { Log("update download: " + ex.InnerException?.Message); Event("update", Updater.Status() with { State = "error", Error = "Update download failed: " + ex.InnerException?.Message }); } });
                return Updater.Status();
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
                    quickResume = Games.QuickState(),
                    // Quick Access › Performance: Windows' power mode, the power limit (AMD handhelds), and whether the
                    // game in front keeps its own limit.
                    powerMode = _power.Mode(),
                    powerModes = new[] { "efficiency", "balanced", "performance" },
                    tdp = await Task.Run(_gm.Tdp.Status),
                    gamePower = _gm.GamePower(),
                    canFpsLimit = false, canOverlay = false,
                };
            }

            // Hearth Task Manager (mantel-tasks.html)
            case "tasks.list": return await Task.Run(_tasks.Processes);
            case "tasks.end": return _tasks.End(Ints(a, "pids"));
            case "tasks.kill": return await Task.Run(() => _tasks.Kill(Ints(a, "pids")));
            case "tasks.alive": return new { alive = _tasks.Alive(Ints(a, "pids")) };
            case "tasks.perf": return await Task.Run(() => _tasks.Performance(_power));
            case "tasks.startup":
            {
                var startup = await Task.Run(_tasks.Startup);
                return new { startup, steam = new { installed = Library.SteamScanner.SteamPath() is not null, on = Games.SteamStartsWithEmber } };
            }
            case "tasks.setStartup": await Task.Run(() => _tasks.SetStartup(Req(a, "id"), Bool(a, "on"))); return null;

            // Hearth Files (mantel-files.html). The clipboard calls run on the UI thread (they come in on it).
            case "files.places": return await Task.Run(() => Files.Places(GameFolders()));
            case "files.list": return await Task.Run(() => Files.List(Req(a, "path"), Bool(a, "hidden")));
            case "files.search": return await Files.Search(Req(a, "path"), Req(a, "query"), Bool(a, "hidden"));
            case "files.open": Files.Open(Req(a, "path")); return null;
            case "files.openWith": Files.OpenWith(Req(a, "path"), Mantel.Native.GetForegroundWindow()); return null;
            case "files.properties": Files.WindowsProperties(Req(a, "path"), Mantel.Native.GetForegroundWindow()); return null;
            case "files.info": return await Files.Info(Req(a, "path"));
            case "files.newFolder": return Files.NewFolder(Req(a, "dir"), Str(a, "name"));
            case "files.rename": return Files.Rename(Req(a, "path"), Req(a, "name"));
            case "files.trash": await Files.Trash(Strs(a, "paths")); return null;
            case "files.delete": await Files.DeleteForever(Strs(a, "paths")); return null;
            case "files.restore": return await Files.Restore(Strs(a, "paths"));
            case "files.emptyRecycleBin": await Files.EmptyRecycleBin(); return null;
            case "files.clipSet": Files.SetClipboard(Strs(a, "paths"), Bool(a, "cut")); return null;
            case "files.clipGet": return Files.GetClipboard();
            case "files.clipClear": Files.ClearClipboard(); return null;
            case "files.paste": return new { id = Files.StartTransfer(Strs(a, "paths"), Req(a, "dest"), Bool(a, "move"), Dict(a, "decide"), Str(a, "fallback")) };
            case "files.cancel": return new { ok = Files.Cancel(Int(a, "id")) };
            case "files.conflicts": return await Task.Run(() => Files.Conflicts(Strs(a, "paths"), Req(a, "dest")));
            case "files.props": return await Files.Props(Strs(a, "paths"));
            case "files.count": return new { id = Files.StartCount(Strs(a, "paths")) };
            case "files.checksum": return await Files.Checksum(Req(a, "path"), Req(a, "algo"));
            case "files.cancelChecksums": Files.CancelChecksums(); return null;
            case "files.setAttributes":
                Files.SetAttributes(Req(a, "path"), a.TryGetProperty("readOnly", out _) ? Bool(a, "readOnly") : null, a.TryGetProperty("hidden", out _) ? Bool(a, "hidden") : null);
                return null;
            case "files.changeDefault": await Files.ChangeDefaultApp(Req(a, "path"), Mantel.Native.GetForegroundWindow()); return null;
            case "files.peek": return await Task.Run(() => Files.Peek(Req(a, "path")));
            case "files.newFile": return Files.NewFile(Req(a, "dir"), Req(a, "name"));
            case "files.templates": return await Task.Run(Files.Templates);
            case "files.fromTemplate": return Files.FromTemplate(Req(a, "template"), Req(a, "dir"), Str(a, "name"));
            case "files.batchRename": return await Task.Run(() => Files.BatchRename(Req(a, "pattern"), Strs(a, "paths")));
            case "files.compress": return new { id = Files.StartCompress(Strs(a, "paths"), Req(a, "archive"), Str(a, "format") ?? "zip") };
            case "files.extract": return new { id = Files.StartExtract(Req(a, "archive"), Str(a, "dest"), Bool(a, "here")) };
            case "files.searchStart": return new { id = Files.StartSearch(Req(a, "path"), Req(a, "query"), Bool(a, "hidden")) };
            case "files.terminal": Files.Terminal(Req(a, "path")); return null;
            case "files.copyText": Files.CopyText(Req(a, "text")); return null;
            case "files.eject": await Task.Run(() => Files.Eject(Req(a, "path"))); return null;

            case "overlay.settings": return OverlaySettings();
            case "overlay.setHoldView":
                Games.OverlayHoldView = Bool(a, "on");
                if (Pad is not null) Pad.HoldViewOpensOverlay = Games.OverlayHoldView;
                return OverlaySettings();
            case "overlay.setGameBarButton":
                Overlay.GameBarButton.Set(Bool(a, "on"));
                return OverlaySettings();
            case "overlay.setQaStyle":
                Games.OverlayQaStyle = Str(a, "style") == "side" ? "side" : "dock";
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
                    startMode = Games.StartMode,
                    desktopStyle = Games.DesktopStyle,
                    wallpaper = Games.MantelWallpaper,
                    pictures = CustomWallpapers(),
                    desktopMode = (_ui as ShellForm)?.Desktop?.Active ?? false,
                };
            case "shell.setDesktopStyle":
            {
                var style = Req(a, "style");
                if (style is not ("mantel" or "mantel-full" or "windows")) throw new ArgumentException("Desktop style is mantel, mantel-full or windows");
                var before = Games.DesktopStyle;
                Games.DesktopStyle = style;
                var form = _ui as ShellForm;
                if (form?.Desktop?.Active == true && form.Mantel is not null && before != style)
                {
                    // Switching to or from full Mantel changes whether Explorer runs: go back through Game Mode.
                    if (style == "mantel-full" || before == "mantel-full")
                        _ui.BeginInvoke((MethodInvoker)(async () => { form.Desktop.Leave(); await Task.Delay(2500); form.Desktop.Enter(); }));
                    else if (style == "mantel") _ui.BeginInvoke((MethodInvoker)(async () => await form.Mantel.StartAsync()));
                    else _ui.BeginInvoke((MethodInvoker)(() => form.Mantel.Stop()));
                }
                return await DispatchAsync("shell.status", a);
            }
            case "shell.setWallpaper":
            {
                var w = Str(a, "name") ?? "";
                if (w.Length > 0 && !System.Text.RegularExpressions.Regex.IsMatch(w, "^(hearth-[a-z]+|anim:(fireside|lava|aurora)|custom:[^\\\\/:*?\"<>|]+)$")) throw new ArgumentException("Unknown wallpaper");
                if (w.StartsWith("custom:") && !File.Exists(Path.Combine(Mantel.MantelHost.CustomWallpaperDir, w[7..]))) throw new ArgumentException("That picture is gone");
                Games.MantelWallpaper = w;
                Mantel.MantelHost.Wallpaper = w;
                if ((_ui as ShellForm)?.Mantel is { Active: true } m) m.ApplyWallpaper();
                return await DispatchAsync("shell.status", a);
            }
            // Hearth Settings › Appearance › Choose a picture…: copied into %LOCALAPPDATA%\Nimbus\wallpapers and used.
            case "shell.pickWallpaper":
            {
                using var dlg = new OpenFileDialog
                {
                    Title = "Choose a wallpaper",
                    Filter = "Pictures (*.jpg;*.jpeg;*.png;*.webp;*.bmp)|*.jpg;*.jpeg;*.png;*.webp;*.bmp",
                    CheckFileExists = true,
                    InitialDirectory = Environment.GetFolderPath(Environment.SpecialFolder.MyPictures),
                };
                if (dlg.ShowDialog(_ui.FindForm()) != DialogResult.OK) return null;
                Directory.CreateDirectory(Mantel.MantelHost.CustomWallpaperDir);
                var name = string.Concat(Path.GetFileName(dlg.FileName).Split(Path.GetInvalidFileNameChars()));
                if (name.Length == 0) name = "wallpaper.jpg";
                var dest = Path.Combine(Mantel.MantelHost.CustomWallpaperDir, name);
                if (!string.Equals(Path.GetFullPath(dlg.FileName), Path.GetFullPath(dest), StringComparison.OrdinalIgnoreCase)) File.Copy(dlg.FileName, dest, true);
                using var pick = JsonDocument.Parse(JsonSerializer.Serialize(new { name = "custom:" + name }));
                return await DispatchAsync("shell.setWallpaper", pick.RootElement.Clone());
            }
            case "switch.status": return await Switch.StatusAsync();
            case "switch.prepare": Switch.Prepare(Int(a, "gb")); return await Switch.StatusAsync();
            case "switch.pickIso":
            {
                string? picked = null; Exception? err = null;
                _ui.Invoke((MethodInvoker)(() => { try { picked = Switch.PickIso(_ui.FindForm()!); } catch (Exception ex) { err = ex; } }));
                if (err is not null) throw err;
                return await Switch.StatusAsync();
            }
            case "switch.download": _ = Switch.DownloadIsoAsync(Str(a, "url") ?? "").ContinueWith(t => { if (t.Exception is not null) Event("switch", new { job = new { kind = "download", state = "error", pct = 0, text = "", error = t.Exception.InnerException?.Message } }); }); return await Switch.StatusAsync();
            case "switch.writeUsb": Switch.WriteUsb(Int(a, "disk")); return await Switch.StatusAsync();
            case "switch.restart": Nimbus.Host.Switch.SwitchToHearth.RestartToBootMenu(); return null;
            case "shell.setStartMode":
            {
                var mode = Req(a, "mode");
                if (mode is not ("game" or "desktop")) throw new ArgumentException("Start mode is game or desktop");
                Games.StartMode = mode;
                return await DispatchAsync("shell.status", a);
            }
            case "shell.register":
                ShellRegistration.Register(Environment.ProcessPath!);
                return new { registered = ShellRegistration.IsNimbus() };
            case "shell.unregister":
                ShellRegistration.Unregister();
                return new { registered = ShellRegistration.IsNimbus() };
            case "shell.returnToDesktop":
                _ui.BeginInvoke((MethodInvoker)(() => (_ui as ShellForm)?.ReturnToDesktop()));
                return null;

            default:
                // Bug reports, collections, per-game settings and power, Hearth Input, Beacon (GameModeBridge.cs).
                if (GameModeBridge.Handles(method)) return await _gm.DispatchAsync(method, a);
                // Mantel's taskbar, desktop, newer Settings pages, clipboard history and Apps (Mantel/DeskBridge.cs).
                if (Mantel.DeskBridge.Handles(method)) return await Mantel.DeskBridge.DispatchAsync(method, a, Event);
                throw new NotSupportedException($"Unknown method: {method}");
        }
    }

    /// <summary>Game folders for Hearth Files' places: every Steam library's steamapps\common, and where Ember
    /// installs Epic and GOG games.</summary>
    /// <summary>The pictures chosen as wallpaper (file names in %LOCALAPPDATA%\Nimbus\wallpapers), A–Z.</summary>
    private static List<string> CustomWallpapers()
    {
        try
        {
            if (!Directory.Exists(Mantel.MantelHost.CustomWallpaperDir)) return new List<string>();
            var ok = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { ".jpg", ".jpeg", ".png", ".webp", ".bmp" };
            return Directory.GetFiles(Mantel.MantelHost.CustomWallpaperDir).Where(f => ok.Contains(Path.GetExtension(f))).Select(f => Path.GetFileName(f)!).OrderBy(f => f, StringComparer.OrdinalIgnoreCase).ToList();
        }
        catch { return new List<string>(); }
    }

    private List<string> GameFolders()
    {
        var list = new List<string>();
        try
        {
            var steam = Library.SteamScanner.SteamPath();
            if (steam is not null) foreach (var lib in Library.SteamScanner.Libraries(steam)) list.Add(Path.Combine(lib, "steamapps", "common"));
        }
        catch { }
        try { var root = Games.Accounts.InstallRoot; if (!string.IsNullOrEmpty(root)) list.Add(root); } catch { }
        return list;
    }

    /// <summary>What desktop.get returns (Hearth Settings and Quick Access read it).</summary>
    private object DesktopSettings()
    {
        var mantel = (_ui as ShellForm)?.Mantel;
        return new
        {
            nightLight = Games.MantelNightLight,
            nightAvailable = mantel?.Night.Available ?? true,
            reduceMotion = Games.MantelReduceMotion,
            pointerSpeed = Games.PointerSpeed,
            lockOnWake = Mantel.WinSettings.LockOnWake,
            powerPress = Mantel.WinSettings.PowerPress,
        };
    }

    private object OverlaySettings() => new
    {
        holdView = Games.OverlayHoldView,
        gameBarButton = Overlay.GameBarButton.Get(),
        hotkey = "Ctrl+Alt+Q",
        qaStyle = Games.OverlayQaStyle,        // null until it has been chosen; the page then keeps its own setting
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

        object? wired = null;
        try { wired = Mantel.WinSettings.Wired(); } catch { }

        return new
        {
            wifi = new { available = wifiOn is not null, on = wifiOn == true, ssid, bars },
            wired,
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

    private static int[] Ints(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Array
            ? v.EnumerateArray().Where(x => x.ValueKind == JsonValueKind.Number).Select(x => (int)x.GetDouble()).ToArray()
            : Array.Empty<int>();

    private static string[] Strs(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Array
            ? v.EnumerateArray().Where(x => x.ValueKind == JsonValueKind.String).Select(x => x.GetString() ?? "").Where(x => x.Length > 0).ToArray()
            : Array.Empty<string>();

    private static Dictionary<string, string> Dict(JsonElement a, string k)
    {
        var d = new Dictionary<string, string>();
        if (a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Object)
            foreach (var p in v.EnumerateObject()) if (p.Value.ValueKind == JsonValueKind.String) d[p.Name] = p.Value.GetString() ?? "";
        return d;
    }

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
