package dev.hearthos.tv

import android.app.Application
import java.io.File

class TvApp : Application() {
    lateinit var settings: SettingsStore
    lateinit var theater: SettingsStore
    /** What Hearth TV notes for itself (last input, last checks): never mixed with the person's settings. */
    lateinit var state: SettingsStore
    lateinit var adb: Adb

    override fun onCreate() {
        super.onCreate()
        app = this
        Logs.init(filesDir)
        settings = SettingsStore(File(filesDir, "settings.json"))
        // Theater's saved links: their own file, kept the same way as the settings.
        theater = SettingsStore(File(filesDir, "theater.json"))
        state = SettingsStore(File(filesDir, "state.json"))
        adb = Adb(this)
        Logs.add("Hearth TV ${BuildConfig.VERSION_NAME} started on ${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL} (Android ${android.os.Build.VERSION.RELEASE})")
    }

    companion object {
        lateinit var app: TvApp
            private set
    }
}
