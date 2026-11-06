// A status chip on a setting card: "Running" / "Good" (ok), "1 failed" (warn), "New" (new), "Off" (off), "Error" (bad).
import QtQuick

Rectangle {
    id: chip
    property string text
    property string kind: "ok"
    implicitWidth: label.implicitWidth + 20
    implicitHeight: 22
    radius: 11
    readonly property color fg: kind === "warn" ? SetStyle.warn : kind === "bad" ? Theme.danger : kind === "new" ? Theme.accentInk
                              : kind === "off" ? Theme.muted : SetStyle.ok
    color: kind === "new" ? Theme.accent : kind === "off" ? SetStyle.card2 : SetStyle.tint(fg, 0.15)
    Text {
        id: label
        anchors.centerIn: parent
        text: chip.text
        color: chip.fg
        font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
    }
}
