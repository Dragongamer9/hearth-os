// Server mode: a button with its controller key ("Switch now  A"). Draw-only; clicks too.
import QtQuick

Rectangle {
    id: b
    property real k: 1
    property string label: ""
    property string keyName: ""
    property bool primary: false
    signal clicked()
    implicitWidth: row.implicitWidth + 40 * k
    implicitHeight: 50 * k
    radius: 12 * k
    color: primary ? Theme.accent : SetStyle.card2
    border.width: primary ? 3 * k : 1
    border.color: primary ? Qt.lighter(Theme.accent, 1.35) : Theme.line
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 10 * b.k
        Text { anchors.verticalCenter: parent.verticalCenter; text: b.label; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 17 * b.k; font.weight: Font.DemiBold }
        Rectangle {
            visible: b.keyName !== ""
            anchors.verticalCenter: parent.verticalCenter
            width: 24 * b.k; height: width; implicitWidth: 24 * b.k; implicitHeight: 24 * b.k; radius: width / 2; color: "transparent"
            border.color: b.primary ? Theme.accentInk : Theme.muted
            Text { anchors.centerIn: parent; text: b.keyName; color: b.primary ? Theme.accentInk : Theme.text; font.pixelSize: 12 * b.k; font.weight: Font.Bold }
        }
    }
    MouseArea { anchors.fill: parent; onClicked: b.clicked() }
}
