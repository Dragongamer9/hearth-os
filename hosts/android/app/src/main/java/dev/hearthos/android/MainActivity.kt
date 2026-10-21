package dev.hearthos.android

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.hardware.input.InputManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.webkit.ConsoleMessage
import android.webkit.JavascriptInterface
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.webkit.WebViewAssetLoader
import java.io.ByteArrayInputStream
import java.io.File
import java.io.FileInputStream
import java.io.IOException

/**
 * Hearth for Android: Hearth's page (ui/index.html, copied into the app's assets when it's built) full screen in a
 * WebView, served from https://appassets.androidplatform.net/ui/ by the asset loader (a real https origin, so
 * fetch and storage work as in the other hosts). [Bridge] is its host; controllers are read here.
 */
class MainActivity : Activity() {
    private lateinit var web: WebView
    private lateinit var bridge: Bridge
    private val main = Handler(Looper.getMainLooper())
    private val repeater = Repeater()
    private val keyRepeatAt = HashMap<String, Long>()
    private var inputListener: InputManager.InputDeviceListener? = null
    /** The Linux desktop's view is open (the page asks, pad.raw): buttons go to the page as presses and releases,
     *  the sticks and triggers as positions (padAxes), the hat (some controllers' d-pad) as d-pad buttons. */
    private var rawPad = false
    private val rawHat = HashSet<String>()
    private var lastAxes = FloatArray(6) { Float.NaN }

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Logs.init(File(filesDir, "logs"))
        Logs.log("Hearth ${BuildConfig.VERSION_NAME} starting · Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT}) · ${Build.MANUFACTURER} ${Build.MODEL}")
        WindowCompat.setDecorFitsSystemWindows(window, false)
        bridge = Bridge(this)

