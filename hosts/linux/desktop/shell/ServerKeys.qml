// Server mode: the controller keys along the bottom of an opened tile's page ("Ⓐ Turn on or off  Ⓧ Empty the cache …
// Ⓑ Back to the dashboard"). keys: [[letter, text, action]]; a click sends that action. Draw-only.
import QtQuick
import QtQuick.Layouts

RowLayout {
    id: ks
    property real k: 1
    property var keys: []
    property string back: "Back to the dashboard"
    signal pressed(string a)
    signal backPressed()
    Layout.fillWidth: true
    spacing: 30 * k
    Repeater {
        model: ks.keys
        delegate: Row {
            required property var modelData
            spacing: 8 * ks.k
            Rectangle {
                width: 26 * ks.k; height: width; radius: modelData[0] === "☰" ? 4 * ks.k : width / 2
                color: "transparent"; border.color: "#c9b79f"; border.width: modelData[0] === "☰" ? 0 : 1.5
                anchors.verticalCenter: parent.verticalCenter
                Text { anchors.centerIn: parent; text: modelData[0]; color: "#fff4df"; font.family: Theme.font; font.pixelSize: (modelData[0] === "☰" ? 20 : 14) * ks.k; font.weight: Font.Bold }
            }
            Text { anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 19 * ks.k }
            MouseArea { width: parent.width; height: parent.height; onClicked: ks.pressed(modelData[2]) }
        }
    }
    Item { Layout.fillWidth: true }
    Row {
        spacing: 8 * ks.k
        Rectangle { width: 26 * ks.k; height: width; radius: width / 2; color: "transparent"; border.color: "#c9b79f"; border.width: 1.5; anchors.verticalCenter: parent.verticalCenter
            Text { anchors.centerIn: parent; text: "B"; color: "#fff4df"; font.pixelSize: 14 * ks.k; font.weight: Font.Bold } }
        Text { anchors.verticalCenter: parent.verticalCenter; text: ks.back; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 19 * ks.k }
        MouseArea { width: parent.width; height: parent.height; onClicked: ks.backPressed() }
    }
}
