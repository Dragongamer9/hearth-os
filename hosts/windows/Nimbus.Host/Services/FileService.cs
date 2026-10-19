using System.Collections.Specialized;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Principal;

namespace Nimbus.Host.Services;

/// <summary>
/// Hearth Files on Windows (mantel-files.html), like Mantel's file manager on Hearth OS: places (your folders,
/// game libraries, drives, the Recycle Bin), folder listings, opening with the default app or another one,
/// copy / cut / paste with progress and cancel, rename, Recycle Bin, delete, new folder and properties.
/// Thumbnails are served to the page by FilesWindow (https://hearth-files.local/thumb?p=…).
/// Long copies and moves run in the background and report through events: files.progress and files.done.
/// </summary>
public sealed class FileService
{
    public const string RecyclePath = "recycle:";
    private readonly Action<string, object> _event;
    private readonly Dictionary<int, CancellationTokenSource> _jobs = new();
    private int _jobId;

    public FileService(Action<string, object> raiseEvent) { _event = raiseEvent; }

    // ------------------------------------------------------------------ places

    public object Places(IEnumerable<string>? gameFolders)
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var folders = new List<object>();
        void Add(string id, string name, string? path, string icon)
        {
            if (!string.IsNullOrEmpty(path) && Directory.Exists(path)) folders.Add(new { id, name, path, icon });
        }
        Add("home", "Home", home, "home");
        Add("desktop", "Desktop", Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "desktop");
        Add("documents", "Documents", Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "doc");
        Add("downloads", "Downloads", KnownFolder(new Guid("374DE290-123F-4565-9164-39C4925E467B")) ?? Path.Combine(home, "Downloads"), "download");
        Add("pictures", "Pictures", Environment.GetFolderPath(Environment.SpecialFolder.MyPictures), "image");
        Add("videos", "Videos", Environment.GetFolderPath(Environment.SpecialFolder.MyVideos), "video");
        Add("music", "Music", Environment.GetFolderPath(Environment.SpecialFolder.MyMusic), "audio");

