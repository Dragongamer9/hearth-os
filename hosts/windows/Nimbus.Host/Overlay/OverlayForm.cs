using System.Drawing.Imaging;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using Microsoft.Win32;

namespace Nimbus.Host.Overlay;

/// <summary>
/// The in-game overlay: a Quick Access panel that slides in from the right edge over whatever is running.
/// It's a normal always-on-top window (no code in the game), so it shows over borderless and windowed games
/// and the "fullscreen optimized" mode most games use on Windows 10/11.
/// The page is the same index.html as Hearth OS's overlay over games (?overlay=full): the same Quick Access, tabs and
/// behaviour everywhere (Open apps included, answered here); in a window the panel's width it fills the window.
/// </summary>
public sealed class OverlayForm : Form
{
    private readonly WebView2 _web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(22, 29, 39) };
    private readonly SystemBridge _bridge;
    private readonly Func<int> _controllers;
    private IntPtr _prevForeground;
    private bool _busy;          // screenshot in progress: ignore focus changes
    private bool _ready;
    private DateTime _shownAt;
    private readonly System.Windows.Forms.Timer _watch = new() { Interval = 500 };

    /// <summary>Raised when you choose "Back to Ember" (the game keeps running).</summary>
    public event Action? HomeRequested;

    public OverlayForm(SystemBridge bridge, Func<int> controllers)
    {
        _bridge = bridge;
        _controllers = controllers;
        Text = "Ember overlay";
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = true;
        BackColor = Color.FromArgb(22, 29, 39);
        Controls.Add(_web);
        Deactivate += (_, _) =>
        {
            // Clicking the game (or Alt+Tab) closes the overlay, like Steam's Quick Access.
            if (!_busy && Visible && DateTime.UtcNow - _shownAt > TimeSpan.FromMilliseconds(400)) HideOverlay(restoreFocus: false);
        };
        // Belt and braces: Deactivate isn't always raised (a game grabbing the foreground, a fullscreen switch),
        // which left the overlay up over the game without the controller. Closed if something else is in front.
        _watch.Tick += (_, _) =>
        {
            if (!Visible || _busy || DateTime.UtcNow - _shownAt < TimeSpan.FromMilliseconds(800)) return;
            var fg = GetForegroundWindow();
            if (fg != Handle && fg != IntPtr.Zero) { SystemBridge.Log("overlay: something else is in front, closing"); HideOverlay(restoreFocus: false); }
        };
    }

    protected override bool ShowWithoutActivation => false;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x00000080 /* WS_EX_TOOLWINDOW: not in Alt+Tab */ | 0x00000008 /* WS_EX_TOPMOST */;
            return cp;
        }
    }

    /// <summary>Creates the page while hidden, so the overlay opens instantly later.</summary>
    public async Task InitAsync(CoreWebView2Environment env, string url, Action<CoreWebView2> setupArt)
    {
        // WebView2 needs a real (shown) window to start; show it off-screen once, then hide it.
        Bounds = new Rectangle(-32000, -32000, 440, 600);
        _busy = true;
        Show();
        try { await _web.EnsureCoreWebView2Async(env); }
        finally { Hide(); _busy = false; }
        var core = _web.CoreWebView2;
        core.Settings.AreDefaultContextMenusEnabled = false;
        core.Settings.IsZoomControlEnabled = false;
        core.Settings.IsStatusBarEnabled = false;
        core.Settings.AreDevToolsEnabled = false;
        core.Settings.AreBrowserAcceleratorKeysEnabled = false;
        setupArt(core);
        core.WebMessageReceived += OnMessage;
        _bridge.AddListener(Post);
        core.Navigate(url);
        _ready = true;
    }

    private void Post(string json)
    {
        if (IsDisposed || _web.CoreWebView2 is null) return;
        if (InvokeRequired) BeginInvoke((MethodInvoker)(() => _web.CoreWebView2?.PostWebMessageAsJson(json)));
        else _web.CoreWebView2.PostWebMessageAsJson(json);
    }

    private void Event(string name, object data) => Post(JsonSerializer.Serialize(new { type = "event", name, data }, new JsonSerializerOptions(JsonSerializerDefaults.Web)));

    private void OnMessage(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        if (!SystemBridge.FromOwnPage(e.Source)) { SystemBridge.Log("overlay: ignored a message from " + e.Source); return; }
        try
        {
            using var doc = JsonDocument.Parse(e.WebMessageAsJson);
            var m = doc.RootElement;
            switch (m.GetProperty("type").GetString())
            {
                case "ready": Event("padHost", new { connected = _controllers() }); break;
                case "call":
                    var method = m.GetProperty("method").GetString();
                    if (method == "overlay.screenshot") _ = ScreenshotAsync(m.GetProperty("id").GetInt32());
                    else if (method is "apps.list" or "apps.focus" or "apps.close") _ = AppsAsync(method, m.Clone());
                    else _bridge.Handle(m.Clone(), Post);
                    break;
                case "overlay":
                    var action = m.GetProperty("action").GetString();
                    if (action == "close") HideOverlay(restoreFocus: true);
                    else if (action == "home") { HideOverlay(restoreFocus: false); HomeRequested?.Invoke(); }
                    break;
                case "log": SystemBridge.Log("overlay: " + m.GetProperty("text").GetString()); break;
                case "power": break;   // power actions are only offered in the main UI
            }
        }
        catch (Exception ex) { SystemBridge.Log("overlay message failed: " + ex.Message); }
    }

    /// <summary>A controller button while the overlay is in front.</summary>
    public void PadButton(string b) => Event("pad", new { b });

    // ------------------------------------------------------------------ show / hide

    public void Toggle()
    {
        if (!_ready) return;
        if (Visible && GetForegroundWindow() == Handle) HideOverlay(restoreFocus: true);
        else ShowOverlay();
    }

    public void ShowOverlay()
    {
        if (!_ready) return;
        var fg = GetForegroundWindow();
        if (fg != Handle) _prevForeground = fg;
        var screen = _prevForeground != IntPtr.Zero ? Screen.FromHandle(_prevForeground) : Screen.PrimaryScreen!;
        var b = screen.Bounds;
        double scale = DpiOf(b) / 96.0;
        Bounds = ShapeFor(_bridge.Games.OverlayQaStyle, b, scale);
        _shownAt = DateTime.UtcNow;
        if (!Visible) Show();
        TopMost = true;
        ForceForeground(Handle);
        _watch.Start();
        _web.Focus();
        Event("overlay.shown", new { });
        SystemBridge.Log("overlay shown");
    }

    /// <summary>Where the overlay window sits, for a Quick Access style. The side panel (the classic) is the column
    /// down the right edge it has always been, 440 px at 100%. The top dock gets the screen's full width and that
    /// same reference size in height instead: a bar across the top with the panel opening under it needs width, and
    /// the page draws its bar in the top ~170 px and scrolls whatever else it needs inside what it is given
    /// (#qam.top in ui/index.html). "side" is the classic panel; anything else - including null, before a style has
    /// been chosen - is the dock, which is the page's own default. The dock's height is the one number to change if
    /// it should be taller or shorter over a game.</summary>
    private static Rectangle ShapeFor(string? style, Rectangle screen, double scale)
    {
        int reference = (int)Math.Round(440 * scale);
        if (style == "side")
        {
            int width = Math.Min(screen.Width, reference);
            return new Rectangle(screen.Right - width, screen.Top, width, screen.Height);
        }
        return new Rectangle(screen.Left, screen.Top, screen.Width, Math.Min(screen.Height, reference));
    }

    /// <summary>The style was changed while the overlay is up (Settings › Appearance inside it): follow it now
    /// rather than at the next open.</summary>
    public void Reshape()
    {
        if (!Visible || _busy) return;
        var screen = Screen.FromHandle(Handle).Bounds;
        Bounds = ShapeFor(_bridge.Games.OverlayQaStyle, screen, DpiOf(screen) / 96.0);
    }

    public void HideOverlay(bool restoreFocus)
    {
        _watch.Stop();
        if (!Visible) return;
        Event("overlay.hidden", new { });
        Hide();
        if (restoreFocus && _prevForeground != IntPtr.Zero && IsWindow(_prevForeground)) SetForegroundWindow(_prevForeground);
    }

    // ------------------------------------------------------------------ Quick Access › Open apps

    /// <summary>The page's apps.* calls: the windows you could Alt+Tab to, switched to or closed from Quick Access
    /// (as on Hearth OS). Ember itself is first; the running game's windows are marked.</summary>
    private async Task AppsAsync(string method, JsonElement m)
    {
        int id = m.GetProperty("id").GetInt32();
        var args = m.TryGetProperty("args", out var a) && a.ValueKind == JsonValueKind.Object ? a : default;
        object? value = null; string? error = null;
        try
        {
            switch (method)
            {
                case "apps.list": value = ListApps(); break;
                case "apps.focus":
                {
                    long win = Num(args, "id"); int pid = (int)Num(args, "pid");
                    HideOverlay(restoreFocus: false);
                    if (pid == Environment.ProcessId || win == 0) { HomeRequested?.Invoke(); break; }
                    var h = new IntPtr(win);
                    if (!IsWindow(h)) throw new InvalidOperationException("That window has closed.");
                    if (IsIconic(h)) ShowWindow(h, 9 /* SW_RESTORE */);
                    ForceForeground(h);
                    break;
                }
                case "apps.close":
                {
                    int pid = (int)Num(args, "pid");
                    if (pid == 0 || pid == Environment.ProcessId) throw new InvalidOperationException("Ember can't close itself here.");
                    if (IsGame(pid)) { value = new { ok = await _bridge.Games.CloseRunningAsync() }; break; }
                    foreach (var w in TopWindows().Where(w => w.Pid == pid)) PostMessage(w.Handle, 0x0010 /* WM_CLOSE */, IntPtr.Zero, IntPtr.Zero);
                    value = new { ok = true };
                    break;
                }
            }
        }
        catch (Exception ex) { error = ex.Message; }
        Post(JsonSerializer.Serialize(error is null
            ? new { type = "result", id, ok = true, value, error = (string?)null }
            : new { type = "result", id, ok = false, value = (object?)null, error = (string?)error },
            new JsonSerializerOptions(JsonSerializerDefaults.Web)));
    }

    private static long Num(JsonElement args, string name) =>
        args.ValueKind == JsonValueKind.Object && args.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Number && v.TryGetInt64(out var n) ? n : 0;

    private List<object> ListApps()
    {
        var list = new List<object>();
        bool anyFocused = false;
        foreach (var w in TopWindows())
        {
            if (w.Pid == Environment.ProcessId) continue;
            bool focused = w.Handle == _prevForeground;
            anyFocused |= focused;
            list.Add(new { id = w.Handle.ToInt64(), pid = w.Pid, title = w.Title, app = ProcessName(w.Pid), hearth = false, game = IsGame(w.Pid), focused });
        }
        list.Insert(0, new { id = 0L, pid = Environment.ProcessId, title = "Ember", app = "ember", hearth = true, game = false, focused = !anyFocused });
        return list;
    }

    private static string ProcessName(int pid)
    {
        try { using var p = Process.GetProcessById(pid); return p.ProcessName; } catch { return ""; }
    }

    /// <summary>Is this process the running game's (its program is in the game's folder)?</summary>
    private bool IsGame(int pid)
    {
        var dirs = _bridge.Games.RunningDirs;
        if (dirs.Length == 0) return false;
        try
        {
            using var p = Process.GetProcessById(pid);
            var exe = p.MainModule?.FileName;
            return exe is not null && dirs.Any(d => !string.IsNullOrEmpty(d) && exe.StartsWith(d.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase));
        }
        catch { return false; }   // another user's or an elevated process: not ours to read
    }

    private readonly record struct TopWindow(IntPtr Handle, int Pid, string Title);

    /// <summary>Windows the taskbar shows: visible, unowned, not tool windows or cloaked (other desktops, UWP ghosts).</summary>
    private static List<TopWindow> TopWindows()
    {
        var found = new List<TopWindow>();
        EnumWindows((h, _) =>
        {
            if (!IsWindowVisible(h) || GetWindow(h, 4 /* GW_OWNER */) != IntPtr.Zero) return true;
            if ((GetWindowLongPtr(h, -20 /* GWL_EXSTYLE */).ToInt64() & 0x80 /* WS_EX_TOOLWINDOW */) != 0) return true;
            if (DwmGetWindowAttribute(h, 14 /* DWMWA_CLOAKED */, out int cloaked, sizeof(int)) == 0 && cloaked != 0) return true;
            int len = GetWindowTextLength(h);
            if (len <= 0) return true;
            var sb = new StringBuilder(len + 1);
            GetWindowText(h, sb, sb.Capacity);
            GetWindowThreadProcessId(h, out uint pid);
            found.Add(new TopWindow(h, (int)pid, sb.ToString()));
            return true;
        }, IntPtr.Zero);
        return found;
    }

    // ------------------------------------------------------------------ screenshot

    private async Task ScreenshotAsync(int callId)
    {
        object? value = null; string? error = null;
        try
        {
            var screen = _prevForeground != IntPtr.Zero ? Screen.FromHandle(_prevForeground) : Screen.FromControl(this);
            _busy = true;
            Hide();
            await Task.Delay(250);   // let the game's frame show without the overlay
            var b = screen.Bounds;
            using var bmp = new Bitmap(b.Width, b.Height, PixelFormat.Format32bppRgb);
            using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(b.Location, Point.Empty, b.Size);

            var title = _bridge.Games.RunningTitle ?? "Screenshot";
            foreach (var c in Path.GetInvalidFileNameChars()) title = title.Replace(c, ' ');
            var dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyPictures), "Screenshots", "Ember");
            Directory.CreateDirectory(dir);
            var file = Path.Combine(dir, $"{title.Trim()} {DateTime.Now:yyyy-MM-dd HH-mm-ss}.png");
            bmp.Save(file, ImageFormat.Png);
            SystemBridge.Log("screenshot " + file);
            value = new { path = file, folder = dir };
        }
        catch (Exception ex) { error = "Couldn't take a screenshot: " + ex.Message; }
        finally
        {
            Show();
            ForceForeground(Handle);
            _shownAt = DateTime.UtcNow;
            _busy = false;
        }
        Post(JsonSerializer.Serialize(error is null
            ? new { type = "result", id = callId, ok = true, value, error = (string?)null }
            : new { type = "result", id = callId, ok = false, value = (object?)null, error = (string?)error },
            new JsonSerializerOptions(JsonSerializerDefaults.Web)));
    }

    // ------------------------------------------------------------------ Win32

    /// <summary>Windows only lets the foreground app hand out focus. Borrow the foreground thread's input state to take it.</summary>
    private static void ForceForeground(IntPtr hwnd)
    {
        var fg = GetForegroundWindow();
        if (fg == hwnd) return;
        uint fgThread = GetWindowThreadProcessId(fg, out _), me = GetCurrentThreadId();
        bool attached = fgThread != 0 && fgThread != me && AttachThreadInput(me, fgThread, true);
        try
        {
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
        finally { if (attached) AttachThreadInput(me, fgThread, false); }
        if (GetForegroundWindow() != hwnd)
        {
            // Last resort: a synthetic Alt press makes Windows treat us as "recently used input".
            keybd_event(0x12, 0, 0, UIntPtr.Zero);
            keybd_event(0x12, 0, 0x0002, UIntPtr.Zero);
            SetForegroundWindow(hwnd);
        }
    }

    private static uint DpiOf(Rectangle screenBounds)
    {
        try
        {
            var mon = MonitorFromPoint(new POINT { X = screenBounds.Left + 1, Y = screenBounds.Top + 1 }, 2 /* nearest */);
            if (GetDpiForMonitor(mon, 0, out uint x, out _) == 0) return x;
        }
        catch { }
        return 96;
    }

    [StructLayout(LayoutKind.Sequential)] private struct POINT { public int X, Y; }
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern bool IsWindow(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] private static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool attach);
    [DllImport("kernel32.dll")] private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] private static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] private static extern IntPtr MonitorFromPoint(POINT pt, uint flags);
    [DllImport("shcore.dll")] private static extern int GetDpiForMonitor(IntPtr hmonitor, int dpiType, out uint dpiX, out uint dpiY);
    private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr hWnd, int cmd);
    [DllImport("user32.dll")] private static extern IntPtr GetWindow(IntPtr hWnd, uint cmd);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")] private static extern IntPtr GetWindowLongPtr(IntPtr hWnd, int index);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll")] private static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("dwmapi.dll")] private static extern int DwmGetWindowAttribute(IntPtr hWnd, int attr, out int value, int size);
}

/// <summary>Windows' "Open Game Bar using this button on a controller" setting (per user).</summary>
public static class GameBarButton
{
    private const string Key = @"Software\Microsoft\GameBar";
    private const string Value = "UseNexusForGameBarEnabled";

    /// <summary>True when Game Bar also opens on the Xbox button (the Windows default).</summary>
    public static bool Get()
    {
        using var k = Registry.CurrentUser.OpenSubKey(Key);
        return k?.GetValue(Value) is not int v || v != 0;
    }

    public static void Set(bool on)
    {
        using var k = Registry.CurrentUser.CreateSubKey(Key);
        k.SetValue(Value, on ? 1 : 0, RegistryValueKind.DWord);
        SystemBridge.Log($"Game Bar Xbox button: {(on ? "on" : "off")}");
    }
}
