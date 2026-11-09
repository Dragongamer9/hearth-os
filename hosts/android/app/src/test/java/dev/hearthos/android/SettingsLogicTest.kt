package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/** The settings bridge's rules (SettingsLogic.kt): what Hearth may change, with which values and permissions, and
 *  where each Android page or panel goes. */
class SettingsLogicTest {
    private fun refused(key: String, v: Any?) {
        try { SettingsLogic.parse(key, v); fail("$key=$v was taken") } catch (e: IllegalArgumentException) { }
    }

    @Test fun volumesAreStepsWithinTheirStream() {
        val max = mapOf("media" to 15, "ring" to 7, "alarm" to 7, "notification" to 7)
        assertEquals(9, SettingsLogic.parse("vol.media", 9, max::getValue).value)
        assertEquals(15, SettingsLogic.parse("vol.media", 40, max::getValue).value)       // never past the stream's max
        assertEquals(0, SettingsLogic.parse("vol.ring", -3, max::getValue).value)
        assertEquals(7, SettingsLogic.parse("vol.alarm", "7", max::getValue).value)
        assertNull(SettingsLogic.parse("vol.notification", 2, max::getValue).needs)     // volumes need no permission
        refused("vol.system", 3)
        refused("vol.media", "loud")
    }

    @Test fun displaySettingsNeedModifySystemSettings() {
        assertEquals("writeSettings", SettingsLogic.parse("brightness", 50).needs)
        assertEquals("writeSettings", SettingsLogic.parse("autoBrightness", true).needs)
        assertEquals("writeSettings", SettingsLogic.parse("autoRotate", false).needs)
        assertEquals("writeSettings", SettingsLogic.parse("timeout", 60).needs)
        assertEquals("dnd", SettingsLogic.parse("dnd", "priority").needs)
        assertNull(SettingsLogic.parse("mute", true).needs)
    }

    @Test fun brightnessIsPercentOnScreenAnd0To255ForAndroid() {
        assertEquals(255, SettingsLogic.percentToBrightness(100.0))
        assertEquals(128, SettingsLogic.percentToBrightness(50.0))
        assertEquals(1, SettingsLogic.percentToBrightness(0.0))          // never fully dark
        assertEquals(255, SettingsLogic.percentToBrightness(140.0))
        assertEquals(100, SettingsLogic.brightnessToPercent(255))
        assertEquals(50, SettingsLogic.brightnessToPercent(128))
        assertEquals(0, SettingsLogic.brightnessToPercent(-4))
        for (p in 0..100 step 5) assertTrue(Math.abs(SettingsLogic.brightnessToPercent(SettingsLogic.percentToBrightness(p.toDouble())) - p) <= 1)
    }

    @Test fun timeoutsSnapToAndroidsList() {
        assertEquals(60_000, SettingsLogic.parse("timeout", 60).value)
        assertEquals(30_000, SettingsLogic.parse("timeout", 40).value)
        assertEquals(1_800_000, SettingsLogic.parse("timeout", 99999).value)
        assertEquals("30 seconds", SettingsLogic.timeoutLabel(30))
        assertEquals("1 minute", SettingsLogic.timeoutLabel(60))
        assertEquals("10 minutes", SettingsLogic.timeoutLabel(600))
        assertEquals("Never", SettingsLogic.timeoutLabel(0))
    }

    @Test fun switchesAndDoNotDisturb() {
        assertEquals(true, SettingsLogic.parse("autoRotate", "true").value)
        assertEquals(false, SettingsLogic.parse("mute", 0).value)
        refused("autoRotate", "sideways")
        assertEquals(2, SettingsLogic.parse("dnd", "priority").value)
        assertEquals(4, SettingsLogic.parse("dnd", "alarms").value)
        assertEquals(3, SettingsLogic.parse("dnd", "none").value)
        assertEquals(1, SettingsLogic.parse("dnd", "all").value)
        refused("dnd", "loud")
        assertEquals("priority", SettingsLogic.dndName(2))
        assertEquals("all", SettingsLogic.dndName(0))                      // unknown: shown as off
    }

    @Test fun onlyKnownSettingsChange() {
        refused("wifi", true)            // Android 10+ doesn't let apps switch these: panels instead
        refused("airplane", true)
        refused("batterySaver", true)
        refused("", 1)
    }

