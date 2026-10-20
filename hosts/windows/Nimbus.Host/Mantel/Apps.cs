using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace Nimbus.Host.Mantel;

/// <summary>Open windows for Mantel's taskbar, and the installed apps for its launcher.</summary>
internal static class Apps
{
    /// <summary>An open window. <c>Key</c> groups the windows of one app on one taskbar button (like Bar.qml's
    /// taskItems); <c>PinId</c> is what pinning it saves and what opening it again starts (a Start menu app id, or
    /// the program's path); <c>Name</c> is the app's name (the Start menu's, else the program's description).</summary>
    public sealed record Win(long Id, string Title, string? App, string? Icon, bool Active, bool Minimized,
        string Key, string PinId, string Name, bool Maximized, bool Own);
    public sealed record StartApp(string Id, string Name, string? Icon);

    private static readonly Dictionary<string, string?> IconByExe = new(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<IntPtr, (uint Pid, string? Aumid, string? Exe, DateTime At)> Ident = new();

    /// <summary>Windows that would show on Windows' own taskbar (visible, not owned, not tool windows, not
    /// hidden by Windows itself or on another virtual desktop), except Ember's.</summary>
    public static List<Win> Windows()
    {
        var list = new List<Win>();
        var fg = Native.GetForegroundWindow();
        int self = Environment.ProcessId;
        var seen = new HashSet<IntPtr>();
        Native.EnumWindows((h, _) =>
        {
            if (!Native.IsWindowVisible(h) || Native.GetWindow(h, Native.GW_OWNER) != IntPtr.Zero) return true;
            int ex = Native.GetWindowLong(h, Native.GWL_EXSTYLE);
            if ((ex & Native.WS_EX_TOOLWINDOW) != 0 && (ex & Native.WS_EX_APPWINDOW) == 0) return true;
            if ((ex & Native.WS_EX_NOACTIVATE) != 0 && (ex & Native.WS_EX_APPWINDOW) == 0) return true;
            if (Native.DwmGetWindowAttribute(h, Native.DWMWA_CLOAKED, out int cloaked, 4) == 0 && cloaked != 0) return true;
            int len = Native.GetWindowTextLength(h);
            if (len == 0) return true;
            Native.GetWindowThreadProcessId(h, out var pid);
            if (pid == self)
            {
                // Ember's own windows stay off the bar, except Hearth's apps (Settings, Task Manager, Files, Apps: apps
                // you switch to, like on Hearth OS). They share a button with their pin (hearth:…).
                var own = new StringBuilder(len + 1);
                Native.GetWindowText(h, own, own.Capacity);
                var ownTitle = own.ToString();
                var (ownId, ownName, ownIcon) = ownTitle switch
                {
                    "Hearth Settings" => ("hearth:settings", "Hearth Settings", "hearth-logo.svg"),
                    TasksWindow.Title => ("hearth:tasks", "Task Manager", "hearth-logo.svg"),
                    FilesWindow.Title => ("hearth:files", "Files", "hearth-logo.svg"),
                    AppsWindow.Title => ("hearth:apps", "Apps", "hearth-logo.svg"),
                    _ => ("", "", ""),
                };
                if (ownId.Length > 0)
                {
                    seen.Add(h);
                    list.Add(new Win(h.ToInt64(), ownTitle, ownId, ownIcon, h == fg, Native.IsIconic(h), ownId, ownId, ownName, IsZoomed(h), true));
                }
                return true;
            }
            var cls = new StringBuilder(256);
            Native.GetClassName(h, cls, cls.Capacity);
            if (cls.ToString() is "Progman" or "WorkerW" or "Shell_TrayWnd" or "Shell_SecondaryTrayWnd") return true;
            var title = new StringBuilder(len + 1);
            Native.GetWindowText(h, title, title.Capacity);
            seen.Add(h);
            var (aumid, exe) = Identify(h, pid);
            var app = Match(aumid, exe);
            string key, pinId, name;
            if (app is not null) { key = pinId = app.Id; name = app.Name; }
            else if (aumid is not null && !IsFrameHost(exe)) { key = pinId = aumid; name = Describe(exe) ?? title.ToString(); }
            else if (exe is not null && !IsFrameHost(exe)) { key = pinId = exe; name = Describe(exe) ?? Path.GetFileNameWithoutExtension(exe); }
            else { key = "win:" + h.ToInt64(); pinId = ""; name = title.ToString(); }
            list.Add(new Win(h.ToInt64(), title.ToString(), exe is null ? null : Path.GetFileNameWithoutExtension(exe),
                app?.Icon ?? IconFor(exe), h == fg, Native.IsIconic(h), key, pinId, name, IsZoomed(h), false));
            return true;
        }, IntPtr.Zero);
        lock (Ident) foreach (var k in Ident.Keys.Where(x => !seen.Contains(x)).ToList()) Ident.Remove(k);
        return list;
    }

    private static bool IsFrameHost(string? exe) => exe is not null && Path.GetFileName(exe).Equals("ApplicationFrameHost.exe", StringComparison.OrdinalIgnoreCase);

    /// <summary>The window's app id (AppUserModelID: set on the window, or the packaged app's) and its program.
    /// Kept for half a minute per window (it's asked every second).</summary>
    private static (string? Aumid, string? Exe) Identify(IntPtr h, uint pid)
    {
        lock (Ident)
            if (Ident.TryGetValue(h, out var c) && c.Pid == pid && DateTime.UtcNow - c.At < TimeSpan.FromSeconds(30)) return (c.Aumid, c.Exe);
        var exe = Native.ProcessPath(pid);
        var aumid = WindowAppId(h) ?? PackagedAppId(pid);
        lock (Ident) Ident[h] = (pid, aumid, exe, DateTime.UtcNow);
        return (aumid, exe);
    }

    private static readonly Dictionary<string, string?> DescByExe = new(StringComparer.OrdinalIgnoreCase);
    /// <summary>A program's own name for itself (its file description), remembered.</summary>
    private static string? Describe(string? exe)
    {
        if (exe is null) return null;
        lock (DescByExe)
        {
            if (DescByExe.TryGetValue(exe, out var known)) return known;
            string? d = null;
            try { d = FileVersionInfo.GetVersionInfo(exe).FileDescription; } catch { }
            return DescByExe[exe] = string.IsNullOrWhiteSpace(d) ? null : d.Trim();
        }
    }

    private static string? IconFor(string? exe)
    {
        if (exe is null) return null;
        lock (IconByExe)
        {
            if (IconByExe.TryGetValue(exe, out var cached)) return cached;
            var icon = Native.ShellIcon(exe, 48);
            if (icon is null)
            {
                try { using var ico = Icon.ExtractAssociatedIcon(exe); icon = ico is null ? null : Native.ToDataUrl(ico.ToBitmap()); } catch { }
            }
            return IconByExe[exe] = icon;
        }
    }

    /// <summary>The icon for a pin that isn't running: its Start menu entry's, or the program's.</summary>
    public static string? PinIcon(string id)
    {
        if (id.StartsWith("hearth:")) return "hearth-logo.svg";
        var apps = _apps;
        var a = apps?.FirstOrDefault(x => string.Equals(x.Id, id, StringComparison.OrdinalIgnoreCase));
        if (a is not null) return a.Icon;
        if (Path.IsPathRooted(id) && File.Exists(id)) return IconFor(id);
        return null;
    }

    public static void Focus(long id)
    {
        var h = new IntPtr(id);
        if (Native.GetForegroundWindow() == h && !Native.IsIconic(h)) { Native.ShowWindow(h, Native.SW_MINIMIZE); return; }
        Activate(h);
    }

    /// <summary>Brings a window to the front (restoring it if it's minimized), without minimizing it when it's already there.</summary>
    public static void Activate(IntPtr h)
    {
        if (Native.IsIconic(h)) Native.ShowWindow(h, Native.SW_RESTORE);
        // Windows only lets the app in front hand over focus; a tap of Alt counts as input and unlocks it.
        Native.keybd_event(0x12, 0, 0, UIntPtr.Zero);
        Native.keybd_event(0x12, 0, 2, UIntPtr.Zero);
        Native.SetForegroundWindow(h);
    }

    public static void Close(long id) => Native.PostMessage(new IntPtr(id), Native.WM_CLOSE, IntPtr.Zero, IntPtr.Zero);

    /// <summary>The taskbar menu's Maximize / Restore size.</summary>
    public static void Maximize(long id, bool on)
    {
        var h = new IntPtr(id);
        if (on) { Activate(h); Native.ShowWindow(h, 3 /* SW_MAXIMIZE */); }
        else Native.ShowWindow(h, Native.SW_RESTORE);
    }

    [DllImport("user32.dll")] private static extern bool IsZoomed(IntPtr h);

    // ------------------------------------------------------------------ app ids

    [DllImport("shell32.dll")] private static extern int SHGetPropertyStoreForWindow(IntPtr hwnd, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out IPropertyStore store);
    [DllImport("ole32.dll")] private static extern int PropVariantClear(ref PROPVARIANT pv);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern int GetApplicationUserModelId(IntPtr process, ref int length, StringBuilder? id);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern int SHGetKnownFolderPath([MarshalAs(UnmanagedType.LPStruct)] Guid id, uint flags, IntPtr token, out IntPtr path);

    [StructLayout(LayoutKind.Sequential, Pack = 4)] private struct PROPERTYKEY { public Guid fmtid; public uint pid; }
    [StructLayout(LayoutKind.Sequential)] private struct PROPVARIANT { public ushort vt; public ushort r1, r2, r3; public IntPtr p; public IntPtr p2; }

    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore
    {
        [PreserveSig] int GetCount(out uint count);
        [PreserveSig] int GetAt(uint index, out PROPERTYKEY key);
        [PreserveSig] int GetValue(ref PROPERTYKEY key, out PROPVARIANT value);
        [PreserveSig] int SetValue(ref PROPERTYKEY key, ref PROPVARIANT value);
        [PreserveSig] int Commit();
    }

    /// <summary>System.AppUserModel.ID set on the window itself (browsers, Store apps' frames, Explorer).</summary>
    private static string? WindowAppId(IntPtr h)
    {
        try
        {
            var iid = typeof(IPropertyStore).GUID;
            if (SHGetPropertyStoreForWindow(h, ref iid, out var store) != 0 || store is null) return null;
            try
            {
                var key = new PROPERTYKEY { fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), pid = 5 };
                if (store.GetValue(ref key, out var pv) != 0) return null;
                try { return pv.vt == 31 /* VT_LPWSTR */ && pv.p != IntPtr.Zero ? NullIfEmpty(Marshal.PtrToStringUni(pv.p)) : null; }
                finally { PropVariantClear(ref pv); }
            }
            finally { Marshal.ReleaseComObject(store); }
        }
        catch { return null; }
    }

