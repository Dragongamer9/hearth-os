package dev.hearthos.tv

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread

/** Theater's saved links (theater.json: {links: [...]}), added from the TV, a phone, a PC (Send to TV) or Android's Share. */
object Theater {
    fun list(): JSONArray = TvApp.app.theater.read().optJSONArray("links") ?: JSONArray()

    @Synchronized fun save(links: JSONArray) { TvApp.app.theater.merge(JSONObject().put("links", links)) }

    /** Saved first; with [play] (Send to TV, a phone's share) it plays as soon as it's saved. */
    fun add(ctx: Context, url: String, from: String, play: Boolean = false, by: String = "", title: String = "") {
        thread(isDaemon = true) {
            val m = Meta.read(url)
            // The page's own title wins; the one the PC sent is for pages that don't say.
            if (title.isNotBlank() && (m.optString("title").isBlank() || m.optString("title") == m.optString("site"))) m.put("title", title)
            m.put("id", System.currentTimeMillis().toString(36)).put("added", System.currentTimeMillis()).put("from", from).put("pct", 0)
            synchronized(this) {
                val cur = list()
                val out = JSONArray().put(m)
                for (i in 0 until cur.length()) if (cur.getJSONObject(i).optString("url") != m.optString("url")) out.put(cur.get(i))
                save(out)
            }
            Logs.add("theater: added ${m.optString("site")} from $from")
            val ev = JSONObject().put("type", "theaterAdded").put("item", m)
            if (play) SendToTv.deliver(ev.put("play", true).put("by", by)) else MainActivity.emit(ev)
        }
    }
}
