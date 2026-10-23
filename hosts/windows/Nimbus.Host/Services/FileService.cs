using System.Collections.Specialized;
using System.Diagnostics;
using System.Formats.Tar;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text.RegularExpressions;

namespace Nimbus.Host.Services;

/// <summary>
/// Hearth Files on Windows (mantel-files.html), like Mantel's file manager on Hearth OS: places (your folders,
/// game libraries, drives, the Recycle Bin), folder listings, opening with the default app or another one,
/// copy / cut / paste with progress and cancel (Replace / Skip / Keep both for names already there; replacing a
/// folder merges into it), rename and batch rename, Recycle Bin, delete, new folder / file / from a template,
/// compress and extract (zip, tar, tar.gz), search with live results, open a terminal there, and the full
/// properties panel (size on disk, owner, attributes, picture size, length, default app, checksums, live
/// folder sizes). Thumbnails are served to the page by FilesWindow (https://hearth-files.local/thumb?p=…).
/// Long work runs in the background and reports through events: files.progress / files.done (copy, move,
/// compress, extract), files.count (folder sizes), files.found / files.searchDone (search), files.sumProgress.
/// </summary>
public sealed class FileService
{
    public const string RecyclePath = "recycle:";
    private readonly Action<string, object> _event;
    private readonly Dictionary<int, CancellationTokenSource> _jobs = new();
    private readonly Dictionary<string, CancellationTokenSource> _sums = new();
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

    // ------------------------------------------------------------------ the full properties panel

    /// <summary>Everything Hearth's Properties panel shows about one item, or a summary of several. Folder sizes
    /// come separately and live (StartCount), checksums on demand (Checksum).</summary>
    public async Task<object> Props(string[] paths)
    {
        if (paths.Length == 1) return await Task.Run(() => PropsOne(paths[0]));
        return await Task.Run(() =>
        {
            var items = new List<object>();
            var parents = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            var types = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            int files = 0, folders = 0;
            foreach (var raw in paths)
            {
                string p;
                try { p = Full(raw); } catch { continue; }
                bool dir = Directory.Exists(p);
                if (!dir && !File.Exists(p)) continue;
                if (dir) folders++; else files++;
                parents.Add(Path.GetDirectoryName(p) ?? "");
                types.Add(dir ? "Folder" : TypeName(p) ?? Path.GetExtension(p));
                items.Add(new { name = Path.GetFileName(p), path = p, dir });
            }
            if (items.Count == 0) throw new FileNotFoundException("These aren't there any more");
            DriveInfo? drive = null;
            try { drive = new DriveInfo(Path.GetPathRoot(Full(paths[0]))!); } catch { }
            return (object)new
            {
                multi = true, count = items.Count, items, files, folders,
                location = parents.Count == 1 ? parents.First() : null,
                type = types.Count == 1 ? types.First() : null,
                driveFree = drive is { IsReady: true } ? drive.AvailableFreeSpace : (long?)null,
                driveTotal = drive is { IsReady: true } ? drive.TotalSize : (long?)null,
            };
        });
    }

    private object PropsOne(string raw)
    {
        var path = Full(raw);
        bool dir = Directory.Exists(path);
        FileSystemInfo fi = dir ? new DirectoryInfo(path) : new FileInfo(path);
        if (!fi.Exists) throw new FileNotFoundException("It isn't there any more");
        var root = Path.GetPathRoot(path) ?? "";
        bool isRoot = dir && string.Equals(root.TrimEnd('\\'), path.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase);
        long size = dir ? 0 : ((FileInfo)fi).Length;
        string? target = null;
        try { target = fi.LinkTarget; } catch { }
        string? type = dir ? (isRoot ? "Drive" : "File folder") : TypeName(path);
        string? version = null, company = null, app = null;
        int? width = null, height = null;
        double? duration = null;
        if (!dir)
        {
            var ext = fi.Extension.ToLowerInvariant();
            if (ext is ".exe" or ".dll")
            {
                try { var v = FileVersionInfo.GetVersionInfo(path); version = v.FileVersion; company = v.CompanyName; } catch { }
            }
            (width, height, duration) = MediaProps(path);
            app = DefaultAppName(fi.Extension);
        }
        DriveInfo? drive = null;
        try { drive = new DriveInfo(root); } catch { }
        bool ready = drive is { IsReady: true };
        return new
        {
            name = fi.Name.Length > 0 ? fi.Name : fi.FullName,
            path = fi.FullName,
            location = Path.GetDirectoryName(fi.FullName),
            dir, isRoot,
            ext = dir ? "" : fi.Extension.TrimStart('.').ToLowerInvariant(),
            kind = dir ? "folder" : KindOf(fi.Extension.TrimStart('.')),
            type,
            size,
            disk = dir ? (long?)null : SizeOnDisk(path, size),
            created = ToMs(fi.CreationTimeUtc),
            modified = ToMs(fi.LastWriteTimeUtc),
            accessed = ToMs(fi.LastAccessTimeUtc),
            readOnly = (fi.Attributes & FileAttributes.ReadOnly) != 0,
            hidden = (fi.Attributes & FileAttributes.Hidden) != 0,
            system = (fi.Attributes & FileAttributes.System) != 0,
            owner = Owner(fi),
            target, width, height, duration, app, version, company,
            canRename = !isRoot,
            driveFree = ready ? drive!.AvailableFreeSpace : (long?)null,
            driveTotal = ready ? drive!.TotalSize : (long?)null,
            driveFormat = ready ? drive!.DriveFormat : null,
            driveLabel = ready ? drive!.VolumeLabel : null,
            driveKind = drive?.DriveType.ToString(),
        };
    }

