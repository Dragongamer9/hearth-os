// A setting card: icon, title, a one-line description, and on the right a status chip, a value, and a chevron only
// when it opens a page (`chevron: true`; SetLinkCard sets it). Info cards (interactive: false) never look clickable.
//   chip / chipKind: a status chip ("Running" ok, "1 failed" warn, "New" new, "Off" off)
//   badge: a small chip right after the title ("New")
//   boxed: the icon in a tinted square (Home server › Services)
import QtQuick
import QtQuick.Layouts

SetBase {
    id: row
    property string icon
    property string title
    property string sub
    property string value
    property string chip
    property string chipKind: "ok"
    property string badge
    property bool boxed: false
    property bool chevron: false
    property string chevronIcon: "chev"   // SetExpander: "chevd" / "chevu"
    property bool danger: false
    property int trailing: 0          // room kept on the right for a control drawn over the row (SetSwitch's toggle)
    Layout.fillWidth: true
    implicitHeight: Math.max(SetStyle.cardHeight - (inset ? 10 : 0), col.implicitHeight + (inset ? 16 : 24))
    // For the screen reader (Orca, Settings › Accessibility): what this row is.
    Accessible.role: Accessible.Button
    Accessible.name: title
    Accessible.description: sub
    Accessible.onPressAction: activated()

    RowLayout {
        anchors { fill: parent; leftMargin: row.inset ? 56 : SetStyle.pad; rightMargin: SetStyle.pad + row.trailing }
        spacing: 16
        Rectangle {
            visible: row.icon !== "" && row.boxed && !row.inset
            Layout.preferredWidth: 40; Layout.preferredHeight: 40
            radius: Math.max(6, Theme.r(10))
            color: SetStyle.card2
            Icon { anchors.centerIn: parent; name: row.icon; size: 22; color: row.danger ? Theme.danger : Theme.text }
        }
        Icon {
            visible: row.icon !== "" && !row.boxed && !row.inset
            size: 22; name: row.icon
            color: row.danger ? Theme.danger : Theme.text
        }
        ColumnLayout {
            id: col
            Layout.fillWidth: true
            spacing: 2
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text {
                    Layout.fillWidth: row.badge === ""
                    Layout.maximumWidth: row.badge !== "" ? Math.max(40, parent.width - 90) : Number.POSITIVE_INFINITY
                    text: row.title; color: row.danger ? Theme.danger : Theme.text
                    font.family: Theme.font; font.pixelSize: row.inset ? 14 : 15; font.weight: Font.Medium
                    elide: Text.ElideRight
                }
                SetChip { visible: row.badge !== ""; text: row.badge; kind: "new" }
                Item { visible: row.badge !== ""; Layout.fillWidth: true }
            }
            Text {
                Layout.fillWidth: true; visible: row.sub !== ""; text: row.sub
                color: Theme.muted; font.family: Theme.font; font.pixelSize: row.inset ? 13 : 13
                wrapMode: Text.WordWrap; lineHeight: 1.08
            }
        }
        SetChip { visible: row.chip !== ""; text: row.chip; kind: row.chipKind }
        Text { visible: row.value !== ""; text: row.value; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; Layout.maximumWidth: 300; elide: Text.ElideRight }
        Icon { visible: row.chevron && row.interactive; name: row.chevronIcon; size: 18; color: Theme.muted; opacity: 0.8 }
    }
}
