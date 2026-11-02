package dev.hearthos.android

import java.net.URLEncoder

/**
 * A system in the ROMs folder: its folder names (ES-DE / EmuDeck names, the first is its key), the file types
 * that are games, and the RetroArch cores that play it (first = the one Hearth asks RetroArch for).
 * The same table as the Linux host's hearth-core/src/emulators.rs, with RetroArch for Android's core names.
 */
data class RomSystem(val dirs: List<String>, val name: String, val ext: Set<String>, val cores: List<String>, val thumbs: String) {
    val key: String get() = dirs[0]
}

/** ROMs: which files are games, their titles and box art, and how RetroArch for Android is started with one. */
object Roms {
    private fun sys(dirs: List<String>, name: String, ext: List<String>, cores: List<String>, thumbs: String) = RomSystem(dirs, name, ext.toSet(), cores, thumbs)

    val SYSTEMS: List<RomSystem> = listOf(
        sys(listOf("nes", "famicom"), "NES", listOf("nes", "fds", "unf", "zip", "7z"), listOf("mesen", "nestopia", "fceumm"), "Nintendo - Nintendo Entertainment System"),
        sys(listOf("snes", "sfc"), "Super Nintendo", listOf("sfc", "smc", "zip", "7z"), listOf("snes9x", "bsnes"), "Nintendo - Super Nintendo Entertainment System"),
        sys(listOf("n64"), "Nintendo 64", listOf("n64", "z64", "v64", "zip", "7z"), listOf("mupen64plus_next_gles3", "parallel_n64"), "Nintendo - Nintendo 64"),
        sys(listOf("gb"), "Game Boy", listOf("gb", "zip", "7z"), listOf("gambatte", "sameboy", "mgba"), "Nintendo - Game Boy"),
        sys(listOf("gbc"), "Game Boy Color", listOf("gbc", "zip", "7z"), listOf("gambatte", "sameboy", "mgba"), "Nintendo - Game Boy Color"),
        sys(listOf("gba"), "Game Boy Advance", listOf("gba", "zip", "7z"), listOf("mgba", "vbam", "gpsp"), "Nintendo - Game Boy Advance"),
        sys(listOf("nds"), "Nintendo DS", listOf("nds", "zip", "7z"), listOf("melondsds", "melonds", "desmume"), "Nintendo - Nintendo DS"),
        sys(listOf("genesis", "megadrive"), "Genesis / Mega Drive", listOf("md", "gen", "smd", "bin", "zip", "7z"), listOf("genesis_plus_gx", "picodrive"), "Sega - Mega Drive - Genesis"),
        sys(listOf("mastersystem"), "Master System", listOf("sms", "zip", "7z"), listOf("genesis_plus_gx", "picodrive"), "Sega - Master System - Mark III"),
        sys(listOf("gamegear"), "Game Gear", listOf("gg", "zip", "7z"), listOf("genesis_plus_gx"), "Sega - Game Gear"),
        sys(listOf("segacd", "megacd"), "Sega CD", listOf("chd", "cue", "m3u"), listOf("genesis_plus_gx", "picodrive"), "Sega - Mega-CD - Sega CD"),
        sys(listOf("saturn"), "Saturn", listOf("chd", "cue", "m3u"), listOf("yabasanshiro", "mednafen_saturn"), "Sega - Saturn"),
        sys(listOf("dreamcast"), "Dreamcast", listOf("chd", "gdi", "cdi", "m3u"), listOf("flycast"), "Sega - Dreamcast"),
        sys(listOf("psx", "ps1"), "PlayStation", listOf("chd", "cue", "pbp", "m3u"), listOf("swanstation", "pcsx_rearmed", "mednafen_psx_hw"), "Sony - PlayStation"),
        sys(listOf("psp"), "PSP", listOf("iso", "cso", "chd"), listOf("ppsspp"), "Sony - PlayStation Portable"),
        sys(listOf("pcengine", "tg16"), "TurboGrafx-16", listOf("pce", "zip", "7z"), listOf("mednafen_pce_fast", "mednafen_pce"), "NEC - PC Engine - TurboGrafx 16"),
        sys(listOf("atari2600"), "Atari 2600", listOf("a26", "bin", "zip"), listOf("stella2014", "stella"), "Atari - 2600"),
        sys(listOf("ngp", "ngpc"), "Neo Geo Pocket", listOf("ngp", "ngc", "zip"), listOf("mednafen_ngp"), "SNK - Neo Geo Pocket Color"),
    )

    /** Folders inside a system folder that hold pictures, saves and such, not games. */
    val SKIP_DIRS = setOf("media", "images", "videos", "manuals", "saves", "states")

    fun systemForDir(dir: String): RomSystem? = dir.lowercase().let { d -> SYSTEMS.firstOrNull { d in it.dirs } }
    fun systemByKey(key: String): RomSystem? = SYSTEMS.firstOrNull { it.key == key }

