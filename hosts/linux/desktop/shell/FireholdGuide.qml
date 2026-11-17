// Firehold's Guide panel over a game (the Guide button, on the Couch; mockup docs/mockups/firehold InGame, 1920 × 1080):
// the game, six quick tiles, the game's display and power settings, and Put it in a window / Home / Quit game. The game
// keeps running underneath. What works today is wired (screenshots and recording through Capture and the recorder, Discord,
// the on-screen keyboard, volume, brightness, VRR and tearing through hearth-screens on KWin, power through hearth-power,
// the performance overlay); the frame limit and resolution scaling need Firehold's own compositor and say so.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "firehold.js" as F

PanelWindow {
    id: gp
    property var fh
    readonly property var C: F.C
    readonly property var game: fh.frontGame()
    readonly property var info: game ? (F.gameFor(game, fh.library) || { title: game.title || "Game", store: "Game" }) : { title: "", store: "" }
    readonly property var gameScreen: game && game.screens && game.screens.length ? game.screens[0] : Quickshell.screens[0]
    screen: gameScreen
    readonly property real k: Math.min((screen ? screen.height : 1080) / 1080, (screen ? screen.width : 1920) / 1920) || 1
    readonly property string font: F.font(Theme.font)

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-firehold-guide"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; right: true }
    implicitWidth: 560 * k
    color: "transparent"

    Component.onCompleted: { fh.guidePanel = gp; refresh(); }
    Component.onDestruction: if (fh.guidePanel === gp) fh.guidePanel = null

    // ------------------------------------------------------------------ focus: tiles (0-5), rows (0-5), buttons (0-2)
    property string zone: "tiles"
    property int idx: 0
    function press(b) {
        if (b === "b" || b === "menu") { fh.guideShown = false; return; }
        if (zone === "tiles") {
            const r = Math.floor(idx / 3), c = idx % 3;
            if (b === "left") { if (c > 0) idx--; else if (idx === 4) volume(-1); }
            else if (b === "right") { if (c < 2) idx++; }
            else if (b === "up") { if (r > 0) idx -= 3; }
            else if (b === "down") { if (r === 0) idx += 3; else { zone = "rows"; idx = 0; } }
            else if (b === "a") tile(idx);
            else if (b === "lb" || b === "lt") { if (idx === 4) volume(-1); else if (idx === 5) bright(false); }
            else if (b === "rb" || b === "rt") { if (idx === 4) volume(1); else if (idx === 5) bright(true); }
            return;
        }
        if (zone === "rows") {
            if (b === "up") { if (idx > 0) idx--; else { zone = "tiles"; idx = 3; } }
            else if (b === "down") { if (idx < rows.length - 1) idx++; else { zone = "buttons"; idx = 0; } }
            else if (b === "left" || b === "right" || b === "a") step(idx, b === "left" ? -1 : 1);
            return;
        }
        if (b === "up") { if (idx > 0) idx--; else { zone = "rows"; idx = rows.length - 1; } }
        else if (b === "down") { if (idx < 2) idx++; }
        else if (b === "a") button(idx);
    }

    // ------------------------------------------------------------------ the quick tiles
    readonly property var shell: ["quickshell", "ipc", "-p", Quickshell.shellDir, "call"]
    property bool recording: false
    function tile(i) {
        if (i === 0) { fh.guideShown = false; Quickshell.execDetached(["sh", "-c", 'sleep 0.5; exec "$@"', "sh"].concat(shell, ["capture", "take", "screen"])); }
        else if (i === 1) { Quickshell.execDetached(shell.concat(["record", "startStop"])); recording = !recording; }
        else if (i === 2) { fh.guideShown = false; Quickshell.execDetached(shell.concat(["discord", "open"])); }
        else if (i === 3) { if (fh.pad) fh.pad.openKeyboard(); }
        else if (i === 4) volume(1);
        else if (i === 5) bright(true);
    }
    readonly property var sink: Buttons.sink
    readonly property int vol: sink && sink.audio ? Math.round(sink.audio.volume * 100) : -1
    function volume(d) { Buttons.volume(d, true); }
    property int brightness: -1
    function bright(up) { brightProc.command = ["brightnessctl", "-m", "-c", "backlight", "set", up ? "5%+" : "5%-"]; brightProc.running = true; }
    Process {
        id: brightProc
        command: ["brightnessctl", "-m", "-c", "backlight", "info"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim().split(","); gp.brightness = f.length >= 4 ? parseInt(f[3]) : -1; } }
    }
    readonly property var tiles: [
        ["Screenshot", "Saved to Pictures"], ["Record", recording ? "Recording… A to stop" : "Start recording"], ["Friends", "Discord"],
        ["Keyboard", "On-screen"], ["Volume", vol >= 0 ? vol + "%" : "No sound device"], ["Brightness", brightness >= 0 ? brightness + "%" : "Set by the screen"]
    ]

    // ------------------------------------------------------------------ the settings rows
    property var screens: ({})        // hearth-screens info
    property var power: ({})          // hearth-power status
    readonly property string outName: gameScreen ? gameScreen.name : ""
    readonly property var out: (screens.outputs || []).find(o => o.name === outName) || null
    readonly property bool kwin: WinList.kwin
    function refresh() { infoProc.running = true; powerProc.running = true; brightProc.command = ["brightnessctl", "-m", "-c", "backlight", "info"]; brightProc.running = true; }
    Process {
        id: infoProc
        command: ["python3", Quickshell.shellDir + "/hearth-screens", "info"]
        stdout: StdioCollector { onStreamFinished: { try { gp.screens = JSON.parse(text.trim().split("\n").pop()); } catch (e) {} } }
    }
    Process {
        id: powerProc
        command: ["python3", Quickshell.shellDir + "/hearth-power", "status"]
        stdout: StdioCollector { onStreamFinished: { try { gp.power = JSON.parse(text.trim().split("\n").pop()); } catch (e) {} } }
    }
    Process {
        id: setProc
        stdout: StdioCollector { onStreamFinished: { const l = text.trim().split("\n").pop() || ""; if (l.startsWith("error: ")) SettingsNav.toast(l.slice(7)); gp.refresh(); } }
    }
    function runSet(cmd) { setProc.command = cmd; setProc.running = true; }
    readonly property var vrr: out && out.vrr ? out.vrr : { capable: false, policy: "never" }
    readonly property string powerMode: power.mode && power.mode.supported ? (power.mode.active || "") : ""
    readonly property var tdp: power.tdp && power.tdp.supported ? power.tdp : ({})
    readonly property bool tearCapable: !!(screens.tearing && screens.tearing.capable)
    readonly property bool tearOn: !!(screens.tearing && screens.tearing.on)
    readonly property var modes: ["power-saver", "balanced", "performance"]
    readonly property var modeNames: ({ "power-saver": "Battery saver", balanced: "Balanced", performance: "Performance" })
    readonly property var overlays: ["off", "basic", "full"]
    // [title, sub, value, colour, works today]
    readonly property var rows: [
        ["Frame limit", "Steadier, cooler, longer battery", "Not yet", C.muted, false],
        ["Resolution scaling", "Hearth upscales the game itself", "Not yet", C.muted, false],
        ["VRR / FreeSync", "Smooth when it dips below 60", !kwin || !vrr.capable ? "Not on this screen" : vrr.policy === "never" ? "Off" : "On", kwin && vrr.capable && vrr.policy !== "never" ? C.green : C.muted, kwin && vrr.capable],
        ["Allow tearing", "Lowest delay for fast games", !tearCapable ? "Needs KWin" : tearOn ? "On" : "Off", tearCapable && tearOn ? C.green : C.muted, tearCapable],
        ["Power", "Handhelds and laptops", powerMode ? (modeNames[powerMode] || powerMode) + (tdp.watts ? " · " + Math.round(tdp.watts) + " W" : "") : "Set by this PC", C.text, powerMode !== ""],
        ["Performance overlay", "Top-left corner", fh.overlay === "off" ? "Off" : fh.overlay === "basic" ? "Basic" : "Full", fh.overlay === "off" ? C.muted : C.text, true]
    ]
    function step(i, d) {
        const r = rows[i];
        if (!r[4]) { SettingsNav.toast(r[0] + ": " + (i < 2 ? "comes with Firehold's own compositor" : r[2])); return; }
        if (i === 2) runSet(["python3", Quickshell.shellDir + "/hearth-screens", "set", JSON.stringify({ outputs: { [outName]: { vrr: vrr.policy === "never" ? "automatic" : "never" } } })]);
        else if (i === 3) runSet(["python3", Quickshell.shellDir + "/hearth-screens", "set", JSON.stringify({ tearing: !tearOn })]);
        else if (i === 4) { const n = modes[(modes.indexOf(powerMode) + (d > 0 ? 1 : modes.length - 1)) % modes.length]; runSet(["python3", Quickshell.shellDir + "/hearth-power", "mode", n]); }
        else if (i === 5) fh.set("overlay", overlays[(overlays.indexOf(fh.overlay) + (d > 0 ? 1 : overlays.length - 1)) % overlays.length]);
    }

    // ------------------------------------------------------------------ the buttons
    function button(i) {
        const g = game;
        if (i === 0) { fh.guideShown = false; if (g) g.fullscreen = false; fh.switchTo("desk", ""); }
        else if (i === 1) fh.home();
        else if (i === 2) { fh.guideShown = false; if (g) g.close(); }
    }

    // ------------------------------------------------------------------ drawing
    component T: Text { color: F.C.text; font.family: F.font(Theme.font); textFormat: Text.PlainText; elide: Text.ElideRight }
    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(23 / 255, 17 / 255, 14 / 255, 0.96)
        Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: 1; color: gp.C.line }
        TapHandler { }
    }
    Column {
        x: 32 * gp.k; y: 32 * gp.k; width: parent.width - 64 * gp.k
        spacing: 20 * gp.k
        Row {
            spacing: 16 * gp.k
            Item {
                width: 56 * gp.k; height: width
                FhGrad { anchors.fill: parent; radius: 14 * gp.k; angle: 110; stops: [[0, "#5b2b1a"], [1, "#a3541e"]] }
                FhArt { anchors.fill: parent; radius: 14 * gp.k; source: gp.info.icon || gp.info.cover || "" }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4 * gp.k
                T { text: gp.info.title; font.pixelSize: 22 * gp.k; font.weight: Font.ExtraBold; width: Math.min(implicitWidth, 420 * gp.k) }
                T { text: "Playing " + F.duration(clock.date.getTime() - (gp.game ? gp.fh.since(gp.game) : clock.date.getTime())) + " · native, no Game Mode"; font.pixelSize: 15 * gp.k; color: gp.C.muted }
            }
        }
        Grid {
            columns: 3
            spacing: 10 * gp.k
            Repeater {
                model: gp.tiles
                delegate: Rectangle {
                    id: qt
                    required property var modelData
                    required property int index
                    readonly property bool ring: gp.zone === "tiles" && gp.idx === index
                    width: (gp.width - 64 * gp.k - 20 * gp.k) / 3; height: 78 * gp.k; radius: 14 * gp.k
                    color: gp.C.card; border.width: ring ? 2 * gp.k : 1; border.color: ring ? gp.C.accent : gp.C.line
                    Column {
                        anchors.centerIn: parent
                        spacing: 6 * gp.k
                        T { anchors.horizontalCenter: parent.horizontalCenter; text: qt.modelData[0]; font.pixelSize: 15 * gp.k; font.weight: Font.DemiBold }
                        T { anchors.horizontalCenter: parent.horizontalCenter; text: qt.modelData[1]; font.pixelSize: 12 * gp.k; font.weight: Font.Medium; color: gp.C.muted }
                    }
                    TapHandler { onTapped: { gp.zone = "tiles"; gp.idx = qt.index; gp.tile(qt.index); } }
                }
            }
        }
        Rectangle {
            width: parent.width; height: rowCol.implicitHeight; radius: 16 * gp.k; color: gp.C.panel; clip: true
            Column {
                id: rowCol
                width: parent.width
                spacing: 4 * gp.k
                Repeater {
                    model: gp.rows
                    delegate: Rectangle {
                        id: sr
                        required property var modelData
                        required property int index
                        readonly property bool ring: gp.zone === "rows" && gp.idx === index
                        width: parent.width; height: 67 * gp.k
                        color: ring ? gp.C.card : "transparent"
                        border.width: ring ? 2 * gp.k : 0; border.color: gp.C.accent; radius: ring ? 12 * gp.k : 0
                        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: gp.C.card }
                        Column {
                            x: 18 * gp.k; anchors.verticalCenter: parent.verticalCenter
                            spacing: 2 * gp.k
                            T { text: sr.modelData[0]; font.pixelSize: 16 * gp.k; font.weight: Font.DemiBold }
                            T { text: sr.modelData[1]; font.pixelSize: 13 * gp.k; color: gp.C.muted }
                        }
                        Rectangle {
                            anchors { right: parent.right; rightMargin: 18 * gp.k; verticalCenter: parent.verticalCenter }
                            width: vt.implicitWidth + 28 * gp.k; height: vt.implicitHeight + 16 * gp.k; radius: 10 * gp.k; color: gp.C.card2
                            T { id: vt; anchors.centerIn: parent; text: "‹ " + sr.modelData[2] + " ›"; font.pixelSize: 14 * gp.k; font.weight: Font.Bold; color: sr.modelData[3] }
                        }
                        TapHandler { onTapped: { gp.zone = "rows"; gp.idx = sr.index; gp.step(sr.index, 1); } }
                    }
                }
            }
        }
    }
    Column {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 32 * gp.k }
        spacing: 10 * gp.k
        Repeater {
            model: [["Put it in a window on the Desk", false], ["Home (the game keeps running)", false], ["Quit game", true]]
            delegate: Rectangle {
                id: bt
                required property var modelData
                required property int index
                readonly property bool ring: gp.zone === "buttons" && gp.idx === index
                width: parent.width; height: 54 * gp.k; radius: 14 * gp.k
                color: modelData[1] ? "transparent" : gp.C.card2
                border.width: modelData[1] ? 2 * gp.k : (ring ? 2 * gp.k : 0)
                border.color: modelData[1] ? (ring ? gp.C.flame : gp.C.red) : gp.C.accent
                T { anchors.centerIn: parent; text: bt.modelData[0]; font.pixelSize: 16 * gp.k; font.weight: bt.modelData[1] ? Font.Bold : Font.DemiBold; color: bt.modelData[1] ? gp.C.redText : gp.C.text }
                TapHandler { onTapped: gp.button(bt.index) }
            }
        }
    }
    SystemClock { id: clock; precision: SystemClock.Minutes }
}
