// Couch, Desk or Away for each screen on Firehold on labwc (1.16, mockups firehold-plasma 04-06): the live modes
// (hearth-firehold-plasma's modes file in $XDG_RUNTIME_DIR, never a setting: "Switching modes is never a settings
// change"), read when the helper says they changed (Firehold's IPC "ask modes"), and what the Desk bar's mode pill and
// its Modes card (the Couch's ModesCard.qml, the same card as under Plasma) need: the "bar" it's given.
// Nothing runs by itself: no timer; the helper only when the modes change or you pick one.
import QtQuick
import Quickshell

Item {
    id: lm
    property var fh: null
    property bool active: false
    FhRun { id: runner }
    function helper(args, cb) { runner.run(args, cb); }
    function lastJson(t) { return runner.lastJson(t); }

    property var modes: ({ screens: [] })
    property bool read: false
    function readModes() {
        if (!active) return;
        helper(["modes"], function (t) { const o = lm.lastJson(t); if (o && Array.isArray(o.screens)) { lm.modes = o; lm.read = true; } });
    }
    onActiveChanged: if (active) readModes()
    Component.onCompleted: if (active) readModes()

    function modeOf(name) {
        for (const s of modes.screens) if (s.name === name) return s.mode;
        return "";
    }
    // What Mantel's posture is from the modes: Couch when every screen that's in use is Couch (Away screens show the
    // Couch's Away page), else Desk.
    readonly property string posture: {
        const live = modes.screens.map(s => s.mode).filter(m => m !== "away");
        if (!modes.screens.length) return "";
        return live.length === 0 || live.every(m => m === "couch") ? "couch" : "desk";
    }
    onPostureChanged: if (active && fh && posture !== "") fh.followModes(posture)

    // ---- the "bar" ModesCard.qml draws from (as Plasma's mode pill gives it)
    readonly property var cfg: fh ? fh.cfg : ({})
    readonly property string myScreen: fh && fh.deskScreen ? fh.deskScreen.name : (Quickshell.screens.length ? Quickshell.screens[0].name : "")
    readonly property var me: {
        for (const s of modes.screens) if (s.name === myScreen) return s;
        return { name: myScreen, mode: fh && fh.posture === "couch" ? "couch" : "desk", tv: false, why: "" };
    }
    readonly property var tv: {
        for (const s of modes.screens) if (s.tv && s.name !== myScreen) return s;
        return null;
    }
    function modeName(m) { return m === "couch" ? "Couch" : m === "away" ? "Away" : "Desk"; }
    function setMode(target, m) { helper(["mode", target, m, "--why", "pick"], function () { lm.readModes(); }); }
    // (Firehold's own settings: Mantel's Firehold writes them, one key at a time, only when you flip one)
    function setSetting(key, value) { if (fh) fh.set(key, value); }
    property bool popupOpen: false
}
