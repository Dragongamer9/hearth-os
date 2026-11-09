package dev.hearthos.android

import java.io.EOFException
import java.io.File
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.nio.file.FileVisitResult
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.SimpleFileVisitor
import java.nio.file.attribute.BasicFileAttributes
import java.nio.file.attribute.PosixFilePermission
import java.security.MessageDigest

/*
 * Hearth Linux, the parts without Android in them (so the unit tests run on a plain JVM): where the Linux comes from,
 * reading its tar archive safely, the PRoot command line, the stand-ins for the /proc files Android hides, package
 * names, and deleting a tree without following its symlinks. LinuxEnv.kt puts these to work. See docs/ANDROID-LINUX.md.
 */

/** The pinned Linux: Debian 13 (trixie) for arm64, the rootfs Termux's proot-distro installs (4.29.0 build). */
object LinuxSources {
    const val DISTRO = "Debian 13 (trixie)"
    const val VERSION = "trixie-aarch64-pd-v4.29.0"
    /** Both serve the same file; the SHA-256 is what's trusted, not the server. proot-distro v4.30.1–v4.34.2 list the
     *  GitHub copy, v4.37.0 the easycli.sh mirror, both with this checksum. */
    val ROOTFS_URLS = listOf(
        "https://github.com/termux/proot-distro/releases/download/v4.29.0/debian-trixie-aarch64-pd-v4.29.0.tar.xz",
        "https://easycli.sh/proot-distro/debian-trixie-aarch64-pd-v4.29.0.tar.xz",
    )
    const val ROOTFS_SHA256 = "3834a11cbc6496935760bdc20cca7e2c25724d0cd8f5e4926da8fd5ca1857918"
    /** proot-distro's tarballs keep everything in one top folder (debian-trixie-aarch64/…): strip it. */
    const val ROOTFS_STRIP = 1
    /** What a Linux needs on the device: the download, the unpacked system and room for apt. */
    const val NEED_FREE_BYTES = 1_500_000_000L
    /** The non-root user Hearth makes. */
    const val USER = "hearth"
    const val USER_ID = 1000
    const val KERNEL_RELEASE = "6.6.0-hearth"
}

// ==================================================================== input checks
object LinuxInput {
    const val MAX_COMMAND = 8000
    const val MAX_PACKAGES = 64
    private val PKG = Regex("^[a-z0-9][a-z0-9+.-]*$")

    /** A command from the page: NULs out, trimmed, not empty, not longer than [MAX_COMMAND]. */
    fun command(raw: String?): String {
        val s = (raw ?: "").replace("\u0000", "").trim()
        if (s.isEmpty()) throw IllegalArgumentException("Type a command first")
        if (s.length > MAX_COMMAND) throw IllegalArgumentException("That command is too long (${s.length} characters, at most $MAX_COMMAND)")
        return s
    }

    fun isPackageName(s: String): Boolean = s.length in 1..128 && PKG.matches(s)

    /** Debian package names, as apt-get gets them: lower case, digits, + . - (never an option like -o). */
    fun packages(list: List<String>): List<String> {
        val out = list.map { it.trim() }.filter { it.isNotEmpty() }.distinct()
        if (out.isEmpty()) throw IllegalArgumentException("Name at least one package")
        if (out.size > MAX_PACKAGES) throw IllegalArgumentException("At most $MAX_PACKAGES packages at a time")
        out.firstOrNull { !isPackageName(it) }?.let { throw IllegalArgumentException("'${it.take(40)}' isn't a package name") }
        return out
    }

    /** "vim git  htop" or "vim, git" → the names. */
    fun splitPackages(text: String): List<String> = text.split(Regex("[\\s,]+")).filter { it.isNotEmpty() }

