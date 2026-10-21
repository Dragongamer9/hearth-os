package dev.hearthos.android

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Environment
import android.os.StatFs
import android.system.Os
import android.system.OsConstants
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject
import org.tukaani.xz.XZInputStream
import java.io.BufferedInputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.FilterInputStream
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.URL
import java.nio.file.Files
import java.nio.file.LinkOption
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import javax.net.ssl.HttpsURLConnection

/**
 * Hearth Linux (Beta Experimental): a real ARM64 Linux (Debian) inside Hearth for Android, without root, run by PRoot.
 *
 * Files: filesDir/linux/ { rootfs/ (the Linux), sysdata/ (stand-ins for the /proc files Android hides), tmp/ (PRoot's
 * own), cache/ (the verified download, so Reset doesn't download again), installed.json }. PRoot and its loader come
 * in the APK as native libraries (lib/<abi>/libproot.so…), because Android 10+ only runs programs from the app's
 * native library folder, never from its data folder.
 *
 * The page's calls (linux.*) answer at once; long work runs on [work] and reports with events: linux.progress
 * {kind, pct, text}, linux.output {job, lines}, linux.exit {job, kind, code}, linux.done {kind, ok, error?, status},
 * linux.x11 {ok, job?, kind, error?} (Termux:X11 started or not), linux.desktop {state, mode, …} (the desktop).
 * One install/apt job at a time; commands from the page run as argv (`/bin/sh -lc <cmd>` inside the Linux, the
 * user's own Linux), never through a shell on Android's side.
 */
class LinuxEnv private constructor(private val ctx: Context, private val version: String) {
    /** Where events go and how activities start: the current Bridge sets them (the page can be reloaded or the
     *  activity made again while Linux jobs keep running; there's one LinuxEnv per app process). */
    @Volatile var emit: (String, Any) -> Unit = { _, _ -> }
    @Volatile var startActivity: (Intent) -> Unit = { ctx.startActivity(it) }

    private val base = File(ctx.filesDir, "linux")
    private val rootfs = File(base, "rootfs")
    private val sysdata = File(base, "sysdata")
    private val prootTmp = File(base, "tmp")
    private val cache = File(base, "cache")
    private val marker = File(base, "installed.json")
    private val tarball = File(cache, LinuxSources.VERSION + ".tar.xz")
    private val libDir = File(ctx.applicationInfo.nativeLibraryDir)
    private val proot = File(libDir, "libproot.so")
    private val loader = File(libDir, "libproot-loader.so")
    private val loader32 = File(libDir, "libproot-loader32.so")

    private val work = Executors.newCachedThreadPool { r -> Thread(r, "hearth-linux").apply { isDaemon = true } }
    private val timers = Executors.newSingleThreadScheduledExecutor { r -> Thread(r, "hearth-linux-timer").apply { isDaemon = true } }

    /** The install/apt/reset job running now (only one at a time). */
    private data class Busy(val kind: String, @Volatile var pct: Int, @Volatile var text: String, @Volatile var cancel: Boolean = false)
    private val busy = AtomicReference<Busy?>(null)
    private class Job(val id: String, val kind: String, val process: Process, val started: Long)
    private val jobs = ConcurrentHashMap<String, Job>()
    private val nextJob = AtomicInteger(1)
    @Volatile private var x11: Process? = null
    @Volatile private var lastError: String? = null
    @Volatile private var sizeCache: Long = -1
    @Volatile private var keptAlive = false

    // ================================================================ the page's calls
    fun call(method: String, a: JSONObject): Any = when (method) {
        "linux.status" -> status()
        "linux.install" -> install(false)
        "linux.run" -> run(a)
        "linux.apt" -> apt(a)
        "linux.stop" -> stop(a.optString("job", ""))
        "linux.openX11" -> openX11(a)
        "linux.trustX11" -> { confirm(a); trustX11(a.optString("signer", "")) }
        "linux.reset" -> { confirm(a); install(true) }
        "linux.remove" -> { confirm(a); remove() }
        "linux.desktop.status" -> desktopStatus()
        "linux.desktop.install" -> desktopInstall()
        "linux.desktop.start" -> desktopStart(a)
        "linux.desktop.stop" -> desktopStop()
        else -> throw Exception("That isn't in Hearth for Android yet")
    }

    private fun confirm(a: JSONObject) { if (!a.optBoolean("confirm")) throw Exception("Confirm first") }

    fun installed(): Boolean = marker.isFile && File(rootfs, "bin").exists()

    fun status(): JSONObject {
        val ok = installed()
        val b = busy.get()
        if (ok && sizeCache < 0 && b == null) sizeCache = SafeFs.size(rootfs)
        val debian = runCatching { File(rootfs, "etc/debian_version").takeIf { ok && it.isFile }?.readText()?.trim() }.getOrNull()
        return JSONObject()
            .put("installed", ok)
            .put("sizeBytes", if (ok) sizeCache.coerceAtLeast(0) else 0)
            .put("distro", LinuxSources.DISTRO)
            .put("version", if (ok) (debian?.let { "Debian $it" } ?: LinuxSources.VERSION) else LinuxSources.VERSION)
            .put("running", jobs.isNotEmpty() || b != null)
            .put("jobs", JSONArray(jobs.values.sortedBy { it.started }.map { JSONObject().put("job", it.id).put("kind", it.kind) }))
            .put("busy", if (b == null) JSONObject.NULL else JSONObject().put("kind", b.kind).put("pct", b.pct).put("text", b.text))
            .put("x11App", x11Info() != null)
            .put("x11Running", x11?.isAlive == true)
            .put("desktopRunning", desk != null)
            .put("supported", unsupported() == null)
            .put("unsupported", unsupported() ?: JSONObject.NULL)
            .put("freeBytes", freeBytes())
            .put("needBytes", LinuxSources.NEED_FREE_BYTES)
            .put("sdcard", sdcard() != null)
            .put("error", lastError ?: JSONObject.NULL)
    }

