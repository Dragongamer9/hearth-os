package dev.hearthos.tv

import android.content.Intent
import android.hardware.input.InputManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.speech.RecognizerIntent
import android.view.InputDevice
import android.view.WindowManager
import android.webkit.JavascriptInterface
import org.json.JSONArray
import org.json.JSONObject
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import kotlin.concurrent.thread

/**
 * The page's way to Android (window.HearthTVNative). Quick answers are returned as JSON text; slow ones take an id
 * and answer later with hearthTV.done(id, result).
 */
class Bridge(private val a: MainActivity) {
    private val app get() = TvApp.app

    private fun later(id: String, work: () -> Any?) {
        thread(isDaemon = true) {
            val r = try { work() } catch (e: Exception) { JSONObject().put("error", e.message ?: e.javaClass.simpleName) }
            val text = when (r) { null -> "null"; is JSONObject, is JSONArray -> r.toString(); is String -> JSONObject.quote(r); else -> r.toString() }
            a.js("hearthTV.done(" + JSONObject.quote(id) + "," + text + ")")
        }
    }

    // Settings (red flag 9: merged, atomic, .bak; see SettingsStore).
    @JavascriptInterface fun settings(): String = app.settings.read().toString()
    @JavascriptInterface fun settingsMerge(patch: String): String = app.settings.merge(JSONObject(patch)).toString()
    @JavascriptInterface fun settingsEvent(): String = (app.settings.event ?: "").also { app.settings.event = null }
    @JavascriptInterface fun state(): String = app.state.read().toString()
    @JavascriptInterface fun stateMerge(patch: String): String = app.state.merge(JSONObject(patch)).toString()

    // Apps.
    @JavascriptInterface fun apps(): String = Apps.list(a).toString()
    @JavascriptInterface fun appOpen(pkg: String): Boolean = Apps.launch(a, pkg)
    @JavascriptInterface fun appSearch(pkg: String, q: String): Boolean = Apps.search(a, pkg, q)
    @JavascriptInterface fun appClearCache(id: String, pkg: String) = later(id) {
        val r = app.adb.shell("pm clear --cache-only $pkg")
        if (r.ok) JSONObject().put("ok", true) else {
            // Older TVs have no --cache-only: Android's own page for the app, where Clear cache is.
            a.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:$pkg")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            JSONObject().put("ok", false).put("opened", true)
        }
    }

    // TV inputs.
    @JavascriptInterface fun inputs(): String = Inputs.list(a).toString()
    @JavascriptInterface fun inputSwitch(id: String): Boolean {
        app.state.merge(JSONObject().put("onInput", id))
        return Inputs.switchTo(a, id)
    }
    @JavascriptInterface fun liveTv(): Boolean = Inputs.liveTv(a)

    // TV Cleanup and the TV settings Hearth TV changes through adb.
    @JavascriptInterface fun cleanupStatus(): String = Cleanup.status(a).toString()
    @JavascriptInterface fun cleanupSet(id: String, group: String, on: Boolean) = later(id) { Cleanup.set(a, group, on) }
    @JavascriptInterface fun cleanupCheck(id: String) = later(id) { JSONObject().put("fixed", JSONArray(Cleanup.watch(a))).put("adb", app.adb.reachable()) }
    @JavascriptInterface fun adbReachable(id: String) = later(id) { app.adb.reachable() }
    /** Only the TV settings Hearth TV's own pages change (HDMI-CEC, ARC, the home app); nothing else. */
    @JavascriptInterface fun tvSetting(id: String, name: String, value: String) = later(id) {
        val allowed = setOf("hdmi_control_enabled", "hdmi_control_auto_wakeup_enabled", "hdmi_control_auto_device_off_enabled", "hdmi_system_audio_control")
        if (name !in allowed || !Regex("^[0-9]$").matches(value)) return@later JSONObject().put("ok", false)
        val r = app.adb.shell("settings put global $name $value")
        JSONObject().put("ok", r.ok).put("adb", !r.out.startsWith("unreachable"))
    }
    @JavascriptInterface fun tvSettingGet(name: String): String = try { Settings.Global.getString(a.contentResolver, name) ?: "" } catch (_: Exception) { "" }