    /** apt-get's arguments for an action from the page. */
    fun aptArgs(action: String, pkgs: List<String>): List<String> = when (action) {
        "install" -> listOf("apt-get", "-y", "--no-install-recommends", "install") + packages(pkgs)
        "remove" -> listOf("apt-get", "-y", "remove") + packages(pkgs)
        "upgrade" -> listOf("apt-get", "-y", "upgrade")
        "update" -> listOf("apt-get", "update")
        else -> throw IllegalArgumentException("Unknown apt action '$action'")
    }

    /** One shell line out of argv (every word single-quoted), for `sh -c` inside the Linux. */
    fun shellJoin(argv: List<String>): String = argv.joinToString(" ") { "'" + it.replace("'", "'\\''") + "'" }
}

// ==================================================================== the fake /proc files
/**
 * Android hides some of /proc from apps (SELinux), and programs in the Linux (apt, ps, top, free…) read them. Like
 * proot-distro, Hearth binds a stand-in over each one it can't read. Written by Hearth, kept outside the rootfs.
 */
object FakeProc {
    data class Entry(val name: String, val guest: String, val content: String)

    fun stat(cpus: Int): String {
        val n = cpus.coerceIn(1, 64)
        val row = longArrayOf(1200, 0, 900, 48000, 60, 0, 20, 0, 0, 0)
        val sb = StringBuilder("cpu  " + row.joinToString(" ") { (it * n).toString() } + "\n")
        for (i in 0 until n) sb.append("cpu$i " + row.joinToString(" ") + "\n")
        sb.append("intr 0\nctxt 100000\nbtime 1700000000\nprocesses 400\nprocs_running 1\nprocs_blocked 0\nsoftirq 0 0 0 0 0 0 0 0 0 0 0\n")
        return sb.toString()
    }

    /** Idle time agrees with [stat]'s idle column (48000 ticks of 1/100 s per CPU). */
    fun uptime(cpus: Int): String = "1000.00 %.2f\n".format(java.util.Locale.US, 480.0 * cpus.coerceIn(1, 64))

    const val LOADAVG = "0.20 0.15 0.10 1/200 400\n"
    val VERSION = "Linux version ${LinuxSources.KERNEL_RELEASE} (hearth@android) (clang) #1 SMP PREEMPT\n"
    val VMSTAT = listOf("nr_free_pages 500000", "nr_inactive_anon 10000", "nr_active_anon 10000", "nr_inactive_file 20000",
        "nr_active_file 20000", "nr_unevictable 0", "nr_dirty 0", "nr_writeback 0", "pgpgin 0", "pgpgout 0", "pswpin 0",
        "pswpout 0", "pgfault 100000", "pgmajfault 100").joinToString("\n", postfix = "\n")

    fun entries(cpus: Int): List<Entry> = listOf(
        Entry("loadavg", "/proc/loadavg", LOADAVG),
        Entry("stat", "/proc/stat", stat(cpus)),
        Entry("uptime", "/proc/uptime", uptime(cpus)),
        Entry("version", "/proc/version", VERSION),
        Entry("vmstat", "/proc/vmstat", VMSTAT),
        Entry("cap_last_cap", "/proc/sys/kernel/cap_last_cap", "40\n"),
        Entry("max_user_watches", "/proc/sys/fs/inotify/max_user_watches", "8192\n"),
        Entry("overflowuid", "/proc/sys/kernel/overflowuid", "65534\n"),
        Entry("overflowgid", "/proc/sys/kernel/overflowgid", "65534\n"),
    )
}

// ==================================================================== the PRoot command line
/**
 * Everything one PRoot start needs. Paths are on Android's side; [command] is argv inside the Linux.
 * [readable] says whether Android lets Hearth read a /proc path itself (then no stand-in is bound over it).
 */
data class ProotSpec(
    val proot: String,
    val rootfs: String,
    val sysdata: String,
    val command: List<String>,
    val root: Boolean = false,
    val cwd: String? = null,
    val sdcard: String? = null,
    val extraBinds: List<Pair<String, String>> = emptyList(),
    val hasDevFd: Boolean = true,
    val readable: (String) -> Boolean = { false },
)

