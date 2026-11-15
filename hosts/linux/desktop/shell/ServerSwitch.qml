// Server mode: an on / off row on an opened tile's page (a store, a device, a quiet time, a rule): its name, a chip
// ("Using it", "Paused 1 h", "Through the router"), the switch, and the controller's focus ring when picked. Draw-only;
// clicks send toggled().
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: r
    property real k: 1
    property string label: ""
    property string sub: ""
    property string chip: ""
    property string chipKind: "ok"        // ok | warn | muted
    property bool on: false
    property bool picked: false
    property bool richLabel: false
    signal toggled()
    Layout.fillWidth: true
    implicitHeight: Math.max(48 * k, col.implicitHeight + 16 * k)
    radius: 10 * k
    color: "transparent"
    border.width: picked ? 2.5 * k : 0
    border.color: Theme.accent
    readonly property color chipC: chipKind === "warn" ? "#f0c060" : chipKind === "muted" ? "#c9b79f" : "#6fe39a"
    RowLayout {
        anchors { fill: parent; leftMargin: r.picked ? 12 * r.k : 0; rightMargin: r.picked ? 12 * r.k : 0 }
        spacing: 14 * r.k
        ColumnLayout {
            id: col
            Layout.fillWidth: true
            spacing: 2
            Text { Layout.fillWidth: true; text: r.label; textFormat: r.richLabel ? Text.StyledText : Text.PlainText; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 18 * r.k; elide: Text.ElideRight }
            Text { visible: r.sub !== ""; Layout.fillWidth: true; text: r.sub; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 13 * r.k; wrapMode: Text.WordWrap }
        }
        Rectangle {
            visible: r.chip !== ""
            implicitWidth: chipT.implicitWidth + 18 * r.k; implicitHeight: 26 * r.k; radius: 6 * r.k
            color: Qt.rgba(r.chipC.r, r.chipC.g, r.chipC.b, 0.14)
            Text { id: chipT; anchors.centerIn: parent; text: r.chip; color: r.chipC; font.family: Theme.font; font.pixelSize: 13 * r.k }
        }
        Rectangle {   // the switch
            implicitWidth: 54 * r.k; implicitHeight: 30 * r.k; radius: height / 2
            color: r.on ? "#ff8c42" : "#3a3330"
            Rectangle {
                width: 22 * r.k; height: width; radius: width / 2; anchors.verticalCenter: parent.verticalCenter
                x: r.on ? parent.width - width - 4 * r.k : 4 * r.k
                color: r.on ? "#1a120a" : "#9a9088"
                Behavior on x { NumberAnimation { duration: 120 } }
            }
        }
    }
    MouseArea { anchors.fill: parent; onClicked: r.toggled() }
    Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: "#3a2f22"; visible: !r.picked }
}
