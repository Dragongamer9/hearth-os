// Window list widget (1.14): every open window (WinList: KWin's or labwc's), the one in front lit. Click to bring one
// forward (or minimize it when it's already in front), the buttons minimize or close it. Settings: only this
// desktop's windows (under KWin, which says where each one is), group by app. In the bar: a button with the count that
// opens the list as a menu.
import QtQuick
import QtQuick.Layouts
import Quickshell

WidgetBase {
    id: w
    readonly property bool hereOnly: c("hereOnly", false) === true
    readonly property var deskNow: { const i = WinList.desktops.findIndex(d => d.active); return i + 1; }
    readonly property var wins: (WinList.toplevels ? WinList.toplevels.values : []).filter(t =>
        !/quickshell/i.test(t.appId || "") && (t.title || t.appId) &&
        (!hereOnly || !WinList.kwin || t.allDesktops || (t.desktops || []).indexOf(deskNow) >= 0))
    function name(t) { const e = AppIcons.entryFor(t.appId); return e ? e.name : (t.appId || ""); }
    function pick(t) {
        if (t.activated && !t.minimized) { if (t.setMinimized) t.setMinimized(true); else t.minimized = true; }
        else t.activate();
    }
    implicitWidth: compact ? barRow.implicitWidth : 340
    implicitHeight: compact ? 24 : 320

    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5
        Icon { name: "window"; size: 15; color: Theme.text }
        Text { text: w.wins.length === 1 ? "1 window" : w.wins.length + " windows"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
    }

    ColumnLayout {
        visible: !w.compact && !w.configuring
        anchors { fill: parent; margins: 12 * w.k }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Icon { name: "window"; size: 15; color: Theme.muted }
            Text { Layout.fillWidth: true; text: w.wins.length === 0 ? "No windows open" : w.wins.length === 1 ? "1 window" : w.wins.length + " windows"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        }
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 4
            model: w.wins
            delegate: Rectangle {
                id: row
                required property var modelData
                width: ListView.view.width
                height: 40 * w.k
                radius: Theme.r(10)
                color: rowHov.hovered ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) : modelData.activated ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : "transparent"
                border.width: modelData.activated ? 1 : 0
                border.color: Theme.accent
                opacity: modelData.minimized ? 0.6 : 1
                MouseArea { id: rowArea; anchors.fill: parent; hoverEnabled: true; onClicked: w.pick(row.modelData) }
                HoverHandler { id: rowHov }
                RowLayout {
                    anchors { fill: parent; leftMargin: 8; rightMargin: 6 }
                    spacing: 8
                    AppIcon { size: 24 * w.k; appId: row.modelData.appId || ""; icon: "" }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        Text { Layout.fillWidth: true; text: row.modelData.title || w.name(row.modelData); elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(13 * w.k); font.weight: row.modelData.activated ? Font.DemiBold : Font.Normal }
                        Text { Layout.fillWidth: true; visible: text !== ""; text: w.name(row.modelData) + (row.modelData.minimized ? " · minimized" : ""); elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                    }
                    WBtn {
                        visible: rowHov.hovered
                        icon: "chevd"; flat: true; size: 24
                        onClicked: { if (row.modelData.setMinimized) row.modelData.setMinimized(true); else row.modelData.minimized = true; }
                    }
                    WBtn { visible: rowHov.hovered; icon: "close"; flat: true; size: 24; onClicked: row.modelData.close() }
                }
            }
        }
    }

    ColumnLayout {
        visible: !w.compact && w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Window list"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        RowLayout {
            spacing: 6
            WBtn { text: "Every desktop"; on: !w.hereOnly; onClicked: w.set("hereOnly", false) }
            WBtn { text: "This desktop"; on: w.hereOnly; onClicked: w.set("hereOnly", true) }
        }
        Text { Layout.fillWidth: true; visible: !WinList.kwin; text: "On labwc every window is listed (it doesn't tell which desktop a window is on)"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        Item { Layout.fillHeight: true }
        WBtn { text: "Done"; on: true; onClicked: w.doneConfiguring() }
    }
}