object ProotArgs {
    fun home(root: Boolean) = if (root) "/root" else "/home/${LinuxSources.USER}"

    fun build(s: ProotSpec): List<String> {
        val a = ArrayList<String>()
        a += s.proot
        a += "--kill-on-exit"
        a += "--link2symlink"
        a += "-L"
        a += "--kernel-release=${LinuxSources.KERNEL_RELEASE}"
        a += if (s.root) "-0" else "--change-id=${LinuxSources.USER_ID}:${LinuxSources.USER_ID}"
        a += "--rootfs=${s.rootfs}"
        a += "--cwd=${s.cwd ?: home(s.root)}"
        a += listOf("--bind=/dev", "--bind=/proc", "--bind=/sys")
        a += "--bind=/dev/urandom:/dev/random"
        if (!s.hasDevFd) {
            a += "--bind=/proc/self/fd:/dev/fd"
            a += listOf("--bind=/proc/self/fd/0:/dev/stdin", "--bind=/proc/self/fd/1:/dev/stdout", "--bind=/proc/self/fd/2:/dev/stderr")
        }
        a += "--bind=${s.rootfs}/tmp:/dev/shm"
        a += "--bind=${s.sysdata}/sys_empty:/sys/fs/selinux"
        for (e in FakeProc.entries(1)) if (!s.readable(e.guest)) a += "--bind=${s.sysdata}/${e.name}:${e.guest}"
        s.sdcard?.let { a += listOf("--bind=$it:/sdcard", "--bind=$it:/mnt/sdcard") }
        for ((from, to) in s.extraBinds) a += "--bind=$from:$to"
        a += s.command
        return a
    }

    /** The environment PRoot starts with (it passes it on to the Linux program). Nothing from Android's own
     *  environment that could change how the guest links (LD_*), only what's listed here. */
    fun env(rootfsDir: String, prootTmp: String, loader: String, loader32: String?, root: Boolean, extra: Map<String, String> = emptyMap()): Map<String, String> {
        val m = LinkedHashMap<String, String>()
        m["HOME"] = home(root)
        m["USER"] = if (root) "root" else LinuxSources.USER
        m["LOGNAME"] = m["USER"]!!
        m["PATH"] = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/local/games:/usr/games"
        m["TERM"] = "dumb"
        m["LANG"] = "C.UTF-8"
        m["TMPDIR"] = "/tmp"
        m["SHELL"] = "/bin/bash"
        m["PROOT_TMP_DIR"] = prootTmp
        m["PROOT_LOADER"] = loader
        if (loader32 != null) m["PROOT_LOADER_32"] = loader32
        m["PROOT_L2S_DIR"] = "$rootfsDir/.l2s"
        m.putAll(extra)
        return m
    }
}

// ==================================================================== tar
/** One entry of a tar archive. [type]: '0' file, '1' hard link, '2' symlink, '5' folder; others are skipped. */
class TarEntry(val name: String, val type: Char, val mode: Int, val size: Long, val link: String)

/** Where a tar member goes under the rootfs, or null to skip it. Never escapes: ".." anywhere is refused, a leading
 *  "/" is dropped (as GNU tar does), "." parts vanish, the first [strip] parts go (the archive's top folder). */
object TarPath {
    fun safe(name: String, strip: Int = 0): String? {
        if (name.indexOf('\u0000') >= 0) return null
        val parts = name.split('/').filter { it.isNotEmpty() && it != "." }
        if (parts.any { it == ".." }) return null
        if (parts.size <= strip) return null
        return parts.drop(strip).joinToString("/")
    }
}

/**
 * Reads a tar stream (ustar, GNU long names and PAX headers: what GNU tar and bsdtar write). [next] gives each entry;
 * read its data from [data] before calling [next] again (anything not read is skipped).
 */
class TarReader(private val input: InputStream) {
    private val header = ByteArray(512)
    private var remaining = 0L      // data bytes of the current entry not read yet
    private var padding = 0L        // zero bytes after it, up to the next 512
    private var globalPax = emptyMap<String, String>()

