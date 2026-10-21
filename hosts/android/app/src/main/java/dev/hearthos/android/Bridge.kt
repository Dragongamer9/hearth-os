package dev.hearthos.android

import android.content.ActivityNotFoundException
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.DocumentsContract
import android.provider.Settings
import androidx.webkit.WebViewCompat
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import kotlin.math.roundToInt

/**
 * Hearth's host on Android: answers the page's calls ({type:'call', id, method, args} -> {type:'result', id, ok,
 * value|error}) and tells it what happens ({type:'event', name, data}), the same messages as the Linux host
 * (hosts/hearth-core bridge.rs) and Ember (hosts/windows). Calls run one at a time on their own thread; anything
 * that starts an activity runs on the main thread.
 *
 * What's here: the library (apps that are games, apps you added, ROMs), starting games and their play time,
 * favorites/hidden/collections, the profile, the device's status (battery, Wi-Fi, volume), Android's settings
 * pages, and logs. Everything else answers "not in Hearth for Android yet", and host.info's features tell the page
 * what to show.
 */
class Bridge(private val act: MainActivity) {
    val version: String = BuildConfig.VERSION_NAME.removeSuffix("-debug")
    private val store = Store(File(act.filesDir, "library.json"))
    private val apps = Apps(act)
    val art = IconArt(act)
    private val roms = RomFolder(act)
    private val dev = DeviceStatus(act)
    private val linux = LinuxEnv.get(act, version).also { l -> l.emit = { n, d -> event(n, d) }; l.startActivity = { start(it) } }
    private val exec = Executors.newSingleThreadExecutor { r -> Thread(r, "hearth-bridge") }
    private val main = Handler(Looper.getMainLooper())

    @Volatile private var appList: List<AppEntry>? = null
    @Volatile private var romList: List<Rom>? = null
    private var pendingPick: Any? = null
    @Volatile var pageReady = false
        private set

    /** The game in front: started at [launchedAt]; [startedAt] once Hearth went to the background (0 before). */
    private data class Running(val id: String, val launchedAt: Long, var startedAt: Long)
    private var running: Running? = null
    private var lastLaunched: String? = null

    init {
        // A game that was playing when Android closed Hearth: its play time is counted when Hearth is back.
        store.data.optJSONObject("running")?.let { r ->
            val id = r.optString("id", "")
            if (id.isNotEmpty()) running = Running(id, r.optLong("launchedAt"), r.optLong("startedAt"))
        }
    }

    // ================================================================ messages
    fun onMessage(text: String) {
        val msg = try { JSONObject(text) } catch (e: Exception) { return }
        exec.execute {
            try { handle(msg) } catch (e: Throwable) { Logs.log("bridge: ${e.stackTraceToString().take(3000)}") }
        }
    }

    private fun handle(msg: JSONObject) {
        when (msg.optString("type")) {
            "call" -> {
                val id = msg.opt("id")
                val method = msg.optString("method")
                val args = msg.optJSONObject("args") ?: JSONObject()
                try {
                    val v = dispatch(method, args, id)
                    if (v !== DEFERRED) reply(id, true, v, null)
                } catch (e: Throwable) {
                    Logs.log("$method failed: $e")
                    reply(id, false, null, e.message ?: e.toString())
                }
            }
            "ready" -> { pageReady = true; Logs.log("page ready"); padsChanged() }
            "log" -> Logs.log("page: " + msg.optString("text").take(4000))
            "power" -> power(msg.optString("action"))
            "resume" -> resume()
            else -> {}
        }
    }

    private fun reply(id: Any?, ok: Boolean, value: Any?, error: String?) {
        val m = JSONObject().put("type", "result").put("id", id ?: JSONObject.NULL).put("ok", ok)
        if (ok) m.put("value", value ?: JSONObject.NULL) else m.put("error", error ?: "Failed")
        post(m)
    }
    fun event(name: String, data: Any) = post(JSONObject().put("type", "event").put("name", name).put("data", data))
    private fun toast(text: String) = post(JSONObject().put("type", "toast").put("text", text))
    private fun post(m: JSONObject) { val s = m.toString(); main.post { act.deliver(s) } }

