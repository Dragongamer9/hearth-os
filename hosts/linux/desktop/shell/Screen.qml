// Build anything (build-anything boards 1-3): the desktop as one screen of blocks, under your windows (windows open over
// it as always, Alt+Tab and Files work as before). The layout is ScreenLayout.screen (hearth-layout screen.rs's shape):
//   grid {cols, rows}   background {kind: "gradient"|"colour"|"picture"|"art", from, to, picture}   look {font, accent, text}
//   blocks [{id, block, x, y, w, h, style {colour, background, text, corners, seeThrough, font, weight, size, focus},
//            shows {...}, press {action, id}, items [...], value}]
//   bars: "hidden" (the screen does what the bar and dock do: its guide, menu and Game Mode blocks) or "" (they stay)
// Every block is Hearth's own: your games (hearth-games), apps (AppEntries), windows (WinList), the battery, the clock,
// what's playing; nothing here writes your settings. Loaded by shell.qml from its file, so if it can't be drawn
// Mantel's bar and dock come back (ScreenLayout.failed).
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
    // a layout to show instead of yours (the gallery's preview and the block editor)
    property var preview: null
    readonly property var lay: preview || ScreenLayout.screen || ({ blocks: [] })

    WlrLayershell.layer: WlrLayer.Bottom
    WlrLayershell.namespace: "hearth-screen"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
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
    readonly property string fontName: look.font || Theme.font
    readonly property color accent: look.accent || "#ffffff"
    readonly property color ink: look.text || "#ffffff"

    // ---- what the blocks share
    property var games: []                 // hearth-games: [{id, title, cover, wide, icon}]
    property int picked: 0                 // the icon row's picked game (its art fills the screen, Play plays it)
    property string tab: ""                // the tabs block's tab ("Desktop" shows only your windows and files)
    property bool guideOpen: false
    readonly property var pickedGame: games.length ? games[Math.min(picked, games.length - 1)] : null
    readonly property string gamesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-games").toString().replace(/^file:\/\//, ""))
    readonly property string userName: {
        const n = (Quickshell.env("USER") || "you");
        return n.charAt(0).toUpperCase() + n.slice(1);
    }
    readonly property var recentGames: {
        const ids = (DeskState.recent || []).filter(r => String(r).startsWith("game:")).map(r => String(r).slice(5));
        const seen = ids.map(id => games.find(g => g.id === id)).filter(g => !!g);
        return seen.concat(games.filter(g => ids.indexOf(g.id) < 0));
    }

    Connections { target: ScreenLayout; function onGuideAskChanged() { scr.guideOpen = !scr.guideOpen; } }
    function play(g) { if (!g) return; DeskState.noteLaunch("game:" + g.id); Quickshell.execDetached(["python3", gamesHelper, "play", g.id]); }
    function openApp(id) {
        const e = AppEntries.byId(id) || AppEntries.byId(id + ".desktop");
        if (e) { DeskState.noteLaunch(e.id); e.execute(); return true; }
        return false;
    }
    // What a block does when pressed: Hearth's own actions only.
    function act(a, arg) {
        switch (a) {
        case "play": play(arg || pickedGame); break;
        case "games": if (launcher) launcher.openTab("games"); break;
        case "menu": if (launcher) launcher.toggle(); break;
        case "search": if (launcher) launcher.openMenuSearch(); break;
        case "quick": if (quick) quick.toggle(); break;
        case "settings": if (settingsApp) settingsApp.open(arg || ""); break;
        case "files": if (filesApp) filesApp.open(arg || ""); else Quickshell.execDetached(["hearth-files-app"]); break;
        case "store": Quickshell.execDetached(["hearth-store-app"]); break;
        case "capture": Quickshell.execDetached(["hearth-capture-app"]); break;
        case "browser": Quickshell.execDetached(["xdg-open", "https://hearthos.fun"]); break;
        case "music": if (!openApp("org.gnome.Rhythmbox3") && !openApp("org.kde.elisa")) Quickshell.execDetached(["xdg-open", Quickshell.env("HOME") + "/Music"]); break;
        case "gamemode": Quickshell.execDetached(["nimbus-mode", "game"]); break;
        case "power": if (powerMenu) powerMenu.open(); break;
        case "guide": scr.guideOpen = !scr.guideOpen; break;
        case "app": openApp(arg); break;
        case "tab": scr.tab = arg || ""; break;
        case "mantel": ScreenLayout.useMantel(); break;
        default: break;
        }
    }
    function ago(i) { return i === 0 ? "Played last" : ""; }

    Process {
        id: gamesProc
        command: ["python3", scr.gamesHelper, "cached"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: { try { const v = JSON.parse(text.trim().split("\n").pop() || "[]"); if (Array.isArray(v)) scr.games = v; } catch (e) {} }
        }
    }
    Timer { interval: 600000; repeat: true; running: true; onTriggered: { gamesProc.running = false; gamesProc.running = true; } }

    // On the Desktop tab only the band with the tabs and the clock takes the pointer: your desktop's icons and files
    // under it work as always.
    Item { id: topBand; anchors { top: parent.top; left: parent.left; right: parent.right } height: scr.margin + scr.cellH * 2 }
    Item { id: wholeScreen; anchors.fill: parent }
    mask: Region { item: scr.tab === "Desktop" ? topBand : wholeScreen }

    // ---- the background (the picked game's art when an icon row has one)
    readonly property var bg: lay.background || ({})
    Rectangle {
        anchors.fill: parent
        visible: scr.tab !== "Desktop"
        gradient: Gradient {
            GradientStop { position: 0; color: scr.bg.kind === "colour" ? (scr.bg.from || "#101418") : (scr.bg.from || "#101820") }
            GradientStop { position: 1; color: scr.bg.kind === "colour" ? (scr.bg.from || "#101418") : (scr.bg.to || "#05070a") }
        }
        Image {
            anchors.fill: parent
            visible: scr.bg.kind === "picture" && !!scr.bg.picture
            source: scr.bg.kind === "picture" && scr.bg.picture ? "file://" + scr.bg.picture : ""
            fillMode: Image.PreserveAspectCrop; asynchronous: true
        }
        Image {
            id: art
            anchors.fill: parent
            visible: scr.bg.kind === "art" && scr.pickedGame
            source: scr.pickedGame ? (scr.pickedGame.wide || scr.pickedGame.cover || "") : ""
            fillMode: Image.PreserveAspectCrop; asynchronous: true; opacity: 0.55
            Behavior on source { enabled: !Settings.reduceMotion }
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
            readonly property var b: modelData
            readonly property var st: b.style || ({})
            readonly property bool alwaysShown: b.block === "tabs" || b.block === "clock" || b.block === "status"
            visible: scr.tab !== "Desktop" || alwaysShown
            x: scr.margin + (b.x || 0) * scr.cellW
            y: scr.margin + (b.y || 0) * scr.cellH
            width: Math.max(1, b.w || 1) * scr.cellW
            height: Math.max(1, b.h || 1) * scr.cellH
            Loader {
                anchors.fill: parent
                property var blk: cell.b
                property var st: cell.st
                sourceComponent: ({ hero: heroC, tilerow: tilerowC, iconrow: iconrowC, tabs: tabsC, clock: clockC, profile: profileC,
                                    battery: batteryC, status: statusC, search: iconBtnC, menu: iconBtnC, gamemode: iconBtnC,
                                    play: playC, text: textC, picture: pictureC, activity: activityC, friends: friendsC,
                                    downloads: downloadsC, windows: windowsC, media: mediaC, notes: textC, dock: dockC,
                                    guide: guideBtnC })[cell.b.block] || unknownC
                onStatusChanged: if (status === Loader.Error) console.warn("Mantel: a " + cell.b.block + " block couldn't be drawn")
            }
        }
    }

    // ---- the Guide: a side menu (the Home button, or the guide block)
    Rectangle {
        id: guide
        visible: scr.guideOpen
        width: Math.min(420, scr.width * 0.24)
        anchors { top: parent.top; bottom: parent.bottom; right: parent.right }
        color: Qt.rgba(0.07, 0.11, 0.09, 0.96)
        MouseArea { anchors.fill: parent }
        Column {
            anchors { fill: parent; margins: 30 }
            spacing: 6
            Text { text: "THE GUIDE · HOME BUTTON"; color: Qt.lighter(scr.accent, 1.2); font.family: scr.fontName; font.pixelSize: 13; font.letterSpacing: 1.5; bottomPadding: 18 }
            Repeater {
                model: [
                    { t: "Home", a: "guide" }, { t: "Quick settings", a: "quick" }, { t: "Friends and chat", a: "app", arg: "hearth-family" },
                    { t: "Capture and share", a: "capture" }, { t: "Your windows (" + WinList.toplevels.values.length + " open)", a: "tab", arg: "Desktop" },
                    { t: "Game Mode", a: "gamemode" }, { t: "Settings", a: "settings" }, { t: "Power", a: "power" },
                    { t: "Back to Hearth OS Mantel", a: "mantel" }
                ]
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: parent.width; height: 48; radius: 4
                    color: index === 0 || gm.containsMouse ? scr.accent : "transparent"
                    Text { anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter } text: modelData.t; font.family: scr.fontName; font.pixelSize: 17; color: index === 0 || gm.containsMouse ? "#0b1a10" : "#e8efe9" }
                    MouseArea { id: gm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { if (modelData.a !== "guide") scr.guideOpen = false; scr.act(modelData.a, modelData.arg); } }
                }
            }
        }
    }
    // Escape closes the Guide; Home (or the Guide key) opens it.
    Item {
        focus: true
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Escape && scr.guideOpen) { scr.guideOpen = false; e.accepted = true; }
            else if (e.key === Qt.Key_Home) { scr.guideOpen = !scr.guideOpen; e.accepted = true; }
            else if (e.key === Qt.Key_Left && scr.picked > 0) { scr.picked--; e.accepted = true; }
            else if (e.key === Qt.Key_Right && scr.picked < scr.games.length - 1) { scr.picked++; e.accepted = true; }
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
        radius: st.corners !== undefined ? st.corners : 8
        color: st.colour ? st.colour : Qt.rgba(1, 1, 1, 0.06)
        opacity: 1
        border.width: hot && (st.focus === "outline" || st.focus === "grow" || !st.focus) ? 4 : 0
        border.color: "white"
        scale: hot && (st.focus === "grow" || !st.focus) ? 1.035 : (hot && st.focus === "lift" ? 1.02 : 1)
        z: hot ? 2 : 0
        Behavior on scale { enabled: !Settings.reduceMotion; NumberAnimation { duration: 140 } }
        MouseArea {
            id: ma
            anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; z: -1
            onClicked: { bx.pressed(); if (bx.action) scr.act(bx.action, bx.arg); }
        }
    }
    component Label: Text { color: scr.ink; font.family: scr.fontName; elide: Text.ElideRight }

    Component { id: unknownC; Item {} }

    // hero: the last game you played (or shows.game), big, with its art; grows with a white outline on focus.
    Component {
        id: heroC
        Box {
            id: hb
            readonly property var g: scr.recentGames.length ? scr.recentGames[0] : null
            st: parent.st
            action: "play"; arg: g
            clip: true
            color: "#1d2a3a"
            Image { anchors.fill: parent; source: hb.g ? (hb.g.wide || hb.g.cover || "") : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
            Rectangle { anchors.fill: parent; radius: hb.radius; gradient: Gradient { GradientStop { position: 0.45; color: "transparent" } GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.7) } } }
            Label { anchors { left: parent.left; bottom: parent.bottom; margins: 18 } text: hb.g ? hb.g.title : "Your games show here"; font.pixelSize: 26; font.bold: true; width: parent.width * 0.6 }
            Label { anchors { right: parent.right; bottom: parent.bottom; margins: 20 } text: hb.g ? "Played last" : ""; font.pixelSize: 14; opacity: 0.85 }
        }
    }

    // tilerow: a title over a row of tiles: your games (shows.source "games"), or the items given (apps and Hearth's own).
    Component {
        id: tilerowC
        Item {
            id: tr
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            readonly property bool fromGames: (blk.shows && blk.shows.source) === "games"
            readonly property var list: fromGames ? scr.recentGames.slice((blk.shows && blk.shows.skip) || 0, ((blk.shows && blk.shows.skip) || 0) + ((blk.shows && blk.shows.count) || 6)) : (blk.items || [])
            Label { id: trTitle; text: (blk.shows && blk.shows.title) || ""; font.pixelSize: 17; opacity: 0.85; visible: text !== "" }
            Row {
                anchors { top: trTitle.visible ? trTitle.bottom : parent.top; topMargin: trTitle.visible ? 12 : 0; bottom: parent.bottom; left: parent.left; right: parent.right }
                spacing: 14
                clip: false
                Repeater {
                    model: tr.list
                    delegate: Box {
                        required property var modelData
                        readonly property var it: modelData
                        readonly property bool game: tr.fromGames
                        width: game ? Math.max(120, tr.width / Math.max(3, tr.list.length) - 14) : (it.wide ? 360 : Math.max(120, (tr.height - 30)))
                        height: parent.height
                        st: tr.st
                        color: game ? "#24324a" : (tr.st.colour || "#1f3a2e")
                        clip: true
                        action: game ? "play" : (it.action || "app")
                        arg: game ? it : (it.arg || it.id)
                        Image { anchors.fill: parent; visible: game; source: game ? (it.cover || it.wide || "") : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                        Rectangle { anchors.fill: parent; visible: game; radius: parent.radius; gradient: Gradient { GradientStop { position: 0.55; color: "transparent" } GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.65) } } }
                        Label { visible: game; anchors { left: parent.left; bottom: parent.bottom; margins: 14 } text: game ? it.title : ""; font.pixelSize: 15; font.bold: true; width: parent.width - 28 }
                        Column {
                            visible: !game
                            anchors { left: parent.left; verticalCenter: parent.verticalCenter; leftMargin: 22 }
                            width: parent.width - 30
                            spacing: 4
                            Label { text: game ? "" : (it.glyph || ""); font.pixelSize: 18 }
                            Label { text: game ? "" : (it.label || ""); font.pixelSize: 16; width: parent.width }
                            Label { text: game ? "" : (it.sub || ""); font.pixelSize: 14; opacity: 0.8; width: parent.width; wrapMode: Text.WordWrap; visible: text !== "" }
                        }
                    }
                }
            }
        }
    }

    // iconrow: a row of your games as rounded icons; the picked one bigger with its outline, its art behind the screen;
    // the library at the end.
    Component {
        id: iconrowC
        Item {
            id: ir
            readonly property real big: height
            readonly property real small: height * 0.62
            Row {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 16
                Repeater {
                    model: scr.games.slice(0, 8)
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
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { if (parent.on) scr.play(modelData); else scr.picked = index; } }
                    }
                }
                Rectangle {
                    width: ir.small; height: width; radius: width * 0.14
                    anchors.verticalCenter: parent.verticalCenter
                    color: Qt.rgba(1, 1, 1, 0.12)
                    Label { anchors.centerIn: parent; text: "▦"; font.pixelSize: parent.width * 0.32 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("games") }
                }
            }
        }
    }

    // tabs: tabs across the screen; "Desktop" shows your windows and files with only the tabs and the clock left.
    Component {
        id: tabsC
        Row {
            readonly property var blk: parent.blk
            spacing: 36
            Repeater {
                model: blk.items || ["Games", "Media", "Desktop"]
                delegate: Label {
                    required property var modelData
                    required property int index
                    readonly property string name: typeof modelData === "string" ? modelData : (modelData.label || "")
                    readonly property bool on: scr.tab === name || (scr.tab === "" && index === 0)
                    text: name; font.pixelSize: 22; font.bold: on; opacity: on ? 1 : 0.75
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.tab = (index === 0 ? "" : parent.name) }
                }
            }
        }
    }

    Component {
        id: clockC
        Label {
            readonly property var st: parent.st
            property date now: new Date()
            Timer { interval: 15000; running: parent.visible; repeat: true; onTriggered: parent.now = new Date() }
            text: Qt.formatTime(now, Settings.clock24 ? "HH:mm" : "h:mm AP")
            font.pixelSize: st.size || 22; font.weight: st.weight || Font.Normal
            horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter
            width: parent.width; height: parent.height
        }
    }

    // profile: your picture (~/.face or your initial), your name, who's online; opens the Guide.
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
            visible: bt && bt.isLaptopBattery
            text: bt && bt.isLaptopBattery ? Math.round(bt.percentage * 100) + "%" : ""
            font.pixelSize: 17; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter
            width: parent.width; height: parent.height
        }
    }

    // status: search, settings, the battery and the time (a corner).
    Component {
        id: statusC
        Row {
            layoutDirection: Qt.RightToLeft
            spacing: 22
            property date now: new Date()
            Timer { interval: 15000; running: parent.visible; repeat: true; onTriggered: parent.now = new Date() }
            Label { text: Qt.formatTime(parent.now, Settings.clock24 ? "HH:mm" : "h:mm AP"); font.pixelSize: 19; anchors.verticalCenter: parent.verticalCenter }
            Label {
                readonly property var bt: UPower.displayDevice
                visible: bt && bt.isLaptopBattery
                text: bt && bt.isLaptopBattery ? Math.round(bt.percentage * 100) + "%" : ""
                font.pixelSize: 17; opacity: 0.85; anchors.verticalCenter: parent.verticalCenter
            }
            Label { text: "⚙"; font.pixelSize: 21; anchors.verticalCenter: parent.verticalCenter; MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("settings") } }
            Label { text: "⌕"; font.pixelSize: 23; anchors.verticalCenter: parent.verticalCenter; MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: scr.act("search") } }
        }
    }

    // search, menu (the flame), gamemode, guide: one round button each.
    Component {
        id: iconBtnC
        Box {
            readonly property var blk: parent.blk
            st: parent.st
            radius: st.corners !== undefined ? st.corners : height / 2
            action: blk.block === "search" ? "search" : blk.block === "menu" ? "menu" : blk.block === "gamemode" ? "gamemode" : "guide"
            Label {
                anchors.centerIn: parent
                text: (parent.blk.shows && parent.blk.shows.label) || (parent.blk.block === "search" ? "⌕  Search" : parent.blk.block === "menu" ? "🔥" : parent.blk.block === "gamemode" ? "Game Mode" : "☰")
                font.pixelSize: Math.min(20, parent.height * 0.4)
            }
        }
    }
    Component { id: guideBtnC; Loader { sourceComponent: iconBtnC; property var blk: parent.blk; property var st: parent.st } }

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
            text: blk.value || (blk.shows && blk.shows.text) || ""
            font.pixelSize: st.size || 18; font.weight: st.weight || Font.Normal
            color: st.text || scr.ink
            wrapMode: Text.WordWrap; width: parent.width; height: parent.height; verticalAlignment: Text.AlignVCenter
        }
    }

    Component {
        id: pictureC
        Image { readonly property var blk: parent.blk; source: blk.value ? "file://" + blk.value : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; width: parent.width; height: parent.height }
    }

    // activity: cards of what you did, from what Hearth knows (no made-up numbers).
    Component {
        id: activityC
        Item {
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            Label { id: acT; text: (blk.shows && blk.shows.title) || "Your activity"; font.pixelSize: 17; opacity: 0.85 }
            Row {
                anchors { top: acT.bottom; topMargin: 12; bottom: parent.bottom; left: parent.left; right: parent.right }
                spacing: 18
                Repeater {
                    model: [
                        { k: "Played last", v: scr.recentGames.length ? scr.recentGames[0].title : "—", s: "" },
                        { k: "Your games", v: String(scr.games.length), s: "in Hearth's library" },
                        { k: "Next up", v: scr.games.some(g => g.update) ? "An update is waiting" : "All up to date", s: "" }
                    ]
                    delegate: Box {
                        required property var modelData
                        width: (parent.width - 36) / 3; height: parent.height
                        st: parent.parent.st
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
            action: "app"; arg: "hearth-family"
            Column {
                anchors { fill: parent; margins: 20 }
                spacing: 6
                Label { text: "Friends"; font.pixelSize: 14; color: scr.accent }
                Label { text: "Family and friends"; font.pixelSize: 16 }
                Label { text: "Open to see who's around"; font.pixelSize: 13; opacity: 0.75 }
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
                Label { text: "⬇"; font.pixelSize: 18 }
                Label { text: scr.games.filter(g => g.update).length ? scr.games.filter(g => g.update).length + " updates waiting" : "Downloads"; font.pixelSize: 16 }
            }
        }
    }

    // windows: your open windows (press one to bring it up).
    Component {
        id: windowsC
        Column {
            spacing: 8
            Label { text: "Your windows"; font.pixelSize: 17; opacity: 0.85 }
            Repeater {
                model: WinList.toplevels.values
                delegate: Box {
                    required property var modelData
                    width: parent.width; height: 44
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
            readonly property var pl: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
            st: parent.st
            onPressed: if (pl && pl.canTogglePlaying) pl.togglePlaying()
            Column {
                anchors { fill: parent; margins: 18 }
                spacing: 4
                Label { text: parent.parent.pl ? (parent.parent.pl.isPlaying ? "Playing" : "Paused") : "Nothing playing"; font.pixelSize: 13; opacity: 0.8 }
                Label { text: parent.parent.pl ? (parent.parent.pl.trackTitle || "") : ""; font.pixelSize: 17; width: parent.width }
                Label { text: parent.parent.pl ? (parent.parent.pl.trackArtist || "") : ""; font.pixelSize: 14; opacity: 0.8; width: parent.width }
            }
        }
    }

    // dock: the apps pinned to your dock.
    Component {
        id: dockC
        Row {
            spacing: 12
            Repeater {
                model: (DeskState.pins || []).slice(0, 10)
                delegate: Box {
                    required property var modelData
                    readonly property var e: AppEntries.byId(modelData)
                    width: parent.height; height: parent.height
                    st: ({ corners: 12, focus: "lift" })
                    onPressed: scr.openApp(modelData)
                    Image { anchors.centerIn: parent; width: parent.width * 0.6; height: width; source: e && e.icon ? Quickshell.iconPath(e.icon, true) : ""; asynchronous: true }
                }
            }
        }
    }
}
