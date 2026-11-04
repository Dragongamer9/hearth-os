// App launcher (Mantel 2026: a large centred panel over the darkened desktop): search, a grid of installed apps,
// plus power actions. Searching in Apps also finds Settings pages and your games; Recent can hold all three.
// Opens with Super, the Nimbus button on the taskbar, or `quickshell ipc ... call launcher toggle`.
// Type to search, arrows to move, Enter to open, Esc to close.
// Two sections: Apps (every app with a .desktop file) and Games (your installed games from Hearth's library, through
// hearth-games; they start through Hearth, like its Play button). Switch with Tab / Shift+Tab, or go up from the
// top row to the section buttons and press left or right (the D-pad).
// In Apps, Hearth's own apps and the apps you installed come first; the apps that came with the system (everything
// else in /usr/share/applications) are in one "System apps" folder at the end. Searching looks through all of them.
// The apps you opened last are in a Recent row above the list (DeskState.recent). Right-click an app for its own
// actions, Pin to taskbar, Add to desktop, and Uninstall for apps from Flathub (shell.qml).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets

PanelWindow {
    id: root
    property bool shown: false
    property string tab: "apps"                // "apps" or "games"
    property bool tabFocus: false              // the section buttons have the keyboard / controller focus
    property var games: []                     // hearth-games: [{id, title, source, cover, wide, icon}]
    property bool gamesLoading: false
    property double gamesAt: 0                 // when Hearth was last asked (it takes a moment)
    property bool folderOpen: false            // the System apps folder is open
    property var systemIds: ({})               // desktop ids that came with the system image
    property var hiddenIds: ({})               // not for this desktop (OnlyShowIn/NotShowIn: docs/APP-KINDS.md)
    readonly property string gamesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-games").toString().replace(/^file:\/\//, ""))
    signal settingsRequested()
    signal pageRequested(string page)          // a Settings page found by the search (or in Recent)
    // Right-click, press and hold, or the Menu key (X on the controller): Open / Pin to taskbar (shell.qml).
    signal menuRequested(var entry, real x, real y)
    function toggle() { shown = !shown; }
    function openTab(t) { setTab(t); shown = true; }
    function setTab(t) {
        if (tab === t) return;
        tab = t;
        folderOpen = false;
        grid.currentIndex = 0;
        if (t === "games") loadGames();
    }
    function nextTab() { setTab(tab === "apps" ? "games" : "apps"); }
    function menuFor(i) {
        if (tab !== "apps") return;            // games: just Play (Enter)
        const e = matches[i];
        if (e && (e.folder || e.page !== undefined || e.title !== undefined)) return;   // the folder, Settings pages, games: just Open
        const cell = grid.itemAtIndex(i);
        if (!e) return;
        const p = cell ? cell.mapToItem(null, cell.width / 2, cell.height / 2) : Qt.point(width / 2, height / 2);
        menuRequested(e, p.x, p.y);
    }

    visible: shown
    onShownChanged: if (shown) { search.text = ""; folderOpen = false; grid.currentIndex = 0; tabFocus = false; search.forceActiveFocus(); loadGames(); systemList.running = true; }
    function closeFolder() { folderOpen = false; grid.currentIndex = Math.max(0, mainApps.length); }

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-launcher"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Qt.rgba(0.02, 0.02, 0.03, 0.72)

    // Every app with a .desktop file, A–Z, filtered by what you type.
    // Not the Hearth app itself (nimbus.desktop, "Hearth"): that's the archived Game Mode in a window, hidden everywhere
    // people look (docs/SETTINGS-MOVE.md); HEARTH_LEGACY_GAME_MODE=1 (the developer switch) shows it again.
    readonly property bool legacyGame: Quickshell.env("HEARTH_LEGACY_GAME_MODE") === "1"
    readonly property var apps: Array.from(DesktopEntries.applications.values)
        .filter(a => !a.noDisplay && !root.hiddenIds[a.id] && !/^(qt5ct|qt6ct)$/.test(a.id || "") && (root.legacyGame || !/^nimbus(\.desktop)?$/.test(a.id || "")))
        .sort((a, b) => a.name.localeCompare(b.name))
    // Hearth's own apps (and Beacon, Xodus Nexus, and Bazaar, the app store Hearth OS picks): never in the folder.
    function ours(a) { return /^(hearth|nimbus|beacon|xodus-nexus$|io\.github\.kolunmi\.Bazaar$)/i.test(a.id || "") || /^Hearth\b/.test(a.name || ""); }
    function systemApp(a) { return !ours(a) && root.systemIds[a.id] === true; }
    readonly property var mainApps: apps.filter(a => !systemApp(a))
    readonly property var sysApps: apps.filter(a => systemApp(a))
    readonly property var folderTile: ({ folder: true, id: "", name: "System apps", icon: "folder", genericName: "", comment: "", keywords: [] })
    readonly property var matches: {
        const q = search.text.trim().toLowerCase();
        if (tab === "games") {
            if (!q) return games;
            return games.filter(g => (g.title || "").toLowerCase().includes(q))
                .sort((a, b) => (b.title.toLowerCase().startsWith(q) ? 1 : 0) - (a.title.toLowerCase().startsWith(q) ? 1 : 0));
        }
        if (!q) return folderOpen ? sysApps : (sysApps.length ? mainApps.concat([folderTile]) : mainApps);
        const hit = a => [a.name, a.genericName, a.comment, Array.from(a.keywords || []).join(" ")].join(" ").toLowerCase().includes(q);
        // Names that start with the query first; then Settings pages and games that match.
        const found = apps.filter(hit).sort((a, b) => (b.name.toLowerCase().startsWith(q) ? 1 : 0) - (a.name.toLowerCase().startsWith(q) ? 1 : 0));
        const pages = settingsPages.filter(p => (p.name + " " + p.words).toLowerCase().includes(q));
        const gs = games.filter(g => (g.title || "").toLowerCase().includes(q)).slice(0, 8);
        return found.concat(pages).concat(gs);
    }

    // Settings pages for the search: [{page, name, icon, words}] from Hearth Settings' list (SettingsApp.qml).
    readonly property var settingsPages: {
        const app = SettingsNav.settingsApp;
        if (!app || !app.pages) return [];
        const kw = app.keywords || {};
        return app.pages.map(p => ({ page: p.id, name: p.label, icon: p.icon, words: kw[p.id] || "" }));
    }
    function launch(entry) {
        if (!entry) return;
        if (entry.folder) { folderOpen = true; grid.currentIndex = 0; search.forceActiveFocus(); return; }
        if (entry.page !== undefined) { DeskState.noteLaunch("page:" + entry.page); root.pageRequested(entry.page); }
        else if (entry.title !== undefined) { DeskState.noteLaunch("game:" + entry.id); Quickshell.execDetached(["python3", gamesHelper, "play", entry.id]); }
        else { entry.execute(); DeskState.noteLaunch(entry.id); }
        shown = false;
    }
    // Recent: apps, games ("game:<id>") and Settings pages ("page:<id>"), newest first.
    function recentEntry(id) {
        if (id.startsWith("page:")) return settingsPages.find(p => p.page === id.slice(5)) || null;
        if (id.startsWith("game:")) return games.find(g => "game:" + g.id === id) || null;
        return apps.find(a => a.id === id) || null;
    }

    // ---- the Games section: what Hearth said last at once, then Hearth's answer now (at most once a minute)
    function loadGames() {
        if (games.length === 0 && !gamesCached.running) gamesCached.running = true;
        if (gamesList.running || Date.now() - gamesAt < 60000) return;
        gamesAt = Date.now();
        gamesLoading = true;
        gamesList.running = true;
    }
    function takeGames(text, fresh) {
        const lines = text.trim().split("\n");
        try {
            const g = JSON.parse(lines[lines.length - 1] || "[]");
            if (Array.isArray(g) && (fresh || games.length === 0) && JSON.stringify(g) !== JSON.stringify(games)) games = g;
        } catch (e) {}
    }
    // The apps that came with the system: the image's own .desktop files. Apps added with apt on the Debian editions
    // (listed by Hearth's package layer in /var/lib/hearth/layer/added-apps) count as yours, not the system's.
    function takeSystem(text) {
        const parts = text.split("\n---\n");
        const added = {};
        for (const l of (parts[1] || "").split("\n")) { const id = l.trim().replace(/\.desktop$/, ""); if (id) added[id] = true; }
        const ids = {};
        for (const l of (parts[0] || "").split("\n")) {
            const id = l.trim().replace(/\.desktop$/, "");
            if (id && l.trim().endsWith(".desktop") && !added[id]) ids[id] = true;
        }
        systemIds = ids;
        // One app of each kind per desktop: what hearth-app-kinds says this desktop doesn't show.
        const hid = {};
        for (const l of (parts[2] || "").split("\n")) { const id = l.trim(); if (id) hid[id] = true; }
        hiddenIds = hid;
    }
    Process {
        id: systemList
        running: true
        command: ["sh", "-c", "ls -1 /usr/share/applications 2>/dev/null; printf '\\n---\\n'; cat /var/lib/hearth/layer/added-apps 2>/dev/null; printf '\\n---\\n'; python3 /usr/libexec/hearth-app-kinds hidden 2>/dev/null; true"]
        stdout: StdioCollector { onStreamFinished: root.takeSystem(text) }
    }
    Process {
        id: gamesCached
        command: ["python3", root.gamesHelper, "cached"]
        stdout: StdioCollector { onStreamFinished: root.takeGames(text, false) }
    }
    Process {
        id: gamesList
        command: ["python3", root.gamesHelper, "list"]
        stdout: StdioCollector { onStreamFinished: { root.takeGames(text, true); root.gamesLoading = false; } }
    }

    MouseArea { anchors.fill: parent; onClicked: root.shown = false } // click outside closes

    Image {   // a warm glow behind the panel
        anchors.centerIn: parent
        width: parent.width * 1.1; height: parent.height
        source: Qt.resolvedUrl("images/ember-glow.png")
        opacity: Theme.dark ? 0.18 : 0.1
        smooth: true
    }
    Item {
        id: box
        readonly property int topRoom: Math.max(DeskState.topInset, 20)
        readonly property int bottomRoom: Math.max(DeskState.bottomInset, 20)
        width: Math.min(parent.width - 60, 1240)
        height: Math.min(parent.height - topRoom - bottomRoom - 30, 860)
        x: (parent.width - width) / 2
        y: topRoom + Math.max(10, (parent.height - topRoom - bottomRoom - height) / 2)
        MouseArea { anchors.fill: parent } // clicks inside don't close
        Rectangle { anchors.fill: parent; radius: Theme.r(26); color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.96); border.color: Theme.line; visible: frame.status !== Loader.Ready }
        Loader { id: frame; anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.r(26)); } }

        ColumnLayout {
            anchors { fill: parent; margins: 30 }
            spacing: 16

            // Sections
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text { text: root.tab === "games" ? "Games" : "Apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold; Layout.rightMargin: 10 }
                TabButton { label: "Apps"; key: "apps" }
                TabButton { label: "Games"; key: "games" }
                // In the System apps folder: where you are, and the way back.
                Rectangle {
                    visible: root.folderOpen && root.tab === "apps" && !search.text
                    implicitWidth: backText.implicitWidth + 28
                    implicitHeight: 38
                    radius: Theme.r(10)
                    color: backArea.containsMouse ? Theme.panel3 : Theme.panel2
                    Text { id: backText; anchors.centerIn: parent; text: "‹ System apps"; color: Theme.text; font.pixelSize: 14; font.family: Theme.font }
                    MouseArea { id: backArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { root.closeFolder(); search.forceActiveFocus(); } }
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: root.tabFocus ? "◀ ▶ switch · ▼ back to the list" : (root.folderOpen && !search.text ? "Esc goes back · Tab switches" : "Tab switches")
                    color: Theme.muted
                    font.pixelSize: 12
                    font.family: Theme.font
                }
            }

            // Search
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 60
                radius: Theme.r(30)
                color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.8)
                border.width: 2
                border.color: search.activeFocus && !root.tabFocus ? Theme.accent : Theme.line
                Icon { x: 20; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 22; color: Theme.accent }
                Text { anchors { right: parent.right; rightMargin: 24; verticalCenter: parent.verticalCenter } visible: !search.text; text: "Just start typing"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                TextInput {
                    id: search
                    anchors { fill: parent; leftMargin: 56; rightMargin: 160 }
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.text
                    selectionColor: Theme.accent
                    font.pixelSize: 19
                    font.family: Theme.font
                    onTextChanged: grid.currentIndex = 0
                    Keys.onEscapePressed: { if (root.tabFocus) root.tabFocus = false; else if (root.folderOpen && !search.text) root.closeFolder(); else root.shown = false; }
                    Keys.onMenuPressed: root.menuFor(grid.currentIndex)
                    Keys.onReturnPressed: { if (root.tabFocus) root.tabFocus = false; else root.launch(root.matches[grid.currentIndex]); }
                    Keys.onEnterPressed: { if (root.tabFocus) root.tabFocus = false; else root.launch(root.matches[grid.currentIndex]); }
                    Keys.onTabPressed: root.nextTab()
                    Keys.onBacktabPressed: root.nextTab()
                    Keys.onDownPressed: { if (root.tabFocus) root.tabFocus = false; else grid.moveCurrentIndexDown(); }
                    Keys.onUpPressed: {
                        // From the top row up to the section buttons.
                        const cols = Math.max(1, Math.floor(grid.width / grid.cellWidth));
                        if (grid.currentIndex < cols) root.tabFocus = true;
                        else grid.moveCurrentIndexUp();
                    }
                    Keys.onRightPressed: event => {
                        if (root.tabFocus) root.setTab("games");
                        else if (cursorPosition === text.length) grid.moveCurrentIndexRight();
                        else event.accepted = false;
                    }
                    Keys.onLeftPressed: event => {
                        if (root.tabFocus) root.setTab("apps");
                        else if (cursorPosition === text.length && text === "") grid.moveCurrentIndexLeft();
                        else event.accepted = false;
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: !search.text
                        text: root.tab === "games" ? "Search games" : "Search apps, games and Settings pages"
                        color: Theme.muted
                        font: search.font
                    }
                }
            }

            // Recently opened apps, games and Settings pages (the newest first), above the list when nothing is typed.
            Text {
                visible: recentRow.visible
                text: "RECENT"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 1.3
            }
            RowLayout {
                id: recentRow
                readonly property var entries: DeskState.recent.map(id => root.recentEntry(id)).filter(Boolean).slice(0, 3)
                Layout.fillWidth: true
                visible: root.tab === "apps" && !search.text && !root.folderOpen && entries.length > 0
                spacing: 12
                Repeater {
                    model: recentRow.visible ? recentRow.entries : []
                    delegate: Rectangle {
                        id: chip
                        required property var modelData
                        readonly property bool isPage: modelData.page !== undefined
                        readonly property bool isGame: modelData.title !== undefined
                        Layout.fillWidth: true
                        Layout.maximumWidth: 380
                        implicitHeight: 72
                        radius: Theme.r(16)
                        color: chipArea.containsMouse ? Theme.panel3 : Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.85)
                        border.color: Theme.line
                        RowLayout {
                            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                            spacing: 14
                            Rectangle {
                                implicitWidth: 44; implicitHeight: 44; radius: Theme.r(12)
                                color: chip.isPage || chip.isGame ? Theme.panel3 : "transparent"
                                AppIcon { anchors.centerIn: parent; visible: !chip.isPage && !chip.isGame; size: 40; icon: chip.modelData.icon || ""; appId: chip.modelData.id || "" }
                                Icon { anchors.centerIn: parent; visible: chip.isPage || chip.isGame; name: chip.isGame ? "pad" : (chip.modelData.icon || "gear"); size: 22 }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Text { Layout.fillWidth: true; text: chip.isGame ? chip.modelData.title : chip.modelData.name; color: Theme.text; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: Theme.font; elide: Text.ElideRight }
                                Text { Layout.fillWidth: true; text: chip.isGame ? "Game" : chip.isPage ? "Settings page" : "App"; color: Theme.muted; font.pixelSize: 12; font.family: Theme.font }
                            }
                        }
                        MouseArea {
                            id: chipArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: Qt.PointingHandCursor
                            onClicked: mouse => {
                                if (mouse.button === Qt.RightButton && !chip.isPage && !chip.isGame) { const p = chip.mapToItem(null, mouse.x, mouse.y); root.menuRequested(chip.modelData, p.x, p.y); }
                                else root.launch(chip.modelData);
                            }
                        }
                    }
                }
                Item { Layout.fillWidth: true }
            }
            Text {
                visible: root.tab === "apps" && !search.text
                text: root.folderOpen ? "SYSTEM APPS" : "ALL APPS"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 1.3
            }

            // Apps / games
            GridView {
                id: grid
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.matches
                cellWidth: root.tab === "games" ? Math.floor(width / Math.max(1, Math.floor(width / 150))) : Math.floor(width / Math.max(1, Math.floor(width / 140)))
                cellHeight: root.tab === "games" ? 236 : 128
                keyNavigationEnabled: true
                highlightFollowsCurrentItem: true
                highlight: Rectangle { radius: Theme.r(14); color: Theme.panel3; border.color: root.tabFocus ? Theme.line : Theme.accent }
                delegate: Item {
                    id: cell
                    required property var modelData
                    required property int index
                    // Games are plain objects with a title; Settings pages ({page, name, icon}) come from the search; apps are desktop entries.
                    readonly property bool isPage: cell.modelData.page !== undefined
                    readonly property bool isGame: cell.modelData.title !== undefined && root.tab === "games"
                    readonly property bool gameHit: cell.modelData.title !== undefined && root.tab !== "games"
                    readonly property string label: (cell.modelData.title !== undefined ? cell.modelData.title : cell.modelData.name) || ""
                    width: grid.cellWidth
                    height: grid.cellHeight
                    Rectangle {
                        anchors { fill: parent; margins: 4 }
                        radius: Theme.r(14)
                        color: hover.containsMouse && grid.currentIndex !== cell.index ? Theme.panel2 : "transparent"
                    }
                    Column {
                        anchors.centerIn: parent
                        spacing: 8
                        AppIcon {
                            visible: !cell.isGame && !cell.isPage && !cell.gameHit
                            anchors.horizontalCenter: parent.horizontalCenter
                            size: 64
                            icon: cell.isGame || cell.isPage || cell.gameHit ? "" : (cell.modelData.icon || "")
                            appId: cell.isGame || cell.isPage || cell.gameHit ? "" : (cell.modelData.id || "")
                        }
                        // A Settings page or a game found by the search: Hearth's own icon in a tile.
                        Rectangle {
                            visible: cell.isPage || cell.gameHit
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 64; height: 64; radius: Theme.r(18)
                            color: Theme.panel3
                            Icon { anchors.centerIn: parent; name: cell.gameHit ? "pad" : (cell.modelData.icon || "gear"); size: 28 }
                        }
                        // The game's cover (its first letter until the picture is in, or without one).
                        Rectangle {
                            visible: cell.isGame
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 120
                            height: 168
                            radius: Theme.r(10)
                            color: Theme.panel3
                            Text {
                                anchors.centerIn: parent
                                visible: cover.status !== Image.Ready
                                text: cell.label.charAt(0).toUpperCase()
                                color: Theme.muted
                                font.pixelSize: 40
                                font.family: Theme.font
                            }
                            Image {
                                id: cover
                                anchors.fill: parent
                                visible: status === Image.Ready
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                                cache: true
                                sourceSize.width: 240
                                source: cell.isGame ? (cell.modelData.cover || cell.modelData.wide || cell.modelData.icon || "") : ""
                            }
                        }
                        Text {
                            width: grid.cellWidth - 16
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                            maximumLineCount: 2
                            wrapMode: Text.Wrap
                            text: cell.label + (cell.isPage ? "\nSettings" : cell.gameHit ? "\nGame" : "")
                            color: Theme.text
                            font.pixelSize: 13
                            font.family: Theme.font
                        }
                    }
                    // Pinned to the taskbar
                    Icon {
                        visible: !cell.isGame && !cell.isPage && !cell.gameHit && !cell.modelData.folder && DeskState.isPinned(cell.modelData.id)
                        anchors { top: parent.top; right: parent.right; margins: 12 }
                        name: "pin"; size: 14; color: Theme.muted
                    }
                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: mouse => {
                            root.tabFocus = false;
                            if (mouse.button === Qt.RightButton) { grid.currentIndex = cell.index; root.menuFor(cell.index); }
                            else root.launch(cell.modelData);
                        }
                        onPressAndHold: { grid.currentIndex = cell.index; root.menuFor(cell.index); }
                    }
                }
                Text {
                    anchors.centerIn: parent
                    width: parent.width - 40
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    visible: grid.count === 0
                    text: search.text ? "No " + (root.tab === "games" ? "games" : "apps") + " match \"" + search.text + "\""
                        : root.tab === "games" ? (root.gamesLoading ? "Finding your games…" : "No games installed yet. Games you install in Hearth show up here.")
                        : "No apps"
                    color: Theme.muted
                    font.pixelSize: 14
                    font.family: Theme.font
                }
            }

            Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.line }
            // Power: Game Mode first and in the accent
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                ActionButton { label: "Game Mode"; icon: "pad"; primary: true; command: ["nimbus-mode", "game"] }
                ActionButton { label: "Settings"; icon: "gear"; command: [] }
                ActionButton { label: "Sleep"; icon: "sleep"; command: ["python3", Buttons.helper, "sleep", "Desktop Mode: menu"] }
                ActionButton { label: "Restart"; icon: "restart"; command: ["systemctl", "reboot"] }
                ActionButton { label: "Shut down"; icon: "power"; command: ["systemctl", "poweroff"] }
                ActionButton { label: "Log out"; icon: "logout"; command: ["nimbus-mode", "logout"] }
                Item { Layout.fillWidth: true }
                Repeater {
                    model: root.width > 1500 ? [{ b: "A", c: "#3fae5a", t: "Open" }, { b: "X", c: "#3b7de0", t: "Options" }, { b: "LB", c: "#6c7480", t: "Tab" }] : []
                    delegate: Row {
                        required property var modelData
                        spacing: 6
                        Rectangle { width: 24; height: 24; radius: 12; color: modelData.c; Text { anchors.centerIn: parent; text: modelData.b; color: "white"; font.pixelSize: modelData.b.length > 1 ? 9 : 11; font.bold: true } }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: modelData.t; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
            }
        }
    }

    // A section button (Apps, Games): lit when it's the one shown, ringed when the D-pad is on the buttons.
    component TabButton: Rectangle {
        id: tb
        property string label
        property string key
        readonly property bool current: root.tab === tb.key
        implicitWidth: tbText.implicitWidth + 44
        implicitHeight: 44
        radius: Theme.r(14)
        color: tb.current ? Theme.accent : (tbArea.containsMouse ? Theme.panel3 : Theme.panel2)
        border.width: root.tabFocus && tb.current ? 2 : 0
        border.color: Theme.text
        Text { id: tbText; anchors.centerIn: parent; text: tb.label; color: tb.current ? Theme.accentInk : Theme.text; font.pixelSize: 14; font.bold: tb.current; font.family: Theme.font }
        MouseArea {
            id: tbArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { root.setTab(tb.key); search.forceActiveFocus(); }
        }
    }

    component ActionButton: Rectangle {
        id: ab
        property string label
        property string icon: ""
        property var command: []
        property bool primary: false
        implicitWidth: abRow.implicitWidth + 36
        implicitHeight: 52
        radius: Theme.r(14)
        color: primary ? (abArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent) : (abArea.containsMouse ? Theme.panel3 : Theme.panel2)
        border.color: primary ? "transparent" : Theme.line
        Row {
            id: abRow
            anchors.centerIn: parent
            spacing: 10
            Icon { visible: ab.icon !== ""; anchors.verticalCenter: parent.verticalCenter; name: ab.icon; size: 20; color: ab.primary ? Theme.accentInk : Theme.text }
            Text { id: abText; anchors.verticalCenter: parent.verticalCenter; text: ab.label; color: ab.primary ? Theme.accentInk : Theme.text; font.pixelSize: 15; font.bold: ab.primary; font.family: Theme.font }
        }
        MouseArea {
            id: abArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { root.shown = false; if (ab.command.length) Quickshell.execDetached(ab.command); else root.settingsRequested(); }
        }
    }
}
