package dev.hearthos.android

import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.net.Inet4Address
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.Socket
import java.net.URLDecoder
import java.net.URLEncoder

/**
 * Share › Hearth TV (docs/HEARTH-TV.md, "Send to TV"): your Hearth TVs on this phone and talking to them. A TV is added
 * from its own add-a-link page (Theater › Add a link: scan the code with the phone's camera, then "Add … to this phone's
 * Share menu", a hearthtv://pair link with the TV's address and key). A shared link goes to the TV's /send; a shared
 * video file plays from this phone (TvShareService) and its address goes to /send.
 *
 * The TVs are kept in hearth-tv.json (Store: atomic, .bak, never defaults over a file that can't be read), only
 * changed when you add one, and kept until you remove them. The TV's address is updated when it moves (found again on
 * the network by its key: /hello).
 */
object TvLink {
    data class Tv(val name: String, val addr: String, val port: Int, val key: String) {
        fun json(): JSONObject = JSONObject().put("name", name).put("addr", addr).put("port", port).put("key", key)
    }

    private fun ipv4(s: String): Boolean {
        val p = s.split('.')
        return p.size == 4 && p.all { x -> x.isNotEmpty() && x.length <= 3 && x.all { it.isDigit() } && x.toInt() <= 255 }
    }

    fun validKey(k: String) = k.length in 16..64 && k.all { it in '0'..'9' || it in 'a'..'f' }

    fun cleanName(n: String) = n.filter { !it.isISOControl() && it != '<' && it != '>' }.trim().take(40).ifBlank { "Hearth TV" }

    /** hearthtv://pair?a=192.168.1.30&p=8467&k=<key>&n=<name>, or null if it isn't one. */
    fun parsePair(uri: String): Tv? {
        if (!uri.startsWith("hearthtv://pair?")) return null
        val q = uri.substringAfter('?').split('&').associate { URLDecoder.decode(it.substringBefore('='), "UTF-8") to URLDecoder.decode(it.substringAfter('=', ""), "UTF-8") }
        val a = q["a"] ?: return null
        val p = q["p"]?.toIntOrNull() ?: return null
        val k = q["k"] ?: return null
        if (!ipv4(a) || p !in 1..65535 || !validKey(k)) return null
        return Tv(cleanName(q["n"] ?: ""), a, p, k)
    }

    fun tvFrom(o: JSONObject): Tv? {
        val t = Tv(cleanName(o.optString("name")), o.optString("addr"), o.optInt("port"), o.optString("key"))
        return if (ipv4(t.addr) && t.port in 1..65535 && validKey(t.key)) t else null
    }

    /** The first web link in what an app shared ("Watch this! https://youtu.be/…"). */
    fun linkIn(text: String?): String? = text?.let { Regex("""https?://[^\s<>"]+""").find(it)?.value?.trimEnd('.', ',', ')', '!', '?') }?.takeIf { it.length <= 4000 }

    fun form(vararg kv: Pair<String, String>) = kv.joinToString("&") { URLEncoder.encode(it.first, "UTF-8") + "=" + URLEncoder.encode(it.second, "UTF-8") }

    // ---------------------------------------------------------------- the TVs on this phone

    fun store(dir: File) = Store(File(dir, "hearth-tv.json"))

