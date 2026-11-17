// App updates notice (1.14, like KDE Discover's): a few minutes after Desktop Mode starts and then every 6 hours,
// hearth-store check looks for Flatpak updates Hearth hasn't told you about yet; when there are some, a Hearth notice
// says so with "Update all" and "See updates". Update all updates every app that asks for nothing new (an update that
// wants new permissions waits for you in Settings › App store › Updates, where you see what it asks for), then says
// how it went. Off: Settings › App store › Updates › "Tell me when app updates are there" (store.json notifyUpdates).
// Loaded by file name from shell.qml, so a problem here only loses the notice. Changes no setting.
//   quickshell ipc -p <shell folder> call storewatch check     look now (and tell even if it was told before)
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: sw
    property var openStore: null             // (part) => Settings › App store at that part ("updates")
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-store").toString().replace(/^file:\/\//, ""))
    property var last: null
    function lastJson(text) {
        const lines = (text || "").trim().split("\n");
        for (let i = lines.length - 1; i >= 0; i--) { try { return JSON.parse(lines[i]); } catch (e) {} }
        return null;
    }
    function look() {
        if (probe.running || updater.running) return;
        if (notice.running) notice.running = false;      // an older notice nobody answered: its buttons stop, a new one comes
        probe.running = true;
    }

    Timer { interval: 6 * 3600 * 1000; repeat: true; running: true; onTriggered: sw.look() }
    Timer { interval: 8 * 60 * 1000; running: true; onTriggered: sw.look() }      // once the desktop has settled
    Process {
        id: probe
        command: ["python3", sw.helper, "check"]
        stdout: StdioCollector { onStreamFinished: { const r = sw.lastJson(text); if (r && r.notify && r.fresh && r.count > 0) sw.tell(r); } }
    }
    function tell(r) {
        last = r;
        const names = r.names.slice(0, 3).join(", ") + (r.count > 3 ? " and " + (r.count - 3) + " more" : "");
        let body = names + ".";
        if (r.waiting > 0) body += " " + r.waiting + (r.waiting === 1 ? " asks" : " ask") + " for new permissions: look at " + (r.waiting === 1 ? "it" : "them") + " in the App store first.";
        notice.command = ["notify-send", "-a", "Hearth", "-i", "system-software-update", "--action=update=Update all",
                          "--action=open=See updates", r.count + (r.count === 1 ? " app update is ready" : " app updates are ready"), body];
        notice.running = true;
    }
    Process {
        id: notice
        stdout: StdioCollector {
            onStreamFinished: {
                const a = text.trim();
                if (a === "update") updater.running = true;
                else if (a === "open" && sw.openStore) sw.openStore("updates");
            }
        }
    }
    Process {
        id: updater
        command: ["python3", sw.helper, "update"]
        stdout: StdioCollector {
            onStreamFinished: {
                const r = sw.lastJson(text), err = (text.trim().split("\n").pop() || "");
                let title, body;
                if (r && r.ok) {
                    const w = r.waiting || [];
                    title = w.length ? "Your other apps are up to date" : "Your apps are up to date";
                    body = w.length ? w.map(x => x.name).join(", ") + " " + (w.length === 1 ? "waits" : "wait") + " for you: new permissions to look at in the App store."
                                    : "Every app update is installed.";
                } else {
                    title = "Apps couldn't be updated";
                    body = err.startsWith("error: ") ? err.slice(7) : "Try again from Settings › App store › Updates.";
                }
                done.command = ["notify-send", "-a", "Hearth", "-i", "system-software-update", "--action=open=Open the App store", title, body];
                done.running = true;
            }
        }
    }
    Process {
        id: done
        stdout: StdioCollector { onStreamFinished: if (text.trim() === "open" && sw.openStore) sw.openStore("updates") }
    }
    IpcHandler {
        target: "storewatch"
        // Looks now; what was told before is told again (the CI pictures, or "tell me again").
        function check(): void { Quickshell.execDetached(["sh", "-c", "rm -f \"${XDG_CACHE_HOME:-$HOME/.cache}/hearth/store/told-updates.txt\""]); retell.restart(); }
    }
    Timer { id: retell; interval: 800; onTriggered: sw.look() }
}
