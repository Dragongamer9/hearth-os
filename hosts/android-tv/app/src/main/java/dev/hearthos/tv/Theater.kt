package dev.hearthos.tv

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread

/** Theater's saved links (theater.json: {links: [...]}), added from the TV, a phone, a PC or Android's Share. */
object Theater {
    fun list(): JSONArray = TvApp.app.theater.read().optJSONArray("links") ?: JSONArray()

    @Synchronized fun save(links: JSONArray) { TvApp.app.theater.merge(JSONObject().put("links", links)) }

    fun add(ctx: Context, url: String, from: String) {
        thread(isDaemon = true) {
            val m = Meta.read(url)
            m.put("id", System.currentTimeMillis().toString(36)).put("added", System.currentTimeMillis()).put("from", from).put("pct", 0)
            synchronized(this) {
                val cur = list()
                val out = JSONArray().put(m)
                for (i in 0 until cur.length()) if (cur.getJSONObject(i).optString("url") != m.optString("url")) out.put(cur.get(i))
                save(out)
            }
            Logs.add("theater: added ${m.optString("site")} from $from")
            MainActivity.emit(JSONObject().put("type", "theaterAdded").put("item", m))
        }
    }
}
