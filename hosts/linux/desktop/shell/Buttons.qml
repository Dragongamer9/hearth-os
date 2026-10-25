pragma Singleton

// The power and volume buttons in Desktop Mode, and their settings (~/.config/nimbus/buttons.json, the same
// file Hearth's Game Mode settings use). hearth-buttons reads the buttons from the kernel; shell.qml runs it
// and calls volume()/mute()/power() here. labwc's key bindings call the same through ipc, as a backup for
// keyboards hearth-buttons can't read; presses that both see are counted once.
//
// The power button has exactly one handler: hearth-buttons whenever it watches a device with the power button
// (labwc's copy of the press is then dropped), labwc's binding only when it doesn't. The press that wakes the device
// reaches labwc too: hearth-buttons ignores it (and says so), and labwc's copy used to go on after 250 ms and put
// the device straight back to sleep (ROG Xbox Ally, Desktop Mode). Sleep goes through `hearth-buttons sleep`, which
// writes who asked to the journal and refuses within a few seconds of waking up.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

Singleton {
    id: root
    property string press: "sleep"       // sleep | menu | shutdown | nothing
    property string hold: "menu"
    property bool overamp: false         // volume up to 150%
    readonly property real maxVolume: overamp ? 1.5 : 1.0
    property var watching: []
    property var problems: []
    property bool helperHasPower: false  // hearth-buttons watches a device with the power button

    signal osd(real volume, bool muted)
    signal powerMenuRequested()

    readonly property var sink: Pipewire.defaultAudioSink
    PwObjectTracker { objects: [root.sink].filter(Boolean) }

    // ---------------------------------------------------------------- settings
    function save() {
        file.setText(JSON.stringify({ press: press, hold: hold, overamp: overamp }, null, 2) + "\n");
    }
    function set(key, value) {
        root[key] = value;
        if (key === "overamp" && !value && sink && sink.audio && sink.audio.volume > 1) sink.audio.volume = 1;
        save();
    }
    FileView {
        id: file
        path: Quickshell.env("HOME") + "/.config/nimbus/buttons.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const o = JSON.parse(text());
                const ok = ["sleep", "menu", "shutdown", "nothing"];
                root.press = ok.includes(o.press) ? o.press : "sleep";
                root.hold = ok.includes(o.hold) ? o.hold : "menu";
                root.overamp = o.overamp === true;
            } catch (e) {}
        }
    }

    // ---------------------------------------------------------------- presses
    // A press labwc passes on (fromHelper false) waits a moment: if hearth-buttons reports the same press
    // meanwhile, it was the same button and is done once.
    property double lastHelperVol: 0
    property double lastHelperPower: 0
    property var pending: null
    Timer {
        id: backup
        interval: 250
        onTriggered: {
            const p = root.pending; root.pending = null;
            if (!p) return;
            const since = Date.now() - (p.kind === "power" ? root.lastHelperPower : root.lastHelperVol);
            if (since < (p.kind === "power" ? 1500 : 600)) return;
            if (p.kind === "vol") root.doVolume(p.d); else if (p.kind === "mute") root.doMute(); else root.doPower("short");
        }
    }
    function later(p) { pending = p; backup.restart(); }

    function volume(d, fromHelper) {
        if (!fromHelper) return later({ kind: "vol", d: d });
        lastHelperVol = Date.now();
        doVolume(d);
    }
    function mute(fromHelper) {
        if (!fromHelper) return later({ kind: "mute" });
        lastHelperVol = Date.now();
        doMute();
    }
    function power(kind, fromHelper) {
        if (!fromHelper) {
            if (helperHasPower) { console.info("power button: labwc's copy of the press left to hearth-buttons"); return; }
            return later({ kind: "power" });
        }
        lastHelperPower = Date.now();
        doPower(kind);
    }

    function doVolume(d) {
        const a = sink ? sink.audio : null;
        if (!a) { Quickshell.execDetached(["wpctl", "set-volume", "-l", String(maxVolume), "@DEFAULT_AUDIO_SINK@", d > 0 ? "5%+" : "5%-"]); return; }
        const v = Math.max(0, Math.min(maxVolume, Math.round((a.volume + d * 0.05) * 20) / 20));
        a.volume = v;
        if (d > 0) a.muted = false;
        osd(v, a.muted);
    }
    function doMute() {
        const a = sink ? sink.audio : null;
        if (!a) { Quickshell.execDetached(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]); return; }
        a.muted = !a.muted;
        osd(a.volume, a.muted);
    }
    // Sleep, with who asked written to the journal (Send logs' Sleep section); refused just after waking up.
    function sleep(who) { Quickshell.execDetached(["python3", root.helper, "sleep", "Desktop Mode: " + who]); }
    function doPower(kind) {
        const what = kind === "long" ? hold : press;
        console.info("power button: " + kind + " press → " + what);
        if (what === "sleep") sleep("power button " + kind + " press (" + (Date.now() - lastHelperPower < 1500 ? "hearth-buttons" : "labwc key binding") + ")");
        else if (what === "shutdown") Quickshell.execDetached(["systemctl", "poweroff"]);
        else if (what === "menu") powerMenuRequested();
    }

    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-buttons").toString().replace(/^file:\/\//, ""))
    Process {
        id: proc
        command: ["python3", root.helper]
        running: true
        stdout: SplitParser {
            onRead: line => {
                let m;
                try { m = JSON.parse(line); } catch (e) { return; }
                if (m.t === "vol") root.volume(m.d, true);
                else if (m.t === "mute") root.mute(true);
                else if (m.t === "power") root.power(m.press, true);
                else if (m.t === "ignored") { root.lastHelperPower = Date.now(); console.info("power button: press ignored " + m.after + " s after waking up"); }
                else if (m.t === "devices") { root.watching = m.list || []; root.problems = m.problems || []; root.helperHasPower = m.power === true; }
            }
        }
        onExited: code => { root.helperHasPower = false; if (code !== 2) restart.start(); }
    }
    Timer { id: restart; interval: 3000; onTriggered: proc.running = true }
}