    /// <summary>A packaged (Store / MSIX) process's app id.</summary>
    private static string? PackagedAppId(uint pid)
    {
        var p = Native.OpenProcess(0x1000 /* PROCESS_QUERY_LIMITED_INFORMATION */, false, pid);
        if (p == IntPtr.Zero) return null;
        try
        {
            int len = 0;
            if (GetApplicationUserModelId(p, ref len, null) != 122 /* ERROR_INSUFFICIENT_BUFFER */ || len <= 0) return null;
            var sb = new StringBuilder(len);
            return GetApplicationUserModelId(p, ref len, sb) == 0 ? NullIfEmpty(sb.ToString()) : null;
        }
        catch { return null; }
        finally { Native.CloseHandle(p); }
    }

    private static string? NullIfEmpty(string? s) => string.IsNullOrWhiteSpace(s) ? null : s;

    /// <summary>The Start menu app a window belongs to: the same app id, or a Start entry pointing at its program.</summary>
    private static StartApp? Match(string? aumid, string? exe)
    {
        var apps = _apps;
        if (apps is null) return null;
        if (aumid is not null)
        {
            var a = apps.FirstOrDefault(x => string.Equals(x.Id, aumid, StringComparison.OrdinalIgnoreCase));
            if (a is not null) return a;
        }
        if (exe is not null && !IsFrameHost(exe))
        {
            var byPath = _byPath;
            if (byPath is not null && byPath.TryGetValue(exe, out var b)) return b;
        }
        return null;
    }

