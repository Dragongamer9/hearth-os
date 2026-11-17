// App tiles widget: big tiles for the apps you open most, on the desktop. Click a tile to open the app. Until you
// pick your own, it shows your taskbar's pinned apps (only shown: nothing is saved until you choose). Settings: tick
// the apps you want (type to find one).
import QtQuick
import QtQuick.Layouts
import Quickshell

WidgetBase {
    id: w
    readonly property var chosen: list("apps")
    readonly property var ids: chosen !== null ? chosen : (DeskState.pins || []).slice(0, 6)
    readonly property var entries: { void DesktopEntries.applications.values.length; return ids.map(id => DesktopEntries.byId(id)).filter(e => !!e); }
    readonly property var allApps: { const a = Array.from(DesktopEntries.applications.values).filter(e => !e.noDisplay); a.sort((x, y) => String(x.name).localeCompare(String(y.name))); return a; }
    implicitWidth: 340; implicitHeight: 210

    GridView {
        id: grid
        visible: !w.configuring
        anchors { fill: parent; margins: 10 }
        interactive: false
        readonly property int cols: Math.max(1, Math.round(width / 104))
        cellWidth: Math.floor(width / cols)
        cellHeight: Math.min(cellWidth, Math.max(90, Math.floor(height / Math.max(1, Math.ceil(w.entries.length / cols)))))
        model: w.entries
        delegate: Item {
            required property var modelData
            width: grid.cellWidth; height: grid.cellHeight
            Rectangle {
                anchors { fill: parent; margins: 4 }
                radius: Theme.r(14)
                color: tm.containsMouse ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25) : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.06)
                border.width: tm.containsMouse ? 1 : 0; border.color: Theme.accent
                AppIcon { anchors { centerIn: parent; verticalCenterOffset: -9 } size: Math.min(parent.width, parent.height) * 0.45; icon: modelData.icon || ""; appId: modelData.id || "" }
                Text { anchors { bottom: parent.bottom; bottomMargin: 6; left: parent.left; right: parent.right; leftMargin: 4; rightMargin: 4 } text: modelData.name; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
            }
            MouseArea { id: tm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { modelData.execute(); DeskState.noteLaunch(modelData.id); } }
        }
    }
    Text { visible: !w.configuring && w.entries.length === 0; anchors.centerIn: parent; text: "Pick apps in this widget's settings"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }

    ColumnLayout {
        visible: w.configuring
        anchors { fill: parent; margins: 12 }
        spacing: 6
        Text { text: "Apps on these tiles"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        WField { id: find; Layout.fillWidth: true; placeholder: "Find an app"; grab: w.configuring }
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: w.configuring ? w.allApps.filter(e => !find.text || String(e.name).toLowerCase().indexOf(find.text.toLowerCase()) >= 0) : []
            delegate: Rectangle {
                required property var modelData
                readonly property bool on: w.ids.indexOf(modelData.id) >= 0
                width: ListView.view.width; height: 32
                radius: Theme.r(6)
                color: am.containsMouse ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) : "transparent"
                RowLayout {
                    anchors { fill: parent; leftMargin: 6; rightMargin: 6 }
                    spacing: 8
                    Rectangle { width: 18; height: 18; radius: 4; color: on ? Theme.accent : "transparent"; border.width: 2; border.color: on ? Theme.accent : Theme.muted
                        Icon { anchors.centerIn: parent; visible: on; name: "check"; size: 12; color: Theme.accentInk } }
                    AppIcon { size: 22; icon: modelData.icon || ""; appId: modelData.id || "" }
                    Text { Layout.fillWidth: true; text: modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                }
                MouseArea { id: am; anchors.fill: parent; hoverEnabled: true; onClicked: w.set("apps", on ? w.ids.filter(x => x !== modelData.id) : w.ids.concat([modelData.id])) }
            }
        }
    }
}
