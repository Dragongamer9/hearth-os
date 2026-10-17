// A setting row: icon, title, a line of explanation, and a value or arrow on the right.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: row
    property string icon
    property string title
    property string sub
    property string value
    property bool chevron: true
    property bool danger: false
    Layout.fillWidth: true
    implicitHeight: Math.max(64, col.implicitHeight + 24)

    RowLayout {
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        spacing: 14
        Rectangle {
            visible: row.icon !== ""
            Layout.preferredWidth: 38; Layout.preferredHeight: 38
            radius: 10
            color: Theme.panel3
            Icon { anchors.centerIn: parent; name: row.icon; size: 20; color: row.danger ? Theme.danger : Theme.text }
        }
        ColumnLayout {
            id: col
            Layout.fillWidth: true
            spacing: 3
            Text { Layout.fillWidth: true; text: row.title; color: row.danger ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; visible: row.sub !== ""; text: row.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        }
        Text { visible: row.value !== ""; text: row.value; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; Layout.maximumWidth: 260; elide: Text.ElideRight }
        Text { visible: row.chevron && row.interactive; text: "›"; color: Theme.muted; font.pixelSize: 20 }
    }
}
