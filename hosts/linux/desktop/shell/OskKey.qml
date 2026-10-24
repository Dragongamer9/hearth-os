// One key of Hearth's on-screen keyboard (OskBoard.qml). A tap types it; a long touch opens its accents.
import QtQuick

Rectangle {
    id: k
    property var owner             // the OskBoard it's on
    property var key: ({})
    property int row: 0
    property int col: 0
    readonly property var th: owner.th
    readonly property var vm: owner.vm
    readonly property bool mod: owner.modKeys.indexOf(key.t) >= 0
    readonly property bool sel: !!vm && vm.active && vm.sel.r === row && vm.sel.c === col
    readonly property bool aimL: !!(vm && vm.split && vm.split.l && vm.split.l.r === row && vm.split.l.c === col)
    readonly property bool aimR: !!(vm && vm.split && vm.split.r && vm.split.r.r === row && vm.split.r.c === col)
    readonly property bool lit: sel || aimL || aimR
    height: owner.keyH
    radius: owner.s.size === "s" ? 9 : owner.s.size === "l" ? 13 : 11
    color: key.t === "none" || key.emoji ? "transparent"
        : lit ? (key.go ? th.accent : th.sel)
        : key.go || key.lock ? th.accent
        : key.on ? Qt.tint(th.key, Qt.rgba(Qt.color(th.accent).r, Qt.color(th.accent).g, Qt.color(th.accent).b, 0.38))
        : mod ? th.mod : th.key
    opacity: key.off ? 0.35 : 1
    border.width: aimL || aimR ? 3 : 0
    border.color: aimR ? "#ffd36b" : th.accent
    scale: touch.pressed ? 0.96 : 1
    Behavior on color { ColorAnimation { duration: 90 } }
    onSelChanged: if (sel) owner.selItem = k
    Component.onCompleted: if (sel) owner.selItem = k
    Text {
        anchors.centerIn: parent
        width: parent.width - 8
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        text: owner.label(k.key)
        color: k.lit && !k.key.go ? k.th.selText : (k.key.go || k.key.lock) ? k.th.ink : k.mod || k.key.t === "none" ? k.th.muted : k.th.text
        font.family: Theme.font
        font.pixelSize: k.key.t === "space" || k.key.t === "layout" ? 12 : k.key.emoji ? Math.round(owner.fontPx * 1.25) : k.key.clip ? Math.round(owner.fontPx * 0.8) : k.mod ? Math.round(owner.fontPx * 0.85) : owner.fontPx
        font.weight: Font.DemiBold
    }
    Text {
        visible: text !== ""
        anchors { top: parent.top; right: parent.right; topMargin: 4; rightMargin: 6 }
        text: owner.glyph(k.key)
        color: k.lit ? k.th.selText : k.th.muted
        opacity: 0.6
        font.pixelSize: 10; font.weight: Font.Black
    }
    MouseArea {
        id: touch
        anchors.fill: parent
        enabled: k.key.t !== "none"
        pressAndHoldInterval: 480
        property bool held: false
        onPressed: held = false
        onPressAndHold: { held = true; owner.longPressed(k.row, k.col); }
        onReleased: if (!held && containsMouse) owner.tapped(k.row, k.col)
        hoverEnabled: false
    }
}
