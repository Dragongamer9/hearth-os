// More panels (1.14; KDE Plasma's "add panel" and "clone panel", written for Hearth): extra bars on any edge of any
// screen, next to the main bar (Islands.qml) and the dock, each holding what you put in it (widget-tools.js
// PANEL_ITEMS: the app menu, your apps, a window list, the pager, a clock, the tray, volume / network / battery,
// notifications, the keyboard layout, Game Mode, and the bar widgets). Made and changed in Arrange the bar
// (BarArrange.qml) and kept in bar.json (BarLayout.qml, one key per panel). A panel on a screen that isn't plugged in
// waits in the file until it's back. Each one reserves its own room like the main bar, on KWin and on labwc.
// Drag and drop board 2: each block in its look (BlockLook.qml), and each panel's own settings (widget-tools.js
// panelOf): any length at any offset along its edge (so more than one shares an edge), corners, fill, see-through,
// blur, the gap between blocks, how they line up, islands or one strip, hide until pointed at, over windows or keeping
// their space, under or above them; a panel inside a bar or a panel is drawn there (NestedPanel.qml).
// What a panel can hold is in PanelBlocks.qml.
// Loaded by shell.qml by file name: a problem here only loses the extra panels, never the main bar.
//   quickshell ipc -p <shell folder> call panels list|count|add <screen> <edge>|clone <screen> <edge>|remove <id>|arrange <tab>
//                                              |move <item> <dir> <zone>|layout|reset|clock <key> <json>
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower
import "widget-tools.js" as T
import "block-look.js" as BL

