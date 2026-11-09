// System Restore's page (loaded by Restore.qml). The approved mockup: the restore points on the left (date, what, type),
// "What will change" on the right, and Make a restore point now / Restore and restart / Cancel at the bottom.
// Mouse, touch, keyboard and the controller (hearth-pad types keys while this window is in front: A Enter, B Esc):
//   Up / Down pick a point (the last row shows older ones), Right goes to "See every changed setting", Down past the
//   list goes to the buttons, Left / Right move between them, A presses, B closes a dialog or the window.
// Everything that changes something runs hearth_restore.py (next to this file): it keeps a copy of today's settings
// first and only uses Hearth's own `hearth-admin rollback` for the version (docs/SYSTEM-RESTORE.md).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

FocusScope {
    id: v
    focus: true
    property string helper: ""
    property var closeWindow: null

    // ---- the mockup's palette
    readonly property color cBg: "#171d25"
    readonly property color cTitle: "#10151b"
    readonly property color cHead: "#1b2838"
    readonly property color cPanel: "#213347"
    readonly property color cLine: "#2f4a63"
    readonly property color cSel: "#2a475e"
    readonly property color cText: "#e6eef5"
    readonly property color cMuted: "#9fb2c3"
    readonly property color cAccent: "#66c0f4"
    readonly property color cInk: "#06121c"
    readonly property string font: "Inter"

    // ---- state
    property var ov: null              // hearth_restore.py overview
    property string phase: "loading"   // loading | idle | confirm | working | changes | done
    property string error: ""
    property string done: ""
    property bool doneRestart: false
    property bool showOlder: false
    property int sel: 1
    property string zone: "list"       // list | aside | footer
    property int footIdx: 1            // 0 make, 1 restore, 2 cancel
    property int dlgIdx: 0
    property var changeList: []
    property var addedList: []
    readonly property int shortCount: 6

    function start() { load(); }
    function load() { if (!overviewProc.running) { overviewProc.command = ["python3", helper, "overview"]; overviewProc.running = true; } }

    readonly property var points: ov && ov.points ? ov.points : []
    readonly property var visiblePoints: showOlder ? points : points.slice(0, shortCount)
    readonly property string running: ov && ov.running ? ov.running : (ov && ov.status && ov.status.booted ? String(ov.status.booted.version || "") : "")
    // Row 0 is where you are now; rows 1.. are the points.
    readonly property int rowCount: 1 + visiblePoints.length
    readonly property bool hasOlder: points.length > shortCount
    readonly property var cur: sel >= 1 && sel <= visiblePoints.length ? visiblePoints[sel - 1] : null
    readonly property string action: cur && cur.plan ? cur.plan.action : "none"
    readonly property bool versionGoesBack: action === "rollback" || action === "queued"
    readonly property bool canRestore: cur !== null && phase === "idle" && (versionGoesBack || cur.changed > 0)

    function when(ts) {
        if (!ts) return "";
        const d = new Date(ts * 1000), now = new Date();
        const t = d.toLocaleTimeString(Qt.locale(), Locale.ShortFormat);
        const y = new Date(now); y.setDate(now.getDate() - 1);
        if (d.toDateString() === now.toDateString()) return "Today, " + t;
        if (d.toDateString() === y.toDateString()) return "Yesterday, " + t;
        return d.toLocaleDateString(Qt.locale(), "MMM d") + ", " + t;
    }
    function day(ts) { return ts ? new Date(ts * 1000).toLocaleDateString(Qt.locale(), "MMM d") : ""; }

    readonly property string pOk: "M5 12l5 5 9-10"
    readonly property string pArrow: "M5 12h14 M13 6l6 6-6 6"
    readonly property string pKeep: "M12 5v14 M5 12h14"
    function chg(t, s, d, kind) {
        const k = { go: ["#22405a", "#9fd6f7"], ok: ["#1f4a32", "#7fe0a4"], keep: ["#3a3326", "#f2c14e"] }[kind];
        return { t: t, s: s, d: d, bg: k[0], fg: k[1] };
    }
    readonly property var changes: {
        if (!cur) return [chg("This is where you are now", "Pick an earlier point to see what would change.", pOk, "ok")];
        const p = cur.plan || {};
        const out = [];
        if (versionGoesBack)
            out.push(chg("Hearth " + (p.from || running) + " → " + p.to, "The whole system goes back. " + (p.from || running) + " stays in the boot menu, so you can come back.", pArrow, "go"));
        else if (action === "none")
            out.push(chg("Hearth " + (running || p.to || "") + " stays", "This point was made on the version you run now.", pOk, "ok"));
        else
            out.push(chg("Hearth " + (running || "") + " stays", p.why || "Only the settings can go back.", pKeep, "keep"));
        if (cur.changed > 0)
            out.push(chg("Settings from " + day(cur.created), cur.changed + (cur.changed === 1 ? " setting" : " settings") + " changed since then go back. A copy of today's is kept.", pArrow, "go"));
        else
            out.push(chg("Your settings are the same", "Nothing changed since then.", pOk, "ok"));
        out.push(chg("Apps you installed stay", "Flatpaks, Windows apps and games aren't removed.", pKeep, "keep"));
        out.push(chg("Your files are not touched", "Documents, pictures, games and saves stay exactly as they are.", pOk, "ok"));
        return out;
    }
    readonly property string goLabel: !cur ? "Restore and restart" : versionGoesBack ? "Restore and restart" : action === "blocked" ? "Restore settings only" : "Restore settings"
    readonly property string footer: !cur ? "" : versionGoesBack ? "Then Hearth restarts (about 2 minutes)" : cur.changed > 0 ? "No restart needed" : "Nothing to put back"

    // ---- what runs
    Process {
        id: overviewProc
        stdout: StdioCollector { id: ovOut }
        onExited: code => {
            let d = null;
            try { d = JSON.parse(ovOut.text); } catch (e) { d = null; }
            if (d && d.points) { v.ov = d; v.sel = Math.min(Math.max(v.sel, d.points.length ? 1 : 0), d.points.length); v.error = ""; }
            else v.error = "System Restore couldn't read its restore points.";
            if (v.phase === "loading") v.phase = "idle";
        }
    }
    Process {
        id: snapProc
        stdout: StdioCollector { id: snapOut }
        onExited: code => {
            let d = null;
            try { d = JSON.parse(snapOut.text); } catch (e) {}
            v.phase = "idle";
            if (!d || d.ok === false) v.error = d && d.error ? d.error : "Couldn't make a restore point.";
            else { v.error = ""; v.sel = 1; }
            v.load();
        }
    }
    Process {
        id: changesProc
        stdout: StdioCollector { id: chOut }
        onExited: code => {
            let d = null;
            try { d = JSON.parse(chOut.text); } catch (e) {}
            v.changeList = d && d.changes ? d.changes : [];
            v.addedList = d && d.added ? d.added : [];
        }
    }
    Process {
        id: applyProc
        stdout: StdioCollector { id: apOut }
        onExited: code => {
            let d = null;
            try { d = JSON.parse(apOut.text); } catch (e) {}
            if (!d || d.ok !== true) {
                v.phase = "idle";
                v.error = d && d.error ? d.error : "System Restore stopped with a problem. Nothing was changed.";
                v.load();
                return;
            }
            v.doneRestart = d.restart === true;
            v.done = d.restart ? "Restarting into Hearth " + (d.plan && d.plan.to ? d.plan.to : "") + "…"
                : (d.changed === 1 ? "1 settings file went back." : d.changed + " settings files went back.") + " A copy of today's settings is kept as a restore point.";
            v.phase = "done";
            if (d.restart) restartTimer.start();
            v.load();
        }
    }
    Timer { id: restartTimer; interval: 1800; onTriggered: Quickshell.execDetached(["systemctl", "reboot"]) }

    function makePoint() {
        if (snapProc.running || phase !== "idle") return;
        phase = "working"; error = "";
        snapProc.command = ["python3", helper, "snapshot", "--kind", "manual"];
        snapProc.running = true;
    }
    function askRestore() { if (canRestore) { phase = "confirm"; dlgIdx = 0; } }
    function restore(settingsOnly) {
        if (!cur || applyProc.running) return;
        phase = "working"; error = "";
        applyProc.command = ["python3", helper, "apply", cur.id].concat(settingsOnly || !versionGoesBack ? ["--settings-only"] : []);
        applyProc.running = true;
    }
    function seeChanges() {
        if (!cur) return;
        changeList = []; addedList = [];
        changesProc.command = ["python3", helper, "changes", cur.id];
        changesProc.running = true;
        phase = "changes";
    }
    function closeAll() { if (closeWindow) closeWindow(); }
    function pick(i) { sel = Math.max(0, Math.min(rowCount - 1, i)); zone = "list"; }

    // ---- keys (the controller types these through hearth-pad)
    readonly property var dlgButtons: phase === "confirm" ? (versionGoesBack ? ["go", "settings", "cancel"] : ["go", "cancel"]) : phase === "done" ? (doneRestart ? [] : ["restart", "ok"]) : phase === "changes" ? ["ok"] : []
    function dlgPress(b) {
        if (b === "go") restore(false);
        else if (b === "settings") restore(true);
        else if (b === "restart") Quickshell.execDetached(["systemctl", "reboot"]);
        else phase = "idle";
    }
    Keys.onPressed: e => {
        const k = e.key;
        if (phase === "confirm" || phase === "done" || phase === "changes") {
            if (k === Qt.Key_Escape || k === Qt.Key_Back) { if (phase !== "done" || !doneRestart) phase = "idle"; }
            else if (k === Qt.Key_Left || k === Qt.Key_Up) dlgIdx = Math.max(0, dlgIdx - 1);
            else if (k === Qt.Key_Right || k === Qt.Key_Down || k === Qt.Key_Tab) dlgIdx = Math.min(dlgButtons.length - 1, dlgIdx + 1);
            else if ((k === Qt.Key_Return || k === Qt.Key_Enter || k === Qt.Key_Space) && dlgButtons.length) dlgPress(dlgButtons[dlgIdx]);
            e.accepted = true;
            return;
        }
        if (phase === "working" || phase === "loading") { e.accepted = true; return; }
        if (k === Qt.Key_Escape || k === Qt.Key_Back) closeAll();
        else if (zone === "list") {
            if (k === Qt.Key_Up) pick(sel - 1);
            else if (k === Qt.Key_Down) { if (sel < rowCount - 1) pick(sel + 1); else if (hasOlder && sel === rowCount - 1) zone = "older"; else zone = "footer"; }
            else if (k === Qt.Key_Right || k === Qt.Key_Tab) zone = "aside";
            else if (k === Qt.Key_Return || k === Qt.Key_Enter) { zone = "footer"; footIdx = 1; }
        } else if (zone === "older") {
            if (k === Qt.Key_Up) zone = "list";
            else if (k === Qt.Key_Down || k === Qt.Key_Tab) zone = "footer";
            else if (k === Qt.Key_Return || k === Qt.Key_Enter || k === Qt.Key_Space) showOlder = !showOlder;
        } else if (zone === "aside") {
            if (k === Qt.Key_Left) zone = "list";
            else if (k === Qt.Key_Down || k === Qt.Key_Tab) zone = "footer";
            else if (k === Qt.Key_Return || k === Qt.Key_Enter) seeChanges();
        } else if (zone === "footer") {
            if (k === Qt.Key_Up) zone = "list";
            else if (k === Qt.Key_Left) footIdx = Math.max(0, footIdx - 1);
            else if (k === Qt.Key_Right) footIdx = Math.min(2, footIdx + 1);
            else if (k === Qt.Key_Tab) zone = "list";
            else if (k === Qt.Key_Return || k === Qt.Key_Enter) { if (footIdx === 0) makePoint(); else if (footIdx === 1) askRestore(); else closeAll(); }
        }
        e.accepted = true;
    }

    // ---- pieces
    component Glyph: Canvas {
        property string d: ""
        property color c: "white"
        property real w: 3
        onDChanged: requestPaint()
        onCChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            ctx.scale(width / 24, height / 24);
            ctx.strokeStyle = c; ctx.lineWidth = w; ctx.lineCap = "round"; ctx.lineJoin = "round";
            ctx.path = d;
            ctx.stroke();
        }
    }
    component Btn: Rectangle {
        id: b
        property string label: ""
        property bool primary: false
        property bool enabledB: true
        property bool focused: false
        signal clicked()
        implicitWidth: bl.implicitWidth + (primary ? 36 : 28)
        implicitHeight: 34
        radius: 6
        color: primary ? (enabledB ? v.cAccent : v.cLine) : (ma.containsMouse ? "#22344a" : "transparent")
        border.width: primary ? 0 : 1
        border.color: v.cLine
        Rectangle { anchors.fill: parent; anchors.margins: -3; radius: 9; color: "transparent"; border.width: 2; border.color: v.cAccent; visible: b.focused }
        Text { id: bl; anchors.centerIn: parent; text: b.label; color: b.primary ? (b.enabledB ? v.cInk : "#7b8fa1") : v.cText; font.family: v.font; font.pixelSize: 13; font.weight: b.primary ? Font.DemiBold : Font.Normal }
        MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: b.enabledB ? Qt.PointingHandCursor : Qt.ArrowCursor; onClicked: if (b.enabledB) b.clicked() }
    }

    Rectangle { anchors.fill: parent; color: v.cBg }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // title bar
        Rectangle {
            Layout.fillWidth: true; Layout.preferredHeight: 40; color: v.cTitle
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 14; spacing: 10
                Glyph { Layout.preferredWidth: 16; Layout.preferredHeight: 16; d: "M3 12a9 9 0 1 0 3-6.7 M3 4v5h5 M12 7v5l3 2"; c: v.cAccent; w: 2 }
                Text { Layout.fillWidth: true; text: "System Restore"; color: v.cText; font.family: v.font; font.pixelSize: 13; font.weight: Font.Medium }
                Item {
                    Layout.preferredWidth: 46; Layout.fillHeight: true
                    Glyph { anchors.centerIn: parent; width: 10; height: 10; d: "M0 0l24 24M24 0L0 24"; c: v.cMuted; w: 2.4 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: v.closeAll() }
                }
            }
        }
        // header
        Rectangle {
            Layout.fillWidth: true; Layout.preferredHeight: hdr.implicitHeight + 32; color: v.cHead
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: v.cLine }
            ColumnLayout {
                id: hdr
                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 22; rightMargin: 22 }
                spacing: 3
                Text { text: "Restore Hearth to an earlier point"; color: v.cText; font.family: v.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Hearth and its settings go back to how they were. Your files, games and saves are never touched."; color: v.cMuted; font.family: v.font; font.pixelSize: 13 }
            }
        }
        // body
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.margins: 16; Layout.leftMargin: 22; Layout.rightMargin: 22
            spacing: 16
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true
                spacing: 0
                // column heads
                Item {
                    Layout.fillWidth: true; Layout.preferredHeight: 28
                    Row {
                        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                        Text { width: parent.width * 0.29; anchors.verticalCenter: parent.verticalCenter; text: "Date and time"; color: v.cMuted; font.family: v.font; font.pixelSize: 12 }
                        Text { width: parent.width * 0.49; anchors.verticalCenter: parent.verticalCenter; text: "Description"; color: v.cMuted; font.family: v.font; font.pixelSize: 12 }
                        Text { width: parent.width * 0.22; anchors.verticalCenter: parent.verticalCenter; text: "Type"; color: v.cMuted; font.family: v.font; font.pixelSize: 12 }
                    }
                    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: v.cLine }
                }
                ListView {
                    id: list
                    Layout.fillWidth: true; Layout.fillHeight: true
                    Layout.topMargin: 4
                    clip: true
                    spacing: 2
                    model: v.rowCount
                    currentIndex: v.sel
                    boundsBehavior: Flickable.StopAtBounds
                    onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                    delegate: Rectangle {
                        id: row
                        required property int index
                        readonly property var p: index >= 1 ? v.visiblePoints[index - 1] : null
                        width: list.width; height: 36; radius: 6
                        color: index === v.sel ? v.cSel : (rma.containsMouse ? "#1d2c3c" : "transparent")
                        border.width: index === v.sel ? 1 : 0
                        border.color: v.zone === "list" ? v.cAccent : v.cLine
                        Row {
                            anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                            Text { width: parent.width * 0.29; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; text: row.p ? v.when(row.p.created) : "Now"; color: v.cText; font.family: v.font; font.pixelSize: 13 }
                            Row {
                                width: parent.width * 0.49; anchors.verticalCenter: parent.verticalCenter; spacing: 8
                                Text { width: Math.min(implicitWidth, parent.width - (chip.visible ? chip.width + 8 : 0)); elide: Text.ElideRight; anchors.verticalCenter: parent.verticalCenter; text: row.p ? row.p.title : "Hearth " + v.running; color: v.cText; font.family: v.font; font.pixelSize: 13 }
                                Rectangle {
                                    id: chip
                                    visible: !row.p
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: chipT.implicitWidth + 14; height: 18; radius: 9; color: "#1f4a32"
                                    Text { id: chipT; anchors.centerIn: parent; text: "Running now"; color: "#7fe0a4"; font.family: v.font; font.pixelSize: 11; font.weight: Font.DemiBold }
                                }
                            }
                            Text { width: parent.width * 0.22; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; text: row.p ? row.p.type : ""; color: v.cMuted; font.family: v.font; font.pixelSize: 13 }
                        }
                        MouseArea { id: rma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { v.pick(row.index); v.forceActiveFocus(); } }
                    }
                }
                Text {
                    visible: v.phase !== "loading" && v.points.length === 0
                    Layout.fillWidth: true; Layout.topMargin: 6
                    wrapMode: Text.WordWrap
                    text: "No restore points yet. Hearth makes one before each update and once a week, or make one now."
                    color: v.cMuted; font.family: v.font; font.pixelSize: 12
                }
                // Show older restore points
                Rectangle {
                    visible: v.hasOlder
                    Layout.topMargin: 10
                    implicitWidth: olderRow.implicitWidth + 12; implicitHeight: 28; radius: 6
                    color: "transparent"; border.width: v.zone === "older" ? 2 : 0; border.color: v.cAccent
                    Row {
                        id: olderRow
                        anchors.verticalCenter: parent.verticalCenter; x: 6; spacing: 10
                        Rectangle {
                            width: 18; height: 18; radius: 4; anchors.verticalCenter: parent.verticalCenter
                            color: v.showOlder ? v.cAccent : "transparent"; border.width: v.showOlder ? 0 : 1; border.color: "#5d7488"
                            Glyph { anchors.centerIn: parent; width: 12; height: 12; visible: v.showOlder; d: v.pOk; c: v.cInk; w: 3 }
                        }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: "Show older restore points"; color: "#c9d6e2"; font.family: v.font; font.pixelSize: 13 }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: v.showOlder = !v.showOlder }
                }
            }
            // What will change
            Rectangle {
                Layout.preferredWidth: 300; Layout.fillHeight: true
                radius: 10; color: v.cPanel; border.width: 1; border.color: v.cLine
                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 16; spacing: 12
                    Text { text: "What will change"; color: v.cText; font.family: v.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Repeater {
                        model: v.changes
                        RowLayout {
                            required property var modelData
                            Layout.fillWidth: true; spacing: 10
                            Rectangle {
                                Layout.alignment: Qt.AlignTop
                                Layout.preferredWidth: 22; Layout.preferredHeight: 22; radius: 11; color: modelData.bg
                                Glyph { anchors.centerIn: parent; width: 12; height: 12; d: modelData.d; c: modelData.fg; w: 3 }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: modelData.t; color: v.cText; font.family: v.font; font.pixelSize: 13; font.weight: Font.Medium }
                                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: modelData.s; color: v.cMuted; font.family: v.font; font.pixelSize: 12; lineHeight: 1.15 }
                            }
                        }
                    }
                    Item { Layout.fillHeight: true }
                    Btn {
                        Layout.fillWidth: true
                        label: "See every changed setting"
                        enabledB: v.cur !== null && v.cur.changed > 0
                        opacity: enabledB ? 1 : 0.5
                        focused: v.zone === "aside"
                        onClicked: v.seeChanges()
                    }
                }
            }
        }
        Text {
            visible: v.error !== ""
            Layout.fillWidth: true; Layout.leftMargin: 22; Layout.rightMargin: 22; Layout.bottomMargin: 8
            wrapMode: Text.WordWrap; text: v.error; color: "#ff8a7a"; font.family: v.font; font.pixelSize: 13
        }
        // footer
        Rectangle {
            Layout.fillWidth: true; Layout.preferredHeight: 58; color: v.cHead
            Rectangle { width: parent.width; height: 1; color: v.cLine }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 16; anchors.rightMargin: 16; spacing: 8
                Btn { label: v.phase === "working" ? "Working…" : "Make a restore point now"; focused: v.zone === "footer" && v.footIdx === 0; onClicked: v.makePoint() }
                Text { Layout.fillWidth: true; horizontalAlignment: Text.AlignRight; rightPadding: 8; elide: Text.ElideLeft; text: v.footer; color: v.cMuted; font.family: v.font; font.pixelSize: 12 }
                Btn { primary: true; label: v.goLabel; enabledB: v.canRestore; focused: v.zone === "footer" && v.footIdx === 1; onClicked: v.askRestore() }
                Btn { label: "Cancel"; focused: v.zone === "footer" && v.footIdx === 2; onClicked: v.closeAll() }
            }
        }
    }

    // ---- dialogs: confirm, every changed setting, working, done
    Rectangle {
        anchors.fill: parent
        visible: v.phase === "confirm" || v.phase === "changes" || v.phase === "working" || v.phase === "done"
        color: "#b3000000"
        MouseArea { anchors.fill: parent }
        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 80, v.phase === "changes" ? 640 : 520)
            height: Math.min(parent.height - 80, dlg.implicitHeight + 40)
            radius: 10; color: v.cPanel; border.width: 1; border.color: v.cLine
            ColumnLayout {
                id: dlg
                anchors { left: parent.left; right: parent.right; top: parent.top; bottom: parent.bottom; margins: 20 }
                spacing: 12
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: v.phase === "confirm" ? "Restore to " + (v.cur ? v.when(v.cur.created) : "") + "?"
                        : v.phase === "changes" ? "Settings that go back"
                        : v.phase === "working" ? "Working…" : (v.doneRestart ? "Restarting" : "Done")
                    color: v.cText; font.family: v.font; font.pixelSize: 17; font.weight: Font.DemiBold
                }
                Text {
                    visible: v.phase !== "changes"
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: v.phase === "confirm"
                        ? (v.versionGoesBack ? "Hearth goes back to " + v.cur.plan.to + " and restarts. " + (v.cur.plan.from || v.running) + " stays in the boot menu, so you can come back from Settings › Updates & recovery or the boot menu. " : "")
                          + (v.cur && v.cur.changed > 0 ? v.cur.changed + (v.cur.changed === 1 ? " setting goes back" : " settings go back") + "; a copy of today's settings is kept first as a restore point. " : "")
                          + "Apps you installed and your files stay."
                        : v.phase === "working" ? "Keeping a copy of today's settings, then putting the restore point back. Don't turn the PC off."
                        : v.done
                    color: v.cMuted; font.family: v.font; font.pixelSize: 13; lineHeight: 1.2
                }
                ListView {
                    visible: v.phase === "changes"
                    Layout.fillWidth: true; Layout.fillHeight: true
                    Layout.preferredHeight: Math.min(360, contentHeight)
                    clip: true
                    model: v.changeList
                    delegate: Item {
                        required property var modelData
                        width: ListView.view.width; height: 34
                        Column {
                            anchors.verticalCenter: parent.verticalCenter; width: parent.width
                            Text { width: parent.width; elide: Text.ElideRight; text: modelData.file + (modelData.key ? " › " + modelData.key : ""); color: v.cText; font.family: v.font; font.pixelSize: 12 }
                            Text { width: parent.width; elide: Text.ElideRight; text: "Now: " + modelData.now + "   →   Then: " + modelData.then; color: v.cMuted; font.family: v.font; font.pixelSize: 11 }
                        }
                    }
                }
                Text {
                    visible: v.phase === "changes" && v.addedList.length > 0
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: "Made since then and kept as they are: " + v.addedList.join(", ")
                    color: v.cMuted; font.family: v.font; font.pixelSize: 12
                }
                Row {
                    Layout.alignment: Qt.AlignRight
                    spacing: 8
                    Repeater {
                        model: v.dlgButtons
                        Btn {
                            required property string modelData
                            required property int index
                            primary: index === 0 && modelData !== "ok"
                            label: modelData === "go" ? v.goLabel : modelData === "settings" ? "Only the settings" : modelData === "restart" ? "Restart now" : modelData === "ok" ? "OK" : "Cancel"
                            focused: v.dlgIdx === index
                            onClicked: v.dlgPress(modelData)
                        }
                    }
                }
            }
        }
    }
}
