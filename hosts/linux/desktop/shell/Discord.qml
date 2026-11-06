// Discord in Desktop Mode (docs/DISCORD.md): the bar's Discord button. Since 1.6 Discord is how Hearth does friends
// (Hearth Friends on Matrix is archived: archive/hosts/linux/desktop/shell/Friends.qml).
//
// The first press asks which Discord he wants (three cards: the official app, Discord in a web window, Dorion). After
// that the button toggles a small Discord window by the bar (above a bottom bar, below a top one, beside a side one;
// about 420×640, resizable, its size kept as state by hearth-discord). It's Discord's own window: replying to
// friends works as in Discord. Pressing the button again (or `quickshell ipc call discord toggle`) hides it by
// minimizing, never quitting, so Discord's notifications keep coming (Mantel's notification server shows them).
// "Open full Discord" (`discord full`) makes it a normal maximized window.
// Hearth only starts, places, shows and hides the window (desktop/shell/hearth-discord). It never reads, scripts or
// injects into it, and never touches a token or cookie.
// Controller: the chooser takes Left/Right, A and B. Discord's window is an ordinary app window: the controller mouse
// and A to click, Guide + X for the on-screen keyboard.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: dc
    property var st: null                 // hearth-discord status
    property bool chooserShown: false
    property int pick: 0                  // the card in front
    property string step: "choose"        // choose | install | busy
    property string busyText: ""
    property string pendingClient: ""
    property real lastAnchor: -1
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-discord").toString().replace(/^file:\/\//, ""))
    readonly property var cards: [
        { id: "official", title: "Discord", tag: "Recommended", body: "The official Discord app: everything Discord has. Installed from Flathub when you press Install." },
        { id: "web", title: "Discord in a web window", tag: "Official · nothing to install", body: "Discord's own web app (discord.com) in a small Hearth Browser window. You sign in on Discord's page." },
        { id: "dorion", title: "Dorion", tag: "Lighter · unofficial", body: "Uses less memory. It's unofficial: Discord's rules don't allow third-party apps, so there's a small risk to your account." }
    ]
    readonly property var classesAll: ({ official: ["discord", "com.discordapp.discord"], web: ["hearthdiscordweb"], dorion: ["dorion"] })

    Component.onCompleted: { SettingsNav.discord = dc; refresh(); }
    onChooserShownChanged: SettingsNav.discordShown = chooserShown

    IpcHandler {
        target: "discord"
        function toggle(): void { dc.toggle(-1); }
        function open(): void { dc.show(-1); }
        function hide(): void { dc.hide(); }
        function full(): void { dc.full(); }
        function chooser(): void { dc.openChooser(); }
        function close(): void { dc.chooserShown = false; }
    }
    // Older keybinds and links to Friends (1.5) open Discord.
    IpcHandler {
        target: "friends"
        function toggle(): void { dc.toggle(-1); }
        function open(): void { dc.show(-1); }
        function close(): void { dc.hide(); }
    }

    // ------------------------------------------------ the helper
    Process {
        id: statusProc
        property var then: null
        command: ["python3", dc.helper, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { dc.st = JSON.parse(text.trim().split("\n").pop()); } catch (e) { }
                const t = statusProc.then; statusProc.then = null;
                if (t) t();
            }
        }
    }
    function refresh(then) { statusProc.then = then || null; if (!statusProc.running) statusProc.running = true; }
    function run(args) { Quickshell.execDetached(["python3", helper].concat(args)); }
    Process {
        id: chooseProc
        property var then: null
        stdout: StdioCollector { onStreamFinished: { try { dc.st = JSON.parse(text.trim().split("\n").pop()); } catch (e) { } const t = chooseProc.then; chooseProc.then = null; if (t) t(); } }
    }
    Process {
        id: installProc
        property string client: ""
        property string err: ""
        stdout: SplitParser {
            onRead: line => {
                if (line.startsWith("progress: ")) { const m = line.slice(10); dc.busyText = m.replace(/^\d+ /, ""); }
                else if (line.startsWith("error: ")) installProc.err = line.slice(7);
            }
        }
        onExited: code => {
            if (code === 0) { dc.chooserShown = false; dc.refresh(() => dc.show(dc.lastAnchor)); }
            else { dc.step = "install"; SettingsNav.toast(installProc.err || "That didn't install"); }
        }
    }

    // ------------------------------------------------ the window
    function classesFor(client) { return (st && st.windowClasses && st.windowClasses[client]) || classesAll[client] || []; }
    function findWindow() {
        const want = st && st.opens ? classesFor(st.opens) : [].concat(classesAll.official, classesAll.web, classesAll.dorion);
        const all = ToplevelManager.toplevels.values;
        for (let i = all.length - 1; i >= 0; i--) {
            const id = (all[i].appId || "").toLowerCase();
            if (want.some(c => id === c || (c === "dorion" && id.indexOf("dorion") >= 0))) return all[i];
        }
        return null;
    }
    // What the button does (one pure function, tested in test_hearth_discord.py).
    function decide(chosen, opens, haveWindow, inFront) {
        if (!chosen) return "choose";
        if (!opens) return "install";
        if (!haveWindow) return "start";
        if (inFront) return "hide";
        return "show";
    }
    function edgeArgs(anchor) {
        const s = Quickshell.screens.length ? Quickshell.screens[0] : null;
        const edge = DeskState.classicBar ? "bottom" : Theme.barEdge;
        const bar = edge === "top" ? DeskState.topInset : edge === "left" ? DeskState.leftInset : edge === "right" ? DeskState.rightInset : DeskState.bottomInset;
        const scr = s ? (s.width + "x" + s.height + "+" + (s.x || 0) + "+" + (s.y || 0)) : "1920x1080+0+0";
        return [edge, scr, String(bar), String(anchor === undefined ? -1 : anchor)];
    }
    function toggle(anchor) {
        lastAnchor = anchor === undefined ? -1 : anchor;
        if (chooserShown) { chooserShown = false; return; }
        refresh(() => {
            const w = findWindow();
            const inFront = !!w && !w.minimized && ToplevelManager.activeToplevel === w;
            act(decide(st && st.chosen, st && st.opens, !!w, inFront), w);
        });
    }
    function act(what, w) {
        switch (what) {
        case "choose": openChooser(); break;
        case "install": openChooser(); if (st && st.client) { pick = Math.max(0, cards.findIndex(c => c.id === st.client)); step = "install"; pendingClient = st.client; } break;
        case "start": run(["open", "--quick"].concat(edgeArgs(lastAnchor))); break;
        case "hide": hideWindow(w); break;
        case "show": showWindow(w); break;
        }
    }
    function showWindow(w) {
        w.minimized = false;
        w.activate();
        if (!w.maximized) run(["place"].concat(edgeArgs(lastAnchor)));
    }
    // Hidden, never quit: Discord keeps running (and notifying). Its size is kept for next time.
    function hideWindow(w) {
        if (!w.maximized) run(["remember"]);
        w.minimized = true;
    }
    function show(anchor) { refresh(() => { const w = findWindow(); if (w) showWindow(w); else act(decide(st && st.chosen, st && st.opens, false, false), null); }); }
    function hide() { const w = findWindow(); if (w) hideWindow(w); }
    function full() {
        refresh(() => {
            const w = findWindow();
            if (w) { w.minimized = false; w.maximized = true; w.activate(); }
            else if (st && st.opens) run(["open", "--full"]);
            else openChooser();
        });
    }

    // ------------------------------------------------ the chooser (first press, or Settings › Discord)
    function openChooser() {
        step = "choose";
        const cur = st && st.client ? cards.findIndex(c => c.id === st.client) : 0;
        pick = Math.max(0, cur);
        chooserShown = true;
    }
    function choose(i) {
        const c = cards[i];
        pendingClient = c.id;
        chooseProc.then = () => {
            const have = st && st.clients ? st.clients[c.id] : false;
            if (have) { chooserShown = false; show(lastAnchor); }
            else step = "install";
        };
        chooseProc.command = ["python3", helper, "choose", c.id];
        chooseProc.running = true;
    }
    function install() {
        if (installProc.running) return;
        step = "busy";
        busyText = "Starting…";
        installProc.err = "";
        installProc.client = pendingClient;
        installProc.command = ["python3", helper, "install", pendingClient];
        installProc.running = true;
    }

    PanelWindow {
        id: chooserWin
        visible: dc.chooserShown
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-discord"
        WlrLayershell.keyboardFocus: dc.chooserShown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "#b0080b10"
        onVisibleChanged: if (visible) keys.forceActiveFocus()
        MouseArea { anchors.fill: parent; onClicked: if (dc.step !== "busy") dc.chooserShown = false }

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 32, 980)
            height: col.implicitHeight + 48
            radius: 20
            color: Theme.panel
            border.color: Theme.line
            MouseArea { anchors.fill: parent }

            Item {
                id: keys
                anchors.fill: parent
                focus: true
                Keys.onPressed: e => {
                    if (dc.step === "choose") {
                        switch (e.key) {
                        case Qt.Key_Left: case Qt.Key_Up: dc.pick = Math.max(0, dc.pick - 1); break;
                        case Qt.Key_Right: case Qt.Key_Down: case Qt.Key_Tab: dc.pick = Math.min(dc.cards.length - 1, dc.pick + 1); break;
                        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: dc.choose(dc.pick); break;
                        case Qt.Key_Escape: dc.chooserShown = false; break;
                        default: return;
                        }
                    } else if (dc.step === "install") {
                        switch (e.key) {
                        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: dc.install(); break;
                        case Qt.Key_Escape: dc.step = "choose"; break;
                        default: return;
                        }
                    } else if (e.key !== Qt.Key_Escape) return;
                    e.accepted = true;
                }
            }

            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 24 }
                spacing: 14
                RowLayout {
                    Layout.fillWidth: true
                    Icon { name: "chat"; size: 22 }
                    Text { Layout.fillWidth: true; text: dc.step === "choose" ? "Which Discord do you want?" : dc.cards[dc.pick].title; color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold; elide: Text.ElideRight }
                    Text { text: dc.step === "choose" ? "◀ ▶ pick  Ⓐ choose  Ⓑ close" : dc.step === "install" ? "Ⓐ install  Ⓑ back" : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
                Text {
                    visible: dc.step === "choose"
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: "The bar's Discord button opens it in a small window you can reply to friends from. You can change this later in Settings › Discord."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
                GridLayout {
                    visible: dc.step === "choose"
                    Layout.fillWidth: true
                    columns: chooserWin.width < 760 ? 1 : 3
                    columnSpacing: 12; rowSpacing: 12
                    Repeater {
                        model: dc.cards
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            Layout.preferredHeight: cardCol.implicitHeight + 28
                            radius: 14
                            color: dc.pick === index ? Theme.panel3 : Theme.panel2
                            border.width: dc.pick === index ? 2 : 1
                            border.color: dc.pick === index ? Theme.accent : Theme.line
                            ColumnLayout {
                                id: cardCol
                                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
                                spacing: 6
                                Text { text: modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.bold: true }
                                Text { text: modelData.tag; color: modelData.id === "dorion" ? Theme.danger : Theme.accent; font.family: Theme.font; font.pixelSize: 12; font.bold: true }
                                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: modelData.body; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; lineHeight: 1.2 }
                                Text {
                                    visible: !!(dc.st && dc.st.clients && dc.st.clients[modelData.id])
                                    text: dc.st && dc.st.client === modelData.id ? "Your pick · installed" : "Installed"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                }
                            }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { dc.pick = index; dc.choose(index); } }
                        }
                    }
                }
                // Install what he picked, only when he presses Install.
                ColumnLayout {
                    visible: dc.step !== "choose"
                    Layout.fillWidth: true
                    spacing: 10
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: dc.step === "busy" ? dc.busyText
                            : dc.pendingClient === "dorion" ? "Install Dorion from its official GitHub release? Hearth checks the download against the checksum GitHub publishes and puts it in your home folder. Dorion is unofficial: Discord's rules don't allow third-party apps, so there's a small risk to your account."
                            : "Install Discord from Flathub? It's the official app; it comes from Flathub, not from Hearth."
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 14
                    }
                    RowLayout {
                        visible: dc.step === "install"
                        spacing: 8
                        Rectangle {
                            implicitWidth: it.implicitWidth + 32; implicitHeight: 42; radius: 12; color: Theme.accent
                            Text { id: it; anchors.centerIn: parent; text: "Install"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.bold: true }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: dc.install() }
                        }
                        Rectangle {
                            implicitWidth: nt.implicitWidth + 32; implicitHeight: 42; radius: 12; color: Theme.panel2
                            Text { id: nt; anchors.centerIn: parent; text: "Not now"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: dc.step = "choose" }
                        }
                    }
                }
            }
        }
    }
}
