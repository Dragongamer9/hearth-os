// A block your AI made (build-anything board 5), on your desktop: `hearth-layout run-block ID` is its own small
// program (stopped by itself past 2 s of processor or 256 MB, and after 10 s), run every few seconds while it's on
// screen; what it read is drawn here: the chip ("Live · Anycubic Chiron"), a picture from the same home address, the
// title, a progress bar, its lines. Before it reaches anything it asks, here on the block: Allow (remembered), Just this
// once, No. Off in Settings › Your desktop › Your blocks.
import QtQuick
import Quickshell
import Quickshell.Io

Rectangle {
    id: ob
    property var blk: ({})
    property var st: ({})
    property color ink: "white"
    property string fontName: Theme.font
    readonly property string bid: String(blk.block || "").replace(/^own:/, "")
    property var out: null
    property bool once: false
    property int every: 30
    radius: st.corners !== undefined ? st.corners : 10
    color: st.colour || "#121212"
    border.width: 2; border.color: Qt.rgba(1, 0.48, 0.18, 0.55)
    clip: true

    function look() { runner.command = ScreenLayout.layoutCmd(["run-block", bid].concat(once ? ["--once"] : [])); runner.running = true; }
    Process {
        id: runner
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = { ok: false, error: "It stopped before it answered (too big or too slow)." }; }
                ob.out = r;
                if (r && r.every) ob.every = r.every;
            }
        }
        onExited: (code) => { if (code !== 0 && !ob.out) ob.out = { ok: false, error: "It stopped by itself (too big or too slow)." }; }
    }
    Timer { interval: Math.max(5, ob.every) * 1000; running: ob.visible && !!ob.out && ob.out.ok === true; repeat: true; onTriggered: ob.look() }
    Component.onCompleted: look()

    // what it read
    Image {
        id: pic
        anchors.fill: parent
        visible: !!ob.out && !!ob.out.picture
        source: ob.out && ob.out.picture ? ob.out.picture + (ob.out.picture.indexOf("?") >= 0 ? "&" : "?") + "t=" + Date.now() : ""
        fillMode: Image.PreserveAspectCrop; asynchronous: true; cache: false; opacity: 0.85
    }
    Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0.4; color: "transparent" } GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.8) } } }
    Rectangle {
        visible: !!ob.out && !!ob.out.chip
        x: 14; y: 14; height: 24; width: chipT.implicitWidth + 30; radius: 6; color: Qt.rgba(0, 0, 0, 0.7)
        Rectangle { x: 10; anchors.verticalCenter: parent.verticalCenter; width: 8; height: 8; radius: 4; color: "white" }
        Text { id: chipT; x: 22; anchors.verticalCenter: parent.verticalCenter; text: ob.out ? ob.out.chip || "" : ""; color: "white"; font.family: ob.fontName; font.pixelSize: 13 }
    }
    Column {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 18 }
        spacing: 6
        visible: !!ob.out && ob.out.ok === true
        Text { width: parent.width; text: ob.out ? ob.out.title || ob.out.name || "" : ""; color: ob.ink; font.family: ob.fontName; font.pixelSize: ob.st.size || 20; font.weight: Font.DemiBold; elide: Text.ElideRight }
        Rectangle {
            visible: !!ob.out && ob.out.progress !== null && ob.out.progress !== undefined
            width: parent.width; height: 6; radius: 3; color: Qt.rgba(1, 1, 1, 0.18)
            Rectangle { width: parent.width * Math.max(0, Math.min(1, (ob.out ? ob.out.progress : 0) / 100)); height: parent.height; radius: 3; color: "#ff7a2f" }
        }
        Repeater {
            model: ob.out && ob.out.lines ? ob.out.lines : []
            delegate: Text { required property var modelData; text: modelData; color: Qt.rgba(1, 1, 1, 0.8); font.family: ob.fontName; font.pixelSize: 13; width: parent.width; elide: Text.ElideRight }
        }
    }

    // its ask: Allow, Just this once, No
    Column {
        anchors { fill: parent; margins: 16 }
        spacing: 8
        visible: !!ob.out && !!ob.out.asks && !ob.out.no
        Text { width: parent.width; wrapMode: Text.WordWrap; text: ob.out && ob.out.asks ? ob.out.asks.title : ""; color: "white"; font.family: ob.fontName; font.pixelSize: 15; font.weight: Font.DemiBold }
        Text { width: parent.width; wrapMode: Text.WordWrap; text: ob.out && ob.out.asks ? ob.out.asks.sub : ""; color: Qt.rgba(1, 1, 1, 0.7); font.family: ob.fontName; font.pixelSize: 12 }
        Row {
            spacing: 8
            Repeater {
                model: [{ t: "Allow", a: "always" }, { t: "Just this once", a: "once" }, { t: "No", a: "no" }]
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: lt.implicitWidth + 24; height: 30; radius: 5
                    color: index === 0 ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.08); border.width: index === 0 ? 0 : 1; border.color: Qt.rgba(1, 1, 1, 0.2)
                    Text { id: lt; anchors.centerIn: parent; text: modelData.t; color: index === 0 ? "#1b1006" : "white"; font.family: ob.fontName; font.pixelSize: 13; font.bold: index === 0 }
                    MouseArea {
                        anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (modelData.a === "once") { ob.once = true; ob.look(); ob.once = false; return; }
                            allowP.command = ScreenLayout.layoutCmd(["block-allow", ob.bid, modelData.a]); allowP.running = true;
                        }
                    }
                }
            }
        }
    }
    Process { id: allowP; onExited: ob.look() }
    Text {
        anchors.centerIn: parent; width: parent.width - 30; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
        visible: !!ob.out && ob.out.ok !== true && (!ob.out.asks || ob.out.no)
        text: !ob.out ? "" : ob.out.no ? (ob.out.asks ? "You said No to " + ob.out.asks.value + ". Settings › Your desktop › Your blocks asks again." : "")
              : ob.out.off ? "Off (Settings › Your desktop › Your blocks)" : (ob.out.error || "")
        color: Qt.rgba(1, 1, 1, 0.7); font.family: ob.fontName; font.pixelSize: 13
    }
}