    /** Runs [block] on the main thread and waits for it (starting activities); its exception comes back here. */
    private fun <T> onMain(block: () -> T): T {
        if (Looper.myLooper() == Looper.getMainLooper()) return block()
        var result: Result<T>? = null
        val done = CountDownLatch(1)
        main.post { result = runCatching(block); done.countDown() }
        if (!done.await(15, TimeUnit.SECONDS)) throw Exception("Android didn't answer in time")
        return result!!.getOrThrow()
    }
    private fun start(i: Intent) = try { onMain { act.startActivity(i) } } catch (e: ActivityNotFoundException) { throw Exception("No app on this device can open that.") }

    private fun req(a: JSONObject, k: String): String = a.optString(k, "").takeIf { it.isNotEmpty() } ?: throw Exception("Missing '$k'")

    // ================================================================ the calls
    private fun dispatch(method: String, a: JSONObject, id: Any?): Any = when (method) {
        "host.info" -> JSONObject()
            .put("platform", "android").put("name", dev.deviceName()).put("host", "Hearth (Android)").put("version", version)
            .put("edition", "android").put("features", JSONArray(FEATURES)).put("deviceId", deviceId())
            .put("setupDone", true).put("live", false).put("autoReport", false).put("isDefaultHome", isDefaultHome())
        "sys.status" -> dev.status()
        "audio.get" -> dev.audio()
        "audio.setVolume" -> { dev.setVolume(a.optDouble("v", 0.0).roundToInt()); JSONObject.NULL }
        "audio.setMute" -> { dev.setMute(a.optBoolean("m")); JSONObject.NULL }
        "display.get" -> dev.display()
        "power.get" -> dev.power()
        "system.info" -> dev.info()
        "pad.status" -> JSONObject().put("connected", dev.controllers().length()).put("mouseMode", false).put("mouseOn", false)
        // Apps can't switch Wi-Fi, Bluetooth or airplane mode on Android 10+: their settings open instead.
        "radios.set" -> { openSettings(if (a.optString("kind") == "bt") "bluetooth" else "wifi", ""); dev.status() }
        "airplane.set" -> { openSettings("airplane", ""); dev.status() }

        "profile.get" -> store.profile(dev.deviceName())
        "profile.set" -> store.profileSet(a, dev.deviceName())
        "profile.pickImage" -> throw Exception("Your own picture isn't in Hearth for Android yet: pick one of the pictures above.")

        "library.get" -> library(a.optBoolean("rescan"))
        "library.setFav" -> { store.setIn("favorites", req(a, "id"), a.optBoolean("on")); JSONObject.NULL }
        "library.setHidden" -> { store.setIn("hidden", req(a, "id"), a.optBoolean("on")); JSONObject.NULL }
        "library.remove" -> { store.setIn("manual", req(a, "id").removePrefix("android:"), false); JSONObject.NULL }
        "library.addManual" -> addApp(a.optString("id", ""))
        "library.collection.create" -> store.collectionCreate(a.optString("name", ""))
        "library.collection.rename" -> { store.collectionRename(req(a, "id"), a.optString("name", "")); JSONObject.NULL }
        "library.collection.remove" -> { store.collectionRemove(req(a, "id")); JSONObject.NULL }
        "library.collection.set" -> { store.collectionSet(req(a, "id"), a.optString("game", ""), a.optBoolean("on")); JSONObject.NULL }
        "library.downloads" -> JSONObject().put("active", JSONArray()).put("recent", JSONArray()).put("speeds", JSONArray())

        "game.launch" -> launch(req(a, "id"))
        "game.close" -> { running?.let { finish(it) }; JSONObject().put("ok", true) }
        "game.info" -> JSONObject()

        "logs.send" -> saveLogs()
        "logs.sendToDeveloper" -> sendLogs(a.optString("note", ""))
        "setup.done" -> JSONObject().put("ok", true)

        "android.status" -> androidStatus()
        "android.openSettings" -> { openSettings(req(a, "page"), a.optString("id", "")); JSONObject().put("ok", true) }
        "android.openUrl" -> { openUrl(req(a, "url")); JSONObject().put("ok", true) }
        "android.apps" -> JSONArray(otherApps().map { JSONObject().put("id", it.pkg).put("title", it.title) })
        "android.setShowAll" -> { store.update { it.put("showAll", a.optBoolean("on")) }; event("library.changed", JSONObject()); androidStatus() }
        "android.roms.pick" -> {
            pendingPick?.let { reply(it, true, null, null) }   // an older picker that never came back
            pendingPick = id
            main.post { act.pickFolder() }
            DEFERRED
        }
        // Hearth Linux (Beta Experimental): LinuxEnv answers at once; long jobs report with linux.* events.
        "linux.status", "linux.install", "linux.run", "linux.apt", "linux.stop", "linux.openX11", "linux.trustX11", "linux.reset", "linux.remove" -> linux.call(method, a)
        "android.roms.forget" -> { romsTree()?.let { releaseTree(it) }; store.update { it.remove("roms") }; romList = null; androidStatus() }
        else -> throw Exception("That isn't in Hearth for Android yet")
    }

