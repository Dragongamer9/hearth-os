package dev.hearthos.tv

import java.io.File
import java.net.HttpURLConnection
import java.net.URL

object Net {
    const val UA = "Mozilla/5.0 (Linux; Android 12; Hearth TV) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36"

    fun get(url: String, max: Int = 2_000_000, headers: Map<String, String> = emptyMap()): Pair<Int, String> {
        val c = URL(url).openConnection() as HttpURLConnection
        c.connectTimeout = 8000; c.readTimeout = 15000; c.instanceFollowRedirects = true
        c.setRequestProperty("User-Agent", UA)
        headers.forEach { (k, v) -> c.setRequestProperty(k, v) }
        return try {
            val code = c.responseCode
            val st = if (code in 200..299) c.inputStream else c.errorStream
            val bytes = st?.use { s ->
                val out = java.io.ByteArrayOutputStream(); val buf = ByteArray(16384); var n: Int
                while (s.read(buf).also { n = it } > 0 && out.size() < max) out.write(buf, 0, n)
                out.toByteArray()
            } ?: ByteArray(0)
            code to String(bytes)
        } finally { c.disconnect() }
    }

    fun download(url: String, to: File, progress: (Long, Long) -> Unit = { _, _ -> }) {
        val c = URL(url).openConnection() as HttpURLConnection
        c.connectTimeout = 10000; c.readTimeout = 30000; c.instanceFollowRedirects = true
        c.setRequestProperty("User-Agent", UA)
        try {
            if (c.responseCode !in 200..299) throw IllegalStateException("HTTP ${c.responseCode}")
            val total = c.contentLengthLong
            to.parentFile?.mkdirs()
            val tmp = File(to.path + ".part")
            c.inputStream.use { inp -> tmp.outputStream().use { out ->
                val buf = ByteArray(65536); var n: Int; var got = 0L
                while (inp.read(buf).also { n = it } > 0) { out.write(buf, 0, n); got += n; progress(got, total) }
            } }
            if (!tmp.renameTo(to)) throw IllegalStateException("couldn't save ${to.name}")
        } finally { c.disconnect() }
    }
}
