// Firehold's Desk (mockup 2): the accent tile at the top left and the focused app's name (Plasma's own task model,
// as its task manager has it). The tile goes to the Couch's Home; KDE's global menu widget next to it has the app's
// menus (File, Edit, View).
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasma5support as P5Support
import org.kde.taskmanager as TaskManager

PlasmoidItem {
    id: root
    preferredRepresentation: fullRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground

    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        filterByVirtualDesktop: false
        filterByScreen: false
        filterByActivity: false
    }
    readonly property var active: tasks.activeTask
    readonly property string appName: active && active.valid ? String(tasks.data(active, TaskManager.AbstractTasksModel.AppName) || "") : ""
    readonly property int pid: active && active.valid ? Number(tasks.data(active, TaskManager.AbstractTasksModel.AppPid) || 0) : 0
    P5Support.DataSource {
        id: sh
        engine: "executable"
        property var cbs: ({})
        property int n: 0
        onNewData: function (source, data) { const cb = sh.cbs[source]; delete sh.cbs[source]; sh.disconnectSource(source); if (cb) cb(String(data["stdout"] || "")); }
        function run(cmd, cb) { sh.n += 1; cmd += " #" + sh.n; sh.cbs[cmd] = cb || null; sh.connectSource(cmd); }
    }
    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    function helper(args, cb) { sh.run("hearth-firehold-plasma " + args.map(q).join(" "), cb); }
    // The app's own menus (KDE's global menu widget next to this one) or, for an app with none, Hearth's standard ones.
    Loader { id: probe; source: "AppMenuProbe.qml" }
    readonly property bool ownMenus: probe.status === Loader.Ready && probe.item.available
    property var screens: []
    function readScreens() { helper(["modes"], function (t) { try { root.screens = JSON.parse(t.trim().split("\n").pop()).screens || []; } catch (e) {} }); }
    function modeName(m) { return m === "couch" ? "Couch" : m === "away" ? "Away" : "Desk"; }
    readonly property var appItems: [[(appName || "This app").toUpperCase() + " · STANDARD FOR EVERY APP", "", "", "head"],
        ["About " + (appName || "this app"), "", "about"], ["Settings for this app", "", "settings"], ["Open its folder", "", "folder"],
        ["Send logs", "", "logs"], ["Quit", "Ctrl+Q", "close"]]
    readonly property var windowItems: [[(appName || "This app").toUpperCase() + " HAS NO MENUS OF ITS OWN · THESE ARE HEARTH'S", "", "", "head"],
        ["Full screen", "F11", "fullscreen"]].concat(screens.map(function (s) {
            return ["Move to screen › " + (s.label || s.name) + " · " + root.modeName(s.mode), "Super+Shift+← →", "move:" + s.name];
        })).concat([["Keep above others", "", "above"], ["Snap left", "Super+←", "snap-left"], ["Snap right", "Super+→", "snap-right"],
        ["Maximize", "Super+↑", "maximize"], ["Minimize", "Super+↓", "minimize"], ["", "", "", "sep"],
        ["Take a screenshot of it", "Alt+Print", "screenshot"], ["Record it", "Super+Alt+R", "record"], ["", "", "", "sep"],
        ["Close", "Alt+F4", "close"], ["Force quit (not answering)", "", "kill", "red"]])
    readonly property var helpItems: [["Search every command", "Alt+Space", "search"], ["Keyboard shortcuts", "", "shortcuts"]]
    function act(a) {
        if (a === "kill" || a === "folder") helper(["do", "window", a + ":" + root.pid]);
        else if (a === "logs") helper(["do", "window", "logs:" + root.appName]);
        else if (a === "about" || a === "settings") helper(["do", "settings-page", "apps"]);
        else helper(["do", "window", a]);
    }
    property string open: ""
    Loader {
        active: root.open !== ""
        source: "StdMenu.qml"
        onLoaded: {
            item.items = root.open === "app" ? root.appItems : root.open === "window" ? root.windowItems : root.helpItems;
            item.anchorItem = root.menuAnchor;
            item.picked.connect(root.act);
            item.visibleChanged.connect(function () { if (!item.visible) root.open = ""; });
            item.visible = true;
        }
    }
    property Item menuAnchor: null
    property Item menuRow: null
    // hearth-firehold-plasma ping menu-window (the checks' pictures): a standard menu opens as if pressed.
    readonly property string ping: Plasmoid.configuration.modes || ""
    onPingChanged: {
        const w = ping.split("|")[0];
        if (w === "menu-off") open = "";
        else if (w.indexOf("menu-") === 0 && menuRow) {
            const which = w.slice(5);
            for (let i = 0; i < menuRow.children.length; i++) if (menuRow.children[i].modelData && menuRow.children[i].modelData[0] === which) menuAnchor = menuRow.children[i];
            if (which === "window") readScreens();
            open = which;
        }
    }

    fullRepresentation: Item {
        Layout.minimumWidth: row.implicitWidth + 8
        Layout.preferredWidth: row.implicitWidth + 8
        Layout.fillHeight: true
        Row {
            id: row
            Component.onCompleted: root.menuRow = row
            anchors.verticalCenter: parent.verticalCenter
            x: 4
            spacing: 14
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 30; height: 30; radius: 7
                color: "#ff8a3d"
                MouseArea { anchors.fill: parent; onClicked: sh.connectSource("hearth-firehold-plasma switch couch home") }
            }
            Text {
                id: nameText
                anchors.verticalCenter: parent.verticalCenter
                text: root.appName || "Firehold"
                color: Kirigami.Theme.textColor; font.pixelSize: 15; font.weight: Font.Bold
                MouseArea { anchors.fill: parent; onClicked: { root.menuAnchor = nameText; root.open = "app"; } }
            }
            // Hearth's standard menus, only for an app with none of its own (mockup 12).
            Repeater {
                model: root.ownMenus || !root.appName ? [] : [["app", root.appName], ["window", "Window"], ["help", "Help"]]
                delegate: Rectangle {
                    id: mb
                    required property var modelData
                    anchors.verticalCenter: parent.verticalCenter
                    width: mt.implicitWidth + 24; height: 30; radius: 8
                    color: root.open === modelData[0] ? "#2eff7a2f" : "transparent"
                    Text { id: mt; anchors.centerIn: parent; text: modelData[1]; color: "#d8d1ca"; font.pixelSize: 15 }
                    MouseArea { anchors.fill: parent; onClicked: { if (modelData[0] === "window") root.readScreens(); root.menuAnchor = mb; root.open = modelData[0]; } }
                }
            }
        }
    }
}