        web = WebView(this)
        web.setBackgroundColor(0xFF0C1016.toInt())
        web.isFocusable = true
        web.isFocusableInTouchMode = true
        web.overScrollMode = View.OVER_SCROLL_NEVER
        web.isVerticalScrollBarEnabled = false
        web.isHorizontalScrollBarEnabled = false
        val s = web.settings
        s.javaScriptEnabled = true
        s.domStorageEnabled = true
        s.mediaPlaybackRequiresUserGesture = false   // the interface's clicks (Web Audio)
        s.allowFileAccess = false
        s.allowContentAccess = false
        s.textZoom = 100                              // Settings › Appearance › Text size does this in Hearth
        s.setSupportZoom(false)
        s.builtInZoomControls = false
        s.displayZoomControls = false
        s.useWideViewPort = true                      // for the narrow-screen viewport (see injectShim)
        s.loadWithOverviewMode = true
        WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG)

        val loader = WebViewAssetLoader.Builder()
            .addPathHandler("/ui/", UiHandler())
            .addPathHandler("/art/", ArtHandler())
            .build()
        web.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? =
                loader.shouldInterceptRequest(request.url)

            // Hearth's own pages stay here; any other link opens in the browser.
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (request.url.host == HOST) return false
                try { startActivity(Intent(Intent.ACTION_VIEW, request.url).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) } catch (_: ActivityNotFoundException) { }
                return true
            }

            override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
                Logs.log("WebView's page process stopped (crashed: ${detail.didCrash()}): starting Hearth's page again")
                recreate()
                return true
            }
        }
        web.webChromeClient = object : WebChromeClient() {
            override fun onConsoleMessage(m: ConsoleMessage): Boolean {
                if (m.messageLevel() == ConsoleMessage.MessageLevel.ERROR) Logs.log("page error: ${m.message()} (${m.sourceId()?.substringAfterLast('/')}:${m.lineNumber()})")
                return true
            }
        }
        web.addJavascriptInterface(HearthJsInterface(bridge), "HearthAndroid")
        setContentView(web)
        immersive()

        val im = getSystemService(INPUT_SERVICE) as InputManager
        inputListener = object : InputManager.InputDeviceListener {
            override fun onInputDeviceAdded(deviceId: Int) = bridge.padsChanged()
            override fun onInputDeviceRemoved(deviceId: Int) = bridge.padsChanged()
            override fun onInputDeviceChanged(deviceId: Int) {}
        }
        im.registerInputDeviceListener(inputListener, main)

        web.loadUrl(START_URL)
        web.requestFocus()
    }

    /** A message for the page, as {type:'result'|'event'|'toast', ...} (main thread). */
    fun deliver(json: String) {
        if (!::web.isInitialized) return
        web.evaluateJavascript("window.__hearthRecv&&window.__hearthRecv($json)", null)
    }

    // ---------------------------------------------------------------- serving the page and the art
    private inner class UiHandler : WebViewAssetLoader.PathHandler {
        override fun handle(path: String): WebResourceResponse? {
            val p = path.ifEmpty { "index.html" }
            if (p.contains("..")) return null
            return try {
                if (p == "index.html") {
                    val html = assets.open("ui/index.html").use { String(it.readBytes(), Charsets.UTF_8) }
                    WebResourceResponse("text/html", "utf-8", ByteArrayInputStream(injectShim(html).toByteArray(Charsets.UTF_8)))
                } else WebResourceResponse(mime(p), null, assets.open("ui/$p"))
            } catch (e: IOException) { null }
        }
    }

    private inner class ArtHandler : WebViewAssetLoader.PathHandler {
        override fun handle(path: String): WebResourceResponse? {
            val parts = path.split('/')           // <package>/<version>/<cover|wide>.png
            if (parts.size != 3) return null
            val f = bridge.art.file(parts[0], parts[1].toLongOrNull() ?: 0L, parts[2].removeSuffix(".png")) ?: return null
            return try {
                WebResourceResponse("image/png", null, 200, "OK", mapOf("Cache-Control" to "max-age=31536000"), FileInputStream(f))
            } catch (e: IOException) { null }
        }
    }

    private fun mime(p: String) = when (p.substringAfterLast('.').lowercase()) {
        "svg" -> "image/svg+xml"; "png" -> "image/png"; "js" -> "text/javascript"; "css" -> "text/css"; "html" -> "text/html"; else -> "application/octet-stream"
    }

    /**
     * The host shim, first thing in <head> (before the page's script looks for its host), like hearth-rs's SHIM:
     * window.nimbusHost with platform 'android'. Small screens (phones and most handhelds, under 1000 dp wide) get
     * the page laid out 1280 px wide and scaled to fit, the size it's designed for.
     */
    private fun injectShim(html: String): String {
        val c = resources.configuration
        val narrow = maxOf(c.screenWidthDp, c.screenHeightDp) < 1000
        var out = html
        if (narrow) out = out.replaceFirst(Regex("""<meta name="viewport" content="[^"]*">"""), """<meta name="viewport" content="width=1280, viewport-fit=cover">""")
        val i = out.indexOf("<head>", ignoreCase = true)
        return if (i < 0) SHIM + out else out.substring(0, i + 6) + SHIM + out.substring(i + 6)
    }

    // ---------------------------------------------------------------- full screen
    private fun immersive() {
        val c = WindowCompat.getInsetsController(window, window.decorView)
        c.hide(WindowInsetsCompat.Type.systemBars())
        c.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) { immersive(); web.requestFocus() }
    }

    // ---------------------------------------------------------------- controllers
    /**
     * Controller buttons become the page's "pad" events (A, B, X, Y, LB, RB, Start, Select, Guide, the d-pad);
     * Back (button or gesture) is B. A keyboard's keys go to the page as keys (its KEYMAP), and the volume keys to
     * Android.
     */
    override fun dispatchKeyEvent(e: KeyEvent): Boolean {
        val code = e.keyCode
        if (code == KeyEvent.KEYCODE_VOLUME_UP || code == KeyEvent.KEYCODE_VOLUME_DOWN || code == KeyEvent.KEYCODE_VOLUME_MUTE) return super.dispatchKeyEvent(e)
        val fromPad = e.isFromSource(InputDevice.SOURCE_GAMEPAD) || e.isFromSource(InputDevice.SOURCE_JOYSTICK) ||
            (e.isFromSource(InputDevice.SOURCE_DPAD) && !e.isFromSource(InputDevice.SOURCE_KEYBOARD))
        val name = when {
            code == KeyEvent.KEYCODE_BACK -> if (rawPad && !fromPad) "back" else "b"
            fromPad || KeyEvent.isGamepadButton(code) -> Pad.buttonName(code)
            else -> null
        } ?: return super.dispatchKeyEvent(e)
        if (rawPad) {
            val now = SystemClock.uptimeMillis()
            when (e.action) {
                KeyEvent.ACTION_DOWN ->
                    if (e.repeatCount == 0) { keyRepeatAt[name] = now; bridge.padRaw(name, true) }
                    else if (name in Pad.DIRECTIONS && now - (keyRepeatAt[name] ?: 0L) >= 110) { keyRepeatAt[name] = now; bridge.padRaw(name, true, repeat = true) }
                KeyEvent.ACTION_UP -> bridge.padRaw(name, false)
            }
            return true
        }
        if (e.action == KeyEvent.ACTION_DOWN) {
            val now = SystemClock.uptimeMillis()
            if (e.repeatCount == 0) { keyRepeatAt[name] = now; bridge.pad(name) }
            else if (name in Pad.DIRECTIONS && now - (keyRepeatAt[name] ?: 0L) >= 110) { keyRepeatAt[name] = now; bridge.pad(name) }
        }
        return true
    }

    /** Sticks and the hat: directions with the page's key repeat (Repeater). */
    override fun dispatchGenericMotionEvent(e: MotionEvent): Boolean {
        if (e.isFromSource(InputDevice.SOURCE_JOYSTICK) && e.action == MotionEvent.ACTION_MOVE) {
            if (rawPad) { rawMotion(e); return true }
            val dirs = Pad.directions(e.getAxisValue(MotionEvent.AXIS_X), e.getAxisValue(MotionEvent.AXIS_Y),
                e.getAxisValue(MotionEvent.AXIS_HAT_X), e.getAxisValue(MotionEvent.AXIS_HAT_Y), e.getAxisValue(MotionEvent.AXIS_RZ), repeater.holding)
            for (k in repeater.update(dirs, SystemClock.uptimeMillis())) bridge.pad(k)
            main.removeCallbacks(tick)
            if (repeater.holding.isNotEmpty()) main.postDelayed(tick, 30)
            return true
        }
        return super.dispatchGenericMotionEvent(e)
    }

    /** Raw mode: the hat as d-pad presses/releases; the sticks and triggers to the page when they moved. */
    private fun rawMotion(e: MotionEvent) {
        val hx = e.getAxisValue(MotionEvent.AXIS_HAT_X)
        val hy = e.getAxisValue(MotionEvent.AXIS_HAT_Y)
        val hat = HashSet<String>()
        if (hy < -0.5f) hat.add("up")
        if (hy > 0.5f) hat.add("down")
        if (hx < -0.5f) hat.add("left")
        if (hx > 0.5f) hat.add("right")
        for (k in hat) if (k !in rawHat) bridge.padRaw(k, true)
        for (k in rawHat) if (k !in hat) bridge.padRaw(k, false)
        rawHat.clear(); rawHat.addAll(hat)
        val v = floatArrayOf(e.getAxisValue(MotionEvent.AXIS_X), e.getAxisValue(MotionEvent.AXIS_Y),
            e.getAxisValue(MotionEvent.AXIS_Z), e.getAxisValue(MotionEvent.AXIS_RZ),
            maxOf(e.getAxisValue(MotionEvent.AXIS_LTRIGGER), e.getAxisValue(MotionEvent.AXIS_BRAKE)),
            maxOf(e.getAxisValue(MotionEvent.AXIS_RTRIGGER), e.getAxisValue(MotionEvent.AXIS_GAS)))
        val moved = v.indices.any { i -> lastAxes[i].isNaN() || Math.abs(v[i] - lastAxes[i]) >= 0.02f || ((v[i] == 0f) != (lastAxes[i] == 0f)) }
        if (moved) { lastAxes = v; bridge.padAxes(v[0], v[1], v[2], v[3], v[4], v[5]) }
    }

    fun setRawPad(on: Boolean) {
        if (rawPad == on) return
        rawPad = on
        repeater.clear(); main.removeCallbacks(tick)
        rawHat.clear(); keyRepeatAt.clear(); lastAxes = FloatArray(6) { Float.NaN }
        Logs.log("controller: ${if (on) "Linux desktop mode (raw)" else "Hearth mode"}")
    }

    private val tick: Runnable = object : Runnable {
        override fun run() {
            for (k in repeater.tick(SystemClock.uptimeMillis())) bridge.pad(k)
            if (repeater.holding.isNotEmpty()) main.postDelayed(this, 30)
        }
    }

    // ---------------------------------------------------------------- lifecycle
    private val statusTimer: Runnable = object : Runnable {
        override fun run() { bridge.pushStatus(); main.postDelayed(this, 30_000) }
    }

    override fun onResume() {
        super.onResume()
        web.onResume()
        bridge.onForeground()
        main.removeCallbacks(statusTimer)
        main.postDelayed(statusTimer, 30_000)
    }

    override fun onPause() {
        super.onPause()
        bridge.onBackground()
        repeater.clear()
        main.removeCallbacks(tick)
        main.removeCallbacks(statusTimer)
        web.onPause()
    }

    /** The Home button while Hearth is the home app (or Hearth's icon again): back to Hearth's Home. */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (intent.hasCategory(Intent.CATEGORY_HOME)) bridge.event("open", org.json.JSONObject().put("view", "home"))
    }

    /** Settings › Games › ROMs folder: Android's folder picker; the answer goes to the bridge. */
    fun pickFolder() {
        try {
            startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION), PICK_FOLDER)
        } catch (e: ActivityNotFoundException) {
            Logs.log("roms: this device has no folder picker")
            bridge.folderPicked(null)
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == PICK_FOLDER) bridge.folderPicked(if (resultCode == RESULT_OK) data?.data else null)
    }

    override fun onDestroy() {
        inputListener?.let { (getSystemService(INPUT_SERVICE) as InputManager).unregisterInputDeviceListener(it) }
        main.removeCallbacksAndMessages(null)
        if (::web.isInitialized) { web.removeJavascriptInterface("HearthAndroid"); web.destroy() }
        bridge.shutdown()
        super.onDestroy()
    }

    companion object {
        const val HOST = WebViewAssetLoader.DEFAULT_DOMAIN
        const val START_URL = "https://$HOST/ui/index.html"
        private const val PICK_FOLDER = 41

        /** window.nimbusHost for the page: messages out through HearthAndroid, in through window.__hearthRecv. */
        const val SHIM = """<script>(function(){
  if (window.nimbusHost || !window.HearthAndroid) return;
  var listeners = [];
  window.__hearthRecv = function (m) { for (var i = 0; i < listeners.length; i++) { try { listeners[i]({ data: m }); } catch (e) { console.error(e); } } };
  window.nimbusHost = {
    platform: 'android',
    postMessage: function (msg) { window.HearthAndroid.postMessage(JSON.stringify(msg)); },
    addEventListener: function (type, fn) { if (type === 'message' && typeof fn === 'function') listeners.push(fn); }
  };
})();</script>
"""
    }
}

/** The page's way in: window.nimbusHost.postMessage(msg) (the shim) -> HearthAndroid.postMessage(json). Public and
 *  top level: WebView finds @JavascriptInterface methods by reflection. */
class HearthJsInterface(private val bridge: Bridge) {
    @JavascriptInterface
    fun postMessage(json: String) = bridge.onMessage(json)
}
