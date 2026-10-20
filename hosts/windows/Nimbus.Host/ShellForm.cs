using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host;

/// <summary>
/// Borderless fullscreen window that hosts the web UI in WebView2 and
/// handles the messages it sends (launch, power, settings...).
/// </summary>
public sealed class ShellForm : Form
{
    private const string VirtualHost = "nimbus.local";
    private const int HotkeyEscapeHatch = 0x4E01;
    private const int HotkeyOverlay = 0x4E02;
    private const int HotkeyGameMode = 0x4E03;

    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(12, 16, 22) };
    private readonly bool _windowed;
    private SystemBridge? _system;
    private Input.ControllerBridge? _pad;
    private Overlay.OverlayForm? _overlay;
    private IntPtr _overlayHandle;
    /// <summary>Desktop Mode (the normal Windows desktop) and the ways back to Game Mode.</summary>
    public DesktopMode? Desktop { get; private set; }
    /// <summary>Hearth's desktop look on Windows, shown in Desktop Mode.</summary>
    public Nimbus.Host.Mantel.MantelHost? Mantel { get; private set; }

    public ShellForm(bool windowed)
    {
        _windowed = windowed;
        Text = "Ember";
        BackColor = Color.FromArgb(12, 16, 22);
        if (windowed)
        {
            ClientSize = new Size(1280, 800);
            StartPosition = FormStartPosition.CenterScreen;
        }
        else
        {
            FormBorderStyle = FormBorderStyle.None;
            WindowState = FormWindowState.Maximized;
        }
        Controls.Add(_web);
        Load += async (_, _) => await InitAsync();

        // As the shell there is nothing behind Nimbus, so don't let Alt+F4 close it by accident.
        FormClosing += (_, e) =>
        {
            if (Program.IsShell && !_leaving && e.CloseReason == CloseReason.UserClosing) e.Cancel = true;
        };
    }

    private bool _leaving;

    private async Task InitAsync()
    {
        var dataDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus", "WebView2");
        CoreWebView2Environment env;
        try
        {
            env = await CoreWebView2Environment.CreateAsync(null, dataDir);
            await _web.EnsureCoreWebView2Async(env);
        }
        catch (Exception ex)
        {
            // Without WebView2 there is no UI. As the shell, fall back to Explorer rather than a black screen.
            SystemBridge.Log("WebView2 failed: " + ex);
            if (Program.IsShell) StartExplorer();
            MessageBox.Show("Ember couldn't start its interface (WebView2): " + ex.Message, "Ember");
            _leaving = true;
            Close();
            return;
        }

        var core = _web.CoreWebView2;
        var s = core.Settings;
        s.AreDefaultContextMenusEnabled = false;
        s.IsZoomControlEnabled = false;
        s.IsStatusBarEnabled = false;
#if DEBUG
        s.AreDevToolsEnabled = true;
        s.AreBrowserAcceleratorKeysEnabled = true;
#else
        s.AreDevToolsEnabled = false;
        s.AreBrowserAcceleratorKeysEnabled = false;
#endif

        var www = Path.Combine(AppContext.BaseDirectory, "wwwroot");
        core.SetVirtualHostNameToFolderMapping(VirtualHost, www, CoreWebView2HostResourceAccessKind.Allow);
        _system = new SystemBridge(this, json => _web.CoreWebView2?.PostWebMessageAsJson(json));
        FormClosed += (_, _) => _system?.Dispose();
        _system.StartBackgroundServices();
        // Controller support for Steam dialogs and native pop-ups (the web UI handles its own).
        _pad = new Input.ControllerBridge(Handle, () => _system?.Games.RunningId is not null, _system.Games.ControllerMouse);
        _system.Pad = _pad;
        _pad.MouseChanged += on => BeginInvoke((MethodInvoker)(() =>
        {
            // Hearth's words while Mantel runs (Pad.qml's toast), Ember's own otherwise.
            Input.Osd.Show(Input.Osd.HearthStyle ? (on ? "Controller mouse on" : "Controller mouse off") : on ? "🖱  Controller mouse on" : "Controller mouse off");
            Post(new { type = "event", name = "padMouse", data = new { on } });
        }));
        _pad.UiButton += b => BeginInvoke((MethodInvoker)(() => Post(new { type = "event", name = "pad", data = new { b } })));
        _pad.ControllersChanged += n => BeginInvoke((MethodInvoker)(() => Post(new { type = "event", name = "padHost", data = new { connected = n } })));
        FormClosed += (_, _) => _pad?.Dispose();
        Desktop = new DesktopMode(this);
        FormClosed += (_, _) => Desktop?.Dispose();
        _pad.DesktopMode = () => Desktop?.Active == true;
        Desktop.Changed += on => Post(new { type = "event", name = "desktopMode", data = new { on } });
        // Mantel: Hearth's desktop look in Desktop Mode (Settings › Shell › Desktop style).
        Mantel = new Nimbus.Host.Mantel.MantelHost(_system, env, $"https://{VirtualHost}/", c =>
        {
            c.SetVirtualHostNameToFolderMapping(VirtualHost, www, CoreWebView2HostResourceAccessKind.Allow);
        });
        Nimbus.Host.Mantel.MantelHost.Wallpaper = Games0().MantelWallpaper;
        Nimbus.Host.Mantel.MantelHost.NightLightOn = Games0().MantelNightLight;
        Nimbus.Host.Mantel.MantelHost.ReduceMotion = Games0().MantelReduceMotion;
        _pad.PointerSpeed = Games0().PointerSpeed;
        Mantel.RecoverTaskbar();
        FormClosed += (_, _) => Mantel?.Dispose();
        Mantel.GameModeRequested += () => Desktop?.Leave();
        Mantel.QuickAccessRequested += () => _overlay?.Toggle();
        Mantel.PowerRequested += a => Power(a);
        Desktop.WantsExplorer = () => Games0().DesktopStyle != "mantel-full";
        Desktop.DesktopShown = async () =>
        {
            var style = Games0().DesktopStyle;
            if (style.StartsWith("mantel") && Mantel is not null) await Mantel.StartAsync(full: style == "mantel-full");
        };
        _pad.DesktopMenu += () => BeginInvoke((MethodInvoker)(() => Mantel?.ToggleLauncherFromPad()));
        // The volume keys in full Mantel: Hearth's volume bar at the top of the screen.
        Mantel.VolumeRequested += d => { try { _system!.VolumeStep(d); var (v, m) = _system.AudioState(); Nimbus.Host.Mantel.VolumeOsd.Present(v, m); } catch (Exception ex) { SystemBridge.Log("volume: " + ex.Message); } };
        // The controller in Mantel, like on Hearth OS: the Xbox button opens the Hearth menu, a tap of View the
        // controls, and Mantel's panels are moved through with the D-pad and A instead of the mouse.
        _pad.MantelActive = () => Mantel?.Active ?? false;
        _pad.MantelPanelInFront = () => Mantel?.PanelInFront ?? false;
        _pad.MantelDesktopWindow = h => Mantel?.IsDesktopWindow(h) ?? false;
        _pad.DesktopGuide += () => BeginInvoke((MethodInvoker)(() => Mantel?.ToggleRadial()));
        _pad.DesktopHelp += () => BeginInvoke((MethodInvoker)(() => Mantel?.ToggleHelp()));
        // Hearth's on-screen keyboard (Y), and the controller picking its keys while it's up; short messages at the top.
        _pad.KeyboardOpen = () => Mantel?.KeyboardOpen ?? false;
        _pad.DesktopKeyboard += () => BeginInvoke((MethodInvoker)(() => Mantel?.ToggleKeyboard()));
        _pad.KeyboardButton += b => BeginInvoke((MethodInvoker)(() => Mantel?.KeyboardButton(b)));
        _pad.Toast += t => BeginInvoke((MethodInvoker)(() => Input.Osd.Show(t)));
        Desktop.DesktopLeaving = () => Mantel?.Stop();
        // As the shell: start on the desktop if that's the choice, or go back to it if Explorer is already the
        // desktop (Ember restarted after an update or a crash while you were on it).
        if (Program.IsShell && (Games0().StartMode == "desktop" || DesktopMode.ExplorerShellRunning()))
            BeginInvoke((MethodInvoker)(() => Desktop.Enter()));
        // As the shell, Ember starts your startup apps (Explorer would have): once per sign-in, after a moment.
        if (Program.IsShell) _ = Task.Delay(TimeSpan.FromSeconds(8)).ContinueWith(_ => StartupApps.RunOncePerSignIn());
        // A just-installed update that runs for a minute is considered good (no rollback needed).
        _ = Task.Delay(TimeSpan.FromMinutes(1)).ContinueWith(_ => Updates.UpdateService.ConfirmHealthy());
        core.WebMessageReceived += OnWebMessage;

        // Game art lives in each store's own folders; serve it on demand from https://nimbus-art.local/{id}/{kind}.
        core.AddWebResourceRequestedFilter($"https://{Library.LibraryService.ArtHost}/*", CoreWebView2WebResourceContext.Image);
        core.WebResourceRequested += (_, e) => ServeArt(env, e);

        core.Navigate($"https://{VirtualHost}/index.html");

        await InitOverlayAsync(env, www);
    }

    /// <summary>The in-game overlay: its own window and page, opened with the Xbox button or Ctrl+Alt+Q.</summary>
    private async Task InitOverlayAsync(CoreWebView2Environment env, string www)
    {
        if (_system is null || _pad is null) return;
        try
        {
            var overlay = new Overlay.OverlayForm(_system, () => _pad?.Connected ?? 0);
            overlay.HomeRequested += () =>
            {
                if (Desktop?.Active == true) { Desktop.Leave(); return; }
                if (WindowState == FormWindowState.Minimized) WindowState = FormWindowState.Normal;
                TopMost = true; TopMost = false;
                Activate();
            };
            await overlay.InitAsync(env, $"https://{VirtualHost}/index.html?overlay=1", c =>
            {
                c.SetVirtualHostNameToFolderMapping(VirtualHost, www, CoreWebView2HostResourceAccessKind.Allow);
                c.AddWebResourceRequestedFilter($"https://{Library.LibraryService.ArtHost}/*", CoreWebView2WebResourceContext.Image);
                c.WebResourceRequested += (_, e) => ServeArt(env, e);
            });
            _overlay = overlay;
            _overlayHandle = overlay.Handle;
            FormClosed += (_, _) => overlay.Dispose();

            _pad.OverlayWindow = () => _overlayHandle;
            _pad.HoldViewOpensOverlay = _system.Games.OverlayHoldView;
            _pad.OverlayToggle += () => BeginInvoke((MethodInvoker)(() => _overlay?.Toggle()));
            _pad.OverlayButton += b => BeginInvoke((MethodInvoker)(() => _overlay?.PadButton(b)));
            _system.Games.GameExited += _ => BeginInvoke((MethodInvoker)(() => _overlay?.HideOverlay(restoreFocus: false)));
            Activate();   // the overlay's first (off-screen) show took focus
            SystemBridge.Log("overlay ready");
        }
        catch (Exception ex) { SystemBridge.Log("overlay failed to start: " + ex.Message); }
    }

    private void ServeArt(CoreWebView2Environment env, CoreWebView2WebResourceRequestedEventArgs e)
    {
        try
        {
            var parts = new Uri(e.Request.Uri).AbsolutePath.Trim('/').Split('/');
            var file = parts.Length == 2 ? _system?.Games.ArtFile(Uri.UnescapeDataString(parts[0]), parts[1]) : null;
            if (file is null || !File.Exists(file))
            {
                e.Response = env.CreateWebResourceResponse(null, 404, "Not Found", "");
                return;
            }
            var type = Path.GetExtension(file).ToLowerInvariant() switch
            {
                ".png" => "image/png", ".webp" => "image/webp", ".gif" => "image/gif", _ => "image/jpeg",
            };
            var stream = new MemoryStream(File.ReadAllBytes(file));
            e.Response = env.CreateWebResourceResponse(stream, 200, "OK", $"Content-Type: {type}\r\nCache-Control: max-age=86400");
        }
        catch
        {
            e.Response = env.CreateWebResourceResponse(null, 500, "Error", "");
        }
    }

    // ------------------------------------------------------------------ bridge

    private void OnWebMessage(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        try
        {
            using var doc = JsonDocument.Parse(e.WebMessageAsJson);
            var m = doc.RootElement;
            var type = m.GetProperty("type").GetString();
            switch (type)
            {
                case "ready": // the page (re)loaded: tell it whether the host is reading the controller
                    Post(new { type = "event", name = "padHost", data = new { connected = _pad?.Connected ?? 0 } });
                    break;
                case "call": _system?.Handle(m.Clone()); break;
                case "log": SystemBridge.Log("ui: " + m.GetProperty("text").GetString()); break; // Clone: the handler is async and outlives this JsonDocument
                case "power": Power(m.GetProperty("action").GetString()); break;
                case "resume": WindowState = FormWindowState.Minimized; break; // Resume: get out of the way so the game is in front
                case "setting": break; // later: brightness, volume, Wi-Fi, TDP
            }
        }
        catch (Exception ex)
        {
            Toast("Host error: " + ex.Message);
        }
    }

    private void Post(object msg) => _web.CoreWebView2?.PostWebMessageAsJson(JsonSerializer.Serialize(msg));
    private void Toast(string text) => Post(new { type = "toast", text });

    private void Power(string? action)
    {
        switch (action)
        {
            case "sleep": Application.SetSuspendState(PowerState.Suspend, false, false); break;
            case "hibernate": Application.SetSuspendState(PowerState.Hibernate, false, false); break;   // Quick Resume games kept
            case "restart": Run("shutdown.exe", "/r /t 0"); break;
            case "shutdown": Run("shutdown.exe", "/s /t 0"); break;
            case "signout": Run("shutdown.exe", "/l"); break;
            case "desktop": Desktop?.Enter(); break;          // Desktop Mode: the normal Windows desktop
            case "explorer": StartExplorer(); break;         // just a File Explorer window
        }
    }

    /// <summary>Stop being the shell: restore Explorer for future sign-ins and switch to it now.</summary>
    /// <summary>Close so the update script can replace the app folder; it starts the new version.</summary>
    public void ExitForUpdate(Updates.UpdateService updates)
    {
        updates.BeginInstall(Program.IsShell, _windowed);
        Program.ExitCode = Updates.UpdateService.UpdateExitCode;
        _leaving = true;
        Close();
    }

    /// <summary>Starts Ember again after switching profiles (with that person's files and Steam account),
    /// without asking "Who's playing?" again.</summary>
    public void RestartForProfile()
    {
        try { File.WriteAllText(Profiles.PickedFile, "1"); } catch { }
        _leaving = true;
        if (Program.IsShell) { Program.ExitCode = Program.RestartExitCode; Close(); return; }   // the watchdog starts it again
        var exe = Environment.ProcessPath!;
        var args = string.Join(" ", Environment.GetCommandLineArgs().Skip(1).Select(x => $"\"{x}\""));
        Process.Start(new ProcessStartInfo("cmd.exe", $"/c timeout /t 2 /nobreak >nul & start \"\" \"{exe}\" {args}") { CreateNoWindow = true, UseShellExecute = false });
        Close();
    }

    public void ReturnToDesktop()
    {
        ShellRegistration.Unregister();
        StartExplorer(); // with the Shell value gone, Explorer starts as the full desktop
        _leaving = true;
        Close();
    }

    /// <summary>Back from Desktop Mode: show Ember fullscreen and in front.</summary>
    public void ShowFullscreen()
    {
        Show();
        if (!_windowed) { FormBorderStyle = FormBorderStyle.None; WindowState = FormWindowState.Normal; WindowState = FormWindowState.Maximized; }
        else if (WindowState == FormWindowState.Minimized) WindowState = FormWindowState.Normal;
        TopMost = true; TopMost = false;
        Activate();
        _web.Focus();
    }

    private Library.LibraryService Games0() => _system!.Games;

    private static void Run(string file, string args) =>
        Process.Start(new ProcessStartInfo(file, args) { UseShellExecute = false, CreateNoWindow = true });

    public static void StartExplorer() =>
        Process.Start(new ProcessStartInfo(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows), "explorer.exe")) { UseShellExecute = true });

    // ------------------------------------------------------------ escape hatch

    // Ctrl+Alt+Shift+E always starts File Explorer, even if the UI is stuck.
    // (Ctrl+Shift+Esc for Task Manager is handled by Windows itself.)
    [DllImport("user32.dll")] private static extern bool RegisterHotKey(IntPtr hWnd, int id, uint mods, uint vk);
    [DllImport("user32.dll")] private static extern bool UnregisterHotKey(IntPtr hWnd, int id);
    private const uint MOD_ALT = 0x1, MOD_CONTROL = 0x2, MOD_SHIFT = 0x4, MOD_NOREPEAT = 0x4000;
    private const int WM_HOTKEY = 0x0312;

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        RegisterHotKey(Handle, HotkeyEscapeHatch, MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT, (uint)Keys.E);
        RegisterHotKey(Handle, HotkeyOverlay, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, (uint)Keys.Q);
        RegisterHotKey(Handle, HotkeyGameMode, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, (uint)Keys.G);
    }

    protected override void OnHandleDestroyed(EventArgs e)
    {
        UnregisterHotKey(Handle, HotkeyEscapeHatch);
        UnregisterHotKey(Handle, HotkeyOverlay);
        UnregisterHotKey(Handle, HotkeyGameMode);
        base.OnHandleDestroyed(e);
    }

    protected override void WndProc(ref Message m)
    {
        if (m.Msg == WM_HOTKEY && m.WParam.ToInt32() == HotkeyEscapeHatch) StartExplorer();
        if (m.Msg == WM_HOTKEY && m.WParam.ToInt32() == HotkeyGameMode) Desktop?.Leave();
        if (m.Msg == WM_HOTKEY && m.WParam.ToInt32() == HotkeyOverlay)
        {
            // In Nimbus itself it opens Quick Access; anywhere else, the overlay.
            if (Form.ActiveForm == this) Post(new { type = "event", name = "pad", data = new { b = "guide" } });
            else _overlay?.Toggle();
        }
        base.WndProc(ref m);
    }
}
