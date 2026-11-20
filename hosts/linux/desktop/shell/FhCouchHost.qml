// Firehold on labwc's side of the shared Couch (1.16, docs/LABWC-FEDORA.md): what the Couch's pages
// (desktop/firehold-plasma/couch/contents/ui, reached here through fhcouch/) need from the desktop they run on, as
// PlasmaHost.qml gives them under Plasma: the helper (hearth-firehold-plasma, its labwc paths), the clock, the windows
// (labwc's list, WinList) and what each does, what's in front, the Guide and Home (Mantel's Firehold.qml decides those).
// Nothing runs by itself here: no timer; a helper only when the Couch asks for something.
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: hh
    property var fh: null                      // Mantel's Firehold.qml
    readonly property bool hosted: true
    property bool front: false                 // set by FireholdCouch.qml: its layer up, nothing over it
    readonly property bool resting: DeskState.idle
    readonly property string mediaFile: Qt.resolvedUrl("FhMedia.qml")
    readonly property string iconFile: Qt.resolvedUrl("FhIcon.qml")
    // (an absolute path: Quickshell keeps a relative one from leaving the shell's folder)
    readonly property string helper: Quickshell.shellDir.replace(/\/shell\/?$/, "") + "/firehold-plasma/hearth-firehold-plasma"

    // ---- the helper: one process per question, its answer to the Couch's callback
    // ("state" and "listen" are for Plasma's controller and requests; on labwc Mantel's own pad helper and IPC do that.)
    readonly property var notHere: ["state", "listen", "ready"]
    Component {
        id: procComp
        Process {
            id: p
            property var cb: null
            stdout: StdioCollector { onStreamFinished: { const f = p.cb; p.cb = null; if (f) f(text); } }
            onExited: Qt.callLater(() => p.destroy())
        }
    }
    function run(args, cb) {
        if (!args || !args.length || notHere.indexOf(args[0]) >= 0) return;
        const p = procComp.createObject(hh, { command: ["python3", helper].concat(args.map(a => String(a))), cb: cb || null });
        if (p) p.running = true;
    }

    // ---- the clock, once a minute
    SystemClock { id: clock; precision: SystemClock.Minutes }
    readonly property var now: clock.date

    // ---- the windows: labwc's (wlr foreign toplevels), as the Couch's pages read them
    // (each window keeps one id while it's open; the same list as before is handed back when nothing the Couch shows
    // changed, so its tiles aren't made again: the rest fixes)
    property var idOf: new Map()
    property var last: ({ key: "", list: [] })
    readonly property var wins: {
        const list = WinList.toplevels ? WinList.toplevels.values : [];
        const out = [];
        for (const t of list) {
            if (!t) continue;
            let id = WinList.widOf(t);
            if (!id) { id = hh.idOf.get(t); if (!id) { id = "lw" + (hh.idOf.size + 1); hh.idOf.set(t, id); } }
            const e = AppIcons.entryFor(t.appId);
            out.push({ id: String(id), uuid: String(id), appId: String(t.appId || ""), appName: e ? String(e.name) : String(t.appId || ""),
                       title: String(t.title || ""), activated: !!t.activated, minimized: !!t.minimized, fullscreen: !!t.fullscreen,
                       pid: 0, tl: t });
        }
        const key = JSON.stringify(out.map(w => [w.id, w.appId, w.appName, w.title, w.activated, w.minimized, w.fullscreen]));
        if (key === hh.last.key) return hh.last.list;
        hh.last.key = key; hh.last.list = out;
        return out;
    }
    // A window you go to: in front, the Couch steps aside (Mantel's view "app"), full screen as the Couch asks.
    function activate(w) {
        if (!w || !w.tl) return;
        if (fh) { fh.guideShown = false; fh.view = "app"; }
        if (w.tl.minimized) w.tl.minimized = false;
        w.tl.activate();
    }
    function toggleFull(w) { if (w && w.tl) w.tl.fullscreen = !w.tl.fullscreen; }
    function closeWin(w) { if (w && w.tl) w.tl.close(); }

    // ---- what Mantel's Firehold decides
    function guide(twice) { if (fh) fh.guide(twice); }
    function home() { if (fh) fh.home(); }
}
