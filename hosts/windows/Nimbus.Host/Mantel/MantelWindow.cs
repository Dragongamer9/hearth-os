using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host.Mantel;

/// <summary>The pieces of Mantel, each its own window showing mantel.html?part=…</summary>
public enum MantelPart { Bar, Launcher, Desktop, Quick, Power, Radial, Help, Keyboard }

/// <summary>A Mantel window, shaped and placed like its counterpart on Hearth OS:
/// the floating rounded taskbar along the bottom (docked as an app bar, or by hand in full Mantel, so
/// maximized windows stop above it), the launcher box in the middle of a dimmed screen, Quick Access along
/// the right edge, the power menu card, and in full Mantel the desktop itself (the wallpaper, behind every
/// other window). The rounded shapes are window regions, so what's around them is really the desktop.</summary>
internal sealed class MantelWindow : Form
{
    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(28, 21, 18) };
    public MantelPart Part { get; }
    private bool _appBar;
    private const uint CallbackMsg = 0x8000 + 0x4D;   // WM_APP + 'M'

    public event Action<JsonElement>? PageMessage;
    /// <summary>A panel (launcher, Quick Access, power menu) closed itself, e.g. by losing focus.</summary>
    public event Action? PanelClosed;
    public bool Ready { get; private set; }

    private bool IsPanel => Part is not (MantelPart.Bar or MantelPart.Desktop or MantelPart.Keyboard);
    /// <summary>Hearth's on-screen keyboard: docked above the taskbar, never takes the focus from the window you type in.</summary>
    private bool IsKeyboard => Part == MantelPart.Keyboard;
    private int _kbHeight;
    /// <summary>The window handle, readable from any thread (the controller's).</summary>
    public IntPtr Hwnd { get; private set; }

    public MantelWindow(MantelPart part)
    {
        Part = part;
        Text = part switch { MantelPart.Desktop => "Mantel desktop", MantelPart.Bar => "Mantel", _ => "Mantel " + part.ToString().ToLowerInvariant() };
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = part != MantelPart.Desktop;
        BackColor = Color.FromArgb(28, 21, 18);
        Controls.Add(_web);
        // Clicking anywhere else closes a panel, like on Hearth OS.
        if (IsPanel) Deactivate += (_, _) => { if (Visible) { Hide(); PanelClosed?.Invoke(); } };
        SizeChanged += (_, _) => Shape();
    }

    protected override bool ShowWithoutActivation => Part is MantelPart.Bar or MantelPart.Desktop or MantelPart.Keyboard;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x00000080 /* WS_EX_TOOLWINDOW: not in Alt+Tab or its own taskbar */;
            if (Part is MantelPart.Desktop or MantelPart.Keyboard) cp.ExStyle |= 0x08000000 /* WS_EX_NOACTIVATE */;
            return cp;
        }
    }

    private int S(double px) => (int)Math.Round(px * DeviceDpi / 96.0);

    public async Task InitAsync(CoreWebView2Environment env, string url, Action<CoreWebView2> setup)
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        int h = BarHeight();
        Bounds = Part switch
        {
            MantelPart.Desktop => scr,
            MantelPart.Bar => BarStrip(),
            _ => new Rectangle(-32000, -32000, S(400), S(400)),
        };
        Show();
        await _web.EnsureCoreWebView2Async(env);
        if (IsPanel || IsKeyboard) Hide();
        var core = _web.CoreWebView2;
        core.Settings.AreDefaultContextMenusEnabled = false;
        core.Settings.IsZoomControlEnabled = false;
        core.Settings.IsStatusBarEnabled = false;
        core.Settings.AreDevToolsEnabled = false;
        core.Settings.AreBrowserAcceleratorKeysEnabled = false;
        setup(core);
        // Pictures you chose as wallpaper (Hearth Settings › Appearance › Choose a picture…) live outside wwwroot.
        if (Part == MantelPart.Desktop) MapWallpapers(core);
        core.WebMessageReceived += (_, e) =>
        {
            try { using var doc = JsonDocument.Parse(e.WebMessageAsJson); PageMessage?.Invoke(doc.RootElement.Clone()); }
            catch (Exception ex) { SystemBridge.Log("mantel message: " + ex.Message); }
        };
        core.Navigate(url);
        if (Part == MantelPart.Bar && FindWindow("Shell_TrayWnd", null) != IntPtr.Zero && !OwnTray) RegisterAppBar();
        if (Part == MantelPart.Desktop) SendToBack();
        Shape();
        Hwnd = Handle;
        Ready = true;
    }

    /// <summary>Serves %LOCALAPPDATA%\Nimbus\wallpapers as https://hearth-walls.local/ (the desktop and Hearth Settings show them).</summary>
    public static void MapWallpapers(CoreWebView2 core)
    {
        try
        {
            Directory.CreateDirectory(MantelHost.CustomWallpaperDir);
            core.SetVirtualHostNameToFolderMapping("hearth-walls.local", MantelHost.CustomWallpaperDir, CoreWebView2HostResourceAccessKind.Allow);
        }
        catch (Exception ex) { SystemBridge.Log("mantel: wallpapers folder: " + ex.Message); }
    }

    // ------------------------------------------------------------------ shape

    /// <summary>An upright screen (a handheld held vertically, a rotated monitor): the bar goes down the left
    /// side, like Hearth's.</summary>
    public static bool Portrait { get { var b = Screen.PrimaryScreen!.Bounds; return b.Height > b.Width; } }

    /// <summary>The bar's strip, as placed for the screen's shape.</summary>
    private Rectangle BarStrip()
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        int t = BarHeight();
        return Portrait ? new Rectangle(scr.Left, scr.Top, t, scr.Height) : new Rectangle(scr.Left, scr.Bottom - t, scr.Width, t);
    }

    /// <summary>The rounded shape (Hearth's radii: 14 for the bar, 20 for panels). The bar floats: 8 px in from
    /// the sides, 4 from the top of its strip and 6 from the bottom of the screen.</summary>
    private void Shape()
    {
        if (!IsHandleCreated || Width <= 0 || Height <= 0) return;
        IntPtr rgn = Part switch
        {
            MantelPart.Bar => Portrait
                ? CreateRoundRectRgn(S(6), S(8), Width - S(4) + 1, Height - S(8) + 1, S(28), S(28))
                : CreateRoundRectRgn(S(8), S(4), Width - S(8) + 1, Height - S(6) + 1, S(28), S(28)),
            MantelPart.Desktop => IntPtr.Zero,
            MantelPart.Radial => CreateEllipticRgn(0, 0, Width + 1, Height + 1),
            // Keyboard.qml: 8 in from the sides, 6 from the top, Theme.radius + 4 (18) round
            MantelPart.Keyboard => CreateRoundRectRgn(S(8), S(6), Width - S(8) + 1, Height + 1, S(36), S(36)),
            _ => CreateRoundRectRgn(0, 0, Width + 1, Height + 1, S(40), S(40)),
        };
        SetWindowRgn(Handle, rgn, true);   // the window owns the region from here on
    }

    // ------------------------------------------------------------------ placement

    /// <summary>Full Mantel hosts the tray itself: there's no Explorer to take app bar messages, so the bar is
    /// placed by hand and the work area is set by FullShell.</summary>
    public static bool OwnTray { get; set; }

    /// <summary>Full Mantel: the bar along the bottom without the app bar machinery.</summary>
    public void PlaceManually()
    {
        RemoveAppBar();
        Bounds = BarStrip();
        Shape();
    }

    /// <summary>The desktop window: full screen, behind everything.</summary>
    public new void SendToBack()
    {
        Bounds = Screen.PrimaryScreen!.Bounds;
        SetWindowPos(Handle, new IntPtr(1) /* HWND_BOTTOM */, 0, 0, 0, 0, 0x2 | 0x1 | 0x10);
    }

    /// <summary>The bar's strip: 60 px like Hearth's (the floating bar inside it is 50).</summary>
    public int BarHeight() => S(60);

    /// <summary>Shows a panel where Hearth puts it: the launcher in the middle of the screen (up to 880×640),
    /// Quick Access along the right edge above the bar, the power menu card in the middle.</summary>
    public void ShowPanel()
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        Rectangle r = Part switch
        {
            MantelPart.Launcher => Center(scr, Math.Min(scr.Width - S(80), S(880)), Math.Min(scr.Height - S(120), S(640))),
            MantelPart.Quick => new Rectangle(scr.Right - Math.Min(S(400), scr.Width - S(32)) - S(12), scr.Top + S(12),
                                              Math.Min(S(400), scr.Width - S(32)), scr.Height - S(12) - S(72)),
            MantelPart.Radial => Center(scr, (int)(Math.Min(scr.Width, scr.Height) * 0.62), (int)(Math.Min(scr.Width, scr.Height) * 0.62)),
            MantelPart.Help => Center(scr, Math.Min(scr.Width - S(48), S(640)), Math.Min(scr.Height - S(48), S(520))),
            _ => Center(scr, S(360), S(430)),
        };
        Bounds = r;
        Shape();
        Show();
        Activate();
        _web.Focus();
    }

    /// <summary>Shows the on-screen keyboard along the bottom, above the taskbar, as tall as Keyboard.qml's
    /// (40 % of the screen at most, at least 260 or 22 % of its width). It keeps its space like the taskbar does,
    /// so maximized windows shrink above it and the field you're typing in stays visible.</summary>
    public void ShowKeyboard()
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        double k = DeviceDpi / 96.0;
        _kbHeight = S(Math.Min(scr.Height / k * 0.4, Math.Max(260, scr.Width / k * 0.22)));
        if (OwnTray) Bounds = KeyboardStrip();
        else if (!_appBar) RegisterAppBar();
        else PlaceAppBar();
        Shape();
        if (!Visible) Show();
    }

    public void HideKeyboard()
    {
        RemoveAppBar();
        Hide();
    }

    /// <summary>The keyboard's space by hand (full Mantel): right above the bar, or along the bottom next to a side bar.</summary>
    private Rectangle KeyboardStrip()
    {
        var scr = Screen.PrimaryScreen!.Bounds;
        int bar = BarHeight();
        return Portrait ? new Rectangle(scr.Left + bar, scr.Bottom - _kbHeight, scr.Width - bar, _kbHeight)
                        : new Rectangle(scr.Left, scr.Bottom - bar - _kbHeight, scr.Width, _kbHeight);
    }

    /// <summary>The keyboard's height, for the work area in full Mantel (0 when it's hidden).</summary>
    public int KeyboardHeight => IsKeyboard && Visible ? _kbHeight : 0;

    private static Rectangle Center(Rectangle scr, int w, int h) => new(scr.Left + (scr.Width - w) / 2, scr.Top + (scr.Height - h) / 2, w, h);

    public void Post(string json)
    {
        if (IsDisposed || _web.CoreWebView2 is null) return;
        if (InvokeRequired) BeginInvoke((MethodInvoker)(() => _web.CoreWebView2?.PostWebMessageAsJson(json)));
        else _web.CoreWebView2.PostWebMessageAsJson(json);
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
        // The keyboard asks for the bottom edge too: Windows stacks it right above the taskbar.
        bool side = Portrait && !IsKeyboard;
        int h = IsKeyboard ? _kbHeight : BarHeight();
        var scr = Screen.PrimaryScreen!.Bounds;
        var strip = IsKeyboard ? new Rectangle(scr.Left, scr.Bottom - h, scr.Width, h) : BarStrip();
        var d = Data();
        d.uEdge = side ? Native.ABE_LEFT : Native.ABE_BOTTOM;
        d.rc = new Native.RECT { Left = strip.Left, Right = strip.Right, Top = strip.Top, Bottom = strip.Bottom };
        Native.SHAppBarMessage(Native.ABM_QUERYPOS, ref d);
        if (side) d.rc.Right = d.rc.Left + h; else d.rc.Top = d.rc.Bottom - h;
        Native.SHAppBarMessage(Native.ABM_SETPOS, ref d);
        Bounds = new Rectangle(d.rc.Left, d.rc.Top, d.rc.Right - d.rc.Left, d.rc.Bottom - d.rc.Top);
        Shape();
    }

    private Native.APPBARDATA Data() => new() { cbSize = Marshal.SizeOf<Native.APPBARDATA>(), hWnd = Handle };

    protected override void WndProc(ref Message m)
    {
        if (Part == MantelPart.Desktop && m.Msg == 0x0046 /* WM_WINDOWPOSCHANGING: the desktop always stays at the back */)
        {
            var wp = Marshal.PtrToStructure<WINDOWPOS>(m.LParam);
            wp.hwndInsertAfter = new IntPtr(1);
            wp.flags &= ~0x0004u;
            Marshal.StructureToPtr(wp, m.LParam, false);
        }
        if ((Part is MantelPart.Desktop or MantelPart.Keyboard) && m.Msg == 0x0021 /* WM_MOUSEACTIVATE */) { m.Result = (IntPtr)3; return; }
        // ABN_POSCHANGED (another app bar or the screen changed): put ours back.
        if (_appBar && m.Msg == CallbackMsg && m.WParam.ToInt32() == 1) PlaceAppBar();
        base.WndProc(ref m);
    }

    [StructLayout(LayoutKind.Sequential)]
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

    [DllImport("user32.dll")] private static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr FindWindow(string? cls, string? title);
    [DllImport("gdi32.dll")] private static extern IntPtr CreateEllipticRgn(int x1, int y1, int x2, int y2);
    [DllImport("gdi32.dll")] private static extern IntPtr CreateRoundRectRgn(int x1, int y1, int x2, int y2, int w, int h);
    [DllImport("user32.dll")] private static extern int SetWindowRgn(IntPtr hWnd, IntPtr hRgn, bool redraw);
}

