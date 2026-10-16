using System.Text;

namespace Nimbus.Host.Library;

/// <summary>
/// Reads Steam's binary caches:
///   appcache/appinfo.vdf     – metadata (name, type, OS) for every app the client knows about
///   appcache/packageinfo.vdf – the packages (licenses) this account owns, with their app ids
/// Together they give the list of games the user owns, without needing a Web API key.
/// Supports appinfo v27/v28/v29 and packageinfo v27/v28.
/// </summary>
public static class SteamCache
{
    /// <summary>Assets: kind (cover/hero/logo/wide) → path under Steam's store_item_assets CDN folder for this app.</summary>
    public sealed record AppInfo(uint AppId, string Name, string Type, string? OsList, string? ReleaseState, Dictionary<string, string> Assets);

    public static Dictionary<uint, AppInfo> ReadAppInfo(string path)
    {
        var result = new Dictionary<uint, AppInfo>();
        if (!File.Exists(path)) return result;
        var b = ReadShared(path);
        int i = 0;
        uint magic = U32(b, ref i);
        U32(b, ref i); // universe

        string[]? keys = null;
        int headerSkip;
        switch (magic)
        {
            case 0x07564429: // v29: key names live in a string table at the end
            {
                long tableOffset = BitConverter.ToInt64(b, i); i += 8;
                int t = (int)tableOffset;
                uint count = U32(b, ref t);
                keys = new string[count];
                for (int k = 0; k < count; k++) keys[k] = CStr(b, ref t);
                headerSkip = 4 + 4 + 8 + 20 + 4 + 20;
                break;
            }
            case 0x07564428: headerSkip = 4 + 4 + 8 + 20 + 4 + 20; break;
            case 0x07564427: headerSkip = 4 + 4 + 8 + 20 + 4; break;
            default: throw new InvalidDataException($"Unknown appinfo.vdf version 0x{magic:X8}");
        }

        while (i + 8 <= b.Length)
        {
            uint appId = U32(b, ref i);
            if (appId == 0) break;
            uint size = U32(b, ref i);
            int end = i + (int)size;
            try
            {
                int j = i + headerSkip;
                var root = ReadKv(b, ref j, keys);
                if (Child(root, "appinfo") is { } info && Child(info, "common") is { } common)
                {
                    var name = common.GetValueOrDefault("name") as string;
                    var type = common.GetValueOrDefault("type") as string;
                    if (name is not null && type is not null)
                        result[appId] = new AppInfo(appId, name.Trim(), type, common.GetValueOrDefault("oslist") as string,
                                                    common.GetValueOrDefault("releasestate") as string, Assets(common));
                }
            }
            catch { /* skip a malformed entry */ }
            i = end;
        }
        return result;
    }

    /// <summary>App ids in every owned package except package 0 (Steam's base package that everyone has).</summary>
    public static HashSet<uint> ReadOwnedAppIds(string path)
    {
        var owned = new HashSet<uint>();
        if (!File.Exists(path)) return owned;
        var b = ReadShared(path);
        int i = 0;
        uint magic = U32(b, ref i);
        U32(b, ref i); // universe
        bool hasToken = magic >= 0x06565528;
        if ((magic & 0xFFFFFF00) != 0x06565500) throw new InvalidDataException($"Unknown packageinfo.vdf version 0x{magic:X8}");

        while (i + 4 <= b.Length)
        {
            uint packageId = U32(b, ref i);
            if (packageId == 0xFFFFFFFF) break;
            i += 20 + 4 + (hasToken ? 8 : 0);
            var root = ReadKv(b, ref i, null);
            if (packageId == 0) continue;
            foreach (var pkg in root.Values.OfType<Dictionary<string, object>>())
            {
                if (Child(pkg, "appids") is not { } ids) continue;
                foreach (var v in ids.Values)
                    if (v is int id && id > 0) owned.Add((uint)id);
            }
        }
        return owned;
    }

    /// <summary>
    /// Art file names from library_assets_full (newer apps use hashed sub-folders, e.g. "f86c…/library_capsule.jpg").
    /// Apps without library art still have a store header image, which the UI can use as a cover.
    /// </summary>
    private static Dictionary<string, string> Assets(Dictionary<string, object> common)
    {
        var a = new Dictionary<string, string>();
        static string? English(Dictionary<string, object>? node, string child) =>
            node is not null && Child(node, child) is { } img && img.GetValueOrDefault("english") is string s && s.Length > 0 ? s : null;

        var full = Child(common, "library_assets_full");
        if (English(full is null ? null : Child(full, "library_capsule"), "image") is { } cover) a["cover"] = cover;
        if (English(full is null ? null : Child(full, "library_hero"), "image") is { } hero) a["hero"] = hero;
        if (English(full is null ? null : Child(full, "library_logo"), "image") is { } logo) a["logo"] = logo;
        var header = English(full is null ? null : Child(full, "library_header"), "image")
                     ?? (Child(common, "header_image")?.GetValueOrDefault("english") as string);
        if (!string.IsNullOrEmpty(header)) a["wide"] = header;
        return a;
    }

    // ------------------------------------------------------------------ binary KeyValues

    private static Dictionary<string, object> ReadKv(byte[] b, ref int i, string[]? keys)
    {
        var d = new Dictionary<string, object>(StringComparer.OrdinalIgnoreCase);
        while (true)
        {
            byte type = b[i++];
            if (type == 0x08) return d;
            string key;
            if (keys is not null) { int k = BitConverter.ToInt32(b, i); i += 4; key = (uint)k < keys.Length ? keys[k] : k.ToString(); }
            else key = CStr(b, ref i);

            object value = type switch
            {
                0x00 => ReadKv(b, ref i, keys),
                0x01 => CStr(b, ref i),
                0x02 => Take(BitConverter.ToInt32(b, i), ref i, 4),
                0x03 => Take(BitConverter.ToSingle(b, i), ref i, 4),
                0x07 => Take(BitConverter.ToUInt64(b, i), ref i, 8),
                0x0A => Take(BitConverter.ToInt64(b, i), ref i, 8),
                _ => throw new InvalidDataException($"Unknown KV type {type}"),
            };
            d[key] = value;
        }
    }

    private static object Take<T>(T v, ref int i, int n) where T : notnull { i += n; return v; }
    private static Dictionary<string, object>? Child(Dictionary<string, object> d, string k) =>
        d.TryGetValue(k, out var v) ? v as Dictionary<string, object> : null;
    private static uint U32(byte[] b, ref int i) { var v = BitConverter.ToUInt32(b, i); i += 4; return v; }
    private static string CStr(byte[] b, ref int i)
    {
        int start = i;
        while (b[i] != 0) i++;
        var s = Encoding.UTF8.GetString(b, start, i - start);
        i++;
        return s;
    }

    // Steam keeps these files open; read without locking them.
    private static byte[] ReadShared(string path)
    {
        using var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        var buf = new byte[fs.Length];
        fs.ReadExactly(buf);
        return buf;
    }
}
