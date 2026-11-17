// Hearth Settings › Network & internet › Proxy: no proxy, automatic (a setup script address) or by hand (web, secure web
// and SOCKS proxies, and addresses that skip it). Saved in ~/.config/nimbus/proxy.json through hearth_settings_io only
// when you press Save (hearth-network proxy-set), then handed to GTK apps, Qt/KDE apps and command-line programs.
// Proxies that need a password aren't offered: Hearth would have to keep it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Proxy"
    property var st: null              // the saved proxy
    property string mode: "none"       // what's picked on the page (saved with Save)
    property string message: ""
    property bool messageBad: false
    readonly property bool changed: !!st && (mode !== st.mode || url.text !== "" || http.text !== "" || https.text !== "" || socks.text !== "" || ignore.text !== "")

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["proxy"], r => { page.st = r; page.mode = r.mode; }, e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-network" }
    function keep(f, saved) { return f.text.trim() !== "" ? f.text.trim() : saved; }
    function save() {
        say("");
        const req = { mode: page.mode };
        if (page.mode === "auto") req.url = keep(url, st.url);
        if (page.mode === "manual") {
            req.http = keep(http, st.http); req.https = keep(https, st.https); req.socks = keep(socks, st.socks);
            req.ignore = keep(ignore, st.ignore);
        }
        job.run(["proxy-set"], r => {
            page.st = r; page.mode = r.mode;
            for (const f of [url, http, https, socks, ignore]) f.text = "";
            page.say(r.mode === "none" ? "No proxy." : "Proxy saved. Apps opened from now on use it; restart open ones.");
        }, e => page.say(e, true), JSON.stringify(req), "save");
    }

    SetCardGroup {
        title: "Proxy"
        SetChoice {
            icon: "globe"; title: "Use a proxy"
            sub: !page.st ? "Checking…" : page.st.mode === "none" ? "Not now: apps go straight to the internet" : page.st.mode === "auto" ? "Automatic, from " + page.st.url : "By hand" + (page.st.http ? " · " + page.st.http : "")
            options: [{ value: "none", label: "No" }, { value: "auto", label: "Automatic" }, { value: "manual", label: "By hand" }]
            current: page.mode
            onChosen: v => page.mode = v
        }
    }
    SetCardGroup {
        title: "Automatic setup"
        visible: page.mode === "auto"
        SetField { id: url; visible: page.mode === "auto"; placeholder: page.st && page.st.url ? "Setup script: " + page.st.url : "Setup script address, like http://wpad.example/proxy.pac" }
    }
    SetCardGroup {
        title: "Proxies"
        visible: page.mode === "manual"
        SetField { id: http; visible: page.mode === "manual"; placeholder: "Web (HTTP)" + (page.st && page.st.http ? ": " + page.st.http : ", like proxy.example.com:3128") }
        SetField { id: https; visible: page.mode === "manual"; placeholder: "Secure web (HTTPS)" + (page.st && page.st.https ? ": " + page.st.https : ", often the same") }
        SetField { id: socks; visible: page.mode === "manual"; placeholder: "SOCKS" + (page.st && page.st.socks ? ": " + page.st.socks : ", like localhost:1080 (optional)") }
        SetField { id: ignore; visible: page.mode === "manual"; placeholder: "Skip the proxy for: " + (page.st ? page.st.ignore : "localhost") }
    }
    SetCardGroup {
        title: ""
        SetRow {
            enabled: page.changed
            icon: "check"; chevron: false; title: job.what === "save" ? "Saving…" : "Save"
            sub: page.changed ? "For web browsers, apps and the terminal" : "Nothing changed"
            onActivated: page.save()
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