/// <summary>The dimmed screen behind the launcher, Quick Access and the power menu (Hearth dims it too).
/// Clicking it closes the panel. It never takes focus, so the panel keeps the keyboard.</summary>
internal sealed class Backdrop : Form
{
    public event Action? Clicked;

    public Backdrop()
    {
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = true;
        BackColor = Color.Black;
        Opacity = 0.4;
        MouseDown += (_, _) => Clicked?.Invoke();
    }

    protected override bool ShowWithoutActivation => true;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x00000080 /* WS_EX_TOOLWINDOW */ | 0x08000000 /* WS_EX_NOACTIVATE */;
            return cp;
        }
    }

    /// <summary>Over the whole screen, dimmed by this much and tinted (the launcher's is a cool dark, the
    /// others plain black).</summary>
    public void ShowDim(double opacity, Color tint)
    {
        Bounds = Screen.PrimaryScreen!.Bounds;
        BackColor = tint;
        Opacity = opacity;
        if (!Visible) Show();
    }
}

/// <summary>The volume bar that shows for a moment when the volume changes (VolumeOsd.qml): a glass pill at
/// the top of the screen with the speaker, an ember-to-flame bar and the percentage. Clicks go through.</summary>
internal sealed class VolumeOsd : Form
{
    private static VolumeOsd? _current;
    private readonly System.Windows.Forms.Timer _hide = new() { Interval = 1500 };
    private int _volume;
    private bool _muted;