    fun next(): TarEntry? {
        skip(remaining + padding); remaining = 0; padding = 0
        var longName: String? = null
        var longLink: String? = null
        var pax = emptyMap<String, String>()
        while (true) {
            if (!readBlock()) return null
            if (header.all { it.toInt() == 0 }) return null       // the end: two zero blocks (one is enough to stop)
            if (!checksumOk()) throw IOException("The Linux download is damaged (a bad tar header)")
            val type = header[156].toInt().toChar()
            val size = number(124, 12)
            if (size < 0) throw IOException("The Linux download is damaged (a bad size)")
            when (type) {
                'L' -> { longName = cString(readBytes(size)); continue }
                'K' -> { longLink = cString(readBytes(size)); continue }
                'x' -> { pax = parsePax(readBytes(size)); continue }
                'g' -> { globalPax = parsePax(readBytes(size)); continue }
            }
            val prefix = str(345, 155)
            val baseName = str(0, 100)
            val name = pax["path"] ?: globalPax["path"] ?: longName ?: if (prefix.isNotEmpty() && magicUstar()) "$prefix/$baseName" else baseName
            val link = pax["linkpath"] ?: longLink ?: str(157, 100)
            val realSize = pax["size"]?.toLongOrNull() ?: size
            val t = if (type == '\u0000') '0' else type
            // Links, folders, devices and FIFOs carry no data (whatever their size field says).
            val dataSize = if (t in "123456") 0L else realSize
            remaining = dataSize
            padding = (512 - dataSize % 512) % 512
            return TarEntry(name, if (t == '7') '0' else t, number(100, 8).toInt() and 0x1ff, dataSize, link)   // no setuid bits on Android's side
        }
    }

    /** The current entry's data. */
    val data: InputStream = object : InputStream() {
        override fun read(): Int {
            if (remaining <= 0) return -1
            val b = input.read(); if (b < 0) throw EOFException("The Linux download ends too soon")
            remaining--; return b
        }
        override fun read(b: ByteArray, off: Int, len: Int): Int {
            if (remaining <= 0) return -1
            val n = input.read(b, off, minOf(len.toLong(), remaining).toInt())
            if (n < 0) throw EOFException("The Linux download ends too soon")
            remaining -= n; return n
        }
    }

    private fun readBlock(): Boolean {
        var got = 0
        while (got < 512) {
            val n = input.read(header, got, 512 - got)
            if (n < 0) { if (got == 0) return false; throw EOFException("The Linux download ends too soon") }
            got += n
        }
        return true
    }

    private fun readBytes(size: Long): ByteArray {
        if (size > 1 shl 20) throw IOException("The Linux download is damaged (a header too big)")
        val b = ByteArray(size.toInt())
        var got = 0
        while (got < b.size) { val n = input.read(b, got, b.size - got); if (n < 0) throw EOFException("The Linux download ends too soon"); got += n }
        skip((512 - size % 512) % 512)
        return b
    }

    private fun skip(n: Long) {
        var left = n
        val buf = ByteArray(8192)
        while (left > 0) { val r = input.read(buf, 0, minOf(left, buf.size.toLong()).toInt()); if (r < 0) throw EOFException("The Linux download ends too soon"); left -= r }
    }

    private fun magicUstar() = str(257, 5) == "ustar"
    private fun str(off: Int, len: Int): String {
        var end = off
        while (end < off + len && header[end].toInt() != 0) end++
        return String(header, off, end - off, Charsets.UTF_8)
    }
    private fun cString(b: ByteArray): String { val z = b.indexOf(0); return String(b, 0, if (z >= 0) z else b.size, Charsets.UTF_8) }

