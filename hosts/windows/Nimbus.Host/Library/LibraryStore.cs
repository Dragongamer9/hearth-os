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

public sealed class LibraryData
{
    public HashSet<string> Favorites { get; set; } = new();
    public HashSet<string> Hidden { get; set; } = new();
    public List<ManualGame> Manual { get; set; } = new();
    public Dictionary<string, GameStats> Stats { get; set; } = new();
    public bool SteamBackground { get; set; } = true;   // keep Steam running hidden (-silent)
    public bool AutoUpdateCheck { get; set; } = true;   // check GitHub for Nimbus updates
    public string ControllerMouse { get; set; } = "steam"; // right stick as mouse: off | steam | always
    public bool OverlayHoldView { get; set; }              // hold View for a second to open the in-game overlay
    public string StartMode { get; set; } = "game";        // as the shell: start in "game" (Ember) or "desktop" (Explorer)
    public string DesktopStyle { get; set; } = "mantel";   // Desktop Mode: "mantel" (Hearth's taskbar and launcher) or "windows"
    public string MantelWallpaper { get; set; } = "hearth-cabin"; // a Hearth wallpaper, or "" for your Windows wallpaper
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
            var tmp = _path + ".tmp";
            File.WriteAllText(tmp, JsonSerializer.Serialize(Data, Json));
            File.Move(tmp, _path, overwrite: true);
        }
    }

    public T Read<T>(Func<LibraryData, T> read)
    {
        lock (_lock) return read(Data);
    }
}
