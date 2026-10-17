// Pick one of a few options (segmented). Left/Right move between them; click works too.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: ch
    property string title
    property string sub
    property var options: []        // [{value, label}]
    property var current
    signal chosen(var value)
    usesLeftRight: true
    Layout.fillWidth: true
    implicitHeight: col.implicitHeight + 28
    readonly property int idx: Math.max(0, options.findIndex(o => o.value === current))
    onAdjusted: d => { const i = Math.max(0, Math.min(options.length - 1, idx + d)); if (options[i]) chosen(options[i].value); }
    onActivated: { if (options.length) chosen(options[(idx + 1) % options.length].value); }

    ColumnLayout {
        id: col
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14; leftMargin: 16; rightMargin: 16 }
        spacing: 10
        Text { text: ch.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
        Text { visible: ch.sub !== ""; Layout.fillWidth: true; text: ch.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: ch.options
                delegate: Rectangle {
                    id: opt
                    required property var modelData
                    readonly property bool lit: modelData.value === ch.current
                    Layout.fillWidth: true
                    implicitHeight: 40
                    radius: 10
                    color: lit ? Theme.accent : Theme.panel3
                    Text { anchors.centerIn: parent; text: opt.modelData.label; color: opt.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: opt.lit ? Font.DemiBold : Font.Normal }
                    MouseArea { anchors.fill: parent; onClicked: { ch.forceActiveFocus(); ch.chosen(opt.modelData.value); } }
                }
            }
        }
    }
}