    /** Octal, or GNU's base-256 for big numbers (high bit set). */
    private fun number(off: Int, len: Int): Long {
        if (header[off].toInt() and 0x80 != 0) {
            var v = 0L
            for (i in off + 1 until off + len) v = (v shl 8) or (header[i].toLong() and 0xff)
            return v
        }
        val s = str(off, len).trim { it == ' ' || it == '\u0000' }
        return if (s.isEmpty()) 0 else s.toLongOrNull(8) ?: -1
    }

    private fun checksumOk(): Boolean {
        val want = number(148, 8)
        var sum = 0L
        for (i in 0 until 512) sum += if (i in 148 until 156) 32 else header[i].toLong() and 0xff
        return sum == want
    }

    private fun parsePax(b: ByteArray): Map<String, String> {
        val out = HashMap<String, String>()
        var i = 0
        while (i < b.size) {
            val sp = (i until b.size).firstOrNull { b[it] == ' '.code.toByte() } ?: break
            val len = String(b, i, sp - i, Charsets.US_ASCII).toIntOrNull() ?: break
            if (len <= 0 || i + len > b.size) break
            val rec = String(b, sp + 1, i + len - sp - 2, Charsets.UTF_8)   // without the trailing \n
            val eq = rec.indexOf('=')
            if (eq > 0) out[rec.substring(0, eq)] = rec.substring(eq + 1)
            i += len
        }
        return out
    }
}

/** What extracting does to the file system; Android uses android.system.Os (exact modes, links), tests the JVM's. */
interface FsOps {
    fun chmod(path: String, mode: Int)
    fun symlink(target: String, path: String)
    fun hardlink(existing: String, path: String)
}

object JvmFsOps : FsOps {
    override fun chmod(path: String, mode: Int) {
        val p = HashSet<PosixFilePermission>()
        val bits = listOf(PosixFilePermission.OTHERS_EXECUTE, PosixFilePermission.OTHERS_WRITE, PosixFilePermission.OTHERS_READ,
            PosixFilePermission.GROUP_EXECUTE, PosixFilePermission.GROUP_WRITE, PosixFilePermission.GROUP_READ,
            PosixFilePermission.OWNER_EXECUTE, PosixFilePermission.OWNER_WRITE, PosixFilePermission.OWNER_READ)
        for (i in bits.indices) if (mode and (1 shl i) != 0) p.add(bits[i])
        Files.setPosixFilePermissions(Paths.get(path), p)
    }
    override fun symlink(target: String, path: String) { Files.createSymbolicLink(Paths.get(path), Paths.get(target)) }
    override fun hardlink(existing: String, path: String) { Files.createLink(Paths.get(path), Paths.get(existing)) }
}

/**
 * Unpacks a rootfs tar into [root], safely: no entry lands outside [root] (names checked by [TarPath], and nothing is
 * written through a symlink: every folder on the way must be a real folder), symlinks are made as they are (their
 * targets are the Linux's business: they're resolved inside PRoot), hard links become links or copies (Android often
 * refuses link(2) in app storage), devices and FIFOs are skipped (PRoot binds the real /dev). The owner always keeps
 * read/write on what it unpacks (folders: rwx), so apt can change them later; folders get their modes at the end.
 */
class TarExtractor(private val root: File, private val strip: Int, private val fs: FsOps = JvmFsOps) {
    var files = 0; private set
    var skipped = 0; private set
    private val dirModes = ArrayList<Pair<String, Int>>()
    private val rootPath: Path = root.toPath().toAbsolutePath().normalize()

