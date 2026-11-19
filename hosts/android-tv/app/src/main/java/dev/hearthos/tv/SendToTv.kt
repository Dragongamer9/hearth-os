package dev.hearthos.tv

import android.content.Context
import android.content.Intent
import org.json.JSONObject
import kotlin.concurrent.thread

/**
 * Send to TV (docs/HEARTH-TV.md): what a Hearth PC sends over Hearth Link (hosts/hearth-link/src/tv.rs) or a phone
 * shares (LocalServer's /send), played here. A link is saved in Theater and played; a video or music file plays in
 * Hearth TV's player from Hearth Link's own address on this TV; a show opens in My Media. Hearth TV comes to the
 * front for it, from whatever app is on.
 */
object SendToTv {
    private val queue = mutableListOf<JSONObject>()
    @Volatile private var polling = false

    /** Asks Hearth Link every 2 seconds for what was sent (tv.inbox). */
    fun start(ctx: Context) {
        if (polling || !Link.available(ctx)) return
        polling = true
        val app = ctx.applicationContext
        thread(name = "hearth-tv-sent", isDaemon = true) {
            while (true) {
                try {
                    val r = Link.call(app, "tv.inbox", JSONObject(), 10_000)
                    val items = r.optJSONObject("data")?.optJSONArray("items")
                    if (items != null) for (i in 0 until items.length()) handle(app, items.getJSONObject(i))
                } catch (e: Exception) { Logs.add("send to tv: $e") }
                try { Thread.sleep(2000) } catch (_: InterruptedException) {}
            }
        }
    }

    fun handle(ctx: Context, it: JSONObject) {
        val kind = it.optString("kind"); val from = it.optString("from").ifBlank { "your PC" }
        Logs.add("send to tv: a $kind from $from")
        when (kind) {
            "link" -> Theater.add(ctx, it.optString("url"), "pc", play = true, by = from, title = it.optString("title"))
            "file", "media" -> deliver(JSONObject().put("type", "sent").put("item", it))
            else -> return
        }
        front(ctx)
    }

    /** Hearth TV in front (it's the home screen; singleTask, so the page and its player stay as they are). */
    fun front(ctx: Context) {
        try { ctx.startActivity(Intent(ctx, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("sent", true)) }
        catch (e: Exception) { Logs.add("send to tv: couldn't come to the front: $e") }
    }

    /** To the page now, or as soon as Hearth TV is open again. */
    fun deliver(o: JSONObject) {
        synchronized(queue) { queue.add(o); while (queue.size > 20) queue.removeAt(0) }
        flush()
    }

    fun flush() {
        // Only while Hearth TV is on the screen: nothing plays behind another app.
        if (!MainActivity.front) return
        val a = MainActivity.current?.get() ?: return
        val l = synchronized(queue) { queue.toList().also { queue.clear() } }
        l.forEach { a.event(it) }
    }
}
