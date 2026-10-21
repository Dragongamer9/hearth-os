package dev.hearthos.android

/**
 * Which installed apps are games in Hearth's library (source "android"): what Android itself calls a game
 * (ApplicationInfo.CATEGORY_GAME, or the older FLAG_IS_GAME), plus emulators, PC-game launchers and game
 * streaming apps, which Android files under other categories. Everything else is an app ("Show all apps").
 */
object GameDetect {
    /** ApplicationInfo.CATEGORY_GAME (Android 8.0+). Kept here so the rule can be tested without Android. */
    const val CATEGORY_GAME = 0

    /** Known emulators, launchers and streaming apps, by package name. */
    val KNOWN: Map<String, String> = linkedMapOf(
        // RetroArch (Play Store, 64-bit and 32-bit builds)
        "com.retroarch" to "RetroArch", "com.retroarch.aarch64" to "RetroArch", "com.retroarch.ra32" to "RetroArch",
        "com.swordfish.lemuroid" to "Lemuroid",
        // Nintendo
        "org.dolphinemu.dolphinemu" to "Dolphin", "org.dolphinemu.mmjr" to "Dolphin MMJR", "org.mm.jr" to "Dolphin MMJR",
        "org.citra.citra_emu" to "Citra", "org.citra.citra_emu.canary" to "Citra Canary",
        "io.github.lime3ds.android" to "Lime3DS", "org.azahar_emu.azahar" to "Azahar", "com.panda3ds.pandroid" to "Panda3DS",
        "org.yuzu.yuzu_emu" to "yuzu", "org.yuzu.yuzu_emu.ea" to "yuzu Early Access",
        "dev.eden.eden_emulator" to "Eden", "org.citron.citron_emu" to "Citron",
        "org.sudachi.sudachi_emu" to "Sudachi", "dev.suyu.suyu_emu" to "suyu", "skyline.emu" to "Skyline",
        "me.magnum.melonds" to "melonDS", "com.dsemu.drastic" to "DraStic",
        "org.mupen64plusae.v3.fzurita" to "Mupen64Plus FZ", "org.mupen64plusae.v3.fzurita.pro" to "Mupen64Plus FZ Pro",
        "com.fastemulator.gba" to "My Boy!", "com.fastemulator.gbc" to "My OldBoy!",
        // Sony
        "org.ppsspp.ppsspp" to "PPSSPP", "org.ppsspp.ppssppgold" to "PPSSPP Gold",
        "xyz.aethersx2.android" to "AetherSX2 / NetherSX2",
        "com.github.stenzek.duckstation" to "DuckStation", "com.epsxe.ePSXe" to "ePSXe",
        "org.vita3k.emulator" to "Vita3K",
        // Sega and others
        "com.flycast.emulator" to "Flycast", "io.recompiled.redream" to "Redream", "org.scummvm.scummvm" to "ScummVM",
        // PC games on Android
        "com.winlator" to "Winlator", "com.xiaoji.egggame" to "GameHub",
        "com.droiddeck.launcher" to "DroidDeck",           // github.com/Droid-Deck/DroidDeck (app/build.gradle)
        // Frontends
        "org.es_de.frontend" to "ES-DE", "com.magneticchen.daijishou" to "Daijisho",
        // Streaming and cloud gaming
        "com.valvesoftware.steamlink" to "Steam Link", "com.limelight" to "Moonlight",
        "com.playstation.remoteplay" to "PS Remote Play",
        "com.microsoft.xboxone.smartglass" to "Xbox", "com.gamepass" to "Xbox Game Pass",
        "com.nvidia.geforcenow" to "GeForce NOW",
    )

    /** Families whose builds and forks keep the same start: RetroArch, Dolphin, PPSSPP, Citra, yuzu, Winlator. */
    val PREFIXES = listOf("com.retroarch.", "org.dolphinemu.", "org.ppsspp.", "org.citra.", "org.yuzu.", "com.winlator.")

    fun isKnown(pkg: String): Boolean = pkg in KNOWN || PREFIXES.any { pkg.startsWith(it) }

    /** [category] is ApplicationInfo.category, [flagIsGame] whether ApplicationInfo.FLAG_IS_GAME is set. */
    fun isGame(pkg: String, category: Int, flagIsGame: Boolean): Boolean =
        flagIsGame || category == CATEGORY_GAME || isKnown(pkg)
}