    fun extract(input: InputStream, cancelled: () -> Boolean = { false }) {
        root.mkdirs()
        val tar = TarReader(input)
        val buf = ByteArray(64 * 1024)
        while (true) {
            if (cancelled()) throw InterruptedException("Cancelled")
            val e = tar.next() ?: break
            val rel = TarPath.safe(e.name, strip)
            if (rel == null) { skipped++; continue }
            val dest = rootPath.resolve(rel).normalize()
            if (!dest.startsWith(rootPath) || dest == rootPath) { skipped++; continue }
            if (!parentsReal(dest)) { skipped++; continue }
            when (e.type) {
                '5' -> {
                    if (Files.isSymbolicLink(dest) || (Files.exists(dest, LinkOption.NOFOLLOW_LINKS) && !Files.isDirectory(dest, LinkOption.NOFOLLOW_LINKS))) Files.delete(dest)
                    Files.createDirectories(dest)
                    fs.chmod(dest.toString(), 0b111_101_101)   // rwxr-xr-x for now (its own mode at the end)
                    dirModes += dest.toString() to (e.mode or 0b111_000_000)
                }
                '0' -> {
                    Files.createDirectories(dest.parent)
                    removeExisting(dest)
                    Files.newOutputStream(dest).use { out -> copy(tar.data, out, buf) }
                    fs.chmod(dest.toString(), e.mode or 0b110_000_000)
                    files++
                }
                '2' -> {
                    Files.createDirectories(dest.parent)
                    removeExisting(dest)
                    if (e.link.isEmpty() || e.link.indexOf('\u0000') >= 0) { skipped++; continue }
                    fs.symlink(e.link, dest.toString())
                    files++
                }
                '1' -> {
                    val targetRel = TarPath.safe(e.link, strip)
                    val target = targetRel?.let { rootPath.resolve(it).normalize() }
                    if (target == null || !target.startsWith(rootPath) || !parentsReal(target) || !Files.isRegularFile(target, LinkOption.NOFOLLOW_LINKS)) { skipped++; continue }
                    Files.createDirectories(dest.parent)
                    removeExisting(dest)
                    try { fs.hardlink(target.toString(), dest.toString()) }
                    catch (x: Exception) { Files.copy(target, dest); fs.chmod(dest.toString(), modeOf(target)) }
                    files++
                }
                else -> skipped++      // char/block devices, FIFOs: PRoot binds the real /dev
            }
        }
        // Folders' own modes last (a read-only folder would have refused the files in it), deepest first.
        for ((path, mode) in dirModes.sortedByDescending { it.first.length }) runCatching { fs.chmod(path, mode) }
    }

    private fun modeOf(p: Path): Int = runCatching {
        var m = 0
        val bits = listOf(PosixFilePermission.OTHERS_EXECUTE, PosixFilePermission.OTHERS_WRITE, PosixFilePermission.OTHERS_READ,
            PosixFilePermission.GROUP_EXECUTE, PosixFilePermission.GROUP_WRITE, PosixFilePermission.GROUP_READ,
            PosixFilePermission.OWNER_EXECUTE, PosixFilePermission.OWNER_WRITE, PosixFilePermission.OWNER_READ)
        val have = Files.getPosixFilePermissions(p, LinkOption.NOFOLLOW_LINKS)
        for (i in bits.indices) if (bits[i] in have) m = m or (1 shl i)
        m
    }.getOrDefault(0b110_100_100)

    /** Every folder between [rootPath] and [dest] is a real folder (or doesn't exist yet), never a symlink. */
    private fun parentsReal(dest: Path): Boolean {
        var p = dest.parent
        while (p != null && p != rootPath) {
            if (Files.isSymbolicLink(p)) return false
            if (Files.exists(p, LinkOption.NOFOLLOW_LINKS) && !Files.isDirectory(p, LinkOption.NOFOLLOW_LINKS)) return false
            p = p.parent
        }
        return p == rootPath
    }

    private fun removeExisting(dest: Path) {
        if (!Files.exists(dest, LinkOption.NOFOLLOW_LINKS)) return
        if (Files.isDirectory(dest, LinkOption.NOFOLLOW_LINKS)) SafeFs.deleteTree(dest.toFile()) else Files.delete(dest)
    }

    private fun copy(i: InputStream, o: OutputStream, buf: ByteArray) { while (true) { val n = i.read(buf); if (n < 0) break; o.write(buf, 0, n) } }
}

