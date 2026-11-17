// Hearth Capture (1.14): Hearth's own screenshot tool in Desktop Mode, in place of KDE's Spectacle. A normal window
// like Task Manager, loaded by shell.qml by file name, so a problem here only loses Capture, never the desktop.
//   Take: all screens, this screen, the window in front, or an area (Mantel's own picker over a still picture of each
//   screen), after a delay or not, with the pointer or not; then the markup editor (CaptureEditor.qml): arrows,
//   boxes, pen, highlighter, text, blur, pixelate, numbered steps, crop; copy, save, save as, show in folder, read the
//   text (tesseract) or a QR code (zbarimg). Recent: the pictures saved lately. Record: the screen recorder
//   (hearth-record, else Hearth Recorder), when one is installed.
// The picture itself is taken by hearth-capture-helper's grab() (hearth-screens when installed, KWin's ScreenShot2
// under KWin, grim on labwc). Opened from the app menu (hearth-screenshot.desktop, "Capture"), `hearth-capture`,
// open.sh capture / capture-region, or:
//   quickshell ipc -p <shell folder> call capture open | close | toggle | take <all|screen|window|area> | recent
// Opening it writes nothing. Your choices (mode, delay, pointer) are kept in ~/.config/nimbus/capture.json through
// SettingsWriter, only when you change one; the recent list is state (~/.local/state/nimbus/capture-history.json).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "capture-draw.js" as Draw

