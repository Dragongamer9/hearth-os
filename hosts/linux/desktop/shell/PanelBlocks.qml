// What a panel can hold (ExtraPanels.qml, and a panel inside a bar or a panel: NestedPanel.qml): the app menu, your
// apps, a window list, the pager, a clock, the tray, volume / network / battery, notifications, the keyboard layout,
// Game Mode, the bar widgets, desktop layouts' blocks, your own blocks and a panel inside (drag and drop board 2).
// Each panel has one; compFor(id) gives the block's component (a copy, "<id>#<key>", is drawn as <id>). Every block
// is set its `panel` (size, vertical, p.edge, screen, width, height) and its `kind` by the Loader that shows it.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower
import "widget-tools.js" as T

Item {
    id: root
    visible: false
    width: 0; height: 0
    function ipc(target, fn) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", target, fn]); }
    function compFor(id) {
        if (typeof id !== "string") return cSpacer;
        if (id.indexOf("panel:") === 0) return cNested;
        const b = T.baseId(id);
        return b === "spacer" ? cSpacer : b === "menu" ? cMenu : b === "apps" ? cApps
            : b === "windows" ? cWindows : b === "clock" ? cClock : b === "tray" ? cTray
            : b === "status" ? cStatus : b === "notifications" ? cNotes : b === "layout" ? cLayout
            : b === "gamemode" ? cGame : blocks.indexOf(b) >= 0 ? cBlock : b.indexOf("own:") === 0 ? cOwn : cWidget;
    }
    // a panel inside this one (NestedPanel.qml, by file name: a problem there only loses the panel inside)
    Component {
        id: cNested
        Loader {
            property var panel: null
            property string kind: ""
            readonly property bool wanted: status === Loader.Ready && !!item && item.wanted
            source: "NestedPanel.qml"
            onLoaded: {
                item.pid = Qt.binding(() => kind.indexOf("panel:") === 0 ? kind.slice(6) : "");
                item.hostSize = Qt.binding(() => panel ? panel.size : 34);
                item.hostVertical = Qt.binding(() => !!panel && panel.vertical);
                item.hostEdge = Qt.binding(() => panel && panel.p ? panel.p.edge : "bottom");
                item.hostScreen = Qt.binding(() => panel ? panel.screen : null);
                item.depth = Qt.binding(() => panel && panel.depth !== undefined ? panel.depth + 1 : 1);
            }
            onStatusChanged: if (status === Loader.Error) console.warn("panels: a panel inside couldn't load (NestedPanel.qml)")
        }
    }
    // Desktop layouts' own blocks: LayoutBlock.qml, the same as in the main bar
    // a block you made in the desktop editor (OwnBarBlock.qml)
    Component { id: cOwn; OwnBarBlock { property var panel: null; side: panel ? panel.size - 10 : 34; vert: panel ? panel.vertical : false } }
    readonly property var blocks: ["overview", "taskview", "appname", "appmenus", "quickaccess", "showdesktop", "power"]
    Component {
        id: cBlock
        LayoutBlock {
            property var panel: null
            side: panel ? panel.size - 10 : 34
            vertical: !!panel && panel.vertical
            edge: panel ? panel.p.edge : "bottom"
        }
    }
    Component { id: cSpacer; Item { implicitWidth: 8; implicitHeight: 8 } }
    Component {
        id: cMenu
        PBtn {
            tip: "Apps"
            onClicked: root.ipc("launcher", "toggle")
            Image { anchors.centerIn: parent; width: parent.side * 0.62; height: width; source: Qt.resolvedUrl("images/hearth-logo.png"); smooth: true; mipmap: true }
        }
    }
    Component {
        id: cApps
        Loader {
            property var panel: null
            source: "TaskStrip.qml"
            onLoaded: {
                item.screen = Qt.binding(() => panel ? panel.screen : null);
                item.vertical = Qt.binding(() => !!panel && panel.vertical);
                item.edge = Qt.binding(() => panel ? panel.p.edge : "bottom");
                item.side = Qt.binding(() => panel ? panel.size - 8 : 36);
                item.barW = Qt.binding(() => panel ? panel.width : 0);
                item.barH = Qt.binding(() => panel ? panel.height : 0);
                item.primary = false;
            }
        }
    }
    Component {
        id: cWindows
        Row {   // every window: its icon (lit when in front); click to bring it forward or minimize it
            property var panel: null
            spacing: 2
            Repeater {
                model: (WinList.toplevels ? WinList.toplevels.values : []).filter(t => !/quickshell/i.test(t.appId || "") && (t.title || t.appId)).slice(0, 14)
                delegate: PBtn {
                    id: wb
                    required property var modelData
                    tip: modelData.title || modelData.appId || ""
                    lit: modelData.activated
                    onClicked: { if (modelData.activated && !modelData.minimized) { if (modelData.setMinimized) modelData.setMinimized(true); else modelData.minimized = true; } else modelData.activate(); }
                    AppIcon { anchors.centerIn: parent; size: wb.side * 0.62; appId: wb.modelData.appId || ""; icon: "" }
                }
            }
        }
    }
    Component {
        id: cClock
        Item {
            property var panel: null
            readonly property bool vert: !!panel && panel.vertical
            implicitWidth: ct.implicitWidth + (vert ? 4 : 16); implicitHeight: ct.implicitHeight + 6
            SystemClock { id: sc; precision: SystemClock.Minutes }
            Column {   // across: 7:12 PM over Tue 17 Nov; down a side: the hours over the minutes
                id: ct
                anchors.centerIn: parent
                Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatTime(sc.date, parent.parent.vert ? (Settings.clock24 ? "HH" : "h") : Settings.clock24 ? "HH:mm" : "h:mm AP"); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                Text { visible: parent.parent.vert; anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatTime(sc.date, "mm"); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                Text { anchors.horizontalCenter: parent.horizontalCenter; text: parent.parent.vert ? Qt.formatDate(sc.date, "d MMM") : Qt.formatDate(sc.date, "ddd d MMM"); color: Theme.muted; font.family: Theme.font; font.pixelSize: parent.parent.vert ? 9 : 10 }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.ipc("calendar", "toggle") }
        }
    }
    Component {
        id: cTray
        Row {
            spacing: 2
            Repeater {
                model: SystemTray.items
                delegate: PBtn {
                    id: tb
                    required property SystemTrayItem modelData
                    tip: modelData.tooltipTitle || modelData.title || modelData.id || ""
                    onClicked: { if (modelData.onlyMenu && modelData.hasMenu) menuNow(); else modelData.activate(); }
                    onRightClicked: { if (modelData.hasMenu) menuNow(); else modelData.secondaryActivate(); }
                    function menuNow() { const w = tb.QsWindow.window; const q = tb.mapToItem(null, 0, tb.height); if (w) modelData.display(w, q.x, q.y); }
                    IconImage { anchors.centerIn: parent; implicitSize: parent.side * 0.55; source: tb.modelData.icon }
                }
            }
        }
    }
    Component {
        id: cStatus
        PBtn {
            id: sbtn
            readonly property var sink: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
            readonly property var batt: UPower.displayDevice
            PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
            implicitWidth: st.implicitWidth + 16
            implicitHeight: Math.max(side, st.implicitHeight + 12)
            tip: (sink ? (sink.muted ? "Muted" : "Volume " + Math.round(sink.volume * 100) + "%") : "") + (batt && batt.isLaptopBattery ? "\nBattery " + Math.round(batt.percentage * 100) + "%" : "")
            onClicked: root.ipc("quick", "toggle")
            onScrolled: d => { if (sink) sink.volume = Math.max(0, Math.min(Buttons.maxVolume, sink.volume + (d > 0 ? 0.05 : -0.05))); }
            Grid {
                id: st
                anchors.centerIn: parent
                columns: sbtn.panel && sbtn.panel.vertical ? 1 : 3
                spacing: 6
                Icon { name: "wifi"; size: 16 }
                Icon { name: !sbtn.sink || sbtn.sink.muted ? "vol-mute" : "vol"; size: 16 }
                Icon { visible: !!sbtn.batt && sbtn.batt.isLaptopBattery; name: "batt"; size: 16 }
            }
        }
    }
    Component {
        id: cNotes
        PBtn {
            tip: NotifyHub.quiet ? "Quiet" : DeskState.unseen + " new"
            badge: NotifyHub.quiet ? 0 : DeskState.unseen
            onClicked: root.ipc("calendar", "toggle")
            Icon { anchors.centerIn: parent; name: NotifyHub.quiet ? "bell-off" : "bell"; size: 17 }
        }
    }
    Component {
        id: cLayout
        PBtn {
            tip: NotifyHub.layoutName
            onClicked: NotifyHub.nextLayout()
            Text { anchors.centerIn: parent; text: NotifyHub.layoutShort || "—"; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; font.bold: true }
        }
    }
    Component {
        id: cGame
        Rectangle {   // Game Mode, always one press away in a panel too (down a side: the controller only); hidden while it's off
            property var panel: null
            visible: !Settings.gameModeOff
            readonly property bool vert: !!panel && panel.vertical
            implicitWidth: vert ? 34 : gr.implicitWidth + 24; implicitHeight: vert ? 34 : 30
            radius: Theme.r(15)
            color: ga.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
            Row { id: gr; anchors.centerIn: parent; spacing: 6; Icon { name: "pad"; size: 16; color: Theme.accentInk } Text { visible: !gr.parent.vert; text: "Game Mode"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 12; font.bold: true } }
            MouseArea { id: ga; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Quickshell.execDetached(["nimbus-mode", "game"]) }
        }
    }
    Component {
        id: cWidget
        Loader {   // a bar widget's small form (widget:weather, widget:monitor…), with its own defaults
            property string kind: ""
            readonly property string type: kind.indexOf("widget:") === 0 ? T.baseId(kind).slice(7) : ""
            source: type ? "Widget" + type.charAt(0).toUpperCase() + type.slice(1) + ".qml" : ""
            onLoaded: { item.compact = true; item.height = 28; }
        }
    }

    component PBtn: Item {
        id: b
        property string tip: ""
        property bool lit: false
        property int badge: 0
        property var panel: null
        default property alias content: holder.data
        readonly property real side: 34
        signal clicked()
        signal rightClicked()
        signal scrolled(int delta)
        implicitWidth: side; implicitHeight: side
        Rectangle { anchors.fill: parent; radius: Theme.r(9); color: ma.containsMouse ? Theme.panel3 : b.lit ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) : "transparent" }
        Rectangle { visible: b.lit; anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } width: 14; height: 3; radius: 1.5; color: Theme.accent }
        Item { id: holder; anchors.fill: parent; readonly property real side: b.side }   // what's put in reads side from here
        Rectangle {
            visible: b.badge > 0
            x: b.side / 2 + 2; y: 2; width: Math.max(15, bt.implicitWidth + 7); height: 15; radius: 7.5; color: Theme.accent
            Text { id: bt; anchors.centerIn: parent; text: b.badge > 9 ? "9+" : String(b.badge); color: Theme.accentInk; font.pixelSize: 9; font.bold: true; font.family: Theme.font }
        }
        MouseArea {
            id: ma
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: m => { if (m.button === Qt.RightButton) b.rightClicked(); else b.clicked(); }
            onWheel: w => b.scrolled(w.angleDelta.y)
        }
    }
}
