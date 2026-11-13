package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ViewportTest {
    @Test fun galaxyTabA9() {
        // 8.7", 1340×800 px at about 1.33: 1005×600 dp. Landscape is laid out like a Steam Deck, portrait at 800 px.
        assertEquals(1280, Viewport.width(1005, 600))
        assertEquals(800, Viewport.width(600, 1005))
        assertTrue(Viewport.isTablet(600))
        assertEquals("width=1280, initial-scale=0.7852, minimum-scale=0.7852, maximum-scale=0.7852, user-scalable=no, viewport-fit=cover", Viewport.meta(1280, 1005))
        assertEquals("width=800, initial-scale=0.7500, minimum-scale=0.7500, maximum-scale=0.7500, user-scalable=no, viewport-fit=cover", Viewport.meta(800, 600))
    }

    @Test fun phonesAndBigScreens() {
        assertEquals(1280, Viewport.width(915, 412))          // a phone on its side
        assertFalse(Viewport.isTablet(412))                    // phones stay landscape
        assertEquals(0, Viewport.width(1600, 1000))            // big tablets and TVs: their own width
        assertEquals(0, Viewport.width(900, 1400))
        assertEquals("width=device-width, initial-scale=1, viewport-fit=cover", Viewport.meta(0, 1600))
        assertEquals(1280, Viewport.width(0, 0))               // unknown: the designed size
    }
}
