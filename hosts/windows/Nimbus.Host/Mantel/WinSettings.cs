using System.Diagnostics;
using System.Net.NetworkInformation;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace Nimbus.Host.Mantel;

/// <summary>
/// The Windows side of a few Hearth Settings rows that Hearth OS does with its own tools:
/// "Ask for my password after sleep" and the power button (Windows' power plan, through powercfg, the same
/// settings as Windows' own Settings app writes), the wired connection on the taskbar, and Aloha Browser's
/// state on the Web browser page. Everything is best effort: null means "can't tell", and the page then
/// leaves the row out.
/// </summary>
internal static class WinSettings
{
    // ------------------------------------------------------------------ powercfg

    private static string? Powercfg(string args)
    {
        try
        {
            using var p = Process.Start(new ProcessStartInfo("powercfg.exe", args)
            {
                UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
            });
            if (p is null) return null;
            var output = p.StandardOutput.ReadToEnd();
            if (!p.WaitForExit(5000)) { try { p.Kill(); } catch { } return null; }
            return p.ExitCode == 0 ? output : null;
        }
        catch (Exception ex) { SystemBridge.Log("powercfg " + args + ": " + ex.Message); return null; }
    }

    /// <summary>A power setting's current value (plugged in) on the active plan. powercfg's words are translated
    /// on other Windows languages, so this reads the first "0x0000000N" line (the AC value comes before DC).</summary>
    private static int? PowerIndex(string sub, string setting)
    {
        var o = Powercfg($"/q SCHEME_CURRENT {sub} {setting}");
        if (o is null) return null;
        var m = Regex.Match(o, @"0x([0-9a-fA-F]{8})\s*$", RegexOptions.Multiline);
        return m.Success ? Convert.ToInt32(m.Groups[1].Value, 16) : null;
    }

    private static bool SetPowerIndex(string sub, string setting, int value)
    {
        bool ok = Powercfg($"/setacvalueindex SCHEME_CURRENT {sub} {setting} {value}") is not null;
        ok &= Powercfg($"/setdcvalueindex SCHEME_CURRENT {sub} {setting} {value}") is not null;
        Powercfg("/setactive SCHEME_CURRENT");
        return ok;
    }

    /// <summary>Windows asks for the password when the PC wakes (powercfg's CONSOLELOCK), or null if unknown.</summary>
    public static bool? LockOnWake { get { var v = PowerIndex("SUB_NONE", "CONSOLELOCK"); return v is null ? null : v != 0; } }

    public static void SetLockOnWake(bool on)
    {
        if (!SetPowerIndex("SUB_NONE", "CONSOLELOCK", on ? 1 : 0)) throw new InvalidOperationException("Windows didn't take the change");
    }

    /// <summary>What a quick press of the power button does: "sleep", "shutdown", "hibernate" or "nothing" (null if unknown).</summary>
    public static string? PowerPress
    {
        get => PowerIndex("SUB_BUTTONS", "PBUTTONACTION") switch
        {
            0 => "nothing", 1 => "sleep", 2 => "hibernate", 3 => "shutdown", 4 => "nothing" /* turn off the display */, _ => null,
        };
    }

    public static void SetPowerPress(string what)
    {
        int v = what switch { "nothing" => 0, "sleep" => 1, "hibernate" => 2, "shutdown" => 3, _ => throw new ArgumentException("Power button: sleep, shutdown or nothing") };
        if (!SetPowerIndex("SUB_BUTTONS", "PBUTTONACTION", v)) throw new InvalidOperationException("Windows didn't take the change");
    }

    // ------------------------------------------------------------------ the wired connection (taskbar)

    /// <summary>A working Ethernet connection (with a gateway), like NetworkManager's "ethernet: connected".</summary>
    public static object? Wired()
    {
        try
        {
            foreach (var n in NetworkInterface.GetAllNetworkInterfaces())
            {
                if (n.OperationalStatus != OperationalStatus.Up) continue;
                if (n.NetworkInterfaceType is not (NetworkInterfaceType.Ethernet or NetworkInterfaceType.GigabitEthernet or NetworkInterfaceType.FastEthernetT or NetworkInterfaceType.FastEthernetFx or NetworkInterfaceType.Ethernet3Megabit)) continue;
                var d = n.Description.ToLowerInvariant();
                if (d.Contains("virtual") || d.Contains("hyper-v") || d.Contains("vpn") || d.Contains("loopback") || d.Contains("bluetooth") || d.Contains("vmware") || d.Contains("virtualbox")) continue;
                if (!n.GetIPProperties().GatewayAddresses.Any(g => !g.Address.Equals(System.Net.IPAddress.Any) && !g.Address.Equals(System.Net.IPAddress.IPv6Any))) continue;
                return new { connected = true, name = n.Name };
            }
        }
        catch { }
        return null;
    }

    // ------------------------------------------------------------------ Aloha Browser (Hearth Settings › Web browser)

    /// <summary>Is Aloha Browser installed (Windows' list of installed apps), its version, and does it open links?</summary>
    public static object BrowserStatus()
    {
        string? version = null; bool installed = false;
        foreach (var hive in new[] { Registry.CurrentUser, Registry.LocalMachine })
            foreach (var path in new[] { @"Software\Microsoft\Windows\CurrentVersion\Uninstall", @"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall" })
            {
                try
                {
                    using var root = hive.OpenSubKey(path);
                    if (root is null) continue;
                    foreach (var name in root.GetSubKeyNames())
                    {
                        using var k = root.OpenSubKey(name);
                        var display = k?.GetValue("DisplayName") as string ?? "";
                        if (!display.Contains("Aloha", StringComparison.OrdinalIgnoreCase)) continue;
                        installed = true;
                        version ??= k?.GetValue("DisplayVersion") as string;
                    }
                }
                catch { }
            }
        bool isDefault = false;
        try
        {
            using var k = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice");
            var prog = k?.GetValue("ProgId") as string ?? "";
            isDefault = prog.Contains("Aloha", StringComparison.OrdinalIgnoreCase);
            if (isDefault) installed = true;
        }
        catch { }
        // Hearth Browser (Firefox with Hearth's settings) too: Settings shows both.
        return new { installed, @default = isDefault, version, hearth = Services.HearthBrowser.Status() };
    }
}
