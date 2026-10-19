using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host.Mantel;

/// <summary>Hearth Settings in Mantel (mantel-settings.html): a normal window you can move, resize and find
/// on the taskbar, like Mantel's settings app on Hearth OS, with a dark title bar in Hearth's colors.</summary>
internal sealed class SettingsWindow : Form
{
    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(15, 11, 9) };
    public event Action<JsonElement>? PageMessage;
    private bool _ready;

    public SettingsWindow()
    {
        Text = "Hearth Settings";
        StartPosition = FormStartPosition.Manual;
        BackColor = Color.FromArgb(15, 11, 9);
        MinimumSize = new Size(S(720), S(480));
        try { Icon = Icon.ExtractAssociatedIcon(Environment.ProcessPath!); } catch { }
        Controls.Add(_web);
        FormClosing += (_, e) => { if (e.CloseReason == CloseReason.UserClosing) { e.Cancel = true; Hide(); } };   // kept for next time
    }

    private int S(double px) => (int)Math.Round(px * DeviceDpi / 96.0);

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        int dark = 1; DwmSetWindowAttribute(Handle, 20 /* DWMWA_USE_IMMERSIVE_DARK_MODE */, ref dark, 4);
        int caption = 0x0012151C;    // COLORREF of #1c1512 (Hearth's panel)
        DwmSetWindowAttribute(Handle, 35 /* DWMWA_CAPTION_COLOR */, ref caption, 4);
        int text = 0x00E3ECF4;       // #f4ece3
        DwmSetWindowAttribute(Handle, 36 /* DWMWA_TEXT_COLOR */, ref text, 4);
    }

    public async Task OpenAsync(CoreWebView2Environment env, string url, Action<CoreWebView2> setup, string? page)
    {
        if (!_ready)
        {
            var wa = Screen.PrimaryScreen!.WorkingArea;
            int w = Math.Min(S(1100), wa.Width - S(40)), h = Math.Min(S(720), wa.Height - S(40));
            Bounds = new Rectangle(wa.Left + (wa.Width - w) / 2, wa.Top + (wa.Height - h) / 2, w, h);
            Show();
            await _web.EnsureCoreWebView2Async(env);
            var core = _web.CoreWebView2;
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsZoomControlEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.AreDevToolsEnabled = false;
            core.Settings.AreBrowserAcceleratorKeysEnabled = false;
            setup(core);
            MantelWindow.MapWallpapers(core);   // thumbnails of the pictures you chose as wallpaper
            core.WebMessageReceived += (_, e) =>
            {
                try { using var doc = JsonDocument.Parse(e.WebMessageAsJson); PageMessage?.Invoke(doc.RootElement.Clone()); }
                catch (Exception ex) { SystemBridge.Log("settings message: " + ex.Message); }
            };
            core.Navigate(url + (string.IsNullOrEmpty(page) ? "" : "?page=" + Uri.EscapeDataString(page)));
            _ready = true;
        }
        else
        {
            if (!Visible) Show();
            if (WindowState == FormWindowState.Minimized) WindowState = FormWindowState.Normal;
            if (!string.IsNullOrEmpty(page)) Post(JsonSerializer.Serialize(new { type = "event", name = "mantel.settingsPage", data = new { page } }));
        }
        Activate();
        Native.ForceForeground(Handle);
        _web.Focus();
    }

    public void Post(string json)
    {
        if (IsDisposed || _web.CoreWebView2 is null) return;
        if (InvokeRequired) BeginInvoke((MethodInvoker)(() => _web.CoreWebView2?.PostWebMessageAsJson(json)));
        else _web.CoreWebView2.PostWebMessageAsJson(json);
    }

    [DllImport("dwmapi.dll")] private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int value, int size);
}
