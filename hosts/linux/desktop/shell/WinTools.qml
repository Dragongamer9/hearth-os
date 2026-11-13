// Windows' tools, the Hearth way (docs/DEVICE-MANAGER.md): Device Manager, the Run box (Super+R) and the Super+X
// menu (also a right-click on the Start button). shell.qml loads this file by name with a Loader, so a problem here
// only loses these three, never the desktop; each window's page is loaded by name again (DeviceManagerView.qml,
// RunView.qml), like Task Manager's.
// Nothing here runs until it's opened: no service, no timer. Opening writes nothing; the Run box keeps its recent
// entries (state, not a setting) in ~/.local/state/nimbus/run-history.json through SettingsWriter, only after
// something typed was opened.
// The commands and the menu's entries are one data file, win-commands.json (run-map.js reads it). Entries whose app
// isn't installed (Hearth Disks, System Restore, Reliability history…) are hidden.
// Mouse, touch, keyboard and the controller (hearth-pad types arrows / Enter (A) / Esc (B) / Menu (X) while one is in
// front: Pad.qml knows the "Device Manager" and "Run" windows and SettingsNav.winxShown).
//   quickshell ipc -p <shell folder> call devices open|close|toggle|scan|select <name>|state
//   quickshell ipc -p <shell folder> call run open|close|toggle|type <text>|go|state
//   quickshell ipc -p <shell folder> call winx open|close|toggle|key <letter>|state
import QtQuick
import QtQuick.Window
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "run-map.js" as RunMap
import "drive-paths.js" as DrivePaths

