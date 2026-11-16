package dev.hearthos.tv

import android.annotation.SuppressLint
import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.speech.RecognizerIntent
import android.view.KeyEvent
import android.view.View
import android.view.WindowManager
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.webkit.WebViewAssetLoader
import org.json.JSONObject
import java.lang.ref.WeakReference

/** Hearth TV: the TV's home screen. One full-screen page (assets/tv/index.html) with Bridge as its way to Android. */
class MainActivity : Activity() {
    lateinit var web: WebView
    private var ready = false
    private val pending = mutableListOf<String>()

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        current = WeakReference(this)
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        val loader = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(this))
            .addPathHandler("/appicon/", AppIcons.Handler(this)).build()
        web = WebView(this)
        web.setBackgroundColor(0xFF171A21.toInt())
        web.isFocusable = true
        web.isFocusableInTouchMode = true
        with(web.settings) {
            javaScriptEnabled = true
            domStorageEnabled = true
            mediaPlaybackRequiresUserGesture = false
            mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
            // The page fits itself to the screen (tv.js fit()): no zooming by the WebView.
            loadWithOverviewMode = false
            useWideViewPort = false
            setSupportZoom(false)
            textZoom = 100
        }
        web.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? = loader.shouldInterceptRequest(request.url)
            override fun onPageFinished(view: WebView, url: String) {
                ready = true
                pending.forEach { web.evaluateJavascript(it, null) }
                pending.clear()
            }
        }
        web.webChromeClient = WebChromeClient()
        web.addJavascriptInterface(Bridge(this), "HearthTVNative")
        setContentView(web)
        web.loadUrl("https://appassets.androidplatform.net/assets/tv/index.html")
        web.requestFocus()
        HearthService.start(this)
        handleShare(intent)
    }

    fun js(code: String) = runOnUiThread { if (ready) web.evaluateJavascript(code, null) else pending += code }

    fun event(o: JSONObject) = js("window.hearthTV && hearthTV.onEvent(" + JSONObject.quote(o.toString()) + ")")

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (!handleShare(intent) && (intent.hasCategory(Intent.CATEGORY_HOME) || intent.getBooleanExtra("home", false))) event(JSONObject().put("type", "home"))
    }

    private fun handleShare(i: Intent?): Boolean {
        if (i?.action != Intent.ACTION_SEND) return false
        val text = i.getStringExtra(Intent.EXTRA_TEXT) ?: return false
        val url = Regex("""https?://\S+""").find(text)?.value ?: return false
        Theater.add(this, url, "share")
        event(JSONObject().put("type", "toast").put("text", "Saving the link in Theater…"))
        return true
    }

    override fun onResume() {
        Tailscale.onStart(this)
        super.onResume()
        TvApp.app.state.merge(JSONObject().put("onInput", JSONObject.NULL))
        event(JSONObject().put("type", "resume"))
    }

    // The remote's keys the page doesn't get by itself.
    override fun dispatchKeyEvent(e: KeyEvent): Boolean {
        if (e.action == KeyEvent.ACTION_DOWN) {
            val name = when (e.keyCode) {
                KeyEvent.KEYCODE_BACK -> "back"
                KeyEvent.KEYCODE_MENU, KeyEvent.KEYCODE_BUTTON_START -> "menu"
                KeyEvent.KEYCODE_SEARCH -> "search"
                KeyEvent.KEYCODE_TV_INPUT -> "inputs"
                KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_MEDIA_PLAY, KeyEvent.KEYCODE_MEDIA_PAUSE, KeyEvent.KEYCODE_BUTTON_X -> "playpause"
                KeyEvent.KEYCODE_MEDIA_FAST_FORWARD -> "ff"
                KeyEvent.KEYCODE_MEDIA_REWIND -> "rew"
                KeyEvent.KEYCODE_BUTTON_B -> "back"
                KeyEvent.KEYCODE_BUTTON_Y -> "y"
                else -> null
            }
            if (name != null) { event(JSONObject().put("type", "key").put("key", name)); return true }
        }
        if (e.action == KeyEvent.ACTION_UP && e.keyCode in listOf(KeyEvent.KEYCODE_BACK, KeyEvent.KEYCODE_MENU, KeyEvent.KEYCODE_SEARCH, KeyEvent.KEYCODE_TV_INPUT,
                KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_MEDIA_PLAY, KeyEvent.KEYCODE_MEDIA_PAUSE, KeyEvent.KEYCODE_MEDIA_FAST_FORWARD, KeyEvent.KEYCODE_MEDIA_REWIND,
                KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_BUTTON_X, KeyEvent.KEYCODE_BUTTON_Y, KeyEvent.KEYCODE_BUTTON_START)) return true
        // A controller's A is OK.
        if (e.keyCode == KeyEvent.KEYCODE_BUTTON_A) {
            return super.dispatchKeyEvent(KeyEvent(e.downTime, e.eventTime, e.action, KeyEvent.KEYCODE_DPAD_CENTER, e.repeatCount))
        }
        return super.dispatchKeyEvent(e)
    }

    @Deprecated("Activity result for voice search")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == VOICE) {
            val said = data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull() ?: ""
            event(JSONObject().put("type", "voice").put("text", said))
        }
    }

    override fun onDestroy() {
        if (current?.get() === this) current = null
        super.onDestroy()
    }

    companion object {
        const val VOICE = 7
        @Volatile var current: WeakReference<MainActivity>? = null
        fun emit(o: JSONObject) { current?.get()?.event(o) }
    }
}