    /// <summary>The space a file really takes (compressed / sparse files less, everything rounded up to clusters).</summary>
    private static long SizeOnDisk(string path, long size)
    {
        try
        {
            uint lo = GetCompressedFileSizeW(path, out uint hi);
            if (lo == 0xFFFFFFFF && Marshal.GetLastWin32Error() != 0) return size;
            long n = ((long)hi << 32) | lo;
            var root = Path.GetPathRoot(path);
            if (root is not null && GetDiskFreeSpaceW(root, out uint spc, out uint bps, out _, out _))
            {
                long cluster = (long)spc * bps;
                if (cluster > 0) n = (n + cluster - 1) / cluster * cluster;
            }
            return n;
        }
        catch { return size; }
    }

    private static string? Owner(FileSystemInfo fi)
    {
        try
        {
            FileSystemSecurity sec = fi is DirectoryInfo d ? d.GetAccessControl(AccessControlSections.Owner) : ((FileInfo)fi).GetAccessControl(AccessControlSections.Owner);
            return sec.GetOwner(typeof(NTAccount))?.Value;
        }
        catch { return null; }
    }

    /// <summary>A picture's width and height and a song's or video's length, from Windows' property system.</summary>
    private static (int?, int?, double?) MediaProps(string path)
    {
        int? w = null, h = null;
        double? secs = null;
        try
        {
            var iid = typeof(IPropertyStore).GUID;
            SHGetPropertyStoreFromParsingName(path, IntPtr.Zero, 0, ref iid, out var store);
            try
            {
                var imageFmt = new Guid("6444048F-4C8B-11D1-8B70-080036B11A03");
                uint? Read(Guid fmt, uint pid)
                {
                    var key = new PROPERTYKEY { fmtid = fmt, pid = pid };
                    store.GetValue(ref key, out var pv);
                    try
                    {
                        return pv.vt switch { 19 => pv.u4, 21 => (uint)Math.Min(uint.MaxValue, pv.u8 / 10_000_000UL), _ => null };
                    }
                    finally { PropVariantClear(ref pv); }
                }
                var hw = Read(imageFmt, 3);
                var hh = Read(imageFmt, 4);
                if (hw is > 0 && hh is > 0) { w = (int)hw; h = (int)hh; }
                var d = Read(new Guid("64440490-4C8B-11D1-8B70-080036B11A03"), 3);
                if (d is > 0) secs = d;
            }
            finally { Marshal.ReleaseComObject(store); }
        }
        catch { }
        return (w, h, secs);
    }

    /// <summary>The app that opens this kind of file ("Photos"), or null.</summary>
    private static string? DefaultAppName(string ext)
    {
        if (string.IsNullOrEmpty(ext)) return null;
        try
        {
            uint len = 0;
            AssocQueryString(0, 4 /* FRIENDLYAPPNAME */, ext, null, null, ref len);
            if (len == 0) return null;
            var sb = new System.Text.StringBuilder((int)len);
            return AssocQueryString(0, 4, ext, null, sb, ref len) == 0 ? sb.ToString() : null;
        }
        catch { return null; }
    }

    public void SetAttributes(string path, bool? readOnly, bool? hidden)
    {
        path = Full(path);
        if (!File.Exists(path) && !Directory.Exists(path)) throw new FileNotFoundException("It isn't there any more");
        var a = File.GetAttributes(path);
        if (readOnly is bool r) a = r ? a | FileAttributes.ReadOnly : a & ~FileAttributes.ReadOnly;
        if (hidden is bool hd) a = hd ? a | FileAttributes.Hidden : a & ~FileAttributes.Hidden;
        File.SetAttributes(path, a);
    }

    /// <summary>Windows' "How do you want to open .x files from now on?" (Always use this app); finishes when it closes.</summary>
    public Task ChangeDefaultApp(string path, IntPtr owner)
    {
        path = Full(path);
        if (!File.Exists(path)) throw new FileNotFoundException("Only files open with an app");
        var done = new TaskCompletionSource();
        var t = new Thread(() =>
        {
            try
            {
                var info = new OPENASINFO { pcszFile = path, pcszClass = null, oaifInFlags = 0x1 /* ALLOW_REGISTRATION */ | 0x2 /* REGISTER_EXT */ | 0x8 /* FORCE_REGISTRATION: "Always use" ticked */ };
                SHOpenWithDialog(owner, ref info);
            }
            catch (Exception ex) { SystemBridge.Log("files: change default app: " + ex.Message); }
            done.TrySetResult();
        }) { IsBackground = true };
        t.SetApartmentState(ApartmentState.STA);
        t.Start();
        return done.Task;
    }

