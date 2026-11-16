package dev.hearthos.tv

import android.app.SearchManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import org.json.JSONArray
import org.json.JSONObject

/** The TV's apps: what a home screen lists and opens. */
object Apps {
    private fun launchIntent(pm: PackageManager, pkg: String): Intent? =
        pm.getLeanbackLaunchIntentForPackage(pkg) ?: pm.getLaunchIntentForPackage(pkg)

    fun list(ctx: Context): JSONArray {
        val pm = ctx.packageManager
        val seen = LinkedHashMap<String, JSONObject>()
        for (cat in listOf(Intent.CATEGORY_LEANBACK_LAUNCHER, Intent.CATEGORY_LAUNCHER)) {
            val q = Intent(Intent.ACTION_MAIN).addCategory(cat)
            for (ri in pm.queryIntentActivities(q, 0)) {
                val pkg = ri.activityInfo.packageName
                if (pkg == ctx.packageName || seen.containsKey(pkg)) continue
                val ver = try { pm.getPackageInfo(pkg, 0).versionName ?: "" } catch (_: Exception) { "" }
                seen[pkg] = JSONObject().put("pkg", pkg).put("label", ri.loadLabel(pm).toString()).put("version", ver)
                    .put("tv", cat == Intent.CATEGORY_LEANBACK_LAUNCHER).put("art", AppIcons.kind(ctx, pkg))
            }
        }
        return JSONArray(seen.values.sortedBy { it.optString("label").lowercase() })
    }

    fun launch(ctx: Context, pkg: String): Boolean {
        val i = launchIntent(ctx.packageManager, pkg) ?: return false
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return try { ctx.startActivity(i); true } catch (e: Exception) { Logs.add("apps: couldn't open $pkg: $e"); false }
    }

    /** Opens [pkg] with [query] already searched when the app takes a search; otherwise just opens it. */
    fun search(ctx: Context, pkg: String, query: String): Boolean {
        val tries = listOf(
            Intent(Intent.ACTION_SEARCH).setPackage(pkg).putExtra(SearchManager.QUERY, query),
            Intent("android.search.action.GLOBAL_SEARCH").setPackage(pkg).putExtra(SearchManager.QUERY, query),
        )
        for (i in tries) {
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            if (i.resolveActivity(ctx.packageManager) != null) {
                try { ctx.startActivity(i); return true } catch (_: Exception) {}
            }
        }
        return launch(ctx, pkg)
    }

    fun installed(ctx: Context, pkg: String): Boolean = try { ctx.packageManager.getPackageInfo(pkg, PackageManager.MATCH_DISABLED_COMPONENTS); true } catch (_: Exception) { false }
}
