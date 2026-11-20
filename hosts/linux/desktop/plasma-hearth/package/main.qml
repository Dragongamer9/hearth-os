// Hearth's panel on KDE Plasma (plasma-hearth; docs/mockups/checklists/plasma-hearth-bar.md): Mantel's own bar
// (Islands.qml), dock (Dock.qml) and app menus (Launcher.qml and its four other styles) inside Plasma's panels, from
// Mantel's own files (install.sh takes out only the Quickshell lines; shim/ does what Quickshell did, with Plasma's
// pieces). One package, three roles (its "role" setting): "bar" (the top bar), "dock", and "menu" (the Hearth flame
// in any panel: org.hearthos.appmenu, Settings › Bar and dock › App menu). The panel's own background is hidden, so
// only Mantel's islands show, and the bar covers Plasma's whole panel window as Mantel's PanelWindow did.
import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore

PlasmoidItem {
    id: main
    readonly property string role: {
        const r = String(Plasmoid.configuration.role || "");
        if (Plasmoid.pluginName === "org.hearthos.appmenu") return "menu";
        return r === "dock" || r === "menu" ? r : "bar";
    }
    preferredRepresentation: fullRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    Plasmoid.constraintHints: role === "bar" ? Plasmoid.CanFillArea : Plasmoid.NoHint
    Plasmoid.status: menus.launcherShown ? PlasmaCore.Types.RequiresAttentionStatus : PlasmaCore.Types.ActiveStatus

    // A Hearth file changed (hearth-plasma-panel sync writes "stamp"): Mantel's files read theirs again.
    readonly property string stamp: String(Plasmoid.configuration.stamp || "")
    onStampChanged: if (stamp !== "") Hearth.stamp = stamp

    // hearth-plasma-panel open WHAT (a shortcut of your own, or CI): "launcher:time", "quick:time"...
    readonly property string openAsk: String(Plasmoid.configuration.open || "")
    property string openSeen: ""
    onOpenAskChanged: {
        if (openAsk === "" || openAsk === openSeen || role !== "bar") return;
        openSeen = openAsk;
        const w = openAsk.split(":")[0];
        if (w === "launcher") menus.toggleLauncher();
        else if (w === "search") menus.openMenuSearch();
        else if (w === "quick") menus.toggleQuick();
        else if (w === "calendar") menus.toggleCalendar();
    }
    Component.onCompleted: openSeen = openAsk

    // Super (Plasma's launcher key, as Kickoff's): the app menu.
    Connections { target: Plasmoid; function onActivated() { menus.toggleLauncher(); } }

    Menus { id: menus; host: main; anchorItem: main.fullRepresentationItem }

    fullRepresentation: Item {
        id: rep
        readonly property bool vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
        Layout.fillWidth: main.role !== "menu"
        Layout.fillHeight: main.role !== "menu"
        Layout.minimumWidth: main.role === "menu" ? Math.max(24, height) : 0
        Layout.preferredWidth: Layout.minimumWidth
        Layout.minimumHeight: main.role === "menu" && vertical ? width : 0

        // The whole panel window, as Mantel's PanelWindow (its islands, gaps and menus are worked out from it).
        Item {
            id: cover
            visible: main.role !== "menu"
            property point o: Qt.point(0, 0)
            x: -o.x
            y: -o.y
            width: rep.Window.width
            height: rep.Window.height
            Loader {
                id: barL
                anchors.fill: parent
                active: main.role === "bar"
                source: "Islands.qml"
                onLoaded: {
                    item.primary = true;
                    item.edge = Qt.binding(() => Theme.barEdge);
                    item.holdsApps = Qt.binding(() => !Theme.dockFloats);
                    item.menu = menus.popProxy(() => 0);
                    item.screen = Qt.binding(() => ({ width: rep.Screen.width, height: rep.Screen.height, name: rep.Screen.name }));
                    item.quickOpen = Qt.binding(() => menus.quickShown);
                    item.calendarOpen = Qt.binding(() => menus.calendarShown);
                    item.menuQuery = Qt.binding(() => menus.menuQuery);
                    item.launcherRequested.connect(() => menus.toggleLauncher());
                    item.quickRequested.connect(() => menus.toggleQuick());
                    item.calendarRequested.connect(() => menus.toggleCalendar());
                    item.settingsRequested.connect(p => SettingsNav.openPage(p));
                    item.keyboardRequested.connect(() => Hearth.run(["python3", Hearth.helper, "osk"]));
                    item.menuSearchRequested.connect(() => menus.openMenuSearch());
                }
                onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: Mantel's bar (Islands.qml) couldn't load")
            }
            Loader {
                id: dockL
                anchors.fill: parent
                active: main.role === "dock"
                source: "Dock.qml"
                onLoaded: {
                    item.primary = true;
                    // The dock's panel goes across the bottom (as Mantel's dock window does): its menus are placed
                    // on the screen from there.
                    item.menu = menus.popProxy(() => 0);
                    item.screen = Qt.binding(() => ({ width: rep.Screen.width, height: rep.Screen.height, name: rep.Screen.name }));
                    item.launcherRequested.connect(() => menus.toggleLauncher());
                }
                onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: Mantel's dock (Dock.qml) couldn't load")
            }
        }
        // "menu": the Hearth flame, as the bar's own app menu button.
        Item {
            anchors.fill: parent
            visible: main.role === "menu"
            Rectangle {
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height) - 4; height: width
                radius: Theme.r(10)
                color: flameArea.containsMouse || menus.launcherShown ? Theme.panel3 : "transparent"
            }
            Image {
                anchors.centerIn: parent
                width: Math.round(Math.min(parent.width, parent.height) * 0.62); height: width
                source: Hearth.shellUrl("images/hearth-logo.png"); smooth: true; mipmap: true
            }
            MouseArea { id: flameArea; anchors.fill: parent; hoverEnabled: true; onClicked: menus.toggleLauncher() }
        }

        // Where this panel's window starts, against this item (Plasma's panel puts its own margins round its
        // widgets): worked out again when the panel lays it out, not on a timer.
        function place() {
            const p = rep.mapToItem(null, 0, 0);
            cover.o = Qt.point(Math.round(p.x), Math.round(p.y));
            if (main.role !== "menu") hideBackground();
            Hearth.screen = { width: rep.Screen.width, height: rep.Screen.height, name: rep.Screen.name };
        }
        // Plasma's own panel background (and its shadow edge) off: Mantel's islands draw the bar.
        function hideBackground() {
            let top = rep;
            while (top.parent) top = top.parent;
            const walk = (it, depth) => {
                if (!it || depth > 7) return;
                if (it !== cover && it.imagePath !== undefined && /panel-background/.test(String(it.imagePath || ""))) it.opacity = 0;
                const kids = it.children || [];
                for (let i = 0; i < kids.length; i++) if (kids[i] !== cover) walk(kids[i], depth + 1);
            };
            walk(top, 0);
        }
        onXChanged: placer.restart()
        onYChanged: placer.restart()
        onWidthChanged: placer.restart()
        onHeightChanged: placer.restart()
        Connections { target: rep.Window.window; ignoreUnknownSignals: true; function onWidthChanged() { placer.restart(); } function onHeightChanged() { placer.restart(); } }
        Timer { id: placer; interval: 40; onTriggered: rep.place() }
        Component.onCompleted: placer.restart()
    }
}
