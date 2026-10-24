// A button: "pill" (the usual), "primary" (accent: Play, Install), "ghost", "danger", "menu" (a full-width line in a
// menu or panel) or "icon" (square, icon only).
import QtQuick

Focusable {
    id: b
    property string text: ""
    property string icon: ""
    property string kind: "pill"
    property string sub: ""
    property string value: ""
    property bool on: false
    property real minW: 0
    readonly property bool primary: kind === "primary"
    readonly property color ink: primary ? Theme.accentInk : kind === "danger" ? Theme.danger : Theme.text
    radius: kind === "menu" ? Theme.u(8) : kind === "icon" ? Theme.u(8) : Theme.u(10)
    implicitHeight: kind === "menu" ? Math.max(Theme.u(46), col.implicitHeight + Theme.u(18)) : kind === "icon" ? Theme.u(44) : Theme.u(44)
    implicitWidth: kind === "icon" ? Theme.u(44) : kind === "menu" ? Math.max(minW, Theme.u(320)) : Math.max(minW, row.implicitWidth + Theme.u(36))

    Rectangle {
        anchors.fill: parent
        radius: b.radius
        color: b.primary ? (b.isFocused || b.hovered ? Qt.lighter(Theme.accent, 1.08) : Theme.accent)
             : b.kind === "ghost" ? (b.isFocused || b.hovered ? Theme.panel2 : "transparent")
             : b.kind === "menu" ? (b.isFocused || b.hovered ? Theme.panel3 : "transparent")
             : b.on ? Theme.alpha(Theme.accent, 0.22)
             : (b.isFocused || b.hovered ? Theme.panel3 : Theme.panel2)
        border.width: b.kind === "ghost" ? 1 : 0
        border.color: Theme.line
        Behavior on color { ColorAnimation { duration: Theme.quick } }
    }
    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: b.kind === "icon" ? (b.width - Theme.u(20)) / 2 : b.kind === "menu" ? Theme.u(14) : Theme.u(18)
        spacing: Theme.u(10)
        Icon { visible: b.icon !== ""; name: b.icon; color: b.ink; size: Theme.u(20); anchors.verticalCenter: parent.verticalCenter }
        Column {
            id: col
            visible: b.kind !== "icon"
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.u(2)
            width: b.kind === "menu" ? b.width - Theme.u(14) * 2 - (b.icon ? Theme.u(30) : 0) - (valueText.visible ? valueText.width + Theme.u(10) : 0) : implicitWidth
            Text {
                text: b.text
                color: b.ink
                font.family: Theme.font
                font.pixelSize: Theme.u(15)
                font.weight: b.primary ? Font.Bold : Font.DemiBold
                elide: Text.ElideRight
                width: b.kind === "menu" ? parent.width : implicitWidth
            }
            Text {
                visible: b.sub !== ""
                text: b.sub
                color: b.primary ? Theme.accentInk : Theme.dim
                font.family: Theme.font
                font.pixelSize: Theme.u(12)
                wrapMode: Text.WordWrap
                width: b.kind === "menu" ? parent.width : Math.min(implicitWidth, Theme.u(420))
            }
        }
    }
    Text {
        id: valueText
        visible: b.value !== "" && b.kind === "menu"
        anchors.right: parent.right
        anchors.rightMargin: Theme.u(14)
        anchors.verticalCenter: parent.verticalCenter
        text: b.value
        color: Theme.dim
        font.family: Theme.font
        font.pixelSize: Theme.u(14)
    }
}
