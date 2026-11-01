using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace HearthSteam.Open;

// Steam Cloud for games Hearth starts directly (the Open engine): Steam Auto-Cloud's save rules from the app info
// ("ufs": savefiles roots, paths, patterns, platforms, root overrides) map each cloud file to a local path, inside the
// game's Proton prefix for Windows builds; before the game starts newer cloud files come down, after it exits changed
// files go up in one upload batch. When both sides changed, the player decides (keep local / keep cloud / keep both);
// nothing is ever deleted without asking. Apps without save rules Hearth can place are left to Valve's client.
// Pure except for files and the ICloudApi (SteamKit2's Cloud service in the engine, a fake in the tests).

public sealed record UfsSave(string Root, string Path, string Pattern, bool Recursive, IReadOnlyList<string> Platforms);
public sealed record UfsOverride(string Root, string Os, string UseInstead, string AddPath);

/// <summary>An app's Steam Auto-Cloud rules (PICS app info › ufs).</summary>
public sealed class Ufs
{
    public long Quota { get; init; }
    public int MaxFiles { get; init; }
    public List<UfsSave> Saves { get; init; } = new();
    public List<UfsOverride> Overrides { get; init; } = new();
    /// <summary>The app uses Steam Cloud at all.</summary>
    public bool HasCloud => Quota > 0 || MaxFiles > 0 || Saves.Count > 0;

    public static Ufs Parse(VdfNode? ufs)
    {
        if (ufs == null) return new Ufs();
        var saves = new List<UfsSave>();
        foreach (var (_, s) in ufs.Get("savefiles")?.Children ?? new())
        {
            var root = s.Str("root") ?? "";
            if (s.Children.Count == 0) continue;
            var plats = (s.Get("platforms")?.Children.Values ?? Enumerable.Empty<VdfNode>()).Select(v => v.Value ?? "").Where(v => v.Length > 0).ToList();
            saves.Add(new UfsSave(root, s.Str("path") ?? "", s.Str("pattern") ?? "*", s.Str("recursive") == "1", plats));
        }
        var ovr = new List<UfsOverride>();
        foreach (var (_, o) in ufs.Get("rootoverrides")?.Children ?? new())
            if (o.Children.Count > 0) ovr.Add(new UfsOverride(o.Str("root") ?? "", o.Str("os") ?? "", o.Str("useinstead") ?? "", o.Str("addpath") ?? ""));
        return new Ufs
        {
            Quota = long.TryParse(ufs.Str("quota"), out var q) ? q : 0, MaxFiles = int.TryParse(ufs.Str("maxnumfiles"), out var m) ? m : 0,
            Saves = saves, Overrides = ovr,
        };
    }
}

/// <summary>Where a cloud root is on this device for one game: Windows roots inside the game's Proton prefix
/// (steamuser's profile), Linux roots in the home folder, GameInstall in the game's folder.</summary>
public sealed class CloudPaths
{
    readonly string gameDir, os, prefix, home;
    readonly ulong steamId64;
    readonly Ufs ufs;

    public CloudPaths(Ufs ufs, string gameDir, string os, string prefix, string home, ulong steamId64)
    { this.ufs = ufs; this.gameDir = gameDir; this.os = os; this.prefix = prefix; this.home = home; this.steamId64 = steamId64; }

    public uint AccountId => (uint)(steamId64 & 0xFFFFFFFF);
    string Profile => System.IO.Path.Combine(prefix, "drive_c", "users", "steamuser");

