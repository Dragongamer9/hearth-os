//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Server mode's one app (docs/SERVER-MODE.md): started by desktop/server/labwc/autostart (quickshell -p <this file>) in the
// Server mode session (nimbus-session run_server). The dashboard (ServerDash.qml) full screen, Hearth Settings when you
// open it from the menu (the same SettingsApp as Mantel's), and the controller through hearth-pad (keys: A Enter, B Esc,
// X the Menu key, D-pad arrows; Guide is Play, ☰ the menu, Y pause / resume). No taskbar, no launcher, no Steam.
// When the dashboard shows, it leaves $XDG_RUNTIME_DIR/hearth-server-dash-up: nimbus-session closes Server mode and opens
// Game Mode if that mark doesn't come.
// For checks (desktop-test.yml): quickshell ipc -p <this file> call server play | menu | detail <n> | idle <dim|off|on> |
//   cancel | state, and call settings page <id>.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
    id: root

    SettingsApp { id: settingsApp }

    // Behind everything (Settings is a window above it while the dashboard steps aside).
    Variants {
        model: Quickshell.screens
        delegate: Component {
            PanelWindow {
                required property ShellScreen modelData
                screen: modelData
                anchors { top: true; bottom: true; left: true; right: true }
                exclusiveZone: -1
                color: Theme.bg
                WlrLayershell.layer: WlrLayer.Background
                WlrLayershell.namespace: "hearth-server-bg"
            }
        }
    }

    ServerDash {
        id: dash
        screen: Quickshell.screens.length ? Quickshell.screens[0] : null
        settings: settingsApp
        visible: !settingsApp.shown
    }

    // The controller: hearth-pad in keys mode the whole time (as picker.qml runs it). Without python3-evdev or
    // /dev/uinput it ends and the keyboard still works.
    Scope {
        id: pad
        readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-pad").toString().replace(/^file:\/\//, ""))
        function send(o) { if (proc.running) proc.write(JSON.stringify(o) + "\n"); }
        function keys() { send({ cmd: "keys", on: true, why: settingsApp.shown ? "window: quickshell Hearth Settings" : "server mode" }); }
        Process {
            id: proc
            command: ["python3", pad.helper]
            running: Quickshell.env("HEARTH_SERVER_NO_PAD") !== "1"
            // 2 and 3 mean something is missing (see hearth-pad): don't keep retrying; anything else, start it again.
            onExited: code => { if (code !== 2 && code !== 3 && Quickshell.env("HEARTH_SERVER_NO_PAD") !== "1") padRestart.start(); }
            stdinEnabled: true
            stdout: SplitParser {
                onRead: line => {
                    let m;
                    try { m = JSON.parse(line); } catch (e) { return; }
                    if (m.t === "ready") { pad.keys(); pad.send({ cmd: "mouse", on: true }); }   // "mouse off" is hearth-pad's whole-controller off switch: never send it here (1.6 did, and only Guide worked)
                    else if (m.t === "radial" && m.open) {
                        // Guide: Play (the Hearth menu isn't here). The radial menu hearth-pad opened closes again at once.
                        pad.send({ cmd: "radial", open: false });
                        if (!settingsApp.shown) dash.pad("guide");
                    }
                    else if (m.t === "launcher") { if (settingsApp.shown) settingsApp.close(); else dash.pad("menu"); }
                    else if (m.t === "osk") { if (!settingsApp.shown) dash.pad("y"); }
                }
            }
        }
        Timer { id: padRestart; interval: 3000; onTriggered: proc.running = true }
        Connections { target: settingsApp; function onShownChanged() { pad.keys(); } }
    }

    IpcHandler {
        target: "server"
        function play(): void { dash.play(); }
        function menu(): void { dash.overlay = "menu"; dash.menuSel = 0; }
        function detail(n: int): void { dash.sel = n; dash.overlay = "detail"; }
        function back(): void { dash.backInfo = { back: true, played: 9660 }; dash.overlay = "back"; }
        function close(): void { dash.overlay = ""; dash.cancelSwitch(); }
        function cancel(): void { dash.remoteCancel(); }
        function idle(s: string): void { if (s === "dim" || s === "off" || s === "on") dash.screenState = s; }
        function state(): string { return JSON.stringify({ overlay: dash.overlay, sel: dash.sel, screen: dash.screenState, count: dash.count, remote: !!dash.remote, tiles: dash.tiles.length, settings: settingsApp.shown }); }
    }
    IpcHandler {
        target: "settings"
        function open(): void { settingsApp.open(""); }
        function page(name: string): void { settingsApp.open(name); }
        function close(): void { settingsApp.close(); }
    }
}
