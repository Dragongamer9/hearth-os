package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RomsTest {
    private val snes get() = Roms.systemByKey("snes")!!
    private val psx get() = Roms.systemByKey("psx")!!

    @Test fun foldersNameTheirSystem() {
        assertEquals("snes", Roms.systemForDir("SNES")?.key)
        assertEquals("genesis", Roms.systemForDir("megadrive")?.key)
        assertEquals("psx", Roms.systemForDir("ps1")?.key)
        assertNull(Roms.systemForDir("music"))
    }

    @Test fun titlesAndThumbnails() {   // the same answers as hearth-core's emulators.rs
        assertEquals("The Legend of Zelda - A Link to the Past", Roms.cleanTitle("Legend of Zelda, The - A Link to the Past (USA) [!].sfc"))
        assertEquals("Sonic _ Knuckles (World)", Roms.thumbName("Sonic & Knuckles (World).md"))
        assertEquals("https://thumbnails.libretro.com/Nintendo%20-%20Super%20Nintendo%20Entertainment%20System/Named_Boxarts/Super%20Mario%20World%20%28USA%29.png",
            Roms.thumbUrl(snes, "Super Mario World (USA).sfc", "Named_Boxarts"))
    }

    @Test fun extensionsPerSystem() {
        val files = listOf("Super Mario World (USA).sfc", "readme.txt", "Chrono Trigger.smc", "box.png", "pack.zip")
        assertEquals(listOf("Super Mario World (USA).sfc", "Chrono Trigger.smc", "pack.zip"), Roms.pickGames(files, snes) { "" })
        assertEquals(emptyList<String>(), Roms.pickGames(listOf("game.gba"), snes) { "" })
    }

    @Test fun playlistsAndCueTracks() {
        val files = listOf("Final Fantasy VII (USA) (Disc 1).chd", "Final Fantasy VII (USA) (Disc 2).chd", "Final Fantasy VII (USA).m3u",
            "Crash Bandicoot (USA).cue", "Crash Bandicoot (USA).bin")
        val m3u = "Final Fantasy VII (USA) (Disc 1).chd\nFinal Fantasy VII (USA) (Disc 2).chd\n"
        assertEquals(listOf("Final Fantasy VII (USA).m3u", "Crash Bandicoot (USA).cue"), Roms.pickGames(files, psx) { if (it.endsWith(".m3u")) m3u else "" })
        // Genesis has .bin games of its own (and no .cue): those stay
        assertEquals(listOf("Sonic.bin"), Roms.pickGames(listOf("Sonic.bin"), Roms.systemByKey("genesis")!!) { "" })
    }

    @Test fun safDocumentsToPaths() {
        assertEquals("/storage/emulated/0/ROMs/snes/x.sfc", Roms.docIdToPath("primary:ROMs/snes/x.sfc"))
        assertEquals("/storage/1A2B-3C4D/roms/gba/y.gba", Roms.docIdToPath("1A2B-3C4D:roms/gba/y.gba"))
        assertEquals("/storage/emulated/0/Download/z.nes", Roms.docIdToPath("raw:/storage/emulated/0/Download/z.nes"))
        assertEquals("/storage/emulated/0", Roms.docIdToPath("primary:"))
        assertNull(Roms.docIdToPath("msf:1234"))
        assertNull(Roms.docIdToPath("no-colon"))
        assertEquals("Internal storage › ROMs › snes", Roms.folderLabel("primary:ROMs/snes"))
        assertEquals("SD card › Emulation", Roms.folderLabel("1A2B-3C4D:Emulation"))
    }

    @Test fun retroArchPaths() {
        assertEquals("/data/data/com.retroarch.aarch64/cores/snes9x_libretro_android.so", Roms.corePath("com.retroarch.aarch64", snes.cores[0]))
        assertEquals("/storage/emulated/0/Android/data/com.retroarch/files/retroarch.cfg", Roms.configPath("com.retroarch"))
        assertEquals("emu:snes:Collections/Metroid.sfc", Roms.gameId(snes, "Collections/Metroid.sfc"))
        // every system has a core and a box-art folder
        for (s in Roms.SYSTEMS) { assert(s.cores.isNotEmpty() && s.thumbs.isNotEmpty() && s.ext.isNotEmpty()) }
        assertEquals(Roms.SYSTEMS.size, Roms.SYSTEMS.map { it.key }.toSet().size)
    }
}
