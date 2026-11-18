package dev.hearthos.android

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.widget.Toast
import kotlin.concurrent.thread

/**
 * "Hearth TV" in the phone's Share menu (docs/HEARTH-TV.md, Send to TV): share any video, or a video's link, from any
 * app and it plays on your Hearth TV (a link is saved in the TV's Theater too). Also opens the TV's
 * hearthtv://pair link (Theater › Add a link on the TV, scanned with the phone's camera), which adds that TV here.
 */
class ShareToTv : Activity() {
    private val dark = android.R.style.Theme_Material_Dialog_Alert

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val i = intent
        when {
            i?.action == Intent.ACTION_VIEW -> pair(i.dataString ?: "")
            i?.action == Intent.ACTION_SEND -> share(i)
            else -> finish()
        }
    }

    private fun say(t: String) = runOnUiThread { Toast.makeText(applicationContext, t, Toast.LENGTH_LONG).show() }

    private fun pair(link: String) {
        val tv = TvLink.parsePair(link) ?: run { say("That isn't a Hearth TV's link"); finish(); return }
        thread(isDaemon = true) {
            val hello = TvLink.hello(tv)
            if (hello == null) { say("Can't reach ${tv.name}: this phone needs to be on the same Wi-Fi as the TV"); finish(); return@thread }
            val named = tv.copy(name = TvLink.cleanName(hello.optString("name", tv.name)))
            try { TvLink.save(TvLink.store(filesDir), named); say("${named.name} is in this phone's Share menu: share any video to Hearth TV") }
            catch (e: Exception) { say("Couldn't save ${named.name} on this phone: ${e.message}") }
            finish()
        }
    }

    private fun share(i: Intent) {
        val tvs = try { TvLink.list(TvLink.store(filesDir)) } catch (_: Exception) { emptyList() }
        @Suppress("DEPRECATION")
        val stream: Uri? = if (Build.VERSION.SDK_INT >= 33) i.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java) else i.getParcelableExtra(Intent.EXTRA_STREAM)
        val link = TvLink.linkIn(i.getStringExtra(Intent.EXTRA_TEXT))
        if (stream == null && link == null) { say("Nothing to send: share a video or a video's link"); finish(); return }
        if (tvs.isEmpty()) {
            AlertDialog.Builder(this, dark).setTitle("Add your Hearth TV first")
                .setMessage("On the TV, open Theater and scan the code under Add a link with this phone's camera. On the page that opens, tap \"Add … to this phone's Share menu\". Then share again.")
                .setPositiveButton("OK") { _, _ -> finish() }.setOnCancelListener { finish() }.show()
            return
        }
        val go = { tv: TvLink.Tv -> if (stream != null) sendFile(tv, stream) else sendLink(tv, link!!) }
        if (tvs.size == 1) { go(tvs[0]); return }
        AlertDialog.Builder(this, dark).setTitle("Play on")
            .setItems(tvs.map { it.name }.toTypedArray()) { _, w -> go(tvs[w]) }
            .setOnCancelListener { finish() }.show()
    }

    private fun sendLink(tv: TvLink.Tv, url: String) {
        say("Sending it to ${tv.name}…")
        thread(isDaemon = true) {
            TvLink.send(tv, url, Build.MODEL).onSuccess { say("Playing on $it") }.onFailure { say(it.message ?: "Couldn't send it to the TV") }
            finish()
        }
    }

    private fun sendFile(tv: TvLink.Tv, uri: Uri) {
        val name = try { contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c -> if (c.moveToFirst()) c.getString(0) else null } } catch (_: Exception) { null } ?: "Video"
        val svc = Intent(this, TvShareService::class.java).setData(uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            .putExtra("tv", tv.json().toString()).putExtra("title", name)
        try { startForegroundService(svc); say("Sending $name to ${tv.name}…") } catch (e: Exception) { say("Couldn't start sending it: ${e.message}") }
        finish()
    }
}
