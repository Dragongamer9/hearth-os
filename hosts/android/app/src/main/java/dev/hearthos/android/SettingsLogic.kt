package dev.hearthos.android

import kotlin.math.roundToInt

/**
 * The pure parts of Hearth's Android settings (AndroidSettings.kt puts them to work): which settings Hearth may
 * change and with what values, which one-time permission each needs, Android's numbers to Hearth's and back, which
 * Android page or panel each "open" row goes to, Wi-Fi and Bluetooth details. No Android classes here, so it's
 * unit-tested on a plain JVM (SettingsLogicTest). Android's constants are copied as plain numbers and strings.
 *
 * Nothing here ever changes a setting by itself: AndroidSettings only calls [parse] for a change the person made.
 */
object SettingsLogic {
    // ---------------------------------------------------------------- what can be changed, and what it needs
    /** The one-time permissions Hearth may ask for, and why (the page shows the text before asking). */
    val PERMS = linkedMapOf(
        "writeSettings" to "Android asks once for \"Modify system settings\" so Hearth can change brightness, screen timeout and auto-rotate for you.",
        "dnd" to "Android asks once for \"Do Not Disturb access\" so Hearth can turn Do Not Disturb on and off.",
        "location" to "Android only tells apps the Wi-Fi network's name, and which networks are nearby, with location permission. Hearth uses it for nothing else.",
        "btConnect" to "Android asks once for \"Nearby devices\" so Hearth can show your paired Bluetooth devices and pair new ones.",
        "batteryOpt" to "Lets Hearth keep running in the background, so Android (Samsung's battery saving especially) doesn't close it or Hearth Linux.",
        "notifications" to "Lets Hearth show a notification while Hearth Linux runs in the background.",
    )

    val STREAMS = listOf("media", "ring", "alarm", "notification")
    /** AudioManager.STREAM_*: music 3, ring 2, alarm 4, notification 5. */
    fun streamId(name: String): Int = when (name) { "media" -> 3; "ring" -> 2; "alarm" -> 4; "notification" -> 5; else -> throw IllegalArgumentException("Unknown volume: $name") }

    /** Screen timeouts Hearth offers, in seconds (Android's own list). */
    val TIMEOUTS = listOf(15, 30, 60, 120, 300, 600, 1800)
    /** Do Not Disturb: NotificationManager.INTERRUPTION_FILTER_* (all 1, priority 2, none 3, alarms 4). */
    val DND = linkedMapOf("all" to 1, "priority" to 2, "none" to 3, "alarms" to 4)
    fun dndName(filter: Int): String = DND.entries.firstOrNull { it.value == filter }?.key ?: "all"

    /** A change the page asked for, checked: [key], the value to write (Android's units) and the permission it needs. */
    data class Change(val key: String, val value: Any, val needs: String?)

    /** Checks a change from the page. Volumes come as Android's steps (0…max of that stream: [maxOf]); brightness as
     *  0–100 %; timeout as seconds; switches as true/false. Anything else is refused with a plain message. */
    fun parse(key: String, raw: Any?, maxOf: (String) -> Int = { 15 }): Change {
        fun bool(): Boolean = when (raw) { is Boolean -> raw; "true", "1", 1 -> true; "false", "0", 0 -> false; else -> throw IllegalArgumentException("$key needs on or off") }
        fun num(): Double = (raw as? Number)?.toDouble() ?: (raw as? String)?.toDoubleOrNull() ?: throw IllegalArgumentException("$key needs a number")
        return when {
            key.startsWith("vol.") -> {
                val s = key.removePrefix("vol.")
                if (s !in STREAMS) throw IllegalArgumentException("Unknown volume: $s")
                Change(key, num().roundToInt().coerceIn(0, maxOf(s).coerceAtLeast(0)), null)
            }
            key == "mute" -> Change(key, bool(), null)
            key == "brightness" -> Change(key, percentToBrightness(num()), "writeSettings")
            key == "autoBrightness" -> Change(key, bool(), "writeSettings")
            key == "autoRotate" -> Change(key, bool(), "writeSettings")
            key == "timeout" -> Change(key, nearestTimeout(num().roundToInt()) * 1000, "writeSettings")
            key == "dnd" -> Change(key, (raw as? String)?.let { DND[it] } ?: throw IllegalArgumentException("Do Not Disturb is all, priority, alarms or none"), "dnd")
            else -> throw IllegalArgumentException("Hearth can't change $key")
        }
    }

