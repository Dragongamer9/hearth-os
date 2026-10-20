namespace Nimbus.Host.Library;

/// <summary>
/// Library › Screenshots: the screenshots taken from the in-game overlay (Pictures\Screenshots\Ember), newest first,
/// to look through and delete. Same answers as hearth-core's gamescope::screenshots. The pictures are served to the
/// page from https://nimbus-art.local/shot/&lt;file name&gt;.
/// </summary>
public static class Screenshots
{
    public static string Dir => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyPictures), "Screenshots", "Ember");
    private static readonly string[] Kinds = { ".png", ".jpg", ".jpeg", ".webp" };

    public static List<object> List()
    {
        var list = new List<(long T, object V)>();
        try
        {
            if (!Directory.Exists(Dir)) return new List<object>();
            foreach (var f in Directory.EnumerateFiles(Dir))
            {
                if (!Kinds.Contains(Path.GetExtension(f).ToLowerInvariant())) continue;
                var info = new FileInfo(f);
                long t = Time.Ms(info.LastWriteTimeUtc);
                var name = Path.GetFileName(f);
                list.Add((t, new { file = f, url = $"https://{LibraryService.ArtHost}/shot/{Uri.EscapeDataString(name)}", name = Path.GetFileNameWithoutExtension(f), time = t, size = info.Length }));
            }
        }
        catch { }
        return list.OrderByDescending(x => x.T).Select(x => x.V).ToList();
    }

    /// <summary>The picture for an art request (a file name in the screenshots folder only), or null.</summary>
    public static string? File(string escapedName)
    {
        var name = Uri.UnescapeDataString(escapedName);
        if (name.Length == 0 || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0 || name.Contains("..")) return null;
        var f = Path.Combine(Dir, name);
        return System.IO.File.Exists(f) && Kinds.Contains(Path.GetExtension(f).ToLowerInvariant()) ? f : null;
    }

    /// <summary>Deletes one screenshot (only from Ember's screenshots folder).</summary>
    public static void Delete(string file)
    {
        string full, dir;
        try { full = Path.GetFullPath(file); dir = Path.GetFullPath(Dir); }
        catch { throw new InvalidOperationException("That screenshot is already gone"); }
        if (!string.Equals(Path.GetDirectoryName(full), dir.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Only Ember's screenshots can be deleted here");
        if (!System.IO.File.Exists(full)) throw new InvalidOperationException("That screenshot is already gone");
        System.IO.File.Delete(full);
    }
}
