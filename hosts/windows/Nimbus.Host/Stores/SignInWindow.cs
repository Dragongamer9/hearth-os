using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Nimbus.Host.Stores;

/// <summary>
/// A full-screen window showing a store's own sign-in page. You type your password into the store's page;
/// Nimbus only watches for the page the store sends you to afterwards, which carries a one-time code.
/// Works with mouse, touch, keyboard and controller (the controller mouse turns on here by itself,
/// Y opens an on-screen keyboard, B cancels).
/// </summary>
public sealed class SignInWindow : Form
{
    private static readonly HashSet<IntPtr> OpenWindows = new();
    /// <summary>For the controller bridge: is this window a sign-in page?</summary>
    public static bool IsSignInWindow(IntPtr hwnd) { lock (OpenWindows) return OpenWindows.Contains(hwnd); }

    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(12, 16, 22) };
    private readonly Label _status = new() { AutoSize = false, Dock = DockStyle.Fill, TextAlign = ContentAlignment.MiddleLeft };
    private readonly string _startUrl;
    private readonly Func<string, string?>? _onStarting;
    private readonly Func<string, CoreWebView2, Task<string?>>? _onLoaded;
    private readonly TaskCompletionSource<string?> _result = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private bool _done;

    private SignInWindow(string title, string startUrl, Func<string, string?>? onStarting, Func<string, CoreWebView2, Task<string?>>? onLoaded)
    {
        _startUrl = startUrl;
        _onStarting = onStarting;
        _onLoaded = onLoaded;

        Text = title;
        FormBorderStyle = FormBorderStyle.None;
        WindowState = FormWindowState.Maximized;
        ShowInTaskbar = false;
        BackColor = Color.FromArgb(12, 16, 22);
        ForeColor = Color.FromArgb(233, 238, 244);
        Font = new Font("Segoe UI", 12f);
        KeyPreview = true;

        var bar = new TableLayoutPanel
        {
            Dock = DockStyle.Top, Height = 68, ColumnCount = 4, RowCount = 1,
            BackColor = Color.FromArgb(22, 29, 39), Padding = new Padding(20, 10, 16, 10),
        };
        bar.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        bar.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        bar.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        bar.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        var heading = new Label { Text = title, AutoSize = true, Font = new Font("Segoe UI Semibold", 15f), Anchor = AnchorStyles.Left, Margin = new Padding(0, 0, 24, 0) };
        _status.ForeColor = Color.FromArgb(150, 164, 182);
        _status.Font = new Font("Segoe UI", 11f);
        _status.Text = "Loading…";
        bar.Controls.Add(heading, 0, 0);
        bar.Controls.Add(_status, 1, 0);
        bar.Controls.Add(MakeButton("⌨  Keyboard  (Y)", (_, _) => TouchKeyboard.Toggle()), 2, 0);
        bar.Controls.Add(MakeButton("✕  Cancel  (B)", (_, _) => Close()), 3, 0);

        Controls.Add(_web);
        Controls.Add(bar);

        Load += async (_, _) => await InitAsync();
        HandleCreated += (_, _) => { lock (OpenWindows) OpenWindows.Add(Handle); };
        HandleDestroyed += (_, _) => { lock (OpenWindows) OpenWindows.Remove(Handle); };
        FormClosed += (_, _) => { _result.TrySetResult(null); _web.Dispose(); };
    }

    private static Button MakeButton(string text, EventHandler click)
    {
        var b = new Button
        {
            Text = text, AutoSize = true, FlatStyle = FlatStyle.Flat, Anchor = AnchorStyles.Right,
            BackColor = Color.FromArgb(36, 46, 60), ForeColor = Color.FromArgb(233, 238, 244),
            Padding = new Padding(10, 4, 10, 4), Margin = new Padding(8, 0, 0, 0), TabStop = false,
        };
        b.FlatAppearance.BorderSize = 0;
        b.Click += click;
        return b;
    }

    protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
    {
        if (keyData == Keys.Escape) { Close(); return true; }
        return base.ProcessCmdKey(ref msg, keyData);
    }

    private async Task InitAsync()
    {
        try
        {
            // Separate browser profile from the Nimbus UI. Cookies are cleared each time so you always choose the account.
            var dir = Path.Combine(HelperTools.DataDir, "SignIn");
            var env = await CoreWebView2Environment.CreateAsync(null, dir);
            await _web.EnsureCoreWebView2Async(env);
            var core = _web.CoreWebView2;
            core.Settings.AreDevToolsEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.CookieManager.DeleteAllCookies();

            // B on the controller sends Esc; inside the page Esc doesn't reach the form, so forward it.
            await core.AddScriptToExecuteOnDocumentCreatedAsync(
                "window.addEventListener('keydown',e=>{if(e.key==='Escape'&&window.chrome&&chrome.webview){chrome.webview.postMessage('esc')}},true);");
            core.WebMessageReceived += (_, e) => { if (e.TryGetWebMessageAsString() == "esc") BeginInvoke((MethodInvoker)Close); };

            core.NavigationStarting += (_, e) =>
            {
                if (_done || _onStarting is null) return;
                string? code;
                try { code = _onStarting(e.Uri); } catch (Exception ex) { Fail(ex); e.Cancel = true; return; }
                if (code is not null) { e.Cancel = true; Finish(code); }
            };
            core.NavigationCompleted += async (_, e) =>
            {
                if (_done) return;
                _status.Text = "Sign in with your account. Ember never sees your password.";
                if (_onLoaded is null) return;
                try
                {
                    var code = await _onLoaded(core.Source, core);
                    if (code is not null) Finish(code);
                }
                catch (Exception ex) { Fail(ex); }
            };
            // Links that open new windows (help pages, "forgot password") stay in this window.
            core.NewWindowRequested += (_, e) =>
            {
                if (e.IsUserInitiated && !e.Uri.StartsWith("javascript:", StringComparison.OrdinalIgnoreCase)) { e.Handled = true; core.Navigate(e.Uri); }
            };
            core.Navigate(_startUrl);
            _web.Focus();
        }
        catch (Exception ex) { Fail(ex); }
    }

    private void Finish(string code)
    {
        if (_done) return;
        _done = true;
        _result.TrySetResult(code);
        BeginInvoke((MethodInvoker)Close);
    }

    private void Fail(Exception ex)
    {
        if (_done) return;
        _done = true;
        _result.TrySetException(ex);
        BeginInvoke((MethodInvoker)Close);
    }

    /// <summary>
    /// Shows the sign-in page and waits. Returns the code the store handed back, or null if you closed the window.
    /// <paramref name="onStarting"/> sees each URL before it loads (return a code to finish);
    /// <paramref name="onLoaded"/> runs after each page loads and may read the page.
    /// </summary>
    public static Task<string?> RunAsync(IWin32Window? owner, string title, string url,
                                         Func<string, string?>? onStarting = null,
                                         Func<string, CoreWebView2, Task<string?>>? onLoaded = null)
    {
        var w = new SignInWindow(title, url, onStarting, onLoaded);
        if (owner is Form f) w.Owner = f;
        w.Show();
        w.Activate();
        return w._result.Task;
    }

    /// <summary>Reads the current page's text as JSON (for pages that answer with a JSON body).</summary>
    public static async Task<JsonDocument?> ReadJsonAsync(CoreWebView2 core)
    {
        var raw = await core.ExecuteScriptAsync("document.body ? document.body.innerText : ''");
        var text = JsonSerializer.Deserialize<string>(raw) ?? "";
        text = text.Trim();
        if (!text.StartsWith('{')) return null;
        try { return JsonDocument.Parse(text); } catch { return null; }
    }

    /// <summary>Gets a query-string value from a URL.</summary>
    public static string? Query(string url, string key)
    {
        var q = url.IndexOf('?');
        if (q < 0) return null;
        foreach (var part in url[(q + 1)..].Split('#')[0].Split('&'))
        {
            var kv = part.Split('=', 2);
            if (kv.Length == 2 && kv[0] == key) return Uri.UnescapeDataString(kv[1].Replace('+', ' '));
        }
        return null;
    }
}