// ==================================================================== files
object SafeFs {
    /** Deletes [dir] and everything in it without ever following a symlink (a link is removed, never what it points
     *  to: a rootfs is full of links to /usr, /etc… that mean the Linux's, not Android's). Read-only folders are made
     *  writable first. */
    fun deleteTree(dir: File) {
        val start = dir.toPath()
        if (!Files.exists(start, LinkOption.NOFOLLOW_LINKS)) return
        if (!Files.isDirectory(start, LinkOption.NOFOLLOW_LINKS)) { Files.delete(start); return }
        Files.walkFileTree(start, object : SimpleFileVisitor<Path>() {
            override fun preVisitDirectory(d: Path, attrs: BasicFileAttributes): FileVisitResult {
                d.toFile().setWritable(true, true); d.toFile().setExecutable(true, true); d.toFile().setReadable(true, true)
                return FileVisitResult.CONTINUE
            }
            override fun visitFile(f: Path, attrs: BasicFileAttributes): FileVisitResult { Files.deleteIfExists(f); return FileVisitResult.CONTINUE }
            override fun visitFileFailed(f: Path, exc: IOException): FileVisitResult { runCatching { Files.deleteIfExists(f) }; return FileVisitResult.CONTINUE }
            override fun postVisitDirectory(d: Path, exc: IOException?): FileVisitResult { Files.deleteIfExists(d); return FileVisitResult.CONTINUE }
        })
    }

    /** Bytes in [dir], links counted as links (not followed). */
    fun size(dir: File): Long {
        if (!dir.exists()) return 0
        var total = 0L
        runCatching {
            Files.walkFileTree(dir.toPath(), object : SimpleFileVisitor<Path>() {
                override fun visitFile(f: Path, attrs: BasicFileAttributes): FileVisitResult { if (attrs.isRegularFile) total += attrs.size(); return FileVisitResult.CONTINUE }
                override fun visitFileFailed(f: Path, exc: IOException): FileVisitResult = FileVisitResult.CONTINUE
            })
        }
        return total
    }

    fun sha256(f: File): String {
        val md = MessageDigest.getInstance("SHA-256")
        f.inputStream().use { i -> val b = ByteArray(1 shl 16); while (true) { val n = i.read(b); if (n < 0) break; md.update(b, 0, n) } }
        return md.digest().joinToString("") { "%02x".format(it) }
    }
}

// ==================================================================== setting up the Linux
/** The files Hearth writes into a fresh rootfs (Termux's proot-distro does the same). Pure text: tests check it. */
object LinuxSetup {
    const val RESOLV = "nameserver 8.8.8.8\nnameserver 1.1.1.1\n"
    const val HOSTS = "127.0.0.1   localhost.localdomain localhost\n::1         localhost.localdomain localhost ip6-localhost ip6-loopback\n" +
        "fe00::0     ip6-localnet\nff00::0     ip6-mcastprefix\nff02::1     ip6-allnodes\nff02::2     ip6-allrouters\n"

    /** /etc/passwd, /etc/group, /etc/shadow, /etc/gshadow with the hearth user added (once). */
    fun addUser(file: String, text: String): String {
        val u = LinuxSources.USER; val id = LinuxSources.USER_ID
        if (text.lineSequence().any { it.startsWith("$u:") }) return text
        val line = when (file) {
            "passwd" -> "$u:x:$id:$id:Hearth:/home/$u:/bin/bash"
            "group" -> "$u:x:$id:"
            "shadow" -> "$u:!:20000:0:99999:7:::"
            "gshadow" -> "$u:!::"
            else -> return text
        }
        return (if (text.isEmpty() || text.endsWith("\n")) text else text + "\n") + line + "\n"
    }

    /** en_US.UTF-8 switched on in /etc/locale.gen (proot-distro's Debian setup does the same). */
    fun localeGen(text: String): String = text.replace(Regex("(?m)^#\\s?(en_US\\.UTF-8\\s+UTF-8)"), "$1")
}
