// Firehold's Couch on one screen (Firehold.qml): the full-screen tiles you use with a controller from the sofa.
// Home (the Continue hero, Recent, Open on this PC), Library (every store in one list), Apps, Store, and Everything open
// (games and windows as one grid). The mockups: docs/mockups/firehold (Main, Library, Switcher), 1920 × 1080; other
// sizes are scaled from that (k). The controller's buttons come from Firehold.qml (press); the mouse and touch work
// too, and moving the mouse is Desk (Automatic).
import QtQuick
import QtQuick.Shapes
import QtQuick.Window
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.UPower
import "firehold.js" as F

PanelWindow {
    id: cw
    property var fh
    property string scrName: ""
    // The CONTINUE line's letter spacing, except with Qt's software renderer: there, spaced text over the big title
    // drew the rest of the page shifted (CI's virtual screen).
    readonly property bool spaced: home.GraphicsInfo.api !== GraphicsInfo.Software        // this screen's name (set once by Firehold.qml: no binding through the window)
    readonly property var pal: F.C
    readonly property real k: Math.min(width / 1920, height / 1080) || 1
    readonly property bool primary: screen === Quickshell.screens[0]
    readonly property bool shown: fh.unlocked && fh.postureOfName(scrName) === "couch" && fh.view === "home"
    readonly property string font: Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : Theme.font

    visible: shown
    color: pal.bg
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-firehold"
    WlrLayershell.keyboardFocus: !shown ? WlrKeyboardFocus.None : searching ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.OnDemand
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    // The screen's own size from the first frame (a first frame drawn smaller was left in the other buffer and showed
    // through, shifted, when only part of the page was drawn again).
    implicitWidth: screen ? screen.width : 1920
    implicitHeight: screen ? screen.height : 1080

    onShownChanged: {
        if (shown) { shownAt = Date.now(); if (primary || !fh.activeCouch) fh.activeCouch = cw; keys.forceActiveFocus(); }
        else if (fh.activeCouch === cw) fh.activeCouch = null;
    }
    Component.onCompleted: if (shown && (primary || !fh.activeCouch)) fh.activeCouch = cw
    Component.onDestruction: if (fh.activeCouch === cw) fh.activeCouch = null
    property double shownAt: 0

    // ------------------------------------------------------------------ focus (the controller's place on the page)
    // Home: zone tabs | hero (0 Resume, 1 Open in a window) | recent (i) | open (i). Library: tabs | chips | grid.
    // Apps / Store: tabs | grid. Everything open: grid only.
    property string zone: "hero"
    property int idx: 0
    property bool searching: false
    readonly property string tab: fh.tab
    readonly property string page: fh.page
    function resetFocus() {
        searching = false;
        if (page === "switcher") { zone = "grid"; idx = 0; return; }
        zone = tab === "home" ? (hero.game ? "hero" : "recent") : "grid";
        idx = 0;
    }
    onTabChanged: resetFocus()
    onPageChanged: resetFocus()

    readonly property var tabs: [["home", "Home"], ["library", "Library"], ["apps", "Apps"], ["store", "Store"]]
    function tabStep(d) {
        const i = tabs.findIndex(t => t[0] === fh.tab);
        fh.tab = tabs[Math.max(0, Math.min(tabs.length - 1, i + d))][0];
    }
    function press(b) {
        if (searching && b !== "b" && b !== "a" && b !== "y") return;   // the keyboard is typing the search
        if (page === "switcher") return switcherPress(b);
        if (b === "lb" || b === "rb") return tabStep(b === "rb" ? 1 : -1);
        if (b === "y") { if (searching) { stopSearch(); return; } fh.tab = "library"; startSearch(); return; }
        if (zone === "tabs") {
            if (b === "left" || b === "right") return tabStep(b === "right" ? 1 : -1);
            if (b === "down" || b === "a") { zone = tab === "home" ? (hero.game ? "hero" : "recent") : (tab === "library" ? "chips" : "grid"); idx = tab === "library" ? chipIndex() : 0; }
            return;
        }
        if (tab === "home") return homePress(b);
        if (tab === "library") return libraryPress(b);
        return gridPress(b, tab === "apps" ? appsGrid : storeGrid);
    }

    // ---- Home
    function homePress(b) {
        const nRecent = recentList.length, nOpen = openRow.length;
        if (zone === "hero") {
            if (b === "left") idx = 0;
            else if (b === "right") idx = 1;
            else if (b === "up") zone = "tabs";
            else if (b === "down") { zone = nRecent ? "recent" : nOpen ? "open" : "hero"; idx = 0; }
            else if (b === "a") fh.play(hero.game, idx === 1);
            else if (b === "x" && hero.game) fh.run(["page", hero.game.id], "");
            return;
        }
        if (zone === "recent") {
            if (b === "left") idx = Math.max(0, idx - 1);
            else if (b === "right") idx = Math.min(nRecent - 1, idx + 1);
            else if (b === "up") { if (hero.game) { zone = "hero"; idx = 0; } else zone = "tabs"; }
            else if (b === "down" && nOpen) { zone = "open"; idx = Math.min(idx, nOpen - 1); }
            else if (b === "a") fh.play(recentList[idx], false);
            else if (b === "x" && recentList[idx]) fh.run(["page", recentList[idx].id], "");
            return;
        }
        if (zone === "open") {
            if (b === "left") idx = Math.max(0, idx - 1);
            else if (b === "right") idx = Math.min(nOpen - 1, idx + 1);
            else if (b === "up") { zone = nRecent ? "recent" : hero.game ? "hero" : "tabs"; idx = Math.min(idx, Math.max(0, nRecent - 1)); }
            else if (b === "a" && openRow[idx]) fh.goTo(openRow[idx].win, false);
            else if (b === "x") { fh.page = "switcher"; }
        }
    }
    // ---- Library
    function chipIndex() { return Math.max(0, F.FILTERS.findIndex(f => f[0] === fh.libFilter)); }
    function libraryPress(b) {
        const chips = F.FILTERS.length;
        if (b === "lt" || b === "rt") { const i = Math.max(0, Math.min(chips - 1, chipIndex() + (b === "rt" ? 1 : -1))); fh.libFilter = F.FILTERS[i][0]; libGrid.currentIndex = 0; return; }
        if (zone === "chips") {
            if (b === "left" || b === "right") { idx = Math.max(0, Math.min(chips - 1, idx + (b === "right" ? 1 : -1))); fh.libFilter = F.FILTERS[idx][0]; libGrid.currentIndex = 0; }
            else if (b === "up") zone = "tabs";
            else if (b === "down" || b === "a") { if (libList.length) { zone = "grid"; idx = 0; libGrid.currentIndex = 0; } }
            return;
        }
        if (searching && b === "b") { stopSearch(); return; }
        gridPress(b, libGrid);
    }
    function gridPress(b, grid) {
        const n = grid.count, cols = grid.cols;
        if (b === "left" || b === "right" || b === "up" || b === "down") {
            const i = F.gridMove(grid.currentIndex, n, cols, b);
            if (i < 0) { zone = tab === "library" ? "chips" : "tabs"; idx = tab === "library" ? chipIndex() : 0; return; }
            grid.currentIndex = i;
            grid.positionViewAtIndex(i, GridView.Contain);
            return;
        }
        if (b === "a" && grid.activate) grid.activate(grid.currentIndex, false);
        else if (b === "x" && grid.options) grid.options(grid.currentIndex);
        else if (b === "b") zone = "tabs";
    }
    // ---- Everything open
    function switcherPress(b) {
        const grid = openGrid, n = grid.count;
        if (b === "b") { fh.page = "tabs"; return; }
        if (n === 0) return;
        const it = fh.openList[grid.currentIndex];
        if (b === "left" || b === "right" || b === "up" || b === "down") {
            const i = F.gridMove(grid.currentIndex, n, 3, b);
            if (i >= 0) { grid.currentIndex = i; grid.positionViewAtIndex(i, GridView.Contain); }
        } else if (b === "a" && it) fh.goTo(it.win, false);
        else if (b === "x" && it) fh.closeWin(it.win);
        else if (b === "y" && it) fh.toggleFull(it.win);
    }
    // ---- the search (Y): the on-screen keyboard types into it
    function startSearch() {
        searching = true;
        zone = "grid";
        Qt.callLater(() => { searchField.forceActiveFocus(); if (fh.pad) fh.pad.openKeyboard(); });
    }
    function stopSearch() {
        searching = false;
        if (fh.pad) fh.pad.closeKeyboard();
        keys.forceActiveFocus();
    }

    // The keyboard and the mouse still work (Always Couch; or before Automatic has switched to Desk).
    Item {
        id: keys
        focus: true
        Keys.onPressed: e => {
            const m = { [Qt.Key_Left]: "left", [Qt.Key_Right]: "right", [Qt.Key_Up]: "up", [Qt.Key_Down]: "down", [Qt.Key_Return]: "a", [Qt.Key_Enter]: "a",
                        [Qt.Key_Escape]: "b", [Qt.Key_Backspace]: "b", [Qt.Key_Tab]: "rb", [Qt.Key_Backtab]: "lb" }[e.key];
            if (m) { cw.press(m); e.accepted = true; }
        }
    }
    // A real mouse moving over the Couch (not the window appearing under a still pointer): Desk, in Automatic.
    HoverHandler {
        property point last: Qt.point(-1, -1)
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        onPointChanged: {
            const p = point.position;
            if (last.x >= 0 && Math.hypot(p.x - last.x, p.y - last.y) > 12 && Date.now() - cw.shownAt > 900) cw.fh.input("mouse");
            last = p;
        }
    }

    // ------------------------------------------------------------------ drawing helpers
    // (Inline components can't see this file's ids: they size themselves from their window, kk = k.)
    component T: Text {
        readonly property real kk: Window.width > 0 ? Math.min(Window.width / 1920, Window.height / 1080) : 1
        color: F.C.text; font.family: F.font(Theme.font); font.pixelSize: 17 * kk; textFormat: Text.PlainText; elide: Text.ElideRight
    }
    component Hint: Row {
        id: hn
        property string key: ""
        property string label: ""
        property string tint: F.KEY[key] || F.C.muted
        readonly property real kk: Window.width > 0 ? Math.min(Window.width / 1920, Window.height / 1080) : 1
        spacing: 10 * kk
        Rectangle {
            width: Math.max(32 * hn.kk, kt.implicitWidth + 16 * hn.kk); height: 32 * hn.kk; radius: 16 * hn.kk
            color: hn.tint; anchors.verticalCenter: parent.verticalCenter
            T { id: kt; anchors.centerIn: parent; text: hn.key; color: F.C.bg; font.pixelSize: 14 * hn.kk; font.weight: Font.ExtraBold }
        }
        T { text: hn.label; anchors.verticalCenter: parent.verticalCenter }
    }
    component HintBar: Rectangle {
        id: hb
        property real pad: 72
        property real tint: 0.7
        readonly property real kk: Window.width > 0 ? Math.min(Window.width / 1920, Window.height / 1080) : 1
        default property alias hints: hrow.data
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: 64 * kk
        color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, tint)
        Row { id: hrow; anchors { right: parent.right; rightMargin: hb.pad * hb.kk; verticalCenter: parent.verticalCenter } spacing: 30 * hb.kk }
    }

    // ------------------------------------------------------------------ the page
    Connections { target: cw.fh; function onRedrawsChanged() { backdrop.requestPaint(); } }
    FhBackdrop {
        id: backdrop
        anchors.fill: parent; k: cw.k
        c0: cw.page === "switcher" ? "#2c160a" : cw.tab === "home" ? "#4a2410" : "#3a1d0e"
        c1: cw.page === "switcher" ? "#160e0a" : cw.tab === "home" ? "#1d120c" : "#1a110c"
    }

    // ---- the top bar (every tab)
    Item {
        id: top
        visible: cw.page === "tabs"
        x: 72 * cw.k; y: 40 * cw.k; width: parent.width - 144 * cw.k; height: 56 * cw.k
        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 28 * cw.k
            Rectangle {
                width: 44 * cw.k; height: 44 * cw.k; radius: 12 * cw.k; color: cw.pal.accent
                anchors.verticalCenter: parent.verticalCenter
                Canvas {
                    visible: cw.tab !== "library"
                    anchors.centerIn: parent; width: 24 * cw.k; height: 24 * cw.k
                    onPaint: {
                        const c = getContext("2d"); c.reset(); c.scale(width / 24, height / 24);
                        c.strokeStyle = "#2a1206"; c.lineWidth = 2.2; c.lineCap = "round"; c.lineJoin = "round";
                        c.beginPath(); c.moveTo(12, 3); c.bezierCurveTo(14, 6, 17, 8, 17, 12); c.arc(12, 12, 5, 0, Math.PI, false);
                        c.bezierCurveTo(7, 10, 8, 9, 9, 8); c.bezierCurveTo(9, 10, 10, 11, 11, 11); c.bezierCurveTo(11, 8, 10, 6, 12, 3); c.stroke();
                    }
                }
            }
            Repeater {
                model: cw.tabs
                delegate: Item {
                    id: tl
                    required property var modelData
                    readonly property bool on: cw.fh.tab === modelData[0]
                    readonly property bool ring: cw.zone === "tabs" && on && cw.page === "tabs"
                    anchors.verticalCenter: parent ? parent.verticalCenter : undefined
                    width: tt.implicitWidth; height: tt.implicitHeight + 9 * cw.k
                    T {
                        id: tt; text: tl.modelData[1]; font.pixelSize: 22 * cw.k
                        font.weight: tl.on ? Font.ExtraBold : (cw.tab === "library" ? Font.Normal : Font.DemiBold)
                        color: tl.on ? cw.pal.text : cw.pal.muted
                    }
                    Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 3 * cw.k; color: tl.on ? cw.pal.accent : "transparent" }
                    Rectangle { anchors { fill: parent; margins: -8 * cw.k } radius: 10 * cw.k; color: "transparent"; border.width: tl.ring ? 2 * cw.k : 0; border.color: cw.pal.flame }
                    TapHandler { onTapped: cw.fh.tab = tl.modelData[0] }
                }
            }
        }
        // Home, Apps, Store: posture, Wi-Fi, battery, the time, you.
        Row {
            visible: cw.tab !== "library"
            anchors { right: parent.right; verticalCenter: parent.verticalCenter }
            spacing: 22 * cw.k
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                height: pillText.implicitHeight + 16 * cw.k; width: pillText.implicitWidth + 52 * cw.k; radius: 22 * cw.k
                color: cw.pal.glass; border.width: 1; border.color: cw.pal.line
                Rectangle { x: 16 * cw.k; anchors.verticalCenter: parent.verticalCenter; width: 10 * cw.k; height: width; radius: width / 2; color: cw.pal.accent }
                T {
                    id: pillText; x: 36 * cw.k; anchors.verticalCenter: parent.verticalCenter; font.pixelSize: 16 * cw.k; font.weight: Font.DemiBold
                    text: cw.fh.mode === "auto" ? "Couch · switches to Desk when you touch the mouse" : "Couch · stays Couch until you change it in Settings"
                }
            }
            T { anchors.verticalCenter: parent.verticalCenter; text: cw.net; color: cw.pal.muted; font.pixelSize: 20 * cw.k; visible: text !== "" }
            T { anchors.verticalCenter: parent.verticalCenter; text: cw.battery; color: cw.pal.muted; font.pixelSize: 20 * cw.k; visible: text !== "" }
            T { anchors.verticalCenter: parent.verticalCenter; text: Qt.formatTime(clock.date, Qt.locale().timeFormat(Locale.ShortFormat).indexOf("AP") >= 0 || Qt.locale().timeFormat(Locale.ShortFormat).indexOf("ap") >= 0 ? "h:mm AP" : "HH:mm"); font.pixelSize: 20 * cw.k; font.weight: Font.Bold }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 44 * cw.k; height: width; radius: width / 2; color: cw.pal.card2
                T { anchors.centerIn: parent; text: cw.initial; font.pixelSize: 20 * cw.k; font.weight: Font.ExtraBold }
            }
        }
        // Library: the search.
        Rectangle {
            visible: cw.tab === "library"
            anchors { right: parent.right; verticalCenter: parent.verticalCenter }
            width: 460 * cw.k; height: 52 * cw.k; radius: 26 * cw.k
            color: cw.pal.glass; border.width: cw.searching ? 2 * cw.k : 1; border.color: cw.searching ? cw.pal.flame : cw.pal.line
            Canvas {
                x: 18 * cw.k; anchors.verticalCenter: parent.verticalCenter; width: 20 * cw.k; height: width
                onPaint: { const c = getContext("2d"); c.reset(); c.scale(width / 24, height / 24); c.strokeStyle = "#a69484"; c.lineWidth = 2; c.lineCap = "round";
                           c.beginPath(); c.arc(11, 11, 7, 0, 2 * Math.PI); c.stroke(); c.beginPath(); c.moveTo(20, 20); c.lineTo(16, 16); c.stroke(); }
            }
            TextInput {
                id: searchField
                x: 50 * cw.k; width: parent.width - 68 * cw.k; anchors.verticalCenter: parent.verticalCenter
                color: cw.pal.text; font.family: cw.font; font.pixelSize: 18 * cw.k; clip: true
                text: cw.fh.libQuery
                onTextEdited: { cw.fh.libQuery = text; libGrid.currentIndex = 0; }
                Keys.onReturnPressed: cw.stopSearch()
                Keys.onEscapePressed: { cw.fh.libQuery = ""; cw.stopSearch(); }
                T { visible: searchField.text === ""; text: "Search " + (cw.fh.counts.all || cw.fh.library.length) + " games"; color: cw.pal.muted; font.pixelSize: 18 * cw.k; anchors.verticalCenter: parent.verticalCenter }
            }
            TapHandler { onTapped: cw.startSearch() }
        }
    }
    SystemClock { id: clock; precision: SystemClock.Minutes }
    readonly property var batt: UPower.displayDevice
    readonly property string battery: batt && batt.isLaptopBattery ? Math.round(batt.percentage * 100) + "%" : ""
    readonly property string initial: (Quickshell.env("USER") || "H").charAt(0).toUpperCase()
    // 1.16: from NetBtWatch.qml, when the network changes (no polling).
    readonly property string net: NetBtWatch.netKind === "wifi" ? "Wi-Fi" : NetBtWatch.netKind === "ethernet" ? "Wired" : "Offline"

    // ================================================================== Home
    readonly property var recentList: F.recent(fh.library, 7, hero.game ? hero.game.id : "")
    readonly property var openRow: fh.openList.filter(o => o.kind === "window").slice(0, 6)
    Item {
        id: home
        anchors.fill: parent
        visible: cw.page === "tabs" && cw.tab === "home"

        // ---- Continue
        Item {
            id: hero
            readonly property var game: cw.fh.continueGame
            readonly property var win: cw.fh.windowFor(game)
            x: 72 * cw.k; y: 136 * cw.k; width: 1776 * cw.k; height: 440 * cw.k
            // (Drawn by Qt itself, not a Canvas: under the software renderer a canvas this big under changing text drew the
            // rest of the page shifted.)
            Rectangle {
                anchors.fill: parent; radius: 24 * cw.k
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop { position: 0; color: "#2b1a2e" }
                    GradientStop { position: 0.55; color: "#5b2b1a" }
                    GradientStop { position: 1; color: "#a3541e" }
                }
            }
            FhArt { anchors.fill: parent; radius: 24 * cw.k; source: hero.game ? (hero.game.hero || "") : ""; opacity: 0.35 }
            // The game's art (a dashed frame until there's a picture).
            Item {
                id: art
                x: parent.width - 60 * cw.k - width; y: 50 * cw.k; width: 640 * cw.k; height: 340 * cw.k
                FhArt { id: heroArt; anchors.fill: parent; radius: 18 * cw.k; source: F.wideArt(hero.game, width * Screen.devicePixelRatio) }
                Shape {
                    anchors.fill: parent; visible: !heroArt.ready
                    ShapePath {
                        strokeColor: Qt.rgba(1, 1, 1, 0.25); strokeWidth: 2 * cw.k; fillColor: "transparent"
                        strokeStyle: ShapePath.DashLine; dashPattern: [1.5, 1.5]
                        PathRectangle { x: 1; y: 1; width: art.width - 2; height: art.height - 2; radius: 18 * cw.k }
                    }
                }
                T { anchors.centerIn: parent; visible: !heroArt.ready; text: "Game art"; color: Qt.rgba(1, 1, 1, 0.6); font.pixelSize: 18 * cw.k }
            }
            Column {
                x: 48 * cw.k; anchors { bottom: parent.bottom; bottomMargin: 48 * cw.k }
                width: 900 * cw.k
                spacing: 14 * cw.k
                T {
                    font.pixelSize: 18 * cw.k; font.weight: Font.Bold; color: cw.pal.flame; font.letterSpacing: cw.spaced ? 1 * cw.k : 0
                    text: !hero.game ? "WELCOME TO THE COUCH" : hero.win ? "CONTINUE · PLAYING " + F.duration(clock.date.getTime() - cw.fh.since(hero.win)).toUpperCase() : "CONTINUE · " + F.ago(hero.game.lastPlayed, clock.date.getTime())
                }
                T { width: parent.width; font.pixelSize: 64 * cw.k; font.weight: Font.ExtraBold; text: hero.game ? hero.game.title : "Your games, all in one place" }
                T {
                    width: parent.width; font.pixelSize: 20 * cw.k; color: cw.pal.sub
                    text: !hero.game ? "Play something from your Library, or open an app; everything you open stays open on the Desk too."
                        : [hero.game.chapter || "",       // the mockup's line: chapter (when the game's data has one), where it's saved, how it runs
                           hero.game.cloudSaves ? "Saved on this PC and in the cloud" : hero.game.savesBackedUp ? "Saved on this PC and backed up" : "", "runs natively, no Game Mode"].filter(Boolean).join(" · ")
                }
                Row {
                    visible: !!hero.game
                    topPadding: 10 * cw.k
                    spacing: 14 * cw.k
                    Rectangle {
                        readonly property bool ring: cw.zone === "hero" && cw.idx === 0
                        height: 60 * cw.k; width: rt.implicitWidth + 68 * cw.k; radius: 16 * cw.k; color: cw.pal.accent
                        border.width: ring ? 3 * cw.k : 0; border.color: cw.pal.flame
                        T { id: rt; anchors.centerIn: parent; text: hero.game && !hero.game.installed ? "Install" : hero.win || (hero.game && hero.game.lastPlayed > 0) ? "Resume" : "Play"; color: cw.pal.ink; font.pixelSize: 22 * cw.k; font.weight: Font.ExtraBold }
                        TapHandler { onTapped: cw.fh.play(hero.game, false) }
                    }
                    Rectangle {
                        readonly property bool ring: cw.zone === "hero" && cw.idx === 1
                        height: 60 * cw.k; width: ow.implicitWidth + 56 * cw.k; radius: 16 * cw.k; color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, 0.55)
                        border.width: ring ? 3 * cw.k : 0; border.color: cw.pal.flame
                        T { id: ow; anchors.centerIn: parent; text: "Open in a window"; font.pixelSize: 20 * cw.k; font.weight: Font.DemiBold }
                        TapHandler { onTapped: cw.fh.play(hero.game, true) }
                    }
                }
            }
        }

        // ---- Recent
        Column {
            x: 72 * cw.k; y: 616 * cw.k
            spacing: 16 * cw.k
            T { text: "Recent"; font.pixelSize: 22 * cw.k; font.weight: Font.Bold }
            Row {
                spacing: 20 * cw.k
                Repeater {
                    model: cw.recentList
                    delegate: Column {
                        id: rc
                        required property var modelData
                        required property int index
                        readonly property bool ring: cw.zone === "recent" && cw.idx === index
                        width: 228 * cw.k
                        spacing: 10 * cw.k
                        Rectangle {
                            width: parent.width; height: 128 * cw.k; radius: 14 * cw.k; color: F.toneFor(rc.modelData.id)
                            border.width: rc.ring ? 3 * cw.k : 0; border.color: cw.pal.accent
                            FhArt { anchors.fill: parent; anchors.margins: rc.ring ? 3 * cw.k : 0; radius: 12 * cw.k; source: F.wideArt(rc.modelData, width * Screen.devicePixelRatio) }
                            Rectangle {
                                visible: !!(rc.modelData.wide || rc.modelData.hero)
                                anchors { left: parent.left; bottom: parent.bottom; margins: 6 * cw.k }
                                width: rs.implicitWidth + 8 * cw.k; height: rs.implicitHeight + 4 * cw.k; radius: 6 * cw.k; color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, 0.55)
                            }
                            T { id: rs; anchors { left: parent.left; bottom: parent.bottom; margins: 10 * cw.k } text: rc.modelData.store; font.pixelSize: 13 * cw.k; color: Qt.rgba(1, 1, 1, 0.75) }
                            TapHandler { onTapped: cw.fh.play(rc.modelData, false) }
                        }
                        T { width: parent.width; text: rc.modelData.title; font.weight: Font.DemiBold }
                    }
                }
            }
        }

        // ---- Open on this PC
        Column {
            x: 72 * cw.k; y: 852 * cw.k
            spacing: 16 * cw.k
            visible: cw.openRow.length > 0
            Row {
                spacing: 0
                T { id: openTitle; text: "Open on this PC"; font.pixelSize: 22 * cw.k; font.weight: Font.Bold }
                T { text: " · the same windows you see on the Desk"; font.pixelSize: 18 * cw.k; font.weight: Font.Medium; color: cw.pal.muted; anchors.baseline: openTitle.baseline }
            }
            Row {
                spacing: 16 * cw.k
                Repeater {
                    model: cw.openRow
                    delegate: Rectangle {
                        id: oc
                        required property var modelData
                        required property int index
                        readonly property bool ring: cw.zone === "open" && cw.idx === index
                        height: 72 * cw.k; width: Math.min(360 * cw.k, ocCol.implicitWidth + 98 * cw.k); radius: 16 * cw.k
                        color: cw.pal.glass; border.width: ring ? 3 * cw.k : 1; border.color: ring ? cw.pal.accent : cw.pal.line
                        Rectangle {
                            id: ocIcon
                            x: 22 * cw.k; anchors.verticalCenter: parent.verticalCenter; width: 40 * cw.k; height: width; radius: 10 * cw.k
                            color: oc.modelData.kind === "game" ? "#5b2b1a" : F.WINDOW_CARDS[oc.index % F.WINDOW_CARDS.length]
                            Image {
                                anchors.centerIn: parent; width: 26 * cw.k; height: width; asynchronous: true; smooth: true; mipmap: true
                                sourceSize.width: Math.ceil(width * Screen.devicePixelRatio); sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
                                source: AppIcons.find("", oc.modelData.win ? oc.modelData.win.appId : "")
                                visible: status === Image.Ready
                            }
                        }
                        Column {
                            id: ocCol
                            x: 76 * cw.k; anchors.verticalCenter: parent.verticalCenter
                            spacing: 2 * cw.k
                            T { width: Math.min(implicitWidth, 260 * cw.k); text: oc.modelData.name; font.pixelSize: 18 * cw.k; font.weight: Font.DemiBold }
                            T { width: Math.min(implicitWidth, 260 * cw.k); text: oc.modelData.sub; font.pixelSize: 14 * cw.k; color: cw.pal.muted; visible: text !== "" }
                        }
                        TapHandler { onTapped: cw.fh.goTo(oc.modelData.win, false) }
                    }
                }
            }
        }

        HintBar {
            Hint { key: "A"; label: "Play" }
            Hint { key: "X"; label: "Options" }
            Hint { key: "Y"; label: "Search" }
            Hint { key: "≡"; label: cw.fh.guideButton === "open" ? "Guide: Everything open" : "Guide: Quick Access" }
            Hint { key: "LB RB"; label: "Tabs" }
        }
    }

    // ================================================================== Library
    readonly property var libList: F.filterGames(fh.library, fh.libFilter, fh.libQuery, fh.playing)
    Item {
        anchors.fill: parent
        visible: cw.page === "tabs" && cw.tab === "library"
        Row {
            x: 72 * cw.k; y: 128 * cw.k
            spacing: 12 * cw.k
            Repeater {
                model: F.chips(cw.fh.counts)
                delegate: Rectangle {
                    id: chip
                    required property var modelData
                    required property int index
                    readonly property bool on: cw.fh.libFilter === modelData.key
                    readonly property bool ring: cw.zone === "chips" && cw.idx === index
                    height: ct.implicitHeight + 24 * cw.k; width: ct.implicitWidth + 40 * cw.k; radius: 22 * cw.k
                    color: on ? cw.pal.accent : cw.pal.glass
                    border.width: ring ? 2 * cw.k : 0; border.color: cw.pal.flame
                    Row {
                        id: ct; anchors.centerIn: parent
                        spacing: 5 * cw.k
                        T { id: ctName; text: chip.modelData.label; font.pixelSize: 17 * cw.k; font.weight: Font.DemiBold; color: chip.on ? cw.pal.ink : cw.pal.text }
                        T { text: String(chip.modelData.n); font.pixelSize: 17 * cw.k; font.weight: Font.Medium; color: chip.on ? cw.pal.ink : cw.pal.text; opacity: 0.7; anchors.baseline: ctName.baseline }
                    }
                    TapHandler { onTapped: { cw.fh.libFilter = chip.modelData.key; libGrid.currentIndex = 0; } }
                }
            }
        }
        GridView {
            id: libGrid
            readonly property int cols: 6
            x: 72 * cw.k; y: 214 * cw.k; width: 1800 * cw.k; height: parent.height - y - 64 * cw.k
            cellWidth: 300 * cw.k; cellHeight: 384 * cw.k
            clip: true
            model: cw.libList
            currentIndex: 0
            highlightFollowsCurrentItem: false
            boundsBehavior: Flickable.StopAtBounds
            function activate(i, w) { const g = cw.libList[i]; if (g) cw.fh.play(g, w); }
            function options(i) { const g = cw.libList[i]; if (g && g.source !== "winapp") cw.fh.run(["page", g.id], ""); }
            delegate: Item {
                id: lt
                required property var modelData
                required property int index
                readonly property bool ring: cw.zone === "grid" && libGrid.currentIndex === index
                width: 276 * cw.k; height: 372 * cw.k
                Rectangle {
                    id: ltCard
                    width: parent.width; height: 330 * cw.k; radius: 16 * cw.k; color: F.toneFor(lt.modelData.id)
                    border.width: lt.ring ? 3 * cw.k : 1; border.color: lt.ring ? cw.pal.accent : cw.pal.line
                    FhArt { anchors.fill: parent; anchors.margins: lt.ring ? 3 * cw.k : 1; radius: 14 * cw.k; source: lt.modelData.cover || "" }
                    Rectangle {
                        x: 12 * cw.k; y: 12 * cw.k; width: lb.implicitWidth + 20 * cw.k; height: lb.implicitHeight + 8 * cw.k; radius: 8 * cw.k
                        color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, 0.65)
                        T { id: lb; anchors.centerIn: parent; text: lt.modelData.store; font.pixelSize: 12 * cw.k; font.weight: Font.Bold }
                    }
                    Rectangle {
                        visible: !!lt.modelData.cover
                        anchors { left: parent.left; bottom: parent.bottom; margins: 8 * cw.k }
                        width: ls.implicitWidth + 12 * cw.k; height: ls.implicitHeight + 6 * cw.k; radius: 6 * cw.k; color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, 0.6)
                    }
                    T { id: ls; anchors { left: parent.left; bottom: parent.bottom; margins: 14 * cw.k } text: F.status(lt.modelData, cw.fh.playing); font.pixelSize: 13 * cw.k; color: Qt.rgba(1, 1, 1, 0.8) }
                    TapHandler { onTapped: { libGrid.currentIndex = lt.index; cw.zone = "grid"; cw.fh.play(lt.modelData, false); } }
                }
                T { y: 340 * cw.k; width: parent.width; text: lt.modelData.title; font.weight: Font.DemiBold }
            }
        }
        T {
            visible: cw.libList.length === 0
            x: 72 * cw.k; y: 240 * cw.k; font.pixelSize: 20 * cw.k; color: cw.pal.muted
            text: !cw.fh.libraryLoaded ? "Reading your library…" : cw.fh.libQuery !== "" ? "No game called “" + cw.fh.libQuery + "”" : "Nothing here yet"
        }
        HintBar {
            T { text: "A Play" }
            T { text: "X Game options" }
            T { text: "Y Search" }
            T { text: "LT RT Filters" }
        }
    }

    // ================================================================== Apps (your apps, as big tiles)
    readonly property var appList: {
        const all = DesktopEntries.applications ? DesktopEntries.applications.values : [];
        return all.filter(e => !e.noDisplay && e.name)
                  .slice().sort((a, b) => a.name.toLowerCase() < b.name.toLowerCase() ? -1 : 1);
    }
    component TileGrid: GridView {
        id: tg
        property var owner: null
        readonly property real kk: owner ? owner.k : 1
        readonly property int cols: 8
        x: 72 * tg.kk; y: 136 * tg.kk; width: 1800 * tg.kk; height: parent.height - y - 64 * tg.kk
        cellWidth: 225 * tg.kk; cellHeight: 230 * tg.kk
        clip: true; currentIndex: 0; boundsBehavior: Flickable.StopAtBounds; highlightFollowsCurrentItem: false
        property var run: null
        function activate(i) { if (run) run(model[i]); }
        delegate: Item {
            id: at
            required property var modelData
            required property int index
            readonly property bool ring: (tg.owner ? tg.owner.zone : "") === "grid" && tg.currentIndex === index
            width: 205 * tg.kk; height: 214 * tg.kk
            Rectangle {
                width: parent.width; height: 170 * tg.kk; radius: 16 * tg.kk; color: F.C.glass
                border.width: at.ring ? 3 * tg.kk : 1; border.color: at.ring ? F.C.accent : F.C.line
                Image {
                    anchors.centerIn: parent; width: 88 * tg.kk; height: width; asynchronous: true; smooth: true; mipmap: true
                    sourceSize.width: Math.ceil(width * Screen.devicePixelRatio); sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
                    source: at.modelData.iconUrl || AppIcons.find(at.modelData.icon || "", at.modelData.id || "")
                }
                TapHandler { onTapped: { tg.currentIndex = at.index; tg.owner.zone = "grid"; tg.activate(at.index); } }
            }
            T { y: 180 * tg.kk; width: parent.width; text: at.modelData.name; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter }
        }
    }
    Item {
        anchors.fill: parent
        visible: cw.page === "tabs" && cw.tab === "apps"
        TileGrid { id: appsGrid; owner: cw; model: cw.appList; run: e => cw.fh.launchApp(e) }
        T { visible: cw.appList.length === 0; x: 72 * cw.k; y: 160 * cw.k; font.pixelSize: 20 * cw.k; color: cw.pal.muted; text: "No apps yet: Bazaar in the Store tab has thousands" }
        HintBar { Hint { key: "A"; label: "Open" } Hint { key: "Y"; label: "Search" } Hint { key: "LB RB"; label: "Tabs" } }
    }
    // ================================================================== Store (where games and apps come from)
    readonly property var storeList: [
        { name: "Steam Store", icon: "steam", id: "steam", url: "steam://store" },
        { name: "Epic Games", icon: "com.heroicgameslauncher.hgl", id: "heroic", url: "heroic://store/epic" },
        { name: "GOG", icon: "com.heroicgameslauncher.hgl", id: "heroic", url: "heroic://store/gog" },
        { name: "Apps (Bazaar)", icon: "io.github.kolunmi.Bazaar", id: "bazaar", ipc: "apps" }
    ]
    Item {
        anchors.fill: parent
        visible: cw.page === "tabs" && cw.tab === "store"
        TileGrid {
            id: storeGrid
            owner: cw
            model: cw.storeList
            run: s => {
                cw.fh.view = "app";
                if (s.ipc) Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", s.ipc, "open"]);
                else Quickshell.execDetached(["xdg-open", s.url]);
            }
        }
        HintBar { Hint { key: "A"; label: "Open" } Hint { key: "LB RB"; label: "Tabs" } }
    }

    // ================================================================== Everything open (Open now)
    Item {
        anchors.fill: parent
        visible: cw.page === "switcher"
        Column {
            x: 96 * cw.k; y: 64 * cw.k
            spacing: 8 * cw.k
            T { text: "Open now"; font.pixelSize: 40 * cw.k; font.weight: Font.ExtraBold }
            T { text: "Games and windows are one list. Pick one to jump to it, in Couch or Desk."; font.pixelSize: 18 * cw.k; color: cw.pal.muted }
        }
        GridView {
            id: openGrid
            x: 96 * cw.k; y: 210 * cw.k; width: parent.width - 192 * cw.k + 40 * cw.k; height: parent.height - y - 64 * cw.k
            cellWidth: (1728 * cw.k + 40 * cw.k) / 3; cellHeight: 376 * cw.k
            clip: false; currentIndex: 0; boundsBehavior: Flickable.StopAtBounds; highlightFollowsCurrentItem: false
            model: cw.fh.openList
            delegate: Item {
                id: sw
                required property var modelData
                required property int index
                readonly property bool ring: cw.page === "switcher" && openGrid.currentIndex === index
                readonly property bool game: modelData.kind === "game"
                readonly property int nth: cw.fh.openList.slice(0, index).filter(o => o.kind === modelData.kind).length
                width: openGrid.cellWidth - 40 * cw.k; height: 336 * cw.k
                Rectangle { visible: sw.ring; x: -8 * cw.k; y: -8 * cw.k; width: parent.width + 16 * cw.k; height: 300 * cw.k + 16 * cw.k; radius: 26 * cw.k; color: Qt.rgba(1, 138 / 255, 61 / 255, 0.22) }
                FhGrad {
                    visible: sw.game; width: parent.width; height: 300 * cw.k; radius: 18 * cw.k; angle: 110
                    stops: [[0, F.GAME_CARDS[sw.nth % 2][0]], [1, F.GAME_CARDS[sw.nth % 2][1]]]
                }
                Rectangle {
                    width: parent.width; height: 300 * cw.k; radius: 18 * cw.k
                    color: sw.game ? "transparent" : F.WINDOW_CARDS[sw.nth % F.WINDOW_CARDS.length]
                    border.width: sw.ring ? 3 * cw.k : 1; border.color: sw.ring ? cw.pal.accent : cw.pal.line
                    // The window's picture (KWin takes one every 3 seconds while this is open: Firehold.qml pictures).
                    FhArt {
                        anchors.fill: parent; anchors.margins: sw.ring ? 3 * cw.k : 1; radius: 16 * cw.k
                        source: cw.fh.pictureOf(sw.modelData.win); live: true
                    }
                    Rectangle {
                        anchors { top: parent.top; right: parent.right; margins: 14 * cw.k }
                        width: tg2.implicitWidth + 24 * cw.k; height: tg2.implicitHeight + 10 * cw.k; radius: 10 * cw.k; color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, 0.65)
                        T { id: tg2; anchors.centerIn: parent; text: sw.game ? "GAME" : "WINDOW"; color: sw.game ? cw.pal.flame : cw.pal.soft; font.pixelSize: 13 * cw.k; font.weight: Font.Bold }
                    }
                    TapHandler { onTapped: { openGrid.currentIndex = sw.index; cw.fh.goTo(sw.modelData.win, false); } }
                }
                Row {
                    y: 312 * cw.k
                    spacing: 10 * cw.k
                    T { id: swName; text: sw.modelData.name; font.pixelSize: 20 * cw.k; font.weight: sw.ring ? Font.ExtraBold : Font.DemiBold }
                    T { text: sw.modelData.sub; font.pixelSize: 15 * cw.k; color: cw.pal.muted; anchors.baseline: swName.baseline; width: Math.min(implicitWidth, sw.width - swName.width - 10 * cw.k) }
                }
            }
        }
        T {
            visible: cw.fh.openList.length === 0
            x: 96 * cw.k; y: 230 * cw.k; font.pixelSize: 20 * cw.k; color: cw.pal.muted
            text: "Nothing is open. Games and apps you start show up here."
        }
        HintBar {
            pad: 96; tint: 0.8
            Hint { key: "A"; label: "Go to it" }
            Hint { key: "X"; label: "Close" }
            Hint { key: "Y"; label: "Window ⇄ full screen" }
            Hint { key: "B"; label: "Back" }
        }
    }
}
