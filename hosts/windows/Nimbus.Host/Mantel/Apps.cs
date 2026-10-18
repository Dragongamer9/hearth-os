using System.Diagnostics;
using System.Text;
using System.Text.Json;

namespace Nimbus.Host.Mantel;

/// <summary>Open windows for Mantel's taskbar, and the installed apps for its launcher.</summary>
internal static class Apps
{
    public sealed record Win(long Id, string Title, string? App, string? Icon, bool Active, bool Minimized);
    public sealed record StartApp(string Id, string Name, string? Icon);

    private static readonly Dictionary<string, string?> IconByExe = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Windows that would show on Windows' own taskbar (visible, not owned, not tool windows, not
    /// hidden by Windows itself), except Ember's.</summary>
    public static List<Win> Windows()
    {
        var list = new List<Win>();
        var fg = Native.GetForegroundWindow();
        int self = Environment.ProcessId;
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
            if (pid == self) return true;
            var cls = new StringBuilder(256);
            Native.GetClassName(h, cls, cls.Capacity);
            if (cls.ToString() is "Progman" or "WorkerW" or "Shell_TrayWnd" or "Shell_SecondaryTrayWnd") return true;
            var title = new StringBuilder(len + 1);
            Native.GetWindowText(h, title, title.Capacity);
            var exe = Native.ProcessPath(pid);
            list.Add(new Win(h.ToInt64(), title.ToString(), exe is null ? null : Path.GetFileNameWithoutExtension(exe), IconFor(exe),
                h == fg, Native.IsIconic(h)));
            return true;
        }, IntPtr.Zero);
        return list;
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

    public static void Focus(long id)
    {
        var h = new IntPtr(id);
        if (Native.GetForegroundWindow() == h && !Native.IsIconic(h)) { Native.ShowWindow(h, Native.SW_MINIMIZE); return; }
        if (Native.IsIconic(h)) Native.ShowWindow(h, Native.SW_RESTORE);
        // Windows only lets the app in front hand over focus; a tap of Alt counts as input and unlocks it.
        Native.keybd_event(0x12, 0, 0, UIntPtr.Zero);
        Native.keybd_event(0x12, 0, 2, UIntPtr.Zero);
        Native.SetForegroundWindow(h);
    }

    public static void Close(long id) => Native.PostMessage(new IntPtr(id), Native.WM_CLOSE, IntPtr.Zero, IntPtr.Zero);

    // ------------------------------------------------------------------ installed apps

    private static List<StartApp>? _apps;
    private static DateTime _appsAt;

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
        _apps = withIcons.ToList();
        _appsAt = DateTime.UtcNow;
        return _apps;
    }

    /// <summary>Uninstallers, readmes and help links clutter the list, like the old Start menu folders.</summary>
    private static bool Skip(string name, string id) =>
        name.StartsWith("Uninstall", StringComparison.OrdinalIgnoreCase) || name.Contains("Readme", StringComparison.OrdinalIgnoreCase) ||
        id.EndsWith(".txt", StringComparison.OrdinalIgnoreCase) || id.EndsWith(".url", StringComparison.OrdinalIgnoreCase) ||
        id.EndsWith(".html", StringComparison.OrdinalIgnoreCase) || id.EndsWith(".chm", StringComparison.OrdinalIgnoreCase);

    public static void Launch(string id)
    {
        if (string.IsNullOrWhiteSpace(id) || id.Contains('"')) return;
        Process.Start(new ProcessStartInfo("explorer.exe", "\"shell:AppsFolder\\" + id + "\"") { UseShellExecute = true });
    }
}
