// One of the Desk bar's menus (mockups firehold-plasma 11, 12), the card itself: its rows (label, a note, the shortcut
// on the right; headings, separators, a "›" row that opens its list beside it, red for Force quit), the mockups' dark
// menu. Drawn in Plasma's popup (app/StdMenu.qml) and in Firehold on labwc's (desktop/shell/FhDeskMenu.qml, 1.16):
// one card on both. A click says which (picked) and the popup closes.
//   items, subItems: [label, shortcut or note on the right, action, kind (head | red | sep | sub | pair | off: greyed), what it
//   does, icon]; iconFile: the icon drawer (KIcon.qml under Plasma, FhIcon.qml under labwc).
import QtQuick
import QtQuick.Layouts

Item {
    id: m
    property var items: []
    property var subItems: []
    property bool subOpen: false
    property string iconFile: ""
    signal picked(string action)
    // (an app's own menus can have several "›" rows: the one that opened says so, and where it is)
    signal subRequested(var row)
    property real subAt: -1
    readonly property real subY: subAt >= 0 ? subAt : firstSubY
    readonly property real firstSubY: {
        let y = 8;
        for (const it of items) {
            if (it[3] === "sub") return y - 8;
            y += it[3] === "sep" ? 13 : it[3] === "head" ? 30 : 38;
        }
        return 0;
    }
    width: card.width + (m.subOpen ? sub.width + 10 : 0)
    height: Math.max(card.height, m.subOpen ? m.subY + sub.height : 0)
    Layout.minimumWidth: width; Layout.preferredWidth: width
    Layout.minimumHeight: height; Layout.preferredHeight: height
    Rectangle {
        id: card
        width: 520; height: m.rowsHeight(m.items) + 16
        radius: 14; color: "#221e1c"; border.width: 1; border.color: "#1affffff"
        Column {
            id: col
            x: 8; y: 8; width: parent.width - 16
            Repeater { model: m.items; delegate: row }
        }
    }
    Rectangle {
        id: sub
        visible: m.subOpen
        x: card.width + 10; y: m.subY
        width: 430; height: m.rowsHeight(m.subItems) + 16
        radius: 14; color: "#221e1c"; border.width: 1; border.color: "#1affffff"
        Column {
            id: subCol
            x: 8; y: 8; width: parent.width - 16
            Repeater { model: m.subItems; delegate: row }
        }
    }
    Component {
        id: row
        Item {
            id: r
            required property var modelData
            readonly property string kind: modelData[3] || ""
            readonly property bool hot: ma.containsMouse || (kind === "sub" && m.subOpen)
            width: parent ? parent.width : 0
            height: kind === "sep" ? 13 : kind === "head" ? 30 : 38
            Rectangle { visible: r.kind === "sep"; anchors.centerIn: parent; width: parent.width - 16; height: 1; color: "#14ffffff" }
            Rectangle {
                visible: r.kind !== "sep" && r.kind !== "head"
                anchors.fill: parent; radius: 8
                color: r.hot ? "#ff7a2f" : "transparent"
            }
            Loader {
                id: ic
                visible: !!r.modelData[5]
                x: 14; anchors.verticalCenter: parent.verticalCenter
                width: visible ? 18 : 0; height: 18
                active: visible
                source: m.iconFile || "KIcon.qml"
                onLoaded: { item.name = Qt.binding(() => r.modelData[5] || ""); if (item.mask !== undefined) { item.mask = true; item.tint = Qt.binding(() => r.hot ? "#1d0d03" : "#f2ede8"); } }
            }
            Text {
                visible: r.kind !== "sep"
                x: 14 + (ic.visible ? 26 : 0); anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 28 - (ic.visible ? 26 : 0) - right.implicitWidth - 12; elide: Text.ElideRight
                textFormat: Text.StyledText
                text: r.modelData[0] + (r.modelData[4] ? "<font color=\"" + (r.hot ? "#5a2a0c" : "#8f8781") + "\"> · " + r.modelData[4] + "</font>" : "")
                color: r.kind === "head" || r.kind === "off" ? "#6f6a66" : r.hot ? "#1d0d03" : r.kind === "red" ? "#ff9b9b" : "#f2ede8"
                font.pixelSize: r.kind === "head" ? 12 : 16; font.weight: r.kind === "head" ? Font.Bold : Font.Normal
                font.letterSpacing: r.kind === "head" ? 1 : 0
            }
            Text {
                id: right
                visible: r.kind !== "sep" && r.kind !== "head"
                anchors.right: parent.right; anchors.rightMargin: 14; anchors.verticalCenter: parent.verticalCenter
                text: r.modelData[1] || ""
                color: r.hot ? "#5a2a0c" : "#6f6a66"; font.pixelSize: 14
            }
            MouseArea {
                id: ma
                anchors.fill: parent
                enabled: r.kind !== "sep" && r.kind !== "head" && r.kind !== "off"
                hoverEnabled: true
                onEntered: if (r.kind === "sub") m.openSub(r)
                onClicked: function (mouse) {
                    if (r.kind === "sub") { m.openSub(r); return; }
                    // "Snap left · right": the left half snaps left, the right half right.
                    const a = r.kind === "pair" ? String(r.modelData[2]).split("|")[mouse.x < width / 2 ? 0 : 1] : r.modelData[2];
                    m.picked(a);
                }
            }
        }
    }
    function openSub(rowItem) { subAt = rowItem.y; subOpen = true; subRequested(rowItem.modelData); }
    function rowsHeight(list) {
        let h = 0;
        for (const it of list) h += it[3] === "sep" ? 13 : it[3] === "head" ? 30 : 38;
        return h;
    }
}
