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

/// <summary>What the UI shows for a store account. Expired: the store refused the saved sign-in (SignInExpiry);
/// the page shows "Sign-in expired" with Sign in again.</summary>
public sealed record AccountDto(
    string Store, bool SignedIn, string? Name, string? Avatar, int Owned, int Installed,
    long Fetched, bool Busy, string? Error, bool ToolReady, string? Note, bool Expired = false, long? ExpiredAt = null);

/// <summary>
/// Store sign-ins that stopped working: the store refused the saved sign-in (its refresh token was revoked or ran
/// out), as opposed to a passing problem (no connection, the store down, slow or rate-limiting), which never
/// counts. The same rules as hearth-core's stores::expiry. Held until the next sign-in, a working renewal or
/// signing out (in memory: after a restart the next refused renewal marks it again).
/// </summary>
public static class SignInExpiry
{
    private static readonly object Lock = new();
    private static readonly Dictionary<string, long> At = new();
    // OAuth error codes that mean the saved sign-in is no good anymore (RFC 6749 §5.2, and Microsoft's).
    private static readonly string[] Dead = { "invalid_grant", "invalid_token", "unauthorized_client", "interaction_required", "login_required", "consent_required" };

    /// <summary>Raised once per expiry (StoreAccounts tells the page through accounts.changed).</summary>
    public static event Action<string>? Marked;

    public static bool IsExpired(string store) { lock (Lock) return At.ContainsKey(store); }
    public static long? ExpiredAt(string store) { lock (Lock) return At.TryGetValue(store, out var t) ? t : null; }
    public static string Message(string store) =>
        $"Your {(store switch { "epic" => "Epic Games", "gog" => "GOG", "xbox" => "Microsoft (Xbox)", _ => "store" })} sign-in expired. Sign in again under Library › Stores.";

    public static bool Mark(string store)
    {
        bool added;
        lock (Lock) added = At.TryAdd(store, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        if (!added) return false;
        SystemBridge.Log($"accounts: {store} sign-in EXPIRED (the store refused the saved sign-in)");
        Marked?.Invoke(store);
        return true;
    }
    public static void Clear(string store) { lock (Lock) At.Remove(store); }

    /// <summary>A token endpoint's answer to renewing a sign-in. Only a clear refusal counts: 400/401/403 naming one
    /// of the codes above, or a bare 401. Server errors, rate limits, timeouts and a 400/403 without such a code
    /// (a captive portal or a proxy can send those) are temporary.</summary>
    public static bool TokenRefused(int status, string body)
    {
        string? err = null;
        try
        {
            using var d = JsonDocument.Parse(body);
            if (d.RootElement.Str("error") is { } e) err = e.Trim().ToLowerInvariant();
        }
        catch (JsonException) { }
        return status switch
        {
            400 or 401 or 403 when err is not null => Dead.Contains(err),
            401 => true,
            _ => false,
        };
    }
}

/// <summary>A store call failed because the store refused the saved sign-in.</summary>
public sealed class StoreExpiredException : InvalidOperationException
{
    public string Store { get; }
    public StoreExpiredException(string store) : base(SignInExpiry.Message(store)) => Store = store;
}

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
