// Firehold's "Switched to Desk" (mockup docs/mockups/firehold Desk): the mouse or the keyboard moved you to the Desk;
// everything stayed open. Top right of the main screen, for 6 seconds; a click closes it.
import QtQuick
import Quickshell
import Quickshell.Wayland
import "firehold.js" as F

PanelWindow {
    id: nt
    property var fh
    readonly property real k: Math.min((screen ? screen.width : 1920) / 1920, (screen ? screen.height : 1080) / 1080) || 1
    screen: Quickshell.screens[0]
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-firehold-notice"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; right: true }
    margins { top: 64 * k; right: 30 * k }
    implicitWidth: 460 * k
    implicitHeight: card.height
    color: "transparent"

    Rectangle {
        id: card
        width: parent.width; height: col.implicitHeight + 32 * nt.k; radius: 14 * nt.k
        color: F.C.card; border.width: 1; border.color: F.C.line
        Rectangle { x: 18 * nt.k; anchors.verticalCenter: parent.verticalCenter; width: 40 * nt.k; height: width; radius: 10 * nt.k; color: F.C.blue }
        Column {
            id: col
            x: 72 * nt.k; width: parent.width - 90 * nt.k; anchors.verticalCenter: parent.verticalCenter
            spacing: 3 * nt.k
            Text { text: nt.fh.noticeTitle; color: F.C.text; font.family: F.font(Theme.font); font.pixelSize: 15 * nt.k; font.weight: Font.Bold }
            Text { width: parent.width; text: nt.fh.noticeText; color: F.C.soft; font.family: F.font(Theme.font); font.pixelSize: 13 * nt.k; wrapMode: Text.WordWrap }
        }
        TapHandler { onTapped: nt.fh.noticeShown = false }
    }
}
