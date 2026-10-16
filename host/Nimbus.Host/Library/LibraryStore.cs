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

public sealed class LibraryData
{
    public HashSet<string> Favorites { get; set; } = new();
    public HashSet<string> Hidden { get; set; } = new();
    public List<ManualGame> Manual { get; set; } = new();
    public Dictionary<string, GameStats> Stats { get; set; } = new();
    public bool SteamBackground { get; set; } = true;   // keep Steam running hidden (-silent)
    public bool AutoUpdateCheck { get; set; } = true;   // check GitHub for Nimbus updates
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