    /// <summary>"{known folder GUID}\Steam\steam.exe" (how Windows names desktop apps without their own id) or a
    /// plain path → the full path; anything else → null.</summary>
    private static string? IdPath(string id)
    {
        try
        {
            if (id.Length > 38 && id[0] == '{' && id[37] == '}' && id[38] == '\\' && Guid.TryParse(id[..38], out var g))
            {
                if (SHGetKnownFolderPath(g, 0, IntPtr.Zero, out var ptr) != 0) return null;
                try { return Path.Combine(Marshal.PtrToStringUni(ptr) ?? "", id[39..]); }
                finally { Marshal.FreeCoTaskMem(ptr); }
            }
            if (Path.IsPathRooted(id)) return id;
        }
        catch { }
        return null;
    }

    // ------------------------------------------------------------------ installed apps

    private static List<StartApp>? _apps;
    private static Dictionary<string, StartApp>? _byPath;
    private static DateTime _appsAt;

    /// <summary>The Start menu list if it's been read already (never waits).</summary>
    public static List<StartApp>? StartAppsCached => _apps;

    /// <summary>Everything in the Start menu (desktop and Store apps), from Windows' own list (Get-StartApps).
    /// Read once and kept for ten minutes; icons come from the shell, like the Start menu's.</summary>
    public static async Task<List<StartApp>> StartAppsAsync(bool refresh = false)
    {
        if (!refresh && _apps is not null && DateTime.UtcNow - _appsAt < TimeSpan.FromMinutes(10)) return _apps;
        var psi = new ProcessStartInfo("powershell.exe", "-NoProfile -NonInteractive -Command \"[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-StartApps | ConvertTo-Json -Compress\"")
        { UseShellExecute = false, RedirectStandardOutput = true, CreateNoWindow = true, StandardOutputEncoding = Encoding.UTF8 };
        var list = new List<StartApp>();
        try
        {
            using var p = Process.Start(psi)!;
            var json = await p.StandardOutput.ReadToEndAsync();
            await p.WaitForExitAsync();
            using var doc = JsonDocument.Parse(string.IsNullOrWhiteSpace(json) ? "[]" : json);
            var items = doc.RootElement.ValueKind == JsonValueKind.Array ? doc.RootElement.EnumerateArray().ToList() : new List<JsonElement> { doc.RootElement };
            var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (var it in items)
            {
                var name = it.TryGetProperty("Name", out var n) ? n.GetString() : null;
                var id = it.TryGetProperty("AppID", out var a) ? a.GetString() : null;
                if (string.IsNullOrWhiteSpace(name) || string.IsNullOrWhiteSpace(id) || !seen.Add(name)) continue;
                if (Skip(name, id)) continue;
                list.Add(new StartApp(id, name, null));
            }
        }
        catch (Exception ex) { SystemBridge.Log("mantel: start apps: " + ex.Message); }
        list.Sort((x, y) => string.Compare(x.Name, y.Name, StringComparison.CurrentCultureIgnoreCase));
        // Icons in parallel (the shell is slow for some Store apps).
        var withIcons = await Task.WhenAll(list.Select(x => Task.Run(() => x with { Icon = Native.ShellIcon(@"shell:AppsFolder\" + x.Id, 64) })));
        var byPath = new Dictionary<string, StartApp>(StringComparer.OrdinalIgnoreCase);
        foreach (var x in withIcons)
        {
            var path = IdPath(x.Id);
            if (path is not null && path.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)) byPath.TryAdd(path, x);
        }
        _byPath = byPath;
        _apps = withIcons.ToList();
        _appsAt = DateTime.UtcNow;
        lock (Ident) Ident.Clear();   // windows matched before the list was there get matched again
        return _apps;
    }

    /// <summary>Uninstallers, readmes and help links clutter the list, like the old Start menu folders.</summary>
    private static bool Skip(string name, string id) =>
        name.StartsWith("Uninstall", StringComparison.OrdinalIgnoreCase) || name.Contains("Readme", StringComparison.OrdinalIgnoreCase) ||
        id.EndsWith(".txt", StringComparison.OrdinalIgnoreCase) || id.EndsWith(".url", StringComparison.OrdinalIgnoreCase) ||
        id.EndsWith(".html", StringComparison.OrdinalIgnoreCase) || id.EndsWith(".chm", StringComparison.OrdinalIgnoreCase);

    /// <summary>Starts an app: a Start menu app id, or a program's path (a pin made from a running window).</summary>
    public static void Launch(string id)
    {
        if (string.IsNullOrWhiteSpace(id) || id.Contains('"')) return;
        try
        {
            if (Path.IsPathRooted(id))
            {
                if (!File.Exists(id)) throw new FileNotFoundException("It isn't there any more");
                Process.Start(new ProcessStartInfo(id) { UseShellExecute = true, WorkingDirectory = Path.GetDirectoryName(id) ?? "" })?.Dispose();
                return;
            }
            Process.Start(new ProcessStartInfo("explorer.exe", "\"shell:AppsFolder\\" + id + "\"") { UseShellExecute = true })?.Dispose();
        }
        catch (Exception ex) { SystemBridge.Log("mantel: launch " + id + ": " + ex.Message); }
    }

    /// <summary>Opens a URI with its app (e.g. ms-settings:nightlight).</summary>
    public static void OpenUri(string uri)
    {
        try { Process.Start(new ProcessStartInfo(uri) { UseShellExecute = true }); }
        catch (Exception ex) { SystemBridge.Log("mantel: couldn't open " + uri + ": " + ex.Message); }
    }
}
