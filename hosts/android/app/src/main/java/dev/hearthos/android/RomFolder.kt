package dev.hearthos.android

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.Settings

/** A ROM found in the ROMs folder. [rel]: its path inside the system's folder ("Collections/Metroid.gba"). */
data class Rom(val id: String, val title: String, val sys: RomSystem, val docId: String, val rel: String, val name: String, val modified: Long, val size: Long)

/**
 * The ROMs folder, chosen with Android's folder picker (Storage Access Framework, with a lasting read permission):
 * one folder per system inside it, named like ES-DE and EmuDeck do (snes, gba, psx…), up to two levels deeper.
 * A folder with a "roms" folder inside (EmuDeck's Emulation folder) works too.
 */
class RomFolder(private val ctx: Context) {
    private data class Doc(val id: String, val name: String, val dir: Boolean, val modified: Long, val size: Long)

    private fun children(tree: Uri, docId: String): List<Doc> {
        val uri = DocumentsContract.buildChildDocumentsUriUsingTree(tree, docId)
        val cols = arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE, DocumentsContract.Document.COLUMN_LAST_MODIFIED, DocumentsContract.Document.COLUMN_SIZE)
        val out = ArrayList<Doc>()
        try {
            ctx.contentResolver.query(uri, cols, null, null, null)?.use { c ->
                while (c.moveToNext()) {
                    val id = c.getString(0) ?: continue
                    out.add(Doc(id, c.getString(1) ?: id.substringAfterLast('/'), c.getString(2) == DocumentsContract.Document.MIME_TYPE_DIR,
                        if (c.isNull(3)) 0L else c.getLong(3), if (c.isNull(4)) 0L else c.getLong(4)))
                }
            }
        } catch (e: Exception) { Logs.log("roms: can't list $docId: $e") }
        return out
    }

    private fun readText(tree: Uri, docId: String): String = try {
        ctx.contentResolver.openInputStream(DocumentsContract.buildDocumentUriUsingTree(tree, docId))?.use { s ->
            String(s.readBytes().let { if (it.size > 65536) it.copyOf(65536) else it })
        } ?: ""
    } catch (e: Exception) { "" }

    /** Every game in the folder, by system then title. */
    fun scan(tree: Uri): List<Rom> {
        val rootId = DocumentsContract.getTreeDocumentId(tree)
        var top = children(tree, rootId)
        if (top.none { it.dir && Roms.systemForDir(it.name) != null }) {
            top.firstOrNull { it.dir && it.name.equals("roms", ignoreCase = true) }?.let { top = children(tree, it.id) }
        }
        val out = ArrayList<Rom>()
        val seen = HashSet<String>()
        for (d in top.filter { it.dir }.sortedBy { it.name.lowercase() }) {
            val sys = Roms.systemForDir(d.name) ?: continue
            for ((doc, rel) in walk(tree, d.id, "", sys, 2)) {
                val id = Roms.gameId(sys, rel)
                if (!seen.add(id)) continue
                out.add(Rom(id, Roms.cleanTitle(doc.name), sys, doc.id, rel, doc.name, doc.modified, doc.size))
            }
        }
        return out
    }

    private fun walk(tree: Uri, dirId: String, prefix: String, sys: RomSystem, depth: Int): List<Pair<Doc, String>> {
        val kids = children(tree, dirId)
        val files = kids.filter { !it.dir }
        val byName = files.associateBy { it.name }
        val games = Roms.pickGames(files.map { it.name }, sys) { m3u -> byName[m3u]?.let { readText(tree, it.id) } ?: "" }
        val out = games.sorted().mapNotNull { n -> byName[n]?.let { it to prefix + n } }.toMutableList()
        if (depth > 0) for (d in kids.filter { it.dir && !it.name.startsWith(".") && it.name.lowercase() !in Roms.SKIP_DIRS }) {
            out.addAll(walk(tree, d.id, prefix + d.name + "/", sys, depth - 1))
        }
        return out
    }

    /** The RetroArch build installed here (null: none). */
    fun retroArch(apps: Apps): String? = Roms.RETROARCH.firstOrNull { apps.installed(it) }

    /**
     * How a ROM starts: in RetroArch with its system's core when RetroArch is installed and the ROM has a file
     * path; otherwise Android's "open with" for the file, so any emulator that opens files can play it.
     * Returns the intent and whether it's RetroArch.
     */
    fun launchIntent(tree: Uri, rom: Rom, apps: Apps): Pair<Intent, Boolean> {
        val ra = retroArch(apps)
        val path = Roms.docIdToPath(rom.docId)
        if (ra != null && path != null) {
            val i = Intent(Intent.ACTION_MAIN).setComponent(ComponentName(ra, Roms.RETROARCH_ACTIVITY))
                .putExtra("ROM", path)
                .putExtra("LIBRETRO", Roms.corePath(ra, rom.sys.cores[0]))
                .putExtra("CONFIGFILE", Roms.configPath(ra))
                .putExtra("IME", Settings.Secure.getString(ctx.contentResolver, Settings.Secure.DEFAULT_INPUT_METHOD) ?: "")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
            return i to true
        }
        val doc = DocumentsContract.buildDocumentUriUsingTree(tree, rom.docId)
        val view = Intent(Intent.ACTION_VIEW).setDataAndType(doc, "application/octet-stream")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        return Intent.createChooser(view, "Play ${rom.title} with").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) to false
    }
}