    /// <summary>The folder a root stands for, or null when Hearth doesn't know it here (Mac roots, a Windows root
    /// for a Linux build without an override, Steam's own legacy storage).</summary>
    public string? Base(string root, int depth = 0)
    {
        if (os == "linux" && depth == 0)
        {
            // A Linux build: Steam's root overrides for Linux say where Windows paths go.
            var o = ufs.Overrides.FirstOrDefault(x => x.Root.Equals(root, StringComparison.OrdinalIgnoreCase) && x.Os.Equals("Linux", StringComparison.OrdinalIgnoreCase));
            if (o != null && o.UseInstead.Length > 0)
            {
                var b = Base(o.UseInstead, 1);
                if (b == null) return null;
                var add = Depot.SafePath(Tokens(o.AddPath));
                return o.AddPath.Length == 0 ? b : add == null ? null : System.IO.Path.Combine(b, add);
            }
        }
        bool win = os == "windows";
        return root switch
        {
            "GameInstall" => gameDir,
            "WinMyDocuments" when win => System.IO.Path.Combine(Profile, "Documents"),
            "WinAppDataLocal" when win => System.IO.Path.Combine(Profile, "AppData", "Local"),
            "WinAppDataLocalLow" when win => System.IO.Path.Combine(Profile, "AppData", "LocalLow"),
            "WinAppDataRoaming" when win => System.IO.Path.Combine(Profile, "AppData", "Roaming"),
            "WinSavedGames" when win => System.IO.Path.Combine(Profile, "Saved Games"),
            "LinuxHome" when !win => home,
            "LinuxXdgDataHome" when !win => System.IO.Path.Combine(home, ".local", "share"),
            "LinuxXdgConfigHome" when !win => System.IO.Path.Combine(home, ".config"),
            _ => null,
        };
    }

    /// <summary>Steam's path tokens: {64BitSteamID}, {Steam3AccountID}; %USERPROFILE% (Windows builds: steamuser in the
    /// prefix). Backslashes become slashes.</summary>
    public string Tokens(string path) => path
        .Replace("{64BitSteamID}", steamId64.ToString(), StringComparison.OrdinalIgnoreCase)
        .Replace("{Steam3AccountID}", AccountId.ToString(), StringComparison.OrdinalIgnoreCase)
        .Replace("%USERPROFILE%/", "", StringComparison.OrdinalIgnoreCase)
        .Replace('\\', '/');

    static readonly Regex RootName = new(@"^%([A-Za-z0-9]+)%(.*)$", RegexOptions.Compiled);

    /// <summary>The local file for a cloud file name ("%WinMyDocuments%My Games/X/save1.sav"), or null when it can't
    /// be placed safely (unknown root, legacy storage, a path that would leave its root folder).</summary>
    public string? Local(string remote)
    {
        var m = RootName.Match(remote.Replace('\\', '/'));
        if (!m.Success) return null;
        var b = Base(m.Groups[1].Value);
        var rel = Depot.SafePath(m.Groups[2].Value);
        if (b == null || rel == null) return null;
        try { return Depot.Inside(b, rel); } catch (InvalidDataException) { return null; }
    }

    /// <summary>The save rules that apply to this build (its platform, or all platforms).</summary>
    public IEnumerable<UfsSave> Rules() => ufs.Saves.Where(s =>
        s.Platforms.Count == 0 || s.Platforms.Any(p => p.Equals("all", StringComparison.OrdinalIgnoreCase) || p.Equals(os, StringComparison.OrdinalIgnoreCase)));

    /// <summary>Can Hearth place this game's saves? (Every rule for this build has a known root; at least one rule.)</summary>
    public (bool Ok, string Why) Supported()
    {
        if (!ufs.HasCloud) return (true, "no Steam Cloud");
        var rules = Rules().ToList();
        if (rules.Count == 0) return (false, "no Steam Cloud save rules for this build");
        foreach (var r in rules) if (Base(r.Root) == null) return (false, $"a save folder Hearth can't place ({r.Root})");
        return (true, $"{rules.Count} save rule(s)");
    }

    static Regex Glob(string pattern) => new("^" + Regex.Escape(pattern.Length == 0 ? "*" : pattern).Replace("\\*", ".*").Replace("\\?", ".") + "$", RegexOptions.IgnoreCase);

