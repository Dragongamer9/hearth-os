package dev.hearthos.tv

import android.accessibilityservice.AccessibilityService
import android.content.Intent
import android.view.KeyEvent
import android.view.accessibility.AccessibilityEvent

/**
 * The TV's Home button opens Hearth TV (docs/HEARTH-TV.md). Google TV sends Home to its own home (or, with that switched
 * off, to a blank Google screen) whatever app is picked, so Hearth TV catches the button itself, the way other Google TV
 * home screens do. Switched on once (the install script, or Settings › Accessibility › Hearth TV); switched off there too.
 * Google TV's home stays installed: when it comes up anyway (after an update, say), Hearth TV comes back in front of it.
 */
class HomeButtonService : AccessibilityService() {
    private val googleHomes = setOf("com.google.android.apps.tv.launcherx", "com.google.android.tvlauncher", "com.google.android.tungsten.setupwraith")

    private fun openHearth() {
        try {
            startActivity(Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)
                .putExtra("home", true))
        } catch (e: Exception) { Logs.add("home button: ${e.message}") }
    }

    override fun onKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode != KeyEvent.KEYCODE_HOME) return false
        if (event.action == KeyEvent.ACTION_UP) openHearth()
        return true
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED && event.packageName?.toString() in googleHomes) openHearth()
    }

    override fun onInterrupt() {}

    override fun onServiceConnected() { Logs.add("home button: Hearth TV catches Home") }
}
