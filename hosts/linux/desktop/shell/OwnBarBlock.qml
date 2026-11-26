import QtQuick
import Quickshell

// A block you made in the desktop editor (DesktopEditor.qml › Make your own; docs/mockups/checklists/drag-drop.md):
// "own:<key>" in a bar or a panel, its look and what it does in bar.json "b:<key>" (BarLayout.qml):
//   { kind: "button" | "text" | "picture" | "spacer", label, icon, run, app, panel, pic }
// A button runs what you gave it (your own command, as you typed it), opens an app or shows a panel, only when you
// press it. Its look, and what its other buttons do (drag and drop board 2), come from bar.json "s:own:<key>" through
// BlockLook.qml, the wrapper Islands.qml and ExtraPanels.qml put around every block (yours included).
Item {
    id: ob
    property string kind: ""                 // "own:<key>" (set by the bar or the panel)
    property real side: 34
    property bool vert: false
    readonly property var def: { const k = kind.indexOf("own:") === 0 ? kind.slice(4) : ""; const d = k ? BarLayout.raw["b:" + k] : null; return d && typeof d === "object" ? d : { kind: "spacer" }; }
    readonly property bool wanted: true
    implicitHeight: side
    implicitWidth: def.kind === "spacer" ? 16 : def.kind === "picture" ? side : vert ? side : Math.max(side, row.implicitWidth + 16)
    Rectangle {
        anchors.fill: parent
        visible: ob.def.kind === "button"
        radius: Theme.r(8)
        color: ma.containsMouse ? Theme.panel3 : "transparent"
    }
    Row {
        id: row
        visible: ob.def.kind === "button" || ob.def.kind === "text"
        anchors.centerIn: parent
        spacing: 6
        Icon { visible: ob.def.kind === "button" && (ob.def.icon || "") !== ""; name: ob.def.icon || "dot"; size: 16; color: Theme.text; anchors.verticalCenter: parent.verticalCenter }
        Text { visible: !ob.vert && (ob.def.label || "") !== ""; text: ob.def.label || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
    }
    Image {
        visible: ob.def.kind === "picture" && (ob.def.pic || "") !== ""
        anchors.fill: parent; anchors.margins: 3
        source: ob.def.pic ? "file://" + ob.def.pic : ""
        fillMode: Image.PreserveAspectFit; asynchronous: true; smooth: true
    }
    Icon { visible: ob.def.kind === "picture" && (ob.def.pic || "") === ""; anchors.centerIn: parent; name: "image"; size: 16; color: Theme.muted }
    MouseArea {
        id: ma
        anchors.fill: parent
        enabled: ob.def.kind === "button" && ((ob.def.run || "") !== "" || (ob.def.app || "") !== "" || (ob.def.panel || "") !== "")
        hoverEnabled: true
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: {
            if ((ob.def.run || "") !== "") Quickshell.execDetached(["sh", "-c", ob.def.run]);
            else if ((ob.def.app || "") !== "") { const e = AppEntries.byId(ob.def.app); if (e) e.execute(); else Quickshell.execDetached(["gtk-launch", ob.def.app]); }
            else if (["launcher", "quick", "calendar", "overview", "clipboard"].indexOf(ob.def.panel) >= 0) Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", ob.def.panel, "toggle"]);
        }
    }
}
