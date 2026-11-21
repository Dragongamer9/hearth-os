package dev.hearthos.tv

import android.content.Context
import android.webkit.WebResourceResponse
import java.io.ByteArrayInputStream

/**
 * Ads and trackers stopped before they load on websites in Theater (the owner, Nov 21: pop-ups with QR codes and VPN
 * deals over the videos, "all scams"). The list is built into the APK (assets/tv/adblock.txt: Hearth TV's own list of
 * the ad networks behind those, hosts/android-tv/adblock/curated.txt, plus the free public lists the build adds), one
 * domain a line; a request to it, or to any of its subdomains, gets nothing back. The page's own site is never blocked.
 */
object AdBlock {
    @Volatile private var set: Set<String>? = null
    @Volatile var stopped = 0
        private set

    fun parse(text: String): Set<String> {
        val out = HashSet<String>(text.length / 16)
        for (raw in text.lineSequence()) {
            val l = raw.substringBefore('#').trim().lowercase()
            if (l.isEmpty()) continue
            // hosts-file lines ("0.0.0.0 ads.example.com") too
            val d = l.split(Regex("""\s+""")).last().trimEnd('.')
            if (d.contains('.') && !d.contains('/') && d != "localhost" && !d.all { it.isDigit() || it == '.' }) out.add(d)
        }
        return out
    }

    /** Read once, off the window's thread (a few hundred milliseconds for the whole list). */
    fun load(ctx: Context) {
        if (set != null) return
        set = try { ctx.assets.open("tv/adblock.txt").bufferedReader().use { parse(it.readText()) } } catch (e: Exception) {
            Logs.add("adblock: no list (${e.message})"); emptySet()
        }
        Logs.add("adblock: ${set?.size ?: 0} ad and tracker domains")
    }

    /** [host] or a domain it's under is on the list, and it isn't the page's own site. */
    fun blocked(host: String?, pageSite: String, list: Set<String>? = set): Boolean {
        val s = list ?: return false
        var h = (host ?: return false).lowercase().trimEnd('.')
        if (pageSite.isNotEmpty() && WebNav.site(h) == pageSite) return false
        while (true) {
            if (h in s) return true
            val i = h.indexOf('.')
            if (i < 0 || h.indexOf('.', i + 1) < 0) return false
            h = h.substring(i + 1)
        }
    }

    fun empty(): WebResourceResponse {
        stopped++
        return WebResourceResponse("text/plain", "utf-8", 403, "Blocked by Hearth TV", mapOf("Cache-Control" to "no-store"), ByteArrayInputStream(ByteArray(0)))
    }

    /**
     * Run in every frame of a website (the player's own frame too): layers put over a video or the whole page by
     * another site (a link or frame to elsewhere, "scan this QR code", "VPN", "you won") are taken away, checked again
     * as the page changes. The site's own play buttons and controls stay: anything holding the video, or only the
     * site's own links, is left alone. Light: it only looks at what's at the middle of each video and of the screen.
     */
    const val CLEAN_JS = """(function(){if(window.__hearthAds)return;window.__hearthAds=1;
var here=location.hostname.replace(/^www\./,'');
function base(h){h=(h||'').replace(/^www\./,'');var p=h.split('.');return p.length>2?p.slice(-2).join('.'):h}
var mine=base(here);
var scam=/\b(vpn|scan|qr code|qr-code|deal|offer|congratulations|you won|winner|prize|install|download now|allow notifications|click allow)\b/i;
function foreign(e){var l=e.querySelectorAll?e.querySelectorAll('a[href],iframe[src]'):[];if(e.tagName==='A'||e.tagName==='IFRAME')l=[e].concat([].slice.call(l));
for(var i=0;i<l.length;i++){try{var u=new URL(l[i].href||l[i].src,location.href);if(/^https?:/.test(u.protocol)&&base(u.hostname)!==mine)return true}catch(x){}}return false}
function holdsVideo(e){return e.tagName==='VIDEO'||(e.querySelector&&e.querySelector('video'))}
function sweep(){var pts=[[innerWidth/2,innerHeight/2]];
[].forEach.call(document.querySelectorAll('video'),function(v){var r=v.getBoundingClientRect();if(r.width>40&&r.height>40)pts.push([r.left+r.width/2,r.top+r.height/2])});
var gone=0;pts.forEach(function(p){var stack=document.elementsFromPoint?document.elementsFromPoint(p[0],p[1]):[];
for(var i=0;i<stack.length;i++){var e=stack[i];if(e===document.body||e===document.documentElement||holdsVideo(e))break;
var top=e;while(top.parentElement&&top.parentElement!==document.body&&!holdsVideo(top.parentElement)){var s=getComputedStyle(top.parentElement);if(s.position!=='fixed'&&s.position!=='absolute')break;top=top.parentElement}
var st=getComputedStyle(top);var r=top.getBoundingClientRect();
var big=r.width*r.height>innerWidth*innerHeight*0.08;
var layered=(st.position==='fixed'||st.position==='absolute')&&(parseInt(st.zIndex)||0)>=10;
if(big&&layered&&(foreign(top)||scam.test((top.innerText||'').slice(0,400)))){top.remove();gone++;break}}});
if(gone)try{HearthAds&&HearthAds.removed(gone)}catch(x){}}
var t=0;function soon(){if(t)return;t=setTimeout(function(){t=0;sweep()},300)}
new MutationObserver(soon).observe(document.documentElement,{childList:true,subtree:true});
setInterval(sweep,2000);document.addEventListener('DOMContentLoaded',sweep);sweep()})();"""
}
