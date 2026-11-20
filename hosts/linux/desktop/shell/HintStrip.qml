// Mantel 2026: the controller's button hints along the bottom of the main screen, for a while after the controller
// was used for Hearth's menus (Pad.qml sets DeskState.padSeen). Console shows them all the time inside its dock
// (Dock.qml), so this stays out of its way there. What each button does is hearth-pad's desktop mapping.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: strip
    readonly property bool always: false
    // 12 s after the controller was last used (1.16: one timer started by the press, instead of one ticking every
    // second all day).
    property bool recent: false
    Connections { target: DeskState; function onPadSeenChanged() { strip.recent = DeskState.padSeen > 0; if (strip.recent) hideAfter.restart(); } }
    Timer { id: hideAfter; interval: 12000; onTriggered: strip.recent = false }
    // Not while a menu or dialog is open (DeskState.overlayShown): it has its own A / B row, and the strip showed
    // through behind it.
    visible: recent && !Theme.hints && !DeskState.overlayShown
    anchors { bottom: true; left: true; right: true }
    implicitHeight: 48
    implicitWidth: 0
    exclusiveZone: always ? 48 : 0
    color: "transparent"
    WlrLayershell.namespace: "hearth-hints"
    WlrLayershell.layer: always ? WlrLayer.Top : WlrLayer.Overlay
    mask: Region { item: strip.always ? bg : null }

    Rectangle {
        id: bg
        anchors.fill: parent
        color: strip.always ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.94) : "transparent"
        Rectangle { visible: strip.always; anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
        Rectangle {   // not Console: a floating pill above the dock
            visible: !strip.always
            anchors.centerIn: parent
            width: hints.implicitWidth + 32; height: 38; radius: Theme.r(19)
            color: Theme.glass; border.color: Theme.line
        }
        RowLayout {
            id: hints
            anchors.centerIn: parent
            spacing: 22
            Repeater {
                model: [{ b: "A", c: "#3fae5a", t: "Open" }, { b: "X", c: "#3b7de0", t: "Options" }, { b: "Y", c: "#d9a521", t: "Keyboard" },
                        { b: "☰", c: "#6c7480", t: "Apps" }, { b: "⌂", c: Theme.accent, t: "Hearth menu" }, { b: "⧉", c: "#6c7480", t: "Controls" }]
                delegate: RowLayout {
                    required property var modelData
                    spacing: 8
                    Rectangle {
                        implicitWidth: 24; implicitHeight: 24; radius: 12
                        color: modelData.c
                        Text { anchors.centerIn: parent; text: modelData.b; color: "white"; font.pixelSize: 12; font.bold: true; font.italic: modelData.b.length === 1 && /[AXY]/.test(modelData.b) }
                    }
                    Text { text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 }
                }
            }
        }
    }
}
