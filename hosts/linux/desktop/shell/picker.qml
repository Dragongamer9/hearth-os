//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Hearth Files in a Quickshell of its own, started by hearth-portal (quickshell -p <this file>):
//   * every app's Open / Save dialog (the xdg-desktop-portal FileChooser backend): HEARTH_PICK_REQUEST is the
//     request (JSON, see hearth-portal), HEARTH_PICK_RESULT where the answer goes ({paths, filter, choices} or
//     {cancel: true}); it ends when the window closes;
//   * Hearth Files where the desktop's own isn't running (Game Mode): HEARTH_FILES_OPEN is the folder or file to
//     show, HEARTH_FILES_PROPS=1 shows only its Properties.
// The controller: in Desktop Mode the desktop's hearth-pad drives it (Pad.qml knows windows called "… – Files");
// in Game Mode (HEARTH_PICKER_PAD=1) it starts its own hearth-pad and draws the on-screen keyboard inside the window.
// For checks: quickshell ipc -p <this file> call picker name <text> | accept | cancel | filter <n> | choice <id> <value>
//   (and call filemanager select|activate <name>, as for the desktop's Hearth Files)
import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    id: root
    readonly property string reqPath: Quickshell.env("HEARTH_PICK_REQUEST") || ""
    readonly property string resPath: Quickshell.env("HEARTH_PICK_RESULT") || ""
    readonly property string openPath: Quickshell.env("HEARTH_FILES_OPEN") || ""
    readonly property bool propsOnly: Quickshell.env("HEARTH_FILES_PROPS") === "1"
    readonly property bool ownPad: Quickshell.env("HEARTH_PICKER_PAD") === "1"
    property bool started: false
    property bool ending: false

    FileManager {
        id: files
        onPicked: result => root.finish(result)
        onShownChanged: if (!shown && root.started && !root.reqPath && !root.propsOnly) root.quit()
        onPropsShownChanged: if (!propsShown && root.started && root.propsOnly) root.quit()
        onOskKey: key => pad.send({ cmd: "type", key: key })
    }

    function begin() {
        if (reqPath) return;        // reqFile starts the picker once it has read the request
        started = true;
        if (propsOnly) files.properties([openPath || Quickshell.env("HOME")]);
        else files.open(openPath || Quickshell.env("HOME"));
    }
    FileView {
        id: reqFile
        path: root.reqPath
        onLoaded: {
            if (!root.reqPath || root.started) return;
            let req = null;
            try { req = JSON.parse(text()); } catch (e) { console.warn("picker: unreadable request", e); }
            if (!req) return root.finish({ cancel: true });
            root.started = true;
            files.startPick(req);
        }
        onLoadFailed: if (root.reqPath) root.finish({ cancel: true })
    }

    // The answer, written whole (then renamed) so hearth-portal never reads half of it.
    Process {
        id: writer
        onExited: root.quit()
    }
    function finish(result) {
        if (ending) return;
        ending = true;
        if (!resPath) return quit();
        writer.command = ["sh", "-c", 'printf "%s" "$1" > "$2.part" && mv -f "$2.part" "$2"', "sh", JSON.stringify(result), resPath];
        writer.running = true;
    }
    function quit() {
        pad.send({ cmd: "keys", on: false });
        Qt.quit();
    }

    // ------------------------------------------------ the controller and the on-screen keyboard
    QtObject {
        id: padApi
        function openKeyboard() {
            if (root.ownPad) pad.send({ cmd: "osk", open: true });
            else Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "keyboard", "open"]);
        }
        function closeKeyboard() {
            if (root.ownPad) { pad.send({ cmd: "osk", open: false }); files.osk = null; }
            else Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "keyboard", "close"]);
        }
    }
    Scope {
        id: pad
        readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-pad").toString().replace(/^file:\/\//, ""))
        function send(o) { if (proc.running) proc.write(JSON.stringify(o) + "\n"); }
        Process {
            id: proc
            command: ["python3", pad.helper]
            running: root.ownPad
            stdinEnabled: true
            stdout: SplitParser {
                onRead: line => {
                    let m;
                    try { m = JSON.parse(line); } catch (e) { return; }
                    if (m.t === "ready") { pad.send({ cmd: "keys", on: true }); pad.send({ cmd: "mouse", on: true }); }
                    else if (m.t === "osk") files.osk = m.open ? m : null;
                }
            }
        }
    }

    // ------------------------------------------------ for checks (desktop-test.yml)
    IpcHandler {
        target: "picker"
        function name(text: string): void { files.pickName = text; }
        function accept(): void { files.pickAccept(null); }
        function confirm(): void { if (files.overlay === "confirm") files.accept(""); }
        function cancel(): void { files.close(); }
        function filter(n: int): void { files.pickFilter = n; }
        function choice(id: string, value: string): void { files.setChoice(id, value); }
        function go(path: string): void { files.navigate(path, ""); }
        function state(): string { return JSON.stringify({ picking: files.picking, path: files.path, entries: files.entries.map(e => e.name), name: files.pickName, filter: files.pickFilter, overlay: files.overlay, title: files.pick ? files.pick.title : "" }); }
    }

    Component.onCompleted: {
        SettingsNav.pad = padApi;
        begin();
    }
}
