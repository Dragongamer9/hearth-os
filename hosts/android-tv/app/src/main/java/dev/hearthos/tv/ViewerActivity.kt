package dev.hearthos.tv

import android.annotation.SuppressLint
import android.app.Activity
import android.graphics.Bitmap
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.Message
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.inputmethod.InputMethodManager
import android.webkit.CookieManager
import android.webkit.JavascriptInterface
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebResourceError
import android.webkit.WebSettings
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import androidx.webkit.WebViewAssetLoader
import org.json.JSONObject

/**
 * Theater: a saved link's page, full screen with nothing of Hearth's in it. The page's own full screen (a video's)
 * fills the TV.
 *
 * Pointer mode (the default; Settings › Remote and controllers › Pointer on websites): a mouse pointer drawn by a
 * see-through layer over the website (assets/tv/pointer.html, pointer.js). ▲▼◀▶ move it; OK is a finger on the page at
 * the pointer (the owner, Nov 20: OK a left click, holding OK a right click), so a short press clicks and holding it
 * long-presses (the site's right-click menu); CH +/− or 2/8 scroll; Back goes back a page (or leaves the page's full
 * screen). Presses are real touch events put into the website's own view, hovering and scrolling real mouse events,
 * so the site's buttons, menus and video players work as with a finger or a mouse; a text field opens the TV's keyboard.
 *
 * Page mode (hold Back, or ☰ / a colour key; the same again goes back): the remote works the page the way the site
 * built it (arrows and OK go to the site); in a video's full screen OK plays/pauses and ◀ ▶ skip 10 s, as before.
 * Play/pause, ⏩ ⏪ and a controller's Y work in both modes.
 */
class ViewerActivity : Activity() {
    private lateinit var web: WebView
    private lateinit var root: FrameLayout
    private var layer: WebView? = null
    private var full: View? = null
    private var fullCallback: WebChromeClient.CustomViewCallback? = null
    private val ui = Handler(Looper.getMainLooper())
    @Volatile private var layerReady = false
    private var pointerMode = true
    private var backDown = false
    private var backLong = false
    /** The touch OK holds down on the website (pointer mode), lifted on OK's release, Back, or leaving. */
    private var pressAt = 0L
    private var pressView: View? = null
    private var pressX = 0f
    private var pressY = 0f

