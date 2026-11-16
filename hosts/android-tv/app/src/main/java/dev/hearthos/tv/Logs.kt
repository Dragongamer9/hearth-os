package dev.hearthos.tv

import android.util.Log
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/** Hearth TV's own log (files/hearth-tv.log, kept under 1 MB), also sent with Send logs. */
object Logs {
    @Volatile private var file: File? = null
    fun init(dir: File) { file = File(dir, "hearth-tv.log") }
    @Synchronized fun add(line: String) {
        Log.i("HearthTV", line)
        val f = file ?: return
        try {
            if (f.length() > 1_000_000) { f.copyTo(File(f.path + ".old"), overwrite = true); f.writeText("") }
            f.appendText(SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US).format(Date()) + " " + line + "\n")
        } catch (_: Exception) {}
    }
    fun text(): String = file?.takeIf { it.exists() }?.readText()?.takeLast(200_000) ?: ""
}