    /** Why this device can't run Hearth Linux, or null. */
    private fun unsupported(): String? {
        if ("arm64-v8a" !in Build.SUPPORTED_ABIS) return "Hearth Linux needs a 64-bit ARM device (this one is ${Build.SUPPORTED_ABIS.firstOrNull() ?: "unknown"})."
        if (!proot.isFile) return "This build of Hearth doesn't include PRoot (lib/arm64-v8a/libproot.so)."
        val page = runCatching { Os.sysconf(OsConstants._SC_PAGESIZE) }.getOrDefault(4096L)
        if (page != 4096L) return "This device uses ${page / 1024} KB memory pages; this version's PRoot is built for 4 KB pages. A 16 KB build is planned."
        return null
    }

    private fun freeBytes(): Long = runCatching { base.mkdirs(); StatFs(base.path).availableBytes }.getOrDefault(-1L)

    // ================================================================ install
    private fun start(kind: String): Busy {
        unsupported()?.let { throw Exception(it) }
        val b = Busy(kind, 0, "Starting…")
        if (!busy.compareAndSet(null, b)) throw Exception("Hearth Linux is busy: ${busy.get()?.text ?: "another job"}. Wait for it to finish.")
        keepAlive()
        return b
    }

    private fun install(reset: Boolean): JSONObject {
        if (!reset && installed()) throw Exception("Linux is already installed")
        val need = LinuxSources.NEED_FREE_BYTES - (if (tarball.isFile) tarball.length() else 0L) - (if (reset) SafeFs.size(rootfs) else 0L)
        val free = freeBytes()
        if (free in 0 until need) throw Exception("Not enough free space: Hearth Linux needs about ${gb(need)}, this device has ${gb(free)} free.")
        val b = start(if (reset) "reset" else "install")
        if (reset) stopAll()
        work.execute { runInstall(b) }
        return JSONObject().put("started", true).put("job", b.kind)
    }

    private fun progress(b: Busy, pct: Int, text: String) {
        if (pct == b.pct && text == b.text) return
        b.pct = pct; b.text = text
        emit("linux.progress", JSONObject().put("kind", b.kind).put("pct", pct).put("text", text))
    }

    private fun runInstall(b: Busy) {
        lastError = null
        try {
            listOf(base, cache, prootTmp, sysdata).forEach { it.mkdirs() }
            marker.delete(); sizeCache = -1
            // 1. The download (0–45 %), unless a verified copy is kept from before.
            if (!(tarball.isFile && SafeFs.sha256(tarball) == LinuxSources.ROOTFS_SHA256)) {
                tarball.delete()
                download(b)
            }
            // 2. Unpacking (45–85 %).
            progress(b, 45, "Unpacking Linux…")
            if (rootfs.exists()) SafeFs.deleteTree(rootfs)
            rootfs.mkdirs()
            val total = tarball.length().coerceAtLeast(1)
            val counting = Counting(FileInputStream(tarball))
            var lastPct = -1
            val ex = TarExtractor(rootfs, LinuxSources.ROOTFS_STRIP, AndroidFsOps)
            XZInputStream(BufferedInputStream(counting, 1 shl 16)).use { xz ->
                ex.extract(xz) {
                    val p = 45 + (40 * counting.count / total).toInt()
                    if (p != lastPct) { lastPct = p; progress(b, p, "Unpacking Linux… ${ex.files} files") }
                    b.cancel
                }
            }
            if (!File(rootfs, "etc").isDirectory || !File(rootfs, "bin").exists()) throw Exception("The Linux download doesn't look like a Linux system (no /etc or /bin)")
            Logs.log("linux: unpacked ${ex.files} files (${ex.skipped} skipped)")
            // 3. Hearth's own files (85–88 %).
            progress(b, 86, "Setting up…")
            setup()
            // 4. apt (88–100 %): the package lists, so Install packages works.
            progress(b, 88, "Getting the package lists (apt update)…")
            val code = runBlocking(b, "locale-gen >/dev/null 2>&1 || true; apt-get update", root = true) { line ->
                progress(b, (b.pct + 1).coerceAtMost(98), line.take(120))
            }
            marker.writeText(JSONObject().put("version", LinuxSources.VERSION).put("at", System.currentTimeMillis()).put("hearth", version).toString())
            sizeCache = -1
            val note = if (code == 0) null else "Linux is installed, but apt update didn't work (no internet?). Try Install packages later."
            progress(b, 100, note ?: "Linux is ready")
            done(b, true, note)
        } catch (e: Throwable) {
            val msg = if (b.cancel) "Cancelled" else (e.message ?: e.toString())
            Logs.log("linux: ${b.kind} failed: ${e.stackTraceToString().take(2000)}")
            lastError = msg
            done(b, false, msg)
        }
    }

    private fun done(b: Busy, ok: Boolean, text: String?) {
        busy.compareAndSet(b, null)
        keepAlive()
        val d = JSONObject().put("kind", b.kind).put("ok", ok).put("status", status())
        if (text != null) d.put(if (ok) "text" else "error", text)
        emit("linux.done", d)
    }