    // ================================================================ library
    private fun appsNow(rescan: Boolean): List<AppEntry> = (if (rescan) null else appList) ?: apps.scan().also { appList = it }

    private fun romsNow(rescan: Boolean): List<Rom> {
        val tree = romsTree() ?: return emptyList()
        return (if (rescan) null else romList) ?: roms.scan(tree).also { romList = it; Logs.log("roms: ${it.size} games") }
    }

    private fun library(rescan: Boolean): JSONObject {
        val list = appsNow(rescan)
        val manual = store.list("manual").toSet()
        val showAll = store.data.optBoolean("showAll")
        val games = ArrayList<JSONObject>()
        var nAndroid = 0; var nOther = 0
        for (a in list) {
            val inLib = a.game || a.pkg in manual
            if (inLib) nAndroid++ else if (showAll) nOther++ else continue
            games.add(appDto(a, inLib, manual))
        }
        val rl = romsNow(rescan)
        rl.forEach { games.add(romDto(it)) }
        games.sortBy { it.optString("title").lowercase() }
        val cols = JSONArray(store.collections().map { c -> JSONObject().put("id", c.optString("id")).put("name", c.optString("name")).put("count", c.optJSONArray("games")?.length() ?: 0) })
        return JSONObject().put("games", JSONArray(games)).put("sources", JSONObject()).put("collections", cols)
            .put("counts", JSONObject().put("android", nAndroid).put("manual", nOther).put("emu", rl.size))
            .put("runningId", running?.takeIf { it.startedAt > 0 }?.id ?: JSONObject.NULL)
    }

    private fun common(id: String, title: String, source: String): JSONObject {
        val st = store.stats(id)
        return JSONObject().put("id", id).put("title", title).put("source", source)
            .put("fav", store.listHas("favorites", id)).put("hidden", store.listHas("hidden", id))
            .put("cols", JSONArray(store.collectionsOf(id)))
            .put("minutes", st.optLong("minutes", 0)).put("lastPlayed", st.optLong("lastPlayed", 0))
            .put("installed", true).put("state", "installed").put("cloud", false).put("canUninstall", false)
    }

    private fun appDto(a: AppEntry, inLib: Boolean, manual: Set<String>): JSONObject {
        val base = "https://${MainActivity.HOST}/art/${a.pkg}/${a.versionCode}/"
        return common("android:${a.pkg}", a.title, if (inLib) "android" else "manual")
            .put("added", a.added).put("path", a.pkg).put("canRemove", !a.game && a.pkg in manual)
            .put("art", JSONObject().put("cover", base + "cover.png").put("wide", base + "wide.png").put("hero", base + "wide.png"))
    }

