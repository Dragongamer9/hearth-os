using System.Diagnostics;
using Microsoft.Win32;

namespace Nimbus.Host;

/// <summary>
/// Starts the apps that should start when you sign in (Discord, Steam, cloud storage, drivers' tray tools),
/// which Explorer normally does. Only when Ember is your shell, once per sign-in, a little after Ember is up.
/// The same places and rules as Explorer:
///   Run and RunOnce keys (your account and the PC), and the Startup folders (yours and everyone's),
///   skipping what you turned off in Task Manager › Startup apps (the StartupApproved entries).
/// </summary>
public static class StartupApps
{
    private const string Run = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string Run32 = @"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
    private const string RunOnce = @"Software\Microsoft\Windows\CurrentVersion\RunOnce";
    private const string Approved = @"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\";

    /// <summary>Starts them once per sign-in (an Ember restart in the same session doesn't start them again).</summary>
    public static void RunOncePerSignIn()
    {
        try
        {
            // A marker in %TEMP% named after this sign-in session's start: survives Ember restarts, not sign-outs.
            var session = Process.GetCurrentProcess().SessionId;
            var boot = DateTime.Now - TimeSpan.FromMilliseconds(Environment.TickCount64);
            var logon = GetLogonTime() ?? boot;
            var marker = Path.Combine(Path.GetTempPath(), $"ember-startup-{session}-{logon:yyyyMMddHHmm}.done");
            if (File.Exists(marker)) return;
            File.WriteAllText(marker, DateTime.Now.ToString("O"));
            int n = 0;
            foreach (var (name, cmd) in Entries()) { if (Start(name, cmd)) n++; }
            SystemBridge.Log($"startup apps: started {n}");
        }
        catch (Exception ex) { SystemBridge.Log("startup apps: " + ex.Message); }
    }

    /// <summary>When this account signed in (the start time of its winlogon-launched shell host).</summary>
    private static DateTime? GetLogonTime()
    {
        try
        {
            int session = Process.GetCurrentProcess().SessionId;
            var p = Process.GetProcessesByName("winlogon").FirstOrDefault(x => x.SessionId == session);
            return p?.StartTime;
        }
        catch { return null; }
    }

    public static IEnumerable<(string Name, string Command)> Entries()
    {
        foreach (var hive in new[] { Registry.CurrentUser, Registry.LocalMachine })
        {
            foreach (var e in RegistryEntries(hive, Run, "Run")) yield return e;
            // RunOnce entries are removed as they're started, so they run once. The PC's own (HKLM) can only be removed
            // with administrator rights; like Explorer, Ember leaves them for an administrator's sign-in then (opening the
            // key for writing fails) instead of starting them again at every sign-in.
            foreach (var e in RegistryEntries(hive, RunOnce, null, deleteAfter: true)) yield return e;
        }
        // 32-bit programs put theirs in the PC's 32-bit Run key, which Explorer starts too (Task Manager's switch: Run32).
        foreach (var e in RegistryEntries(Registry.LocalMachine, Run32, "Run32")) yield return e;
        foreach (var (folder, approvedKey, hive) in new[]
        {
            (Environment.GetFolderPath(Environment.SpecialFolder.Startup), "StartupFolder", Registry.CurrentUser),
            (Environment.GetFolderPath(Environment.SpecialFolder.CommonStartup), "StartupFolder", Registry.LocalMachine),
        })
        {
            if (!Directory.Exists(folder)) continue;
            foreach (var f in Directory.GetFiles(folder))
            {
                var name = Path.GetFileName(f);
                if (name.Equals("desktop.ini", StringComparison.OrdinalIgnoreCase)) continue;
                if (!Enabled(hive, approvedKey, name)) continue;
                yield return (name, f);
            }
        }
    }

    private static IEnumerable<(string, string)> RegistryEntries(RegistryKey hive, string path, string? approvedKey, bool deleteAfter = false)
    {
        var list = new List<(string, string)>();
        try
        {
            using var k = hive.OpenSubKey(path, writable: deleteAfter);
            if (k is null) return list;
            foreach (var name in k.GetValueNames())
            {
                if (k.GetValue(name) is not string cmd || string.IsNullOrWhiteSpace(cmd)) continue;
                if (approvedKey is not null && !Enabled(hive, approvedKey, name)) continue;
                list.Add((name, cmd));
                // RunOnce entries run once: "!name" ones only differ in when Explorer deletes them (after they ran).
                if (deleteAfter) { try { k.DeleteValue(name); } catch { } }
            }
        }
        catch { }
        return list;
    }

    /// <summary>Task Manager's on/off switch: first byte 2 (or no entry) = on, 3 = off.</summary>
    private static bool Enabled(RegistryKey hive, string approvedKey, string name)
    {
        try
        {
            using var k = hive.OpenSubKey(Approved + approvedKey);
            return k?.GetValue(name) is not byte[] b || b.Length == 0 || (b[0] & 1) == 0;
        }
        catch { return true; }
    }

    private static bool Start(string name, string command)
    {
        try
        {
            if (File.Exists(command))   // a shortcut or program in a Startup folder
            {
                Process.Start(new ProcessStartInfo(command) { UseShellExecute = true, WorkingDirectory = Path.GetDirectoryName(command) });
                return true;
            }
            var (file, args) = Split(Environment.ExpandEnvironmentVariables(command));
            Process.Start(new ProcessStartInfo(file, args) { UseShellExecute = true, WorkingDirectory = Path.GetDirectoryName(file) ?? "" });
            return true;
        }
        catch (Exception ex) { SystemBridge.Log($"startup apps: {name}: {ex.Message}"); return false; }
    }

    /// <summary>"C:\Program Files\X\x.exe" --tray  →  (path, args); unquoted paths with spaces are matched up to ".exe".</summary>
    public static (string File, string Args) Split(string cmd)
    {
        cmd = cmd.Trim();
        if (cmd.StartsWith('"'))
        {
            int end = cmd.IndexOf('"', 1);
            if (end > 0) return (cmd[1..end], cmd[(end + 1)..].Trim());
        }
        int exe = cmd.IndexOf(".exe", StringComparison.OrdinalIgnoreCase);
        if (exe > 0) return (cmd[..(exe + 4)], cmd[(exe + 4)..].Trim());
        int sp = cmd.IndexOf(' ');
        return sp > 0 ? (cmd[..sp], cmd[(sp + 1)..]) : (cmd, "");
    }
}
