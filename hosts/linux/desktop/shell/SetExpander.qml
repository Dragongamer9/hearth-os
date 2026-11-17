// An expander card: a setting card that folds its details away (Keyboard › On-screen keyboard, Display › Night light).
// A / Enter (or a click) opens and closes it; the rows inside are drawn flat and indented and take focus only while it's
// open. `anchor`: opened by itself when Settings is opened at it (`settings page keyboard:osk`, SettingsNav.pageAnchor).
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: xp
    property string icon
    property string title
    property string sub
    property string value
    property string chip
    property string chipKind: "ok"
    property bool open: false
    property string anchor
    default property alias content: body.data
    readonly property alias head: head
    Layout.fillWidth: true
    spacing: 0
    function markInset() { for (const c of body.children) if (c.inset !== undefined) c.inset = true; }
    Component.onCompleted: {
        markInset();
        // The anchor is cleared a moment later, not here: opening Settings can make the page twice in a row (the frame
        // loads it when the page changes and again when the window comes up), and the second one must still see it.
        if (anchor !== "" && SettingsNav.pageAnchor === anchor) { const a = anchor; open = true; Qt.callLater(() => { head.forceActiveFocus(); }); Qt.callLater(() => { if (SettingsNav.pageAnchor === a) SettingsNav.pageAnchor = ""; }); }
    }

    SetRow {
        id: head
        icon: xp.icon; title: xp.title; sub: xp.sub; value: xp.value; chip: xp.chip; chipKind: xp.chipKind
        chevron: true
        chevronIcon: xp.open ? "chevu" : "chevd"
        onActivated: xp.open = !xp.open
        Accessible.role: Accessible.Button
        Accessible.description: (xp.open ? "Open. " : "Closed. ") + xp.sub
    }
    Rectangle {
        visible: xp.open
        Layout.fillWidth: true
        implicitHeight: body.implicitHeight
        color: SetStyle.sunk
        radius: SetStyle.radius
        border.width: 1
        border.color: SetStyle.tint(Theme.text, 0.05)
        ColumnLayout {
            id: body
            width: parent.width
            spacing: 0
            onChildrenChanged: xp.markInset()
        }
    }
}