    /// <summary>The local save files the rules match, with their cloud names. Links aren't followed.</summary>
    public List<LocalSave> Scan(int max = 10_000)
    {
        var list = new List<LocalSave>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var r in Rules())
        {
            var b = Base(r.Root);
            var path = Tokens(r.Path).Trim('/');
            var safe = path.Length == 0 ? "" : Depot.SafePath(path);
            if (b == null || safe == null) continue;
            var dir = safe.Length == 0 ? b : System.IO.Path.Combine(b, safe);
            if (!Directory.Exists(dir)) continue;
            var glob = Glob(r.Pattern);
            IEnumerable<string> files;
            try
            {
                files = Directory.EnumerateFiles(dir, "*", new EnumerationOptions
                {
                    RecurseSubdirectories = r.Recursive, MaxRecursionDepth = 12, IgnoreInaccessible = true, AttributesToSkip = FileAttributes.ReparsePoint,
                }).ToList();
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException) { continue; }
            foreach (var f in files)
            {
                if (!glob.IsMatch(System.IO.Path.GetFileName(f))) continue;
                var rel = System.IO.Path.GetRelativePath(dir, f).Replace('\\', '/');
                var name = $"%{r.Root}%" + (safe.Length > 0 ? safe + "/" : "") + rel;
                if (!seen.Add(name)) continue;
                var fi = new FileInfo(f);
                list.Add(new LocalSave(name, f, fi.Length, new DateTimeOffset(fi.LastWriteTimeUtc).ToUnixTimeSeconds()));
                if (list.Count >= max) return list;
            }
        }
        return list;
    }
}

public sealed record LocalSave(string Name, string Path, long Size, long Time)
{
    public string Sha() { using var fs = File.OpenRead(Path); return Convert.ToHexString(SHA1.HashData(fs)).ToLowerInvariant(); }
}

/// <summary>A file in the app's Steam Cloud (its full name, SHA-1 in hex, time, size; Deleted: removed there).</summary>
public sealed record CloudFile(string Name, string Sha, long Time, long Size, bool Deleted = false);

public sealed record CloudList(ulong ChangeNumber, List<CloudFile> Files);

/// <summary>What Steam's Cloud service does for the sync (SteamKit2's unified Cloud messages in the engine).</summary>
public interface ICloudApi
{
    Task<CloudList> List(uint app, ulong syncedChangeNumber);
    /// <summary>A file's content as Steam serves it (possibly zipped; the sync unpacks and checks it).</summary>
    Task<byte[]> Download(uint app, string name);
    Task<ulong> BeginBatch(uint app, IReadOnlyList<string> upload, IReadOnlyList<string> delete);
    /// <summary>Begins the file's upload in the batch, sends its blocks, commits it. Throws when it didn't go up.</summary>
    Task Upload(uint app, ulong batch, string name, byte[] data, byte[] sha, long time);
    Task CompleteBatch(uint app, ulong batch, bool ok);
}

public sealed record SyncPlan(List<CloudFile> Download, List<LocalSave> Upload, List<string> Conflicts, int Same, int RemoteDeleted, int LocalDeleted);

