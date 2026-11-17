// Firehold's Desk: every window's title bar in Firehold's look (mockup docs/mockups/firehold Desk): "App — place" and
// three round buttons on the right (minimize, maximize, close in the accent colour); a game in a window gets the accent
// border and its own line ("Ember Trails · running · 60 fps limit · VRR") with the chip "Full screen: F11 or ≡".
// Drawn over KWin's own title bars (WinList tells where each window is: its frame), only where no window is above it
// (Firehold keeps the order you went to them in, the order KWin stacks them), and it takes no click but its buttons':
// dragging a title bar is KWin's, as before. Only on KWin, in the Desk posture (Firehold.qml loads it by file name).
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "firehold.js" as F

PanelWindow {
    id: fr
    property var fh
    screen: Quickshell.screens[0]
    readonly property int titleH: 31          // KWin's title bar in Hearth's frames (hearth_aurorae TITLE_H + the border)
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-firehold-frames"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    mask: Region { id: hits }

    // The windows on this screen with a frame, top first.
    readonly property var wins: F.stackOrder(fh.wins.filter(w => !w.minimized && !w.fullscreen && w.frame && w.frame.w > 40 && !F.ownWindow(w)
                                                                  && (!w.screens || !w.screens.length || w.screens[0] === screen)),
                                             fh.recent, w => WinList.widOf(w))
    // VRR on this screen (hearth-screens info, read when this comes up).
    property bool vrr: false
    Process {
        running: true
        command: ["python3", Quickshell.shellDir + "/hearth-screens", "info"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const o = JSON.parse(text.trim().split("\n").pop());
                    const out = (o.outputs || []).find(x => fr.screen && x.name === fr.screen.name);
                    fr.vrr = !!(out && out.vrr && out.vrr.policy && out.vrr.policy !== "never");
                } catch (e) {}
            }
        }
    }
    function local(f) { return { x: f.x - (screen ? screen.x : 0), y: f.y - (screen ? screen.y : 0), w: f.w, h: f.h }; }

    Repeater {
        model: fr.wins
        delegate: Item {
            id: one
            required property var modelData
            required property int index
            readonly property var win: modelData
            readonly property var g: fr.local(win.frame)
            readonly property bool game: F.isGame(win, fr.fh.library)
            readonly property var gameInfo: game ? (F.gameFor(win, fr.fh.library) || { title: win.title || "Game" }) : null
            readonly property var name: F.windowName(win, fr.fh.entryName(win))
            // What of this window's bar (and a game's border) shows: minus every window above it.
            readonly property var above: fr.wins.slice(0, index).map(w => fr.local(w.frame))
            readonly property var parts: F.visibleParts(game
                    ? [{ x: g.x - 2, y: g.y - 2, w: g.w + 4, h: fr.titleH + 2 }, { x: g.x - 2, y: g.y + fr.titleH, w: 2, h: g.h - fr.titleH + 2 },
                       { x: g.x + g.w, y: g.y + fr.titleH, w: 2, h: g.h - fr.titleH + 2 }, { x: g.x - 2, y: g.y + g.h, w: g.w + 4, h: 2 }]
                    : [{ x: g.x, y: g.y, w: g.w, h: fr.titleH }], above)
            Repeater {
                model: one.parts
                delegate: Item {
                    required property var modelData
                    x: modelData.x; y: modelData.y; width: modelData.w; height: modelData.h
                    clip: true
                    // The whole bar and border, seen through this visible part.
                    Item {
                        x: one.g.x - parent.x; y: one.g.y - parent.y; width: one.g.w; height: one.g.h
                        Rectangle {
                            visible: one.game
                            x: -2; y: -2; width: parent.width + 4; height: parent.height + 4; radius: 14
                            color: "transparent"; border.width: 2; border.color: F.C.accent
                        }
                        Rectangle {
                            width: parent.width; height: fr.titleH; radius: 12
                            color: F.C.panel2
                            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 12; color: F.C.panel2 }
                            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: F.C.line; visible: !one.game }
                            Row {
                                x: 16; anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - 32 - (one.game ? chip.width + 12 : 74)
                                clip: true
                                Text {
                                    text: one.game ? one.gameInfo.title : one.name.name + (one.name.sub ? " — " + one.name.sub : "")
                                    color: F.C.text; font.family: F.font(Theme.font); font.pixelSize: 14; font.weight: Font.DemiBold
                                }
                                Text {
                                    visible: one.game
                                    text: one.game ? F.gameLine("", one.win.minimized, fr.fh.frameLimit, fr.vrr) : ""
                                    color: F.C.muted; font.family: F.font(Theme.font); font.pixelSize: 14; font.weight: Font.Medium
                                }
                            }
                            // A game: the chip. Anything else: minimize, maximize, close (accent).
                            Rectangle {
                                id: chip
                                visible: one.game
                                anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                                width: ct.implicitWidth + 20; height: ct.implicitHeight + 6; radius: 10; color: F.C.card
                                Text { id: ct; anchors.centerIn: parent; text: "Full screen: F11 or ≡"; color: F.C.flame; font.family: F.font(Theme.font); font.pixelSize: 12; font.weight: Font.Bold }
                                TapHandler { onTapped: one.win.fullscreen = true }
                            }
                            Row {
                                visible: !one.game
                                anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter }
                                spacing: 10
                                Repeater {
                                    model: ["min", "max", "close"]
                                    delegate: Rectangle {
                                        required property string modelData
                                        width: 14; height: 14; radius: 7
                                        color: modelData === "close" ? F.C.accent : F.C.card2
                                        TapHandler {
                                            onTapped: {
                                                if (parent.modelData === "min") one.win.minimized = true;
                                                else if (parent.modelData === "max") one.win.maximized = !one.win.maximized;
                                                else one.win.close();
                                            }
                                        }
                                        Component.onCompleted: if (one.index === 0) fr.addHit(this)
                                        Component.onDestruction: fr.dropHit(this)
                                    }
                                }
                            }
                            Component.onCompleted: if (one.index === 0 && one.game) fr.addHit(chip)
                            Component.onDestruction: fr.dropHit(chip)
                        }
                    }
                }
            }
        }
    }
    // Only the window in front's buttons (or a game's chip) take clicks: the rest of every bar is KWin's (dragging,
    // double-click, its menu), and a window behind is brought to the front by KWin as before.
    property var hitItems: []
    function addHit(it) { hitItems = hitItems.concat([it]); }
    function dropHit(it) { hitItems = hitItems.filter(x => x !== it); }
    Component { id: hitRegion; Region {} }
    onHitItemsChanged: {
        const rs = [], olds = Array.from(hits.regions);
        for (const it of hitItems) if (it) rs.push(hitRegion.createObject(hits, { item: it }));
        hits.regions = rs;
        for (const old of olds) old.destroy();
    }
}
