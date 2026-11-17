// Folder view widget: a folder's files right on the desktop (as KDE Plasma's folder view), as icons or a list.
// Double-click (or click in the list) opens with your usual app; folders open in Files. Looked at again every
// 10 s while on screen. Settings: which folder (Downloads by default) and icons or list.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

WidgetBase {
    id: w
    readonly property string home: Quickshell.env("HOME") || "/"
    readonly property string folder: c("path", home + "/Downloads")
    readonly property string view: c("view", "icons")
    property var data: ({ items: [] })
    implicitWidth: 400; implicitHeight: 320

    Process {
        id: ls
        command: ["python3", w.helper, "folder", w.folder]
        running: w.live
        stdout: StdioCollector { onStreamFinished: { try { w.data = JSON.parse(text.trim().split("\n").pop()); } catch (e) {} } }
    }
    onFolderChanged: ls.running = true
    Timer { interval: 10000; running: w.live; repeat: true; onTriggered: if (!ls.running) ls.running = true }
    function iconFor(k) { return k === "folder" ? "folder" : k === "image" ? "image" : k === "video" ? "film" : k === "audio" ? "headphones" : k === "doc" ? "doc" : "file"; }

    ColumnLayout {
        visible: !w.configuring
        anchors { fill: parent; margins: 12 }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Icon { name: "folder"; size: 18; color: Theme.accent }
            Text { Layout.fillWidth: true; text: w.data.name || w.folder.split("/").pop(); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideMiddle }
            Text { text: (w.data.items || []).length + ((w.data.more || 0) > 0 ? "+" : "") + " items"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            WBtn { icon: "open"; flat: true; size: 24; onClicked: w.open(w.folder) }
        }
        Text { visible: !!w.data.error; text: w.data.error || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        Text { visible: !w.data.error && (w.data.items || []).length === 0; text: "This folder is empty"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        GridView {
            visible: w.view === "icons"
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: Math.floor(width / Math.max(1, Math.floor(width / 92))); cellHeight: 92
            model: w.view === "icons" ? (w.data.items || []) : []
            delegate: Item {
                required property var modelData
                width: GridView.view.cellWidth; height: 92
                Rectangle { anchors { fill: parent; margins: 2 } radius: Theme.r(8); color: im.containsMouse ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) : "transparent" }
                Item {
                    id: pic
                    anchors { top: parent.top; topMargin: 6; horizontalCenter: parent.horizontalCenter }
                    width: 48; height: 48
                    Icon { anchors.centerIn: parent; visible: thumb.status !== Image.Ready; name: w.iconFor(modelData.kind); size: 40; color: modelData.kind === "folder" ? Theme.accent : Theme.text }
                    Image { id: thumb; anchors.fill: parent; visible: status === Image.Ready; source: modelData.kind === "image" ? "file://" + modelData.path : ""; sourceSize: Qt.size(96, 96); fillMode: Image.PreserveAspectCrop; asynchronous: true }
                }
                Text {
                    anchors { top: pic.bottom; topMargin: 4; left: parent.left; right: parent.right; leftMargin: 4; rightMargin: 4 }
                    text: modelData.name; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideMiddle; maximumLineCount: 2; wrapMode: Text.WrapAnywhere
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 12
                    style: Text.Raised; styleColor: "#55000000"
                }
                MouseArea { id: im; anchors.fill: parent; hoverEnabled: true; onDoubleClicked: w.open(modelData.path) }
            }
        }
        ListView {
            visible: w.view === "list"
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: w.view === "list" ? (w.data.items || []) : []
            delegate: Rectangle {
                required property var modelData
                width: ListView.view.width; height: 30
                radius: Theme.r(6)
                color: lm.containsMouse ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) : "transparent"
                RowLayout {
                    anchors { fill: parent; leftMargin: 6; rightMargin: 6 }
                    spacing: 8
                    Icon { name: w.iconFor(modelData.kind); size: 18; color: modelData.kind === "folder" ? Theme.accent : Theme.text }
                    Text { Layout.fillWidth: true; text: modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideMiddle }
                    Text { text: Qt.formatDate(new Date(modelData.mtime * 1000), "d MMM"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                }
                MouseArea { id: lm; anchors.fill: parent; hoverEnabled: true; onClicked: w.open(modelData.path) }
            }
        }
    }
    ColumnLayout {
        visible: w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Folder"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: ["Desktop", "Downloads", "Documents", "Pictures", "Videos", "Music"]
                delegate: WBtn { required property var modelData; text: modelData; on: w.folder === w.home + "/" + modelData; onClicked: w.set("path", w.home + "/" + modelData) }
            }
        }
        WField { id: pf; Layout.fillWidth: true; placeholder: "Or type a folder: " + w.folder; onAccepted: if (text.trim()) { w.set("path", text.trim().replace(/^~/, w.home)); text = ""; } }
        RowLayout {
            spacing: 6
            WBtn { text: "Icons"; on: w.view === "icons"; onClicked: w.set("view", "icons") }
            WBtn { text: "List"; on: w.view === "list"; onClicked: w.set("view", "list") }
        }
        Item { Layout.fillHeight: true }
    }
}
