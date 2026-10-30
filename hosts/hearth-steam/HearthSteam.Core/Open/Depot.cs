using System.Security.Cryptography;
using Microsoft.Win32.SafeHandles;

namespace HearthSteam.Open;

// Depot contents and how to bring a game folder to a manifest: new installs, delta updates (chunks the old version
// already has are copied, not downloaded), verifying (every chunk's SHA-1 is checked) and resuming. Pure file work:
// the chunks come from a callback (Steam's CDN through SteamKit2 in the service, a fake in the tests).

/// <summary>One chunk of a file: its SHA-1 (of the uncompressed data, which is also its id on Steam's CDN), where it
/// goes in the file, and its sizes.</summary>
public sealed record ChunkRef(byte[] Sha, ulong Offset, uint Length, uint CompressedLength, uint Checksum = 0);

public sealed record DepotFile(string Path, ulong Size, int Flags, byte[] Hash, IReadOnlyList<ChunkRef> Chunks, string? LinkTarget = null)
{
    public const int Executable = 32, Directory = 64, Symlink = 512;
    public bool IsDir => (Flags & Directory) != 0;
    public bool IsLink => (Flags & Symlink) != 0;
}

public sealed class FileJob
{
    public required DepotFile File { get; init; }
    /// <summary>The same path in the old version, whose chunks can be copied instead of downloaded.</summary>
    public DepotFile? Old { get; init; }
    /// <summary>No old manifest: what's on disk at this path is checked chunk by chunk (verify / resume).</summary>
    public bool CheckDisk { get; init; }
}

public sealed class Plan
{
    public List<string> Dirs { get; } = new();
    public List<DepotFile> Links { get; } = new();
    public List<FileJob> Files { get; } = new();
    public List<string> Delete { get; } = new();
    /// <summary>Uncompressed bytes of every file that needs work (the most that may be downloaded).</summary>
    public ulong Bytes { get; set; }
}

public static class Depot
{
    /// <summary>A depot path as a safe relative path with '/' (Windows depots use '\'), or null when it would leave
    /// the game folder (absolute, "..", empty parts, NUL).</summary>
    public static string? SafePath(string raw)
    {
        if (string.IsNullOrEmpty(raw) || raw.Contains('\0')) return null;
        var parts = raw.Replace('\\', '/').Split('/', StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length == 0 || raw.StartsWith('/') || raw.StartsWith('\\') || (raw.Length > 1 && raw[1] == ':')) return null;
        foreach (var p in parts) if (p == "." || p == "..") return null;
        return string.Join('/', parts);
    }

    public static bool SameHash(byte[]? a, byte[]? b) => a != null && b != null && a.AsSpan().SequenceEqual(b);

    /// <summary>Is this the chunk (its SHA-1 matches)?</summary>
    public static bool Verify(ReadOnlySpan<byte> data, byte[] sha) => SHA1.HashData(data).AsSpan().SequenceEqual(sha);

    /// <summary>What to do to bring a folder from <paramref name="old"/> (null: unknown, so check the disk) to
    /// <paramref name="next"/>. <paramref name="verify"/> checks every file's chunks on disk even when unchanged.</summary>
    public static Plan Build(IReadOnlyList<DepotFile> next, IReadOnlyList<DepotFile>? old, Func<string, ulong?> sizeOnDisk, bool verify = false)
    {
        var plan = new Plan();
        var prev = new Dictionary<string, DepotFile>(StringComparer.Ordinal);
        foreach (var f in old ?? Array.Empty<DepotFile>()) { var p = SafePath(f.Path); if (p != null) prev[p] = f with { Path = p }; }
        var keep = new HashSet<string>(StringComparer.Ordinal);
        foreach (var raw in next)
        {
            var path = SafePath(raw.Path);
            if (path == null) continue;
            var f = raw with { Path = path };
            keep.Add(path);
            if (f.IsDir) { plan.Dirs.Add(path); continue; }
            if (f.IsLink) { plan.Links.Add(f); continue; }
            prev.TryGetValue(path, out var o);
            var size = sizeOnDisk(path);
            // Unchanged since the old version and still there at its size: nothing to do (unless verifying).
            if (!verify && o != null && SameHash(o.Hash, f.Hash) && size == f.Size) continue;
            var job = new FileJob { File = f, Old = verify ? null : o, CheckDisk = size != null && (verify || o == null) };
            plan.Files.Add(job);
            plan.Bytes += f.Size;
        }
        foreach (var p in prev.Keys) if (!keep.Contains(p) && !prev[p].IsDir) plan.Delete.Add(p);
        return plan;
    }