    private fun romDto(r: Rom): JSONObject = common(r.id, r.title, "emu")
        .put("added", r.modified).put("sizeBytes", r.size).put("system", r.sys.name).put("canRemove", false)
        .put("path", Roms.docIdToPath(r.docId) ?: r.rel)
        .put("art", JSONObject().put("cover", Roms.thumbUrl(r.sys, r.name, "Named_Boxarts"))
            .put("wide", Roms.thumbUrl(r.sys, r.name, "Named_Snaps")).put("hero", Roms.thumbUrl(r.sys, r.name, "Named_Snaps")))

    private fun dtoFor(id: String): JSONObject {
        val manual = store.list("manual").toSet()
        appList?.firstOrNull { "android:${it.pkg}" == id }?.let { return appDto(it, it.game || it.pkg in manual, manual) }
        romList?.firstOrNull { it.id == id }?.let { return romDto(it) }
        return JSONObject().put("id", id)
    }

    /** Apps not in the library (for "Add an app"). */
    private fun otherApps(): List<AppEntry> { val manual = store.list("manual").toSet(); return appsNow(false).filter { !it.game && it.pkg !in manual } }

    private fun addApp(pkg: String): JSONObject {
        if (pkg.isEmpty()) throw Exception("Pick an app from the list")
        val a = appsNow(false).firstOrNull { it.pkg == pkg } ?: appsNow(true).firstOrNull { it.pkg == pkg } ?: throw Exception("That app isn't on this device anymore.")
        store.setIn("manual", pkg, true)
        return appDto(a, true, store.list("manual").toSet())
    }

    // ================================================================ playing
    private fun launch(id: String): JSONObject {
        val now = System.currentTimeMillis()
        when {
            id.startsWith("android:") -> {
                val pkg = id.removePrefix("android:")
                val i = apps.launchIntent(pkg) ?: throw Exception("That app can't be started from Hearth (it has no start screen), or it was uninstalled.")
                start(i)
            }
            id.startsWith("emu:") -> {
                val tree = romsTree() ?: throw Exception("Choose your ROMs folder again: Settings › Games › ROMs folder.")
                val rom = romsNow(false).firstOrNull { it.id == id } ?: throw Exception("That game isn't in your ROMs folder anymore.")
                val (i, retroArch) = roms.launchIntent(tree, rom, apps)
                Logs.log("roms: ${rom.name} with ${if (retroArch) "RetroArch (${rom.sys.cores[0]})" else "an app you pick"}")
                start(i)
            }
            else -> throw Exception("That game isn't in the library anymore.")
        }
        running = Running(id, now, 0)
        lastLaunched = id
        store.update { it.put("running", JSONObject().put("id", id).put("launchedAt", now).put("startedAt", 0)) }
        Logs.log("launch $id")
        return JSONObject().put("tracked", true)
    }

    /** Hearth went to the background: the game it just started is in front now. */
    fun onBackground() = exec.execute {
        val r = running ?: return@execute
        val now = System.currentTimeMillis()
        if (r.startedAt == 0L && now - r.launchedAt < 60_000) {
            r.startedAt = now
            store.update { it.put("running", JSONObject().put("id", r.id).put("launchedAt", r.launchedAt).put("startedAt", now)) }
            event("game.started", JSONObject().put("id", r.id))
        }
    }

    /** Hearth is in front again: the game is over (Android has no way to see it still running behind). */
    fun onForeground() = exec.execute {
        val r = running ?: return@execute
        if (r.startedAt > 0) finish(r)
        else if (System.currentTimeMillis() - r.launchedAt > 60_000) { running = null; store.update { it.remove("running") } }
        event("status", dev.status())
    }

    private fun finish(r: Running) {
        val now = System.currentTimeMillis()
        running = null
        if (r.startedAt > 0) {
            val minutes = Math.round((now - r.startedAt).coerceAtLeast(0) / 60000.0)
            store.addPlay(r.id, minutes, now)
            Logs.log("played ${r.id}: $minutes min")
        }
        store.update { it.remove("running") }
        event("game.exited", dtoFor(r.id))
    }

