// A choice in Settings › Boot & startup: a boot splash or a startup animation, with its picture (the preview's poster
// frame; it plays while the card is selected or under the mouse), its name and what it looks like. Activating it
// opens the preview (SetBootPreview.qml); it never chooses by itself.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: card
    property var item: ({})             // {id, label, about, preview: {sheet, frames, poster}, pair}
    property var geo: null              // the sheets' frame size, columns, fps (hearth-boot-look status: preview)
    property bool inUse: false
    property string pairLabel: ""       // "Matches the Aurora startup animation"
    readonly property bool hasPicture: !!geo && !!item.preview && !!item.preview.sheet
    Layout.fillWidth: true
    implicitHeight: Math.max(96, info.implicitHeight + 24)
    Accessible.role: Accessible.Button
    Accessible.name: (item.label || "") + (inUse ? ", in use" : "")
    Accessible.description: (item.about || "") + ". Opens a preview; nothing changes until you choose Use this."
    Accessible.onPressAction: activated()

    HoverHandler { id: hv }

    RowLayout {
        anchors { fill: parent; leftMargin: 12; rightMargin: 16; topMargin: 12; bottomMargin: 12 }
        spacing: 14
        Rectangle {
            Layout.preferredWidth: 128; Layout.preferredHeight: 72
            color: "#0b1016"
            border.width: 1; border.color: Theme.line
            clip: true
            SetBootFrames {
                anchors { fill: parent; margins: 1 }
                visible: card.hasPicture
                sheet: card.hasPicture ? card.item.preview.sheet : ""
                frames: card.hasPicture ? card.item.preview.frames : 1
                poster: card.hasPicture ? card.item.preview.poster : 0
                columns: card.geo ? card.geo.columns : 8
                frameWidth: card.geo ? card.geo.frameWidth : 400
                frameHeight: card.geo ? card.geo.frameHeight : 225
                fps: card.geo ? card.geo.fps : 10
                playing: card.activeFocus || hv.hovered
            }
            Icon { visible: !card.hasPicture; anchors.centerIn: parent; name: "display"; size: 22; color: Theme.muted }
        }
        ColumnLayout {
            id: info
            Layout.fillWidth: true
            spacing: 3
            Text { Layout.fillWidth: true; text: card.item.label || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; text: card.item.about || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
            Text { Layout.fillWidth: true; visible: card.pairLabel !== ""; text: card.pairLabel; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.italic: true; wrapMode: Text.WordWrap }
        }
        Text { visible: card.inUse; text: "In use"; color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
        Text { text: "Preview  ›"; color: card.activeFocus ? Theme.text : Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
    }
}
