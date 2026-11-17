// Firehold's Desk: a game in a window (mockup docs/mockups/firehold Desk). While the game is the window in front and
// isn't full screen, its frame gets the accent border and its own title line: the game's name, whether it's running
// or paused (minimized), VRR when its screen has it on, and the chip "Full screen: F11 or ≡" (a click, or any
// controller button in Automatic, which goes to the Couch where games are full screen). Only on KWin, which tells
// Mantel where the window is (WinList's frame); it never takes a click but the chip's.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "firehold.js" as F

PanelWindow {
    id: gf
    property var fh
    property var win: null
    readonly property var g: win ? (F.gameFor(win, fh.library) || { title: win.title || "Game" }) : { title: "" }
    readonly property var fr: win && win.frame ? win.frame : ({})
    readonly property var scr: win && win.screens && win.screens.length ? win.screens[0] : Quickshell.screens[0]
    screen: scr
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-firehold-game-frame"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; left: true }
    margins { left: (fr.x || 0) - (scr ? scr.x : 0) - 2; top: (fr.y || 0) - (scr ? scr.y : 0) - 2 }
    implicitWidth: (fr.w || 100) + 4
    implicitHeight: (fr.h || 100) + 4
    color: "transparent"
    mask: Region { item: chip }

    // VRR on the game's screen (hearth-screens info, read once when it shows).
    property bool vrr: false
    Process {
        running: true
        command: ["python3", Quickshell.shellDir + "/hearth-screens", "info"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const o = JSON.parse(text.trim().split("\n").pop());
                    const out = (o.outputs || []).find(x => gf.scr && x.name === gf.scr.name);
                    gf.vrr = !!(out && out.vrr && out.vrr.policy && out.vrr.policy !== "never");
                } catch (e) {}
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: "transparent"; radius: 14; border.width: 2; border.color: F.C.accent
    }
    Rectangle {
        id: strip
        x: 2; y: 2; width: parent.width - 4; height: 40
        color: F.C.panel2; radius: 12
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 12; color: F.C.panel2 }
        Row {
            x: 14; anchors.verticalCenter: parent.verticalCenter
            spacing: 0
            Text { text: gf.g.title; color: F.C.text; font.family: F.font(Theme.font); font.pixelSize: 14; font.weight: Font.DemiBold }
            Text {
                color: F.C.muted; font.family: F.font(Theme.font); font.pixelSize: 14; font.weight: Font.Medium
                text: " · " + (gf.win && gf.win.minimized ? "paused" : "running") + (gf.vrr ? " · VRR" : "")
            }
        }
        Rectangle {
            id: chip
            anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
            width: ct.implicitWidth + 20; height: ct.implicitHeight + 6; radius: 10; color: F.C.card
            Text { id: ct; anchors.centerIn: parent; text: "Full screen: F11 or ≡"; color: F.C.flame; font.family: F.font(Theme.font); font.pixelSize: 12; font.weight: Font.Bold }
            TapHandler { onTapped: if (gf.win) gf.win.fullscreen = true }
        }
    }
}
