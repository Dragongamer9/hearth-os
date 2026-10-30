namespace HearthSteam;

/// <summary>What hearth-steam reads from the user's normal Steam install: where it is, which client build it has,
/// its game libraries and its per-game compatibility tool (Proton) choices. Read only.</summary>
public static class SteamFiles
{
    public const string ManifestName = "steam_client_ubuntu12";

    /// <summary>The user's normal Steam folders, most likely first (native; the Flatpak's data is used for its
    /// libraries and Proton choices only, never to load a client from).</summary>
    public static IEnumerable<string> Candidates(string home) => new[]
    {
        Path.Combine(home, ".local/share/Steam"),
        Path.Combine(home, ".steam/debian-installation"),
        Path.Combine(home, ".steam/steam"),
        Path.Combine(home, ".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    };

    public static bool LooksLikeSteam(string dir) => Directory.Exists(Path.Combine(dir, "steamapps")) || File.Exists(Path.Combine(dir, "steam.sh"));

    public static string? UserSteam(string home) => Candidates(home).FirstOrDefault(LooksLikeSteam);

    /// <summary>The client build a Steam folder has (package/steam_client_ubuntu12.manifest's "version").</summary>
    public static long? ClientVersion(string steamDir)
    {
        var f = Path.Combine(steamDir, "package", ManifestName + ".manifest");
        try { return File.Exists(f) ? ManifestVersion(File.ReadAllText(f)) : null; } catch (IOException) { return null; }
    }

    public static long? ManifestVersion(string manifestText)
    {
        var v = Vdf.Parse(manifestText).Str("ubuntu12", "version");
        return long.TryParse(v, out var n) ? n : null;
    }

    /// <summary>Where to load steamclient.so from: the first folder whose client build is exactly the one
    /// OpenSteamworks was made for (its interfaces change with every Steam build). Null with why when none is.</summary>
    public static (string? dir, string reason) PickClient(long pinned, IEnumerable<(string dir, long? version)> found)
    {
        var list = found.ToList();
        foreach (var (dir, version) in list)
            if (version == pinned && File.Exists(Path.Combine(dir, "linux64", "steamclient.so"))) return (dir, "ok");
        if (list.Count == 0) return (null, "no Steam client found");
        return (null, $"Steam client build {string.Join(", ", list.Select(f => f.version?.ToString() ?? "unknown"))} isn't build {pinned} that the lightweight engine supports");
    }

    /// <summary>Steam libraries listed in a Steam folder's libraryfolders.vdf (the folder itself first).</summary>
    public static List<string> Libraries(string steamDir)
    {
        var list = new List<string> { steamDir };
        var f = Path.Combine(steamDir, "steamapps", "libraryfolders.vdf");
        if (!File.Exists(f)) f = Path.Combine(steamDir, "config", "libraryfolders.vdf");
        if (!File.Exists(f)) return list;
        var lf = Vdf.Parse(File.ReadAllText(f)).Get("libraryfolders");
        foreach (var e in lf?.Children.Values ?? Enumerable.Empty<VdfNode>())
        {
            var p = e.Str("path");
            if (!string.IsNullOrEmpty(p) && !list.Any(x => SamePath(x, p))) list.Add(p);
        }
        return list;
    }

    /// <summary>The user's compatibility tool per app from config/config.vdf (CompatToolMapping). App 0 is the
    /// default for all games ("Enable Steam Play for all other titles").</summary>
    public static Dictionary<uint, string> CompatMapping(string configVdfText)
    {
        var map = new Dictionary<uint, string>();
        var m = Vdf.Parse(configVdfText).Get("InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping");
        foreach (var (k, v) in m?.Children ?? new())
        {
            var name = v.Str("name");
            if (uint.TryParse(k, out var app) && !string.IsNullOrEmpty(name)) map[app] = name;
        }
        return map;
    }

    static bool SamePath(string a, string b) => string.Equals(a.TrimEnd('/'), b.TrimEnd('/'), StringComparison.Ordinal);

    /// <summary>Is the normal Steam client running? Then the lightweight engine stays off: two Steam clients on
    /// the same libraries at once can corrupt them.</summary>
    public static bool ClassicRunning(string procDir = "/proc")
    {
        try
        {
            foreach (var d in Directory.EnumerateDirectories(procDir))
            {
                var name = Path.GetFileName(d);
                if (name.Length == 0 || !name.All(char.IsAsciiDigit) || name == Environment.ProcessId.ToString()) continue;
                string comm;
                try { comm = File.ReadAllText(Path.Combine(d, "comm")).Trim(); } catch (Exception) { continue; }
                if (comm is "steam" or "steamwebhelper") return true;
                byte[] cmd;
                try { cmd = File.ReadAllBytes(Path.Combine(d, "cmdline")); } catch (Exception) { continue; }
                if (IsClassicCmdline(cmd)) return true;
            }
        }
        catch (Exception) { }
        return false;
    }

    public static bool IsClassicCmdline(byte[] cmd)
    {
        var first = System.Text.Encoding.UTF8.GetString(cmd).Split('\0').FirstOrDefault() ?? "";
        return first.EndsWith("ubuntu12_32/steam") || first.EndsWith("/steamwebhelper");
    }
}
