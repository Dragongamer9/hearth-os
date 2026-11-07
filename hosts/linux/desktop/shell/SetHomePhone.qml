// Hearth Settings › Home server › Phone notifications (docs/SERVER-APPS.md): Home server alerts (a drive failing or
// filling up, a backup failing) and server apps that stopped, sent to the ntfy app on your phone. Off until you turn it
// on; a private topic is made only when you press Make a topic; ntfy.sh or your own ntfy (Server apps). Checked every 5
// minutes by your own timer while it's on; the same alert is sent again only after a day. Through hearth-server-apps.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Phone notifications"
    property var st: null
    property string msg: ""
    property bool bad: false
    property bool serverOpen: false
    readonly property var cfg: st ? st.settings : null
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["ntfy"], r => page.st = r, e => page.st = { error: e }); }
    function set(k, v, ok) { act.run(["ntfy-set", k, v], r => { page.st = r; if (ok) page.say(ok); }, e => { page.say(e, true); page.refresh(); }); }
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-server-apps" }
    SetJob { id: act; helper: "hearth-server-apps" }

    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 4
        text: "Get a notification on your phone when something on this server needs you, wherever you are. Install the free ntfy app (Android or iPhone), make a topic here, and subscribe to it in the app."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.error
        Layout.fillWidth: true
        text: page.st ? (page.st.error || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetCardGroup {
        title: "Your topic"
        visible: !!page.cfg
        SetRow {
            icon: "bell"; title: page.cfg && page.cfg.topic ? "Subscribe to this in the ntfy app" : "Make a topic"
            sub: page.st && page.st.url ? page.st.url + " · anyone with this name can read it, so it's long and random"
                 : "A private, random name your phone subscribes to. Nothing is sent until you turn notifications on"
            value: page.cfg && page.cfg.topic ? "Copy" : "Make one"
            onActivated: {
                if (page.st && page.st.url) { Quickshell.execDetached(["wl-copy", page.st.url]); page.say("Copied ✓ Paste it in ntfy's Subscribe to topic (with the server if it isn't ntfy.sh)"); }
                else page.set("topic", "new", "Topic made: subscribe to it in the ntfy app");
            }
        }
        SetRow {
            visible: !!page.cfg && page.cfg.topic !== ""
            icon: "restart"; title: "Make a new topic"
            sub: "If someone else got the old one. Subscribe to the new one on your phone afterwards"
            onActivated: page.set("topic", "new", "New topic made: subscribe to it on your phone")
        }
        SetRow {
            icon: "globe"; title: "Server"
            sub: page.cfg ? page.cfg.server + (page.cfg.server === "https://ntfy.sh" ? " (free, run by ntfy's maker)" : "") + " · your own ntfy is in Server apps" : ""
            value: "Change"
            onActivated: page.serverOpen = !page.serverOpen
        }
        SetField {
            visible: page.serverOpen
            Layout.leftMargin: 56; Layout.rightMargin: 18; Layout.topMargin: 6; Layout.bottomMargin: 6
            placeholder: "https://ntfy.sh, or your own like http://this-pc:2586, then Enter"
            onSubmitted: t => { page.set("server", t.trim(), "Server saved ✓"); page.serverOpen = false; text = ""; }
        }
    }
    SetCardGroup {
        title: "Notifications"
        visible: !!page.cfg
        SetSwitch {
            icon: "bell"; title: "Send notifications to my phone"
            sub: !page.cfg || !page.cfg.topic ? "Make a topic first" : "Checked every 5 minutes; the same problem is sent again after a day if it's still there"
            checked: !!page.cfg && page.cfg.on
            onToggled: v => page.set("on", v ? "on" : "off", v ? "Phone notifications on" : "Phone notifications off")
        }
        SetSwitch {
            icon: "warn"; title: "Home server alerts"
            sub: "What Alerts & power finds: a drive failing or very hot, a drive filling up, a backup failing or late, a mirror missing a drive (turn alerts on there too)"
            checked: !!page.cfg && page.cfg.alerts
            onToggled: v => page.set("alerts", v ? "on" : "off")
        }
        SetSwitch {
            icon: "server"; title: "Server apps that stop"
            sub: "A game server or app that stopped and couldn't start again"
            checked: !!page.cfg && page.cfg.apps
            onToggled: v => page.set("apps", v ? "on" : "off")
        }
        SetRow {
            visible: !!page.cfg && page.cfg.topic !== ""
            icon: "check"; title: "Send a test"
            sub: page.st && page.st.last ? (page.st.last.ok ? "The last one went out" : "The last one didn't go out: " + (page.st.last.error || "")) : "One notification now, to check your phone gets it"
            onActivated: act.run(["ntfy-test"], () => { page.say("Sent ✓ Check your phone"); page.refresh(); }, e => { page.say(e, true); page.refresh(); })
        }
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
