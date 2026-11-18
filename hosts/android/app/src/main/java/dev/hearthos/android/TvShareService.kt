package dev.hearthos.android

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.Uri
import android.os.Build
import android.os.IBinder
import java.io.FileInputStream
import java.net.ServerSocket
import java.net.Socket
import java.security.SecureRandom
import kotlin.concurrent.thread

/**
 * A video or song shared to Hearth TV from this phone (Share › Hearth TV): it stays on the phone and the TV plays it
 * from here, with seeking, while this runs (a notification with Stop). Only the TV it was sent to gets an answer, at
 * an address with a random token in it, and only for that one file. It stops by itself after 4 hours.
 */
class TvShareService : Service() {
    @Volatile private var server: ServerSocket? = null
    @Volatile private var uri: Uri? = null
    @Volatile private var token = ""
    @Volatile private var tvAddr = ""
    @Volatile private var size = -1L
    @Volatile private var mime = "video/mp4"
    private var stopAt = 0L

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent == null || intent.action == STOP) { stopSelf(); return START_NOT_STICKY }
        val u = intent.data ?: run { stopSelf(); return START_NOT_STICKY }
        val tv = intent.getStringExtra("tv")?.let { runCatching { TvLink.tvFrom(org.json.JSONObject(it)) }.getOrNull() } ?: run { stopSelf(); return START_NOT_STICKY }
        val title = intent.getStringExtra("title") ?: "Video"
        foreground(tv.name, title)
        uri = u; tvAddr = tv.addr; mime = contentResolver.getType(u) ?: "video/mp4"
        size = try { contentResolver.openFileDescriptor(u, "r")?.use { it.statSize } ?: -1L } catch (_: Exception) { -1L }
        token = ByteArray(16).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
        stopAt = System.currentTimeMillis() + 4 * 3600_000L
        val ss = server ?: ServerSocket(0).also { server = it; thread(name = "hearth-tv-share", isDaemon = true) { serve(it) } }
        val me = TvLink.myAddress()
        thread(isDaemon = true) {
            val r = if (me == null) Result.failure(Exception("This phone isn't on a network")) else
                TvLink.send(tv, "http://$me:${ss.localPort}/$token/" + safe(title), Build.MODEL, "file", title)
            r.onFailure { Logs.log("share to tv: ${it.message}"); toast(it.message ?: "Couldn't send it to the TV"); stopSelf() }
            r.onSuccess { toast("Playing on $it") }
        }
        return START_NOT_STICKY
    }

    private fun safe(n: String) = n.map { if (it.isLetterOrDigit() || it in ".-_") it else '_' }.joinToString("").take(80).ifBlank { "video" }

    private fun toast(t: String) = android.os.Handler(mainLooper).post { android.widget.Toast.makeText(this, t, android.widget.Toast.LENGTH_LONG).show() }

    private fun foreground(tv: String, title: String) {
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel("tv", "Playing on Hearth TV", NotificationManager.IMPORTANCE_LOW))
        val stop = PendingIntent.getService(this, 0, Intent(this, TvShareService::class.java).setAction(STOP), PendingIntent.FLAG_IMMUTABLE)
        val n = Notification.Builder(this, "tv").setContentTitle("Playing on $tv").setContentText("$title · from this phone")
            .setSmallIcon(android.R.drawable.ic_media_play).setOngoing(true)
            .addAction(Notification.Action.Builder(null, "Stop", stop).build()).build()
        if (Build.VERSION.SDK_INT >= 34) startForeground(7, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE) else startForeground(7, n)
    }

    private fun serve(ss: ServerSocket) {
        try {
            while (!ss.isClosed) {
                val s = ss.accept()
                if (System.currentTimeMillis() > stopAt) { s.close(); android.os.Handler(mainLooper).post { stopSelf() }; continue }
                thread(isDaemon = true) { try { answer(s) } catch (_: Exception) {} finally { runCatching { s.close() } } }
            }
        } catch (_: Exception) {}
    }

    /** GET or HEAD /<token>/<name>, with or without a range; only from the TV. */
    private fun answer(s: Socket) {
        s.soTimeout = 30_000
        val inp = s.getInputStream().bufferedReader()
        val first = inp.readLine() ?: return
        var range: String? = null
        while (true) { val h = inp.readLine() ?: break; if (h.isEmpty()) break; if (h.lowercase().startsWith("range:")) range = h.substringAfter(':').trim() }
        val (method, path) = first.split(' ').let { (it.getOrNull(0) ?: "") to (it.getOrNull(1) ?: "") }
        val out = s.getOutputStream()
        val u = uri
        if (s.inetAddress.hostAddress != tvAddr || u == null || path.trimStart('/').substringBefore('/') != token || method !in listOf("GET", "HEAD") || size < 0) {
            out.write("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".toByteArray()); return
        }
        val r = TvLink.range(range, size)
        if (r != null && r.first < 0) { out.write("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */$size\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".toByteArray()); return }
        val (a, b) = r ?: (0L to size - 1)
        val len = if (size == 0L) 0L else b - a + 1
        out.write(("HTTP/1.1 ${if (r != null) "206 Partial Content" else "200 OK"}\r\nContent-Type: $mime\r\nContent-Length: $len\r\nAccept-Ranges: bytes\r\nConnection: close\r\n" +
            (if (r != null) "Content-Range: bytes $a-$b/$size\r\n" else "") + "\r\n").toByteArray())
        if (method == "HEAD" || len == 0L) return
        contentResolver.openFileDescriptor(u, "r")?.use { fd ->
            FileInputStream(fd.fileDescriptor).use { f ->
                val ch = f.channel; ch.position(a)
                val buf = ByteArray(256 * 1024); var left = len
                while (left > 0) { val n = f.read(buf, 0, minOf(buf.size.toLong(), left).toInt()); if (n <= 0) break; out.write(buf, 0, n); left -= n }
                out.flush()
            }
        }
    }

    override fun onDestroy() { runCatching { server?.close() }; server = null; super.onDestroy() }

    companion object { const val STOP = "dev.hearthos.android.TV_STOP" }
}