public static class CloudRules
{
    /// <summary>What to do with each file (pure). <paramref name="known"/>: each file's SHA-1 at the last sync on this
    /// device. Changed on one side only: that side wins (download / upload). Changed on both: a conflict for the
    /// player. Removed on one side: nothing is deleted on the other (counted only).</summary>
    public static SyncPlan Plan(IEnumerable<CloudFile> remote, IEnumerable<(LocalSave File, string Sha)> local, IReadOnlyDictionary<string, string> known)
    {
        var cmp = StringComparer.OrdinalIgnoreCase;
        var r = remote.GroupBy(f => f.Name, cmp).ToDictionary(g => g.Key, g => g.Last(), cmp);
        var l = local.GroupBy(f => f.File.Name, cmp).ToDictionary(g => g.Key, g => g.Last(), cmp);
        var k = new Dictionary<string, string>(known, cmp);
        var down = new List<CloudFile>(); var up = new List<LocalSave>(); var conflicts = new List<string>();
        int same = 0, rdel = 0, ldel = 0;
        foreach (var name in r.Keys.Union(l.Keys, cmp).OrderBy(n => n, StringComparer.Ordinal))
        {
            r.TryGetValue(name, out var rf);
            bool hasR = rf != null && !rf.Deleted;
            bool hasL = l.TryGetValue(name, out var lf);
            k.TryGetValue(name, out var b);
            string? rs = hasR ? rf!.Sha.ToLowerInvariant() : null, ls = hasL ? lf.Sha.ToLowerInvariant() : null;
            if (rs != null && rs == ls) { same++; continue; }
            bool rChanged = rs != b?.ToLowerInvariant(), lChanged = ls != b?.ToLowerInvariant();
            if (!hasR && hasL) { if (b != null && !lChanged) rdel++; else up.Add(lf.File); continue; }   // removed in the cloud: kept here
            if (hasR && !hasL) { if (b != null && !rChanged) ldel++; else down.Add(rf!); continue; }    // removed here: kept in the cloud
            if (!hasR && !hasL) continue;
            if (rChanged && !lChanged) down.Add(rf!);
            else if (lChanged && !rChanged) up.Add(lf.File);
            else conflicts.Add(name);
        }
        return new SyncPlan(down, up, conflicts, same, rdel, ldel);
    }

    /// <summary>A downloaded file as the game expects it: as is when its SHA-1 matches, else unpacked from Steam's
    /// zip (one file inside). Null when neither matches.</summary>
    public static byte[]? Content(byte[] data, string sha)
    {
        static string H(byte[] d) => Convert.ToHexString(SHA1.HashData(d)).ToLowerInvariant();
        if (H(data) == sha.ToLowerInvariant()) return data;
        if (data.Length > 4 && data[0] == (byte)'P' && data[1] == (byte)'K')
        {
            try
            {
                using var z = new ZipArchive(new MemoryStream(data), ZipArchiveMode.Read);
                var e = z.Entries.FirstOrDefault(x => x.Length > 0 || x.Name.Length > 0);
                if (e == null) return null;
                using var s = e.Open(); using var ms = new MemoryStream(); s.CopyTo(ms);
                var raw = ms.ToArray();
                return H(raw) == sha.ToLowerInvariant() ? raw : null;
            }
            catch (InvalidDataException) { return null; }
        }
        return null;
    }
}

/// <summary>The sync itself, per app: before a direct start, after the game exits, and the player's answer to a
/// conflict. Its record (each file's SHA-1 at the last sync, the change number, a waiting conflict) is in
/// ~/.local/state/hearth-steam/cloud/&lt;app&gt;.json. Logs counts only, never file names.</summary>
public sealed class CloudSync
{
    readonly ICloudApi api;
    readonly string stateDir, copiesDir;
    readonly Action<string> log;
    public string MachineName { get; init; } = "Hearth OS";

    public CloudSync(ICloudApi api, string stateDir, string copiesDir, Action<string>? log = null)
    { this.api = api; this.stateDir = stateDir; this.copiesDir = copiesDir; this.log = log ?? (_ => { }); }

    string RecordPath(uint app) => System.IO.Path.Combine(stateDir, "cloud", $"{app}.json");

    JsonObject Record(uint app)
    {
        try { var p = RecordPath(app); return File.Exists(p) ? JsonNode.Parse(File.ReadAllText(p)) as JsonObject ?? new() : new(); }
        catch (Exception e) when (e is IOException or System.Text.Json.JsonException) { return new(); }
    }

    void Save(uint app, JsonObject rec)
    {
        var p = RecordPath(app);
        Secrets.EnsurePrivateDir(System.IO.Path.GetDirectoryName(p)!);
        File.WriteAllText(p + ".tmp", rec.ToJsonString());
        File.Move(p + ".tmp", p, true);
    }

    static Dictionary<string, string> Known(JsonObject rec)
    {
        var d = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (rec["files"] is JsonObject f) foreach (var (k, v) in f) if (v?.GetValue<string>() is string s) d[k] = s;
        return d;
    }

