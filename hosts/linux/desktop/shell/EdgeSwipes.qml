// Touch gestures for handhelds and touchscreens: swipe in from an edge.
//   right edge → Quick Access     left edge → keyboard     bottom edge → apps
// Each edge is a strip a few pixels wide; only a press that then moves inward counts, so a mouse at the
// edge of the screen behaves as usual.
import QtQuick
import Quickshell
import Quickshell.Wayland

Scope {
    id: root
    required property ShellScreen screen
    signal swiped(string edge)

    component Strip: PanelWindow {
        id: strip
        property string edge
        screen: root.screen
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-edge"
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        implicitWidth: 4
        implicitHeight: 4
        MouseArea {
            anchors.fill: parent
            property point start
            property bool done
            onPressed: m => { start = Qt.point(m.x, m.y); done = false; }
            onPositionChanged: m => {
                if (done) return;
                const dx = m.x - start.x, dy = m.y - start.y;
                const inward = strip.edge === "left" ? dx : strip.edge === "right" ? -dx : -dy;
                if (inward > 36) { done = true; root.swiped(strip.edge); }
            }
        }
    }

    Strip {
        edge: "left"
        anchors { left: true; top: true; bottom: true }
        margins { top: 80; bottom: 80 }
    }
    Strip {
        edge: "right"
        anchors { right: true; top: true; bottom: true }
        margins { top: 80; bottom: 80 }
    }
    Strip {
        edge: "bottom"
        anchors { bottom: true; left: true; right: true }
        margins { left: 120; right: 120 }
    }
}
