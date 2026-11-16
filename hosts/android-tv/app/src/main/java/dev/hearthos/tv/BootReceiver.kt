package dev.hearthos.tv

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** After the TV starts (or Hearth TV is updated): the companion service, TV Cleanup's check, the start-on choice. */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        Logs.add("boot: ${intent.action}")
        HearthService.start(ctx, boot = true)
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            val inputs = TvApp.app.settings.read().optJSONObject("inputs")
            val last = TvApp.app.state.read().optString("lastInput")
            if (inputs?.optString("startOn") == "last" && last.isNotEmpty()) Inputs.switchTo(ctx, last)
        }
    }
}
