namespace HearthSteam.Open;

// The Open engine's view of Steam's app info (PICS product info, as KeyValues). Pure: tested with text KeyValues, fed
// with the converted SteamKit2 KeyValue in the service.

public sealed record DepotInfo(uint Id, ulong Manifest, ulong Size, string OsList, string OsArch, string Language,
    uint DlcAppId, bool SharedInstall, uint DepotFromApp, bool LowViolence, bool Encrypted);

public sealed record LaunchEntry(string Exe, string Args, string WorkingDir, string OsList, string OsArch, string Type, string BetaKey);

public sealed class AppInfo
{
    public uint AppId { get; init; }
    public string Name { get; init; } = "";
    public string Type { get; init; } = "";
    public string InstallDir { get; init; } = "";
    public uint BuildId { get; init; }
    public string OsList { get; init; } = "";
    /// <summary>VAC-secured (Steam's store category 8).</summary>
    public bool Vac { get; init; }
    public List<DepotInfo> Depots { get; init; } = new();
    public List<LaunchEntry> Launch { get; init; } = new();

    public bool IsGame => Type.Equals("game", StringComparison.OrdinalIgnoreCase) || Type.Equals("demo", StringComparison.OrdinalIgnoreCase);

    static uint U(string? s) => uint.TryParse(s, out var n) ? n : 0;
    static ulong UL(string? s) => ulong.TryParse(s, out var n) ? n : 0;

    /// <summary>From the app's KeyValues (the "appinfo" node: common, config, depots…), for one branch.</summary>
    public static AppInfo Parse(uint appId, VdfNode app, string branch = "public")
    {
        var depots = new List<DepotInfo>();
        var dn = app.Get("depots");
        foreach (var (k, d) in dn?.Children ?? new())
        {
            if (!uint.TryParse(k, out var id) || d.Children.Count == 0) continue;
            // Manifests: "manifests" { "public" { "gid" "…" "size" "…" } } (or the older "public" "gid").
            var m = d.Get("manifests", branch);
            ulong gid = m?.Value != null ? UL(m.Value) : UL(m?.Str("gid"));
            ulong size = UL(m?.Str("size"));
            bool enc = gid == 0 && d.Get("encryptedmanifests", branch) != null;
            depots.Add(new DepotInfo(id, gid, size, d.Str("config", "oslist") ?? "", d.Str("config", "osarch") ?? "",
                d.Str("config", "language") ?? "", U(d.Str("dlcappid")), d.Str("sharedinstall") == "1", U(d.Str("depotfromapp")),
                d.Str("config", "lowviolence") == "1", enc));
        }
        var launch = new List<LaunchEntry>();
        foreach (var (_, l) in app.Get("config", "launch")?.Children ?? new())
        {
            var exe = l.Str("executable");
            if (string.IsNullOrWhiteSpace(exe)) continue;
            launch.Add(new LaunchEntry(exe, l.Str("arguments") ?? "", l.Str("workingdir") ?? "", l.Str("config", "oslist") ?? "",
                l.Str("config", "osarch") ?? "", l.Str("type") ?? "", l.Str("config", "betakey") ?? ""));
        }
        return new AppInfo
        {
            AppId = appId,
            Name = app.Str("common", "name") ?? "",
            Type = app.Str("common", "type") ?? "",
            InstallDir = app.Str("config", "installdir") ?? "",
            BuildId = U(dn?.Str("branches", branch, "buildid")),
            OsList = app.Str("common", "oslist") ?? "",
            Vac = app.Str("common", "category", "category_8") == "1",
            Depots = depots,
            Launch = launch,
        };
    }

    static bool Lists(string list, string os) => list.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries).Any(x => x.Equals(os, StringComparison.OrdinalIgnoreCase));

    /// <summary>Which build to install: the Linux one when the game has one (and launch entries for it) and the user
    /// didn't pick Proton for it in Steam; otherwise the Windows one, run with Proton.</summary>
    public string ChooseOs(bool userPickedProton)
    {
        bool linuxDepot = Depots.Any(d => Lists(d.OsList, "linux"));
        bool linuxLaunch = Launch.Any(l => Lists(l.OsList, "linux"));
        return !userPickedProton && (Lists(OsList, "linux") || linuxDepot) && linuxLaunch ? "linux" : "windows";
    }

    /// <summary>The depots to download for an OS and language, as Steam would: the OS's (or OS-less) depots, 64-bit
    /// before 32-bit, the language's (or language-less), DLC depots only when owned, no low-violence depots and no
    /// shared redistributables (another app's depots: Proton and umu bring their own).</summary>
    public List<DepotInfo> PickDepots(string os, string language, Func<uint, bool> ownsApp)
    {
        var ok = Depots.Where(d => d.Manifest != 0 && !d.SharedInstall && d.DepotFromApp == 0 && !d.LowViolence
            && (d.OsList.Length == 0 || Lists(d.OsList, os))
            && (d.Language.Length == 0 || d.Language.Equals(language, StringComparison.OrdinalIgnoreCase))
            && (d.DlcAppId == 0 || ownsApp(d.DlcAppId))).ToList();
        bool has64 = ok.Any(d => d.OsArch == "64");
        return ok.Where(d => d.OsArch.Length == 0 || d.OsArch == "64" || (!has64 && d.OsArch == "32")).ToList();
    }

    /// <summary>The default launch entry for an OS (no beta key, the plain "default"/empty type first).</summary>
    public LaunchEntry? LaunchFor(string os)
    {
        var c = Launch.Where(l => l.BetaKey.Length == 0 && (l.OsList.Length == 0 || Lists(l.OsList, os))
                                  && (l.OsArch.Length == 0 || l.OsArch == "64" || os == "windows")).ToList();
        return c.FirstOrDefault(l => l.Type is "" or "default" && Lists(l.OsList, os))
               ?? c.FirstOrDefault(l => l.Type is "" or "default")
               ?? c.FirstOrDefault(l => l.Type != "none" && l.Type != "server" && l.Type != "editor");
    }
}

/// <summary>A licence package's app list (PICS package KeyValues: "appids" { "0" "220" … }).</summary>
public static class PackageInfo
{
    public static List<uint> Apps(VdfNode package) =>
        (package.Get("appids")?.Children.Values ?? Enumerable.Empty<VdfNode>()).Select(v => uint.TryParse(v.Value, out var a) ? a : 0).Where(a => a > 0).Distinct().ToList();
}
