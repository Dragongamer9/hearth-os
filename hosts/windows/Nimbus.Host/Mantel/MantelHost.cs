using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Mantel on Windows: Hearth's desktop look in Ember's Desktop Mode. Its own taskbar (start button, pinned
/// and open apps, Wi-Fi, volume, battery, clock, Quick Access, Game Mode), an app launcher with search, and
/// Hearth wallpapers.
///
/// Windows' own taskbar is hidden underneath (set to auto-hide so it doesn't keep its space, and kept out of
/// sight), not removed: Explorer still runs, so tray icons and app notifications keep working, and the tray
/// button on Mantel's taskbar shows Windows' taskbar for a moment to reach them. Everything is put back the
/// way it was when you leave Mantel.
/// </summary>
public sealed class MantelHost : IDisposable
{
    private readonly SystemBridge _bridge;
    private readonly CoreWebView2Environment _env;
    private readonly string _baseUrl;
    private readonly Action<CoreWebView2> _setup;
    private MantelWindow? _bar, _launcher, _desk, _quick, _power, _radial, _help, _keyboard;
    /// <summary>The window the on-screen keyboard types into (the last one in front that isn't Mantel's own).</summary>
    private IntPtr _typeTarget;
    private Task _typing = Task.CompletedTask;
    /// <summary>Hearth's night light (Settings › Appearance), while Mantel runs.</summary>
    internal NightLight Night { get; } = new();
    /// <summary>Settings › Appearance › Night light and Reduce motion (set from Ember's saved settings).</summary>
    public static bool NightLightOn { get; set; }
    public static bool ReduceMotion { get; set; }
    /// <summary>Where pictures you choose as wallpaper are kept (served to the pages as https://hearth-walls.local/).</summary>
    public static string CustomWallpaperDir { get; } = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus", "wallpapers");
    private IntPtr _beforeRadial;
    private SettingsWindow? _settings;
    private TasksWindow? _tasks;
    private FilesWindow? _files;
    private Backdrop? _dim;
    private readonly TrayHost _tray = new();
    private readonly FullShell _full = new();
    /// <summary>Full Mantel: no Explorer at all (Ember is the whole desktop and hosts the tray itself).</summary>
    public bool Full { get; private set; }
    public event Action<int>? VolumeRequested;
    private readonly System.Windows.Forms.Timer _tick = new() { Interval = 1000 };
    private string _lastWindows = "";
    private DateTime _peekUntil = DateTime.MinValue;
    private int? _savedTaskbarState;
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);
    private static readonly string StateFile = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus", "mantel.json");

    public bool Active { get; private set; }
    public event Action? GameModeRequested;
    public event Action? QuickAccessRequested;
    public event Action<string>? PowerRequested;

    public MantelHost(SystemBridge bridge, CoreWebView2Environment env, string baseUrl, Action<CoreWebView2> setup)
    {
        _bridge = bridge;
        _env = env;
        _baseUrl = baseUrl;
        _setup = setup;
        _bridge.AddListener(json => { if (Active) { _bar?.Post(json); _launcher?.Post(json); _quick?.Post(json); } _settings?.Post(json); _tasks?.Post(json); _files?.Post(json); });
        _tick.Tick += (_, _) => Tick();
        _tray.Changed += () => _bar?.BeginInvoke((MethodInvoker)PushTray);
        _full.WinKey += () => _bar?.BeginInvoke((MethodInvoker)ToggleLauncher);
        _full.TaskManagerKey += () => _bar?.BeginInvoke((MethodInvoker)(() => _ = OpenAppAsync("tasks")));
        _full.VolumeKey += d => _bar?.BeginInvoke((MethodInvoker)(() => VolumeRequested?.Invoke(d)));
        // The screen turned (a handheld held upright) or changed: put the bar on the right edge for its shape.
        Microsoft.Win32.SystemEvents.DisplaySettingsChanged += (_, _) => _bar?.BeginInvoke((MethodInvoker)Replace);
    }

    // ------------------------------------------------------------------ start / stop

    public async Task StartAsync(bool full = false)
    {
        if (Active) return;
        // Full Mantel needs Explorer gone (Ember as the shell); with Explorer running, Mantel goes on top of it.
        if (full && DesktopMode.ExplorerShellRunning()) { SystemBridge.Log("mantel: Explorer is running, so Mantel goes on top of it"); full = false; }
        Active = true;
        Full = full;
        SystemBridge.Log("mantel: starting" + (full ? " (full, no Explorer)" : ""));
        try
        {
            MantelWindow.OwnTray = full;
            if (full)
            {
                if (!_tray.Start()) SystemBridge.Log("mantel: couldn't host the tray");
                _desk ??= await Create(MantelPart.Desktop);
                _desk.Show();
                _desk.SendToBack();
            }
            else { HideWindowsTaskbar(); ApplyWallpaper(); }
            _bar ??= await Create(MantelPart.Bar);
            _launcher ??= await Create(MantelPart.Launcher);
            _quick ??= await Create(MantelPart.Quick);
            _power ??= await Create(MantelPart.Power);
            _radial ??= await Create(MantelPart.Radial);
            _help ??= await Create(MantelPart.Help);
            _keyboard ??= await Create(MantelPart.Keyboard);
            _dim ??= new Backdrop();
            _dim.Clicked -= HidePanels; _dim.Clicked += HidePanels;
            _bar.Show();
            if (full) { _bar.PlaceManually(); _full.Start(_bar.BarHeight()); }
            else { _bar.EnsureAppBar(); _bar.PlaceAppBar(); }
            foreach (var w in new[] { _bar, _launcher, _desk, _quick, _power, _radial, _help, _keyboard }) if (w is not null) PushState(w);
            Input.Osd.HearthStyle = true;
            Night.Enabled = NightLightOn;
            PushTray();
            _tick.Start();
            PushWindows(force: true);
        }
        catch (Exception ex)
        {
            SystemBridge.Log("mantel: couldn't start: " + ex);
            Stop();   // never leave you without a taskbar
        }
    }

    public void Stop()
    {
        if (!Active) return;
        Active = false;
        SystemBridge.Log("mantel: stopping");
        _tick.Stop();
        HidePanels();
        HideKeyboard();
        Night.Enabled = false;
        Input.Osd.HearthStyle = false;
        if (_bar is not null) { _bar.RemoveAppBar(); _bar.Hide(); }
        if (Full) { _full.Stop(); _tray.Stop(); _desk?.Hide(); MantelWindow.OwnTray = false; }
        else RestoreWindowsTaskbar();
        _settings?.Hide();
        _tasks?.Close();
        _files?.Close();
        Full = false;
    }

    private async Task<MantelWindow> Create(MantelPart part)
    {
        var w = new MantelWindow(part);
        w.PageMessage += m => OnMessage(m, w);
        w.PanelClosed += () => { if (!AnyPanelOpen()) _dim?.Hide(); };
        await w.InitAsync(_env, _baseUrl + "mantel.html?part=" + part.ToString().ToLowerInvariant(), _setup);
        return w;
    }

    private void Replace()
    {
        if (!Active || _bar is null) return;
        HidePanels();
        if (Full) { _bar.PlaceManually(); _full.SetWorkArea(_bar.BarHeight()); _desk?.SendToBack(); }
        else _bar.PlaceAppBar();
        if (_keyboard is { Visible: true }) ShowKeyboard();
    }

    // ------------------------------------------------------------------ the on-screen keyboard (Keyboard.qml)

    /// <summary>Hearth's on-screen keyboard is up (the controller then picks keys instead of moving the pointer).</summary>
    public bool KeyboardOpen => Active && _keyboard is { Visible: true };

    /// <summary>Y on the controller, the taskbar's keyboard button, Quick Access or the Hearth menu: Hearth's keyboard
    /// (Windows' touch keyboard when Mantel isn't running).</summary>
    public void ToggleKeyboard()
    {
        if (!Active || _keyboard is null) { new Thread(Stores.TouchKeyboard.Toggle) { IsBackground = true }.Start(); return; }
        if (_keyboard.Visible) HideKeyboard(); else ShowKeyboard();
    }

    private void ShowKeyboard()
    {
        if (_keyboard is null) return;
        var fg = Native.GetForegroundWindow();
        if (fg != IntPtr.Zero && !IsMantelWindow(fg)) _typeTarget = fg;
        _keyboard.ShowKeyboard();
        if (Full && _bar is not null && !MantelWindow.Portrait) _full.SetWorkArea(_bar.BarHeight() + _keyboard.KeyboardHeight);
        Send(_keyboard, "mantel.opened", new { });
        // Back to the window you were typing in (showing the keyboard can take the focus on some PCs).
        if (_typeTarget != IntPtr.Zero && Native.GetForegroundWindow() != _typeTarget) Native.SetForegroundWindow(_typeTarget);
    }

    private void HideKeyboard()
    {
        if (_keyboard is not { Visible: true }) return;
        _keyboard.HideKeyboard();
        if (Full && _bar is not null) _full.SetWorkArea(_bar.BarHeight());
    }

    /// <summary>A controller button while the keyboard is up (the page moves the highlight and picks keys).</summary>
    public void KeyboardButton(string b) { if (_keyboard is not null) Send(_keyboard, "mantel.kbdButton", new { b }); }

    private bool IsMantelWindow(IntPtr h) =>
        h != IntPtr.Zero && new[] { _bar, _launcher, _desk, _quick, _power, _radial, _help, _keyboard }.Any(w => w is not null && w.Hwnd == h);

    /// <summary>Types a key from the on-screen keyboard into the window you were in (in order, one after another).</summary>
    private void TypeKey(string key, string ch, bool shift, bool ctrl, bool alt)
    {
        var fg = Native.GetForegroundWindow();
        bool ours = IsMantelWindow(fg);
        if (!ours && fg != IntPtr.Zero) _typeTarget = fg;
        var target = _typeTarget;
        _typing = _typing.ContinueWith(_ =>
        {
            try
            {
                if (ours && target != IntPtr.Zero) { Native.SetForegroundWindow(target); Thread.Sleep(40); }
                KeyTyper.Type(key, ch, shift, ctrl, alt);
            }
            catch (Exception ex) { SystemBridge.Log("keyboard: " + ex.Message); }
        }, TaskScheduler.Default);
    }

    /// <summary>Settings › Appearance › Night light changed (or Mantel started): warm the screen or not.</summary>
    public void ApplyNightLight() { if (Active) Night.Enabled = NightLightOn; }

    /// <summary>Settings › Appearance › Reduce motion changed: the desktop's wallpaper stands still (or moves again).</summary>
    public void ApplyReduceMotion() { foreach (var w in new[] { _desk }) if (w is not null) PushState(w); }

    public void ToggleLauncherFromPad() { if (Active) ToggleLauncher(); }
    /// <summary>The power menu (the power button's "menu" choice, or the controller), like PowerMenu.qml.</summary>
    public void ShowPowerMenu() { if (Active) TogglePanel(_power); }
    public void ToggleQuick() { if (Active) TogglePanel(_quick); }
    /// <summary>The Hearth menu (the Xbox button in Desktop Mode), like RadialMenu.qml.</summary>
    public void ToggleRadial() { if (!Active) return; if (_radial is { Visible: false }) _beforeRadial = Native.GetForegroundWindow(); TogglePanel(_radial); }
    /// <summary>The controller's controls (a tap of View), like Help.qml.</summary>
    public void ToggleHelp() { if (Active) TogglePanel(_help); }
    /// <summary>Is one of Mantel's panels in front? (Then the controller moves through it instead of being a mouse.)</summary>
    public bool PanelInFront
    {
        get
        {
            if (!Active) return false;
            var fg = Native.GetForegroundWindow();
            // Hearth's own apps (Task Manager, Files) are moved through with the D-pad and A too, like on Hearth OS.
            return fg != IntPtr.Zero && (Panels.Any(p => p is not null && p.Hwnd == fg) || _tasks?.Hwnd == fg || _files?.Hwnd == fg);
        }
    }
    private MantelWindow?[] Panels => new[] { _launcher, _quick, _power, _radial, _help };

    private void ToggleLauncher() => TogglePanel(_launcher);

    private bool AnyPanelOpen() => Panels.Any(p => p is { Visible: true });

    /// <summary>Opens one panel over a dimmed screen (closing any other), or closes it if it's open.</summary>
    private void TogglePanel(MantelWindow? panel)
    {
        if (panel is null) return;
        if (panel.Visible) { HidePanels(); return; }
        foreach (var p in Panels) if (p is not null && p != panel && p.Visible) p.Hide();
        // How dark: the launcher like Hearth's (#b0080b10), Quick Access lighter, the power menu darker.
        var (op, tint) = panel.Part switch
        {
            MantelPart.Launcher => (0.69, Color.FromArgb(8, 11, 16)),
            MantelPart.Quick => (0.30, Color.Black),
            MantelPart.Help => (0.0, Color.Black),
            _ => (0.55, Color.Black),
        };
        if (op > 0) _dim?.ShowDim(op, tint); else _dim?.Hide();
        panel.ShowPanel();
        Send(panel, "mantel.opened", new { });
    }

    private void HidePanels()
    {
        foreach (var p in Panels) if (p is { Visible: true }) p.Hide();
        _dim?.Hide();
    }

    private void PushTray()
    {
        if (_bar is null) return;
        var icons = Full ? _tray.Icons.Select(i => new { key = i.Key, tip = i.Tip, image = i.Image }).ToList<object>() : new List<object>();
        Send(_bar, "mantel.tray", new { full = Full, icons });
    }

    // ------------------------------------------------------------------ Windows' taskbar

    private static IEnumerable<IntPtr> TaskbarWindows()
    {
        var main = Native.FindWindow("Shell_TrayWnd", null);
        if (main != IntPtr.Zero) yield return main;
        IntPtr prev = IntPtr.Zero;
        while ((prev = Native.FindWindowEx(IntPtr.Zero, prev, "Shell_SecondaryTrayWnd", null)) != IntPtr.Zero) yield return prev;
    }

    private void HideWindowsTaskbar()
    {
        var main = Native.FindWindow("Shell_TrayWnd", null);
        if (main == IntPtr.Zero) return;
        var d = new Native.APPBARDATA { cbSize = System.Runtime.InteropServices.Marshal.SizeOf<Native.APPBARDATA>(), hWnd = main };
        if (_savedTaskbarState is null)
        {
            var saved = Load();
            _savedTaskbarState = saved.TaskbarState ?? (int)Native.SHAppBarMessage(Native.ABM_GETSTATE, ref d).ToUInt32();
            saved.TaskbarState = _savedTaskbarState;
            Save(saved);
        }
        d.lParam = new IntPtr(Native.ABS_AUTOHIDE);
        Native.SHAppBarMessage(Native.ABM_SETSTATE, ref d);   // gives its space to Mantel's taskbar
        foreach (var h in TaskbarWindows()) Native.ShowWindow(h, Native.SW_HIDE);
    }

    private void RestoreWindowsTaskbar()
    {
        var main = Native.FindWindow("Shell_TrayWnd", null);
        if (main != IntPtr.Zero && _savedTaskbarState is int st)
        {
            var d = new Native.APPBARDATA { cbSize = System.Runtime.InteropServices.Marshal.SizeOf<Native.APPBARDATA>(), hWnd = main, lParam = new IntPtr(st) };
            Native.SHAppBarMessage(Native.ABM_SETSTATE, ref d);
        }
        _savedTaskbarState = null;
        var s = Load(); s.TaskbarState = null; Save(s);
        foreach (var h in TaskbarWindows()) Native.ShowWindow(h, Native.SW_SHOWNA);
    }

    /// <summary>At Ember's start: if Mantel was running when Ember last stopped (a crash, a power cut),
    /// give Windows' taskbar its setting back.</summary>
    public void RecoverTaskbar()
    {
        var s = Load();
        if (s.TaskbarState is int st) { _savedTaskbarState = st; RestoreWindowsTaskbar(); SystemBridge.Log("mantel: put Windows' taskbar back after an unclean stop"); }
    }

    /// <summary>The tray button: Windows' taskbar (with its tray icons) for ten seconds.</summary>
    private void PeekTray()
    {
        _peekUntil = DateTime.UtcNow.AddSeconds(10);
        foreach (var h in TaskbarWindows()) Native.ShowWindow(h, Native.SW_SHOWNA);
        var tray = Native.FindWindow("Shell_TrayWnd", null);
        if (tray != IntPtr.Zero) Native.SetForegroundWindow(tray);
    }

    private void Tick()
    {
        if (!Active) return;
        var fg = Native.GetForegroundWindow();
        if (fg != IntPtr.Zero && !IsMantelWindow(fg)) _typeTarget = fg;
        if (Full) { _desk?.SendToBack(); PushWindows(); return; }
        // Explorer brings its taskbar back now and then (Start menu, a restart of Explorer): hide it again,
        // except while you're using it (the tray peek, or the Start menu open on top).
        if (DateTime.UtcNow > _peekUntil && !StartMenuOpen())
            foreach (var h in TaskbarWindows()) if (Native.IsWindowVisible(h)) Native.ShowWindow(h, Native.SW_HIDE);
        PushWindows();
    }

    private static bool StartMenuOpen()
    {
        var fg = Native.GetForegroundWindow();
        var cls = new System.Text.StringBuilder(128);
        Native.GetClassName(fg, cls, cls.Capacity);
        return cls.ToString() is "Windows.UI.Core.CoreWindow" or "Shell_TrayWnd" or "NotifyIconOverflowWindow" or "TopLevelWindowForOverflowXamlIsland";
    }

    // ------------------------------------------------------------------ messages from the pages

    private void OnMessage(JsonElement m, MantelWindow from)
    {
        var type = m.TryGetProperty("type", out var t) ? t.GetString() : null;
        if (type == "call") { _bridge.Handle(m, from.Post); return; }
        if (type == "log") { SystemBridge.Log("mantel page: " + m.GetProperty("text").GetString()); return; }
        if (type == "ready") { PushState(from); PushWindows(force: true); return; }
        if (type != "mantel") return;
        string action = m.GetProperty("action").GetString() ?? "";
        string Str(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "";
        long Long(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt64() : 0;
        bool Bool(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.True;
        switch (action)
        {
            case "apps": _ = PushAppsAsync(from, m.TryGetProperty("refresh", out var r) && r.ValueKind == JsonValueKind.True); break;
            case "launch":
            {
                var id = Str("id");
                HidePanels();
                if (id.StartsWith("hearth:")) _ = OpenAppAsync(id[7..], null);
                else Apps.Launch(id);
                break;
            }
            case "openApp": HidePanels(); _ = OpenAppAsync(Str("app"), m.TryGetProperty("path", out var op) && op.ValueKind == JsonValueKind.String ? op.GetString() : null); break;
            case "closeApp":
                if (Str("app") == "tasks") _tasks?.Close();
                if (Str("app") == "files") _files?.Close();
                break;
            case "focus": Apps.Focus(Long("id")); HidePanels(); break;
            case "close": Apps.Close(Long("id")); break;
            case "launcher": ToggleLauncher(); break;
            case "trayClick": _tray.Click(Str("key"), Str("button"), (int)Long("x"), (int)Long("y")); break;
            case "hideLauncher": case "hidePanel": HidePanels(); break;
            case "tray": PeekTray(); break;
            case "quick": TogglePanel(_quick); break;
            case "powerMenu": TogglePanel(_power); break;
            case "game": HidePanels(); GameModeRequested?.Invoke(); break;
            case "power": HidePanels(); PowerRequested?.Invoke(Str("what")); break;
            case "lock": HidePanels(); Native.LockWorkStation(); break;
            case "keyboard": HidePanels(); ToggleKeyboard(); break;
            case "keyboardClose": HideKeyboard(); break;
            case "type": TypeKey(Str("key"), Str("ch"), Bool("shift"), Bool("ctrl"), Bool("alt")); break;
            case "help": HidePanels(); TogglePanel(_help); break;
            case "nightlight": HidePanels(); Apps.OpenUri("ms-settings:nightlight"); break;
            case "radial": RadialPick(Str("pick")); break;
            case "pin": SetPin(Str("id"), Str("name"), m.TryGetProperty("on", out var on) && on.ValueKind == JsonValueKind.True); break;
            case "settings": HidePanels(); _ = OpenSettingsAsync(Str("page")); break;
            case "windowsSettings": HidePanels(); Apps.Launch("windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel"); break;
            case "closeSettings": _settings?.Hide(); break;
            case "uri":
            {
                // Windows Settings pages, and Aloha Browser's site (Hearth Settings › Web browser › Install).
                var u = Str("uri");
                if (u.StartsWith("ms-settings:") || u.StartsWith("https://alohabrowser.com/")) { HidePanels(); Apps.OpenUri(u); }
                break;
            }
            case "accent":   // a new accent color: every Mantel window takes it
                foreach (var w in new[] { _bar, _launcher, _quick, _power, _radial, _help, _keyboard }) if (w is not null) Send(w, "profile", new { accent = Str("accent") });
                break;
        }
    }

    /// <summary>Hearth Settings (mantel-settings.html), opened on a page if one's given.</summary>
    public async Task OpenSettingsAsync(string? page = null)
    {
        if (_settings is null || _settings.IsDisposed)
        {
            _settings = new SettingsWindow();
            _settings.PageMessage += m =>
            {
                var type = m.TryGetProperty("type", out var t) ? t.GetString() : null;
                if (type == "call") _bridge.Handle(m, _settings.Post);
                else if (type == "mantel") OnMessage(m, _bar!);
            };
        }
        try { await _settings.OpenAsync(_env, _baseUrl + "mantel-settings.html", _setup, page); }
        catch (Exception ex) { SystemBridge.Log("mantel: settings: " + ex.Message); }
    }

    /// <summary>Opens one of Hearth's own apps: "tasks" (Hearth Task Manager), "files" (Hearth Files, optionally at
    /// a folder) or "settings". From the launcher (ids hearth:tasks, hearth:files), a pin, or {type:"mantel",
    /// action:"openApp", app, path}.</summary>
    public async Task OpenAppAsync(string app, string? path = null)
    {
        try
        {
            switch (app)
            {
                case "settings": await OpenSettingsAsync(); break;
                case "tasks":
                    if (_tasks is null || _tasks.IsDisposed) { _tasks = new TasksWindow(); Wire(_tasks); }
                    await _tasks.OpenAsync(_env, _baseUrl, _setup, null);
                    break;
                case "files":
                    if (_files is null || _files.IsDisposed) { _files = new FilesWindow(); Wire(_files); }
                    await _files.OpenAsync(_env, _baseUrl, _setup, string.IsNullOrEmpty(path) ? null : "path=" + Uri.EscapeDataString(path));
                    break;
            }
        }
        catch (Exception ex) { SystemBridge.Log($"mantel: {app}: {ex.Message}"); }
    }

    private void Wire(AppWindow w)
    {
        w.PageMessage += m =>
        {
            var type = m.TryGetProperty("type", out var t) ? t.GetString() : null;
            if (type == "call") _bridge.Handle(m, w.Post);
            else if (type == "log") SystemBridge.Log(w.Text + " page: " + (m.TryGetProperty("text", out var x) ? x.GetString() : ""));
            else if (type == "mantel" && _bar is not null) OnMessage(m, _bar);
        };
        w.FormClosed += (_, _) =>
        {
            if (ReferenceEquals(_tasks, w)) _tasks = null;
            if (ReferenceEquals(_files, w)) _files = null;
        };
    }

    /// <summary>What the Hearth menu's items do (Pad.qml's run()).</summary>
    private void RadialPick(string pick)
    {
        HidePanels();
        switch (pick)
        {
            case "apps": ToggleLauncher(); break;
            case "keyboard": ToggleKeyboard(); break;
            case "quick": TogglePanel(_quick); break;
            case "close": if (_beforeRadial != IntPtr.Zero) Native.PostMessage(_beforeRadial, 0x0010 /* WM_CLOSE */, IntPtr.Zero, IntPtr.Zero); break;
            case "game": GameModeRequested?.Invoke(); break;
            case "screenshot": _ = Task.Delay(400).ContinueWith(_ => Native.Screenshot()); break;
            case "mouse": _bridge.Pad?.ToggleMouse(); break;
            case "help": TogglePanel(_help); break;
        }
    }

    private static void Send(MantelWindow w, string name, object data) =>
        w.Post(JsonSerializer.Serialize(new { type = "event", name, data }, Json));

    private void PushWindows(bool force = false)
    {
        if (_bar is null) return;
        List<Apps.Win> wins;
        try { wins = Apps.Windows(); } catch { return; }
        var json = JsonSerializer.Serialize(wins, Json);
        if (!force && json == _lastWindows) return;
        _lastWindows = json;
        _bar.Post(JsonSerializer.Serialize(new { type = "event", name = "mantel.windows", data = wins }, Json));
    }

    private async Task PushAppsAsync(MantelWindow to, bool refresh)
    {
        var apps = await Apps.StartAppsAsync(refresh);
        Send(to, "mantel.apps", apps);
    }

    // ------------------------------------------------------------------ pins and wallpaper (mantel.json)

    private sealed class SavedState
    {
        public List<Pin> Pins { get; set; } = new();
        public string? WindowsWallpaper { get; set; }
        /// <summary>Windows' taskbar setting before Mantel set it to auto-hide (kept on disk so a crash can't
        /// leave it changed: the next start puts it back).</summary>
        public int? TaskbarState { get; set; }
    }
    public sealed record Pin(string Id, string Name);

    private static SavedState Load()
    {
        try { return JsonSerializer.Deserialize<SavedState>(File.ReadAllText(StateFile), Json) ?? new SavedState(); }
        catch { return new SavedState(); }
    }
    private static void Save(SavedState s)
    {
        try { Directory.CreateDirectory(Path.GetDirectoryName(StateFile)!); File.WriteAllText(StateFile, JsonSerializer.Serialize(s, Json)); }
        catch (Exception ex) { SystemBridge.Log("mantel: save: " + ex.Message); }
    }

    private void PushState(MantelWindow to) => Send(to, "mantel.state", new { pins = Load().Pins, full = Full, wallpaper = Wallpaper, reduceMotion = ReduceMotion });

    private void SetPin(string id, string name, bool on)
    {
        if (string.IsNullOrEmpty(id)) return;
        var s = Load();
        s.Pins.RemoveAll(p => p.Id == id);
        if (on) s.Pins.Add(new Pin(id, name));
        Save(s);
        foreach (var w in new[] { _bar, _launcher, _quick, _radial }) if (w is not null) PushState(w);
    }

    /// <summary>The wallpaper chosen in Settings › Shell: one of Hearth's (in wwwroot/wallpapers), or Windows'
    /// own ("" puts back the one you had before Mantel).</summary>
    public static string Wallpaper { get; set; } = "anim:fireside";

    public void ApplyWallpaper()
    {
        if (Full) { if (_desk is not null) PushState(_desk); return; }   // full Mantel draws its own desktop
        if (Wallpaper.StartsWith("anim:")) return;                     // moving wallpapers need full Mantel
        var s = Load();
        try
        {
            if (s.WindowsWallpaper is null)
            {
                var sb = new System.Text.StringBuilder(520);
                if (Native.SystemParametersInfo(Native.SPI_GETDESKWALLPAPER, (uint)sb.Capacity, sb, 0)) { s.WindowsWallpaper = sb.ToString(); Save(s); }
            }
            string? file = string.IsNullOrEmpty(Wallpaper) ? s.WindowsWallpaper
                : Wallpaper.StartsWith("custom:") ? Path.Combine(CustomWallpaperDir, Path.GetFileName(Wallpaper[7..]))
                : Path.Combine(AppContext.BaseDirectory, "wwwroot", "wallpapers", Path.GetFileName(Wallpaper) + ".jpg");
            if (!string.IsNullOrEmpty(file) && (File.Exists(file) || string.IsNullOrEmpty(Wallpaper)))
                Native.SystemParametersInfo(Native.SPI_SETDESKWALLPAPER, 0, file, Native.SPIF_UPDATEINIFILE | Native.SPIF_SENDCHANGE);
        }
        catch (Exception ex) { SystemBridge.Log("mantel: wallpaper: " + ex.Message); }
    }

    public void Dispose()
    {
        Stop();
        _tick.Dispose();
        _bar?.Dispose();
        _launcher?.Dispose();
        _desk?.Dispose();
        _quick?.Dispose();
        _power?.Dispose();
        _radial?.Dispose();
        _help?.Dispose();
        _keyboard?.Dispose();
        Night.Dispose();
        _settings?.Dispose();
        _tasks?.Dispose();
        _files?.Dispose();
        _dim?.Dispose();
        _tray.Dispose();
        _full.Dispose();
    }
}
