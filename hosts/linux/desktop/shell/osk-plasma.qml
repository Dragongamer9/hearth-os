// Hearth Plasma's controller: everything Hearth Classic's Pad.qml offers, over KDE Plasma (docs/PLASMA.md). Plasma
// stays Plasma; this adds what a controller needs on top, as layer-shell windows that never take the keyboard focus:
// the Hearth menu (Guide: Mantel's RadialMenu.qml), the controls card and short messages (View: Help.qml) and the
// on-screen keyboard (Guide + X, Y, or by itself on a text field: Keyboard.qml, Hearth's shared engine, docs/OSK.md).
// hearth-pad --plasma reads the controller (one helper for both desktops): right stick pointer, triggers click, left
// stick scrolls, D-pad and A / B as keys in Plasma's menus and KDE apps (KWin's hearthpad script says what's in front),
// LB / RB switch windows, View + a button for KWin's window shortcuts, and types through its uinput keyboard (KWin
// has no wtype). Started by /usr/share/hearth-plasma/xdg/autostart/hearth-osk.desktop:
//   quickshell -p /usr/share/nimbus/desktop/shell/osk-plasma.qml
// If quickshell, python3-evdev or /dev/uinput is missing, Plasma works as usual, just without the controller.
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
    RadialMenu {
        id: radial
        mouseOn: pad.mouseOn
        onPicked: action => pad.run(action)
        onDismissed: radial.open = false
        // Picked or closed with the mouse/touch: hearth-pad still thinks the menu is open.
        onOpenChanged: if (!radial.open) pad.send({ cmd: "radial", open: false })
    }
    Help {
        id: help
        controls: [
            ["Right stick", "Move the pointer (press it in: precise)"],
            ["Left stick", "Scroll"],
            ["A  /  RT", "Click (in menus A chooses, RT still clicks)"],
            ["X  /  LT", "Right-click"],
            ["B", "Back (Esc)"],
            ["Y", "On-screen keyboard"],
            ["D-pad", "Move around menus and lists (arrow keys)"],
            ["LB  /  RB", "Switch windows"],
            ["Menu ☰", "Apps"],
            ["Guide", "Hearth menu  ·  Guide + X: keyboard"],
            ["View + D-pad", "Tile left / right, maximize, minimize"],
            ["View + A", "The taskbar (D-pad along it, A opens)"],
            ["View + X  /  Y", "All windows  /  clipboard history"],
            ["View + B", "Close the window"],
            ["View", "This list"]
        ]
    }

    Scope {
        id: pad
        property bool mouseOn: true
        property string problem: ""
        readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-pad").toString().replace(/^file:\/\//, ""))
        readonly property string share: Quickshell.env("NIMBUS_SHARE") || "/usr/share/nimbus"

        function send(o) { if (proc.running) proc.write(JSON.stringify(o) + "\n"); }
        function openKeyboard() { keyboard.openKb(); }
        function closeKeyboard() { keyboard.closeKb(); }
        function setMouse(on) { mouseOn = on; send({ cmd: "mouse", on: on }); help.toast("Controller mouse " + (on ? "on" : "off")); }
        // Plasma's launcher (Hearth's, org.hearth.launcher, or whichever launcher the panel has), as Super does.
        function openLauncher() {
            Quickshell.execDetached(["sh", "-c", "dbus-send --session --type=method_call --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu 2>/dev/null || qdbus6 org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu"]);
        }
        // What the Hearth menu's items do in Plasma (Pad.qml's run() in Hearth Classic).
        function run(action) {
            switch (action) {
            case "apps": openLauncher(); break;
            case "keyboard": openKeyboard(); break;
            case "quick": send({ cmd: "plasma", do: "quick" }); break;       // Hearth Quick Access (Meta+Alt+A)
            case "close": send({ cmd: "plasma", do: "close" }); break;       // Alt+F4
            case "game": Quickshell.execDetached(["nimbus-mode", "game"]); break;
            case "screenshot": Quickshell.execDetached(["sh", "-c", "sleep 0.5; spectacle -f -b -n 2>/dev/null || exec sh \"$0\" screenshot", share + "/desktop/open.sh"]); break;
            case "mouse": setMouse(!mouseOn); break;
            case "help": help.shown = true; break;
            }
        }

        Process {
            id: proc
            command: ["python3", pad.helper, "--plasma"]
            running: true
            stdinEnabled: true
            stdout: SplitParser { onRead: line => pad.handle(line) }
            stderr: SplitParser { onRead: line => console.warn("hearth-pad --plasma:", line) }
            // 2 and 3 mean something is missing (see hearth-pad): don't keep retrying.
            onExited: code => { if (code !== 2 && code !== 3) restart.start(); }
        }
        Timer { id: restart; interval: 3000; onTriggered: proc.running = true }

        function handle(line) {
            let m;
            try { m = JSON.parse(line); } catch (e) { return; }
            switch (m.t) {
            case "ready":
                problem = "";
                send({ cmd: "mouse", on: mouseOn });
                if (keyboard.open) send({ cmd: "osk", open: true, split: keyboard.cfg.fast === "split" });
                break;
            case "error": problem = m.text; console.warn("hearth-pad:", m.text); break;
            case "radial": radial.update(m); break;
            case "osk": if (m.open) keyboard.openKb(); else keyboard.closeKb(); break;
            case "oskpad": keyboard.padButton(m.b, !!m.down); break;
            case "oskaxes": keyboard.padAxes(m.lx || 0, m.ly || 0, m.rx || 0, m.ry || 0); break;
            case "launcher": openLauncher(); break;
            case "help": help.toggle(); break;
            case "toast": help.toast(m.text); break;
            case "mouse": mouseOn = m.on; break;
            }
        }
        Component.onCompleted: keyboard.pad = pad
    }

    // For checks and key bindings: quickshell ipc -p <this file> call controller <function>
    IpcHandler {
        target: "controller"
        function openMenu(): void { radial.sel = -1; radial.open = true; pad.send({ cmd: "radial", open: true }); }
        function openKeyboard(): void { pad.openKeyboard(); }
        function toggleHelp(): void { help.toggle(); }
        function state(): string { return JSON.stringify({ running: proc.running, problem: pad.problem, mouse: pad.mouseOn, keyboard: keyboard.open, menu: radial.open }); }
    }
}
