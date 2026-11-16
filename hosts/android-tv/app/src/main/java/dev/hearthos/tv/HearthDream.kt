package dev.hearthos.tv

import android.annotation.SuppressLint
import android.service.dreams.DreamService
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.webkit.WebViewAssetLoader

/** Hearth TV's screensaver as the TV's own (the TV's Settings › Screensaver): the same page as inside Hearth TV. */
class HearthDream : DreamService() {
    @SuppressLint("SetJavaScriptEnabled")
    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        isInteractive = false
        isFullscreen = true
        val loader = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(this)).build()
        val web = WebView(this)
        web.settings.javaScriptEnabled = true
        web.settings.domStorageEnabled = true
        web.setBackgroundColor(0xFF000000.toInt())
        web.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? = loader.shouldInterceptRequest(request.url)
        }
        web.addJavascriptInterface(DreamBridge(), "HearthTVNative")
        setContentView(web)
        web.loadUrl("https://appassets.androidplatform.net/assets/tv/index.html#screensaver")
    }

    /** The screensaver only reads settings (which pictures, the clock) and Theater/My Media's pictures. */
    class DreamBridge {
        @android.webkit.JavascriptInterface fun settings(): String = TvApp.app.settings.read().toString()
        @android.webkit.JavascriptInterface fun state(): String = TvApp.app.state.read().toString()
        @android.webkit.JavascriptInterface fun theater(): String = Theater.list().toString()
        @android.webkit.JavascriptInterface fun version(): String = BuildConfig.VERSION_NAME
        @android.webkit.JavascriptInterface fun log(line: String) = Logs.add("screensaver: " + line.take(300))
    }
}
