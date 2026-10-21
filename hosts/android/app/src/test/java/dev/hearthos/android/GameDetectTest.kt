package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GameDetectTest {
    @Test fun androidsOwnGameCategoryCounts() {
        assertTrue(GameDetect.isGame("com.example.puzzle", GameDetect.CATEGORY_GAME, false))
        assertTrue(GameDetect.isGame("com.example.old", -1, true))
        assertFalse(GameDetect.isGame("com.example.notes", -1, false))
        assertFalse(GameDetect.isGame("com.example.music", 1, false))   // CATEGORY_AUDIO
    }

    @Test fun emulatorsAndLaunchersAreGames() {
        for (p in listOf("com.retroarch", "com.retroarch.aarch64", "org.dolphinemu.dolphinemu", "org.ppsspp.ppsspp", "xyz.aethersx2.android",
                "org.citra.citra_emu", "io.github.lime3ds.android", "com.github.stenzek.duckstation", "org.vita3k.emulator", "com.winlator",
                "com.droiddeck.launcher", "com.valvesoftware.steamlink", "com.limelight", "com.nvidia.geforcenow", "com.gamepass"))
            assertTrue(p, GameDetect.isGame(p, -1, false))
    }

    @Test fun forksOfAFamilyCount() {
        assertTrue(GameDetect.isKnown("org.dolphinemu.dolphinemu.debug"))
        assertTrue(GameDetect.isKnown("com.retroarch.nightly"))
        assertFalse(GameDetect.isKnown("com.retroarchive"))   // a prefix needs its dot
    }

    @Test fun droidDeckIsKnownByItsPackage() = assertEquals("DroidDeck", GameDetect.KNOWN["com.droiddeck.launcher"])
}
