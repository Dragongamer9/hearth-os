// The Hearth taskbar: launcher button, virtual desktops, pinned and open apps, then tray, volume, network,
// battery, Quick Access, keyboard, notifications, clock and a Game Mode button. A floating rounded bar along
// the bottom of each screen (down the left side when the screen is upright); windows don't cover it.
// Apps: pinned ones stay (DeskState, ~/.config/nimbus/desk.json); windows of the same app share a button.
// Click focuses (again: minimizes; several windows: a list of them to pick from), middle-click opens another one,
// drag a pinned app along the bar to put it somewhere else, right-click (X on the controller) for the app's own
// actions (New private window...), Pin / Unpin, Move to desktop, Maximize and Close.
// Status icons: click opens Quick Access, right-click a menu (sound output, network or Bluetooth settings...),
// middle-click on the volume mutes. The thin strip at the very end shows the desktop (click again: the windows).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower

PanelWindow {
    id: bar
    signal launcherRequested()
    signal quickRequested()
    signal keyboardRequested()
    signal calendarRequested()
    signal settingsRequested(string page)
    property var menu: null                // PopMenu, for the right-click menus
    property bool primary: false           // the main screen's bar (Super+1-9, the screenshot tool ask it)
    onPrimaryChanged: if (primary) DeskState.mainBar = bar
    Component.onCompleted: if (primary) DeskState.mainBar = bar

    // Upright screen (a handheld held vertically, a rotated monitor): the bar goes down the left side.
    readonly property bool portrait: screen && screen.height > screen.width
    readonly property int thickness: 60

    anchors { bottom: true; left: true; right: !portrait; top: portrait }
    implicitHeight: portrait ? 0 : thickness
    implicitWidth: portrait ? thickness : 0
    exclusiveZone: thickness
    color: "transparent"
    WlrLayershell.namespace: "hearth-bar"
    WlrLayershell.layer: WlrLayer.Top

    // ------------------------------------------------------------------ apps on the taskbar
    // Hearth's own windows (Settings, Files, Task Manager...) are part of this shell: Quickshell's app id.
    function ownWindow(t) {
        const a = t.appId || "";
        return /quickshell/i.test(a) || (a === "" && /^(Hearth Settings|Task Manager|Files)$/.test(t.title || ""));
    }
    // Pinned apps first (in pin order), then the other open apps; each app's windows share one button.
    readonly property var taskItems: {
        const out = [], byKey = {};
        void DesktopEntries.applications.values.length;   // look again once the app list has loaded
        for (const id of DeskState.pins) {
            const entry = DesktopEntries.byId(id);
            if (!entry || byKey[id]) continue;
            byKey[id] = { key: id, id: id, entry: entry, pinned: true, own: false, windows: [] };
            out.push(byKey[id]);
        }
        for (const t of ToplevelManager.toplevels.values) {
            const own = ownWindow(t);
            const entry = own ? null : AppIcons.entryFor(t.appId);
            const key = own ? "own:" + t.title : entry ? entry.id : "app:" + (t.appId || t.title);
            if (!byKey[key]) {
                byKey[key] = { key: key, id: entry ? entry.id : "", entry: entry, pinned: false, own: own, windows: [] };
                out.push(byKey[key]);
            }
            byKey[key].windows.push(t);
        }
        return out;
    }
    function appName(it) {
        if (it.entry) return it.entry.name;
        const w = it.windows[0];
        return w ? (w.title || w.appId) : "";
    }
    // Super+number and a click on an app with one window: open it, bring it to the front, or (in front) minimize
    // it; several windows: the next one.
    function cycleApp(it) {
        const ws = it.windows;
        if (!ws.length) { if (it.entry) it.entry.execute(); return; }
        const i = ws.findIndex(w => w.activated);
        if (ws.length === 1) { if (i === 0) ws[0].minimized = true; else ws[0].activate(); }
        else ws[(i + 1) % ws.length].activate();       // i = -1: the first
    }
    // A click: several windows show a list to pick from (like Windows' taskbar), one is shown or minimized.
    function focusApp(it, btn) {
        if (it.windows.length > 1 && btn && menu) windowList(btn, it);
        else cycleApp(it);
    }
    function windowList(btn, it) {
        const list = it.windows.slice(0, 12).map(w => ({
            label: w.title || appName(it), icon: "window", checked: w.activated ? true : undefined,
            run: () => { w.activate(); }
        }));
        if (it.entry && !it.own) list.push({ separator: true }, { label: "New window", icon: "plus", run: () => it.entry.execute() });
        const p = menuPoint(btn);
        menu.popup(screen, p.x, p.y, { title: appName(it), above: !portrait }, list);
    }
    // Middle-click: another window of the app (like Windows and KDE); Hearth's own windows have no second one.
    function newInstance(it) {
        if (it.entry && !it.own) it.entry.execute();
        else if (!it.windows.length) cycleApp(it);
    }
    // Super+1-9: the app at that place on the taskbar.
    function activateNth(n) {
        const it = taskItems[n - 1];
        if (it) cycleApp(it);
    }

    // ---- Show desktop (the strip at the end of the bar, like Windows): every window minimized, then back.
    function toggleShowDesktop() {
        const open = ToplevelManager.toplevels.values.filter(t => !t.minimized);
        if (open.length) {
            const front = open.find(t => t.activated) || null;
            DeskState.shownDesktop = { windows: open, front: front };
            open.forEach(t => t.minimized = true);
            return;
        }
        const was = DeskState.shownDesktop;
        DeskState.shownDesktop = null;
        if (!was) return;
        const still = ToplevelManager.toplevels.values;
        was.windows.filter(t => still.indexOf(t) >= 0 && t !== was.front).forEach(t => t.minimized = false);
        if (was.front && still.indexOf(was.front) >= 0) was.front.activate();
    }

    // ---- Dragging pinned apps along the bar (the order is saved with hearth-desk pins-order).
    property int dragFrom: -1              // index in taskItems of the app being dragged
    property int dragTo: -1                // where it would go
    readonly property int pinnedCount: taskItems.filter(t => t.pinned).length
    // Where the dragged app would go among the other pinned ones: after every one whose middle it has passed.
    function dropIndexAt(scenePos) {
        let n = 0;
        for (let i = 0; i < pinnedCount; i++) {
            if (i === dragFrom) continue;
            const it = taskRepeater.itemAt(i);
            if (!it) continue;
            const c = it.mapToItem(null, it.width / 2, it.height / 2);
            if ((portrait ? c.y : c.x) < scenePos) n++;
        }
        return n;
    }
    function finishDrag() {
        if (dragFrom >= 0 && dragTo >= 0 && dragTo !== dragFrom) {
            const order = taskItems.filter(t => t.pinned).map(t => t.id);
            const moved = order.splice(dragFrom, 1)[0];
            order.splice(dragTo, 0, moved);
            DeskState.reorderPins(order.concat(DeskState.pins.filter(id => order.indexOf(id) < 0)));
        }
        dragFrom = -1;
        dragTo = -1;
    }
    // Screen coordinates just outside the bar next to an item, for menus.
    function menuPoint(item) {
        const p = item.mapToItem(null, item.width / 2, item.height / 2);
        if (portrait) return Qt.point(bar.width + 4, p.y - 20);
        return Qt.point(p.x - 134, (screen ? screen.height : 1080) - bar.height + 2);
    }
    // labwc's SendToDesktop through its Super+Alt+number keys (rc.xml), typed with wtype after focusing the window.
    function moveToDesktop(win, n) {
        win.activate();
        mover.command = ["sh", "-c", "sleep 0.25; exec wtype -M logo -M alt -k \"$0\" -m alt -m logo", String(n)];
        mover.running = true;
    }
    Process {
        id: mover
        onExited: code => {
            if (code === 127) SettingsNav.toast("Moving windows needs wtype. The window's title bar menu can move it too");
        }
    }
    // Right-click on a status icon: a short menu of where to go.
    function statusMenu(btn, title, list) {
        if (!menu) return;
        const p = menuPoint(btn);
        menu.popup(screen, p.x, p.y, { title: title, above: !portrait }, list);
    }
    // Right-click on the volume: which speakers or headphones to use, mute, and Sound settings.
    function soundMenu(btn) {
        const sink = Pipewire.defaultAudioSink;
        const outs = Pipewire.nodes ? Pipewire.nodes.values.filter(n => n.audio && n.isSink && !n.isStream) : [];
        const list = outs.slice(0, 8).map(n => ({
            label: n.description || n.nickname || n.name || "Speakers", checked: n === sink ? true : undefined, icon: "vol",
            run: () => { Pipewire.preferredDefaultAudioSink = n; }
        }));
        if (list.length) list.push({ separator: true });
        const a = sink ? sink.audio : null;
        if (a) list.push({ label: a.muted ? "Unmute" : "Mute", icon: a.muted ? "vol" : "vol-mute", run: () => { a.muted = !a.muted; } });
        list.push({ label: "Sound settings", icon: "gear", run: () => bar.settingsRequested("sound") });
        statusMenu(btn, "Sound output", list);
    }
    function taskMenu(btn, it) {
        if (!menu) return;
        const ws = it.windows;
        const win = ws.find(w => w.activated) || ws[0] || null;
        const list = [];
        // The app's own actions from its .desktop file (New private window, Compose message...), like a jump list.
        const acts = it.entry && !it.own && it.entry.actions ? Array.from(it.entry.actions).slice(0, 6) : [];
        for (const a of acts) list.push({ label: a.name, icon: "open", run: () => a.execute() });
        if (acts.length) list.push({ separator: true });
        if (it.entry && !it.own) list.push({ label: ws.length ? "New window" : "Open", icon: "open", run: () => it.entry.execute() });
        if (it.entry && !it.own) {
            const pinned = DeskState.isPinned(it.entry.id);
            list.push({ label: pinned ? "Unpin from taskbar" : "Pin to taskbar", icon: pinned ? "unpin" : "pin", run: () => DeskState.togglePin(it.entry.id) });
        }
        if (win) {
            const sets = desks.item ? desks.item.sets : [];
            const count = sets.length > 1 ? sets.length : 4;
            const sub = [];
            for (let i = 0; i < count; i++) {
                const here = sets.length > 1 && sets[i].active;
                sub.push({ label: (sets.length > 1 && sets[i].name ? sets[i].name : "Desktop " + (i + 1)) + (here ? " (this one)" : ""), enabled: !here, run: () => moveToDesktop(win, i + 1) });
            }
            if (list.length) list.push({ separator: true });
            list.push({ label: "Move to desktop", icon: "desktops", sub: sub });
            if (!win.maximized) list.push({ label: "Maximize", icon: "window", run: () => { win.activate(); win.maximized = true; } });
            else list.push({ label: "Restore size", icon: "window", run: () => { win.maximized = false; } });
            list.push({ separator: true });
            if (ws.length > 1) list.push({ label: "Close all " + ws.length + " windows", icon: "close", danger: true, run: () => ws.forEach(w => w.close()) });
            else list.push({ label: "Close window", icon: "close", danger: true, run: () => win.close() });
        }
        const p = menuPoint(btn);
        menu.popup(screen, p.x, p.y, { title: appName(it), above: !portrait }, list);
    }

    Rectangle {
        id: body
        anchors {
            fill: parent
            leftMargin: bar.portrait ? 6 : 8
            rightMargin: bar.portrait ? 4 : 8
            bottomMargin: bar.portrait ? 8 : 6
            topMargin: bar.portrait ? 8 : 4
        }
        radius: Theme.radius
        color: Theme.panel
        border.color: Theme.line
        border.width: 1

        GridLayout {
            anchors {
                fill: parent
                leftMargin: bar.portrait ? 0 : 8
                rightMargin: bar.portrait ? 0 : 8
                topMargin: bar.portrait ? 8 : 0
                bottomMargin: bar.portrait ? 8 : 0
            }
            flow: bar.portrait ? GridLayout.TopToBottom : GridLayout.LeftToRight
            rows: bar.portrait ? -1 : 1
            columns: bar.portrait ? 1 : -1
            rowSpacing: 6
            columnSpacing: 6

            // Launcher (the Hearth mark)
            BarButton {
                tip: "Apps (Super)"
                onClicked: bar.launcherRequested()
                Image {
                    anchors.centerIn: parent
                    width: 32; height: 32
                    source: Qt.resolvedUrl("images/hearth-logo.png")
                    smooth: true; mipmap: true
                }
            }

            // Virtual desktops (hidden when there's only one, or labwc / Quickshell can't tell)
            Loader {
                id: desks
                Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                visible: status === Loader.Ready && !!item && item.wanted
                source: "DeskSwitcher.qml"
                onLoaded: item.portrait = Qt.binding(() => bar.portrait)
            }

            Separator {}

            // Pinned and open apps
            Repeater {
                id: taskRepeater
                model: bar.taskItems
                delegate: BarButton {
                    id: task
                    required property var modelData
                    required property int index
                    readonly property var wins: modelData.windows
                    readonly property bool focused: wins.some(w => w.activated)
                    tip: wins.length > 1 ? bar.appName(modelData) + " (" + wins.length + " windows)\n" + wins.slice(0, 6).map(w => "• " + (w.title || bar.appName(modelData))).join("\n")
                        : wins.length === 1 ? (wins[0].title || bar.appName(modelData)) : bar.appName(modelData)
                    active: focused
                    running: wins.length > 0
                    count: wins.length
                    draggable: modelData.pinned
                    // Where a dragged app would go: a lit line at that end of the button.
                    dropMark: bar.dragFrom >= 0 && bar.dragTo === index && bar.dragTo !== bar.dragFrom ? (bar.dragTo < bar.dragFrom ? -1 : 1) : 0
                    onClicked: bar.focusApp(modelData, task)
                    onMiddleClicked: bar.newInstance(modelData)
                    onRightClicked: bar.taskMenu(task, modelData)
                    onDragStarted: { bar.dragFrom = index; bar.dragTo = index; }
                    onDragMoved: pos => bar.dragTo = bar.dropIndexAt(pos)
                    onDragEnded: bar.finishDrag()
                    AppIcon {
                        anchors.centerIn: parent
                        size: 28
                        // Hearth's own windows (Settings, Files, Task Manager) show their own Hearth icons.
                        source: task.modelData.own ? AppIcons.own(task.wins.length ? task.wins[0].title : "") : ""
                        icon: task.modelData.entry ? (task.modelData.entry.icon || "") : ""
                        appId: task.wins.length ? (task.wins[0].appId || "") : (task.modelData.id || "")
                    }
                }
            }

            Item { Layout.fillWidth: !bar.portrait; Layout.fillHeight: bar.portrait }

            // Tray icons (apps like Discord, Steam)
            Repeater {
                model: SystemTray.items
                delegate: BarButton {
                    id: tray
                    required property SystemTrayItem modelData
                    narrow: true
                    tip: (modelData.tooltipTitle || modelData.title || modelData.id || "")
                        + (modelData.tooltipDescription ? "\n" + modelData.tooltipDescription.replace(/<[^>]*>/g, "") : "")
                    // Click opens the app (or its menu, for apps that only have a menu); right-click its menu.
                    onClicked: { if (modelData.onlyMenu && modelData.hasMenu) showMenu(); else modelData.activate(); }
                    onMiddleClicked: modelData.secondaryActivate()
                    onRightClicked: { if (modelData.hasMenu) showMenu(); else modelData.secondaryActivate(); }
                    onScrolled: delta => modelData.scroll(delta, false)
                    function showMenu() {
                        const p = tray.mapToItem(null, 0, 0);
                        if (bar.portrait) modelData.display(bar, p.x + tray.width + 8, p.y);
                        else modelData.display(bar, p.x, p.y - 8);
                    }
                    IconImage { anchors.centerIn: parent; implicitSize: 20; source: tray.modelData.icon }
                }
            }

            // Volume: click mutes, scroll changes it
            PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
            BarButton {
                id: vol
                readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
                readonly property int pct: audio ? Math.round(audio.volume * 100) : 0
                visible: audio !== null
                wide: !bar.portrait
                tip: (audio && audio.muted ? "Muted" : "Volume " + pct + "%") + "\nScroll to change, middle-click to mute"
                onClicked: bar.quickRequested()
                onMiddleClicked: if (audio) audio.muted = !audio.muted
                onRightClicked: bar.soundMenu(vol)
                onScrolled: delta => { if (audio) audio.volume = Math.max(0, Math.min(Buttons.maxVolume, audio.volume + (delta > 0 ? 0.05 : -0.05))); }
                StatusLabel {
                    icon: !vol.audio || vol.audio.muted ? "audio-volume-muted-symbolic" : vol.pct > 66 ? "audio-volume-high-symbolic" : vol.pct > 33 ? "audio-volume-medium-symbolic" : "audio-volume-low-symbolic"
                    label: bar.portrait || (vol.audio && vol.audio.muted) ? "" : vol.pct + "%"
                }
            }

            // Network, read from NetworkManager every 10 seconds
            BarButton {
                id: net
                property string kind: ""     // wifi | ethernet | ""
                property string name: ""
                tip: name ? "Connected to " + name : "Not connected"
                onClicked: bar.quickRequested()
                onRightClicked: bar.statusMenu(net, name ? "Connected to " + name : "Not connected", [
                    { label: "Wi-Fi settings", icon: "wifi", run: () => bar.settingsRequested("wifi") },
                    { label: "Quick Access", icon: "menu", run: () => bar.quickRequested() }
                ])
                StatusLabel {
                    icon: net.kind === "wifi" ? "network-wireless-signal-good-symbolic" : net.kind === "ethernet" ? "network-wired-symbolic" : "network-offline-symbolic"
                    label: ""
                }
                Process {
                    id: nmcli
                    command: ["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"]
                    stdout: StdioCollector {
                        onStreamFinished: {
                            let k = "", n = "";
                            for (const line of text.split("\n")) {
                                const f = line.split(":");
                                if (f[1] === "connected" && (f[0] === "wifi" || f[0] === "ethernet") && !k) { k = f[0]; n = f.slice(2).join(":").replace(/\\(.)/g, "$1"); }   // nmcli -t puts a backslash before ":" in names
                            }
                            net.kind = k; net.name = n;
                        }
                    }
                }
                Timer { interval: 10000; running: true; repeat: true; triggeredOnStart: true; onTriggered: nmcli.running = true }
            }

            // Battery (laptops and handhelds only)
            BarButton {
                id: batt
                readonly property var dev: UPower.displayDevice
                readonly property int pct: dev ? Math.round(dev.percentage * 100) : 0
                readonly property bool charging: dev && dev.state === UPowerDeviceState.Charging
                visible: dev && dev.isLaptopBattery
                wide: !bar.portrait
                tip: pct + "%" + (charging ? ", charging" : "")
                onClicked: bar.quickRequested()
                onRightClicked: bar.statusMenu(batt, batt.tip, [{ label: "Power settings", icon: "power", run: () => bar.settingsRequested("power") }])
                StatusLabel {
                    icon: batt.charging ? "battery-good-charging-symbolic" : batt.pct > 60 ? "battery-good-symbolic" : batt.pct > 20 ? "battery-low-symbolic" : "battery-caution-symbolic"
                    label: bar.portrait ? "" : batt.pct + "%"
                }
            }

            // Bluetooth (when there's an adapter): lit when a device is connected; click for Bluetooth settings.
            BarButton {
                id: bt
                property bool present: false
                property bool powered: false
                property var connected: []          // names of connected devices
                visible: present
                tip: !powered ? "Bluetooth is off" : connected.length ? "Connected: " + connected.join(", ") : "Bluetooth is on"
                onClicked: bar.settingsRequested("bluetooth")
                onRightClicked: bar.statusMenu(bt, bt.tip, [
                    { label: bt.powered ? "Turn Bluetooth off" : "Turn Bluetooth on", icon: "bt", run: () => { btSet.command = ["bluetoothctl", "power", bt.powered ? "off" : "on"]; btSet.running = true; } },
                    { label: "Bluetooth settings", icon: "gear", run: () => bar.settingsRequested("bluetooth") }
                ])
                Icon { anchors.centerIn: parent; name: "bt"; size: 18; color: !bt.powered ? Theme.muted : bt.connected.length ? Theme.accent : Theme.text }
                // Asked every 15 s (BlueZ through bluetoothctl, like Settings › Bluetooth).
                Process {
                    id: btProc
                    command: ["sh", "-c", "command -v bluetoothctl >/dev/null || exit 3; bluetoothctl --timeout 3 show 2>/dev/null | grep -E '^Controller|Powered:'; echo ---; bluetoothctl --timeout 3 devices Connected 2>/dev/null"]
                    stdout: StdioCollector {
                        onStreamFinished: {
                            const parts = text.split("---");
                            bt.present = /Controller/.test(parts[0] || "");
                            bt.powered = /Powered:\s*yes/.test(parts[0] || "");
                            bt.connected = (parts[1] || "").split("\n").map(l => l.match(/^Device\s+\S+\s+(.+)$/)).filter(Boolean).map(m => m[1].trim());
                        }
                    }
                }
                Process { id: btSet; onExited: btProc.running = true }
                Timer { interval: 15000; running: true; repeat: true; triggeredOnStart: true; onTriggered: if (!btProc.running) btProc.running = true }
            }

            // Hearth Friends (Friends.qml): chats and friends on Matrix, with a badge for unread messages
            BarButton {
                tip: SettingsNav.friendsUnread > 0 ? "Friends · " + SettingsNav.friendsUnread + " unread" : "Friends"
                onClicked: if (SettingsNav.friends) SettingsNav.friends.toggle()
                Icon { anchors.centerIn: parent; name: "user"; size: 20 }
                Rectangle {
                    visible: SettingsNav.friendsUnread > 0
                    x: parent.width / 2 + 1; y: 5
                    width: Math.max(16, fbadge.implicitWidth + 8); height: 16; radius: 8
                    color: Theme.accent
                    border.color: Theme.panel; border.width: 1
                    Text { id: fbadge; anchors.centerIn: parent; text: SettingsNav.friendsUnread > 9 ? "9+" : String(SettingsNav.friendsUnread); color: Theme.accentInk; font.pixelSize: 10; font.bold: true; font.family: Theme.font }
                }
            }
            // Quick Access and the on-screen keyboard (handy on touchscreens)
            BarButton {
                tip: "Quick Access"
                onClicked: bar.quickRequested()
                Icon { anchors.centerIn: parent; name: "open-menu-symbolic"; size: 20 }
            }
            BarButton {
                tip: "Keyboard"
                onClicked: bar.keyboardRequested()
                Icon { anchors.centerIn: parent; name: "input-keyboard-symbolic"; size: 20 }
            }

            // Notifications: the bell (crossed out with Do Not Disturb on), a dot for ones you haven't seen
            BarButton {
                tip: DeskState.dnd ? "Do not disturb is on" : DeskState.unseen > 0 ? DeskState.unseen + (DeskState.unseen === 1 ? " new notification" : " new notifications") : "Notifications"
                onClicked: bar.calendarRequested()
                onRightClicked: DeskState.setDnd(!DeskState.dnd)
                Icon { anchors.centerIn: parent; name: DeskState.dnd ? "bell-off" : "bell"; size: 20; color: DeskState.dnd ? Theme.muted : Theme.text }
                // How many you haven't seen, on the bell.
                Rectangle {
                    visible: DeskState.unseen > 0 && !DeskState.dnd
                    x: parent.width / 2 + 1; y: 5
                    width: Math.max(16, badge.implicitWidth + 8); height: 16; radius: 8
                    color: Theme.accent
                    border.color: Theme.panel; border.width: 1
                    Text {
                        id: badge
                        anchors.centerIn: parent
                        text: DeskState.unseen > 9 ? "9+" : String(DeskState.unseen)
                        color: Theme.accentInk; font.pixelSize: 10; font.bold: true; font.family: Theme.font
                    }
                }
            }

            // Clock: click for the calendar and notifications
            SystemClock { id: clock; precision: SystemClock.Minutes }
            BarButton {
                wide: !bar.portrait
                tip: Qt.formatDate(clock.date, "dddd, MMMM d, yyyy")
                onClicked: bar.calendarRequested()
                Column {
                    anchors.centerIn: parent
                    Text {
                        anchors.horizontalCenter: bar.portrait ? parent.horizontalCenter : undefined
                        anchors.right: bar.portrait ? undefined : parent.right
                        text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : bar.portrait ? "h:mm" : "h:mm AP")
                        color: Theme.text; font.pixelSize: bar.portrait ? 13 : 14; font.bold: true; font.family: Theme.font
                    }
                    Text {
                        visible: !bar.portrait
                        anchors.right: parent.right
                        text: Qt.formatDate(clock.date, "ddd, MMM d")
                        color: Theme.muted; font.pixelSize: 11; font.family: Theme.font
                    }
                }
            }

            // Back to Game Mode
            Rectangle {
                Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                implicitWidth: bar.portrait ? 44 : gm.implicitWidth + 28
                implicitHeight: bar.portrait ? 44 : 36
                radius: 10
                color: gmArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
                Text { id: gm; visible: !bar.portrait; anchors.centerIn: parent; text: "Game Mode"; color: Theme.accentInk; font.bold: true; font.pixelSize: 13; font.family: Theme.font }
                Icon { visible: bar.portrait; anchors.centerIn: parent; name: "input-gaming-symbolic"; size: 22; color: Theme.accentInk }
                MouseArea { id: gmArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Quickshell.execDetached(["nimbus-mode", "game"]) }
            }

            // Show desktop: the thin strip at the very end (Windows' corner); click again for the windows.
            Rectangle {
                id: showDesk
                Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                implicitWidth: bar.portrait ? 32 : 8
                implicitHeight: bar.portrait ? 8 : 36
                radius: 3
                color: sdArea.containsMouse ? Theme.panel3 : "transparent"
                border.color: sdArea.containsMouse ? Theme.line : "transparent"
                MouseArea {
                    id: sdArea
                    anchors.fill: parent
                    anchors.margins: -4
                    hoverEnabled: true
                    onClicked: bar.toggleShowDesktop()
                }
                PopupWindow {
                    visible: sdArea.containsMouse
                    anchor.item: showDesk
                    anchor.rect.x: bar.portrait ? showDesk.width + 12 : showDesk.width / 2 - implicitWidth + 8
                    anchor.rect.y: bar.portrait ? showDesk.height / 2 - implicitHeight / 2 : -implicitHeight - 14
                    implicitWidth: sdTip.implicitWidth + 20
                    implicitHeight: sdTip.implicitHeight + 12
                    color: "transparent"
                    Rectangle {
                        anchors.fill: parent
                        radius: 8
                        color: Theme.panel2
                        border.color: Theme.line
                        Text { id: sdTip; anchors.centerIn: parent; text: "Show desktop (Super+D)"; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks

    component BarButton: Item {
        id: btn
        property string tip: ""
        property bool active: false
        property bool running: false       // an app with windows open (a pinned one may have none)
        property int count: 0              // how many windows
        property bool narrow: false
        property bool wide: false
        property bool draggable: false     // a pinned app: can be dragged along the bar
        property int dropMark: 0           // -1 / 1: a dragged app would go before / after this one
        property bool dragging: false
        property real dragOffset: 0
        default property alias content: holder.data
        signal clicked()
        signal middleClicked()
        signal rightClicked()
        signal scrolled(int delta)
        signal dragStarted()
        signal dragMoved(real pos)        // the pointer along the bar (scene x, or y on a side bar)
        signal dragEnded()
        z: dragging ? 5 : 0
        transform: Translate { x: bar.portrait ? 0 : btn.dragOffset; y: bar.portrait ? btn.dragOffset : 0 }

        implicitWidth: narrow ? 30 : wide ? Math.max(44, (holder.children.length ? holder.children[0].implicitWidth : 0) + 20) : 44
        implicitHeight: narrow && bar.portrait ? 30 : 44
        Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
        Rectangle {
            anchors.fill: parent
            radius: 10
            color: area.containsMouse ? Theme.panel3 : btn.active ? Theme.panel2 : "transparent"
        }
        Item { id: holder; anchors.fill: parent; opacity: btn.dragging ? 0.75 : 1 }
        Rectangle {                        // where a dragged app would land
            visible: btn.dropMark !== 0
            color: Theme.accent
            radius: 1
            width: bar.portrait ? parent.width - 8 : 3
            height: bar.portrait ? 3 : parent.height - 8
            x: bar.portrait ? 4 : (btn.dropMark < 0 ? -4 : parent.width + 1)
            y: bar.portrait ? (btn.dropMark < 0 ? -4 : parent.height + 1) : 4
        }
        Rectangle { // open indicator under the icon (beside it on a side bar): long and lit when in front
            readonly property int len: btn.active ? 16 : btn.count > 1 ? 10 : 5
            visible: btn.active || btn.running
            x: bar.portrait ? 1 : (parent.width - width) / 2
            y: bar.portrait ? (parent.height - height) / 2 : parent.height - height - 2
            width: bar.portrait ? 3 : len
            height: bar.portrait ? len : 3
            radius: 2
            color: btn.active ? Theme.accent : Theme.muted
            Behavior on width { NumberAnimation { duration: Theme.quick } }
            Behavior on height { NumberAnimation { duration: Theme.quick } }
        }
        MouseArea {
            id: area
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
            property real startPos: 0
            property bool dragged: false
            onPressed: mouse => {
                startPos = bar.portrait ? mouse.y : mouse.x;
                dragged = false;
            }
            onPositionChanged: mouse => {
                if (!btn.draggable || !(mouse.buttons & Qt.LeftButton)) return;
                const d = (bar.portrait ? mouse.y : mouse.x) - startPos;
                if (!btn.dragging && Math.abs(d) > 12) { btn.dragging = true; dragged = true; btn.dragStarted(); }
                if (btn.dragging) {
                    btn.dragOffset = d;
                    const p = area.mapToItem(null, mouse.x, mouse.y);
                    btn.dragMoved(bar.portrait ? p.y : p.x);
                }
            }
            onReleased: mouse => {
                if (btn.dragging) { btn.dragging = false; btn.dragOffset = 0; btn.dragEnded(); }
            }
            onCanceled: { if (btn.dragging) { btn.dragging = false; btn.dragOffset = 0; btn.dragEnded(); } }
            onClicked: mouse => {
                if (dragged) { dragged = false; return; }
                if (mouse.button === Qt.MiddleButton) btn.middleClicked();
                else if (mouse.button === Qt.RightButton) btn.rightClicked();
                else btn.clicked();
            }
            onWheel: wheel => btn.scrolled(wheel.angleDelta.y)
        }
        // Hover label above the bar
        PopupWindow {
            visible: area.containsMouse && btn.tip !== "" && !btn.dragging
            anchor.item: btn
            anchor.rect.x: bar.portrait ? btn.width + 12 : btn.width / 2 - implicitWidth / 2
            anchor.rect.y: bar.portrait ? btn.height / 2 - implicitHeight / 2 : -implicitHeight - 10
            implicitWidth: tipText.implicitWidth + 20
            implicitHeight: tipText.implicitHeight + 12
            color: "transparent"
            Rectangle {
                anchors.fill: parent
                radius: 8
                color: Theme.panel2
                border.color: Theme.line
                Text { id: tipText; anchors.centerIn: parent; text: btn.tip; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
            }
        }
    }

    component Separator: Rectangle {
        Layout.preferredWidth: bar.portrait ? 28 : 1
        Layout.preferredHeight: bar.portrait ? 1 : 24
        Layout.alignment: Qt.AlignCenter
        color: Theme.line
    }

    component StatusLabel: Row {
        property string icon
        property string label
        anchors.centerIn: parent
        spacing: 6
        Icon { anchors.verticalCenter: parent.verticalCenter; size: 18; name: parent.icon }
        Text { anchors.verticalCenter: parent.verticalCenter; visible: text !== ""; text: parent.label; color: Theme.text; font.pixelSize: 13; font.family: Theme.font }
    }
}
