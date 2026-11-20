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
    readonly property string rawName: active && active.valid ? String(tasks.data(active, TaskManager.AbstractTasksModel.AppName) || "") : ""
    readonly property string rawTitle: active && active.valid ? String(tasks.data(active, Qt.DisplayRole) || "") : ""
    readonly property string rawId: active && active.valid ? String(tasks.data(active, TaskManager.AbstractTasksModel.AppId) || "") : ""
    // A runtime's name (qml, wine, java, python) says nothing: the window's title does (Trains 2010).
    readonly property string frontName: /^(qml|qml6|wine.*|java|python3?|.*\.exe)$/i.test(rawName) ? rawTitle.split(" — ")[0].split(" - ")[0] : rawName
    // The app stays named while one of Plasma's own popups (a menu from this bar) has the keyboard.
    property string appName: ""
    onFrontNameChanged: if (frontName !== "" && !/plasmashell|org\.kde\.plasmashell/i.test(rawId)) appName = frontName
    property int pid: 0
    readonly property int rawPid: active && active.valid ? Number(tasks.data(active, TaskManager.AbstractTasksModel.AppPid) || 0) : 0
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
    // Whether the app in front has menus of its own (hearth-firehold-plasma menus PID: a Qt Widgets or GTK app with
    // the app menu module), asked when the app in front changes.
    property bool ownMenus: true
    onRawPidChanged: if (rawPid > 0 && !/plasmashell/i.test(rawId)) pid = rawPid
    onPidChanged: if (pid > 0) helper(["menus", String(pid)], function (t) { root.ownMenus = t.trim() !== "standard"; })
    property int listenFails: 0
    function listen() {
        helper(["listen", "bar-app"], function (t) {
            const l = t.trim();
            if (l === "" || l.indexOf("error") === 0) { root.listenFails += 1; if (root.listenFails < 3) root.listen(); return; }
            root.listenFails = 0;
            root.handlePing(l);
            root.listen();
        });
    }
    Component.onCompleted: listen()
    property var screens: []
    function readScreens(then) { helper(["modes"], function (t) { try { root.screens = JSON.parse(t.trim().split("\n").pop()).screens || []; } catch (e) {} if (then) then(); }); }
    function modeName(m) { return m === "couch" ? "Couch" : m === "away" ? "Away" : "Desk"; }
    // The screen the app in front is on (Plasma's task model has its screen's place).
    readonly property var frontGeo: active && active.valid ? tasks.data(active, TaskManager.AbstractTasksModel.ScreenGeometry) : null
    readonly property string frontScreen: {
        if (!frontGeo) return "";
        for (const s of Qt.application.screens) if (s.virtualX === frontGeo.x && s.virtualY === frontGeo.y) return s.name;
        return "";
    }
    readonly property string nameUp: (appName || "This app").toUpperCase()
    // [label, shortcut, action, kind, what it does] (StdMenu.qml)
    readonly property var appItems: [["THE \u201C" + nameUp + "\u201D MENU · STANDARD FOR EVERY APP", "", "", "head"],
        ["About " + (appName || "this app"), "", "about", "", "From its desktop entry"],
        ["Settings for this app", "", "settings", "", "Proton version, controller layout, which screen it opens on"],
        ["Open its folder", "", "folder", "", "Where it's installed"],
        ["Send logs", "", "logs", "", "When it misbehaves"], ["Quit", "Ctrl+Q", "close"]]
    readonly property var windowItems: [[nameUp + " HAS NO MENUS OF ITS OWN · THESE ARE HEARTH'S", "", "", "head"],
        ["Full screen", "F11", "fullscreen"], ["Move to screen ›", "Super+Shift+← →", "", "sub"],
        ["Keep above others", "", "above"], ["Snap left · right", "Super+← →", "snap-left|snap-right", "pair"],
        ["Maximize", "Super+↑", "maximize"], ["Minimize", "Super+↓", "minimize"], ["", "", "", "sep"],
        ["Take a screenshot of it", "Alt+Print", "screenshot"], ["Record it", "Super+Alt+R", "record"], ["", "", "", "sep"],
        ["Close", "Alt+F4", "close"], ["Force quit (not answering)", "", "kill", "red"]]
    // Move to screen ›: each screen with its mode; where the app would go full screen (a Couch screen), where it is.
    readonly property var moveItems: screens.map(function (s) {
        const here = s.name === root.frontScreen;
        return [(s.label || s.name) + " · " + root.modeName(s.mode) + (here ? " ✓" : ""),
                here ? "here now" : s.mode === "couch" ? "goes full screen" : s.mode === "away" ? "waits there" : "in a window", "move:" + s.name,
                "", "", s.tv ? "video-television" : "video-display"];
    })
    readonly property var helpItems: [["HELP", "", "", "head"], ["Search every command", "Alt+Space", "search"],
        ["Keyboard shortcuts", "", "shortcuts", "", "Hearth's and KDE's in one list"]]
    function act(a) {
        if (a === "kill" || a === "folder") helper(["do", "window", a + ":" + root.pid]);
        else if (a === "logs") helper(["do", "window", "logs:" + root.appName]);
        else if (a === "about" || a === "settings") helper(["do", "settings-page", "apps"]);
        else helper(["do", "window", a]);
    }
    property string open: ""
    // The menu is made with its items already in it (its window takes its size from them when it first shows).
    Loader {
        id: menuLoader
        active: root.open !== ""
        onActiveChanged: if (active) setSource("StdMenu.qml", {
            items: root.open === "app" ? root.appItems : root.open === "window" || root.open === "move" ? root.windowItems : root.helpItems,
            subItems: root.moveItems, subOpen: root.open === "move", anchorItem: root.menuAnchor })
        onLoaded: {
            item.picked.connect(root.act);
            item.visibleChanged.connect(function () { if (!item.visible) root.open = ""; });
            item.visible = true;
        }
    }
    property Item menuAnchor: null
    property Item menuRow: null
    // hearth-firehold-plasma ping menu-window (the checks' pictures): a standard menu opens as if pressed.
    readonly property string ping: Plasmoid.configuration.modes || ""
    onPingChanged: handlePing(ping)
    function handlePing(v) {
        const w = v.split("|")[0];
        if (w === "menu-off") open = "";
        else if (w.indexOf("menu-") === 0 && menuRow) {
            const which = w.slice(5);
            const btn = which === "move" ? "window" : which;
            for (let i = 0; i < menuRow.children.length; i++) if (menuRow.children[i].modelData && menuRow.children[i].modelData[0] === btn) menuAnchor = menuRow.children[i];
            // Window's screens read first, so Move to screen has them when it opens.
            if (btn === "window") readScreens(function () { root.open = which; });
            else open = which;
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
                    color: root.open === modelData[0] || (root.open === "move" && modelData[0] === "window") ? "#2eff7a2f" : "transparent"
                    Text { id: mt; anchors.centerIn: parent; text: modelData[1]; color: Kirigami.Theme.textColor; font.pixelSize: 15 }
                    MouseArea { anchors.fill: parent; onClicked: { root.menuAnchor = mb; const w = modelData[0]; if (w === "window") root.readScreens(function () { root.open = w; }); else root.open = w; } }
                }
            }
        }
    }
}
