// Hearth Plasma's on-screen keyboard: the same keyboard as Hearth Classic's (Keyboard.qml, Hearth's shared engine in
// osk/hearth-osk.js; docs/OSK.md), as a layer-shell window over KDE Plasma that never takes the keyboard focus, so
// what it types goes to the window you're in. Guide + X on the controller opens and closes it (like Steam + X), as do
// B and its hide key. hearth-pad --osk reads the controller: it does nothing until Guide + X, holds the controller
// for the keyboard while it's open (apps don't see the presses), and types through its uinput keyboard (KWin has
// no wtype). Started by /usr/share/hearth-plasma/xdg/autostart/hearth-osk.desktop:
//   quickshell -p /usr/share/nimbus/desktop/shell/osk-plasma.qml
// If quickshell, python3-evdev or /dev/uinput is missing, Plasma works as usual, just without it.
import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    id: root

    Keyboard {
        id: keyboard
        useWtype: false
        onCloseRequested: keyboard.closeKb()
    }

    Scope {
        id: pad
        function send(o) { if (proc.running) proc.write(JSON.stringify(o) + "\n"); }
        readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-pad").toString().replace(/^file:\/\//, ""))

        Process {
            id: proc
            command: ["python3", pad.helper, "--osk"]
            running: true
            stdinEnabled: true
            stdout: SplitParser { onRead: line => pad.handle(line) }
            stderr: SplitParser { onRead: line => console.warn("hearth-pad --osk:", line) }
            // 2 and 3 mean something is missing (see hearth-pad): don't keep retrying.
            onExited: code => { if (code !== 2 && code !== 3) restart.start(); }
        }
        Timer { id: restart; interval: 3000; onTriggered: proc.running = true }

        function handle(line) {
            let m;
            try { m = JSON.parse(line); } catch (e) { return; }
            switch (m.t) {
            case "ready": if (keyboard.open) send({ cmd: "osk", open: true, split: keyboard.cfg.fast === "split" }); break;
            case "osk": if (m.open) keyboard.openKb(); else keyboard.closeKb(); break;
            case "oskpad": keyboard.padButton(m.b, !!m.down); break;
            case "oskaxes": keyboard.padAxes(m.lx || 0, m.ly || 0, m.rx || 0, m.ry || 0); break;
            }
        }
        Component.onCompleted: keyboard.pad = pad
    }
}
