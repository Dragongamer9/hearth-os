// Mantel's lock screen on KWin (docs/KWIN-INPUT.md). KWin doesn't accept ext-session-lock-v1 (what Lock.qml's
// WlSessionLock needs on labwc), so on KWin the screen is locked by KWin's own locker, which is fail-safe: if this
// screen ever fails to load or crashes, KWin keeps everything covered ("the screen locker is broken"), it never
// opens by itself. This file is only what KWin's locker (kscreenlocker_greet) shows: the same look as Lock.qml
// (clock, date, password, its own on-screen keyboard for touch and the controller's D-pad and A).
// KWin's locker gives it `authenticator` (PAM): startAuthenticating(), respond(password), succeeded / failed.
// Plain QtQuick only (no Plasma or Kirigami), so it loads wherever kscreenlocker does. Tested in CI with
// kscreenlocker_greet --testing --theme <this package> (.github/workflows/kwin-input.yml).
import QtQuick
import QtQuick.Layouts

Item {
    id: root
    // Hearth's warm dark palette (Theme.qml's own defaults).
    readonly property color bg: "#0f0b09"
    readonly property color panel2: "#261d18"
    readonly property color panel3: "#33271f"
    readonly property color line: "#3b2e26"
    readonly property color text: "#f4ece3"
    readonly property color muted: "#a69484"
    readonly property color danger: "#ff5d5d"
    readonly property color accent: "#ff8a3d"
    readonly property color accentInk: "#1a0f08"
    readonly property string font: "Inter, Noto Sans, sans-serif"

    property string typed: ""
    property string message: ""
    property string pending: ""      // typed before PAM asked (it's handed over when it asks)
    readonly property bool busy: typeof authenticator !== "undefined" && authenticator && authenticator.busy === true
    readonly property string user: typeof kscreenlocker_userName !== "undefined" ? kscreenlocker_userName : ""

    // Greeter hooks KWin's locker may call (Breeze has them too); harmless if it doesn't.
    signal clearPassword()
    onClearPassword: typed = ""
    property bool viewVisible: true

    function hasAuth() { return typeof authenticator !== "undefined" && authenticator !== null; }
    function begin() {
        if (!hasAuth()) return;
        if (typeof authenticator.startAuthenticating === "function") authenticator.startAuthenticating();
    }
    function tryUnlock() {
        if (!hasAuth()) { message = "The lock screen can't check passwords here"; return; }
        message = "";
        const pw = typed;
        typed = "";
        if (typeof authenticator.respond === "function") {
            // Plasma 6: answer PAM's question; if it hasn't asked yet, keep the answer until it does.
            if (authenticator.promptForSecret) authenticator.respond(pw);
            else { pending = pw; begin(); }
        } else if (typeof authenticator.tryUnlock === "function") {
            authenticator.tryUnlock(pw);                 // Plasma 5's locker
        }
    }
    Connections {
        target: root.hasAuth() ? authenticator : null
        ignoreUnknownSignals: true
        function onPromptForSecretChanged() {
            if (root.pending !== "" && authenticator.promptForSecret) {
                const pw = root.pending;
                root.pending = "";
                authenticator.respond(pw);
            }
        }
        function onSucceeded() { Qt.quit(); }
        function onFailed(kind) {
            if (kind !== undefined && kind !== 0) return;   // fingerprint / smartcard: not used by Hearth
            root.message = "That password isn't right";
            root.pending = "";
            retry.restart();
        }
        function onErrorMessageChanged() { if (authenticator.errorMessage) root.message = authenticator.errorMessage; }
    }
    Timer { id: retry; interval: 400; onTriggered: root.begin() }
    Component.onCompleted: { begin(); kb.forceActiveFocus(); }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#17110e" }
            GradientStop { position: 0.7; color: "#0f0b09" }
            GradientStop { position: 1.0; color: "#2a140a" }
        }
    }
    Image {
        anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom
        width: parent.width * 1.2; height: parent.height * 0.6
        source: "file:///usr/share/nimbus/desktop/shell/images/ember-glow.png"
        opacity: 0.5
        asynchronous: true
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 760)
        spacing: 14

        Text {
            id: clock
            Layout.alignment: Qt.AlignHCenter
            color: root.text; font.family: root.font; font.pixelSize: 84; font.weight: Font.DemiBold
            text: Qt.formatTime(new Date(), "h:mm")
        }
        Text { id: date; Layout.alignment: Qt.AlignHCenter; color: root.muted; font.family: root.font; font.pixelSize: 20; text: Qt.formatDate(new Date(), "dddd, MMMM d") }
        Timer { interval: 5000; running: true; repeat: true
            onTriggered: { clock.text = Qt.formatTime(new Date(), "h:mm"); date.text = Qt.formatDate(new Date(), "dddd, MMMM d"); } }
        Text { Layout.alignment: Qt.AlignHCenter; visible: root.user !== ""; text: root.user; color: root.text; font.family: root.font; font.pixelSize: 17 }

        Rectangle {
            Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 18
            Layout.preferredWidth: 420; Layout.preferredHeight: 56; radius: 14
            color: root.panel2; border.color: root.message ? root.danger : root.line; border.width: 2
            Text {
                anchors.centerIn: parent
                text: root.busy && root.typed === "" ? "Checking…" : root.typed.length ? "•".repeat(root.typed.length) : "Password"
                color: root.typed.length ? root.text : root.muted; font.family: root.font; font.pixelSize: 22
            }
        }
        Text { Layout.alignment: Qt.AlignHCenter; visible: root.message !== ""; text: root.message; color: root.danger; font.family: root.font; font.pixelSize: 14 }

        // Its own keyboard: tap, or arrows / D-pad and Enter (the regular on-screen keyboard can't show here).
        Item {
            id: kb
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(root.width - 80, 760)
            Layout.preferredHeight: 4 * 54
            focus: true
            property bool upper: false
            property bool sym: false
            property bool navigating: false
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
                else if (k === "#+=") sym = true;
                else if (k === "abc") sym = false;
                else if (k === "Unlock") root.tryUnlock();
                else { root.typed += label(k) === "Space" ? " " : label(k); if (upper) upper = false; }
            }
            Keys.onPressed: e => {
                if (e.key === Qt.Key_Left) c = Math.max(0, c - 1);
                else if (e.key === Qt.Key_Right) c = Math.min(rows[r].length - 1, c + 1);
                else if (e.key === Qt.Key_Up) r = Math.max(0, r - 1);
                else if (e.key === Qt.Key_Down) r = Math.min(rows.length - 1, r + 1);
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { if (navigating) press(rows[r][c]); else root.tryUnlock(); }
                else if (e.key === Qt.Key_Backspace) root.typed = root.typed.slice(0, -1);
                else if (e.key === Qt.Key_Escape) root.typed = "";
                else if (e.text && e.text.length === 1 && e.text >= " ") root.typed += e.text;
                else return;
                if (e.key === Qt.Key_Left || e.key === Qt.Key_Right || e.key === Qt.Key_Up || e.key === Qt.Key_Down) navigating = true;
                else if (e.key !== Qt.Key_Return && e.key !== Qt.Key_Enter) navigating = false;
                e.accepted = true;
            }
            Column {
                anchors.fill: parent
                spacing: 6
                Repeater {
                    model: kb.rows
                    delegate: Row {
                        id: krow
                        required property var modelData
                        required property int index
                        spacing: 6
                        Repeater {
                            model: krow.modelData
                            delegate: Rectangle {
                                id: key
                                required property string modelData
                                required property int index
                                readonly property bool lit: kb.navigating && kb.r === krow.index && kb.c === index
                                width: (kb.width - 9 * 6) / 10; height: 48; radius: 10
                                color: modelData === "Unlock" ? root.accent : (modelData === "⇧" && kb.upper) ? root.panel3 : tap.pressed ? root.panel3 : root.panel2
                                border.width: 2; border.color: lit ? root.accent : "transparent"
                                Text {
                                    anchors.centerIn: parent; text: kb.label(key.modelData)
                                    color: key.modelData === "Unlock" ? root.accentInk : root.text
                                    font.family: root.font; font.pixelSize: key.modelData.length > 1 ? 15 : 20; font.weight: Font.Medium
                                }
                                MouseArea { id: tap; anchors.fill: parent; onClicked: { kb.r = krow.index; kb.c = key.index; kb.press(key.modelData); kb.forceActiveFocus(); } }
                            }
                        }
                    }
                }
            }
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "Type your password, or pick keys with the D-pad / right stick and A"
            color: root.muted; font.family: root.font; font.pixelSize: 13
        }
    }
}
