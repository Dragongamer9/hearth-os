// Hearth Plasma: Hearth's app launcher, ported from Hearth Classic's (Mantel's Launcher.qml). The Hearth mark on the
// taskbar (and Super) opens it: search, then two sections, Apps (Hearth's own apps and yours first, the apps that
// came with the system in one "System apps" folder at the end) and Games (your installed games from Hearth's library,
// started through Hearth). Arrows move, Enter opens, Tab switches section, Esc closes. Power actions along the bottom.
// The lists come from desktop/plasma/hearth-plasma (apps: the same .desktop reading as Hearth Files; games:
// hearth-games), so it shows exactly what Mantel's launcher shows.
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Plasmoid.icon: "hearth"
    toolTipMainText: "Apps"
    toolTipSubText: "Your apps and games (Super)"
    switchWidth: Kirigami.Units.gridUnit * 20
    switchHeight: Kirigami.Units.gridUnit * 20

    readonly property string helper: "/usr/share/nimbus/desktop/plasma/hearth-plasma"
    property var apps: []                 // hearth-plasma apps
    property var games: []                // hearth-plasma games
    property string section: "apps"       // apps | games
    property bool folderOpen: false       // inside the System apps folder
    property string query: ""

    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    function lastJson(out, fallback) {
        const lines = String(out || "").trim().split("\n");
        try { return JSON.parse(lines[lines.length - 1]); } catch (e) { return fallback; }
    }
    // Commands through Plasma's executable engine; each run gets its own source name so two can't collide.
    P5Support.DataSource {
        id: exec
        engine: "executable"
        connectedSources: []
        property var callbacks: ({})
        property int n: 0
        onNewData: (source, data) => {
            const cb = callbacks[source];
            delete callbacks[source];
            disconnectSource(source);
            if (cb) cb(data["stdout"] || "", data["exit code"]);
        }
        function run(cmd, cb) {
            const src = cmd + " #" + (++n);
            callbacks[src] = cb || null;
            connectSource(src);
        }
    }
    function refresh() {
        exec.run("python3 " + helper + " apps", out => { const a = lastJson(out, null); if (Array.isArray(a)) root.apps = a; });
        exec.run("python3 " + helper + " games", out => {
            const g = lastJson(out, null);
            if (Array.isArray(g)) root.games = g;
            // Then what Hearth lists now (the cached copy shows at once; this can take a few seconds).
            exec.run("python3 " + helper + " games fresh", out2 => { const g2 = lastJson(out2, null); if (Array.isArray(g2) && g2.length) root.games = g2; });
        });
    }
    Component.onCompleted: refresh()
    onExpandedChanged: if (root.expanded) { query = ""; folderOpen = false; refresh(); }

    function matches(a, t) {
        return [a.name, a.generic, a.id].concat(a.keywords || []).some(x => String(x || "").toLowerCase().indexOf(t) >= 0);
    }
    readonly property var mainApps: apps.filter(a => !a.system)
    readonly property var sysApps: apps.filter(a => a.system)
    readonly property var shown: {
        const t = query.trim().toLowerCase();
        if (section === "games")
            return games.filter(g => !t || String(g.title || "").toLowerCase().indexOf(t) >= 0).map(g => ({ kind: "game", game: g, name: g.title, icon: g.icon || g.cover || "" }));
        if (t) return apps.filter(a => matches(a, t)).map(a => ({ kind: "app", app: a, name: a.name, icon: a.icon }));
        if (folderOpen) return sysApps.map(a => ({ kind: "app", app: a, name: a.name, icon: a.icon }));
        const list = mainApps.map(a => ({ kind: "app", app: a, name: a.name, icon: a.icon }));
        if (sysApps.length) list.push({ kind: "folder", name: "System apps", icon: "folder" });
        return list;
    }
    function activate(item) {
        if (!item) return;
        if (item.kind === "folder") { folderOpen = true; return; }
        if (item.kind === "game") exec.run("python3 " + helper + " play " + q(item.game.id));
        else exec.run("python3 " + helper + " launch " + q(item.app.id));
        root.expanded = false;
    }
    function action(cmd) { exec.run(cmd); root.expanded = false; }

    fullRepresentation: Item {
        id: full
        Layout.preferredWidth: Kirigami.Units.gridUnit * 46
        Layout.preferredHeight: Kirigami.Units.gridUnit * 32
        Layout.minimumWidth: Kirigami.Units.gridUnit * 24
        Layout.minimumHeight: Kirigami.Units.gridUnit * 20
        focus: true
        Connections {
            target: root
            function onExpandedChanged() { if (root.expanded) { search.text = ""; search.forceActiveFocus(); } }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.largeSpacing

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.largeSpacing
                PlasmaComponents.TextField {
                    id: search
                    Layout.fillWidth: true
                    placeholderText: root.section === "games" ? "Search your games" : "Search apps"
                    onTextChanged: root.query = text
                    Keys.onDownPressed: { grid.forceActiveFocus(); grid.currentIndex = 0; }
                    Keys.onReturnPressed: root.activate(root.shown[0])
                    Keys.onTabPressed: root.section = root.section === "apps" ? "games" : "apps"
                    Keys.onEscapePressed: root.expanded = false
                }
                PlasmaComponents.TabBar {
                    id: tabs
                    currentIndex: root.section === "games" ? 1 : 0
                    PlasmaComponents.TabButton { text: "Apps"; onClicked: { root.section = "apps"; root.folderOpen = false; } }
                    PlasmaComponents.TabButton { text: "Games"; onClicked: root.section = "games" }
                }
            }

            PlasmaComponents.ToolButton {
                visible: root.folderOpen && root.section === "apps" && root.query === ""
                text: "‹ System apps"
                onClicked: root.folderOpen = false
            }

            PlasmaComponents.ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                GridView {
                    id: grid
                    clip: true
                    model: root.shown
                    readonly property int cols: Math.max(3, Math.floor(width / (Kirigami.Units.gridUnit * 7)))
                    cellWidth: width / cols
                    cellHeight: root.section === "games" ? cellWidth * 1.35 : Kirigami.Units.gridUnit * 6.5
                    keyNavigationEnabled: true
                    highlightFollowsCurrentItem: true
                    highlight: Rectangle { radius: Kirigami.Units.smallSpacing * 3; color: Kirigami.Theme.highlightColor; opacity: 0.25 }
                    Keys.onReturnPressed: root.activate(root.shown[currentIndex])
                    Keys.onEnterPressed: root.activate(root.shown[currentIndex])
                    Keys.onTabPressed: root.section = root.section === "apps" ? "games" : "apps"
                    Keys.onEscapePressed: root.expanded = false
                    Keys.onUpPressed: event => { if (currentIndex < cols) search.forceActiveFocus(); else event.accepted = false; }
                    delegate: MouseArea {
                        id: cell
                        required property var modelData
                        required property int index
                        width: grid.cellWidth
                        height: grid.cellHeight
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton
                        onClicked: root.activate(modelData)
                        onContainsMouseChanged: if (containsMouse) grid.currentIndex = index
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: Kirigami.Units.smallSpacing * 2
                            spacing: Kirigami.Units.smallSpacing
                            Item {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                // A game's cover when Hearth has one; else the app's (or the game's) icon.
                                Image {
                                    id: cover
                                    anchors.fill: parent
                                    visible: cell.modelData.kind === "game" && status === Image.Ready
                                    source: cell.modelData.kind === "game" ? (cell.modelData.game.cover || "") : ""
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                }
                                Kirigami.Icon {
                                    anchors.centerIn: parent
                                    visible: !cover.visible
                                    width: Kirigami.Units.iconSizes.huge
                                    height: width
                                    source: cell.modelData.icon || (cell.modelData.kind === "game" ? "input-gamepad" : "application-x-executable")
                                    fallback: "application-x-executable"
                                }
                            }
                            PlasmaComponents.Label {
                                Layout.fillWidth: true
                                text: cell.modelData.name
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                maximumLineCount: 2
                                wrapMode: Text.Wrap
                            }
                        }
                    }
                    PlasmaExtras.PlaceholderMessage {
                        anchors.centerIn: parent
                        width: parent.width - Kirigami.Units.gridUnit * 4
                        visible: grid.count === 0
                        iconName: root.section === "games" ? "input-gamepad" : "search"
                        text: root.section === "games" ? (root.query ? "No games match" : "No installed games yet") : "No apps match"
                        explanation: root.section === "games" && !root.query ? "Games you install in Hearth show up here." : ""
                    }
                }
            }

            // Like Mantel's launcher: settings, Game Mode and power along the bottom.
            RowLayout {
                Layout.fillWidth: true
                PlasmaComponents.ToolButton { icon.name: "hearth-settings"; text: "Settings"; onClicked: root.action("hearth-settings") }
                PlasmaComponents.ToolButton { icon.name: "input-gamepad"; text: "Game Mode"; onClicked: root.action("nimbus-mode game") }
                Item { Layout.fillWidth: true }
                PlasmaComponents.ToolButton { icon.name: "system-lock-screen"; text: "Lock"; onClicked: root.action("loginctl lock-session") }
                PlasmaComponents.ToolButton { icon.name: "system-suspend"; text: "Sleep"; onClicked: root.action("systemctl suspend") }
                PlasmaComponents.ToolButton {
                    icon.name: "system-reboot"; text: "Restart"
                    onClicked: root.action("qdbus6 org.kde.Shutdown /Shutdown org.kde.Shutdown.logoutAndReboot || systemctl reboot")
                }
                PlasmaComponents.ToolButton {
                    icon.name: "system-shutdown"; text: "Shut down"
                    onClicked: root.action("qdbus6 org.kde.Shutdown /Shutdown org.kde.Shutdown.logoutAndShutdown || systemctl poweroff")
                }
            }
        }
    }
}
