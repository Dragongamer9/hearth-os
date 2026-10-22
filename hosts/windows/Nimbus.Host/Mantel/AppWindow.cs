using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host.Mantel;

/// <summary>One of Mantel's own apps (Hearth Task Manager, Hearth Files): a normal window you can move, resize
/// and find on Mantel's taskbar, showing an HTML page from wwwroot in WebView2, with a dark title bar in Hearth's
/// colors, like Hearth Settings (SettingsWindow). Closing it really closes it (nothing keeps running).
/// Messages: {type:"call"} goes to the SystemBridge, {type:"mantel"} to MantelHost (both wired by MantelHost).</summary>
internal class AppWindow : Form
{
    protected readonly WebView2 Web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(15, 11, 9) };
    public event Action<JsonElement>? PageMessage;
    private bool _ready;
    private readonly string _page;
    private readonly int _w, _h;
    /// <summary>The window handle, readable from any thread (the controller's).</summary>
    public IntPtr Hwnd { get; private set; }

    public AppWindow(string title, string page, int width, int height)
    {
        Text = title;
        _page = page;
        _w = width; _h = height;
        StartPosition = FormStartPosition.Manual;
        BackColor = Color.FromArgb(15, 11, 9);
        MinimumSize = new Size(S(640), S(440));
        try { Icon = Icon.ExtractAssociatedIcon(Environment.ProcessPath!); } catch { }
        Controls.Add(Web);
    }

    protected int S(double px) => (int)Math.Round(px * DeviceDpi / 96.0);

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        Hwnd = Handle;
        int dark = 1; DwmSetWindowAttribute(Handle, 20 /* DWMWA_USE_IMMERSIVE_DARK_MODE */, ref dark, 4);
        int caption = 0x0012151C;    // COLORREF of #1c1512 (Hearth's panel)
        DwmSetWindowAttribute(Handle, 35 /* DWMWA_CAPTION_COLOR */, ref caption, 4);
        int text = 0x00E3ECF4;       // #f4ece3
        DwmSetWindowAttribute(Handle, 36 /* DWMWA_TEXT_COLOR */, ref text, 4);
    }

    protected override void OnFormClosed(FormClosedEventArgs e)
    {
        Hwnd = IntPtr.Zero;
        base.OnFormClosed(e);
    }

    /// <summary>Extra WebView2 setup for this app (e.g. Files' thumbnails), after the shared setup.</summary>
    protected virtual void Setup(CoreWebView2Environment env, CoreWebView2 core) { }

    /// <summary>Opens the window (or brings it forward). <paramref name="query"/>: e.g. "path=…" for Files; on a
    /// window that's already open it's passed to the page as the event mantel.appArgs.</summary>
    public async Task OpenAsync(CoreWebView2Environment env, string baseUrl, Action<CoreWebView2> setup, string? query)
    {
        if (!_ready)
        {
            _ready = true;
            var wa = Screen.PrimaryScreen!.WorkingArea;
            int w = Math.Min(S(_w), wa.Width - S(40)), h = Math.Min(S(_h), wa.Height - S(40));
            Bounds = new Rectangle(wa.Left + (wa.Width - w) / 2, wa.Top + (wa.Height - h) / 2, w, h);
            Show();
            await Web.EnsureCoreWebView2Async(env);
            var core = Web.CoreWebView2;
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsZoomControlEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.AreDevToolsEnabled = false;
            core.Settings.AreBrowserAcceleratorKeysEnabled = false;
            setup(core);
            Setup(env, core);
            core.WebMessageReceived += (_, e) =>
            {
                if (!SystemBridge.FromOwnPage(e.Source)) return;   // only Ember's own pages
                try { using var doc = JsonDocument.Parse(e.WebMessageAsJson); PageMessage?.Invoke(doc.RootElement.Clone()); }
                catch (Exception ex) { SystemBridge.Log(Text + " message: " + ex.Message); }
            };
            core.Navigate(baseUrl + _page + (string.IsNullOrEmpty(query) ? "" : "?" + query));
        }
        else
        {
            if (!Visible) Show();
            if (WindowState == FormWindowState.Minimized) WindowState = FormWindowState.Normal;
            if (!string.IsNullOrEmpty(query)) Post(JsonSerializer.Serialize(new { type = "event", name = "mantel.appArgs", data = new { query } }));
        }
        Activate();
        Native.ForceForeground(Handle);
        Web.Focus();
    }

    public void Post(string json)
    {
        if (IsDisposed || Web.IsDisposed) return;
        if (InvokeRequired) { try { BeginInvoke((MethodInvoker)(() => { if (!Web.IsDisposed) Web.CoreWebView2?.PostWebMessageAsJson(json); })); } catch { } return; }
        Web.CoreWebView2?.PostWebMessageAsJson(json);
    }

    [DllImport("dwmapi.dll")] private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int value, int size);
}

