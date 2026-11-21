package dev.hearthos.tv

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Theater's ad blocker (the owner, Nov 21: QR-code and VPN-deal pop-ups over the videos). */
class AdBlockTest {
    private val list = AdBlock.parse("# a comment\npopads.net\n0.0.0.0 ads.example.com\nadsterra.com # the scam overlays\n\nlocalhost\n127.0.0.1\n")

    @Test fun readsPlainAndHostsLines() {
        assertTrue("popads.net" in list && "ads.example.com" in list && "adsterra.com" in list)
        assertFalse("localhost" in list || "127.0.0.1" in list)
    }

    @Test fun blocksTheDomainAndItsSubdomains() {
        assertTrue(AdBlock.blocked("popads.net", "tubi.tv", list))
        assertTrue(AdBlock.blocked("c1.popads.net", "tubi.tv", list))
        assertTrue(AdBlock.blocked("x.ads.example.com", "tubi.tv", list))
        assertFalse(AdBlock.blocked("example.com", "tubi.tv", list))
        assertFalse(AdBlock.blocked("notpopads.net", "tubi.tv", list))
    }

    @Test fun neverThePagesOwnSite() {
        assertFalse(AdBlock.blocked("ads.example.com", "example.com", list))
    }

    @Test fun theCleanerLeavesVideosAlone() {
        assertTrue(AdBlock.CLEAN_JS.contains("holdsVideo(e)"))
        assertTrue(AdBlock.CLEAN_JS.contains("vpn"))
    }
}
