package dev.hearthos.tv

/**
 * Where a website in Theater may go (ViewerActivity), pure so it's tested on its own. The owner, Nov 21: videos on the
 * saved links didn't load, showed an error screen, went to a random other website, or played black.
 *
 * - The site itself, anywhere on it: always.
 * - Another site: only when you clicked something (a link or a button: the press was yours), or the server sent the
 *   page on. A page sending you off to another site by itself (an ad's script) is stopped and the page stays.
 * - Links for apps (intent:, market:, a phone's app links): their web address when they have one, else nothing
 *   (Android's "page can't be loaded" error screen came from these).
 * - New windows (pop-ups, pop-unders): never opened.
 */
object WebNav {
    sealed class Go {
        object Allow : Go()
        object Block : Go()
        data class Instead(val url: String) : Go()
    }

    private val TWO_PART = setOf("co.uk", "org.uk", "ac.uk", "gov.uk", "com.au", "net.au", "org.au", "co.nz", "co.jp", "com.br", "com.mx", "co.in", "co.za", "com.tr")

    /** "www.example.co.uk" → "example.co.uk", "player.vimeo.com" → "vimeo.com". */
    fun site(host: String?): String {
        val h = (host ?: "").lowercase().trimEnd('.')
        if (h.isEmpty()) return ""
        if (h.all { it.isDigit() || it == '.' } || h.contains(':')) return h // an address
        val p = h.split('.')
        if (p.size <= 2) return h
        val last2 = p.takeLast(2).joinToString(".")
        return if (last2 in TWO_PART && p.size >= 3) p.takeLast(3).joinToString(".") else last2
    }

    private fun hostOf(url: String): String? = Regex("""^[a-zA-Z][a-zA-Z0-9+.-]*://([^/?#:]+)""").find(url)?.groupValues?.get(1)

    /** The web address an intent: link carries (S.browser_fallback_url), if any. */
    fun fallback(url: String): String? {
        val m = Regex("""[#;]S\.browser_fallback_url=([^;]+)""").find(url) ?: return null
        val u = try { java.net.URLDecoder.decode(m.groupValues[1], "UTF-8") } catch (_: Exception) { return null }
        return if (u.startsWith("https://") || u.startsWith("http://")) u else null
    }

    /** A page in the main frame going to [url] from the page at [from]; [gesture]: your press started it; [redirect]: the
     *  site's server sent it on (a link that goes through another address on its way). */
    fun decide(url: String, from: String?, gesture: Boolean, redirect: Boolean = false): Go {
        val scheme = url.substringBefore(':').lowercase()
        if (scheme == "about" || scheme == "data" || scheme == "blob" || scheme == "javascript") return Go.Allow
        if (scheme != "http" && scheme != "https") {
            if (scheme == "intent") fallback(url)?.let { return Go.Instead(it) }
            return Go.Block
        }
        val a = site(hostOf(url))
        val b = site(from?.let { hostOf(it) })
        if (b.isEmpty() || a == b) return Go.Allow
        return if (gesture || redirect) Go.Allow else Go.Block
    }

    /** A user agent sites play video for: the TV's own Chrome version, without WebView's "; wv" mark (which some sites
     *  refuse) and without "Hearth TV". */
    fun userAgent(default: String?): String {
        val d = default ?: ""
        if (d.isBlank()) return Net.UA.replace("Android 12; Hearth TV", "Android 12")
        return d.replace("; wv)", ")").replace(Regex("""\s?Version/\d+(\.\d+)*"""), "")
    }
}
