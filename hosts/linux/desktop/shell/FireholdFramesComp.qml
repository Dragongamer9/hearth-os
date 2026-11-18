// Firehold's Desk on Firehold's own compositor (HEARTH_WM=firehold): the title bars in Firehold's look (mockup
// docs/mockups/firehold/2-desk.png) are drawn by the compositor itself (firehold-comp frames.rs), so they're always in
// the right order and drag, click and resize like any frame. This only tells it the look and what to call each window,
// the way FireholdFrames.qml writes them over KWin's bars: "App — place" with three round buttons, or a game's name,
// its line ("running · 60 fps limit · VRR") and the chip "Full screen: F11 or ≡" with the accent border.
// Sent again whenever a name changes or the compositor's window list connects again; back to Hearth's plain frames
// when this goes away (the Couch, Firehold locked) and, by the compositor itself, whenever Mantel's connection ends.
import QtQuick
import Quickshell
import Quickshell.Io
import "firehold.js" as F

Scope {
    id: fc
    property var fh

    // VRR on the game's screen (the compositor's own answer, from the Guide's helper's status).
    property bool vrr: false
    Process {
        id: gameProbe
        command: ["python3", Quickshell.shellDir + "/hearth-firehold-game", "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const o = JSON.parse(text.trim().split("\n").pop());
                    fc.vrr = !!(o && o.game && o.game.vrrActive);
                } catch (e) {}
            }
        }
    }
    Timer { interval: 5000; repeat: true; running: true; triggeredOnStart: true; onTriggered: if (!gameProbe.running) gameProbe.running = true }

    readonly property var names: {
        const out = {};
        for (const w of fc.fh.wins) {
            const id = WinList.widOf(w);
            if (!id) continue;
            if (F.isGame(w, fc.fh.library)) {
                const g = F.gameFor(w, fc.fh.library) || { title: w.title || "Game" };
                out[id] = { title: g.title, game: true, line: F.gameLine("", w.minimized, fc.fh.frameLimit, fc.vrr) };
            } else {
                const n = F.windowName(w, fc.fh.entryName(w));
                out[id] = { title: n.name + (n.sub ? " — " + n.sub : ""), game: false, line: "" };
            }
        }
        return out;
    }
    readonly property var message: ({ op: "frames", style: "firehold", names: fc.names,
                                      colours: { panel2: F.C.panel2, text: F.C.text, muted: F.C.muted, line: F.C.line, card: F.C.card,
                                                 card2: F.C.card2, accent: F.C.accent, flame: F.C.flame } })
    property string sent: ""
    function push() {
        const m = JSON.stringify(message);
        if (m === sent || !WinList.fhReady) return;
        if (WinList.send(message)) sent = m;
    }
    onMessageChanged: pushSoon.restart()
    Timer { id: pushSoon; interval: 60; onTriggered: fc.push() }
    Connections {
        target: WinList
        function onFhReadyChanged() { fc.sent = ""; if (WinList.fhReady) pushSoon.restart(); }
    }
    Component.onCompleted: push()
    Component.onDestruction: WinList.send({ op: "frames", style: "hearth" })
}