    // Android's own pages.
    @JavascriptInterface fun openSystem(which: String): Boolean {
        val i = when (which) {
            "picture" -> Intent("android.settings.DISPLAY_SETTINGS")
            "sound" -> Intent(Settings.ACTION_SOUND_SETTINGS)
            "bluetooth" -> Intent(Settings.ACTION_BLUETOOTH_SETTINGS)
            "network" -> Intent(Settings.ACTION_WIFI_SETTINGS)
            "developer" -> Intent(Settings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS)
            "screensaver" -> Intent(Settings.ACTION_DREAM_SETTINGS)
            else -> Intent(Settings.ACTION_SETTINGS)
        }.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return try { a.startActivity(i); true } catch (_: Exception) {
            try { a.startActivity(Intent(Settings.ACTION_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)); true } catch (_: Exception) { false }
        }
    }

    // Remotes and controllers connected now (Android's input devices with buttons).
    @JavascriptInterface fun controllers(): String {
        val out = JSONArray()
        val im = a.getSystemService(InputManager::class.java)
        for (devId in im.inputDeviceIds) {
            val d = im.getInputDevice(devId) ?: continue
            if (d.isVirtual) continue
            val s = d.sources
            val pad = (s and InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD || (s and InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
            val keys = (s and InputDevice.SOURCE_KEYBOARD) == InputDevice.SOURCE_KEYBOARD || (s and InputDevice.SOURCE_DPAD) == InputDevice.SOURCE_DPAD
            if (!pad && !keys) continue
            out.put(JSONObject().put("name", d.name).put("pad", pad).put("external", Build.VERSION.SDK_INT >= 29 && d.isExternal))
        }
        return out.toString()
    }

    // Theater.
    @JavascriptInterface fun theater(): String = Theater.list().toString()
    @JavascriptInterface fun theaterSave(links: String) = Theater.save(JSONArray(links))
    @JavascriptInterface fun theaterAdd(url: String) = Theater.add(a, url, "tv")
    @JavascriptInterface fun addPage(): String {
        LocalServer.start()
        val url = LocalServer.url(a) ?: return JSONObject().put("ok", false).toString()
        return JSONObject().put("ok", true).put("url", url).put("qr", Qr.dataUrl(url)).toString()
    }
    @JavascriptInterface fun openViewer(url: String, title: String) {
        a.startActivity(Intent(a, ViewerActivity::class.java).putExtra("url", url).putExtra("title", title))
    }

    // Voice search (Android's own speech screen, the remote's mic).
    @JavascriptInterface fun voice(): Boolean = try {
        @Suppress("DEPRECATION")
        a.startActivityForResult(Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            .putExtra(RecognizerIntent.EXTRA_PROMPT, "Say a title"), MainActivity.VOICE)
        true
    } catch (_: Exception) { false }

    // Hearth Link (pairing, your devices).
    @JavascriptInterface fun link(id: String, cmd: String, args: String) = later(id) { Link.call(a, cmd, JSONObject(args.ifBlank { "{}" })) }
    @JavascriptInterface fun linkAvailable(): Boolean = Link.available(a)

    // Waking the PC (Wake-on-LAN magic packet to the whole network) before My Media plays.
    @JavascriptInterface fun wake(mac: String): Boolean {
        val hex = mac.replace(Regex("[^0-9A-Fa-f]"), "")
        if (hex.length != 12) return false
        val m = ByteArray(6) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
        val pkt = ByteArray(6 + 16 * 6)
        for (i in 0 until 6) pkt[i] = 0xFF.toByte()
        for (r in 0 until 16) System.arraycopy(m, 0, pkt, 6 + r * 6, 6)
        thread(isDaemon = true) {
            try { DatagramSocket().use { s -> s.broadcast = true; for (p in listOf(9, 7)) s.send(DatagramPacket(pkt, pkt.size, InetAddress.getByName("255.255.255.255"), p)) } }
            catch (e: Exception) { Logs.add("wake: ${e.message}") }
        }
        return true
    }

    /** Jellyfin's own "who's here" broadcast (UDP 7359): the servers on this network. */
    @JavascriptInterface fun findJellyfin(id: String) = later(id) {
        val out = JSONArray()
        try {
            DatagramSocket().use { s ->
                s.broadcast = true; s.soTimeout = 1500
                val q = "Who is JellyfinServer?".toByteArray()
                s.send(DatagramPacket(q, q.size, InetSocketAddress("255.255.255.255", 7359)))
                val buf = ByteArray(4096)
                val until = System.currentTimeMillis() + 2000
                while (System.currentTimeMillis() < until) {
                    val p = DatagramPacket(buf, buf.size)
                    try { s.receive(p) } catch (_: Exception) { break }
                    try { out.put(JSONObject(String(p.data, 0, p.length))) } catch (_: Exception) {}
                }
            }
        } catch (e: Exception) { Logs.add("jellyfin: discovery: ${e.message}") }
        out
    }

    /** The PC's network address (MAC) for Wake the PC, from the TV's neighbour table (through adb). */
    @JavascriptInterface fun learnMac(id: String, host: String) = later(id) {
        if (!Regex("^[A-Za-z0-9.-]{1,253}$").matches(host)) return@later ""
        val ip = try { InetAddress.getByName(host).hostAddress ?: host } catch (_: Exception) { host }
        val r = app.adb.shell("ping -c 1 -W 1 $ip >/dev/null 2>&1; ip neigh show $ip")
        Regex("lladdr ([0-9a-fA-F:]{17})").find(r.out)?.groupValues?.get(1) ?: ""
    }

    /** Match the video's frame rate (Settings › Picture and sound): the display mode nearest a multiple of [fps]. */
    @JavascriptInterface fun frameRate(fps: Double) = a.runOnUiThread {
        val d = a.window.decorView.display ?: return@runOnUiThread
        val lp = a.window.attributes
        if (fps <= 0) { lp.preferredDisplayModeId = 0; a.window.attributes = lp; return@runOnUiThread }
        val cur = d.mode
        val best = d.supportedModes.filter { it.physicalWidth == cur.physicalWidth && it.physicalHeight == cur.physicalHeight }
            .minByOrNull { m -> val k = Math.round(m.refreshRate / fps).coerceAtLeast(1L); Math.abs(m.refreshRate - k * fps) * 10 + (if (k > 2) k.toDouble() else 0.0) }
        if (best != null) { lp.preferredDisplayModeId = best.modeId; a.window.attributes = lp; Logs.add("display: ${best.refreshRate} Hz for $fps fps") }
    }

    @JavascriptInterface fun tailscaleInstalled(): Boolean = Apps.installed(a, Tailscale.PKG)
    @JavascriptInterface fun tailscaleConnect() { Tailscale.connect(a) }

    // Updates (About).
    @JavascriptInterface fun version(): String = BuildConfig.VERSION_NAME
    @JavascriptInterface fun device(): String = JSONObject().put("model", Build.MODEL).put("maker", Build.MANUFACTURER)
        .put("build", Build.DISPLAY).put("android", Build.VERSION.RELEASE).put("sdk", Build.VERSION.SDK_INT).toString()
    @JavascriptInterface fun updateCheck(id: String) = later(id) { Updates.check() }
    @JavascriptInterface fun updateInstall(id: String, url: String) = later(id) {
        Updates.install(a, url) { p -> a.event(JSONObject().put("type", "updateProgress").put("pct", p)) }
    }

    // Small things.
    @JavascriptInterface fun keepOn(on: Boolean) = a.runOnUiThread {
        if (on) a.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) else a.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
    @JavascriptInterface fun log(line: String) = Logs.add("page: " + line.take(500))
    @JavascriptInterface fun logs(): String = Logs.text()
}
