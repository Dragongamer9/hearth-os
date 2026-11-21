// The Couch's top bar (mockups firehold-plasma 01-03): the Firehold flame tile and the tabs Home, Library, Apps, Media,
// Store; on the right the downloads pill (how many, the running one's ring), the controllers' batteries, the mode
// pill ("Couch · on the TV": this screen's mode and what it is), Wi-Fi, the clock and your letter or picture.
import QtQuick
import QtQuick.Shapes
import "fhp.js" as H

Item {
    id: bar
    property QtObject couch
    readonly property real k: couch.k
    readonly property var dls: (couch.cards.downloads || [])
    readonly property var running: dls.filter(function (d) { return d.state === "running"; })
    readonly property string pads: H.padsPill(couch.cards.controllers)

    Logo { id: logo; k: bar.k; anchors.verticalCenter: parent.verticalCenter }
    Row {
        id: tabRow
        anchors.left: logo.right; anchors.leftMargin: 34 * bar.k
        anchors.verticalCenter: parent.verticalCenter
        spacing: 30 * bar.k
        Repeater {
            model: [["home", "Home"], ["library", "Library"], ["apps", "Apps"], ["media", "Media"], ["store", "Store"]]
            delegate: Item {
                required property var modelData
                readonly property bool on: bar.couch.tab === modelData[0]
                width: label.implicitWidth; height: 48 * bar.k
                T {
                    id: label
                    couchRef: bar.couch
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: -4 * bar.k
                    text: modelData[1]
                    font.pixelSize: 22 * bar.k
                    font.weight: Font.DemiBold
                    color: parent.on ? "#ffffff" : H.P.tab
                }
                Rectangle {
                    visible: parent.on
                    anchors.top: label.bottom; anchors.topMargin: 8 * bar.k
                    width: label.width; height: 3 * bar.k
                    color: H.P.ac
                }
                MouseArea { anchors.fill: parent; onClicked: bar.couch.tab = modelData[0] }
            }
        }
    }
    Row {
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: 16 * bar.k
        Pill {
            couch: bar.couch
            visible: bar.dls.length > 0
            anchors.verticalCenter: parent.verticalCenter
            readonly property int n: bar.running.length || bar.dls.length
            text: n + (n === 1 ? " download" : " downloads")
            weight: Font.Medium
            Item {
                id: dlRing
                width: 20 * bar.k; height: width
                readonly property real pct: bar.running.length ? bar.running[0].percent : (bar.dls.length ? Math.max(0, bar.dls[0].percent) : 0)
                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer
                    ShapePath {
                        strokeColor: "#444444"; strokeWidth: 3 * bar.k; fillColor: "transparent"
                        PathAngleArc { centerX: 10 * bar.k; centerY: 10 * bar.k; radiusX: 8 * bar.k; radiusY: 8 * bar.k; startAngle: 0; sweepAngle: 360 }
                    }
                    ShapePath {
                        strokeColor: H.P.ac; strokeWidth: 3 * bar.k; fillColor: "transparent"
                        PathAngleArc { centerX: 10 * bar.k; centerY: 10 * bar.k; radiusX: 8 * bar.k; radiusY: 8 * bar.k; startAngle: -90; sweepAngle: 360 * Math.max(0.05, dlRing.pct / 100) }
                    }
                }
            }
        }
        Pill {
            couch: bar.couch
            visible: bar.pads !== ""
            anchors.verticalCenter: parent.verticalCenter
            text: bar.pads
            weight: Font.Medium
            Glyph { name: "pad"; size: 18 * bar.k; color: "#d8d1ca" }
        }
        Pill {
            couch: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            text: "Couch · on the " + (bar.couch.screenIsTv ? "TV" : bar.couch.screenWord)
            clickable: true
            onClicked: bar.couch.page = "settings"
            Rectangle { width: 9 * bar.k; height: width; radius: width / 2; color: H.P.ac }
        }
        T {
            couchRef: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            visible: text !== ""
            text: bar.couch.sys.net || ""
            color: H.P.mut
            font.pixelSize: 19 * bar.k
        }
        T {
            couchRef: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            visible: typeof bar.couch.sys.battery === "number" && bar.couch.sys.battery >= 0
            text: bar.couch.sys.battery + "%"
            color: H.P.mut
            font.pixelSize: 19 * bar.k
        }
        T {
            couchRef: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            text: Qt.formatTime(bar.couch.now, "h:mm AP")
            font.pixelSize: 22 * bar.k
            font.weight: Font.Bold
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 44 * bar.k; height: width; radius: width / 2
            color: "#2b2522"
            clip: true
            T {
                couchRef: bar.couch
                anchors.centerIn: parent
                visible: face.status !== Image.Ready
                text: bar.couch.sys.initial || "·"
                font.pixelSize: 19 * bar.k
                font.weight: Font.ExtraBold
            }
            Image {
                id: face
                anchors.fill: parent
                visible: status === Image.Ready
                source: bar.couch.sys.face ? "file://" + bar.couch.sys.face : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 88
            }
        }
    }
}
