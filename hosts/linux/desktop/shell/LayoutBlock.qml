// Desktop layouts' own blocks (hosts/hearth-layout blocks.rs; mockups docs/mockups/desktop-layouts): the same in the
// main bar (Islands.qml) and in an extra panel (ExtraPanels.qml), and drawn the same by Mantel in Rust.
//   overview     the desktops pill: one dot per desktop, the one you're on long; click for the overview (GNOME-style)
//   taskview     Task view: the overview (Windows 11-style)
//   appname      the app in front, by name, bold; click for its app menu (macOS-style)
//   appmenus     the app in front's own menus (File, Edit, View…; FhGlobalMenu.qml), or Hearth's Window and Help
//   quickaccess  Quick Access (the panel with Wi-Fi, sound, brightness…)
//   showdesktop  Show desktop: a thin strip at the end (Plasma- and Windows-style); click again brings them back
// Nothing here runs anything by itself: menus are read only while an app is in front with appmenus shown.
import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: blk
    property string kind: ""
    property real side: 34                  // the bar's button size
    property bool vertical: false
    property string edge: "top"
    property var menu: DeskState.popMenu    // PopMenu (shell.qml)
    readonly property bool wanted: kind === "appname" || kind === "appmenus" ? !vertical && front !== null : true
    readonly property var front: WinList.activeToplevel
    readonly property string appName: {
        if (!front) return "";
        const e = AppIcons.entryFor(front.appId);
        const n = e ? String(e.name) : String(front.appId || "");
        return !e || /^(qml|qml6|wine.*|java|python3?|.*\.exe)$/i.test(n) || n === "" ? String(front.title || n).split(" — ")[0].split(" - ")[0] : n;
    }
    implicitWidth: !wanted ? 0 : loader.item ? loader.item.implicitWidth : side
    implicitHeight: !wanted ? 0 : loader.item ? loader.item.implicitHeight : side
    Layout.alignment: Qt.AlignCenter

    function overview() {
        if (WinList.kwin) WinList.effect("overview");
        else Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "overview", "toggle"]);
    }
    function ipc(target, fn) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", target, fn]); }
    // Show desktop, as the dock does it (Dock.qml), for layouts with no dock.
    function showDesktop() {
        if (DeskState.mainBar && DeskState.mainBar.toggleShowDesktop) { DeskState.mainBar.toggleShowDesktop(); return; }
        const open = WinList.toplevels.values.filter(t => !t.minimized);
        if (open.length) {
            DeskState.shownDesktop = { windows: open, front: open.find(t => t.activated) || null };
            open.forEach(t => t.minimized = true);
            return;
        }
        const was = DeskState.shownDesktop;
        DeskState.shownDesktop = null;
        if (!was) return;
        const still = WinList.toplevels.values;
        was.windows.filter(t => still.indexOf(t) >= 0 && t !== was.front).forEach(t => t.minimized = false);
        if (was.front && still.indexOf(was.front) >= 0) was.front.activate();
    }
    function popupFrom(item, title, list) {
        if (!menu) return;
        const p = item.mapToItem(null, 0, 0), win = item.QsWindow ? item.QsWindow.window : null;
        const sh = win && win.screen ? win.screen.height : 1080;
        const below = edge !== "bottom";
        menu.popup(win ? win.screen : null, p.x, below ? p.y + item.height + 6 : sh - (win ? win.height : 0) - 2, { title: title, above: !below }, list);
    }
    // deskmenus.js rows ([label, shortcut, action, kind]) as PopMenu items.
    function itemsOf(rows) {
        return rows.filter(r => r[3] !== "head").map(r => r[3] === "sep" ? { separator: true }
            : { label: r[0].replace(/ ›$/, ""), shortcut: r[1] || "", enabled: r[3] !== "off", danger: r[3] === "red", run: () => blk.act(r[2]) });
    }
    function act(a) {
        const w = front;
        if (!a) return;
        if (a.indexOf("id:") === 0) { gm.click(a.slice(3)); return; }
        if (a.indexOf("|") > 0) a = a.split("|")[0];
        if (a === "close") { if (w) w.close(); }
        else if (a === "fullscreen") { if (w) w.fullscreen = !w.fullscreen; }
        else if (a === "maximize") { if (w) w.maximized = !w.maximized; }
        else if (a === "minimize") { if (w) w.minimized = true; }
        else if (a === "about" || a === "settings") Quickshell.execDetached(["hearth-settings", "apps"]);
        else if (a === "logs") Quickshell.execDetached(["hearth-send-logs", ("Sent from the bar: " + appName).slice(0, 200)]);
        else if (a === "screenshot") ipc("capture", "take");
        else if (a === "search") ipc("searchbar", "key");
        else if (a === "shortcuts") Quickshell.execDetached(["hearth-settings", "keyboard"]);
    }
    // Hearth's standard menus for an app with none of its own (as Firehold's Desk bar: deskmenus.js)
    function appRows() { return [["About " + (appName || "this app"), "", "about"], ["Settings for this app", "", "settings"], ["Send logs", "", "logs"], ["", "", "", "sep"], ["Quit", "Ctrl+Q", "close"]]; }
    function windowRows() { return [["Full screen", "F11", "fullscreen"], ["Maximize", "Super+↑", "maximize"], ["Minimize", "Super+↓", "minimize"], ["", "", "", "sep"], ["Take a screenshot", "Alt+Print", "screenshot"], ["", "", "", "sep"], ["Close", "Alt+F4", "close"]]; }
    function helpRows() { return [["Search every command", "Alt+Space", "search"], ["Keyboard shortcuts", "", "shortcuts"]]; }
    function openMenu(which, item) {
        if (which.indexOf("own:") === 0) {
            gm.read(() => {
                const m = gm.menus.find(x => "own:" + x.id === which);
                if (m) popupFrom(item, m.label, itemsOf(gm.rows(m.items)));
            });
        } else if (which === "app") popupFrom(item, appName, itemsOf(appRows()));
        else if (which === "window") popupFrom(item, "Window", itemsOf(windowRows()));
        else if (which === "help") popupFrom(item, "Help", itemsOf(helpRows()));
    }

    FhGlobalMenu { id: gm; active: blk.kind === "appmenus" && blk.wanted }

    Loader {
        id: loader
        anchors.centerIn: parent
        sourceComponent: blk.kind === "overview" ? cOverview : blk.kind === "taskview" ? cTask : blk.kind === "appname" ? cName
            : blk.kind === "appmenus" ? cMenus : blk.kind === "quickaccess" ? cQuick : blk.kind === "showdesktop" ? cShow
            : blk.kind === "power" ? cPower : null
    }

    component Btn: Item {
        id: b
        property string tip: ""
        property bool lit: false
        default property alias content: holder.data
        signal clicked()
        signal rightClicked()
        implicitWidth: blk.side; implicitHeight: blk.side
        Rectangle { anchors.fill: parent; radius: Theme.r(9); color: ma.containsMouse || b.lit ? Theme.panel3 : "transparent" }
        Item { id: holder; anchors.fill: parent }
        MouseArea {
            id: ma
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: m => { if (m.button === Qt.RightButton) b.rightClicked(); else b.clicked(); }
        }
        ToolTipLite { text: b.tip; shown: ma.containsMouse && b.tip !== "" }
    }
    component ToolTipLite: Rectangle {
        property string text: ""
        property bool shown: false
        visible: shown
        z: 10
        width: tt.implicitWidth + 14; height: tt.implicitHeight + 8
        radius: Theme.r(6)
        color: Theme.panel; border.color: Theme.line
        x: (parent.width - width) / 2
        y: blk.edge === "bottom" ? -height - 6 : parent.height + 6
        Text { id: tt; anchors.centerIn: parent; text: parent.text; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
    }

    // ---- the desktops pill (GNOME-style): the one you're on long
    Loader { id: deskData; active: blk.kind === "overview"; visible: false; source: "DeskSwitcher.qml" }
    Component {
        id: cOverview
        Rectangle {
            readonly property var sets: deskData.item ? deskData.item.sets : []
            readonly property int at: deskData.item ? deskData.item.activeIndex : 0
            readonly property int n: Math.max(1, sets.length)
            implicitWidth: blk.vertical ? Math.round(blk.side * 0.7) : row.implicitWidth + 16
            implicitHeight: blk.vertical ? row.implicitHeight + 16 : Math.round(blk.side * 0.7)
            radius: Theme.r(Math.min(width, height) / 2)
            color: pa.containsMouse ? Theme.panel3 : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.12)
            Grid {
                id: row
                anchors.centerIn: parent
                columns: blk.vertical ? 1 : 0
                rows: blk.vertical ? 0 : 1
                spacing: 5
                Repeater {
                    model: parent.parent.n
                    Rectangle {
                        required property int index
                        readonly property bool on: index === Math.max(0, row.parent.at)
                        width: blk.vertical ? 6 : on ? 22 : 6
                        height: blk.vertical ? (on ? 22 : 6) : 6
                        radius: 3
                        color: on ? Theme.text : Theme.muted
                        Behavior on width { enabled: !Settings.reduceMotion; NumberAnimation { duration: 160 } }
                    }
                }
            }
            MouseArea { id: pa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: blk.overview() }
        }
    }
    Component {
        id: cTask
        Btn { tip: "Task view (Super+Tab)"; onClicked: blk.overview(); Icon { anchors.centerIn: parent; name: "desktops"; size: blk.side * 0.55 } }
    }
    Component {
        id: cName
        Item {
            id: nb
            implicitWidth: nt.implicitWidth + 16; implicitHeight: blk.side
            Rectangle { anchors.fill: parent; radius: Theme.r(8); color: na.containsMouse ? Theme.panel3 : "transparent" }
            Text { id: nt; anchors.centerIn: parent; text: blk.appName; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Bold; elide: Text.ElideRight; width: Math.min(implicitWidth, 240) }
            MouseArea { id: na; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: blk.openMenu("app", nb) }
        }
    }
    Component {
        id: cMenus
        Row {
            spacing: 2
            Repeater {
                model: gm.menus.length ? gm.menus.map(m => ["own:" + m.id, String(m.label).replace(/_/g, "")]) : [["window", "Window"], ["help", "Help"]]
                Item {
                    id: mb
                    required property var modelData
                    implicitWidth: mt.implicitWidth + 18; implicitHeight: blk.side
                    Rectangle { anchors.fill: parent; radius: Theme.r(8); color: ma2.containsMouse ? Theme.panel3 : "transparent" }
                    Text { id: mt; anchors.centerIn: parent; text: mb.modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                    MouseArea { id: ma2; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: blk.openMenu(mb.modelData[0], mb) }
                }
            }
        }
    }
    Component {
        id: cQuick
        Btn { tip: "Quick Access"; onClicked: blk.ipc("quick", "toggle"); Icon { anchors.centerIn: parent; name: "grid"; size: blk.side * 0.5 } }
    }
    Component {
        id: cPower
        Btn { tip: "Power"; onClicked: blk.ipc("leave", "powermenu"); Icon { anchors.centerIn: parent; name: "power"; size: blk.side * 0.5 } }
    }
    Component {
        id: cShow
        Item {   // a thin strip at the end of the bar
            implicitWidth: blk.vertical ? blk.side : 10
            implicitHeight: blk.vertical ? 10 : blk.side
            Rectangle {
                anchors.centerIn: parent
                width: blk.vertical ? parent.width - 8 : 1; height: blk.vertical ? 1 : parent.height - 8
                color: sa.containsMouse ? Theme.accent : Theme.line
            }
            MouseArea { id: sa; anchors.fill: parent; anchors.margins: -2; hoverEnabled: true; onClicked: blk.showDesktop() }
            ToolTipLite { text: "Show desktop"; shown: sa.containsMouse }
        }
    }
}