    /** Download to a .part file (picked up again where it stopped), https only, then the SHA-256 must match. */
    private fun download(b: Busy) {
        val part = File(cache, tarball.name + ".part")
        var last: Exception? = null
        for (url in LinuxSources.ROOTFS_URLS) {
            if (b.cancel) throw InterruptedException("Cancelled")
            try {
                fetch(url, part, b)
                progress(b, 44, "Checking the download…")
                val sum = SafeFs.sha256(part)
                if (sum != LinuxSources.ROOTFS_SHA256) {
                    part.delete()
                    throw Exception("The download's checksum doesn't match (got ${sum.take(12)}…): it was damaged or changed on the server.")
                }
                if (!part.renameTo(tarball)) throw Exception("Couldn't keep the download")
                return
            } catch (e: InterruptedException) { throw e }
            catch (e: Exception) { last = e; Logs.log("linux: download from $url failed: $e") }
        }
        throw last ?: Exception("Couldn't download Linux")
    }

    private fun fetch(url: String, part: File, b: Busy) {
        var have = if (part.isFile) part.length() else 0L
        val c = URL(url).openConnection() as HttpURLConnection
        c.instanceFollowRedirects = true          // https → https only (HttpURLConnection never follows a switch to http)
        c.connectTimeout = 20_000; c.readTimeout = 60_000
        c.setRequestProperty("User-Agent", "Hearth/$version (Android)")
        if (have > 0) c.setRequestProperty("Range", "bytes=$have-")
        try {
            val code = c.responseCode
            if (c !is HttpsURLConnection || c.url.protocol != "https") throw Exception("Refused a download that isn't https")
            val append = when (code) {
                206 -> true
                200 -> { have = 0; false }
                416 -> return                                     // already complete: the checksum decides
                else -> throw Exception("The server answered $code")
            }
            val len = c.contentLengthLong
            val total = if (len > 0) have + len else -1L
            var got = have
            var lastPct = -1
            c.inputStream.use { inp ->
                FileOutputStream(part, append).use { out ->
                    val buf = ByteArray(1 shl 16)
                    while (true) {
                        if (b.cancel) throw InterruptedException("Cancelled")
                        val n = inp.read(buf); if (n < 0) break
                        out.write(buf, 0, n); got += n
                        val p = if (total > 0) (44 * got / total).toInt() else 0
                        if (p != lastPct) { lastPct = p; progress(b, p, "Downloading Linux… ${mb(got)}" + (if (total > 0) " of ${mb(total)}" else "")) }
                    }
                }
            }
            if (total > 0 && got < total) throw Exception("The download stopped early (${mb(got)} of ${mb(total)})")
        } finally { c.disconnect() }
    }

    /** /etc/resolv.conf, /etc/hosts, the hearth user and its home, en_US.UTF-8, /tmp, and the /proc stand-ins. */
    private fun setup() {
        writeFresh("etc/resolv.conf", LinuxSetup.RESOLV)
        writeFresh("etc/hosts", LinuxSetup.HOSTS)
        for (f in listOf("passwd", "group", "shadow", "gshadow")) {
            val file = inRoot("etc/$f") ?: continue
            if (!file.exists() && f == "gshadow") continue
            file.setWritable(true, true)
            file.writeText(LinuxSetup.addUser(f, if (file.isFile) file.readText() else ""))
        }
        inRoot("etc/locale.gen")?.takeIf { it.isFile }?.let { it.setWritable(true, true); it.writeText(LinuxSetup.localeGen(it.readText())) }
        val home = inRoot("home/${LinuxSources.USER}")
        if (home != null && !home.exists()) {
            home.mkdirs()
            inRoot("etc/skel")?.listFiles()?.forEach { f ->
                if (Files.isRegularFile(f.toPath(), LinkOption.NOFOLLOW_LINKS)) runCatching { f.copyTo(File(home, f.name), overwrite = true) }
            }
        }
        for (d in listOf("tmp", ".l2s")) inRoot(d)?.let { it.mkdirs(); runCatching { Os.chmod(it.path, if (d == "tmp") 0b111_111_111 else 0b111_000_000) } }
        writeSysdata()
    }

    private fun writeSysdata() {
        sysdata.mkdirs()
        File(sysdata, "sys_empty").mkdirs()
        val cpus = Runtime.getRuntime().availableProcessors()
        for (e in FakeProc.entries(cpus)) {
            val f = File(sysdata, e.name)
            if (Files.isSymbolicLink(f.toPath())) f.delete()
            f.writeText(e.content)
        }
    }

    /** A path inside the rootfs whose folders are all real folders (no symlink on the way), or null. */
    private fun inRoot(rel: String): File? {
        var p = rootfs
        val parts = rel.split('/')
        for ((i, part) in parts.withIndex()) {
            p = File(p, part)
            if (i < parts.size - 1 && (Files.isSymbolicLink(p.toPath()) || (p.exists() && !p.isDirectory))) return null
        }
        return p
    }

    /** Replaces a file in the rootfs (a symlink there is removed, never written through). */
    private fun writeFresh(rel: String, text: String) {
        val f = inRoot(rel) ?: return
        Files.deleteIfExists(f.toPath())
        f.parentFile?.mkdirs()
        f.writeText(text)
    }

    // ================================================================ running things in the Linux
    private fun readable(path: String): Boolean = try { FileInputStream(path).use { it.read() }; true } catch (e: Exception) { false }

    /** Android's shared storage, when Hearth may read it (All files access); otherwise nothing is bound. */
    @Suppress("DEPRECATION")
    private fun sdcard(): String? = runCatching {
        val d = Environment.getExternalStorageDirectory()
        d.takeIf { it.canRead() && it.list() != null }?.absolutePath
    }.getOrNull()