Scope {
    id: wt
    // The shell's own apps, handed over by shell.qml (wire): Task Manager, Files, Settings, the launcher, the lock.
    property var tasks: null
    property var files: null
    property var settings: null
    property var launcher: null
    property var lockScreen: null
    function wire(o) {
        tasks = o.tasks || null; files = o.files || null; settings = o.settings || null;
        launcher = o.launcher || null; lockScreen = o.lockScreen || null;
    }
    readonly property string shellDir: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")).replace(/\/$/, "")
    readonly property string openSh: shellDir.replace(/\/shell$/, "") + "/open.sh"
    readonly property string home: Quickshell.env("HOME") || "/"
    function say(t) { SettingsNav.toast(t); }

    // ------------------------------------------------ win-commands.json (read only)
    property var data: ({ commands: [], vars: {}, msSettings: {}, winx: [] })
    property bool dataOk: false
    FileView {
        id: dataFile
        path: wt.shellDir + "/win-commands.json"
        blockLoading: true
        printErrors: false
        onLoaded: {
            try { wt.data = JSON.parse(text()); wt.dataOk = true; }
            catch (e) { console.warn("WinTools: win-commands.json couldn't be read: " + e); }
        }
    }

    // ------------------------------------------------ which apps are installed (asked when something opens; never kept)
    property var have: ({})
    function installed(name) { return have[name] === true; }
    Process {
        id: haveProc
        stdout: StdioCollector {
            onStreamFinished: {
                const h = {};
                for (const l of text.split("\n")) if (l.trim()) h[l.trim()] = true;
                wt.have = h;
            }
        }
    }
    function checkApps() {
        if (haveProc.running) return;
        haveProc.command = ["sh", "-c", 'for c; do command -v "$c" >/dev/null 2>&1 && echo "$c"; done', "sh"].concat(RunMap.needed(data));
        haveProc.running = true;
    }

    // ------------------------------------------------ This PC's drive letters, for C:\ and D:\ (read only)
    property var letterCtx: ({ home: wt.home, user: Quickshell.env("USER") || wt.home.split("/").pop(), drives: [{ letter: "C", path: "/" }] })
    Process {
        id: lettersProc
        command: ["python3", wt.shellDir + "/hearth_drive_letters.py", "show"]
        stdout: StdioCollector {
            onStreamFinished: {
                let d = null;
                try { d = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { d = null; }
                const drives = [{ letter: "C", path: "/" }];
                const l = d && d.letters ? d.letters : {};
                for (const k in l) if (l[k] && l[k].letter && l[k].path) drives.push({ letter: l[k].letter, path: l[k].path, label: l[k].name || "" });
                wt.letterCtx = { home: wt.home, user: wt.letterCtx.user, drives: drives };
            }
        }
    }
    function fromWindows(t) { return DrivePaths.fromWindows(t, letterCtx); }

    // ------------------------------------------------ doing what an entry or a command says (Run, Super+X)
    // d: a "do" object from win-commands.json or what RunMap.resolve gave. False when it couldn't be done.
    function act(d) {
        if (!d) return false;
        switch (d.kind) {
        case "tasks": if (tasks) tasks.open(d.page || ""); return !!tasks;
        case "devices": openDevices(); return true;
        case "settings": if (settings) settings.open(d.page || ""); return !!settings;
        case "files": if (files) files.open(d.path || ""); return !!files;
        case "exec": Quickshell.execDetached(d.argv); return true;
        case "terminal": Quickshell.execDetached(["sh", openSh, "terminal"]); return true;
        case "screenshot": Quickshell.execDetached(["sh", openSh, "screenshot-region"]); return true;
        case "keyboard": if (SettingsNav.pad) SettingsNav.pad.openKeyboard(); return !!SettingsNav.pad;
        case "run": openRun(); return true;
        case "search": if (launcher) launcher.openMenuSearch(); return !!launcher;
        case "desktop": if (DeskState.mainBar) DeskState.mainBar.toggleShowDesktop(); return !!DeskState.mainBar;
        case "url": Quickshell.execDetached(["xdg-open", d.url]); return true;
        case "app": {
            const e = launcher ? launcher.apps.find(a => a.id === d.id) : null;
            if (!e) return false;
            launcher.launch(e);
            return true;
        }
        case "path": openPath(d.path); return true;
        }
        return false;
    }
    // A folder opens in Files; a file opens in its app; smb:// in Files.
    Process {
        id: pathProc
        property string target: ""
        onExited: code => {
            if (code === 10 || /^smb:\/\//.test(target)) { if (wt.files) wt.files.open(target); }
            else if (code === 11) Quickshell.execDetached(["xdg-open", target]);
            else wt.say("Hearth can't find " + target);
        }
    }
    function openPath(p) {
        if (/^smb:\/\//.test(p)) { if (files) files.open(p); return; }
        pathProc.target = p;
        pathProc.command = ["sh", "-c", 'if [ -d "$1" ]; then exit 10; elif [ -e "$1" ]; then exit 11; else exit 12; fi', "sh", p];
        pathProc.running = true;
    }

    // ================================================ the Run box
    property bool runShown: false
    property var runView: null
    readonly property string historyPath: (Quickshell.env("XDG_STATE_HOME") || (home + "/.local/state")) + "/nimbus/run-history.json"
    property var history: []
    SettingsWriter { id: historyWriter; path: wt.historyPath }
    FileView {
        id: historyFile
        path: wt.runShown ? wt.historyPath : ""
        blockLoading: true
        printErrors: false
        onLoaded: {
            let d = null;
            try { d = JSON.parse(text()); } catch (e) { d = null; }
            if (d && Array.isArray(d.recent)) { if (!historyWriter.owns("recent")) wt.history = d.recent.filter(x => typeof x === "string").slice(0, 12); }
            else if (text().trim() !== "") historyWriter.repair();   // damaged: put back from .bak, never written over
        }
    }
    // Something typed was opened: it goes to the top of the recent list (the only write the Run box makes).
    function remember(entry) {
        const next = RunMap.remember(history, entry, 12);
        if (JSON.stringify(next) === JSON.stringify(history)) return;
        history = next;
        historyWriter.merge({ recent: next });
    }
    function openRun() {
        winxShown = false;
        checkApps();
        if (!lettersProc.running) lettersProc.running = true;
        if (runShown && runView && runView.item && runView.item.focusField) { runView.item.focusField(); return; }
        runShown = true;
    }
    function closeRun() { runShown = false; }

    LazyLoader {
        active: wt.runShown
        FloatingWindow {
            id: runWin
            title: "Run"
            implicitWidth: 600
            implicitHeight: 420
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) wt.runShown = false
            Component.onCompleted: wt.runView = runLoader
            Component.onDestruction: if (wt.runView === runLoader) wt.runView = null
            Loader {
                id: runLoader
                anchors.fill: parent
                focus: true
                source: "RunView.qml"
                onStatusChanged: if (status === Loader.Error) console.warn("Run: its page (RunView.qml) couldn't load")
                onLoaded: if (item && item.focusField) item.focusField()
            }
            Text {
                anchors.centerIn: parent
                visible: runLoader.status === Loader.Error
                width: parent.width - 60
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: "Run couldn't load. Send logs from Settings › Updates & recovery so it can be fixed."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }

    // ================================================ Device Manager
    property bool devShown: false
    property var devView: null
    function openDevices() {
        winxShown = false;
        checkApps();
        devShown = true;
    }
    function closeDevices() { devShown = false; }

    LazyLoader {
        active: wt.devShown
        FloatingWindow {
            id: devWin
            title: "Device Manager"
            implicitWidth: 1100
            implicitHeight: 720
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) wt.devShown = false
            Component.onCompleted: wt.devView = devLoader
            Component.onDestruction: if (wt.devView === devLoader) wt.devView = null
            Loader {
                id: devLoader
                anchors.fill: parent
                focus: true
                source: "DeviceManagerView.qml"
                onStatusChanged: if (status === Loader.Error) console.warn("Device Manager: its page (DeviceManagerView.qml) couldn't load")
                onLoaded: if (item && item.start) item.start()
            }
            Text {
                anchors.centerIn: parent
                visible: devLoader.status === Loader.Error
                width: parent.width - 80
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: "Device Manager couldn't load its page. Send logs from Settings › Updates & recovery so it can be fixed."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }

    // ================================================ the Super+X menu
    property bool winxShown: false
    Binding { target: SettingsNav; property: "winxShown"; value: wt.winxShown }
    property var winxScreen: null
    property real winxX: -1
    property real winxY: -1
    property string winxEdge: ""
    property int winxSel: 0
    property bool winxSub: false          // the Shut down or sign out list is open
    property int winxSubSel: 0
    readonly property var winxList: RunMap.winxItems(data, installed)
    readonly property var powerList: [
        { label: "Sign out", key: "I", cmd: ["nimbus-mode", "logout"] },
        { label: "Lock", key: "L", lock: true },
        { label: "Sleep", key: "S", cmd: ["python3", Buttons.helper, "sleep", "Desktop Mode: Super+X menu"] },
        { label: "Shut down", key: "U", cmd: ["systemctl", "poweroff"] },
        { label: "Restart", key: "R", cmd: ["systemctl", "reboot"] },
        { label: "Game Mode", key: "G", cmd: ["nimbus-mode", "game"] }
    ]
    // From Super+X: the corner the Start button is in (Settings › Personalization › Bar and dock).
    function openWinx() {
        winxScreen = Quickshell.screens[0];
        winxEdge = Theme.dockFloats ? "bottom" : Theme.barEdge;
        winxX = -1; winxY = -1;
        showWinx();
    }
    // From a right-click on the Start button: next to that button (item: the button, edge: the bar's edge).
    function winxFrom(scr, item, edge) {
        winxScreen = scr || Quickshell.screens[0];
        winxEdge = edge || "bottom";
        winxX = -1; winxY = -1;
        if (item) {
            const p = item.mapToItem(null, 0, 0);
            const ww = item.Window.width, wh = item.Window.height;
            const sw = winxScreen ? winxScreen.width : 1920, sh = winxScreen ? winxScreen.height : 1080;
            if (winxEdge === "top") { winxX = p.x; winxY = wh + 6; }
            else if (winxEdge === "left") { winxX = ww + 6; winxY = p.y; }
            else if (winxEdge === "right") { winxX = sw - ww - 6 - 300; winxY = p.y; }
            else { winxX = p.x; winxY = -(wh + 6); }      // above the bar: placed from the bottom
            if (!(sh > 0)) { winxX = -1; winxY = -1; }
        }
        showWinx();
    }
    function showWinx() {
        checkApps();
        winxSel = 0;
        winxSub = false;
        winxShown = true;
    }
    function toggleWinx() { if (winxShown) winxShown = false; else openWinx(); }
    function winxPick(i) {
        const it = winxList[i];
        if (!it || it.sep) return;
        if (it.do && it.do.kind === "power") { winxSel = i; winxSub = true; winxSubSel = 0; return; }
        winxShown = false;
        const d = it.do;
        Qt.callLater(() => wt.act(d));
    }
    function powerPick(i) {
        const it = powerList[i];
        if (!it) return;
        winxShown = false;
        if (it.lock) { if (lockScreen) lockScreen.lock(); return; }
        Quickshell.execDetached(it.cmd);
    }
    function winxKey(letter) {
        const k = String(letter || "").toUpperCase();
        if (winxSub) { const j = powerList.findIndex(x => x.key === k); if (j >= 0) powerPick(j); return j >= 0; }
        const i = winxList.findIndex(x => !x.sep && x.key === k);
        if (i >= 0) winxPick(i);
        return i >= 0;
    }
    function winxMove(dir) {
        if (winxSub) { winxSubSel = (winxSubSel + dir + powerList.length) % powerList.length; return; }
        const n = winxList.length;
        if (!n) return;
        let i = winxSel;
        for (let k = 0; k < n; k++) { i = (i + dir + n) % n; if (!winxList[i].sep) { winxSel = i; return; } }
    }
    // An entry's label with its letter underlined (the first one that matches, as Windows does).
    function underlined(label, key) {
        const i = label.toUpperCase().indexOf(String(key || "").toUpperCase());
        const esc = s => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
        return i < 0 ? esc(label) : esc(label.slice(0, i)) + "<u>" + esc(label.charAt(i)) + "</u>" + esc(label.slice(i + 1));
    }

    LazyLoader {
        active: wt.winxShown
        PanelWindow {
            id: winxWin
            screen: wt.winxScreen
            visible: true
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-winx"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                onClicked: wt.winxShown = false
            }
            Item {
                id: winxKeys
                anchors.fill: parent
                focus: true
                Component.onCompleted: forceActiveFocus()
                Keys.onPressed: event => {
                    switch (event.key) {
                    case Qt.Key_Up: wt.winxMove(-1); break;
                    case Qt.Key_Down: case Qt.Key_Tab: wt.winxMove(1); break;
                    case Qt.Key_Right: if (!wt.winxSub && wt.winxList[wt.winxSel] && wt.winxList[wt.winxSel].do.kind === "power") wt.winxPick(wt.winxSel); break;
                    case Qt.Key_Left: case Qt.Key_Escape: case Qt.Key_Backspace:
                        if (wt.winxSub) wt.winxSub = false; else wt.winxShown = false;
                        break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                        if (wt.winxSub) wt.powerPick(wt.winxSubSel); else wt.winxPick(wt.winxSel);
                        break;
                    case Qt.Key_Menu: case Qt.Key_Super_L: case Qt.Key_Super_R: wt.winxShown = false; break;
                    default:
                        if (!(event.text && /^[a-z]$/i.test(event.text) && wt.winxKey(event.text))) return;
                    }
                    event.accepted = true;
                }
            }

            Rectangle {
                id: card
                readonly property real sw: winxWin.width > 0 ? winxWin.width : 1920
                readonly property real sh: winxWin.height > 0 ? winxWin.height : 1080
                width: 300
                height: winxCol.implicitHeight + 12
                x: Math.max(8, Math.min(sw - width - 8, wt.winxX >= 0 ? wt.winxX : (wt.winxEdge === "right" ? sw - width - 72 : wt.winxEdge === "left" ? 72 : 12)))
                y: Math.max(8, Math.min(sh - height - 8,
                    wt.winxY > 0 ? wt.winxY
                    : wt.winxY < -1 ? sh + wt.winxY - height
                    : wt.winxEdge === "top" ? 60 : wt.winxEdge === "bottom" ? sh - height - 60 : sh - height - 12))
                radius: Theme.r(10)
                color: Theme.panel2
                border.color: Theme.line
                border.width: 1
                MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }   // a click inside doesn't close it
                Column {
                    id: winxCol
                    x: 6; y: 6
                    width: parent.width - 12
                    spacing: 1
                    Repeater {
                        model: wt.winxList
                        delegate: Item {
                            id: row
                            required property var modelData
                            required property int index
                            width: winxCol.width
                            height: modelData.sep ? 9 : 32
                            Rectangle { visible: row.modelData.sep === true; anchors.centerIn: parent; width: parent.width - 16; height: 1; color: Theme.line }
                            Rectangle {
                                visible: !row.modelData.sep
                                anchors.fill: parent
                                radius: Theme.r(6)
                                readonly property bool lit: wt.winxSel === row.index && (!wt.winxSub || row.modelData.do.kind === "power")
                                color: lit ? Theme.panel3 : ma.containsMouse ? Qt.rgba(Theme.panel3.r, Theme.panel3.g, Theme.panel3.b, 0.6) : "transparent"
                                border.width: lit ? 1 : 0
                                border.color: Theme.accent
                                Text {
                                    anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                                    text: row.modelData.sep ? "" : wt.underlined(row.modelData.label, row.modelData.key)
                                    textFormat: Text.StyledText
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                                }
                                Text {
                                    anchors { right: parent.right; rightMargin: row.modelData.do && row.modelData.do.kind === "power" ? 26 : 10; verticalCenter: parent.verticalCenter }
                                    text: row.modelData.key || ""
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                                }
                                Text {
                                    visible: !!row.modelData.do && row.modelData.do.kind === "power"
                                    anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                                    text: "›"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15
                                }
                                MouseArea {
                                    id: ma
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onEntered: if (!wt.winxSub) wt.winxSel = row.index
                                    onClicked: wt.winxPick(row.index)
                                }
                            }
                        }
                    }
                }
            }
            // Shut down or sign out ›: beside the menu (above it when there's no room on the side).
            Rectangle {
                id: sub
                visible: wt.winxSub
                width: 220
                height: subCol.implicitHeight + 12
                x: card.x + card.width + 6 + width <= card.sw - 8 ? card.x + card.width + 6 : Math.max(8, card.x - width - 6)
                y: Math.max(8, Math.min(card.sh - height - 8, card.y + card.height - height))
                radius: Theme.r(10)
                color: Theme.panel2
                border.color: Theme.line
                border.width: 1
                MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
                Column {
                    id: subCol
                    x: 6; y: 6
                    width: parent.width - 12
                    spacing: 1
                    Repeater {
                        model: wt.powerList
                        delegate: Rectangle {
                            id: prow
                            required property var modelData
                            required property int index
                            width: subCol.width
                            height: 32
                            radius: Theme.r(6)
                            readonly property bool lit: wt.winxSubSel === index
                            color: lit ? Theme.panel3 : pma.containsMouse ? Qt.rgba(Theme.panel3.r, Theme.panel3.g, Theme.panel3.b, 0.6) : "transparent"
                            border.width: lit ? 1 : 0
                            border.color: Theme.accent
                            Text {
                                anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                                text: wt.underlined(prow.modelData.label, prow.modelData.key)
                                textFormat: Text.StyledText
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                            }
                            Text {
                                anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                                text: prow.modelData.key
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                            }
                            MouseArea {
                                id: pma
                                anchors.fill: parent
                                hoverEnabled: true
                                onEntered: wt.winxSubSel = prow.index
                                onClicked: wt.powerPick(prow.index)
                            }
                        }
                    }
                }
            }
        }
    }

    Component.onCompleted: SettingsNav.winTools = wt
    Component.onDestruction: if (SettingsNav.winTools === wt) SettingsNav.winTools = null

    // ================================================ IPC (key bindings in labwc/rc.xml, checks in desktop-test.yml)
    IpcHandler {
        target: "devices"
        function open(): void { wt.openDevices(); }
        function close(): void { wt.closeDevices(); }
        function toggle(): void { if (wt.devShown) wt.closeDevices(); else wt.openDevices(); }
        function scan(): void { if (wt.devView && wt.devView.item) wt.devView.item.scan(); }
        function select(name: string): void { if (wt.devView && wt.devView.item) wt.devView.item.selectByName(name); }
        function state(): string { return wt.devView && wt.devView.item ? wt.devView.item.stateJson() : "{}"; }
    }
    IpcHandler {
        target: "run"
        function open(): void { wt.openRun(); }
        function close(): void { wt.closeRun(); }
        function toggle(): void { if (wt.runShown) wt.closeRun(); else wt.openRun(); }
        function type(text: string): void { if (wt.runView && wt.runView.item) wt.runView.item.setText(text); }
        function go(): void { if (wt.runView && wt.runView.item) wt.runView.item.go(); }
        function state(): string { return wt.runView && wt.runView.item ? wt.runView.item.stateJson() : "{}"; }
    }
    IpcHandler {
        target: "winx"
        function open(): void { wt.openWinx(); }
        function close(): void { wt.winxShown = false; }
        function toggle(): void { wt.toggleWinx(); }
        function key(letter: string): void { if (wt.winxShown) wt.winxKey(letter); }
        function state(): string { return JSON.stringify({ shown: wt.winxShown, sub: wt.winxSub, items: wt.winxList.filter(x => !x.sep).map(x => x.label) }); }
    }
}
