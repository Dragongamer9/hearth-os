package dev.hearthos.android

import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.content.pm.ResolveInfo
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RadialGradient
import android.graphics.Shader
import android.os.Build
import java.io.File

/** An app with a launcher icon. [game]: Android calls it a game, or it's a known emulator or game launcher. */
data class AppEntry(val pkg: String, val title: String, val game: Boolean, val added: Long, val versionCode: Long)

/** The apps on this device that can be started (phone and TV launchers), and how to start one. */
class Apps(private val ctx: Context) {
    private val pm: PackageManager get() = ctx.packageManager

    @Suppress("DEPRECATION")
    private fun query(intent: Intent): List<ResolveInfo> =
        if (Build.VERSION.SDK_INT >= 33) pm.queryIntentActivities(intent, PackageManager.ResolveInfoFlags.of(0L))
        else pm.queryIntentActivities(intent, 0)

    @Suppress("DEPRECATION")
    private fun info(pkg: String): PackageInfo? = try {
        if (Build.VERSION.SDK_INT >= 33) pm.getPackageInfo(pkg, PackageManager.PackageInfoFlags.of(0L)) else pm.getPackageInfo(pkg, 0)
    } catch (e: PackageManager.NameNotFoundException) { null }

    @Suppress("DEPRECATION")
    fun versionCode(pkg: String): Long { val i = info(pkg) ?: return 0; return if (Build.VERSION.SDK_INT >= 28) i.longVersionCode else i.versionCode.toLong() }

    /** Every startable app but Hearth itself, by title. */
    @Suppress("DEPRECATION")
    fun scan(): List<AppEntry> {
        val seen = LinkedHashMap<String, AppEntry>()
        for (cat in listOf(Intent.CATEGORY_LAUNCHER, Intent.CATEGORY_LEANBACK_LAUNCHER)) {
            for (ri in query(Intent(Intent.ACTION_MAIN).addCategory(cat))) {
                val ai = ri.activityInfo?.applicationInfo ?: continue
                val pkg = ai.packageName
                if (pkg in seen || pkg.startsWith(SELF)) continue
                val pi = info(pkg)
                val game = GameDetect.isGame(pkg, ai.category, (ai.flags and ApplicationInfo.FLAG_IS_GAME) != 0)
                val ver = if (pi == null) 0L else if (Build.VERSION.SDK_INT >= 28) pi.longVersionCode else pi.versionCode.toLong()
                seen[pkg] = AppEntry(pkg, ai.loadLabel(pm).toString().ifBlank { pkg }, game, pi?.firstInstallTime ?: 0L, ver)
            }
        }
        return seen.values.sortedBy { it.title.lowercase() }
    }

    fun installed(pkg: String): Boolean = info(pkg) != null

    /** The intent that starts an app the way its launcher icon does (phone first, then TV). */
    fun launchIntent(pkg: String): Intent? = (pm.getLaunchIntentForPackage(pkg) ?: pm.getLeanbackLaunchIntentForPackage(pkg))
        ?.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)

    companion object { const val SELF = "dev.hearthos.android" }
}

/**
 * Library art for Android apps, made from their icons: a cover (300 x 450) and a wide picture (460 x 215), the
 * icon on a background in its own colour. Made the first time the page asks for one (art/<package>/<version>/
 * cover.png through the asset loader), kept in the app's cache, one at a time so memory stays small.
 */
class IconArt(private val ctx: Context) {
    private val dir = File(ctx.cacheDir, "art")
    private val lock = Any()

    fun file(pkg: String, version: Long, kind: String): File? {
        if (!Regex("^[A-Za-z0-9._]+$").matches(pkg) || kind !in KINDS) return null
        val f = File(dir, "${pkg}_${version}_$kind.png")
        if (f.exists()) return f
        synchronized(lock) {
            if (!f.exists()) try { render(pkg, kind, f) } catch (e: Throwable) { Logs.log("art: $pkg $kind: $e") }
        }
        return f.takeIf { it.exists() }
    }

    private fun render(pkg: String, kind: String, out: File) {
        val icon = try { ctx.packageManager.getApplicationIcon(pkg) } catch (e: PackageManager.NameNotFoundException) { return }
        val cover = kind == "cover"
        val w = if (cover) 300 else 460
        val h = if (cover) 450 else 215
        val size = if (cover) 168 else 132
        val ib = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
        icon.setBounds(0, 0, size, size)
        icon.draw(Canvas(ib))
        val base = averageColor(ib)
        val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        val c = Canvas(bmp)
        val p = Paint(Paint.ANTI_ALIAS_FLAG)
        p.shader = LinearGradient(0f, 0f, 0f, h.toFloat(), shade(base, 0.42f), shade(base, 0.12f), Shader.TileMode.CLAMP)
        c.drawRect(0f, 0f, w.toFloat(), h.toFloat(), p)
        val cx = w / 2f; val cy = if (cover) h * 0.44f else h / 2f
        p.shader = RadialGradient(cx, cy, size * 1.1f, (base and 0x00ffffff) or 0x66000000, base and 0x00ffffff, Shader.TileMode.CLAMP)
        c.drawCircle(cx, cy, size * 1.1f, p)
        c.drawBitmap(ib, cx - size / 2f, cy - size / 2f, Paint(Paint.FILTER_BITMAP_FLAG))
        dir.mkdirs()
        val tmp = File(out.path + ".tmp")
        tmp.outputStream().use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }
        tmp.renameTo(out)
        ib.recycle(); bmp.recycle()
    }

    /** The icon's colour: the average of its visible pixels (Hearth's teal for an empty icon). */
    private fun averageColor(b: Bitmap): Int {
        val px = IntArray(b.width * b.height)
        b.getPixels(px, 0, b.width, 0, 0, b.width, b.height)
        var r = 0L; var g = 0L; var bl = 0L; var n = 0L
        for (i in px.indices step 3) { val c = px[i]; if (Color.alpha(c) < 128) continue; r += Color.red(c); g += Color.green(c); bl += Color.blue(c); n++ }
        return if (n == 0L) Color.rgb(0x3f, 0xd0, 0xb8) else Color.rgb((r / n).toInt(), (g / n).toInt(), (bl / n).toInt())
    }

    /** The colour, darker: [value] is its brightness (0..1), with the saturation kept moderate. */
    private fun shade(color: Int, value: Float): Int {
        val hsv = FloatArray(3)
        Color.colorToHSV(color, hsv)
        hsv[1] = hsv[1].coerceAtMost(0.75f); hsv[2] = value
        return Color.HSVToColor(hsv)
    }

    fun clear() { dir.listFiles()?.forEach { it.delete() } }

    companion object { val KINDS = setOf("cover", "wide") }
}
