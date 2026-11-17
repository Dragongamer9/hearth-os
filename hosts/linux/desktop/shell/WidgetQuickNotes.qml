// Quick notes widget: a short list you can tick off (shopping, today's jobs). Type and press Enter to add; click the
// box to tick; × takes one away; "Clear ticked" takes away the ones done. Kept in widgets.json for this widget only.
import QtQuick
import QtQuick.Layouts

WidgetBase {
    id: w
    readonly property var items: Array.isArray(c("items", null)) ? c("items", []).filter(i => i && typeof i.t === "string") : []
    readonly property string title: c("title", "Quick notes")
    implicitWidth: 310; implicitHeight: 330
    function setItems(l) { set("items", l); }

    ColumnLayout {
        anchors { fill: parent; margins: 14 }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: w.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { text: w.items.filter(i => !i.done).length + " left"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        }
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 2
            model: w.items
            delegate: RowLayout {
                required property var modelData
                required property int index
                width: ListView.view.width
                spacing: 8
                Rectangle {
                    width: 20; height: 20; radius: 5
                    color: modelData.done ? Theme.accent : "transparent"
                    border.width: 2; border.color: modelData.done ? Theme.accent : Theme.muted
                    Icon { anchors.centerIn: parent; visible: !!modelData.done; name: "check"; size: 14; color: Theme.accentInk }
                    MouseArea { anchors.fill: parent; anchors.margins: -6; cursorShape: Qt.PointingHandCursor
                        onClicked: { const l = w.items.slice(); l[index] = { t: modelData.t, done: !modelData.done }; w.setItems(l); } }
                }
                Text {
                    Layout.fillWidth: true
                    text: modelData.t
                    color: modelData.done ? Theme.muted : Theme.text
                    font.family: Theme.font; font.pixelSize: 15; font.strikeout: !!modelData.done
                    wrapMode: Text.WordWrap
                }
                WBtn { icon: "close"; flat: true; size: 22; onClicked: { const l = w.items.slice(); l.splice(index, 1); w.setItems(l); } }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            WField {
                id: add
                Layout.fillWidth: true
                placeholder: "Add a note, then Enter"
                onAccepted: if (text.trim()) { w.setItems(w.items.concat([{ t: text.trim(), done: false }])); text = ""; }
            }
            WBtn { visible: w.items.some(i => i.done); text: "Clear ticked"; onClicked: w.setItems(w.items.filter(i => !i.done)) }
        }
    }
}