    private fun process(inner: List<String>, root: Boolean, extraEnv: Map<String, String> = emptyMap()): Process {
        if (!installed() && busy.get()?.kind != "install" && busy.get()?.kind != "reset") throw Exception("Install Linux first")
        unsupported()?.let { throw Exception(it) }
        if (!File(sysdata, "stat").isFile) writeSysdata()
        prootTmp.mkdirs()
        val argv = ProotArgs.build(ProotSpec(
            proot = proot.path, rootfs = rootfs.path, sysdata = sysdata.path, command = inner, root = root,
            sdcard = sdcard(), hasDevFd = File("/dev/fd").exists(), readable = ::readable))
        val pb = ProcessBuilder(argv).redirectErrorStream(true).directory(base)
        val env = pb.environment()
        val keep = listOf("ANDROID_ROOT", "ANDROID_DATA", "ANDROID_ART_ROOT", "ANDROID_I18N_ROOT", "ANDROID_TZDATA_ROOT", "ANDROID_RUNTIME_ROOT")
            .associateWith { System.getenv(it) }.filterValues { it != null }
        env.clear()
        env.putAll(keep.mapValues { it.value!! })
        env.putAll(ProotArgs.env(rootfs.path, prootTmp.path, loader.path, loader32.takeIf { it.isFile }?.path, root, extraEnv))
        return pb.start()
    }

    /** Runs a shell line in the Linux and waits (install's apt update); each output line goes to [line]. */
    private fun runBlocking(b: Busy, cmd: String, root: Boolean, line: (String) -> Unit): Int {
        val p = process(listOf("/bin/sh", "-c", cmd), root, mapOf("DEBIAN_FRONTEND" to "noninteractive"))
        val id = b.kind
        val buf = ArrayList<String>()
        var lastFlush = System.currentTimeMillis()
        p.inputStream.bufferedReader().useLines { seq ->
            for (l in seq) {
                if (b.cancel) { p.destroy(); break }
                val t = l.take(2000)
                line(t); buf.add(t)
                if (buf.size >= 50 || System.currentTimeMillis() - lastFlush > 150) { output(id, buf); lastFlush = System.currentTimeMillis() }
            }
        }
        output(id, buf)
        if (!p.waitFor(30, TimeUnit.SECONDS)) p.destroyForcibly()
        return runCatching { p.exitValue() }.getOrDefault(-1)
    }

    private fun output(job: String, buf: MutableList<String>) {
        if (buf.isEmpty()) return
        emit("linux.output", JSONObject().put("job", job).put("lines", JSONArray(buf.toList())))
        buf.clear()
    }

    /** Starts [cmd] (`/bin/sh -lc` inside the Linux) as a job: its output streams as linux.output, then linux.exit. */
    private fun startJob(kind: String, cmd: String, root: Boolean, timeoutSec: Int, extraEnv: Map<String, String> = emptyMap(),
                         onLine: ((String) -> Boolean)? = null, after: ((Int) -> Unit)? = null): String {
        if (jobs.size >= 6) throw Exception("Too many commands running: stop one first")
        val id = "j" + nextJob.getAndIncrement()
        val p = process(listOf("/bin/sh", "-lc", cmd), root, extraEnv)
        val job = Job(id, kind, p, System.currentTimeMillis())
        jobs[id] = job
        keepAlive()
        Logs.log("linux: $id ($kind${if (root) ", root" else ""}) started")
        if (timeoutSec > 0) timers.schedule(Runnable { if (p.isAlive) { output(id, mutableListOf("[Hearth: stopped after $timeoutSec s]")); stopProcess(p) } }, timeoutSec.toLong(), TimeUnit.SECONDS)
        work.execute {
            val buf = ArrayList<String>()
            var lastFlush = System.currentTimeMillis()
            try {
                p.inputStream.bufferedReader().useLines { seq ->
                    for (l in seq) {
                        if (onLine != null && !onLine(l)) continue          // a line the job's owner took (progress)
                        buf.add(l.take(2000))
                        if (buf.size >= 100 || System.currentTimeMillis() - lastFlush > 120) { output(id, buf); lastFlush = System.currentTimeMillis() }
                    }
                }
            } catch (e: Exception) { buf.add("[Hearth: ${e.message}]") }
            output(id, buf)
            val code = runCatching { p.waitFor() }.getOrDefault(-1)
            jobs.remove(id)
            sizeCache = -1
            Logs.log("linux: $id exited $code")
            emit("linux.exit", JSONObject().put("job", id).put("kind", kind).put("code", code))
            after?.invoke(code)
            keepAlive()
        }
        return id
    }

    private fun run(a: JSONObject): JSONObject {
        if (!installed()) throw Exception("Install Linux first")
        val cmd = LinuxInput.command(a.optString("cmd", ""))
        val id = startJob("run", cmd, a.optBoolean("root"), a.optInt("timeout", 0).coerceIn(0, 24 * 3600))
        return JSONObject().put("job", id)
    }

    private fun apt(a: JSONObject): JSONObject {
        if (!installed()) throw Exception("Install Linux first")
        val action = a.optString("action", "")
        val list = a.optJSONArray("packages")?.let { arr -> (0 until arr.length()).map { arr.optString(it, "") } }
            ?: LinuxInput.splitPackages(a.optString("packages", ""))
        val argv = LinuxInput.aptArgs(action, list)
        // No package lists yet (or they were cleaned): get them first.
        val lists = File(rootfs, "var/lib/apt/lists").list()?.any { it.contains("_Packages") } == true
        val cmd = (if (!lists || action == "upgrade") "apt-get update && " else "") + LinuxInput.shellJoin(argv)
        val b = start("apt")
        progress(b, 0, "apt-get ${argv.drop(argv.indexOf(action).coerceAtLeast(0)).joinToString(" ").take(100)}")
        return try {
            val id = startJob("apt", cmd, root = true, timeoutSec = 0, extraEnv = mapOf("DEBIAN_FRONTEND" to "noninteractive")) { code: Int ->
                done(b, code == 0, if (code == 0) "Done" else "apt-get stopped with code $code")
            }
            JSONObject().put("job", id)
        } catch (e: Exception) { busy.compareAndSet(b, null); keepAlive(); throw e }
    }

