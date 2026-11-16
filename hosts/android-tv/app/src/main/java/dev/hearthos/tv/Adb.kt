package dev.hearthos.tv

import android.content.Context
import dadb.AdbKeyPair
import dadb.Dadb
import java.io.File

/**
 * Hearth TV's way to the TV's own adb (127.0.0.1:5555), with its own key. Only TV Cleanup and a few TV settings use
 * it (switching packages off and on, HDMI-CEC, the default home app). The first time, the TV asks "Allow debugging?":
 * tick Always allow. The laptop's install script turns network debugging on (docs/HEARTH-TV.md).
 */
class Adb(private val ctx: Context) {
    private val priv = File(ctx.filesDir, "adbkey")
    private val pub = File(ctx.filesDir, "adbkey.pub")
    private val lock = Any()

    private fun keys(): AdbKeyPair {
        if (!priv.exists() || !pub.exists()) AdbKeyPair.generate(priv, pub)
        return AdbKeyPair.read(priv, pub)
    }

    data class Result(val ok: Boolean, val out: String, val code: Int)

    /** Runs [cmd] in the TV's shell. ok=false with the reason when adb can't be reached or wasn't allowed. */
    fun shell(cmd: String): Result = synchronized(lock) {
        try {
            Dadb.create("127.0.0.1", 5555, keys()).use { d ->
                val r = d.shell(cmd)
                Result(r.exitCode == 0, r.allOutput.trim(), r.exitCode)
            }
        } catch (e: Exception) {
            Logs.add("adb: $cmd: ${e.javaClass.simpleName}: ${e.message}")
            Result(false, "unreachable: ${e.message ?: e.javaClass.simpleName}", -1)
        }
    }

    /** Installs an APK (Hearth TV's own update) through adb: pushed to /data/local/tmp, then pm install -r. */
    fun install(apk: File): Result = synchronized(lock) {
        try {
            Dadb.create("127.0.0.1", 5555, keys()).use { d ->
                val remote = "/data/local/tmp/hearth-tv-update.apk"
                d.push(apk, remote)
                val r = d.shell("pm install -r $remote; rm -f $remote")
                Result(r.exitCode == 0 && r.allOutput.contains("Success"), r.allOutput.trim(), r.exitCode)
            }
        } catch (e: Exception) { Result(false, "unreachable: ${e.message}", -1) }
    }

    fun reachable(): Boolean = shell("echo ok").let { it.ok && it.out.contains("ok") }
}
