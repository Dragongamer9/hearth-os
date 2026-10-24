// Hearth Settings › Hearth Link: your Hearth devices (hosts/hearth-link). Pair with a code, see your devices, the
// games being copied here, and turn sync on or off. Games are picked in Hearth itself (Library › Get from another
// Hearth). Everything goes through hearth-link-settings (the service's socket).
// Pairing: "Pair a device" shows a 6-digit code (the devices on this network list this one as ready) and, when
// asked, a short code for a device somewhere else. Pairing with another: pick it from the list and type its code,
// or type a short code; a wrong code keeps the field open with the tries left.
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
    property string joiningName: ""
    property bool pairingBusy: false
    readonly property var kinds: ({ handheld: "Handheld", laptop: "Laptop", desktop: "Desktop", tablet: "Tablet", tv: "TV" })
    readonly property string abc: "23456789ABCDEFGHJKMNPQRSTUVWXYZ"
    function say(t, bad) { message = t; messageBad = !!bad; }
    function minsLeft(until) { const m = Math.max(0, Math.ceil((until - Date.now()) / 60000)); return m <= 1 ? "less than a minute left" : m + " min left"; }
    function what(n) { return [kinds[n.kind] || "", n.edition || ""].filter(x => x).join(" · "); }
    // Short codes (hosts/hearth-link shortcode.rs): K7M-4QX-92HC, no 0/O/1/I/L, a check character.
    function squash(t) { return (t || "").replace(/[\s\-._·]/g, "").toUpperCase(); }
    function shortFmt(t) { const c = squash(t).slice(0, 10); return [c.slice(0, 3), c.slice(3, 6), c.slice(6)].filter(x => x).join("-"); }
    function shortCheck(t) {
        const c = squash(t);
        if (/[01OIL]/.test(c)) return "That code doesn't look right: codes never have 0, O, 1, I or L in them";
        for (let i = 0; i < c.length; i++) if (abc.indexOf(c[i]) < 0) return "That code doesn't look right: \"" + c[i] + "\" isn't in codes";
        if (c.length !== 10) return "That code doesn't look right: it's 10 characters, and this is " + c.length;
        let sum = 0;
        for (let i = 0; i < 10; i++) sum += (i + 1) * abc.indexOf(c[i]);
        return sum % 31 ? "That code doesn't look right: one of the characters is off. Check it on the other device" : "";
    }
    function eventLine(ev) {
        if (!ev) return "Waiting for the other device…";
        if (ev.state === "checking") return "Found " + ev.name + ": checking the code…";
        if (ev.state === "wrong") return ev.name + " typed a wrong code (" + ev.triesLeft + (ev.triesLeft === 1 ? " try" : " tries") + " left)";
        return "Waiting for the other device…";
    }
    function joiningLine() {
        const j = page.st && page.st.joining;
        if (!j) return "Pairing…";
        return ({ looking: "Looking up the code…", found: "Found " + j.name, connecting: "Found " + j.name + ": connecting…", checking: "Checking the code with " + j.name + "…" })[j.state] || "Pairing…";
    }
    // A pairing: a wrong code keeps the field open (with the tries left), anything else closes it.
    function join(args, input, okText) {
        page.pairingBusy = true;
        page.say("");
        act.run(args, r => { page.pairingBusy = false; page.joining = ""; page.say("Paired ✓ with " + ((r && r.name) || page.joiningName)); page.refresh(); },
                e => { page.pairingBusy = false; const wrong = /^Wrong code/.test(e) && !/last try/.test(e); if (!wrong) page.joining = ""; page.say(e, true); if (wrong) code.focusInput(); page.refresh(); }, input || "");
    }
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
        sub: page.st && page.st.me ? [page.what(page.st.me), "Its name on your other Hearth devices"].filter(x => x).join(" · ") : ""
    }
    SetField { id: newName; visible: !!page.st && !!page.st.me; placeholder: "Rename this device"; onSubmitted: t => { if (t.trim()) page.run(["set", "name", t.trim()], "Renamed"); newName.text = ""; } }
    SetRow {
        visible: !!page.st && !!page.st.me && !page.st.pairing
        icon: "lock"; title: "Pair a device"
        sub: page.st && page.st.pairEvent && page.st.pairEvent.state === "paired" ? "Paired ✓ with " + page.st.pairEvent.name + ". Select to pair another"
           : page.st && page.st.pairEvent && page.st.pairEvent.state === "locked" ? "Too many wrong codes, so pairing stopped. Select for a new code"
           : "Shows a code to type on your other Hearth device"
        onActivated: page.run(["pair-open"])
    }
    SetRow {
        visible: !!page.st && !!page.st.pairing && !!page.st.pairing.code
        icon: "lock"; chevron: false
        title: page.st && page.st.pairing && page.st.pairing.code ? "Code: " + page.st.pairing.code.slice(0, 3) + " " + page.st.pairing.code.slice(3) : ""
        sub: page.st && page.st.pairing ? page.eventLine(page.st.pairEvent) + " On the other device: Settings › Hearth Link, pick " + page.st.me.name + " and type this code. " + page.minsLeft(page.st.pairing.until) + "." : ""
    }
    SetRow {
        visible: !!page.st && !!page.st.pairing && !!page.st.pairing.netCode
        icon: "globe"; chevron: false
        title: page.st && page.st.pairing && page.st.pairing.netCode ? "Code for somewhere else: " + page.st.pairing.netCode : ""
        sub: page.st && page.st.pairing && page.st.pairing.netCode ? "On the other device: Settings › Hearth Link › Enter a code. Works once, " + page.minsLeft(page.st.pairing.netUntil) + "." : ""
    }
    SetRow {
        visible: !!page.st && !!page.st.pairing && !page.st.pairing.netCode && page.st.shortCodes !== false
        icon: "globe"; title: "Not on the same network?"; sub: "Get a code to type on a device somewhere else (over the internet)"
        onActivated: page.run(["pair-open-internet"])
    }
    SetRow {
        visible: !!page.st && !!page.st.pairing
        icon: "close"; title: "Stop pairing"; chevron: false
        onActivated: page.run(["pair-close"])
    }
    SetHeader { text: "Pair with a device"; visible: !!page.st && !!page.st.me }
    SetRow {
        visible: page.pairingBusy
        icon: "update"; chevron: false; title: page.joiningLine()
    }
    Repeater {
        model: page.st ? (page.st.nearby || []) : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.kind === "handheld" || modelData.kind === "tablet" ? "pad" : "display"; title: modelData.name
            chevron: modelData.pairing
            sub: (page.what(modelData) ? page.what(modelData) + " · " : "")
                 + (page.joining === modelData.id ? "Type the 6-digit code it shows below, then Enter" : modelData.pairing ? "Ready to pair: select, then type the code it shows" : "To pair, select Pair a device on it first")
            onActivated: {
                if (!modelData.pairing) { page.say("On " + modelData.name + ", select Settings › Hearth Link › Pair a device first", true); return; }
                page.joining = modelData.id; page.joiningName = modelData.name; page.say(""); code.focusInput();
            }
        }
    }
    Text {
        visible: !!page.st && !!page.st.me && !(page.st.nearby || []).some(n => n.pairing)
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Looking for Hearth devices on this network. On the other device, select Settings › Hearth Link › Pair a device."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetField {
        id: code
        visible: page.joining !== ""
        placeholder: "The 6-digit code " + (page.joiningName || "it") + " shows"
        onSubmitted: t => { const c = t.replace(/\D/g, ""); if (c.length !== 6) { page.say("The code is 6 digits", true); return; } page.join(["join", page.joining], c); code.text = ""; }
    }
    SetField {
        id: shortField
        visible: !!page.st && !!page.st.me && page.st.shortCodes !== false
        placeholder: "Enter a code from a device somewhere else (like K7M-4QX-92HC), then Enter"
        onTextChanged: { const f = page.shortFmt(text); if (f !== text && page.squash(text).length <= 10) text = f; }
        onSubmitted: t => {
            const bad = page.shortCheck(t);
            if (bad) { page.say(bad, true); return; }
            page.joiningName = "the other device"; page.join(["join-code"], page.shortFmt(t)); shortField.text = "";
        }
    }
    SetRow {
        visible: !!page.st && !!page.st.me
        icon: "info"; chevron: false; title: "Other ways: this device's long link"
        sub: page.st && page.st.me ? page.st.me.ticket : ""
    }
    SetField {
        id: far
        visible: !!page.st && !!page.st.me
        placeholder: "Other ways: paste the other device's long link, then Enter"
        onSubmitted: t => { if (t.trim()) { page.joining = t.trim(); page.joiningName = "the other device"; far.text = ""; code.focusInput(); } }
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
