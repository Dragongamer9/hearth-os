using System.Text;

namespace HearthSteam.Open;

/// <summary>steamapps/appmanifest_&lt;id&gt;.acf, written the way Steam writes it, so Valve's client and Hearth both
/// see the game: installed (StateFlags 4) with its build and depots, or being installed/updated (1026 / 1030) with
/// byte counts that Hearth's Downloads page reads.</summary>
public sealed class AppManifest
{
    public const int UpdateRequired = 2, FullyInstalled = 4, UpdateStarted = 1024;
    public const int NewInstall = UpdateRequired | UpdateStarted;                    // 1026
    public const int Updating = FullyInstalled | UpdateRequired | UpdateStarted;     // 1030

    public uint AppId { get; set; }
    public string Name { get; set; } = "";
    public string InstallDir { get; set; } = "";
    public int StateFlags { get; set; } = NewInstall;
    public ulong LastOwner { get; set; }
    public uint BuildId { get; set; }
    public uint TargetBuildId { get; set; }
    public ulong SizeOnDisk { get; set; }
    public ulong BytesToDownload { get; set; }
    public ulong BytesDownloaded { get; set; }
    public long LastUpdated { get; set; }
    public long LastPlayed { get; set; }
    public int UpdateResult { get; set; }
    public string Language { get; set; } = "english";
    /// <summary>Depot id → (manifest id, size).</summary>
    public SortedDictionary<uint, (ulong Manifest, ulong Size, uint DlcAppId)> Depots { get; } = new();

    public bool Installed => (StateFlags & FullyInstalled) != 0;

    public static string Path(string library, uint appId) => System.IO.Path.Combine(library, "steamapps", $"appmanifest_{appId}.acf");

    static string Q(string s) => s.Replace("\\", "\\\\").Replace("\"", "\\\"");

    public string Text()
    {
        var sb = new StringBuilder();
        void L(string k, object v) => sb.Append("\t\"").Append(k).Append("\"\t\t\"").Append(Q(Convert.ToString(v, System.Globalization.CultureInfo.InvariantCulture) ?? "")).Append("\"\n");
        sb.Append("\"AppState\"\n{\n");
        L("appid", AppId); L("Universe", 1); L("LauncherPath", ""); L("name", Name); L("StateFlags", StateFlags); L("installdir", InstallDir);
        L("LastUpdated", LastUpdated); L("LastPlayed", LastPlayed); L("SizeOnDisk", SizeOnDisk); L("StagingSize", 0); L("buildid", BuildId);
        L("LastOwner", LastOwner); L("DownloadType", 1); L("UpdateResult", UpdateResult); L("BytesToDownload", BytesToDownload);
        L("BytesDownloaded", BytesDownloaded); L("BytesToStage", BytesToDownload); L("BytesStaged", BytesDownloaded); L("TargetBuildID", TargetBuildId);
        L("AutoUpdateBehavior", 0); L("AllowOtherDownloadsWhileRunning", 0); L("ScheduledAutoUpdate", 0);
        sb.Append("\t\"InstalledDepots\"\n\t{\n");
        foreach (var (id, d) in Depots)
        {
            sb.Append("\t\t\"").Append(id).Append("\"\n\t\t{\n");
            sb.Append("\t\t\t\"manifest\"\t\t\"").Append(d.Manifest).Append("\"\n");
            sb.Append("\t\t\t\"size\"\t\t\"").Append(d.Size).Append("\"\n");
            if (d.DlcAppId != 0) sb.Append("\t\t\t\"dlcappid\"\t\t\"").Append(d.DlcAppId).Append("\"\n");
            sb.Append("\t\t}\n");
        }
        sb.Append("\t}\n");
        sb.Append("\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"").Append(Q(Language)).Append("\"\n\t}\n");
        sb.Append("\t\"MountedConfig\"\n\t{\n\t\t\"language\"\t\t\"").Append(Q(Language)).Append("\"\n\t}\n");
        sb.Append("}\n");
        return sb.ToString();
    }

    public static AppManifest? Parse(string text)
    {
        var s = Vdf.Parse(text).Get("AppState");
        if (s == null || !uint.TryParse(s.Str("appid"), out var app)) return null;
        static ulong N(string? v) => ulong.TryParse(v, out var n) ? n : 0;
        var m = new AppManifest
        {
            AppId = app, Name = s.Str("name") ?? "", InstallDir = s.Str("installdir") ?? "", StateFlags = (int)N(s.Str("StateFlags")),
            LastOwner = N(s.Str("LastOwner")), BuildId = (uint)N(s.Str("buildid")), TargetBuildId = (uint)N(s.Str("TargetBuildID")),
            SizeOnDisk = N(s.Str("SizeOnDisk")), BytesToDownload = N(s.Str("BytesToDownload")), BytesDownloaded = N(s.Str("BytesDownloaded")),
            LastUpdated = (long)N(s.Str("LastUpdated")), LastPlayed = (long)N(s.Str("LastPlayed")), UpdateResult = (int)N(s.Str("UpdateResult")),
            Language = s.Str("UserConfig", "language") ?? "english",
        };
        foreach (var (k, d) in s.Get("InstalledDepots")?.Children ?? new())
            if (uint.TryParse(k, out var id)) m.Depots[id] = (N(d.Str("manifest")), N(d.Str("size")), (uint)N(d.Str("dlcappid")));
        return m;
    }

    public static AppManifest? Load(string library, uint appId)
    {
        try { var p = Path(library, appId); return File.Exists(p) ? Parse(File.ReadAllText(p)) : null; }
        catch (IOException) { return null; }
    }

    /// <summary>Writes the record atomically (a temp file moved over it), as Steam's readers may look at any time.</summary>
    public void Save(string library)
    {
        var p = Path(library, AppId);
        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(p)!);
        var tmp = $"{p}.{Environment.CurrentManagedThreadId}.hearth-tmp";
        File.WriteAllText(tmp, Text(), new UTF8Encoding(false));
        File.Move(tmp, p, overwrite: true);
    }

    /// <summary>A folder name Steam would accept for steamapps/common: one plain path part.</summary>
    public static bool SafeDirName(string d) => d.Length > 0 && d != "." && d != ".." && !d.Contains('/') && !d.Contains('\\') && !d.Contains('\0') && d.Trim() == d;

    public static string FolderName(string installDir, string name, uint appId)
    {
        if (SafeDirName(installDir)) return installDir;
        var s = new string(name.Where(c => !"/\\:*?\"<>|".Contains(c) && !char.IsControl(c)).ToArray()).Trim().Trim('.').Trim();
        return s.Length == 0 ? $"app_{appId}" : s;
    }
}
