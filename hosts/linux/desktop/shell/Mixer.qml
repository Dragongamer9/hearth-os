// The volume mixer (Hearth 1.10), like Windows 11's: which speakers or headphones to use, the system volume, and a row
// per app playing sound (its icon, name, where it plays, mute and a slider). Opens from the sound icon's right-click
// menu ("Volume mixer") and from Settings › Sound.
//
// PipeWire through Quickshell (the same as the bar's volume and Settings › Sound). Each app's level is remembered by
// the app's name in ~/.config/nimbus/app-volumes.json, only when you move its slider (SettingsWriter: just that app's
// line is merged in, atomically, with a .bak), and put back when that app plays sound again. Opening the mixer writes
// nothing. Right-click an app (or press X on the controller) to send it to another output.
// Keys and controller: Up/Down rows, Left/Right volume, Enter/A mutes (on the output row: the list of outputs), X / the
// Menu key: "Play on…", Esc/B closes. Loaded by shell.qml through a Loader: a problem here only loses the mixer.
//   quickshell ipc -p <shell folder> call mixer open|close|toggle|state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pipewire
import "mixer.js" as M

Scope {
    id: mx
    property bool shown: false
    property var atScreen: null
    property real atX: -1               // where the sound icon is (screen coordinates), or -1 for the top right
    property real atY: -1
    property bool above: false          // the bar is at the bottom: the mixer opens above that point
    property int current: 1             // 0 the output picker, 1 System, 2… the apps
    property string menuFor: ""         // "" | "out" (the outputs list) | an app's name ("Play on…")
    property int menuIndex: 0
    ExtrasPal { id: pal }

    function open() { openNear(null, -1, -1, false); }
    function openNear(scr, x, y, up) {
        atScreen = scr || (Quickshell.screens.length ? Quickshell.screens[0] : null);
        atX = x; atY = y; above = !!up;
        current = appKeys.length ? 2 : 1;
        menuFor = "";
        shown = true;
    }
    function close() { shown = false; menuFor = ""; }
    function toggle() { shown ? close() : open(); }
    onShownChanged: SettingsNav.mixerShown = shown
    Component.onCompleted: SettingsNav.mixer = mx

    // ------------------------------------------------ stand-in sound for the Desktop Mode test (no PipeWire there)
    readonly property string fixturePath: Quickshell.env("HEARTH_MIXER_FIXTURE") || ""
    property var fix: null
    Process {
        running: mx.fixturePath !== ""
        command: ["cat", mx.fixturePath]
        stdout: StdioCollector { onStreamFinished: { try { mx.fix = JSON.parse(text); } catch (e) { mx.fix = null; } } }
    }
    function fixApp(key) { return mx.fix ? (mx.fix.apps || []).find(a => a.name === key) || null : null; }
    function setFix(key, change) {
        if (!mx.fix) return;
        const f = JSON.parse(JSON.stringify(mx.fix));
        if (key === "") change(f); else { const a = (f.apps || []).find(x => x.name === key); if (a) change(a); }
        mx.fix = f;
    }

    // ------------------------------------------------ each app's level, remembered
    readonly property string memPath: Quickshell.env("HOME") + "/.config/nimbus/app-volumes.json"
    property var remembered: ({})
    property bool memRead: false
    SettingsWriter { id: writer; path: mx.memPath }
    FileView {
        id: memFile
        path: mx.memPath
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                // Damaged (a power cut): put back from its .bak; nothing here writes over it meanwhile.
                if (text().trim() !== "") writer.repair();
            } else {
                mx.remembered = M.cleanRemembered(o);
            }
            mx.memRead = true;
        }
        onLoadFailed: mx.memRead = true      // no file yet: nothing remembered
    }
    // Only when you move an app's slider: that app's line, merged into the latest file.
    function remember(key, v) {
        const c = M.rememberChange(key, v);
        if (!c) return;
        const r = Object.assign({}, remembered);
        r[key] = c[key];
        remembered = r;
        writer.merge(c);
    }

    // ------------------------------------------------ PipeWire
    readonly property var nodes: Pipewire.nodes ? Pipewire.nodes.values : []
    readonly property var outputs: nodes.filter(n => n.audio && n.isSink && !n.isStream)
    readonly property var streams: nodes.filter(n => n.audio && n.isStream && !n.isSink)
    readonly property var sink: Pipewire.defaultAudioSink
    PwObjectTracker { objects: [mx.sink].concat(mx.streams).concat(mx.outputs).filter(Boolean) }

    function keyOf(n) { const k = M.appKey(n && n.properties ? n.properties : {}); return k || ("stream " + (n ? n.id : "")); }
    function nodesOf(key) { return streams.filter(n => keyOf(n) === key); }
    function devName(n) { return n ? (n.description || n.nickname || n.name || "Speakers") : "No output"; }
    function devIcon(name) {
        const s = String(name || "").toLowerCase();
        return /head|ear|bluetooth|airpod|buds/.test(s) ? "headphones" : /hdmi|tv|display|monitor/.test(s) ? "tv" : "vol";
    }

    // The apps, one row each (a browser's tabs are one row). Only the list of names: it changes when an app starts or
    // stops playing, not while you drag a slider (the rows stay put under the pointer).
    readonly property string keysJoined: {
        if (mx.fix) return (mx.fix.apps || []).map(a => a.name).join("\n");
        return M.group(streams.map(n => ({ id: n.id, key: keyOf(n) }))).map(g => g.key).join("\n");
    }
    property var appKeys: []
    onKeysJoinedChanged: {
        appKeys = keysJoined === "" ? [] : keysJoined.split("\n");
        current = Math.min(current, 1 + appKeys.length);
        if (menuFor !== "" && menuFor !== "out" && appKeys.indexOf(menuFor) < 0) menuFor = "";
    }

    // Per app: its level, mute, icon and where it plays.
    function volOf(key) {
        if (mx.fix) { const a = fixApp(key); return a ? a.volume : 0; }
        const ns = nodesOf(key);
        return ns.length && ns[0].audio ? ns[0].audio.volume : 0;
    }
    function mutedOf(key) {
        if (mx.fix) { const a = fixApp(key); return a ? !!a.muted : false; }
        const ns = nodesOf(key);
        return ns.length && ns[0].audio ? ns[0].audio.muted : false;
    }
    function iconOf(key) {
        if (mx.fix) { const a = fixApp(key); return a && a.icon ? AppIcons.find(a.icon, a.icon) : ""; }
        const ns = nodesOf(key);
        const p = ns.length && ns[0].properties ? ns[0].properties : {};
        return AppIcons.find(M.iconName(p), String(p["application.process.binary"] || ""));
    }
    function setVol(key, v, save) {
        v = M.clamp(v);
        if (mx.fix) setFix(key, a => { a.volume = v; if (v > 0) a.muted = false; });
        else for (const n of nodesOf(key)) if (n.audio) { n.audio.volume = v; if (v > 0 && n.audio.muted) n.audio.muted = false; }
        if (save) remember(key, v);
    }
    function toggleMute(key) {
        const m = !mutedOf(key);
        if (mx.fix) setFix(key, a => { a.muted = m; });
        else for (const n of nodesOf(key)) if (n.audio) n.audio.muted = m;
    }
    // "Play on…": WirePlumber moves the app's sound when its target changes (pw-metadata, part of PipeWire).
    function sendTo(key, o) {
        if (mx.fix) { setFix(key, a => { a.out = o.name; }); return; }
        const out = o ? o.node : null;
        if (!out) return;
        const serial = out.properties && out.properties["object.serial"] ? String(out.properties["object.serial"]) : out.name;
        const ids = nodesOf(key).map(n => String(n.id));
        if (!ids.length || mover.running) return;
        // Every stream of the app at once (a browser's tabs): its target is the output's serial (WirePlumber 0.5) and id
        // (older WirePlumber).
        mover.command = ["sh", "-c", 'out=$1; sid=$2; shift 2; for i; do pw-metadata "$i" target.object "$out" >/dev/null; pw-metadata "$i" target.node "$sid" >/dev/null; done', "sh", serial, String(out.id)].concat(ids);
        mover.running = true;
    }
    Process {
        id: mover
        onExited: code => { if (code === 127) SettingsNav.toast("Moving an app's sound needs pw-metadata (PipeWire's tools)"); }
    }

    // The outputs (the picker and "Play on…"): [{name, node}]
    readonly property var outList: mx.fix ? (mx.fix.outputs || []).map(o => ({ name: o.name, node: null }))
        : outputs.map(n => ({ name: devName(n), node: n }))
    readonly property string defaultOut: mx.fix ? String(mx.fix.default || "") : devName(sink)
    function pickOutput(i) {
        const o = outList[i];
        if (!o) return;
        if (mx.fix) setFix("", f => { f.default = o.name; });
        else Pipewire.preferredDefaultAudioSink = o.node;
        menuFor = "";
    }
    readonly property real sysVol: mx.fix ? (mx.fix.system || 0) : (sink && sink.audio ? sink.audio.volume : 0)
    readonly property bool sysMuted: mx.fix ? !!mx.fix.systemMuted : !!(sink && sink.audio && sink.audio.muted)
    function setSys(v) {
        v = M.clamp(v, Buttons.maxVolume);
        if (mx.fix) setFix("", f => { f.system = v; f.systemMuted = false; });
        else if (sink && sink.audio) { sink.audio.muted = false; sink.audio.volume = v; }
    }
    function toggleSysMute() {
        if (mx.fix) setFix("", f => { f.systemMuted = !f.systemMuted; });
        else if (sink && sink.audio) sink.audio.muted = !sink.audio.muted;
    }

    // ------------------------------------------------ putting an app's level back when it plays again
    // Each new stream once (by its PipeWire id), when it's ready and the remembered levels have been read.
    property var handled: ({})
    function tryRestore(n) {
        if (mx.fix || !memRead || !n || !n.audio || handled[n.id]) return;
        if (n.ready === false) return;           // asked again when it is
        const h = Object.assign({}, handled);
        h[n.id] = true;
        handled = h;
        const r = M.restoreLevel(remembered, M.appKey(n.properties || {}), n.audio.volume);
        if (r !== null) n.audio.volume = r;
    }
    // Tried for each stream when the list changes, a few times while one isn't ready yet.
    property int restoreTries: 0
    onStreamsChanged: { restoreTries = 0; restorer.restart(); }
    onMemReadChanged: { restoreTries = 0; restorer.restart(); }
    Timer {
        id: restorer
        interval: 400
        repeat: true
        onTriggered: {
            for (const n of mx.streams) mx.tryRestore(n);
            mx.restoreTries++;
            if (mx.restoreTries > 10 || mx.streams.every(n => mx.handled[n.id])) stop();
        }
    }

    IpcHandler {
        target: "mixer"
        function open(): void { mx.open(); }
        function close(): void { mx.close(); }
        function toggle(): void { mx.toggle(); }
        function state(): string { return JSON.stringify({ shown: mx.shown, apps: mx.appKeys, current: mx.current, menu: mx.menuFor, remembered: mx.remembered }); }
    }

    // ------------------------------------------------ the keys
    function rowKey(r) { return r >= 2 ? appKeys[r - 2] || "" : ""; }
    function key(e) {
        const menuLen = menuFor === "out" ? outList.length : menuFor !== "" ? outList.length : 0;
        switch (e.key) {
        case Qt.Key_Up: case Qt.Key_Down: {
            const d = e.key === Qt.Key_Down ? 1 : -1;
            if (menuFor !== "") menuIndex = Math.max(0, Math.min(menuLen - 1, menuIndex + d));
            else current = M.moveRow(current, d, appKeys.length);
            break;
        }
        case Qt.Key_Left: case Qt.Key_Right: {
            if (menuFor !== "") break;
            const d = e.key === Qt.Key_Right ? 1 : -1;
            if (current === 1) setSys(M.step(sysVol, d, Buttons.maxVolume));
            else if (current >= 2) { const k = rowKey(current); if (k) setVol(k, M.step(volOf(k), d), true); }
            break;
        }
        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
            if (menuFor === "out") pickOutput(menuIndex);
            else if (menuFor !== "") { const o = outList[menuIndex]; if (o) sendTo(menuFor, o); menuFor = ""; }
            else if (current === 0) { menuFor = "out"; menuIndex = Math.max(0, outList.findIndex(o => o.name === defaultOut)); }
            else if (current === 1) toggleSysMute();
            else { const k = rowKey(current); if (k) toggleMute(k); }
            break;
        case Qt.Key_Menu: case Qt.Key_X:
            if (current >= 2 && menuFor === "") openSendTo(rowKey(current));
            break;
        case Qt.Key_Escape:
            if (menuFor !== "") menuFor = ""; else close();
            break;
        default:
            return;
        }
        e.accepted = true;
    }
    function openSendTo(k) {
        if (!k) return;
        current = 2 + appKeys.indexOf(k);
        menuFor = k;
        menuIndex = 0;
    }

    // ------------------------------------------------ the popup
    component MixSlider: Item {
        id: sl
        property real value: 0
        property real maxValue: 1
        property bool off: false
        signal moved(real v)
        implicitHeight: 16
        readonly property real frac: Math.max(0, Math.min(1, value / maxValue))
        Rectangle { id: track; anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 6; radius: 3; color: pal.line }
        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: Math.max(6, track.width * sl.frac); height: 6; radius: 3; color: sl.off ? pal.off : pal.accent }
        Rectangle { x: track.width * sl.frac - 8; anchors.verticalCenter: parent.verticalCenter; width: 16; height: 16; radius: 8; color: "#ffffff" }
        MouseArea {
            anchors { fill: parent; topMargin: -6; bottomMargin: -6 }
            cursorShape: Qt.PointingHandCursor
            function at(x) { sl.moved(sl.maxValue * Math.max(0, Math.min(1, x / width))); }
            onPressed: m => at(m.x)
            onPositionChanged: m => { if (pressed) at(m.x); }
            onWheel: w => sl.moved(Math.max(0, Math.min(sl.maxValue, sl.value + (w.angleDelta.y > 0 ? 0.05 : -0.05))))
        }
    }
    component MixLabel: Text { color: pal.muted; font.family: pal.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.4 }
    // A list of outputs (the picker, and "Play on…"): the lit one follows the keys.
    component OutList: Column {
        id: ol
        property string forKey: ""
        property string chosen: ""
        spacing: 2
        Repeater {
            model: mx.outList
            delegate: Rectangle {
                id: orow
                required property var modelData
                required property int index
                readonly property bool lit: mx.menuIndex === index
                width: ol.width; height: 32; radius: Theme.r(6)
                color: lit ? pal.sel : om.containsMouse ? Qt.lighter(pal.box, 1.15) : "transparent"
                border.color: lit ? pal.accent : "transparent"
                Row {
                    anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                    spacing: 10
                    Icon { name: mx.devIcon(orow.modelData.name); size: 15; color: pal.accent }
                    Text { text: orow.modelData.name; color: pal.text; font.family: pal.font; font.pixelSize: 13 }
                    Icon { visible: orow.modelData.name === ol.chosen; name: "check"; size: 14; color: pal.accent }
                }
                MouseArea {
                    id: om; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                    onEntered: mx.menuIndex = orow.index
                    onClicked: { if (ol.forKey === "out") mx.pickOutput(orow.index); else { mx.sendTo(ol.forKey, orow.modelData); mx.menuFor = ""; } }
                }
            }
        }
    }

    PanelWindow {
        id: panel
        visible: mx.shown
        screen: mx.atScreen || (Quickshell.screens.length ? Quickshell.screens[0] : null)
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-mixer"
        WlrLayershell.keyboardFocus: mx.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "transparent"
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        MouseArea { anchors.fill: parent; onClicked: mx.close() }   // a click anywhere else closes it
        Item { id: keys; focus: true; Keys.onPressed: e => mx.key(e) }

        Rectangle {
            id: card
            width: Math.min(480, parent.width - 24)
            height: Math.min(parent.height - 24, col.implicitHeight + 36)
            x: mx.atX < 0 ? parent.width - width - 12 : Math.max(12, Math.min(parent.width - width - 12, mx.atX - width / 2))
            y: mx.atY < 0 ? 52 : mx.above ? Math.max(12, mx.atY - height - 6) : Math.max(12, Math.min(parent.height - height - 12, mx.atY + 6))
            radius: Theme.r(12)
            color: pal.card
            border.color: pal.edge
            clip: true
            MouseArea { anchors.fill: parent }   // clicks on the card don't close it

            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 18 }
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Text { Layout.fillWidth: true; text: "Volume mixer"; color: pal.text; font.family: pal.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                    Rectangle {
                        width: 28; height: 28; radius: Theme.r(6)
                        color: gearM.containsMouse ? pal.box : "transparent"
                        Icon { anchors.centerIn: parent; name: "gear"; size: 16; color: pal.muted }
                        MouseArea { id: gearM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { mx.close(); SettingsNav.openPage("sound"); } }
                    }
                }

                // OUTPUT: the device, then System
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: outCol.implicitHeight + 24
                    radius: Theme.r(10)
                    color: pal.box
                    border.color: pal.line
                    Column {
                        id: outCol
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
                        spacing: 8
                        MixLabel { text: "OUTPUT" }
                        Rectangle {
                            width: parent.width; height: 34; radius: Theme.r(6)
                            color: pal.well
                            border.color: mx.current === 0 && mx.menuFor === "" ? pal.accent : pal.line
                            Row {
                                anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                                spacing: 10
                                Icon { name: mx.devIcon(mx.defaultOut); size: 16; color: pal.accent }
                                Text { text: mx.defaultOut; color: pal.text; font.family: pal.font; font.pixelSize: 13; elide: Text.ElideRight; width: outCol.width - 80 }
                            }
                            Icon { anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }; name: mx.menuFor === "out" ? "chevu" : "chevd"; size: 12; color: pal.muted }
                            MouseArea {
                                anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                                onClicked: { mx.current = 0; if (mx.menuFor === "out") mx.menuFor = ""; else { mx.menuFor = "out"; mx.menuIndex = Math.max(0, mx.outList.findIndex(o => o.name === mx.defaultOut)); } }
                            }
                        }
                        OutList { visible: mx.menuFor === "out"; width: parent.width; forKey: "out"; chosen: mx.defaultOut }
                        Text {
                            visible: mx.menuFor === "out" && mx.outList.length === 0
                            text: "No speakers or headphones found"; color: pal.muted; font.family: pal.font; font.pixelSize: 12
                        }
                        Rectangle {
                            width: parent.width; height: 34; radius: Theme.r(6)
                            color: mx.current === 1 ? pal.sel : "transparent"
                            border.color: mx.current === 1 ? pal.accent : "transparent"
                            RowLayout {
                                anchors { fill: parent; leftMargin: 6; rightMargin: 6 }
                                spacing: 12
                                Text { Layout.preferredWidth: 84; text: "System"; color: pal.text; font.family: pal.font; font.pixelSize: 13 }
                                MixSlider {
                                    Layout.fillWidth: true
                                    value: mx.sysVol; maxValue: Buttons.maxVolume; off: mx.sysMuted
                                    onMoved: v => { mx.current = 1; mx.setSys(v); }
                                }
                                Text {
                                    Layout.preferredWidth: 44; horizontalAlignment: Text.AlignRight
                                    text: mx.sysMuted ? "Muted" : String(Math.round(mx.sysVol * 100))
                                    color: mx.sysMuted ? pal.warn : pal.text; font.family: pal.font; font.pixelSize: 13
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: mx.toggleSysMute() }
                                }
                            }
                        }
                    }
                }

                MixLabel { text: "APPS" }

                Text {
                    visible: mx.appKeys.length === 0
                    Layout.fillWidth: true
                    text: "No apps are playing sound right now. When one does, it shows up here."
                    wrapMode: Text.WordWrap
                    color: pal.muted; font.family: pal.font; font.pixelSize: 13
                }

                Repeater {
                    model: mx.appKeys
                    delegate: ColumnLayout {
                        id: app
                        required property string modelData
                        required property int index
                        readonly property bool lit: mx.current === index + 2
                        readonly property real vol: mx.volOf(modelData)
                        readonly property bool muted: mx.mutedOf(modelData)
                        readonly property string icon: mx.iconOf(modelData)
                        readonly property var firstNode: mx.fix ? null : (mx.nodesOf(modelData)[0] || null)
                        Layout.fillWidth: true
                        spacing: 4
                        PwNodeLinkTracker { id: links; node: app.firstNode }
                        readonly property string out: {
                            if (mx.fix) { const a = mx.fixApp(app.modelData); return a && a.out ? a.out : mx.defaultOut; }
                            const g = links.linkGroups || [];
                            for (const l of g) if (l && l.target && l.target !== app.firstNode) return mx.devName(l.target);
                            return mx.defaultOut;
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 48
                            radius: Theme.r(8)
                            color: app.lit ? pal.sel : rowM.containsMouse ? Qt.rgba(1, 1, 1, 0.03) : "transparent"
                            border.color: app.lit ? pal.accent : "transparent"
                            MouseArea {
                                id: rowM
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                onClicked: m => { mx.current = app.index + 2; if (m.button === Qt.RightButton) mx.openSendTo(app.modelData); }
                            }
                            RowLayout {
                                anchors { fill: parent; leftMargin: 4; rightMargin: 6 }
                                spacing: 12
                                Rectangle {
                                    Layout.preferredWidth: 32; Layout.preferredHeight: 32
                                    radius: Theme.r(8)
                                    color: app.icon ? "transparent" : M.color(app.modelData)
                                    Text { visible: !app.icon; anchors.centerIn: parent; text: M.letter(app.modelData); color: "#06121c"; font.family: pal.font; font.pixelSize: 15; font.weight: Font.Bold }
                                    Image { visible: !!app.icon; anchors.fill: parent; source: app.icon; sourceSize: Qt.size(64, 64); fillMode: Image.PreserveAspectFit; asynchronous: true }
                                }
                                Column {
                                    Layout.preferredWidth: 110
                                    spacing: 1
                                    Text { width: 110; text: app.modelData; elide: Text.ElideRight; color: pal.text; font.family: pal.font; font.pixelSize: 13 }
                                    Text { width: 110; text: app.out; elide: Text.ElideRight; color: pal.muted; font.family: pal.font; font.pixelSize: 11 }
                                }
                                Item {
                                    Layout.preferredWidth: 20; Layout.preferredHeight: 20
                                    Icon { anchors.centerIn: parent; name: app.muted ? "vol-mute" : "vol"; size: 16; color: app.muted ? pal.warn : pal.muted }
                                    MouseArea { anchors { fill: parent; margins: -4 }; cursorShape: Qt.PointingHandCursor; onClicked: { mx.current = app.index + 2; mx.toggleMute(app.modelData); } }
                                }
                                MixSlider {
                                    Layout.fillWidth: true
                                    value: app.vol; maxValue: 1; off: app.muted
                                    onMoved: v => { mx.current = app.index + 2; mx.setVol(app.modelData, v, true); }
                                }
                                Text {
                                    Layout.preferredWidth: 44; horizontalAlignment: Text.AlignRight
                                    text: app.muted ? "Muted" : String(Math.round(app.vol * 100))
                                    color: app.muted ? pal.warn : pal.text; font.family: pal.font; font.pixelSize: 13
                                }
                            }
                        }
                        // "Play <app> on…"
                        Rectangle {
                            visible: mx.menuFor === app.modelData
                            Layout.fillWidth: true
                            Layout.leftMargin: 48
                            implicitHeight: sendCol.implicitHeight + 16
                            radius: Theme.r(8)
                            color: pal.box
                            border.color: pal.line
                            Column {
                                id: sendCol
                                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8 }
                                spacing: 4
                                MixLabel { text: "PLAY " + app.modelData.toUpperCase() + " ON" }
                                OutList { width: parent.width; forKey: app.modelData; chosen: app.out }
                                Text {
                                    visible: mx.outList.length === 0
                                    text: "No other speakers or headphones found"; color: pal.muted; font.family: pal.font; font.pixelSize: 12
                                }
                            }
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.topMargin: 4
                    text: "Each app’s level is remembered. Right-click an app to send it to another output (headset, TV)."
                    wrapMode: Text.WordWrap
                    color: pal.muted; font.family: pal.font; font.pixelSize: 12; lineHeight: 1.2
                }
            }
        }
    }
}
