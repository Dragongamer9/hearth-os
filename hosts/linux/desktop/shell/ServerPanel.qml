// Server mode: a panel on an opened tile's page (the cache, the ad blocker, quiet hours & rules): its title in small
// capitals and its rows. Draw-only.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: p
    default property alias body: inner.data
    property string title: ""
    property real k: 1
    Layout.fillWidth: true
    implicitHeight: inner.implicitHeight + 40 * k
    radius: 14 * k; color: "#211b14"; border.color: "#3a2f22"
    ColumnLayout {
        id: inner
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 * p.k }
        spacing: 10 * p.k
        Text { text: p.title.toUpperCase(); color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 16 * p.k; font.weight: Font.DemiBold; font.letterSpacing: 1.2 * p.k }
    }
}
