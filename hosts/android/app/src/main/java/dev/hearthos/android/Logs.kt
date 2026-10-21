package dev.hearthos.android

import android.util.Log
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URI
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Hearth's log on this device: one file that keeps the newest lines (about 512 KB, plus the one before it), and
 * the bug report made from it: "Save logs" (a file in Hearth's folder on the shared storage) and "Send logs to the
 * developer" (the same text to Hearth OS's bug-report service, like the Linux and Windows hosts).
 */
object Logs {
    const val SERVICE = "https://hearth-counter.game305-cl.workers.dev"
    private const val MAX_FILE = 512 * 1024L
    /** The service keeps up to about 1.8 MB; Hearth sends at most this much, its newest lines. */
    const val MAX_SEND = 600_000

    private var dir: File? = null
    private val stamp = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US)

    fun init(folder: File) { folder.mkdirs(); dir = folder }

    private val current get() = dir?.let { File(it, "hearth.log") }
    private val previous get() = dir?.let { File(it, "hearth.log.1") }

    @Synchronized
    fun log(line: String) {
        Log.i("Hearth", line)
        val f = current ?: return
        try {
            if (f.length() > MAX_FILE) { previous?.delete(); f.renameTo(previous!!) }
            f.appendText("${stamp.format(Date())} $line\n")
        } catch (_: Exception) { }
    }

    /** The whole report: a header the service checks ("Hearth logs"), the device, then the log, oldest first. */
    @Synchronized
    fun report(version: String, device: JSONObject): String {
        val sb = StringBuilder()
        sb.append("Hearth logs · ").append(SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.US).format(Date())).append(" · Hearth ").append(version).append(" (Android)\n\n")
        sb.append("\n===== About this device =====\n").append(device.toString(2)).append('\n')
        sb.append("\n===== Hearth log =====\n")
        for (f in listOfNotNull(previous, current)) if (f.exists()) sb.append(runCatching { f.readText() }.getOrDefault(""))
        return sb.toString()
    }

    /** Too long for the service: the header and device kept, then the newest lines. */
    fun trimForSending(text: String, max: Int = MAX_SEND): String {
        if (text.length <= max) return text
        val headEnd = text.indexOf("\n===== Hearth log").let { if (it < 0) 0 else it }
        val mark = "\n===== Hearth log =====\n[… older lines left out …]\n"
        val tail = text.substring(text.length - (max - headEnd - mark.length).coerceAtLeast(1000))
        return text.substring(0, headEnd) + mark + tail.substringAfter('\n')
    }

    /** The person's note ("What happened?") for an HTTP header: printable ASCII kept, the rest %-encoded (UTF-8). */
    fun headerNote(note: String): String {
        var bytes = note.trim().toByteArray(Charsets.UTF_8)
        if (bytes.size > 1000) {
            var cut = 1000
            while (cut > 0 && (bytes[cut].toInt() and 0xC0) == 0x80) cut--   // not inside a letter
            bytes = bytes.copyOf(cut)
        }
        val sb = StringBuilder()
        for (b in bytes) {
            val c = b.toInt() and 0xff
            if ((c in 'a'.code..'z'.code) || (c in 'A'.code..'Z'.code) || (c in '0'.code..'9'.code) || " -_.~!*'()".indexOf(c.toChar()) >= 0) sb.append(c.toChar())
            else sb.append('%').append(String.format(Locale.US, "%02X", c))
        }
        return sb.toString()
    }

    /** Sends [text] to the bug-report service; returns its code. Throws with a sentence to show when it can't. */
    fun send(text: String, version: String, note: String): String {
        val c = URI("$SERVICE/v1/logs").toURL().openConnection() as HttpURLConnection
        try {
            c.requestMethod = "POST"
            c.connectTimeout = 20_000; c.readTimeout = 60_000
            c.doOutput = true
            c.setRequestProperty("Content-Type", "text/plain; charset=utf-8")
            c.setRequestProperty("X-Hearth-Version", version)
            c.setRequestProperty("X-Hearth-Kind", "android")
            if (note.isNotBlank()) c.setRequestProperty("X-Hearth-Note", headerNote(note))
            val body = trimForSending(text).toByteArray(Charsets.UTF_8)
            c.setFixedLengthStreamingMode(body.size)
            c.outputStream.use { it.write(body) }
            val ok = c.responseCode in 200..299
            val reply = (if (ok) c.inputStream else c.errorStream)?.bufferedReader()?.use { it.readText() } ?: ""
            val j = runCatching { JSONObject(reply) }.getOrNull()
            val code = j?.optString("code", "")?.takeIf { it.isNotEmpty() && ok }
            return code ?: throw Exception("Sending didn't work${j?.optString("error", "")?.takeIf { it.isNotEmpty() }?.let { " ($it)" } ?: ""}. Check the internet connection and try again.")
        } finally { c.disconnect() }
    }
}
