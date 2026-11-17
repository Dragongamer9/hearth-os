// Web page widget (1.14): a web page on the desktop, drawn fresh every few minutes (or with ↻) by a browser with no
// window, in a profile of its own (hearth-widgets webshot: Firefox, else Chromium or Chrome). Drawn in its own
// process and shown as a picture, so a heavy or broken page can never slow down or take the desktop with it; click it
// to open the page in your browser. Settings: the address and how often it's drawn again.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

WidgetBase {
    id: w
    readonly property string url: c("url", "")
    readonly property int every: c("every", 15)
    implicitWidth: 480
    implicitHeight: 360
    property var shot: ({})
    property bool loading: false
    // The page's size: the widget's, in 40-pixel steps (a resize doesn't ask for a new picture at every step)
    readonly property int pw: Math.round(Math.max(240, width - 16) / 40) * 40
    readonly property int ph: Math.round(Math.max(160, height - 52) / 40) * 40
    function refresh() {
        if (!url || proc.running) return;
        loading = true;
        proc.command = ["python3", w.helper, "webshot", url, String(pw), String(ph)];
        proc.running = true;
    }
    Process {
        id: proc
        stdout: StdioCollector {
            onStreamFinished: {
                w.loading = false;
                try { w.shot = JSON.parse(text.trim().split("\n").pop()); } catch (e) { w.shot = { error: "Couldn't draw the page" }; }
                img.n++;
            }
        }
    }
    onUrlChanged: refresh()
    onLiveChanged: if (live && !shot.image) refresh()
    Timer { interval: Math.max(1, w.every) * 60000; running: w.live && w.url !== ""; repeat: true; onTriggered: w.refresh() }
    Timer { id: settle; interval: 1500; onTriggered: w.refresh() }
    onPwChanged: if (shot.image) settle.restart()
    onPhChanged: if (shot.image) settle.restart()
    Component.onCompleted: refresh()

    ColumnLayout {
        visible: !w.configuring
        anchors { fill: parent; margins: 8 }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 6
            spacing: 6
            Icon { name: "globe"; size: 15; color: Theme.muted }
            Text {
                Layout.fillWidth: true
                text: w.shot.title || w.url.replace(/^https?:\/\//, "") || "Web page"
                elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
            }
            Text { visible: !!w.shot.time && !w.loading; text: w.shot.time ? Qt.formatTime(new Date(w.shot.time * 1000), Settings.clock24 ? "HH:mm" : "h:mm AP") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
            WBtn { icon: "restart"; flat: true; size: 24; enabled: !w.loading && w.url !== ""; onClicked: w.refresh() }
            WBtn { icon: "open"; flat: true; size: 24; enabled: w.url !== ""; onClicked: w.open(w.url) }
            Item { implicitWidth: 64 }   // room for the widget's own handle, gear and ×
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: Theme.r(10)
            color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.6)
            clip: true
            Image {
                id: img
                property int n: 0
                anchors.fill: parent
                visible: !!w.shot.image
                source: w.shot.image ? "file://" + w.shot.image + "?" + n : ""
                fillMode: Image.PreserveAspectCrop
                horizontalAlignment: Image.AlignLeft
                verticalAlignment: Image.AlignTop
                cache: false
                asynchronous: true
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: w.open(w.url) }
            }
            ColumnLayout {
                visible: !w.shot.image
                anchors.centerIn: parent
                width: parent.width - 40
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: !w.url ? "Pick a web page in its settings (the gear)" : w.loading ? "Drawing the page…" : (w.shot.error || "")
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }
                WBtn { Layout.alignment: Qt.AlignHCenter; visible: !w.url; text: "Choose a page"; on: true; onClicked: w.wantsSettings() }
            }
            Rectangle {   // drawing again over the last picture: a thin bar along the top
                visible: w.loading && !!w.shot.image
                width: parent.width * 0.3; height: 3; radius: 2; color: Theme.accent
                SequentialAnimation on x { running: w.loading; loops: Animation.Infinite; NumberAnimation { from: 0; to: img.width * 0.7; duration: 900 } NumberAnimation { from: img.width * 0.7; to: 0; duration: 900 } }
            }
        }
    }

    ColumnLayout {
        visible: w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Web page"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        WField {
            id: addr
            Layout.fillWidth: true
            grab: w.configuring
            placeholder: "https://…"
            text: w.url
            onAccepted: w.save()
        }
        Text { text: "Draw it again every"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: [5, 15, 30, 60]
                delegate: WBtn { required property int modelData; text: modelData < 60 ? modelData + " min" : "1 hour"; on: w.every === modelData; onClicked: w.set("every", modelData) }
            }
        }
        Text { Layout.fillWidth: true; text: "Pages are drawn by a browser in a profile of its own: your browser's logins and history aren't used"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap }
        Item { Layout.fillHeight: true }
        RowLayout {
            WBtn { text: "Save"; on: true; onClicked: w.save() }
            WBtn { text: "Cancel"; onClicked: { addr.text = w.url; w.doneConfiguring(); } }
        }
    }
    function save() {
        let u = addr.text.trim();
        if (u && !/^(https?|file):\/\//i.test(u)) u = "https://" + u;
        if (u !== url) set("url", u);
        doneConfiguring();
    }
}
