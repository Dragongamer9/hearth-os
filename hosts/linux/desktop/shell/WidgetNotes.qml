// Sticky note widget: a coloured note you type straight into. What you type is kept a moment after you stop typing
// (and when you click away), merged into widgets.json for this note only. Settings: its colour and text size.
import QtQuick
import QtQuick.Layouts

WidgetBase {
    id: w
    paintsOwn: true
    readonly property var colors: ({ yellow: "#ffe58a", pink: "#ffb3c7", green: "#b8eeb0", blue: "#a9d8ff", purple: "#d4c2ff", dark: "#3a3430" })
    readonly property string tint: colors[c("color", "yellow")] ? c("color", "yellow") : "yellow"
    readonly property color ink: tint === "dark" ? "#f4ece3" : "#2b2117"
    readonly property int textSize: c("size", 17)
    implicitWidth: 260; implicitHeight: 240

    function keep() { if (edit.text !== c("text", "")) set("text", edit.text); }
    Timer { id: saveSoon; interval: 900; onTriggered: w.keep() }

    Rectangle {
        anchors.fill: parent
        radius: Theme.r(6)
        color: w.colors[w.tint]
        Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 22; radius: parent.radius; color: Qt.darker(w.colors[w.tint], 1.08) }
    }
    Flickable {
        visible: !w.configuring
        anchors { fill: parent; margins: 14; topMargin: 26 }
        contentHeight: edit.paintedHeight
        clip: true
        TextEdit {
            id: edit
            width: parent.width
            text: w.c("text", "")
            wrapMode: TextEdit.Wrap
            color: w.ink
            selectionColor: Qt.darker(w.colors[w.tint], 1.4)
            font.family: Theme.font
            font.pixelSize: w.textSize
            onTextChanged: if (activeFocus) saveSoon.restart()
            onActiveFocusChanged: if (!activeFocus) w.keep()
            Text { visible: !edit.text && !edit.activeFocus; text: "Write something…"; color: Qt.rgba(w.ink.r, w.ink.g, w.ink.b, 0.45); font: edit.font }
        }
    }
    ColumnLayout {
        visible: w.configuring
        anchors { fill: parent; margins: 14; topMargin: 28 }
        spacing: 8
        Text { text: "Note colour"; color: w.ink; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: Object.keys(w.colors)
                delegate: Rectangle {
                    required property var modelData
                    width: 32; height: 32; radius: 16
                    color: w.colors[modelData]
                    border.width: w.tint === modelData ? 3 : 1
                    border.color: w.tint === modelData ? Theme.accent : Qt.rgba(0, 0, 0, 0.3)
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: w.set("color", modelData) }
                }
            }
        }
        Text { text: "Text size"; color: w.ink; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
        RowLayout {
            spacing: 6
            Repeater {
                model: [{ l: "Small", v: 14 }, { l: "Medium", v: 17 }, { l: "Large", v: 22 }]
                delegate: WBtn { required property var modelData; text: modelData.l; on: w.textSize === modelData.v; onClicked: w.set("size", modelData.v) }
            }
        }
        Item { Layout.fillHeight: true }
    }
}
