package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LogsTest {
    @Test fun notesFitInAHeader() {   // the same as hearth-core's logs.rs header_note
        assertEquals("Game froze", Logs.headerNote("  Game froze  "))
        assertEquals("a%0Ab", Logs.headerNote("a\nb"))
        assertEquals("%C3%A9", Logs.headerNote("é"))
        assertEquals("x%3A y", Logs.headerNote("x: y"))
        assertTrue(Logs.headerNote("é".repeat(800)).length <= 3000)
    }

    @Test fun longLogsKeepTheirHeadAndNewestLines() {
        val head = "Hearth logs · now · Hearth 0.2.1 (Android)\n\n\n===== About this device =====\n{}\n"
        val text = head + "\n===== Hearth log =====\n" + (1..50_000).joinToString("\n") { "line $it" }
        val t = Logs.trimForSending(text, 100_000)
        assertTrue(t.startsWith("Hearth logs"))
        assertTrue(t.length <= 100_000)
        assertTrue(t.endsWith("line 50000"))
        assertTrue(t.contains("older lines left out"))
        assertEquals("short", Logs.trimForSending("short"))
    }
}