    private fun stopProcess(p: Process) {
        p.destroy()                                   // SIGTERM: PRoot (--kill-on-exit) takes everything it started with it
        timers.schedule(Runnable { if (p.isAlive) p.destroyForcibly() }, 3, TimeUnit.SECONDS)
    }

    private fun stop(job: String): JSONObject {
        val b = busy.get()
        // The desktop's install is an apt job: cancelling it stops apt-get (and the busy state ends with the job).
        if (b != null && b.kind == "desktop" && (job == "desktop" || job.isEmpty())) {
            jobs.values.filter { it.kind == "desktop-install" }.forEach { stopProcess(it.process) }
            return JSONObject().put("ok", true)
        }
        if (b != null && (job == b.kind || job.isEmpty() && b.kind != "apt")) { b.cancel = true; return JSONObject().put("ok", true) }
        val j = jobs[job] ?: throw Exception("That command isn't running anymore")
        stopProcess(j.process)
        return JSONObject().put("ok", true)
    }

    private fun stopAll() {
        desk?.let { endDesktop(it) }
        jobs.values.forEach { stopProcess(it.process) }
        x11?.let { stopProcess(it) }; x11 = null
    }

    private fun remove(): JSONObject {
        if (busy.get() != null) throw Exception("Hearth Linux is busy: wait for it to finish first")
        stopAll()
        Thread.sleep(500)
        SafeFs.deleteTree(base)
        sizeCache = -1; lastError = null
        keepAlive()
        Logs.log("linux: removed")
        return status()
    }

    // ================================================================ Termux:X11
    /** The SHA-256 of the certificate the installed Termux:X11 is signed with (lowercase hex), or null. */
    private fun x11Signer(): String? = runCatching {
        val pm = ctx.packageManager
        val certs: List<ByteArray> = if (Build.VERSION.SDK_INT >= 28) {
            val info = pm.getPackageInfo(X11_PACKAGE, PackageManager.GET_SIGNING_CERTIFICATES)
            val si = info.signingInfo ?: return@runCatching null
            (if (si.hasMultipleSigners()) si.apkContentsSigners else si.signingCertificateHistory)?.map { it.toByteArray() } ?: emptyList()
        } else {
            @Suppress("DEPRECATION")
            pm.getPackageInfo(X11_PACKAGE, PackageManager.GET_SIGNATURES).signatures?.map { it.toByteArray() } ?: emptyList()
        }
        val first = certs.firstOrNull() ?: return@runCatching null
        java.security.MessageDigest.getInstance("SHA-256").digest(first).joinToString("") { "%02x".format(it) }
    }.getOrNull()

    /** Termux:X11's code runs inside Hearth (its X server), so the person allows it once; the signer they allowed is
     *  remembered, and a Termux:X11 signed by someone else (replaced, or an impostor under the same name) asks again. */
    private val x11Trust get() = File(base, "x11-trusted-signer")
    private fun trustX11(signer: String): JSONObject {
        val now = x11Signer() ?: throw Exception("Termux:X11 isn't installed")
        if (signer != now) throw Exception("Termux:X11 changed while you were deciding. Try again.")
        base.mkdirs(); x11Trust.writeText(now)
        Logs.log("x11: Termux:X11 allowed (signer ${now.take(16)}…)")
        return JSONObject().put("ok", true)
    }

    private fun x11Info() = runCatching {
        @Suppress("DEPRECATION")
        ctx.packageManager.getApplicationInfo(X11_PACKAGE, 0)
    }.getOrNull()

    /**
     * Shows a Linux program in Termux:X11 (com.termux.x11, installed on its own; Termux itself isn't needed). Its X
     * server is code inside the Termux:X11 APK that normally runs from Termux's shell (`termux-x11 :0`, which starts
     * `app_process` with the APK on CLASSPATH). Hearth starts it the same way, with TMPDIR = the rootfs's /tmp, so the
     * server's socket is /tmp/.X11-unix/X0 inside the Linux (DISPLAY=:0) and it finds the Linux's keyboard data
     * (/usr/share/X11/xkb, package xkb-data). The server tells the Termux:X11 app (a broadcast) and the app shows it.
     */
    private fun openX11(a: JSONObject): JSONObject {
        if (!installed()) throw Exception("Install Linux first")
        val cmd = LinuxInput.command(a.optString("cmd", "").ifBlank { "xterm" })
        x11Needs(cmd, checkProgram = true)?.let { return it }
        x11Launch("x11", cmd)
        return JSONObject().put("ok", true).put("starting", true)
    }

