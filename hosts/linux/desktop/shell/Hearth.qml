pragma Singleton

// What Firehold's shared Quick Settings and notifications (plasma-hearth/package FireholdQuick, FireholdCards,
// FireholdHistory: mockups firehold-plasma 07-10) ask of "Hearth" (on Plasma: plasma-hearth/shim/Hearth.qml), given
// by Mantel on Firehold on labwc (1.16): a program and its answer (call), one detached (run), the environment, the
// backend hearth-quick, the screen they open on and where the Desk bar is. Nothing runs by itself here.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: h
    // (absolute paths: Quickshell keeps a relative one from leaving the shell's folder)
    readonly property string desktop: Quickshell.shellDir.replace(/\/shell\/?$/, "")
    readonly property string quick: desktop + "/plasma-hearth/hearth-quick"
    readonly property string helper: desktop + "/firehold-plasma/hearth-firehold-plasma"
    readonly property string style: "firehold"
    readonly property string imageFile: ""      // (Mantel's notifications keep the picture's file: nothing else needed)
    // Set by Firehold.qml: the screen Quick Settings opens on ({name, width, height}) and the Desk bar (top, its height
    // when it's there, 0 on the Couch).
    property var screen: ({ name: "", width: 1920, height: 1080 })
    property int panelEdge: 0
    property int panelSize: 0
    property string stamp: ""
    function env(k) { return Quickshell.env(k) || ""; }
    function shellUrl(p) { return "file://" + Quickshell.shellDir + "/" + p; }

    Component {
        id: procComp
        Process {
            id: p
            property var cb: null
            stdout: StdioCollector { onStreamFinished: { const f = p.cb; p.cb = null; if (f) { try { f(text, 0); } catch (e) { console.warn("firehold: " + e); } } } }
            onExited: Qt.callLater(() => p.destroy())
        }
    }
    function call(argv, cb) {
        if (!argv || !argv.length) { if (cb) cb("", 1); return; }
        const p = procComp.createObject(h, { command: argv.map(a => String(a)), cb: cb || null });
        if (p) p.running = true;
    }
    function run(argv) { if (argv && argv.length) Quickshell.execDetached(argv.map(a => String(a))); }
}
