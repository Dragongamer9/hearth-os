package dev.hearthos.tv

import android.content.Context
import android.content.Intent
import android.media.tv.TvContract
import android.media.tv.TvInputInfo
import android.media.tv.TvInputManager
import org.json.JSONArray
import org.json.JSONObject

/** The TV's inputs (HDMI, the antenna, AV) from Android TV's input framework, and switching to one. */
object Inputs {
    private fun tim(ctx: Context) = ctx.getSystemService(Context.TV_INPUT_SERVICE) as? TvInputManager

    fun list(ctx: Context): JSONArray {
        val out = JSONArray()
        val m = tim(ctx) ?: return out
        val infos = try { m.tvInputList } catch (e: Exception) { Logs.add("inputs: list failed: $e"); emptyList<TvInputInfo>() }
        for (info in infos) {
            // Hardware inputs only: an app's own channels (a streaming app's "TV input") aren't inputs on the TV.
            if (info.isPassthroughInput || info.type == TvInputInfo.TYPE_TUNER) {
                val state = try { m.getInputState(info.id) } catch (_: Exception) { TvInputManager.INPUT_STATE_CONNECTED }
                val kind = when (info.type) {
                    TvInputInfo.TYPE_HDMI -> "hdmi"
                    TvInputInfo.TYPE_TUNER -> "tuner"
                    TvInputInfo.TYPE_COMPOSITE, TvInputInfo.TYPE_SVIDEO, TvInputInfo.TYPE_SCART, TvInputInfo.TYPE_COMPONENT, TvInputInfo.TYPE_VGA -> "av"
                    else -> "other"
                }
                val label = try { info.loadLabel(ctx).toString() } catch (_: Exception) { info.id }
                val custom = try { info.loadCustomLabel(ctx)?.toString() } catch (_: Exception) { null }
                out.put(JSONObject().put("id", info.id).put("label", label).put("custom", custom ?: "").put("kind", kind)
                    .put("hidden", try { info.isHidden(ctx) } catch (_: Exception) { false })
                    .put("state", when (state) {
                        TvInputManager.INPUT_STATE_CONNECTED -> "signal"
                        TvInputManager.INPUT_STATE_CONNECTED_STANDBY -> "standby"
                        else -> "none"
                    }))
            }
        }
        return out
    }

    fun switchTo(ctx: Context, id: String): Boolean {
        val m = tim(ctx)
        val info = m?.tvInputList?.firstOrNull { it.id == id }
        val uri = if (info != null && info.type == TvInputInfo.TYPE_TUNER) TvContract.buildChannelsUriForInput(id) else TvContract.buildChannelUriForPassthroughInput(id)
        val i = Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return try {
            ctx.startActivity(i)
            TvApp.app.state.merge(JSONObject().put("lastInput", id))
            Logs.add("inputs: switched to $id")
            true
        } catch (e: Exception) {
            Logs.add("inputs: couldn't switch to $id: $e")
            // The TV's own Live TV app as a fallback for the antenna.
            liveTv(ctx)
        }
    }

    /** The TV's Live TV app (antenna channels): TCL's, or Android's own. */
    fun liveTv(ctx: Context): Boolean {
        val i = Intent(Intent.ACTION_VIEW, TvContract.Channels.CONTENT_URI).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return try { ctx.startActivity(i); true } catch (_: Exception) {
            for (p in listOf("com.tcl.tvinput", "com.tcl.tv", "com.google.android.tv", "com.android.tv")) if (Apps.launch(ctx, p)) return true
            false
        }
    }
}