    private VolumeOsd()
    {
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = true;
        BackColor = Color.FromArgb(28, 21, 18);
        DoubleBuffered = true;
        _hide.Tick += (_, _) => { _hide.Stop(); Hide(); };
    }

    private int S(double px) => (int)Math.Round(px * DeviceDpi / 96.0);
    protected override bool ShowWithoutActivation => true;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x00000080 /* WS_EX_TOOLWINDOW */ | 0x08000000 /* WS_EX_NOACTIVATE */ | 0x00000020 /* WS_EX_TRANSPARENT */ | 0x00080000 /* WS_EX_LAYERED */;
            return cp;
        }
    }

    public static void Present(int volume, bool muted)
    {
        _current ??= new VolumeOsd();
        _current.ShowLevel(volume, muted);
    }

    private void ShowLevel(int volume, bool muted)
    {
        _volume = volume; _muted = muted;
        var scr = Screen.PrimaryScreen!.Bounds;
        int w = S(360), h = S(64);
        Bounds = new Rectangle(scr.Left + (scr.Width - w) / 2, scr.Top + S(28), w, h);
        Region = RoundRegion(w, h, h / 2);
        if (!Visible) Show();
        Invalidate();
        _hide.Stop(); _hide.Start();
    }

    private static Region RoundRegion(int w, int h, int r)
    {
        using var p = new System.Drawing.Drawing2D.GraphicsPath();
        p.AddArc(0, 0, 2 * r, h, 90, 180);
        p.AddArc(w - 2 * r, 0, 2 * r, h, 270, 180);
        p.CloseFigure();
        return new Region(p);
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        var g = e.Graphics;
        g.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;
        g.TextRenderingHint = System.Drawing.Text.TextRenderingHint.ClearTypeGridFit;
        var text = Color.FromArgb(244, 236, 227);
        // the speaker
        int cx = S(20), cy = Height / 2;
        using (var pen = new Pen(text, S(1.8f)) { LineJoin = System.Drawing.Drawing2D.LineJoin.Round, StartCap = System.Drawing.Drawing2D.LineCap.Round, EndCap = System.Drawing.Drawing2D.LineCap.Round })
        {
            float k = S(22) / 24f;
            PointF P(float x, float y) => new(cx + x * k, cy - S(11) + y * k);
            g.DrawPolygon(pen, new[] { P(4, 9.5f), P(7.5f, 9.5f), P(12, 5.5f), P(12, 18.5f), P(7.5f, 14.5f), P(4, 14.5f) });
            if (_muted || _volume <= 0) { g.DrawLine(pen, P(16, 9.5f), P(21, 14.5f)); g.DrawLine(pen, P(21, 9.5f), P(16, 14.5f)); }
            else { g.DrawArc(pen, cx + 11.5f * k, cy - S(11) + 9 * k, 8 * k, 6 * k, -90, 180); g.DrawArc(pen, cx + 10.5f * k, cy - S(11) + 6.5f * k, 15 * k, 11 * k, -90, 180); }
        }
        // the bar
        int left = S(20 + 22 + 14), right = Width - S(22 + 52 + 14), barH = S(6), top = cy - barH / 2;
        using (var track = new SolidBrush(Color.FromArgb(51, 39, 31))) FillRound(g, track, left, top, right - left, barH);
        int fill = _muted ? 0 : (int)((right - left) * Math.Clamp(_volume / 100.0, 0, 1));
        if (fill > 0)
        {
            using var grad = new System.Drawing.Drawing2D.LinearGradientBrush(new Rectangle(left, top, Math.Max(1, right - left), barH), Color.FromArgb(255, 138, 61), Color.FromArgb(255, 209, 102), 0f);
            FillRound(g, grad, left, top, Math.Max(barH, fill), barH);
        }
        // the percentage
        using var font = new Font("Segoe UI Semibold", 11.25f);
        var label = _muted ? "Muted" : _volume + "%";
        var size = g.MeasureString(label, font);
        using var tb = new SolidBrush(text);
        g.DrawString(label, font, tb, Width - S(22) - size.Width, cy - size.Height / 2);
        // the edge
        using var edge = new Pen(Color.FromArgb(59, 46, 38), 1);
        using var path = new System.Drawing.Drawing2D.GraphicsPath();
        int r = Height / 2 - 1;
        path.AddArc(0, 0, 2 * r, Height - 1, 90, 180);
        path.AddArc(Width - 2 * r - 1, 0, 2 * r, Height - 1, 270, 180);
        path.CloseFigure();
        g.DrawPath(edge, path);
    }

    private static void FillRound(Graphics g, Brush b, int x, int y, int w, int h)
    {
        using var p = new System.Drawing.Drawing2D.GraphicsPath();
        int r = h;
        p.AddArc(x, y, r, h, 90, 180);
        p.AddArc(x + w - r, y, r, h, 270, 180);
        p.CloseFigure();
        g.FillPath(b, p);
    }

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        SetLayeredWindowAttributes(Handle, 0, 245, 0x2 /* LWA_ALPHA */);
    }

    [DllImport("user32.dll")] private static extern bool SetLayeredWindowAttributes(IntPtr h, uint key, byte alpha, uint flags);
}
