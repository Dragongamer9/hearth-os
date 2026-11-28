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

    // Claude's own screen (the owner, Nov 28: "only thing that shows on main is server mode, and on the virtual
    // monitor claude works"): a virtual monitor labwc adds as Server mode starts (desktop/server/labwc/autostart,
    // HEARTH-CLAUDE). Every app window opens there; the dashboard stays on the real screen.
    readonly property string claudeName: "HEARTH-CLAUDE"
    function realScreen() { for (let i = 0; i < Quickshell.screens.length; i++) if (Quickshell.screens[i].name !== root.claudeName) return Quickshell.screens[i]; return Quickshell.screens.length ? Quickshell.screens[0] : null; }
    function claudeScreen() { for (let i = 0; i < Quickshell.screens.length; i++) if (Quickshell.screens[i].name === root.claudeName) return Quickshell.screens[i]; return null; }
    property bool claudeOpen: false

    ServerDash {
        id: dash
        screen: root.realScreen()
        settings: settingsApp
        visible: !settingsApp.shown
    }

    // "Claude's screen": a button in the corner of the real screen; pressed, a large window over the dashboard (not full
    // screen: the dashboard shows around it) with what's on the virtual monitor, live. Close (or B / Esc) hides it again.
    PanelWindow {
        id: claudeBtn
        screen: root.realScreen()
        visible: !!root.claudeScreen() && !settingsApp.shown && !root.claudeOpen
        anchors { top: true; right: true }
        margins { top: 18; right: 18 }
        implicitWidth: btnRow.implicitWidth + 28
        implicitHeight: 40
        color: "transparent"
        exclusiveZone: 0
        WlrLayershell.layer: WlrLayer.Top
        WlrLayershell.namespace: "hearth-server-claude-button"
        Rectangle {
            anchors.fill: parent
            radius: 20
            color: Qt.rgba(0.12, 0.09, 0.07, 0.92)
            border.color: Theme.accent
            border.width: 1
            Row {
                id: btnRow
                anchors.centerIn: parent
                spacing: 8
                Rectangle { width: 8; height: 8; radius: 4; anchors.verticalCenter: parent.verticalCenter; color: Theme.accent }
                Text { text: "Claude's screen"; color: Theme.text; font.pixelSize: 15; font.weight: Font.DemiBold }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.claudeOpen = true }
        }
    }
    PanelWindow {
        id: claudeView
        screen: root.realScreen()
        visible: root.claudeOpen && !!root.claudeScreen()
        anchors { top: true; bottom: true; left: true; right: true }
        exclusiveZone: -1
        color: Qt.rgba(0, 0, 0, 0.45)
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-server-claude-view"
        WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        MouseArea { anchors.fill: parent; onClicked: root.claudeOpen = false }
        Rectangle {
            id: frame
            anchors.centerIn: parent
            width: Math.round(parent.width * 0.78)
            height: Math.round(parent.height * 0.82)
            radius: 16
            color: Theme.bg
            border.color: Theme.accent
            border.width: 2
            focus: true
            Keys.onEscapePressed: root.claudeOpen = false
            MouseArea { anchors.fill: parent }   // clicks on the window don't close it
            Row {
                id: head
                anchors { top: parent.top; left: parent.left; topMargin: 14; leftMargin: 20 }
                spacing: 10
                Rectangle { width: 10; height: 10; radius: 5; anchors.verticalCenter: parent.verticalCenter; color: Theme.accent }
                Text { text: "Claude's screen"; color: Theme.text; font.pixelSize: 19; font.weight: Font.DemiBold }
                Text { text: "what's open on the virtual monitor, live"; color: Theme.muted; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter }
            }
            Rectangle {
                anchors { top: parent.top; right: parent.right; topMargin: 10; rightMargin: 14 }
                width: closeTxt.implicitWidth + 28; height: 34; radius: 17
                color: Qt.rgba(1, 1, 1, 0.08)
                Text { id: closeTxt; anchors.centerIn: parent; text: "Close  (B)"; color: Theme.text; font.pixelSize: 14 }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.claudeOpen = false }
            }
            ScreencopyView {
                anchors { top: head.bottom; left: parent.left; right: parent.right; bottom: parent.bottom; margins: 16; topMargin: 12 }
                captureSource: root.claudeOpen ? root.claudeScreen() : null
                live: root.claudeOpen
                paintCursor: true
            }
        }
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
                    else if (m.t === "help") { if (!settingsApp.shown) dash.pad("view"); }   // View tapped: X-ray in Ithil's brain (1.13)
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
        function pick(n: int): void { dash.sel = n; }
        function back(): void { dash.backInfo = { back: true, played: 9660 }; dash.overlay = "back"; }
        function close(): void { dash.overlay = ""; dash.cancelSwitch(); }
        // Claude's screen (the virtual monitor) shown over the dashboard, or hidden again
        function claude(on: bool): void { root.claudeOpen = on; }
        function cancel(): void { dash.remoteCancel(); }
        // An opened page's buttons, for the screenshot test (its keys don't reach the dashboard there): "new" a new rule,
        // "window" Ithil's next time window
        function press(b: string): void {
            if (b === "new" && dash.rulesDet) dash.rulesDet.newRule();
            else if (b === "window" && dash.ithilView) dash.ithilView.cycle(1);
        }
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
