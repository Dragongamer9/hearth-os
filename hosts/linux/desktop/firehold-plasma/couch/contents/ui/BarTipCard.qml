// The Desk bar's first-run tip (mockup firehold-plasma 11), the card itself: each part of the bar, left to right, and
// Alt+Space. Drawn in Plasma's popup (posture/BarTip.qml) and by Firehold on labwc's Desk bar (1.16).
import QtQuick
import "fhp.js" as H

Rectangle {
    id: tip
    readonly property QtObject ui: QtObject { property real k: 1; property string fontName: Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : "Noto Sans" }
    readonly property var parts: [["F", "The app in front", "Its name; click: About, Settings, Quit"],
                                  ["menu", "Its menus", "KDE's global menu, from the app itself"],
                                  ["clock", "Date and clock", "Click: calendar and notifications"],
                                  ["dot", "Mode pill", "This screen's mode, and the TV's"],
                                  ["download", "Downloads", "Steam, Heroic, Beacon and Firefox in one list"],
                                  ["link", "Hearth Link", "Your devices and Dad's PC"],
                                  ["pad", "Controllers", "Batteries; who is player 1"],
                                  ["speaker", "Quick Settings", "Wi-Fi, sound, power, modes"],
                                  ["bell", "Notifications", "How many; click for all of them"]]
    width: 520; height: col.implicitHeight + 44
    radius: 14; color: "#221e1c"; border.width: 1; border.color: "#1affffff"
    Column {
        id: col
        x: 18; y: 22
        width: parent.width - 36
        spacing: 8
        T { couchRef: tip.ui; text: "THE BAR, LEFT TO RIGHT"; color: H.P.dim; font.pixelSize: 12; font.weight: Font.Bold; font.letterSpacing: 1 }
        Repeater {
            model: tip.parts
            delegate: Row {
                required property var modelData
                spacing: 12
                Rectangle {
                    width: 32; height: 32; radius: 8; color: "#14ffffff"
                    T { visible: modelData[0] === "F"; couchRef: tip.ui; anchors.centerIn: parent; text: "F"; font.pixelSize: 14; font.weight: Font.ExtraBold }
                    Rectangle { visible: modelData[0] === "dot"; anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: "#f2ede8" }
                    Glyph { visible: ["F", "dot"].indexOf(modelData[0]) < 0; anchors.centerIn: parent; name: modelData[0]; size: 16 }
                }
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    T { couchRef: tip.ui; text: modelData[1]; font.pixelSize: 15; font.weight: Font.Bold }
                    T { couchRef: tip.ui; text: " · " + modelData[2]; color: H.P.mut; font.pixelSize: 14 }
                }
            }
        }
        T { couchRef: tip.ui; text: "Alt+Space: search every command of the app in front"; color: H.P.dim; font.pixelSize: 13 }
    }
}
