// Firehold's Settings › Couch and Desk (mockup docs/mockups/firehold Posture, 1920 × 1080): Automatic / Always Couch /
// Always Desk, Per screen, Start in, Guide button, Games and Big Picture. Every choice is one key merged into
// ~/.config/nimbus/firehold.json (Firehold.qml's set: SettingsWriter); opening it writes nothing. The list on the left
// opens Hearth Settings at that page (the same Settings, with the controller's look). Controller: D-pad, A changes,
// B / Esc closes.
import QtQuick
import Quickshell
import Quickshell.Wayland
import "firehold.js" as F

PanelWindow {
    id: st
    property var fh
    readonly property var pal: F.C
    screen: Quickshell.screens[0]
    readonly property real k: Math.min((screen ? screen.width : 1920) / 1920, (screen ? screen.height : 1080) / 1080) || 1
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-firehold-settings"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: pal.bg
    Component.onCompleted: { fh.settingsPanel = st; keys.forceActiveFocus(); }
    Component.onDestruction: if (fh.settingsPanel === st) fh.settingsPanel = null

    readonly property var nav: [["system", "System"], ["", "Couch and Desk"], ["display", "Display"], ["gaming", "Gaming"], ["controller", "Controllers"],
                                ["wifi", "Network"], ["sound", "Sound"], ["accounts", "Accounts"], ["updates", "Updates"]]
    readonly property var modes: [
        ["auto", "Automatic", "Controller → Couch, mouse or keyboard → Desk"],
        ["couch", "Always Couch", "A console: tiles, full screen, Guide button"],
        ["desk", "Always Desk", "A PC: windows, bar and dock"]
    ]
    readonly property var screensInfo: Quickshell.screens.map(s => ({ name: s.name, label: F.screenLabel(s.name, s.model, 0) }))
    property bool perOpen: false
    // [key, title, sub, value, choices]
    readonly property var rows: {
        const r = [["perScreen", "Per screen", "The TV stays Couch while the monitor is Desk", F.perScreenText(fh.perScreen, screensInfo), null]];
        if (perOpen) for (const s of screensInfo) {
            const v = fh.perScreen[s.name] === "couch" || fh.perScreen[s.name] === "desk" ? fh.perScreen[s.name] : "";
            r.push(["screen:" + s.name, "    " + s.label + " (" + s.name + ")", "Its own posture, or the same as the others", v === "couch" ? "Couch" : v === "desk" ? "Desk" : "Same as the others", ["", "couch", "desk"]]);
        }
        r.push(["startIn", "Start in", "What you see when the PC turns on", F.START[fh.startIn], ["couch", "desk", "last"]]);
        r.push(["guide", "Guide button", "Press once: Quick Access. Twice: everything open", F.GUIDE[fh.guideButton], ["quick", "open"]]);
        r.push(["games", "Games", "Open full screen on the Couch, in a window on the Desk", F.GAMES[fh.gamesOpen], ["follow", "fullscreen", "window"]]);
        r.push(["bigPicture", "Big Picture", "Steam's own TV mode isn't needed; Steam is an app like the others", fh.bigPicture ? "On" : "Off", [false, true]]);
        return r;
    }
    function current(key) {
        if (key === "startIn") return fh.startIn;
        if (key === "guide") return fh.guideButton;
        if (key === "games") return fh.gamesOpen;
        if (key === "bigPicture") return fh.bigPicture;
        if (key.startsWith("screen:")) { const v = fh.perScreen[key.slice(7)]; return v === "couch" || v === "desk" ? v : ""; }
        return null;
    }
    function change(i, d) {
        const r = rows[i];
        if (!r) return;
        if (r[0] === "perScreen") { perOpen = !perOpen; return; }
        const ch = r[4], cur = current(r[0]);
        const n = ch[(ch.indexOf(cur) + (d < 0 ? ch.length - 1 : 1)) % ch.length];
        if (r[0].startsWith("screen:")) {
            const ps = Object.assign({}, fh.perScreen);
            if (n === "") delete ps[r[0].slice(7)]; else ps[r[0].slice(7)] = n;
            fh.set("perScreen", ps);
        } else fh.set(r[0], n);
    }

    // ---- focus: nav (i) | cards (i) | rows (i)
    property string zone: "cards"
    property int idx: Math.max(0, modes.findIndex(m => m[0] === fh.mode))
    function press(b) {
        if (b === "b" || b === "menu") { fh.settingsShown = false; return; }
        if (zone === "nav") {
            if (b === "up") idx = Math.max(0, idx - 1);
            else if (b === "down") idx = Math.min(nav.length - 1, idx + 1);
            else if (b === "right") { zone = "cards"; idx = Math.max(0, modes.findIndex(m => m[0] === fh.mode)); }
            else if (b === "a") openNav(idx);
            return;
        }
        if (zone === "cards") {
            if (b === "left") { if (idx > 0) idx--; else { zone = "nav"; idx = 1; } }
            else if (b === "right") idx = Math.min(2, idx + 1);
            else if (b === "down") { zone = "rows"; idx = 0; }
            else if (b === "a") fh.set("posture", modes[idx][0]);
            return;
        }
        if (b === "up") { if (idx > 0) idx--; else { zone = "cards"; idx = Math.max(0, modes.findIndex(m => m[0] === fh.mode)); } }
        else if (b === "down") idx = Math.min(rows.length - 1, idx + 1);
        else if (b === "a" || b === "right") change(idx, 1);
        else if (b === "left") { if (rows[idx] && rows[idx][4]) change(idx, -1); else { zone = "nav"; idx = 1; } }
    }
    function openNav(i) {
        const p = nav[i][0];
        if (p === "") return;
        fh.settingsShown = false;
        if (fh.settingsApp) fh.settingsApp.open(p);
    }
    Item {
        id: keys
        focus: true
        Keys.onPressed: e => {
            const m = { [Qt.Key_Left]: "left", [Qt.Key_Right]: "right", [Qt.Key_Up]: "up", [Qt.Key_Down]: "down", [Qt.Key_Return]: "a", [Qt.Key_Enter]: "a", [Qt.Key_Space]: "a", [Qt.Key_Escape]: "b" }[e.key];
            if (m) { st.press(m); e.accepted = true; }
        }
    }

    component T: Text { color: F.C.text; font.family: F.font(Theme.font); textFormat: Text.PlainText; elide: Text.ElideRight }
    FhBackdrop { anchors.fill: parent; k: st.k; c0: "#3a1d0e"; c1: "#1a110c" }
    TapHandler { onTapped: (ev) => { const p = ev.position; if (p.x < frame.x || p.y < frame.y || p.x > frame.x + frame.width || p.y > frame.y + frame.height) st.fh.settingsShown = false; } }
    Rectangle {
        id: frame
        x: 200 * st.k; y: 70 * st.k; width: 1520 * st.k; height: 940 * st.k; radius: 22 * st.k
        color: st.pal.panel; border.width: 1; border.color: st.pal.line; clip: true
        // The list of pages
        Rectangle {
            x: 1; y: 1; width: 340 * st.k; height: parent.height - 2; color: st.pal.panel2; radius: 21 * st.k
            Rectangle { anchors { right: parent.right; top: parent.top; bottom: parent.bottom } width: 22 * st.k; color: st.pal.panel2 }
            Column {
                x: 18 * st.k; y: 32 * st.k; width: parent.width - 36 * st.k
                spacing: 6 * st.k
                Repeater {
                    model: st.nav
                    delegate: Rectangle {
                        id: nv
                        required property var modelData
                        required property int index
                        readonly property bool on: modelData[0] === ""
                        readonly property bool ring: st.zone === "nav" && st.idx === index
                        width: parent.width; height: nt.implicitHeight + 26 * st.k; radius: 12 * st.k
                        color: on ? st.pal.card2 : "transparent"
                        border.width: ring ? 2 * st.k : 0; border.color: st.pal.accent
                        T { id: nt; x: 16 * st.k; anchors.verticalCenter: parent.verticalCenter; text: nv.modelData[1]; font.pixelSize: 17 * st.k; font.weight: nv.on ? Font.Bold : Font.Medium; color: nv.on ? st.pal.text : st.pal.muted }
                        TapHandler { onTapped: st.openNav(nv.index) }
                    }
                }
            }
        }
        // The page
        Column {
            x: 340 * st.k + 52 * st.k; y: 40 * st.k; width: parent.width - 340 * st.k - 104 * st.k
            spacing: 24 * st.k
            T { text: "Couch and Desk"; font.pixelSize: 34 * st.k; font.weight: Font.ExtraBold }
            T {
                width: Math.min(parent.width, 900 * st.k); wrapMode: Text.WordWrap; elide: Text.ElideNone; font.pixelSize: 17 * st.k; color: st.pal.soft
                text: "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything."
            }
            Row {
                spacing: 18 * st.k
                Repeater {
                    model: st.modes
                    delegate: Rectangle {
                        id: md
                        required property var modelData
                        required property int index
                        readonly property bool on: st.fh.mode === modelData[0]
                        readonly property bool ring: st.zone === "cards" && st.idx === index
                        width: (st.width > 0 ? (1520 * st.k - 340 * st.k - 104 * st.k - 36 * st.k) / 3 : 300); height: mdCol.implicitHeight + 36 * st.k; radius: 16 * st.k
                        color: st.pal.panel2
                        border.width: 2 * st.k; border.color: on ? st.pal.accent : st.pal.line
                        Rectangle { visible: md.ring && !md.on; anchors { fill: parent; margins: -5 * st.k } radius: 20 * st.k; color: "transparent"; border.width: 2 * st.k; border.color: st.pal.flame }
                        Column {
                            id: mdCol
                            x: 16 * st.k; y: 16 * st.k; width: parent.width - 32 * st.k
                            spacing: 10 * st.k
                            Item {
                                width: parent.width; height: 130 * st.k
                                // Automatic: half Couch, half Desk; Always Couch: the ember gradient; Always Desk: plain.
                                FhGrad { visible: md.index === 1; anchors.fill: parent; radius: 12 * st.k; angle: 110; stops: [[0, "#2b1a2e"], [1, "#a3541e"]] }
                                Rectangle { visible: md.index === 2; anchors.fill: parent; radius: 12 * st.k; color: "#2a2420" }
                                FhGrad { visible: md.index === 0; anchors.fill: parent; radius: 12 * st.k; angle: 90; stops: [[0, "#5b2b1a"], [0.5, "#5b2b1a"], [0.5, "#2a2420"], [1, "#2a2420"]] }
                            }
                            T { text: md.modelData[1]; font.pixelSize: 18 * st.k; font.weight: Font.Bold }
                            T { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; text: md.modelData[2]; font.pixelSize: 14 * st.k; color: st.pal.muted }
                        }
                        TapHandler { onTapped: { st.zone = "cards"; st.idx = md.index; st.fh.set("posture", md.modelData[0]); } }
                    }
                }
            }
            Rectangle {
                width: parent.width; height: rowsCol.implicitHeight; radius: 16 * st.k; color: st.pal.panel2; clip: true
                Column {
                    id: rowsCol
                    width: parent.width
                    Repeater {
                        model: st.rows
                        delegate: Rectangle {
                            id: rw
                            required property var modelData
                            required property int index
                            readonly property bool ring: st.zone === "rows" && st.idx === index
                            width: parent.width; height: rwCol.implicitHeight + 36 * st.k
                            color: ring ? st.pal.card : "transparent"
                            border.width: ring ? 2 * st.k : 0; border.color: st.pal.accent; radius: ring ? 12 * st.k : 0
                            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: st.pal.card }
                            Column {
                                id: rwCol
                                x: 22 * st.k; anchors.verticalCenter: parent.verticalCenter; width: parent.width - pill.width - 64 * st.k
                                spacing: 3 * st.k
                                T { width: parent.width; text: rw.modelData[1]; font.pixelSize: 17 * st.k; font.weight: Font.DemiBold }
                                T { width: parent.width; text: rw.modelData[2]; font.pixelSize: 14 * st.k; color: st.pal.muted }
                            }
                            Rectangle {
                                id: pill
                                anchors { right: parent.right; rightMargin: 22 * st.k; verticalCenter: parent.verticalCenter }
                                width: pv.implicitWidth + 32 * st.k; height: pv.implicitHeight + 20 * st.k; radius: 12 * st.k; color: st.pal.card
                                T { id: pv; anchors.centerIn: parent; text: rw.modelData[3]; font.pixelSize: 15 * st.k; font.weight: Font.Bold }
                            }
                            TapHandler { onTapped: { st.zone = "rows"; st.idx = rw.index; st.change(rw.index, 1); } }
                        }
                    }
                }
            }
        }
    }
}
