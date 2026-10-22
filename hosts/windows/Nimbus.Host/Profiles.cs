using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace Nimbus.Host;

/// <summary>
/// Profiles on Ember, like Hearth's: several people on one PC, each with their own Steam account, Epic/GOG/Xbox
/// sign-ins, favorites and play time; installed games are shared. A person's own files live in Ember's data
/// folder while their profile is in use and are put away in profiles\&lt;id&gt;\ otherwise (switching restarts Ember).
/// Parental controls: a household PIN, minutes of play a day, allowed games. Port of hosts/hearth-core/src/profiles.rs
/// (same profiles.json shape; the PIN is PBKDF2 here, since Ember's data never meets Hearth's).
/// </summary>
public sealed class Profiles
{
    private static readonly string[] Own = { "library.json", "profile", "accounts", @"legendary\user.json", @"gog\auth.json" };
    private static readonly string[] Caches = { @"cache\epic-owned.json", @"cache\gog-owned.json", @"cache\xbox-titles.json" };
    private static readonly string[] Colors = { "#ff8a3d", "#3fd0b8", "#5aa9ff", "#b57bff", "#ffd166", "#e5483b", "#7ad15a" };

    private readonly string _dir;
    private JsonObject _data;
    private DateTime _pinUntil = DateTime.MinValue;
    private string PathOf => Path.Combine(_dir, "profiles.json");
    private string Stash(string id) => Path.Combine(_dir, "profiles", id);

    public Profiles(string dir)
    {
        _dir = dir;
        Directory.CreateDirectory(dir);
        JsonObject? d = null;
        try { if (File.Exists(PathOf)) d = JsonNode.Parse(File.ReadAllText(PathOf)) as JsonObject; } catch { }
        if (d?["list"] is not JsonArray l || l.Count == 0)
            d = new JsonObject { ["active"] = "main", ["askAtStart"] = false, ["list"] = new JsonArray(new JsonObject { ["id"] = "main", ["name"] = null, ["color"] = Colors[0], ["steam"] = null }) };
        _data = d;
        if (!List.Any(p => (string?)p["id"] == Active)) _data["active"] = (string?)List[0]["id"];
    }

    private JsonArray ListArr => (JsonArray)_data["list"]!;
    private List<JsonObject> List => ListArr.OfType<JsonObject>().ToList();
    public string Active => (string?)_data["active"] ?? "main";
    public string? NameOf(string id) => (string?)Find(id)?["name"];
    private JsonObject? Find(string id) => List.FirstOrDefault(p => (string?)p["id"] == id);
    private void Save() { var t = PathOf + ".tmp"; File.WriteAllText(t, _data.ToJsonString(new JsonSerializerOptions { WriteIndented = true })); File.Move(t, PathOf, true); }

    // ---------------------------------------------------------------- PIN
    public bool HasPin => _data["pin"]?["hash"] is not null;
    public bool Unlocked => HasPin && DateTime.UtcNow < _pinUntil;
    private static string Hash(string pin, byte[] salt) => Convert.ToHexString(Rfc2898DeriveBytes.Pbkdf2(pin, salt, 120_000, HashAlgorithmName.SHA256, 32)).ToLowerInvariant();
    public bool CheckPin(string pin)
    {
        if (!HasPin) return true;
        var salt = Convert.FromHexString((string?)_data["pin"]!["salt"] ?? "");
        return CryptographicOperations.FixedTimeEquals(System.Text.Encoding.ASCII.GetBytes(Hash(pin, salt)), System.Text.Encoding.ASCII.GetBytes((string?)_data["pin"]!["hash"] ?? ""));
    }
    /// <summary>The PIN unlocks managing profiles for a few minutes.</summary>
    public void NeedPin() { if (HasPin && DateTime.UtcNow >= _pinUntil) throw new InvalidOperationException("Enter the PIN first"); }
    public bool Unlock(string pin)
    {
        if (!CheckPin(pin)) { SystemBridge.Log("profiles: wrong PIN"); throw new InvalidOperationException("That PIN isn't right"); }
        _pinUntil = DateTime.UtcNow.AddMinutes(5);
        return true;
    }
    public void SetPin(string pin)
    {
        NeedPin();
        if (pin.Length == 0)
        {
            _data.Remove("pin");
            foreach (var p in List) p.Remove("limits");
            Save(); return;
        }
        if (pin.Length is < 4 or > 8 || !pin.All(char.IsAsciiDigit)) throw new InvalidOperationException("The PIN is 4 to 8 numbers");
        var salt = RandomNumberGenerator.GetBytes(16);
        _data["pin"] = new JsonObject { ["salt"] = Convert.ToHexString(salt).ToLowerInvariant(), ["hash"] = Hash(pin, salt) };
        _pinUntil = DateTime.UtcNow.AddMinutes(5);
        Save();
    }

