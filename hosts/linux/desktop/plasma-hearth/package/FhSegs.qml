// Firehold's segmented choice (Couch / Desk / Away, the outputs, the power profile), mockups 07-08.
import QtQuick
import "fh.js" as F

Rectangle {
    id: s
    property var items: []          // [{id, text, color?}]
    property string current: ""
    property bool focused: false
    property real itemWidth: 0      // 0: as wide as its words
    property real itemHeight: 38
    signal picked(string id)
    function adjust(d) {
        const i = items.findIndex(x => x.id === current);
        const n = Math.max(0, Math.min(items.length - 1, (i < 0 ? 0 : i) + d));
        if (items[n] && items[n].id !== current) picked(items[n].id);
    }
    function activate() { adjust(1); }
    radius: 10
    color: F.seg
    border.width: focused ? 2 : 0
    border.color: F.accent
    implicitWidth: r.implicitWidth + 8
    implicitHeight: itemHeight + 8
    Row {
        id: r
        x: 4; y: 4
        spacing: 2
        Repeater {
            model: s.items
            delegate: Rectangle {
                required property var modelData
                readonly property bool sel: modelData.id === s.current
                width: s.itemWidth > 0 ? s.itemWidth : lab.implicitWidth + 30
                height: s.itemHeight
                radius: 8
                color: sel ? (modelData.color || F.accent) : "transparent"
                Text { id: lab; anchors.centerIn: parent; text: parent.modelData.text; color: parent.sel ? "#ffffff" : F.muted; font.pixelSize: 15; font.weight: parent.sel ? Font.DemiBold : Font.Medium; font.family: F.font(Qt.fontFamilies()) }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: s.picked(parent.modelData.id) }
            }
        }
    }
}
