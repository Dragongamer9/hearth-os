package dev.hearthos.android

import android.app.ActivityManager
import android.bluetooth.BluetoothManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiManager
import android.os.BatteryManager
import android.os.Build
import android.os.Environment
import android.os.StatFs
import android.os.SystemClock
import android.provider.Settings
import android.view.InputDevice
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.roundToInt

/**
 * What the top bar, Quick Access and Settings show about the device, in the same shapes as the other hosts'
 * sys.status / audio.get / power.get / display.get / system.info. Nothing here needs a runtime permission.
 */
class DeviceStatus(private val ctx: Context) {
    private val audio get() = ctx.getSystemService(Context.AUDIO_SERVICE) as AudioManager

    fun battery(): JSONObject {
        val b = ctx.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        val present = b?.getBooleanExtra(BatteryManager.EXTRA_PRESENT, true) ?: false
        val level = b?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
        val scale = b?.getIntExtra(BatteryManager.EXTRA_SCALE, 100) ?: 100
        val status = b?.getIntExtra(BatteryManager.EXTRA_STATUS, -1) ?: -1
        val o = JSONObject().put("present", present && level >= 0)
        if (level >= 0 && scale > 0) o.put("pct", (level * 100f / scale).roundToInt())
        o.put("charging", status == BatteryManager.BATTERY_STATUS_CHARGING || status == BatteryManager.BATTERY_STATUS_FULL)
        return o
    }

    @Suppress("DEPRECATION")
    fun wifi(): JSONObject {
        val wm = ctx.applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager
        val cm = ctx.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val caps = runCatching { cm.getNetworkCapabilities(cm.activeNetwork) }.getOrNull()
        val onWifi = caps?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        val o = JSONObject().put("available", wm != null).put("on", wm?.isWifiEnabled == true)
        if (onWifi && wm != null) {
            val info = runCatching { wm.connectionInfo }.getOrNull()
            // The network's name needs location access on Android 8.1+; without it Android says "<unknown ssid>".
            val ssid = info?.ssid?.trim('"')?.takeIf { it.isNotEmpty() && it != "<unknown ssid>" } ?: "Wi-Fi"
            val rssi = info?.rssi ?: -100
            val bars = if (Build.VERSION.SDK_INT >= 30) { val max = wm.maxSignalLevel.coerceAtLeast(1); (wm.calculateSignalLevel(rssi) * 4f / max).roundToInt() }
                       else WifiManager.calculateSignalLevel(rssi, 5)
            o.put("ssid", ssid).put("bars", bars.coerceIn(0, 4))
        } else o.put("ssid", JSONObject.NULL).put("bars", 0)
        o.put("online", caps?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true)
        return o
    }

    fun bluetooth(): JSONObject {
        val ad = (ctx.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
        val on = try { ad?.isEnabled == true } catch (e: SecurityException) { false }
        return JSONObject().put("available", ad != null).put("on", on)
    }

    fun airplane(): Boolean = Settings.Global.getInt(ctx.contentResolver, Settings.Global.AIRPLANE_MODE_ON, 0) != 0

    // ---------------------------------------------------------------- sound (media volume, 0-100 like the others)
    fun audio(): JSONObject {
        val a = audio
        val max = a.getStreamMaxVolume(AudioManager.STREAM_MUSIC).coerceAtLeast(1)
        val v = (a.getStreamVolume(AudioManager.STREAM_MUSIC) * 100f / max).roundToInt()
        return JSONObject().put("volume", v).put("muted", a.isStreamMute(AudioManager.STREAM_MUSIC)).put("max", 100).put("devices", JSONArray())
    }
    fun setVolume(pct: Int) {
        val a = audio
        val max = a.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
        a.setStreamVolume(AudioManager.STREAM_MUSIC, (pct.coerceIn(0, 100) * max / 100f).roundToInt(), 0)
    }
    fun setMute(on: Boolean) = audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, if (on) AudioManager.ADJUST_MUTE else AudioManager.ADJUST_UNMUTE, 0)

    @Suppress("DEPRECATION")
    fun display(): JSONObject {
        val m = ctx.resources.displayMetrics
        val wm = ctx.getSystemService(Context.WINDOW_SERVICE) as android.view.WindowManager
        val hz = runCatching { wm.defaultDisplay.refreshRate.roundToInt() }.getOrDefault(0)
        val w = maxOf(m.widthPixels, m.heightPixels); val h = minOf(m.widthPixels, m.heightPixels)
        // Brightness belongs to Android (changing it needs a special permission): no slider in Hearth.
        return JSONObject().put("supported", false).put("brightness", 0).put("width", w).put("height", h)
            .put("refreshRate", hz).put("resolution", "$w × $h")
    }

    fun status(): JSONObject = JSONObject().put("wifi", wifi()).put("bt", bluetooth()).put("airplane", airplane())
        .put("battery", battery()).put("audio", audio()).put("display", JSONObject().put("supported", false))

    fun power(): JSONObject = battery().put("mode", JSONObject.NULL).put("modes", JSONArray()).put("modeSupported", false)

    fun deviceName(): String {
        val n = runCatching { Settings.Global.getString(ctx.contentResolver, Settings.Global.DEVICE_NAME) }.getOrNull()
        return n?.takeIf { it.isNotBlank() } ?: model()
    }
    fun model(): String {
        val man = Build.MANUFACTURER.replaceFirstChar { it.uppercase() }
        return if (Build.MODEL.startsWith(Build.MANUFACTURER, ignoreCase = true)) Build.MODEL else "$man ${Build.MODEL}"
    }

    fun info(): JSONObject {
        val mem = ActivityManager.MemoryInfo().also { (ctx.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(it) }
        val st = StatFs(Environment.getDataDirectory().path)
        val cpu = if (Build.VERSION.SDK_INT >= 31 && Build.SOC_MODEL != Build.UNKNOWN) "${Build.SOC_MANUFACTURER} ${Build.SOC_MODEL}" else Build.HARDWARE
        return JSONObject()
            .put("device", model()).put("hostname", deviceName())
            .put("os", "Android").put("osVersion", Build.VERSION.RELEASE)
            .put("osPretty", "Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})")
            .put("cpu", cpu).put("cores", Runtime.getRuntime().availableProcessors())
            .put("memoryBytes", mem.totalMem)
            .put("storage", JSONObject().put("totalBytes", st.totalBytes).put("freeBytes", st.availableBytes))
            .put("uptimeSec", SystemClock.elapsedRealtime() / 1000)
    }

    /** Controllers plugged in or paired: real devices that are gamepads or joysticks. */
    fun controllers(): JSONArray {
        val out = JSONArray()
        for (id in InputDevice.getDeviceIds()) {
            val d = InputDevice.getDevice(id) ?: continue
            val s = d.sources
            val pad = (s and InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD || (s and InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
            if (pad && !d.isVirtual) out.put(JSONObject().put("name", d.name).put("id", id))
        }
        return out
    }
}
