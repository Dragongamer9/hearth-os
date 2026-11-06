// An on/off setting: "On" / "Off" and the switch on the right of the card.
import QtQuick
import QtQuick.Layouts

SetRow {
    id: sw
    property bool checked: false
    property bool showState: chip === ""   // the "On" / "Off" word next to the switch (a status chip says it instead)
    signal toggled(bool on)
    chevron: false
    trailing: showState ? 88 : 54     // the switch's room: a long explanation wraps before it instead of under it
    onActivated: toggled(!checked)
    Accessible.role: Accessible.CheckBox   // for the screen reader
    Accessible.checkable: true
    Accessible.checked: checked

    Row {
        anchors { right: parent.right; rightMargin: SetStyle.pad; verticalCenter: parent.verticalCenter }
        spacing: 10
        Text { visible: sw.showState; anchors.verticalCenter: parent.verticalCenter; text: sw.checked ? "On" : "Off"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
        Rectangle {
            width: 44; height: 22; radius: 11
            color: sw.checked ? Theme.accent : "transparent"
            border.width: 1.5
            border.color: sw.checked ? Theme.accent : Theme.muted
            Behavior on color { ColorAnimation { duration: Theme.quick } }
            Rectangle {
                width: sw.checked ? 13 : 12; height: width; radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                x: sw.checked ? parent.width - width - 5 : 4.5
                color: sw.checked ? Theme.accentInk : Theme.muted
                Behavior on x { NumberAnimation { duration: Theme.quick } }
            }
        }
    }
}
