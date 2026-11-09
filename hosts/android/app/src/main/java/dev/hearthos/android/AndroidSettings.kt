package dev.hearthos.android

import android.Manifest
import android.annotation.SuppressLint
import android.app.NotificationManager
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioManager
import android.net.Uri
import android.net.wifi.WifiManager
import android.net.wifi.WifiNetworkSuggestion
import android.nfc.NfcAdapter
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
import org.json.JSONArray
import org.json.JSONObject

/**
 * Hearth's Settings for the device itself, inside Hearth wherever Android lets an app do it (no root, no device-owner
 * tricks). [SettingsLogic] holds the rules; this reads and writes Android.
 *
 * Inline: the four volumes and mute (AudioManager, no permission); brightness, auto-brightness, screen timeout and
 * auto-rotate (Settings.System, after the one-time "Modify system settings"); Do Not Disturb (after the one-time
 * "Do Not Disturb access"); Wi-Fi's network, signal and nearby networks (location permission for the name), joining a
 * network (Android's own "Save network?" dialog); Bluetooth's paired devices ("Nearby devices"), pairing (Android's
 * companion chooser over Hearth), turning Bluetooth on (Android's dialog); battery level, charging, battery saver's
 * state; whether Hearth may run in the background (Android's dialog).
 *
 * Android 10+ doesn't let apps switch Wi-Fi, mobile data, NFC, airplane mode or battery saver: those open Android's
 * panel (over Hearth) or its exact page. Nothing here changes a setting unless the person changed it in Hearth.
 */
class AndroidSettings(private val ctx: Context) {
    private val audio get() = ctx.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private val nm get() = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    private val pm get() = ctx.getSystemService(Context.POWER_SERVICE) as PowerManager
    private val wifiManager get() = ctx.applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager
    private val btAdapter: BluetoothAdapter? get() = (ctx.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
    val samsung = SettingsLogic.isSamsung(Build.MANUFACTURER)

    // ---------------------------------------------------------------- permissions
    private fun granted(p: String) = ctx.checkSelfPermission(p) == PackageManager.PERMISSION_GRANTED

    fun has(perm: String): Boolean = when (perm) {
        "writeSettings" -> Settings.System.canWrite(ctx)
        "dnd" -> nm.isNotificationPolicyAccessGranted
        "location" -> granted(Manifest.permission.ACCESS_FINE_LOCATION)
        "btConnect" -> Build.VERSION.SDK_INT < 31 || granted(Manifest.permission.BLUETOOTH_CONNECT)
        "batteryOpt" -> pm.isIgnoringBatteryOptimizations(ctx.packageName)
        "notifications" -> Build.VERSION.SDK_INT < 33 || granted(Manifest.permission.POST_NOTIFICATIONS)
        else -> false
    }

    /** The runtime permissions behind [perm] (asked with Android's own dialog), or null for the special ones. */
    fun runtimePerms(perm: String): Array<String>? = when (perm) {
        "location" -> arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)
        "btConnect" -> if (Build.VERSION.SDK_INT >= 31) arrayOf(Manifest.permission.BLUETOOTH_CONNECT) else null
        "notifications" -> if (Build.VERSION.SDK_INT >= 33) arrayOf(Manifest.permission.POST_NOTIFICATIONS) else null
        else -> null
    }

    /** The Android page (or dialog) where the person gives a special permission. */
    @SuppressLint("BatteryLife")
    fun permIntent(perm: String): Intent? = when (perm) {
        "writeSettings" -> Intent(Settings.ACTION_MANAGE_WRITE_SETTINGS, Uri.parse("package:" + ctx.packageName))
        "dnd" -> Intent(Settings.ACTION_NOTIFICATION_POLICY_ACCESS_SETTINGS)
        // A small dialog over Hearth ("Let app always run in background?"); Hearth is a home app and runs Linux
        // programs the person started, which is what the exemption is for.
        "batteryOpt" -> Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:" + ctx.packageName))
        else -> null
    }

    fun perms(): JSONObject = JSONObject().also { o -> SettingsLogic.PERMS.keys.forEach { o.put(it, has(it)) } }

