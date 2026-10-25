// App launcher: a full-screen overlay with search and a grid of installed apps, plus power actions.
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
    readonly property string gamesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-games").toString().replace(/^file:\/\//, ""))
    signal settingsRequested()
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
        if (e && e.folder) return;             // the System apps folder: just Open
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
    color: "#b0080b10"

    // Every app with a .desktop file, A–Z, filtered by what you type.
    readonly property var apps: Array.from(DesktopEntries.applications.values)
        .filter(a => !a.noDisplay && !/^(qt5ct|qt6ct)$/.test(a.id || ""))
        .sort((a, b) => a.name.localeCompare(b.name))
    // Hearth's own apps (and Beacon, and Bazaar, the app store Hearth OS picks): never in the folder.
    function ours(a) { return /^(hearth|nimbus|beacon|io\.github\.kolunmi\.Bazaar$)/i.test(a.id || "") || /^Hearth\b/.test(a.name || ""); }
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
        // Names that start with the query first.
        return apps.filter(hit).sort((a, b) => (b.name.toLowerCase().startsWith(q) ? 1 : 0) - (a.name.toLowerCase().startsWith(q) ? 1 : 0));
    }

    function launch(entry) {
        if (!entry) return;
        if (entry.folder) { folderOpen = true; grid.currentIndex = 0; search.forceActiveFocus(); return; }
        if (tab === "games") Quickshell.execDetached(["python3", gamesHelper, "play", entry.id]);
        else { entry.execute(); DeskState.noteLaunch(entry.id); }
        shown = false;
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
    }
    Process {
        id: systemList
        running: true
        command: ["sh", "-c", "ls -1 /usr/share/applications 2>/dev/null; printf '\\n---\\n'; cat /var/lib/hearth/layer/added-apps 2>/dev/null; true"]
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

    Rectangle {
        id: box
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 880)
        height: Math.min(parent.height - 120, 640)
        radius: 20
        color: Theme.panel
        border.color: Theme.line
        MouseArea { anchors.fill: parent } // clicks inside don't close

        ColumnLayout {
            anchors { fill: parent; margins: 22 }
            spacing: 16

            // Sections
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                TabButton { label: "Apps"; key: "apps" }
                TabButton { label: "Games"; key: "games" }
                // In the System apps folder: where you are, and the way back.
                Rectangle {
                    visible: root.folderOpen && root.tab === "apps" && !search.text
                    implicitWidth: backText.implicitWidth + 28
                    implicitHeight: 38
                    radius: 10
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
                implicitHeight: 48
                radius: 12
                color: Theme.bg2
                border.color: search.activeFocus && !root.tabFocus ? Theme.accent : Theme.line
                TextInput {
                    id: search
                    anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.text
                    selectionColor: Theme.accent
                    font.pixelSize: 17
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
                        text: root.tab === "games" ? "Search games" : "Search apps"
                        color: Theme.muted
                        font: search.font
                    }
                }
            }

            // Recently opened apps (the newest first), above the list when nothing is typed.
            Flow {
                id: recentRow
                readonly property var entries: DeskState.recent.map(id => root.apps.find(a => a.id === id)).filter(Boolean).slice(0, 6)
                Layout.fillWidth: true
                visible: root.tab === "apps" && !search.text && !root.folderOpen && entries.length > 0
                spacing: 8
                Text {
                    height: 36
                    verticalAlignment: Text.AlignVCenter
                    text: "Recent"
                    color: Theme.muted
                    font.pixelSize: 12
                    font.family: Theme.font
                    rightPadding: 4
                }
                Repeater {
                    model: recentRow.visible ? recentRow.entries : []
                    delegate: Rectangle {
                        id: chip
                        required property var modelData
                        implicitWidth: chipRow.implicitWidth + 20
                        implicitHeight: 36
                        radius: 10
                        color: chipArea.containsMouse ? Theme.panel3 : Theme.panel2
                        Row {
                            id: chipRow
                            anchors.centerIn: parent
                            spacing: 8
                            AppIcon { anchors.verticalCenter: parent.verticalCenter; size: 22; icon: chip.modelData.icon || ""; appId: chip.modelData.id || "" }
                            Text { anchors.verticalCenter: parent.verticalCenter; text: chip.modelData.name; color: Theme.text; font.pixelSize: 13; font.family: Theme.font; elide: Text.ElideRight; width: Math.min(implicitWidth, 140) }
                        }
                        MouseArea {
                            id: chipArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: Qt.PointingHandCursor
                            onClicked: mouse => {
                                if (mouse.button === Qt.RightButton) { const p = chip.mapToItem(null, mouse.x, mouse.y); root.menuRequested(chip.modelData, p.x, p.y); }
                                else root.launch(chip.modelData);
                            }
                        }
                    }
                }
            }

            // Apps / games
            GridView {
                id: grid
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.matches
                cellWidth: root.tab === "games" ? Math.floor(width / Math.max(1, Math.floor(width / 150))) : Math.floor(width / Math.max(1, Math.floor(width / 120)))
                cellHeight: root.tab === "games" ? 236 : 112
                keyNavigationEnabled: true
                highlightFollowsCurrentItem: true
                highlight: Rectangle { radius: 14; color: Theme.panel3; border.color: root.tabFocus ? Theme.line : Theme.accent }
                delegate: Item {
                    id: cell
                    required property var modelData
                    required property int index
                    // Games are plain objects with a title; apps are desktop entries.
                    readonly property bool isGame: cell.modelData.title !== undefined
                    readonly property string label: (cell.isGame ? cell.modelData.title : cell.modelData.name) || ""
                    width: grid.cellWidth
                    height: grid.cellHeight
                    Rectangle {
                        anchors { fill: parent; margins: 4 }
                        radius: 14
                        color: hover.containsMouse && grid.currentIndex !== cell.index ? Theme.panel2 : "transparent"
                    }
                    Column {
                        anchors.centerIn: parent
                        spacing: 8
                        AppIcon {
                            visible: !cell.isGame
                            anchors.horizontalCenter: parent.horizontalCenter
                            size: 48
                            icon: cell.isGame ? "" : (cell.modelData.icon || "")
                            appId: cell.isGame ? "" : (cell.modelData.id || "")
                        }
                        // The game's cover (its first letter until the picture is in, or without one).
                        Rectangle {
                            visible: cell.isGame
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 120
                            height: 168
                            radius: 10
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
                            text: cell.label
                            color: Theme.text
                            font.pixelSize: 12
                            font.family: Theme.font
                        }
                    }
                    // Pinned to the taskbar
                    Icon {
                        visible: !cell.isGame && !cell.modelData.folder && DeskState.isPinned(cell.modelData.id)
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

            // Power
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                ActionButton { label: "Game Mode"; primary: true; command: ["nimbus-mode", "game"] }
                ActionButton { label: "Settings"; command: [] }
                Item { Layout.fillWidth: true }
                ActionButton { label: "Sleep"; command: ["python3", Buttons.helper, "sleep", "Desktop Mode: menu"] }
                ActionButton { label: "Restart"; command: ["systemctl", "reboot"] }
                ActionButton { label: "Shut down"; command: ["systemctl", "poweroff"] }
                ActionButton { label: "Log out"; command: ["nimbus-mode", "logout"] }
            }
        }
    }

    // A section button (Apps, Games): lit when it's the one shown, ringed when the D-pad is on the buttons.
    component TabButton: Rectangle {
        id: tb
        property string label
        property string key
        readonly property bool current: root.tab === tb.key
        implicitWidth: tbText.implicitWidth + 36
        implicitHeight: 38
        radius: 10
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
        property var command: []
        property bool primary: false
        implicitWidth: abText.implicitWidth + 32
        implicitHeight: 40
        radius: 10
        color: primary ? (abArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent) : (abArea.containsMouse ? Theme.panel3 : Theme.panel2)
        Text { id: abText; anchors.centerIn: parent; text: ab.label; color: ab.primary ? Theme.accentInk : Theme.text; font.pixelSize: 13; font.bold: ab.primary; font.family: Theme.font }
        MouseArea {
            id: abArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { root.shown = false; if (ab.command.length) Quickshell.execDetached(ab.command); else root.settingsRequested(); }
        }
    }
}
