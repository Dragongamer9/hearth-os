// Firmware updates, told (1.14, KDE Discover's firmware notification, in Hearth's way): five minutes after Desktop Mode
// starts and then once a day, fwupd's list (hearth-firmware devices, as you, read only) is looked at; a part with a new
// firmware gets one notification ("Firmware update for …", Open opens Settings › Firmware & device security). Each
// device and version is told once (~/.local/state/nimbus/firmware-told.json, state, not a setting). Off with
// Settings.firmwareNotify (the page's switch). Nothing is ever installed from here. Loaded by shell.qml by file name.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: fw
    property var settings: null              // Hearth Settings (shell.qml sets it)
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-firmware").toString().replace(/^file:\/\//, ""))
    readonly property string statePath: Quickshell.env("HOME") + "/.local/state/nimbus/firmware-told.json"
    property var told: ({})
    property bool toldRead: false
    SettingsWriter { id: writer; path: fw.statePath }
    FileView {
        path: fw.statePath
        onLoaded: { try { const o = JSON.parse(text()); fw.told = o && typeof o === "object" ? o : {}; } catch (e) { fw.told = {}; } fw.toldRead = true; }
        onLoadFailed: fw.toldRead = true
    }
    Timer { interval: 300000; running: true; onTriggered: fw.check() }
    Timer { interval: 86400000; running: true; repeat: true; onTriggered: fw.check() }
    function check() { if (Settings.firmwareNotify && fw.toldRead && !look.running) look.running = true; }
    Process {
        id: look
        command: ["python3", fw.helper, "devices"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop()); } catch (e) { return; }
                const fresh = (r && r.devices || []).filter(d => d.update && d.update.version && fw.told[d.id] !== d.update.version);
                if (!fresh.length) return;
                const ch = {};
                for (const d of fresh) ch[d.id] = d.update.version;
                writer.merge(ch);
                fw.told = Object.assign({}, fw.told, ch);
                const one = fresh[0];
                const title = fresh.length === 1 ? "Firmware update for " + one.name : fresh.length + " firmware updates";
                const body = fresh.length === 1 ? "Version " + one.update.version + (one.update.urgency === "high" || one.update.urgency === "critical" ? ": an important fix" : "") + (one.system ? ". This one is installed from the PC maker's tools" : "")
                    : fresh.map(d => d.name).slice(0, 3).join(", ");
                tell.command = ["notify-send", "-a", "Hearth", "-i", "software-update-available", "-A", "open=Open", "-w", title, body];
                if (!tell.running) tell.running = true;
            }
        }
    }
    Process {
        id: tell
        stdout: SplitParser { onRead: l => { if (l.trim() === "open" && fw.settings) fw.settings.open("firmware"); } }
    }
    IpcHandler {
        target: "firmwarewatch"
        function check(): void { fw.check(); }
    }
}
