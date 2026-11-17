package dev.hearthos.tv

import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream

/**
 * Hearth TV's settings file (RED FLAG, the owner's rule 9 and docs/SETTINGS-PERSISTENCE.md):
 * - atomic: a temporary file with its own name in the same folder, fsync, rename over the old one;
 * - FILE.bak: the last good version, copied before each write that changes the file;
 * - never defaults over a damaged file: it's put back from FILE.bak, or kept as FILE.broken-<time>, and the person
 *   is told once (an event the page shows);
 * - merge on write: only the keys that changed are changed, on the latest file, one writer at a time;
 * - defaults per key: the page fills in missing keys when it reads; they're never written by reading.
 */
class SettingsStore(private val file: File) {
    private val lock = Any()
    /** Set when the file had to be put back or set aside: shown to the person once. */
    @Volatile var event: String? = null

    fun read(): JSONObject = synchronized(lock) { load() }

    /** Merges [patch] into the file: keys with a value are set, keys set to null are removed. Returns the result. */
    fun merge(patch: JSONObject): JSONObject = synchronized(lock) {
        val cur = load()
        var changed = false
        val keys = patch.keys()
        while (keys.hasNext()) {
            val k = keys.next()
            val v = patch.opt(k)
            if (v == null || v == JSONObject.NULL) {
                if (cur.has(k)) { cur.remove(k); changed = true }
            } else if (!cur.has(k) || cur.opt(k).toString() != v.toString()) {
                cur.put(k, v); changed = true
            }
        }
        if (changed) write(cur)
        cur
    }

    private fun parse(f: File): JSONObject? = try {
        val t = f.readText()
        if (t.isBlank()) null else JSONObject(t)
    } catch (_: Exception) { null }

    private fun load(): JSONObject {
        if (!file.exists()) return JSONObject()
        parse(file)?.let { return it }
        // Damaged: put back the last good version, or set it aside. Never defaults over it.
        val bak = File(file.path + ".bak")
        val good = if (bak.exists()) parse(bak) else null
        if (good != null) {
            writeAtomic(good.toString(2))
            event = "Your Hearth TV settings file was damaged, so the last good copy was put back."
            Logs.add("settings: ${file.name} damaged, restored from .bak")
            return good
        }
        val broken = File(file.path + ".broken-" + System.currentTimeMillis())
        file.renameTo(broken)
        event = "Your Hearth TV settings file was damaged and there was no good copy. It was kept as ${broken.name}."
        Logs.add("settings: ${file.name} damaged, no .bak; kept as ${broken.name}")
        return JSONObject()
    }

    private fun write(obj: JSONObject) {
        if (file.exists() && parse(file) != null) file.copyTo(File(file.path + ".bak"), overwrite = true)
        writeAtomic(obj.toString(2))
    }

    private fun writeAtomic(text: String) {
        file.parentFile?.mkdirs()
        val tmp = File(file.parentFile, ".${file.name}.${android.os.Process.myPid()}.${System.nanoTime()}.tmp")
        FileOutputStream(tmp).use { out ->
            out.write(text.toByteArray())
            out.fd.sync()
        }
        if (!tmp.renameTo(file)) { tmp.delete(); throw IllegalStateException("couldn't save ${file.name}") }
    }
}
