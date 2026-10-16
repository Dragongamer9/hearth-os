using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace Nimbus.Host.Stores;

/// <summary>A game you own in a store account (installed or not), as cached on disk.</summary>
public sealed class OwnedGame
{
    public string Id { get; set; } = "";              // the store's own id (Epic app name, GOG product id, Xbox PFN)
    public string Title { get; set; } = "";
    public Dictionary<string, string> Art { get; set; } = new();
    public long Added { get; set; }
    public long LastPlayed { get; set; }
    /// <summary>Why it can't be installed from Nimbus, if it can't (e.g. "Installs through the EA app").</summary>
    public string? Note { get; set; }
}

public sealed class OwnedCache
{
    public long Fetched { get; set; }                  // unix ms
    public List<OwnedGame> Games { get; set; } = new();

    private static readonly JsonSerializerOptions Json = new() { WriteIndented = false };

    public static OwnedCache Load(string path)
    {
        try { return File.Exists(path) ? JsonSerializer.Deserialize<OwnedCache>(File.ReadAllText(path)) ?? new() : new(); }
        catch { return new(); }
    }

    public void Save(string path)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var tmp = path + ".tmp";
        File.WriteAllText(tmp, JsonSerializer.Serialize(this, Json));
        File.Move(tmp, path, overwrite: true);
    }
}

/// <summary>What the UI shows for a store account.</summary>
public sealed record AccountDto(
    string Store, bool SignedIn, string? Name, string? Avatar, int Owned, int Installed,
    long Fetched, bool Busy, string? Error, bool ToolReady, string? Note);

/// <summary>Small secrets (Xbox sign-in token) encrypted with Windows DPAPI for the current user.</summary>
public static class SecureStore
{
    private static string Dir => Path.Combine(HelperTools.DataDir, "accounts");

    public static void Save(string name, string value)
    {
        Directory.CreateDirectory(Dir);
        var data = ProtectedData.Protect(Encoding.UTF8.GetBytes(value), Encoding.UTF8.GetBytes("Nimbus." + name), DataProtectionScope.CurrentUser);
        var path = Path.Combine(Dir, name + ".bin");
        File.WriteAllBytes(path + ".tmp", data);
        File.Move(path + ".tmp", path, overwrite: true);
    }

    public static string? Load(string name)
    {
        try
        {
            var path = Path.Combine(Dir, name + ".bin");
            if (!File.Exists(path)) return null;
            return Encoding.UTF8.GetString(ProtectedData.Unprotect(File.ReadAllBytes(path), Encoding.UTF8.GetBytes("Nimbus." + name), DataProtectionScope.CurrentUser));
        }
        catch { return null; }
    }

    public static void Delete(string name)
    {
        try { File.Delete(Path.Combine(Dir, name + ".bin")); } catch { }
    }
}

internal static class J
{
    public static string? Str(this JsonElement e, string k) =>
        e.ValueKind == JsonValueKind.Object && e.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

    public static JsonElement Obj(this JsonElement e, string k) =>
        e.ValueKind == JsonValueKind.Object && e.TryGetProperty(k, out var v) ? v : default;

    public static bool True(this JsonElement e, string k) =>
        e.ValueKind == JsonValueKind.Object && e.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.True;

    public static IEnumerable<JsonElement> Arr(this JsonElement e, string k) =>
        e.ValueKind == JsonValueKind.Object && e.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Array ? v.EnumerateArray() : Enumerable.Empty<JsonElement>();

    public static long Ms(string? iso) =>
        iso is not null && DateTimeOffset.TryParse(iso, System.Globalization.CultureInfo.InvariantCulture,
            System.Globalization.DateTimeStyles.AssumeUniversal, out var d) ? d.ToUnixTimeMilliseconds() : 0;

    public static string Https(string url) => url.StartsWith("//") ? "https:" + url : url.StartsWith("http://") ? "https://" + url[7..] : url;
}
