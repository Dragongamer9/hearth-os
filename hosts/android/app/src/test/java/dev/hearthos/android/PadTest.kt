package dev.hearthos.android

import android.view.KeyEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PadTest {
    @Test fun buttonsHaveThePagesNames() {
        assertEquals("a", Pad.buttonName(KeyEvent.KEYCODE_BUTTON_A))
        assertEquals("b", Pad.buttonName(KeyEvent.KEYCODE_BUTTON_B))
        assertEquals("guide", Pad.buttonName(KeyEvent.KEYCODE_BUTTON_MODE))
        assertEquals("select", Pad.buttonName(KeyEvent.KEYCODE_BUTTON_SELECT))
        assertEquals("lb", Pad.buttonName(KeyEvent.KEYCODE_BUTTON_L1))
        assertEquals("up", Pad.buttonName(KeyEvent.KEYCODE_DPAD_UP))
        assertNull(Pad.buttonName(KeyEvent.KEYCODE_VOLUME_UP))
    }

    @Test fun sticksWithHysteresis() {
        assertEquals(setOf("left"), Pad.directions(-0.6f, 0f, 0f, 0f, 0f, emptySet()))
        assertEquals(emptySet<String>(), Pad.directions(-0.45f, 0f, 0f, 0f, 0f, emptySet()))   // not far enough to start
        assertEquals(setOf("left"), Pad.directions(-0.45f, 0f, 0f, 0f, 0f, setOf("left")))     // far enough to keep
        assertEquals(setOf("down", "right"), Pad.directions(0f, 0f, 1f, 1f, 0f, emptySet()))   // the hat
        assertEquals(setOf("rup"), Pad.directions(0f, 0f, 0f, 0f, -0.9f, emptySet()))
    }

    @Test fun repeatLikeThePage() {
        val r = Repeater()
        assertEquals(listOf("down"), r.update(setOf("down"), 0))
        assertEquals(emptyList<String>(), r.update(setOf("down"), 50))   // still held: no new press
        assertEquals(emptyList<String>(), r.tick(300))
        assertEquals(listOf("down"), r.tick(420))
        assertEquals(emptyList<String>(), r.tick(480))
        assertEquals(listOf("down"), r.tick(530))
        assertEquals(emptyList<String>(), r.update(emptySet(), 600))
        assertEquals(emptyList<String>(), r.tick(1000))
        r.update(setOf("rup"), 0)
        assertEquals(emptyList<String>(), r.tick(2000))                  // the library jump doesn't repeat
    }
}
