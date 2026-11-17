// Ink and toner running low, told (1.14, KDE's printer ink notices, in Hearth's way). Three minutes after Desktop Mode
// starts and then every three hours, hearth-ink asks CUPS (read only) how full each printer's cartridges are; one at or
// under its low mark (the printer's own, or 15 %) gets one notification ("Magenta ink is low on Brother DCP L2550", Open
// shows Settings › Printers › Ink & toner). Each cartridge is told once until it's been filled again
// (~/.local/state/nimbus/ink-told.json, state, not a setting). Off with Settings.inkNotify. Loaded by shell.qml by file
// name: a problem here only loses these notices.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: iw
    property var settings: null
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-ink").toString().replace(/^file:\/\//, ""))
    readonly property string statePath: Quickshell.env("HOME") + "/.local/state/nimbus/ink-told.json"
    property var told: ({})
    property bool toldRead: false
    SettingsWriter { id: writer; path: iw.statePath }
    FileView {
        path: iw.statePath
        onLoaded: { try { const o = JSON.parse(text()); iw.told = o && typeof o === "object" ? o : {}; } catch (e) { iw.told = {}; } iw.toldRead = true; }
        onLoadFailed: iw.toldRead = true
    }
    Timer { interval: 180000; running: true; onTriggered: iw.check() }
    Timer { interval: 3 * 3600000; running: true; repeat: true; onTriggered: iw.check() }
    function check() { if (Settings.inkNotify && iw.toldRead && !look.running) look.running = true; }
    Process {
        id: look
        command: ["python3", iw.helper, "levels"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop()); } catch (e) { return; }
                const ch = {}, low = [];
                for (const p of (r && r.printers) || []) for (const m of p.markers || []) {
                    if (m.level < 0) continue;
                    const key = p.name + "/" + m.name;
                    const isLow = m.level <= Math.max(1, m.low || 15);
                    if (isLow && !iw.told[key]) { ch[key] = true; low.push({ printer: p.name.replace(/_/g, " "), m: m }); }
                    else if (!isLow && iw.told[key] && m.level > (m.low || 15) + 10) ch[key] = false;   // filled again
                }
                if (Object.keys(ch).length) { writer.merge(ch); iw.told = Object.assign({}, iw.told, ch); }
                if (!low.length) return;
                const one = low[0];
                const what = /toner/i.test(one.m.type + one.m.name) ? "toner" : "ink";
                const title = low.length === 1 ? (one.m.name + (/(ink|toner)/i.test(one.m.name) ? "" : " " + what) + " is low") : low.length + " cartridges are low";
                const body = low.length === 1 ? one.m.level + " % left in " + one.printer : low.map(l => l.m.name + " (" + l.m.level + " %)").slice(0, 4).join(", ") + " · " + one.printer;
                tell.command = ["notify-send", "-a", "Printers", "-i", "printer", "-A", "open=Open", "-w", title, body];
                if (!tell.running) tell.running = true;
            }
        }
    }
    Process {
        id: tell
        stdout: SplitParser { onRead: l => { if (l.trim() === "open" && iw.settings) iw.settings.open("ink-levels"); } }
    }
    IpcHandler {
        target: "inkwatch"
        function check(): void { iw.check(); }
    }
}
