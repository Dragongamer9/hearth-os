// Pager widget (1.14): your virtual desktops side by side, the one you're on lit, each with outlines of its windows
// and their icons (where they are and how big, under KWin: WinList's frames; labwc doesn't tell other programs which
// desktop a window is on, so there it shows the desktops and the windows of the one you're on). Click a desktop to go
// there, scroll to step through them. Settings: names or numbers. In the bar: small pills, like the desktops button.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.WindowManager

WidgetBase {
    id: w
    readonly property bool names: c("names", false) === true
    readonly property var sets: {
        if (WinList.wmDesktops) return WinList.desktops.map(d => ({ name: d.name || "", active: d.active === true, set: null }));
        const all = Array.from(WindowManager.windowsets || []).filter(x => x && x.shouldDisplay !== false);
        all.sort((a, b) => String(a.name || "").localeCompare(String(b.name || ""), undefined, { numeric: true }));
        return all.map(x => ({ name: x.name || "", active: x.active, set: x }));
    }
    readonly property var shown: sets.length ? sets : [{ name: "Desktop 1", active: true, set: null }]
    function go(i) {
        if (WinList.wmDesktops) { WinList.goDesktop(i + 1); return; }
        const s = sets[i];
        if (s && s.set && s.set.canActivate !== false) s.set.activate();
    }
    readonly property var wins: (WinList.toplevels ? WinList.toplevels.values : []).filter(t => !t.minimized && !/quickshell/i.test(t.appId || ""))
    function winsOn(i, active) {
        if (WinList.wmDesktops) return wins.filter(t => t.allDesktops || (t.desktops || []).indexOf(i + 1) >= 0);
        return active ? wins : [];
    }
    readonly property real sw: Quickshell.screens.length ? Quickshell.screens[0].width : 1920
    readonly property real sh: Quickshell.screens.length ? Quickshell.screens[0].height : 1080
    implicitWidth: compact ? barRow.implicitWidth : 420
    implicitHeight: compact ? 24 : 170

    Row {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 3
        Repeater {
            model: w.shown
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: modelData.active ? 30 : 20; height: 20; radius: Theme.r(10)
                color: modelData.active ? Theme.accent : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1)
                Text { anchors.centerIn: parent; text: parent.index + 1; color: parent.modelData.active ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.bold: parent.modelData.active }
                MouseArea { anchors.fill: parent; onClicked: w.go(parent.index) }
            }
        }
    }

    ColumnLayout {
        visible: !w.compact && !w.configuring
        anchors { fill: parent; margins: 12 * w.k }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Icon { name: "desktops"; size: 15; color: Theme.muted }
            Text { Layout.fillWidth: true; text: w.shown.length === 1 ? "1 desktop" : w.shown.length + " desktops"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        }
        Item {
            id: strip
            Layout.fillWidth: true
            Layout.fillHeight: true
            readonly property int n: w.shown.length
            readonly property real gap: 8
            // Each desktop as big as fits, the screen's shape
            readonly property real cellW: Math.min((width - gap * (n - 1)) / n, height * w.sw / w.sh)
            readonly property real cellH: cellW * w.sh / w.sw
            WheelHandler { onWheel: ev => { const i = w.shown.findIndex(s => s.active); w.go(Math.max(0, Math.min(w.shown.length - 1, i + (ev.angleDelta.y < 0 ? 1 : -1)))); } }
            Row {
                anchors.centerIn: parent
                spacing: strip.gap
                Repeater {
                    model: w.shown
                    delegate: Rectangle {
                        id: desk
                        required property var modelData
                        required property int index
                        width: strip.cellW; height: strip.cellH
                        radius: Theme.r(8)
                        color: modelData.active ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.45)
                        border.width: modelData.active ? 2 : 1
                        border.color: modelData.active ? Theme.accent : Theme.line
                        clip: true
                        readonly property real kx: width / w.sw
                        readonly property real ky: height / w.sh
                        Repeater {
                            model: w.winsOn(desk.index, desk.modelData.active)
                            delegate: Rectangle {
                                required property var modelData
                                required property int index
                                readonly property var f: modelData.frame || ({})
                                readonly property bool known: f.w > 0 && f.h > 0
                                x: known ? f.x * desk.kx : 6 + (index % 4) * 18
                                y: known ? f.y * desk.ky : desk.height - 22 - Math.floor(index / 4) * 18
                                width: known ? Math.max(8, f.w * desk.kx) : 16
                                height: known ? Math.max(6, f.h * desk.ky) : 16
                                radius: 3
                                color: modelData.activated ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.22) : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1)
                                border.width: 1; border.color: modelData.activated ? Theme.accent : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.35)
                                AppIcon { anchors.centerIn: parent; size: Math.min(18, parent.width - 2, parent.height - 2); visible: size >= 8; appId: parent.modelData.appId || ""; icon: "" }
                            }
                        }
                        Text {
                            anchors { left: parent.left; bottom: parent.bottom; margins: 5 }
                            text: w.names && desk.modelData.name ? desk.modelData.name : String(desk.index + 1)
                            color: desk.modelData.active ? Theme.accent : Theme.muted
                            font.family: Theme.font; font.pixelSize: 11; font.bold: desk.modelData.active
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: w.go(desk.index) }
                    }
                }
            }
        }
    }

    ColumnLayout {
        visible: !w.compact && w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Pager"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        RowLayout {
            spacing: 6
            WBtn { text: "Numbers"; on: !w.names; onClicked: w.set("names", false) }
            WBtn { text: "Names"; on: w.names; onClicked: w.set("names", true) }
        }
        Text { Layout.fillWidth: true; text: "How many desktops there are: Settings › Your desktop"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        Item { Layout.fillHeight: true }
        WBtn { text: "Done"; on: true; onClicked: w.doneConfiguring() }
    }
}
