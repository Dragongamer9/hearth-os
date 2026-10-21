package dev.hearthos.android

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

/**
 * Keeps Hearth Linux's processes (PRoot and what runs in it, the Termux:X11 server) running while Hearth isn't in
 * front: without a foreground service Android freezes a background app's processes (cached-app freezer) or closes
 * them. LinuxEnv starts it when something starts and stops it when nothing runs anymore. It does nothing itself.
 */
class LinuxService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val nm = getSystemService(NotificationManager::class.java)
        nm?.createNotificationChannel(NotificationChannel(CHANNEL, "Hearth Linux", NotificationManager.IMPORTANCE_LOW).apply {
            description = "Shown while Linux programs run in Hearth"
        })
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        val n = NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_launcher_foreground)
            .setContentTitle("Hearth Linux is running")
            .setContentText("Linux programs keep running while Hearth is in the background")
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setContentIntent(open)
            .build()
        try {
            ServiceCompat.startForeground(this, NOTIFICATION_ID, n,
                if (Build.VERSION.SDK_INT >= 34) ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE else 0)
        } catch (e: Exception) {
            Logs.log("linux: foreground service refused: $e")
            stopSelf()
        }
        return START_NOT_STICKY
    }

    companion object {
        private const val CHANNEL = "linux"
        private const val NOTIFICATION_ID = 7
    }
}
