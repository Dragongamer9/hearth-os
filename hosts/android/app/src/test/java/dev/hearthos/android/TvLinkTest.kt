package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class TvLinkTest {
    @get:Rule val tmp = TemporaryFolder()
    private val key = "0123456789abcdef01234567"

    @Test fun theTvsPairLinkIsChecked() {
        val tv = TvLink.parsePair("hearthtv://pair?a=192.168.1.30&p=8467&k=$key&n=Living+room+TV")!!
        assertEquals(TvLink.Tv("Living room TV", "192.168.1.30", 8467, key), tv)
        assertNull(TvLink.parsePair("hearthtv://pair?a=evil.example&p=8467&k=$key"))
        assertNull(TvLink.parsePair("hearthtv://pair?a=192.168.1.30&p=99999&k=$key"))
        assertNull(TvLink.parsePair("hearthtv://pair?a=192.168.1.30&p=8467&k=short"))
        assertNull(TvLink.parsePair("https://pair?a=192.168.1.30&p=8467&k=$key"))
        assertEquals("Hearth TV", TvLink.parsePair("hearthtv://pair?a=10.0.0.2&p=1&k=$key&n=%3C%3E")!!.name)
    }

    @Test fun theLinkInWhatWasShared() {
        assertEquals("https://youtu.be/aqz-KE-bpKQ", TvLink.linkIn("Watch this! https://youtu.be/aqz-KE-bpKQ."))
        assertEquals("https://www.twitch.tv/x?y=1", TvLink.linkIn("https://www.twitch.tv/x?y=1"))
        assertNull(TvLink.linkIn("no link here"))
        assertNull(TvLink.linkIn(null))
    }

    @Test fun tvsAreKeptAndAddedByKey() {
        val f = tmp.newFolder()
        TvLink.save(TvLink.store(f), TvLink.Tv("Living room", "192.168.1.30", 8467, key))
        TvLink.save(TvLink.store(f), TvLink.Tv("Bedroom", "192.168.1.31", 8467, "f".repeat(24)))
        // The living room TV moved to another address: updated, not added twice.
        TvLink.save(TvLink.store(f), TvLink.Tv("Living room", "192.168.1.44", 8467, key))
        val l = TvLink.list(TvLink.store(f))
        assertEquals(listOf("192.168.1.44", "192.168.1.31"), l.map { it.addr })
        TvLink.remove(TvLink.store(f), key)
        assertEquals(listOf("Bedroom"), TvLink.list(TvLink.store(f)).map { it.name })
    }

    @Test fun rangesForThePlayer() {
        assertNull(TvLink.range(null, 100))
        assertEquals(0L to 99L, TvLink.range("bytes=0-", 100))
        assertEquals(10L to 19L, TvLink.range("bytes=10-19", 100))
        assertEquals(90L to 99L, TvLink.range("bytes=-10", 100))
        assertEquals(90L to 99L, TvLink.range("bytes=90-500", 100))
        assertEquals(-1L to -1L, TvLink.range("bytes=100-", 100))
        assertEquals(-1L to -1L, TvLink.range("items=0-1", 100))
    }

    @Test fun theTvsAnswer() {
        val (c, o) = TvLink.parseAnswer("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"name\":\"Living room TV\"}")
        assertEquals(200, c); assertEquals("Living room TV", o.optString("name"))
        assertEquals(403, TvLink.parseAnswer("HTTP/1.1 403 Error\r\n\r\n{\"ok\":false,\"error\":\"Scan the code\"}").first)
    }
}
