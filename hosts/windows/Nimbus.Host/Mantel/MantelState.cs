using System.Text.Json;

namespace Nimbus.Host.Mantel;

/// <summary>
/// What Mantel on Windows remembers (%LOCALAPPDATA%\Nimbus\mantel.json), like hearth-desk's desk.json on Hearth OS:
/// apps pinned to the taskbar, the desktop icons' places and whether they show, Do Not Disturb, the 12/24-hour
/// clock; plus Windows-only things (the wallpaper and taskbar setting from before Mantel). Shared by MantelHost
/// (the windows) and DeskBridge (the bridge calls); every change raises <see cref="Changed"/>.
/// </summary>
internal static class MantelState
{
    public const int MaxPins = 40;
    private static readonly object Lock = new();
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);
    private static readonly string File_ = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Nimbus", "mantel.json");

    /// <summary>Something in the file changed (pins, a switch): the pages are told again. Raised on the caller's thread.</summary>
    public static event Action? Changed;

    public sealed class Data
    {
        public List<Pin> Pins { get; set; } = new();
        public string? WindowsWallpaper { get; set; }
        /// <summary>Windows' taskbar setting before Mantel set it to auto-hide (kept on disk so a crash can't
        /// leave it changed: the next start puts it back).</summary>
        public int? TaskbarState { get; set; }
        public bool DesktopIcons { get; set; } = true;
        public bool Dnd { get; set; }
        public bool Clock24 { get; set; }
        /// <summary>Desktop icon places: file name → [column, row].</summary>
        public Dictionary<string, int[]> Icons { get; set; } = new();
    }

    /// <summary>A pinned app: a Start menu app id (shell:AppsFolder\…), an .exe path, or one of Hearth's own (hearth:files).</summary>
    public sealed class Pin
    {
        public string Id { get; set; } = "";
        public string Name { get; set; } = "";
        public Pin() { }
        public Pin(string id, string name) { Id = id; Name = name; }
    }

    public static Data Load()
    {
        lock (Lock)
        {
            try
            {
                var d = JsonSerializer.Deserialize<Data>(File.ReadAllText(File_), Json) ?? new Data();
                d.Pins ??= new();
                d.Pins.RemoveAll(p => p is null || string.IsNullOrWhiteSpace(p.Id));
                d.Icons ??= new();
                return d;
            }
            catch { return new Data(); }
        }
    }

    public static void Save(Data d, bool notify = true)
    {
        lock (Lock)
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(File_)!);
                var tmp = File_ + ".tmp";
                File.WriteAllText(tmp, JsonSerializer.Serialize(d, Json));
                File.Move(tmp, File_, true);
            }
            catch (Exception ex) { SystemBridge.Log("mantel: save: " + ex.Message); }
        }
        if (notify) Changed?.Invoke();
    }

    /// <summary>Changes the saved state in one go (load, change, save).</summary>
    public static Data Update(Action<Data> change, bool notify = true)
    {
        Data d;
        lock (Lock) { d = Load(); change(d); Save(d, false); }
        if (notify) Changed?.Invoke();
        return d;
    }

    public static bool IsPinned(string id) => Load().Pins.Any(p => string.Equals(p.Id, id, StringComparison.OrdinalIgnoreCase));

    /// <summary>Pins (on) or unpins an app. Throws with a message for the page when the taskbar is full.</summary>
    public static void SetPin(string id, string name, bool on)
    {
        if (string.IsNullOrWhiteSpace(id) || id.Length > 1024) throw new ArgumentException("That app can't be pinned");
        Update(d =>
        {
            bool had = d.Pins.Any(p => string.Equals(p.Id, id, StringComparison.OrdinalIgnoreCase));
            if (!on) { d.Pins.RemoveAll(p => string.Equals(p.Id, id, StringComparison.OrdinalIgnoreCase)); return; }
            if (had) return;
            if (d.Pins.Count >= MaxPins) throw new InvalidOperationException("The taskbar is full. Unpin an app first");
            d.Pins.Add(new Pin(id, string.IsNullOrWhiteSpace(name) ? id : name));
        });
    }
}
