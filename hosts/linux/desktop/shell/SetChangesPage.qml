// Settings › Home › All changes (1.14): every change in the history, newest first, each with Undo.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "All changes"
    SetChanges { id: changes; count: 200; running: page.visible }
    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 12
        text: changes.items.length ? "What was changed in Settings, newest first. Undo puts the old value back."
            : "Nothing changed yet. What you change in Settings shows here, with Undo."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    Repeater {
        model: changes.items
        delegate: SetChangeRow { required property var modelData; change: modelData; onUndone: changes.load() }
    }
}
