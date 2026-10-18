using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host.Mantel;

/// <summary>A Mantel window: the taskbar (docked to the bottom of the screen as an app bar, so maximized
/// windows stop above it) or the launcher (a panel above the taskbar). Both show mantel.html.</summary>
internal sealed class MantelWindow : Form
{
    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(15, 11, 9) };
    private readonly bool _bar;
    private bool _appBar;
    private const uint CallbackMsg = 0x8000 + 0x4D;   // WM_APP + 'M'

    public event Action<JsonElement>? PageMessage;
    public bool Ready { get; private set; }

    public MantelWindow(bool bar)
    {
        _bar = bar;
        Text = bar ? "Mantel" : "Mantel launcher";
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = true;
        BackColor = Color.FromArgb(15, 11, 9);
        Controls.Add(_web);
        if (!bar) Deactivate += (_, _) => { if (Visible) Hide(); };   // clicking elsewhere closes the launcher
    }

    protected override bool ShowWithoutActivation => _bar;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x00000080 /* WS_EX_TOOLWINDOW: not in Alt+Tab or its own taskbar */;
            return cp;
        }
    }

    public async Task InitAsync(CoreWebView2Environment env, string url, Action<CoreWebView2> setup)
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        int h = BarHeight();
        Bounds = _bar ? new Rectangle(scr.Left, scr.Bottom - h, scr.Width, h) : new Rectangle(-32000, -32000, 760, 640);
        Show();
        await _web.EnsureCoreWebView2Async(env);
        if (!_bar) Hide();
        var core = _web.CoreWebView2;
        core.Settings.AreDefaultContextMenusEnabled = false;
        core.Settings.IsZoomControlEnabled = false;
        core.Settings.IsStatusBarEnabled = false;
        core.Settings.AreDevToolsEnabled = false;
        core.Settings.AreBrowserAcceleratorKeysEnabled = false;
        setup(core);
        core.WebMessageReceived += (_, e) =>
        {
            try { using var doc = JsonDocument.Parse(e.WebMessageAsJson); PageMessage?.Invoke(doc.RootElement.Clone()); }
            catch (Exception ex) { SystemBridge.Log("mantel message: " + ex.Message); }
        };
        core.Navigate(url);
        if (_bar) RegisterAppBar();
        Ready = true;
    }

    public int BarHeight() => (int)Math.Round(54 * DeviceDpi / 96.0);

    public void Post(string json)
    {
        if (IsDisposed || _web.CoreWebView2 is null) return;
        if (InvokeRequired) BeginInvoke((MethodInvoker)(() => _web.CoreWebView2?.PostWebMessageAsJson(json)));
        else _web.CoreWebView2.PostWebMessageAsJson(json);
    }

    /// <summary>The launcher, above the taskbar on the left.</summary>
    public void ShowLauncher(int barHeight)
    {
        var wa = Screen.PrimaryScreen!.Bounds;
        int w = (int)Math.Round(760 * DeviceDpi / 96.0), h = (int)Math.Round(640 * DeviceDpi / 96.0);
        int margin = (int)Math.Round(10 * DeviceDpi / 96.0);
        Bounds = new Rectangle(wa.Left + margin, wa.Bottom - barHeight - h - margin, Math.Min(w, wa.Width - 2 * margin), h);
        Show();
        Activate();
        _web.Focus();
    }

    // ------------------------------------------------------------------ app bar

    private void RegisterAppBar()
    {
        var d = Data();
        d.uCallbackMessage = CallbackMsg;
        Native.SHAppBarMessage(Native.ABM_NEW, ref d);
        _appBar = true;
        PlaceAppBar();
    }

    /// <summary>Reserves the bottom of the screen (Windows then keeps maximized windows above it).</summary>
    public void PlaceAppBar()
    {
        if (!_appBar) return;
        var scr = Screen.PrimaryScreen!.Bounds;
        int h = BarHeight();
        var d = Data();
        d.uEdge = Native.ABE_BOTTOM;
        d.rc = new Native.RECT { Left = scr.Left, Right = scr.Right, Top = scr.Bottom - h, Bottom = scr.Bottom };
        Native.SHAppBarMessage(Native.ABM_QUERYPOS, ref d);
        d.rc.Top = d.rc.Bottom - h;
        Native.SHAppBarMessage(Native.ABM_SETPOS, ref d);
        Bounds = new Rectangle(d.rc.Left, d.rc.Top, d.rc.Right - d.rc.Left, d.rc.Bottom - d.rc.Top);
    }

    private Native.APPBARDATA Data() => new() { cbSize = System.Runtime.InteropServices.Marshal.SizeOf<Native.APPBARDATA>(), hWnd = Handle };

    protected override void WndProc(ref Message m)
    {
        // ABN_POSCHANGED (another app bar or the screen changed): put ours back.
        if (_appBar && m.Msg == CallbackMsg && m.WParam.ToInt32() == 1) PlaceAppBar();
        base.WndProc(ref m);
    }

    public void RemoveAppBar()
    {
        if (!_appBar) return;
        var d = Data();
        Native.SHAppBarMessage(Native.ABM_REMOVE, ref d);
        _appBar = false;
    }

    protected override void OnFormClosed(FormClosedEventArgs e)
    {
        RemoveAppBar();
        base.OnFormClosed(e);
    }
}
