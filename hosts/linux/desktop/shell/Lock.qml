// The lock screen for Desktop Mode (Mantel): Super+L, "Lock" in the power menu, or by itself when the device
// wakes from sleep (Hearth Settings › Sign-in › Lock when waking). Covers every screen (the compositor keeps
// everything else hidden until it's unlocked) and checks your password with the system (PAM), like the login
// screen. It has its own keyboard on screen (tap, or move with the arrow keys / D-pad and press Enter), since
// the regular on-screen keyboard can't show over a locked screen. On KWin it hands locking to KWin's locker with
// Hearth's own lock screen in it (desktop/kwin-lock, docs/KWIN-INPUT.md).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pam

Scope {
    id: root
    property bool locked: false
    onLockedChanged: {
        if (locked && onKwin) lockedAt = new Date();   // lockedAt: KWin's locker said so
        DeskState.locked = locked;
        // logind's LockedHint (what Phone link and other apps read to know it's locked; KWin's locker sets its own).
        // Only a hint: whatever happens to it, locking itself doesn't depend on it.
        if (!onKwin) Quickshell.execDetached(["busctl", "call", "--system", "org.freedesktop.login1", "/org/freedesktop/login1/session/auto",
                                              "org.freedesktop.login1.Session", "SetLockedHint", "b", locked ? "true" : "false"]);
        // 1.14: vaults set to lock with the screen are locked (hearth-vaults), and the fingerprint (if you turned it on)
        // is asked for next to the password. Leaving the lock screen stops the fingerprint.
        if (locked) { vaultLock.running = false; vaultLock.running = true; fpTries = 0; fpInfo = ""; if (!onKwin) { fpCheck.running = false; fpCheck.running = true; } }
        else {
            if (fpPam.active) fpPam.abort();
            // A vault that couldn't lock with the screen (files open in it): said once you're back, so it's not a surprise.
            if (vaultProblems.length) {
                const t = vaultProblems.join(" ");
                vaultProblems = [];
                Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "dialog-warning", "A vault stayed unlocked", t + " Lock it from the bar's vault button or Settings › Vaults."]);
            }
        }
    }
    property var vaultProblems: []     // what hearth-vaults lock-all --screen couldn't lock
    readonly property string vaultsHelper: decodeURIComponent(Qt.resolvedUrl("hearth-vaults").toString().replace(/^file:\/\//, ""))
    Process {
        id: vaultLock
        command: ["sh", "-c", "[ -e \"$HOME/.config/nimbus/vaults.json\" ] || exit 0; exec python3 \"$0\" lock-all --screen", root.vaultsHelper]
        stdout: StdioCollector {
            onStreamFinished: {
                try { const o = JSON.parse(text.trim().split("\n").pop() || "{}"); root.vaultProblems = (o && Array.isArray(o.problems)) ? o.problems : []; }
                catch (e) { root.vaultProblems = []; }
            }
        }
    }

    // ---- Fingerprint (1.14, docs/FINGERPRINT.md): a second PAM conversation (service hearth-fingerprint) NEXT TO the
    // password one, never in its place: the password always unlocks. Only when your name is in
    // /etc/nimbus/fingerprint/lock (Settings › Sign-in options); a few tries, then the reader rests until the next lock.
    property bool fpOn: false
    property string fpInfo: ""
    property int fpTries: 0
    Process {
        id: fpCheck
        command: ["sh", "-c", "grep -qx -- \"$1\" \"$0\" 2>/dev/null && command -v fprintd-list >/dev/null && echo on", "/etc/nimbus/fingerprint/lock", Quickshell.env("USER") || ""]
        stdout: StdioCollector {
            onStreamFinished: {
                root.fpOn = text.trim() === "on";
                if (root.fpOn && root.locked && !root.onKwin) root.startFp();
            }
        }
    }
    function startFp() {
        if (fpPam.active || !locked) return;
        fpInfo = "Or touch the fingerprint reader";
        fpPam.start();
    }
    Timer { id: fpRetry; interval: 1500; onTriggered: root.startFp() }
    PamContext {
        id: fpPam
        config: "hearth-fingerprint"
        onPamMessage: {
            if (fpPam.responseRequired) fpPam.respond("");      // pam_fprintd asks nothing; never the password here
            else if (fpPam.message !== "") root.fpInfo = /place|swipe|touch/i.test(fpPam.message) ? "Or touch the fingerprint reader" : fpPam.message;
        }
        onCompleted: result => {
            if (result === PamResult.Success) { root.locked = false; root.typed = ""; root.fpInfo = ""; return; }
            root.fpTries++;
            root.fpInfo = root.fpTries < 3 ? "That finger wasn't recognised. Try again, or type your password" : "Type your password (the reader rests until the next lock)";
            if (root.fpTries < 3 && root.locked) fpRetry.start();
        }
        onError: root.fpInfo = ""
    }
    property string typed: ""
    property string message: ""
    property bool busy: false
    // On KWin (HEARTH_WM=kwin) there's no ext-session-lock-v1, so WlSessionLock would cover nothing: KWin's own
    // locker locks instead (fail-safe: it stays covered even if its screen crashes), showing Hearth's lock screen
    // (desktop/kwin-lock). `locked` then follows KWin's locker (org.freedesktop.ScreenSaver ActiveChanged).
    readonly property bool onKwin: Quickshell.env("HEARTH_WM") === "kwin"

    property var lockedAt: new Date()     // when it locked (system-settings); KWin's locker: when it said so
    function lock() {
        if (onKwin) { kwinLock.running = false; kwinLock.running = true; return; }
        if (!locked) { typed = ""; message = ""; lockedAt = new Date(); locked = true; }
    }

    // KWin: first hearth-lockcheck (is KWin's locker there, and will it show Hearth's lock screen? If it couldn't, KWin
    // would keep the screen covered by its emergency screen: on a handheld without a keyboard, a lockout). Only then
    // lock through KWin's locker, and check it really locked. Either failing: say so loudly, never a silent "locked".
    readonly property string lockcheck: decodeURIComponent(Qt.resolvedUrl("hearth-lockcheck").toString().replace(/^file:\/\//, ""))
    Process {
        id: kwinLock
        command: ["sh", "-c", "out=$(python3 \"$0\") || { echo \"nolock $out\"; exit 0; }; gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.Lock >/dev/null 2>&1; for i in 1 2 3 4 5 6 7 8 9 10; do gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.GetActive 2>/dev/null | grep -q true && { echo locked; exit 0; }; sleep 0.5; done; echo failed", root.lockcheck]
        stdout: SplitParser {
            onRead: l => {
                const line = l.trim();
                if (line === "locked") { root.locked = true; return; }
                root.locked = false;
                let why = "KWin's screen locker didn't answer";
                if (line.indexOf("nolock ") === 0) {
                    try { why = JSON.parse(line.slice(7)).why || why; } catch (e) { why = "the lock screen isn't ready"; }
                    why += ". Hearth didn't lock, so you can't be locked out";
                }
                console.warn("Lock: not locked: " + why);
                SettingsNav.toast("The screen isn't locked: " + why);
                Quickshell.execDetached(["notify-send", "-u", "critical", "-a", "Hearth", "The screen isn't locked",
                    why + ". Nothing is covering your desktop. Try Lock again, or sign out."]);
            }
        }
    }
    // Warms hearth-lockcheck's test of the lock screen at start, so the first lock doesn't wait for it.
    Process { running: root.onKwin; command: ["python3", root.lockcheck] }
    Process {
        running: root.onKwin
        command: ["gdbus", "monitor", "--session", "--dest", "org.freedesktop.ScreenSaver", "--object-path", "/ScreenSaver"]
        stdout: SplitParser {
            onRead: l => {
                if (l.indexOf("ActiveChanged (true") >= 0) root.locked = true;
                else if (l.indexOf("ActiveChanged (false") >= 0) root.locked = false;
            }
        }
    }
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

    // `loginctl lock-session` (and apps that ask the system to lock, like a password manager's "lock now"): this
    // session's Lock signal from logind.
    Process {
        running: true
        command: ["sh", "-c", "p=$(busctl --system call org.freedesktop.login1 /org/freedesktop/login1 org.freedesktop.login1.Manager GetSession s \"${XDG_SESSION_ID:-auto}\" 2>/dev/null | cut -d' ' -f2 | tr -d '\"'); [ -n \"$p\" ] && exec gdbus monitor --system --dest org.freedesktop.login1 --object-path \"$p\""]
        stdout: SplitParser { onRead: l => { if (l.indexOf(".Session.Lock (") >= 0) root.lock(); } }
    }

    WlSessionLock {
        id: session
        locked: root.locked && !root.onKwin

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
                // Notifications from apps you let show here (Settings › Notifications, 1.14; none unless you turn it on
                // for an app): the three newest since the screen locked, the app's name and the title only.
                Repeater {
                    model: root.locked ? DeskState.history.filter(h => h.lock && h.time >= root.lockedAt).slice(0, 3) : []
                    delegate: Rectangle {
                        required property var modelData
                        Layout.alignment: Qt.AlignHCenter
                        Layout.preferredWidth: 420; implicitHeight: 52; radius: 12
                        color: Theme.panel2; border.color: Theme.line
                        Column {
                            anchors.verticalCenter: parent.verticalCenter; anchors.left: parent.left; anchors.leftMargin: 14
                            width: parent.width - 28
                            Text { text: modelData.app; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight; width: parent.width }
                            Text { text: modelData.summary; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight; width: parent.width }
                        }
                    }
                }

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
                Row {
                    Layout.alignment: Qt.AlignHCenter
                    visible: root.fpInfo !== ""
                    spacing: 8
                    Icon { anchors.verticalCenter: parent.verticalCenter; name: "fingerprint"; size: 20; color: Theme.accent }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: root.fpInfo; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                }

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
