using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Mantel on Windows: Hearth's desktop look in Ember's Desktop Mode. Its own taskbar (start button, virtual desktops,
/// pinned and open apps, tray, Wi-Fi, volume, battery, notifications, clock, Quick Access, Game Mode), an app launcher
/// with search, right-click menus, the clock's panel (calendar and notifications), clipboard history, Hearth wallpapers
/// and, in full Mantel, desktop icons.
///
/// Windows' own taskbar is hidden underneath (set to auto-hide so it doesn't keep its space, and kept out of
/// sight), not removed: Explorer still runs, so tray icons and app notifications keep working, and the tray
/// button on Mantel's taskbar shows Windows' taskbar for a moment to reach them. Everything is put back the
/// way it was when you leave Mantel.
/// </summary>
public sealed partial class MantelHost : IDisposable
{
    private readonly SystemBridge _bridge;
    private readonly CoreWebView2Environment _env;
    private readonly string _baseUrl;
    private readonly Action<CoreWebView2> _setup;
    private MantelWindow? _bar, _launcher, _desk, _quick, _power, _radial, _help, _keyboard, _menu, _clock, _clip, _dialog;
    /// <summary>The window the on-screen keyboard (and clipboard history) types into (the last one in front that isn't Mantel's own).</summary>
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
    private AppsWindow? _apps;
    private Backdrop? _dim;
    private readonly TrayHost _tray = new();
    private readonly FullShell _full = new();
    /// <summary>The window a right-click menu or dialog was opened from (its choice goes back there).</summary>
    private MantelWindow? _menuFrom, _dialogFrom;
    private bool _menuAbove;
    private DateTime _clockClosedAt = DateTime.MinValue;
    private Point _menuAt;
    private readonly List<FileSystemWatcher> _deskWatch = new();
    private readonly System.Windows.Forms.Timer _deskSoon = new() { Interval = 500 };
    /// <summary>Full Mantel: no Explorer at all (Ember is the whole desktop and hosts the tray itself).</summary>
    public bool Full { get; private set; }
    public event Action<int>? VolumeRequested;
    private readonly System.Windows.Forms.Timer _tick = new() { Interval = 1000 };
    private string _lastWindows = "", _lastDesks = "";
    private DateTime _peekUntil = DateTime.MinValue;
    private int? _savedTaskbarState;
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);

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
        _bridge.AddListener(json =>
        {
            if (Active) { _bar?.Post(json); _launcher?.Post(json); _quick?.Post(json); _clock?.Post(json); }
            _settings?.Post(json); _tasks?.Post(json); _files?.Post(json); _apps?.Post(json);
            RememberNote(json);
        });
        _tick.Tick += (_, _) => Tick();
        _tray.Changed += () => _bar?.BeginInvoke((MethodInvoker)PushTray);
        _tray.Balloon += (app, title, text) => _bar?.BeginInvoke((MethodInvoker)(() => Notifications.Add(app, title, text)));
        _full.WinKey += () => _bar?.BeginInvoke((MethodInvoker)ToggleLauncher);
        _full.TaskManagerKey += () => _bar?.BeginInvoke((MethodInvoker)(() => _ = OpenAppAsync("tasks")));
        _full.VolumeKey += d => _bar?.BeginInvoke((MethodInvoker)(() => VolumeRequested?.Invoke(d)));
        // The screen turned (a handheld held upright) or changed: put the bar on the right edge for its shape.
        Microsoft.Win32.SystemEvents.DisplaySettingsChanged += (_, _) => _bar?.BeginInvoke((MethodInvoker)Replace);
        // Pins, Do Not Disturb, the clock, desktop icons on or off: every Mantel window hears about it.
        MantelState.Changed += () => _bar?.BeginInvoke((MethodInvoker)PushStateAll);
        Notifications.Changed += () => _bar?.BeginInvoke((MethodInvoker)PushNotes);
        MantelWindow.KeepOpenFor = h => h != IntPtr.Zero && (h == _menu?.Hwnd || h == _dialog?.Hwnd);
        _deskSoon.Tick += (_, _) => { _deskSoon.Stop(); if (_desk is not null) Send(_desk, "mantel.deskChanged", new { }); };
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
            _ = Apps.StartAppsAsync();   // so open windows are matched to their apps (one button per app) from the start
            if (full)
            {
                if (!_tray.Start()) SystemBridge.Log("mantel: couldn't host the tray");
                MantelWindow.DesktopTakesKeys = true;   // the desktop icons take the arrows, Enter, F2 and Delete
                _desk ??= await Create(MantelPart.Desktop);
                _desk.Show();
                _desk.SendToBack();
                WatchDesktop(true);
            }
            else { HideWindowsTaskbar(); ApplyWallpaper(); }
            _bar ??= await Create(MantelPart.Bar);
            _launcher ??= await Create(MantelPart.Launcher);
            _quick ??= await Create(MantelPart.Quick);
            _power ??= await Create(MantelPart.Power);
            _radial ??= await Create(MantelPart.Radial);
            _help ??= await Create(MantelPart.Help);
            _keyboard ??= await Create(MantelPart.Keyboard);
            _menu ??= await Create(MantelPart.Menu);
            _clock ??= await Create(MantelPart.Clock);
            _clip ??= await Create(MantelPart.Clip);
            _dialog ??= await Create(MantelPart.Dialog);
            _dim ??= new Backdrop();
            _dim.Clicked -= HidePanels; _dim.Clicked += HidePanels;
            _bar.Show();
            if (full) { _bar.PlaceManually(); _full.Start(_bar.BarHeight()); }
            else { _bar.EnsureAppBar(); _bar.PlaceAppBar(); }
            PushStateAll();
            Input.Osd.HearthStyle = true;
            Night.Enabled = NightLightOn;
            PushTray();
            PushNotes();
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
        if (Full) { _full.Stop(); _tray.Stop(); _desk?.Hide(); WatchDesktop(false); MantelWindow.OwnTray = false; }
        else RestoreWindowsTaskbar();
        _settings?.Hide();
        _tasks?.Close();
        _files?.Close();
        _apps?.Close();
        Full = false;
    }

    private MantelWindow?[] All => new[] { _bar, _launcher, _desk, _quick, _power, _radial, _help, _keyboard, _menu, _clock, _clip, _dialog };

    private async Task<MantelWindow> Create(MantelPart part)
    {
        var w = new MantelWindow(part);
        w.PageMessage += m => OnMessage(m, w);
        w.PanelClosed += () => PanelClosed(w);
        await w.InitAsync(_env, _baseUrl + "mantel.html?part=" + part.ToString().ToLowerInvariant(), _setup);
        return w;
    }

    /// <summary>A panel closed itself (it lost the front). A menu that closed this way takes the panel it came from
    /// with it unless that panel has the front again (you clicked back into it).</summary>
    private void PanelClosed(MantelWindow w)
    {
        if (w == _menu && _menuFrom is { } from && IsPanelWindow(from) && from.Visible && Native.GetForegroundWindow() != from.Hwnd) from.Hide();
        if (w == _dialog && _dialogFrom is { } df && IsPanelWindow(df) && df.Visible && Native.GetForegroundWindow() != df.Hwnd) df.Hide();
        if (w == _clock) _clockClosedAt = DateTime.UtcNow;
        if (!AnyPanelOpen()) _dim?.Hide();
    }

    private bool IsPanelWindow(MantelWindow w) => Panels.Contains(w);

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
        // Back to the window you were typing in (showing the keyboard can take the focus on some PCs). A dialog of
        // Mantel's own (rename, new folder) keeps it: that's where you're typing.
        if (_dialog is { Visible: true }) return;
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

    private bool IsMantelWindow(IntPtr h) => h != IntPtr.Zero && All.Any(w => w is not null && w.Hwnd == h);

    /// <summary>Full Mantel's desktop window (the controller treats it like any app's window: D-pad arrows, Y keyboard).</summary>
    public bool IsDesktopWindow(IntPtr h) => h != IntPtr.Zero && _desk is not null && _desk.Hwnd == h;

    /// <summary>Types a key from the on-screen keyboard into the window you were in (in order, one after another).
    /// Into Mantel's own dialog when that's open (renaming a desktop icon with the controller).</summary>
    private void TypeKey(string key, string ch, bool shift, bool ctrl, bool alt)
    {
        var fg = Native.GetForegroundWindow();
        bool ours = IsMantelWindow(fg);
        if (!ours && fg != IntPtr.Zero) _typeTarget = fg;
        var target = _dialog is { Visible: true } d ? d.Hwnd : _typeTarget;
        bool switchTo = fg != target && target != IntPtr.Zero;
        _typing = _typing.ContinueWith(_ =>
        {
            try
            {
                if (switchTo) { Native.SetForegroundWindow(target); Thread.Sleep(40); }
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
    /// <summary>Hearth's clipboard history (Clipboard.qml): from the launcher, Settings › Clipboard, or here.</summary>
    public void ToggleClipboard() { if (Active) ToggleClip(); }
    /// <summary>Is one of Mantel's panels in front? (Then the controller moves through it instead of being a mouse.)</summary>
    public bool PanelInFront
    {
        get
        {
            if (!Active) return false;
            var fg = Native.GetForegroundWindow();
            // Hearth's own apps (Task Manager, Files, Apps) are moved through with the D-pad and A too, like on Hearth OS.
            return fg != IntPtr.Zero && (Panels.Any(p => p is not null && p.Hwnd == fg) || _tasks?.Hwnd == fg || _files?.Hwnd == fg || _apps?.Hwnd == fg);
        }
    }
    private MantelWindow?[] Panels => new[] { _launcher, _quick, _power, _radial, _help, _menu, _clock, _clip, _dialog };

    private void ToggleLauncher() => TogglePanel(_launcher);

    private bool AnyPanelOpen() => Panels.Any(p => p is { Visible: true });

    /// <summary>Opens one panel over a dimmed screen (closing any other), or closes it if it's open.</summary>
    private void TogglePanel(MantelWindow? panel)
    {
        if (panel is null) return;
        if (panel.Visible) { HidePanels(); return; }
        foreach (var p in Panels) if (p is not null && p != panel && p.Visible) p.Hide();
        // How dark: the launcher and clipboard history like Hearth's (#b0080b10), Quick Access lighter, the power menu darker.
        var (op, tint) = panel.Part switch
        {
            MantelPart.Launcher or MantelPart.Clip => (0.69, Color.FromArgb(8, 11, 16)),
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

    // ------------------------------------------------------------------ right-click menus (PopMenu.qml)

    /// <summary>Opens Hearth's right-click menu at a point on the screen (screen pixels), above it when it comes from
    /// the taskbar. The items come from the page that asked; the choice goes back to it (mantel.menuPick).</summary>
    private void ShowMenu(MantelWindow from, int x, int y, bool above, string title, JsonElement items, double cssHeight)
    {
        if (_menu is null) return;
        _menuFrom = from; _menuAbove = above; _menuAt = new Point(x, y);
        Send(_menu, "mantel.menu", new { title, items = items.ValueKind == JsonValueKind.Array ? (object)items : Array.Empty<object>() });
        PlaceMenu(cssHeight);
    }

    private void PlaceMenu(double cssHeight)
    {
        if (_menu is null) return;
        var scr = Screen.FromPoint(_menuAt).Bounds;
        int w = _menu.Px(268), h = _menu.Px(Math.Max(40, cssHeight)), m = _menu.Px(8);
        int x = Math.Max(scr.Left + m, Math.Min(scr.Right - w - m, _menuAt.X));
        int y = Math.Max(scr.Top + m, Math.Min(scr.Bottom - h - m, _menuAbove ? _menuAt.Y - h - m : _menuAt.Y));
        _menu.ShowAt(new Rectangle(x, y, w, h));
    }

    private void CloseMenu(bool refocus)
    {
        if (_menu is { Visible: true }) _menu.Hide();
        if (refocus && _menuFrom is { } f && IsPanelWindow(f) && f.Visible) f.Refocus();
    }

    // ------------------------------------------------------------------ the clock's panel, clipboard history, dialogs

    /// <summary>The clock's panel (ClockPanel.qml): notifications and the calendar, above the clock, from the right
    /// (next to a side bar on an upright screen). No dimming, like Hearth's.</summary>
    private void ToggleClock()
    {
        if (_clock is null || _bar is null) return;
        if (_clock.Visible) { HidePanels(); return; }
        // A click on the clock while its panel is open: the panel closed itself as the click took the front; leave it closed.
        if (DateTime.UtcNow - _clockClosedAt < TimeSpan.FromMilliseconds(400)) return;
        HidePanels();
        PlaceClock(560);
        Send(_clock, "mantel.opened", new { });
        Notifications.Seen();
    }

    private void PlaceClock(double cssHeight)
    {
        if (_clock is null || _bar is null) return;
        var scr = Screen.PrimaryScreen!.Bounds;
        int bar = _bar.BarHeight();
        int w = Math.Min(_clock.Px(380), scr.Width - _clock.Px(24));
        int maxH = scr.Height - _clock.Px(24) - (MantelWindow.Portrait ? 0 : bar);
        int h = Math.Min(maxH, _clock.Px(cssHeight));
        int x = MantelWindow.Portrait ? scr.Left + bar + _clock.Px(6) : scr.Right - w - _clock.Px(12);
        int y = scr.Bottom - h - (MantelWindow.Portrait ? _clock.Px(12) : bar + _clock.Px(6));
        _clock.ShowAt(new Rectangle(x, y, w, h), activate: !_clock.Visible);
    }

    private void ToggleClip()
    {
        if (_clip is null) return;
        var fg = Native.GetForegroundWindow();
        if (fg != IntPtr.Zero && !IsMantelWindow(fg)) _typeTarget = fg;
        TogglePanel(_clip);
    }

    /// <summary>Pastes a clipboard history item into the window you were in: it becomes the clipboard's content, then
    /// Ctrl+V is typed there.</summary>
    private void PasteClip(string id)
    {
        HidePanels();
        bool ok;
        try { ok = ClipboardHistory.Select(id); }
        catch (Exception ex) { SystemBridge.Log("clipboard paste: " + ex.Message); ok = false; }
        if (!ok) { Input.Osd.Show("That isn't in the clipboard history any more"); return; }
        var target = _typeTarget;
        _typing = _typing.ContinueWith(_ =>
        {
            try
            {
                Thread.Sleep(120);
                if (target != IntPtr.Zero) { Native.SetForegroundWindow(target); Thread.Sleep(60); }
                KeyTyper.Type("v", "v", false, true, false);
            }
            catch (Exception ex) { SystemBridge.Log("clipboard paste: " + ex.Message); }
        }, TaskScheduler.Default);
    }

    /// <summary>A small question or a card of details (DeskDialog.qml), centered over a dimmed screen.</summary>
    private void ShowDialog(MantelWindow from, JsonElement m)
    {
        if (_dialog is null) return;
        _dialogFrom = from;
        Send(_dialog, "mantel.dialog", m);
        var scr = Screen.PrimaryScreen!.Bounds;
        int w = Math.Min(scr.Width - _dialog.Px(48), _dialog.Px(460)), h = _dialog.Px(240);
        _dim?.ShowDim(0.5, Color.Black);
        _dialog.ShowAt(new Rectangle(scr.Left + (scr.Width - w) / 2, scr.Top + (scr.Height - h) / 2, w, h));
    }

    private void FitDialog(double cssHeight)
    {
        if (_dialog is not { Visible: true }) return;
        var scr = Screen.PrimaryScreen!.Bounds;
        int w = _dialog.Width, h = Math.Min(scr.Height - _dialog.Px(48), _dialog.Px(cssHeight));
        _dialog.ShowAt(new Rectangle(scr.Left + (scr.Width - w) / 2, scr.Top + (scr.Height - h) / 2, w, h), activate: false);
    }

    private void CloseDialog()
    {
        if (_dialog is { Visible: true }) _dialog.Hide();
        if (!AnyPanelOpen()) _dim?.Hide();
    }

    // ------------------------------------------------------------------ notifications

    /// <summary>Ember's own messages that pass through the bridge go into the clock panel's history (and pop up, unless Do Not Disturb is on).</summary>
    private void RememberNote(string json)
    {
        if (!Active || !(json.Contains("\"name\":\"osd\"") || json.Contains("\"name\":\"install.done\"") || json.Contains("\"name\":\"update\""))) return;
        try
        {
            using var doc = JsonDocument.Parse(json);
            var r = doc.RootElement;
            var name = r.GetProperty("name").GetString();
            var d = r.GetProperty("data");
            string S(string k) => d.ValueKind == JsonValueKind.Object && d.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "";
            if (name == "osd" && S("kind") == "msg") Notifications.Add("Ember", S("text"), "");
            else if (name == "install.done")
                Notifications.Add("Ember", S("error").Length > 0 ? S("title") + " didn't install" : S("title") + " is installed", S("error"));
            else if (name == "update" && S("state") == "ready" && d.TryGetProperty("latest", out var l) && l.ValueKind == JsonValueKind.Object && l.TryGetProperty("version", out var ver))
                Notifications.Add("Ember", "Ember " + ver.GetString() + " is ready", "Restart Ember to finish updating.");
        }
        catch { }
    }

    private int _lastNoteKey;
    private void PushNotes()
    {
        var notes = DeskBridge.Notes();
        foreach (var w in new[] { _bar, _clock }) if (w is not null) Send(w, "mantel.notes", notes);
        // A new one pops up (Hearth's glass pill at the top), unless Do Not Disturb is on.
        var newest = Notifications.All.FirstOrDefault();
        if (newest is not null && newest.Key > _lastNoteKey)
        {
            _lastNoteKey = newest.Key;
            if (Active && !MantelState.Load().Dnd && _clock is not { Visible: true })
                Input.Osd.Show((newest.App.Length > 0 && newest.App != "Ember" ? newest.App + ": " : "") + (newest.Summary.Length > 0 ? newest.Summary : newest.Body));
        }
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
            var saved = MantelState.Load();
            _savedTaskbarState = saved.TaskbarState ?? (int)Native.SHAppBarMessage(Native.ABM_GETSTATE, ref d).ToUInt32();
            saved.TaskbarState = _savedTaskbarState;
            MantelState.Save(saved, notify: false);
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
        MantelState.Update(s => s.TaskbarState = null, notify: false);
        foreach (var h in TaskbarWindows()) Native.ShowWindow(h, Native.SW_SHOWNA);
    }

    /// <summary>At Ember's start: if Mantel was running when Ember last stopped (a crash, a power cut),
    /// give Windows' taskbar its setting back.</summary>
    public void RecoverTaskbar()
    {
        var s = MantelState.Load();
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
        PushDesks();
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

    /// <summary>Windows' virtual desktops for the taskbar's switcher, when they change.</summary>
    private void PushDesks(bool force = false)
    {
        if (_bar is null) return;
        var desks = VirtualDesktops.Get();
        var json = JsonSerializer.Serialize(desks, Json);
        if (!force && json == _lastDesks) return;
        _lastDesks = json;
        Send(_bar, "mantel.desks", desks);
    }

    // ------------------------------------------------------------------ the desktop folder (full Mantel's icons)

    private void WatchDesktop(bool on)
    {
        foreach (var w in _deskWatch) w.Dispose();
        _deskWatch.Clear();
        if (!on) return;
        foreach (var dir in new[] { DesktopIcons.UserDir, DesktopIcons.PublicDir }.Distinct())
        {
            try
            {
                if (string.IsNullOrEmpty(dir) || !Directory.Exists(dir)) continue;
                var w = new FileSystemWatcher(dir) { IncludeSubdirectories = false, NotifyFilter = NotifyFilters.FileName | NotifyFilters.DirectoryName | NotifyFilters.Attributes };
                FileSystemEventHandler changed = (_, _) => _bar?.BeginInvoke((MethodInvoker)(() => { _deskSoon.Stop(); _deskSoon.Start(); }));
                w.Created += changed; w.Deleted += changed; w.Changed += changed;
                w.Renamed += (_, _) => changed(this, new FileSystemEventArgs(WatcherChangeTypes.Renamed, dir, null));
                w.EnableRaisingEvents = true;
                _deskWatch.Add(w);
            }
            catch (Exception ex) { SystemBridge.Log("mantel: desktop folder: " + ex.Message); }
        }
    }

    // ------------------------------------------------------------------ messages from the pages

    private void OnMessage(JsonElement m, MantelWindow from)
    {
        var type = m.TryGetProperty("type", out var t) ? t.GetString() : null;
        if (type == "call") { _bridge.Handle(m, from.Post); return; }
        if (type == "log") { SystemBridge.Log("mantel page: " + m.GetProperty("text").GetString()); return; }
        if (type == "ready")
        {
            PushState(from);
            PushWindows(force: true);
            if (from == _bar) { PushTray(); PushNotes(); PushDesks(force: true); }
            if (from == _clock) PushNotes();
            return;
        }
        if (type != "mantel") return;
        string action = m.TryGetProperty("action", out var ac) ? ac.GetString() ?? "" : "";
        string Str(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "";
        long Long(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? (long)v.GetDouble() : 0;
        double Num(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetDouble() : 0;
        bool Bool(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.True;
        switch (action)
        {
            case "apps": _ = PushAppsAsync(from, Bool("refresh")); break;
            // The launcher's Games and Windows sections (MantelGames.cs).
            case "games": _ = PushGamesAsync(from); break;
            case "windowsApps": _ = PushWindowsAppsAsync(from); break;
            case "playGame": HidePanels(); PlayGame(Str("id")); break;
            case "launch":
            {
                var id = Str("id");
                HidePanels();
                if (id.StartsWith("hearth:")) _ = OpenAppAsync(id[7..], null);
                else if (id.StartsWith("ms-settings:")) Apps.OpenUri(id);   // the Windows section's Settings without its app id
                else Apps.Launch(id);
                break;
            }
            case "openApp": HidePanels(); _ = OpenAppAsync(Str("app"), m.TryGetProperty("path", out var op) && op.ValueKind == JsonValueKind.String ? op.GetString() : null); break;
            case "closeApp":
                if (Str("app") == "tasks") _tasks?.Close();
                if (Str("app") == "files") _files?.Close();
                if (Str("app") == "apps") _apps?.Close();
                break;
            case "focus":
                // The app's window was in front when you clicked it: minimize it (Bar.qml's focusApp).
                if (Bool("min") && !Native.IsIconic(new IntPtr(Long("id")))) Native.ShowWindow(new IntPtr(Long("id")), Native.SW_MINIMIZE);
                else Apps.Focus(Long("id"));
                HidePanels();
                break;
            case "activate": Apps.Activate(new IntPtr(Long("id"))); HidePanels(); break;
            case "close": Apps.Close(Long("id")); break;
            case "maximize": Apps.Maximize(Long("id"), Bool("on")); break;
            case "launcher": ToggleLauncher(); break;
            case "trayClick": _tray.Click(Str("key"), Str("button"), (int)Long("x"), (int)Long("y")); break;
            case "hideLauncher": case "hidePanel": HidePanels(); break;
            case "tray": PeekTray(); break;
            case "quick": TogglePanel(_quick); break;
            case "powerMenu": TogglePanel(_power); break;
            case "game": HidePanels(); GameModeRequested?.Invoke(); break;
            case "power": HidePanels(); PowerRequested?.Invoke(Str("what")); break;
            case "lock": HidePanels(); Native.LockWorkStation(); break;
            case "keyboard": if (from != _dialog) HidePanels(); ToggleKeyboard(); break;
            case "keyboardClose": HideKeyboard(); break;
            case "type": TypeKey(Str("key"), Str("ch"), Bool("shift"), Bool("ctrl"), Bool("alt")); break;
            case "help": HidePanels(); TogglePanel(_help); break;
            case "nightlight": HidePanels(); Apps.OpenUri("ms-settings:nightlight"); break;
            case "radial": RadialPick(Str("pick")); break;
            case "pin":
                try { MantelState.SetPin(Str("id"), Str("name"), Bool("on")); }
                catch (Exception ex) { Input.Osd.Show(ex.Message); }
                break;
            // right-click menus
            case "menu": ShowMenu(from, (int)Long("x"), (int)Long("y"), Bool("above"), Str("title"), m.TryGetProperty("items", out var items) ? items : default, Num("h")); break;
            case "menuFit": if (_menu is { Visible: true } && from == _menu) PlaceMenu(Num("h")); break;
            case "menuPick":
            {
                var to = _menuFrom;
                CloseMenu(refocus: true);
                if (to is not null) Send(to, "mantel.menuPick", new { id = Str("id") });
                break;
            }
            case "menuClose": CloseMenu(refocus: true); break;
            case "barTouch":
                // A press on the taskbar (which never takes the front, so open menus don't notice): it closes them, like a
                // click anywhere outside does; the clock's panel stays for the clock and bell, which toggle it.
                if (_menu is { Visible: true } && _menuFrom == _bar) CloseMenu(refocus: false);
                if (_clock is { Visible: true } && Str("what") != "clock") { _clock.Hide(); if (!AnyPanelOpen()) _dim?.Hide(); }
                break;
            // the clock's panel, clipboard history, dialogs
            case "clock": ToggleClock(); break;
            case "clockFit": if (_clock is { Visible: true }) PlaceClock(Num("h")); break;
            case "clip": ToggleClip(); break;
            case "clipPaste": PasteClip(Str("id")); break;
            case "dialog": ShowDialog(from, m); break;
            case "dialogFit": FitDialog(Num("h")); break;
            case "dialogDone":
            {
                var to = _dialogFrom;
                CloseDialog();
                if (to is not null) Send(to, "mantel.dialogDone", new { req = Num("req"), value = Str("value"), ok = Bool("ok") });
                break;
            }
            case "settings": HidePanels(); _ = OpenSettingsAsync(Str("page")); break;
            case "windowsSettings": HidePanels(); Apps.Launch("windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel"); break;
            case "closeSettings": _settings?.Hide(); break;
            case "taskView": HidePanels(); KeyTyper.Combo(0x5B, 0x09); break;
            case "toast": Input.Osd.Show(Str("text")); break;
            case "openUrl":
            {
                // Apps: an app's website (from winget), and App Installer in the Microsoft Store.
                var u = Str("url");
                if (u.StartsWith("https://") || u.StartsWith("http://") || u.StartsWith("ms-windows-store://pdp/")) Apps.OpenUri(u);
                break;
            }
            case "openByName":
            {
                // Apps › Open: the installed app's Start menu entry, found by its name (winget doesn't say how to start it).
                var name = Str("name");
                if (name.Trim().Length < 2) break;
                string N(string x) => new string(x.ToLowerInvariant().Where(char.IsLetterOrDigit).ToArray());
                var list = Apps.StartAppsCached ?? new List<Apps.StartApp>();
                var hit = list.FirstOrDefault(a => N(a.Name) == N(name)) ?? list.FirstOrDefault(a => N(a.Name).StartsWith(N(name)) || (N(name).StartsWith(N(a.Name)) && N(a.Name).Length > 3));
                if (hit is not null) Apps.Launch(hit.Id); else Input.Osd.Show("Find " + name + " in the launcher");
                break;
            }
            case "uri":
            {
                // Windows Settings pages, and Aloha Browser's site (Hearth Settings › Web browser › Install).
                var u = Str("uri");
                if (u.StartsWith("ms-settings:") || u.StartsWith("https://alohabrowser.com/")) { HidePanels(); Apps.OpenUri(u); }
                break;
            }
            case "accent":   // a new accent color: every Mantel window takes it
                foreach (var w in All) if (w is not null) Send(w, "profile", new { accent = Str("accent") });
                _apps?.Post(JsonSerializer.Serialize(new { type = "event", name = "profile", data = new { accent = Str("accent") } }, Json));
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
                else if (type == "mantel" && _bar is not null) OnMessage(m, _bar);
            };
        }
        try { await _settings.OpenAsync(_env, _baseUrl + "mantel-settings.html", _setup, page); }
        catch (Exception ex) { SystemBridge.Log("mantel: settings: " + ex.Message); }
    }

    /// <summary>Opens one of Hearth's own apps: "tasks" (Hearth Task Manager), "files" (Hearth Files, optionally at
    /// a folder), "apps" (Apps), "clipboard" (clipboard history) or "settings". From the launcher (ids hearth:…),
    /// a pin, or {type:"mantel", action:"openApp", app, path}.</summary>
    public async Task OpenAppAsync(string app, string? path = null)
    {
        try
        {
            switch (app)
            {
                case "settings": await OpenSettingsAsync(); break;
                case "clipboard": ToggleClip(); break;
                case "tasks":
                    if (_tasks is null || _tasks.IsDisposed) { _tasks = new TasksWindow(); Wire(_tasks); }
                    await _tasks.OpenAsync(_env, _baseUrl, _setup, null);
                    break;
                case "files":
                    if (_files is null || _files.IsDisposed) { _files = new FilesWindow(); Wire(_files); }
                    await _files.OpenAsync(_env, _baseUrl, _setup, string.IsNullOrEmpty(path) ? null : "path=" + Uri.EscapeDataString(path));
                    break;
                case "apps":
                    if (_apps is null || _apps.IsDisposed) { _apps = new AppsWindow(); Wire(_apps); }
                    await _apps.OpenAsync(_env, _baseUrl, _setup, string.IsNullOrEmpty(path) ? null : "app=" + Uri.EscapeDataString(path));
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
            if (ReferenceEquals(_apps, w)) _apps = null;
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
        // Pins made before the list was read get their icons now.
        if (to == _launcher && _bar is not null) PushState(_bar);
    }

    // ------------------------------------------------------------------ state (mantel.json)

    private void PushState(MantelWindow to)
    {
        var d = MantelState.Load();
        var pins = d.Pins.Select(p => new { id = p.Id, name = p.Name, icon = Apps.PinIcon(p.Id) }).ToList();
        Send(to, "mantel.state", new { pins, full = Full, wallpaper = Wallpaper, reduceMotion = ReduceMotion, desktopIcons = d.DesktopIcons, dnd = d.Dnd, clock24 = d.Clock24 });
    }

    private void PushStateAll() { foreach (var w in All) if (w is not null) PushState(w); }

    /// <summary>The wallpaper chosen in Settings › Shell: one of Hearth's (in wwwroot/wallpapers), or Windows'
    /// own ("" puts back the one you had before Mantel).</summary>
    public static string Wallpaper { get; set; } = "anim:fireside";

    public void ApplyWallpaper()
    {
        if (Full) { if (_desk is not null) PushState(_desk); return; }   // full Mantel draws its own desktop
        if (Wallpaper.StartsWith("anim:")) return;                     // moving wallpapers need full Mantel
        var s = MantelState.Load();
        try
        {
            if (s.WindowsWallpaper is null)
            {
                var sb = new System.Text.StringBuilder(520);
                if (Native.SystemParametersInfo(Native.SPI_GETDESKWALLPAPER, (uint)sb.Capacity, sb, 0)) { var was = sb.ToString(); MantelState.Update(x => x.WindowsWallpaper = was, notify: false); s.WindowsWallpaper = was; }
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
        _deskSoon.Dispose();
        foreach (var w in All) w?.Dispose();
        Night.Dispose();
        _settings?.Dispose();
        _tasks?.Dispose();
        _files?.Dispose();
        _apps?.Dispose();
        _dim?.Dispose();
        _tray.Dispose();
        _full.Dispose();
    }
}
