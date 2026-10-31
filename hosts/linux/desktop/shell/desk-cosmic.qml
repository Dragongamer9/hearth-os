// Hearth COSMIC's companion (docs/COSMIC.md): Mantel's pieces on top of System76's COSMIC, as layer-shell windows over
// it. Hearth's launcher (Apps and Games, Menu on the controller, Super, the panel's Hearth entry), Quick Access, Hearth
// Link and Hearth Storage (CosmicHub.qml), the Hearth menu (Guide), the controls card (View) and Hearth's on-screen
// keyboard (Guide + X, or Y), all the same QML as Hearth Classic's. The controller: hearth-pad --cosmic (CosmicPad):
// pointer, clicks, scrolling, the D-pad as arrow keys, window switching and closing through COSMIC's own shortcuts,
// per-app keys for COSMIC Files, Terminal and Edit. Typing goes through hearth-pad's uinput keyboard.
// Started by /usr/share/hearth-cosmic/xdg/autostart/hearth-cosmic.desktop (only in Hearth COSMIC):
//   quickshell -p /usr/share/nimbus/desktop/shell/desk-cosmic.qml
// Called by desktop/cosmic/hearth-cosmic (quickshell ipc ... call hearth launcher|quick|link|storage|menu|keyboard).
// If quickshell, python3-evdev or /dev/uinput is missing, COSMIC works as usual, just without these.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
    id: root

    Launcher {
        id: launcherWin
        onSettingsRequested: Quickshell.execDetached(["cosmic-settings"])
    }
    QuickAccess {
        id: quickWin
        mouseOn: pad.mouseOn
        nightAvailable: false
        onMouseToggled: enable => pad.setMouse(enable)
        onKeyboardRequested: pad.openKeyboard()
        onSettingsRequested: Quickshell.execDetached(["cosmic-settings"])
    }
    CosmicHub { id: hub }
    RadialMenu { id: radial; mouseOn: pad.mouseOn }
    Keyboard {
        id: kb
        useWtype: false          // typing through hearth-pad's uinput keyboard, as in Hearth Plasma
    }
    Help {
        id: helpWin
        controls: [
            ["Right stick", "Move the pointer (press it in: precise)"],
            ["Left stick  /  LT RT", "Scroll"],
            ["A", "Enter (a click right after moving the pointer)"],
            ["X", "Right-click"],
            ["B", "Back (COSMIC Files: the folder before)"],
            ["Y", "Search and the on-screen keyboard"],
            ["D-pad", "Move the selection (arrow keys)"],
            ["LB  /  RB", "Switch windows (COSMIC's apps: tabs)"],
            ["View + LB / RB", "Tabs (COSMIC's apps: switch windows)"],
            ["View + Y", "Search and the on-screen keyboard"],
            ["Menu ☰", "Hearth's apps and games"],
            ["Guide", "Hearth menu"],
            ["Guide + X", "On-screen keyboard"],
            ["View + D-pad", "Move left / right, maximize, minimize"],
            ["View + B", "Close the window"]
        ]
    }

    Scope {
        id: pad
        property bool mouseOn: true
        property string problem: ""
        readonly property var active: ToplevelManager.activeToplevel
        readonly property string appId: active ? String(active.appId || "") : ""
        // Hearth's own menus are open: the controller types arrows / Enter / Esc there (hearth-pad keys {on}).
        readonly property bool keysContext: launcherWin.shown || quickWin.shown || hub.shown
        readonly property bool fullscreenApp: active !== null && active.fullscreen === true

        function send(o) { if (proc.running) proc.write(JSON.stringify(o) + "\n"); }
        function sync() {
            send({ cmd: "keys", on: keysContext });
            send({ cmd: "pause", on: fullscreenApp });
            send({ cmd: "mouse", on: mouseOn });
            send({ cmd: "app", id: appId });
        }
        onKeysContextChanged: send({ cmd: "keys", on: keysContext })
        onFullscreenAppChanged: send({ cmd: "pause", on: fullscreenApp })
        onAppIdChanged: send({ cmd: "app", id: appId })

        function openMenu() { radial.sel = -1; radial.open = true; send({ cmd: "radial", open: true }); }
        function openKeyboard() { kb.openKb(); }
        function closeKeyboard() { kb.closeKb(); }
        function setMouse(on) { mouseOn = on; send({ cmd: "mouse", on: on }); helpWin.toast("Controller mouse " + (on ? "on" : "off")); }

        readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-pad").toString().replace(/^file:\/\//, ""))
        Process {
            id: proc
            command: ["python3", pad.helper, "--cosmic"]
            running: true
            stdinEnabled: true
            stdout: SplitParser { onRead: line => pad.handle(line) }
            stderr: SplitParser { onRead: line => console.warn("hearth-pad --cosmic:", line) }
            // 2 and 3 mean something is missing (see hearth-pad): don't keep retrying.
            onExited: code => { if (code !== 2 && code !== 3) restart.start(); }
        }
        Timer { id: restart; interval: 3000; onTriggered: proc.running = true }

        function handle(line) {
            let m;
            try { m = JSON.parse(line); } catch (e) { return; }
            switch (m.t) {
            case "ready": problem = ""; sync(); if (kb.open) send({ cmd: "osk", open: true, split: kb.cfg.fast === "split" }); break;
            case "error": problem = m.text; break;
            case "radial": radial.update(m); break;
            case "osk": if (m.open) openKeyboard(); else closeKeyboard(); break;
            case "oskpad": kb.padButton(m.b, !!m.down); break;
            case "oskaxes": kb.padAxes(m.lx || 0, m.ly || 0, m.rx || 0, m.ry || 0); break;
            case "launcher": launcherWin.toggle(); break;
            case "help": helpWin.toggle(); break;
            case "toast": helpWin.toast(m.text); break;
            case "mouse": mouseOn = m.on; break;
            }
        }

        // What the Hearth menu's items do.
        function run(action) {
            switch (action) {
            case "apps": launcherWin.shown = true; break;
            case "keyboard": openKeyboard(); break;
            case "quick": quickWin.shown = true; break;
            // COSMIC's own Close (Super+Q) through hearth-pad's keyboard: works whichever window is in front.
            case "close": if (active) active.close(); else send({ cmd: "key", key: "close" }); break;
            case "game": Quickshell.execDetached(["nimbus-mode", "game"]); break;
            case "screenshot": Quickshell.execDetached(["sh", "-c", "sleep 0.4; exec cosmic-screenshot --interactive=false || exec grim"]); break;
            case "mouse": setMouse(!mouseOn); break;
            case "help": helpWin.shown = true; break;
            }
        }
        Connections {
            target: radial
            function onPicked(action) { pad.run(action); }
            function onDismissed() { radial.open = false; }
            function onOpenChanged() { if (!radial.open) pad.send({ cmd: "radial", open: false }); }
        }
        Connections {
            target: kb
            function onCloseRequested() { pad.closeKeyboard(); }
        }
        Component.onCompleted: kb.pad = pad
    }

    // hearth-cosmic (the panel's Hearth entries, Super, Super+G's neighbours) and the CI test call these.
    IpcHandler {
        target: "hearth"
        function launcher(): void { launcherWin.toggle(); }
        function games(): void { launcherWin.openTab("games"); }
        function quick(): void { quickWin.toggle(); }
        function link(): void { hub.toggle("link"); }
        function storage(): void { hub.toggle("storage"); }
        function menu(): void { pad.openMenu(); }
        function keyboard(): void { if (kb.open) pad.closeKeyboard(); else pad.openKeyboard(); }
        function help(): void { helpWin.toggle(); }
        function close(): void { launcherWin.shown = false; quickWin.shown = false; hub.shown = false; radial.open = false; helpWin.shown = false; }
        function app(): string { return pad.appId; }
        function problem(): string { return pad.problem; }
    }
}
