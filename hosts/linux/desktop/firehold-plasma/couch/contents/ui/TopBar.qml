// The Couch's top bar (mockups 1 and 5): the logo tile, the tabs, and on the right the posture pill, Wi-Fi, battery,
// the clock and your initial (the Library has its search box there instead).
import QtQuick

Item {
    id: bar
    property QtObject couch
    readonly property real k: couch.k

    Logo { id: logo; k: bar.k; anchors.verticalCenter: parent.verticalCenter }
    Row {
        id: tabRow
        anchors.left: logo.right; anchors.leftMargin: 28 * bar.k
        anchors.verticalCenter: parent.verticalCenter
        spacing: 28 * bar.k
        Repeater {
            model: [["home", "Home"], ["library", "Library"], ["apps", "Apps"], ["store", "Store"]]
            delegate: Item {
                required property var modelData
                readonly property bool on: bar.couch.tab === modelData[0]
                width: label.implicitWidth; height: 48 * bar.k
                T {
                    id: label
                    couchRef: bar.couch
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData[1]
                    font.pixelSize: 24 * bar.k
                    font.weight: parent.on ? Font.Bold : Font.Medium
                    color: parent.on ? bar.couch.pal.text : bar.couch.pal.dim
                }
                Rectangle {
                    visible: parent.on
                    anchors.top: label.bottom; anchors.topMargin: 2 * bar.k
                    width: label.width; height: 3 * bar.k; radius: height / 2
                    color: bar.couch.pal.accent
                }
                MouseArea { anchors.fill: parent; onClicked: bar.couch.tab = modelData[0] }
            }
        }
    }
    Row {
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: 22 * bar.k
        visible: bar.couch.tab !== "library"
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            height: 38 * bar.k; radius: height / 2
            width: pillRow.implicitWidth + 36 * bar.k
            color: "#2a201b"
            Row {
                id: pillRow
                anchors.centerIn: parent
                spacing: 10 * bar.k
                Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 9 * bar.k; height: width; radius: width / 2; color: bar.couch.pal.accent }
                T {
                    couchRef: bar.couch
                    text: bar.couch.mode === "auto" ? "Couch · switches to Desk when you touch the mouse" : "Couch"
                    font.pixelSize: 16 * bar.k
                    font.weight: Font.Medium
                }
            }
            MouseArea { anchors.fill: parent; onClicked: bar.couch.page = "settings" }
        }
        T {
            couchRef: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            visible: text !== ""
            text: bar.couch.sys.net || ""
            color: bar.couch.pal.dim
            font.pixelSize: 20 * bar.k
        }
        T {
            couchRef: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            visible: typeof bar.couch.sys.battery === "number" && bar.couch.sys.battery >= 0
            text: bar.couch.sys.battery + "%"
            color: bar.couch.pal.dim
            font.pixelSize: 20 * bar.k
        }
        T {
            couchRef: bar.couch
            anchors.verticalCenter: parent.verticalCenter
            text: Qt.formatTime(bar.couch.now, "h:mm AP")
            font.pixelSize: 21 * bar.k
            font.weight: Font.Bold
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 44 * bar.k; height: width; radius: width / 2
            color: "#2a201b"
            T {
                couchRef: bar.couch
                anchors.centerIn: parent
                text: bar.couch.sys.initial || "·"
                font.pixelSize: 20 * bar.k
                font.weight: Font.Bold
            }
        }
    }
}
