pragma Singleton

// Hearth's system sounds (1.14; Settings › Sounds, SetSounds.qml). The set: Hearth's own short sounds
// (desktop/sounds/hearth, made by tools/make-hearth-sounds.py, GPL-3.0 like Hearth), the freedesktop sound theme when
// it's installed (played from /usr/share/sounds/freedesktop, never copied: desktop/sounds/LICENSES.md), or none.
// Each event can be turned off. ~/.config/nimbus/sounds.json holds only what you changed (SettingsWriter: merged,
// atomic, .bak); opening the page writes nothing. Notifications still follow each app's own Sound switch.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: snd
    readonly property string path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/sounds.json"
    readonly property string ownDir: Quickshell.shellDir + "/../sounds/hearth/stereo"
    readonly property string fdoDir: "/usr/share/sounds/freedesktop/stereo"
    property var raw: ({})
    // The events, in the page's order. on: the default (volume clicks and the charger off, as KDE has them).
    readonly property var events: [
        { id: "message-new-instant", label: "Notification", sub: "For apps whose Sound is on (Settings › Notifications)", on: true, icon: "bell" },
        { id: "dialog-warning", label: "Warning", sub: "Hearth's own warnings: low space, a settings file put back", on: true, icon: "warn" },
        { id: "dialog-error", label: "Something went wrong", sub: "Urgent notifications", on: true, icon: "close" },
        { id: "complete", label: "Finished", sub: "A copy, download or other job is done", on: true, icon: "check" },
        { id: "device-added", label: "Drive plugged in", sub: "A USB stick, card or disk", on: true, icon: "drive" },
        { id: "device-removed", label: "Drive removed", sub: "", on: true, icon: "drive" },
        { id: "battery-low", label: "Battery low", sub: "", on: true, icon: "batt-low" },
        { id: "power-plug", label: "Charger plugged in", sub: "", on: false, icon: "batt-charging" },
        { id: "power-unplug", label: "Charger unplugged", sub: "", on: false, icon: "batt" },
        { id: "audio-volume-change", label: "Volume keys", sub: "A click as the volume changes", on: false, icon: "vol" },
        { id: "bell", label: "Caps Lock", sub: "As Caps Lock goes on or off", on: false, icon: "kbd" }
    ]
    readonly property string set: raw.set === "freedesktop" || raw.set === "off" ? raw.set : "hearth"
    property bool fdoInstalled: false
    function isOn(id) {
        const e = events.find(x => x.id === id);
        const o = raw.events && typeof raw.events === "object" ? raw.events[id] : undefined;
        return typeof o === "boolean" ? o : (e ? e.on : true);
    }
    function fileFor(id, which) {
        const s = which || set;
        if (s === "off") return "";
        return s === "freedesktop" ? fdoDir + "/" + id + ".oga" : ownDir + "/" + id + ".wav";
    }
    // Plays an event (when it's on and the set isn't Off). The freedesktop set falls back to Hearth's own file for
    // the few names it hasn't got.
    function play(id) { if (set !== "off" && isOn(id)) preview(id, set); }
    function preview(id, which) {
        const f = fileFor(id, which), own = fileFor(id, "hearth");
        if (!f) return;
        Quickshell.execDetached(["sh", "-c", 'f="$1"; [ -f "$f" ] || f="$2"; [ -f "$f" ] || exit 0; ' +
            'if command -v pw-play >/dev/null; then exec pw-play --media-role=Notification "$f"; elif command -v paplay >/dev/null; then exec paplay "$f"; elif command -v aplay >/dev/null; then exec aplay -q "$f"; fi', "sh", f, own]);
        lastPlayed = id;
    }
    property string lastPlayed: ""        // (checks: ipc call sounds last)

    function setSet(v) { if (v !== set) put({ set: v }); }
    function setOn(id, on) {
        if (isOn(id) === on) return;
        const ev = Object.assign({}, raw.events && typeof raw.events === "object" ? raw.events : {});
        ev[id] = on;
        put({ events: ev });
    }
    function put(o) { raw = Object.assign({}, raw, o); writer.merge(o); }
    SettingsWriter { id: writer; path: snd.path; history: true }
    FileView {
        path: snd.path
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") { console.warn("sounds: sounds.json couldn't be read; putting back the last good copy"); writer.repair(); }
                return;
            }
            const r = Object.assign({}, o);
            for (const k in snd.raw) if (writer.owns(k)) r[k] = snd.raw[k];
            snd.raw = r;
        }
    }
    Process { running: true; command: ["test", "-f", snd.fdoDir + "/bell.oga"]; onExited: code => snd.fdoInstalled = code === 0 }

    IpcHandler {
        target: "sounds"
        function play(id: string): void { snd.play(id); }
        function last(): string { return snd.lastPlayed; }
    }
}
