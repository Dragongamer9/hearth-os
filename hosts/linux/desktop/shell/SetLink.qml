// Hearth Settings › Hearth Link: your Hearth devices (hosts/hearth-link). Pair with a code, see your devices, the
// games being copied here, and turn sync on or off. Games are picked in Hearth itself (Library › Get from another
// Hearth). Everything goes through hearth-link-settings (the service's socket).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Hearth Link"
    property var st: null
    property string message: ""
    property bool messageBad: false
    property string joining: ""
    function say(t, bad) { message = t; messageBad = !!bad; }
    function refresh() { job.run(["status"], r => page.st = r, e => { page.st = { error: e }; }); }
    function bytes(n) { return n >= 1e9 ? (n / 1e9).toFixed(1) + " GB" : Math.round((n || 0) / 1e6) + " MB"; }
    function jobLine(t) {
        const names = { waiting: "Waiting for the other device", interrupted: "Interrupted", preparing: "Getting ready", comparing: "Comparing with what's here",
                        copying: "Copying", finishing: "Finishing", paused: "Paused", done: "Done", error: "Didn't work", cancelled: "Cancelled" };
        let s = names[t.state] || t.state;
        if (t.state === "copying" && t.total) s += " · " + Math.floor(100 * t.done / t.total) + "% of " + bytes(t.total) + (t.speed ? " · " + bytes(t.speed) + "/s" : "");
        if (t.state === "done" && t.note) s += " · " + t.note;
        if (t.state === "error" && t.error) s += ": " + t.error;
        if (t.relay && t.state !== "done") s += " · through a relay (slower)";
        return s;
    }
    Component.onCompleted: refresh()
    Timer { interval: 2000; running: page.visible; repeat: true; onTriggered: if (!job.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-link-settings" }
    SetJob { id: act; helper: "hearth-link-settings" }
    function run(args, ok, input) { act.run(args, () => { if (ok) page.say(ok); page.refresh(); }, e => page.say(e, true), input || ""); }

    Text {
        visible: !!page.st && !!page.st.error
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.st && page.st.error ? page.st.error : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetHeader { text: "This device"; visible: !!page.st && !!page.st.me }
    SetRow {
        visible: !!page.st && !!page.st.me
        icon: "display"; chevron: false; title: page.st && page.st.me ? page.st.me.name : ""
        sub: page.st && page.st.me ? "Its link, for pairing from somewhere else: " + page.st.me.ticket : ""
    }
    SetField { id: newName; visible: !!page.st && !!page.st.me; placeholder: "Rename this device"; onSubmitted: t => { if (t.trim()) page.run(["set", "name", t.trim()], "Renamed"); newName.text = ""; } }
    SetRow {
        visible: !!page.st && !!page.st.me && !page.st.pairing
        icon: "lock"; title: "Pair a device"; sub: "Shows a code to enter on your other Hearth device"
        onActivated: page.run(["pair-open"])
    }
    SetRow {
        visible: !!page.st && !!page.st.pairing
        icon: "lock"; chevron: false
        title: page.st && page.st.pairing ? "Code: " + page.st.pairing.code.slice(0, 3) + " " + page.st.pairing.code.slice(3) : ""
        sub: "Enter it on the other device: Settings › Hearth Link. Select to stop pairing."
        onActivated: page.run(["pair-close"])
    }
    SetHeader { text: "Hearth devices on this network"; visible: !!page.st && (page.st.nearby || []).length > 0 }
    Repeater {
        model: page.st ? (page.st.nearby || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "display"; title: modelData.name
            sub: page.joining === modelData.id ? "Type the code it shows below, then Enter" : modelData.pairing ? "Showing a pairing code: select to enter it" : "Open Pair a device on it first"
            onActivated: { page.joining = modelData.id; code.focusInput(); }
        }
    }
    SetField {
        id: code
        visible: page.joining !== ""
        placeholder: "The 6-digit code"
        onSubmitted: t => { page.run(["join", page.joining], "Paired", t); page.joining = ""; code.text = ""; }
    }
    SetField {
        id: far
        visible: !!page.st && !!page.st.me
        placeholder: "Pair with a device somewhere else: paste its link, then Enter"
        onSubmitted: t => { if (t.trim()) { page.joining = t.trim(); far.text = ""; code.focusInput(); } }
    }
    SetHeader { text: "Your devices"; visible: !!page.st && !!page.st.peers }
    Repeater {
        model: page.st ? (page.st.peers || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "display"; title: modelData.name
            sub: (modelData.online ? "Online" : "Not on right now") + " · select to unpair"
            value: modelData.online ? "Online" : ""
            onActivated: page.run(["unpair", modelData.id], "Unpaired " + modelData.name)
        }
    }
    SetHeader { text: "Copies"; visible: !!page.st && (page.st.transfers || []).length > 0 }
    Repeater {
        model: page.st ? (page.st.transfers || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "store"; title: modelData.title + (modelData.from ? " from " + modelData.from : "")
            sub: page.jobLine(modelData) + (["copying", "waiting", "preparing", "comparing"].includes(modelData.state) ? " · select to pause" : ["paused", "error", "interrupted"].includes(modelData.state) ? " · select to resume" : "")
            onActivated: {
                if (["copying", "waiting", "preparing", "comparing"].includes(modelData.state)) page.run(["pause", modelData.id], "Paused");
                else if (["paused", "error", "interrupted"].includes(modelData.state)) page.run(["resume", modelData.id], "Resumed");
            }
        }
    }
    SetRow {
        visible: !!page.st && (page.st.peers || []).length > 0
        icon: "store"; title: "Get a game from another Hearth"; sub: "Picked in Game Mode: Library › Get from another Hearth. Select to switch to Game Mode."
        onActivated: Quickshell.execDetached(["nimbus-mode", "game"])
    }
    Repeater {
        model: page.st ? (page.st.signInHints || []) : []
        delegate: Text { required property var modelData; Layout.fillWidth: true; Layout.leftMargin: 4; text: modelData.text; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
    }
    SetHeader { text: "Settings"; visible: !!page.st && !!page.st.settings }
    SetSwitch {
        visible: !!page.st && !!page.st.settings
        icon: "update"; title: "Keep Hearth data in sync"
        sub: "Favorites, collections, play time, per-game settings, profile, appearance, controller layouts. Never sign-ins."
        checked: !!page.st && !!page.st.settings && page.st.settings.sync
        onToggled: v => page.run(["set", "sync", v ? "on" : "off"])
    }
    SetSwitch {
        visible: !!page.st && !!page.st.settings
        icon: "store"; title: "Accept games from my devices without asking"
        checked: !!page.st && !!page.st.settings && page.st.settings.alwaysAccept
        onToggled: v => page.run(["set", "alwaysAccept", v ? "on" : "off"])
    }
    SetChoice {
        visible: !!page.st && !!page.st.settings
        title: "Relay"; sub: "Only when two devices can't connect directly over the internet; the data stays encrypted end to end"
        options: [{ value: "default", label: "n0's public relays" }, { value: "off", label: "Off (same network only)" }]
        current: page.st && page.st.settings ? page.st.settings.relay : "default"
        onChosen: v => page.run(["set", "relay", v], "Hearth Link restarts with the new setting")
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
