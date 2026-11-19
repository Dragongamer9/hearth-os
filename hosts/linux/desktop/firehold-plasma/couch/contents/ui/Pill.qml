// The mockups' .pill: a round-ended chip (top bar, controllers, filters).
import QtQuick

Rectangle {
    id: p
    property QtObject couch
    readonly property real k: couch.k
    property string text: ""
    property color textColor: "#f2ede8"
    property real fontSize: 16
    property int weight: Font.DemiBold
    property real padX: 16
    property real padY: 9
    default property alias lead: leadRow.data
    property bool clickable: false
    signal clicked()
    height: label.implicitHeight + 2 * padY * k
    width: row.implicitWidth + 2 * padX * k
    radius: height / 2
    color: "#12ffffff"
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 9 * p.k
        Row { id: leadRow; visible: children.length > 0; anchors.verticalCenter: parent.verticalCenter; spacing: 6 * p.k }
        T { id: label; couchRef: p.couch; anchors.verticalCenter: parent.verticalCenter; text: p.text; color: p.textColor; font.pixelSize: p.fontSize * p.k; font.weight: p.weight }
    }
    MouseArea { anchors.fill: parent; enabled: p.clickable; onClicked: p.clicked() }
}
