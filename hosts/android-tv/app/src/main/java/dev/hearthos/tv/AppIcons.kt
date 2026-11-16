package dev.hearthos.tv

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import android.webkit.WebResourceResponse
import androidx.webkit.WebViewAssetLoader
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File

/**
 * Each app's own picture from the TV, for its tile: the wide banner Android TV apps come with (what Google TV's home
 * shows), or the app's icon when it has no banner. Served to the page at /appicon/<package> and kept in the cache
 * folder until the app updates.
 */
object AppIcons {
    /** "banner" or "icon": which picture [pkg] has. */
    fun kind(ctx: Context, pkg: String): String = if (banner(ctx, pkg) != null) "banner" else "icon"

    private fun banner(ctx: Context, pkg: String): Drawable? {
        val pm = ctx.packageManager
        return try {
            pm.getLeanbackLaunchIntentForPackage(pkg)?.let { pm.getActivityBanner(it) }
                ?: pm.getApplicationBanner(pkg)
        } catch (_: Exception) { null }
    }

    private fun png(d: Drawable, maxW: Int): ByteArray {
        val w0 = d.intrinsicWidth.takeIf { it > 0 } ?: 320
        val h0 = d.intrinsicHeight.takeIf { it > 0 } ?: 180
        val k = minOf(1f, maxW.toFloat() / w0)
        val w = (w0 * k).toInt().coerceAtLeast(1); val h = (h0 * k).toInt().coerceAtLeast(1)
        val bmp = if (d is BitmapDrawable && d.bitmap != null && k == 1f) d.bitmap else {
            Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888).also { b -> val c = Canvas(b); d.setBounds(0, 0, w, h); d.draw(c) }
        }
        return ByteArrayOutputStream().also { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }.toByteArray()
    }

    fun bytes(ctx: Context, pkg: String): ByteArray? {
        if (!Regex("^[A-Za-z0-9._]{1,200}$").matches(pkg)) return null
        val ver = try { androidx.core.content.pm.PackageInfoCompat.getLongVersionCode(ctx.packageManager.getPackageInfo(pkg, 0)) } catch (_: Exception) { return null }
        val dir = File(ctx.cacheDir, "appicons").apply { mkdirs() }
        val f = File(dir, "$pkg-$ver.png")
        if (f.exists()) return f.readBytes()
        val d = banner(ctx, pkg) ?: try { ctx.packageManager.getApplicationIcon(pkg) } catch (_: Exception) { return null }
        val b = try { png(d, 640) } catch (e: Exception) { Logs.add("app picture $pkg: ${e.message}"); return null }
        dir.listFiles { x -> x.name.startsWith("$pkg-") }?.forEach { it.delete() }
        f.writeBytes(b)
        return b
    }

    class Handler(private val ctx: Context) : WebViewAssetLoader.PathHandler {
        override fun handle(path: String): WebResourceResponse? {
            val b = bytes(ctx, path.substringBefore('?').trim('/')) ?: return WebResourceResponse("image/png", null, 404, "Not found", emptyMap(), ByteArrayInputStream(ByteArray(0)))
            return WebResourceResponse("image/png", null, ByteArrayInputStream(b)).apply { responseHeaders = mapOf("Cache-Control" to "max-age=86400") }
        }
    }
}