    /** What stops Termux:X11 from showing [cmd] (the app missing, not allowed yet, packages missing) as the page's
     *  answer, or null when it can start. */
    private fun x11Needs(cmd: String, checkProgram: Boolean): JSONObject? {
        x11Info() ?: return JSONObject().put("ok", false).put("needApp", true).put("url", X11_URL)
            .put("text", "Install Termux:X11 (com.termux.x11) to see Linux apps. Termux itself isn't needed.")
        val signer = x11Signer() ?: throw Exception("Couldn't read who signed Termux:X11, so Hearth won't run it.")
        val trusted = runCatching { x11Trust.readText().trim() }.getOrNull()
        if (trusted != signer)
            return JSONObject().put("ok", false).put("needTrust", true).put("signer", signer).put("changed", trusted != null)
                .put("text", (if (trusted != null) "Termux:X11 is now signed by someone else than the one you allowed (it was replaced). " else "") +
                    "To show Linux apps, Hearth runs Termux:X11's display code inside itself, with Hearth's permissions. Only allow it if you installed Termux:X11 yourself from its official page (github.com/termux/termux-x11 or F-Droid). Signer: ${signer.take(16)}…")
        // What's missing first: the X keyboard data (xkb-data), and the program itself when it's a plain name that
        // isn't in the Linux yet (xterm by default), offered as a package of the same name.
        val need = ArrayList<String>()
        if (!File(rootfs, "usr/share/X11/xkb").isDirectory) need += "xkb-data"
        val prog = cmd.substringBefore(' ')
        if (checkProgram && LinuxInput.isPackageName(prog) && !inPath(prog)) need += prog
        if (need.isNotEmpty())
            return JSONObject().put("ok", false).put("needPackages", JSONArray(need))
                .put("text", "Install these packages first: ${need.joinToString(" ")}" + (if ("xkb-data" in need) " (xkb-data is the keyboard data every X app needs)" else ""))
        return null
    }

    private fun inPath(prog: String) = listOf("usr/bin", "bin", "usr/local/bin", "usr/games", "usr/sbin")
        .any { File(rootfs, "$it/$prog").exists() || Files.isSymbolicLink(File(rootfs, "$it/$prog").toPath()) }