/// <summary>Hearth Task Manager (mantel-tasks.html).</summary>
internal sealed class TasksWindow : AppWindow
{
    public const string Title = "Hearth Task Manager";
    public TasksWindow() : base(Title, "mantel-tasks.html", 1080, 720) { }
}

/// <summary>Apps (mantel-apps.html): Mantel's app store, backed by winget (Mantel/Winget.cs), like AppStore.qml on Hearth OS.</summary>
internal sealed class AppsWindow : AppWindow
{
    public const string Title = "Apps";
    public AppsWindow() : base(Title, "mantel-apps.html", 1100, 720) { }
}

/// <summary>Hearth Files (mantel-files.html). Pictures, videos and apps get their thumbnails or icons from Windows'
/// shell, served to the page from https://hearth-files.local/thumb?p=&lt;path&gt;&amp;s=&lt;size&gt; (PNG).</summary>
internal sealed class FilesWindow : AppWindow
{
    public const string Title = "Hearth Files";
    public const string ThumbHost = "hearth-files.local";
    private static readonly SemaphoreSlim Gate = new(4);

    public FilesWindow() : base(Title, "mantel-files.html", 1180, 760) { }

    protected override void Setup(CoreWebView2Environment env, CoreWebView2 core) => ServeThumbs(env, core);

    /// <summary>Serves https://hearth-files.local/thumb?p=…&amp;s=… (thumbnails, or icons with icon=1) to a page: Hearth
    /// Files, and full Mantel's desktop icons (shortcuts show their target's icon).</summary>
    public static void ServeThumbs(CoreWebView2Environment env, CoreWebView2 core)
    {
        core.AddWebResourceRequestedFilter($"https://{ThumbHost}/*", CoreWebView2WebResourceContext.Image);
        core.WebResourceRequested += async (_, e) =>
        {
            CoreWebView2Deferral? deferral = null;
            try
            {
                var uri = new Uri(e.Request.Uri);
                string path = Query(uri.Query, "p") ?? "";
                int size = int.TryParse(Query(uri.Query, "s"), out var px) ? Math.Clamp(px, 16, 512) : 256;
                bool iconOnly = Query(uri.Query, "icon") == "1";
                if (path.Length == 0) { e.Response = env.CreateWebResourceResponse(null, 404, "Not Found", ""); return; }
                deferral = e.GetDeferral();
                byte[]? png = null;
                await Gate.WaitAsync();
                try { png = await Task.Run(() => Native.ShellImagePng(path, size, iconOnly ? 4 : 0)); }
                finally { Gate.Release(); }
                // Back on the UI thread (the await resumes there): WebView2 objects are made here.
                e.Response = png is null
                    ? env.CreateWebResourceResponse(null, 404, "Not Found", "")
                    : env.CreateWebResourceResponse(new MemoryStream(png), 200, "OK", "Content-Type: image/png\r\nCache-Control: max-age=600");
            }
            catch
            {
                try { e.Response = env.CreateWebResourceResponse(null, 500, "Error", ""); } catch { }
            }
            finally { deferral?.Complete(); }
        };
    }

    /// <summary>One value from "?a=1&amp;b=2" (the page encodes with encodeURIComponent).</summary>
    private static string? Query(string query, string key)
    {
        foreach (var part in query.TrimStart('?').Split('&'))
        {
            int i = part.IndexOf('=');
            var k = i < 0 ? part : part[..i];
            if (k == key) return Uri.UnescapeDataString(i < 0 ? "" : part[(i + 1)..]);
        }
        return null;
    }
}
