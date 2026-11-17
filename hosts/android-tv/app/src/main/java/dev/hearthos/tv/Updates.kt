package dev.hearthos.tv

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * Hearth TV updating itself from inside the app (Settings › About). New versions come with Hearth OS's GitHub releases
 * as hearth-tv-<version>.apk (android-tv.yml adds it). With adb reachable the update installs by itself (Install
 * updates by themselves on); otherwise Android's own install screen asks.
 */
object Updates {
    private const val API = "https://api.github.com/repos/Dragongamer9/hearth-os/releases?per_page=15"
    private val NAME = Regex("""^hearth-tv-(\d+(?:\.\d+){0,2})\.apk$""")

    fun newer(a: String, b: String): Boolean {
        val x = a.split(".").map { it.toIntOrNull() ?: 0 }; val y = b.split(".").map { it.toIntOrNull() ?: 0 }
        for (i in 0 until maxOf(x.size, y.size)) { val d = x.getOrElse(i) { 0 } - y.getOrElse(i) { 0 }; if (d != 0) return d > 0 }
        return false
    }

    /** {current, latest, url, newer} or {error}. */
    fun check(): JSONObject {
        val cur = BuildConfig.VERSION_NAME.removeSuffix("-debug")
        return try {
            val (code, body) = Net.get(API, headers = mapOf("Accept" to "application/vnd.github+json"))
            if (code != 200) return JSONObject().put("current", cur).put("error", "GitHub answered $code")
            val rels = JSONArray(body)
            var best: String? = null; var url: String? = null
            for (i in 0 until rels.length()) {
                val r = rels.getJSONObject(i)
                if (r.optBoolean("draft") || r.optBoolean("prerelease")) continue
                val assets = r.optJSONArray("assets") ?: continue
                for (j in 0 until assets.length()) {
                    val a = assets.getJSONObject(j)
                    val m = NAME.find(a.optString("name")) ?: continue
                    val v = m.groupValues[1]
                    if (best == null || newer(v, best)) { best = v; url = a.optString("browser_download_url") }
                }
            }
            JSONObject().put("current", cur).put("latest", best ?: cur).put("url", url ?: "").put("newer", best != null && newer(best, cur))
        } catch (e: Exception) {
            JSONObject().put("current", cur).put("error", "Couldn't reach GitHub: ${e.message}")
        }
    }

    /** Downloads [url] and installs it: through adb when it can (no questions), otherwise Android's install screen. */
    fun install(ctx: Context, url: String, onProgress: (Int) -> Unit): JSONObject {
        val f = File(ctx.cacheDir, "updates/hearth-tv.apk")
        Net.download(url, f) { got, total -> if (total > 0) onProgress((got * 100 / total).toInt()) }
        val adb = TvApp.app.adb
        if (adb.reachable()) {
            val r = adb.install(f)
            if (r.ok) { Logs.add("updates: installed through adb"); return JSONObject().put("ok", true).put("how", "adb") }
            Logs.add("updates: adb install failed: ${r.out}")
        }
        if (Build.VERSION.SDK_INT >= 26 && !ctx.packageManager.canRequestPackageInstalls()) {
            ctx.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + ctx.packageName)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            return JSONObject().put("ok", false).put("message", "Allow Hearth TV to install updates, then try again")
        }
        val uri = FileProvider.getUriForFile(ctx, ctx.packageName + ".files", f)
        ctx.startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/vnd.android.package-archive")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK))
        return JSONObject().put("ok", true).put("how", "android")
    }
}