    // ---------------------------------------------------------------- reading
    fun state(dev: DeviceStatus): JSONObject {
        val a = audio
        val streams = JSONObject()
        for (s in SettingsLogic.STREAMS) {
            val id = SettingsLogic.streamId(s)
            streams.put(s, JSONObject().put("v", a.getStreamVolume(id)).put("max", a.getStreamMaxVolume(id)).put("muted", a.isStreamMute(id)))
        }
        val ringer = when (a.ringerMode) { AudioManager.RINGER_MODE_SILENT -> "silent"; AudioManager.RINGER_MODE_VIBRATE -> "vibrate"; else -> "normal" }
        val cr = ctx.contentResolver
        val bright = Settings.System.getInt(cr, Settings.System.SCREEN_BRIGHTNESS, 128)
        val display = JSONObject()
            .put("canWrite", has("writeSettings"))
            .put("brightness", SettingsLogic.brightnessToPercent(bright))
            .put("auto", Settings.System.getInt(cr, Settings.System.SCREEN_BRIGHTNESS_MODE, 0) == Settings.System.SCREEN_BRIGHTNESS_MODE_AUTOMATIC)
            .put("timeoutSec", Settings.System.getInt(cr, Settings.System.SCREEN_OFF_TIMEOUT, 30000) / 1000)
            .put("timeouts", JSONArray(SettingsLogic.TIMEOUTS))
            .put("rotate", Settings.System.getInt(cr, Settings.System.ACCELEROMETER_ROTATION, 0) == 1)
            .put("dark", (ctx.resources.configuration.uiMode and android.content.res.Configuration.UI_MODE_NIGHT_MASK) == android.content.res.Configuration.UI_MODE_NIGHT_YES)
        val dnd = JSONObject().put("access", has("dnd")).put("filter", SettingsLogic.dndName(nm.currentInterruptionFilter))
        val w = dev.wifi()
        w.put("canSeeName", has("location"))
        val bt = bluetooth()
        val battery = dev.battery().put("saver", pm.isPowerSaveMode).put("background", has("batteryOpt"))
        val nfc = NfcAdapter.getDefaultAdapter(ctx).let { n -> JSONObject().put("supported", n != null).put("on", runCatching { n?.isEnabled == true }.getOrDefault(false)) }
        return JSONObject()
            .put("sdk", Build.VERSION.SDK_INT).put("samsung", samsung).put("panels", Build.VERSION.SDK_INT >= 29)
            .put("audio", JSONObject().put("streams", streams).put("ringer", ringer))
            .put("display", display).put("dnd", dnd).put("wifi", w).put("bt", bt).put("battery", battery).put("nfc", nfc)
            .put("airplane", dev.airplane()).put("perms", perms())
    }

    @SuppressLint("MissingPermission")
    fun bluetooth(): JSONObject {
        val ad = btAdapter ?: return JSONObject().put("supported", false).put("on", false).put("paired", JSONArray()).put("canList", false)
        val canList = has("btConnect")
        val on = runCatching { ad.isEnabled }.getOrDefault(false)
        val paired = JSONArray()
        if (canList && on) runCatching {
            for (d in ad.bondedDevices.orEmpty().sortedBy { (it.name ?: "").lowercase() }) {
                val cls = runCatching { d.bluetoothClass?.deviceClass ?: 0 }.getOrDefault(0)
                paired.put(JSONObject().put("name", d.name ?: d.address).put("address", d.address).put("kind", SettingsLogic.btKind(cls, d.name)))
            }
        }
        return JSONObject().put("supported", true).put("on", on).put("canList", canList).put("paired", paired)
    }

