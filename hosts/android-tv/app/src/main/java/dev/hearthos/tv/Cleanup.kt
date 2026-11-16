package dev.hearthos.tv

import android.content.Context
import android.content.pm.PackageManager
import org.json.JSONArray
import org.json.JSONObject

/**
 * TV Cleanup: groups of the TV's own packages that are switched off (pm disable-user), never uninstalled, and can be
 * switched back on. Which packages are in each group is assets/tv/cleanup.json (checked against this TV's own list
 * when Hearth TV was installed, docs/HEARTH-TV.md). Settings › TV Cleanup shows each group's state; "Watch for things
 * turning back on" checks after the TV starts and after updates, switches them off again and says so.
 */
object Cleanup {
    private fun groups(ctx: Context): JSONObject =
        JSONObject(ctx.assets.open("tv/cleanup.json").bufferedReader().readText()).getJSONObject("groups")

    /** What the TV needs to work: never switched off, even when a list names it. */
    private fun never(ctx: Context): Set<String> {
        val a = JSONObject(ctx.assets.open("tv/cleanup.json").bufferedReader().readText()).optJSONArray("never") ?: JSONArray()
        return (0 until a.length()).map { a.getString(it) }.toSet()
    }

    /** Packages of [group] that are on this TV. */
    fun packages(ctx: Context, group: String): List<String> {
        val g = groups(ctx).optJSONObject(group) ?: return emptyList()
        val list = g.optJSONArray("packages") ?: JSONArray()
        val no = never(ctx)
        return (0 until list.length()).map { list.getString(it) }.filter { it !in no && Apps.installed(ctx, it) }
    }

    private fun enabled(ctx: Context, pkg: String): Boolean = try {
        when (ctx.packageManager.getApplicationEnabledSetting(pkg)) {
            PackageManager.COMPONENT_ENABLED_STATE_DISABLED, PackageManager.COMPONENT_ENABLED_STATE_DISABLED_USER -> false
            PackageManager.COMPONENT_ENABLED_STATE_DEFAULT -> ctx.packageManager.getApplicationInfo(pkg, 0).enabled
            else -> true
        }
    } catch (_: Exception) { false }

    /** {group: {on: bool, packages: [{pkg, on}]}} for every group, from Android itself (no adb needed). */
    fun status(ctx: Context): JSONObject {
        val out = JSONObject()
        val gs = groups(ctx)
        for (name in gs.keys()) {
            val pk = packages(ctx, name)
            val arr = JSONArray()
            var anyOn = false
            for (p in pk) { val on = enabled(ctx, p); if (on) anyOn = true; arr.put(JSONObject().put("pkg", p).put("on", on)) }
            out.put(name, JSONObject().put("on", anyOn).put("present", pk.isNotEmpty()).put("packages", arr))
        }
        return out
    }

    /** Switches [group] on or off through adb. Returns {ok, message}. */
    fun set(ctx: Context, group: String, on: Boolean): JSONObject {
        val pk = packages(ctx, group)
        if (pk.isEmpty()) return JSONObject().put("ok", true).put("message", "Nothing from this group is on this TV")
        val adb = TvApp.app.adb
        val failed = mutableListOf<String>()
        for (p in pk) {
            val r = adb.shell(if (on) "pm enable $p" else "pm disable-user --user 0 $p")
            if (!r.ok) {
                if (r.out.startsWith("unreachable")) return JSONObject().put("ok", false).put("adb", false)
                    .put("message", "Hearth TV can't reach the TV's adb: turn on network debugging (docs/HEARTH-TV.md)")
                failed += p
            }
        }
        Logs.add("cleanup: $group ${if (on) "on" else "off"}" + if (failed.isEmpty()) "" else " (failed: $failed)")
        return JSONObject().put("ok", failed.isEmpty()).put("message", if (failed.isEmpty()) "" else "Couldn't change: " + failed.joinToString())
    }

    /**
     * After the TV starts or an update: groups the person switched off that came back on are switched off again.
     * Returns the names of the groups it switched off again (the page and a notice tell the person).
     */
    fun watch(ctx: Context): List<String> {
        val s = TvApp.app.settings.read()
        val wanted = s.optJSONObject("cleanup") ?: return emptyList()
        if (wanted.optBoolean("watch", true) == false) return emptyList()
        val st = status(ctx)
        val fixed = mutableListOf<String>()
        for (g in wanted.keys()) {
            if (g == "watch") continue
            if (wanted.optBoolean(g, true) == false && st.optJSONObject(g)?.optBoolean("on") == true) {
                if (set(ctx, g, false).optBoolean("ok")) fixed += g
            }
        }
        if (fixed.isNotEmpty()) {
            TvApp.app.state.merge(JSONObject().put("cleanupEvent", JSONObject().put("at", System.currentTimeMillis()).put("groups", JSONArray(fixed))))
        }
        TvApp.app.state.merge(JSONObject().put("cleanupChecked", System.currentTimeMillis()))
        return fixed
    }
}
