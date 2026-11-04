// Connects hearth-pad (the controller helper) with the shell: starts it, draws what it asks for
// (Hearth menu, keyboard, controls), and tells it what's on screen (a menu with keyboard focus is open,
// a fullscreen game is in front). If python3-evdev or /dev/uinput is missing, the reason is kept in
// `problem` and the desktop works as usual with mouse, keyboard and touch.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: pad
    required property var launcher
    required property var quick
    required property var radial
    required property var keyboard
    required property var help
    required property var settings
    required property var tasks
    required property var files
    required property var powerMenu
    required property var liveWelcome
    property var steamFallback: null   // SteamFallback.qml
    property bool mouseOn: true
    property string problem: ""
    // The controller app in front that has the controller (hearth-pad's owner {app}): the helper stands aside for
    // it (docs/CONTROLLER-FOCUS.md); ownerMouse: Guide + R3 gave the pointer back in it.
    property string owner: ""
    property bool ownerMouse: false
    property string simDir: ""          // checks only (desktop-test.yml): hearth-pad with a simulated controller

    readonly property var active: ToplevelManager.activeToplevel
    // The controller types arrow keys / Enter / Esc instead of moving the pointer in Hearth's own menus
    // and in Hearth Settings when it's the window in front. Hearth Files' windows are "<folder> – Files", and every
    // app's Open / Save dialog (picker.qml, another Quickshell) is "<title> – Files". Hearth Files built from Dolphin
    // (app id org.kde.dolphin, hosts/hearth-kde) too: it does LB/RB, LT/RT and View itself (hearthgamepad.cpp).
    readonly property bool keysContext: launcher.shown || quick.shown || powerMenu.shown || liveWelcome.shown || (steamFallback !== null && steamFallback.shown) || (settings.shown && active !== null && active.title === "Hearth Settings") || (tasks.shown && active !== null && active.title === "Task Manager") || (active !== null && (active.title === "Files" || active.title.endsWith(" – Files") || active.appId === "org.kde.dolphin"))
        || SettingsNav.clipShown || SettingsNav.friendsShown || (SettingsNav.appsShown && active !== null && active.title === "Apps")   // clipboard history, Apps
        || (SettingsNav.debShown && active !== null && active.title === "Install Package")   // the .deb installer
        || DeskState.overlayShown   // right-click menus, the desktop's dialogs, the clock's panel
    readonly property bool fullscreenApp: active !== null && active.fullscreen === true
    // The window in front (labwc's foreign-toplevel list): hearth-pad decides from its app id whether the app reads
    // the controller itself (controller-apps.json, X-Hearth-Controller=true in its .desktop file, or a claim).
    readonly property string frontApp: active !== null ? (active.appId || "") : ""
    function sendFocus() { send({ cmd: "focus", app: frontApp, title: active !== null ? (active.title || "") : "" }); }
    onFrontAppChanged: sendFocus()

    // Hearth's menus that take the keyboard (Quick Access, the launcher, the power menu, the clipboard, Friends, the
    // desktop's menus): when one closes, the window that was in front gets the keyboard (and so the controller: a
    // WebKit app only reads it while its window is focused) back, unless something else came to the front meanwhile.
    readonly property bool menuOpen: launcher.shown || quick.shown || powerMenu.shown || SettingsNav.clipShown || SettingsNav.friendsShown || DeskState.overlayShown
    property var beforeMenu: null
    onMenuOpenChanged: {
        if (menuOpen) { beforeMenu = active; refocus.stop(); }
        else refocus.restart();
    }
    Timer {
        id: refocus
        interval: 180
        onTriggered: {
            const w = pad.beforeMenu;
            pad.beforeMenu = null;
            if (!w || pad.menuOpen) return;
            const cur = ToplevelManager.activeToplevel;
            if ((cur === w || cur === null) && ToplevelManager.toplevels.values.indexOf(w) >= 0) w.activate();
        }
    }

    function send(o) { if (proc.running) proc.write(JSON.stringify(o) + "\n"); }
    function sync() {
        sendFocus();
        send({ cmd: "keys", on: keysContext });
        send({ cmd: "pause", on: fullscreenApp });
        send({ cmd: "mouse", on: mouseOn });
        send({ cmd: "speed", value: Settings.pointerSpeed });
    }
    Connections {
        target: Settings
        function onPointerSpeedChanged() { pad.send({ cmd: "speed", value: Settings.pointerSpeed }); }
    }
    onKeysContextChanged: send({ cmd: "keys", on: keysContext })
    onFullscreenAppChanged: send({ cmd: "pause", on: fullscreenApp })

    function openMenu() {
        radial.sel = -1;
        radial.open = true;
        send({ cmd: "radial", open: true });
    }
    // The on-screen keyboard (Keyboard.qml) works without hearth-pad too: touch, the mouse, and typing through wtype.
    function openKeyboard() { keyboard.openKb(); }
    function closeKeyboard() { keyboard.closeKb(); }
    function setMouse(on) { mouseOn = on; send({ cmd: "mouse", on: on }); help.toast("Controller mouse " + (on ? "on" : "off")); }
    // The Hearth menu's Controller mouse over a controller app: the pointer in that app (like Guide + R3), on / off.
    function toggleMouse() { if (owner !== "") send({ cmd: "padmouse" }); else setMouse(!mouseOn); }

    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-pad").toString().replace(/^file:\/\//, ""))

    Process {
        id: proc
        command: ["python3", pad.helper]
        environment: pad.simDir !== "" ? { "HEARTH_PAD_SIM": pad.simDir } : ({})
        running: true
        stdinEnabled: true
        stdout: SplitParser { onRead: line => pad.handle(line) }
        stderr: SplitParser { onRead: line => console.warn("hearth-pad:", line) }
        // 2 and 3 mean something is missing (see hearth-pad): don't keep retrying.
        onExited: code => { if (code !== 2 && code !== 3) restart.start(); }
    }
    Timer { id: restart; interval: 3000; onTriggered: proc.running = true }

    function handle(line) {
        let m;
        try { m = JSON.parse(line); } catch (e) { return; }
        // The controller was just used for Hearth's menus: Mantel 2026's button-hint strip shows for a while (HintStrip.qml).
        if (["radial", "osk", "oskpad", "launcher", "help"].indexOf(m.t) >= 0) DeskState.padSeen = Date.now();
        switch (m.t) {
        case "ready": problem = ""; sync(); if (keyboard.open) send({ cmd: "osk", open: true, split: keyboard.cfg.fast === "split" }); break;
        case "error": problem = m.text; break;
        case "radial": radial.update(m); break;
        case "osk": if (m.open) openKeyboard(); else closeKeyboard(); break;
        case "oskpad": keyboard.padButton(m.b, !!m.down); break;
        case "oskaxes": keyboard.padAxes(m.lx || 0, m.ly || 0, m.rx || 0, m.ry || 0); break;
        case "launcher": launcher.toggle(); break;
        case "help": help.toggle(); break;
        case "toast": help.toast(m.text); break;
        case "mouse": mouseOn = m.on; break;
        case "owner": owner = m.app || ""; ownerMouse = m.mouse === true; break;
        }
    }

    // What the Hearth menu's items do.
    function run(action) {
        const share = Quickshell.env("NIMBUS_SHARE") || "/usr/local/share/nimbus";
        switch (action) {
        case "apps": launcher.shown = true; break;
        case "keyboard": openKeyboard(); break;
        case "quick": quick.shown = true; break;
        case "close": if (active) active.close(); break;
        case "game": Quickshell.execDetached(["nimbus-mode", "game"]); break;
        case "screenshot": Quickshell.execDetached(["sh", "-c", "sleep 0.4; exec sh \"$0\" screenshot", share + "/desktop/open.sh"]); break;
        case "mouse": toggleMouse(); break;
        case "help": help.shown = true; break;
        }
    }

    Connections {
        target: pad.radial
        function onPicked(action) { pad.run(action); }
        function onDismissed() { pad.radial.open = false; }
        // Picked or closed with the mouse/touch: hearth-pad still thinks the menu is open.
        function onOpenChanged() { if (!pad.radial.open) pad.send({ cmd: "radial", open: false }); }
    }
    Connections {
        target: pad.keyboard
        function onCloseRequested() { pad.closeKeyboard(); }
    }
    Component.onCompleted: keyboard.pad = pad

    // For checks (desktop-test.yml): what's in front and who has the controller; hearth-pad again with a simulated
    // controller (HEARTH_PAD_SIM: presses from <dir>/in, what it would send in <dir>/out.log).
    IpcHandler {
        target: "pad"
        function state(): string {
            return JSON.stringify({ front: pad.frontApp, title: pad.active !== null ? (pad.active.title || "") : "", owner: pad.owner,
                                    ownerMouse: pad.ownerMouse, keys: pad.keysContext, menuOpen: pad.menuOpen, running: proc.running, problem: pad.problem });
        }
        function sim(dir: string): void {
            pad.simDir = dir;
            if (proc.running) { proc.running = false; restart.start(); } else proc.running = true;
        }
    }
}
