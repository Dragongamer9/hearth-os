namespace Nimbus.Host.Mantel;

/// <summary>
/// The launcher's Windows section: the programs that come with Windows, each only if this PC has it. Store-style apps
/// (Calculator, Notepad, Paint, Photos... on Windows 11) are found in the Start menu's list by their app id and open
/// through shell:AppsFolder like the Start menu opens them; classic tools are found by their file (devmgmt.msc,
/// regedit.exe...). Icons come from the shell. The ids are what Mantel's launch and pins already understand (an app id
/// or a path), so these can be pinned to the taskbar too.
/// </summary>
internal static class WindowsApps
{
    /// <summary>A program: its name here, the Start menu app ids that are it (newest first), the Start menu names that
    /// are it (English Windows, for apps whose id differs between versions), and its files.</summary>
    private sealed record Known(string Name, string[] Ids, string[] StartNames, string[] Files, string? Uri = null);

    private static string Sys(string f) => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), f);
    private static string Win(string f) => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows), f);
    private static string Pf(string f) => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), f);
    private static string Pf86(string f) => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86), f);

    private static Known[] Table() => new Known[]
    {
        new("File Explorer", new[] { "Microsoft.Windows.Explorer" }, new[] { "File Explorer" }, new[] { Win("explorer.exe") }),
        new("Settings", new[] { "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel" }, new[] { "Settings" }, Array.Empty<string>(), "ms-settings:"),
        new("Control Panel", new[] { "Microsoft.Windows.ControlPanel" }, new[] { "Control Panel" }, new[] { Sys("control.exe") }),
        new("Task Manager", new[] { "Microsoft.Windows.TaskManager" }, new[] { "Task Manager" }, new[] { Sys("Taskmgr.exe") }),
        new("Notepad", new[] { "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App" }, new[] { "Notepad" }, new[] { Win("notepad.exe"), Sys("notepad.exe") }),
        new("Paint", new[] { "Microsoft.Paint_8wekyb3d8bbwe!App" }, new[] { "Paint" }, new[] { Sys("mspaint.exe") }),
        new("Calculator", new[] { "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App" }, new[] { "Calculator" }, new[] { Sys("calc.exe") }),
        new("Snipping Tool", new[] { "Microsoft.ScreenSketch_8wekyb3d8bbwe!App" }, new[] { "Snipping Tool" }, new[] { Sys("SnippingTool.exe") }),
        new("Windows Terminal", new[] { "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App", "Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe!App" }, new[] { "Terminal" }, Array.Empty<string>()),
        new("PowerShell", Array.Empty<string>(), Array.Empty<string>(), new[] { Pf(@"PowerShell\7\pwsh.exe"), Sys(@"WindowsPowerShell\v1.0\powershell.exe") }),
        new("Command Prompt", Array.Empty<string>(), Array.Empty<string>(), new[] { Sys("cmd.exe") }),
        new("Microsoft Edge", new[] { "MSEdge" }, new[] { "Microsoft Edge" }, new[] { Pf86(@"Microsoft\Edge\Application\msedge.exe"), Pf(@"Microsoft\Edge\Application\msedge.exe") }),
        new("Photos", new[] { "Microsoft.Windows.Photos_8wekyb3d8bbwe!App" }, new[] { "Photos" }, Array.Empty<string>()),
        new("Media Player", new[] { "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic" }, new[] { "Media Player" }, new[] { Pf(@"Windows Media Player\wmplayer.exe") }),
        new("Clock", new[] { "Microsoft.WindowsAlarms_8wekyb3d8bbwe!App" }, new[] { "Clock", "Alarms & Clock" }, Array.Empty<string>()),
        new("Camera", new[] { "Microsoft.WindowsCamera_8wekyb3d8bbwe!App" }, new[] { "Camera" }, Array.Empty<string>()),
        new("Microsoft Store", new[] { "Microsoft.WindowsStore_8wekyb3d8bbwe!App" }, new[] { "Microsoft Store" }, Array.Empty<string>()),
        new("Device Manager", Array.Empty<string>(), Array.Empty<string>(), new[] { Sys("devmgmt.msc") }),
        new("Disk Management", Array.Empty<string>(), Array.Empty<string>(), new[] { Sys("diskmgmt.msc") }),
        new("Registry Editor", Array.Empty<string>(), new[] { "Registry Editor" }, new[] { Win("regedit.exe") }),
        new("System Information", Array.Empty<string>(), new[] { "System Information" }, new[] { Sys("msinfo32.exe") }),
        new("Character Map", Array.Empty<string>(), new[] { "Character Map" }, new[] { Sys("charmap.exe") }),
        new("Remote Desktop Connection", Array.Empty<string>(), new[] { "Remote Desktop Connection" }, new[] { Sys("mstsc.exe") }),
        new("On-Screen Keyboard", Array.Empty<string>(), new[] { "On-Screen Keyboard" }, new[] { Sys("osk.exe") }),
        new("Magnifier", Array.Empty<string>(), new[] { "Magnifier" }, new[] { Sys("Magnify.exe") }),
    };

    private static List<Apps.StartApp>? _list;
    private static DateTime _at;

    /// <summary>The programs this PC has, in the table's order. Read once and kept for ten minutes.</summary>
    public static async Task<List<Apps.StartApp>> ListAsync()
    {
        if (_list is not null && DateTime.UtcNow - _at < TimeSpan.FromMinutes(10)) return _list;
        var start = await Apps.StartAppsAsync();
        var list = await Task.Run(() =>
        {
            var found = new List<Apps.StartApp>();
            foreach (var k in Table())
            {
                // The Start menu's own entry first: its id opens it the right way, and it has the right icon.
                var s = k.Ids.Select(id => start.FirstOrDefault(a => string.Equals(a.Id, id, StringComparison.OrdinalIgnoreCase))).FirstOrDefault(a => a is not null)
                    ?? k.StartNames.Select(n => start.FirstOrDefault(a => string.Equals(a.Name, n, StringComparison.OrdinalIgnoreCase))).FirstOrDefault(a => a is not null);
                if (s is not null) { found.Add(new Apps.StartApp(s.Id, k.Name, s.Icon ?? Native.ShellIcon(@"shell:AppsFolder\" + s.Id, 64))); continue; }
                var file = k.Files.FirstOrDefault(File.Exists);
                if (file is not null) { found.Add(new Apps.StartApp(file, k.Name, IconOf(file))); continue; }
                if (k.Uri is not null) found.Add(new Apps.StartApp(k.Uri, k.Name, IconOf(Sys("SystemSettingsAdminFlows.exe")) ?? IconOf(Sys("control.exe"))));
            }
            return found;
        });
        _list = list;
        _at = DateTime.UtcNow;
        return list;
    }

    private static string? IconOf(string file)
    {
        if (!File.Exists(file)) return null;
        try { return Native.ShellIcon(file, 64); } catch { return null; }
    }
}
