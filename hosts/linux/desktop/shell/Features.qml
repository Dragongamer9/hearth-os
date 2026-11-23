pragma Singleton

// Your features (Describe it's Keep this; hosts/hearth-layout feature.rs, ~/.config/nimbus/features): read with
// `hearth-layout features` when Mantel starts and whenever one is kept or turned on or off (reload()). Each one is
// drawn where it belongs (FeaturePill.qml: the bar's middle for "bar"), reads what it shows only while it's on the
// screen (hearth-feature-data), and a click does only one of Hearth's own actions (run()).
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: fs
    property var list: []
    function inPart(p) { return list.filter(x => x.on && x.feature && x.feature.part === p).map(x => x.feature); }
    function reload() { proc.running = false; proc.running = true; }
    Process {
        id: proc
        running: true
        command: ["hearth-layout", "features"]
        stdout: StdioCollector { onStreamFinished: { try { const v = JSON.parse(text.trim() || "[]"); fs.list = Array.isArray(v) ? v : []; } catch (e) { fs.list = []; } } }
    }
    function ipc(target, fn) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", target, fn]); }
    // A click: Hearth's own actions only (feature.rs ACTIONS; the target was checked when it was kept).
    function run(c) {
        if (!c || !c.action) return;
        const t = String(c.target || "");
        switch (c.action) {
        case "open-app": { const e = AppIcons.entryFor(t); if (e) e.execute(); else Quickshell.execDetached(["gtk-launch", t]); break; }
        case "open-settings": Quickshell.execDetached(["hearth-settings", t]); break;
        case "open-folder": Quickshell.execDetached(["xdg-open", (Quickshell.env("HOME") || "") + "/" + t.replace(/^\/+/, "")]); break;
        case "open-url": if (/^https:\/\/\S+$/.test(t)) Quickshell.execDetached(["xdg-open", t]); break;
        case "game-mode": Quickshell.execDetached(["nimbus-mode", "game"]); break;
        case "show-desktop": ipc("taskbar", "showDesktop"); break;
        case "overview": ipc("overview", "toggle"); break;
        case "menu": ipc("launcher", "toggle"); break;
        case "quickaccess": ipc("quick", "toggle"); break;
        case "volume-mute": Quickshell.execDetached(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]); break;
        case "screenshot": Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "capture", "take", "screen"]); break;
        case "lock": ipc("lock", "lock"); break;
        case "dnd": DeskState.dnd = !DeskState.dnd; break;
        }
    }
}
