using System.Text.Json;

namespace Nimbus.Host.Library;

public sealed class ManualGame
{
    public string Id { get; set; } = "";
    public string Title { get; set; } = "";
    public string Path { get; set; } = "";
    public string? Args { get; set; }
    public long Added { get; set; }
}

public sealed class GameStats
{
    public long LastPlayed { get; set; }   // unix ms
    public int Minutes { get; set; }
}

/// <summary>How you look in Nimbus: name, picture, colors.</summary>
public sealed class ProfileData
{
    public string? Name { get; set; }                  // null: your Windows name
    public string? Status { get; set; }                // short line under your name
    public string Avatar { get; set; } = "initials";   // initials | preset:N | custom | steam | xbox | gog
    public string Accent { get; set; } = "#3fd0b8";
    public string Background { get; set; } = "default"; // default | preset:N | custom
    public long ImageStamp { get; set; }               // changes when a custom picture changes (cache busting)
}

/// <summary>A collection: your own group of games, shown as a Library tab (like hearth-core's collections.rs).</summary>
public sealed class CollectionData
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public List<string> Games { get; set; } = new();
}

/// <summary>Per-game settings (game page › Game settings), the Windows part of hearth-core's game_options.rs.</summary>
public sealed class GameOptionsData
{
    public int Tdp { get; set; }                       // power limit in watts while it plays (0 = the usual)
    public bool CloudSaves { get; set; } = true;       // Epic / GOG cloud saves for games Ember installed
}

public sealed class LibraryData
{
    public HashSet<string> Favorites { get; set; } = new();
    public List<CollectionData> Collections { get; set; } = new();
    public Dictionary<string, GameOptionsData> GameOptions { get; set; } = new();
    public int TdpWatts { get; set; }                   // the power limit Ember last set (0 = never set one)
    public HashSet<string> Hidden { get; set; } = new();
    public List<ManualGame> Manual { get; set; } = new();
    public Dictionary<string, GameStats> Stats { get; set; } = new();
    public bool SteamBackground { get; set; } = true;   // keep Steam running hidden (-silent), when SteamOnDemand is off
    public bool SteamOnDemand { get; set; } = true;     // "Start Steam only when needed" (SteamControl): starts hidden when needed, closes when idle
    public string? SteamLibrary { get; set; }           // the Steam library new Steam games install to (null: SteamControl.ChooseLibrary)
    public bool AutoUpdateCheck { get; set; } = true;   // check GitHub for Nimbus updates
    public bool QuickResume { get; set; } = true;       // starting another game puts the running one aside (frozen)
    public string ControllerMouse { get; set; } = "steam"; // right stick as mouse: off | steam | always
    public bool OverlayHoldView { get; set; }              // hold View for a second to open the in-game overlay
    public string? OverlayQaStyle { get; set; }            // Quick Access style: "dock" (along the top) or "side" (classic right-hand panel); null until chosen
    public string StartMode { get; set; } = "game";        // as the shell: start in "game" (Ember) or "desktop" (Explorer)
    public string DesktopStyle { get; set; } = "mantel";   // Desktop Mode: "mantel" (Hearth's taskbar and launcher) or "windows"
    public string MantelWallpaper { get; set; } = "anim:fireside"; // the Hearth fireside (moving), a Hearth picture, "custom:<file>", or "" for your Windows wallpaper
    public bool MantelNightLight { get; set; }             // Hearth Settings › Appearance › Night light (warmer screen from sunset to sunrise)
    public bool MantelReduceMotion { get; set; }           // Hearth Settings › Appearance › Reduce motion (still wallpaper)
    public double PointerSpeed { get; set; } = 1.0;        // Hearth Settings › Controller › Pointer speed (0.4 – 2)
    public string? InstallRoot { get; set; }            // where Nimbus installs Epic and GOG games
    public Dictionary<string, string> GogInstalls { get; set; } = new();   // GOG product id → folder
    public ProfileData Profile { get; set; } = new();
    public Dictionary<string, bool> Sources { get; set; } = new()
    {
        ["steam"] = true, ["epic"] = true, ["gog"] = true, ["xbox"] = true,
    };
}

/// <summary>Nimbus's own per-user data: favorites, hidden games, manual games, play stats, enabled sources.</summary>
public sealed class LibraryStore
{
    private static readonly JsonSerializerOptions Json = new() { WriteIndented = true };
    private readonly string _path;
    private readonly object _lock = new();

    public LibraryData Data { get; }

    public LibraryStore()
    {
        var dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus");
        Directory.CreateDirectory(dir);
        _path = Path.Combine(dir, "library.json");
        Data = Load() ?? new LibraryData();
    }

    private LibraryData? Load()
    {
        try { return File.Exists(_path) ? JsonSerializer.Deserialize<LibraryData>(File.ReadAllText(_path)) : null; }
        catch { return null; } // corrupt file: start fresh rather than crash the shell
    }

    /// <summary>Run a change under the lock and save it.</summary>
    public void Update(Action<LibraryData> change)
    {
        lock (_lock)
        {
            change(Data);
            lock (SaveGate)
            {
                if (_paused) return;   // a profile switch is under way: library.json is the next person's now
                var tmp = _path + ".tmp";
                File.WriteAllText(tmp, JsonSerializer.Serialize(Data, Json));
                File.Move(tmp, _path, overwrite: true);
            }
        }
    }

    // A profile switch (Profiles.SwitchTo) moves library.json away and brings the next person's; Ember restarts after
    // it. A save in between (a game ending, a store list arriving) would write this person's library over theirs, so
    // saves stop for the switch. When this returns, no save is half done.
    private static readonly object SaveGate = new();
    private static bool _paused;
    public static void PauseSaves() { lock (SaveGate) _paused = true; }
    /// <summary>The switch didn't happen: saving carries on.</summary>
    public static void ResumeSaves() { lock (SaveGate) _paused = false; }

    public T Read<T>(Func<LibraryData, T> read)
    {
        lock (_lock) return read(Data);
    }
}
