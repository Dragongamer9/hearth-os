// The dock's and the taskbar's hover label, with pictures of the app's windows above it on Firehold's own compositor
// (its window IPC takes them: WinList.takePictures, every 2 seconds while it's up). On KWin, KWin draws its own live
// previews instead (hearth-previews.qml), and on labwc there are no pictures: then this is the label alone.
import QtQuick
import QtQuick.Window
import Quickshell

Rectangle {
    id: tip
    property string text: ""
    property var wins: []
    property bool shown: false
    readonly property var pics: wins.slice(0, 3)
    readonly property bool withPictures: pics.length > 0
    readonly property real picW: 200
    implicitWidth: Math.max(label.implicitWidth + 20, withPictures ? row.implicitWidth + 16 : 0)
    implicitHeight: label.implicitHeight + 12 + (withPictures ? row.implicitHeight + 8 : 0)
    radius: Theme.r(8)
    color: Theme.panel2
    border.color: Theme.line
    Timer {
        interval: 2000; repeat: true; triggeredOnStart: true
        running: tip.shown && tip.withPictures
        onTriggered: WinList.takePictures(tip.pics)
    }
    Row {
        id: row
        visible: tip.withPictures
        x: 8; y: 8
        spacing: 8
        Repeater {
            model: tip.pics
            delegate: Rectangle {
                required property var modelData
                width: tip.picW; height: Math.round(tip.picW * 0.6); radius: Theme.r(6)
                color: Theme.bg2; clip: true
                Image {
                    anchors.fill: parent; anchors.margins: 2
                    fillMode: Image.PreserveAspectFit; asynchronous: false; cache: false; smooth: true; mipmap: true
                    sourceSize: Qt.size(Math.ceil(width * Screen.devicePixelRatio), Math.ceil(height * Screen.devicePixelRatio))
                    source: WinList.pictureUrl(parent.modelData)
                }
            }
        }
    }
    Text {
        id: label
        anchors.horizontalCenter: parent.horizontalCenter
        y: tip.withPictures ? row.y + row.implicitHeight + 6 : 6
        text: tip.text; color: Theme.text; font.pixelSize: 12; font.family: Theme.font
    }
}
