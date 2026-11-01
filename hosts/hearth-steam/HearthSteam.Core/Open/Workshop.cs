using System.Security.Cryptography;
using System.Text;

namespace HearthSteam.Open;

// Beta branches and Workshop items for the Open engine. Pure: the engine asks Steam (PICS branches, SteamKit2's beta
// password check, the PublishedFile service) and downloads with the same depot code as games.

/// <summary>Beta branches: which ones a player can pick, and the manifests of a password-protected one (its depots'
/// manifest ids come encrypted in the app info; Steam's beta password check gives the key).</summary>
public static class Betas
{
    /// <summary>The branches to offer: the default first, then betas by name; password-protected ones say so.</summary>
    public static List<BranchInfo> Offer(AppInfo app)
    {
        var list = app.Branches.Where(b => !b.Name.Equals("public", StringComparison.OrdinalIgnoreCase)).OrderBy(b => b.Name, StringComparer.OrdinalIgnoreCase).ToList();
        var pub = app.Branches.FirstOrDefault(b => b.Name.Equals("public", StringComparison.OrdinalIgnoreCase)) ?? new BranchInfo("public", app.BuildFor("public"), false, "", 0);
        list.Insert(0, pub);
        return list;
    }

    /// <summary>"" for the default branch, else the branch's name as the app info spells it. Unknown: an error.</summary>
    public static string Pick(AppInfo app, string? wanted)
    {
        if (string.IsNullOrEmpty(wanted) || wanted.Equals("public", StringComparison.OrdinalIgnoreCase)) return "";
        var b = app.Branches.FirstOrDefault(x => x.Name.Equals(wanted, StringComparison.OrdinalIgnoreCase))
                ?? throw new EngineException(Codes.BadRequest, $"This game has no beta called {wanted}.");
        return b.Name;
    }

    /// <summary>A depot's manifest id from its encrypted form (AES-ECB with the beta's key, as Steam's client does).</summary>
    public static ulong DecryptGid(string hex, byte[] key)
    {
        var data = Convert.FromHexString(hex);
        using var aes = Aes.Create();
        aes.Key = key;
        var plain = aes.DecryptEcb(data, PaddingMode.PKCS7);
        if (plain.Length < 8) throw new InvalidDataException("Bad encrypted manifest id.");
        return BitConverter.ToUInt64(plain, 0);
    }

    /// <summary>The app info with a protected branch's manifests filled in from the key (null key: unchanged).</summary>
    public static AppInfo WithKey(AppInfo app, byte[]? key)
    {
        if (key == null || !app.Depots.Any(d => d.Encrypted && d.EncryptedGid.Length > 0)) return app;
        var depots = app.Depots.Select(d => d.Encrypted && d.EncryptedGid.Length > 0 ? d with { Manifest = DecryptGid(d.EncryptedGid, key), Encrypted = false } : d).ToList();
        return new AppInfo
        {
            AppId = app.AppId, Name = app.Name, Type = app.Type, InstallDir = app.InstallDir, BuildId = app.BuildId, OsList = app.OsList, Vac = app.Vac,
            Depots = depots, Launch = app.Launch, Branches = app.Branches, Branch = app.Branch, Ufs = app.Ufs, WorkshopDepot = app.WorkshopDepot,
        };
    }
}

/// <summary>A subscribed Workshop item as Steam's PublishedFile service describes it: its UGC manifest (hcontent_file)
/// or, for old items, a single file's URL.</summary>
public sealed record WorkshopItem(ulong Id, string Title, ulong Manifest, ulong Size, long TimeUpdated, string FileUrl = "", string FileName = "");

/// <summary>steamapps/workshop/appworkshop_&lt;app&gt;.acf, written the way Steam writes it, so Valve's client and the
/// game see the items in steamapps/workshop/content/&lt;app&gt;/&lt;id&gt;.</summary>
public sealed class WorkshopManifest
{
    public uint AppId { get; set; }
    public uint LastBuildId { get; set; }
    public long TimeLastUpdated { get; set; }
    public uint SubscribedBy { get; set; }
    /// <summary>Item id → (size on disk, time updated, manifest).</summary>
    public SortedDictionary<ulong, (ulong Size, long TimeUpdated, ulong Manifest)> Items { get; } = new();

    public static string Path(string library, uint appId) => System.IO.Path.Combine(library, "steamapps", "workshop", $"appworkshop_{appId}.acf");
    public static string ContentDir(string library, uint appId, ulong item) => System.IO.Path.Combine(library, "steamapps", "workshop", "content", appId.ToString(), item.ToString());