        var games = new List<object>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var g in gameFolders ?? Array.Empty<string>())
        {
            try
            {
                if (!Directory.Exists(g) || !seen.Add(g)) continue;
                var root = Path.GetPathRoot(g)?.TrimEnd('\\') ?? "";
                var label = g.Contains(@"\steamapps\", StringComparison.OrdinalIgnoreCase) ? "Steam" : "Games";
                games.Add(new { id = "games:" + g, name = $"{label} ({root})", path = g, icon = "game" });
            }
            catch { }
        }

        var drives = new List<object>();
        foreach (var d in DriveInfo.GetDrives())
        {
            try
            {
                if (!d.IsReady) continue;
                string kind = d.DriveType switch
                {
                    DriveType.Removable => "usb", DriveType.Network => "network", DriveType.CDRom => "disc", _ => "drive",
                };
                string letter = d.Name.TrimEnd('\\');
                string label = string.IsNullOrWhiteSpace(d.VolumeLabel) ? (kind == "usb" ? "USB drive" : kind == "network" ? "Network drive" : "Local disk") : d.VolumeLabel;
                drives.Add(new { id = "drive:" + d.Name, name = $"{label} ({letter})", path = d.Name, icon = kind, removable = d.DriveType == DriveType.Removable, total = d.TotalSize, free = d.AvailableFreeSpace });
            }
            catch { }
        }
        return new { home, folders, games, drives, recycle = new { id = "recycle", name = "Recycle Bin", path = RecyclePath, icon = "trash", count = RecycleCount() } };
    }

    private static string? KnownFolder(Guid id)
    {
        try
        {
            if (SHGetKnownFolderPath(id, 0, IntPtr.Zero, out var p) != 0) return null;
            try { return Marshal.PtrToStringUni(p); } finally { Marshal.FreeCoTaskMem(p); }
        }
        catch { return null; }
    }

    // ------------------------------------------------------------------ listing

    public sealed record Entry(string Name, string Path, bool Dir, long Size, long Modified, string Ext, bool Hidden, string Kind, string? Original = null, long? Deleted = null);

    public object List(string path, bool showHidden)
    {
        if (path == RecyclePath) return new { path, name = "Recycle Bin", parent = (string?)null, entries = RecycleList(), recycle = true };
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("No folder");
        path = Full(path);
        var dir = new DirectoryInfo(path);
        if (!dir.Exists) throw new DirectoryNotFoundException("That folder doesn't exist any more");
        var list = new List<Entry>();
        try
        {
            foreach (var i in dir.EnumerateFileSystemInfos("*", new EnumerationOptions { IgnoreInaccessible = true, AttributesToSkip = 0, RecurseSubdirectories = false }))
            {
                bool hidden = (i.Attributes & (FileAttributes.Hidden | FileAttributes.System)) != 0;
                if (hidden && !showHidden) continue;
                bool isDir = (i.Attributes & FileAttributes.Directory) != 0;
                long size = isDir ? 0 : ((FileInfo)i).Length;
                var ext = isDir ? "" : i.Extension.TrimStart('.').ToLowerInvariant();
                list.Add(new Entry(i.Name, i.FullName, isDir, size, ToMs(i.LastWriteTimeUtc), ext, hidden, isDir ? "folder" : KindOf(ext)));
                if (list.Count >= 20000) break;
            }
        }
        catch (UnauthorizedAccessException) { throw new UnauthorizedAccessException("You don't have permission to open this folder"); }
        return new { path = dir.FullName, name = dir.Parent is null ? dir.FullName : dir.Name, parent = dir.Parent?.FullName, entries = list, recycle = false };
    }

    private static long ToMs(DateTime utc) => utc.Year < 1971 ? 0 : new DateTimeOffset(DateTime.SpecifyKind(utc, DateTimeKind.Utc)).ToUnixTimeMilliseconds();

    private static readonly Dictionary<string, string> Kinds = new(StringComparer.OrdinalIgnoreCase)
    {
        ["jpg"] = "image", ["jpeg"] = "image", ["png"] = "image", ["gif"] = "image", ["bmp"] = "image", ["webp"] = "image", ["heic"] = "image", ["tif"] = "image", ["tiff"] = "image", ["ico"] = "image", ["svg"] = "image", ["avif"] = "image",
        ["mp4"] = "video", ["mkv"] = "video", ["webm"] = "video", ["mov"] = "video", ["avi"] = "video", ["wmv"] = "video", ["m4v"] = "video",
        ["mp3"] = "audio", ["flac"] = "audio", ["wav"] = "audio", ["ogg"] = "audio", ["m4a"] = "audio", ["aac"] = "audio", ["opus"] = "audio", ["wma"] = "audio",
        ["zip"] = "archive", ["7z"] = "archive", ["rar"] = "archive", ["tar"] = "archive", ["gz"] = "archive", ["xz"] = "archive", ["iso"] = "archive",
        ["exe"] = "app", ["msi"] = "app", ["lnk"] = "app", ["url"] = "app", ["bat"] = "app", ["cmd"] = "app", ["appx"] = "app", ["msix"] = "app",
        ["pdf"] = "doc", ["doc"] = "doc", ["docx"] = "doc", ["txt"] = "doc", ["md"] = "doc", ["rtf"] = "doc", ["odt"] = "doc", ["xls"] = "doc", ["xlsx"] = "doc", ["ppt"] = "doc", ["pptx"] = "doc", ["csv"] = "doc", ["log"] = "doc", ["json"] = "doc", ["ini"] = "doc", ["xml"] = "doc",
    };
    private static string KindOf(string ext) => Kinds.TryGetValue(ext, out var k) ? k : "file";

    /// <summary>Search below a folder for names containing the text (up to 500, at most 10 seconds).</summary>
    public async Task<object> Search(string path, string query, bool showHidden)
    {
        path = Full(path);
        query = query.Trim();
        if (query.Length == 0) return new { entries = Array.Empty<Entry>(), truncated = false };
        var found = new List<Entry>();
        bool truncated = false;
        await Task.Run(() =>
        {
            var until = DateTime.UtcNow.AddSeconds(10);
            var opts = new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = showHidden ? (FileAttributes)0 : FileAttributes.Hidden | FileAttributes.System };
            try
            {
                foreach (var i in new DirectoryInfo(path).EnumerateFileSystemInfos("*", opts))
                {
                    if (DateTime.UtcNow > until || found.Count >= 500) { truncated = true; break; }
                    if (!i.Name.Contains(query, StringComparison.CurrentCultureIgnoreCase)) continue;
                    bool isDir = (i.Attributes & FileAttributes.Directory) != 0;
                    var ext = isDir ? "" : i.Extension.TrimStart('.').ToLowerInvariant();
                    long size = 0;
                    try { size = isDir ? 0 : ((FileInfo)i).Length; } catch { }
                    found.Add(new Entry(i.Name, i.FullName, isDir, size, ToMs(i.LastWriteTimeUtc), ext, (i.Attributes & FileAttributes.Hidden) != 0, isDir ? "folder" : KindOf(ext)));
                }
            }
            catch { }
        });
        return new { entries = found, truncated };
    }

    // ------------------------------------------------------------------ open

    public void Open(string path)
    {
        path = Full(path);
        if (!File.Exists(path) && !Directory.Exists(path)) throw new FileNotFoundException("It isn't there any more");
        Process.Start(new ProcessStartInfo(path) { UseShellExecute = true, WorkingDirectory = Path.GetDirectoryName(path) ?? "" })?.Dispose();
    }

    /// <summary>Windows' "How do you want to open this file?" dialog (on its own thread: it waits for you).</summary>
    public void OpenWith(string path, IntPtr owner)
    {
        path = Full(path);
        if (!File.Exists(path)) throw new FileNotFoundException("Open with… is for files");
        var t = new Thread(() =>
        {
            try
            {
                var info = new OPENASINFO { pcszFile = path, pcszClass = null, oaifInFlags = 0x1 /* ALLOW_REGISTRATION */ | 0x4 /* EXEC */ | 0x20 /* HIDE_REGISTRATION */ };
                int hr = SHOpenWithDialog(owner, ref info);
                if (hr < 0 && hr != unchecked((int)0x800704C7) /* cancelled */) throw new InvalidOperationException("SHOpenWithDialog " + hr);
            }
            catch (Exception ex)
            {
                SystemBridge.Log("files: open with: " + ex.Message);
                try { Process.Start(new ProcessStartInfo("rundll32.exe", "shell32.dll,OpenAs_RunDLL " + path) { UseShellExecute = false })?.Dispose(); } catch { }
            }
        }) { IsBackground = true };
        t.SetApartmentState(ApartmentState.STA);
        t.Start();
    }

    /// <summary>Windows' own Properties sheet (details, security, previous versions…).</summary>
    public void WindowsProperties(string path, IntPtr owner)
    {
        path = Full(path);
        if (!SHObjectProperties(owner, 0x2 /* SHOP_FILEPATH */, path, null)) throw new InvalidOperationException("Windows couldn't show its properties");
    }

    /// <summary>What Hearth's properties panel shows. A folder's size is added up (for at most 3 seconds).</summary>
    public async Task<object> Info(string path)
    {
        path = Full(path);
        bool dir = Directory.Exists(path);
        FileSystemInfo fi = dir ? new DirectoryInfo(path) : new FileInfo(path);
        if (!fi.Exists) throw new FileNotFoundException("It isn't there any more");
        long size = 0; int files = 0, folders = 0; bool partial = false;
        if (dir)
        {
            await Task.Run(() =>
            {
                var until = DateTime.UtcNow.AddSeconds(3);
                try
                {
                    foreach (var i in ((DirectoryInfo)fi).EnumerateFileSystemInfos("*", new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = 0 }))
                    {
                        if (DateTime.UtcNow > until) { partial = true; break; }
                        if ((i.Attributes & FileAttributes.Directory) != 0) folders++;
                        else { files++; try { size += ((FileInfo)i).Length; } catch { } }
                    }
                }
                catch { partial = true; }
            });
        }
        else size = ((FileInfo)fi).Length;
        string? type = null, version = null, company = null;
        if (!dir)
        {
            type = TypeName(path);
            if (fi.Extension.Equals(".exe", StringComparison.OrdinalIgnoreCase) || fi.Extension.Equals(".dll", StringComparison.OrdinalIgnoreCase))
            {
                try { var v = FileVersionInfo.GetVersionInfo(path); version = v.FileVersion; company = v.CompanyName; } catch { }
            }
        }
        DriveInfo? drive = null;
        try { drive = new DriveInfo(Path.GetPathRoot(path)!); } catch { }
        return new
        {
            name = fi.Name.Length > 0 ? fi.Name : fi.FullName,
            path = fi.FullName,
            location = Path.GetDirectoryName(fi.FullName),
            dir,
            type = dir ? "Folder" : type,
            size, files, folders, partial,
            created = ToMs(fi.CreationTimeUtc),
            modified = ToMs(fi.LastWriteTimeUtc),
            accessed = ToMs(fi.LastAccessTimeUtc),
            hidden = (fi.Attributes & FileAttributes.Hidden) != 0,
            readOnly = (fi.Attributes & FileAttributes.ReadOnly) != 0,
            version, company,
            driveFree = drive is { IsReady: true } ? drive.AvailableFreeSpace : (long?)null,
            driveTotal = drive is { IsReady: true } ? drive.TotalSize : (long?)null,
        };
    }

    private static string? TypeName(string path)
    {
        var info = new SHFILEINFO();
        var r = SHGetFileInfo(path, 0, ref info, (uint)Marshal.SizeOf<SHFILEINFO>(), 0x400 /* SHGFI_TYPENAME */);
        return r == IntPtr.Zero ? null : info.szTypeName;
    }

    // ------------------------------------------------------------------ simple changes

    public object NewFolder(string parent, string? name)
    {
        parent = Full(parent);
        var baseName = string.IsNullOrWhiteSpace(name) ? "New folder" : CheckName(name);
        var target = Unique(Path.Combine(parent, baseName));
        Directory.CreateDirectory(target);
        return new { path = target, name = Path.GetFileName(target) };
    }

    public object Rename(string path, string newName)
    {
        path = Full(path);
        newName = CheckName(newName);
        var parent = Path.GetDirectoryName(path) ?? throw new InvalidOperationException("A drive can't be renamed here");
        var target = Path.Combine(parent, newName);
        bool dir = Directory.Exists(path);
        if (!dir && !File.Exists(path)) throw new FileNotFoundException("It isn't there any more");
        if (string.Equals(path, target, StringComparison.Ordinal)) return new { path = target };
        bool caseOnly = string.Equals(path, target, StringComparison.OrdinalIgnoreCase);
        if (!caseOnly && (File.Exists(target) || Directory.Exists(target))) throw new IOException($"There's already something called \"{newName}\" here");
        if (caseOnly)
        {
            // Windows ignores case: go through a temporary name.
            var tmp = Path.Combine(parent, newName + ".hearth-rename-" + Environment.TickCount64);
            if (dir) { Directory.Move(path, tmp); Directory.Move(tmp, target); } else { File.Move(path, tmp); File.Move(tmp, target); }
        }
        else if (dir) Directory.Move(path, target);
        else File.Move(path, target);
        return new { path = target };
    }

    private static string CheckName(string name)
    {
        name = name.Trim();
        if (name.Length == 0 || name == "." || name == "..") throw new ArgumentException("Give it a name");
        if (name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0) throw new ArgumentException("A name can't contain \\ / : * ? \" < > |");
        if (name.EndsWith('.') || name.EndsWith(' ')) throw new ArgumentException("A name can't end with a dot or a space");
        var stem = Path.GetFileNameWithoutExtension(name).ToUpperInvariant();
        if (stem is "CON" or "PRN" or "AUX" or "NUL" || (stem.Length == 4 && (stem.StartsWith("COM") || stem.StartsWith("LPT")) && char.IsDigit(stem[3])))
            throw new ArgumentException("Windows keeps that name for itself");
        return name;
    }

    /// <summary>"name.ext", or "name (2).ext", "name (3).ext"… if that's taken.</summary>
    private static string Unique(string path)
    {
        if (!File.Exists(path) && !Directory.Exists(path)) return path;
        var dir = Path.GetDirectoryName(path)!;
        bool isDir = Directory.Exists(path);
        var stem = isDir ? Path.GetFileName(path) : Path.GetFileNameWithoutExtension(path);
        var ext = isDir ? "" : Path.GetExtension(path);
        for (int i = 2; i < 10000; i++)
        {
            var p = Path.Combine(dir, $"{stem} ({i}){ext}");
            if (!File.Exists(p) && !Directory.Exists(p)) return p;
        }
        throw new IOException("Couldn't find a free name");
    }

    /// <summary>Moves to the Recycle Bin (Windows' own way, so Restore works), quietly.</summary>
    public async Task Trash(string[] paths)
    {
        var list = paths.Select(Full).Where(p => File.Exists(p) || Directory.Exists(p)).ToArray();
        if (list.Length == 0) return;
        foreach (var p in list) if (Path.GetPathRoot(p) == p) throw new InvalidOperationException("A whole drive can't go to the Recycle Bin");
        int rc = await Task.Run(() =>
        {
            var op = new SHFILEOPSTRUCT
            {
                hwnd = IntPtr.Zero,
                wFunc = FO_DELETE,
                pFrom = string.Join("\0", list) + "\0\0",
                pTo = null,
                fFlags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI | FOF_WANTNUKEWARNING,
            };
            int r = SHFileOperation(ref op);
            return r == 0 && op.fAnyOperationsAborted ? -1 : r;
        });
        if (rc == -1) throw new OperationCanceledException("Stopped");
        if (rc != 0) throw new IOException($"Windows couldn't move it to the Recycle Bin (error 0x{rc:X})");
    }

    /// <summary>Deletes for good (the page asks first).</summary>
    public async Task DeleteForever(string[] paths)
    {
        var errors = new List<string>();
        await Task.Run(() =>
        {
            foreach (var raw in paths)
            {
                try
                {
                    if (raw.StartsWith(RecyclePath)) { RecycleDelete(raw[RecyclePath.Length..]); continue; }
                    var p = Full(raw);
                    if (Path.GetPathRoot(p) == p) throw new InvalidOperationException("A whole drive can't be deleted");
                    if (Directory.Exists(p)) { ClearReadOnly(p); Directory.Delete(p, true); }
                    else if (File.Exists(p)) { File.SetAttributes(p, FileAttributes.Normal); File.Delete(p); }
                }
                catch (Exception ex) { errors.Add($"{Path.GetFileName(raw)}: {ex.Message}"); }
            }
        });
        if (errors.Count > 0) throw new IOException(string.Join("; ", errors.Take(3)));
    }

    private static void ClearReadOnly(string dir)
    {
        try
        {
            foreach (var f in Directory.EnumerateFiles(dir, "*", new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = 0 }))
                if ((File.GetAttributes(f) & FileAttributes.ReadOnly) != 0) File.SetAttributes(f, FileAttributes.Normal);
        }
        catch { }
    }

    // ------------------------------------------------------------------ clipboard (shared with Explorer and other apps)

    /// <summary>Puts files on Windows' clipboard, as Explorer does (cut = "move" drop effect). UI thread only.</summary>
    public void SetClipboard(string[] paths, bool cut)
    {
        var files = new StringCollection();
        foreach (var p in paths) files.Add(Full(p));
        var data = new DataObject();
        data.SetFileDropList(files);
        data.SetData("Preferred DropEffect", new MemoryStream(BitConverter.GetBytes(cut ? 2 : 5)));
        Clipboard.SetDataObject(data, true);
    }

    /// <summary>The files on Windows' clipboard and whether they were cut. UI thread only.</summary>
    public object GetClipboard()
    {
        try
        {
            if (!Clipboard.ContainsFileDropList()) return new { paths = Array.Empty<string>(), cut = false };
            var list = Clipboard.GetFileDropList().Cast<string>().ToArray();
            bool cut = false;
            if (Clipboard.GetData("Preferred DropEffect") is MemoryStream ms && ms.Length >= 4)
            {
                var b = new byte[4]; ms.Position = 0; ms.Read(b, 0, 4);
                cut = (BitConverter.ToInt32(b, 0) & 2) != 0;
            }
            return new { paths = list, cut };
        }
        catch { return new { paths = Array.Empty<string>(), cut = false }; }
    }

    public void ClearClipboard() { try { Clipboard.Clear(); } catch { } }

    // ------------------------------------------------------------------ copy and move, with progress

    /// <summary>Starts copying (or moving) into a folder. Returns the job id; progress comes as events.</summary>
    public int StartTransfer(string[] sources, string destDir, bool move)
    {
        destDir = Full(destDir);
        if (!Directory.Exists(destDir)) throw new DirectoryNotFoundException("The folder to paste into isn't there");
        var src = sources.Select(Full).Where(p => File.Exists(p) || Directory.Exists(p)).Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
        if (src.Length == 0) throw new FileNotFoundException("Nothing to paste (the files were moved or deleted)");
        foreach (var s in src)
        {
            var withSep = s.TrimEnd('\\') + "\\";
            if (Directory.Exists(s) && (destDir.TrimEnd('\\') + "\\").StartsWith(withSep, StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("A folder can't be put inside itself");
        }
        var cts = new CancellationTokenSource();
        int id;
        lock (_jobs) { id = ++_jobId; _jobs[id] = cts; }
        _ = Task.Run(() => RunTransfer(id, src, destDir, move, cts.Token));
        return id;
    }

    public bool Cancel(int id)
    {
        lock (_jobs) { if (_jobs.TryGetValue(id, out var c)) { c.Cancel(); return true; } }
        return false;
    }

    private void RunTransfer(int id, string[] sources, string destDir, bool move, CancellationToken ct)
    {
        string verb = move ? "Moving" : "Copying";
        long total = 0, done = 0;
        int count = 0;
        var sw = Stopwatch.StartNew();
        long lastReport = -1000;
        string? error = null;
        bool cancelled = false;
        var created = new List<string>();
        void Report(string current, bool force = false)
        {
            if (!force && sw.ElapsedMilliseconds - lastReport < 200) return;
            lastReport = sw.ElapsedMilliseconds;
            double secs = Math.Max(0.001, sw.Elapsed.TotalSeconds);
            _event("files.progress", new { id, op = move ? "move" : "copy", done, total, count, current, text = $"{verb} {current}", bytesPerSec = (long)(done / secs) });
        }
        try
        {
            string destRoot = Path.GetPathRoot(destDir) ?? "";
            // Work out the size first (moves within a drive are instant renames and don't count).
            var plan = new List<(string Src, bool Rename)>();
            foreach (var s in sources)
            {
                bool sameDrive = string.Equals(Path.GetPathRoot(s), destRoot, StringComparison.OrdinalIgnoreCase);
                bool rename = move && sameDrive;
                plan.Add((s, rename));
                if (!rename) total += SizeOf(s, ct);
            }
            Report("…", true);
            foreach (var (s, rename) in plan)
            {
                ct.ThrowIfCancellationRequested();
                var name = Path.GetFileName(s.TrimEnd('\\'));
                if (string.IsNullOrEmpty(name)) name = s.TrimEnd('\\', ':');   // a whole drive: a folder named after it
                var target = Path.Combine(destDir, name);
                // Pasting into the same folder (or over something with that name) keeps both: "name (2)".
                if (string.Equals(Path.GetFullPath(s).TrimEnd('\\'), Path.GetFullPath(target).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))
                {
                    if (move) continue;   // moving into where it already is: nothing to do
                    target = Unique(target);
                }
                else target = Unique(target);
                if (rename)
                {
                    if (Directory.Exists(s)) Directory.Move(s, target); else File.Move(s, target);
                    count++;
                    Report(name);
                    continue;
                }
                created.Add(target);
                if (Directory.Exists(s)) CopyDir(s, target, ref done, ref count, ct, Report);
                else CopyFile(s, target, ref done, ct, Report);
                count++;
                if (move)
                {
                    if (Directory.Exists(s)) { ClearReadOnly(s); Directory.Delete(s, true); }
                    else { File.SetAttributes(s, FileAttributes.Normal); File.Delete(s); }
                }
                created.Remove(target);
            }
        }
        catch (OperationCanceledException) { cancelled = true; }
        catch (Exception ex) { error = ex.Message; }
        if (cancelled)
        {
            // Take away the half-copied item (whatever finished before stays, like Explorer).
            foreach (var c in created)
            {
                try { if (Directory.Exists(c)) Directory.Delete(c, true); else if (File.Exists(c)) File.Delete(c); } catch { }
            }
        }
        lock (_jobs) { if (_jobs.Remove(id, out var cts)) cts.Dispose(); }
        Report("", true);
        _event("files.done", new { id, op = move ? "move" : "copy", ok = error is null && !cancelled, cancelled, error, dest = destDir, count });
    }

    private static long SizeOf(string path, CancellationToken ct)
    {
        if (File.Exists(path)) { try { return new FileInfo(path).Length; } catch { return 0; } }
        long n = 0;
        foreach (var f in new DirectoryInfo(path).EnumerateFiles("*", new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = 0 }))
        {
            ct.ThrowIfCancellationRequested();
            try { n += f.Length; } catch { }
        }
        return n;
    }

    private delegate void ReportFn(string current, bool force = false);

    private static void CopyDir(string src, string dst, ref long done, ref int count, CancellationToken ct, ReportFn report)
    {
        Directory.CreateDirectory(dst);
        foreach (var d in Directory.EnumerateDirectories(src))
        {
            ct.ThrowIfCancellationRequested();
            // Junctions and symbolic links (like "Application Data"): skip, don't follow.
            if ((File.GetAttributes(d) & FileAttributes.ReparsePoint) != 0) continue;
            CopyDir(d, Path.Combine(dst, Path.GetFileName(d)), ref done, ref count, ct, report);
        }
        foreach (var f in Directory.EnumerateFiles(src))
        {
            ct.ThrowIfCancellationRequested();
            CopyFile(f, Path.Combine(dst, Path.GetFileName(f)), ref done, ct, report);
            count++;
        }
    }

    private static void CopyFile(string src, string dst, ref long done, CancellationToken ct, ReportFn report)
    {
        var name = Path.GetFileName(src);
        var buffer = new byte[1 << 20];
        bool ok = false;
        try
        {
            using (var input = new FileStream(src, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete, 1 << 16, FileOptions.SequentialScan))
            using (var output = new FileStream(dst, FileMode.CreateNew, FileAccess.Write, FileShare.None, 1 << 16))
            {
                try { output.SetLength(input.Length); } catch { }
                int read;
                while ((read = input.Read(buffer, 0, buffer.Length)) > 0)
                {
                    ct.ThrowIfCancellationRequested();
                    output.Write(buffer, 0, read);
                    done += read;
                    report(name);
                }
            }
            try
            {
                File.SetLastWriteTimeUtc(dst, File.GetLastWriteTimeUtc(src));
                File.SetAttributes(dst, File.GetAttributes(src) & ~FileAttributes.ReadOnly);
            }
            catch { }
            ok = true;
        }
        finally
        {
            if (!ok) { try { File.Delete(dst); } catch { } }
        }
    }

    private static string Full(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("No path");
        path = Environment.ExpandEnvironmentVariables(path.Trim());
        var full = Path.GetFullPath(path);
        // "C:" alone means "the current folder on C:": make it the drive's root.
        if (full.Length == 2 && full[1] == ':') full += "\\";
        return full;
    }

    // ------------------------------------------------------------------ the Recycle Bin

    /// <summary>Your recycle bin folders on every drive: X:\$Recycle.Bin\&lt;your SID&gt;. Each deleted item is a
    /// pair: $R… (the item) and $I… (where it came from, when, and its size).</summary>
    private static IEnumerable<string> RecycleFolders()
    {
        string? sid = null;
        try { using var id = WindowsIdentity.GetCurrent(); sid = id.User?.Value; } catch { }
        if (sid is null) yield break;
        foreach (var d in DriveInfo.GetDrives())
        {
            string? p = null;
            try { if (d.IsReady && d.DriveType is DriveType.Fixed or DriveType.Removable) p = Path.Combine(d.Name, "$Recycle.Bin", sid); } catch { }
            if (p is not null && Directory.Exists(p)) yield return p;
        }
    }

    private static int RecycleCount()
    {
        int n = 0;
        foreach (var f in RecycleFolders()) { try { n += Directory.EnumerateFiles(f, "$I*").Count(); } catch { } }
        return n;
    }

    private static List<Entry> RecycleList()
    {
        var list = new List<Entry>();
        foreach (var folder in RecycleFolders())
        {
            IEnumerable<string> infos;
            try { infos = Directory.EnumerateFiles(folder, "$I*").ToList(); } catch { continue; }
            foreach (var info in infos)
            {
                var (original, size, deleted) = ReadInfo(info);
                if (original is null) continue;
                var item = Path.Combine(folder, "$R" + Path.GetFileName(info)[2..]);
                bool dir = Directory.Exists(item);
                if (!dir && !File.Exists(item)) continue;
                var name = Path.GetFileName(original.TrimEnd('\\'));
                var ext = dir ? "" : Path.GetExtension(name).TrimStart('.').ToLowerInvariant();
                list.Add(new Entry(name, RecyclePath + item, dir, dir ? 0 : size, ToMs(deleted), ext, false, dir ? "folder" : KindOf(ext), original, ToMs(deleted)));
            }
        }
        return list;
    }

    /// <summary>$I file: version (8 bytes), size (8), deletion time (8, FILETIME), then the original path: version 1 a
    /// fixed 260 UTF-16 characters, version 2 a length (4) and that many characters.</summary>
    private static (string? Original, long Size, DateTime Deleted) ReadInfo(string infoFile)
    {
        try
        {
            var b = File.ReadAllBytes(infoFile);
            if (b.Length < 24) return (null, 0, default);
            long version = BitConverter.ToInt64(b, 0), size = BitConverter.ToInt64(b, 8);
            var when = DateTime.FromFileTimeUtc(BitConverter.ToInt64(b, 16));
            string path;
            if (version >= 2 && b.Length >= 28)
            {
                int chars = BitConverter.ToInt32(b, 24);
                path = System.Text.Encoding.Unicode.GetString(b, 28, Math.Min(chars * 2, b.Length - 28));
            }
            else path = System.Text.Encoding.Unicode.GetString(b, 24, Math.Min(520, b.Length - 24));
            return (path.TrimEnd('\0'), size, when);
        }
        catch { return (null, 0, default); }
    }

    private static (string Item, string Info) RecyclePair(string item)
    {
        var full = Path.GetFullPath(item);
        var name = Path.GetFileName(full);
        var folder = Path.GetDirectoryName(full) ?? "";
        if (!name.StartsWith("$R") || !folder.Contains(@"\$Recycle.Bin\", StringComparison.OrdinalIgnoreCase)) throw new ArgumentException("Not in the Recycle Bin");
        return (full, Path.Combine(folder, "$I" + name[2..]));
    }

    /// <summary>Puts recycled items back where they were (or "name (2)" if something's there now).</summary>
    public async Task<object> Restore(string[] items)
    {
        int restored = 0;
        var errors = new List<string>();
        await Task.Run(() =>
        {
            foreach (var raw in items)
            {
                try
                {
                    var (item, info) = RecyclePair(raw.StartsWith(RecyclePath) ? raw[RecyclePath.Length..] : raw);
                    var (original, _, _) = ReadInfo(info);
                    if (original is null) throw new IOException("Windows didn't record where it came from");
                    Directory.CreateDirectory(Path.GetDirectoryName(original)!);
                    var target = Unique(original);
                    bool sameDrive = string.Equals(Path.GetPathRoot(item), Path.GetPathRoot(target), StringComparison.OrdinalIgnoreCase);
                    if (Directory.Exists(item))
                    {
                        if (sameDrive) Directory.Move(item, target);
                        else throw new IOException("It belongs on another drive");
                    }
                    else File.Move(item, target);
                    try { File.Delete(info); } catch { }
                    restored++;
                }
                catch (Exception ex) { errors.Add(ex.Message); }
            }
        });
        if (restored == 0 && errors.Count > 0) throw new IOException(errors[0]);
        return new { restored, errors };
    }

    private static void RecycleDelete(string item)
    {
        var (full, info) = RecyclePair(item);
        if (Directory.Exists(full)) { ClearReadOnly(full); Directory.Delete(full, true); }
        else if (File.Exists(full)) { File.SetAttributes(full, FileAttributes.Normal); File.Delete(full); }
        try { File.Delete(info); } catch { }
    }

    public async Task EmptyRecycleBin()
    {
        int hr = await Task.Run(() => SHEmptyRecycleBin(IntPtr.Zero, null, 0x1 /* NOCONFIRMATION */ | 0x2 /* NOPROGRESSUI */ | 0x4 /* NOSOUND */));
        if (hr < 0 && hr != unchecked((int)0x8000FFFF) /* already empty */) throw new IOException($"Windows couldn't empty the Recycle Bin (0x{hr:X8})");
    }

    // ------------------------------------------------------------------ native

    private const uint FO_DELETE = 3;
    private const ushort FOF_SILENT = 0x4, FOF_NOCONFIRMATION = 0x10, FOF_ALLOWUNDO = 0x40, FOF_NOERRORUI = 0x400, FOF_WANTNUKEWARNING = 0x4000;

    // 64-bit layout (Ember is x64 / ARM64 only), so the default packing is right.
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SHFILEOPSTRUCT
    {
        public IntPtr hwnd;
        public uint wFunc;
        public string? pFrom;
        public string? pTo;
        public ushort fFlags;
        [MarshalAs(UnmanagedType.Bool)] public bool fAnyOperationsAborted;
        public IntPtr hNameMappings;
        public string? lpszProgressTitle;
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern int SHFileOperation(ref SHFILEOPSTRUCT op);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct OPENASINFO
    {
        public string? pcszFile;
        public string? pcszClass;
        public int oaifInFlags;
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern int SHOpenWithDialog(IntPtr hwnd, ref OPENASINFO info);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SHObjectProperties(IntPtr hwnd, uint type, string obj, string? page);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern int SHEmptyRecycleBin(IntPtr hwnd, string? root, uint flags);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SHFILEINFO
    {
        public IntPtr hIcon;
        public int iIcon;
        public uint dwAttributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string szDisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)] public string szTypeName;
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr SHGetFileInfo(string path, uint attrs, ref SHFILEINFO info, uint size, uint flags);

    [DllImport("shell32.dll")] private static extern int SHGetKnownFolderPath([MarshalAs(UnmanagedType.LPStruct)] Guid id, uint flags, IntPtr token, out IntPtr path);
}
