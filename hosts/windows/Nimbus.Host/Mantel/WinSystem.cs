using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.Management;
using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace Nimbus.Host.Mantel;

/// <summary>
/// What Hearth Settings' newer pages need from Windows (SetDateTime / SetKeyboard / SetDefaults / SetUsers /
/// SetPrinters on Hearth OS): the clock and time zone, keyboard layouts, default apps, accounts and printers.
/// Where Windows wants administrator rights, Ember asks through the same helper "Switch to Hearth OS" uses
/// (a copy of Ember started with Windows' UAC prompt, <see cref="Elevated"/>); where Windows doesn't let apps
/// change something at all (default apps, adding an account), the page opens Windows' own page for it.
/// </summary>
internal static class WinSystem
{
    // ================================================================== Date & time

    public static object TimeGet()
    {
        TimeZoneInfo.ClearCachedData();
        var tz = TimeZoneInfo.Local;
        var now = DateTimeOffset.Now;
        return new
        {
            now = now.ToString("yyyy-MM-dd'T'HH:mm:sszzz", CultureInfo.InvariantCulture),
            timezone = tz.Id,
            zoneName = City(tz),
            region = Region(tz),
            offset = Offset(tz.GetUtcOffset(DateTime.UtcNow)),
            ntp = NtpOn(),
            available = true,
            admin = true,
            canNtp = W32TimeInstalled(),
        };
    }

    /// <summary>Every time zone Windows knows: [{zone, region, city, offset}], grouped by region like Hearth OS's.</summary>
    public static object TimeZones()
    {
        var list = TimeZoneInfo.GetSystemTimeZones()
            .Select(z => new { zone = z.Id, region = Region(z), city = City(z), offset = Offset(z.GetUtcOffset(DateTime.UtcNow)) })
            .OrderBy(z => z.region, StringComparer.OrdinalIgnoreCase).ThenBy(z => z.city, StringComparer.CurrentCultureIgnoreCase)
            .ToList();
        return new { zones = list };
    }

    /// <summary>"(UTC-08:00) Pacific Time (US &amp; Canada)" → "Pacific Time (US &amp; Canada)".</summary>
    private static string City(TimeZoneInfo z) => Regex.Replace(z.DisplayName, @"^\(UTC[^)]*\)\s*", "");

    /// <summary>The time zone's part of the world, from its IANA name (America/Los_Angeles → America).</summary>
    private static string Region(TimeZoneInfo z)
    {
        try
        {
            if (TimeZoneInfo.TryConvertWindowsIdToIanaId(z.Id, out var iana) && iana.Contains('/'))
            {
                var r = iana.Split('/')[0];
                return r == "Etc" ? "Other" : r;
            }
        }
        catch { }
        return "Other";
    }

    private static string Offset(TimeSpan t) => (t < TimeSpan.Zero ? "-" : "+") + t.Duration().ToString(@"hh\:mm", CultureInfo.InvariantCulture);

    private const string W32Time = @"SYSTEM\CurrentControlSet\Services\W32Time";

    private static bool W32TimeInstalled()
    {
        try { using var k = Registry.LocalMachine.OpenSubKey(W32Time); return k is not null; } catch { return false; }
    }

    /// <summary>Windows' "Set time automatically" (W32Time's Type: NTP / AllSync on, NoSync off).</summary>
    private static bool NtpOn()
    {
        try { using var k = Registry.LocalMachine.OpenSubKey(W32Time + @"\Parameters"); return !string.Equals(k?.GetValue("Type") as string, "NoSync", StringComparison.OrdinalIgnoreCase); }
        catch { return true; }
    }

    /// <summary>The time zone, with Windows' own tzutil (people may change their time zone without administrator
    /// rights on Windows). Only names Windows knows are passed on.</summary>
    public static async Task TimeSetZone(string id)
    {
        if (!TimeZoneInfo.GetSystemTimeZones().Any(z => z.Id == id)) throw new ArgumentException("Windows doesn't know that time zone");
        var (code, output) = await Run("tzutil.exe", "/s \"" + id + "\"");
        if (code != 0) throw new InvalidOperationException("Windows didn't change the time zone" + (output.Length > 0 ? ": " + output : ""));
        TimeZoneInfo.ClearCachedData();
    }

    public static Task TimeSetNtp(bool on) => Elevated.RunAsync("ntp", on ? "on" : "off");

