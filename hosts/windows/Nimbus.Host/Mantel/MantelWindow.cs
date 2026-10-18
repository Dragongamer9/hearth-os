using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host.Mantel;

/// <summary>A Mantel window: the taskbar (docked to the bottom of the screen as an app bar, so maximized
/// windows stop above it), the launcher (a panel above the taskbar), or, in full Mantel, the desktop itself
/// (the wallpaper, still or moving, behind every other window). All show mantel.html.</summary>
internal sealed class MantelWindow : Form
{
    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(15, 11, 9) };
    private readonly bool _bar;
    private readonly bool _desktop;
    private bool _appBar;
    private const uint CallbackMsg = 0x8000 + 0x4D;   // WM_APP + 'M'

    public event Action<JsonElement>? PageMessage;
    public bool Ready { get; private set; }

    public MantelWindow(bool bar, bool desktop = false)
    {
        _bar = bar;
        _desktop = desktop;
        Text = desktop ? "Mantel desktop" : bar ? "Mantel" : "Mantel launcher";
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = !desktop;
        BackColor = Color.FromArgb(15, 11, 9);
        Controls.Add(_web);
        if (!bar && !desktop) Deactivate += (_, _) => { if (Visible) Hide(); };   // clicking elsewhere closes the launcher
    }

    protected override bool ShowWithoutActivation => _bar || _desktop;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x00000080 /* WS_EX_TOOLWINDOW: not in Alt+Tab or its own taskbar */;
            if (_desktop) cp.ExStyle |= 0x08000000 /* WS_EX_NOACTIVATE */;
            return cp;
        }
    }

    public async Task InitAsync(CoreWebView2Environment env, string url, Action<CoreWebView2> setup)
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        int h = BarHeight();
        Bounds = _desktop ? scr : _bar ? new Rectangle(scr.Left, scr.Bottom - h, scr.Width, h) : new Rectangle(-32000, -32000, 760, 640);
        Show();
        await _web.EnsureCoreWebView2Async(env);
        if (!_bar && !_desktop) Hide();
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
        if (_bar && FindWindow("Shell_TrayWnd", null) != IntPtr.Zero && !OwnTray) RegisterAppBar();
        if (_desktop) SendToBack();
        Ready = true;
    }

    /// <summary>Full Mantel hosts the tray itself: there's no Explorer to take app bar messages, so the bar is
    /// placed by hand and the work area is set by FullShell.</summary>
    public static bool OwnTray { get; set; }

    /// <summary>Full Mantel: the bar along the bottom without the app bar machinery.</summary>
    public void PlaceManually()
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        int h = BarHeight();
        RemoveAppBar();
        Bounds = new Rectangle(scr.Left, scr.Bottom - h, scr.Width, h);
    }

    /// <summary>The desktop window: full screen, behind everything.</summary>
    public new void SendToBack()
    {
        Bounds = Screen.PrimaryScreen!.Bounds;
        SetWindowPos(Handle, new IntPtr(1) /* HWND_BOTTOM */, 0, 0, 0, 0, 0x2 | 0x1 | 0x10);
    }

    [System.Runtime.InteropServices.DllImport("user32.dll")] private static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)] private static extern IntPtr FindWindow(string? cls, string? title);

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

    public void EnsureAppBar() { if (!_appBar && !OwnTray) RegisterAppBar(); }

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
        if (_desktop && m.Msg == 0x0046 /* WM_WINDOWPOSCHANGING: the desktop always stays at the back */)
        {
            var wp = System.Runtime.InteropServices.Marshal.PtrToStructure<WINDOWPOS>(m.LParam);
            wp.hwndInsertAfter = new IntPtr(1);
            wp.flags &= ~0x0004u;
            System.Runtime.InteropServices.Marshal.StructureToPtr(wp, m.LParam, false);
        }
        if (_desktop && m.Msg == 0x0021 /* WM_MOUSEACTIVATE */) { m.Result = (IntPtr)3; return; }
        // ABN_POSCHANGED (another app bar or the screen changed): put ours back.
        if (_appBar && m.Msg == CallbackMsg && m.WParam.ToInt32() == 1) PlaceAppBar();
        base.WndProc(ref m);
    }

    [System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
    private struct WINDOWPOS { public IntPtr hwnd, hwndInsertAfter; public int x, y, cx, cy; public uint flags; }

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
