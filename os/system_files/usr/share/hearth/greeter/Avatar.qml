// An account's picture in a circle (AccountsService's, set in Settings › Accounts), or its first letter. Drawn on a
// Canvas so it's round with any renderer (software too).
import QtQuick

Item {
    id: av
    property string source: ""
    property string label: "?"
    property bool lit: false
    implicitWidth: 120; implicitHeight: 120
    readonly property string url: source !== "" ? "file://" + source : ""
    Image { id: probe; visible: false; source: av.url; asynchronous: false }
    Rectangle {
        anchors.fill: parent
        radius: width / 2
        visible: !cv.ready
        color: Theme.accent
        Text {
            anchors.centerIn: parent
            text: (av.label || "?").charAt(0).toUpperCase()
            color: Theme.accentInk
            font.family: Theme.font; font.pixelSize: Math.round(parent.width * 0.42); font.weight: Font.Bold
        }
    }
    Canvas {
        id: cv
        anchors.fill: parent
        property bool ready: false
        onPaint: {
            const c = getContext("2d");
            c.reset();
            if (!ready || probe.status !== Image.Ready) return;
            const iw = probe.implicitWidth, ih = probe.implicitHeight, s = Math.min(iw, ih);
            c.save();
            c.beginPath(); c.arc(width / 2, height / 2, width / 2, 0, 2 * Math.PI); c.closePath(); c.clip();
            c.drawImage(av.url, (iw - s) / 2, (ih - s) / 2, s, s, 0, 0, width, height);
            c.restore();
        }
        onImageLoaded: { ready = isImageLoaded(av.url); requestPaint(); }
        Connections {
            target: av
            function onUrlChanged() { cv.ready = false; if (av.url) cv.loadImage(av.url); cv.requestPaint(); }
        }
        Component.onCompleted: if (av.url) loadImage(av.url)
    }
    Rectangle {
        anchors.fill: parent; anchors.margins: -6
        radius: width / 2
        color: "transparent"
        border.width: 3
        border.color: av.lit ? Theme.accent : "transparent"
    }
}
