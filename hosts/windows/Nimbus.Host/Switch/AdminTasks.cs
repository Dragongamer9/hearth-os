using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Win32;
using Microsoft.Win32.SafeHandles;

namespace Nimbus.Host.Switch;

/// <summary>
/// The steps of "Switch to Hearth OS" that need administrator rights, run in a separate copy of Ember that
/// Windows starts after its password / UAC prompt ("Ember.exe --admin TASK ARGS"). Each writes its progress
/// to a small JSON file the normal Ember reads, and exits 0 when it worked.
///
///   prepare SIZE_GB            turn off Fast Startup and hibernation, shrink C: by SIZE_GB (Windows does that
///                              itself, BitLocker or not), leaving free space for Hearth's installer
///   writeusb DISK_NUMBER ISO   erase that USB drive and write the Hearth installer onto it
/// and for Hearth Settings in Mantel (Mantel/WinSystem.cs, through Mantel/Elevated):
///   ntp on|off                 Windows' "Set time automatically" (W32Time)
///   time-set YYYY-MM-DDTHH:MM  set the clock by hand
///   user-remove NAME keep-home|delete-home   remove a local account (and its files)
/// </summary>
public static class AdminTasks
{
    public static int Run(string[] args)
    {
        // --admin TASK PROGRESSFILE ...
        int i = Array.FindIndex(args, a => a.Equals("--admin", StringComparison.OrdinalIgnoreCase));
        if (i < 0 || args.Length < i + 3) return 2;
        string task = args[i + 1], progress = args[i + 2];
        var rest = args.Skip(i + 3).ToArray();
        void Report(string state, double pct, string text, string? error = null) =>
            WriteProgress(progress, new { state, pct, text, error, at = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() });
        try
        {
            switch (task)
            {
                case "prepare": Prepare(int.Parse(rest[0]), Report); break;
                case "writeusb": WriteUsb(int.Parse(rest[0]), rest[1], Report); break;
                case "ntp": Ntp(rest[0] == "on", Report); break;
                case "time-set": TimeSet(rest[0], Report); break;
                case "user-remove": UserRemove(rest[0], rest[1] == "delete-home", Report); break;
                // The power limit (Services/TdpService.cs): tdp WATTS [DOWNLOAD_FOLDER]
                case "tdp": Services.TdpService.RunElevated(rest, Report); break;
                default: Report("error", 0, "", "Unknown task " + task); return 2;
            }
            return 0;
        }
        catch (Exception ex)
        {
            Report("error", 0, "", ex.Message);
            return 1;
        }
    }

    private static void WriteProgress(string file, object data)
    {
        try { File.WriteAllText(file + ".tmp", JsonSerializer.Serialize(data)); File.Move(file + ".tmp", file, true); } catch { }
    }

    // ------------------------------------------------------------------ prepare: make room on C:

    private static void Prepare(int sizeGb, Action<string, double, string, string?> report)
    {
        if (sizeGb < 32 || sizeGb > 4096) throw new ArgumentException("Size out of range");
        report("running", 5, "Turning off Fast Startup", null);
        // Fast Startup leaves Windows' drive half-asleep, so Linux can't change it safely.
        using (var k = Registry.LocalMachine.CreateSubKey(@"SYSTEM\CurrentControlSet\Control\Session Manager\Power"))
            k.SetValue("HiberbootEnabled", 0, RegistryValueKind.DWord);
        Exec("powercfg.exe", "/hibernate off");

        report("running", 15, "Checking how much room Windows can give", null);
        var query = Diskpart("select volume C\r\nshrink querymax\r\n");
        var m = System.Text.RegularExpressions.Regex.Match(query, @"(\d+)\s*(GB|MB)", System.Text.RegularExpressions.RegexOptions.IgnoreCase);
        if (m.Success)
        {
            long maxMb = long.Parse(m.Groups[1].Value) * (m.Groups[2].Value.Equals("GB", StringComparison.OrdinalIgnoreCase) ? 1024 : 1);
            if (maxMb < sizeGb * 1024L)
                throw new InvalidOperationException($"Windows can only give {maxMb / 1024} GB right now (files it can't move are in the way). Try a smaller size, or run Disk Cleanup first.");
        }

        report("running", 30, $"Shrinking Windows' drive by {sizeGb} GB (this can take several minutes)", null);
        var outp = Diskpart($"select volume C\r\nshrink desired={sizeGb * 1024L} minimum={sizeGb * 1024L}\r\n");
        if (!System.Text.RegularExpressions.Regex.IsMatch(outp, @"successfully shrunk|reduced the volume", System.Text.RegularExpressions.RegexOptions.IgnoreCase))
            throw new InvalidOperationException("Windows couldn't shrink its drive: " + LastLine(outp));
        report("done", 100, $"Made {sizeGb} GB of room for Hearth", null);
    }

