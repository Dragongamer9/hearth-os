// One block added to a Mantel app from its Blocks window (drag and drop board 3; AppBlocks.qml puts it among a bar's
// own items, or anywhere on the window): one of the app's buttons ("action": Sort my Downloads, Share, Open in
// terminal…), a button you made (runs your own command, as you typed it, only when you press it), a count, words or a
// spacer. Its look, what it shows and does, when it shows and how it moves come from Customize (app-blocks.js
// checkLook). While editing it's dragged anywhere and right-clicked for its menu (AppBlocksEditor.qml).
import QtQuick
import QtQuick.Layouts
import Quickshell
import "app-blocks.js" as L

Item {
    id: bk
    property var ab: null
    property var block: ({})
    readonly property bool free: !block.in
    readonly property var look: { if (!ab) return ({}); void ab.effLooks; return ab.effLooks[block.id] || ({}); }
    readonly property var act: ab && block.kind === "action" ? ab.actionOf(block.ref) : null
    readonly property bool editing: !!ab && ab.editing
    readonly property bool lit: editing && ab.picked === block.id
    // what's in it (Shows): its own (the action's name, your words), a count, words, the clock, space
    readonly property string shows: look.shows && look.shows !== "own" ? look.shows : block.kind === "count" ? "count" : block.kind === "words" ? "words" : block.kind === "spacer" ? "space" : "own"
    readonly property var countNow: ab && (shows === "count") ? ab.countValue(look.showsRef || block.ref || (ab.counts.length ? ab.counts[0].id : "")) : ""
    property var lastCount: undefined
    onCountNowChanged: { if (lastCount !== undefined && Number(countNow) > Number(lastCount) && look.bounce) bounce.restart(); lastCount = countNow; }
    property date now: new Date()
    Timer { interval: 30000; repeat: true; running: bk.shows === "clock" && bk.visible; onTriggered: bk.now = new Date() }
    readonly property string words: {
        const vals = Object.assign({}, ab ? ab.wordValues : {}, { count: countNow });
        if (look.words) return L.fillWords(look.words, vals);
        if (shows === "count") { const c = ab ? ab.countOf(look.showsRef || block.ref) : null; return (c ? c.label + " · " : "") + (countNow === "" ? "–" : countNow); }
        if (shows === "clock") return Qt.formatTime(now, "h:mm AP");
        if (shows === "words") return L.fillWords(block.label || "Your words", vals);
        if (shows === "space") return "";
        if (block.kind === "action") return block.label || (act ? act.label : "A button");
        return block.label || "My button";
    }
    readonly property string iconName: look.icon || (block.kind === "action" && act ? (act.icon || "") : block.kind === "button" ? "rocket" : "")
    readonly property string showWhat: look.show || (shows === "space" ? "" : iconName && words ? "both" : words ? "words" : "icon")
    readonly property real fs: look.fontSize || 13
    readonly property real side: look.hMode === "fixed" && look.hPx ? look.hPx : free ? 36 : 32
    // When it shows
    readonly property bool something: shows !== "count" || (countNow !== "" && Number(countNow) !== 0)
    readonly property bool whenOk: editing || L.showsNow(look, hov.hovered, something, ab ? ab.page : "")
    readonly property var m: look.m || [0, 0, 0, 0]
    implicitHeight: side + m[0] + m[2]
    implicitWidth: (look.wMode === "fixed" && look.wPx ? look.wPx : shows === "space" ? (editing ? 28 : 16) : Math.max(side, row.implicitWidth + 2 * (look.pad !== undefined ? look.pad : 10))) + m[1] + m[3]
    width: implicitWidth; height: implicitHeight
    // Width: Fill (in a bar, the room that's left)
    Layout.fillWidth: look.wMode === "fill"
    x: free && ab ? Math.round((block.x || 0) * ab.width) : 0
    y: free && ab ? Math.round((block.y || 0) * ab.height) : 0
    visible: whenOk || look.when === "hover"
    opacity: (whenOk ? 1 : 0.001) * (editing && ab.dragId === block.id ? 0.35 : 1) * appearT
    z: ab ? 1 + ab.layerOf(block.id) : 1
    // Motion
    property real appearT: 1
    property real bounceT: 0
    onVisibleChanged: if (visible && !editing && (look.appear === "fade" || look.appear === "pop" || look.appear === "slide")) { appearT = 0; appearAnim.restart(); }
    NumberAnimation { id: appearAnim; target: bk; property: "appearT"; from: 0; to: 1; duration: 240; easing.type: Easing.OutCubic }
    SequentialAnimation {
        id: bounce
        NumberAnimation { target: bk; property: "bounceT"; from: 0; to: 1; duration: 110; easing.type: Easing.OutQuad }
        NumberAnimation { target: bk; property: "bounceT"; from: 1; to: 0; duration: 280; easing.type: Easing.OutBounce }
    }
    transform: [
        Translate { y: (bk.look.appear === "slide" ? (1 - bk.appearT) * 12 : 0) - bk.bounceT * 5 },
        Scale { origin.x: bk.width / 2; origin.y: bk.height / 2; xScale: bk.look.appear === "pop" ? 0.85 + 0.15 * bk.appearT : 1; yScale: xScale }
    ]

    Item {
        id: face
        x: bk.m[3]; y: bk.m[0]
        width: bk.width - bk.m[1] - bk.m[3]; height: bk.height - bk.m[0] - bk.m[2]
        readonly property real rad: bk.look.corners !== undefined ? bk.look.corners : Theme.r(8)
        Repeater {
            model: bk.look.shadow === "deep" ? 3 : bk.look.shadow === "soft" ? 2 : 0
            delegate: Rectangle {
                required property int index
                anchors { fill: parent; margins: -(index + 1) * 2; topMargin: (index + 1) * 2 }
                radius: face.rad + index + 1
                color: Qt.rgba(0, 0, 0, (bk.look.shadow === "deep" ? 0.24 : 0.16) / (index + 1))
            }
        }
        Repeater {
            model: bk.look.edge === "glow" ? 3 : 0
            delegate: Rectangle {
                required property int index
                anchors { fill: parent; margins: -(index + 1) * 2 }
                radius: face.rad + (index + 1) * 2
                color: "transparent"; border.width: 2
                border.color: { const c = bk.look.edgeColor ? Qt.color(bk.look.edgeColor) : Theme.accent; return Qt.rgba(c.r, c.g, c.b, 0.34 / (index + 1)); }
            }
        }
        Rectangle {
            anchors.fill: parent
            radius: face.rad
            readonly property color base: bk.look.fill ? Qt.color(bk.look.fill) : bk.free ? Theme.panel2 : "transparent"
            color: {
                const l = bk.look;
                let c = base;
                if (ma.pressed && l.pressed === "darker") return Qt.darker(c.a > 0 ? c : Theme.panel3, 1.3);
                if (ma.pressed && l.pressed === "own" && l.pressedColor) return l.pressedColor;
                if (hov.hovered && !bk.editing) {
                    if (l.hover === "accent") return Theme.accent;
                    if (l.hover === "own" && l.hoverColor) return l.hoverColor;
                    if (l.hover === "lighter" || !l.hover) return c.a > 0 ? Qt.lighter(c, 1.25) : Theme.panel3;
                }
                return c.a === 0 ? "transparent" : Qt.rgba(c.r, c.g, c.b, c.a * (1 - (l.see || 0) / 100));
            }
            border.width: bk.lit ? 2 : bk.look.edge === "line" || bk.look.edge === "glow" ? (bk.look.edgeW || 1) : (bk.editing ? 1 : 0)
            border.color: bk.lit ? Theme.accent : bk.look.edgeColor ? bk.look.edgeColor : bk.look.edge ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.6)
        }
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 6
            Icon {
                visible: (bk.showWhat === "icon" || bk.showWhat === "both") && bk.iconName !== ""
                name: bk.iconName || "dot"; size: bk.look.iconSize || 16
                color: hov.hovered && bk.look.hover === "accent" && !bk.editing ? Theme.accentInk : bk.look.iconColor || Theme.text
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                visible: (bk.showWhat === "words" || bk.showWhat === "both") && bk.words !== ""
                text: bk.words
                color: hov.hovered && bk.look.hover === "accent" && !bk.editing ? Theme.accentInk : bk.look.textColor || Theme.text
                font.family: bk.look.font || Theme.font
                font.pixelSize: bk.fs
                font.weight: bk.look.weight === "bold" ? Font.Bold : bk.look.weight === "light" ? Font.Light : bk.look.weight === "medium" ? Font.Medium : Font.Normal
                anchors.verticalCenter: parent.verticalCenter
            }
            Text { visible: bk.shows === "space" && bk.editing; text: "space"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
        }
    }
    HoverHandler { id: hov; onHoveredChanged: if (hovered && !bk.editing && bk.look.does && bk.look.does.hover) bk.ab.runDoes(bk.look.does.hover, 0, 0) }
    // what it does: press (its own: the app's action, or your command), right-click, middle-click, scroll
    function pressed(x, y) {
        const d = look.does && look.does.press && look.does.press.k ? look.does.press : null;
        const q = mapToItem(ab, x, y);
        if (d) return ab.runDoes(d, q.x, q.y);
        if (block.kind === "action") return ab.runAction(block.ref, q.x, q.y + 4);
        if (block.kind === "button") {
            if (block.run) Quickshell.execDetached(["sh", "-c", block.run]);
            else ab.toast("Right-click it › Customize › Does to say what this button runs.", false);
        }
    }
    MouseArea {
        id: ma
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        preventStealing: bk.editing
        cursorShape: bk.editing ? Qt.OpenHandCursor : Qt.PointingHandCursor
        onPressed: m => { if (bk.editing && m.button === Qt.LeftButton && bk.ab.editorItem) bk.ab.editorItem.grabBlock(bk.block.id, mapToItem(bk.ab, m.x, m.y)); }
        onPositionChanged: m => { if (bk.editing && pressed && bk.ab.editorItem) bk.ab.editorItem.moveDrag(mapToItem(bk.ab, m.x, m.y)); }
        onReleased: m => { if (bk.editing && m.button === Qt.LeftButton && bk.ab.editorItem) bk.ab.editorItem.endDrag(); }
        onClicked: m => {
            if (bk.editing) {
                if (m.button === Qt.RightButton && bk.ab.editorItem) { const q = mapToItem(bk.ab, m.x, m.y); bk.ab.editorItem.partMenu(bk.block.id, q.x, q.y); }
                return;
            }
            const how = m.button === Qt.RightButton ? "right" : m.button === Qt.MiddleButton ? "middle" : "";
            if (how) { const d = bk.look.does && bk.look.does[how]; if (d && d.k) { const q = mapToItem(bk.ab, m.x, m.y); bk.ab.runDoes(d, q.x, q.y); } return; }
            bk.pressed(m.x, m.y);
        }
        onWheel: w => { const d = bk.look.does && bk.look.does.scroll; if (!bk.editing && d && d.k) bk.ab.runDoes(d, 0, 0); else w.accepted = false; }
    }
}
