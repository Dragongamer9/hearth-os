package dev.hearthos.tv

import android.content.ComponentName
import android.content.Context
import android.content.Intent

/**
 * Tailscale, when it's on the TV: My Media reaches the PC through it. TCL's TVs block Tailscale from starting by
 * itself after a restart, so Hearth TV (which the TV always opens) asks it to connect when Hearth TV starts, using
 * Tailscale's own "connect" request. Switched off in Settings › My Media.
 */
object Tailscale {
    const val PKG = "com.tailscale.ipn"

    fun connect(ctx: Context) {
        if (!Apps.installed(ctx, PKG)) return
        try {
            ctx.sendBroadcast(Intent("com.tailscale.ipn.CONNECT_VPN").setComponent(ComponentName(PKG, "com.tailscale.ipn.IPNReceiver")))
            Logs.add("tailscale: asked to connect")
        } catch (e: Exception) { Logs.add("tailscale: ${e.message}") }
    }

    /** Once each time Hearth TV starts, unless switched off (settings media.tailscale). */
    private var done = false
    fun onStart(ctx: Context) {
        if (done) return
        done = true
        val on = try { TvApp.app.settings.read().optJSONObject("media")?.optBoolean("tailscale", true) ?: true } catch (_: Exception) { true }
        if (on) connect(ctx)
    }
}
