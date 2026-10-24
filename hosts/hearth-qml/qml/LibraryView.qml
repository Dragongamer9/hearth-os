// Library (the web page's viewLibrary, the grid layout): tabs (LB / RB), sorting, search (Y), every game's cover.
import QtQuick

Item {
    id: lib
    readonly property var tabs: {
        const t = [["all", "All games"], ["inst", "Installed"], ["fav", "Favorites"]];
        const have = {};
        App.rev;
        for (const x of App.games) have[x.src] = true;
        for (const s of ["steam", "epic", "gog", "xbox", "manual", "beacon", "emu", "flatpak"]) if (have[s]) t.push([s, Theme.sourceNames[s] || s]);
        if (App.games.some(x => x.hidden)) t.push(["hidden", "Hidden"]);
        return t;
    }
    readonly property var list: {
        App.rev;
        const t = App.libTab, q = App.query.trim().toLowerCase();
        let L = t === "hidden" ? App.games.filter(x => x.hidden) : App.visibleGames();
        if (t === "inst") L = L.filter(x => x.inst || App.isActive(x));
        else if (t === "fav") L = L.filter(x => x.fav);
        else if (t !== "all" && t !== "hidden") L = L.filter(x => x.src === t);
        if (q) L = L.filter(x => x.title.toLowerCase().indexOf(q) >= 0);
        if (App.libSort === "recent") L.sort((a, b) => (b.last - a.last) || a.title.localeCompare(b.title));
        else L.sort((a, b) => a.title.localeCompare(b.title));
        return L;
    }
    Connections {
        target: App
        function onTabStepped(dir) {
            if (App.view !== "library" || App.layers.length) return;
            const i = lib.tabs.findIndex(t => t[0] === App.libTab);
            const n = lib.tabs[(i + dir + lib.tabs.length) % lib.tabs.length];
            App.libTab = n[0];
            page.toTop();
            App.later(30, () => Nav.focusKey("lt-" + n[0]));
        }
    }

    Scroller {
        id: page
        anchors.fill: parent
        contentHeight: col.height + Theme.u(40)
        Column {
            id: col
            x: Theme.gut
            width: page.width - Theme.gut * 2
            topPadding: Theme.u(14)
            spacing: Theme.u(14)

            Flow {
                width: parent.width
                spacing: Theme.u(8)
                Repeater {
                    model: lib.tabs
                    delegate: Chip {
                        required property var modelData
                        text: modelData[1]
                        on: App.libTab === modelData[0]
                        navKey: "lt-" + modelData[0]
                        onActivated: { App.libTab = modelData[0]; page.toTop(); }
                    }
                }
                Item { width: Theme.u(12); height: 1 }
                Chip { text: App.libSort === "alpha" ? "A–Z" : "Recently played"; navKey: "l-sort"; onActivated: App.libSort = App.libSort === "alpha" ? "recent" : "alpha" }
                Chip { text: App.query ? "Search: " + App.query : "Search"; on: App.query !== ""; navKey: "l-search"; onActivated: App.openLayer("search") }
                Chip { visible: App.query !== ""; text: "Clear search"; navKey: "l-clear"; onActivated: App.query = "" }
            }
            Text {
                text: lib.list.length + (lib.list.length === 1 ? " game" : " games") + (App.query ? " matching “" + App.query + "”" : "")
                color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13)
            }
            Text {
                visible: lib.list.length === 0
                text: App.libLoading ? "Loading your games…" : App.query ? "No games match your search." : "Nothing here yet."
                color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(16)
                topPadding: Theme.u(30)
            }
            Flow {
                id: grid
                width: parent.width
                spacing: Theme.u(16)
                bottomPadding: Theme.u(30)
                Repeater {
                    model: lib.list
                    delegate: GameTile {
                        required property var modelData
                        required property int index
                        property bool seen: false
                        readonly property bool near: grid.y + col.y + y + height > page.contentY - Theme.u(400) && grid.y + col.y + y < page.contentY + page.height + Theme.u(400)
                        onNearChanged: if (near) seen = true
                        Component.onCompleted: if (near) seen = true
                        loadArt: seen
                        game: modelData
                        navKey: "lg-" + modelData.id
                        navFirst: index === 0
                        width: Theme.u(150)
                        height: Theme.u(225)
                    }
                }
            }
        }
    }
}
