// A slider: Left/Right (or the D-pad) change it in steps; click or drag the bar.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: sl
    property string icon
    property string title
    property real value: 0          // 0..1
    property real step: 0.05
    property string valueText: Math.round(value * 100) + "%"
    signal moved(real v)
    usesLeftRight: true
    Layout.fillWidth: true
    implicitHeight: 76
    onAdjusted: d => moved(Math.max(0, Math.min(1, value + d * step)))

    RowLayout {
        anchors { fill: parent; leftMargin: 16; rightMargin: 18 }
        spacing: 14
        Rectangle {
            Layout.preferredWidth: 38; Layout.preferredHeight: 38; radius: 10; color: Theme.panel3
            Icon { anchors.centerIn: parent; name: sl.icon; size: 20 }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 10
            RowLayout {
                Layout.fillWidth: true
                Text { text: sl.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
                Text { text: sl.valueText; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            }
            Item {
                id: track
                Layout.fillWidth: true
                implicitHeight: 8
                Rectangle { anchors.fill: parent; radius: 4; color: Theme.panel3 }
                Rectangle {
                    width: Math.max(8, track.width * Math.max(0, Math.min(1, sl.value))); height: parent.height; radius: 4
                    gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: Theme.ember } GradientStop { position: 1; color: Theme.flame } }
                }
                MouseArea {
                    anchors { fill: parent; topMargin: -14; bottomMargin: -14 }
                    onPressed: m => { sl.forceActiveFocus(); sl.moved(Math.max(0, Math.min(1, m.x / track.width))); }
                    onPositionChanged: m => { if (pressed) sl.moved(Math.max(0, Math.min(1, m.x / track.width))); }
                }
            }
        }
    }
}
