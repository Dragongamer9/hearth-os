package dev.hearthos.tv

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Where a website in Theater may go (the owner, Nov 21: videos went to random other websites or an error screen). */
class WebNavTest {
    @Test fun sites() {
        assertEquals("vimeo.com", WebNav.site("player.vimeo.com"))
        assertEquals("example.co.uk", WebNav.site("www.example.co.uk"))
        assertEquals("localhost", WebNav.site("localhost"))
        assertEquals("192.168.1.5", WebNav.site("192.168.1.5"))
    }

    @Test fun theSiteItselfAlways() {
        assertEquals(WebNav.Go.Allow, WebNav.decide("https://www.tubi.tv/movies/1", "https://tubi.tv/home", false))
        assertEquals(WebNav.Go.Allow, WebNav.decide("https://tubi.tv/", null, false))
    }

    @Test fun anotherSiteOnlyWhenYouPressed() {
        assertEquals(WebNav.Go.Block, WebNav.decide("https://ads.example.net/win", "https://tubi.tv/movies/1", false))
        assertEquals(WebNav.Go.Allow, WebNav.decide("https://other.org/", "https://tubi.tv/", true))
        assertEquals(WebNav.Go.Allow, WebNav.decide("https://cdn.other.org/v", "https://tubi.tv/", false, redirect = true))
    }

    @Test fun appLinksNeverShowAnErrorScreen() {
        assertEquals(WebNav.Go.Block, WebNav.decide("market://details?id=x", "https://tubi.tv/", true))
        assertEquals(WebNav.Go.Block, WebNav.decide("intent://x#Intent;scheme=tubi;end", "https://tubi.tv/", true))
        assertEquals(WebNav.Go.Instead("https://tubi.tv/watch/1"),
            WebNav.decide("intent://x#Intent;scheme=tubi;S.browser_fallback_url=https%3A%2F%2Ftubi.tv%2Fwatch%2F1;end", "https://tubi.tv/", true))
        assertEquals(WebNav.Go.Allow, WebNav.decide("about:blank", "https://tubi.tv/", false))
    }

    @Test fun aUserAgentSitesPlayFor() {
        val wv = "Mozilla/5.0 (Linux; Android 12; Smart TV Pro; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/131.0.6778.39 Safari/537.36"
        val ua = WebNav.userAgent(wv)
        assertFalse(ua.contains("; wv"))
        assertFalse(ua.contains("Version/4.0"))
        assertTrue(ua.contains("Chrome/131.0.6778.39"))
        assertFalse(WebNav.userAgent(null).contains("Hearth TV"))
    }
}