Scope {
    id: root
    readonly property string mainScreen: Quickshell.screens.length ? Quickshell.screens[0].name : ""
    function ipc(target, fn) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", target, fn]); }
    function screenFor(n) { return Quickshell.screens.find(s => s.name === n) || null; }
    function add(screen, edge, items) {
        const sn = screenFor(screen) ? screen : mainScreen;
        const clash = T.panelClash(BarLayout.panels, sn, edge, mainScreen, DeskState.barEdge, Theme.dockFloats);
        if (clash) { SettingsNav.toast(clash + ": pick another edge"); return ""; }
        const id = BarLayout.addPanel(sn, edge, items || ["menu", "windows", "spacer", "clock", "spacer", "status", "notifications"]);
        SettingsNav.toast("A new panel is on the " + edge + (Quickshell.screens.length > 1 ? " of " + sn : ""));
        return id;
    }

    Variants {
        // a panel inside a bar or a panel (inside: …) is drawn there (NestedPanel.qml), not on its own
        model: BarLayout.panels.filter(p => !p.inside && root.screenFor(p.screen || root.mainScreen) !== null)
        delegate: Component {
            PanelWindow {
                id: pw
                required property var modelData
                readonly property var p: modelData
                readonly property bool vertical: p.free ? p.orient === "down" : (p.edge === "left" || p.edge === "right")
                readonly property int size: p.size
                // desktop layouts' panel styles: "" full width, "floating" (off the edge, rounded), "centred" (only as
                // long as what it holds, in the middle of its edge); and (drag and drop board 2) any length along its
                // edge, at any offset from the corner, so more than one can share an edge
                readonly property bool floats: p.free || p.style === "floating" || p.style === "centred" || p.len !== "full"
                readonly property int gap: p.free ? 0 : floats ? 8 : 0
                readonly property real alongScreen: screen ? (vertical ? screen.height : screen.width) : 1920
                // its length: a free one as asked (or as what it holds); on an edge full, fit, a share or px
                readonly property real bodyLen: p.free ? (p.length > 0 ? p.length : Math.max(60, body.want))
                    : T.lengthOf(p.len, p.lenV, alongScreen - gap * 2, body.want)
                // Hide until pointed at: a thin handle at its edge brings it back
                property bool peek: false
                readonly property bool hiddenNow: p.autoHide && !peek && !hov.hovered
                screen: root.screenFor(p.screen || root.mainScreen)
                anchors { top: p.free || p.edge !== "bottom"; bottom: !p.free && p.edge !== "top"; left: p.free || p.edge !== "right"; right: !p.free && p.edge !== "left" }
                margins.left: p.free && screen ? Math.round(p.fx * Math.max(0, screen.width - implicitWidth)) : 0
                margins.top: p.free && screen ? Math.round(p.fy * Math.max(0, screen.height - implicitHeight)) : 0
                implicitHeight: p.free ? (vertical ? bodyLen : size) : vertical ? 0 : size + gap * 2
                implicitWidth: p.free ? (vertical ? size : bodyLen) : vertical ? size + gap * 2 : 0
                // above windows (over them) or keeping their space; hidden until pointed at keeps none
                exclusiveZone: p.free || p.over || p.autoHide ? 0 : size + gap * 2
                // only the panel itself takes clicks (a centred one's empty sides don't)
                mask: Region { item: pw.hiddenNow ? handle : body }
                color: "transparent"
                WlrLayershell.namespace: "hearth-panel"
                WlrLayershell.layer: p.layer === "bottom" ? WlrLayer.Bottom : WlrLayer.Top
                WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

                PanelBlocks { id: pb }

                Item {
                    id: handle
                    width: pw.vertical ? 6 : Math.max(60, Math.min(220, body.width))
                    height: pw.vertical ? Math.max(60, Math.min(220, body.height)) : 6
                    x: pw.vertical ? (pw.p.edge === "right" ? pw.width - width : 0) : body.x + (body.width - width) / 2
                    y: pw.vertical ? body.y + (body.height - height) / 2 : (pw.p.edge === "bottom" ? pw.height - height : 0)
                    Rectangle { anchors.centerIn: parent; width: pw.vertical ? 3 : 60; height: pw.vertical ? 60 : 3; radius: 2; color: Theme.muted; opacity: pw.hiddenNow ? 0.6 : 0 }
                    HoverHandler { onHoveredChanged: if (hovered) { pw.peek = true; peekOff.restart(); } }
                }
                Timer { id: peekOff; interval: 900; onTriggered: { if (!hov.hovered) pw.peek = false; else peekOff.restart(); } }

                Rectangle {
                    id: body
                    readonly property real along: pw.vertical ? pw.height : pw.width
                    readonly property real want: (pw.vertical ? row.implicitHeight : row.implicitWidth) + 24
                    readonly property real len: pw.p.free ? along : Math.max(40, Math.min(along - pw.gap * 2, pw.bodyLen))
                    readonly property real start: pw.p.free ? 0 : pw.p.offset >= 0 ? Math.max(pw.gap, Math.min(along - len - pw.gap, pw.p.offset + pw.gap)) : (along - len) / 2
                    x: pw.vertical ? pw.gap : start
                    y: pw.vertical ? start : pw.gap
                    width: pw.vertical ? pw.size : len
                    height: pw.vertical ? len : pw.size
                    opacity: pw.hiddenNow ? 0 : 1
                    Behavior on opacity { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.quick } }
                    radius: pw.p.corners >= 99 ? Math.min(width, height) / 2 : pw.p.corners >= 0 ? pw.p.corners : pw.floats ? Theme.r(Math.min(14, pw.size / 2)) : 0
                    border.width: pw.floats && !pw.p.islands ? 1 : 0
                    border.color: Theme.line
                    // its fill and see-through (islands: each block has its own, the panel none)
                    color: pw.p.islands ? "transparent"
                        : pw.p.fill !== "" ? BL.argb(pw.p.fill, 1 - pw.p.see / 100)
                        : pw.p.see > 0 ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 1 - pw.p.see / 100) : Theme.glass
                    HoverHandler { id: hov }
                    // blur behind it: the wallpaper, blurred (Frost.qml), under its fill
                    Loader { z: -1; anchors.fill: parent; active: pw.p.blur && !pw.p.islands; source: "Frost.qml"; onLoaded: item.rad = Qt.binding(() => body.radius) }
                    Rectangle {   // the line toward the windows
                        visible: !pw.floats && !pw.p.islands
                        color: Theme.line
                        width: pw.vertical ? 1 : parent.width
                        height: pw.vertical ? parent.height : 1
                        x: pw.p.edge === "left" ? parent.width - 1 : 0
                        y: pw.p.edge === "top" ? parent.height - 1 : 0
                    }
                    Loader { anchors.fill: parent; active: Theme.frame === "frost" && !pw.p.islands && !pw.p.blur && pw.p.fill === ""; source: "Frame.qml" }
                }
                MouseArea {
                    anchors.fill: body
                    acceptedButtons: Qt.RightButton
                    onClicked: { BarLayout.arrangeTab = "panels"; BarLayout.arranging = true; }
                }
                GridLayout {
                    id: row
                    anchors { fill: body; leftMargin: pw.vertical ? 4 : 8; rightMargin: pw.vertical ? 4 : 8; topMargin: pw.vertical ? 8 : 4; bottomMargin: pw.vertical ? 8 : 4 }
                    opacity: body.opacity
                    flow: pw.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
                    rows: pw.vertical ? -1 : 1
                    columns: pw.vertical ? 1 : -1
                    rowSpacing: pw.p.gap >= 0 ? pw.p.gap : 6
                    columnSpacing: pw.p.gap >= 0 ? pw.p.gap : 6
                    // its blocks, lined up (start, centre, end or spread: "~" is a stretch), each in its look
                    Repeater {
                        model: T.lineup(pw.p.items, pw.p.lineup)
                        delegate: BlockLook {
                            id: slot
                            required property string modelData
                            readonly property bool stretch: modelData === "~" || T.baseId(modelData) === "spacer" || wantsFill
                            bid: modelData === "~" ? "" : modelData
                            vert: pw.vertical
                            side: pw.size
                            island: pw.p.islands && !stretch
                            screenName: pw.screen ? pw.screen.name : ""
                            Layout.alignment: Qt.AlignCenter
                            Layout.fillWidth: stretch && !pw.vertical
                            Layout.fillHeight: stretch && pw.vertical
                            visible: modelData === "~" || (shownNow && ld.status !== Loader.Error && (!ld.item || ld.item.wanted !== false))
                            Loader {
                                id: ld
                                active: slot.modelData !== "~"
                                sourceComponent: pb.compFor(slot.modelData)
                                onLoaded: { if (item.panel !== undefined) item.panel = pw; if (item.kind !== undefined) item.kind = T.baseId(slot.modelData); }
                                onStatusChanged: if (status === Loader.Error) console.warn("panels: " + slot.modelData + " couldn't load")
                            }
                        }
                    }
                }
            }
        }
    }

    IpcHandler {
        target: "panels"
        function list(): string { return JSON.stringify(BarLayout.panels); }
        function count(): int { return BarLayout.panels.length; }
        function add(screen: string, edge: string): string { return root.add(screen, edge, null); }
        function clone(screen: string, edge: string): string { return root.add(screen, edge, T.cloneMain(BarLayout.raw.items)); }
        function remove(id: string): void { BarLayout.removePanel(id); }
        function arrange(tab: string): void { BarLayout.arrangeTab = tab || "items"; BarLayout.arranging = true; }
        function done(): void { BarLayout.arranging = false; }
        // The bar's own items (as Arrange the bar's buttons): move <id> <-1|1|0> [start|end|hidden]
        function move(id: string, dir: int, zone: string): string { BarLayout.move(id, dir, zone || ""); return JSON.stringify(BarLayout.layout); }
        function layout(): string { return JSON.stringify(BarLayout.layout); }
        function reset(): void { BarLayout.resetItems(); }
        // The clock's panel (as Settings › Date & time): clock clockZones|clockAlt|clockHolidays <json>
        function clock(key: string, json: string): void { try { BarLayout.setClock(key, JSON.parse(json)); } catch (e) { console.warn("panels: clock " + key + ": " + e); } }
    }
}
