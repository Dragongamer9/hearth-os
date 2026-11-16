package dev.hearthos.tv

import android.content.Context
import android.net.LocalSocket
import android.net.LocalSocketAddress
import org.json.JSONObject
import java.io.File
import kotlin.concurrent.thread

/**
 * Hearth Link on the TV: the same service as on every Hearth device (hosts/hearth-link, built for Android by CI and
 * shipped as lib/<abi>/libhearthlink.so), run as its own program with its folders in Hearth TV's storage. The page
 * talks to it the same way Hearth's other pages do: one JSON line over its socket (status, pair.open, pair.join...).
 */
object Link {
    @Volatile private var proc: Process? = null
    @Volatile private var keepRunning = false

    private fun bin(ctx: Context) = File(ctx.applicationInfo.nativeLibraryDir, "libhearthlink.so")
    private fun home(ctx: Context) = File(ctx.filesDir, "link").apply { mkdirs() }
    private fun runDir(ctx: Context) = File(ctx.filesDir, "run").apply { mkdirs() }
    fun socket(ctx: Context) = File(runDir(ctx), "hearth-link/link.sock")

    fun available(ctx: Context) = bin(ctx).exists()

    fun start(ctx: Context) {
        if (!available(ctx)) { Logs.add("link: not in this build"); return }
        if (keepRunning) return
        keepRunning = true
        thread(name = "hearth-link", isDaemon = true) {
            var fails = 0
            while (keepRunning) {
                val started = System.currentTimeMillis()
                try {
                    val pb = ProcessBuilder(bin(ctx).path, "serve").redirectErrorStream(true)
                    val env = pb.environment()
                    env["HOME"] = ctx.filesDir.path
                    env["HEARTH_LINK_HOME"] = home(ctx).path
                    env["XDG_RUNTIME_DIR"] = runDir(ctx).path
                    env["XDG_DATA_HOME"] = File(ctx.filesDir, "data").path
                    env["XDG_CONFIG_HOME"] = File(ctx.filesDir, "config").path
                    env["TMPDIR"] = ctx.cacheDir.path
                    env["HEARTH_LINK_KIND"] = "tv"
                    env["HEARTH_LINK_EDITION"] = "Hearth TV"
                    val p = pb.start()
                    proc = p
                    p.inputStream.bufferedReader().forEachLine { Logs.add(it) }
                    val code = p.waitFor()
                    Logs.add("link: stopped ($code)")
                } catch (e: Exception) { Logs.add("link: couldn't start: ${e.message}") }
                // A quick crash backs off; a restart it asked for (a new relay setting) comes right back.
                fails = if (System.currentTimeMillis() - started < 10_000) fails + 1 else 0
                if (fails > 5) { Logs.add("link: keeps stopping; giving up until Hearth TV starts again"); keepRunning = false; break }
                Thread.sleep(1000L * fails)
            }
        }
    }

    /** One request to the service: {ok, data} or {ok:false, error}. Blocking (call it off the main thread). */
    fun call(ctx: Context, cmd: String, args: JSONObject, timeoutMs: Int = 30_000): JSONObject {
        if (!available(ctx)) return JSONObject().put("ok", false).put("error", "Hearth Link isn't in this build")
        return try {
            LocalSocket().use { s ->
                s.connect(LocalSocketAddress(socket(ctx).path, LocalSocketAddress.Namespace.FILESYSTEM))
                s.soTimeout = timeoutMs
                s.outputStream.write((JSONObject().put("id", 1).put("cmd", cmd).put("args", args).toString() + "\n").toByteArray())
                s.outputStream.flush()
                val line = s.inputStream.bufferedReader().readLine() ?: return JSONObject().put("ok", false).put("error", "Hearth Link closed the connection")
                JSONObject(line)
            }
        } catch (e: Exception) {
            JSONObject().put("ok", false).put("error", if (socket(ctx).exists()) (e.message ?: "Hearth Link didn't answer") else "Hearth Link is starting")
        }
    }
}
