// The lock screen for Desktop Mode (Mantel): Super+L, "Lock" in the power menu, or by itself when the device
// wakes from sleep (Hearth Settings › Sign-in › Lock when waking). Covers every screen (the compositor keeps
// everything else hidden until it's unlocked) and checks your password with the system (PAM), like the login
// screen. It has its own keyboard on screen (tap, or move with the arrow keys / D-pad and press Enter), since
// the regular on-screen keyboard can't show over a locked screen.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pam

Scope {
    id: root
    property bool locked: false
    property string typed: ""
    property string message: ""
    property bool busy: false

    function lock() { if (!locked) { typed = ""; message = ""; locked = true; } }
    function tryUnlock() {
        if (busy) return;
        busy = true; message = "";
        pam.start();
    }

    PamContext {
        id: pam
        onPamMessage: { if (pam.responseRequired) pam.respond(root.typed); }
        onCompleted: result => {
            root.busy = false;
            if (result === PamResult.Success) { root.locked = false; root.typed = ""; }
            else { root.message = "That password isn't right"; root.typed = ""; }
        }
        onError: { root.busy = false; root.message = "Couldn't check the password"; }
    }

    // Lock when going to sleep (so it's locked when the device wakes), if that's turned on.
    Process {
        running: Settings.lockOnWake
        command: ["gdbus", "monitor", "--system", "--dest", "org.freedesktop.login1", "--object-path", "/org/freedesktop/login1"]
        stdout: SplitParser { onRead: l => { if (l.indexOf("PrepareForSleep (true") >= 0) root.lock(); } }
    }

    WlSessionLock {
        id: session
        locked: root.locked

        WlSessionLockSurface {
            id: surface
            color: Theme.bg

            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: "#17110e" }
                    GradientStop { position: 0.7; color: "#0f0b09" }
                    GradientStop { position: 1.0; color: "#2a140a" }
                }
            }
            Image { anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom; width: parent.width * 1.2; height: parent.height * 0.6; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.5 }

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 80, 760)
                spacing: 14

                Text {
                    id: clock
                    Layout.alignment: Qt.AlignHCenter
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 84; font.weight: Font.DemiBold
                    text: Qt.formatTime(new Date(), "h:mm")
                    Timer { interval: 5000; running: root.locked; repeat: true; triggeredOnStart: true
                        onTriggered: { clock.text = Qt.formatTime(new Date(), "h:mm"); date.text = Qt.formatDate(new Date(), "dddd, MMMM d"); } }
                }
                Text { id: date; Layout.alignment: Qt.AlignHCenter; color: Theme.muted; font.family: Theme.font; font.pixelSize: 20 }

                Rectangle {
                    Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 24
                    Layout.preferredWidth: 420; Layout.preferredHeight: 56; radius: 14
                    color: Theme.panel2; border.color: root.message ? Theme.danger : Theme.line; border.width: 2
                    Text {
                        anchors.centerIn: parent
                        text: root.busy ? "Checking…" : root.typed.length ? "•".repeat(root.typed.length) : "Password"
                        color: root.typed.length ? Theme.text : Theme.muted; font.family: Theme.font; font.pixelSize: 22
                    }
                }
                Text { Layout.alignment: Qt.AlignHCenter; visible: root.message !== ""; text: root.message; color: Theme.danger; font.family: Theme.font; font.pixelSize: 14 }

                // Its own keyboard: tap, or arrows / D-pad and Enter.
                Item {
                    id: kb
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: Math.min(surface.width - 80, 760)
                    Layout.preferredHeight: 5 * 54
                    focus: true
                    property bool upper: false
                    property bool sym: false
                    property int r: 1
                    property int c: 0
                    readonly property var letters: [
                        ["1","2","3","4","5","6","7","8","9","0"],
                        ["q","w","e","r","t","y","u","i","o","p"],
                        ["a","s","d","f","g","h","j","k","l","⌫"],
                        ["⇧","z","x","c","v","b","n","m","#+=","Unlock"]]
                    readonly property var symbols: [
                        ["!","@","#","$","%","^","&","*","(",")"],
                        ["-","_","=","+","[","]","{","}","\\","|"],
                        [";",":","'","\"",",",".","<",">","/","⌫"],
                        ["⇧","?","~","`"," ","€","£","¥","abc","Unlock"]]
                    readonly property var rows: sym ? symbols : letters
                    function label(k) { return (upper && k.length === 1 && /[a-z]/.test(k)) ? k.toUpperCase() : (k === " " ? "Space" : k); }
                    function press(k) {
                        if (k === "⌫") root.typed = root.typed.slice(0, -1);
                        else if (k === "⇧") upper = !upper;
                        else if (k === "#+=" ) sym = true;
                        else if (k === "abc") sym = false;
                        else if (k === "Unlock") root.tryUnlock();
                        else { root.typed += label(k) === "Space" ? " " : label(k); if (upper) upper = false; }
                    }
                    Keys.onPressed: e => {
                        if (e.key === Qt.Key_Left) c = Math.max(0, c - 1);
                        else if (e.key === Qt.Key_Right) c = Math.min(rows[r].length - 1, c + 1);
                        else if (e.key === Qt.Key_Up) r = Math.max(0, r - 1);
                        else if (e.key === Qt.Key_Down) r = Math.min(rows.length - 1, r + 1);
                        else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) {
                            // Moving around the keys: Enter presses the lit key. Typing: Enter unlocks.
                            if (navigating) press(rows[r][c]); else root.tryUnlock();
                        }
                        else if (e.key === Qt.Key_Backspace) root.typed = root.typed.slice(0, -1);
                        else if (e.key === Qt.Key_Escape) root.typed = "";
                        else if (e.text && e.text.length === 1 && e.text >= " ") root.typed += e.text;   // a real keyboard
                        else return;
                        if (e.key === Qt.Key_Left || e.key === Qt.Key_Right || e.key === Qt.Key_Up || e.key === Qt.Key_Down) navigating = true;
                        else if (e.key !== Qt.Key_Return && e.key !== Qt.Key_Enter) navigating = false;
                        e.accepted = true;
                    }
                    property bool navigating: false

                    Column {
                        anchors.fill: parent
                        spacing: 6
                        Repeater {
                            model: kb.rows
                            delegate: Row {
                                id: line
                                required property var modelData
                                required property int index
                                spacing: 6
                                Repeater {
                                    model: line.modelData
                                    delegate: Rectangle {
                                        id: key
                                        required property string modelData
                                        required property int index
                                        readonly property bool lit: kb.navigating && kb.r === line.index && kb.c === index
                                        width: (kb.width - 9 * 6) / 10; height: 48; radius: 10
                                        color: modelData === "Unlock" ? Theme.accent : (modelData === "⇧" && kb.upper) ? Theme.panel3 : tap.pressed ? Theme.panel3 : Theme.panel2
                                        border.width: 2; border.color: lit ? Theme.accent : "transparent"
                                        Text {
                                            anchors.centerIn: parent; text: kb.label(key.modelData)
                                            color: key.modelData === "Unlock" ? Theme.accentInk : Theme.text
                                            font.family: Theme.font; font.pixelSize: key.modelData.length > 1 ? 15 : 20; font.weight: Font.Medium
                                        }
                                        MouseArea { id: tap; anchors.fill: parent; onClicked: { kb.r = line.index; kb.c = key.index; kb.press(key.modelData); kb.forceActiveFocus(); } }
                                    }
                                }
                            }
                        }
                    }
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Type your password, or pick keys with the D-pad / right stick and A"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
            }
            Component.onCompleted: kb.forceActiveFocus()
        }
    }
}