    // ---------------------------------------------------------------- numbers
    /** Settings.System.SCREEN_BRIGHTNESS is 0–255; Hearth shows 0–100 %. Never fully dark (1 at the least). */
    fun percentToBrightness(pct: Double): Int = (pct.coerceIn(0.0, 100.0) * 255 / 100).roundToInt().coerceIn(1, 255)
    fun brightnessToPercent(v: Int): Int = (v.coerceIn(0, 255) * 100 / 255.0).roundToInt()

    fun nearestTimeout(sec: Int): Int = TIMEOUTS.minByOrNull { Math.abs(it - sec) }!!

    fun timeoutLabel(sec: Int): String = when {
        sec <= 0 -> "Never"
        sec < 60 -> "$sec seconds"
        sec < 3600 -> (sec / 60).let { if (it == 1) "1 minute" else "$it minutes" }
        else -> (sec / 3600).let { if (it == 1) "1 hour" else "$it hours" }
    }

    // ---------------------------------------------------------------- Wi-Fi
    /** Android's SSID as people read it: no quotes; null when Android hides it ("<unknown ssid>" without location). */
    fun cleanSsid(s: String?): String? = s?.trim()?.removeSurrounding("\"")?.takeIf { it.isNotEmpty() && it != "<unknown ssid>" && it != "0x" }

    /** Signal bars 0–4 from the RSSI (dBm), like Android's own five levels. */
    fun bars(rssi: Int): Int = when { rssi >= -55 -> 4; rssi >= -66 -> 3; rssi >= -77 -> 2; rssi >= -88 -> 1; else -> 0 }

    /** "open", "wpa" (WPA/WPA2 personal), "wpa3" (SAE) or "enterprise" (Hearth can't join those itself) from a scan's capabilities. */
    fun security(caps: String): String = when {
        caps.contains("EAP") -> "enterprise"
        caps.contains("SAE") && !caps.contains("PSK") -> "wpa3"
        caps.contains("PSK") || caps.contains("WPA") || caps.contains("SAE") -> "wpa"
        caps.contains("WEP") -> "wep"
        else -> "open"
    }

    /** A network to join from Hearth: a name of 1–32 bytes, and a WPA password of 8–63 characters (or none when open). */
    fun checkJoin(ssid: String, password: String, security: String): String? = when {
        ssid.isEmpty() || ssid.toByteArray().size > 32 -> "A network's name is 1 to 32 letters"
        security == "enterprise" -> "Hearth can't join work or school networks itself: they open in Android's Wi-Fi settings"
        security == "wep" -> "That network uses old WEP security, which Android no longer joins"
        security == "open" -> null
        password.length !in 8..63 -> "A Wi-Fi password is 8 to 63 characters"
        else -> null
    }

    // ---------------------------------------------------------------- Bluetooth
    /** What a Bluetooth device is, from its major device class (BluetoothClass.Device.Major: peripheral 0x0500, audio
     *  0x0400) and its minor bits (gamepad/joystick 0x08/0x04 in peripheral). */
    fun btKind(deviceClass: Int, name: String?): String {
        val major = deviceClass and 0x1F00
        val n = (name ?: "").lowercase()
        return when {
            major == 0x0500 && (deviceClass and 0x0C) != 0 -> "controller"
            Regex("controller|gamepad|xbox|dualsense|dualshock|wireless controller|8bitdo|pro controller|joy-con").containsMatchIn(n) -> "controller"
            major == 0x0400 -> "audio"
            major == 0x0500 -> "input"
            else -> "other"
        }
    }

