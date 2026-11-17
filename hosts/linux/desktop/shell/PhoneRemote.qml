// Settings › Phone › "This PC's keyboard and mouse on the phone" (Hearth Connect's plugins/remote.rs, KDE Connect's
// Remote keyboard and the phone's mouse receiver): a touchpad here moves a pointer on the phone (tap: click, right
// button: long press, two-finger / wheel: scroll), and "Type on the phone" sends what you type to the field the
// phone has open. Esc ends typing (the Esc button sends one to the phone). It talks to the service over its own
// socket (the one hearth-phone uses; only this user can open it), so the pointer keeps up. Sends only; saves nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetBase {
    id: rc
    property var phone: null
    readonly property bool ready: !!phone && phone.reachable && !!phone.remoteInput
    property bool typing: false
    property string said: ""
    Layout.fillWidth: true
    implicitHeight: col.implicitHeight + 32
    interactive: ready
    onActivated: if (ready) startTyping()
    function startTyping() { typing = true; catcher.forceActiveFocus(); }
    function stopTyping() { typing = false; rc.forceActiveFocus(); }
    border.color: typing ? Theme.accent : SetStyle.tint(Theme.text, 0.05)
    border.width: typing ? 2 : 1

    // ---- the service's socket
    readonly property string sockPath: Quickshell.env("HEARTH_CONNECT_SOCKET") || ((Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth/connect.sock")
    property int seq: 0
    Socket {
        id: sock
        path: rc.sockPath
        connected: rc.visible && rc.ready
        parser: SplitParser { onRead: line => { try { const v = JSON.parse(line); if (v.ok === false && v.error) rc.said = v.error; } catch (e) {} } }
    }
    function send(cmd, args) {
        if (!phone || !sock.connected) return;
        args.id = phone.id;
        sock.write(JSON.stringify({ id: ++seq, cmd: cmd, args: args }) + "\n");
        sock.flush();
    }
    // Moves add up and go ~30 times a second.
    property real mx: 0
    property real my: 0
    Timer { interval: 33; repeat: true; running: rc.mx !== 0 || rc.my !== 0
        onTriggered: { rc.send("remote_mouse", { dx: Math.round(rc.mx * 10) / 10, dy: Math.round(rc.my * 10) / 10 }); rc.mx = 0; rc.my = 0; } }

    // ---- typing
    readonly property var specials: ({
        [Qt.Key_Backspace]: "Backspace", [Qt.Key_Tab]: "Tab", [Qt.Key_Left]: "Left", [Qt.Key_Up]: "Up", [Qt.Key_Right]: "Right",
        [Qt.Key_Down]: "Down", [Qt.Key_PageUp]: "PageUp", [Qt.Key_PageDown]: "PageDown", [Qt.Key_Home]: "Home", [Qt.Key_End]: "End",
        [Qt.Key_Return]: "Enter", [Qt.Key_Enter]: "Enter", [Qt.Key_Delete]: "Delete",
        [Qt.Key_F1]: "F1", [Qt.Key_F2]: "F2", [Qt.Key_F3]: "F3", [Qt.Key_F4]: "F4", [Qt.Key_F5]: "F5", [Qt.Key_F6]: "F6",
        [Qt.Key_F7]: "F7", [Qt.Key_F8]: "F8", [Qt.Key_F9]: "F9", [Qt.Key_F10]: "F10", [Qt.Key_F11]: "F11", [Qt.Key_F12]: "F12"
    })
    // While typing, the keys go here (not to the card, whose Up/Down/Enter move around Settings).
    Item {
        id: catcher
        width: 0; height: 0
        onActiveFocusChanged: if (!activeFocus) rc.typing = false
        Keys.onPressed: e => rc.typed(e)
    }
    function typed(e) {
        e.accepted = true;
        if (e.key === Qt.Key_Escape) return stopTyping();
        const mods = { shift: !!(e.modifiers & Qt.ShiftModifier), ctrl: !!(e.modifiers & Qt.ControlModifier), alt: !!(e.modifiers & Qt.AltModifier) };
        const sp = specials[e.key];
        if (sp) return send("remote_key", Object.assign({ special: sp }, mods));
        if (e.text && e.text.length === 1 && e.text.charCodeAt(0) >= 32) return send("remote_key", Object.assign({ key: e.text }, mods, { shift: false }));
        if (mods.ctrl && e.key >= Qt.Key_A && e.key <= Qt.Key_Z) return send("remote_key", Object.assign({ key: String.fromCharCode(97 + e.key - Qt.Key_A) }, mods));
    }

    ColumnLayout {
        id: col
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Icon { name: "kbd"; size: 22; color: Theme.text }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text { Layout.fillWidth: true; text: "This PC's keyboard and mouse on " + (rc.phone ? rc.phone.name : "the phone"); color: Theme.text; font.family: Theme.font; font.pixelSize: SetStyle.titleSize; elide: Text.ElideRight }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: !rc.phone || !rc.phone.reachable ? "Works while the phone is connected"
                        : !rc.phone.remoteInput ? "On the phone: KDE Connect › this PC › turn on Remote keyboard and Mouse receiver"
                        : rc.said !== "" ? rc.said
                        : rc.phone.keyboard ? "The phone's KDE Connect keyboard is on: what you type goes into the field it has open"
                        : "Pick \"KDE Connect Remote Keyboard\" as the keyboard on the phone to type there"
                    color: rc.said !== "" ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: SetStyle.subSize
                }
            }
            SetChip { visible: rc.ready; text: rc.phone && rc.phone.keyboard ? "Keyboard ready" : "Pointer only"; kind: rc.phone && rc.phone.keyboard ? "ok" : "off" }
        }
        // The touchpad.
        Rectangle {
            id: pad
            Layout.fillWidth: true
            Layout.preferredHeight: 200
            radius: SetStyle.radius
            color: SetStyle.sunk
            border.width: 1
            border.color: padMouse.pressed ? Theme.accent : SetStyle.tint(Theme.text, 0.08)
            opacity: rc.ready ? 1 : 0.5
            Text {
                anchors.centerIn: parent
                width: parent.width - 40
                horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
                text: rc.ready ? "Touchpad for " + rc.phone.name + "\nDrag to move · click to tap · right-click for a long press · wheel to scroll" : "Touchpad"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
            MouseArea {
                id: padMouse
                anchors.fill: parent
                enabled: rc.ready
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                property real lx: 0
                property real ly: 0
                property bool moved: false
                cursorShape: Qt.CrossCursor
                onPressed: m => { lx = m.x; ly = m.y; moved = false; if (!rc.typing) rc.forceActiveFocus(); }
                onPositionChanged: m => {
                    const dx = m.x - lx, dy = m.y - ly;
                    if (Math.abs(dx) + Math.abs(dy) > 0.5) { rc.mx += dx * 1.6; rc.my += dy * 1.6; lx = m.x; ly = m.y; if (Math.abs(dx) + Math.abs(dy) > 2) moved = true; }
                }
                onReleased: m => {
                    if (moved) return;
                    rc.send("remote_mouse", { click: m.button === Qt.RightButton ? "right" : m.button === Qt.MiddleButton ? "middle" : "single" });
                }
                onDoubleClicked: rc.send("remote_mouse", { click: "double" })
                onWheel: w => rc.send("remote_mouse", { scroll: -w.angleDelta.y / 24 })
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            SetButton { text: rc.typing ? "Typing on the phone… (Esc stops)" : "Type on the phone"; icon: "kbd"; primary: rc.typing; enabled: rc.ready
                onActivated: rc.typing ? rc.stopTyping() : rc.startTyping() }
            SetButton { text: "Esc"; enabled: rc.ready; onActivated: rc.send("remote_key", { special: "Escape" }) }
            SetButton { text: "Enter"; enabled: rc.ready; onActivated: rc.send("remote_key", { special: "Enter" }) }
            SetButton { text: "Back space"; enabled: rc.ready; onActivated: rc.send("remote_key", { special: "Backspace" }) }
            Item { Layout.fillWidth: true }
        }
    }
}