    /// <summary>A conflict waiting for the player ({local, cloud} counts), or null.</summary>
    public JsonObject? Pending(uint app) => Record(app)["conflict"]?.DeepClone() as JsonObject;

    async Task<(CloudList remote, List<(LocalSave, string)> local, SyncPlan plan, JsonObject rec)> Look(uint app, CloudPaths paths)
    {
        var rec = Record(app);
        var remote = await api.List(app, 0);
        // Only files that map to a place on this device take part (other platforms' saves stay in the cloud as they are).
        var mine = remote.Files.Where(f => paths.Local(f.Name) != null).ToList();
        // A local file takes the cloud's spelling of its name (Windows paths ignore case; Steam's names don't).
        var spelled = mine.GroupBy(f => f.Name, StringComparer.OrdinalIgnoreCase).ToDictionary(g => g.Key, g => g.First().Name, StringComparer.OrdinalIgnoreCase);
        var local = paths.Scan().Select(s => spelled.TryGetValue(s.Name, out var n) && n != s.Name ? s with { Name = n } : s).Select(s => (s, s.Sha())).ToList();
        return (remote, local, CloudRules.Plan(mine, local, Known(rec)), rec);
    }

    static void Remember(JsonObject rec, string name, string sha)
    {
        if (rec["files"] is not JsonObject f) rec["files"] = f = new JsonObject();
        f[name] = sha.ToLowerInvariant();
    }

    async Task<int> DownloadAll(uint app, IEnumerable<CloudFile> files, CloudPaths paths, JsonObject rec, string? copyTo)
    {
        int n = 0;
        foreach (var f in files)
        {
            var dest = paths.Local(f.Name);
            if (dest == null) continue;
            var data = CloudRules.Content(await api.Download(app, f.Name), f.Sha) ?? throw new InvalidDataException("A cloud file didn't match its checksum.");
            Directory.CreateDirectory(System.IO.Path.GetDirectoryName(dest)!);
            if (copyTo != null && File.Exists(dest))
            {
                var keep = System.IO.Path.Combine(copyTo, f.Name.TrimStart('%').Replace('%', '/').Replace(':', '_'));
                Directory.CreateDirectory(System.IO.Path.GetDirectoryName(keep)!);
                File.Copy(dest, keep, true);
            }
            var tmp = dest + ".hearth-cloud";
            await File.WriteAllBytesAsync(tmp, data);
            if (f.Time > 0) File.SetLastWriteTimeUtc(tmp, DateTimeOffset.FromUnixTimeSeconds(f.Time).UtcDateTime);
            File.Move(tmp, dest, true);
            Remember(rec, f.Name, f.Sha);
            n++;
        }
        return n;
    }

    /// <summary>All changed files in one upload batch (begin, each file, complete; a failure completes the batch as
    /// failed and throws).</summary>
    async Task<int> UploadAll(uint app, IReadOnlyList<LocalSave> files, JsonObject rec)
    {
        if (files.Count == 0) return 0;
        var batch = await api.BeginBatch(app, files.Select(f => f.Name).ToList(), Array.Empty<string>());
        int n = 0;
        try
        {
            foreach (var f in files)
            {
                var data = await File.ReadAllBytesAsync(f.Path);
                var sha = SHA1.HashData(data);
                await api.Upload(app, batch, f.Name, data, sha, f.Time);
                Remember(rec, f.Name, Convert.ToHexString(sha));
                n++;
            }
        }
        catch
        {
            try { await api.CompleteBatch(app, batch, false); } catch (Exception) { }
            throw;
        }
        await api.CompleteBatch(app, batch, true);
        return n;
    }

    JsonObject Conflict(uint app, JsonObject rec, SyncPlan plan, string when)
    {
        rec["conflict"] = new JsonObject { ["files"] = plan.Conflicts.Count, ["when"] = when, ["at"] = DateTimeOffset.UtcNow.ToUnixTimeSeconds() };
        Save(app, rec);
        log($"cloud {app}: {plan.Conflicts.Count} file(s) changed both here and in Steam Cloud ({when}); asking");
        return new JsonObject { ["conflict"] = true, ["files"] = plan.Conflicts.Count, ["when"] = when };
    }