    // ------------------------------------------------------------------ write the installer USB

    private static void WriteUsb(int disk, string iso, Action<string, double, string, string?> report)
    {
        if (!File.Exists(iso)) throw new FileNotFoundException("The Hearth installer file wasn't found", iso);
        // Only a removable USB disk, never the one Windows runs from.
        if (!IsUsbDisk(disk)) throw new InvalidOperationException("That drive isn't a USB drive");
        report("running", 1, "Erasing the USB drive", null);
        Diskpart($"select disk {disk}\r\nattributes disk clear readonly\r\nclean\r\n");
        Thread.Sleep(1500);

        long total = new FileInfo(iso).Length, done = 0;
        using var src = new FileStream(iso, FileMode.Open, FileAccess.Read, FileShare.Read, 1 << 20);
        using var handle = CreateFile($@"\\.\PhysicalDrive{disk}", 0x40000000 /* GENERIC_WRITE */, 0x3 /* share read|write */, IntPtr.Zero,
            3 /* OPEN_EXISTING */, 0x80000000 /* WRITE_THROUGH */, IntPtr.Zero);
        if (handle.IsInvalid) throw new IOException("Couldn't open the USB drive (error " + Marshal.GetLastWin32Error() + ")");
        using var dst = new FileStream(handle, FileAccess.Write, 0, false);
        var buf = new byte[4 << 20];   // 4 MB at a time, whole sectors
        var sw = Stopwatch.StartNew();
        long lastReport = 0;
        int n;
        while ((n = src.Read(buf, 0, buf.Length)) > 0)
        {
            int len = (n + 511) / 512 * 512;   // whole sectors (the last block is padded)
            if (len > n) Array.Clear(buf, n, len - n);
            dst.Write(buf, 0, len);
            done += n;
            if (sw.ElapsedMilliseconds - lastReport > 500)
            {
                lastReport = sw.ElapsedMilliseconds;
                double mbps = done / 1e6 / Math.Max(0.5, sw.Elapsed.TotalSeconds);
                report("running", 2 + 96.0 * done / total, $"Writing the installer: {done / 1e9:0.0} of {total / 1e9:0.0} GB · {mbps:0} MB/s", null);
            }
        }
        dst.Flush();
        report("done", 100, "The Hearth installer USB is ready", null);
    }

    private static bool IsUsbDisk(int disk)
    {
        try
        {
            using var s = new System.Management.ManagementObjectSearcher($"SELECT InterfaceType, MediaType, Index FROM Win32_DiskDrive WHERE Index = {disk}");
            foreach (System.Management.ManagementObject d in s.Get())
            {
                var iface = d["InterfaceType"]?.ToString() ?? "";
                var media = d["MediaType"]?.ToString() ?? "";
                return iface.Equals("USB", StringComparison.OrdinalIgnoreCase) || media.Contains("Removable", StringComparison.OrdinalIgnoreCase);
            }
        }
        catch { }
        return false;
    }

    // ------------------------------------------------------------------ Hearth Settings (Mantel)

