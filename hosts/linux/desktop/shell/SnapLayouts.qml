// Snap layouts (Hearth 1.10), like Windows 11's: six layouts with their zones; pick a zone and the window you were in
// snaps there. Opens with Super+Z, a right-click on a window's Maximize button, or "Snap layouts…" in a title bar's
// right-click menu. (labwc draws the title bars itself and can't tell anyone the pointer is resting on Maximize, so
// hovering doesn't open it: right-click does.) The mouse, the keyboard (arrows, Enter, Esc) or the controller (D-pad,
// A, B) pick a zone.
//
// It uses Mantel's own snapping: the halves are Super+Left / Super+Right, the other zones labwc regions behind hidden
// Super+Ctrl+Shift+F1–F11 keys (rc.xml), typed with wtype once this flyout has closed and your window has the keyboard
// again. The zones are in snap-layouts.js. Loaded by shell.qml through a Loader: a problem here only loses this.
//   quickshell ipc -p <shell folder> call snap open|close|toggle|state
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "snap-layouts.js" as Snap

Scope {
    id: sn
    property bool shown: false
    property int current: 0             // the lit zone (an index into Snap.flat())
    property bool mouseLit: false       // lit by the pointer rather than the keys
    property var win: null              // the window you were in when it opened
    property var atScreen: null
    readonly property var all: Snap.flat()
    readonly property var offsets: { const o = []; let n = 0; for (const l of Snap.LAYOUTS) { o.push(n); n += l.zones.length; } return o; }
    readonly property var lit: all[current] || null
    ExtrasPal { id: pal }

    function screenOf(t) {
        try { if (t && t.screens && t.screens.length) return t.screens[0]; } catch (e) {}
        return Quickshell.screens.length ? Quickshell.screens[0] : null;
    }
    function open() {
        if (shown) return;
        win = ToplevelManager.activeToplevel;
        atScreen = screenOf(win);
        current = 0;
        mouseLit = false;
        shown = true;
    }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(); }
    onShownChanged: SettingsNav.snapShown = shown
    Component.onCompleted: SettingsNav.snap = sn

    function pick(i) {
        const f = all[i];
        if (!f) return;
        const q = Snap.LAYOUTS[f.layout].zones[f.zone];
        const had = win !== null;
        close();
        if (!had) { SettingsNav.toast("Click a window first, then press Super+Z"); return; }
        // A moment for the flyout to close and your window to have the keyboard again, then labwc's shortcut.
        typer.command = ["sh", "-c", "sleep 0.3; exec wtype \"$@\"", "sh"].concat(Snap.wtypeArgs(q));
        typer.running = true;
    }
    Process {
        id: typer
        onExited: code => { if (code === 127) SettingsNav.toast("Snap layouts need wtype. The window's title bar menu has Snap too"); }
    }

    IpcHandler {
        target: "snap"
        function open(): void { sn.open(); }
        function close(): void { sn.close(); }
        function toggle(): void { sn.toggle(); }
        function state(): string { return JSON.stringify({ shown: sn.shown, zone: sn.lit ? sn.lit.id : "", window: sn.win !== null }); }
    }

    PanelWindow {
        id: panel
        visible: sn.shown
        screen: sn.atScreen || (Quickshell.screens.length ? Quickshell.screens[0] : null)
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-snap"
        WlrLayershell.keyboardFocus: sn.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "transparent"
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        MouseArea { anchors.fill: parent; onClicked: sn.close() }   // a click anywhere else closes it

        Item {
            id: keys
            focus: true
            Keys.onPressed: e => {
                const dirs = { [Qt.Key_Left]: "left", [Qt.Key_Right]: "right", [Qt.Key_Up]: "up", [Qt.Key_Down]: "down" };
                if (dirs[e.key]) { sn.current = Snap.step(sn.current, dirs[e.key]); sn.mouseLit = false; }
                else if (e.key === Qt.Key_Tab) { sn.current = (sn.current + 1) % sn.all.length; sn.mouseLit = false; }
                else if (e.key === Qt.Key_Backtab) { sn.current = (sn.current + sn.all.length - 1) % sn.all.length; sn.mouseLit = false; }
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) sn.pick(sn.current);
                else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Z) sn.close();
                else return;
                e.accepted = true;
            }
        }

        Rectangle {
            id: card
            width: 380
            height: grid.height + foot.height + 12 * 2 + 10
            x: Math.round((parent.width - width) / 2)
            y: 14
            radius: Theme.r(12)
            color: pal.card
            border.color: pal.edge
            MouseArea { anchors.fill: parent; hoverEnabled: true }   // clicks on the card don't close it

            Grid {
                id: grid
                x: 12; y: 12
                columns: 3
                spacing: 10
                Repeater {
                    model: Snap.LAYOUTS
                    delegate: Rectangle {
                        id: tile
                        required property var modelData
                        required property int index
                        width: (card.width - 24 - 20) / 3
                        height: 70
                        radius: Theme.r(8)
                        color: pal.well
                        Repeater {
                            model: tile.modelData.zones
                            delegate: Rectangle {
                                id: zr
                                required property var modelData
                                required property int index
                                readonly property int flatIndex: sn.offsets[tile.index] + index
                                readonly property real iw: tile.width - 8
                                readonly property real ih: tile.height - 8
                                x: 4 + iw * modelData.x / 100 + 1.5
                                y: 4 + ih * modelData.y / 100 + 1.5
                                width: iw * modelData.w / 100 - 3
                                height: ih * modelData.h / 100 - 3
                                radius: Theme.r(4)
                                color: sn.current === flatIndex ? pal.accent : zm.containsMouse ? Qt.lighter(pal.zone, 1.25) : pal.zone
                                Behavior on color { ColorAnimation { duration: 90 } }
                                MouseArea {
                                    id: zm
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onEntered: { sn.current = zr.flatIndex; sn.mouseLit = true; }
                                    onClicked: sn.pick(zr.flatIndex)
                                }
                            }
                        }
                    }
                }
            }
            Text {
                id: foot
                anchors { left: parent.left; right: parent.right; top: grid.bottom; topMargin: 10; leftMargin: 12; rightMargin: 12 }
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: sn.win === null ? "Click a window first, then press Super+Z"
                    : (sn.lit ? sn.lit.label : "") + " · or Super+Z with the controller’s D-pad"
                color: pal.muted; font.family: pal.font; font.pixelSize: 12
            }
        }
    }
}
