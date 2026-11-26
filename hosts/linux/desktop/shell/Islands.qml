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
// Drag and drop board 2 (the desktop editor, DesktopEditor.qml): every block in its look (BlockLook.qml, bar.json
// "s:<block id>"), and the bar's own settings (bar.json "bar", widget-tools.js barLook): its edge or anywhere on the
// screen, its length and offset from the corner, thickness, corners, fill, see-through, blur, the gap between blocks,
// how they line up, islands or one strip, hide until pointed at, over windows or keeping their space, which screen.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower
import "widget-tools.js" as T
import "block-look.js" as BL

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
    // the bar's own settings (drag and drop board 2); nothing set: as the look and Settings have it
    readonly property var mb: BarLayout.mainBar
    readonly property bool free: mb.edge === "free"
    readonly property bool onThisScreen: T.onScreen(mb.screen, screen ? screen.name : "", Quickshell.screens.map(s => s.name))
    readonly property bool fullLength: !free && mb.len === "full"
    readonly property bool centredAlong: !free && !fullLength && mb.offset < 0
    readonly property real screenAlong: screen ? (vertical ? screen.height : screen.width) : 1920
    readonly property real fitLength: (vertical ? mainRow.implicitHeight : mainRow.implicitWidth) + (pillCentred ? pill.width + 24 : 0) + (centred ? centreIsland.width + 24 : 0) + inner.along * 2 + 16
    readonly property real alongLen: T.lengthOf(mb.len === "full" && free ? "fit" : mb.len, mb.lenV, screenAlong, fitLength)
    readonly property string style: mb.islands === "islands" ? "floating" : mb.islands === "strip" ? "bar"
        : vertical && Theme.mainStyle === "shelf" ? "bar" : Theme.mainStyle   // floating | bar | shelf
    readonly property bool floating: style === "floating"
    readonly property bool shelf: style === "shelf"
    readonly property bool compact: screen && screen.width <= 1440 && !shelf
    readonly property real k: shelf ? 1.4 : 1
    readonly property int islandH: mb.thick > 0 ? mb.thick : shelf ? 72 : vertical ? (compact ? 52 : 56) : Theme.thin ? 32 : style === "bar" ? 44 : compact ? 48 : 52
    readonly property int gapOut: floating ? 10 : shelf ? 18 : 0      // from the screen's edge
    readonly property int gapIn: floating ? 6 : shelf ? 10 : 0        // toward the windows
    readonly property int gapTop: gapOut                                // (the old name, kept for the look's parts)
    readonly property int band: islandH + gapOut + gapIn
    readonly property bool gaming: Theme.look === "gaming"
    // The clock in the middle of a horizontal bar; in a bar with your apps, or down a side, a compact clock at the end.
    readonly property bool pillCentred: !vertical && !holdsApps && !shelf && !centred && Settings.clockAt === ""
    // a layout's clock at the end, after the status island (like other desktops); "end-one": one line
    readonly property bool clockLast: Settings.clockAt === "end" || Settings.clockAt === "end-one"
    // a layout's Show desktop strip at the very end, after Game Mode (Plasma-, Windows-style)
    readonly property bool showLast: { const e = BarLayout.layout.end || []; return e.length > 0 && e[e.length - 1] === "showdesktop"; }
    // Desktop layouts (hearth-layout): blocks in the middle of a bar across the top or bottom (centred, the
    // flame, Search, Task view and your apps, centred). Down a side they go at the start.
    readonly property bool centred: !vertical && !shelf && (BarLayout.layout.centre || []).length > 0

    // ---- auto-hide
    readonly property bool autoHide: (mb.autoHide === "on" || (mb.autoHide === "" && Theme.barAutoHide)) && !shelf
    readonly property bool hiddenNow: autoHide && !hov.hovered && !DeskState.dockPeek && !holdOpen.running && !quickOpen && !calendarOpen && !DeskState.overlayShown
    readonly property int reserved: autoHide || free || mb.over || !onThisScreen ? 0 : band
    Timer { id: holdOpen; interval: 700 }

    // on its edge, full length or shorter at an offset (or centred); anywhere: from the top-left corner
    anchors {
        top: free || edge === "top" || (vertical && (fullLength || !centredAlong))
        bottom: !free && (edge === "bottom" || (vertical && fullLength))
        left: free || edge === "left" || (!vertical && (fullLength || !centredAlong))
        right: !free && (edge === "right" || (!vertical && fullLength))
    }
    margins.left: free && screen ? Math.round(mb.fx * Math.max(0, screen.width - implicitWidth)) : !free && !vertical && !fullLength && !centredAlong ? Math.min(mb.offset, Math.max(0, screenAlong - alongLen)) : 0
    margins.top: free && screen ? Math.round(mb.fy * Math.max(0, screen.height - implicitHeight)) : !free && vertical && !fullLength && !centredAlong ? Math.min(mb.offset, Math.max(0, screenAlong - alongLen)) : 0
    implicitHeight: vertical ? (fullLength ? 0 : alongLen) : band
    implicitWidth: vertical ? band : (fullLength ? 0 : alongLen)
    exclusiveZone: reserved
    color: "transparent"
    WlrLayershell.namespace: "hearth-islands"
    WlrLayershell.layer: mb.layer === "bottom" ? WlrLayer.Bottom : WlrLayer.Top
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
    visible: settingsReady && !remapping && !SettingsNav.fireholdDesk && !SettingsNav.fireholdCouch && onThisScreen   // Firehold's Desk has its own bar (FireholdDesk.qml), its Couch none; a bar set to another screen isn't drawn here
    // the desktop you're on, for blocks shown only on some desktops (DeskNow.qml; without it they show on every one)
    Loader { active: isl.primary; source: "DeskNow.qml"; onStatusChanged: if (status === Loader.Error) console.warn("Mantel: the desktop you're on can't be told (DeskNow.qml); blocks show on every desktop") }
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
        const p0 = menuPoint(btn);
        const sink = Pipewire.defaultAudioSink;
        const outs = Pipewire.nodes ? Pipewire.nodes.values.filter(n => n.audio && n.isSink && !n.isStream) : [];
        const list = outs.slice(0, 8).map(n => ({
            label: n.description || n.nickname || n.name || "Speakers", checked: n === sink ? true : undefined, icon: "vol",
            run: () => { Pipewire.preferredDefaultAudioSink = n; }
        }));
        if (list.length) list.push({ separator: true });
        const a = sink ? sink.audio : null;
        if (a) list.push({ label: a.muted ? "Unmute" : "Mute", icon: a.muted ? "vol" : "vol-mute", run: () => { a.muted = !a.muted; } });
        if (SettingsNav.mixer) list.push({ label: "Volume mixer", icon: "vol", run: () => SettingsNav.mixer.openNear(screen, p0.x + (menu ? menu.cardWidth / 2 : 134), p0.y, edge === "bottom") });
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
    // 1.16: the network and Bluetooth from NetBtWatch.qml, read when they change (no polling).
    readonly property string netKind: NetBtWatch.netKind
    readonly property string netName: NetBtWatch.netName
    readonly property bool btPresent: NetBtWatch.btPresent
    readonly property bool btPowered: NetBtWatch.btPowered
    readonly property var btConnected: NetBtWatch.btConnected
    Process { id: btSet; onExited: NetBtWatch.refreshBt() }
    readonly property var front: WinList.activeToplevel
    SystemClock { id: clock; precision: SystemClock.Minutes }

    // The newest notification that popped up, shown in the pill for a few seconds (1.14: DeskState.popped, so quiet
    // ones, Do Not Disturb and automatic quiet stay quiet here too).
    property var flash: null
    Connections {
        target: DeskState
        function onPopped(n) { isl.flash = n; flashTimer.restart(); }
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

            // Right-click the bar itself (between its islands): arrange the bar's items, add a panel (1.14, BarArrange.qml).
            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.RightButton
                onClicked: mouse => isl.statusMenu(inner, "The bar", [
                    { label: "Arrange the bar…", icon: "grid", run: () => BarLayout.arranging = true },
                    { label: "Add a panel…", icon: "plus", run: () => { BarLayout.arranging = true; BarLayout.arrangeTab = "panels"; } },
                    { label: "Describe it…", icon: "sparkle", run: () => Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "describe", "open", "bar"]) },
                    { label: "Desktop layout…", icon: "desktops", run: () => isl.settingsRequested("desktop-layout") },
                    { label: "Bar and dock settings", icon: "gear", run: () => isl.settingsRequested("bar-dock") }
                ])
            }
            // One full-width bar (Steam-style, Zen, and One main bar in those looks): a flat strip along the edge.
            Rectangle {
                id: strip
                anchors.fill: parent
                visible: isl.style === "bar"
                // its fill, see-through and corners (the bar's own settings), else the look's
                color: isl.mb.fill !== "" ? BL.argb(isl.mb.fill, 1 - isl.mb.see / 100)
                    : isl.mb.see > 0 ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 1 - isl.mb.see / 100) : Theme.glass
                radius: isl.mb.corners >= 99 ? Math.min(width, height) / 2 : isl.mb.corners >= 0 ? isl.mb.corners : (isl.free || !isl.fullLength ? Theme.r(10) : 0)
                Loader { z: -1; anchors.fill: parent; active: isl.mb.blur && isl.style === "bar"; source: "Frost.qml"; onLoaded: item.rad = Qt.binding(() => strip.radius) }
                Rectangle {   // the line on the side toward the windows
                    visible: isl.fullLength && !isl.free
                    color: Theme.line
                    width: isl.vertical ? 1 : parent.width
                    height: isl.vertical ? parent.height : 1
                    x: isl.edge === "left" ? parent.width - 1 : 0
                    y: isl.edge === "top" ? parent.height - 1 : 0
                }
                Loader { anchors.fill: parent; active: Theme.frame === "frost" && isl.style === "bar" && !isl.mb.blur && isl.mb.fill === ""; source: "Frame.qml" }
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
                rowSpacing: isl.mb.gap >= 0 ? isl.mb.gap : isl.floating ? 10 : 6
                columnSpacing: isl.mb.gap >= 0 ? isl.mb.gap : isl.floating ? 10 : 6

                // ---- line up (the bar's own setting): start, centre or end put the room before and after
                Item { Layout.fillWidth: !isl.vertical && (isl.mb.lineup === "end" || isl.mb.lineup === "centre"); Layout.fillHeight: isl.vertical && (isl.mb.lineup === "end" || isl.mb.lineup === "centre") }

                // ---- Game Mode first (a layout's choice)
                Loader { active: Settings.gameModeAt === "start"; visible: active && !Settings.gameModeOff; sourceComponent: cGameMode; Layout.alignment: Qt.AlignCenter; Layout.preferredHeight: isl.vertical ? isl.islandH - 6 : isl.style === "bar" ? isl.islandH - 8 : isl.islandH; Layout.preferredWidth: isl.vertical ? isl.islandH - 6 : (item ? item.implicitWidth : 0) }

                // ---- start island: the flame, the desktops, the window in front
                Island {
                    id: leftIsland
                    visible: isl.zoneItems("start").length > 0
                    along: (isl.vertical ? leftRow.implicitHeight : leftRow.implicitWidth) + (isl.floating ? 16 : 4)
                    grows: isl.zoneFills("start")
                    GridLayout {
                        id: leftRow
                        anchors.centerIn: leftIsland.grows ? undefined : parent
                        anchors.fill: leftIsland.grows ? parent : undefined
                        flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
                        rows: isl.vertical ? -1 : 1
                        columns: isl.vertical ? 1 : -1
                        rowSpacing: isl.mb.gap >= 0 ? isl.mb.gap : 8 * isl.k
                        columnSpacing: isl.mb.gap >= 0 ? isl.mb.gap : 8 * isl.k
                        // 1.14: the bar's own items in your order (Arrange the bar: BarArrange.qml, BarLayout.qml)
                        Repeater { model: isl.zoneItems("start"); delegate: BarSlot {} }
                    }
                }

                // ---- your apps (floating app menu off): the dock attached to the bar
                Island {
                    visible: isl.holdsApps && !isl.centred && apps.status === Loader.Ready
                    along: apps.item ? (isl.vertical ? apps.item.implicitHeight : apps.item.implicitWidth) + (isl.floating ? 16 : 4) : 0
                    Loader {
                        id: apps
                        anchors.centerIn: parent
                        active: isl.holdsApps && !isl.centred
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

                // ---- widgets you put in the bar (1.14, BarWidgets.qml; Add widgets › Add to bar)
                Island {
                    visible: !isl.vertical && barWidgets.status === Loader.Ready && !!barWidgets.item && barWidgets.item.wanted
                    along: barWidgets.item ? barWidgets.item.implicitWidth + 28 : 0
                    Loader { id: barWidgets; anchors.centerIn: parent; active: !isl.vertical; source: "BarWidgets.qml" }
                }

                Item { Layout.fillWidth: !isl.vertical && isl.mb.lineup === "spread"; Layout.fillHeight: isl.vertical && isl.mb.lineup === "spread" }

                // ---- the compact clock (with your apps in the bar, or down a side): click for the dashboard
                Loader { active: !isl.pillCentred && !isl.shelf && !isl.clockLast; visible: active; sourceComponent: cClock; Layout.fillHeight: !isl.vertical; Layout.fillWidth: isl.vertical; Layout.preferredWidth: isl.vertical ? (isl.islandH || 52) : (item ? item.along : 0); Layout.preferredHeight: isl.vertical ? (item ? item.along : 0) : (isl.islandH || 52) }

                // ---- Game Mode before the status island (a layout's choice)
                Loader { active: Settings.gameModeAt === "before-status"; visible: active && !Settings.gameModeOff; sourceComponent: cGameMode; Layout.alignment: Qt.AlignCenter; Layout.preferredHeight: isl.vertical ? isl.islandH - 6 : isl.style === "bar" ? isl.islandH - 8 : isl.islandH; Layout.preferredWidth: isl.vertical ? isl.islandH - 6 : (item ? item.implicitWidth : 0) }

                // ---- the status island
                Island {
                    id: statusIsland
                    lit: isl.quickOpen
                    pill: Settings.statusPill === "on"
                    along: (isl.vertical ? statusRow.implicitHeight : statusRow.implicitWidth) + (isl.floating ? 16 : 4)
                    grows: isl.zoneFills("end")
                    MouseArea { anchors.fill: parent; onClicked: isl.quickRequested() }   // the island's gaps open Quick settings too
                    GridLayout {
                        id: statusRow
                        anchors.centerIn: statusIsland.grows ? undefined : parent
                        anchors.fill: statusIsland.grows ? parent : undefined
                        flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
                        rows: isl.vertical ? -1 : 1
                        columns: isl.vertical ? 1 : -1
                        rowSpacing: isl.mb.gap >= 0 ? isl.mb.gap : isl.shelf ? 12 : 2
                        columnSpacing: isl.mb.gap >= 0 ? isl.mb.gap : isl.shelf ? 12 : 2
                        Repeater { model: isl.zoneItems("end"); delegate: BarSlot {} }
                    }
                }

                // ---- Game Mode: always here, in every look and on every edge, the biggest target in the bar (last, unless
                // a layout put it first or before the status island: one of the three is always on)
                // ---- a layout's clock at the end
                Loader { active: !isl.pillCentred && !isl.shelf && isl.clockLast; visible: active; sourceComponent: cClock; Layout.fillHeight: !isl.vertical; Layout.fillWidth: isl.vertical; Layout.preferredWidth: isl.vertical ? (isl.islandH || 52) : (item ? item.along : 0); Layout.preferredHeight: isl.vertical ? (item ? item.along : 0) : (isl.islandH || 52) }

                Loader { active: Settings.gameModeAt !== "start" && Settings.gameModeAt !== "before-status"; visible: active && !Settings.gameModeOff; sourceComponent: cGameMode; Layout.alignment: Qt.AlignCenter; Layout.preferredHeight: isl.vertical ? isl.islandH - 6 : isl.style === "bar" ? isl.islandH - 8 : isl.islandH; Layout.preferredWidth: isl.vertical ? isl.islandH - 6 : (item ? item.implicitWidth : 0) }
                // ---- a layout's Show desktop strip, last
                Loader { active: isl.showLast; visible: active; sourceComponent: cBlock; Layout.alignment: Qt.AlignCenter; onLoaded: item.kind = "showdesktop" }
                Item { Layout.fillWidth: !isl.vertical && (isl.mb.lineup === "start" || isl.mb.lineup === "centre"); Layout.fillHeight: isl.vertical && (isl.mb.lineup === "start" || isl.mb.lineup === "centre") }
            }

            // ---- the middle of the bar (desktop layouts): the blocks put there, then your apps when the bar holds them
            Island {
                id: centreIsland
                visible: isl.centred
                anchors.horizontalCenter: parent.horizontalCenter
                height: parent.height
                width: centreRow.implicitWidth + (isl.floating ? 16 : 4)
                RowLayout {
                    id: centreRow
                    anchors.centerIn: parent
                    spacing: isl.mb.gap >= 0 ? isl.mb.gap : 8 * isl.k
                    Repeater { model: isl.centred ? isl.zoneItems("centre") : []; delegate: BarSlot {} }
                    Loader {
                        id: centreApps
                        Layout.alignment: Qt.AlignCenter
                        active: isl.holdsApps && isl.centred
                        source: "TaskStrip.qml"
                        onLoaded: {
                            item.menu = Qt.binding(() => isl.menu);
                            item.screen = Qt.binding(() => isl.screen);
                            item.vertical = false;
                            item.edge = Qt.binding(() => isl.edge);
                            item.side = Qt.binding(() => Math.max(30, isl.islandH - (isl.floating ? 10 : 8)));
                            item.barW = Qt.binding(() => isl.width);
                            item.barH = Qt.binding(() => isl.height);
                            item.primary = Qt.binding(() => isl.primary);
                        }
                        onStatusChanged: if (status === Loader.Error) console.warn("Mantel: your apps couldn't load in the middle of the bar (TaskStrip.qml)")
                    }
                }
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
                // (1.16: not while nobody's there: the clock's minute changed its width, and the animation drew ~20 frames a minute)
                Behavior on width { enabled: !Settings.reduceMotion && !DeskState.idle; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: isl.calendarRequested() }
                RowLayout {
                    id: pillRow
                    anchors.centerIn: parent
                    spacing: 12
                    // your features in the bar (Describe it: Features.qml)
                    Repeater { model: Features.inPart("bar"); FeaturePill { required property var modelData; feature: modelData; side: isl.islandH - 16 } }
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

    // ------------------------------------------------------------------ the bar's own items (1.14)
    // Each can go at the start or the end of the bar, in any order, or be hidden (BarLayout.qml; Settings › Bar and
    // dock › Arrange the bar, or right-click the bar). Game Mode always stays, last.
    function zoneItems(z) {
        // down a side, a layout's middle blocks go at the start
        const l = z === "start" && !isl.centred ? (BarLayout.layout.start || []).concat(isl.vertical || isl.shelf ? (BarLayout.layout.centre || []) : [])
            : z === "end" && isl.showLast ? (BarLayout.layout.end || []).slice(0, -1)
            : BarLayout.layout[z] || [];
        const out = [];
        for (const id of l) {
            out.push(id);
            if (id === "menu" && isl.shelf) out.push("shelfclock");    // Console: the big clock sits next to the mark
        }
        return out;
    }
    // a block in this part of the bar set to Fill (drag and drop board 2: Width › Fill)
    function zoneFills(z) { return zoneItems(z).some(id => BL.normalize(BarLayout.lookOf(id)).w === "fill"); }
    function compFor(id0) {
        const id = T.baseId(id0);
        if (id.indexOf("panel:") === 0) return cNested;   // a panel inside the bar (NestedPanel.qml)
        switch (id) {
        case "menu": return cMenu;
        case "search": return cSearch;
        case "shelfclock": return cShelfClock;
        case "desktops": return cDesks;
        case "activity": return cActivity;
        case "window": return cWindow;
        case "jobs": return cJobs;
        case "tray": return cTray;
        case "background": return cBackground;
        case "privacy": return cPrivacy;
        case "printers": return cPrinters;
        case "disks": return cDisks;
        case "discord": return cDiscord;
        case "keyboard": return cKeyboard;
        case "layout": return cLayout;
        case "network": return cNetwork;
        case "bluetooth": return cBluetooth;
        case "volume": return cVolume;
        case "battery": return cBattery;
        case "notifications": return cNotifications;
        case "overview": return cBlock;
        case "taskview": return cBlock;
        case "appname": return cBlock;
        case "appmenus": return cBlock;
        case "quickaccess": return cBlock;
        case "showdesktop": return cBlock;
        case "power": return cBlock;
        }
        if (id.indexOf("own:") === 0) return cOwn;   // a block you made (OwnBarBlock.qml)
        return null;
    }
    // a block of the bar in its look (BlockLook.qml: drag and drop board 2)
    component BarSlot: BlockLook {
        id: slot
        required property string modelData
        bid: modelData
        vert: isl.vertical
        side: isl.islandH
        screenName: isl.screen ? isl.screen.name : ""
        Layout.alignment: Qt.AlignCenter
        Layout.fillWidth: wantsFill && !isl.vertical
        Layout.fillHeight: wantsFill && isl.vertical
        visible: shownNow && slotLd.status === Loader.Ready && !!slotLd.item && slotLd.item.wanted !== false
        Loader {
            id: slotLd
            sourceComponent: isl.compFor(slot.modelData)
            onLoaded: {
                if (item && item.objectName === "layoutBlock") item.kind = T.baseId(slot.modelData);
                if (item && item.objectName === "ownBlock") item.kind = slot.modelData;
                if (item && item.objectName === "nestedPanel") item.kind = slot.modelData;
            }
            onStatusChanged: if (status === Loader.Error) console.warn("Mantel: the bar's " + slot.modelData + " couldn't load")
        }
    }
    // a panel inside the main bar (NestedPanel.qml, by file name: a problem there only loses the panel inside)
    Component {
        id: cNested
        Loader {
            objectName: "nestedPanel"
            property string kind: ""
            readonly property bool wanted: status === Loader.Ready && !!item && item.wanted
            source: "NestedPanel.qml"
            onLoaded: {
                item.pid = Qt.binding(() => kind.indexOf("panel:") === 0 ? kind.slice(6) : "");
                item.hostSize = Qt.binding(() => Math.max(26, isl.islandH - 8));
                item.hostVertical = Qt.binding(() => isl.vertical);
                item.hostEdge = Qt.binding(() => isl.edge);
                item.hostScreen = Qt.binding(() => isl.screen);
            }
            onStatusChanged: if (status === Loader.Error) console.warn("Mantel: a panel inside the bar couldn't load (NestedPanel.qml)")
        }
    }

    // Desktop layouts' own blocks (LayoutBlock.qml: the same in the extra panels)
    Component { id: cOwn; OwnBarBlock { objectName: "ownBlock"; side: Math.max(26, isl.islandH - 12); vert: isl.vertical } }
    Component {
        id: cBlock
        LayoutBlock {
            objectName: "layoutBlock"
            side: Math.max(26, isl.islandH - 12)
            vertical: isl.vertical
            edge: isl.edge
            menu: isl.menu
        }
    }
    // the compact clock (a component, so a layout can put it at the end)
    Component {
        id: cClock
        Island {
            id: clockIsland
            lit: isl.calendarOpen
            along: (isl.vertical ? clockCol.implicitHeight : clockCol.implicitWidth) + (isl.vertical ? 16 : 24)
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: isl.calendarRequested() }
            ColumnLayout {
                id: clockCol
                anchors.centerIn: parent
                spacing: 0
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: (Settings.clockAt === "end-one" && !isl.vertical ? Qt.formatDate(clock.date, "ddd d MMM") + "  " : "") + Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : (isl.vertical ? "h:mm" : "h:mm AP"))
                    color: Theme.text; font.family: Theme.font; font.weight: Font.DemiBold
                    font.pixelSize: isl.vertical ? 14 : (isl.islandH > 40 ? 14 : 13)
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    visible: Settings.clockAt === "end-one" && !isl.vertical ? false : isl.vertical ? !Settings.clock24 : isl.islandH > 40
                    text: isl.vertical ? Qt.formatTime(clock.date, "AP") : Qt.formatDate(clock.date, "ddd d MMM")
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                }
            }
        }
    }
    Component {
        id: cGameMode
        GameModeButton {}
    }
    Component {
        id: cMenu
        IslandButton {
            objectName: "appMenuButton"
            id: startMenuBtn
            tip: "Apps (Super)"
            onClicked: isl.launcherRequested()
            onRightClicked: if (SettingsNav.winTools) SettingsNav.winTools.winxFrom(isl.screen, startMenuBtn, isl.edge)   // the Super+X menu
            Image { anchors.centerIn: parent; width: 26 * isl.k; height: width; source: Qt.resolvedUrl("images/hearth-logo.png"); smooth: true; mipmap: true }
        }
    }
    Component {
        id: cSearch
        Rectangle {   // the Tiles menu's search field: click to search (what you type shows here)
            objectName: "menuSearchField"
            // the Tiles menu's field; with a desktop layout that put Search in the bar, every menu's
            readonly property bool wanted: isl.menuSearchShown || (Settings.layout !== "" && !isl.vertical && !isl.shelf && !isl.compact)
            // at the end of the bar only its magnifier
            readonly property bool iconOnly: (BarLayout.layout.end || []).indexOf("search") >= 0
            implicitWidth: iconOnly ? Math.max(26, isl.islandH - 12) : 300; implicitHeight: Math.max(26, isl.islandH - 14)
            radius: Theme.r(8)
            color: msa.containsMouse ? Theme.panel3 : iconOnly ? "transparent" : Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.85)
            border.width: iconOnly ? 0 : isl.menuQuery !== "" ? 2 : 1
            border.color: isl.menuQuery !== "" ? Theme.accent : Theme.line
            Icon { x: parent.iconOnly ? (parent.width - width) / 2 : 12; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: parent.iconOnly ? 18 : 15; color: parent.iconOnly ? Theme.text : Theme.muted }
            Text {
                visible: !parent.iconOnly
                anchors { left: parent.left; right: parent.right; leftMargin: 36; rightMargin: 10; verticalCenter: parent.verticalCenter }
                text: isl.menuQuery !== "" ? isl.menuQuery : "Type to search"
                color: isl.menuQuery !== "" ? Theme.text : Theme.muted
                elide: Text.ElideLeft
                font.family: Theme.font; font.pixelSize: 14
            }
            MouseArea { id: msa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.IBeamCursor; onClicked: isl.menuSearchRequested() }
        }
    }
    Component {
        id: cShelfClock
        Text {   // Console: the big clock sits next to the mark
            readonly property bool wanted: isl.shelf
            text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : "h:mm")
            color: Theme.text; font.family: Theme.font; font.pixelSize: 38; font.weight: Font.DemiBold
            Text { anchors { left: parent.right; leftMargin: 6; baseline: parent.baseline } visible: !Settings.clock24; text: Qt.formatTime(clock.date, "AP"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 }
        }
    }
    Component {
        id: cDesks
        Loader {
            id: desks
            readonly property bool wanted: !isl.shelf && status === Loader.Ready && !!item && item.wanted
            active: !isl.shelf
            source: "DeskSwitcher.qml"
            onLoaded: { item.island = true; item.portrait = Qt.binding(() => isl.vertical); item.pillSize = Qt.binding(() => isl.islandH > 40 ? 24 : 20); }
        }
    }
    Component {
        id: cActivity
        // ---- Activities (1.14, Activities.qml): the one you're on, once you have more than one
        Loader {
            readonly property bool wanted: !isl.shelf && status === Loader.Ready && !!item && item.wanted
            active: !isl.shelf
            source: "ActivityChip.qml"
            onLoaded: item.pillSize = Qt.binding(() => isl.islandH > 40 ? 24 : 20)
        }
    }
    Component {
        id: cWindow
        RowLayout {   // the window in front (across the top or bottom, without your apps in the bar)
            readonly property bool wanted: !isl.compact && !isl.shelf && !isl.vertical && !isl.holdsApps && isl.front !== null && (isl.front.title || "") !== ""
            spacing: 8
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
    Component {
        id: cTray
        GridLayout {   // apps' own tray icons (StatusNotifierItem)
            readonly property bool wanted: trayRep.count > 0
            flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
            rows: isl.vertical ? -1 : 1
            columns: isl.vertical ? 1 : -1
            rowSpacing: isl.shelf ? 12 : 2
            columnSpacing: isl.shelf ? 12 : 2
            Repeater {
                id: trayRep
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
        }
    }
    // ---- copies and downloads (KDE's job progress, NotifyHub.qml): a ring with how far they are; click for the list
    Component {
        id: cJobs
        IslandButton {
            id: jobsBtn
            wanted: NotifyHub.runningJobs.length > 0
            tip: NotifyHub.runningJobs.map(j => j.title + (j.percent >= 0 ? " · " + j.percent + "%" : "")).join("\n") + "\nClick for details"
            onClicked: isl.calendarRequested()
            onRightClicked: isl.statusMenu(jobsBtn, "Copies and downloads", NotifyHub.runningJobs.map(j => ({
                label: (j.suspended ? "Resume " : "Pause ") + j.title, icon: j.suspended ? "play" : "pause", enabled: j.suspendable,
                run: () => NotifyHub.jobAction(j.id, j.suspended ? "resume" : "suspend") })).concat(NotifyHub.runningJobs.filter(j => j.killable).map(j => ({
                label: "Cancel " + j.title, icon: "close", run: () => NotifyHub.jobAction(j.id, "cancel") }))))
            Ring { anchors.centerIn: parent; size: 22 * isl.k; value: NotifyHub.jobsPercent }
            Icon { anchors.centerIn: parent; name: "sync"; size: 12 * isl.k; color: Theme.accent }
        }
    }
    // ---- apps running in the background (no window of their own): the portal's list, else Flatpak's
    Component {
        id: cBackground
        IslandButton {
            id: bgBtn
            wanted: NotifyHub.backgroundApps.length > 0 && !isl.compact
            badge: NotifyHub.backgroundApps.length > 1 ? NotifyHub.backgroundApps.length : 0
            tip: "Running in the background:\n" + NotifyHub.backgroundApps.map(a => isl.appLabel(a.app) + (a.message ? " · " + a.message : "")).join("\n")
            onClicked: isl.statusMenu(bgBtn, "Background apps", NotifyHub.backgroundApps.map(a => ({ label: "Quit " + isl.appLabel(a.app), icon: "close", run: () => NotifyHub.send({ op: "quitApp", app: a.app }) }))
                .concat([{ separator: true }, { label: "Apps settings", icon: "gear", run: () => isl.settingsRequested("apps") }]))
            onRightClicked: clicked()
            Icon { anchors.centerIn: parent; name: "rocket"; size: 18 * isl.k }
        }
    }
    function appLabel(id) { const e = id ? AppEntries.byId(id) : null; return e ? e.name : String(id || "An app").split(".").pop(); }
    // ---- the camera or the microphone in use: always shown while they are (never hidden by compact screens)
    Component {
        id: cPrivacy
        IslandButton {
            id: privBtn
            readonly property var cam: NotifyHub.cameraApps
            readonly property var micUsers: NotifyHub.micApps
            wanted: cam.length > 0 || micUsers.length > 0
            wideLabel: ""
            tip: (cam.length ? "Camera in use: " + cam.map(a => a.name).join(", ") : "") + (cam.length && micUsers.length ? "\n" : "")
                + (micUsers.length ? (NotifyHub.micMuted ? "Microphone (muted) wanted by: " : "Microphone in use: ") + micUsers.join(", ") : "")
            onClicked: isl.statusMenu(privBtn, "In use now", cam.map(a => ({ label: "Camera: " + a.name, icon: "cam", enabled: false }))
                .concat(micUsers.map(a => ({ label: "Microphone: " + a, icon: "mic", enabled: false })))
                .concat([{ separator: true }, { label: NotifyHub.micMuted ? "Unmute the microphone" : "Mute the microphone", icon: "mic", run: () => { if (NotifyHub.mic) NotifyHub.mic.muted = !NotifyHub.mic.muted; } },
                         { label: "Sound settings", icon: "gear", run: () => isl.settingsRequested("sound") }]))
            onRightClicked: clicked()
            Rectangle { anchors.centerIn: parent; width: privRow.implicitWidth + 10; height: 22 * isl.k; radius: height / 2; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22) }
            Row {
                id: privRow
                anchors.centerIn: parent
                spacing: 2
                Icon { visible: privBtn.cam.length > 0; name: "cam"; size: 15 * isl.k; color: Theme.accent }
                Icon { visible: privBtn.micUsers.length > 0; name: "mic"; size: 15 * isl.k; color: NotifyHub.micMuted ? Theme.muted : Theme.accent }
            }
        }
    }
    // ---- printing: while something is waiting to print
    Component {
        id: cPrinters
        IslandButton {
            id: prBtn
            wanted: NotifyHub.printJobs.length > 0
            badge: NotifyHub.printJobs.length
            tip: NotifyHub.printJobs.length === 1 ? "1 document printing on " + NotifyHub.printJobs[0].printer : NotifyHub.printJobs.length + " documents waiting to print"
            onClicked: isl.statusMenu(prBtn, "Printing", NotifyHub.printJobs.map(j => ({ label: "Cancel " + j.job + (j.size ? " (" + Math.max(1, Math.round(j.size / 1024)) + " KB)" : ""), icon: "close", run: () => NotifyHub.send({ op: "cancelPrint", job: j.job }) }))
                .concat([{ separator: true }, { label: "Printers", icon: "printer", run: () => isl.settingsRequested("printers") }]))
            onRightClicked: clicked()
            Icon { anchors.centerIn: parent; name: "printer"; size: 18 * isl.k }
        }
    }
    // ---- disks and devices: USB sticks, cards and other plugged-in drives (open, safely remove)
    Component {
        id: cDisks
        IslandButton {
            id: dkBtn
            wanted: NotifyHub.disks.length > 0
            tip: NotifyHub.disks.map(d => d.name + (d.size ? " · " + d.size : "") + (d.mount ? "" : " (not open)")).join("\n")
            onClicked: {
                const list = [];
                for (const d of NotifyHub.disks) {
                    list.push({ label: (d.mount ? "Open " : "Open ") + d.name + (d.size ? "  " + d.size : ""), icon: "drive", run: () => { if (d.mount) Quickshell.execDetached(["xdg-open", d.mount]); else NotifyHub.disk("mount", d.path); } });
                    if (d.mount) list.push({ label: "Safely remove " + d.name, icon: "close", run: () => NotifyHub.disk("unmount", d.path) });
                    else list.push({ label: "Eject " + d.name, icon: "close", run: () => NotifyHub.disk("eject", d.drive || d.path) });
                }
                list.push({ separator: true }, { label: "Storage settings", icon: "gear", run: () => isl.settingsRequested("storage") });
                isl.statusMenu(dkBtn, "Disks and devices", list);
            }
            onRightClicked: clicked()
            Icon { anchors.centerIn: parent; name: "drive"; size: 18 * isl.k }
        }
    }
    Component {
        id: cDiscord
        IslandButton {   // Discord (Discord.qml): its small window by the bar
            id: discordIsl
            wanted: !isl.compact
            tip: "Discord"
            onClicked: if (SettingsNav.discord) { const p = discordIsl.mapToItem(null, discordIsl.width / 2, discordIsl.height / 2); SettingsNav.discord.toggle(isl.edge === "left" || isl.edge === "right" ? p.y : p.x); }
            Icon { anchors.centerIn: parent; name: "chat"; size: 20 * isl.k }
        }
    }
    Component {
        id: cKeyboard
        IslandButton {   // on-screen keyboard
            wanted: !isl.compact
            tip: "Keyboard"
            onClicked: isl.keyboardRequested()
            Icon { anchors.centerIn: parent; name: "kbd"; size: 20 * isl.k }
        }
    }
    // ---- the keyboard layout, once you have more than one (click: the next one; right-click: pick)
    Component {
        id: cLayout
        IslandButton {
            id: layBtn
            wanted: (NotifyHub.layout.layouts || []).length > 1
            tip: NotifyHub.layoutName + (NotifyHub.layout.canSwitch ? "\nClick for the next layout" : "\nSwitch with your layout keys")
            onClicked: if (NotifyHub.layout.canSwitch) NotifyHub.nextLayout(); else isl.settingsRequested("keyboard")
            onRightClicked: isl.statusMenu(layBtn, "Keyboard layout", (NotifyHub.layout.layouts || []).map((l, i) => ({ label: l.name, checked: i === NotifyHub.layout.index ? true : undefined, enabled: NotifyHub.layout.canSwitch, run: () => NotifyHub.setLayout(i) }))
                .concat([{ separator: true }, { label: "Keyboard settings", icon: "gear", run: () => isl.settingsRequested("keyboard") }]))
            Text { anchors.centerIn: parent; text: NotifyHub.layoutShort; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 * isl.k; font.bold: true }
        }
    }
    Component {
        id: cNetwork
        GridLayout {   // network, with the vaults' button before it (1.14, VaultStore.qml: shown when you have a vault)
        flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rows: isl.vertical ? -1 : 1
        columns: isl.vertical ? 1 : -1
        rowSpacing: 2; columnSpacing: 2
        IslandButton {
            id: vaultIsl
            visible: VaultStore.vaults.length > 0
            tip: VaultStore.tip
            onClicked: isl.statusMenu(vaultIsl, "Vaults", VaultStore.menu(p => isl.settingsRequested(p)))
            onRightClicked: isl.statusMenu(vaultIsl, "Vaults", VaultStore.menu(p => isl.settingsRequested(p)))
            Icon { anchors.centerIn: parent; name: "vault"; size: 19 * isl.k; color: VaultStore.opened.length ? Theme.accent : Theme.text }
        }
        IslandButton {   // network (1.14: with its speed when Settings › Bar and dock › Network speed is on)
            id: net
            wideLabel: Settings.netSpeed && !isl.vertical && NetSpeed.text !== "" ? NetSpeed.text : ""
            tip: (isl.netName ? "Connected to " + isl.netName : "Not connected") + (Settings.netSpeed && NetSpeed.text ? "\n" + NetSpeed.text : "")
            onClicked: isl.quickRequested()
            onRightClicked: isl.statusMenu(net, net.tip, [
                { label: "Wi-Fi settings", icon: "wifi", run: () => isl.settingsRequested("wifi") },
                { label: "Connection details", icon: "gear", run: () => isl.settingsRequested("net-details") },
                { label: Settings.netSpeed ? "Hide the network speed" : "Show the network speed", icon: "speed", run: () => NetSpeed.toggle() },
                { label: "Quick settings", icon: "menu", run: () => isl.quickRequested() }
            ])
            Icon { anchors.centerIn: parent; size: 19 * isl.k; name: isl.netKind === "wifi" ? "wifi" : isl.netKind === "ethernet" ? "wired" : "offline" }
        }
        }
    }
    Component {
        id: cBluetooth
        IslandButton {   // Bluetooth
            id: bt
            wanted: isl.btPresent && !isl.compact
            tip: !isl.btPowered ? "Bluetooth is off" : isl.btConnected.length ? "Connected: " + isl.btConnected.join(", ") : "Bluetooth is on"
            onClicked: isl.quickRequested()
            onRightClicked: isl.statusMenu(bt, bt.tip, [
                { label: isl.btPowered ? "Turn Bluetooth off" : "Turn Bluetooth on", icon: "bt", run: () => { btSet.command = ["bluetoothctl", "power", isl.btPowered ? "off" : "on"]; btSet.running = true; } },
                { label: "Bluetooth settings", icon: "gear", run: () => isl.settingsRequested("bluetooth") }
            ])
            Icon { anchors.centerIn: parent; name: "bt"; size: 18 * isl.k; color: !isl.btPowered ? Theme.muted : isl.btConnected.length ? Theme.accent : Theme.text }
        }
    }
    Component {
        id: cVolume
        IslandButton {   // volume
            id: vol
            wanted: isl.audio !== null
            tip: (isl.audio && isl.audio.muted ? "Muted" : "Volume " + isl.volPct + "%") + "\nScroll to change, middle-click to mute"
            onClicked: isl.quickRequested()
            onMiddleClicked: if (isl.audio) isl.audio.muted = !isl.audio.muted
            onRightClicked: isl.soundMenu(vol)
            onScrolled: delta => { if (isl.audio) isl.audio.volume = Math.max(0, Math.min(Buttons.maxVolume, isl.audio.volume + (delta > 0 ? 0.05 : -0.05))); }
            Icon { anchors.centerIn: parent; size: 19 * isl.k; name: !isl.audio || isl.audio.muted ? "vol-mute" : isl.volPct > 33 ? "vol" : "vol-low" }
        }
    }
    Component {
        id: cBattery
        IslandButton {   // battery
            id: battBtn
            wanted: isl.hasBatt
            wideLabel: isl.vertical ? "" : isl.battPct + "%"
            tip: isl.battPct + "%" + (isl.charging ? ", charging" : "")
            onClicked: isl.quickRequested()
            onRightClicked: isl.statusMenu(battBtn, battBtn.tip, [{ label: "Power settings", icon: "power", run: () => isl.settingsRequested("power") }])
            Icon { anchors.centerIn: parent; size: 19 * isl.k; name: isl.charging ? "batt-charging" : isl.battPct > 20 ? "batt" : "batt-low" }
        }
    }
    Component {
        id: cNotifications
        IslandButton {   // notifications (1.14: also when it's quiet by itself, and why)
            wanted: !isl.compact || DeskState.unseen > 0
            tip: DeskState.dnd ? "Do not disturb is on" : NotifyHub.autoReason !== "" ? "Quiet " + NotifyHub.autoLabel + (NotifyHub.missed ? " · " + NotifyHub.missed + " waiting" : "")
                : DeskState.unseen > 0 ? DeskState.unseen + (DeskState.unseen === 1 ? " new notification" : " new notifications") : "Notifications"
            badge: NotifyHub.quiet ? 0 : DeskState.unseen
            onClicked: isl.calendarRequested()
            onRightClicked: DeskState.setDnd(!DeskState.dnd)
            Icon { anchors.centerIn: parent; name: NotifyHub.quiet ? "bell-off" : "bell"; size: 19 * isl.k; color: NotifyHub.quiet ? Theme.muted : Theme.text }
            Rectangle {   // automatic quiet: a small moon by the bell
                visible: NotifyHub.autoReason !== "" && !DeskState.dnd
                x: parent.width / 2 + 3; y: parent.height / 2 + 1; width: 10; height: 10; radius: 5
                color: Theme.accent; border.color: Theme.panel; border.width: 1
            }
        }
    }
    // A thin progress ring (the jobs button, the dock's icons)
    component Ring: Canvas {
        property real size: 22
        property int value: -1
        width: size; height: size
        onValueChanged: requestPaint()
        onPaint: {
            const g = getContext("2d"); g.reset();
            const r = width / 2 - 2;
            g.lineWidth = 2.5; g.strokeStyle = Theme.css(Theme.text, 0.18);
            g.beginPath(); g.arc(width / 2, height / 2, r, 0, Math.PI * 2); g.stroke();
            if (value < 0) return;
            g.strokeStyle = Theme.css(Theme.accent, 1); g.lineCap = "round";
            g.beginPath(); g.arc(width / 2, height / 2, r, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * Math.max(0.02, value / 100)); g.stroke();
        }
    }

    // ------------------------------------------------------------------ building blocks
    // An island: the look's frame (Frame.qml) when floating; nothing of its own inside a full-width bar.
    // `along` is its length along the bar; across, it fills the bar.
    component Island: Item {
        property bool lit: false
        property real along: 0
        property real rad: isl.mb.corners >= 99 ? Math.round(isl.islandH / 2) : isl.mb.corners >= 0 ? isl.mb.corners : isl.gaming ? 0 : Theme.r(isl.islandH / 2.4)
        property bool pill: false              // framed even in a full-width bar (a layout's status pill)
        property bool grows: false             // a block in it is set to Fill: it takes the room the bar has
        readonly property bool framed: isl.floating || pill
        // the bar's own fill, see-through or blur (drag and drop board 2) instead of the look's frame
        readonly property bool ownFill: isl.mb.fill !== "" || isl.mb.see > 0 || isl.mb.blur
        Layout.fillHeight: !isl.vertical || grows
        Layout.fillWidth: isl.vertical || grows
        Layout.preferredWidth: isl.vertical ? (isl.islandH || 52) : along
        Layout.preferredHeight: isl.vertical ? along : (isl.islandH || 52)
        Frame { anchors.fill: parent; visible: !(parent.ownFill && parent.framed); flat: !isl.floating && !parent.pill; lit: parent.lit; rad: parent.pill && !isl.floating ? Theme.r(height / 2) : parent.rad }
        Rectangle {
            id: ownIsland
            anchors.fill: parent
            visible: parent.ownFill && parent.framed
            radius: parent.pill && !isl.floating ? Theme.r(height / 2) : parent.rad
            color: isl.mb.fill !== "" ? BL.argb(isl.mb.fill, 1 - isl.mb.see / 100) : Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 1 - isl.mb.see / 100)
            border.width: 1
            border.color: parent.lit ? Theme.accent : Theme.line
            Loader { z: -1; anchors.fill: parent; active: isl.mb.blur && ownIsland.visible; source: "Frost.qml"; onLoaded: item.rad = Qt.binding(() => ownIsland.radius) }
        }
    }

    // Back to Game Mode: dressed per look, hidden only while Game Mode is off on this PC (Settings › System › Startup).
    component GameModeButton: Item {
        id: gmb
        visible: !Settings.gameModeOff
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
        property bool wanted: true         // shown at all (the bar's own items, BarSlot)
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