    // ---------------------------------------------------------------- the list
    private static bool IsColor(string? c) => c is { Length: 7 } && c[0] == '#' && c[1..].All(Uri.IsHexDigit);

    public JsonObject Create(string name, string? color)
    {
        NeedPin();
        var clean = new string(name.Trim().Take(32).ToArray());
        if (clean.Length == 0) throw new InvalidOperationException("Give the profile a name");
        int n = List.Count;
        if (n >= 8) throw new InvalidOperationException("Ember has room for 8 profiles");
        var id = "p" + DateTimeOffset.UtcNow.ToUnixTimeMilliseconds().ToString("x");
        var p = new JsonObject { ["id"] = id, ["name"] = clean, ["color"] = IsColor(color) ? color : Colors[n % Colors.Length], ["steam"] = null };
        ListArr.Add(p);
        // Its own library.json from the start: favorites and play time are theirs (your own added games come along on switching).
        Directory.CreateDirectory(Stash(id));
        Save();
        return (JsonObject)p.DeepClone();
    }
    public JsonObject Update(string id, string? name, string? color, string? steam, bool setSteam)
    {
        var p = Find(id) ?? throw new InvalidOperationException("No such profile");
        // Your own name and color are yours to change; anything else needs the PIN.
        if (!(id == Active && !setSteam)) NeedPin();
        if (name is not null) { var t = new string(name.Trim().Take(32).ToArray()); if (t.Length > 0) p["name"] = t; }
        if (IsColor(color)) p["color"] = color;
        if (setSteam) p["steam"] = string.IsNullOrEmpty(steam) ? null : new string(steam.Take(64).ToArray());
        Save();
        return (JsonObject)p.DeepClone();
    }
    public void Remove(string id)
    {
        NeedPin();
        if (id == Active) throw new InvalidOperationException("Switch to another profile before removing this one");
        if (List.Count <= 1) throw new InvalidOperationException("There has to be one profile");
        var p = Find(id); if (p is not null) ListArr.Remove(p);
        try { Directory.Delete(Stash(id), true); } catch { }
        Save();
    }
    public void SetAsk(bool on) { NeedPin(); _data["askAtStart"] = on; Save(); }

