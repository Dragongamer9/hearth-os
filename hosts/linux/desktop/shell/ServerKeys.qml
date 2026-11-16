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
        // An Item round each key, so its click area can fill it (a MouseArea sized to its own Row made the Row grow
        // forever: opening the cache, ad blocker or rules page froze the dashboard)
        delegate: Item {
            id: key
            required property var modelData
            implicitWidth: kr.implicitWidth; implicitHeight: kr.implicitHeight
            Row {
                id: kr
                spacing: 8 * ks.k
                Rectangle {
                    width: 26 * ks.k; height: width; radius: key.modelData[0] === "☰" ? 4 * ks.k : width / 2
                    color: "transparent"; border.color: "#c9b79f"; border.width: key.modelData[0] === "☰" ? 0 : 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    Text { anchors.centerIn: parent; text: key.modelData[0]; color: "#fff4df"; font.family: Theme.font; font.pixelSize: (key.modelData[0] === "☰" ? 20 : 14) * ks.k; font.weight: Font.Bold }
                }
                Text { anchors.verticalCenter: parent.verticalCenter; text: key.modelData[1]; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 19 * ks.k }
            }
            MouseArea { anchors.fill: parent; onClicked: ks.pressed(key.modelData[2]) }
        }
    }
    Item { Layout.fillWidth: true }
    Item {
        implicitWidth: br.implicitWidth; implicitHeight: br.implicitHeight
        Row {
            id: br
            spacing: 8 * ks.k
            Rectangle { width: 26 * ks.k; height: width; radius: width / 2; color: "transparent"; border.color: "#c9b79f"; border.width: 1.5; anchors.verticalCenter: parent.verticalCenter
                Text { anchors.centerIn: parent; text: "B"; color: "#fff4df"; font.pixelSize: 14 * ks.k; font.weight: Font.Bold } }
            Text { anchors.verticalCenter: parent.verticalCenter; text: ks.back; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 19 * ks.k }
        }
        MouseArea { anchors.fill: parent; onClicked: ks.backPressed() }
    }
}
