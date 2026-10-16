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

    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(12, 16, 22) };
    private readonly bool _windowed;
    private SystemBridge? _system;

    public ShellForm(bool windowed)
    {
        _windowed = windowed;
        Text = "Nimbus";
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
            MessageBox.Show("Nimbus couldn't start its interface (WebView2): " + ex.Message, "Nimbus");
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
        // A just-installed update that runs for a minute is considered good (no rollback needed).
        _ = Task.Delay(TimeSpan.FromMinutes(1)).ContinueWith(_ => Updates.UpdateService.ConfirmHealthy());
        core.WebMessageReceived += OnWebMessage;

        // Game art lives in each store's own folders; serve it on demand from https://nimbus-art.local/{id}/{kind}.
        core.AddWebResourceRequestedFilter($"https://{Library.LibraryService.ArtHost}/*", CoreWebView2WebResourceContext.Image);
        core.WebResourceRequested += (_, e) => ServeArt(env, e);

        core.Navigate($"https://{VirtualHost}/index.html");
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
                case "ready": break; // later: push the real library here
                case "call": _system?.Handle(m.Clone()); break;
                case "log": SystemBridge.Log("ui: " + m.GetProperty("text").GetString()); break; // Clone: the handler is async and outlives this JsonDocument
                case "power": Power(m.GetProperty("action").GetString()); break;
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
            case "restart": Run("shutdown.exe", "/r /t 0"); break;
            case "shutdown": Run("shutdown.exe", "/s /t 0"); break;
            case "signout": Run("shutdown.exe", "/l"); break;
            case "desktop": StartExplorer(); break;          // opens a File Explorer window
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

    public void ReturnToDesktop()
    {
        ShellRegistration.Unregister();
        StartExplorer(); // with the Shell value gone, Explorer starts as the full desktop
        _leaving = true;
        Close();
    }

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
    }

    protected override void OnHandleDestroyed(EventArgs e)
    {
        UnregisterHotKey(Handle, HotkeyEscapeHatch);
        base.OnHandleDestroyed(e);
    }

    protected override void WndProc(ref Message m)
    {
        if (m.Msg == WM_HOTKEY && m.WParam.ToInt32() == HotkeyEscapeHatch) StartExplorer();
        base.WndProc(ref m);
    }
}
