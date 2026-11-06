// Controller button hints for the app menus: [{b: "A", t: "Open"}, ...] as round (A B X Y) or square (LB RB) keys.
import QtQuick
import QtQuick.Layouts

RowLayout {
    id: h
    property var model: []
    property color ink: Theme.muted
    property real k: 1
    spacing: 14 * k
    readonly property var colors: ({ A: "#3fae5a", B: "#d9534f", X: "#3b7de0", Y: "#d9a521" })
    Repeater {
        model: h.model
        delegate: RowLayout {
            required property var modelData
            spacing: 6 * h.k
            Rectangle {
                readonly property bool round: modelData.b.length === 1
                implicitWidth: round ? 22 * h.k : keyText.implicitWidth + 12 * h.k
                implicitHeight: 22 * h.k
                radius: round ? height / 2 : Theme.r(5)
                color: "transparent"
                border.width: 1.5
                border.color: h.colors[modelData.b] || h.ink
                Text { id: keyText; anchors.centerIn: parent; text: modelData.b; color: h.colors[modelData.b] || Theme.text; font.family: Theme.font; font.pixelSize: (modelData.b.length > 1 ? 9 : 11) * h.k; font.bold: true }
            }
            Text { text: modelData.t; color: h.ink; font.family: Theme.font; font.pixelSize: 13 * h.k }
        }
    }
}