    /// <summary>The full path of a depot path inside the game folder, refusing anything that would land outside it or
    /// go through a link (a link in the game folder is never followed when writing).</summary>
    public static string Inside(string root, string rel)
    {
        var safe = SafePath(rel) ?? throw new InvalidDataException("Unsafe path in the depot.");
        var rootFull = Path.GetFullPath(root).TrimEnd('/');
        var full = Path.GetFullPath(Path.Combine(rootFull, safe));
        if (!full.StartsWith(rootFull + "/", StringComparison.Ordinal)) throw new InvalidDataException("Path outside the game folder.");
        var dir = Path.GetDirectoryName(full)!;
        while (dir.Length > rootFull.Length)
        {
            var info = new DirectoryInfo(dir);
            if (info.Exists && info.LinkTarget != null) throw new InvalidDataException("A link in the game folder.");
            dir = Path.GetDirectoryName(dir)!;
        }
        return full;
    }
}

/// <summary>Carries a plan out in a game folder. Each changed file is put together in a staging folder (chunks from
/// the old file, the file already on disk, or downloaded; every one SHA-1 checked) and then moved into place, so a
/// stopped download never leaves a half-written game file.</summary>
public sealed class DepotWriter
{
    readonly string root, staging;
    readonly Func<ChunkRef, CancellationToken, Task<byte[]>> fetch;
    public int Parallel { get; init; } = 8;
    long done;
    public long Done => Interlocked.Read(ref done);
    public long Downloaded;  // bytes fetched (compressed), for the speed
    public Action<long>? Progress { get; init; }

    public DepotWriter(string root, string staging, Func<ChunkRef, CancellationToken, Task<byte[]>> fetch)
    { this.root = root; this.staging = staging; this.fetch = fetch; }

    void Add(long n) { var d = Interlocked.Add(ref done, n); Progress?.Invoke(d); }

    public async Task Apply(Plan plan, CancellationToken ct)
    {
        Directory.CreateDirectory(root);
        Directory.CreateDirectory(staging);
        foreach (var d in plan.Dirs) Directory.CreateDirectory(Depot.Inside(root, d + "/x")[..^2]);
        foreach (var job in plan.Files) { ct.ThrowIfCancellationRequested(); await ApplyFile(job, ct); }
        foreach (var l in plan.Links) MakeLink(l);
        foreach (var p in plan.Delete)
        {
            var full = Depot.Inside(root, p);
            var fi = new FileInfo(full);
            if (fi.Exists || fi.LinkTarget != null) fi.Delete();
        }
        try { Directory.Delete(staging, true); } catch (IOException) { }
    }

    void MakeLink(DepotFile l)
    {
        var full = Depot.Inside(root, l.Path);
        var target = l.LinkTarget ?? "";
        // Only links that stay inside the game folder.
        if (target.Length == 0 || target.StartsWith('/')) return;
        var resolved = Path.GetFullPath(Path.Combine(Path.GetDirectoryName(full)!, target));
        if (!resolved.StartsWith(Path.GetFullPath(root).TrimEnd('/') + "/", StringComparison.Ordinal)) return;
        Directory.CreateDirectory(Path.GetDirectoryName(full)!);
        var fi = new FileInfo(full);
        if (fi.LinkTarget == target) return;
        if (fi.Exists || fi.LinkTarget != null) fi.Delete();
        File.CreateSymbolicLink(full, target);
    }

