// A PIN pad for a settings page (Server mode's web page PIN): works with only a controller (the D-pad moves between the
// keys, A presses one), with a keyboard (type the digits, Backspace, Enter) and with a mouse or touch. Needed because the
// on-screen keyboard isn't there in Server mode's Settings, and a text box there couldn't be typed into with a pad.
// Up from the top row and Down from the bottom row leave the pad; Esc (B) goes back like every other card.
//   SetPinPad { minLength: 4; maxLength: 12; onSubmitted: pin => ... }   pad.clear(); pad.focusPad()
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: pad
    property int minLength: 4
    property int maxLength: 12
    property string hint: ""
    property string digits: ""
    property int sel: 0                       // the key the controller is on (0-11: 1 2 3 / 4 5 6 / 7 8 9 / ⌫ 0 ✓)
    readonly property var keys: ["1", "2", "3", "4", "5", "6", "7", "8", "9", "back", "0", "ok"]
    readonly property bool ready: digits.length >= minLength && digits.length <= maxLength
    signal submitted(string pin)
    function clear() { digits = ""; }
    function focusPad() { sel = 0; pad.forceActiveFocus(); }
    function press(k) {
        if (k === "back") digits = digits.slice(0, -1);
        else if (k === "ok") { if (ready) submitted(digits); }
        else if (digits.length < maxLength) digits += k;
    }

    Layout.fillWidth: true
    implicitHeight: col.implicitHeight + 28
    radius: SetStyle.radius
    color: SetStyle.card
    border.width: 1
    border.color: SetStyle.tint(Theme.text, 0.05)
    activeFocusOnTab: true
    z: activeFocus ? 2 : 0
    Accessible.role: Accessible.EditableText
    Accessible.name: "PIN, " + digits.length + " digits typed"

    Rectangle {
        visible: pad.activeFocus
        anchors { fill: parent; margins: -9 }
        radius: pad.radius + 9
        color: "transparent"
        border.width: 6
        border.color: SetStyle.tint(Theme.accent, 0.22)
    }

    function scrollIntoView() {
        let p = pad.parent;
        while (p && !(p.contentItem !== undefined && p.contentHeight !== undefined && p.contentY !== undefined)) p = p.parent;
        if (!p) return;
        const y = pad.mapToItem(p.contentItem, 0, 0).y;
        if (y < p.contentY + 12) p.contentY = Math.max(0, y - 12);
        else if (y + pad.height > p.contentY + p.height - 12) p.contentY = Math.min(p.contentHeight - p.height, y + pad.height - p.height + 12);
    }
    onActiveFocusChanged: if (activeFocus) scrollIntoView()

    Keys.onPressed: e => {
        const t = e.text;
        if (t.length === 1 && t >= "0" && t <= "9") { pad.press(t); e.accepted = true; return; }
        switch (e.key) {
        case Qt.Key_Left: if (pad.sel % 3 > 0) pad.sel--; break;
        case Qt.Key_Right: if (pad.sel % 3 < 2) pad.sel++; break;
        case Qt.Key_Up: if (pad.sel >= 3) pad.sel -= 3; else pad.nextItemInFocusChain(false).forceActiveFocus(); break;
        case Qt.Key_Down: if (pad.sel < 9) pad.sel += 3; else pad.nextItemInFocusChain(true).forceActiveFocus(); break;
        case Qt.Key_Tab: pad.nextItemInFocusChain(true).forceActiveFocus(); break;
        case Qt.Key_Backtab: pad.nextItemInFocusChain(false).forceActiveFocus(); break;
        case Qt.Key_Backspace: case Qt.Key_Delete: pad.press("back"); break;
        case Qt.Key_Space: pad.press(pad.keys[pad.sel]); break;
        // Enter: A on a controller presses the key it's on; once the PIN is long enough, the keyboard's Enter saves it
        // from anywhere (a controller can also move to ✓).
        case Qt.Key_Return: case Qt.Key_Enter:
            if (e.modifiers === Qt.NoModifier && pad.ready && pad.lastTyped) pad.press("ok"); else pad.press(pad.keys[pad.sel]);
            break;
        case Qt.Key_Escape: SettingsNav.back(); break;
        default: return;
        }
        if (e.key !== Qt.Key_Return && e.key !== Qt.Key_Enter) pad.lastTyped = false;
        e.accepted = true;
    }
    Keys.onReleased: e => { if (e.text.length === 1 && e.text >= "0" && e.text <= "9") pad.lastTyped = true; }
    property bool lastTyped: false            // the last thing was a typed digit (a keyboard), so Enter means "save"

    ColumnLayout {
        id: col
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Text {
                Layout.fillWidth: true
                text: pad.digits.length ? "●".repeat(pad.digits.length) : (pad.hint || ("A PIN of " + pad.minLength + " to " + pad.maxLength + " digits"))
                color: pad.digits.length ? Theme.text : Theme.muted
                font.family: Theme.font; font.pixelSize: pad.digits.length ? 22 : 14; font.letterSpacing: pad.digits.length ? 4 : 0
                elide: Text.ElideRight
            }
            Text {
                text: pad.digits.length + " / " + pad.maxLength
                color: pad.ready || !pad.digits.length ? Theme.muted : Theme.danger
                font.family: Theme.font; font.pixelSize: 13
            }
        }
        GridLayout {
            Layout.alignment: Qt.AlignHCenter
            columns: 3; rowSpacing: 8; columnSpacing: 8
            Repeater {
                model: pad.keys
                Rectangle {
                    id: key
                    required property string modelData
                    required property int index
                    readonly property bool on: pad.activeFocus && pad.sel === index
                    readonly property bool dim: modelData === "ok" && !pad.ready
                    Layout.preferredWidth: 92; Layout.preferredHeight: 46
                    radius: Math.max(6, Theme.r(8))
                    color: modelData === "ok" && pad.ready ? Theme.accent : (keyHover.containsMouse ? SetStyle.cardHover : SetStyle.card2)
                    border.width: on ? 3 : 0
                    border.color: Theme.accent
                    opacity: dim ? 0.5 : 1
                    Accessible.role: Accessible.Button
                    Accessible.name: modelData === "back" ? "Delete" : modelData === "ok" ? "Save the PIN" : modelData
                    Text {
                        anchors.centerIn: parent
                        visible: key.modelData !== "back" && key.modelData !== "ok"
                        text: key.modelData
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.Medium
                    }
                    Text {
                        anchors.centerIn: parent
                        visible: key.modelData === "ok"
                        text: "Save"
                        color: pad.ready ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold
                    }
                    Text {
                        anchors.centerIn: parent
                        visible: key.modelData === "back"
                        text: "⌫"
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 20
                    }
                    MouseArea {
                        id: keyHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: { pad.sel = key.index; pad.forceActiveFocus(); pad.press(key.modelData); }
                    }
                }
            }
        }
        Text {
            Layout.fillWidth: true
            text: "Controller: move with the D-pad, A presses a key, then Save. Keyboard: type the digits, then Enter."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }
}
