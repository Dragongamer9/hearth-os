// Photo frame widget: your pictures one after another, fading from one to the next. From Pictures, Hearth Photos'
// favourites, one of its albums, or a folder. Click opens the picture. Settings: where from, how often, fill or fit.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

WidgetBase {
    id: w
    readonly property string source: c("source", "pictures")
    readonly property int every: c("every", 30)
    readonly property bool fill: c("fit", "fill") === "fill"
    property var data: ({ items: [], albums: [] })
    property int at: 0
    property bool flip: false
    implicitWidth: 380; implicitHeight: 270
    paintsOwn: true

    Process {
        id: ls
        command: ["python3", w.helper, "photos", w.source]
        running: w.live
        stdout: StdioCollector { onStreamFinished: { try { w.data = JSON.parse(text.trim().split("\n").pop()); w.at = 0; w.show(); } catch (e) {} } }
    }
    onSourceChanged: ls.running = true
    Timer { interval: 600000; running: w.live; repeat: true; onTriggered: if (!ls.running) ls.running = true }
    Timer { interval: Math.max(5, w.every) * 1000; running: w.live && (w.data.items || []).length > 1; repeat: true; onTriggered: { w.at = (w.at + 1) % w.data.items.length; w.show(); } }
    readonly property string currentPath: (data.items || []).length ? data.items[at % data.items.length] : ""
    function show() {
        const p = currentPath ? "file://" + currentPath : "";
        if (flip) a.source = p; else b.source = p;
        flip = !flip;
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.r(14)
        color: "#000000"
        clip: true
        Image { id: a; anchors.fill: parent; fillMode: w.fill ? Image.PreserveAspectCrop : Image.PreserveAspectFit; asynchronous: true; sourceSize.width: 1200
                opacity: !w.flip && status === Image.Ready ? 1 : 0; Behavior on opacity { NumberAnimation { duration: Settings.reduceMotion ? 0 : 900 } } }
        Image { id: b; anchors.fill: parent; fillMode: w.fill ? Image.PreserveAspectCrop : Image.PreserveAspectFit; asynchronous: true; sourceSize.width: 1200
                opacity: w.flip && status === Image.Ready ? 1 : 0; Behavior on opacity { NumberAnimation { duration: Settings.reduceMotion ? 0 : 900 } } }
        MouseArea { anchors.fill: parent; onDoubleClicked: w.open(w.currentPath); onClicked: if ((w.data.items || []).length > 1) { w.at = (w.at + 1) % w.data.items.length; w.show(); } }
        ColumnLayout {
            anchors.centerIn: parent
            visible: (w.data.items || []).length === 0 && !w.configuring
            Icon { Layout.alignment: Qt.AlignHCenter; name: "image"; size: 40; color: "#bbbbbb" }
            Text { Layout.alignment: Qt.AlignHCenter; text: "No pictures in " + (w.data.label || "Pictures") + " yet"; color: "#dddddd"; font.family: Theme.font; font.pixelSize: 14 }
        }
        Rectangle {
            visible: w.configuring
            anchors.fill: parent
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.94)
            ColumnLayout {
                anchors { fill: parent; margins: 14 }
                spacing: 8
                Text { text: "Show pictures from"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Flow {
                    Layout.fillWidth: true
                    spacing: 6
                    WBtn { text: "Pictures"; on: w.source === "pictures"; onClicked: w.set("source", "pictures") }
                    WBtn { text: "Photos favourites"; on: w.source === "favorites"; onClicked: w.set("source", "favorites") }
                    Repeater {
                        model: w.data.albums || []
                        delegate: WBtn { required property var modelData; text: modelData.name; on: w.source === "album:" + modelData.id; onClicked: w.set("source", "album:" + modelData.id) }
                    }
                }
                WField { Layout.fillWidth: true; placeholder: "Or a folder (~/Pictures/Holidays)"; onAccepted: if (text.trim()) { w.set("source", "folder:" + text.trim().replace(/^~/, Quickshell.env("HOME"))); text = ""; } }
                RowLayout {
                    spacing: 6
                    Text { text: "Next every"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    Repeater {
                        model: [{ l: "10 s", v: 10 }, { l: "30 s", v: 30 }, { l: "1 min", v: 60 }, { l: "5 min", v: 300 }]
                        delegate: WBtn { required property var modelData; text: modelData.l; on: w.every === modelData.v; onClicked: w.set("every", modelData.v) }
                    }
                }
                RowLayout {
                    spacing: 6
                    WBtn { text: "Fill"; on: w.fill; onClicked: w.set("fit", "fill") }
                    WBtn { text: "Fit"; on: !w.fill; onClicked: w.set("fit", "fit") }
                }
                Item { Layout.fillHeight: true }
            }
        }
    }
}