    static byte[]? ReadAt(SafeFileHandle? h, ulong offset, uint length)
    {
        if (h == null) return null;
        var buf = new byte[length];
        int n = 0;
        while (n < buf.Length) { int r = RandomAccess.Read(h, buf.AsSpan(n), (long)offset + n); if (r <= 0) return null; n += r; }
        return buf;
    }

    async Task ApplyFile(FileJob job, CancellationToken ct)
    {
        var f = job.File;
        var target = Depot.Inside(root, f.Path);
        Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        var targetInfo = new FileInfo(target);
        if (targetInfo.LinkTarget != null) { targetInfo.Delete(); targetInfo.Refresh(); }
        var chunks = f.Chunks.OrderBy(c => c.Offset).ToList();

        // Already right on disk (verify / resume): leave it.
        if (job.CheckDisk && targetInfo.Exists && (ulong)targetInfo.Length == f.Size)
        {
            using var h0 = File.OpenHandle(target, FileMode.Open, FileAccess.Read);
            if (chunks.All(c => ReadAt(h0, c.Offset, c.Length) is { } d && Depot.Verify(d, c.Sha))) { Add((long)f.Size); SetMode(target, f); return; }
        }

        var oldPath = job.Old != null ? target : null;
        var oldBySha = new Dictionary<string, ChunkRef>();
        foreach (var c in job.Old?.Chunks ?? Array.Empty<ChunkRef>()) oldBySha.TryAdd(Convert.ToHexString(c.Sha), c);
        var stage = Path.Combine(staging, Convert.ToHexString(SHA1.HashData(System.Text.Encoding.UTF8.GetBytes(f.Path))));
        using (var src = (oldPath != null || job.CheckDisk) && targetInfo.Exists ? File.OpenHandle(target, FileMode.Open, FileAccess.Read) : null)
        using (var dst = File.OpenHandle(stage, FileMode.Create, FileAccess.ReadWrite, FileShare.None, FileOptions.None, (long)f.Size))
        {
            await System.Threading.Tasks.Parallel.ForEachAsync(chunks, new ParallelOptions { MaxDegreeOfParallelism = Parallel, CancellationToken = ct }, async (c, t) =>
            {
                byte[]? data = null;
                if (src != null && oldBySha.TryGetValue(Convert.ToHexString(c.Sha), out var oc)) data = ReadAt(src, oc.Offset, oc.Length);
                else if (src != null && job.CheckDisk) data = ReadAt(src, c.Offset, c.Length);
                if (data == null || !Depot.Verify(data, c.Sha))
                {
                    data = null;
                    for (int attempt = 0; attempt < 3 && data == null; attempt++)
                    {
                        var got = await fetch(c, t);
                        Interlocked.Add(ref Downloaded, c.CompressedLength);
                        if (got.Length >= c.Length && Depot.Verify(got.AsSpan(0, (int)c.Length), c.Sha)) data = got;
                    }
                    if (data == null) throw new InvalidDataException($"A chunk of {f.Path} didn't match its checksum three times.");
                }
                RandomAccess.Write(dst, data.AsSpan(0, (int)c.Length), (long)c.Offset);
                Add(c.Length);
            });
            RandomAccess.SetLength(dst, (long)f.Size);
        }
        File.Move(stage, target, overwrite: true);
        SetMode(target, f);
    }

    static void SetMode(string path, DepotFile f)
    {
        if (OperatingSystem.IsWindows()) return;
        var m = UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.GroupRead | UnixFileMode.OtherRead;
        if ((f.Flags & DepotFile.Executable) != 0) m |= UnixFileMode.UserExecute | UnixFileMode.GroupExecute | UnixFileMode.OtherExecute;
        File.SetUnixFileMode(path, m);
    }
}
