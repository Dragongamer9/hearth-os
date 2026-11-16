package dev.hearthos.tv

import android.annotation.SuppressLint
import android.app.Activity
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout

/**
 * Theater: a saved link's page, full screen with nothing of Hearth's around it. The remote: OK play/pause, ◀ ▶ skip
 * 10 s, ▲ ▼ scroll, Back leaves. The page's own full screen (a video's) fills the TV.
 */
class ViewerActivity : Activity() {
    private lateinit var web: WebView
    private lateinit var root: FrameLayout
    private var full: View? = null

    private fun video(js: String) = web.evaluateJavascript(
        "(function(){var v=[].slice.call(document.querySelectorAll('video')).sort(function(a,b){return b.clientWidth*b.clientHeight-a.clientWidth*a.clientHeight})[0];if(!v)return false;$js;return true})()", null)

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        root = FrameLayout(this)
        web = WebView(this)
        with(web.settings) {
            javaScriptEnabled = true; domStorageEnabled = true; mediaPlaybackRequiresUserGesture = false
            userAgentString = Net.UA.replace("Android 12; Hearth TV", "Android 12")
        }
        web.webViewClient = WebViewClient()
        web.webChromeClient = object : WebChromeClient() {
            override fun onShowCustomView(view: View, callback: CustomViewCallback) { full = view; root.addView(view) }
            override fun onHideCustomView() { full?.let { root.removeView(it) }; full = null }
        }
        root.addView(web)
        setContentView(root)
        web.loadUrl(intent.getStringExtra("url") ?: "about:blank")
        web.requestFocus()
    }

    override fun dispatchKeyEvent(e: KeyEvent): Boolean {
        if (e.action != KeyEvent.ACTION_DOWN) return super.dispatchKeyEvent(e)
        when (e.keyCode) {
            KeyEvent.KEYCODE_BACK, KeyEvent.KEYCODE_BUTTON_B -> {
                if (full != null) { web.evaluateJavascript("document.exitFullscreen&&document.exitFullscreen()", null); return true }
                if (web.canGoBack()) web.goBack() else finish(); return true
            }
            KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_BUTTON_X -> { video("v.paused?v.play():v.pause()"); return true }
            KeyEvent.KEYCODE_DPAD_LEFT, KeyEvent.KEYCODE_MEDIA_REWIND -> { video("v.currentTime=Math.max(0,v.currentTime-10)"); return true }
            KeyEvent.KEYCODE_DPAD_RIGHT, KeyEvent.KEYCODE_MEDIA_FAST_FORWARD -> { video("v.currentTime=v.currentTime+10"); return true }
            KeyEvent.KEYCODE_DPAD_UP -> { web.pageUp(false); return true }
            KeyEvent.KEYCODE_DPAD_DOWN -> { web.pageDown(false); return true }
            KeyEvent.KEYCODE_BUTTON_Y -> { video("(v.requestFullscreen||v.webkitRequestFullscreen).call(v)"); return true }
        }
        return super.dispatchKeyEvent(e)
    }

    override fun onDestroy() { web.destroy(); super.onDestroy() }
}
