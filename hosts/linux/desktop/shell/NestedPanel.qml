// A panel inside a bar or a panel (drag and drop board 2, "panels inside panels"): "panel:<id>" among a bar's or a
// panel's blocks, its record bar.json "p:<id>" with inside: <where> (widget-tools.js panelOf). It draws its own box
// (its fill, see-through, blur, corners, gap, line-up, islands) and its blocks, each in its look (BlockLook.qml), with
// PanelBlocks.qml's blocks. Loaded by file name (PanelBlocks.qml, Islands.qml), so a problem here only loses the panel
// inside; panels inside panels go three deep at most.
import QtQuick
import QtQuick.Layouts
import Quickshell
import "widget-tools.js" as T
import "block-look.js" as BL

Item {
    id: np
    property string pid: ""
    property real hostSize: 34
    property bool hostVertical: false
    property string hostEdge: "bottom"
    property var hostScreen: null
    property int depth: 1
    // what its blocks read as their panel (PanelBlocks.qml): size, vertical, p.edge, screen, width, height, depth
    readonly property var p: { const r = T.panelOf(pid, BarLayout.raw["p:" + pid]); if (r) r.edge = hostEdge; return r; }
    readonly property bool vertical: hostVertical
    readonly property int size: Math.max(20, Math.round(Math.min(p ? p.size : hostSize, hostSize - 6)))
    readonly property var screen: hostScreen
    readonly property bool wanted: !!p && depth <= 3 && pid !== ""
    readonly property real want: (vertical ? row.implicitHeight : row.implicitWidth) + 12
    readonly property real len: !p ? 0 : p.len === "px" && p.lenV > 0 ? p.lenV : want
    implicitWidth: !wanted ? 0 : vertical ? size : len
    implicitHeight: !wanted ? 0 : vertical ? len : size

    PanelBlocks { id: pb }

    Rectangle {
        id: box
        anchors.fill: parent
        visible: np.wanted
        radius: !np.p ? 0 : np.p.corners >= 99 ? Math.min(width, height) / 2 : np.p.corners >= 0 ? np.p.corners : Theme.r(10)
        color: !np.p || np.p.islands ? "transparent" : np.p.fill !== "" ? BL.argb(np.p.fill, 1 - np.p.see / 100)
            : Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 1 - (np.p ? np.p.see : 0) / 100)
        border.width: np.p && !np.p.islands ? 1 : 0
        border.color: Theme.line
        Loader { z: -1; anchors.fill: parent; active: !!np.p && np.p.blur && !np.p.islands; source: "Frost.qml"; onLoaded: item.rad = Qt.binding(() => box.radius) }
    }
    GridLayout {
        id: row
        anchors { fill: parent; leftMargin: np.vertical ? 2 : 6; rightMargin: np.vertical ? 2 : 6; topMargin: np.vertical ? 6 : 2; bottomMargin: np.vertical ? 6 : 2 }
        visible: np.wanted
        flow: np.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rows: np.vertical ? -1 : 1
        columns: np.vertical ? 1 : -1
        rowSpacing: np.p && np.p.gap >= 0 ? np.p.gap : 4
        columnSpacing: np.p && np.p.gap >= 0 ? np.p.gap : 4
        Repeater {
            model: np.wanted ? T.lineup(np.p.items, np.p.lineup) : []
            delegate: BlockLook {
                id: slot
                required property string modelData
                readonly property bool stretch: modelData === "~" || T.baseId(modelData) === "spacer" || wantsFill
                bid: modelData === "~" ? "" : modelData
                vert: np.vertical
                side: np.size
                island: !!np.p && np.p.islands && !stretch
                screenName: np.screen ? np.screen.name : ""
                Layout.alignment: Qt.AlignCenter
                Layout.fillWidth: stretch && !np.vertical
                Layout.fillHeight: stretch && np.vertical
                visible: modelData === "~" || (shownNow && ld.status !== Loader.Error && (!ld.item || ld.item.wanted !== false))
                Loader {
                    id: ld
                    active: slot.modelData !== "~"
                    sourceComponent: pb.compFor(slot.modelData)
                    onLoaded: { if (item.panel !== undefined) item.panel = np; if (item.kind !== undefined) item.kind = T.baseId(slot.modelData); }
                    onStatusChanged: if (status === Loader.Error) console.warn("panels: " + slot.modelData + " couldn't load in a panel inside")
                }
            }
        }
    }
}
