// Files a move put aside, told (red flag 9, Nov 27, 2026). Two minutes after Desktop Mode starts, hearth-old-files
// (as you, read only) looks for ~/.old-hearth-<date> folders that still hold things your home folder doesn't have. For
// each one not answered yet, one notification: "Bring back" puts back everything in it that isn't in your home folder
// now (never replacing anything; hard links, so no extra space), "Keep" keeps it and stops asking, "Delete now…"
// opens Settings › System › Storage, where deleting asks once more. Unanswered, it asks again a day later. Nothing
// here deletes anything, and nothing happens without a press. Loaded by shell.qml by file name: a problem here only
// loses this notice.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: ow
    property var settings: null
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-old-files").toString().replace(/^file:\/\//, ""))
    property string folder: ""
    Timer { interval: 120000; running: true; onTriggered: if (!look.running) look.running = true }
    Process {
        id: look
        command: ["python3", ow.helper, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop()); } catch (e) { return; }
                const f = ((r && r.folders) || []).find(x => x.ask && x.back > 0);
                console.log("OldFilesWatch: " + (((r && r.folders) || []).length) + " folder(s)" + (f ? ", asking about " + f.name : ""));
                if (!f) return;
                ow.folder = f.name;
                asked.command = ["python3", ow.helper, "asked", f.name];
                if (!asked.running) asked.running = true;
                const body = "When this PC moved to another Hearth OS on " + f.date + ", " + f.back + " item" + (f.back === 1 ? "" : "s")
                    + " from your home folder went to ~/" + f.name + ". Bring back puts back everything that isn't in your home folder now and replaces nothing. Nothing is deleted unless you say so.";
                tell.command = ["notify-send", "-a", "Hearth", "-u", "normal", "-A", "back=Bring back", "-A", "keep=Keep", "-A", "delete=Delete now…", "-w",
                                "Bring back what the move put aside?", body];
                if (!tell.running) tell.running = true;
            }
        }
    }
    Process { id: asked }
    Process {
        id: tell
        stdout: SplitParser {
            onRead: l => {
                const k = l.trim();
                if (k === "back") { act.command = ["python3", ow.helper, "bring-back", ow.folder]; if (!act.running) act.running = true; }
                else if (k === "keep") { act.command = ["python3", ow.helper, "keep", ow.folder]; if (!act.running) act.running = true; }
                else if (k === "delete" && ow.settings) ow.settings.open("storage");
            }
        }
    }
    Process {
        id: act
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop()); } catch (e) { r = null; }
                if (!r || r.linked === undefined) return;     // Keep: nothing more to say
                const n = (r.linked || 0) + (r.copied || 0);
                const failed = (r.failed || []).length;
                done.command = ["notify-send", "-a", "Hearth", n + " file" + (n === 1 ? "" : "s") + " brought back",
                                "Nothing in your home folder was replaced" + (failed ? "; " + failed + " couldn't be brought back (Settings › System › Storage)" : "") + ". ~/" + ow.folder + " stays until you delete it."];
                if (!done.running) done.running = true;
            }
        }
    }
    Process { id: done }
    IpcHandler {
        target: "oldfiles"
        function check(): void { if (!look.running) look.running = true; }
    }
}