    public string Text()
    {
        var sb = new StringBuilder();
        void L(int tabs, string k, object v) => sb.Append('\t', tabs).Append('"').Append(k).Append("\"\t\t\"").Append(Convert.ToString(v, System.Globalization.CultureInfo.InvariantCulture)).Append("\"\n");
        sb.Append("\"AppWorkshop\"\n{\n");
        L(1, "appid", AppId);
        L(1, "SizeOnDisk", Items.Values.Aggregate(0UL, (t, i) => t + i.Size));
        L(1, "NeedsUpdate", 0); L(1, "NeedsDownload", 0);
        L(1, "TimeLastUpdated", TimeLastUpdated); L(1, "TimeLastAppRan", 0); L(1, "LastBuildID", LastBuildId);
        sb.Append("\t\"WorkshopItemsInstalled\"\n\t{\n");
        foreach (var (id, i) in Items)
        {
            sb.Append("\t\t\"").Append(id).Append("\"\n\t\t{\n");
            L(3, "size", i.Size); L(3, "timeupdated", i.TimeUpdated); L(3, "manifest", i.Manifest);
            sb.Append("\t\t}\n");
        }
        sb.Append("\t}\n\t\"WorkshopItemDetails\"\n\t{\n");
        foreach (var (id, i) in Items)
        {
            sb.Append("\t\t\"").Append(id).Append("\"\n\t\t{\n");
            L(3, "manifest", i.Manifest); L(3, "timeupdated", i.TimeUpdated); L(3, "timetouched", TimeLastUpdated); L(3, "subscribedby", SubscribedBy);
            sb.Append("\t\t}\n");
        }
        sb.Append("\t}\n}\n");
        return sb.ToString();
    }

    public static WorkshopManifest? Parse(string text)
    {
        var s = Vdf.Parse(text).Get("AppWorkshop");
        if (s == null || !uint.TryParse(s.Str("appid"), out var app)) return null;
        static ulong N(string? v) => ulong.TryParse(v, out var n) ? n : 0;
        var m = new WorkshopManifest { AppId = app, LastBuildId = (uint)N(s.Str("LastBuildID")), TimeLastUpdated = (long)N(s.Str("TimeLastUpdated")) };
        foreach (var (k, d) in s.Get("WorkshopItemsInstalled")?.Children ?? new())
            if (ulong.TryParse(k, out var id)) m.Items[id] = (N(d.Str("size")), (long)N(d.Str("timeupdated")), N(d.Str("manifest")));
        foreach (var (_, d) in s.Get("WorkshopItemDetails")?.Children ?? new())
            if (uint.TryParse(d.Str("subscribedby"), out var by) && by != 0) { m.SubscribedBy = by; break; }
        return m;
    }

    public static WorkshopManifest Load(string library, uint appId)
    {
        try { var p = Path(library, appId); if (File.Exists(p) && Parse(File.ReadAllText(p)) is { } m) return m; }
        catch (IOException) { }
        return new WorkshopManifest { AppId = appId };
    }

    public void Save(string library)
    {
        var p = Path(library, AppId);
        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(p)!);
        var tmp = p + ".hearth-tmp";
        File.WriteAllText(tmp, Text(), new UTF8Encoding(false));
        File.Move(tmp, p, true);
    }
}

public static class WorkshopRules
{
    /// <summary>What a sync does: download items that are new or updated since they were installed, and remove items
    /// no longer subscribed, but only when Steam gave a real subscription list (an empty or failed list never
    /// removes anything).</summary>
    public static (List<WorkshopItem> Get, List<ulong> Remove) Plan(IReadOnlyList<WorkshopItem>? subscribed, WorkshopManifest installed)
    {
        var get = new List<WorkshopItem>();
        var remove = new List<ulong>();
        if (subscribed == null) return (get, remove);
        foreach (var i in subscribed)
        {
            if (i.Manifest == 0 && i.FileUrl.Length == 0) continue;   // nothing Hearth can download
            if (!installed.Items.TryGetValue(i.Id, out var have) || have.TimeUpdated < i.TimeUpdated || (i.Manifest != 0 && have.Manifest != i.Manifest)) get.Add(i);
        }
        if (subscribed.Count > 0)
        {
            var ids = subscribed.Select(i => i.Id).ToHashSet();
            remove.AddRange(installed.Items.Keys.Where(k => !ids.Contains(k)));
        }
        return (get, remove);
    }

    /// <summary>Removes one item's folder (only steamapps/workshop/content/&lt;app&gt;/&lt;id&gt;; a link there is removed,
    /// never followed) and its record.</summary>
    public static void Remove(string library, WorkshopManifest m, ulong item)
    {
        var dir = new DirectoryInfo(WorkshopManifest.ContentDir(library, m.AppId, item));
        if (dir.LinkTarget != null) dir.Delete();
        else if (dir.Exists) dir.Delete(true);
        m.Items.Remove(item);
    }

    /// <summary>A plain file name for an old single-file item (no folders, nothing hidden that escapes).</summary>
    public static string FileName(string name, ulong id)
    {
        var n = System.IO.Path.GetFileName((name ?? "").Replace('\\', '/'));
        n = new string(n.Where(c => !char.IsControl(c) && c != '/' && c != '\0').ToArray()).Trim();
        return n.Length == 0 || n == "." || n == ".." ? $"{id}.bin" : n;
    }
}