    /// <summary>Before a direct start: newer cloud files come down. A conflict (or one still waiting) stops here and
    /// asks the player; nothing is started until it's answered.</summary>
    public async Task<JsonObject> Before(uint app, CloudPaths paths)
    {
        var (ok, why) = paths.Supported();
        if (!ok) return new JsonObject { ["supported"] = false, ["why"] = why };
        var (remote, _, plan, rec) = await Look(app, paths);
        if (plan.Conflicts.Count > 0) return Conflict(app, rec, plan, "before");
        rec.Remove("conflict");
        int got = await DownloadAll(app, plan.Download, paths, rec, null);
        rec["changeNumber"] = remote.ChangeNumber;
        Save(app, rec);
        log($"cloud {app}: before start, {got} down, {plan.Same} unchanged, {plan.Upload.Count} to go up after");
        return new JsonObject { ["supported"] = true, ["downloaded"] = got, ["same"] = plan.Same };
    }

    /// <summary>After the game exits: changed files go up in one batch. When the cloud changed meanwhile too, the
    /// player is asked.</summary>
    public async Task<JsonObject> After(uint app, CloudPaths paths)
    {
        var (ok, why) = paths.Supported();
        if (!ok) return new JsonObject { ["supported"] = false, ["why"] = why };
        var (remote, _, plan, rec) = await Look(app, paths);
        if (plan.Conflicts.Count > 0) return Conflict(app, rec, plan, "after");
        int up = await UploadAll(app, plan.Upload, rec);
        int got = await DownloadAll(app, plan.Download, paths, rec, null);
        rec.Remove("conflict");
        rec["changeNumber"] = remote.ChangeNumber;
        Save(app, rec);
        log($"cloud {app}: after the game, {up} up, {got} down, {plan.Same} unchanged");
        return new JsonObject { ["supported"] = true, ["uploaded"] = up, ["downloaded"] = got };
    }

    /// <summary>The player's answer to a conflict: "local" (this device's saves go to the cloud), "cloud" (the cloud's
    /// come here; this device's are kept aside in Hearth's state folder) or "both" (the cloud's come here; this
    /// device's are kept in <c>copiesDir</c>, where the player can find them). Files changed on one side only follow
    /// the usual rules.</summary>
    public async Task<JsonObject> Resolve(uint app, string choice, CloudPaths paths)
    {
        if (choice is not ("local" or "cloud" or "both")) throw new EngineException(Codes.BadRequest, "Keep local, cloud or both.");
        var (remote, local, plan, rec) = await Look(app, paths);
        var conflicted = new HashSet<string>(plan.Conflicts, StringComparer.OrdinalIgnoreCase);
        var ups = plan.Upload.ToList();
        var downs = plan.Download.ToList();
        if (choice == "local") ups.AddRange(local.Select(x => x.Item1).Where(f => conflicted.Contains(f.Name)));
        var stamp = DateTime.Now.ToString("yyyy-MM-dd HH.mm.ss");
        string copyTo = choice == "both" ? System.IO.Path.Combine(copiesDir, app.ToString(), stamp) : System.IO.Path.Combine(stateDir, "cloud-kept", app.ToString(), stamp);
        int up = await UploadAll(app, ups, rec);
        int got = await DownloadAll(app, downs, paths, rec, null);
        int replaced = choice == "local" ? 0 : await DownloadAll(app, remote.Files.Where(f => conflicted.Contains(f.Name) && !f.Deleted), paths, rec, copyTo);
        rec.Remove("conflict");
        rec["changeNumber"] = remote.ChangeNumber;
        Save(app, rec);
        log($"cloud {app}: conflict answered ({choice}): {up} up, {got + replaced} down");
        var o = new JsonObject { ["resolved"] = choice, ["uploaded"] = up, ["downloaded"] = got + replaced };
        if (choice == "both" && replaced > 0) o["copies"] = copyTo;
        return o;
    }
}
