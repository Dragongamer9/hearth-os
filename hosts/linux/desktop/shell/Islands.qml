// Mantel 2026: the main bar. Floating islands (Hearth, Gaming, Glass, Synthwave, Cabin), one full-width bar
// (Steam-style; Zen's is thin) or Console's big status row (theme/palettes.json "islandStyle", or your own
// Settings › Appearance › Bar and dock choice). At the top by default; with Bar layout "One main bar" on the edge
// you pick (bottom, top, left or right), and with the floating app menu off it holds your apps too (TaskStrip.qml):
//   start  the Hearth flame (opens the app menu), the desktops, the window in front (icon + title), your apps
//   (Gaming) a performance island: FPS (only when MangoHud logs it), CPU, GPU, memory, temperatures
//   centre the clock pill: grows to show what's playing, and briefly the newest notification; click for the
//          notification centre and dashboard (in a bar with your apps, or down the side, a compact clock at the end)
//   end    the status island: tray, Discord, keyboard, Wi-Fi, Bluetooth, volume, battery, notifications (click: Quick
//          settings; right-click menus; middle-click mutes; scroll changes the volume), then the Game Mode button,
//          always shown, in every look and on every edge, one press away.
// Left / right: icons stacked, the clock compact, menus and tips open toward the screen.
// Auto-hide (Bar and dock › Auto-hide, in One main bar or with the apps in the bar): hidden until the pointer touches
// the edge; it reserves no space then, and when the space it reserves changes it's put back on screen once so labwc
// re-fits maximized windows (the Game Mode return fix the dock has).
// Loaded by shell.qml by file name: if this file can't load, the classic taskbar (Bar.qml) comes up instead.
// Handheld screens (1440 px wide or less) keep only the essentials here; the rest is in Quick settings.
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
    id: isl
    signal launcherRequested()
    signal quickRequested()
    signal keyboardRequested()
    signal calendarRequested()
    signal settingsRequested(string page)
    signal menuSearchRequested()           // the Tiles menu's search field: open the menu, ready to search
    property string menuQuery: ""          // what you typed in the Tiles menu (Launcher.menuQuery, given by shell.qml)
    // The Tiles app menu has its search field here, next to the flame (as in the mockup), on a bar across the top or
    // bottom with room for it. The menu itself then has none at the top (Launcher.barSearch).
    readonly property bool menuSearchShown: Theme.appMenu === "tiles" && !vertical && !shelf && !compact
    property var menu: null                // PopMenu
    property bool primary: false
    property bool quickOpen: false         // Quick settings is open: the status island is ringed
    property bool calendarOpen: false      // the dashboard is open: the pill is ringed
    property string edge: "top"            // top | bottom | left | right (Theme.barEdge, given by shell.qml)
    property bool holdsApps: false         // the floating app menu is off: your apps are in this bar

    readonly property bool vertical: edge === "left" || edge === "right"
    readonly property string style: vertical && Theme.mainStyle === "shelf" ? "bar" : Theme.mainStyle   // floating | bar | shelf
    readonly property bool floating: style === "floating"
    readonly property bool shelf: style === "shelf"
    readonly property bool compact: screen && screen.width <= 1440 && !shelf
    readonly property real k: shelf ? 1.4 : 1
    readonly property int islandH: shelf ? 72 : vertical ? (compact ? 52 : 56) : Theme.thin ? 32 : style === "bar" ? 44 : compact ? 48 : 52
    readonly property int gapOut: floating ? 10 : shelf ? 18 : 0      // from the screen's edge
    readonly property int gapIn: floating ? 6 : shelf ? 10 : 0        // toward the windows
    readonly property int gapTop: gapOut                                // (the old name, kept for the look's parts)
    readonly property int band: islandH + gapOut + gapIn
    readonly property bool gaming: Theme.look === "gaming"
    // The clock in the middle of a horizontal bar; in a bar with your apps, or down a side, a compact clock at the end.
    readonly property bool pillCentred: !vertical && !holdsApps && !shelf

    // ---- auto-hide
    readonly property bool autoHide: Theme.barAutoHide && !shelf
    readonly property bool hiddenNow: autoHide && !hov.hovered && !DeskState.dockPeek && !holdOpen.running && !quickOpen && !calendarOpen && !DeskState.overlayShown
    readonly property int reserved: autoHide ? 0 : band
    Timer { id: holdOpen; interval: 700 }

    anchors { top: edge !== "bottom"; bottom: edge !== "top"; left: edge !== "right"; right: edge !== "left" }
    implicitHeight: vertical ? 0 : band
    implicitWidth: vertical ? band : 0
    exclusiveZone: reserved
    color: "transparent"
    WlrLayershell.namespace: "hearth-islands"
    WlrLayershell.layer: WlrLayer.Top
    // Only the bar (or, hidden, the thin handle at the edge) takes clicks.
    mask: Region { item: isl.hiddenNow ? handle : whole }
    Item { id: whole; anchors.fill: parent }

    function claim() {
        if (!primary) return;
        DeskState.barOwner = isl;
        DeskState.classicBar = false;
        DeskState.barEdge = edge;
        DeskState.barTop = edge === "top" ? reserved : 0;
        DeskState.barBottom = edge === "bottom" ? reserved : 0;
        DeskState.barLeft = edge === "left" ? reserved : 0;
        DeskState.barRight = edge === "right" ? reserved : 0;
    }
    onPrimaryChanged: claim()
    onEdgeChanged: claim()
    onReservedChanged: { claim(); refit(); }
    Component.onCompleted: claim()
    Component.onDestruction: {
        if (DeskState.barOwner !== isl) return;
        DeskState.barOwner = null;
        DeskState.barTop = 0; DeskState.barBottom = 0; DeskState.barLeft = 0; DeskState.barRight = 0;
    }
    // As the dock: wait for your settings (at most a few seconds, so it always comes up), and when the space it
    // reserves changes, off screen for a moment and back, so labwc works out the free area again.
    property bool settingsReady: Settings.loaded
    Timer { interval: 4000; running: !isl.settingsReady; onTriggered: isl.settingsReady = true }
    property bool remapping: false
    Timer { id: remapTimer; interval: 150; onTriggered: isl.remapping = false }
    visible: settingsReady && !remapping
    property int lastReserved: -1
    onSettingsReadyChanged: refit()
    function refit() {
        if (!settingsReady) return;
        if (lastReserved >= 0 && lastReserved !== reserved) { remapping = true; remapTimer.restart(); }
        lastReserved = reserved;
    }

    // Menus open next to the bar, toward the screen.
    function menuPoint(item) {
        const sw = screen ? screen.width : 1920, sh = screen ? screen.height : 1080;
        const p = item.mapToItem(null, item.width / 2, item.height / 2);
        const half = menu ? menu.cardWidth / 2 : 134;   // the right-click menu's look sets its width (Theme.menuStyle)
        if (edge === "bottom") return Qt.point(Math.max(8, Math.min(sw - 300, p.x - half)), sh - isl.band - 2);
        if (edge === "left") return Qt.point(isl.band + 2, Math.max(8, p.y - 20));
        if (edge === "right") return Qt.point(sw - isl.band - 290, Math.max(8, p.y - 20));
        return Qt.point(Math.max(8, Math.min(sw - 300, p.x - half)), isl.band + 2);
    }
    function statusMenu(btn, title, list) {
        if (!menu) return;
        const p = menuPoint(btn);
        menu.popup(screen, p.x, p.y, { title: title, above: edge === "bottom" }, list);
    }

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
        list.push({ label: "Sound settings", icon: "gear", run: () => isl.settingsRequested("sound") });
        statusMenu(btn, "Sound output", list);
    }

    // ------------------------------------------------------------------ system state (as the classic bar)
    PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
    readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
    readonly property int volPct: audio ? Math.round(audio.volume * 100) : 0
    readonly property var batt: UPower.displayDevice
    readonly property bool hasBatt: batt && batt.isLaptopBattery
    readonly property int battPct: batt ? Math.round(batt.percentage * 100) : 0
    readonly property bool charging: batt && batt.state === UPowerDeviceState.Charging
    property string netKind: ""
    property string netName: ""
    Process {
        id: nmcli
        command: ["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"]
        stdout: StdioCollector {
            onStreamFinished: {
                let k = "", n = "";
                for (const line of text.split("\n")) {
                    const f = line.split(":");
                    if (f[1] === "connected" && (f[0] === "wifi" || f[0] === "ethernet") && !k) { k = f[0]; n = f.slice(2).join(":").replace(/\\(.)/g, "$1"); }
                }
                isl.netKind = k; isl.netName = n;
            }
        }
    }
    Timer { interval: 10000; running: true; repeat: true; triggeredOnStart: true; onTriggered: nmcli.running = true }
    property bool btPresent: false
    property bool btPowered: false
    property var btConnected: []
    Process {
        id: btProc
        command: ["sh", "-c", "command -v bluetoothctl >/dev/null || exit 3; bluetoothctl --timeout 3 show 2>/dev/null | grep -E '^Controller|Powered:'; echo ---; bluetoothctl --timeout 3 devices Connected 2>/dev/null"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("---");
                isl.btPresent = /Controller/.test(parts[0] || "");
                isl.btPowered = /Powered:\s*yes/.test(parts[0] || "");
                isl.btConnected = (parts[1] || "").split("\n").map(l => l.match(/^Device\s+\S+\s+(.+)$/)).filter(Boolean).map(m => m[1].trim());
            }
        }
    }
    Process { id: btSet; onExited: btProc.running = true }
    Timer { interval: 15000; running: true; repeat: true; triggeredOnStart: true; onTriggered: if (!btProc.running) btProc.running = true }
    readonly property var front: ToplevelManager.activeToplevel
    SystemClock { id: clock; precision: SystemClock.Minutes }

    // The newest notification, shown in the pill for a few seconds (not with Do not disturb).
    property var flash: null
    property int seenSeq: -1
    Connections {
        target: DeskState
        function onHistoryChanged() {
            const n = DeskState.history[0];
            if (!n || n.key === isl.seenSeq) return;
            isl.seenSeq = n.key;
            if (DeskState.dnd || DeskState.isMuted(n.app)) return;
            isl.flash = n;
            flashTimer.restart();
        }
    }
    Timer { id: flashTimer; interval: 5000; onTriggered: isl.flash = null }


    // ------------------------------------------------------------------ auto-hide: the handle at the edge
    Item {
        id: handle
        width: isl.vertical ? 10 : 220
        height: isl.vertical ? 220 : 10
        x: isl.edge === "right" ? parent.width - width : isl.edge === "left" ? 0 : (parent.width - width) / 2
        y: isl.edge === "bottom" ? parent.height - height : isl.edge === "top" ? 0 : (parent.height - height) / 2
        Rectangle {
            anchors.centerIn: parent
            width: isl.vertical ? 4 : 80; height: isl.vertical ? 80 : 4; radius: 2
            color: Theme.muted; opacity: isl.hiddenNow ? 0.6 : 0
        }
        HoverHandler { onHoveredChanged: if (hovered) holdOpen.restart() }
    }

    // ------------------------------------------------------------------ the band
    Item {
        id: bandItem
        width: parent.width; height: parent.height
        // Hidden (auto-hide): slid off past the edge.
        readonly property real off: isl.hiddenNow ? isl.band + 4 : 0
        x: isl.edge === "left" ? -off : isl.edge === "right" ? off : 0
        y: isl.edge === "top" ? -off : isl.edge === "bottom" ? off : 0
        Behavior on x { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Behavior on y { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        HoverHandler { id: hov; onHoveredChanged: if (!hovered) holdOpen.restart() }

        Item {
            id: inner
            readonly property int along: isl.floating ? 12 : isl.shelf ? 40 : 0
            anchors {
                fill: parent
                topMargin: isl.edge === "top" ? isl.gapOut : isl.edge === "bottom" ? isl.gapIn : inner.along
                bottomMargin: isl.edge === "bottom" ? isl.gapOut : isl.edge === "top" ? isl.gapIn : inner.along
                leftMargin: isl.edge === "left" ? isl.gapOut : isl.edge === "right" ? isl.gapIn : inner.along
                rightMargin: isl.edge === "right" ? isl.gapOut : isl.edge === "left" ? isl.gapIn : inner.along
            }

            // One full-width bar (Steam-style, Zen, and One main bar in those looks): a flat strip along the edge.
            Rectangle {
                anchors.fill: parent
                visible: isl.style === "bar"
                color: Theme.glass
                Rectangle {   // the line on the side toward the windows
                    color: Theme.line
                    width: isl.vertical ? 1 : parent.width
                    height: isl.vertical ? parent.height : 1
                    x: isl.edge === "left" ? parent.width - 1 : 0
                    y: isl.edge === "top" ? parent.height - 1 : 0
                }
                Loader { anchors.fill: parent; active: Theme.frame === "frost" && isl.style === "bar"; source: "Frame.qml" }
            }

            GridLayout {
                id: mainRow
                anchors.fill: parent
                anchors.leftMargin: isl.style === "bar" && !isl.vertical ? 8 : 0
                anchors.rightMargin: isl.style === "bar" && !isl.vertical ? 6 : 0
                anchors.topMargin: isl.style === "bar" && isl.vertical ? 8 : 0
                anchors.bottomMargin: isl.style === "bar" && isl.vertical ? 8 : 0
                flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
                rows: isl.vertical ? -1 : 1
                columns: isl.vertical ? 1 : -1
                rowSpacing: isl.floating ? 10 : 6
                columnSpacing: isl.floating ? 10 : 6

                // ---- start island: the flame, the desktops, the window in front
                Island {
                    id: leftIsland
                    along: (isl.vertical ? leftRow.implicitHeight : leftRow.implicitWidth) + (isl.floating ? 16 : 4)
                    GridLayout {
                        id: leftRow
                        anchors.centerIn: parent
                        flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
                        rows: isl.vertical ? -1 : 1
                        columns: isl.vertical ? 1 : -1
                        rowSpacing: 8 * isl.k
                        columnSpacing: 8 * isl.k
                        IslandButton {
                            objectName: "appMenuButton"
                            tip: "Apps (Super)"
                            onClicked: isl.launcherRequested()
                            Image { anchors.centerIn: parent; width: 26 * isl.k; height: width; source: Qt.resolvedUrl("images/hearth-logo.png"); smooth: true; mipmap: true }
                        }
                        Rectangle {   // the Tiles menu's search field: click to search (what you type shows here)
                            objectName: "menuSearchField"
                            visible: isl.menuSearchShown
                            Layout.alignment: Qt.AlignCenter
                            implicitWidth: 300; implicitHeight: Math.max(26, isl.islandH - 14)
                            radius: Theme.r(8)
                            color: msa.containsMouse ? Theme.panel3 : Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.85)
                            border.width: isl.menuQuery !== "" ? 2 : 1
                            border.color: isl.menuQuery !== "" ? Theme.accent : Theme.line
                            Icon { x: 12; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 15; color: Theme.muted }
                            Text {
                                anchors { left: parent.left; right: parent.right; leftMargin: 36; rightMargin: 10; verticalCenter: parent.verticalCenter }
                                text: isl.menuQuery !== "" ? isl.menuQuery : "Type to search"
                                color: isl.menuQuery !== "" ? Theme.text : Theme.muted
                                elide: Text.ElideLeft
                                font.family: Theme.font; font.pixelSize: 14
                            }
                            MouseArea { id: msa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.IBeamCursor; onClicked: isl.menuSearchRequested() }
                        }
                        Text {   // Console: the big clock sits next to the mark
                            visible: isl.shelf
                            text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : "h:mm")
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 38; font.weight: Font.DemiBold
                            Text { anchors { left: parent.right; leftMargin: 6; baseline: parent.baseline } visible: !Settings.clock24; text: Qt.formatTime(clock.date, "AP"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 }
                        }
                        Loader {
                            id: desks
                            Layout.alignment: Qt.AlignCenter
                            visible: !isl.shelf && status === Loader.Ready && !!item && item.wanted
                            active: !isl.shelf
                            source: "DeskSwitcher.qml"
                            onLoaded: { item.island = true; item.portrait = Qt.binding(() => isl.vertical); item.pillSize = Qt.binding(() => isl.islandH > 40 ? 24 : 20); }
                        }
                        RowLayout {   // the window in front (across the top or bottom, without your apps in the bar)
                            visible: !isl.compact && !isl.shelf && !isl.vertical && !isl.holdsApps && isl.front !== null && (isl.front.title || "") !== ""
                            spacing: 8
                            Rectangle { visible: desks.visible; implicitWidth: 1; implicitHeight: 22; color: Theme.line }
                            AppIcon { size: 20; appId: isl.front ? (isl.front.appId || "") : ""; icon: "" }
                            Text {
                                Layout.maximumWidth: 260
                                text: isl.front ? (isl.front.title || "") : ""
                                elide: Text.ElideRight
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                                font.capitalization: isl.gaming ? Font.AllUppercase : Font.MixedCase
                                font.letterSpacing: isl.gaming ? 1.5 : 0
                            }
                        }
                    }
                }

                // ---- your apps (floating app menu off): the dock attached to the bar
                Island {
                    visible: isl.holdsApps && apps.status === Loader.Ready
                    along: apps.item ? (isl.vertical ? apps.item.implicitHeight : apps.item.implicitWidth) + (isl.floating ? 16 : 4) : 0
                    Loader {
                        id: apps
                        anchors.centerIn: parent
                        active: isl.holdsApps
                        source: "TaskStrip.qml"
                        onLoaded: {
                            item.menu = Qt.binding(() => isl.menu);
                            item.screen = Qt.binding(() => isl.screen);
                            item.vertical = Qt.binding(() => isl.vertical);
                            item.edge = Qt.binding(() => isl.edge);
                            item.side = Qt.binding(() => Math.max(30, isl.islandH - (isl.floating ? 10 : 8)));
                            item.barW = Qt.binding(() => isl.width);
                            item.barH = Qt.binding(() => isl.height);
                            item.primary = Qt.binding(() => isl.primary);
                        }
                        onStatusChanged: if (status === Loader.Error) console.warn("Mantel: your apps couldn't load in the bar (TaskStrip.qml); the app menu still opens from the flame")
                    }
                }

                // ---- Gaming: the performance island
                Island {
                    visible: Theme.perfIsland && !isl.compact && !isl.vertical && perf.status === Loader.Ready
                    along: perf.item ? perf.item.implicitWidth + 24 : 0
                    Loader { id: perf; anchors.centerIn: parent; active: Theme.perfIsland && !isl.compact && !isl.vertical; source: "PerfIsland.qml" }
                }

                Item { Layout.fillWidth: !isl.vertical; Layout.fillHeight: isl.vertical }

                // ---- the compact clock (with your apps in the bar, or down a side): click for the dashboard
                Island {
                    id: clockIsland
                    visible: !isl.pillCentred && !isl.shelf
                    lit: isl.calendarOpen
                    along: (isl.vertical ? clockCol.implicitHeight : clockCol.implicitWidth) + (isl.vertical ? 16 : 24)
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: isl.calendarRequested() }
                    ColumnLayout {
                        id: clockCol
                        anchors.centerIn: parent
                        spacing: 0
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : (isl.vertical ? "h:mm" : "h:mm AP"))
                            color: Theme.text; font.family: Theme.font; font.weight: Font.DemiBold
                            font.pixelSize: isl.vertical ? 14 : (isl.islandH > 40 ? 14 : 13)
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            visible: isl.vertical ? !Settings.clock24 : isl.islandH > 40
                            text: isl.vertical ? Qt.formatTime(clock.date, "AP") : Qt.formatDate(clock.date, "ddd d MMM")
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                        }
                    }
                }

                // ---- the status island
                Island {
                    id: statusIsland
                    lit: isl.quickOpen
                    along: (isl.vertical ? statusRow.implicitHeight : statusRow.implicitWidth) + (isl.floating ? 16 : 4)
                    MouseArea { anchors.fill: parent; onClicked: isl.quickRequested() }   // the island's gaps open Quick settings too
                    GridLayout {
                        id: statusRow
                        anchors.centerIn: parent
                        flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
                        rows: isl.vertical ? -1 : 1
                        columns: isl.vertical ? 1 : -1
                        rowSpacing: isl.shelf ? 12 : 2
                        columnSpacing: isl.shelf ? 12 : 2
                        Repeater {
                            model: (isl.compact || isl.vertical) && !Theme.hints ? [] : SystemTray.items
                            delegate: IslandButton {
                                id: tray
                                required property SystemTrayItem modelData
                                narrow: !isl.shelf
                                tip: (modelData.tooltipTitle || modelData.title || modelData.id || "")
                                    + (modelData.tooltipDescription ? "\n" + modelData.tooltipDescription.replace(/<[^>]*>/g, "") : "")
                                onClicked: { if (modelData.onlyMenu && modelData.hasMenu) showMenu(); else modelData.activate(); }
                                onMiddleClicked: modelData.secondaryActivate()
                                onRightClicked: { if (modelData.hasMenu) showMenu(); else modelData.secondaryActivate(); }
                                onScrolled: delta => modelData.scroll(delta, false)
                                function showMenu() { const p = tray.mapToItem(null, 0, tray.height); modelData.display(isl, p.x, isl.edge === "bottom" ? p.y - tray.height - 8 : p.y + 8); }
                                IconImage { anchors.centerIn: parent; implicitSize: 20 * isl.k; source: tray.modelData.icon }
                            }
                        }
                        IslandButton {   // Discord (Discord.qml): its small window by the bar
                            id: discordIsl
                            visible: !isl.compact
                            tip: "Discord"
                            onClicked: if (SettingsNav.discord) { const p = discordIsl.mapToItem(null, discordIsl.width / 2, discordIsl.height / 2); SettingsNav.discord.toggle(isl.edge === "left" || isl.edge === "right" ? p.y : p.x); }
                            Icon { anchors.centerIn: parent; name: "chat"; size: 20 * isl.k }
                        }
                        IslandButton {   // on-screen keyboard
                            visible: !isl.compact
                            tip: "Keyboard"
                            onClicked: isl.keyboardRequested()
                            Icon { anchors.centerIn: parent; name: "kbd"; size: 20 * isl.k }
                        }
                        IslandButton {   // network
                            id: net
                            tip: isl.netName ? "Connected to " + isl.netName : "Not connected"
                            onClicked: isl.quickRequested()
                            onRightClicked: isl.statusMenu(net, net.tip, [
                                { label: "Wi-Fi settings", icon: "wifi", run: () => isl.settingsRequested("wifi") },
                                { label: "Quick settings", icon: "menu", run: () => isl.quickRequested() }
                            ])
                            Icon { anchors.centerIn: parent; size: 19 * isl.k; name: isl.netKind === "wifi" ? "wifi" : isl.netKind === "ethernet" ? "wired" : "offline" }
                        }
                        IslandButton {   // Bluetooth
                            id: bt
                            visible: isl.btPresent && !isl.compact
                            tip: !isl.btPowered ? "Bluetooth is off" : isl.btConnected.length ? "Connected: " + isl.btConnected.join(", ") : "Bluetooth is on"
                            onClicked: isl.quickRequested()
                            onRightClicked: isl.statusMenu(bt, bt.tip, [
                                { label: isl.btPowered ? "Turn Bluetooth off" : "Turn Bluetooth on", icon: "bt", run: () => { btSet.command = ["bluetoothctl", "power", isl.btPowered ? "off" : "on"]; btSet.running = true; } },
                                { label: "Bluetooth settings", icon: "gear", run: () => isl.settingsRequested("bluetooth") }
                            ])
                            Icon { anchors.centerIn: parent; name: "bt"; size: 18 * isl.k; color: !isl.btPowered ? Theme.muted : isl.btConnected.length ? Theme.accent : Theme.text }
                        }
                        IslandButton {   // volume
                            id: vol
                            visible: isl.audio !== null
                            tip: (isl.audio && isl.audio.muted ? "Muted" : "Volume " + isl.volPct + "%") + "\nScroll to change, middle-click to mute"
                            onClicked: isl.quickRequested()
                            onMiddleClicked: if (isl.audio) isl.audio.muted = !isl.audio.muted
                            onRightClicked: isl.soundMenu(vol)
                            onScrolled: delta => { if (isl.audio) isl.audio.volume = Math.max(0, Math.min(Buttons.maxVolume, isl.audio.volume + (delta > 0 ? 0.05 : -0.05))); }
                            Icon { anchors.centerIn: parent; size: 19 * isl.k; name: !isl.audio || isl.audio.muted ? "vol-mute" : isl.volPct > 33 ? "vol" : "vol-low" }
                        }
                        IslandButton {   // battery
                            id: battBtn
                            visible: isl.hasBatt
                            wideLabel: isl.vertical ? "" : isl.battPct + "%"
                            tip: isl.battPct + "%" + (isl.charging ? ", charging" : "")
                            onClicked: isl.quickRequested()
                            onRightClicked: isl.statusMenu(battBtn, battBtn.tip, [{ label: "Power settings", icon: "power", run: () => isl.settingsRequested("power") }])
                            Icon { anchors.centerIn: parent; size: 19 * isl.k; name: isl.charging ? "batt-charging" : isl.battPct > 20 ? "batt" : "batt-low" }
                        }
                        IslandButton {   // notifications
                            visible: !isl.compact || DeskState.unseen > 0
                            tip: DeskState.dnd ? "Do not disturb is on" : DeskState.unseen > 0 ? DeskState.unseen + (DeskState.unseen === 1 ? " new notification" : " new notifications") : "Notifications"
                            badge: DeskState.dnd ? 0 : DeskState.unseen
                            onClicked: isl.calendarRequested()
                            onRightClicked: DeskState.setDnd(!DeskState.dnd)
                            Icon { anchors.centerIn: parent; name: DeskState.dnd ? "bell-off" : "bell"; size: 19 * isl.k; color: DeskState.dnd ? Theme.muted : Theme.text }
                        }
                    }
                }

                // ---- Game Mode: always here, in every look and on every edge, the biggest target in the bar
                GameModeButton { Layout.preferredHeight: isl.vertical ? isl.islandH - 6 : isl.style === "bar" ? isl.islandH - 8 : isl.islandH; Layout.preferredWidth: isl.vertical ? isl.islandH - 6 : implicitWidth }
            }

            // ---- the centre pill: the clock, what's playing, the newest notification
            Island {
                id: pill
                visible: isl.pillCentred
                lit: isl.calendarOpen
                anchors.horizontalCenter: parent.horizontalCenter
                height: parent.height
                width: pillRow.implicitWidth + 28
                rad: Theme.r(height / 2)
                Behavior on width { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: isl.calendarRequested() }
                RowLayout {
                    id: pillRow
                    anchors.centerIn: parent
                    spacing: 12
                    // the newest notification, for a few seconds
                    RowLayout {
                        visible: isl.flash !== null
                        spacing: 8
                        AppIcon { size: 22; genericFallback: false; visible: hasIcon; icon: isl.flash ? (isl.flash.icon || "") : ""; appId: isl.flash ? (isl.flash.entry || "") : "" }
                        Text {
                            Layout.maximumWidth: isl.compact ? 200 : 320
                            text: isl.flash ? [isl.flash.app, isl.flash.summary].filter(Boolean).join(" · ") : ""
                            elide: Text.ElideRight
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                        }
                    }
                    Loader {
                        id: media
                        visible: isl.flash === null && status === Loader.Ready && !!item && item.playing
                        active: isl.pillCentred
                        source: "MediaPill.qml"
                        onLoaded: item.compact = Qt.binding(() => isl.compact)
                    }
                    Rectangle { visible: media.visible; implicitWidth: 1; implicitHeight: 22; color: Theme.line }
                    Text {
                        text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : "h:mm AP")
                        color: Theme.text
                        font.family: isl.gaming || Theme.look === "synthwave" ? "DejaVu Sans Mono" : Theme.font
                        font.pixelSize: Theme.thin ? 13 : 15; font.weight: Font.DemiBold
                        font.letterSpacing: isl.gaming ? 1.5 : 0
                    }
                    Text {
                        visible: !media.visible && isl.flash === null && !isl.compact
                        text: Qt.formatDate(clock.date, "ddd d MMM")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                    }
                    Rectangle {   // play / pause
                        visible: media.visible
                        implicitWidth: isl.islandH - 14; implicitHeight: implicitWidth
                        radius: Theme.r(implicitWidth / 2)
                        color: ppArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
                        Text { anchors.centerIn: parent; text: "❚❚"; color: Theme.accentInk; font.pixelSize: 12 }
                        MouseArea { id: ppArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: if (media.item) media.item.toggle() }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks
    // An island: the look's frame (Frame.qml) when floating; nothing of its own inside a full-width bar.
    // `along` is its length along the bar; across, it fills the bar.
    component Island: Item {
        property bool lit: false
        property real along: 0
        property real rad: isl.gaming ? 0 : Theme.r(isl.islandH / 2.4)
        Layout.fillHeight: !isl.vertical
        Layout.fillWidth: isl.vertical
        Layout.preferredWidth: isl.vertical ? (isl.islandH || 52) : along
        Layout.preferredHeight: isl.vertical ? along : (isl.islandH || 52)
        Frame { anchors.fill: parent; flat: !isl.floating; lit: parent.lit; rad: parent.rad }
    }

    // Back to Game Mode: dressed per look, never hidden.
    component GameModeButton: Item {
        id: gmb
        objectName: "gameModeButton"
        implicitWidth: isl.vertical ? isl.islandH - 6 : gmRow.implicitWidth + (isl.shelf ? 56 : 36)
        Layout.alignment: Qt.AlignCenter
        readonly property bool white: Theme.look === "glass"
        Repeater {   // Cabin: an ember glow around it
            model: Theme.look === "cabin" ? 3 : 0
            delegate: Rectangle {
                required property int index
                anchors.fill: parent; anchors.margins: -(index + 1) * 3
                radius: Theme.r(gmb.height / 2) + (index + 1) * 3
                color: "transparent"; border.width: 3
                border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25 - index * 0.07)
            }
        }
        Rectangle {
            anchors.fill: parent
            visible: !isl.gaming
            radius: isl.style === "bar" ? Theme.r(6) : Theme.r(height / 2)
            color: gmb.white ? (gmArea.containsMouse ? "#ffffff" : "#f2f6ff") : gmArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
        }
        Rectangle {   // Synthwave: pink chrome
            anchors.fill: parent
            visible: Theme.look === "synthwave"
            radius: Theme.r(height / 2)
            opacity: gmArea.containsMouse ? 0.85 : 1
            gradient: Gradient { GradientStop { position: 0; color: "#ff9bd5" } GradientStop { position: 0.48; color: "#ff3fa4" } GradientStop { position: 0.52; color: "#c81d7e" } GradientStop { position: 1; color: "#ff5fb4" } }
        }
        Canvas {   // Gaming: a cut-corner button, cyan to magenta
            id: gmCanvas
            anchors.fill: parent
            visible: isl.gaming
            readonly property string key: Theme.accent + "/" + Theme.flame + "/" + width + "x" + height + "/" + gmArea.containsMouse
            onKeyChanged: requestPaint()
            onVisibleChanged: if (visible) requestPaint()
            onPaint: {
                const c = getContext("2d"), w = width, h = height, k = Math.min(12, h / 3);
                c.reset();
                if (w <= 0 || h <= 0) return;
                const g = c.createLinearGradient(0, 0, w, 0);
                g.addColorStop(0, Theme.css(Theme.accent, 1)); g.addColorStop(1, Theme.css(Theme.flame, 1));
                c.beginPath(); c.moveTo(k, 0); c.lineTo(w, 0); c.lineTo(w, h - k); c.lineTo(w - k, h); c.lineTo(0, h); c.lineTo(0, k); c.closePath();
                c.fillStyle = g; c.fill();
                if (gmArea.containsMouse) { c.fillStyle = "rgba(255,255,255,0.15)"; c.fill(); }
            }
        }
        RowLayout {
            id: gmRow
            anchors.centerIn: parent
            spacing: 8
            Icon { name: "pad"; size: 20 * isl.k; color: gmb.white ? "#17203a" : Theme.accentInk }
            Text {
                visible: !isl.vertical   // down a side: just the controller icon, in the accent
                text: isl.gaming ? "GAME MODE" : "Game Mode"
                color: gmb.white ? "#17203a" : Theme.accentInk
                font.family: Theme.font; font.bold: true; font.pixelSize: Math.round((Theme.thin ? 12 : 14) * isl.k)
                font.letterSpacing: isl.gaming ? 2 : 0
            }
        }
        MouseArea { id: gmArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Quickshell.execDetached(["nimbus-mode", "game"]) }
    }

    component IslandButton: Item {
        id: btn
        property string tip: ""
        property bool narrow: false
        property int badge: 0
        property string wideLabel: ""
        default property alias content: holder.data
        signal clicked()
        signal middleClicked()
        signal rightClicked()
        signal scrolled(int delta)
        readonly property real side: isl.shelf ? 60 : Math.max(26, isl.islandH - 12)
        implicitWidth: wideLabel !== "" ? side + wl.implicitWidth + 4 : narrow ? side * 0.8 : side
        implicitHeight: side
        Layout.alignment: Qt.AlignCenter
        Rectangle {
            anchors.fill: parent
            radius: isl.shelf ? height / 2 : Theme.r(10)
            color: area.containsMouse ? Theme.panel3 : isl.shelf ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.6) : "transparent"
        }
        Item { id: holder; width: btn.side; height: btn.side }
        Text {
            id: wl
            visible: btn.wideLabel !== ""
            anchors { left: holder.right; verticalCenter: parent.verticalCenter; leftMargin: -6 }
            text: btn.wideLabel; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(13 * isl.k)
        }
        Rectangle {
            visible: btn.badge > 0
            x: btn.side / 2 + 2; y: 3
            width: Math.max(16, badgeText.implicitWidth + 8); height: 16; radius: 8
            color: Theme.accent
            border.color: Theme.panel; border.width: 1
            Text { id: badgeText; anchors.centerIn: parent; text: btn.badge > 9 ? "9+" : String(btn.badge); color: Theme.accentInk; font.pixelSize: 10; font.bold: true; font.family: Theme.font }
        }
        MouseArea {
            id: area
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
            onClicked: mouse => {
                if (mouse.button === Qt.MiddleButton) btn.middleClicked();
                else if (mouse.button === Qt.RightButton) btn.rightClicked();
                else btn.clicked();
            }
            onWheel: wheel => btn.scrolled(wheel.angleDelta.y)
        }
        PopupWindow {
            visible: area.containsMouse && btn.tip !== ""
            anchor.item: btn
            anchor.rect.x: isl.edge === "left" ? btn.width + 12 : isl.edge === "right" ? -implicitWidth - 12 : Math.round(btn.width / 2 - implicitWidth / 2)
            anchor.rect.y: isl.edge === "bottom" ? -implicitHeight - 12 : isl.vertical ? Math.round(btn.height / 2 - implicitHeight / 2) : Math.round(btn.height + 12)
            implicitWidth: tipText.implicitWidth + 20
            implicitHeight: tipText.implicitHeight + 12
            color: "transparent"
            Rectangle {
                anchors.fill: parent
                radius: Theme.r(8)
                color: Theme.panel2
                border.color: Theme.line
                Text { id: tipText; anchors.centerIn: parent; text: btn.tip; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
            }
        }
    }
}