    @Test fun everyPermissionSaysWhy() {
        for ((k, why) in SettingsLogic.PERMS) assertTrue("$k has no reason", why.length > 20)
        for (k in listOf("writeSettings", "dnd", "location", "btConnect", "batteryOpt", "notifications")) assertTrue(k in SettingsLogic.PERMS)
    }

    @Test fun wifiDetails() {
        assertEquals("Casa", SettingsLogic.cleanSsid("\"Casa\""))
        assertNull(SettingsLogic.cleanSsid("<unknown ssid>"))
        assertNull(SettingsLogic.cleanSsid(""))
        assertNull(SettingsLogic.cleanSsid(null))
        assertEquals(4, SettingsLogic.bars(-40)); assertEquals(2, SettingsLogic.bars(-75)); assertEquals(0, SettingsLogic.bars(-95))
        assertEquals("wpa", SettingsLogic.security("[WPA2-PSK-CCMP][ESS]"))
        assertEquals("wpa3", SettingsLogic.security("[RSN-SAE-CCMP][ESS]"))
        assertEquals("wpa", SettingsLogic.security("[RSN-PSK+SAE-CCMP][ESS]"))   // transition mode joins as WPA2
        assertEquals("enterprise", SettingsLogic.security("[WPA2-EAP-CCMP][ESS]"))
        assertEquals("open", SettingsLogic.security("[ESS]"))
        assertNull(SettingsLogic.checkJoin("Casa", "hunter22", "wpa"))
        assertNotNull(SettingsLogic.checkJoin("Casa", "short", "wpa"))
        assertNull(SettingsLogic.checkJoin("Cafe", "", "open"))
        assertNotNull(SettingsLogic.checkJoin("", "", "open"))
        assertNotNull(SettingsLogic.checkJoin("Work", "x", "enterprise"))
        assertNotNull(SettingsLogic.checkJoin("a".repeat(33), "hunter22", "wpa"))
    }

    @Test fun bluetoothKinds() {
        assertEquals("controller", SettingsLogic.btKind(0x0508, "Pad"))
        assertEquals("controller", SettingsLogic.btKind(0, "Xbox Wireless Controller"))
        assertEquals("controller", SettingsLogic.btKind(0, "DualSense Wireless Controller"))
        assertEquals("audio", SettingsLogic.btKind(0x0418, "Galaxy Buds2"))
        assertEquals("input", SettingsLogic.btKind(0x0540, "Keyboard K380"))
        assertEquals("other", SettingsLogic.btKind(0x0200, "Phone"))
    }

    @Test fun pagesAndPanels() {
        for (p in listOf("wifi", "bluetooth", "display", "darkmode", "textsize", "sound", "battery", "saver", "batteryOpt", "airplane", "home", "app",
                "notifications", "allNotifications", "dndSettings", "storage", "date", "language", "accessibility", "developer", "about", "nfc", "data", "settings")) {
            val a = SettingsLogic.pageActions(p, 34)
            assertTrue("$p has no page", a.isNotEmpty())
            assertEquals("$p must end at Settings", "android.settings.SETTINGS", a.last())
        }
        assertEquals("android.settings.APPLICATION_DEVELOPMENT_SETTINGS", SettingsLogic.pageActions("developer", 34)[0])
        assertEquals("android.settings.APP_NOTIFICATION_SETTINGS", SettingsLogic.pageActions("notifications", 34)[0])
        assertTrue(SettingsLogic.pageTakesPackage("app"))
        assertFalse(SettingsLogic.pageTakesPackage("wifi"))
        try { SettingsLogic.pageActions("nuke", 34); fail() } catch (e: IllegalArgumentException) { }
        assertEquals("android.settings.panel.action.INTERNET_CONNECTIVITY", SettingsLogic.panelAction("internet", 34))
        assertEquals("android.settings.panel.action.VOLUME", SettingsLogic.panelAction("volume", 29))
        assertNull(SettingsLogic.panelAction("wifi", 28))                 // Android 9: the page instead
        assertEquals("wifi", SettingsLogic.panelFallback("internet"))
        assertEquals("sound", SettingsLogic.panelFallback("volume"))
    }

    @Test fun samsung() {
        assertTrue(SettingsLogic.isSamsung("samsung"))
        assertTrue(SettingsLogic.isSamsung("Samsung"))
        assertFalse(SettingsLogic.isSamsung("OUKITEL"))
        assertFalse(SettingsLogic.isSamsung(null))
    }
}