    fun ext(file: String): String = file.substringAfterLast('.', "").lowercase()
    private fun stem(file: String): String { val b = file.substringAfterLast('/'); val i = b.lastIndexOf('.'); return if (i > 0) b.substring(0, i) else b }

    /** "Legend of Zelda, The - A Link to the Past (USA) [!].sfc" -> "The Legend of Zelda - A Link to the Past" */
    fun cleanTitle(file: String): String {
        val base = file.substringAfterLast('/')
        var t = stem(file).replace(Regex("\\s*[(\\[][^)\\]]*[)\\]]"), "").replace('_', ' ')
        t = t.split(Regex("\\s+")).filter { it.isNotEmpty() }.joinToString(" ")
        val m = Regex("^(.*?), (The|A|An)(\\b.*)$").find(t)
        if (m != null) t = "${m.groupValues[2]} ${m.groupValues[1]}${m.groupValues[3]}"
        return t.ifEmpty { base }
    }

    /** libretro's thumbnail name for a ROM: &*\/:`<>?\| become _. */
    fun thumbName(file: String): String = stem(file).map { if ("&*/:`<>?\\|".contains(it)) '_' else it }.joinToString("")

    private fun enc(s: String): String = URLEncoder.encode(s, "UTF-8").replace("+", "%20")

    /** Box art ("Named_Boxarts") or a screenshot ("Named_Snaps") from the libretro thumbnail project. */
    fun thumbUrl(sys: RomSystem, file: String, kind: String): String =
        "https://thumbnails.libretro.com/${enc(sys.thumbs)}/$kind/${enc(thumbName(file))}.png"

    /**
     * Of the files in one folder, the games: the system's file types, without the discs a playlist (.m3u) lists
     * and without the .bin tracks of a .cue. [playlist] gives an .m3u file's text.
     */
    fun pickGames(files: List<String>, sys: RomSystem, playlist: (String) -> String): List<String> {
        val inPlaylist = HashSet<String>()
        for (f in files.filter { ext(it) == "m3u" }) {
            for (l in playlist(f).lines()) { val n = l.trim(); if (n.isNotEmpty()) inPlaylist.add(n.substringAfterLast('/').substringAfterLast('\\').lowercase()) }
        }
        val hasCue = files.any { ext(it) == "cue" }
        return files.filter { f ->
            val e = ext(f)
            e in sys.ext && f.lowercase() !in inPlaylist && !(e == "bin" && hasCue && "cue" in sys.ext)
        }
    }

    fun gameId(sys: RomSystem, rel: String): String = "emu:${sys.key}:$rel"

    /**
     * A file path for a Storage Access Framework document of Android's external storage provider, which RetroArch
     * needs (it opens ROMs by path, with its own storage permission): "primary:ROMs/snes/x.sfc" ->
     * "/storage/emulated/0/ROMs/snes/x.sfc", "1A2B-3C4D:roms/x" -> "/storage/1A2B-3C4D/roms/x". Null for documents
     * of other providers (those open with "Play with…" instead).
     */
    fun docIdToPath(docId: String): String? {
        if (docId.startsWith("raw:")) return docId.removePrefix("raw:").takeIf { it.startsWith("/") }
        val i = docId.indexOf(':')
        if (i <= 0) return null
        val vol = docId.substring(0, i); val rest = docId.substring(i + 1).trimStart('/')
        val base = when {
            vol == "primary" -> "/storage/emulated/0"
            vol == "home" -> "/storage/emulated/0/Documents"
            Regex("^[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}$").matches(vol) -> "/storage/$vol"
            else -> return null
        }
        return if (rest.isEmpty()) base else "$base/$rest"
    }

    /** "primary:ROMs" -> "Internal storage › ROMs" (how the chosen folder is shown in Settings). */
    fun folderLabel(docId: String): String {
        val i = docId.indexOf(':')
        if (i <= 0) return docId
        val vol = docId.substring(0, i); val rest = docId.substring(i + 1).trim('/').replace("/", " › ")
        val where = when (vol) { "primary" -> "Internal storage"; "home" -> "Documents"; else -> "SD card" }
        return if (rest.isEmpty()) where else "$where › $rest"
    }

    // ---------------------------------------------------------------- RetroArch for Android
    /** RetroArch's builds, in the order Hearth prefers them. */
    val RETROARCH = listOf("com.retroarch.aarch64", "com.retroarch", "com.retroarch.ra32")
    /** RetroArch's documented "start this game" activity (the same in every build). */
    const val RETROARCH_ACTIVITY = "com.retroarch.browser.retroactivity.RetroActivityFuture"

    fun coreFile(core: String) = "${core}_libretro_android.so"
    /** Where a RetroArch build keeps the cores it downloaded (Online Updater › Core Downloader). */
    fun corePath(pkg: String, core: String) = "/data/data/$pkg/cores/${coreFile(core)}"
    /** RetroArch's own settings file (RetroArch 1.9+ keeps it in its app folder on the shared storage). */
    fun configPath(pkg: String) = "/storage/emulated/0/Android/data/$pkg/files/retroarch.cfg"
}
