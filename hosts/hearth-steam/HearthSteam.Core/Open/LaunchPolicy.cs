using System.Text.Json.Nodes;

namespace HearthSteam.Open;

/// <summary>How a game starts in the Open engine. Hearth starts a game itself (natively, or with Proton through umu,
/// as for Epic and GOG games) only when it's safe to: not VAC-secured, no Steamworks API in its files, its program
/// not wrapped by Steam's DRM, and a known way to start it. A first direct start that fails is remembered and that
/// game then always starts through Valve's own client. Everything else starts through Valve's client: licence checks
/// stay Valve's.</summary>
public static class LaunchPolicy
{
    static readonly string[] SteamworksFiles =
    {
        "steam_api.dll", "steam_api64.dll", "libsteam_api.so", "steamclient.dll", "steamclient64.dll", "steamclient.so",
        "libsteam_api.dylib", "steam_appid.txt",
    };

    /// <summary>Any Steamworks API library in the game's files (any folder).</summary>
    public static string? SteamworksFile(IEnumerable<string> depotPaths)
    {
        foreach (var p in depotPaths)
        {
            var name = p.Replace('\\', '/').Split('/').Last();
            if (SteamworksFiles.Any(s => s.Equals(name, StringComparison.OrdinalIgnoreCase))) return name;
        }
        return null;
    }

    /// <summary>Does a Windows program carry Steam's DRM wrapper (a PE section named ".bind")? Reads the section
    /// table from the PE header only.</summary>
    public static bool HasDrmWrapper(Stream exe)
    {
        try
        {
            var br = new BinaryReader(exe);
            if (exe.Length < 0x40) return false;
            exe.Position = 0;
            if (br.ReadUInt16() != 0x5A4D) return false;          // MZ
            exe.Position = 0x3C;
            var pe = br.ReadUInt32();
            if (pe + 24 > exe.Length) return false;
            exe.Position = pe;
            if (br.ReadUInt32() != 0x00004550) return false;      // PE\0\0
            exe.Position = pe + 6;
            int sections = br.ReadUInt16();
            exe.Position = pe + 20;
            int optSize = br.ReadUInt16();
            long table = pe + 24 + optSize;
            for (int i = 0; i < sections && i < 96; i++)
            {
                exe.Position = table + i * 40;
                var name = System.Text.Encoding.ASCII.GetString(br.ReadBytes(8)).TrimEnd('\0');
                if (name == ".bind") return true;
            }
        }
        catch (Exception e) when (e is IOException or EndOfStreamException or ArgumentException) { }
        return false;
    }

    public sealed record Decision(bool Direct, string Reason, LaunchEntry? Entry = null, string Os = "", bool FirstRun = false);

    /// <summary>The decision (pure). <paramref name="remembered"/>: "direct" (a direct start worked before), "client"
    /// (it failed, or the user chose Valve's client), or null.</summary>
    public static Decision Decide(AppInfo app, string os, IEnumerable<string> depotPaths, Func<string, bool> exeHasDrm, string? remembered)
    {
        if (app.Vac) return new(false, "VAC-secured: Valve's client");
        if (remembered == "client") return new(false, "starts through Valve's client (remembered)");
        if (SteamworksFile(depotPaths) is { } f) return new(false, $"uses Steamworks ({f}): Valve's client");
        var entry = app.LaunchFor(os);
        if (entry == null) return new(false, "no launch entry for " + os);
        if (Depot.SafePath(entry.Exe) == null) return new(false, "unsafe launch path");
        if (os == "windows" && entry.Exe.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) && exeHasDrm(entry.Exe)) return new(false, "Steam DRM wrapper: Valve's client");
        return new(true, remembered == "direct" ? "DRM-free (worked before)" : "DRM-free (first start is a test)", entry, os, remembered != "direct");
    }

    /// <summary>Was a first direct start a success? Running for 20 seconds (or a clean exit after that) counts.</summary>
    public static bool FirstRunOk(TimeSpan ranFor, bool stillRunning) => stillRunning || ranFor >= TimeSpan.FromSeconds(20);
}

/// <summary>Small per-user JSON state of the Open engine in ~/.local/state/hearth-steam (0700): remembered launch
/// ways, the install queue. Never secrets.</summary>
public sealed class StateFile
{
    readonly string path;
    readonly object gate = new();
    public JsonObject Data { get; private set; }

    public StateFile(string path)
    {
        this.path = path;
        try { Data = File.Exists(path) ? JsonNode.Parse(File.ReadAllText(path)) as JsonObject ?? new() : new(); }
        catch (Exception e) when (e is IOException or System.Text.Json.JsonException) { Data = new(); }
    }

    public void Update(Action<JsonObject> change)
    {
        lock (gate)
        {
            change(Data);
            Secrets.EnsurePrivateDir(Path.GetDirectoryName(path)!);
            var tmp = path + ".tmp";
            File.WriteAllText(tmp, Data.ToJsonString());
            File.Move(tmp, path, true);
        }
    }

    public string? Launch(uint app) => Data["launch"]?[app.ToString()]?.GetValue<string>();
    public void RememberLaunch(uint app, string how) => Update(d => { if (d["launch"] is not JsonObject o) d["launch"] = o = new JsonObject(); o[app.ToString()] = how; });
}
