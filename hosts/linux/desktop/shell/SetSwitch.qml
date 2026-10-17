// An on/off setting.
import QtQuick
import QtQuick.Layouts

SetRow {
    id: sw
    property bool checked: false
    signal toggled(bool on)
    chevron: false
    onActivated: toggled(!checked)

    Rectangle {
        anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter }
        width: 46; height: 26; radius: 13
        color: sw.checked ? Theme.accent : Theme.panel3
        Behavior on color { ColorAnimation { duration: Theme.quick } }
        Rectangle {
            width: 20; height: 20; radius: 10
            y: 3; x: sw.checked ? parent.width - width - 3 : 3
            color: sw.checked ? Theme.accentInk : Theme.text
            Behavior on x { NumberAnimation { duration: Theme.quick } }
        }
    }
}