    public static Task TimeSet(string when)
    {
        if (!DateTime.TryParseExact(when, "yyyy-MM-dd HH:mm", CultureInfo.InvariantCulture, DateTimeStyles.None, out _))
            throw new ArgumentException("Type the date like 2026-10-20 and the time like 14:05");
        return Elevated.RunAsync("time-set", when.Replace(' ', 'T'));
    }

    // ================================================================== Keyboard

    private static readonly Regex TipRx = new(@"^[0-9A-Fa-f]{4}:([0-9A-Fa-f]{8}|\{[0-9A-Fa-f-]{36}\}\{[0-9A-Fa-f-]{36}\})$");

    public sealed record KbLayout(string Tip, string Lang, string Name);

    /// <summary>Your layouts in order (Windows' language list, each language's keyboards).</summary>
    private static async Task<List<KbLayout>> CurrentLayouts()
    {
        var json = await PowerShell("$o = foreach ($l in Get-WinUserLanguageList) { foreach ($t in $l.InputMethodTips) { [pscustomobject]@{ tip = $t; lang = $l.LanguageTag; langName = $l.LocalizedName } } }; ConvertTo-Json -Compress -InputObject @($o)");
        var names = LayoutNames();
        var layouts = new List<KbLayout>();
        try
        {
            using var doc = JsonDocument.Parse(string.IsNullOrWhiteSpace(json) ? "[]" : json);
            if (doc.RootElement.ValueKind == JsonValueKind.Array)
                foreach (var e in doc.RootElement.EnumerateArray())
                {
                    var tip = e.TryGetProperty("tip", out var tp) ? tp.GetString() ?? "" : "";
                    var langName = e.TryGetProperty("langName", out var ln) ? ln.GetString() ?? "" : "";
                    var klid = tip.Contains(':') ? tip[(tip.IndexOf(':') + 1)..] : "";
                    var layout = names.TryGetValue(klid, out var n) ? n : "";
                    layouts.Add(new KbLayout(tip, e.TryGetProperty("lang", out var lg) ? lg.GetString() ?? "" : "", LayoutLabel(langName, layout)));
                }
        }
        catch (Exception ex) { SystemBridge.Log("keyboard: " + ex.Message); }
        return layouts;
    }

    /// <summary>Your layouts, the switch shortcut, and how a held key repeats.</summary>
    public static async Task<object> KeyboardGet()
    {
        var layouts = await CurrentLayouts();
        return new
        {
            layouts = layouts.Select(l => new { tip = l.Tip, lang = l.Lang, name = l.Name }).ToList(),
            @switch = SwitchKey(),
            delay = DelayMs(),
            rate = RateCps(),
        };
    }

    private static string LayoutLabel(string language, string layout) =>
        layout.Length == 0 ? language : language.Length == 0 ? layout : language + " · " + layout;

    /// <summary>Every keyboard layout Windows has (HKLM\…\Keyboard Layouts), as language:layout input method tips.</summary>
    public static object KeyboardLayouts()
    {
        var list = new List<KbLayout>();
        foreach (var (klid, text) in LayoutNames())
        {
            if (klid.Length != 8) continue;
            if (!int.TryParse(klid[4..], NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var lcid)) continue;
            string lang;
            try { lang = CultureInfo.GetCultureInfo(lcid).DisplayName; } catch { continue; }
            list.Add(new KbLayout(klid[4..] + ":" + klid, lang, LayoutLabel(lang, text)));
        }
        return new { layouts = list.OrderBy(x => x.Name, StringComparer.CurrentCultureIgnoreCase).Select(l => new { tip = l.Tip, lang = l.Lang, name = l.Name }).ToList() };
    }

    private static Dictionary<string, string> LayoutNames()
    {
        var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        try
        {
            using var root = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Control\Keyboard Layouts");
            if (root is null) return map;
            foreach (var klid in root.GetSubKeyNames())
            {
                using var k = root.OpenSubKey(klid);
                if (k?.GetValue("Layout Text") is string t && t.Length > 0) map[klid.ToUpperInvariant()] = t;
            }
        }
        catch { }
        return map;
    }