    fun list(s: Store): List<Tv> {
        val a = s.data.optJSONArray("tvs") ?: return emptyList()
        return (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::tvFrom) }
    }

    /** Adds a TV, or updates the one with the same key (a TV that moved, or was renamed). Nothing else changes. */
    fun save(s: Store, tv: Tv) = s.update { d ->
        val old = d.optJSONArray("tvs") ?: JSONArray()
        val out = JSONArray()
        var placed = false
        for (i in 0 until old.length()) {
            val o = old.optJSONObject(i) ?: continue
            if (o.optString("key") == tv.key) { if (!placed) out.put(tv.json()); placed = true } else out.put(o)
        }
        if (!placed) out.put(tv.json())
        d.put("tvs", out)
    }

    fun remove(s: Store, key: String) = s.update { d ->
        val old = d.optJSONArray("tvs") ?: JSONArray()
        val out = JSONArray()
        for (i in 0 until old.length()) old.optJSONObject(i)?.takeIf { it.optString("key") != key }?.let { out.put(it) }
        d.put("tvs", out)
    }

    // ---------------------------------------------------------------- talking to a TV (plain http on your network)

    /** One request to the TV's add-a-link server: (status, JSON answer). Raw sockets: only the TV, only on your network. */
    fun request(tv: Tv, method: String, path: String, body: String = "", timeoutMs: Int = 6000): Pair<Int, JSONObject> {
        return Socket().use { s ->
            s.connect(InetSocketAddress(tv.addr, tv.port), timeoutMs)
            s.soTimeout = timeoutMs
            val b = body.toByteArray()
            val head = "$method $path HTTP/1.1\r\nHost: ${tv.addr}:${tv.port}\r\nConnection: close\r\n" +
                (if (method == "POST") "Content-Type: application/x-www-form-urlencoded\r\nContent-Length: ${b.size}\r\n" else "") + "\r\n"
            s.getOutputStream().apply { write(head.toByteArray()); write(b); flush() }
            val text = s.getInputStream().readBytes().take(65536).toByteArray().toString(Charsets.UTF_8)
            parseAnswer(text)
        }
    }

    fun parseAnswer(text: String): Pair<Int, JSONObject> {
        val status = text.substringBefore("\r\n").split(' ').getOrNull(1)?.toIntOrNull() ?: 0
        val body = text.substringAfter("\r\n\r\n", "")
        val o = try { JSONObject(body) } catch (_: Exception) { JSONObject().put("ok", false).put("error", "The TV's answer wasn't readable") }
        return status to o
    }

    fun hello(tv: Tv): JSONObject? = try { request(tv, "GET", "/hello?k=${tv.key}").let { if (it.first == 200 && it.second.optBoolean("ok")) it.second else null } } catch (_: Exception) { null }

    /** Plays it on the TV now (saved in its Theater too). kind "link" or "file" (a file's address on this phone). */
    fun send(tv: Tv, url: String, from: String, kind: String = "link", title: String = ""): Result<String> = try {
        val (code, o) = request(tv, "POST", "/send?k=${tv.key}", form("url" to url, "from" to from, "kind" to kind, "title" to title))
        if (code == 200 && o.optBoolean("ok")) Result.success(o.optString("name", tv.name)) else Result.failure(Exception(o.optString("error").ifBlank { "The TV said no ($code)" }))
    } catch (e: Exception) { Result.failure(Exception("Can't reach ${tv.name}: is it on, and on the same Wi-Fi as this phone?")) }

    /** This phone's address on the home network (Wi-Fi first). */
    fun myAddress(): String? = try {
        NetworkInterface.getNetworkInterfaces().toList().filter { it.isUp && !it.isLoopback }
            .sortedBy { if (it.name.startsWith("wlan")) 0 else 1 }
            .flatMap { it.inetAddresses.toList() }.firstOrNull { it is Inet4Address && !it.isLoopbackAddress }?.hostAddress
    } catch (_: Exception) { null }

    /** "bytes=a-b" for a file of `size` bytes: (first, last) inclusive; null = the whole file; (-1,-1) = can't be served. */
    fun range(h: String?, size: Long): Pair<Long, Long>? {
        if (h == null) return null
        val t = h.trim()
        if (!t.startsWith("bytes=") || size <= 0) return -1L to -1L
        val spec = t.removePrefix("bytes=").split(',').first().trim()
        val a = spec.substringBefore('-', ""); val b = spec.substringAfter('-', "")
        val (first, last) = if (a.isEmpty()) { val n = b.toLongOrNull() ?: return -1L to -1L; if (n <= 0) return -1L to -1L; maxOf(0L, size - n) to size - 1 }
            else { val f = a.toLongOrNull() ?: return -1L to -1L; f to (if (b.isEmpty()) size - 1 else minOf(b.toLongOrNull() ?: return -1L to -1L, size - 1)) }
        return if (first > last || first >= size) -1L to -1L else first to last
    }
}
