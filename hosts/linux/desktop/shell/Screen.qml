// Build anything (build-anything boards 1-3): the desktop as one screen of blocks, under your windows (windows open over
// it as always, Alt+Tab and Files work as before). The layout is ScreenLayout.screen, in hearth-layout's screen.rs shape
// (one format for both Mantels, checked there):
//   grid {cols, rows}   background {kind: gradient|colour|paper|art|picture|wallpaper|none, from, to, value}
//   look {font, accent, text, sound, animation}   bars: "hidden" | ""   said: the words that made it
//   blocks [{id, block, x, y, w, h, title, shows, skip, count, items, value, press {action, target},
//            style {colour, background, text, corners, seeThrough, font, weight, size, focus, sound, animation, aboveWindows}}]
// Every block is Hearth's own: your games (hearth-games), apps (AppEntries), windows (WinList), the battery, the clock,
// what's playing, the weather (Open-Meteo, for the place picked in Settings › Display), the system's readings (Stats).
// Blocks your AI made ("own:<id>") run boxed in (OwnBlock.qml, hearth-layout run-block). Nothing here writes your
// settings. Loaded by shell.qml from its file, so if it can't be drawn Mantel's bar and dock come back.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.UPower
import Quickshell.Services.Mpris

PanelWindow {
    id: scr
    property var launcher: null
    property var quick: null
    property var settingsApp: null
    property var filesApp: null
    property var powerMenu: null
    // a layout to show instead of yours (Try it full screen in the block editor)
    property var preview: null
    signal editRequested()
    readonly property var lay: preview || ScreenLayout.screen || ({ blocks: [] })
    Component.onCompleted: if (!preview) ScreenLayout.drawn++
    Component.onDestruction: if (!preview) ScreenLayout.drawn--

    WlrLayershell.layer: preview ? WlrLayer.Top : WlrLayer.Bottom
    WlrLayershell.namespace: "hearth-screen"
    WlrLayershell.keyboardFocus: preview ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.OnDemand
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    // ---- the grid
    readonly property int cols: Math.max(1, (lay.grid && lay.grid.cols) || 24)
    readonly property int rows: Math.max(1, (lay.grid && lay.grid.rows) || 14)
    readonly property real margin: Math.round(Math.min(width, height) * 0.035)
    readonly property real cellW: (width - margin * 2) / cols
    readonly property real cellH: (height - margin * 2) / rows
    readonly property var look: lay.look || ({})
    readonly property color accent: look.accent || "#ffffff"
    readonly property color ink: look.text || "#ffffff"
    // Hearth's own fonts for a look's names (screen.rs FONTS); a name it hasn't got is Hearth's.
    function fontFor(n) {
        switch (n || look.font || "") {
        case "Noto Serif": return "Noto Serif";
        case "Noto Sans Mono": case "Pixel": return "Noto Sans Mono";
        case "Inter Display": return "Inter Display";
        case "Condensed": return "Noto Sans";
        default: return Theme.font;
        }
    }
    readonly property string fontName: fontFor("")

    // ---- what the blocks share
    property var games: []                 // hearth-games: [{id, title, cover, wide, icon, update}]
    property int picked: 0                 // the icon row's picked game (its art fills the screen, Play plays it)
    property string tab: ""                // the tabs block's tab ("windows" shows your windows and files)
    property bool guideOpen: false
    readonly property var pickedGame: games.length ? games[Math.min(picked, games.length - 1)] : null
    readonly property string gamesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-games").toString().replace(/^file:\/\//, ""))
    readonly property string widgetsHelper: decodeURIComponent(Qt.resolvedUrl("hearth-widgets").toString().replace(/^file:\/\//, ""))
    readonly property string userName: {
        const n = (Quickshell.env("USER") || "you");
        return n.charAt(0).toUpperCase() + n.slice(1);
    }
    readonly property var recentGames: {
        const ids = (DeskState.recent || []).filter(r => String(r).startsWith("game:")).map(r => String(r).slice(5));
        const seen = ids.map(id => games.find(g => g.id === id)).filter(g => !!g);
        return seen.concat(games.filter(g => ids.indexOf(g.id) < 0));
    }
    // A row's items (glyph: one of Hearth's own icons, icons/): Hearth's own things by name (screen.rs ITEMS) or an app by its menu entry ("app:<id>").
    function itemOf(id) {
        const own = {
            games: { glyph: "pad", label: "Games", action: "games" }, store: { glyph: "store", label: "Hearth Store", action: "store" },
            files: { glyph: "folder", label: "Files", action: "files" }, browser: { glyph: "globe", label: "Browser", action: "browser" },
            music: { glyph: "headphones", label: "Music", action: "music" }, friends: { glyph: "family", label: "Friends", action: "friends", wide: true },
            downloads: { glyph: "update", label: "Downloads", action: "store" }, settings: { glyph: "gear", label: "Settings", action: "settings" },
            capture: { glyph: "cam", label: "Capture", action: "capture" }, notes: { glyph: "doc", label: "Notes", action: "notes" },
            gamemode: { glyph: "pad", label: "Game Mode", action: "game-mode" }, power: { glyph: "power", label: "Power", action: "power" },
            quick: { glyph: "menu", label: "Quick settings", action: "quickaccess" }, search: { glyph: "search", label: "Search", action: "search" },
            menu: { glyph: "home", label: "Start", action: "menu" }, windows: { glyph: "window", label: "Desktop", action: "tab", target: "windows" },
            bin: { glyph: "trash", label: "Bin", action: "open-folder", target: "trash:///" }, home: { glyph: "home", label: "My Hearth", action: "open-folder", target: "~" }
        };
        if (own[id]) return own[id];
        const a = String(id).startsWith("app:") ? String(id).slice(4) : "";
        const e = a ? (AppEntries.byId(a) || AppEntries.byId(a + ".desktop")) : null;
        return { glyph: "", label: e ? e.name : a, icon: e ? e.icon : "", action: "open-app", target: a };
    }

    function play(g) { if (!g) return; DeskState.noteLaunch("game:" + g.id); Quickshell.execDetached(["python3", gamesHelper, "play", g.id]); }
    function openApp(id) {
        const e = AppEntries.byId(id) || AppEntries.byId(id + ".desktop");
        if (e) { DeskState.noteLaunch(e.id); e.execute(); return true; }
        return false;
    }
    // A block's sound (Hearth's own: desktop/sounds/hearth/stereo/block-*.wav), unless Settings › Sounds is off.
    function sound(name) {
        const n = name || look.sound || "";
        if (!n || n === "none" || Sounds.set === "off") return;
        Sounds.preview("block-" + n, "hearth");
    }
    Connections { target: ScreenLayout; function onGuideAskChanged() { scr.guideOpen = !scr.guideOpen; } }
    // What a block does when pressed: Hearth's own actions only (feature.rs ACTIONS, and the screen's own).
    function act(a, arg) {
        switch (a) {
        case "play": play(arg && arg.id ? arg : pickedGame); break;
        case "games": if (launcher) launcher.openTab("games"); break;
        case "menu": if (launcher) launcher.toggle(); break;
        case "search": if (launcher) launcher.openMenuSearch(); break;
        case "quick": case "quickaccess": if (quick) quick.toggle(); break;
        case "settings": case "open-settings": if (settingsApp) settingsApp.open(arg || ""); break;
        case "files": if (filesApp) filesApp.open(""); else Quickshell.execDetached(["hearth-files-app"]); break;
        case "open-folder": {
            const p = !arg || arg === "~" ? Quickshell.env("HOME") : String(arg).replace(/^~/, Quickshell.env("HOME"));
            if (filesApp) filesApp.open(p); else Quickshell.execDetached(["hearth-files-app", p]);
            break;
        }
        case "store": Quickshell.execDetached(["hearth-store-app"]); break;
        case "capture": case "screenshot": Quickshell.execDetached(["hearth-capture-app"]); break;
        case "notes": if (!openApp("hearth-notes")) Quickshell.execDetached(["hearth-notes-app"]); break;
        case "friends": if (!openApp("hearth-family")) Quickshell.execDetached(["hearth-family-app"]); break;
        case "browser": Quickshell.execDetached(["xdg-open", "https://hearthos.fun"]); break;
        case "open-url": if (String(arg).startsWith("https://")) Quickshell.execDetached(["xdg-open", String(arg)]); break;
        case "music": if (!openApp("org.gnome.Rhythmbox3") && !openApp("org.kde.elisa")) Quickshell.execDetached(["xdg-open", Quickshell.env("HOME") + "/Music"]); break;
        case "gamemode": case "game-mode": Quickshell.execDetached(["nimbus-mode", "game"]); break;
        case "power": if (powerMenu) powerMenu.open(); break;
        case "lock": Quickshell.execDetached(["loginctl", "lock-session"]); break;
        case "guide": scr.guideOpen = !scr.guideOpen; break;
        case "app": case "open-app": openApp(arg); break;
        case "tab": case "show-desktop": scr.tab = arg || "windows"; break;
        case "overview": Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "overview", "toggle"]); break;
        case "edit": scr.editRequested(); break;
        case "mantel": ScreenLayout.useMantel(); break;
        default: break;
        }
    }

    Process {
        id: gamesProc
        command: ["python3", scr.gamesHelper, "cached"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: { try { const v = JSON.parse(text.trim().split("\n").pop() || "[]"); if (Array.isArray(v)) scr.games = v; } catch (e) {} }
        }
    }
    Timer { interval: 600000; repeat: true; running: scr.visible; onTriggered: { gamesProc.running = false; gamesProc.running = true; } }

    // On the windows tab only the band with the tabs and the clock takes the pointer: your desktop's icons and files
    // under it work as always.
    Item { id: topBand; anchors { top: parent.top; left: parent.left; right: parent.right } height: scr.margin + scr.cellH * 2 }
    Item { id: wholeScreen; anchors.fill: parent }
    mask: Region { item: scr.tab === "windows" ? topBand : wholeScreen }

    // ---- the background (the picked game's art behind an icon row)
    readonly property var bg: lay.background || ({})
    Rectangle {
        anchors.fill: parent
        visible: scr.tab !== "windows" && scr.bg.kind !== "wallpaper" && scr.bg.kind !== "none"
        gradient: Gradient {
            GradientStop { position: 0; color: scr.bg.from || "#101820" }
            GradientStop { position: 1; color: scr.bg.kind === "colour" ? (scr.bg.from || "#101418") : (scr.bg.to || scr.bg.from || "#05070a") }
        }
        // paper: ruled lines and a margin
        Repeater {
            model: scr.bg.kind === "paper" ? Math.ceil(scr.height / 48) : 0
            delegate: Rectangle { required property int index; x: 0; y: 120 + index * 48; width: scr.width; height: 1; color: Qt.rgba(0.35, 0.45, 0.7, 0.18) }
        }
        Rectangle { visible: scr.bg.kind === "paper"; x: scr.margin + scr.cellW * 2.3; width: 2; height: parent.height; color: Qt.rgba(0.85, 0.3, 0.3, 0.45) }
        Image {
            anchors.fill: parent
            visible: scr.bg.kind === "picture" && !!scr.bg.value
            source: scr.bg.kind === "picture" && scr.bg.value ? "file://" + String(scr.bg.value).replace(/^~/, Quickshell.env("HOME")) : ""
            fillMode: Image.PreserveAspectCrop; asynchronous: true
        }
        Image {
            id: art
            anchors.fill: parent
            visible: scr.bg.kind === "art" && !!scr.pickedGame
            source: scr.pickedGame ? (scr.pickedGame.wide || scr.pickedGame.cover || "") : ""
            fillMode: Image.PreserveAspectCrop; asynchronous: true; opacity: 0.55
        }
        Rectangle {
            anchors.fill: parent
            visible: art.visible
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: Qt.rgba(0, 0, 0, 0.75) }
                GradientStop { position: 0.6; color: Qt.rgba(0, 0, 0, 0.2) }
                GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.05) }
            }
        }
    }

    // ---- the blocks
    Repeater {
        model: scr.lay.blocks || []
        delegate: Item {
            id: cell
            required property var modelData
            required property int index
            readonly property var b: modelData
            readonly property var st: b.style || ({})
            readonly property bool alwaysShown: b.block === "tabs" || b.block === "clock" || b.block === "status" || b.block === "bar"
            visible: (scr.tab !== "windows" || alwaysShown) && (b.block !== "guide" || scr.guideOpen)
            readonly property real gx: scr.margin + (b.x || 0) * scr.cellW
            readonly property real gy: scr.margin + (b.y || 0) * scr.cellH
            x: gx; y: gy
            z: b.block === "guide" ? 5 : 0
            width: Math.max(1, b.w || 1) * scr.cellW
            height: Math.max(1, b.h || 1) * scr.cellH
            // how it comes in (its Animation, else the look's), once, unless Reduce motion is on
            readonly property string anim: st.animation || scr.look.animation || ""
            opacity: 0
            Component.onCompleted: {
                if (Settings.reduceMotion || anim === "" || anim === "none") { opacity = 1; return; }
                if (anim === "slide-left") x = gx + 60; else if (anim === "slide-right") x = gx - 60;
                else if (anim === "slide-up") y = gy + 50; else if (anim === "slide-down") y = gy - 50;
                else if (anim === "zoom") scale = 0.9;
                inTimer.interval = Math.max(1, 40 * index); inTimer.start();
            }
            Timer { id: inTimer; onTriggered: { cell.opacity = 1; cell.x = Qt.binding(() => cell.gx); cell.y = Qt.binding(() => cell.gy); cell.scale = 1; } }
            Behavior on opacity { NumberAnimation { duration: 260 } }
            Behavior on x { enabled: cell.opacity < 1; NumberAnimation { duration: 320; easing.type: Easing.OutCubic } }
            Behavior on y { enabled: cell.opacity < 1; NumberAnimation { duration: 320; easing.type: Easing.OutCubic } }
            Behavior on scale { NumberAnimation { duration: 300; easing.type: Easing.OutBack } }
            Loader {
                anchors.fill: parent
                property var blk: cell.b
                property var st: cell.st
                sourceComponent: String(cell.b.block).startsWith("own:") ? ownC
                    : ({ hero: heroC, tilerow: tilerowC, iconrow: iconrowC, tabs: tabsC, clock: clockC, profile: profileC,
                         battery: batteryC, status: statusC, search: iconBtnC, menu: iconBtnC, gamemode: iconBtnC,
                         play: playC, text: textC, picture: pictureC, activity: activityC, friends: friendsC,
                         downloads: downloadsC, windows: windowsC, media: mediaC, notes: notesC, dock: dockC,
                         guide: guideC, weather: weatherC, widget: widgetC, list: listC, icons: iconsC, bar: barC })[cell.b.block] || unknownC
                onStatusChanged: if (status === Loader.Error) console.warn("Mantel: a " + cell.b.block + " block couldn't be drawn")
            }
        }
    }
    // Without a guide block, the Home button's Guide slides in on the right.
    readonly property bool hasGuide: (lay.blocks || []).some(b => b.block === "guide")
    Loader {
        active: !scr.hasGuide && scr.guideOpen
        width: Math.min(420, scr.width * 0.24)
        anchors { top: parent.top; bottom: parent.bottom; right: parent.right }
        z: 6
        sourceComponent: guideC
        property var blk: ({ block: "guide" })
        property var st: ({})
    }

    // Keys: Home (or the controller's Guide button) opens the Guide, Escape closes it; ◀ ▶ move the icon row's pick
    // and Enter plays it.
    Item {
        focus: true
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Escape && scr.preview) { scr.preview = null; e.accepted = true; }
            else if (e.key === Qt.Key_Escape && scr.guideOpen) { scr.guideOpen = false; e.accepted = true; }
            else if (e.key === Qt.Key_Home) { scr.guideOpen = !scr.guideOpen; e.accepted = true; }
            else if (e.key === Qt.Key_Left && scr.picked > 0) { scr.picked--; scr.sound(""); e.accepted = true; }
            else if (e.key === Qt.Key_Right && scr.picked < scr.games.length - 1) { scr.picked++; scr.sound(""); e.accepted = true; }
            else if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && scr.pickedGame) { scr.play(scr.pickedGame); e.accepted = true; }
        }
    }

    // ================================================================== the blocks' drawings
    // A block's own box: its colour, see-through, corners, and what it does on focus (grow, outline, glow, lift).
    component Box: Rectangle {
        id: bx
        property var st: ({})
        property bool hot: ma.containsMouse
        property string action: ""
        property var arg: null
        signal pressed()
        readonly property string focusKind: st.focus || "grow"
        radius: st.corners !== undefined ? st.corners : 8
        color: st.colour ? st.colour : Qt.rgba(1, 1, 1, 0.06)
        opacity: st.seeThrough ? 1 - st.seeThrough / 100 * 0.9 : 1
        border.width: hot && (focusKind === "outline" || focusKind === "grow") ? 4 : (hot && focusKind === "glow" ? 2 : 0)
        border.color: focusKind === "glow" ? scr.accent : "white"
        scale: hot && focusKind === "grow" ? 1.035 : 1
        transform: Translate { y: bx.hot && bx.focusKind === "lift" ? -6 : 0 }
        z: hot ? 2 : 0
        Behavior on scale { enabled: !Settings.reduceMotion; NumberAnimation { duration: 140 } }
        onHotChanged: if (hot) scr.sound(st.sound)
        MouseArea {
            id: ma
            anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; z: -1
            onClicked: { bx.pressed(); if (bx.action) scr.act(bx.action, bx.arg); }
        }
    }
    component Label: Text { color: scr.ink; font.family: scr.fontName; elide: Text.ElideRight }
    // What a press does: the block's own press, else what it does anyway.
    function pressOf(blk, dflt, dfltArg) { return blk && blk.press && blk.press.action ? [blk.press.action, blk.press.target || dfltArg] : [dflt, dfltArg]; }

    Component { id: unknownC; Item {} }
    Component { id: ownC; OwnBlock { blk: parent.blk; st: parent.st; ink: scr.ink; fontName: scr.fontName } }

    // hero: the last game you played, big, with its art (a title over it: "Jump back in"); grows with a white outline.
    Component {
        id: heroC
        Item {
            id: hi
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            readonly property var g: scr.recentGames.length ? scr.recentGames[0] : null
            Label { text: hi.blk.title || ""; font.pixelSize: 17; opacity: 0.85; visible: text !== ""; y: -30 }
            Box {
                id: hb
                anchors.fill: parent
                st: hi.st
                action: scr.pressOf(hi.blk, "play")[0]; arg: scr.pressOf(hi.blk, "play", hi.g)[1]
                clip: true
                color: hi.st.colour || "#1d2a3a"
                Image { anchors.fill: parent; source: hi.g ? (hi.g.wide || hi.g.cover || "") : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                Rectangle { anchors.fill: parent; radius: hb.radius; gradient: Gradient { GradientStop { position: 0.45; color: "transparent" } GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.7) } } }
                Label { anchors { left: parent.left; bottom: parent.bottom; margins: 18 } text: hi.g ? hi.g.title : "Your games show here"; font.pixelSize: hi.st.size || 26; font.bold: true; width: parent.width * 0.6 }
                Label { anchors { right: parent.right; bottom: parent.bottom; margins: 20 } text: hi.g && (DeskState.recent || []).length ? "Played last" : ""; font.pixelSize: 14; opacity: 0.85 }
            }
        }
    }

    // tilerow: your games (shows "recent" or "games", skip and count), or a row of Hearth's things and apps (items);
    // shows "words": the items as words; shows "sticky": a sticky note of them.
    Component {
        id: tilerowC
        Item {
            id: tr
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            readonly property bool fromGames: blk.shows === "recent" || blk.shows === "games"
            readonly property var src: blk.shows === "recent" ? scr.recentGames : scr.games
            readonly property var list: fromGames ? src.slice(blk.skip || 0, (blk.skip || 0) + (blk.count || 99)) : (blk.items || []).map(i => scr.itemOf(i))
            Label { id: trTitle; text: tr.blk.title || ""; font.pixelSize: 17; opacity: 0.85; visible: text !== "" }
            // a sticky note: a paper square, slightly turned, its items as handwritten lines
            Rectangle {
                visible: tr.blk.shows === "sticky"
                anchors.fill: parent
                color: tr.st.colour || "#fff3a0"; rotation: 3
                Column {
                    anchors { fill: parent; margins: 20 }
                    spacing: 6
                    Repeater {
                        model: tr.blk.shows === "sticky" ? tr.list : []
                        delegate: Text {
                            required property var modelData
                            text: modelData.label; color: tr.st.text || "#2b2b2b"; font.family: scr.fontName; font.pixelSize: tr.st.size || 18
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { scr.sound(tr.st.sound); scr.act(modelData.action, modelData.target); } }
                        }
                    }
                }
            }
            Row {
                visible: tr.blk.shows !== "sticky"
                anchors { top: trTitle.visible ? trTitle.bottom : parent.top; topMargin: trTitle.visible ? 12 : 0; bottom: parent.bottom; left: parent.left; right: parent.right }
                spacing: tr.blk.shows === "words" ? 34 : 14
                Repeater {
                    model: tr.blk.shows === "sticky" ? [] : tr.list
                    delegate: Box {
                        id: tb
                        required property var modelData
                        readonly property var it: modelData
                        readonly property bool game: tr.fromGames
                        readonly property bool words: tr.blk.shows === "words"
                        width: words ? wl.implicitWidth + 8 : game ? Math.max(120, (tr.width - 14 * (tr.list.length - 1)) / Math.max(1, tr.list.length)) : (it.wide ? Math.max(200, parent.height * 2.1) : Math.max(110, parent.height))
                        height: parent.height
                        st: tr.st
                        color: words ? "transparent" : game ? (tr.st.colour || "#24324a") : (tr.st.colour || "#1f3a2e")
                        clip: true
                        action: game ? scr.pressOf(tr.blk, "play")[0] : (it.action || "")
                        arg: game ? it : it.target
                        Image { anchors.fill: parent; visible: tb.game; source: tb.game ? (tb.it.cover || tb.it.wide || "") : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                        Rectangle { anchors.fill: parent; visible: tb.game; radius: tb.radius; gradient: Gradient { GradientStop { position: 0.55; color: "transparent" } GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.65) } } }
                        Label { visible: tb.game; anchors { left: parent.left; bottom: parent.bottom; margins: 14 } text: tb.game ? tb.it.title : ""; font.pixelSize: 15; font.bold: true; width: parent.width - 28 }
                        Row {
                            id: wl
                            visible: tb.words; spacing: 8; anchors.verticalCenter: parent.verticalCenter
                            Icon { name: tb.it.glyph || ""; visible: tb.words && name !== ""; size: (tr.st.size || 18) * 1.1; color: tr.st.text || scr.ink; anchors.verticalCenter: parent.verticalCenter }
                            Label { text: tb.words ? tb.it.label : ""; font.pixelSize: tr.st.size || 18; color: tr.st.text || scr.ink }
                        }
                        Column {
                            visible: !tb.game && !tb.words
                            anchors { left: parent.left; verticalCenter: parent.verticalCenter; leftMargin: 22 }
                            width: parent.width - 30
                            spacing: 4
                            Icon { name: tb.it.glyph || ""; visible: name !== ""; size: 22; color: tr.st.text || scr.ink }
                            Label { text: tb.it.label || ""; font.pixelSize: tr.st.size || 16; width: parent.width; color: tr.st.text || scr.ink }
                            Label { text: tb.it.action === "friends" ? "Who's around: Family and friends" : ""; font.pixelSize: 13; opacity: 0.8; width: parent.width; visible: text !== "" }
                        }
                    }
                }
            }
        }
    }

    // iconrow: your games as rounded icons; the picked one bigger with its outline, its art behind the screen; the library
    // at the end.
    Component {
        id: iconrowC
        Item {
            id: ir
            readonly property var blk: parent.blk
            readonly property real big: height
            readonly property real small: height * 0.62
            Row {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 16
                Repeater {
                    model: scr.games.slice(0, ir.blk.count || 8)
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool on: index === scr.picked
                        width: on ? ir.big : ir.small; height: width
                        anchors.verticalCenter: parent.verticalCenter
                        radius: width * 0.14
                        color: "#3a2a5a"
                        border.width: on ? 4 : 0; border.color: "white"
                        clip: true
                        Behavior on width { enabled: !Settings.reduceMotion; NumberAnimation { duration: 150 } }
                        Image { anchors.fill: parent; anchors.margins: parent.on ? 6 : 0; source: modelData.cover || modelData.icon || ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { if (parent.on) scr.play(modelData); else { scr.picked = index; scr.sound((ir.blk.style || {}).sound); } } }
                    }
                }
                Rectangle {
                    width: ir.small; height: width; radius: width * 0.14
                    anchors.verticalCenter: parent.verticalCenter
                    color: Qt.rgba(1, 1, 1, 0.12)
                    Icon { anchors.centerIn: parent; name: "grid"; size: parent.width * 0.34; color: scr.ink }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("games") }
                }
            }
        }
    }

    // tabs: Games, Media (what's playing and your music), Desktop (your windows and files; the tabs and clock stay).
    Component {
        id: tabsC
        Row {
            id: tbs
            readonly property var blk: parent.blk
            spacing: 36
            Repeater {
                model: tbs.blk.items && tbs.blk.items.length ? tbs.blk.items : ["games", "music", "windows"]
                delegate: Label {
                    required property var modelData
                    required property int index
                    readonly property string key: index === 0 ? "" : String(modelData)
                    readonly property bool on: scr.tab === key
                    text: modelData === "music" ? "Media" : modelData === "windows" ? "Desktop" : scr.itemOf(modelData).label
                    font.pixelSize: 22; font.bold: on; opacity: on ? 1 : 0.75
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { scr.tab = parent.key; scr.sound(""); } }
                }
            }
        }
    }
    // The Media tab: what's playing, big.
    Loader {
        active: scr.tab === "music"
        x: scr.margin; y: scr.margin + scr.cellH * 2
        width: scr.cellW * 12; height: scr.cellH * 4
        sourceComponent: mediaC
        property var blk: ({ block: "media" })
        property var st: ({ corners: 12 })
    }

    // clock: the time (shows "date": the date; "ring": the time inside a glowing ring, a HUD's).
    Component {
        id: clockC
        Item {
            id: ck
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            property date now: new Date()
            Timer { interval: 15000; running: ck.visible; repeat: true; triggeredOnStart: true; onTriggered: ck.now = new Date() }
            Rectangle {
                visible: ck.blk.shows === "ring"
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height); height: width; radius: width / 2
                color: "transparent"; border.width: 3; border.color: Qt.rgba(scr.accent.r, scr.accent.g, scr.accent.b, 0.8)
                Rectangle { anchors.centerIn: parent; width: parent.width * 0.72; height: width; radius: width / 2; color: "transparent"; border.width: 1; border.color: Qt.rgba(scr.accent.r, scr.accent.g, scr.accent.b, 0.35) }
            }
            Label {
                anchors.fill: parent
                text: ck.blk.shows === "date" ? Qt.formatDate(ck.now, "dddd, d MMMM") : Qt.formatTime(ck.now, Settings.clock24 || ck.blk.shows === "ring" ? "HH:mm" : "h:mm AP")
                font.pixelSize: ck.st.size || 22; font.weight: ck.st.weight || Font.Normal; font.family: scr.fontFor(ck.st.font)
                color: ck.st.text || scr.ink
                horizontalAlignment: ck.blk.shows === "ring" ? Text.AlignHCenter : (ck.blk.x || 0) > scr.cols / 2 ? Text.AlignRight : Text.AlignLeft
                verticalAlignment: Text.AlignVCenter
            }
        }
    }

    // profile: your picture (~/.face or your initial), your name, online; opens the Guide.
    Component {
        id: profileC
        Row {
            spacing: 14
            Rectangle {
                width: 46; height: 46; radius: 23; color: scr.accent
                anchors.verticalCenter: parent.verticalCenter
                clip: true
                Label { anchors.centerIn: parent; text: scr.userName.charAt(0); color: "#0b1a10"; font.pixelSize: 19; font.bold: true }
                Image { anchors.fill: parent; source: "file://" + Quickshell.env("HOME") + "/.face"; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.guideOpen = !scr.guideOpen }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Label { text: scr.userName; font.pixelSize: 17; font.bold: true }
                Label { text: "Online"; font.pixelSize: 13; opacity: 0.8 }
            }
        }
    }

    Component {
        id: batteryC
        Label {
            readonly property var bt: UPower.displayDevice
            text: bt && bt.isLaptopBattery ? Math.round(bt.percentage * 100) + "%" : "Plugged in"
            font.pixelSize: 17; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter
        }
    }

    // status: search, settings, the battery and the time (a corner).
    Component {
        id: statusC
        Row {
            id: sc
            layoutDirection: Qt.RightToLeft
            spacing: 22
            property date now: new Date()
            Timer { interval: 15000; running: sc.visible; repeat: true; onTriggered: sc.now = new Date() }
            Label { text: Qt.formatTime(sc.now, Settings.clock24 ? "HH:mm" : "h:mm AP"); font.pixelSize: 19; anchors.verticalCenter: parent.verticalCenter }
            Label {
                readonly property var bt: UPower.displayDevice
                visible: bt && bt.isLaptopBattery
                text: bt && bt.isLaptopBattery ? Math.round(bt.percentage * 100) + "%" : ""
                font.pixelSize: 17; opacity: 0.85; anchors.verticalCenter: parent.verticalCenter
            }
            Icon { name: "gear"; size: 22; color: scr.ink; anchors.verticalCenter: parent.verticalCenter; MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("settings") } }
            Icon { name: "search"; size: 22; color: scr.ink; anchors.verticalCenter: parent.verticalCenter; MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("search") } }
        }
    }

    // search, menu (the flame), gamemode: one button each.
    Component {
        id: iconBtnC
        Box {
            id: ib
            readonly property var blk: parent.blk
            st: parent.st
            radius: st.corners !== undefined ? st.corners : height / 2
            action: scr.pressOf(blk, blk.block === "search" ? "search" : blk.block === "menu" ? "menu" : "game-mode")[0]
            arg: scr.pressOf(blk, "")[1]
            Row {
                anchors.centerIn: parent; spacing: 8
                Icon { name: ib.blk.block === "search" ? "search" : ib.blk.block === "menu" ? "grid" : "pad"; size: Math.min(28, ib.height * 0.45); color: ib.st.text || scr.ink; anchors.verticalCenter: parent.verticalCenter; visible: ib.blk.block !== "gamemode" || ib.width > ib.height * 2 }
                Label { text: ib.blk.value || (ib.blk.block === "search" ? "Search" : ib.blk.block === "menu" ? "" : "Game Mode"); visible: text !== ""; font.pixelSize: Math.min(22, ib.height * 0.38); color: ib.st.text || scr.ink; anchors.verticalCenter: parent.verticalCenter }
            }
        }
    }

    // play: the picked game's name big, Play and ⋯.
    Component {
        id: playC
        Column {
            spacing: 26
            Label { text: scr.pickedGame ? scr.pickedGame.title : "Pick a game"; font.pixelSize: 72; font.weight: Font.Light; width: parent.width }
            Row {
                spacing: 14
                Rectangle {
                    width: 150; height: 56; radius: 28; color: "white"
                    Text { anchors.centerIn: parent; text: "Play"; color: "#111"; font.family: scr.fontName; font.pixelSize: 20; font.bold: true }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.play(scr.pickedGame) }
                }
                Rectangle {
                    width: 56; height: 56; radius: 28; color: Qt.rgba(1, 1, 1, 0.16)
                    Label { anchors.centerIn: parent; text: "⋯"; font.pixelSize: 20 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("games") }
                }
            }
        }
    }

    Component {
        id: textC
        Label {
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            text: blk.value || ""
            font.pixelSize: st.size || 18; font.weight: st.weight || Font.Normal; font.family: scr.fontFor(st.font)
            color: st.text || scr.ink
            wrapMode: Text.WordWrap; verticalAlignment: Text.AlignVCenter
        }
    }

    // picture: a picture of yours (value), or shows "fire": Hearth's fireside, glowing.
    Component {
        id: pictureC
        Rectangle {
            id: pc
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            radius: st.corners || 0
            clip: true
            color: blk.shows === "fire" ? "#2a1408" : "transparent"
            Rectangle {
                visible: pc.blk.shows === "fire"
                anchors.centerIn: parent
                width: parent.width * 0.7; height: parent.height * 0.8; radius: width / 2
                gradient: Gradient { GradientStop { position: 0; color: "#ffcf7a" } GradientStop { position: 0.45; color: "#ff7a2f" } GradientStop { position: 1; color: "transparent" } }
                opacity: 0.85
                SequentialAnimation on opacity {
                    running: !Settings.reduceMotion && pc.visible; loops: Animation.Infinite
                    NumberAnimation { to: 0.65; duration: 1400; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 0.9; duration: 1700; easing.type: Easing.InOutSine }
                }
            }
            Image { anchors.fill: parent; visible: !!pc.blk.value; source: pc.blk.value ? "file://" + String(pc.blk.value).replace(/^~/, Quickshell.env("HOME")) : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
        }
    }

    // activity: cards of what Hearth knows you did (no made-up numbers).
    Component {
        id: activityC
        Item {
            id: av
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            Label { id: acT; text: av.blk.title || "Your activity"; font.pixelSize: 17; opacity: 0.85 }
            Row {
                id: avRow
                anchors { top: acT.bottom; topMargin: 12; bottom: parent.bottom; left: parent.left; right: parent.right }
                spacing: 18
                Repeater {
                    model: [
                        { k: "Played last", v: scr.recentGames.length && (DeskState.recent || []).some(r => String(r).startsWith("game:")) ? scr.recentGames[0].title : "—", s: "" },
                        { k: "Your games", v: String(scr.games.length), s: "in Hearth's library" },
                        { k: "Next up", v: scr.games.some(g => g.update) ? "An update is waiting" : "All up to date", s: "" }
                    ]
                    delegate: Box {
                        required property var modelData
                        width: (avRow.width - 36) / 3; height: avRow.height
                        st: av.st
                        Column {
                            anchors { fill: parent; margins: 20 }
                            spacing: 6
                            Label { text: modelData.k; font.pixelSize: 14; opacity: 0.8 }
                            Label { text: modelData.v; font.pixelSize: 30; width: parent.width }
                            Label { text: modelData.s; font.pixelSize: 14; opacity: 0.8 }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: friendsC
        Box {
            st: parent.st
            action: "friends"
            Column {
                anchors { fill: parent; margins: 20 }
                spacing: 6
                Label { text: "Friends"; font.pixelSize: 14; color: scr.accent }
                Label { text: "Family and friends"; font.pixelSize: 16 }
                Label { text: "Who's around"; font.pixelSize: 13; opacity: 0.75 }
            }
        }
    }

    Component {
        id: downloadsC
        Box {
            st: parent.st
            action: "store"
            Column {
                anchors { left: parent.left; verticalCenter: parent.verticalCenter; leftMargin: 22 }
                spacing: 4
                Icon { name: "update"; size: 22; color: scr.ink }
                Label { text: scr.games.filter(g => g.update).length ? scr.games.filter(g => g.update).length + " updates waiting" : "Downloads"; font.pixelSize: 16 }
            }
        }
    }

    // windows: your open windows (press one to bring it up).
    Component {
        id: windowsC
        Column {
            id: wc
            spacing: 8
            readonly property var blk: parent.blk
            Label { text: wc.blk.title || "Your windows"; font.pixelSize: 17; opacity: 0.85 }
            Repeater {
                model: WinList.toplevels.values
                delegate: Box {
                    required property var modelData
                    width: wc.width; height: 44
                    st: ({ corners: 6, focus: "outline" })
                    onPressed: modelData.activate()
                    Label { anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter } text: modelData.title || modelData.appId; font.pixelSize: 15; width: parent.width - 28 }
                }
            }
        }
    }

    Component {
        id: mediaC
        Box {
            id: mb
            readonly property var pl: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
            st: parent.st
            onPressed: if (pl && pl.canTogglePlaying) pl.togglePlaying()
            Column {
                anchors { fill: parent; margins: 18 }
                spacing: 4
                Label { text: mb.pl ? (mb.pl.isPlaying ? "Playing" : "Paused") : "Nothing playing"; font.pixelSize: 13; opacity: 0.8 }
                Label { text: mb.pl ? (mb.pl.trackTitle || "") : "Music, videos and podcasts show here"; font.pixelSize: 17; width: parent.width }
                Label { text: mb.pl ? (mb.pl.trackArtist || "") : ""; font.pixelSize: 14; opacity: 0.8; width: parent.width }
            }
        }
    }

    // notes: your words (value), one per line: "☐ " and "☑ " lines are a checklist you can tick (kept only while
    // it's open: the editor's Shows keeps the words). A title over them; with a colour, a window's title bar.
    Component {
        id: notesC
        Rectangle {
            id: nt
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            readonly property bool win: !!st.colour && st.colour !== "#00000000"
            property var lines: String(blk.value || "").split("\n")
            color: win ? st.colour : "transparent"
            radius: st.corners || 0
            border.width: win ? 2 : 0; border.color: "#808080"
            Rectangle {
                visible: nt.win
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 3 }
                height: 26; color: scr.accent
                Text { anchors { left: parent.left; leftMargin: 8; verticalCenter: parent.verticalCenter } text: nt.blk.title || "Notes"; color: "white"; font.family: scr.fontFor(nt.st.font); font.pixelSize: 14; font.bold: true }
            }
            Column {
                id: ntCol
                anchors { fill: parent; margins: nt.win ? 12 : 0; topMargin: nt.win ? 40 : 0 }
                spacing: 8
                Text { visible: !nt.win && !!nt.blk.title; text: nt.blk.title || ""; color: nt.st.text || scr.ink; font.family: scr.fontFor(nt.st.font); font.pixelSize: (nt.st.size || 22) * 1.05 }
                Repeater {
                    model: nt.lines
                    delegate: Text {
                        required property var modelData
                        required property int index
                        text: modelData; color: nt.st.text || scr.ink; font.family: scr.fontFor(nt.st.font); font.pixelSize: nt.st.size || 18
                        width: ntCol.width; wrapMode: Text.WordWrap
                        MouseArea {
                            anchors.fill: parent
                            enabled: modelData.startsWith("☐") || modelData.startsWith("☑")
                            cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: { const l = nt.lines.slice(); l[index] = (modelData.startsWith("☐") ? "☑" : "☐") + modelData.slice(1); nt.lines = l; scr.sound(nt.st.sound); }
                        }
                    }
                }
            }
        }
    }

    // dock: the apps pinned to your dock.
    Component {
        id: dockC
        Row {
            id: dk
            spacing: 12
            Repeater {
                model: (DeskState.pins || []).slice(0, 10)
                delegate: Box {
                    id: dkb
                    required property var modelData
                    readonly property var e: AppEntries.byId(modelData)
                    width: dk.height; height: dk.height
                    st: ({ corners: 12, focus: "lift" })
                    onPressed: scr.openApp(modelData)
                    Image { anchors.centerIn: parent; width: parent.width * 0.6; height: width; source: dkb.e && dkb.e.icon ? Quickshell.iconPath(dkb.e.icon, true) : ""; asynchronous: true }
                }
            }
        }
    }

    // guide: the Guide, a side menu (the Home button, the profile, or the Guide key): Home, Quick settings, Friends and
    // chat, Capture and share, Your windows, Game Mode, Settings, Power, Change this desktop, Back to Hearth OS Mantel.
    Component {
        id: guideC
        Rectangle {
            readonly property var st: parent.st || ({})
            color: st.colour || Qt.rgba(0.07, 0.11, 0.09, 0.97)
            MouseArea { anchors.fill: parent }
            Column {
                id: guideCol
                anchors { fill: parent; margins: 30 }
                spacing: 6
                Text { text: "THE GUIDE · HOME BUTTON"; color: Qt.lighter(scr.accent, 1.2); font.family: scr.fontName; font.pixelSize: 13; font.letterSpacing: 1.5; bottomPadding: 18 }
                Repeater {
                    model: [
                        { t: "Home", a: "guide" }, { t: "Quick settings", a: "quick" }, { t: "Friends and chat", a: "friends" },
                        { t: "Capture and share", a: "capture" }, { t: "Your windows (" + WinList.toplevels.values.length + " open)", a: "tab", arg: "windows" },
                        { t: "Game Mode", a: "game-mode" }, { t: "Settings", a: "settings" }, { t: "Power", a: "power" },
                        { t: "Change this desktop", a: "edit" }, { t: "Back to Hearth OS Mantel", a: "mantel" }
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        width: guideCol.width; height: 48; radius: 4
                        color: index === 0 || gm.containsMouse ? scr.accent : "transparent"
                        Text { anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter } text: modelData.t; font.family: scr.fontName; font.pixelSize: 17; color: index === 0 || gm.containsMouse ? "#0b1a10" : "#e8efe9" }
                        MouseArea { id: gm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { scr.guideOpen = false; scr.sound(""); if (modelData.a !== "guide") scr.act(modelData.a, modelData.arg); } }
                    }
                }
            }
        }
    }

    // weather: now, for the place picked in Settings › Display (Night light's place), from Open-Meteo.
    Component {
        id: weatherC
        Label {
            id: wx
            readonly property var st: parent.st
            property var now: null
            readonly property bool placed: Settings.nightPlace !== ""
            text: !placed ? "Pick your place in Settings › Display for the weather" : now ? Math.round(now.temp) + "° · " + Settings.nightPlace : "Looking outside…"
            font.pixelSize: st.size || 18; color: st.text || scr.ink; font.family: scr.fontFor(st.font)
            verticalAlignment: Text.AlignVCenter
            Process {
                id: wxp
                command: ["python3", scr.widgetsHelper, "weather", String(Settings.nightLat), String(Settings.nightLon)]
                stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); if (!o.error) wx.now = o; } catch (e) {} } }
            }
            Timer { interval: 900000; running: wx.visible && wx.placed; repeat: true; triggeredOnStart: true; onTriggered: { wxp.running = false; wxp.running = true; } }
        }
    }

    // widget: one of Hearth's readouts. shows "system": CPU, GPU, memory and the battery, a HUD's readout.
    Component {
        id: widgetC
        Column {
            id: wd
            readonly property var st: parent.st
            spacing: 6
            Stats { id: stats; running: wd.visible }
            readonly property var bt: UPower.displayDevice
            Repeater {
                model: [
                    "CPU " + (stats.cpu >= 0 ? stats.cpu + "%" : "—"),
                    "GPU " + (stats.gpuTemp >= 0 ? stats.gpuTemp + "°" : stats.gpu >= 0 ? stats.gpu + "%" : "—"),
                    "MEM " + (stats.ram >= 0 && stats.ramTotal > 0 ? Math.round(stats.ram / stats.ramTotal * 100) + "%" : "—"),
                    "BAT " + (wd.bt && wd.bt.isLaptopBattery ? Math.round(wd.bt.percentage * 100) + "%" : "AC")
                ]
                delegate: Text { required property var modelData; text: modelData; color: wd.st.text || scr.ink; font.family: scr.fontFor(wd.st.font || "Noto Sans Mono"); font.pixelSize: wd.st.size || 15 }
            }
        }
    }

    // list: a plain list of words (right-aligned on the right of the screen): your games (shows "games"), or items.
    Component {
        id: listC
        Column {
            id: ls
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            readonly property bool right: (blk.x || 0) > scr.cols / 2
            spacing: 6
            Repeater {
                model: ls.blk.shows === "games" || ls.blk.shows === "recent" ? (ls.blk.shows === "recent" ? scr.recentGames : scr.games).slice(0, ls.blk.count || 6).map(g => ({ label: g.title.toUpperCase(), g: g }))
                                                                              : (ls.blk.items || []).map(i => scr.itemOf(i))
                delegate: Text {
                    required property var modelData
                    width: ls.width; horizontalAlignment: ls.right ? Text.AlignRight : Text.AlignLeft
                    text: modelData.label; color: ls.st.text || scr.ink; font.family: scr.fontFor(ls.st.font || "Noto Sans Mono"); font.pixelSize: ls.st.size || 15
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { scr.sound(ls.st.sound); if (modelData.g) scr.play(modelData.g); else scr.act(modelData.action, modelData.target); } }
                }
            }
        }
    }

    // icons: desktop icons of Hearth's things (My Hearth, the Bin, Files, Games), in a grid down the left.
    Component {
        id: iconsC
        Grid {
            id: ic
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            columns: 2; columnSpacing: 30; rowSpacing: 22
            Repeater {
                model: (ic.blk.items || []).map(i => scr.itemOf(i))
                delegate: Column {
                    required property var modelData
                    spacing: 4; width: 90
                    Icon { anchors.horizontalCenter: parent.horizontalCenter; name: modelData.glyph || "home"; size: 30; color: ic.st.text || scr.ink }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.label; color: ic.st.text || scr.ink; font.family: scr.fontFor(ic.st.font); font.pixelSize: ic.st.size || 14 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onDoubleClicked: { scr.sound(ic.st.sound); scr.act(modelData.action, modelData.target); } onClicked: scr.sound(ic.st.sound) }
                }
            }
        }
    }

    // bar: a bar across its place: its items as buttons (menu is a Start button with the flame), your windows, the clock.
    Component {
        id: barC
        Rectangle {
            id: br
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            property date now: new Date()
            Timer { interval: 15000; running: br.visible; repeat: true; onTriggered: br.now = new Date() }
            color: st.colour || Qt.rgba(0, 0, 0, 0.5)
            radius: st.corners || 0
            border.width: st.corners === 0 ? 1 : 0; border.color: "#ffffff"
            Row {
                anchors { left: parent.left; leftMargin: 6; verticalCenter: parent.verticalCenter }
                spacing: 6
                Repeater {
                    model: (br.blk.items || ["menu"]).filter(i => i !== "windows")
                    delegate: Rectangle {
                        id: bb
                        required property var modelData
                        readonly property var it: scr.itemOf(modelData)
                        width: bt.implicitWidth + 24; height: br.height - 8; color: br.st.colour || "transparent"
                        border.width: 1; border.color: "#808080"
                        Row {
                            id: bt; anchors.centerIn: parent; spacing: 6
                            Icon { name: bb.it.glyph || ""; visible: name !== ""; size: 16; color: br.st.text || scr.ink; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: bb.it.label; color: br.st.text || scr.ink; font.family: scr.fontFor(br.st.font); font.pixelSize: br.st.size || 14; font.bold: true }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { scr.sound(br.st.sound); scr.act(bb.it.action, bb.it.target); } }
                    }
                }
                Repeater {
                    model: (br.blk.items || []).indexOf("windows") >= 0 ? WinList.toplevels.values : []
                    delegate: Rectangle {
                        required property var modelData
                        width: 160; height: br.height - 8; color: modelData.activated ? Qt.darker(br.st.colour || "#c0c0c0", 1.15) : (br.st.colour || "transparent")
                        border.width: 1; border.color: "#808080"; clip: true
                        Text { anchors { left: parent.left; leftMargin: 8; verticalCenter: parent.verticalCenter } text: modelData.title || modelData.appId; color: br.st.text || scr.ink; font.family: scr.fontFor(br.st.font); font.pixelSize: 13; width: parent.width - 16; elide: Text.ElideRight }
                        MouseArea { anchors.fill: parent; onClicked: modelData.activate() }
                    }
                }
            }
            Text { anchors { right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter } text: Qt.formatTime(br.now, Settings.clock24 ? "HH:mm" : "h:mm AP"); color: br.st.text || scr.ink; font.family: scr.fontFor(br.st.font); font.pixelSize: 14 }
        }
    }
}
