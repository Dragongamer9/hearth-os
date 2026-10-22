namespace Nimbus.Host;

/// <summary>
/// Hearth Terminal (hosts/hearth-terminal) comes in Ember's package as hearth-terminal.exe, next to Nimbus.exe.
/// This keeps a "Hearth Terminal" shortcut for it in your Start menu, which is also where Mantel's app list comes from.
/// </summary>
internal static class HearthTerminal
{
    public static string Exe => Path.Combine(AppContext.BaseDirectory, "hearth-terminal.exe");

    /// <summary>Makes (or repoints) the Start menu shortcut. Does nothing in a build without the terminal.</summary>
    public static void EnsureStartMenuEntry()
    {
        try
        {
            if (!File.Exists(Exe)) return;
            var programs = Environment.GetFolderPath(Environment.SpecialFolder.Programs);
            var lnk = Path.Combine(programs, "Hearth Terminal.lnk");
            var shellType = Type.GetTypeFromProgID("WScript.Shell");
            if (shellType is null) return;
            dynamic shell = Activator.CreateInstance(shellType)!;
            dynamic sc = shell.CreateShortcut(lnk);
            // Already right: leave it (and its place in the Start menu) alone.
            if (File.Exists(lnk) && string.Equals((string)sc.TargetPath, Exe, StringComparison.OrdinalIgnoreCase)) return;
            sc.TargetPath = Exe;
            sc.WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            sc.Description = "Hearth Terminal: PowerShell, Command Prompt and WSL";
            sc.IconLocation = Exe + ",0";
            sc.Save();
        }
        catch (Exception ex) { SystemBridge.Log("hearth terminal shortcut: " + ex.Message); }
    }
}
