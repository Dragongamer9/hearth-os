// SSH and GPG passphrases asked in Hearth's look (1.14): hearth-askpass (SSH_ASKPASS: ssh, ssh-add, git over ssh) and
// pinentry-hearth (GnuPG) ask here through IPC, with a private FIFO for the answer (hearth_askpass.py). The prompt looks
// like Hearth's password prompt (PolkitPrompt.qml): who's asking, what for, the field, OK / Cancel; controller
// friendly (A OK, B cancel, Y the on-screen keyboard in password mode).
//
// The passphrase goes from the field straight into that FIFO (only when it IS a FIFO in your runtime folder's
// hearth-askpass/) and the field is emptied: never logged, stored or sent anywhere else. When Mantel isn't running,
// the helpers ask the classic way (ksshaskpass, pinentry-qt…), so a passphrase can always be typed.
//   quickshell ipc -p <shell folder> call askpass ping | demo | demoSsh | close
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: ap
    property var queue: []                 // [{reply, kind, title, desc, prompt}]
    readonly property var cur: queue.length ? queue[0] : null
    readonly property bool active: cur !== null
    readonly property string dir: (Quickshell.env("XDG_RUNTIME_DIR") || "/run/user/0") + "/hearth-askpass/"
    readonly property bool gpg: !!cur && /gnupg|gpg|pgp/i.test(cur.title)
    readonly property bool secret: !!cur && cur.kind === "secret"
    readonly property bool asksText: !!cur && (cur.kind === "secret" || cur.kind === "visible")
    property var promptScreen: null

    function validReply(p) { return typeof p === "string" && p.indexOf(dir) === 0 && /^[0-9a-f]{24}$/.test(p.slice(dir.length)); }
    // What the helpers send: the question's text (gpg-agent's has line breaks), never a secret.
    function add(reply, kind, title, desc, prompt) {
        if (reply !== "" && !validReply(reply)) { console.warn("AskPass: a question without a private reply place, ignored"); return; }
        queue = queue.concat([{ reply: reply, kind: ["secret", "visible", "confirm", "message"].indexOf(kind) >= 0 ? kind : "secret",
                                title: title || "", desc: desc || "", prompt: prompt || "" }]);
    }
    onActiveChanged: {
        if (active) { const t = WinList.activeToplevel; promptScreen = (t && t.screens && t.screens.length ? t.screens[0] : null) || Quickshell.screens[0]; }
        const pad = SettingsNav.pad;
        if (!active && pad) pad.closeKeyboard();
        else if (active && pad && pad.keyboard && pad.keyboard.open) pad.openKeyboard(secret ? "password" : "text");
    }
    // The controller types keys into it like Hearth's password prompt (Pad.qml: A Enter, B Esc, Y the keyboard).
    Binding { target: SettingsNav; property: "polkitShown"; value: true; when: ap.active }
    signal clearField()
    // Answer the question in front: "ok\n<text>" or "cancel\n" into its FIFO, then the next one.
    function answer(ok, text) {
        const q = cur;
        clearField();
        queue = queue.slice(1);
        if (!q || q.reply === "") return;
        const w = writer.createObject(ap, { reply: q.reply, payload: (ok ? "ok\n" + (text || "") : "cancel\n") });
        w.running = true;
    }
    Component {
        id: writer
        Process {
            id: wp
            property string reply
            property string payload
            // Only into a FIFO (the helper's), never a file: a helper that gave up has removed it already.
            command: ["sh", "-c", "[ -p \"$0\" ] || { cat >/dev/null; exit 0; }; exec cat > \"$0\"", reply]
            stdinEnabled: true
            onStarted: { wp.write(wp.payload); wp.payload = ""; wp.stdinEnabled = false; }
            onExited: wp.destroy()
        }
    }

    IpcHandler {
        target: "askpass"
        function ping(): string { return "pong"; }
        function ask(reply: string, kind: string, title: string, desc: string, prompt: string): void { ap.add(reply, kind, title, desc, prompt); }
        function demo(): void { ap.add("", "secret", "GnuPG", "Please enter the passphrase to unlock the OpenPGP secret key:\n\"Sam Fireside <sam@example.com>\"\n255-bit EDDSA key, ID 5A3F 91C2 0B7E 44D1,\ncreated 2026-03-02.", "Passphrase:"); }
        function demoSsh(): void { ap.add("", "secret", "SSH", "Enter passphrase for key '/home/sam/.ssh/id_ed25519':", "Passphrase"); }
        function close(): void { while (ap.active) ap.answer(false, ""); }
    }

    LazyLoader {
        active: ap.active
        PanelWindow {
            id: win
            screen: ap.promptScreen || Quickshell.screens[0]
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-askpass"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            color: "#b3060809"
            Component.onCompleted: focusLater.start()
            Timer { id: focusLater; interval: 50; onTriggered: { if (ap.asksText) field.forceActiveFocus(); else okBtn.forceActiveFocus(); } }
            Connections { target: ap; function onClearField() { field.text = ""; showBtn.reveal = false; focusLater.restart(); } }
            MouseArea { anchors.fill: parent }

            Rectangle {
                id: card
                anchors.horizontalCenter: parent.horizontalCenter
                y: Math.max(24, parent.height * 0.14)
                width: Math.min(parent.width - 32, 620)
                height: col.implicitHeight + 56
                radius: Math.max(10, Theme.r(22))
                color: Theme.panel
                border.color: Theme.line
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 24; rightMargin: 24 } height: 3; radius: 2; color: Theme.accent }
                ColumnLayout {
                    id: col
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
                    spacing: 16
                    RowLayout {
                        spacing: 14
                        Rectangle {
                            implicitWidth: 52; implicitHeight: 52; radius: 14
                            color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
                            Icon { anchors.centerIn: parent; name: ap.gpg ? "key" : "lock"; size: 28; color: Theme.accent }
                        }
                        ColumnLayout {
                            spacing: 3
                            Text { text: ap.gpg ? "GnuPG is asking" : (ap.cur && ap.cur.title === "SSH" ? "SSH is asking" : (ap.cur ? ap.cur.title : "") + " is asking"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.Medium }
                            Text {
                                text: !ap.cur ? "" : ap.cur.kind === "confirm" ? "Are you sure?" : ap.cur.kind === "message" ? "A message" : ap.gpg ? "Your key's passphrase" : ap.cur.kind === "visible" ? "An answer" : "Your key's passphrase"
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold
                            }
                        }
                    }
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        visible: text !== ""
                        text: ap.cur ? ap.cur.desc : ""
                        color: Theme.text; opacity: 0.85; font.family: Theme.font; font.pixelSize: 14
                    }
                    ColumnLayout {
                        visible: ap.asksText
                        Layout.fillWidth: true
                        spacing: 6
                        Text { text: ap.cur ? (ap.cur.prompt || "Passphrase").replace(/:\s*$/, "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 50; radius: 10
                            color: Theme.bg2
                            border.width: field.activeFocus ? 2 : 1; border.color: field.activeFocus ? Theme.accent : Theme.line
                            TextInput {
                                id: field
                                anchors { left: parent.left; right: showBtn.left; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 8 }
                                echoMode: !ap.secret || showBtn.reveal ? TextInput.Normal : TextInput.Password
                                passwordCharacter: "•"
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 18
                                clip: true
                                inputMethodHints: ap.secret ? (Qt.ImhHiddenText | Qt.ImhSensitiveData | Qt.ImhNoPredictiveText) : Qt.ImhNoPredictiveText
                                onAccepted: ap.answer(true, field.text)
                                Keys.onEscapePressed: ap.answer(false, "")
                                Keys.onDownPressed: okBtn.forceActiveFocus()
                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    visible: field.text === ""
                                    text: (ap.secret ? "Type the passphrase" : "Type your answer") + "  ·  Y opens the keyboard"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                                }
                            }
                            CaptureBtn {
                                id: showBtn
                                property bool reveal: false
                                anchors { right: parent.right; rightMargin: 6; verticalCenter: parent.verticalCenter }
                                visible: ap.secret
                                icon: "eye"; tip: reveal ? "Hide it" : "Show it"; selected: reveal
                                arrowsMove: false
                                onActivated: { reveal = !reveal; field.forceActiveFocus(); }
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10
                        CaptureBtn { visible: ap.asksText; icon: "kbd"; text: "Keyboard"; onActivated: { field.forceActiveFocus(); if (SettingsNav.pad) SettingsNav.pad.openKeyboard(ap.secret ? "password" : "text"); } }
                        Item { Layout.fillWidth: true }
                        CaptureBtn { visible: !ap.cur || ap.cur.kind !== "message"; text: "Cancel"; onActivated: ap.answer(false, "") }
                        CaptureBtn { id: okBtn; primary: true; icon: ap.asksText ? "lock" : "check"; text: ap.cur && ap.cur.kind === "confirm" ? "Yes" : "OK"; onActivated: ap.answer(true, field.text); Keys.onEscapePressed: ap.answer(false, "") }
                    }
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: "A OK  ·  B cancel" + (ap.asksText ? "  ·  Y keyboard" : "") + "      Hearth never keeps what you type here"
                        color: Theme.muted; opacity: 0.8; font.family: Theme.font; font.pixelSize: 11
                    }
                }
                Keys.onEscapePressed: ap.answer(false, "")
            }
        }
    }
}