    /** "Back to game": the last game to the front again (an app's task, or RetroArch). */
    private fun resume() {
        val id = lastLaunched ?: return
        val pkg = when {
            id.startsWith("android:") -> id.removePrefix("android:")
            id.startsWith("emu:") -> roms.retroArch(apps)
            else -> null
        } ?: return
        apps.launchIntent(pkg)?.let { runCatching { start(it) } }
    }

    private fun power(action: String) {
        if (action != "desktop") return
        if (isDefaultHome()) toast("Hearth is your home app. To use another one: Settings › System › Hearth is your home app.")
        else main.post { act.moveTaskToBack(true) }
    }

    // ================================================================ Android's settings, the home app
    private fun openSettings(page: String, id: String) {
        val i = when (page) {
            "wifi" -> Intent(Settings.ACTION_WIFI_SETTINGS)
            "bluetooth" -> Intent(Settings.ACTION_BLUETOOTH_SETTINGS)
            "display" -> Intent(Settings.ACTION_DISPLAY_SETTINGS)
            "sound" -> Intent(Settings.ACTION_SOUND_SETTINGS)
            "battery" -> Intent(Intent.ACTION_POWER_USAGE_SUMMARY)
            "airplane" -> Intent(Settings.ACTION_AIRPLANE_MODE_SETTINGS)
            "home" -> Intent(Settings.ACTION_HOME_SETTINGS)
            "app" -> Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", id.removePrefix("android:").ifEmpty { act.packageName }, null))
            "settings" -> Intent(Settings.ACTION_SETTINGS)
            else -> throw Exception("Unknown settings page: $page")
        }.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val fallbacks = listOfNotNull(if (page == "home") Intent(Settings.ACTION_MANAGE_DEFAULT_APPS_SETTINGS) else null, Intent(Settings.ACTION_SETTINGS))
        for (x in listOf(i) + fallbacks) {
            try { onMain { act.startActivity(x.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }; return } catch (e: ActivityNotFoundException) { continue }
        }
        throw Exception("This device doesn't have that settings page.")
    }

    /** A web page in the browser (https only: the page links to things like Termux:X11's releases). */
    private fun openUrl(url: String) {
        val u = Uri.parse(url)
        if (u.scheme != "https" || u.host.isNullOrEmpty() || url.length > 2000) throw Exception("Only https links open from Hearth")
        start(Intent(Intent.ACTION_VIEW, u).addCategory(Intent.CATEGORY_BROWSABLE).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    @Suppress("DEPRECATION")
    fun isDefaultHome(): Boolean = try {
        val home = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME)
        val ri = if (Build.VERSION.SDK_INT >= 33) act.packageManager.resolveActivity(home, PackageManager.ResolveInfoFlags.of(PackageManager.MATCH_DEFAULT_ONLY.toLong()))
                 else act.packageManager.resolveActivity(home, PackageManager.MATCH_DEFAULT_ONLY)
        ri?.activityInfo?.packageName == act.packageName
    } catch (e: Exception) { false }

    // ================================================================ ROMs folder
    private fun romsTree(): Uri? {
        val s = store.data.optJSONObject("roms")?.optString("tree", "")?.takeIf { it.isNotEmpty() } ?: return null
        val uri = Uri.parse(s)
        return uri.takeIf { u -> act.contentResolver.persistedUriPermissions.any { it.uri == u && it.isReadPermission } }
    }
    private fun releaseTree(u: Uri) = runCatching { act.contentResolver.releasePersistableUriPermission(u, Intent.FLAG_GRANT_READ_URI_PERMISSION) }

    /** The folder picker came back (null: cancelled). */
    fun folderPicked(uri: Uri?) = exec.execute {
        val id = pendingPick ?: return@execute
        pendingPick = null
        if (uri == null) { reply(id, true, null, null); return@execute }
        try {
            act.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            romsTree()?.takeIf { it != uri }?.let { releaseTree(it) }
            store.update { it.put("roms", JSONObject().put("tree", uri.toString()).put("docId", DocumentsContract.getTreeDocumentId(uri))) }
            romList = null
            Logs.log("roms: folder ${DocumentsContract.getTreeDocumentId(uri)}")
            reply(id, true, androidStatus(), null)
        } catch (e: Exception) {
            Logs.log("roms: couldn't keep the folder: $e")
            reply(id, false, null, "Hearth couldn't keep access to that folder. Pick a folder on this device or its SD card.")
        }
    }

    private fun androidStatus(): JSONObject {
        val roms = JSONObject()
        val tree = romsTree()
        if (tree == null) roms.put("folder", JSONObject.NULL)
        else {
            val rl = romsNow(false)
            roms.put("folder", Roms.folderLabel(DocumentsContract.getTreeDocumentId(tree))).put("count", rl.size)
                .put("systems", JSONArray(Roms.SYSTEMS.map { s -> JSONObject().put("dir", s.key).put("name", s.name).put("count", rl.count { it.sys == s }) }))
        }
        val ra = this.roms.retroArch(apps)
        return JSONObject().put("showAll", store.data.optBoolean("showAll")).put("home", JSONObject().put("isDefault", isDefaultHome()))
            .put("roms", roms).put("retroarch", if (ra == null) JSONObject.NULL else JSONObject().put("pkg", ra))
            .put("pads", dev.controllers())
    }

    // ================================================================ controllers
    /** A controller button for the page (the same "pad" event the Linux host sends). */
    fun pad(button: String) = event("pad", JSONObject().put("b", button))

    /** Controllers came or went: Hearth reads them itself, so the page doesn't also read them (padHost). */
    fun padsChanged() = exec.execute { event("padHost", JSONObject().put("connected", dev.controllers().length())) }

    fun pushStatus() = exec.execute { if (pageReady) event("status", dev.status()) }

    // ================================================================ logs
    private fun deviceReport(): JSONObject {
        val i = dev.info()
        i.put("hearth", version).put("webview", runCatching { WebViewCompat.getCurrentWebViewPackage(act)?.versionName }.getOrNull() ?: "?")
        i.put("controllers", dev.controllers()).put("retroarch", roms.retroArch(apps) ?: "no").put("defaultHome", isDefaultHome())
        i.put("library", JSONObject().put("apps", appList?.size ?: -1).put("games", appList?.count { it.game } ?: -1).put("roms", romList?.size ?: -1))
        runCatching { i.put("linux", linux.status()) }
        return i
    }

    private fun saveLogs(): JSONObject {
        val text = Logs.report(version, deviceReport())
        val dir = act.getExternalFilesDir(null) ?: act.filesDir
        val f = File(dir, "hearth-logs-" + SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(Date()) + ".txt")
        f.writeText(text)
        dir.listFiles { x -> x.name.startsWith("hearth-logs-") }?.sortedByDescending { it.name }?.drop(5)?.forEach { it.delete() }
        return JSONObject().put("path", f.absolutePath).put("copies", JSONArray())
    }

    private fun sendLogs(note: String): JSONObject {
        val saved = saveLogs()
        val code = Logs.send(File(saved.getString("path")).readText(), version, note)
        Logs.log("logs sent: $code")
        return saved.put("code", code).put("url", "${Logs.SERVICE}/v1/logs/$code")
    }

    /** The player counter's id for this device: the same after updates, nothing personal in it. */
    private fun deviceId(): String {
        val aid = Settings.Secure.getString(act.contentResolver, Settings.Secure.ANDROID_ID) ?: "unknown"
        return MessageDigest.getInstance("MD5").digest("hearth-android:$aid".toByteArray()).joinToString("") { "%02x".format(it) }
    }

    fun shutdown() { exec.shutdown() }

    companion object {
        private val DEFERRED = Any()
        /** What the page may show (HOSTINFO.features). */
        val FEATURES = listOf("android", "sendLogs", "linux")
    }
}
