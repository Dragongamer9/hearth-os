pragma Singleton

// The power and volume buttons in Desktop Mode, and their settings (~/.config/nimbus/buttons.json, the same
// file Hearth's Game Mode settings use). hearth-buttons reads the buttons from the kernel; shell.qml runs it
// and calls volume()/mute()/power() here. labwc's key bindings call the same through ipc, as a backup for
// keyboards hearth-buttons can't read; presses that both see are counted once.
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
        if (!fromHelper) return later({ kind: "power" });
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
    function doPower(kind) {
        const what = kind === "long" ? hold : press;
        if (what === "sleep") Quickshell.execDetached(["systemctl", "suspend"]);
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
                else if (m.t === "devices") { root.watching = m.list || []; root.problems = m.problems || []; }
            }
        }
        onExited: code => { if (code !== 2) restart.start(); }
    }
    Timer { id: restart; interval: 3000; onTriggered: proc.running = true }
}