    // ---------------------------------------------------------------- Android's pages and panels
    /** Hearth's name for an Android settings page → Android's intent action (fallbacks after it). "app"-like pages
     *  take a package. A page Android doesn't have on this device falls back to the next, then to Settings. */
    fun pageActions(page: String, sdk: Int): List<String> = when (page) {
        "wifi" -> listOf("android.settings.WIFI_SETTINGS")
        "bluetooth" -> listOf("android.settings.BLUETOOTH_SETTINGS")
        "display" -> listOf("android.settings.DISPLAY_SETTINGS")
        "darkmode" -> listOf("android.settings.DARK_THEME_SETTINGS", "android.settings.DISPLAY_SETTINGS")
        "textsize" -> listOf("android.settings.TEXT_READING_SETTINGS", "android.settings.DISPLAY_SETTINGS")
        "sound" -> listOf("android.settings.SOUND_SETTINGS")
        "battery" -> listOf("android.intent.action.POWER_USAGE_SUMMARY")
        "saver" -> listOf("android.settings.BATTERY_SAVER_SETTINGS", "android.intent.action.POWER_USAGE_SUMMARY")
        "batteryOpt" -> listOf("android.settings.IGNORE_BATTERY_OPTIMIZATION_SETTINGS", "android.intent.action.POWER_USAGE_SUMMARY")
        "airplane" -> listOf("android.settings.AIRPLANE_MODE_SETTINGS")
        "home" -> listOf("android.settings.HOME_SETTINGS", "android.settings.MANAGE_DEFAULT_APPS_SETTINGS")
        "app" -> listOf("android.settings.APPLICATION_DETAILS_SETTINGS")
        "notifications" -> if (sdk >= 26) listOf("android.settings.APP_NOTIFICATION_SETTINGS", "android.settings.APPLICATION_DETAILS_SETTINGS") else listOf("android.settings.APPLICATION_DETAILS_SETTINGS")
        "allNotifications" -> listOf("android.settings.NOTIFICATION_SETTINGS", "android.settings.SOUND_SETTINGS")
        "dndSettings" -> listOf("android.settings.ZEN_MODE_SETTINGS", "android.settings.SOUND_SETTINGS")
        "storage" -> listOf("android.settings.INTERNAL_STORAGE_SETTINGS", "android.settings.MEMORY_CARD_SETTINGS")
        "date" -> listOf("android.settings.DATE_SETTINGS")
        "language" -> listOf("android.settings.LOCALE_SETTINGS")
        "accessibility" -> listOf("android.settings.ACCESSIBILITY_SETTINGS")
        "developer" -> listOf("android.settings.APPLICATION_DEVELOPMENT_SETTINGS", "android.settings.DEVICE_INFO_SETTINGS")
        "about" -> listOf("android.settings.DEVICE_INFO_SETTINGS")
        "nfc" -> listOf("android.settings.NFC_SETTINGS", "android.settings.WIRELESS_SETTINGS")
        "data" -> listOf("android.settings.DATA_USAGE_SETTINGS", "android.settings.WIRELESS_SETTINGS")
        "writeSettings" -> listOf("android.settings.action.MANAGE_WRITE_SETTINGS")
        "dndAccess" -> listOf("android.settings.NOTIFICATION_POLICY_ACCESS_SETTINGS")
        "settings" -> listOf("android.settings.SETTINGS")
        else -> throw IllegalArgumentException("Unknown settings page: $page")
    }.let { if (it.last() == "android.settings.SETTINGS") it else it + "android.settings.SETTINGS" }

    /** Pages that are about one app (they get package:<pkg>). */
    fun pageTakesPackage(page: String): Boolean = page in setOf("app", "writeSettings")

    /** Settings panels (Android 10+): they slide up over Hearth instead of leaving it. Older Android opens the page. */
    fun panelAction(name: String, sdk: Int): String? = if (sdk < 29) null else when (name) {
        "internet" -> "android.settings.panel.action.INTERNET_CONNECTIVITY"
        "wifi" -> "android.settings.panel.action.WIFI"
        "nfc" -> "android.settings.panel.action.NFC"
        "volume" -> "android.settings.panel.action.VOLUME"
        else -> throw IllegalArgumentException("Unknown panel: $name")
    }
    /** The page a panel falls back to on Android 9 and older. */
    fun panelFallback(name: String): String = when (name) { "internet" -> "wifi"; "wifi" -> "wifi"; "nfc" -> "nfc"; "volume" -> "sound"; else -> "settings" }

    /** Is this a Samsung device (One UI)? From Build.MANUFACTURER. */
    fun isSamsung(manufacturer: String?): Boolean = manufacturer.equals("samsung", ignoreCase = true)
}
