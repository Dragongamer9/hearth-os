// The bottom of Game Mode: what's running, and what the buttons do here.
import QtQuick

Rectangle {
    id: foot
    property var hints: []        // [[glyph, label], ...]
    height: Theme.u(46)
    color: Theme.bg2
    Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }

    Row {
        visible: App.running !== ""
        anchors { left: parent.left; leftMargin: Theme.gut; verticalCenter: parent.verticalCenter }
        spacing: Theme.u(8)
        Rectangle { width: Theme.u(8); height: width; radius: width / 2; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
        Text {
            text: { const x = App.g(App.running); return x ? "Playing " + x.title : "A game is running"; }
            color: Theme.text
            font.family: Theme.font
            font.pixelSize: Theme.u(13)
            anchors.verticalCenter: parent.verticalCenter
        }
    }
    Row {
        anchors { right: parent.right; rightMargin: Theme.gut; verticalCenter: parent.verticalCenter }
        spacing: Theme.u(18)
        Repeater {
            model: foot.hints
            delegate: Row {
                required property var modelData
                spacing: Theme.u(7)
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    readonly property string g: modelData[0]
                    width: Math.max(Theme.u(22), gl.implicitWidth + Theme.u(12)); height: Theme.u(22); radius: height / 2
                    color: g === "A" ? "#3a7d3a" : g === "B" ? "#9a3434" : g === "X" ? "#2f5f9a" : g === "Y" ? "#9a7f2a" : Theme.panel3
                    Text { id: gl; anchors.centerIn: parent; text: parent.g; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(11); font.weight: Font.Bold }
                }
                Text { anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13) }
            }
        }
    }
}
