// Couch Home (mockups docs/mockups/firehold-plasma 01 and 02, checklist firehold-plasma.md 1-2). From the top: the
// game to continue (its chapter when the game or Beacon says it, where it's saved, its campaign's progress), Continue
// playing and Recently added (every store, Hearth's library: `nimbus --list-games --all` through hearth-firehold), and
// beside them Downloads and updates (Steam's and Heroic's own files on this PC, never the Steam account), Controllers
// (their batteries) and what's playing (MPRIS). Further down: Watch and listen (your media apps), Beacon's row, the Steam
// row and From your Hearth (your devices on Hearth Link, never Steam friends). The controller's focus moves between
// rows and tiles and the page jumps to keep it in view (no animation: the owner's rule on lag).
// Rows: 0 the hero's buttons, 1 Continue playing, 2 Recently added, 3 Watch and listen, 4 Beacon, 5 Steam, 6 From your Hearth.
import QtQuick
import QtQuick.Shapes
import "firehold.js" as F
import "fhp.js" as H

Item {
    id: home
    property QtObject couch
    readonly property real k: couch.k
    readonly property var now: couch.now.getTime()
    readonly property var cards: couch.cards
    readonly property var prog: cards.progress || ({})
    readonly property var added: cards.added || ({})
    readonly property var dlMap: H.downloadsMap(cards.downloads)
    readonly property var game: couch.continueGame
    readonly property var gp: game ? (prog[game.id] || null) : null
    readonly property var running: game ? couch.windowFor(game) : null
    readonly property var contRow: H.continueRow(couch.library, couch.pins, game ? game.id : "", 5)
    readonly property var addRow: H.addedRow(couch.library, added, now, 5)
    readonly property var media: H.mediaApps(couch.apps)
    readonly property var beacon: H.launcherRow(couch.library.filter(function (g) { return g.source === "beacon"; }), added)
    readonly property var steamAll: couch.library.filter(function (g) { return g.source === "steam" && g.installed; })
    readonly property var steam: steamAll.slice().sort(function (a, b) { return (b.lastPlayed || 0) - (a.lastPlayed || 0); }).slice(0, 7)
    readonly property var devices: (cards.devices || []).slice(0, 3)
    readonly property var player: couch.player
    readonly property var lowPad: H.lowPad(cards.controllers)
    readonly property bool toastShown: !!lowPad && couch.toastSeen !== lowPad.name + lowPad.percent

    property int row: 0
    property int col: 0
    readonly property var rows: [game ? 4 : 1, contRow.length, addRow.length, media.length, beacon.slice(0, 6).length, steam.length, devices.length]
    function rowGame(r, c) {
        if (r === 0) return game;
        if (r === 1) return contRow[c];
        if (r === 2) return addRow[c];
        if (r === 4) return beacon[c];
        if (r === 5) return steam[c];
        return null;
    }
    function key(kk) {
        if (kk === Qt.Key_Up || kk === Qt.Key_Down) {
            let r = row;
            do { r += kk === Qt.Key_Up ? -1 : 1; } while (r >= 0 && r < rows.length && rows[r] === 0);
            if (r >= 0 && r < rows.length) { row = r; col = Math.min(col, rows[r] - 1); }
            return true;
        }
        if (kk === Qt.Key_Left) { col = Math.max(0, col - 1); return true; }
        if (kk === Qt.Key_Right) { col = Math.min(rows[row] - 1, col + 1); return true; }
        if (kk === Qt.Key_Return || kk === Qt.Key_Enter) { activate(row, col); return true; }
        if (kk === Qt.Key_X) { const g = rowGame(row, col); if (g) couch.openOptions(g); return true; }
        if (kk === Qt.Key_Y) { couch.tab = "library"; couch.key(Qt.Key_Y); return true; }
        if (kk === Qt.Key_Escape && row > 2) { row = 0; col = 0; return true; }
        if (kk === Qt.Key_Escape && toastShown) { couch.toastSeen = lowPad.name + lowPad.percent; return true; }
        return false;
    }
    function activate(r, c) {
        if (r === 0) {
            if (!game) { couch.tab = "library"; return; }
            if (c === 0) couch.play(game, false);
            else if (c === 1) couch.play(game, true);
            else if (c === 2) couch.run(["page", game.id]);
            else couch.openOptions(game);
        } else if (r === 3) { if (media[c]) couch.run(["do", "media-app", media[c].app.id]); }
        else if (r === 6) { const d = devices[c]; const card = d ? H.deviceCard(d, now) : null; if (card && card.action) couch.run(["do", card.action, d.id]); }
        else { const g = rowGame(r, c); if (g) couch.play(g, false); }
    }
    function sel(r, c) { return couch.front && row === r && col === c; }
    // The page keeps the focus in view: the first screen for the top rows, else the rows further down from Watch and listen.
    readonly property real scrollTo: row <= 2 ? 0 : Math.max(0, Math.min(page2.y - 22 * k, flick.contentHeight - flick.height))

    Flickable {
        id: flick
        y: 100 * home.k
        width: parent.width; height: parent.height - y
        clip: true
        contentWidth: width
        contentHeight: page2.y + page2.height + 160 * home.k
        contentY: home.scrollTo
        interactive: true
        boundsBehavior: Flickable.StopAtBounds

        // ------------------------------------------------------------ the hero: the game to continue
        Item {
            id: hero
            x: 72 * home.k; y: 12 * home.k
            width: flick.width - 144 * home.k; height: 380 * home.k
            visible: true
            RoundClip {
                anchors.fill: parent
                radius: 26 * home.k
                Shape {
                    anchors.fill: parent
                    ShapePath {
                        strokeWidth: -1
                        fillGradient: LinearGradient {
                            x1: 0; y1: 0; x2: hero.width; y2: hero.height * 0.18
                            GradientStop { position: 0.0; color: "#2a1a3a" }
                            GradientStop { position: 0.55; color: "#4a2418" }
                            GradientStop { position: 1.0; color: "#a0521e" }
                        }
                        startX: 0; startY: 0
                        PathLine { x: hero.width; y: 0 }
                        PathLine { x: hero.width; y: hero.height }
                        PathLine { x: 0; y: hero.height }
                        PathLine { x: 0; y: 0 }
                    }
                }
                // The game's art on the right, fading in from the left (the mockup's mask).
                Item {
                    anchors.right: parent.right
                    width: 1000 * home.k; height: parent.height
                    visible: !!home.game
                    Art { anchors.fill: parent; game: home.game; want: "hero"; motif: true; opacity: 0.95 }
                    Rectangle {
                        anchors.fill: parent
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop { position: 0.0; color: "#40241d" }
                            GradientStop { position: 0.45; color: "transparent" }
                        }
                    }
                }
            }
            Column {
                x: 52 * home.k; y: 40 * home.k
                width: 1100 * home.k
                spacing: 0
                T {
                    couchRef: home.couch
                    text: home.game ? "CONTINUE · " + F.ago(home.game.lastPlayed || 0, home.now) + " ON THIS PC" : "WELCOME TO FIREHOLD"
                    color: H.P.ac2
                    font.pixelSize: 19 * home.k; font.weight: Font.Bold; font.letterSpacing: 1.5 * home.k
                }
                Item { width: 1; height: 18 * home.k; visible: series.visible }
                T {
                    id: series
                    couchRef: home.couch
                    visible: text !== ""
                    text: home.gp && home.gp.series ? home.gp.series : ""
                    color: "#e7d9cc"
                    font.pixelSize: 23 * home.k; font.weight: Font.DemiBold
                }
                Item { width: 1; height: (series.visible ? 2 : 14) * home.k }
                T {
                    couchRef: home.couch
                    width: parent.width
                    text: home.game ? home.game.title : "Your games, all in one place"
                    font.pixelSize: 68 * home.k; font.weight: Font.ExtraBold; font.letterSpacing: -1.36 * home.k
                    lineHeight: 1.05
                }
                Item { width: 1; height: 16 * home.k; visible: chapter.visible }
                T {
                    id: chapter
                    couchRef: home.couch
                    visible: text !== ""
                    width: parent.width
                    text: H.chapterLine(home.gp)
                    color: "#f0e4d8"
                    font.pixelSize: 22 * home.k; font.weight: Font.Bold
                }
                Item { width: 1; height: (chapter.visible ? 8 : 16) * home.k }
                T {
                    couchRef: home.couch
                    width: parent.width
                    text: home.game ? H.detailsLine(home.game, home.gp, home.cards.savesToStorage === true) : "Steam, Heroic, Beacon, emulators and your own games"
                    color: "#cdbfb2"
                    font.pixelSize: 18 * home.k
                }
                Item { width: 1; height: 24 * home.k }
                Row {
                    spacing: 14 * home.k
                    Repeater {
                        model: home.game ? ["▶ " + (home.running ? "Resume" : "Continue"), "Open in a window", "Game page", "⋯"] : ["Open the Library"]
                        delegate: Item {
                            required property var modelData
                            required property int index
                            readonly property bool primary: index === 0
                            width: b.width; height: b.height
                            Rectangle {
                                id: b
                                height: 60 * home.k
                                width: btnText.implicitWidth + (modelData === "⋯" ? 44 : 60) * home.k
                                radius: 14 * home.k
                                color: parent.primary ? H.P.ac : "#14ffffff"
                                T {
                                    id: btnText
                                    couchRef: home.couch
                                    anchors.centerIn: parent
                                    text: modelData
                                    color: parent.parent.primary ? H.P.ink : H.P.tx
                                    font.pixelSize: 22 * home.k; font.weight: Font.Bold
                                }
                            }
                            Ring { visible: home.sel(0, index); k: home.k; r: 14; anchors.fill: b }
                            MouseArea { anchors.fill: parent; onClicked: { home.row = 0; home.col = index; home.activate(0, index); } }
                        }
                    }
                }
            }
            // The campaign's progress, when the game or Beacon says how far you are.
            Rectangle {
                visible: !!home.gp && typeof home.gp.percent === "number"
                anchors.right: parent.right; anchors.rightMargin: 36 * home.k
                anchors.bottom: parent.bottom; anchors.bottomMargin: 30 * home.k
                width: 370 * home.k; height: side.implicitHeight + 32 * home.k
                radius: 18 * home.k
                color: "#8c0f0a0a"
                border.width: 1; border.color: "#12ffffff"
                Column {
                    id: side
                    x: 20 * home.k; y: 16 * home.k
                    width: parent.width - 40 * home.k
                    spacing: 0
                    T { couchRef: home.couch; text: home.gp && home.gp.campaign ? "Campaign" : "Progress"; color: H.P.mut; font.pixelSize: 14 * home.k }
                    Item { width: 1; height: 6 * home.k }
                    Item {
                        width: parent.width; height: done.implicitHeight
                        T { id: done; couchRef: home.couch; text: (home.gp ? Math.round(home.gp.percent) : 0) + "% done"; font.pixelSize: 17 * home.k; font.weight: Font.Bold }
                        T {
                            couchRef: home.couch
                            anchors.right: parent.right; anchors.verticalCenter: done.verticalCenter
                            visible: H.hours(home.game, home.gp) > 0
                            text: H.hours(home.game, home.gp) + " h played"
                            color: H.P.mut; font.pixelSize: 14 * home.k
                        }
                    }
                    Item { width: 1; height: 10 * home.k }
                    Bar { width: parent.width; value: home.gp ? home.gp.percent : 0; k: home.k }
                    Item { width: 1; height: 10 * home.k; visible: alsoOn.visible }
                    T {
                        id: alsoOn
                        couchRef: home.couch
                        visible: text !== ""
                        width: parent.width
                        text: home.cards.alsoOn ? "Also on the " + home.cards.alsoOn + " · saves in step" : ""
                        color: H.P.mut; font.pixelSize: 14 * home.k
                    }
                }
            }
        }

        // ------------------------------------------------------------ Continue playing and Recently added
        Column {
            id: left
            x: 72 * home.k; y: 416 * home.k
            width: 1230 * home.k
            spacing: 0
            RowHead { couch: home.couch; width: parent.width; title: "Continue playing"; note: home.contRow.length >= 5 ? "RB for more" : ""; visible: home.contRow.length > 0 }
            Item { width: 1; height: 14 * home.k; visible: home.contRow.length > 0 }
            Row {
                spacing: 20 * home.k
                Repeater {
                    model: home.contRow
                    delegate: GameTile {
                        required property var modelData
                        required property int index
                        couch: home.couch
                        game: modelData
                        progress: home.prog[modelData.id] && typeof home.prog[modelData.id].percent === "number" ? home.prog[modelData.id].percent : -1
                        sub: H.continueSub(modelData, home.prog[modelData.id], home.now, home.couch.sys.user)
                        focused: home.sel(1, index)
                        onActivated: { home.row = 1; home.col = index; home.activate(1, index); }
                    }
                }
            }
            Item { width: 1; height: 20 * home.k; visible: home.addRow.length > 0 }
            RowHead { couch: home.couch; width: parent.width; title: "Recently added"; note: "from every store"; visible: home.addRow.length > 0 }
            Item { width: 1; height: 14 * home.k; visible: home.addRow.length > 0 }
            Row {
                spacing: 20 * home.k
                Repeater {
                    model: home.addRow
                    delegate: GameTile {
                        required property var modelData
                        required property int index
                        couch: home.couch
                        game: modelData
                        badge: H.badge(modelData, home.dlMap, home.added, home.now, true)
                        sub: H.addedSub(modelData, home.prog[modelData.id], home.added[modelData.id] || modelData.added, home.now)
                        focused: home.sel(2, index)
                        onActivated: { home.row = 2; home.col = index; home.activate(2, index); }
                    }
                }
            }
        }

        // ------------------------------------------------------------ the cards on the right
        Column {
            id: right
            x: 1340 * home.k; y: 416 * home.k
            width: 508 * home.k
            spacing: 14 * home.k
            HomeDownloads { couch: home.couch; width: parent.width }
            HomeControllers { couch: home.couch; width: parent.width }
            Loader {
                id: mediaCard
                width: parent.width
                visible: status === Loader.Ready && item.has
                // Plasma's own media model (MPRIS); if it can't load, only this card is missing.
                source: "MediaCard.qml"
                onLoaded: item.couch = home.couch
            }
        }

        // ------------------------------------------------------------ further down
        Column {
            id: page2
            x: 72 * home.k
            y: Math.max(left.y + left.height, right.y + right.height, 920 * home.k) + 72 * home.k
            width: flick.width - 144 * home.k
            spacing: 0
            RowHead { couch: home.couch; width: parent.width; title: "Watch and listen"; note: "opens full screen on the " + (home.couch.screenIsTv ? "TV" : home.couch.screenWord) + " · B comes back here"; visible: home.media.length > 0 }
            Item { width: 1; height: 12 * home.k; visible: home.media.length > 0 }
            Row {
                spacing: 16 * home.k
                visible: home.media.length > 0
                Repeater {
                    model: home.media
                    delegate: Item {
                        required property var modelData
                        required property int index
                        width: 206 * home.k; height: mtile.height + 7 * home.k + msub.implicitHeight
                        RoundClip {
                            id: mtile
                            width: 206 * home.k; height: 90 * home.k
                            radius: 14 * home.k
                            Art { anchors.fill: parent; game: ({ id: modelData.key, tones: [modelData.c1, modelData.c2] }); motif: false }
                            Rectangle { x: parent.width * 0.8 - 90 * home.k * 0.4; y: -50 * home.k; width: 90 * home.k; height: width; radius: width / 2; color: "#12ffffff" }
                            Rectangle { x: -20 * home.k; y: 55 * home.k; width: 70 * home.k; height: width; radius: width / 2; color: "#26000000" }
                            T { couchRef: home.couch; x: 16 * home.k; y: 28 * home.k; text: modelData.name; font.pixelSize: 22 * home.k; font.weight: Font.ExtraBold; width: parent.width - 24 * home.k }
                        }
                        Ring { visible: home.sel(3, index); k: home.k; r: 14; anchors.fill: mtile }
                        T {
                            id: msub
                            couchRef: home.couch
                            anchors.top: mtile.bottom; anchors.topMargin: 7 * home.k
                            width: parent.width
                            text: H.mediaSub(modelData, home.player, home.cards.mediaNotes)
                            color: H.P.mut; font.pixelSize: 13 * home.k
                        }
                        MouseArea { anchors.fill: parent; onClicked: { home.row = 3; home.col = index; home.activate(3, index); } }
                    }
                }
            }
            Item { width: 1; height: 26 * home.k; visible: home.beacon.length > 0 }
            RowHead {
                couch: home.couch; width: parent.width; visible: home.beacon.length > 0
                title: "Beacon · Battle for Middle-earth and more"
                note: home.beacon.length + (home.beacon.length === 1 ? " game" : " games") + " · Beacon keeps their mods and patches"
            }
            Item { width: 1; height: 14 * home.k; visible: home.beacon.length > 0 }
            Row {
                spacing: 20 * home.k
                visible: home.beacon.length > 0
                Repeater {
                    model: home.beacon.slice(0, 6)
                    delegate: GameTile {
                        required property var modelData
                        required property int index
                        readonly property bool big: home.sel(4, index)
                        couch: home.couch
                        game: modelData
                        boxW: big ? 330 : 250; boxH: big ? 186 : 140
                        nameSize: big ? 21 : 17; subSize: big ? 15 : 13; nameGap: big ? 14 : 9
                        tag: modelData.tag || (home.prog[modelData.id] && home.prog[modelData.id].mod ? "Beacon · mod" : "Beacon")
                        badge: H.badge(modelData, home.dlMap, home.added, home.now)
                        sub: home.beaconSub(modelData)
                        focused: big
                        onActivated: { home.row = 4; home.col = index; home.activate(4, index); }
                    }
                }
            }
            Item { width: 1; height: 22 * home.k; visible: home.steam.length > 0 }
            RowHead {
                couch: home.couch; width: parent.width; visible: home.steam.length > 0
                title: "Steam · " + home.steamAll.length + " installed"; note: "sorted by last played"
            }
            Item { width: 1; height: 14 * home.k; visible: home.steam.length > 0 }
            Row {
                spacing: 20 * home.k
                visible: home.steam.length > 0
                Repeater {
                    model: home.steam
                    delegate: GameTile {
                        required property var modelData
                        required property int index
                        couch: home.couch
                        game: modelData
                        boxW: 226; boxH: 127
                        badge: H.badge(modelData, home.dlMap, home.added, home.now)
                        focused: home.sel(5, index)
                        onActivated: { home.row = 5; home.col = index; home.activate(5, index); }
                    }
                }
            }
            Item { width: 1; height: 25 * home.k; visible: home.devices.length > 0 }
            T { couchRef: home.couch; visible: home.devices.length > 0; text: "From your Hearth"; font.pixelSize: 22 * home.k; font.weight: Font.Bold }
            Item { width: 1; height: 12 * home.k; visible: home.devices.length > 0 }
            Row {
                spacing: 14 * home.k
                visible: home.devices.length > 0
                Repeater {
                    model: home.devices
                    delegate: Item {
                        required property var modelData
                        required property int index
                        readonly property var c: H.deviceCard(modelData, home.now)
                        width: dcard.width; height: dcard.height
                        Rectangle {
                            id: dcard
                            width: [470, 380, 340][index] * home.k
                            height: 68 * home.k
                            radius: 18 * home.k
                            color: "#db1e1a18"; border.width: 1; border.color: "#12ffffff"
                            Rectangle {
                                id: dico
                                x: 18 * home.k; anchors.verticalCenter: parent.verticalCenter
                                width: 40 * home.k; height: width; radius: 11 * home.k
                                color: ["#2a3a2a", "#3a2a22", "#2a2a36"][index % 3]
                                T { couchRef: home.couch; anchors.centerIn: parent; text: H.deviceLetter(modelData.name); color: index === 0 ? "#cceeee" : H.P.tx; font.pixelSize: 16 * home.k; font.weight: Font.ExtraBold }
                            }
                            Column {
                                anchors.left: dico.right; anchors.leftMargin: 14 * home.k
                                anchors.right: dbtn.visible ? dbtn.left : parent.right; anchors.rightMargin: 14 * home.k
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 3 * home.k
                                T { couchRef: home.couch; width: parent.width; text: parent.parent.parent.c.title; font.pixelSize: 16 * home.k; font.weight: Font.Bold }
                                T { couchRef: home.couch; width: parent.width; text: parent.parent.parent.c.sub; color: H.P.mut; font.pixelSize: 14 * home.k }
                            }
                            Rectangle {
                                id: dbtn
                                visible: parent.parent.c.button !== ""
                                anchors.right: parent.right; anchors.rightMargin: 18 * home.k
                                anchors.verticalCenter: parent.verticalCenter
                                width: dbt.implicitWidth + 28 * home.k; height: dbt.implicitHeight + 16 * home.k
                                radius: 10 * home.k; color: H.P.nb
                                T { id: dbt; couchRef: home.couch; anchors.centerIn: parent; text: parent.parent.parent.c.button; font.pixelSize: 14 * home.k; font.weight: Font.DemiBold }
                            }
                        }
                        Ring { visible: home.sel(6, index); k: home.k; r: 18; anchors.fill: dcard }
                        MouseArea { anchors.fill: parent; onClicked: { home.row = 6; home.col = index; home.activate(6, index); } }
                    }
                }
            }
        }
    }
    function beaconSub(g) {
        const p = prog[g.id] || {};
        const d = dlMap[g.id];
        if (g.update && p.version) return (p.mod ? p.mod + " · " : "") + p.version + " ready" + (p.size ? " · " + p.size : "");
        if (p.campaign && p.chapter) return p.campaign.replace(/ campaign$/i, "") + " · Chapter " + p.chapter;
        if (!g.installed) return "Not installed";
        if (d) return d.state === "running" ? "Downloading · " + d.percent + "%" : "Waiting to download";
        return g.lastPlayed ? "Installed · " + H.agoShort(g.lastPlayed, now).toLowerCase().replace(/^(\d+) days$/, "$1 days ago") : "Installed";
    }

    // ---------------------------------------------------------------- a TV-style notice (a controller's battery is low)
    Rectangle {
        visible: home.toastShown
        x: 72 * home.k; anchors.bottom: parent.bottom; anchors.bottomMargin: 22 * home.k
        width: 600 * home.k; height: 64 * home.k
        radius: 16 * home.k
        color: H.P.note; border.width: 1; border.color: "#17ffffff"
        Rectangle {
            id: tico
            x: 18 * home.k; anchors.verticalCenter: parent.verticalCenter
            width: 34 * home.k; height: width; radius: 11 * home.k; color: "#3a3020"
            Glyph { anchors.centerIn: parent; name: "pad"; size: 20 * home.k; color: H.P.warn }
        }
        Column {
            anchors.left: tico.right; anchors.leftMargin: 14 * home.k; anchors.verticalCenter: parent.verticalCenter
            T { couchRef: home.couch; text: home.lowPad ? (home.lowPad.player === 1 ? "Your controller is at " : "Player " + home.lowPad.player + "'s controller is at ") + home.lowPad.percent + "%" : ""; font.pixelSize: 16 * home.k; font.weight: Font.Bold }
            T { couchRef: home.couch; text: "Plug it in soon · hold ≡ to see all notifications"; color: H.P.mut; font.pixelSize: 14 * home.k }
        }
        MouseArea { anchors.fill: parent; onClicked: home.couch.toastSeen = home.lowPad.name + home.lowPad.percent }
    }

    // ---------------------------------------------------------------- hints
    Row {
        anchors.right: parent.right; anchors.rightMargin: 72 * home.k
        anchors.bottom: parent.bottom; anchors.bottomMargin: 26 * home.k
        spacing: 30 * home.k
        Hint { couch: home.couch; key: "A"; label: home.row === 0 ? "Continue" : "Play" }
        Hint { couch: home.couch; key: "X"; label: "Options" }
        Hint { couch: home.couch; key: "Y"; label: "Search" }
        Hint { couch: home.couch; key: "≡"; label: "Quick Settings"; visible: home.row <= 2 }
        Hint { couch: home.couch; key: "⧉"; label: "Everything open"; visible: home.row <= 2 }
        Hint { couch: home.couch; key: "LB RB"; label: "Tabs" }
        Hint { couch: home.couch; key: "B"; label: "Back to top"; visible: home.row > 2 }
    }
}
