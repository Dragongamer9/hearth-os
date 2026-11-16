package dev.hearthos.tv

import android.content.Context
import android.net.wifi.WifiManager
import org.json.JSONObject
import java.net.Inet4Address
import java.net.NetworkInterface
import java.net.ServerSocket
import java.net.Socket
import java.net.URLDecoder
import java.security.SecureRandom
import kotlin.concurrent.thread

/**
 * Theater's "Add a link" page on your home network: the QR code on the TV opens http://<tv>:8467/add?k=<key> on your
 * phone (or a Hearth PC's browser); paste a link there and it's saved in Theater. The key is random and kept on the
 * TV, so nobody else on the network can add links without having seen the code.
 */
object LocalServer {
    const val PORT = 8467
    @Volatile private var started = false
    var onAdd: ((String) -> Unit)? = null

    fun key(): String {
        val st = TvApp.app.state.read()
        st.optString("addKey").takeIf { it.length >= 16 }?.let { return it }
        val b = ByteArray(12); SecureRandom().nextBytes(b)
        val k = b.joinToString("") { "%02x".format(it) }
        TvApp.app.state.merge(JSONObject().put("addKey", k))
        return k
    }

    fun address(ctx: Context): String? {
        try {
            for (ni in NetworkInterface.getNetworkInterfaces()) {
                if (!ni.isUp || ni.isLoopback) continue
                for (a in ni.inetAddresses) if (a is Inet4Address && !a.isLoopbackAddress) return a.hostAddress
            }
        } catch (_: Exception) {}
        return null
    }

    fun url(ctx: Context): String? = address(ctx)?.let { "http://$it:$PORT/add?k=${key()}" }

    fun start() {
        if (started) return
        started = true
        thread(name = "hearth-tv-add", isDaemon = true) {
            try {
                ServerSocket(PORT).use { ss -> while (true) { val s = ss.accept(); thread(isDaemon = true) { serve(s) } } }
            } catch (e: Exception) { Logs.add("add-a-link page stopped: ${e.message}"); started = false }
        }
    }

    private fun page(body: String) = """<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Hearth TV · Add a link</title>
<style>body{margin:0;font-family:system-ui,sans-serif;background:#171a21;color:#dce5ed;display:flex;min-height:100vh;align-items:center;justify-content:center}
main{width:min(92vw,520px);background:#1e2633;border:1px solid #32404f;border-radius:8px;padding:28px;box-sizing:border-box}
h1{margin:0 0 6px;font-size:24px}p{color:#8f98a0;margin:0 0 18px}input{width:100%;box-sizing:border-box;height:52px;border-radius:4px;border:2px solid #66c0f4;background:#171a21;color:#dce5ed;font-size:17px;padding:0 14px}
button{margin-top:14px;width:100%;height:52px;border:0;border-radius:4px;background:#1a9fff;color:#06121d;font-size:18px;font-weight:700}.ok{color:#4cc38a;font-weight:700;font-size:20px}</style></head>
<body><main>$body</main></body></html>"""

    private fun serve(s: Socket) {
        s.use { sock ->
            sock.soTimeout = 10000
            val inp = sock.getInputStream().bufferedReader()
            val first = inp.readLine() ?: return
            var len = 0
            while (true) { val h = inp.readLine() ?: break; if (h.isEmpty()) break; if (h.lowercase().startsWith("content-length:")) len = h.substringAfter(":").trim().toIntOrNull() ?: 0 }
            val (method, target) = first.split(" ").let { (it.getOrNull(0) ?: "") to (it.getOrNull(1) ?: "/") }
            val q = target.substringAfter("?", "").split("&").associate { it.substringBefore("=") to URLDecoder.decode(it.substringAfter("=", ""), "UTF-8") }
            val okKey = q["k"] == key()
            val html = when {
                !target.startsWith("/add") || !okKey -> page("<h1>Hearth TV</h1><p>Scan the code on the TV (Theater › Add a link) to add a link.</p>")
                method == "POST" -> {
                    val body = CharArray(minOf(len, 8192)).let { b -> var r = 0; while (r < b.size) { val n = inp.read(b, r, b.size - r); if (n < 0) break; r += n }; String(b, 0, r) }
                    val url = body.split("&").firstOrNull { it.startsWith("url=") }?.substringAfter("=")?.let { URLDecoder.decode(it, "UTF-8") }?.trim() ?: ""
                    if (url.startsWith("http://") || url.startsWith("https://")) {
                        onAdd?.invoke(url)
                        page("<h1>Added</h1><p class=\"ok\">✓ It's in Theater on the TV.</p><form method=\"post\" action=\"/add?k=${key()}\"><input name=\"url\" placeholder=\"Paste another link\" inputmode=\"url\"><button>Add to Theater</button></form>")
                    } else page("<h1>Add a link</h1><p>That isn't a web link. Paste a link that starts with https://</p><form method=\"post\" action=\"/add?k=${key()}\"><input name=\"url\" inputmode=\"url\" autofocus><button>Add to Theater</button></form>")
                }
                else -> page("<h1>Add a link to Hearth TV</h1><p>Paste a video link (YouTube, Twitch, any video page). It shows up in Theater on the TV.</p><form method=\"post\" action=\"/add?k=${key()}\"><input name=\"url\" placeholder=\"https://\" inputmode=\"url\" autofocus><button>Add to Theater</button></form>")
            }
            val bytes = html.toByteArray()
            val out = sock.getOutputStream()
            out.write("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: ${bytes.size}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n".toByteArray())
            out.write(bytes); out.flush()
        }
    }
}
