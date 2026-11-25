// Describe it (docs/mockups/desktop-layouts boards 7-9; checklist parts 7-9): "Editing your desktop". Pick a part (a
// bar, the dock, a panel, the clock's calendar, Quick Access, the app menu), type what you'd like (and add a sketch or a
// picture, or draw one here), and your AI makes it (hearth-layout describe: only Hearth's own parts, checked). A
// layout shows as NOW and NEW side by side with what it changed ("Try it, nothing is saved yet"); Keep this puts it in
// place (hearth-layout apply-json, with Undo), Try again asks again, Undo puts back what was there. A new feature
// shows as its parts (Shows, Click…) with Keep this, Change it…, Try again, Undo and Send this feature to Hearth…;
// what Hearth can't do yet shows Why, the closest it can make (Add this instead) and exactly what an idea sends, sent
// only with Send this idea to Hearth. Opened from the bar's right-click menu, Settings › Desktop layout, or
// `quickshell ipc -p <shell> call describe open <part>`. Nothing runs by itself: the AI is asked only on Make it.
// For one of Mantel's apps (Files, Settings, Task Manager, Capture…; AppBlocks.qml opens it with openApp): "Editing
// <app>", your words go to `hearth-layout describe-app <app>` with the app's blocks (HEARTH_APP_BLOCKS), what it would
// change shows as its chips, and Keep this keeps it with `hearth-layout app-keep` (Undo: app-undo); the app follows.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: dz
    property bool shown: false
    property string part: "bar"
    property string text: ""
    property string sketch: ""          // a picture file (added, or drawn here)
    property bool busy: false
    property var result: null           // hearth-layout describe's answer
    property string error: ""
    property bool kept: false
    property bool drawing: false
    property bool sharing: false
    property bool tellMe: true
    property string sent: ""
    property var now: null              // today's layout (for NOW)
    // one of Mantel's apps (AppBlocks.qml): its id, its name and its blocks (hearth-layout's HEARTH_APP_BLOCKS)
    property bool appMode: false
    property string appId: ""
    property string appName: ""
    property string appCatalog: ""
    visible: shown
    anchors { top: true; bottom: true; left: true; right: true }
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    WlrLayershell.namespace: "hearth-describe"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    readonly property var parts: [["bar", "The bar"], ["dock", "The dock"], ["panel", "A panel"], ["calendar", "The clock's calendar"], ["quickaccess", "Quick Access"], ["menu", "The app menu"]]
    function partName(p) { const x = parts.find(a => a[0] === p); return x ? x[1] : p; }
    readonly property bool isLayout: !!result && result.kind === "layout"
    readonly property bool isFeature: !!result && result.kind === "feature"
    readonly property bool isNeeds: !!result && result.kind === "needs"
    readonly property bool isApp: !!result && result.kind === "app"

    function open(p) {
        appMode = false;
        part = p || "bar"; result = null; error = ""; kept = false; sent = ""; sharing = false;
        shown = true; input.forceActiveFocus();
        run(["hearth-layout", "now"], o => { try { dz.now = JSON.parse(o); } catch (e) { dz.now = null; } });
    }
    function close() { shown = false; drawing = false; }
    function openApp(a, n, c) {
        appMode = true; appId = a; appName = n || a; appCatalog = c || "";
        part = "app:" + a; result = null; error = ""; kept = false; sent = ""; sharing = false;
        shown = true; input.forceActiveFocus();
    }
    readonly property var appEnv: ({ HEARTH_DESCRIBE_APP: appName, HEARTH_APP_BLOCKS: appCatalog })

    Component {
        id: runComp
        Process {
            id: rp
            property var cb: null
            property int left: 3
            property string out: ""
            property string err: ""
            property int code: 0
            function step() { if (--left === 0) { const f = cb; cb = null; if (f) f(out, err, code); Qt.callLater(() => rp.destroy()); } }
            stdout: StdioCollector { onStreamFinished: { rp.out = text; rp.step(); } }
            stderr: StdioCollector { onStreamFinished: { rp.err = text; rp.step(); } }
            onExited: c => { rp.code = c; rp.step(); }
        }
    }
    function run(argv, cb, env) {
        // The environment through `env` on the command line: assigning a JS object to Process.environment throws
        // ("Cannot assign QJSValue to QVariantHash") and nothing ran (the owner's laptop, Debian test-1.18, Nov 24).
        const pre = env ? ["env"].concat(Object.keys(env).map(k => k + "=" + env[k])) : [];
        const p = runComp.createObject(dz, { command: pre.concat(argv), cb: cb || null });
        if (p) p.running = true;
    }
    function make() {
        if (busy || (text.trim() === "" && sketch === "")) return;
        busy = true; error = ""; result = null; kept = false; sent = ""; sharing = false;
        if (appMode) return run(["hearth-layout", "describe-app", appId, text, sketch], (out, err, code) => {
            dz.busy = false;
            let r = null;
            try { r = JSON.parse(out.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
            if (!r || r.ok !== true) { dz.error = (r && r.error) || err.trim() || "Your AI didn't answer."; return; }
            dz.result = r;
        }, appEnv);
        run(["hearth-layout", "describe", part, text, sketch], (out, err, code) => {
            dz.busy = false;
            let r = null;
            try { r = JSON.parse(out.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
            if (!r || r.ok !== true) { dz.error = (r && r.error) || err.trim() || "Your AI didn't answer."; return; }
            dz.result = r;
        }, { HEARTH_DESCRIBE_APP: "Mantel " + (Quickshell.env("HEARTH_MANTEL_VERSION") || "") });
    }
    function keep() {
        if (isApp) return run(["hearth-layout", "app-keep", appId, JSON.stringify({ layout: result.layout || {}, features: result.features || [] })],
                              (o, e, c) => { if (c === 0) { dz.kept = true; AppBlocksHub.changed(dz.appId); } else dz.error = (e || o).trim(); }, appEnv);
        if (isLayout) run(["hearth-layout", "apply-json", JSON.stringify(result.layout)], (o, e, c) => { if (c === 0) dz.kept = true; else dz.error = (e || o).trim(); });
        else if (isFeature) run(["hearth-layout", "keep-feature", JSON.stringify(result.feature)], (o, e, c) => { if (c === 0) { dz.kept = true; Features.reload(); } else dz.error = (e || o).trim(); });
    }
    function undo() {
        if (!kept) { result = null; return; }
        if (isApp) return run(["hearth-layout", "app-undo", appId], () => { dz.kept = false; dz.result = null; AppBlocksHub.changed(dz.appId); });
        if (isLayout) run(["hearth-layout", "undo"], () => { dz.kept = false; dz.result = null; });
        else if (isFeature) run(["hearth-layout", "feature-on", result.feature.id, "false"], () => { dz.kept = false; dz.result = null; Features.reload(); });
    }
    function sendIdea(idea) {
        run(["hearth-layout", "send-idea", JSON.stringify(idea)], (o, e, c) => {
            if (c === 0) {
                dz.sent = "Sent. Thank you" + (dz.tellMe ? ": you'll see a notice in the Hearth Store if it's added." : ".");
                if (dz.tellMe) run(["sh", "-c", 'd="${XDG_CONFIG_HOME:-$HOME/.config}/nimbus"; mkdir -p "$d"; printf "%s\\n" "$1" >> "$d/ideas-sent.txt"', "sh", (JSON.parse(o.trim() || "{}").key || "")]);
            } else dz.error = (e || o).trim();
        });
    }
    // Add a sketch or a picture
    Process {
        id: pick
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Add a sketch or a picture' --getopenfilename \"$HOME/Pictures\" 'image/png image/jpeg'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Add a sketch or a picture' --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.png *.jpg *.jpeg'; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f !== "") dz.sketch = f; } }
    }

    Rectangle { anchors.fill: parent; color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.985) }

    // ---- the top strip
    Rectangle {
        id: strip
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 56
        color: Theme.panel
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.line }
        RowLayout {
            anchors { fill: parent; leftMargin: 24; rightMargin: 24 }
            spacing: 12
            Icon { name: "sun"; size: 20; color: Theme.accent }
            Text { text: dz.appMode ? "Editing " + dz.appName : "Editing your desktop"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
            Text { visible: !dz.appMode; text: "· " + dz.partName(dz.part) + (dz.isFeature || dz.isNeeds ? " · a new feature" : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
            Item { Layout.fillWidth: true }
            Text { visible: !dz.result && !dz.appMode; text: "Click any part of the desktop to change it: a bar, the dock, the clock's calendar, Quick Access, the app menu"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            Text { visible: !dz.result && dz.appMode; text: "Say how you'd like it. To move parts by hand: right-click any part › Edit layout"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            Btn { label: "Done"; onClicked: dz.close() }
        }
    }
    // the parts, to pick one
    Row {
        visible: !dz.result && !dz.busy && !dz.appMode
        anchors { top: strip.bottom; topMargin: 30; horizontalCenter: parent.horizontalCenter }
        spacing: 10
        Repeater {
            model: dz.parts
            Btn { required property var modelData; label: modelData[1]; lit: dz.part === modelData[0]; onClicked: dz.part = modelData[0] }
        }
    }

    // ---- a layout: NOW → NEW, what it changed (board 7)
    Item {
        visible: dz.isLayout
        anchors { top: strip.bottom; topMargin: 56; left: parent.left; right: parent.right; leftMargin: 150; rightMargin: 50 }
        height: 420
        Column {
            id: nowCol
            spacing: 12
            Cap { text: "NOW" }
            Rectangle { width: 520; height: 300; radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
                LayoutMini { anchors { fill: parent; margins: 18 } lay: dz.now } }
        }
        Text { x: nowCol.width + 40; y: 190; text: "→"; color: Theme.accent; font.pixelSize: 44 }
        Column {
            x: nowCol.width + 120
            spacing: 12
            Cap { text: "NEW · TRY IT, NOTHING IS SAVED YET"; color: Theme.accent }
            Rectangle { width: 620; height: 360; radius: Theme.r(12); color: Theme.panel; border.color: Theme.accent; border.width: 2
                LayoutMini { anchors { fill: parent; margins: 20 } lay: dz.isLayout ? dz.result.layout : null } }
        }
        Column {
            anchors.right: parent.right
            width: 420
            spacing: 14
            Cap { text: "WHAT IT CHANGED" }
            Rectangle {
                width: parent.width; height: chg.implicitHeight + 36; radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
                Column { id: chg; x: 20; y: 18; width: parent.width - 40; spacing: 6
                    Repeater { model: dz.isLayout ? dz.result.changes : []
                        Text { required property string modelData; width: chg.width; text: "• " + modelData; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 } } }
            }
            MadeBy {}
            Text { width: parent.width; wrapMode: Text.WordWrap; text: "It only rearranges Mantel's own parts, so it can't break your desktop. Undo is always here."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            Text { visible: dz.isLayout && dz.result.coming && dz.result.coming.length > 0; width: parent.width; wrapMode: Text.WordWrap; text: "Coming soon in this layout: " + (dz.isLayout ? (dz.result.coming || []).join(", ") : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            Row { spacing: 10
                Btn { label: dz.kept ? "Kept" : "Keep this"; primary: !dz.kept; onClicked: if (!dz.kept) dz.keep() }
                Btn { label: "Try again"; onClicked: dz.make() }
                Btn { label: "Undo"; onClicked: dz.undo() } }
        }
    }

    // ---- one of Mantel's apps: what it would change (the chips) and what your AI said, Keep this / Try again / Undo
    Column {
        visible: dz.isApp
        anchors { top: strip.bottom; topMargin: 56; horizontalCenter: parent.horizontalCenter }
        width: Math.min(900, dz.width - 200)
        spacing: 14
        Cap { text: "WHAT IT CHANGES IN " + dz.appName.toUpperCase() + " · NOTHING IS KEPT YET"; color: Theme.accent }
        Flow {
            width: parent.width; spacing: 8
            Repeater {
                model: dz.isApp ? (dz.result.chips || []) : []
                Rectangle { required property var modelData; width: chipT.implicitWidth + 24; height: 32; radius: 16; color: "transparent"; border.color: Theme.accent
                    Text { id: chipT; anchors.centerIn: parent; text: parent.modelData.text || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 } }
            }
        }
        Rectangle {
            width: parent.width; height: appChg.implicitHeight + 36; radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
            visible: dz.isApp && (dz.result.changes || []).length > 0
            Column { id: appChg; x: 20; y: 18; width: parent.width - 40; spacing: 6
                Repeater { model: dz.isApp ? (dz.result.changes || []) : []
                    Text { required property string modelData; width: appChg.width; text: "• " + modelData; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 } } }
        }
        MadeBy {}
        Text { width: parent.width; wrapMode: Text.WordWrap; text: "It only moves, sizes, hides and restyles " + dz.appName + "'s own parts and menus, so nothing it does changes. Undo is always here, and right-click › Back to the original in the app."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        Row { spacing: 10
            Btn { label: dz.kept ? "Kept" : "Keep this"; primary: !dz.kept; onClicked: if (!dz.kept) dz.keep() }
            Btn { label: "Try again"; onClicked: dz.make() }
            Btn { label: "Undo"; onClicked: dz.undo() }
            Btn { label: "Done"; onClicked: dz.close() } }
    }

    // ---- a new feature (board 8): your bar as it'll be, the new feature lit in its place (nothing kept yet)
    Item {
        id: featBar
        visible: dz.isFeature
        anchors { top: strip.bottom; topMargin: 14; left: parent.left; right: parent.right; leftMargin: 8; rightMargin: 8 }
        height: 56
        readonly property var f: dz.isFeature ? dz.result.feature : null
        // start: the flame and the desktops
        Rectangle {
            anchors { left: parent.left; verticalCenter: parent.verticalCenter }
            width: fbStart.implicitWidth + 24; height: 44; radius: 22; color: Theme.panel; border.color: Theme.line
            Row { id: fbStart; anchors.centerIn: parent; spacing: 12
                Image { anchors.verticalCenter: parent.verticalCenter; width: 22; height: 22; source: Qt.resolvedUrl("images/hearth-logo.png"); smooth: true; mipmap: true }
                Repeater { model: 3
                    Rectangle { required property int index; anchors.verticalCenter: parent.verticalCenter; width: index === 0 ? 28 : 18; height: 18; radius: 9; color: index === 0 ? Theme.accent : "transparent"
                        Text { anchors.centerIn: parent; text: index + 1; color: index === 0 ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 11 } } } }
        }
        // the new feature, lit
        Rectangle {
            anchors.centerIn: parent
            width: fbPill.implicitWidth + 28; height: 46; radius: 23; color: Theme.panel; border.color: Theme.accent; border.width: 2
            Rectangle { anchors.fill: parent; anchors.margins: -5; radius: 28; color: "transparent"; border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.35); border.width: 3 }
            Row { id: fbPill; anchors.centerIn: parent; spacing: 10
                Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 24; height: 24; radius: 12; color: Theme.accent
                    Text { anchors.centerIn: parent; text: featBar.f ? featBar.f.name.charAt(0).toUpperCase() : ""; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold } }
                Column { anchors.verticalCenter: parent.verticalCenter
                    Text { text: featBar.f ? featBar.f.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                    Text { text: featBar.f ? (featBar.f.shows || []).map(s => s.label || s.source).join(" · ") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 } }
                Rectangle { anchors.verticalCenter: parent.verticalCenter; width: newT.implicitWidth + 16; height: 20; radius: 10; color: "transparent"; border.color: Theme.accent
                    Text { id: newT; anchors.centerIn: parent; text: "NEW"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 11 } } }
        }
        // end: the status icons and Game Mode
        Row {
            anchors { right: parent.right; verticalCenter: parent.verticalCenter }
            spacing: 8
            Rectangle { width: fbEnd.implicitWidth + 26; height: 44; radius: 22; color: Theme.panel; border.color: Theme.line
                Row { id: fbEnd; anchors.centerIn: parent; spacing: 12
                    Icon { name: "wifi"; size: 17; color: Theme.text }
                    Icon { name: "vol"; size: 17; color: Theme.text }
                    Icon { name: "bell"; size: 17; color: Theme.text } } }
            Rectangle { width: gmT.implicitWidth + 50; height: 44; radius: 22; color: Theme.accent
                Row { anchors.centerIn: parent; spacing: 8
                    Icon { name: "pad"; size: 18; color: Theme.accentInk }
                    Text { id: gmT; text: "Game Mode"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold } } }
        }
    }
    Item {
        visible: dz.isFeature
        anchors { top: strip.bottom; topMargin: 130; left: parent.left; right: parent.right; leftMargin: 150; rightMargin: 120 }
        height: 520
        Column {
            spacing: 12
            Cap { text: "THE NEW FEATURE · MADE FROM HEARTH'S OWN PARTS"; color: Theme.accent }
            Rectangle {
                width: 860; height: fcol.implicitHeight + 44; radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
                Column {
                    id: fcol; x: 22; y: 22; width: parent.width - 44; spacing: 12
                    RowLayout { width: parent.width
                        Text { text: dz.isFeature ? dz.result.feature.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                        Item { Layout.fillWidth: true }
                        Rectangle { implicitWidth: safeT.implicitWidth + 20; implicitHeight: 26; radius: 13; color: "transparent"; border.color: Theme.good || "#6fd38a"
                            Text { id: safeT; anchors.centerIn: parent; text: "✓ Safe: only parts Hearth already has"; color: Theme.good || "#6fd38a"; font.family: Theme.font; font.pixelSize: 12 } } }
                    Repeater {
                        model: dz.isFeature ? (dz.result.feature.shows || []) : []
                        Row4 { required property var modelData; kind: "SHOWS"; what: (modelData.label ? modelData.label + ": " : "") + modelData.source; from: "from Hearth (" + modelData.source + ")" }
                    }
                    Row4 { visible: dz.isFeature && !!dz.result.feature.click; kind: "CLICK"; what: dz.isFeature && dz.result.feature.click ? dz.result.feature.click.action + (dz.result.feature.click.target ? " · " + dz.result.feature.click.target : "") : ""; from: "Hearth's own" }
                    Row4 { kind: "LOOKS"; what: "In " + dz.partName(dz.isFeature ? dz.result.feature.part : "") + ", in your look's colours"; from: "your look's colours" }
                    Text { text: "Updates every " + (dz.isFeature ? dz.result.feature.every : 30) + " s, only while it's showing · turn it off any time in Settings › Your features"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                }
            }
        }
        Column {
            anchors.right: parent.right
            width: 740
            spacing: 14
            Cap { text: "WHAT YOU ASKED" }
            Rectangle { width: parent.width; height: asked.implicitHeight + 40; radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
                Text { id: asked; x: 20; y: 20; width: parent.width - 40; wrapMode: Text.WordWrap; text: "\"" + dz.text + "\""; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 } }
            MadeBy {}
            Row { spacing: 10
                Btn { label: dz.kept ? "Kept" : "Keep this"; primary: !dz.kept; onClicked: if (!dz.kept) dz.keep() }
                Btn { label: "Change it…"; onClicked: { dz.result = null; input.forceActiveFocus(); } }
                Btn { label: "Try again"; onClicked: dz.make() }
                Btn { label: "Undo"; onClicked: dz.undo() } }
            Rectangle { width: parent.width; height: shareCol.implicitHeight + 36; radius: Theme.r(12); color: "transparent"; border.color: Theme.line
                Column { id: shareCol; x: 18; y: 18; width: parent.width - 36; spacing: 10
                    Text { text: "Share it"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: "Like it? Send it to Hearth's makers: a good one can come to everyone in a Hearth OS update."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    Btn { label: "Send this feature to Hearth…"; icon: "share"; onClicked: dz.sharing = true }
                    Text { visible: dz.sharing && dz.sent === ""; width: parent.width; wrapMode: Text.WordWrap; font.family: "monospace"; font.pixelSize: 12; color: Theme.text
                        text: "Idea for: " + dz.partName(dz.part) + "\nAsked: \"" + dz.text + "\"\nThe feature: " + (dz.isFeature ? dz.result.described : "") + "\nNo name, no files, no account, nothing else." }
                    Btn { visible: dz.sharing && dz.sent === ""; primary: true; label: "Send"; onClicked: dz.sendIdea({ format: 1, part: dz.part, asked: dz.text, needs: [], closest: dz.result.feature, sketch: dz.sketch !== "", hearth: "", app: "", kept: dz.kept }) }
                    Text { visible: dz.sent !== ""; text: dz.sent; color: Theme.good || "#6fd38a"; font.family: Theme.font; font.pixelSize: 13 } }
            }
        }
    }

    // ---- this needs something new (board 9)
    Rectangle {
        visible: dz.isNeeds
        anchors { top: strip.bottom; topMargin: 54; horizontalCenter: parent.horizontalCenter }
        width: 1180; height: needsRow.implicitHeight + 120
        radius: Theme.r(14); color: Theme.panel; border.color: Theme.line
        RowLayout {
            x: 34; y: 30; spacing: 14
            Rectangle { implicitWidth: nl.implicitWidth + 22; implicitHeight: 26; radius: 13; color: "transparent"; border.color: Theme.warn || "#f2b84b"
                Text { id: nl; anchors.centerIn: parent; text: "Needs something new"; color: Theme.warn || "#f2b84b"; font.family: Theme.font; font.pixelSize: 13 } }
            Text { text: "This needs something Hearth can't do yet"; color: Theme.text; font.family: Theme.font; font.pixelSize: 24; font.weight: Font.DemiBold }
        }
        RowLayout {
            id: needsRow
            x: 34; y: 86; width: parent.width - 68; spacing: 20
            Column {
                Layout.preferredWidth: 620; spacing: 12
                Cap { text: "YOU ASKED" }
                Rectangle { width: parent.width; height: na.implicitHeight + 36; radius: Theme.r(10); color: Theme.bg2; border.color: Theme.line
                    Text { id: na; x: 18; y: 18; width: parent.width - 36; wrapMode: Text.WordWrap; text: "\"" + dz.text + "\""; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 } }
                Cap { text: "WHY" }
                Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 14
                    text: dz.isNeeds ? (dz.result.why || ("It needs " + (dz.result.needs || []).join(", ") + ".")) : "" }
                Cap { visible: dz.isNeeds && !!dz.result.closest; text: "THE CLOSEST IT CAN MAKE NOW" }
                Rectangle { visible: dz.isNeeds && !!dz.result.closest; width: parent.width; height: 68; radius: Theme.r(10); color: Theme.bg2; border.color: Theme.line
                    RowLayout { anchors { fill: parent; leftMargin: 16; rightMargin: 16 } spacing: 12
                        Rectangle { implicitWidth: 40; implicitHeight: 40; radius: 10; color: Theme.accent; Icon { anchors.centerIn: parent; name: "sparkle"; size: 18; color: Theme.accentInk } }
                        Column { Layout.fillWidth: true
                            Text { text: dz.isNeeds && dz.result.closest ? dz.result.closest.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 }
                            Text { width: parent.width; elide: Text.ElideRight; text: dz.isNeeds ? (dz.result.closestDescribed || "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 } }
                        Btn { label: dz.kept ? "Added" : "Add this instead"; onClicked: if (!dz.kept) dz.run(["hearth-layout", "keep-feature", JSON.stringify(dz.result.closest)], (o, e, c) => { if (c === 0) dz.kept = true; else dz.error = (e || o).trim(); }) } } }
            }
            Column {
                Layout.fillWidth: true; spacing: 12
                Cap { text: "SEND IT AS AN IDEA · EXACTLY WHAT GOES" }
                Rectangle { width: parent.width; height: pv.implicitHeight + 40; radius: Theme.r(10); color: Theme.bg2; border.color: Theme.line
                    Text { id: pv; x: 18; y: 20; width: parent.width - 36; wrapMode: Text.WordWrap; text: dz.isNeeds ? dz.result.preview : ""; color: Theme.text; font.family: "monospace"; font.pixelSize: 13; lineHeight: 1.3 } }
                Row { spacing: 10
                    Rectangle { width: 20; height: 20; radius: 4; color: dz.tellMe ? Theme.accent : "transparent"; border.color: dz.tellMe ? Theme.accent : Theme.muted
                        Text { anchors.centerIn: parent; visible: dz.tellMe; text: "✓"; color: Theme.accentInk; font.pixelSize: 12 }
                        MouseArea { anchors.fill: parent; onClicked: dz.tellMe = !dz.tellMe } }
                    Text { text: "Tell me if it's added (a notice in the Hearth Store)"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 } }
                Item { width: 1; height: 18 }
                Row { anchors.right: parent.right; spacing: 10
                    Text { visible: dz.sent !== ""; anchors.verticalCenter: parent.verticalCenter; text: dz.sent; color: Theme.good || "#6fd38a"; font.family: Theme.font; font.pixelSize: 13 }
                    Btn { visible: dz.sent === ""; label: "Not now"; onClicked: dz.result = null }
                    Btn { visible: dz.sent === ""; primary: true; icon: "share"; label: "Send this idea to Hearth"; onClicked: dz.sendIdea(Object.assign({}, dz.result.idea, { kept: dz.kept })) } }
            }
        }
    }

    Text { visible: dz.error !== ""; anchors { horizontalCenter: parent.horizontalCenter; bottom: box.top; bottomMargin: 16 } text: dz.error; color: Theme.red || "#e5534b"; font.family: Theme.font; font.pixelSize: 14 }
    Text { visible: dz.busy; anchors.centerIn: parent; text: "Your AI is making it…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 }

    // ---- the box to type in, with the sketch (board 7, bottom)
    Rectangle {
        id: box
        // under a result it's hidden (boards 8 and 9); Change it… and Try again bring it back
        visible: !dz.isNeeds && !dz.isFeature
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 150; rightMargin: 50; bottomMargin: 40 }
        height: 182
        radius: Theme.r(16); color: Theme.panel; border.color: Theme.accent; border.width: 2
        RowLayout {
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
            spacing: 16
            Rectangle {
                visible: dz.sketch !== ""
                implicitWidth: 120; implicitHeight: 84; radius: 10; color: "#f3ece4"; clip: true
                Image { anchors.fill: parent; anchors.margins: 4; fillMode: Image.PreserveAspectFit; source: dz.sketch ? "file://" + dz.sketch : ""; cache: false }
                Rectangle { anchors { right: parent.right; top: parent.top; margins: 4 } width: 20; height: 20; radius: 10; color: "#2a2420"
                    Text { anchors.centerIn: parent; text: "✕"; color: "#fff"; font.pixelSize: 10 }
                    MouseArea { anchors.fill: parent; onClicked: dz.sketch = "" } }
            }
            TextInput {
                id: input
                Layout.fillWidth: true
                text: dz.text
                onTextChanged: dz.text = text
                color: Theme.text; font.family: Theme.font; font.pixelSize: 19
                wrapMode: TextInput.Wrap
                onAccepted: dz.make()
                Keys.onEscapePressed: dz.close()
                Text { visible: input.text === ""; text: "Say how you'd like " + (dz.appMode ? dz.appName : dz.partName(dz.part).toLowerCase()) + "…"; color: Theme.muted; font: input.font }
            }
        }
        RowLayout {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 20 }
            spacing: 10
            Btn { icon: "image"; label: "Add a sketch or a picture"; onClicked: pick.running = true }
            Btn { label: "Draw it here"; onClicked: dz.drawing = true }
            Text { Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                text: dz.appMode ? "Try: \"the preview on the left, a bigger folder and no status bar\" · \"put the toolbar at the bottom\""
                                 : "Try: \"make the calendar show the whole month and my next three events\" · \"a dock on the left, icons only\"" }
            Btn { primary: true; label: dz.busy ? "Making it…" : "Make it"; onClicked: dz.make() }
        }
    }

    // ---- Draw it here: a sketch with the mouse or a pen, kept as a picture for the AI
    Rectangle {
        visible: dz.drawing
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.6)
        MouseArea { anchors.fill: parent }
        Rectangle {
            anchors.centerIn: parent
            width: 1000; height: 640; radius: Theme.r(14); color: Theme.panel; border.color: Theme.line
            Text { x: 22; y: 18; text: "Draw it here"; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
            Rectangle {
                id: paper
                x: 22; y: 56; width: parent.width - 44; height: parent.height - 130; radius: 10; color: "#f3ece4"
                property var strokes: []
                property var cur: []
                Canvas {
                    id: ink
                    anchors.fill: parent
                    onPaint: {
                        const c = getContext("2d"); c.reset(); c.lineWidth = 4; c.lineCap = "round"; c.lineJoin = "round"; c.strokeStyle = "#2a2420";
                        for (const s of paper.strokes.concat([paper.cur])) { if (s.length < 2) continue; c.beginPath(); c.moveTo(s[0].x, s[0].y); for (const p of s) c.lineTo(p.x, p.y); c.stroke(); }
                    }
                }
                MouseArea {
                    anchors.fill: parent
                    onPressed: m => { paper.cur = [Qt.point(m.x, m.y)]; }
                    onPositionChanged: m => { paper.cur = paper.cur.concat([Qt.point(m.x, m.y)]); ink.requestPaint(); }
                    onReleased: { paper.strokes = paper.strokes.concat([paper.cur]); paper.cur = []; ink.requestPaint(); }
                }
            }
            Row {
                anchors { right: parent.right; bottom: parent.bottom; margins: 20 }
                spacing: 10
                Btn { label: "Clear"; onClicked: { paper.strokes = []; paper.cur = []; ink.requestPaint(); } }
                Btn { label: "Cancel"; onClicked: dz.drawing = false }
                Btn {
                    primary: true; label: "Use this sketch"
                    onClicked: {
                        const f = (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-sketch-" + Date.now() + ".png";
                        paper.grabToImage(r => { if (r.saveToFile(f)) { dz.sketch = f; dz.drawing = false; } else dz.error = "The sketch couldn't be kept."; });
                    }
                }
            }
        }
    }

    // ---- small parts
    component Cap: Text { color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 1.2 }
    component MadeBy: Row {
        spacing: 10
        Rectangle { width: 14; height: 3; radius: 1.5; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
        Text { text: "Made by your AI: " + (dz.result && dz.result.engine ? dz.result.engine : "your AI") + " (or any AI you pick in Settings)"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
    }
    component Row4: Rectangle {
        property string kind
        property string what
        property string from
        width: parent ? parent.width : 600; height: 40; radius: Theme.r(8); color: Theme.bg2; border.color: Theme.line
        RowLayout { anchors { fill: parent; leftMargin: 14; rightMargin: 14 } spacing: 12
            Text { text: parent.parent.kind; color: Theme.accent; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.Bold; font.letterSpacing: 1 }
            Text { Layout.fillWidth: true; elide: Text.ElideRight; text: parent.parent.what; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { text: parent.parent.from; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 } }
    }
    component Btn: Rectangle {
        id: b
        property string label
        property string icon: ""
        property bool primary: false
        property bool lit: false
        signal clicked()
        implicitWidth: br.implicitWidth + 30; implicitHeight: 38
        width: implicitWidth; height: implicitHeight
        radius: Theme.r(8)
        color: primary ? (ba.containsMouse ? Qt.lighter(Theme.accent, 1.1) : Theme.accent) : (ba.containsMouse || lit ? Theme.panel3 : Theme.bg2)
        border.color: lit ? Theme.accent : primary ? "transparent" : Theme.line
        Row { id: br; anchors.centerIn: parent; spacing: 8
            Icon { visible: b.icon !== ""; name: b.icon; size: 16; color: b.primary ? Theme.accentInk : Theme.text; anchors.verticalCenter: parent.verticalCenter }
            Text { text: b.label; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: b.primary ? Font.DemiBold : Font.Medium; anchors.verticalCenter: parent.verticalCenter } }
        MouseArea { id: ba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.clicked() }
    }
}
