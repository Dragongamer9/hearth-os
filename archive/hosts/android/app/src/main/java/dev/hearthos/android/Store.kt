package dev.hearthos.android

import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * Hearth's own data on this device, one JSON file in the app's private folder (like library.json on Linux and
 * Windows): favorites, hidden games, collections, play time, apps added to the library, the ROMs folder and the
 * profile. Written whole to a temporary file and renamed, so it's never left half-written.
 *
 *   {"favorites":[id], "hidden":[id], "manual":[package], "collections":[{id,name,games:[id]}],
 *    "stats":{id:{minutes,lastPlayed}}, "showAll":bool, "roms":{tree,docId}, "profile":{...}, "running":{id,startedAt}}
 */
class Store(private val file: File) {
    @get:Synchronized
    var data: JSONObject = load()
        private set

    private fun load(): JSONObject = try {
        if (file.exists()) JSONObject(file.readText()) else JSONObject()
    } catch (e: Exception) {
        // A broken file isn't thrown away: it's kept next to the new one for a bug report.
        runCatching { file.copyTo(File(file.path + ".broken"), overwrite = true) }
        JSONObject()
    }

    @Synchronized
    fun update(change: (JSONObject) -> Unit) {
        change(data)
        file.parentFile?.mkdirs()
        val tmp = File(file.path + ".tmp")
        tmp.writeText(data.toString(1))
        if (!tmp.renameTo(file)) { file.delete(); if (!tmp.renameTo(file)) throw java.io.IOException("Couldn't save ${file.name}") }
    }

    // ---------------------------------------------------------------- lists of ids (favorites, hidden, manual)
    @Synchronized
    fun list(name: String): List<String> {
        val a = data.optJSONArray(name) ?: return emptyList()
        return (0 until a.length()).mapNotNull { a.optString(it, null) }
    }
    fun listHas(name: String, id: String): Boolean = id in list(name)
    fun setIn(name: String, id: String, on: Boolean) = update { d ->
        val items = (d.optJSONArray(name)?.let { a -> (0 until a.length()).map { a.optString(it) } } ?: emptyList()).filter { it != id }.toMutableList()
        if (on) items.add(id)
        d.put(name, JSONArray(items))
    }

    // ---------------------------------------------------------------- play time
    @Synchronized
    fun stats(id: String): JSONObject = data.optJSONObject("stats")?.optJSONObject(id) ?: JSONObject()

    /** A game was played [minutes] minutes, just now. */
    fun addPlay(id: String, minutes: Long, now: Long) = update { d ->
        val stats = d.optJSONObject("stats") ?: JSONObject().also { d.put("stats", it) }
        val s = stats.optJSONObject(id) ?: JSONObject().also { stats.put(id, it) }
        s.put("minutes", s.optLong("minutes", 0) + minutes)
        s.put("lastPlayed", now)
    }

    // ---------------------------------------------------------------- collections (port of hearth-core collections.rs)
    @Synchronized
    fun collections(): List<JSONObject> {
        val a = data.optJSONArray("collections") ?: return emptyList()
        return (0 until a.length()).mapNotNull { a.optJSONObject(it) }
    }
    private fun saveCollections(cols: List<JSONObject>) = update { it.put("collections", JSONArray(cols)) }

    fun collectionCreate(name: String, now: Long = System.nanoTime()): JSONObject {
        val clean = name.trim().take(40)
        require(clean.isNotEmpty()) { "Give the collection a name" }
        val cols = collections()
        require(cols.size < MAX_COLLECTIONS) { "Hearth has room for $MAX_COLLECTIONS collections" }
        require(cols.none { it.optString("name").equals(clean, ignoreCase = true) }) { "There's already a collection called $clean" }
        val c = JSONObject().put("id", "c" + java.lang.Long.toHexString(now and 0xffffffffffffL)).put("name", clean).put("games", JSONArray())
        saveCollections(cols + c)
        return c
    }
    fun collectionRename(id: String, name: String) {
        val clean = name.trim().take(40)
        require(clean.isNotEmpty()) { "Give the collection a name" }
        val cols = collections()
        val c = cols.firstOrNull { it.optString("id") == id } ?: throw IllegalArgumentException("No such collection")
        c.put("name", clean)
        saveCollections(cols)
    }
    fun collectionRemove(id: String) = saveCollections(collections().filter { it.optString("id") != id })
    fun collectionSet(id: String, game: String, on: Boolean) {
        val cols = collections()
        val c = cols.firstOrNull { it.optString("id") == id } ?: throw IllegalArgumentException("No such collection")
        val a = c.optJSONArray("games") ?: JSONArray()
        val games = (0 until a.length()).map { a.optString(it) }.filter { it != game }.toMutableList()
        if (on) games.add(game)
        c.put("games", JSONArray(games))
        saveCollections(cols)
    }
    /** The ids of the collections [game] is in. */
    fun collectionsOf(game: String): List<String> = collections().filter { c ->
        val a = c.optJSONArray("games") ?: return@filter false
        (0 until a.length()).any { a.optString(it) == game }
    }.map { it.optString("id") }

    // ---------------------------------------------------------------- profile (port of hearth-core profile.rs)
    @Synchronized
    fun profile(defaultName: String): JSONObject {
        val p = data.optJSONObject("profile") ?: JSONObject()
        val name = p.optString("name", "").takeIf { it.isNotEmpty() && !p.isNull("name") } ?: defaultName
        return JSONObject()
            .put("name", name)
            .put("status", if (p.isNull("status")) JSONObject.NULL else p.opt("status") ?: JSONObject.NULL)
            .put("avatar", p.optString("avatar", "initials"))
            .put("accent", p.optString("accent", "#3fd0b8"))
            .put("background", p.optString("background", "default"))
            .put("customAvatar", JSONObject.NULL).put("customBackground", JSONObject.NULL)
            .put("storeAvatars", JSONObject().put("steam", JSONObject.NULL).put("xbox", JSONObject.NULL).put("gog", JSONObject.NULL))
    }

    /** Changes the profile with only good values (a name up to 32 letters, a #rrggbb accent…). */
    fun profileSet(c: JSONObject, defaultName: String): JSONObject {
        fun clip(k: String, n: Int): Any = c.optString(k, "").trim().take(n).ifEmpty { null } ?: JSONObject.NULL
        update { d ->
            val p = d.optJSONObject("profile") ?: JSONObject().also { d.put("profile", it) }
            if (c.has("name") && !c.isNull("name")) p.put("name", clip("name", 32))
            if (c.has("status") && !c.isNull("status")) p.put("status", clip("status", 60))
            c.optString("avatar", "").takeIf { it.isNotEmpty() }?.let { p.put("avatar", it) }
            c.optString("accent", "").takeIf { Regex("^#[0-9a-fA-F]{6}$").matches(it) }?.let { p.put("accent", it) }
            c.optString("background", "").takeIf { it.isNotEmpty() }?.let { p.put("background", it) }
        }
        return profile(defaultName)
    }

    companion object { const val MAX_COLLECTIONS = 30 }
}
