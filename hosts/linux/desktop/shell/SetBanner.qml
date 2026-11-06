// A banner over a page's cards when something needs a look ("1 thing needs a look · The cloud backup didn't finish").
// The whole banner takes focus; A (or a click) does its action.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: bn
    property string icon: "warn"
    property string title
    property string sub
    property string action
    property string kind: "warn"      // warn | bad | info
    readonly property color tone: kind === "bad" ? Theme.danger : kind === "info" ? Theme.accent : SetStyle.warn
    Layout.fillWidth: true
    interactive: action !== ""
    implicitHeight: Math.max(60, col.implicitHeight + 24)
    color: SetStyle.mix(SetStyle.card, tone, 0.12)
    border.width: 1
    border.color: SetStyle.tint(tone, 0.35)
    radius: SetStyle.bigRadius
    Accessible.role: Accessible.AlertMessage
    Accessible.name: title
    Accessible.description: sub

    RowLayout {
        anchors { fill: parent; leftMargin: SetStyle.pad; rightMargin: 12 }
        spacing: 14
        Icon { name: bn.icon; size: 22; color: bn.tone }
        ColumnLayout {
            id: col
            Layout.fillWidth: true
            spacing: 1
            Text { Layout.fillWidth: true; text: bn.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; visible: bn.sub !== ""; text: bn.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
        }
        Rectangle {
            visible: bn.action !== ""
            implicitWidth: actText.implicitWidth + 32; implicitHeight: 34
            radius: Math.max(6, Theme.r(8))
            color: SetStyle.card2
            Text { id: actText; anchors.centerIn: parent; text: bn.action; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
        }
    }
}
