package dev.hearthos.tv

import android.net.Uri
import org.json.JSONObject

/** A saved link's title, picture, site and kind (video, live stream, playlist), read from the page itself. */
object Meta {
    private fun og(html: String, prop: String): String? {
        val re = Regex("""<meta[^>]+(?:property|name)=["']$prop["'][^>]*>""", RegexOption.IGNORE_CASE)
        val tag = re.find(html)?.value ?: return null
        return Regex("""content=["']([^"']*)["']""", RegexOption.IGNORE_CASE).find(tag)?.groupValues?.get(1)?.let { unescape(it) }
    }
    private fun unescape(s: String) = s.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">")

    fun youtubeId(u: Uri): String? = when {
        u.host?.endsWith("youtu.be") == true -> u.pathSegments.firstOrNull()
        u.host?.contains("youtube.com") == true && u.path == "/watch" -> u.getQueryParameter("v")
        u.host?.contains("youtube.com") == true && (u.path?.startsWith("/shorts/") == true || u.path?.startsWith("/live/") == true || u.path?.startsWith("/embed/") == true) -> u.pathSegments.getOrNull(1)
        else -> null
    }

    private fun clock(secs: Long): String {
        val h = secs / 3600; val m = (secs % 3600) / 60; val s = secs % 60
        return if (h > 0) "%d:%02d:%02d".format(h, m, s) else "%d:%02d".format(m, s)
    }

    fun read(url: String): JSONObject {
        val u = Uri.parse(url.trim())
        val host = (u.host ?: "").removePrefix("www.").removePrefix("m.")
        val out = JSONObject().put("url", url.trim()).put("site", host).put("kind", "Videos").put("len", "")
        val yt = youtubeId(u)
        val list = if (host.contains("youtube.com")) u.getQueryParameter("list") else null
        if (host.contains("twitch.tv")) out.put("kind", "Live streams").put("len", "LIVE")
        if (list != null && yt == null) out.put("kind", "Playlists").put("len", "Playlist")
        try {
            if (yt != null || list != null) {
                val (c, b) = Net.get("https://www.youtube.com/oembed?format=json&url=" + Uri.encode(url.trim()))
                if (c == 200) { val o = JSONObject(b); out.put("title", o.optString("title")); out.put("by", o.optString("author_name")) }
                if (yt != null) out.put("image", "https://i.ytimg.com/vi/$yt/hqdefault.jpg").put("yt", yt)
                if (list != null) out.put("list", list)
            }
            if (!out.has("title") || !out.has("image")) {
                val (c, html) = Net.get(url.trim(), max = 600_000)
                if (c in 200..299) {
                    if (!out.has("title")) out.put("title", og(html, "og:title") ?: Regex("<title[^>]*>([^<]*)</title>", RegexOption.IGNORE_CASE).find(html)?.groupValues?.get(1)?.let { unescape(it.trim()) } ?: host)
                    if (!out.has("image")) og(html, "og:image")?.let { out.put("image", it) }
                    og(html, "og:video:duration")?.toLongOrNull()?.let { if (out.optString("len").isEmpty()) out.put("len", clock(it)) }
                    if (og(html, "og:type")?.contains("live", true) == true) out.put("kind", "Live streams").put("len", "LIVE")
                }
            }
        } catch (e: Exception) { Logs.add("theater: couldn't read $host: ${e.message}") }
        if (!out.has("title") || out.optString("title").isBlank()) out.put("title", host)
        return out
    }
}
