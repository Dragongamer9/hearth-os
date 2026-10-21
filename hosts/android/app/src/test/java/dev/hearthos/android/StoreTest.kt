package dev.hearthos.android

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class StoreTest {
    @get:Rule val tmp = TemporaryFolder()

    @Test fun favoritesAndHiddenSurviveARestart() {
        val f = tmp.newFolder().resolve("library.json")
        val s = Store(f)
        s.setIn("favorites", "android:com.a", true)
        s.setIn("favorites", "emu:snes:x.sfc", true)
        s.setIn("favorites", "android:com.a", true)          // twice: still once
        s.setIn("hidden", "android:com.b", true)
        s.setIn("favorites", "emu:snes:x.sfc", false)
        val again = Store(f)
        assertEquals(listOf("android:com.a"), again.list("favorites"))
        assertTrue(again.listHas("hidden", "android:com.b"))
        assertFalse(again.listHas("hidden", "android:com.a"))
    }

    @Test fun playTimeAddsUp() {
        val s = Store(tmp.newFolder().resolve("l.json"))
        s.addPlay("android:com.game", 12, 1000)
        s.addPlay("android:com.game", 30, 2000)
        assertEquals(42, s.stats("android:com.game").optLong("minutes"))
        assertEquals(2000, s.stats("android:com.game").optLong("lastPlayed"))
        assertEquals(0, s.stats("android:other").optLong("minutes"))
    }

    @Test fun collections() {
        val s = Store(tmp.newFolder().resolve("l.json"))
        val c = s.collectionCreate("  Couch games  ", 1)
        assertEquals("Couch games", c.getString("name"))
        try { s.collectionCreate("couch GAMES", 2); fail("a second one with the same name") } catch (e: IllegalArgumentException) { }
        try { s.collectionCreate("   ", 3); fail("an empty name") } catch (e: IllegalArgumentException) { }
        val id = c.getString("id")
        s.collectionSet(id, "android:com.a", true)
        s.collectionSet(id, "android:com.b", true)
        s.collectionSet(id, "android:com.a", false)
        assertEquals(listOf(id), s.collectionsOf("android:com.b"))
        assertEquals(emptyList<String>(), s.collectionsOf("android:com.a"))
        s.collectionRename(id, "Party")
        assertEquals("Party", s.collections()[0].getString("name"))
        s.collectionRemove(id)
        assertEquals(0, s.collections().size)
    }

    @Test fun profileKeepsGoodValuesOnly() {   // like hearth-core's profile.rs
        val s = Store(tmp.newFolder().resolve("l.json"))
        assertEquals("Pixel 9", s.profile("Pixel 9").getString("name"))
        var p = s.profileSet(JSONObject().put("name", "  Sam  ").put("accent", "#zzzzzz").put("background", "preset:3"), "Pixel 9")
        assertEquals("Sam", p.getString("name")); assertEquals("#3fd0b8", p.getString("accent")); assertEquals("preset:3", p.getString("background"))
        p = s.profileSet(JSONObject().put("accent", "#ff9f43"), "Pixel 9")
        assertEquals("#ff9f43", p.getString("accent")); assertEquals("Sam", p.getString("name"))
        p = s.profileSet(JSONObject().put("name", ""), "Pixel 9")
        assertEquals("Pixel 9", p.getString("name"))
    }

    @Test fun aBrokenFileStartsOverAndIsKept() {
        val dir = tmp.newFolder(); val f = dir.resolve("l.json")
        f.writeText("{not json")
        val s = Store(f)
        assertEquals(emptyList<String>(), s.list("favorites"))
        assertTrue(dir.resolve("l.json.broken").exists())
        s.setIn("manual", "com.app", true)
        assertEquals(listOf("com.app"), Store(f).list("manual"))
    }
}
