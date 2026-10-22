namespace Nimbus.Host;

/// <summary>Hearth Disks (hosts/hearth-disks), the disk manager, ships inside Ember's package as hearth-disks.exe.
/// This keeps its Start menu entry ("Hearth Disks") there and pointing at it, also after an update or a move
/// (install-nimbus.ps1 makes it on a fresh install).</summary>
internal static class DisksShortcut
{
    public static void Ensure()
    {
        try
        {
            var dir = Path.GetDirectoryName(Environment.ProcessPath);
            if (dir is null) return;
            var exe = Path.Combine(dir, "hearth-disks.exe");
            if (!File.Exists(exe)) return;
            var lnk = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs), "Hearth Disks.lnk");
            var shellType = Type.GetTypeFromProgID("WScript.Shell");
            if (shellType is null) return;
            dynamic shell = Activator.CreateInstance(shellType)!;
            dynamic sc = shell.CreateShortcut(lnk);
            if (File.Exists(lnk) && string.Equals((string)sc.TargetPath, exe, StringComparison.OrdinalIgnoreCase)) return;
            sc.TargetPath = exe;
            sc.WorkingDirectory = dir;
            sc.Description = "See your drives and their health, format USB drives and SD cards, and set up drives for games";
            sc.IconLocation = exe + ",0";
            sc.Save();
        }
        catch (Exception ex) { SystemBridge.Log("disks shortcut: " + ex.Message); }
    }
}