Scope {
    id: cap
    property bool shown: false
    property string page: "take"          // take | edit | recent
    property string phase: ""             // "" | wait (hidden, counting down) | grab | pick (the area picker)
    property bool hiding: phase !== ""
    property int countdown: 0
    property string pendingMode: ""
    property string shot: ""              // the picture being edited
    property var crop: null               // {x, y, w, h} in its pixels, or null for all of it
    property int shotSeq: 0               // a new picture for the editor (it starts again)
    property var freeze: ({})             // the area picker's still pictures: screen name -> {path, width, height}
    property string pickHint: ""
    property string selScreen: ""
    property var sel: null                // the picker's box, in that screen's logical pixels
    property bool selSizing: false        // the D-pad / arrows size the box instead of moving it
    property string error: ""
    property string lastSaved: ""
    property var tools: ({})
    property var recent: []
    property string screenName: ""        // the screen Capture's window was on ("This screen")
    property var editor: null             // CaptureEditor.qml while it's open
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-capture-helper").toString().replace(/^file:\/\//, ""))

    // The controller types arrow keys / Enter / Esc on Capture's pages and in the picker (Pad.qml), and is the
    // pointer in the editor (drawing needs one).
    Binding { target: SettingsNav; property: "captureShown"; value: cap.phase === "pick" || (cap.shown && !cap.hiding && cap.page !== "edit") }
    Binding { target: SettingsNav; property: "captureOpen"; value: cap.shown || cap.phase !== "" }

    // ------------------------------------------------ your choices (capture.json)
    readonly property string prefsPath: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/capture.json"
    property var prefs: ({})
    function pref(k, d) { const v = prefs[k]; return v === undefined || v === null ? d : v; }
    function setPref(k, v) {
        if (JSON.stringify(prefs[k]) === JSON.stringify(v)) return;
        const p = Object.assign({}, prefs); p[k] = v; prefs = p;
        const c = {}; c[k] = v; writer.merge(c);
    }
    readonly property string mode: ["all", "screen", "window", "area"].indexOf(pref("mode", "area")) >= 0 ? pref("mode", "area") : "area"
    readonly property int delay: [0, 3, 5, 10].indexOf(pref("delay", 0)) >= 0 ? pref("delay", 0) : 0
    readonly property bool pointer: pref("pointer", false) === true
    SettingsWriter { id: writer; path: cap.prefsPath }
    FileView {
        path: cap.prefsPath
        blockLoading: true
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            const t = text();
            let d = null;
            try { d = JSON.parse(t); } catch (e) { d = null; }
            if (d === null || typeof d !== "object" || Array.isArray(d)) { if (t.trim() !== "") writer.repair(); return; }
            for (const k in cap.prefs) if (writer.owns(k)) d[k] = cap.prefs[k];
            cap.prefs = d;
        }
    }

    // ------------------------------------------------ the helper
    Component {
        id: jobC
        Process {
            id: p
            property var cb: null
            property string input: ""
            stdinEnabled: input !== ""
            onStarted: { if (p.input !== "") { p.write(p.input); p.input = ""; } }
            stdout: StdioCollector {
                onStreamFinished: {
                    let j = null;
                    const lines = text.trim().split("\n");
                    try { j = JSON.parse(lines[lines.length - 1]); } catch (e) { j = null; }
                    if (p.cb) p.cb(j || {});
                    p.destroy(200);
                }
            }
        }
    }
    function run(args, cb, input) {
        const p = jobC.createObject(cap, { command: ["python3", cap.helper].concat(args), cb: cb || null, input: input || "" });
        p.running = true;
    }
    function loadTools() { run(["tools"], j => cap.tools = j); }
    function loadRecent() { run(["history"], j => cap.recent = j.items || []); }

    function open(p) {
        if (phase !== "") return;
        page = p || (shot !== "" && page === "edit" ? "edit" : "take");
        shown = true;
        loadTools();
        loadRecent();
    }
    function close() { if (phase === "") shown = false; }

    // ------------------------------------------------ taking a picture
    function take(m) {
        if (phase !== "") return;
        error = "";
        pendingMode = ["all", "screen", "window", "area"].indexOf(m) >= 0 ? m : mode;
        if (Object.keys(tools).length === 0) loadTools();
        phase = "wait";                       // the window goes away first
        countdown = delay;
        hideWait.restart();
    }
    Timer { id: hideWait; interval: 450; onTriggered: { if (cap.countdown > 0) tick.restart(); else cap.grabNow(); } }
    Timer {
        id: tick; interval: 1000
        onTriggered: {
            cap.countdown--;
            if (cap.countdown > 0) tick.restart();
            else afterPill.restart();         // the countdown's pill is gone before the picture is taken
        }
    }
    Timer { id: afterPill; interval: 300; onTriggered: cap.grabNow() }
    function cancelTake() { hideWait.stop(); tick.stop(); afterPill.stop(); countdown = 0; phase = ""; shown = true; }

    function screenList() { return Quickshell.screens.map(s => s.name); }
    function grabNow() {
        phase = "grab";
        const ptr = pointer ? ["--pointer"] : [];
        const m = pendingMode;
        if (m === "area") { pickArea(""); return; }
        const args = ["grab", m].concat(ptr);
        if (m === "screen" && (screenName || screenList()[0])) args.push("--screen", screenName || screenList()[0]);
        run(args, j => {
            if (j.ok) { edit(j.path, null); return; }
            if (j.pick) { pickArea("Draw around the window you want (this desktop can't find it by itself)"); return; }
            failed(j.error || "Couldn't take the picture");
        });
    }
    function pickArea(hint) {
        phase = "grab";
        run(["grab-each"].concat(pointer ? ["--pointer"] : []).concat(screenList()), j => {
            if (!j.ok) { failed(j.error || "Couldn't take the picture"); return; }
            freeze = j.shots || {};
            pickHint = hint;
            sel = null; selSizing = false;
            selScreen = freeze[screenName] ? screenName : screenList()[0];
            phase = "pick";
        });
    }
    function failed(msg) { phase = ""; error = msg; page = "take"; shown = true; }
    function screenByName(n) { return Quickshell.screens.find(s => s.name === n) || null; }
    // The box picked on a screen (logical pixels) → the editor, cropped to it.
    function pickDone() {
        const s = screenByName(selScreen), f = freeze[selScreen];
        if (!s || !f || !sel || sel.w < 4 || sel.h < 4) return;
        const c = Draw.pickerToCrop(sel, s.width, s.height, f.width || s.width, f.height || s.height);
        phase = "";
        edit(f.path, c);
    }
    function pickCancel() { phase = ""; sel = null; page = "take"; shown = true; }
    function edit(path, c) {
        shot = path; crop = c; shotSeq++;
        phase = ""; page = "edit"; shown = true;
    }

    function saved(path) { lastSaved = path; loadRecent(); }

    IpcHandler {
        target: "capture"
        function open(): void { cap.open("take"); }
        function close(): void { cap.close(); }
        function toggle(): void { if (cap.shown) cap.close(); else cap.open("take"); }
        function recent(): void { cap.open("recent"); }
        function take(m: string): void { if (!cap.shown) { cap.loadTools(); } cap.take(m); }
        // Checks (capture-polkit-test.yml): the picker's box (X,Y,W,H on its screen) and Take, the editor's sample
        // markup, Save, and what's on screen.
        function pick(geometry: string): void {
            const g = geometry.split(",").map(v => parseInt(v));
            if (cap.phase !== "pick" || g.length !== 4) return;
            cap.sel = { x: g[0], y: g[1], w: g[2], h: g[3] };
        }
        function pickTake(): void { cap.pickDone(); }
        function edit(path: string): void { cap.edit(path, null); }
        function sample(): void { if (cap.editor) cap.editor.sample(); }
        function save(): void { if (cap.editor) cap.editor.saveAndCopy(); }
        function ocr(): void { if (cap.editor) cap.editor.readText(); }
        function qr(): void { if (cap.editor) cap.editor.readQr(); }
        function state(): string {
            return JSON.stringify({ shown: cap.shown, page: cap.page, phase: cap.phase, shot: cap.shot, crop: cap.crop, sel: cap.sel,
                error: cap.error, saved: cap.lastSaved, recent: cap.recent.length, tools: cap.tools,
                editor: cap.editor ? cap.editor.stateObj() : null });
        }
    }

    // ------------------------------------------------ the countdown
    LazyLoader {
        active: cap.phase === "wait" && cap.countdown > 0 && tick.running
        PanelWindow {
            screen: cap.screenByName(cap.screenName) || Quickshell.screens[0]
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-capture-countdown"
            exclusionMode: ExclusionMode.Ignore
            anchors.top: true
            margins.top: 40
            implicitWidth: 260; implicitHeight: 64
            color: "transparent"
            Rectangle {
                anchors.fill: parent; radius: 32
                color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.94); border.color: Theme.line
                Row {
                    anchors.centerIn: parent; spacing: 12
                    CaptureGlyph { anchors.verticalCenter: parent.verticalCenter; name: "timer"; size: 22; color: Theme.accent }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "Taking it in " + cap.countdown; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                }
                MouseArea { anchors.fill: parent; onClicked: cap.cancelTake() }
            }
        }
    }

    // ------------------------------------------------ the area picker: a still picture of each screen
    LazyLoader {
        active: cap.phase === "pick"
        Variants {
            model: Quickshell.screens
            PanelWindow {
                id: pw
                required property var modelData
                readonly property string sname: modelData.name
                readonly property bool mine: cap.selScreen === sname
                readonly property var s: mine ? cap.sel : null
                screen: modelData
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.namespace: "hearth-capture-area"
                WlrLayershell.keyboardFocus: mine ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
                exclusionMode: ExclusionMode.Ignore
                anchors { top: true; bottom: true; left: true; right: true }
                color: "black"
                onMineChanged: if (mine) keys.forceActiveFocus()
                Component.onCompleted: if (mine) keys.forceActiveFocus()

                Image {
                    anchors.fill: parent
                    source: cap.freeze[pw.sname] ? "file://" + cap.freeze[pw.sname].path : ""
                    fillMode: Image.Stretch
                    smooth: true
                    cache: false
                }
                // Dimmed outside the box (all of it when there's no box on this screen).
                Rectangle { color: "#8c05070a"; x: 0; y: 0; width: parent.width; height: pw.s ? pw.s.y : parent.height }
                Rectangle { visible: !!pw.s; color: "#8c05070a"; x: 0; y: pw.s ? pw.s.y + pw.s.h : 0; width: parent.width; height: pw.s ? parent.height - pw.s.y - pw.s.h : 0 }
                Rectangle { visible: !!pw.s; color: "#8c05070a"; x: 0; y: pw.s ? pw.s.y : 0; width: pw.s ? pw.s.x : 0; height: pw.s ? pw.s.h : 0 }
                Rectangle { visible: !!pw.s; color: "#8c05070a"; x: pw.s ? pw.s.x + pw.s.w : 0; y: pw.s ? pw.s.y : 0; width: pw.s ? parent.width - pw.s.x - pw.s.w : 0; height: pw.s ? pw.s.h : 0 }
                Rectangle {
                    visible: !!pw.s
                    x: pw.s ? pw.s.x - 1 : 0; y: pw.s ? pw.s.y - 1 : 0
                    width: pw.s ? pw.s.w + 2 : 0; height: pw.s ? pw.s.h + 2 : 0
                    color: "transparent"; border.width: 2; border.color: Theme.accent
                    Repeater {
                        model: 4
                        Rectangle {
                            required property int index
                            width: 12; height: 12; radius: 6; color: Theme.accent; border.color: "white"; border.width: 2
                            x: (index % 2 === 0 ? 0 : parent.width) - 6
                            y: (index < 2 ? 0 : parent.height) - 6
                        }
                    }
                }
                // The size, by the box.
                Rectangle {
                    visible: !!pw.s
                    x: pw.s ? Math.min(parent.width - width - 8, pw.s.x) : 0
                    y: pw.s ? (pw.s.y > 40 ? pw.s.y - height - 8 : Math.min(parent.height - height - 8, pw.s.y + pw.s.h + 8)) : 0
                    width: sizeT.implicitWidth + 20; height: 28; radius: 14
                    color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.95)
                    Text {
                        id: sizeT; anchors.centerIn: parent
                        text: pw.s ? Math.round(pw.s.w) + " × " + Math.round(pw.s.h) + (cap.selSizing ? "  ·  sizing" : "") : ""
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                    }
                }

                MouseArea {
                    id: drag
                    anchors.fill: parent
                    cursorShape: Qt.CrossCursor
                    property real sx: 0
                    property real sy: 0
                    property bool moving: false
                    property var start: null
                    onPressed: m => {
                        cap.selScreen = pw.sname;
                        const c = cap.sel;
                        moving = c && pw.mine && m.x >= c.x && m.x <= c.x + c.w && m.y >= c.y && m.y <= c.y + c.h;
                        sx = m.x; sy = m.y; start = c;
                        if (!moving) cap.sel = { x: m.x, y: m.y, w: 0, h: 0 };
                    }
                    onPositionChanged: m => {
                        if (!pressed) return;
                        if (moving) cap.sel = Draw.moveSel(start, m.x - sx, m.y - sy, pw.width, pw.height);
                        else { const r = Draw.normRect(sx, sy, Math.max(0, Math.min(pw.width, m.x)), Math.max(0, Math.min(pw.height, m.y))); cap.sel = r; }
                    }
                    onReleased: { if (cap.sel && (cap.sel.w < 4 || cap.sel.h < 4) && !moving) cap.sel = null; }
                    onDoubleClicked: cap.pickDone()
                }

                // What to do, at the top; Take and Cancel by the box.
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: 28
                    width: hintRow.implicitWidth + 36; height: 46; radius: 23
                    color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.95); border.color: Theme.line
                    Row {
                        id: hintRow; anchors.centerIn: parent; spacing: 10
                        CaptureGlyph { anchors.verticalCenter: parent.verticalCenter; name: "area"; size: 20; color: Theme.accent }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: cap.pickHint !== "" ? cap.pickHint : "Drag around what you want"
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "D-pad move  ·  X size  ·  A take  ·  B cancel"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        }
                    }
                }
                Row {
                    visible: !!pw.s && !drag.pressed && pw.s.w >= 4
                    spacing: 8
                    x: pw.s ? Math.max(8, Math.min(pw.width - width - 8, pw.s.x + pw.s.w - width)) : 0
                    y: pw.s ? (pw.s.y + pw.s.h + 56 < pw.height ? pw.s.y + pw.s.h + 10 : Math.max(8, pw.s.y + pw.s.h - height - 10)) : 0
                    CaptureBtn { text: "Cancel"; arrowsMove: false; onActivated: cap.pickCancel() }
                    CaptureBtn { text: "Take"; glyph: "area"; primary: true; arrowsMove: false; onActivated: cap.pickDone() }
                }

                Item {
                    id: keys
                    focus: true
                    Keys.onPressed: e => {
                        const W = pw.width, H = pw.height, step = (e.modifiers & Qt.ShiftModifier) ? 4 : 24;
                        let c = cap.sel || Draw.startSel(W, H);
                        const dx = e.key === Qt.Key_Left ? -step : e.key === Qt.Key_Right ? step : 0;
                        const dy = e.key === Qt.Key_Up ? -step : e.key === Qt.Key_Down ? step : 0;
                        if (dx || dy) { cap.sel = cap.selSizing ? Draw.resizeSel(c, dx, dy, W, H) : Draw.moveSel(c, dx, dy, W, H); }
                        else if (e.key === Qt.Key_Menu || e.key === Qt.Key_Tab || e.key === Qt.Key_Space) { if (!cap.sel) cap.sel = c; cap.selSizing = !cap.selSizing; }
                        else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { if (!cap.sel) cap.sel = c; else cap.pickDone(); }
                        else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Back) cap.pickCancel();
                        else return;
                        e.accepted = true;
                    }
                }
            }
        }
    }

    // ------------------------------------------------ the window
    LazyLoader {
        active: cap.shown && !cap.hiding
        FloatingWindow {
            id: win
            title: "Capture"
            implicitWidth: 1120
            implicitHeight: 720
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible && !cap.hiding) cap.shown = false
            onScreenChanged: if (screen) cap.screenName = screen.name
            Component.onCompleted: { if (screen) cap.screenName = screen.name; firstFocus.restart(); }
            Timer { id: firstFocus; interval: 60; onTriggered: { if (cap.page === "take") takeBtn.forceActiveFocus(); else if (cap.page === "recent") tabRecent.forceActiveFocus(); } }

            // The header: Capture, its pages, and what the controller does.
            Rectangle {
                id: head
                anchors { left: parent.left; right: parent.right; top: parent.top }
                height: 64; color: Theme.bg2
                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.line }
                Row {
                    anchors { left: parent.left; leftMargin: 22; verticalCenter: parent.verticalCenter }
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 10; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
                        anchors.verticalCenter: parent.verticalCenter
                        Icon { anchors.centerIn: parent; name: "cam"; size: 20; color: Theme.accent }
                    }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "Capture"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                    Item { width: 18; height: 1 }
                    CaptureBtn { id: tabTake; anchors.verticalCenter: parent.verticalCenter; text: "Take"; selected: cap.page === "take"; onActivated: cap.page = "take" }
                    CaptureBtn { anchors.verticalCenter: parent.verticalCenter; text: "Edit"; visible: cap.shot !== ""; selected: cap.page === "edit"; onActivated: cap.page = "edit" }
                    CaptureBtn { id: tabRecent; anchors.verticalCenter: parent.verticalCenter; text: "Recent" + (cap.recent.length ? "  " + cap.recent.length : ""); selected: cap.page === "recent"; onActivated: { cap.page = "recent"; cap.loadRecent(); } }
                }
                Text {
                    anchors { right: parent.right; rightMargin: 22; verticalCenter: parent.verticalCenter }
                    text: cap.page === "edit" ? "Ctrl+Z undo  ·  Ctrl+C copy  ·  Ctrl+S save" : "A choose  ·  B close"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
            }

            Item {
                anchors { left: parent.left; right: parent.right; top: head.bottom; bottom: parent.bottom }
                Keys.onPressed: e => { if (e.key === Qt.Key_Escape && cap.page !== "edit") { cap.close(); e.accepted = true; } }

                // ---------------- Take
                Item {
                    id: takePage
                    anchors.fill: parent
                    visible: cap.page === "take"
                    Keys.onEscapePressed: cap.close()
                    RowLayout {
                        anchors { fill: parent; margins: 26 }
                        spacing: 26
                        ColumnLayout {
                            Layout.fillHeight: true
                            Layout.preferredWidth: 560
                            Layout.maximumWidth: 560
                            spacing: 14
                            Text { text: "What to capture"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
                            GridLayout {
                                columns: 2; columnSpacing: 12; rowSpacing: 12
                                Layout.fillWidth: true
                                Repeater {
                                    model: [
                                        { id: "area", g: "area", t: "An area", d: "Drag a box on a still picture" },
                                        { id: "window", g: "window", t: "A window", d: "The window in front" },
                                        { id: "screen", g: "screen", t: "This screen", d: "The screen Capture is on" },
                                        { id: "all", g: "all", t: "All screens", d: "Everything, side by side" }
                                    ]
                                    delegate: Rectangle {
                                        id: tile
                                        required property var modelData
                                        readonly property bool chosen: cap.mode === modelData.id
                                        Layout.fillWidth: true
                                        implicitHeight: 92
                                        radius: Math.max(8, Theme.r(14))
                                        activeFocusOnTab: true
                                        color: chosen ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : tm.containsMouse ? Theme.panel3 : Theme.panel2
                                        border.width: chosen ? 2 : 1; border.color: chosen ? Theme.accent : Theme.line
                                        Rectangle { visible: tile.activeFocus; anchors { fill: parent; margins: -4 } radius: tile.radius + 4; color: "transparent"; border.width: 3; border.color: Theme.accent }
                                        Row {
                                            anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter }
                                            spacing: 14
                                            CaptureGlyph { anchors.verticalCenter: parent.verticalCenter; name: tile.modelData.g; size: 34; color: tile.chosen ? Theme.accent : Theme.text }
                                            Column {
                                                anchors.verticalCenter: parent.verticalCenter; spacing: 3
                                                Text { text: tile.modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                                                Text { text: tile.modelData.d; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                            }
                                        }
                                        MouseArea { id: tm; anchors.fill: parent; hoverEnabled: true; onClicked: { tile.forceActiveFocus(); cap.setPref("mode", tile.modelData.id); } onDoubleClicked: cap.take(tile.modelData.id) }
                                        Keys.onPressed: e => {
                                            if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) cap.setPref("mode", modelData.id);
                                            else if (e.key === Qt.Key_Right || e.key === Qt.Key_Down) nextItemInFocusChain(true).forceActiveFocus();
                                            else if (e.key === Qt.Key_Left || e.key === Qt.Key_Up) nextItemInFocusChain(false).forceActiveFocus();
                                            else return;
                                            e.accepted = true;
                                        }
                                    }
                                }
                            }
                            Text { Layout.topMargin: 6; text: "Wait before taking it"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                            Row {
                                spacing: 8
                                Repeater {
                                    model: [0, 3, 5, 10]
                                    CaptureBtn {
                                        required property int modelData
                                        glyph: modelData === 0 ? "" : "timer"
                                        text: modelData === 0 ? "No wait" : modelData + " s"
                                        selected: cap.delay === modelData
                                        onActivated: cap.setPref("delay", modelData)
                                    }
                                }
                            }
                            CaptureBtn {
                                Layout.topMargin: 4
                                glyph: "pointer"
                                text: cap.pointer ? "The pointer is in the picture" : "Leave the pointer out"
                                selected: cap.pointer
                                onActivated: cap.setPref("pointer", !cap.pointer)
                            }
                            Item { Layout.fillHeight: true }
                            Text {
                                visible: cap.error !== ""
                                Layout.fillWidth: true; wrapMode: Text.WordWrap
                                text: cap.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13
                            }
                            Row {
                                spacing: 12
                                CaptureBtn { id: takeBtn; big: true; primary: true; glyph: cap.mode; text: cap.delay > 0 ? "Take in " + cap.delay + " s" : "Take a screenshot"; onActivated: cap.take(cap.mode) }
                                CaptureBtn {
                                    big: true; glyph: "record"; text: "Record"
                                    enabledLook: (cap.tools.record || "") !== ""
                                    onActivated: {
                                        if ((cap.tools.record || "") === "") { cap.error = "No screen recorder is installed (Hearth Recorder)."; return; }
                                        cap.close();
                                        cap.run(["record"], j => { if (!j.ok) cap.error = j.error || ""; });
                                    }
                                }
                            }
                        }
                        // Recent, at a glance.
                        Rectangle {
                            Layout.fillWidth: true; Layout.fillHeight: true
                            radius: Math.max(8, Theme.r(16)); color: Theme.panel; border.color: Theme.line
                            ColumnLayout {
                                anchors { fill: parent; margins: 18 }
                                spacing: 12
                                RowLayout {
                                    Layout.fillWidth: true
                                    Text { Layout.fillWidth: true; text: "Recent"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
                                    CaptureBtn { text: "See all"; visible: cap.recent.length > 0; onActivated: cap.page = "recent" }
                                }
                                GridLayout {
                                    Layout.fillWidth: true
                                    columns: 2; columnSpacing: 10; rowSpacing: 10
                                    Repeater {
                                        model: cap.recent.slice(0, 6)
                                        delegate: Thumb { required property var modelData; host: cap; item: modelData; Layout.fillWidth: true; implicitHeight: 118 }
                                    }
                                }
                                Text {
                                    visible: cap.recent.length === 0
                                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                                    text: "Screenshots you save show up here. They're kept in Pictures › Screenshots."
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                }
                                Item { Layout.fillHeight: true }
                                Text {
                                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                                    text: "Print Screen saves the whole screen right away · Super+Shift+S picks an area"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                }
                            }
                        }
                    }
                }

                // ---------------- Edit
                Loader {
                    anchors.fill: parent
                    active: cap.shot !== ""
                    visible: cap.page === "edit"
                    source: "CaptureEditor.qml"
                    onStatusChanged: if (status === Loader.Error) console.warn("Capture: the editor (CaptureEditor.qml) couldn't load")
                    onLoaded: { item.cap = cap; cap.editor = item; item.start(); }
                    Component.onDestruction: cap.editor = null
                }

                // ---------------- Recent
                Item {
                    anchors.fill: parent
                    visible: cap.page === "recent"
                    Text {
                        visible: cap.recent.length === 0
                        anchors.centerIn: parent
                        text: "Nothing saved yet. Screenshots you save are kept in Pictures › Screenshots."
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 15
                    }
                    GridView {
                        id: grid
                        anchors { fill: parent; margins: 22 }
                        clip: true
                        cellWidth: Math.floor(width / Math.max(1, Math.floor(width / 260)))
                        cellHeight: 210
                        model: cap.recent
                        keyNavigationEnabled: true
                        focus: cap.page === "recent"
                        delegate: Item {
                            required property var modelData
                            required property int index
                            width: grid.cellWidth; height: grid.cellHeight
                            Thumb {
                                anchors { fill: parent; margins: 7 }
                                host: cap; item: parent.modelData; big: true
                                current: grid.currentIndex === parent.index && grid.activeFocus
                            }
                        }
                        Keys.onReturnPressed: if (currentIndex >= 0) cap.edit(cap.recent[currentIndex].path, null)
                        Keys.onPressed: e => {
                            if (e.key === Qt.Key_Delete && currentIndex >= 0) { cap.run(["forget", cap.recent[currentIndex].path], () => cap.loadRecent()); e.accepted = true; }
                            else if (e.key === Qt.Key_Menu && currentIndex >= 0) { cap.run(["folder", cap.recent[currentIndex].path]); e.accepted = true; }
                            else if (e.key === Qt.Key_Escape) { cap.page = "take"; e.accepted = true; }
                        }
                    }
                    Text {
                        anchors { right: parent.right; bottom: parent.bottom; margins: 12 }
                        visible: cap.recent.length > 0
                        text: "A edit  ·  X show in folder  ·  Delete take off the list"
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                    }
                }
            }
        }
    }

    // A recent picture: its thumbnail, when, and (big) what you can do with it.
    component Thumb: Rectangle {
        id: th
        property var host: null
        property var item: ({})
        property bool big: false
        property bool current: false
        radius: Math.max(6, Theme.r(12))
        color: Theme.panel2
        border.width: current ? 3 : 1; border.color: current ? Theme.accent : Theme.line
        clip: true
        Image {
            id: pic
            anchors { left: parent.left; right: parent.right; top: parent.top; bottom: info.top; margins: 6 }
            source: th.item && th.item.path ? "file://" + th.item.path : ""
            sourceSize.width: 480
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: false
        }
        Column {
            id: info
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 8 }
            spacing: 2
            Text { width: parent.width; elide: Text.ElideMiddle; text: (th.item.path || "").split("/").pop(); color: Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.Medium }
            Text { visible: th.big; text: Draw.when(th.item.time || 0, Date.now()) + (th.item.width ? "  ·  " + th.item.width + " × " + th.item.height : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
        }
        Row {
            visible: th.big && (hov.containsMouse || th.current)
            anchors { right: parent.right; top: parent.top; margins: 10 }
            spacing: 6
            CaptureBtn { icon: "copy"; tip: "Copy"; onActivated: th.host.run(["copy", th.item.path]) }
            CaptureBtn { icon: "folder"; tip: "Show in folder"; onActivated: th.host.run(["folder", th.item.path]) }
            CaptureBtn { icon: "close"; tip: "Take off the list"; onActivated: th.host.run(["forget", th.item.path], () => th.host.loadRecent()) }
        }
        MouseArea { id: hov; anchors.fill: parent; hoverEnabled: true; z: -1; onClicked: th.host.edit(th.item.path, null) }
    }
}