    /**
     * Starts Termux:X11's server (once), brings Termux:X11 to the front, waits (up to 15 s) for its display and then
     * runs [cmd] on it as job [kind]. All of it on [work], never on the bridge's thread: the page hears linux.x11
     * {ok, job, kind} or {ok:false, error, kind}. [then] gets the job (or null when it didn't start).
     */
    private fun x11Launch(kind: String, cmd: String, then: ((String?) -> Unit)? = null) {
        val ai = x11Info() ?: throw Exception("Termux:X11 isn't installed")
        work.execute {
            try {
                val socket = File(rootfs, "tmp/.X11-unix/X0")
                if (x11?.isAlive != true) {
                    runCatching { socket.delete(); File(rootfs, "tmp/.X0-lock").delete() }
                    val pb = ProcessBuilder("/system/bin/app_process", "-Xnoimage-dex2oat", "/", "--nice-name=hearth-x11", X11_ENTRY, ":0")
                        .redirectErrorStream(true).directory(File(rootfs, "tmp"))
                    val env = pb.environment()
                    env.keys.filter { it.startsWith("LD_") || it == "CLASSPATH" }.forEach { env.remove(it) }
                    env["CLASSPATH"] = ai.sourceDir
                    env["TMPDIR"] = File(rootfs, "tmp").path
                    env["XKB_CONFIG_ROOT"] = File(rootfs, "usr/share/X11/xkb").path
                    val p = pb.start()
                    x11 = p
                    keepAlive()
                    work.execute {
                        runCatching { p.inputStream.bufferedReader().useLines { s -> s.forEach { Logs.log("x11: " + it.take(300)) } } }
                        Logs.log("x11: server exited ${runCatching { p.waitFor() }.getOrDefault(-1)}")
                        if (x11 === p) x11 = null
                        keepAlive()
                    }
                }
                startActivity(Intent().setComponent(ComponentName(X11_PACKAGE, "$X11_PACKAGE.MainActivity")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                val until = System.currentTimeMillis() + 15_000
                while (!socket.exists() && System.currentTimeMillis() < until && x11?.isAlive == true) Thread.sleep(250)
                if (!socket.exists()) throw Exception("Termux:X11's display didn't start. Open Termux:X11 once on its own, then try again (Settings › System › Save logs has the details).")
                val id = startJob(kind, cmd, root = false, timeoutSec = 0, extraEnv = mapOf("DISPLAY" to ":0"), after = { code -> x11JobEnded(kind, code) })
                then?.invoke(id)
                emit("linux.x11", JSONObject().put("ok", true).put("job", id).put("kind", kind))
            } catch (e: Exception) {
                Logs.log("x11: $kind didn't start: ${e.message}")
                then?.invoke(null)
                emit("linux.x11", JSONObject().put("ok", false).put("kind", kind).put("error", e.message ?: e.toString()))
            }
        }
    }

    /** A job on Termux:X11 ended: when it was the desktop's session, the desktop is over. */
    private fun x11JobEnded(kind: String, code: Int) {
        if (kind != "desktop") return
        val d = desk
        if (d != null && d.mode == "x11" && d.job != null && !jobs.containsKey(d.job!!)) desk = null
        emit("linux.desktop", JSONObject().put("state", "stopped").put("mode", "x11").put("code", code))
    }

    // ================================================================ the Linux desktop (stage 2)
    /**
     * XFCE, shown one of two ways (the person's choice is remembered): "hearth" (default) runs TigerVNC's Xvnc in the
     * Linux and shows it in Hearth's own page with noVNC, through [WsBridge]; "x11" shows the session in Termux:X11.
     * One desktop at a time. Events: linux.desktop {state: 'ready'|'stopped'|'failed', mode, …}.
     */
    private class Desk(val mode: String, val width: Int, val height: Int, val vncPort: Int, val password: String,
                       val token: String, val ws: WsBridge?) {
        @Volatile var job: String? = null
        @Volatile var ready = false
        @Volatile var stopping = false
    }
    @Volatile private var desk: Desk? = null
    private val deskModeFile get() = File(base, "desktop-mode")
    private val loopback: InetAddress = InetAddress.getByAddress(byteArrayOf(127, 0, 0, 1))

    /** The packages the desktop still needs (empty: it can start). A program can be a link (Debian's alternatives:
     *  /usr/bin/Xvnc → /etc/alternatives/Xvnc, which only resolves inside the Linux). */
    private fun desktopMissing(): List<String> = DesktopPlan.NEEDS
        .filter { (path, _) -> val f = File(rootfs, path); !(f.exists() || Files.isSymbolicLink(f.toPath())) }
        .map { it.second }.distinct()

    private fun savedMode(): String = runCatching { deskModeFile.readText().trim() }.getOrNull()?.takeIf { it in DesktopPlan.MODES } ?: "hearth"

    private fun desktopStatus(): JSONObject {
        val ok = installed()
        val missing = if (ok) desktopMissing() else DesktopPlan.NEEDS.map { it.second }.distinct()
        val d = desk
        val o = JSONObject().put("linux", ok).put("installed", ok && missing.isEmpty()).put("missing", JSONArray(missing))
            .put("packages", JSONArray(DesktopPlan.PACKAGES)).put("mode", savedMode()).put("x11App", x11Info() != null)
            .put("installing", busy.get()?.kind == "desktop")
            .put("running", d != null).put("method", d?.mode ?: JSONObject.NULL).put("ready", d?.ready == true)
        if (d != null && d.mode == "hearth") o.put("width", d.width).put("height", d.height)
        return o
    }

    /** One apt job (as root) with the desktop's packages; its progress comes from apt's status lines. */
    private fun desktopInstall(): JSONObject {
        if (!installed()) throw Exception("Install Linux first")
        val b = start("desktop")
        progress(b, 1, "Getting the package lists…")
        return try {
            val id = startJob("desktop-install", DesktopPlan.installCommand(), root = true, timeoutSec = 0,
                extraEnv = mapOf("DEBIAN_FRONTEND" to "noninteractive"),
                onLine = { line ->
                    val st = AptStatus.parse(line)
                    if (st != null) { progress(b, AptStatus.overall(st).coerceIn(b.pct, 99), st.text.take(120).ifEmpty { b.text }); false }
                    else { if (b.pct < 5 && line.isNotBlank()) progress(b, b.pct, line.take(120)); true }
                },
                after = { code ->
                    sizeCache = -1
                    val missing = desktopMissing()
                    val ok = code == 0 && missing.isEmpty()
                    if (ok) progress(b, 100, "The Linux desktop is installed")
                    done(b, ok, when {
                        ok -> "The Linux desktop is installed"
                        code == 0 -> "apt finished, but ${missing.joinToString(" ")} is still missing"
                        else -> "apt-get stopped with code $code"
                    })
                })
            JSONObject().put("started", true).put("job", id)
        } catch (e: Exception) { busy.compareAndSet(b, null); keepAlive(); throw e }
    }

    private fun desktopStart(a: JSONObject): JSONObject {
        if (!installed()) throw Exception("Install Linux first")
        unsupported()?.let { throw Exception(it) }
        val mode = DesktopPlan.mode(a.optString("mode", "").ifBlank { savedMode() })
        val size = if (mode == "hearth") DesktopPlan.geometry(a.optInt("width", 0), a.optInt("height", 0)) else null
        val missing = desktopMissing()
        if (missing.isNotEmpty()) return JSONObject().put("ok", false).put("needInstall", true).put("missing", JSONArray(missing))
            .put("text", "The Linux desktop needs these packages first: ${missing.joinToString(" ")}")
        if (busy.get()?.kind == "desktop") throw Exception("The desktop is still being installed")
        base.mkdirs(); deskModeFile.writeText(mode)
        desk?.let { d ->
            if (d.mode == mode && !d.stopping) {
                if (mode == "x11") work.execute { runCatching { startActivity(Intent().setComponent(ComponentName(X11_PACKAGE, "$X11_PACKAGE.MainActivity")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) } }
                return connectInfo(d)
            }
            endDesktop(d)
        }
        return if (mode == "hearth") startHearthDesktop(size!!.first, size.second) else startX11Desktop()
    }

    private fun connectInfo(d: Desk): JSONObject {
        val o = JSONObject().put("ok", true).put("mode", d.mode).put("running", true).put("ready", d.ready)
        if (d.mode == "hearth") o.put("port", d.ws!!.port).put("token", d.token).put("password", d.password).put("width", d.width).put("height", d.height)
        return o
    }

    private fun startHearthDesktop(w: Int, h: Int): JSONObject {
        val vncPort = ServerSocket(0, 1, loopback).use { it.localPort }
        val password = VncPassword.random()
        writePasswd(password)
        val token = WsHandshake.token()
        val ws = WsBridge(token, WsBridge.PAGE_ORIGIN, { Socket().apply { connect(InetSocketAddress(loopback, vncPort), 2000) } }, { Logs.log("desktop: $it") }).start()
        val d = Desk("hearth", w, h, vncPort, password, token, ws)
        desk = d
        try {
            d.job = startJob("desktop", DesktopPlan.script(w, h, vncPort), root = false, timeoutSec = 0,
                extraEnv = mapOf("TERM" to "xterm-256color"),
                onLine = { line -> Logs.log("desktop: " + line.take(300)); true },
                after = { code -> desktopEnded(d, code) })
        } catch (e: Exception) { ws.stop(); if (desk === d) desk = null; throw e }
        work.execute { waitForVnc(d) }
        Logs.log("desktop: starting ${w}x$h, VNC on 127.0.0.1:$vncPort, WebSocket on 127.0.0.1:${ws.port}")
        return connectInfo(d)
    }

    /** The password file Xvnc reads (-rfbauth): the random password of this start, as `vncpasswd -f` writes it. */
    private fun writePasswd(password: String) {
        val f = inRoot(DesktopPlan.PASSWD) ?: throw Exception("Couldn't write the desktop's password (a link is in the way)")
        val dir = f.parentFile!!
        if (Files.isSymbolicLink(dir.toPath())) throw Exception("Couldn't write the desktop's password (a link is in the way)")
        dir.mkdirs()
        Files.deleteIfExists(f.toPath())
        FileOutputStream(f).use { it.write(VncPassword.file(password)) }
        runCatching { Os.chmod(f.path, 0b110_000_000); Os.chmod(dir.path, 0b111_000_000) }
    }

    /** Ready when Xvnc answers on its port with "RFB 003.00x" (up to 45 s: the first start of XFCE is slow). */
    private fun waitForVnc(d: Desk) {
        val until = System.currentTimeMillis() + 45_000
        while (System.currentTimeMillis() < until) {
            if (desk !== d || d.stopping) return
            val answered = runCatching {
                Socket().use { s ->
                    s.connect(InetSocketAddress(loopback, d.vncPort), 500)
                    s.soTimeout = 2000
                    val b = ByteArray(4); var got = 0
                    while (got < 4) { val n = s.getInputStream().read(b, got, 4 - got); if (n < 0) break; got += n }
                    got == 4 && String(b, Charsets.US_ASCII) == "RFB "
                }
            }.getOrDefault(false)
            if (answered) {
                if (desk === d && !d.ready) {
                    d.ready = true
                    Logs.log("desktop: ready")
                    emit("linux.desktop", connectInfo(d).put("state", "ready"))
                }
                return
            }
            Thread.sleep(300)
        }
        if (desk === d && !d.ready) {
            Logs.log("desktop: Xvnc didn't answer in 45 s")
            endDesktop(d)
            emit("linux.desktop", JSONObject().put("state", "failed").put("mode", "hearth")
                .put("error", "The desktop didn't start (Xvnc didn't answer). Settings › System › Save logs has the details."))
        }
    }

    private fun desktopEnded(d: Desk, code: Int) {
        d.ws?.stop()
        if (desk === d) desk = null
        runCatching { inRoot(DesktopPlan.PASSWD)?.let { Files.deleteIfExists(it.toPath()) } }
        Logs.log("desktop: ended ($code)")
        emit("linux.desktop", JSONObject().put("state", "stopped").put("mode", d.mode).put("code", code))
    }

    private fun startX11Desktop(): JSONObject {
        x11Needs(DesktopPlan.SESSION, checkProgram = false)?.let { return it }
        val d = Desk("x11", 0, 0, 0, "", "", null)
        desk = d
        try {
            x11Launch("desktop", DesktopPlan.SESSION) { id ->
                if (id == null) { if (desk === d) desk = null }
                else { d.job = id; d.ready = true; if (desk !== d || d.stopping) jobs[id]?.let { stopProcess(it.process) } }
            }
        } catch (e: Exception) { if (desk === d) desk = null; throw e }
        return JSONObject().put("ok", true).put("mode", "x11").put("starting", true)
    }

    private fun endDesktop(d: Desk) {
        d.stopping = true
        d.ws?.stop()
        d.job?.let { id -> jobs[id]?.let { stopProcess(it.process) } }
        if (desk === d) desk = null
    }

    private fun desktopStop(): JSONObject {
        val d = desk ?: return JSONObject().put("ok", true).put("running", false)
        endDesktop(d)
        Logs.log("desktop: stopped from Hearth")
        return JSONObject().put("ok", true).put("running", false)
    }

    // ================================================================ staying alive in the background
    /** While something runs (an install, a command, the X server), a foreground service keeps Android from freezing
     *  or closing Hearth's processes when Hearth isn't in front (e.g. while Termux:X11 is). */
    @Synchronized private fun keepAlive() {
        val active = busy.get() != null || jobs.isNotEmpty() || x11?.isAlive == true
        if (active == keptAlive) return
        keptAlive = active
        val i = Intent(ctx, LinuxService::class.java)
        try { if (active) ContextCompat.startForegroundService(ctx, i) else ctx.stopService(i) }
        catch (e: Exception) { Logs.log("linux: keep-alive service: $e") }
    }

    // ================================================================ helpers
    private class Counting(i: InputStream) : FilterInputStream(i) {
        @Volatile var count = 0L
        override fun read(): Int { val b = super.read(); if (b >= 0) count++; return b }
        override fun read(b: ByteArray, off: Int, len: Int): Int { val n = super.read(b, off, len); if (n > 0) count += n; return n }
    }

    private object AndroidFsOps : FsOps {
        override fun chmod(path: String, mode: Int) = Os.chmod(path, mode)
        override fun symlink(target: String, path: String) = Os.symlink(target, path)
        override fun hardlink(existing: String, path: String) = Os.link(existing, path)
    }

    private fun mb(b: Long) = "%.0f MB".format(java.util.Locale.US, b / 1e6)
    private fun gb(b: Long) = "%.1f GB".format(java.util.Locale.US, b / 1e9)

    companion object {
        @Volatile private var one: LinuxEnv? = null
        @Synchronized fun get(ctx: Context, version: String): LinuxEnv = one ?: LinuxEnv(ctx.applicationContext, version).also { one = it }

        const val X11_PACKAGE = "com.termux.x11"
        const val X11_ENTRY = "com.termux.x11.CmdEntryPoint"
        const val X11_URL = "https://github.com/termux/termux-x11/releases"
    }
}