    private fun video(js: String) = web.evaluateJavascript(
        "(function(){var v=[].slice.call(document.querySelectorAll('video')).sort(function(a,b){return b.clientWidth*b.clientHeight-a.clientWidth*a.clientHeight})[0];if(!v)return false;$js;return true})()", null)

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        root = FrameLayout(this)
        web = WebView(this)
        web.isFocusable = true
        web.isFocusableInTouchMode = true
        // Videos on the saved links (the owner, Nov 21: they didn't load, showed an error screen, went to a random other
        // website or played black): the TV's own Chrome as the user agent (no "; wv", which some players refuse),
        // pages from http and https mixed, other sites' players' cookies, pop-ups never opened, and where a page may go
        // decided by WebNav.
        with(web.settings) {
            javaScriptEnabled = true; domStorageEnabled = true; mediaPlaybackRequiresUserGesture = false
            userAgentString = WebNav.userAgent(try { WebSettings.getDefaultUserAgent(this@ViewerActivity) } catch (_: Exception) { null })
            mixedContentMode = WebSettings.MIXED_CONTENT_COMPATIBILITY_MODE
            setSupportMultipleWindows(true)
            javaScriptCanOpenWindowsAutomatically = false
            loadWithOverviewMode = true; useWideViewPort = true
        }
        try { CookieManager.getInstance().setAcceptThirdPartyCookies(web, true) } catch (_: Exception) {}
        web.setBackgroundColor(Color.BLACK)
        web.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (!request.isForMainFrame) return false
                val url = request.url.toString()
                return when (val g = WebNav.decide(url, view.url, request.hasGesture(), request.isRedirect)) {
                    WebNav.Go.Allow -> false
                    WebNav.Go.Block -> { Logs.add("theater: stopped going to ${url.take(160)}"); true }
                    is WebNav.Go.Instead -> { view.loadUrl(g.url); true }
                }
            }
            override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
                if (request.isForMainFrame) Logs.add("theater: ${error.errorCode} ${error.description} at ${request.url.toString().take(160)}")
            }
            // The page's renderer crashed (a heavy video page): the page again instead of a dead, black screen.
            override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
                Logs.add("theater: the page's renderer stopped (crashed=${detail.didCrash()}); opening it again")
                val url = view.url ?: intent.getStringExtra("url")
                ui.post { recreateWith(url) }
                return true
            }
        }
        web.webChromeClient = object : WebChromeClient() {
            override fun onShowCustomView(view: View, callback: CustomViewCallback) {
                full = view; fullCallback = callback
                view.setBackgroundColor(Color.BLACK)
                root.addView(view, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
                layer?.bringToFront()           // the pointer stays over the video's full screen
            }
            override fun onHideCustomView() { full?.let { root.removeView(it) }; full = null; fullCallback = null; web.requestFocus() }
            // Pop-ups and pop-unders are never opened (they were the "random other website").
            override fun onCreateWindow(view: WebView, isDialog: Boolean, isUserGesture: Boolean, resultMsg: Message): Boolean {
                Logs.add("theater: a pop-up wasn't opened")
                return false
            }
            // Videos that need DRM (Widevine) may use it; nothing else (camera, microphone) is ever allowed.
            override fun onPermissionRequest(request: PermissionRequest) {
                val ok = request.resources.filter { it == PermissionRequest.RESOURCE_PROTECTED_MEDIA_ID }
                if (ok.isNotEmpty()) request.grant(ok.toTypedArray()) else request.deny()
            }
            // No grey "play" picture before a video starts: black, like a TV.
            override fun getDefaultVideoPoster(): Bitmap? = Bitmap.createBitmap(1, 1, Bitmap.Config.ARGB_8888).apply { eraseColor(Color.BLACK) }
        }
        root.addView(web)

        // The pointer's settings (read only: defaults are never written) and the look Hearth TV has now.
        val p = try { TvApp.app.settings.read().optJSONObject("pointer") } catch (_: Exception) { null }
        val ps = PointerSettings.of(p?.optString("speed"), p?.optString("size"), if (p?.has("startOn") == true) p.optBoolean("startOn", true) else null)
        pointerMode = ps.startOn
        val look = try { JSONObject(intent.getStringExtra("look") ?: "{}") } catch (_: Exception) { JSONObject() }
        addLayer(JSONObject().put("speed", ps.speed).put("size", ps.size).put("startOn", ps.startOn).put("look", look))

        setContentView(root)
        web.loadUrl(intent.getStringExtra("url") ?: "about:blank")
        web.requestFocus()
    }

    /** The page again after its renderer stopped: a new view, so it plays instead of staying black. */
    private fun recreateWith(url: String?) {
        if (isFinishing) return
        intent.putExtra("url", url ?: "about:blank")
        recreate()
    }

    /** The see-through layer with the pointer (Hearth's own page from the APK, never part of the website). */
    @SuppressLint("SetJavaScriptEnabled")
    private fun addLayer(cfg: JSONObject) {
        try {
            val loader = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(this)).build()
            val l = WebView(this)
            l.setBackgroundColor(Color.TRANSPARENT)
            l.isFocusable = false
            l.isFocusableInTouchMode = false
            l.isClickable = false
            l.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            l.settings.javaScriptEnabled = true
            l.webViewClient = object : WebViewClient() {
                override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? = loader.shouldInterceptRequest(request.url)
            }
            l.addJavascriptInterface(Native(), "HearthPointerNative")
            root.addView(l, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
            l.loadUrl("https://appassets.androidplatform.net/assets/tv/pointer.html#" + Uri.encode(cfg.toString()))
            layer = l
        } catch (e: Exception) {
            // No pointer: the remote works the page itself, as before.
            Logs.add("pointer: no layer: ${e.message}")
            layer = null
        }
    }

    private fun layerJs(code: String) { if (layerReady) layer?.evaluateJavascript("window.hp&&hp.$code", null) }

    private fun setMode(on: Boolean, tell: Boolean = true) {
        pointerMode = on
        if (tell) layerJs("mode('${if (on) "pointer" else "page"}')")
    }

    /** Back: leave the page's full screen, go back a page, or leave the website. */
    private fun goBack() {
        if (full != null) {
            web.evaluateJavascript("document.exitFullscreen&&document.exitFullscreen()", null)
            ui.postDelayed({ if (full != null) fullCallback?.onCustomViewHidden() }, 400)
            return
        }
        if (web.canGoBack()) web.goBack() else finish()
    }

    override fun dispatchKeyEvent(e: KeyEvent): Boolean {
        val k = PointerKey.of(e.keyCode)
        val down = e.action == KeyEvent.ACTION_DOWN
        val up = e.action == KeyEvent.ACTION_UP
        val first = down && e.repeatCount == 0
        val pointer = pointerMode && layerReady
        when (k) {
            // Back acts when it's let go, so holding it can switch between pointer and page mode.
            PointerKey.BACK -> {
                if (first) { backDown = true; backLong = false }
                else if (down && backDown && !backLong && layerReady && PointerKey.isLong(e.downTime, e.eventTime)) { backLong = true; setMode(!pointerMode) }
                else if (up && backDown) { backDown = false; if (!backLong) { if (pointer) layerJs("back()") else goBack() } }
                return true
            }
            PointerKey.MODE -> { if (!layerReady) return super.dispatchKeyEvent(e); if (first) setMode(!pointerMode); return true }
            PointerKey.PLAYPAUSE -> { if (first) video("v.paused?v.play():v.pause()"); return true }
            PointerKey.FF -> { if (down) video("v.currentTime=v.currentTime+10"); return true }
            PointerKey.REW -> { if (down) video("v.currentTime=Math.max(0,v.currentTime-10)"); return true }
            PointerKey.FULLSCREEN -> { if (first) video("(v.requestFullscreen||v.webkitRequestFullscreen).call(v)"); return true }
            else -> {}
        }
        if (pointer) {
            when {
                k.isDirection -> { if (first) layerJs("key('${k.js}',true)") else if (up) layerJs("key('${k.js}',false)"); return true }
                k == PointerKey.OK -> { if (first) layerJs("ok(true)") else if (up) layerJs("ok(false)"); return true }
                k == PointerKey.SCROLL_UP -> { if (down) layerJs("scroll(-1,0)"); return true }
                k == PointerKey.SCROLL_DOWN -> { if (down) layerJs("scroll(1,0)"); return true }
                k == PointerKey.SCROLL_LEFT -> { if (down) layerJs("scroll(0,-1)"); return true }
                k == PointerKey.SCROLL_RIGHT -> { if (down) layerJs("scroll(0,1)"); return true }
            }
            return super.dispatchKeyEvent(e)
        }
        // Page mode: the site gets the keys, except in a video's full screen (OK play/pause, ◀ ▶ 10 s, as before).
        if (full != null) {
            if (k == PointerKey.OK) { if (first) video("v.paused?v.play():v.pause()"); return true }
            if (k == PointerKey.LEFT || k == PointerKey.RIGHT) {
                if (down) video(if (k == PointerKey.LEFT) "v.currentTime=Math.max(0,v.currentTime-10)" else "v.currentTime=v.currentTime+10")
                return true
            }
        }
        return super.dispatchKeyEvent(e)
    }

    // ---------- Real input for the website, at the pointer ----------
    private fun target(): View = full ?: web

    private fun touch(t: View, action: Int, x: Float, y: Float, downAt: Long): Boolean {
        val ev = MotionEvent.obtain(downAt, SystemClock.uptimeMillis(), action, x, y, 0)
        ev.source = InputDevice.SOURCE_TOUCHSCREEN
        val r = t.dispatchTouchEvent(ev)
        ev.recycle()
        return r
    }

    private fun mouse(action: Int, x: Float, y: Float, buttons: Int = 0, v: Float = 0f, h: Float = 0f, downAt: Long = SystemClock.uptimeMillis()): MotionEvent {
        val pp = arrayOf(MotionEvent.PointerProperties().apply { id = 0; toolType = MotionEvent.TOOL_TYPE_MOUSE })
        val pc = arrayOf(MotionEvent.PointerCoords().apply {
            this.x = x; this.y = y; pressure = if (buttons != 0) 1f else 0f; size = 1f
            if (v != 0f) setAxisValue(MotionEvent.AXIS_VSCROLL, v)
            if (h != 0f) setAxisValue(MotionEvent.AXIS_HSCROLL, h)
        })
        return MotionEvent.obtain(downAt, SystemClock.uptimeMillis(), action, 1, pp, pc, 0, buttons, 1f, 1f, 0, 0, InputDevice.SOURCE_MOUSE, 0)
    }

    private fun generic(t: View, ev: MotionEvent) { try { t.dispatchGenericMotionEvent(ev) } finally { ev.recycle() } }

    /** Lifts the press OK holds down; [cancel] (Hearth TV leaving the screen) clicks nothing. */
    private fun lift(x: Float, y: Float, cancel: Boolean = false) {
        val pv = pressView ?: return
        if (pressAt == 0L) return
        touch(pv, if (cancel) MotionEvent.ACTION_CANCEL else MotionEvent.ACTION_UP, x, y, pressAt)
        pressAt = 0L; pressView = null
        if (!cancel) ui.postDelayed({ keyboardIfTyping() }, 380)
    }

    /** A tap that lands on a text field opens the TV's keyboard. */
    private fun keyboardIfTyping() {
        if (full != null) return
        web.evaluateJavascript(EDITING) { r ->
            if (r != null && r.contains("edit")) {
                web.requestFocus()
                getSystemService(InputMethodManager::class.java)?.showSoftInput(web, InputMethodManager.SHOW_IMPLICIT)
            }
        }
    }

    /** What pointer.js asks for (window.HearthPointerNative); [fx], [fy] are fractions of the screen. */
    inner class Native {
        private fun at(fx: Double, fy: Double, work: (View, Float, Float) -> Unit) = ui.post {
            val t = target()
            work(t, (fx.coerceIn(0.0, 1.0) * t.width).toFloat(), (fy.coerceIn(0.0, 1.0) * t.height).toFloat())
        }
        @JavascriptInterface fun ready() { layerReady = true; ui.post { if (!pointerMode) layerJs("mode('page')") } }
        /** Hovering; while OK holds a press down, the finger moves (dragging a slider or a video's bar). */
        @JavascriptInterface fun move(fx: Double, fy: Double) = at(fx, fy) { t, x, y ->
            val pv = pressView
            if (pressAt != 0L && pv != null) { pressX = x; pressY = y; touch(pv, MotionEvent.ACTION_MOVE, x, y, pressAt) }
            else generic(t, mouse(MotionEvent.ACTION_HOVER_MOVE, x, y))
        }
        /** OK down: a finger on the page at the pointer. */
        @JavascriptInterface fun press(fx: Double, fy: Double) = at(fx, fy) { t, x, y ->
            if (pressAt != 0L) return@at
            pressAt = SystemClock.uptimeMillis(); pressView = t; pressX = x; pressY = y
            touch(t, MotionEvent.ACTION_DOWN, x, y, pressAt)
        }
        /** OK up: the finger lifts. Held long enough, Chromium has made it a long-press (the site's menu). */
        @JavascriptInterface fun release(fx: Double, fy: Double) = at(fx, fy) { _, x, y -> lift(x, y) }
        /** [sx], [sy]: how far to scroll, in fractions of the screen (a mouse wheel's turn at the pointer). */
        @JavascriptInterface fun scroll(fx: Double, fy: Double, sx: Double, sy: Double) = at(fx, fy) { t, x, y ->
            val vc = ViewConfiguration.get(this@ViewerActivity)
            val v = PointerKey.wheelV((sy * t.height).toFloat(), vc.scaledVerticalScrollFactor)
            val h = PointerKey.wheelH((sx * t.width).toFloat(), vc.scaledHorizontalScrollFactor)
            if (v != 0f || h != 0f) generic(t, mouse(MotionEvent.ACTION_SCROLL, x, y, v = v, h = h))
        }
        @JavascriptInterface fun back() { ui.post { this@ViewerActivity.goBack() } }
        @JavascriptInterface fun setMode(m: String) { ui.post { this@ViewerActivity.setMode(m == "pointer", tell = false) } }
        @JavascriptInterface fun log(s: String) { Logs.add("pointer: " + s.take(300)) }
    }

    override fun onPause() {
        layerJs("release()")
        lift(pressX, pressY, cancel = true)
        super.onPause()
    }

    override fun onDestroy() {
        ui.removeCallbacksAndMessages(null)
        layer?.destroy()
        web.destroy()
        super.onDestroy()
    }

    companion object {
        /** Is the page's focused element (inside its frames too, where it can see them) a text field? */
        private const val EDITING = "(function(){var a=document.activeElement;try{while(a&&(a.tagName==='IFRAME'||a.tagName==='FRAME')&&a.contentDocument)a=a.contentDocument.activeElement}catch(e){}" +
            "if(!a)return '';if(a.isContentEditable)return 'edit';var t=(a.tagName||'').toUpperCase();if(t==='TEXTAREA')return 'edit';" +
            "if(t==='INPUT'){var y=(a.type||'text').toLowerCase();return ['button','checkbox','radio','submit','reset','range','color','file','image','hidden'].indexOf(y)<0?'edit':''}return ''})()"
    }
}
