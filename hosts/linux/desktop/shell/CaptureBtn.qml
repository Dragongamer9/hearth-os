// A button for Hearth Capture and the password prompt (Capture.qml, CaptureEditor.qml, PolkitPrompt.qml): a text, a
// Hearth icon (Icon.qml) or a drawn tool picture (CaptureGlyph.qml), or both. Takes focus like a Settings card: the
// arrow keys (the D-pad: Pad.qml types arrow keys while Capture's pages are open) move along the row, Enter / Space
// (A) presses it, with the same thick accent ring the controller shows everywhere else.
import QtQuick

Rectangle {
    id: b
    property string text: ""
    property string icon: ""          // Hearth's icon set (icons/)
    property string glyph: ""         // a drawn picture (CaptureGlyph.qml): arrow, box, pen, highlighter...
    property bool primary: false
    property bool selected: false     // a tool or a choice that's on
    property bool enabledLook: true   // false: shown dimmed (and still says why when pressed)
    property bool big: false
    property bool arrowsMove: true    // arrow keys go to the next / previous control
    property color swatch: "transparent"   // a colour choice: the button is that colour
    property string tip: ""
    signal activated()

    activeFocusOnTab: true
    implicitWidth: swatch.a > 0 ? 34 : row.implicitWidth + (text !== "" ? 28 : 18)
    implicitHeight: big ? 52 : 38
    radius: Math.max(6, Theme.r(9))
    opacity: enabledLook ? 1 : 0.45
    color: swatch.a > 0 ? swatch
        : primary ? Theme.accent
        : selected ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
        : hover.containsMouse ? Theme.panel3 : Theme.panel2
    border.width: selected || swatch.a > 0 ? 2 : 1
    border.color: selected ? Theme.accent : swatch.a > 0 ? Qt.rgba(1, 1, 1, 0.25) : Theme.line
    Accessible.role: Accessible.Button
    Accessible.name: text !== "" ? text : tip

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8
        CaptureGlyph {
            visible: b.glyph !== ""
            anchors.verticalCenter: parent.verticalCenter
            name: b.glyph; size: b.big ? 22 : 18
            color: b.primary ? Theme.accentInk : b.selected ? Theme.accent : Theme.text
        }
        Icon {
            visible: b.icon !== "" && b.glyph === ""
            anchors.verticalCenter: parent.verticalCenter
            name: b.icon; size: b.big ? 22 : 17
            color: b.primary ? Theme.accentInk : b.selected ? Theme.accent : Theme.text
        }
        Text {
            visible: b.text !== ""
            anchors.verticalCenter: parent.verticalCenter
            text: b.text
            color: b.primary ? Theme.accentInk : Theme.text
            font.family: Theme.font; font.pixelSize: b.big ? 16 : 14
            font.weight: b.primary || b.selected ? Font.DemiBold : Font.Medium
        }
    }
    // The controller's focus ring (as SetBase.qml): a soft glow and a crisp accent line, a little outside.
    Rectangle {
        visible: b.activeFocus
        anchors { fill: parent; margins: -7 }
        radius: b.radius + 7
        color: "transparent"; border.width: 5
        border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25)
    }
    Rectangle {
        visible: b.activeFocus
        anchors { fill: parent; margins: -3 }
        radius: b.radius + 3
        color: "transparent"; border.width: 2; border.color: Theme.accent
    }
    MouseArea {
        id: hover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: { b.forceActiveFocus(); b.activated(); }
    }
    Keys.onPressed: e => {
        if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) { b.activated(); e.accepted = true; }
        else if (b.arrowsMove && (e.key === Qt.Key_Right || e.key === Qt.Key_Down)) { const n = b.nextItemInFocusChain(true); if (n) n.forceActiveFocus(); e.accepted = true; }
        else if (b.arrowsMove && (e.key === Qt.Key_Left || e.key === Qt.Key_Up)) { const n = b.nextItemInFocusChain(false); if (n) n.forceActiveFocus(); e.accepted = true; }
    }
}
