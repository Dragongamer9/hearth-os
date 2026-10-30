namespace HearthSteam.Open;

/// <summary>When the Open engine says no (and Hearth does it with Valve's client instead), and removing a game's files
/// safely. Pure, for the tests.</summary>
public static class OpenRules
{
    /// <summary>Null when the Open engine can install the app; else (code, why). Every "no" makes Hearth use
    /// Valve's client.</summary>
    public static (string code, string why)? CanInstall(AppInfo? app, IReadOnlyList<DepotInfo> picked, bool classicRunning, bool signedIn)
    {
        if (classicRunning) return (Codes.ClassicRunning, "The normal Steam client is running.");
        if (!signedIn) return (Codes.SignedOut, "Not signed in.");
        if (app == null) return (Codes.Unsupported, "Steam didn't give this app's details.");
        if (!app.IsGame) return (Codes.Unsupported, $"Only games ({app.Type}).");
        if (app.Depots.Any(d => d.Encrypted && d.Manifest == 0) && picked.Count == 0) return (Codes.Unsupported, "Its depots are encrypted.");
        if (picked.Count == 0) return (Codes.Unsupported, "No depots to download for this device.");
        return null;
    }

    /// <summary>The library an app goes to: the one that already has its record (an update), else the chosen one,
    /// else the first.</summary>
    public static string PickLibrary(IReadOnlyList<string> libraries, Func<string, bool> hasRecord, string? chosen)
    {
        foreach (var l in libraries) if (hasRecord(l)) return l;
        var same = string.IsNullOrEmpty(chosen) ? null : libraries.FirstOrDefault(l => l.TrimEnd('/') == chosen.TrimEnd('/'));
        if (same != null) return same;
        if (!string.IsNullOrEmpty(chosen) && Directory.Exists(Path.Combine(chosen, "steamapps"))) return chosen;
        return libraries.FirstOrDefault() ?? throw new EngineException(Codes.Unsupported, "No Steam library.");
    }

    /// <summary>Removes an installed game from a library: its folder in steamapps/common (one plain folder name; a
    /// link there is removed, never followed), the Open engine's staging and saved manifests for it, then the
    /// record. Proton prefixes (compatdata: saves) are kept.</summary>
    public static void Uninstall(string library, uint appId)
    {
        var apps = Path.Combine(library, "steamapps");
        var acf = AppManifest.Load(library, appId) ?? throw new EngineException(Codes.Unsupported, "Not installed in this library.");
        var aside = AppManifest.Path(library, appId) + ".hearth-removing";
        File.Move(AppManifest.Path(library, appId), aside, true);
        if (AppManifest.SafeDirName(acf.InstallDir))
        {
            var dir = new DirectoryInfo(Path.Combine(apps, "common", acf.InstallDir));
            if (dir.LinkTarget != null) dir.Delete();
            else if (dir.Exists && Path.GetFullPath(dir.Parent!.FullName) == Path.GetFullPath(Path.Combine(apps, "common"))) dir.Delete(true);
        }
        var mine = new DirectoryInfo(Path.Combine(apps, OpenPaths.Folder, appId.ToString()));
        if (mine.Exists && mine.LinkTarget == null) mine.Delete(true);
        foreach (var sub in new[] { "shadercache", "downloading", "temp" })
        {
            var d = new DirectoryInfo(Path.Combine(apps, sub, appId.ToString()));
            if (d.LinkTarget != null) d.Delete(); else if (d.Exists) d.Delete(true);
        }
        File.Delete(aside);
    }
}

/// <summary>Where the Open engine keeps its own things inside a Steam library (Valve's client ignores the folder).</summary>
public static class OpenPaths
{
    public const string Folder = ".hearth-steam";
    public static string AppDir(string library, uint appId) => Path.Combine(library, "steamapps", Folder, appId.ToString());
    public static string Staging(string library, uint appId) => Path.Combine(AppDir(library, appId), "staging");
    public static string Manifest(string library, uint appId, uint depot, ulong gid) => Path.Combine(AppDir(library, appId), $"{depot}_{gid}.manifest");
}
