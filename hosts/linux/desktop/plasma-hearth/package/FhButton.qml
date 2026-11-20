// Firehold's button (Settings, Lock, Power, Go, a notification's actions), mockups 07-10.
import QtQuick
import "fh.js" as F

Rectangle {
    id: b
    property string text: ""
    property string glyph: ""
    property bool primary: false
    property bool focused: false
    property real pad: 14
    property real h: 34
    property int size: 14
    signal pressed()
    function activate() { pressed(); }
    implicitWidth: rowB.implicitWidth + pad * 2
    implicitHeight: h
    radius: 8
    color: primary ? F.accent : ma.containsMouse ? "#3a312c" : "#302824"
    border.width: focused ? 2 : 0
    border.color: primary ? "#ffffff" : F.accent
    Row {
        id: rowB
        anchors.centerIn: parent
        spacing: 6
        Icon { visible: b.glyph !== ""; anchors.verticalCenter: parent.verticalCenter; name: b.glyph; size: b.size; color: "#ffffff" }
        Text { anchors.verticalCenter: parent.verticalCenter; text: b.text; color: "#ffffff"; font.pixelSize: b.size; font.weight: Font.DemiBold; font.family: F.font(Qt.fontFamilies()) }
    }
    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.pressed() }
}
