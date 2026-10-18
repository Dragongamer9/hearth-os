using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Mantel on Windows: Hearth's desktop look in Ember's Desktop Mode. Its own taskbar (start button, pinned
/// and open apps, Wi-Fi, volume, battery, clock, Quick Access, Game Mode), an app launcher with search, and
/// Hearth wallpapers.
///
/// Windows' own taskbar is hidden underneath (set to auto-hide so it doesn't keep its space, and kept out of
/// sight), not removed: Explorer still runs, so tray icons and app notifications keep working, and the tray
/// button on Mantel's taskbar shows Windows' taskbar for a moment to reach them. Everything is put back the
/// way it was when you leave Mantel.
/// </summary>
internal sealed class MantelHost : IDisposable
{
    private readonly SystemBridge _bridge;
    private readonly CoreWebView2Environment _env;
    private readonly string _baseUrl;
    private readonly Action<CoreWebView2> _setup;
    private MantelWindow? _bar, _launcher;
    private readonly System.Windows.Forms.Timer _tick = new() { Interval = 1000 };
    private string _lastWindows = "";
    private DateTime _peekUntil = DateTime.MinValue;
    private int? _savedTaskbarState;
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);
    private static readonly string StateFile = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus", "mantel.json");

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
        _bridge.AddListener(json => { if (Active) { _bar?.Post(json); _launcher?.Post(json); } });
        _tick.Tick += (_, _) => Tick();
    }

    // ------------------------------------------------------------------ start / stop

    public async Task StartAsync()
    {
        if (Active) return;
        Active = true;
        SystemBridge.Log("mantel: starting");
        try
        {
            HideWindowsTaskbar();
            ApplyWallpaper();
            _bar ??= await Create(bar: true);
            _launcher ??= await Create(bar: false);
            _bar.Show();
            _bar.PlaceAppBar();
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
        _launcher?.Hide();
        if (_bar is not null) { _bar.RemoveAppBar(); _bar.Hide(); }
        RestoreWindowsTaskbar();
    }

    private async Task<MantelWindow> Create(bool bar)
    {
        var w = new MantelWindow(bar);
        w.PageMessage += m => OnMessage(m, w);
        await w.InitAsync(_env, _baseUrl + (bar ? "mantel.html?part=bar" : "mantel.html?part=launcher"), _setup);
        return w;
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
            var saved = Load();
            _savedTaskbarState = saved.TaskbarState ?? (int)Native.SHAppBarMessage(Native.ABM_GETSTATE, ref d).ToUInt32();
            saved.TaskbarState = _savedTaskbarState;
            Save(saved);
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
        var s = Load(); s.TaskbarState = null; Save(s);
        foreach (var h in TaskbarWindows()) Native.ShowWindow(h, Native.SW_SHOWNA);
    }

    /// <summary>At Ember's start: if Mantel was running when Ember last stopped (a crash, a power cut),
    /// give Windows' taskbar its setting back.</summary>
    public void RecoverTaskbar()
    {
        var s = Load();
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

    // ------------------------------------------------------------------ messages from the pages

    private void OnMessage(JsonElement m, MantelWindow from)
    {
        var type = m.TryGetProperty("type", out var t) ? t.GetString() : null;
        if (type == "call") { _bridge.Handle(m, from.Post); return; }
        if (type == "log") { SystemBridge.Log("mantel page: " + m.GetProperty("text").GetString()); return; }
        if (type == "ready") { PushState(from); PushWindows(force: true); return; }
        if (type != "mantel") return;
        string action = m.GetProperty("action").GetString() ?? "";
        string Str(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() ?? "" : "";
        long Long(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt64() : 0;
        switch (action)
        {
            case "apps": _ = PushAppsAsync(from, m.TryGetProperty("refresh", out var r) && r.ValueKind == JsonValueKind.True); break;
            case "launch": Apps.Launch(Str("id")); _launcher?.Hide(); break;
            case "focus": Apps.Focus(Long("id")); _launcher?.Hide(); break;
            case "close": Apps.Close(Long("id")); break;
            case "launcher":
                if (_launcher is null) break;
                if (_launcher.Visible) _launcher.Hide();
                else { _launcher.ShowLauncher(_bar?.Height ?? 54); Send(_launcher, "mantel.opened", new { }); }
                break;
            case "hideLauncher": _launcher?.Hide(); break;
            case "tray": PeekTray(); break;
            case "quick": QuickAccessRequested?.Invoke(); break;
            case "game": _launcher?.Hide(); GameModeRequested?.Invoke(); break;
            case "power": _launcher?.Hide(); PowerRequested?.Invoke(Str("what")); break;
            case "pin": SetPin(Str("id"), Str("name"), m.TryGetProperty("on", out var on) && on.ValueKind == JsonValueKind.True); break;
            case "settings": _launcher?.Hide(); Apps.Launch("windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel"); break;
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
    }

    // ------------------------------------------------------------------ pins and wallpaper (mantel.json)

    private sealed class SavedState
    {
        public List<Pin> Pins { get; set; } = new();
        public string? WindowsWallpaper { get; set; }
        /// <summary>Windows' taskbar setting before Mantel set it to auto-hide (kept on disk so a crash can't
        /// leave it changed: the next start puts it back).</summary>
        public int? TaskbarState { get; set; }
    }
    public sealed record Pin(string Id, string Name);

    private static SavedState Load()
    {
        try { return JsonSerializer.Deserialize<SavedState>(File.ReadAllText(StateFile), Json) ?? new SavedState(); }
        catch { return new SavedState(); }
    }
    private static void Save(SavedState s)
    {
        try { Directory.CreateDirectory(Path.GetDirectoryName(StateFile)!); File.WriteAllText(StateFile, JsonSerializer.Serialize(s, Json)); }
        catch (Exception ex) { SystemBridge.Log("mantel: save: " + ex.Message); }
    }

    private void PushState(MantelWindow to) => Send(to, "mantel.state", new { pins = Load().Pins });

    private void SetPin(string id, string name, bool on)
    {
        if (string.IsNullOrEmpty(id)) return;
        var s = Load();
        s.Pins.RemoveAll(p => p.Id == id);
        if (on) s.Pins.Add(new Pin(id, name));
        Save(s);
        foreach (var w in new[] { _bar, _launcher }) if (w is not null) PushState(w);
    }

    /// <summary>The wallpaper chosen in Settings › Shell: one of Hearth's (in wwwroot/wallpapers), or Windows'
    /// own ("" puts back the one you had before Mantel).</summary>
    public static string Wallpaper { get; set; } = "hearth-cabin";

    public void ApplyWallpaper()
    {
        var s = Load();
        try
        {
            if (s.WindowsWallpaper is null)
            {
                var sb = new System.Text.StringBuilder(520);
                if (Native.SystemParametersInfo(Native.SPI_GETDESKWALLPAPER, (uint)sb.Capacity, sb, 0)) { s.WindowsWallpaper = sb.ToString(); Save(s); }
            }
            string? file = string.IsNullOrEmpty(Wallpaper) ? s.WindowsWallpaper
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
        _bar?.Dispose();
        _launcher?.Dispose();
    }
}