    /// <summary>Saves the layouts (in this order: the first is the one Windows starts with), the switch shortcut and
    /// key repeat. Layouts go through Set-WinUserLanguageList (Windows' supported way); languages that lose all
    /// their layouts are removed from the list, new ones are added.</summary>
    public static async Task<object> KeyboardSet(string[] tips, string? switchKey, int delay, int rate)
    {
        if (tips.Length == 0) throw new ArgumentException("Keep at least one layout");
        if (tips.Length > 4) throw new ArgumentException("Up to 4 layouts");
        foreach (var t in tips) if (!TipRx.IsMatch(t)) throw new ArgumentException("That layout isn't one Windows knows");
        var currentTips = (await CurrentLayouts()).Select(l => l.Tip).ToList();
        if (!currentTips.SequenceEqual(tips, StringComparer.OrdinalIgnoreCase))
        {
            var want = string.Join(",", tips.Select(t => "'" + t + "'"));
            var script = $$"""
                $want = @({{want}})
                $old = Get-WinUserLanguageList
                $list = New-Object 'System.Collections.Generic.List[Microsoft.InternationalSettings.Commands.WinUserLanguage]'
                $byLcid = @{}
                foreach ($l in $old) { foreach ($t in $l.InputMethodTips) { $byLcid[$t.Substring(0,4).ToUpper()] = $l } }
                foreach ($tip in $want) {
                  $lcid = $tip.Substring(0,4).ToUpper()
                  $lang = $list | Where-Object { $_.InputMethodTips | Where-Object { $_.Substring(0,4).ToUpper() -eq $lcid } } | Select-Object -First 1
                  if (-not $lang) {
                    $lang = $byLcid[$lcid]
                    if (-not $lang) { $tag = [Globalization.CultureInfo]::GetCultureInfo([Convert]::ToInt32($lcid, 16)).Name; $lang = New-Object Microsoft.InternationalSettings.Commands.WinUserLanguage $tag }
                    if ($list -notcontains $lang) { $lang.InputMethodTips.Clear(); $list.Add($lang) }
                  }
                  if (-not ($lang.InputMethodTips -contains $tip)) { $lang.InputMethodTips.Add($tip) }
                }
                Set-WinUserLanguageList -LanguageList $list -Force -WarningAction SilentlyContinue
                'ok'
                """;
            var outp = await PowerShell(script);
            if (!outp.Contains("ok")) throw new InvalidOperationException("Windows didn't save the layouts" + (outp.Length > 0 ? ": " + outp.Trim() : ""));
        }
        SetSwitchKey(switchKey);
        // Key repeat: Windows takes a delay of 0–3 (250–1000 ms) and a speed of 0–31 (about 2.5–30 a second).
        int d = Math.Clamp((int)Math.Round((Math.Clamp(delay, 250, 1000) - 250) / 250.0), 0, 3);
        int s = Math.Clamp((int)Math.Round((Math.Clamp(rate, 2.5, 30) - 2.5) / 27.5 * 31), 0, 31);
        SystemParametersInfo(0x0017 /* SPI_SETKEYBOARDDELAY */, (uint)d, IntPtr.Zero, 3);
        SystemParametersInfo(0x000B /* SPI_SETKEYBOARDSPEED */, (uint)s, IntPtr.Zero, 3);
        return await KeyboardGet();
    }

    private static int DelayMs() { uint v = 1; SystemParametersInfo(0x0016 /* SPI_GETKEYBOARDDELAY */, 0, ref v, 0); return 250 + (int)Math.Min(3, v) * 250; }
    private static int RateCps() { uint v = 31; SystemParametersInfo(0x000A /* SPI_GETKEYBOARDSPEED */, 0, ref v, 0); return (int)Math.Round(2.5 + Math.Min(31, v) / 31.0 * 27.5); }

    /// <summary>Windows' "switch input language" shortcut (HKCU\Keyboard Layout\Toggle: 1 Alt+Shift, 2 Ctrl+Shift, 3 none).
    /// Windows+Space always works too.</summary>
    private static string SwitchKey()
    {
        try
        {
            using var k = Registry.CurrentUser.OpenSubKey(@"Keyboard Layout\Toggle");
            var v = (k?.GetValue("Language Hotkey") ?? k?.GetValue("Hotkey")) as string;
            return v switch { "1" or null => "alt_shift", "2" => "ctrl_shift", _ => "" };
        }
        catch { return "alt_shift"; }
    }

    private static void SetSwitchKey(string? key)
    {
        if (key is null) return;
        string v = key switch { "alt_shift" => "1", "ctrl_shift" => "2", "" => "3", _ => throw new ArgumentException("Unknown shortcut") };
        using (var k = Registry.CurrentUser.CreateSubKey(@"Keyboard Layout\Toggle"))
        {
            k.SetValue("Language Hotkey", v, RegistryValueKind.String);
            k.SetValue("Hotkey", v, RegistryValueKind.String);
            k.SetValue("Layout Hotkey", "3", RegistryValueKind.String);
        }
        SystemParametersInfo(0x005B /* SPI_SETLANGTOGGLE */, 0, IntPtr.Zero, 0);
    }