    /// <summary>Adds up folders in the background: files.count events {id, size, files, folders, done}.</summary>
    public int StartCount(string[] paths)
    {
        var cts = new CancellationTokenSource();
        int id;
        lock (_jobs) { id = ++_jobId; _jobs[id] = cts; }
        var ct = cts.Token;
        _ = Task.Run(() =>
        {
            long size = 0;
            int files = 0, folders = 0;
            var sw = Stopwatch.StartNew();
            long last = 0;
            void Report(bool done) => _event("files.count", new { id, size, files, folders, done });
            try
            {
                foreach (var raw in paths)
                {
                    var p = Full(raw);
                    if (File.Exists(p)) { files++; size += SafeLength(p); continue; }
                    if (!Directory.Exists(p)) continue;
                    var opts = new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = FileAttributes.ReparsePoint };
                    foreach (var i in new DirectoryInfo(p).EnumerateFileSystemInfos("*", opts))
                    {
                        ct.ThrowIfCancellationRequested();
                        if ((i.Attributes & FileAttributes.Directory) != 0) folders++;
                        else { files++; try { size += ((FileInfo)i).Length; } catch { } }
                        if (sw.ElapsedMilliseconds - last > 250) { last = sw.ElapsedMilliseconds; Report(false); }
                    }
                }
                Report(true);
            }
            catch (OperationCanceledException) { }
            catch { Report(true); }
            finally { lock (_jobs) { if (_jobs.Remove(id, out var c)) c.Dispose(); } }
        });
        return id;
    }

    /// <summary>MD5, SHA-1 or SHA-256 of a file (files.sumProgress events on the way). CancelChecksums stops it.</summary>
    public async Task<object> Checksum(string path, string algo)
    {
        path = Full(path);
        if (!File.Exists(path)) throw new FileNotFoundException("Only files have checksums");
        algo = algo.ToLowerInvariant().Replace("-", "");
        if (algo is not ("md5" or "sha1" or "sha256")) throw new ArgumentException("Hearth Files knows MD5, SHA-1 and SHA-256");
        var key = path + "|" + algo;
        var cts = new CancellationTokenSource();
        lock (_sums) { if (_sums.Remove(key, out var old)) old.Cancel(); _sums[key] = cts; }
        try
        {
            return await Task.Run(() =>
            {
                using HashAlgorithm h = algo == "md5" ? MD5.Create() : algo == "sha1" ? SHA1.Create() : SHA256.Create();
                using var s = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete, 1 << 16, FileOptions.SequentialScan);
                var buf = new byte[1 << 20];
                long done = 0, total = s.Length, last = 0;
                var sw = Stopwatch.StartNew();
                int n;
                while ((n = s.Read(buf, 0, buf.Length)) > 0)
                {
                    cts.Token.ThrowIfCancellationRequested();
                    h.TransformBlock(buf, 0, n, null, 0);
                    done += n;
                    if (sw.ElapsedMilliseconds - last > 200) { last = sw.ElapsedMilliseconds; _event("files.sumProgress", new { path, algo, done, total }); }
                }
                h.TransformFinalBlock(Array.Empty<byte>(), 0, 0);
                return (object)new { algo, sum = Convert.ToHexString(h.Hash!).ToLowerInvariant() };
            }, cts.Token);
        }
        finally
        {
            lock (_sums) { if (_sums.TryGetValue(key, out var c) && c == cts) _sums.Remove(key); }
            cts.Dispose();
        }
    }

    public void CancelChecksums()
    {
        lock (_sums) { foreach (var c in _sums.Values) c.Cancel(); }
    }

    /// <summary>The start of a text file, for the quick preview (Space).</summary>
    public object Peek(string path)
    {
        path = Full(path);
        var buf = new byte[65536];
        int n;
        using (var s = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete)) n = s.Read(buf, 0, buf.Length);
        if (Array.IndexOf(buf, (byte)0, 0, Math.Min(n, 8192)) >= 0) throw new InvalidDataException("This isn't a text file");
        return new { text = System.Text.Encoding.UTF8.GetString(buf, 0, n), more = n == buf.Length };
    }

    // ------------------------------------------------------------------ new files, batch rename, terminal

    public object NewFile(string dir, string name)
    {
        dir = Full(dir);
        var target = Path.Combine(dir, CheckName(name));
        if (File.Exists(target) || Directory.Exists(target)) throw new IOException($"There's already something called \"{Path.GetFileName(target)}\" here");
        using (new FileStream(target, FileMode.CreateNew)) { }
        return new { path = target, name = Path.GetFileName(target) };
    }

    /// <summary>Document templates: yours and everyone's (the Templates folders), for New document › …</summary>
    public object Templates()
    {
        var list = new List<object>();
        foreach (var dir in new[] { Environment.GetFolderPath(Environment.SpecialFolder.Templates), Environment.GetFolderPath(Environment.SpecialFolder.CommonTemplates) })
        {
            try
            {
                if (string.IsNullOrEmpty(dir) || !Directory.Exists(dir)) continue;
                foreach (var f in Directory.EnumerateFiles(dir))
                {
                    var n = Path.GetFileName(f);
                    if (n.Equals("desktop.ini", StringComparison.OrdinalIgnoreCase) || (File.GetAttributes(f) & FileAttributes.Hidden) != 0) continue;
                    list.Add(new { name = Path.GetFileNameWithoutExtension(f), file = n, path = f });
                }
            }
            catch { }
        }
        return new { templates = list };
    }

    public object FromTemplate(string template, string dir, string? name)
    {
        template = Full(template);
        dir = Full(dir);
        if (!File.Exists(template)) throw new FileNotFoundException("That template isn't there any more");
        var target = Unique(Path.Combine(dir, CheckName(string.IsNullOrWhiteSpace(name) ? Path.GetFileName(template) : name)));
        File.Copy(template, target);
        File.SetAttributes(target, File.GetAttributes(target) & ~(FileAttributes.ReadOnly | FileAttributes.Hidden | FileAttributes.System));
        return new { path = target, name = Path.GetFileName(target) };
    }

    /// <summary>"Holiday #" → Holiday 1.jpg, Holiday 2.jpg… (## pads: 01). Extensions stay.</summary>
    public object BatchRename(string pattern, string[] paths)
    {
        if (!pattern.Contains('#')) pattern += " #";
        var m = Regex.Match(pattern, "#+");
        var plan = new List<(string From, string To)>();
        for (int i = 0; i < paths.Length; i++)
        {
            var p = Full(paths[i]);
            bool dir = Directory.Exists(p);
            if (!dir && !File.Exists(p)) throw new FileNotFoundException($"\"{Path.GetFileName(p)}\" isn't there any more");
            var name = CheckName(pattern[..m.Index] + (i + 1).ToString().PadLeft(m.Length, '0') + pattern[(m.Index + m.Length)..] + (dir ? "" : Path.GetExtension(p)));
            plan.Add((p, Path.Combine(Path.GetDirectoryName(p)!, name)));
        }
        var from = new HashSet<string>(plan.Select(x => x.From), StringComparer.OrdinalIgnoreCase);
        foreach (var (_, to) in plan)
            if ((File.Exists(to) || Directory.Exists(to)) && !from.Contains(to)) throw new IOException($"There's already something called \"{Path.GetFileName(to)}\" there");
        var temps = new List<(string Tmp, string From, string To)>();
        try
        {
            foreach (var (f, to) in plan)
            {
                var tmp = Path.Combine(Path.GetDirectoryName(f)!, ".hearth-rename-" + Guid.NewGuid().ToString("N")[..8]);
                if (Directory.Exists(f)) Directory.Move(f, tmp); else File.Move(f, tmp);
                temps.Add((tmp, f, to));
            }
            foreach (var (tmp, _, to) in temps) { if (Directory.Exists(tmp)) Directory.Move(tmp, to); else File.Move(tmp, to); }
        }
        catch
        {
            foreach (var (tmp, f, _) in temps)
            {
                try { if (Directory.Exists(tmp)) Directory.Move(tmp, f); else if (File.Exists(tmp)) File.Move(tmp, f); } catch { }
            }
            throw;
        }
        return new { paths = plan.Select(x => x.To).ToArray() };
    }

    /// <summary>Hearth Terminal (or Windows Terminal, or PowerShell) in this folder.</summary>
    public void Terminal(string path)
    {
        path = Full(path);
        if (!Directory.Exists(path)) path = Path.GetDirectoryName(path) ?? path;
        if (File.Exists(HearthTerminal.Exe))
        {
            var psi = new ProcessStartInfo(HearthTerminal.Exe) { UseShellExecute = false, WorkingDirectory = path };
            psi.ArgumentList.Add("--cwd");
            psi.ArgumentList.Add(path);
            Process.Start(psi)?.Dispose();
            return;
        }
        try
        {
            var wt = new ProcessStartInfo("wt.exe") { UseShellExecute = true, WorkingDirectory = path };
            wt.ArgumentList.Add("-d");
            wt.ArgumentList.Add(path);
            Process.Start(wt)?.Dispose();
        }
        catch
        {
            Process.Start(new ProcessStartInfo("powershell.exe") { UseShellExecute = true, WorkingDirectory = path })?.Dispose();
        }
    }

    /// <summary>Text on Windows' clipboard (Copy path). UI thread only.</summary>
    public void CopyText(string text) { if (!string.IsNullOrEmpty(text)) Clipboard.SetText(text); }

    /// <summary>Safely removes a USB drive or ejects a disc, as Explorer's Eject does.</summary>
    public void Eject(string drive)
    {
        var root = (Path.GetPathRoot(Full(drive)) ?? "").TrimEnd('\\');
        if (root.Length != 2) throw new ArgumentException("Only drives can be ejected");
        var shellType = Type.GetTypeFromProgID("Shell.Application") ?? throw new InvalidOperationException("Windows' shell isn't there");
        dynamic shell = Activator.CreateInstance(shellType)!;
        dynamic? item = shell.NameSpace(17 /* This PC */)?.ParseName(root);
        if (item is null) throw new InvalidOperationException("Windows doesn't see that drive");
        item.InvokeVerb("Eject");
    }

    // ------------------------------------------------------------------ search, live

    /// <summary>Looks below a folder for names containing the text (or matching * and ?): files.found events with
    /// the matches as they come, then files.searchDone. Stopped with Cancel(id).</summary>
    public int StartSearch(string path, string query, bool showHidden)
    {
        path = Full(path);
        query = query.Trim();
        if (query.Length == 0) throw new ArgumentException("Type something to search for");
        Regex? wild = query.IndexOfAny(new[] { '*', '?' }) >= 0
            ? new Regex("^" + Regex.Escape(query).Replace("\\*", ".*").Replace("\\?", ".") + "$", RegexOptions.IgnoreCase | RegexOptions.CultureInvariant)
            : null;
        var cts = new CancellationTokenSource();
        int id;
        lock (_jobs) { id = ++_jobId; _jobs[id] = cts; }
        var ct = cts.Token;
        _ = Task.Run(() =>
        {
            var batch = new List<Entry>();
            int count = 0;
            bool truncated = false, cancelled = false;
            var sw = Stopwatch.StartNew();
            long last = 0;
            void Flush() { if (batch.Count == 0) return; _event("files.found", new { id, entries = batch.ToArray() }); batch.Clear(); }
            try
            {
                var opts = new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = showHidden ? FileAttributes.ReparsePoint : FileAttributes.Hidden | FileAttributes.System | FileAttributes.ReparsePoint };
                foreach (var i in new DirectoryInfo(path).EnumerateFileSystemInfos("*", opts))
                {
                    ct.ThrowIfCancellationRequested();
                    bool hit = wild is not null ? wild.IsMatch(i.Name) : i.Name.Contains(query, StringComparison.CurrentCultureIgnoreCase);
                    if (hit)
                    {
                        bool isDir = (i.Attributes & FileAttributes.Directory) != 0;
                        var ext = isDir ? "" : i.Extension.TrimStart('.').ToLowerInvariant();
                        long size = 0;
                        try { size = isDir ? 0 : ((FileInfo)i).Length; } catch { }
                        batch.Add(new Entry(i.Name, i.FullName, isDir, size, ToMs(i.LastWriteTimeUtc), ext, (i.Attributes & FileAttributes.Hidden) != 0, isDir ? "folder" : KindOf(ext)));
                        if (++count >= 5000) { truncated = true; break; }
                    }
                    if (sw.ElapsedMilliseconds - last > 250) { last = sw.ElapsedMilliseconds; Flush(); }
                }
            }
            catch (OperationCanceledException) { cancelled = true; }
            catch (Exception ex) { SystemBridge.Log("files: search: " + ex.Message); }
            Flush();
            lock (_jobs) { if (_jobs.Remove(id, out var c)) c.Dispose(); }
            _event("files.searchDone", new { id, count, truncated, cancelled });
        });
        return id;
    }

    // ------------------------------------------------------------------ archives

    /// <summary>Packs sources into archive (zip, tar.gz or tar) in the background, with files.progress events.</summary>
    public int StartCompress(string[] sources, string archive, string fmt)
    {
        archive = Full(archive);
        CheckName(Path.GetFileName(archive));
        if (File.Exists(archive) || Directory.Exists(archive)) throw new IOException($"There's already something called \"{Path.GetFileName(archive)}\" here");
        fmt = fmt is "tar.gz" or "tar" ? fmt : "zip";
        var src = sources.Select(Full).Where(p => File.Exists(p) || Directory.Exists(p)).ToArray();
        if (src.Length == 0) throw new FileNotFoundException("Nothing to compress");
        var cts = new CancellationTokenSource();
        int id;
        lock (_jobs) { id = ++_jobId; _jobs[id] = cts; }
        _ = Task.Run(() => RunArchive(id, "compress", cts.Token, archive, (report) => Compress(src, archive, fmt, report, cts.Token)));
        return id;
    }

    /// <summary>Unpacks an archive: here = next to it (straight in if it holds one folder, else in a folder named
    /// after it), otherwise into dest.</summary>
    public int StartExtract(string archive, string? dest, bool here)
    {
        archive = Full(archive);
        if (!File.Exists(archive)) throw new FileNotFoundException("That archive isn't there any more");
        var to = here ? null : Full(dest ?? throw new ArgumentException("Where to?"));
        var cts = new CancellationTokenSource();
        int id;
        lock (_jobs) { id = ++_jobId; _jobs[id] = cts; }
        _ = Task.Run(() => RunArchive(id, "extract", cts.Token, archive, (report) => Extract(archive, to, report, cts.Token)));
        return id;
    }

    private delegate void ArchiveReport(long done, long total, string current);

    private void RunArchive(int id, string op, CancellationToken ct, string archive, Func<ArchiveReport, (string Path, List<string> Made)> work)
    {
        var sw = Stopwatch.StartNew();
        long lastReport = -1000;
        string? error = null, result = null;
        bool cancelled = false;
        void Report(long done, long total, string current)
        {
            if (sw.ElapsedMilliseconds - lastReport < 200 && done < total) return;
            lastReport = sw.ElapsedMilliseconds;
            double secs = Math.Max(0.001, sw.Elapsed.TotalSeconds);
            _event("files.progress", new { id, op, done, total, count = 0, current, text = (op == "compress" ? "Compressing " : "Unpacking ") + current, bytesPerSec = (long)(done / secs) });
        }
        List<string>? made = null;
        try
        {
            Report(0, 0, Path.GetFileName(archive));
            var r = work(Report);
            result = r.Path;
            made = r.Made;
        }
        catch (OperationCanceledException) { cancelled = true; }
        catch (InvalidDataException ex) { error = "The archive is damaged: " + ex.Message; }
        catch (Exception ex) { error = ex.Message; }
        lock (_jobs) { if (_jobs.Remove(id, out var cts)) cts.Dispose(); }
        _event("files.done", new { id, op, ok = error is null && !cancelled, cancelled, error, dest = Path.GetDirectoryName(archive), count = 1, path = result });
    }

    private static (string, List<string>) Compress(string[] sources, string archive, string fmt, ArchiveReport report, CancellationToken ct)
    {
        var files = new List<(string Full, string Name, bool Dir)>();
        foreach (var s in sources)
        {
            var parent = Path.GetDirectoryName(s.TrimEnd('\\')) ?? "";
            if (Directory.Exists(s))
            {
                files.Add((s, Path.GetRelativePath(parent, s).Replace('\\', '/'), true));
                foreach (var i in new DirectoryInfo(s).EnumerateFileSystemInfos("*", new EnumerationOptions { IgnoreInaccessible = true, RecurseSubdirectories = true, AttributesToSkip = FileAttributes.ReparsePoint }))
                    files.Add((i.FullName, Path.GetRelativePath(parent, i.FullName).Replace('\\', '/'), (i.Attributes & FileAttributes.Directory) != 0));
            }
            else files.Add((s, Path.GetFileName(s), false));
        }
        long total = files.Where(f => !f.Dir).Sum(f => SafeLength(f.Full)), done = 0;
        var part = Path.Combine(Path.GetDirectoryName(archive)!, "." + Path.GetFileName(archive) + ".hearth-part");
        try
        {
            using (var fs = new FileStream(part, FileMode.Create))
            {
                if (fmt == "zip")
                {
                    using var zip = new ZipArchive(fs, ZipArchiveMode.Create);
                    foreach (var (full, name, dir) in files)
                    {
                        ct.ThrowIfCancellationRequested();
                        if (dir) { zip.CreateEntry(name + "/"); continue; }
                        var e = zip.CreateEntry(name, CompressionLevel.Optimal);
                        try { e.LastWriteTime = File.GetLastWriteTime(full); } catch { }
                        using var es = e.Open();
                        using var input = new FileStream(full, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
                        CopyWithProgress(input, es, ref done, total, Path.GetFileName(full), report, ct);
                    }
                }
                else
                {
                    using Stream outer = fmt == "tar.gz" ? new GZipStream(fs, CompressionLevel.Optimal, leaveOpen: true) : fs;
                    using var tw = new TarWriter(outer, TarEntryFormat.Pax, leaveOpen: true);
                    foreach (var (full, name, dir) in files)
                    {
                        ct.ThrowIfCancellationRequested();
                        if (dir) { tw.WriteEntry(new PaxTarEntry(TarEntryType.Directory, name + "/")); continue; }
                        using var input = new FileStream(full, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
                        var counting = new ProgressStream(input, n => { done += n; report(done, total, Path.GetFileName(full)); }, ct);
                        var te = new PaxTarEntry(TarEntryType.RegularFile, name) { DataStream = counting };
                        try { te.ModificationTime = File.GetLastWriteTimeUtc(full); } catch { }
                        tw.WriteEntry(te);
                    }
                }
            }
            File.Move(part, archive);
        }
        catch
        {
            try { File.Delete(part); } catch { }
            throw;
        }
        report(total, total, Path.GetFileName(archive));
        return (archive, new List<string> { archive });
    }

    private static void CopyWithProgress(Stream input, Stream output, ref long done, long total, string name, ArchiveReport report, CancellationToken ct)
    {
        var buf = new byte[1 << 20];
        int n;
        while ((n = input.Read(buf, 0, buf.Length)) > 0)
        {
            ct.ThrowIfCancellationRequested();
            output.Write(buf, 0, n);
            done += n;
            report(done, total, name);
        }
    }

    private static string? SafeMember(string name)
    {
        var parts = name.Replace('\\', '/').Split('/', StringSplitOptions.RemoveEmptyEntries).Where(x => x != ".").ToArray();
        if (parts.Length == 0 || parts.Contains("..") || name.StartsWith('/') || name.StartsWith('\\') || parts[0].Contains(':')) return null;
        return string.Join('/', parts);
    }

    private static (string, List<string>) Extract(string archive, string? dest, ArchiveReport report, CancellationToken ct)
    {
        var lower = archive.ToLowerInvariant();
        bool isZip = lower.EndsWith(".zip") || lower.EndsWith(".cbz") || lower.EndsWith(".jar") || lower.EndsWith(".apk");
        bool isTar = lower.EndsWith(".tar") || lower.EndsWith(".tar.gz") || lower.EndsWith(".tgz");
        if (!isZip && !isTar)
        {
            // Everything else through 7-Zip, if it's installed.
            var seven = new[] { Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86) }
                .Select(d => Path.Combine(d, "7-Zip", "7z.exe")).FirstOrDefault(File.Exists);
            if (seven is null) throw new InvalidOperationException("Hearth Files opens .zip, .tar and .tar.gz; install 7-Zip for the others");
            var outDir = dest ?? Unique(Path.Combine(Path.GetDirectoryName(archive)!, StemOf(Path.GetFileName(archive))));
            var psi = new ProcessStartInfo(seven) { UseShellExecute = false, CreateNoWindow = true };
            foreach (var a in new[] { "x", "-y", "-o" + outDir, archive }) psi.ArgumentList.Add(a);
            using var p = Process.Start(psi)!;
            while (!p.WaitForExit(200)) { if (ct.IsCancellationRequested) { try { p.Kill(true); } catch { } ct.ThrowIfCancellationRequested(); } }
            if (p.ExitCode != 0) throw new IOException("7-Zip couldn't unpack it");
            return (outDir, new List<string>());
        }
        // What's inside: the names (safe ones only) and whether it's all in one folder.
        var names = new List<string>();
        if (isZip) { using var z = ZipFile.OpenRead(archive); names.AddRange(z.Entries.Select(e => e.FullName)); }
        else
        {
            using var fs = File.OpenRead(archive);
            using Stream s = lower.EndsWith(".tar") ? fs : new GZipStream(fs, CompressionMode.Decompress);
            using var tr = new TarReader(s);
            TarEntry? te;
            while ((te = tr.GetNextEntry()) is not null) names.Add(te.Name);
        }
        var safe = names.Select(SafeMember).Where(x => x is not null).Select(x => x!).ToList();
        var tops = safe.Select(x => x.Split('/')[0]).Distinct(StringComparer.OrdinalIgnoreCase).ToList();
        string root;
        string? renameTop = null;
        var parentDir = Path.GetDirectoryName(archive)!;
        if (dest is null)
        {
            bool oneFolder = tops.Count == 1 && safe.Any(x => x.Contains('/') || names.Any(n => n.TrimEnd('/') == tops[0] && n.EndsWith('/')));
            if (oneFolder)
            {
                root = parentDir;
                var t = Path.Combine(root, tops[0]);
                if (File.Exists(t) || Directory.Exists(t)) renameTop = Path.GetFileName(Unique(t));
            }
            else root = Unique(Path.Combine(parentDir, StemOf(Path.GetFileName(archive))));
        }
        else root = dest;
        var made = new List<string>();
        bool rootExisted = Directory.Exists(root);
        Directory.CreateDirectory(root);
        if (!rootExisted) made.Add(root);
        var rootFull = Path.GetFullPath(root).TrimEnd('\\') + "\\";
        string? Place(string name)
        {
            var m = SafeMember(name);
            if (m is null) return null;
            if (renameTop is not null) { var rest = m.Contains('/') ? m[(m.IndexOf('/') + 1)..] : ""; m = renameTop + (rest.Length > 0 ? "/" + rest : ""); }
            var full = Path.GetFullPath(Path.Combine(root, m.Replace('/', '\\')));
            if (!full.StartsWith(rootFull, StringComparison.OrdinalIgnoreCase)) return null;
            var top = Path.Combine(root, m.Split('/')[0]);
            if (rootExisted && !made.Contains(top) && !File.Exists(top) && !Directory.Exists(top)) made.Add(top);
            return full;
        }
        try
        {
            if (isZip)
            {
                using var z = ZipFile.OpenRead(archive);
                long total = z.Entries.Sum(e => e.Length), done = 0;
                foreach (var e in z.Entries)
                {
                    ct.ThrowIfCancellationRequested();
                    var full = Place(e.FullName);
                    if (full is null) continue;
                    if (e.FullName.EndsWith('/') || e.FullName.EndsWith('\\')) { Directory.CreateDirectory(full); continue; }
                    Directory.CreateDirectory(Path.GetDirectoryName(full)!);
                    using (var input = e.Open())
                    using (var output = new FileStream(full, FileMode.Create))
                        CopyWithProgress(input, output, ref done, total, e.Name, report, ct);
                    try { File.SetLastWriteTime(full, e.LastWriteTime.DateTime); } catch { }
                }
            }
            else
            {
                using var fs = File.OpenRead(archive);
                long total = fs.Length;
                var counting = new ProgressStream(fs, _ => report(fs.Position, total, Path.GetFileName(archive)), ct);
                using Stream s = lower.EndsWith(".tar") ? counting : new GZipStream(counting, CompressionMode.Decompress);
                using var tr = new TarReader(s);
                TarEntry? e;
                while ((e = tr.GetNextEntry()) is not null)
                {
                    ct.ThrowIfCancellationRequested();
                    var full = Place(e.Name);
                    if (full is null) continue;
                    if (e.EntryType == TarEntryType.Directory) { Directory.CreateDirectory(full); continue; }
                    if (e.EntryType is not (TarEntryType.RegularFile or TarEntryType.V7RegularFile or TarEntryType.ContiguousFile) || e.DataStream is null) continue;
                    Directory.CreateDirectory(Path.GetDirectoryName(full)!);
                    using (var output = new FileStream(full, FileMode.Create)) e.DataStream.CopyTo(output);
                    try { File.SetLastWriteTimeUtc(full, e.ModificationTime.UtcDateTime); } catch { }
                }
            }
        }
        catch
        {
            for (int i = made.Count - 1; i >= 0; i--) { try { DeleteAny(made[i]); } catch { } }
            throw;
        }
        var result = dest is null && root == parentDir ? Path.Combine(root, renameTop ?? tops.FirstOrDefault() ?? "") : root;
        return (result, made);
    }

    private static string StemOf(string name)
    {
        foreach (var x in new[] { ".tar.gz", ".tgz", ".tar", ".zip", ".7z", ".rar", ".cbz", ".cbr", ".jar", ".apk", ".xz", ".gz", ".bz2", ".zst", ".iso" })
            if (name.EndsWith(x, StringComparison.OrdinalIgnoreCase) && name.Length > x.Length) return name[..^x.Length];
        return Path.GetFileNameWithoutExtension(name);
    }

    /// <summary>A read-only stream that reports how much was read and stops when cancelled.</summary>
    private sealed class ProgressStream : Stream
    {
        private readonly Stream _inner;
        private readonly Action<int> _read;
        private readonly CancellationToken _ct;
        public ProgressStream(Stream inner, Action<int> read, CancellationToken ct) { _inner = inner; _read = read; _ct = ct; }
        public override bool CanRead => true;
        public override bool CanSeek => _inner.CanSeek;
        public override bool CanWrite => false;
        public override long Length => _inner.Length;
        public override long Position { get => _inner.Position; set => _inner.Position = value; }
        public override void Flush() { }
        public override int Read(byte[] buffer, int offset, int count)
        {
            _ct.ThrowIfCancellationRequested();
            int n = _inner.Read(buffer, offset, count);
            if (n > 0) _read(n);
            return n;
        }
        public override long Seek(long offset, SeekOrigin origin) => _inner.Seek(offset, origin);
        public override void SetLength(long value) => throw new NotSupportedException();
        public override void Write(byte[] buffer, int offset, int count) => throw new NotSupportedException();
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
    public int StartTransfer(string[] sources, string destDir, bool move, Dictionary<string, string>? decide = null, string? fallback = null)
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
        var d = decide ?? new Dictionary<string, string>();
        var f = fallback is "replace" or "skip" ? fallback : "rename";
        _ = Task.Run(() => RunTransfer(id, src, destDir, move, cts.Token, d, f));
        return id;
    }

    public bool Cancel(int id)
    {
        lock (_jobs) { if (_jobs.TryGetValue(id, out var c)) { c.Cancel(); return true; } }
        return false;
    }

    private void RunTransfer(int id, string[] sources, string destDir, bool move, CancellationToken ct, Dictionary<string, string> decide, string fallback)
    {
        string verb = move ? "Moving" : "Copying";
        long total = 0, done = 0;
        int count = 0;
        var sw = Stopwatch.StartNew();
        long lastReport = -1000;
        string? error = null;
        bool cancelled = false;
        int skipped = 0;
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
                // Pasting into the same folder keeps both: "name (2)". Something else with that name: what the
                // page decided (Replace merges folders, Skip, Keep both).
                if (string.Equals(Path.GetFullPath(s).TrimEnd('\\'), Path.GetFullPath(target).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))
                {
                    if (move) continue;   // moving into where it already is: nothing to do
                    target = Unique(target);
                }
                else if (File.Exists(target) || Directory.Exists(target))
                {
                    var choice = decide.TryGetValue(name, out var c) ? c : fallback;
                    if (choice == "skip") { skipped++; continue; }
                    if (choice == "replace")
                    {
                        bool srcDir = Directory.Exists(s), dstDir = Directory.Exists(target);
                        if (srcDir && dstDir)
                        {
                            MergeDir(s, target, move, rename, ref done, ref count, ct, Report);
                            count++;
                            continue;
                        }
                        if (!srcDir && !dstDir && !rename)
                        {
                            // A file over a file: written next to it and swapped in, so stopping keeps the old one.
                            CopyFileReplace(s, target, ref done, ct, Report);
                            count++;
                            if (move) { File.SetAttributes(s, FileAttributes.Normal); File.Delete(s); }
                            continue;
                        }
                        DeleteAny(target);
                    }
                    else target = Unique(target);
                }
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
        _event("files.done", new { id, op = move ? "move" : "copy", ok = error is null && !cancelled, cancelled, error, dest = destDir, count, skipped });
    }

    /// <summary>Puts the folder src into the folder dst that's there already: new things go in, clashing files are
    /// replaced, clashing folders merged the same way. rename: moving on the same drive (instant moves).</summary>
    private static void MergeDir(string src, string dst, bool move, bool rename, ref long done, ref int count, CancellationToken ct, ReportFn report)
    {
        Directory.CreateDirectory(dst);
        foreach (var d in Directory.EnumerateDirectories(src))
        {
            ct.ThrowIfCancellationRequested();
            if ((File.GetAttributes(d) & FileAttributes.ReparsePoint) != 0) continue;
            var t = Path.Combine(dst, Path.GetFileName(d));
            if (File.Exists(t)) DeleteAny(t);
            if (Directory.Exists(t)) MergeDir(d, t, move, rename, ref done, ref count, ct, report);
            else if (move && rename) Directory.Move(d, t);
            else CopyDir(d, t, ref done, ref count, ct, report);
        }
        foreach (var f in Directory.EnumerateFiles(src))
        {
            ct.ThrowIfCancellationRequested();
            var t = Path.Combine(dst, Path.GetFileName(f));
            if (Directory.Exists(t)) DeleteAny(t);
            if (move && rename)
            {
                if (File.Exists(t)) File.SetAttributes(t, FileAttributes.Normal);
                File.Move(f, t, true);
            }
            else CopyFileReplace(f, t, ref done, ct, report);
            count++;
        }
        if (move)
        {
            try { ClearReadOnly(src); Directory.Delete(src, true); } catch { }
        }
    }

    private static void CopyFileReplace(string src, string dst, ref long done, CancellationToken ct, ReportFn report)
    {
        if (!File.Exists(dst)) { CopyFile(src, dst, ref done, ct, report); return; }
        var part = Path.Combine(Path.GetDirectoryName(dst)!, "." + Path.GetFileName(dst) + ".hearth-part");
        if (File.Exists(part)) File.Delete(part);
        CopyFile(src, part, ref done, ct, report);
        File.SetAttributes(dst, FileAttributes.Normal);
        File.Move(part, dst, true);
    }

    private static void DeleteAny(string p)
    {
        if (Directory.Exists(p)) { ClearReadOnly(p); Directory.Delete(p, true); }
        else if (File.Exists(p)) { File.SetAttributes(p, FileAttributes.Normal); File.Delete(p); }
    }

    /// <summary>What's already in destDir with the same name as something about to go there.</summary>
    public object Conflicts(string[] sources, string destDir)
    {
        destDir = Full(destDir);
        var list = new List<object>();
        foreach (var raw in sources)
        {
            string s;
            try { s = Full(raw); } catch { continue; }
            var name = Path.GetFileName(s.TrimEnd('\\'));
            if (string.IsNullOrEmpty(name)) continue;
            if (string.Equals((Path.GetDirectoryName(s.TrimEnd('\\')) ?? "").TrimEnd('\\'), destDir.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase)) continue;
            var t = Path.Combine(destDir, name);
            bool tDir = Directory.Exists(t), sDir = Directory.Exists(s);
            if (!tDir && !File.Exists(t)) continue;
            list.Add(new
            {
                name, src = s, target = t, srcDir = sDir, targetDir = tDir,
                srcSize = sDir ? 0 : SafeLength(s), targetSize = tDir ? 0 : SafeLength(t),
                srcModified = ToMs(sDir ? Directory.GetLastWriteTimeUtc(s) : File.GetLastWriteTimeUtc(s)),
                targetModified = ToMs(tDir ? Directory.GetLastWriteTimeUtc(t) : File.GetLastWriteTimeUtc(t)),
            });
        }
        return new { conflicts = list, dest = destDir };
    }

    private static long SafeLength(string p) { try { return new FileInfo(p).Length; } catch { return 0; } }

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

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern uint GetCompressedFileSizeW(string name, out uint high);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetDiskFreeSpaceW(string root, out uint sectorsPerCluster, out uint bytesPerSector, out uint freeClusters, out uint totalClusters);
    [DllImport("shlwapi.dll", CharSet = CharSet.Unicode)] private static extern int AssocQueryString(uint flags, int str, string assoc, string? extra, System.Text.StringBuilder? outStr, ref uint outLen);

    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    private struct PROPERTYKEY { public Guid fmtid; public uint pid; }
    // PROPVARIANT, enough of it for numbers (VT_UI4 = 19, VT_UI8 = 21); 24 bytes on 64-bit.
    [StructLayout(LayoutKind.Explicit, Size = 24)]
    private struct PROPVARIANT
    {
        [FieldOffset(0)] public ushort vt;
        [FieldOffset(8)] public uint u4;
        [FieldOffset(8)] public ulong u8;
    }
    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore
    {
        void GetCount(out uint count);
        void GetAt(uint index, out PROPERTYKEY key);
        void GetValue(ref PROPERTYKEY key, out PROPVARIANT value);
        void SetValue(ref PROPERTYKEY key, ref PROPVARIANT value);
        void Commit();
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    private static extern void SHGetPropertyStoreFromParsingName(string path, IntPtr bindCtx, int flags, ref Guid riid, [MarshalAs(UnmanagedType.Interface)] out IPropertyStore store);
    [DllImport("ole32.dll")] private static extern int PropVariantClear(ref PROPVARIANT pv);

    [DllImport("shell32.dll")] private static extern int SHGetKnownFolderPath([MarshalAs(UnmanagedType.LPStruct)] Guid id, uint flags, IntPtr token, out IntPtr path);
}
