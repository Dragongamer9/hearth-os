pragma Singleton

// Your vaults for the bar's vault button (1.14, hearth-vaults, SetVaults.qml): read every 15 seconds, only once you have
// a vaults list; lock one or all from the bar. Unlocking asks for the password in Settings › Vaults.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property var vaults: []
    readonly property var opened: vaults.filter(v => v.open)
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-vaults").toString().replace(/^file:\/\//, ""))
    readonly property string tip: opened.length ? opened.map(v => v.name).join(", ") + (opened.length === 1 ? " is unlocked" : " are unlocked") : "Vaults: all locked"
    function refresh() { if (!list.running) list.running = true; }
    function act(args) { run.command = ["python3", helper].concat(args); run.running = true; }
    // The bar's menu: lock or open each unlocked vault, unlock (in Settings) each locked one. openSettings(page).
    function menu(openSettings) {
        const out = [];
        for (const v of vaults) {
            const one = v;
            if (one.open) {
                out.push({ label: "Lock " + one.name, icon: "lock", run: () => root.act(["lock", one.id]) });
                out.push({ label: "Open " + one.name, icon: "folder", run: () => Quickshell.execDetached(["xdg-open", one.mount]) });
            } else {
                out.push({ label: "Unlock " + one.name + "…", icon: "vault", run: () => openSettings("vaults:" + one.id) });
            }
        }
        if (opened.length > 1) out.push({ label: "Lock them all", icon: "lock", run: () => root.act(["lock-all"]) });
        out.push({ label: "Vaults settings", icon: "gear", run: () => openSettings("vaults") });
        return out;
    }
    Process {
        id: list
        command: ["sh", "-c", "[ -e \"$HOME/.config/nimbus/vaults.json\" ] || { echo '{\"vaults\": []}'; exit 0; }; exec python3 \"$0\" list", root.helper]
        stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop() || "{}"); root.vaults = (o && o.vaults) || []; } catch (e) {} } }
    }
    Process {
        id: run
        stdout: StdioCollector { onStreamFinished: { const err = text.split("\n").find(l => l.startsWith("error: ")); if (err) SettingsNav.toast(err.slice(7)); root.refresh(); } }
    }
    Timer { interval: 15000; repeat: true; running: true; triggeredOnStart: true; onTriggered: root.refresh() }
}
