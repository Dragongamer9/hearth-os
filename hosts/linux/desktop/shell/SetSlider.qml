// A slider: Left/Right (or the D-pad) change it in steps; click or drag the bar. On a wide card the bar sits on the
// right with its value; on a narrow one, under the title.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: sl
    property string icon
    property string title
    property string sub
    property real value: 0          // 0..1
    property real step: 0.05
    property string valueText: Math.round(value * 100) + "%"
    signal moved(real v)
    usesLeftRight: true
    Layout.fillWidth: true
    readonly property bool wide: width >= 640
    readonly property int textLeft: (inset ? 56 : SetStyle.pad) + (icon !== "" && !inset ? 38 : 0)
    implicitHeight: wide ? Math.max(SetStyle.cardHeight, txt.implicitHeight + 24) : txt.implicitHeight + 52
    onAdjusted: d => moved(Math.max(0, Math.min(1, value + d * step)))
    Accessible.role: Accessible.Slider
    Accessible.name: title
    Accessible.description: valueText

    Icon { visible: sl.icon !== "" && !sl.inset; x: SetStyle.pad; y: sl.wide ? (sl.height - height) / 2 : 14; size: 22; name: sl.icon; color: Theme.text }
    ColumnLayout {
        id: txt
        x: sl.textLeft
        y: sl.wide ? (sl.height - height) / 2 : 14
        width: sl.wide ? sl.width - sl.textLeft - bar.width - SetStyle.pad * 2 : sl.width - sl.textLeft - SetStyle.pad
        spacing: 2
        Text { Layout.fillWidth: true; text: sl.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium; elide: Text.ElideRight }
        Text { visible: sl.sub !== ""; Layout.fillWidth: true; text: sl.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
    }
    Row {
        id: bar
        x: sl.wide ? sl.width - width - SetStyle.pad : sl.textLeft
        y: sl.wide ? (sl.height - height) / 2 : txt.y + txt.height + 12
        spacing: 12
        Item {
            id: track
            width: sl.wide ? 240 : sl.width - sl.textLeft - SetStyle.pad - valueLabel.width - 12
            height: 20
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 4; radius: 2; color: SetStyle.card2 }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: Math.max(4, track.width * Math.max(0, Math.min(1, sl.value))); height: 4; radius: 2
                color: Theme.accent
            }
            Rectangle {
                width: 20; height: 20; radius: 10
                x: track.width * Math.max(0, Math.min(1, sl.value)) - 10
                color: Theme.accent
                border.width: 4
                border.color: SetStyle.card
            }
            MouseArea {
                anchors { fill: parent; topMargin: -12; bottomMargin: -12 }
                cursorShape: Qt.PointingHandCursor
                onPressed: m => { sl.forceActiveFocus(); sl.moved(Math.max(0, Math.min(1, m.x / track.width))); }
                onPositionChanged: m => { if (pressed) sl.moved(Math.max(0, Math.min(1, m.x / track.width))); }
            }
        }
        Text { id: valueLabel; anchors.verticalCenter: parent.verticalCenter; text: sl.valueText; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; width: Math.max(implicitWidth, 44); horizontalAlignment: Text.AlignRight }
    }
}