    // ---------------------------------------------------------------- parental limits
    public JsonObject Limits(string id)
    {
        var l = Find(id)?["limits"] as JsonObject;
        return new JsonObject { ["minutesPerDay"] = (int?)l?["minutesPerDay"] ?? 0, ["allowed"] = l?["allowed"] is JsonArray a ? a.DeepClone() : null };
    }
    public bool Restricted(string id) { var l = Limits(id); return HasPin && ((int?)l["minutesPerDay"] > 0 || l["allowed"] is JsonArray); }
    public JsonObject SetLimits(string id, JsonElement patch)
    {
        NeedPin();
        var p = Find(id) ?? throw new InvalidOperationException("No such profile");
        if (!HasPin) throw new InvalidOperationException("Set a PIN first, so limits can only be changed by you");
        var l = Limits(id);
        if (patch.TryGetProperty("minutesPerDay", out var m)) l["minutesPerDay"] = Math.Clamp((int)Math.Round(m.ValueKind == JsonValueKind.Number ? m.GetDouble() : 0), 0, 1440);
        if (patch.TryGetProperty("allowed", out var al))
            l["allowed"] = al.ValueKind == JsonValueKind.Array ? new JsonArray(al.EnumerateArray().Select(x => x.ToString()).Distinct().Take(2000).Select(x => (JsonNode)JsonValue.Create(x)!).ToArray()) : null;
        p["limits"] = l.DeepClone();
        Save();
        return l;
    }
    private static string Today => DateTime.Now.ToString("yyyy-MM-dd");
    public double PlayedToday(string id) => _data["today"]?[id] is JsonObject t && (string?)t["date"] == Today ? (double?)t["minutes"] ?? 0 : 0;
    public void AddPlayed(string id, double minutes)
    {
        if (minutes <= 0) return;
        var prev = PlayedToday(id);
        if (_data["today"] is not JsonObject) _data["today"] = new JsonObject();
        _data["today"]![id] = new JsonObject { ["date"] = Today, ["minutes"] = Math.Round((prev + minutes) * 10) / 10 };
        Save();
    }
    /// <summary>Can the active profile start this game now? (ok, reason, minutes left or null)</summary>
    public (bool Ok, string? Reason, int? Left) CanPlay(string? game, double sessionMinutes)
    {
        var id = Active;
        if (!Restricted(id)) return (true, null, null);
        var l = Limits(id);
        if (l["allowed"] is JsonArray list && game is not null && !list.Any(x => (string?)x == game))
            return (false, "This game needs a grown-up to allow it (Settings › Profiles › Limits).", null);
        var perDay = (int?)l["minutesPerDay"] ?? 0;
        if (perDay == 0) return (true, null, null);
        var left = perDay - PlayedToday(id) - sessionMinutes;
        return left > 0 ? (true, null, (int)Math.Floor(left)) : (false, "That's all the play time for today. More tomorrow!", 0);
    }

    /// <summary>What Settings › Profiles and "Who's playing?" show.</summary>
    public object Info(bool picked) => new
    {
        active = Active,
        askAtStart = (bool?)_data["askAtStart"] ?? false,
        picked,
        hasPin = HasPin,
        unlocked = Unlocked,
        restricted = Restricted(Active),
        left = CanPlay(null, 0).Left,
        list = List.Select(p =>
        {
            var o = (JsonObject)p.DeepClone();
            var id = (string?)p["id"] ?? "";
            o["current"] = id == Active; o["limits"] = Limits(id); o["playedToday"] = Math.Round(PlayedToday(id));
            return o;
        }).ToList(),
        steamAccounts = SteamAccounts(),
    };

    // ---------------------------------------------------------------- switching
    /// <summary>Moves a file or folder, trying again for a moment if something holds it. Returns false when there's
    /// nothing to move.</summary>
    private static bool MovePath(string src, string dst)
    {
        if (!File.Exists(src) && !Directory.Exists(src)) return false;
        Directory.CreateDirectory(Path.GetDirectoryName(dst)!);
        try { if (Directory.Exists(dst)) Directory.Delete(dst, true); else if (File.Exists(dst)) File.Delete(dst); } catch { }
        for (int i = 0; ; i++)
        {
            try { if (Directory.Exists(src)) Directory.Move(src, dst); else File.Move(src, dst); return true; }
            catch (IOException) when (i < 5) { Thread.Sleep(300); }
            catch (UnauthorizedAccessException) when (i < 5) { Thread.Sleep(300); }
        }
    }

    /// <summary>The first file under this path that another program has open (so it can't be moved), or null.</summary>
    private static string? LockedFile(string path)
    {
        IEnumerable<string> files = File.Exists(path) ? new[] { path }
            : Directory.Exists(path) ? Directory.EnumerateFiles(path, "*", SearchOption.AllDirectories) : Array.Empty<string>();
        foreach (var f in files)
        {
            try { using var _ = new FileStream(f, FileMode.Open, FileAccess.Read, FileShare.None); }
            catch (FileNotFoundException) { }
            catch (Exception) { return f; }
        }
        return null;
    }

