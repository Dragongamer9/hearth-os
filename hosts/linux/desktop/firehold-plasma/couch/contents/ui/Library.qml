// Library (mockup firehold-plasma 03's page, as 1.16 draws it): every game from every store, the filter chips with
// their counts (All, Installed, Steam, Beacon, Heroic, Windows apps, Your games, Emulators), wide tiles six a row with
// the store's tag, the search box at the end of the chips (Y). X on a tile opens its Game options.
import QtQuick
import "firehold.js" as F
import "fhp.js" as H

Item {
    id: lib
    property QtObject couch
    readonly property real k: couch.k
    property string filter: "all"
    property string query: ""
    function heroic(g) { return ["gog", "epic", "amazon"].indexOf(g.source) >= 0; }
    readonly property var chips: {
        const L = couch.library;
        const c = [{ key: "all", label: "All", n: couch.counts.all || L.length }, { key: "installed", label: "Installed", n: L.filter(function (g) { return g.installed; }).length },
                   { key: "steam", label: "Steam", n: L.filter(function (g) { return g.source === "steam"; }).length },
                   { key: "beacon", label: "Beacon", n: L.filter(function (g) { return g.source === "beacon"; }).length },
                   { key: "heroic", label: "Heroic", n: L.filter(lib.heroic).length },
                   { key: "winapp", label: "Windows apps", n: L.filter(function (g) { return g.kind === "winapp"; }).length },
                   { key: "yours", label: "Your games", n: L.filter(function (g) { return g.kind === "yours"; }).length },
                   { key: "emu", label: "Emulators", n: L.filter(function (g) { return g.kind === "emu"; }).length }];
        return c.filter(function (x) { return x.key === "all" || x.key === "installed" || x.n > 0; });
    }
    readonly property var games: {
        const L = filter === "beacon" ? couch.library.filter(function (g) { return g.source === "beacon"; })
                : filter === "heroic" ? couch.library.filter(lib.heroic) : couch.library;
        return F.filterGames(L, filter === "beacon" || filter === "heroic" ? "all" : filter, query, couch.playing);
    }
    function tagFor(g) {
        if (heroic(g)) return g.store + " · Heroic";
        if (g.devBuild) return g.store + " · dev build";
        return g.store || "";
    }
    property int current: 0
    onGamesChanged: if (current >= games.length) current = Math.max(0, games.length - 1)

    function key(kk) {
        if (search.activeFocus) return false;
        const n = games.length;
        if (kk === Qt.Key_Left) current = Math.max(0, current - 1);
        else if (kk === Qt.Key_Right) current = Math.min(n - 1, current + 1);
        else if (kk === Qt.Key_Up) current = Math.max(0, current - grid.cols);
        else if (kk === Qt.Key_Down) current = Math.min(n - 1, current + grid.cols);
        else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) couch.play(games[current], false);
        else if (kk === Qt.Key_X) { if (games[current]) couch.openOptions(games[current]); }
        else if (kk === Qt.Key_Y) search.forceActiveFocus();
        else if (kk === Qt.Key_Comma || kk === Qt.Key_Period) {
            const keys = chips.map(function (c) { return c.key; });
            const i = Math.max(0, Math.min(keys.length - 1, keys.indexOf(filter) + (kk === Qt.Key_Comma ? -1 : 1)));
            filter = keys[i];
        } else return false;
        grid.positionViewAtIndex(current, GridView.Contain);
        return true;
    }

    // ---------------------------------------------------------------- search
    Rectangle {
        x: parent.width - 452 * lib.k; y: 124 * lib.k
        width: 380 * lib.k; height: 44 * lib.k; radius: height / 2
        visible: lib.couch.optionsGame === null
        color: "#1f1714"
        border.width: search.activeFocus ? 2 * lib.k : 1
        border.color: search.activeFocus ? lib.couch.pal.accent : lib.couch.pal.line
        Rectangle {
            x: 26 * lib.k; anchors.verticalCenter: parent.verticalCenter
            width: 14 * lib.k; height: width; radius: width / 2
            color: "transparent"; border.width: 2 * lib.k; border.color: lib.couch.pal.dim
        }
        TextInput {
            id: search
            x: 54 * lib.k; anchors.verticalCenter: parent.verticalCenter
            width: parent.width - 80 * lib.k
            color: lib.couch.pal.text
            font.family: lib.couch.fontName
            font.pixelSize: 17 * lib.k
            clip: true
            onTextChanged: { lib.query = text; lib.current = 0; }
            Keys.onEscapePressed: lib.couch.forceActiveFocus()
            Keys.onReturnPressed: lib.couch.forceActiveFocus()
            Keys.onDownPressed: lib.couch.forceActiveFocus()
        }
        T {
            couchRef: lib.couch
            anchors.verticalCenter: parent.verticalCenter; x: search.x
            visible: search.text === ""
            text: "Search " + (lib.couch.counts.all || lib.couch.library.length) + " games"
            color: lib.couch.pal.dim
            font.pixelSize: 17 * lib.k
        }
        MouseArea { anchors.fill: parent; onClicked: search.forceActiveFocus() }
    }

    // ---------------------------------------------------------------- the chips (the mockup's pills: "All · 412")
    Row {
        x: 72 * lib.k; y: 124 * lib.k
        spacing: 10 * lib.k
        Repeater {
            model: lib.chips
            delegate: Pill {
                required property var modelData
                couch: lib.couch
                text: modelData.label + " · " + modelData.n
                color: lib.filter === modelData.key ? H.P.ac : "#12ffffff"
                textColor: lib.filter === modelData.key ? H.P.ink : H.P.tx
                clickable: true
                onClicked: { lib.filter = modelData.key; lib.current = 0; }
            }
        }
    }

    // ---------------------------------------------------------------- the tiles
    GridView {
        id: grid
        readonly property int cols: 6
        x: 72 * lib.k; y: 200 * lib.k
        width: cols * cellWidth; height: parent.height - y - 90 * lib.k
        cellWidth: 274 * lib.k; cellHeight: 166 * lib.k
        clip: true
        interactive: true
        boundsBehavior: Flickable.StopAtBounds
        model: lib.games
        currentIndex: lib.current
        highlightMoveDuration: 0
        cacheBuffer: 0
        topMargin: 8 * lib.k; leftMargin: 8 * lib.k
        delegate: Item {
            id: cell
            required property var modelData
            required property int index
            width: 248 * lib.k; height: 140 * lib.k
            RoundClip {
                id: tile
                anchors.fill: parent
                radius: 14 * lib.k
                Art { anchors.fill: parent; game: cell.modelData }
                Rectangle {
                    x: 10 * lib.k; anchors.bottom: parent.bottom; anchors.bottomMargin: 9 * lib.k
                    width: tag.implicitWidth + 14 * lib.k; height: tag.implicitHeight + 6 * lib.k
                    radius: 6 * lib.k; color: "#59000000"
                    T { id: tag; couchRef: lib.couch; anchors.centerIn: parent; text: lib.tagFor(cell.modelData); color: "#d9ffffff"; font.pixelSize: 12 * lib.k; font.weight: Font.DemiBold }
                }
                Rectangle {
                    readonly property string st: F.status(cell.modelData, lib.couch.playing)
                    visible: st === "Playing" || st === "Update ready" || st === "Not installed"
                    anchors.right: parent.right; anchors.rightMargin: 9 * lib.k; y: 9 * lib.k
                    width: stt.implicitWidth + 14 * lib.k; height: stt.implicitHeight + 6 * lib.k
                    radius: 6 * lib.k; color: "#73000000"
                    T { id: stt; couchRef: lib.couch; anchors.centerIn: parent; text: parent.st.toUpperCase(); color: parent.st === "Not installed" ? "#cccccc" : H.P.ac2; font.pixelSize: 11 * lib.k; font.weight: Font.Bold }
                }
            }
            Ring { visible: cell.index === lib.current && lib.couch.front; k: lib.k; r: 14; anchors.fill: tile }
            MouseArea { anchors.fill: parent; onClicked: { lib.current = cell.index; lib.couch.play(cell.modelData, false); } }
        }
    }
    T {
        couchRef: lib.couch
        visible: lib.games.length === 0
        x: 72 * lib.k; y: 240 * lib.k
        text: lib.couch.libraryLoaded ? "No games here" : "Reading your games…"
        color: lib.couch.pal.dim; font.pixelSize: 22 * lib.k
    }

    Row {
        anchors.right: parent.right; anchors.rightMargin: 72 * lib.k
        anchors.bottom: parent.bottom; anchors.bottomMargin: 26 * lib.k
        spacing: 30 * lib.k
        visible: lib.couch.optionsGame === null
        Hint { couch: lib.couch; key: "A"; label: "Play" }
        Hint { couch: lib.couch; key: "X"; label: "Game options" }
        Hint { couch: lib.couch; key: "Y"; label: "Search" }
        Hint { couch: lib.couch; key: "LT RT"; label: "Filters" }
    }
}
