package dev.hearthos.tv

import android.view.KeyEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The remote's buttons on a website (ViewerActivity) and the pointer's settings. */
class PointerKeysTest {
    @Test fun dpadMovesThePointer() {
        assertEquals(PointerKey.UP, PointerKey.of(KeyEvent.KEYCODE_DPAD_UP))
        assertEquals(PointerKey.DOWN, PointerKey.of(KeyEvent.KEYCODE_DPAD_DOWN))
        assertEquals(PointerKey.LEFT, PointerKey.of(KeyEvent.KEYCODE_DPAD_LEFT))
        assertEquals(PointerKey.RIGHT, PointerKey.of(KeyEvent.KEYCODE_DPAD_RIGHT))
        assertEquals("left", PointerKey.LEFT.js)
        assertTrue(PointerKey.UP.isDirection)
        assertFalse(PointerKey.OK.isDirection)
    }

    @Test fun okAndAControllersA() {
        for (k in listOf(KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER, KeyEvent.KEYCODE_BUTTON_A))
            assertEquals(PointerKey.OK, PointerKey.of(k))
    }

    @Test fun backAndB() {
        assertEquals(PointerKey.BACK, PointerKey.of(KeyEvent.KEYCODE_BACK))
        assertEquals(PointerKey.BACK, PointerKey.of(KeyEvent.KEYCODE_BUTTON_B))
    }

    @Test fun menuAndColourKeysSwitchModes() {
        for (k in listOf(KeyEvent.KEYCODE_MENU, KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_PROG_RED, KeyEvent.KEYCODE_PROG_GREEN, KeyEvent.KEYCODE_PROG_YELLOW, KeyEvent.KEYCODE_PROG_BLUE))
            assertEquals(PointerKey.MODE, PointerKey.of(k))
    }

    @Test fun channelAndNumberKeysScroll() {
        assertEquals(PointerKey.SCROLL_UP, PointerKey.of(KeyEvent.KEYCODE_CHANNEL_UP))
        assertEquals(PointerKey.SCROLL_DOWN, PointerKey.of(KeyEvent.KEYCODE_CHANNEL_DOWN))
        assertEquals(PointerKey.SCROLL_UP, PointerKey.of(KeyEvent.KEYCODE_PAGE_UP))
        assertEquals(PointerKey.SCROLL_DOWN, PointerKey.of(KeyEvent.KEYCODE_PAGE_DOWN))
        assertEquals(PointerKey.SCROLL_UP, PointerKey.of(KeyEvent.KEYCODE_2))
        assertEquals(PointerKey.SCROLL_DOWN, PointerKey.of(KeyEvent.KEYCODE_8))
        assertEquals(PointerKey.SCROLL_LEFT, PointerKey.of(KeyEvent.KEYCODE_4))
        assertEquals(PointerKey.SCROLL_RIGHT, PointerKey.of(KeyEvent.KEYCODE_6))
    }

    @Test fun videoKeysKeepWorking() {
        assertEquals(PointerKey.PLAYPAUSE, PointerKey.of(KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE))
        assertEquals(PointerKey.FF, PointerKey.of(KeyEvent.KEYCODE_MEDIA_FAST_FORWARD))
        assertEquals(PointerKey.REW, PointerKey.of(KeyEvent.KEYCODE_MEDIA_REWIND))
        assertEquals(PointerKey.FULLSCREEN, PointerKey.of(KeyEvent.KEYCODE_BUTTON_Y))
    }

    @Test fun volumeHomeAndOthersAreLeftToTheTv() {
        for (k in listOf(KeyEvent.KEYCODE_VOLUME_UP, KeyEvent.KEYCODE_VOLUME_DOWN, KeyEvent.KEYCODE_VOLUME_MUTE, KeyEvent.KEYCODE_HOME, KeyEvent.KEYCODE_TV_INPUT, KeyEvent.KEYCODE_A))
            assertEquals(PointerKey.OTHER, PointerKey.of(k))
    }

    @Test fun longPress() {
        assertFalse(PointerKey.isLong(1000, 1599))
        assertTrue(PointerKey.isLong(1000, 1600))
    }

    @Test fun wheel() {
        // Scrolling down (positive pixels) is a wheel turned towards you: negative on Android's vertical axis.
        assertEquals(-2f, PointerKey.wheelV(128f, 64f), 1e-6f)
        assertEquals(1.5f, PointerKey.wheelV(-96f, 64f), 1e-6f)
        assertEquals(2f, PointerKey.wheelH(128f, 64f), 1e-6f)
        assertEquals(0f, PointerKey.wheelV(100f, 0f), 0f)
    }

    @Test fun settingsDefaultsAndChoices() {
        assertEquals(PointerSettings("normal", "m", true), PointerSettings.of(null, null, null))
        assertEquals(PointerSettings("normal", "m", true), PointerSettings.of("", "huge", null))
        assertEquals(PointerSettings("fast", "l", false), PointerSettings.of("fast", "l", false))
        assertEquals(PointerSettings("slow", "s", true), PointerSettings.of("slow", "s", true))
    }
}
