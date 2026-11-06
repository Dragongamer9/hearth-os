// Pick one of a few options (segmented). On a wide card the options sit on the right; when they don't fit, under the
// title. Left/Right move between them; click works too.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: ch
    property string icon
    property string title
    property string sub
    property var options: []        // [{value, label, disabled}] (a disabled one is shown greyed and can't be picked)
    property var current
    signal chosen(var value)
    usesLeftRight: true
    Layout.fillWidth: true
    readonly property int textLeft: (inset ? 56 : SetStyle.pad) + (icon !== "" && !inset ? 38 : 0)
    readonly property bool inline: measure.implicitWidth + textLeft + SetStyle.pad * 2 + 260 <= width
    implicitHeight: inline ? Math.max(SetStyle.cardHeight - (inset ? 10 : 0), txt.implicitHeight + 24)
                           : txt.implicitHeight + segs.implicitHeight + 38
    readonly property int idx: Math.max(0, options.findIndex(o => o.value === current))
    function pickable(i) { return !!options[i] && !options[i].disabled; }
    onAdjusted: d => { let i = idx + d; while (i >= 0 && i < options.length && !pickable(i)) i += d; if (pickable(i)) chosen(options[i].value); }
    onActivated: { for (let k = 1; k <= options.length; k++) { const i = (idx + k) % options.length; if (pickable(i)) { if (i !== idx) chosen(options[i].value); return; } } }
    Accessible.role: Accessible.ComboBox
    Accessible.name: title
    Accessible.description: (options[idx] || {}).label || ""

    Icon {
        visible: ch.icon !== "" && !ch.inset
        x: SetStyle.pad
        y: ch.inline ? (ch.height - height) / 2 : 14 + Math.max(0, (titleText.implicitHeight - height) / 2)
        size: 22; name: ch.icon; color: Theme.text
    }
    ColumnLayout {
        id: txt
        x: ch.textLeft
        y: ch.inline ? (ch.height - height) / 2 : 14
        width: ch.inline ? ch.width - ch.textLeft - segs.width - SetStyle.pad * 2 : ch.width - ch.textLeft - SetStyle.pad
        spacing: 2
        Text { id: titleText; Layout.fillWidth: true; text: ch.title; color: Theme.text; font.family: Theme.font; font.pixelSize: ch.inset ? 14 : 15; font.weight: Font.Medium; elide: Text.ElideRight }
        Text { visible: ch.sub !== ""; Layout.fillWidth: true; text: ch.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: ch.inset ? 13 : 13; wrapMode: Text.WordWrap }
    }
    // The options at their natural size, to see whether they fit on the right.
    Row {
        id: measure
        visible: false
        spacing: 3
        padding: 3
        Repeater { model: ch.options; delegate: Item { required property var modelData; implicitWidth: m.implicitWidth + 28; implicitHeight: 28; Text { id: m; text: modelData.label; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold } } }
    }
    Rectangle {
        id: segs
        x: ch.inline ? ch.width - width - SetStyle.pad : ch.textLeft
        y: ch.inline ? (ch.height - height) / 2 : txt.y + txt.height + 10
        width: ch.inline ? measure.implicitWidth + 2 : Math.min(measure.implicitWidth + 2, ch.width - ch.textLeft - SetStyle.pad)
        implicitHeight: flow.implicitHeight + 6
        height: implicitHeight
        radius: Math.max(6, Theme.r(9))
        color: SetStyle.sunk
        border.width: 1
        border.color: Theme.line
        Flow {
            id: flow
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 3 }
            spacing: 3
            Repeater {
                model: ch.options
                delegate: Rectangle {
                    id: opt
                    required property var modelData
                    readonly property bool lit: modelData.value === ch.current
                    implicitWidth: lbl.implicitWidth + 28
                    implicitHeight: 28
                    radius: Math.max(4, Theme.r(6))
                    color: lit ? Theme.accent : optMouse.containsMouse ? SetStyle.card2 : "transparent"
                    opacity: opt.modelData.disabled ? 0.45 : 1
                    Text { id: lbl; anchors.centerIn: parent; text: opt.modelData.label; color: opt.lit ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 14; font.weight: opt.lit ? Font.DemiBold : Font.Normal }
                    MouseArea { id: optMouse; anchors.fill: parent; hoverEnabled: true; enabled: !opt.modelData.disabled; cursorShape: Qt.PointingHandCursor; onClicked: { ch.forceActiveFocus(); ch.chosen(opt.modelData.value); } }
                }
            }
        }
    }
}
