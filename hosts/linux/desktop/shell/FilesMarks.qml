// The small marks on an item's icon in Hearth Files (1.14): its git status (a coloured badge at the bottom right:
// M changed, + added, ? not in git yet, ! in conflict, ✓ up to date) and its tags (coloured dots at the bottom left,
// as Dolphin shows them). Draw-only: FilesExtras.qml says what to show.
import QtQuick

Item {
    id: m
    property var entry: null
    property var fx: null                  // FilesExtras
    property real size: 64
    anchors.fill: parent
    readonly property string git: fx && entry ? fx.gitOf(entry) : ""
    readonly property var tags: entry && entry.tags ? entry.tags.slice(0, 3) : []
    readonly property real dot: Math.max(7, Math.min(14, size * 0.16))

    Rectangle {
        visible: m.git !== "" && m.git !== "ignored"
        width: Math.min(28, Math.max(14, m.size * 0.3)); height: width
        radius: width / 2
        anchors { right: parent.right; bottom: parent.bottom; rightMargin: -2; bottomMargin: -2 }
        color: m.fx ? m.fx.gitColor(m.git) : "transparent"
        border.width: 2
        border.color: Theme.panel
        Text {
            anchors.centerIn: parent
            text: m.fx && m.fx.gitLook[m.git] ? m.fx.gitLook[m.git].mark : ""
            color: "white"
            font.family: Theme.font; font.pixelSize: Math.max(9, parent.width * 0.56); font.weight: Font.Bold
        }
    }
    Row {
        visible: m.tags.length > 0
        anchors { left: parent.left; bottom: parent.bottom; leftMargin: -1; bottomMargin: -1 }
        spacing: -m.dot * 0.3
        Repeater {
            model: m.tags
            delegate: Rectangle {
                required property string modelData
                width: m.dot; height: m.dot; radius: m.dot / 2
                color: m.fx ? m.fx.tagColor(modelData) : Theme.accent
                border.width: 1.5
                border.color: Theme.panel
            }
        }
    }
}