    private static void Ntp(bool on, Action<string, double, string, string?> report)
    {
        using (var k = Registry.LocalMachine.CreateSubKey(@"SYSTEM\CurrentControlSet\Services\W32Time\Parameters"))
            k.SetValue("Type", on ? "NTP" : "NoSync", RegistryValueKind.String);
        if (on)
        {
            RunTool("sc.exe", "config w32time start= auto");
            RunTool("net.exe", "start w32time");
            RunTool("w32tm.exe", "/config /update");
            RunTool("w32tm.exe", "/resync /nowait");
        }
        else RunTool("w32tm.exe", "/config /update");
        report("done", 100, on ? "The clock is set from the internet" : "Automatic time is off", null);
    }

    private static void TimeSet(string when, Action<string, double, string, string?> report)
    {
        if (!DateTime.TryParseExact(when, "yyyy-MM-ddTHH:mm", System.Globalization.CultureInfo.InvariantCulture, System.Globalization.DateTimeStyles.None, out var t))
            throw new ArgumentException("Bad date");
        var st = new SYSTEMTIME { wYear = (ushort)t.Year, wMonth = (ushort)t.Month, wDay = (ushort)t.Day, wHour = (ushort)t.Hour, wMinute = (ushort)t.Minute };
        // Twice, as Windows' documentation says (the first call can land on the wrong side of a daylight-saving change).
        if (!SetLocalTime(ref st) || !SetLocalTime(ref st)) throw new InvalidOperationException("Windows didn't set the clock (error " + Marshal.GetLastWin32Error() + ")");
        report("done", 100, "Clock set", null);
    }

    private static void UserRemove(string name, bool deleteHome, Action<string, double, string, string?> report)
    {
        if (!System.Text.RegularExpressions.Regex.IsMatch(name, @"^[A-Za-z0-9][A-Za-z0-9 ._-]{0,19}$")) throw new ArgumentException("That isn't an account name");
        if (string.Equals(name, Environment.UserName, StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("You can't remove the account you're using");
        string? sid = null;
        try { sid = ((System.Security.Principal.SecurityIdentifier)new System.Security.Principal.NTAccount(Environment.MachineName, name).Translate(typeof(System.Security.Principal.SecurityIdentifier))).Value; } catch { }
        var (code, output) = RunTool("net.exe", $"user \"{name}\" /delete");
        if (code != 0) throw new InvalidOperationException("Windows didn't remove the account: " + LastLine(output));
        if (deleteHome && sid is not null)
        {
            try
            {
                using var q = new System.Management.ManagementObjectSearcher($"SELECT * FROM Win32_UserProfile WHERE SID = '{sid}'");
                foreach (System.Management.ManagementObject p in q.Get()) using (p) p.Delete();
            }
            catch (Exception ex) { throw new InvalidOperationException("The account is removed, but its files couldn't be deleted: " + ex.Message); }
        }
        report("done", 100, "Account removed", null);
    }

    private static (int Code, string Output) RunTool(string file, string args)
    {
        using var p = Process.Start(new ProcessStartInfo(file, args) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!;
        string o = p.StandardOutput.ReadToEnd() + p.StandardError.ReadToEnd();
        p.WaitForExit();
        return (p.ExitCode, o);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SYSTEMTIME { public ushort wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds; }
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetLocalTime(ref SYSTEMTIME t);

    // ------------------------------------------------------------------ helpers

    private static string Diskpart(string script)
    {
        var file = Path.Combine(Path.GetTempPath(), $"ember-diskpart-{Guid.NewGuid():N}.txt");
        File.WriteAllText(file, script);
        try { return Exec("diskpart.exe", $"/s \"{file}\""); }
        finally { try { File.Delete(file); } catch { } }
    }

    private static string Exec(string file, string args)
    {
        var p = Process.Start(new ProcessStartInfo(file, args) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!;
        string o = p.StandardOutput.ReadToEnd() + p.StandardError.ReadToEnd();
        p.WaitForExit();
        SystemBridge.Log($"admin: {file} {args.Split(' ')[0]} → {p.ExitCode}");
        return o;
    }

    private static string LastLine(string s) => s.Split('\n').Select(l => l.Trim()).LastOrDefault(l => l.Length > 0) ?? "";

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr sec, uint disposition, uint flags, IntPtr template);
}