    /// <summary>Puts the current person's files away and brings out the other's, with Steam signed in as theirs.
    /// Ember restarts afterwards. All or nothing: if a file can't be moved, everything moved so far goes back and the
    /// profile stays as it was.</summary>
    public JsonObject SwitchTo(string id, bool gameRunning)
    {
        var to = Find(id) ?? throw new InvalidOperationException("No such profile");
        if (gameRunning) throw new InvalidOperationException("Close the game first");
        if (Restricted(Active) && id != Active) NeedPin();
        var from = Active;
        if (id == from) return (JsonObject)to.DeepClone();
        SystemBridge.Log($"profiles: switching to {(string?)to["name"] ?? id}");
        var away = Stash(from);
        var back = Stash(id);
        var lib = Path.Combine(_dir, "library.json");
        // From here until Ember restarts, its library isn't saved over the next person's (LibraryStore.PauseSaves).
        Library.LibraryStore.PauseSaves();
        var moved = new List<(string From, string To)>();
        try
        {
            // Nothing is moved while another program holds one of the files.
            foreach (var rel in Own)
                if ((LockedFile(Path.Combine(_dir, rel)) ?? LockedFile(Path.Combine(back, rel))) is { } busy)
                    throw new InvalidOperationException($"Another program is using {Path.GetFileName(busy)}. Close it and try again.");
            StopSteam();
            SetSteamAccount((string?)to["steam"] ?? "");
            // Your own added games come along to every profile.
            JsonArray manual = new();
            try { manual = (JsonNode.Parse(File.ReadAllText(lib))?["Manual"] as JsonArray)?.DeepClone() as JsonArray ?? new(); } catch { }
            try { Directory.Delete(away, true); } catch { }
            Directory.CreateDirectory(away);
            foreach (var rel in Own)
                if (MovePath(Path.Combine(_dir, rel), Path.Combine(away, rel))) moved.Add((Path.Combine(_dir, rel), Path.Combine(away, rel)));
            foreach (var rel in Own)
                if (MovePath(Path.Combine(back, rel), Path.Combine(_dir, rel))) moved.Add((Path.Combine(back, rel), Path.Combine(_dir, rel)));
            JsonObject d;
            try { d = JsonNode.Parse(File.ReadAllText(lib)) as JsonObject ?? new(); } catch { d = new(); }
            var mine = d["Manual"] as JsonArray ?? new JsonArray();
            foreach (var g in manual) if (!mine.Any(m => (string?)m?["Id"] == (string?)g?["Id"])) mine.Add(g!.DeepClone());
            d["Manual"] = mine.DeepClone();
            if (d["Profile"] is null) d["Profile"] = new JsonObject { ["Name"] = (string?)to["name"], ["Accent"] = (string?)to["color"] ?? "#3fd0b8" };
            File.WriteAllText(lib, d.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
            _data["active"] = id;
            Save();
        }
        catch (Exception ex)
        {
            SystemBridge.Log($"profiles: switch to {id} FAILED ({ex.Message}); putting back {moved.Count} moved item(s)");
            _data["active"] = from;
            // Back in the opposite order: the next person's files to their stash first, then the current person's.
            for (int i = moved.Count - 1; i >= 0; i--)
            {
                try { MovePath(moved[i].To, moved[i].From); }
                catch (Exception undo) { SystemBridge.Log($"profiles: couldn't put back {moved[i].From}: {undo.Message}"); }
            }
            try { SetSteamAccount((string?)Find(from)?["steam"] ?? ""); } catch { }
            try { Save(); } catch { }
            Library.LibraryStore.ResumeSaves();
            throw ex is InvalidOperationException ? ex : new InvalidOperationException("Couldn't switch profiles: " + ex.Message);
        }
        try { Directory.Delete(back, true); } catch { }
        foreach (var rel in Caches) try { File.Delete(Path.Combine(_dir, rel)); } catch { }
        SystemBridge.Log($"profiles: switched from {from} to {id} ({moved.Count} item(s) moved)");
        return (JsonObject)to.DeepClone();
    }

    // ---------------------------------------------------------------- Steam's account
    private static string? SteamExe() => Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam", "SteamExe", null) as string;
    private static string? LoginUsers() => Library.SteamScanner.SteamPath() is { } s ? Path.Combine(s, "config", "loginusers.vdf") : null;

    public sealed record SteamAccount(string account, string name);
    public static List<SteamAccount> SteamAccounts()
    {
        var f = LoginUsers();
        if (f is null || !File.Exists(f)) return new();
        var text = File.ReadAllText(f);
        var list = new List<SteamAccount>();
        foreach (Match m in Regex.Matches(text, "\"\\d{17}\"\\s*\\{([^{}]*)\\}"))
        {
            string? Field(string k) { var x = Regex.Match(m.Groups[1].Value, $"\"{k}\"\\s+\"([^\"]*)\""); return x.Success ? x.Groups[1].Value : null; }
            if (Field("AccountName") is { } acc) list.Add(new SteamAccount(acc, Field("PersonaName") ?? acc));
        }
        return list;
    }

    /// <summary>Closes Steam (it saves its sign-in when it exits), waiting up to about 10 seconds.</summary>
    public static void StopSteam()
    {
        bool Up() => Process.GetProcessesByName("steam").Length > 0;
        if (!Up()) return;
        try { if (SteamExe() is { } exe) Process.Start(new ProcessStartInfo(exe, "-shutdown") { UseShellExecute = false }); } catch { }
        for (int i = 0; i < 26 && Up(); i++) Thread.Sleep(300);
        foreach (var p in Process.GetProcessesByName("steam")) { try { p.Kill(true); } catch { } p.Dispose(); }
        Thread.Sleep(1000);
    }

    /// <summary>Steam signs in as this account next time it starts ("" shows its own sign-in).</summary>
    public static void SetSteamAccount(string account)
    {
        try
        {
            using var k = Registry.CurrentUser.CreateSubKey(@"Software\Valve\Steam");
            k.SetValue("AutoLoginUser", account);
            if (account.Length > 0) k.SetValue("RememberPassword", 1, RegistryValueKind.DWord);
        }
        catch (Exception ex) { SystemBridge.Log("profiles: Steam registry: " + ex.Message); }
        var f = LoginUsers();
        if (account.Length == 0 || f is null || !File.Exists(f)) return;
        try
        {
            var want = account.ToLowerInvariant();
            var text = Regex.Replace(File.ReadAllText(f), "(\"\\d{17}\"\\s*\\{)([^{}]*)(\\})", m =>
            {
                var body = m.Groups[2].Value;
                var name = Regex.Match(body, "\"AccountName\"\\s+\"([^\"]*)\"").Groups[1].Value.ToLowerInvariant();
                bool mine = name == want;
                string Set(string b, string key, string val) => Regex.IsMatch(b, $"\"{key}\"\\s+\"[^\"]*\"")
                    ? Regex.Replace(b, $"(\"{key}\"\\s+)\"[^\"]*\"", $"$1\"{val}\"") : b.TrimEnd() + $"\n\t\t\"{key}\"\t\t\"{val}\"\n\t";
                body = Set(body, "MostRecent", mine ? "1" : "0");
                if (mine) body = Set(body, "AllowAutoLogin", "1");
                return m.Groups[1].Value + body + m.Groups[3].Value;
            });
            File.WriteAllText(f, text);
        }
        catch (Exception ex) { SystemBridge.Log("profiles: loginusers.vdf: " + ex.Message); }
    }

    /// <summary>Started again after a switch: the "Who's playing?" screen doesn't show again.</summary>
    public static string PickedFile => Path.Combine(Stores.HelperTools.DataDir, "picked");
}