    /// <summary>Desktop Mode's keyboard shortcuts on Windows (what Mantel and Windows do).</summary>
    public static object Shortcuts()
    {
        var list = new List<object>
        {
            new { keys = "Windows", action = "Open or close the app launcher" },
            new { keys = "Windows+V", action = "Windows' clipboard history (Hearth's is in the launcher)" },
            new { keys = "Alt+Tab", action = "Switch to the next window" },
            new { keys = "Alt+F4", action = "Close the window" },
            new { keys = "Windows+Left / Right", action = "Snap the window to the left or right half" },
            new { keys = "Windows+Up / Down", action = "Maximize or restore the window" },
            new { keys = "Windows+Ctrl+Left / Right", action = "Go to the previous or next desktop" },
            new { keys = "Windows+Tab", action = "Task View: desktops and windows" },
            new { keys = "Windows+L", action = "Lock the screen" },
            new { keys = "Windows+Space", action = "Switch keyboard layout" },
            new { keys = "Ctrl+Shift+Esc", action = "Open Task Manager" },
            new { keys = "Print Screen", action = "Take a screenshot" },
        };
        return new { shortcuts = list };
    }

    // ================================================================== Default apps

    /// <summary>Which app opens web links, folders, text, pictures, videos and music. Windows 11 doesn't let apps
    /// change these (it asks you in Settings › Default apps), so each row opens that page.</summary>
    public static object Defaults()
    {
        (string kind, string label, string assoc)[] kinds =
        {
            ("browser", "Web browser", "http"), ("files", "Files", "Folder"), ("text", "Text editor", ".txt"),
            ("image", "Pictures", ".jpg"), ("video", "Videos", ".mp4"), ("music", "Music", ".mp3"),
        };
        var list = new List<object>();
        foreach (var (kind, label, assoc) in kinds)
        {
            string? name = Assoc(assoc, 4 /* ASSOCSTR_FRIENDLYAPPNAME */), exe = Assoc(assoc, 2 /* ASSOCSTR_EXECUTABLE */);
            if (kind == "files") name ??= "File Explorer";
            string? icon = null;
            try { if (exe is not null && File.Exists(exe)) icon = Native.ShellIcon(exe, 32); } catch { }
            list.Add(new { kind, label, current = name ?? "", icon });
        }
        return new { kinds = list };
    }

    private static string? Assoc(string what, int str)
    {
        try
        {
            uint size = 0;
            AssocQueryString(0x40 /* ASSOCF_NOTRUNCATE */, str, what, null, null, ref size);
            if (size == 0) return null;
            var sb = new StringBuilder((int)size);
            return AssocQueryString(0x40, str, what, null, sb, ref size) == 0 && sb.Length > 0 ? sb.ToString() : null;
        }
        catch { return null; }
    }

    // ================================================================== Users

