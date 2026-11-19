// One of the Desk bar's standard menus (mockup firehold-plasma 12): the app's own name menu, Window, Help, for an app
// with no menus of its own. Plasma's popup under its button, the mockup's dark menu; a click does it and closes it.
import QtQuick
import org.kde.plasma.core as PlasmaCore

PlasmaCore.Dialog {
    id: m
    property var items: []          // [label, shortcut, action, head|red|sep]
    property Item anchorItem: null
    signal picked(string action)
    visualParent: anchorItem
    location: PlasmaCore.Types.TopEdge
    type: PlasmaCore.Dialog.PopupMenu
    backgroundHints: PlasmaCore.Dialog.NoBackground
    hideOnWindowDeactivate: true
    flags: Qt.WindowStaysOnTopHint
    mainItem: card
    Rectangle {
        id: card
        width: 520; height: col.implicitHeight + 16
        radius: 14; color: "#221e1c"; border.width: 1; border.color: "#1affffff"
        Column {
            id: col
            x: 8; y: 8; width: parent.width - 16
            Repeater {
                model: m.items
                delegate: Item {
                    required property var modelData
                    width: col.width
                    height: modelData[3] === "sep" ? 13 : modelData[3] === "head" ? 30 : 38
                    Rectangle { visible: modelData[3] === "sep"; anchors.centerIn: parent; width: parent.width - 16; height: 1; color: "#14ffffff" }
                    Rectangle {
                        visible: modelData[3] !== "sep" && modelData[3] !== "head"
                        anchors.fill: parent; radius: 8
                        color: ma.containsMouse ? "#ff7a2f" : "transparent"
                    }
                    Text {
                        visible: modelData[3] !== "sep"
                        x: 14; anchors.verticalCenter: parent.verticalCenter
                        text: modelData[0]
                        color: modelData[3] === "head" ? "#6f6a66" : ma.containsMouse ? "#1d0d03" : modelData[3] === "red" ? "#ff9b9b" : "#f2ede8"
                        font.pixelSize: modelData[3] === "head" ? 12 : 16; font.weight: modelData[3] === "head" ? Font.Bold : Font.Normal
                        font.letterSpacing: modelData[3] === "head" ? 1 : 0
                    }
                    Text {
                        visible: modelData[3] !== "sep" && modelData[3] !== "head"
                        anchors.right: parent.right; anchors.rightMargin: 14; anchors.verticalCenter: parent.verticalCenter
                        text: modelData[1] || ""
                        color: ma.containsMouse ? "#5a2a0c" : "#6f6a66"; font.pixelSize: 14
                    }
                    MouseArea {
                        id: ma
                        anchors.fill: parent
                        enabled: modelData[3] !== "sep" && modelData[3] !== "head"
                        hoverEnabled: true
                        onClicked: { m.picked(modelData[2]); m.visible = false; }
                    }
                }
            }
        }
    }
}
