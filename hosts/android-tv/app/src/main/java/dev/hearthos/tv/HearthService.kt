package dev.hearthos.tv

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.tv.TvInputManager
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread

/**
 * Hearth TV's companion while the TV is on: Hearth Link, Theater's add-a-link page, TV Cleanup's check after the TV
 * starts, and the TV inputs watcher ("Come back to Hearth TV when that device turns off").
 */
class HearthService : Service() {
    private var inputs: TvInputManager? = null
    private val cb = object : TvInputManager.TvInputCallback() {
        override fun onInputStateChanged(inputId: String, state: Int) {
            val s = TvApp.app.settings.read()
            val inputsCfg = s.optJSONObject("inputs") ?: JSONObject()
            val current = TvApp.app.state.read().optString("onInput")
            if (inputId == current && state == TvInputManager.INPUT_STATE_DISCONNECTED && inputsCfg.optBoolean("backHome", true)) {
                Logs.add("inputs: $inputId turned off, back to Hearth TV")
                TvApp.app.state.merge(JSONObject().put("onInput", JSONObject.NULL))
                startActivity(Intent(this@HearthService, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            }
            MainActivity.emit(JSONObject().put("type", "inputs"))
        }
        override fun onInputAdded(inputId: String) { MainActivity.emit(JSONObject().put("type", "inputs")) }
        override fun onInputRemoved(inputId: String) { MainActivity.emit(JSONObject().put("type", "inputs")) }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel("hearth", "Hearth TV", NotificationManager.IMPORTANCE_MIN))
        val n = Notification.Builder(this, "hearth").setContentTitle("Hearth TV").setContentText("Hearth Link and TV Cleanup")
            .setSmallIcon(android.R.drawable.star_on).build()
        if (Build.VERSION.SDK_INT >= 34) startForeground(1, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE) else startForeground(1, n)
        Link.start(this)
        LocalServer.onAdd = { url -> Theater.add(this, url, "phone") }
        LocalServer.start()
        inputs = getSystemService(Context.TV_INPUT_SERVICE) as? TvInputManager
        try { inputs?.registerCallback(cb, Handler(Looper.getMainLooper())) } catch (e: Exception) { Logs.add("inputs: no watcher: $e") }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.getBooleanExtra("boot", false) == true) thread(isDaemon = true) {
            // TV Cleanup: anything an update turned back on is switched off again, and the person is told.
            val fixed = try { Cleanup.watch(this) } catch (e: Exception) { Logs.add("cleanup watch: $e"); emptyList() }
            if (fixed.isNotEmpty()) MainActivity.emit(JSONObject().put("type", "cleanupFixed").put("groups", JSONArray(fixed)))
        }
        return START_STICKY
    }

    override fun onDestroy() {
        try { inputs?.unregisterCallback(cb) } catch (_: Exception) {}
        super.onDestroy()
    }

    companion object {
        fun start(ctx: Context, boot: Boolean = false) {
            val i = Intent(ctx, HearthService::class.java).putExtra("boot", boot)
            try { ctx.startForegroundService(i) } catch (e: Exception) { Logs.add("service: couldn't start: $e") }
        }
    }
}
