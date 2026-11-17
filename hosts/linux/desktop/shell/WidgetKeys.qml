// Keyboard indicator widget (1.14): Caps Lock, Num Lock and the keyboard layout (NotifyHub.qml: the keyboards' LEDs
// and KWin's layouts, or Settings › Keyboard's on labwc). Click the layout for the next one (where it can switch). In
// the bar: the layout and a lit A while Caps Lock is on.
import QtQuick
import QtQuick.Layouts

WidgetBase {
    id: w
    readonly property bool caps: NotifyHub.keys.caps === true
    readonly property bool num: NotifyHub.keys.num === true
    implicitWidth: compact ? barRow.implicitWidth : 280
    implicitHeight: compact ? 24 : 140

    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6
        Key { label: NotifyHub.layoutShort || "—"; lit: false; small: true; onPicked: NotifyHub.nextLayout() }
        Key { visible: w.caps; label: "A"; lit: true; small: true }
        Key { visible: w.num; label: "1"; lit: true; small: true }
    }
    ColumnLayout {
        visible: !w.compact
        anchors { fill: parent; margins: 14 * w.k }
        spacing: 8 * w.k
        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Key { label: "Caps"; lit: w.caps }
            Key { label: "Num"; lit: w.num }
            Item { Layout.fillWidth: true }
            Key { label: NotifyHub.layoutShort || "—"; lit: false; wide: true; onPicked: NotifyHub.nextLayout() }
        }
        Text {
            Layout.fillWidth: true
            text: (NotifyHub.layoutName || "Keyboard") + ((NotifyHub.layout.layouts || []).length > 1 ? (NotifyHub.layout.canSwitch ? " · click for the next layout" : " · " + (NotifyHub.layout.layouts.length) + " layouts") : "")
            color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k); elide: Text.ElideRight
        }
        Text {
            visible: !NotifyHub.keys.known
            Layout.fillWidth: true
            text: "No keyboard lights to read here"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
        }
    }

    component Key: Rectangle {
        property string label: ""
        property bool lit: false
        property bool small: false
        property bool wide: false
        signal picked()
        implicitWidth: small ? Math.max(22, kt.implicitWidth + 10) : wide ? Math.max(64, kt.implicitWidth + 24) * w.k : 64 * w.k
        implicitHeight: small ? 22 : 54 * w.k
        radius: Theme.r(small ? 6 : 10)
        color: lit ? Theme.accent : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.08)
        border.color: lit ? Theme.accent : Theme.line
        Text { id: kt; anchors.centerIn: parent; text: parent.label; color: parent.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: parent.small ? 12 : Math.round(16 * w.k); font.bold: true }
        Rectangle { visible: !parent.small; anchors { top: parent.top; right: parent.right; margins: 7 } width: 7; height: 7; radius: 4; color: parent.lit ? Theme.accentInk : Theme.muted; opacity: parent.lit ? 1 : 0.4 }
        MouseArea { anchors.fill: parent; onClicked: parent.picked() }
    }
}