    public static object Users()
    {
        var me = WindowsIdentity.GetCurrent();
        string mySid = me.User?.Value ?? "";
        var admins = AdminSids();
        var list = new List<object>();
        try
        {
            using var q = new ManagementObjectSearcher("SELECT Name, FullName, SID, Disabled FROM Win32_UserAccount WHERE LocalAccount = TRUE");
            foreach (ManagementObject u in q.Get())
            {
                using (u)
                {
                    var sid = u["SID"] as string ?? "";
                    if (u["Disabled"] is true || sid.EndsWith("-500") || sid.EndsWith("-501") || sid.EndsWith("-503") || sid.EndsWith("-504")) continue;
                    var name = u["Name"] as string ?? "";
                    list.Add(new { name, fullname = u["FullName"] as string ?? "", admin = admins.Contains(sid), me = sid == mySid, picture = Picture(sid), player = false, home = Path.Combine(Path.GetDirectoryName(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile)) ?? @"C:\Users", name) });
                }
            }
        }
        catch (Exception ex) { SystemBridge.Log("users: " + ex.Message); }
        bool iAmAdmin = admins.Contains(mySid) || new WindowsPrincipal(me).IsInRole(WindowsBuiltInRole.Administrator);
        return new { users = list, canAdmin = true, admin = iAmAdmin };
    }

    private static HashSet<string> AdminSids()
    {
        var set = new HashSet<string>();
        try
        {
            var group = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null).Translate(typeof(NTAccount)).Value;
            var name = group.Contains('\\') ? group[(group.IndexOf('\\') + 1)..] : group;
            using var q = new ManagementObjectSearcher($"ASSOCIATORS OF {{Win32_Group.Domain='{Environment.MachineName}',Name='{name.Replace("'", "\\'")}'}} WHERE AssocClass=Win32_GroupUser Role=GroupComponent");
            foreach (ManagementObject m in q.Get()) using (m) if (m["SID"] is string s) set.Add(s);
        }
        catch (Exception ex) { SystemBridge.Log("users: admins: " + ex.Message); }
        return set;
    }

    /// <summary>The account's picture (Windows keeps a path to it per account), as a data URL, if it can be read.</summary>
    private static string? Picture(string sid)
    {
        try
        {
            using var k = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\AccountPicture\Users\" + sid);
            var path = (k?.GetValue("Image96") ?? k?.GetValue("Image64") ?? k?.GetValue("Image192")) as string;
            if (path is null || !File.Exists(path)) return null;
            using var img = Image.FromFile(path);
            return Native.ToDataUrl(new Bitmap(img, new Size(96, 96)));
        }
        catch { return null; }
    }

    private static readonly Regex UserRx = new(@"^[A-Za-z0-9][A-Za-z0-9 ._-]{0,19}$");

    public static Task UserRemove(string name, bool deleteFiles)
    {
        if (!UserRx.IsMatch(name)) throw new ArgumentException("That isn't an account name");
        if (string.Equals(name, Environment.UserName, StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("You can't remove the account you're using");
        return Elevated.RunAsync("user-remove", name, deleteFiles ? "delete-home" : "keep-home");
    }

    // ================================================================== Printers

    public static object Printers()
    {
        var list = new List<object>();
        bool spooler = true;
        try
        {
            using var q = new ManagementObjectSearcher("SELECT Name, Default, PrinterStatus, WorkOffline, PortName, Location, Network, Comment, DriverName FROM Win32_Printer");
            foreach (ManagementObject p in q.Get())
            {
                using (p)
                {
                    var status = Convert.ToInt32(p["PrinterStatus"] ?? 3);
                    bool offline = p["WorkOffline"] is true || status == 7;
                    string state = offline ? "offline" : status == 4 ? "printing" : "idle";
                    var port = p["PortName"] as string ?? "";
                    list.Add(new
                    {
                        name = p["Name"] as string ?? "",
                        info = p["Name"] as string ?? "",
                        @default = p["Default"] is true,
                        state,
                        reason = offline ? "Offline" : "",
                        location = p["Location"] as string ?? "",
                        uri = port,
                        network = p["Network"] is true || port.StartsWith("WSD", StringComparison.OrdinalIgnoreCase) || port.StartsWith("IP_", StringComparison.OrdinalIgnoreCase) || port.StartsWith("\\\\"),
                        usb = port.StartsWith("USB", StringComparison.OrdinalIgnoreCase),
                        driver = p["DriverName"] as string ?? "",
                    });
                }
            }
        }
        catch (Exception ex) { spooler = false; SystemBridge.Log("printers: " + ex.Message); }
        return new { printers = list, cups = spooler, canAdmin = true };
    }

    private static ManagementObject FindPrinter(string name)
    {
        using var q = new ManagementObjectSearcher("SELECT * FROM Win32_Printer");
        foreach (ManagementObject p in q.Get())
        {
            if (string.Equals(p["Name"] as string, name, StringComparison.Ordinal)) return p;
            p.Dispose();
        }
        throw new InvalidOperationException("That printer isn't there any more");
    }

    public static void PrinterDefault(string name)
    {
        using var p = FindPrinter(name);
        var r = Convert.ToUInt32(p.InvokeMethod("SetDefaultPrinter", null) ?? 0u);
        if (r != 0) throw new InvalidOperationException($"Windows didn't make it the default (error {r})");
        // Windows 11 can pick the default printer by itself ("Let Windows manage my default printer"): that would undo this.
        try { using var k = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows NT\CurrentVersion\Windows"); k.SetValue("LegacyDefaultPrinterMode", 1, RegistryValueKind.DWord); } catch { }
    }

    public static void PrinterTest(string name)
    {
        using var p = FindPrinter(name);
        var r = Convert.ToUInt32(p.InvokeMethod("PrintTestPage", null) ?? 0u);
        if (r != 0) throw new InvalidOperationException($"Windows didn't send the test page (error {r})");
    }

    public static void PrinterRemove(string name)
    {
        using var p = FindPrinter(name);
        try { p.Delete(); }
        catch (ManagementException ex) when (ex.ErrorCode == ManagementStatus.AccessDenied)
        { throw new UnauthorizedAccessException("Removing this printer needs administrator rights: remove it in Windows Settings › Printers"); }
    }

    // ================================================================== helpers

    /// <summary>Runs a program and returns its exit code and output (no window).</summary>
    private static async Task<(int Code, string Output)> Run(string exe, string args)
    {
        var psi = new ProcessStartInfo(exe, args) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
        using var p = Process.Start(psi) ?? throw new InvalidOperationException("Couldn't start " + exe);
        var o = p.StandardOutput.ReadToEndAsync();
        var e = p.StandardError.ReadToEndAsync();
        await p.WaitForExitAsync();
        return (p.ExitCode, ((await o) + (await e)).Trim());
    }

    /// <summary>Runs a PowerShell script (passed encoded, so nothing in it needs quoting) and returns what it printed.</summary>
    private static async Task<string> PowerShell(string script)
    {
        var enc = Convert.ToBase64String(Encoding.Unicode.GetBytes("[Console]::OutputEncoding=[Text.Encoding]::UTF8; $ProgressPreference='SilentlyContinue'; " + script));
        var psi = new ProcessStartInfo("powershell.exe", "-NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand " + enc)
        { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true, StandardOutputEncoding = Encoding.UTF8 };
        using var p = Process.Start(psi) ?? throw new InvalidOperationException("Couldn't start PowerShell");
        var o = p.StandardOutput.ReadToEndAsync();
        var e = p.StandardError.ReadToEndAsync();
        await p.WaitForExitAsync();
        var err = (await e).Trim();
        if (err.Length > 0) SystemBridge.Log("powershell: " + (err.Length > 300 ? err[..300] : err));
        return (await o).Trim();
    }

    [DllImport("user32.dll", SetLastError = true)] private static extern bool SystemParametersInfo(uint action, uint param, IntPtr pv, uint flags);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool SystemParametersInfo(uint action, uint param, ref uint pv, uint flags);
    [DllImport("shlwapi.dll", CharSet = CharSet.Unicode)] private static extern int AssocQueryString(uint flags, int str, string assoc, string? extra, StringBuilder? outp, ref uint size);
}

/// <summary>
/// Settings changes that need administrator rights: a copy of Ember started through Windows' UAC prompt
/// ("Nimbus.exe --admin TASK PROGRESSFILE ARGS", Switch/AdminTasks.cs, the same helper "Switch to Hearth OS" uses).
/// It writes how it went to the progress file; this waits for it and turns a failure into a message.
/// </summary>
internal static class Elevated
{
    public static async Task RunAsync(string task, params string[] args)
    {
        foreach (var a in args) if (a.Contains('"') || a.Contains('\0')) throw new ArgumentException("Bad argument");
        var progress = Path.Combine(Path.GetTempPath(), $"ember-admin-{Guid.NewGuid():N}.json");
        Process p;
        try
        {
            p = Process.Start(new ProcessStartInfo(Environment.ProcessPath!, $"--admin {task} \"{progress}\" " + string.Join(" ", args.Select(a => "\"" + a + "\"")))
            { UseShellExecute = true, Verb = "runas" }) ?? throw new InvalidOperationException("Windows didn't start the helper");
        }
        catch (Win32Exception ex) when (ex.NativeErrorCode == 1223)
        {
            throw new OperationCanceledException("Windows needs your OK (the administrator prompt) for this.");
        }
        using (p)
        {
            await p.WaitForExitAsync();
            string? error = null;
            try
            {
                if (File.Exists(progress))
                {
                    using var doc = JsonDocument.Parse(File.ReadAllText(progress));
                    if (doc.RootElement.TryGetProperty("error", out var e) && e.ValueKind == JsonValueKind.String) error = e.GetString();
                }
            }
            catch { }
            try { File.Delete(progress); } catch { }
            if (p.ExitCode != 0) throw new InvalidOperationException(string.IsNullOrEmpty(error) ? $"That didn't work (code {p.ExitCode})" : error);
        }
    }
}