    // ---------------------------------------------------------------- changing (only what the person changed)
    /** Changes one setting. Returns {ok:true} or {ok:false, need:<perm>, why} when Android needs a permission first. */
    fun set(key: String, raw: Any?): JSONObject {
        val c = SettingsLogic.parse(key, raw) { s -> audio.getStreamMaxVolume(SettingsLogic.streamId(s)) }
        if (c.needs != null && !has(c.needs)) return JSONObject().put("ok", false).put("need", c.needs).put("why", SettingsLogic.PERMS[c.needs])
        val cr = ctx.contentResolver
        try {
            when {
                key.startsWith("vol.") -> audio.setStreamVolume(SettingsLogic.streamId(key.removePrefix("vol.")), c.value as Int, 0)
                key == "mute" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, if (c.value as Boolean) AudioManager.ADJUST_MUTE else AudioManager.ADJUST_UNMUTE, 0)
                key == "brightness" -> Settings.System.putInt(cr, Settings.System.SCREEN_BRIGHTNESS, c.value as Int)
                key == "autoBrightness" -> Settings.System.putInt(cr, Settings.System.SCREEN_BRIGHTNESS_MODE,
                    if (c.value as Boolean) Settings.System.SCREEN_BRIGHTNESS_MODE_AUTOMATIC else Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL)
                key == "timeout" -> Settings.System.putInt(cr, Settings.System.SCREEN_OFF_TIMEOUT, c.value as Int)
                key == "autoRotate" -> Settings.System.putInt(cr, Settings.System.ACCELEROMETER_ROTATION, if (c.value as Boolean) 1 else 0)
                key == "dnd" -> nm.setInterruptionFilter(c.value as Int)
            }
        } catch (e: SecurityException) {
            // Do Not Disturb blocks ring and notification volumes without DND access, for example.
            val need = if (key.startsWith("vol.") || key == "dnd") "dnd" else c.needs
            return JSONObject().put("ok", false).put("need", need ?: JSONObject.NULL).put("why", need?.let { SettingsLogic.PERMS[it] } ?: "Android didn't allow that change.")
        }
        Logs.log("settings: $key changed in Hearth")
        return JSONObject().put("ok", true)
    }

    // ---------------------------------------------------------------- panels and pages
    fun panelIntent(name: String): Intent? = SettingsLogic.panelAction(name, Build.VERSION.SDK_INT)?.let { Intent(it) }

    /** The intents for one of Hearth's names for an Android page, best first. [pkg]: the app, for app pages. */
    fun pageIntents(page: String, pkg: String): List<Intent> {
        val forPkg = pkg.ifEmpty { ctx.packageName }
        return SettingsLogic.pageActions(page, Build.VERSION.SDK_INT).map { action ->
            val i = Intent(action)
            when (action) {
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Settings.ACTION_MANAGE_WRITE_SETTINGS -> i.data = Uri.fromParts("package", forPkg, null)
                "android.settings.APP_NOTIFICATION_SETTINGS" -> i.putExtra("android.provider.extra.APP_PACKAGE", forPkg)
            }
            i
        }
    }

    @Suppress("DEPRECATION")
    fun uninstallIntent(pkg: String): Intent {
        require(pkg.isNotEmpty() && pkg != ctx.packageName) { "Hearth can't uninstall that" }
        return Intent(Intent.ACTION_UNINSTALL_PACKAGE, Uri.fromParts("package", pkg, null)).putExtra(Intent.EXTRA_RETURN_RESULT, false)
    }

    // ---------------------------------------------------------------- Wi-Fi
    @SuppressLint("MissingPermission")
    @Suppress("DEPRECATION")
    fun wifiScan(): JSONObject {
        val wm = wifiManager ?: return JSONObject().put("networks", JSONArray()).put("supported", false)
        if (!has("location")) return JSONObject().put("networks", JSONArray()).put("need", "location").put("why", SettingsLogic.PERMS["location"])
        runCatching { wm.startScan() }    // throttled by Android (4 a minute); the last results come back regardless
        val best = LinkedHashMap<String, JSONObject>()
        for (r in runCatching { wm.scanResults }.getOrDefault(emptyList()).sortedByDescending { it.level }) {
            val ssid = SettingsLogic.cleanSsid(if (Build.VERSION.SDK_INT >= 33) r.wifiSsid?.toString() else r.SSID) ?: continue
            if (ssid in best) continue
            best[ssid] = JSONObject().put("ssid", ssid).put("bars", SettingsLogic.bars(r.level)).put("security", SettingsLogic.security(r.capabilities ?: ""))
        }
        return JSONObject().put("networks", JSONArray(best.values.take(30))).put("on", wm.isWifiEnabled)
    }

    /** Joining: Android 11+ shows its own "Save this network?" dialog over Hearth (ACTION_WIFI_ADD_NETWORKS); older
     *  Android gets it as a suggestion (Android connects when it's in range and tells the person). */
    fun wifiJoinIntent(ssid: String, password: String, security: String): Intent? {
        SettingsLogic.checkJoin(ssid, password, security)?.let { throw IllegalArgumentException(it) }
        if (Build.VERSION.SDK_INT < 29) return null
        val b = WifiNetworkSuggestion.Builder().setSsid(ssid)
        when (security) { "wpa" -> b.setWpa2Passphrase(password); "wpa3" -> b.setWpa3Passphrase(password) }
        val s = b.build()
        if (Build.VERSION.SDK_INT >= 30) {
            return Intent(Settings.ACTION_WIFI_ADD_NETWORKS).putExtras(Bundle().apply { putParcelableArrayList(Settings.EXTRA_WIFI_NETWORK_LIST, arrayListOf(s)) })
        }
        val r = wifiManager?.addNetworkSuggestions(listOf(s)) ?: -1
        if (r != WifiManager.STATUS_NETWORK_SUGGESTIONS_SUCCESS && r != WifiManager.STATUS_NETWORK_SUGGESTIONS_ERROR_ADD_DUPLICATE)
            throw IllegalStateException("Android didn't take the network (error $r)")
        return null
    }

    // ---------------------------------------------------------------- Bluetooth
    fun btEnableIntent(): Intent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)

    @SuppressLint("MissingPermission")
    fun bond(d: BluetoothDevice): Boolean = runCatching { d.bondState == BluetoothDevice.BOND_BONDED || d.createBond() }.getOrDefault(false)
}
