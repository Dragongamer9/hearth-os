// Hearth Settings › Accounts › Sign-in options: sign in automatically, lock after sleep, password, and (1.14) your
// fingerprint: add and forget fingers (fprintd, hearth-fingerprint) and where it's used: the lock screen and
// administrator prompts (hearth-admin fingerprint-places; docs/FINGERPRINT.md). A fingerprint is only ever NEXT TO
// your password: the password always works, everywhere. ("Start in" moved to
// Gaming, StartInChoice.qml; the device's name to Home, which has Rename.)
// No desktop choice: Mantel is the only desktop. The parts that need root go through hearth-admin (pkexec; Hearth's
// polkit rule lets the person at the device do this without a prompt).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Sign-in"
    readonly property string helper: "/usr/libexec/nimbus/hearth-admin"
    readonly property string conf: Quickshell.env("HOME") + "/.config/nimbus/session.conf"
    property bool canAdmin: false
    property bool autologin: true
    property string startMode: "game"
    property string hostname: ""
    property string busy: ""
    property string message: ""
    property bool messageBad: false

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { info.running = true; }
    property var fp: null                 // hearth-fingerprint status
    property string fpMessage: ""
    property bool fpBad: false
    property string fpConfirm: ""
    readonly property var fingerNames: ({ "right-index-finger": "Right index finger", "left-index-finger": "Left index finger",
        "right-thumb": "Right thumb", "left-thumb": "Left thumb", "right-middle-finger": "Right middle finger",
        "left-middle-finger": "Left middle finger", "right-ring-finger": "Right ring finger", "left-ring-finger": "Left ring finger",
        "right-little-finger": "Right little finger", "left-little-finger": "Left little finger" })
    readonly property var freeFingers: Object.keys(fingerNames).filter(f => !fp || fp.fingers.indexOf(f) < 0)
    function fpSay(t, bad) { fpMessage = t; fpBad = !!bad; }
    function fpLoad() { fpJob.run(["status"], r => page.fp = r, e => page.fpSay(e, true)); }
    function enroll(f) { fpSay(""); fpJob.run(["enroll", f], r => { page.fp = r; page.fpSay((page.fingerNames[f] || f) + " is saved. Touch the reader to unlock where it's turned on below."); }, e => { page.fpSay(e, true); page.fpLoad(); }, "", "enroll"); }
    function places(req, ok) { fpSay(""); fpAdmin.run(["fingerprint-places"], () => { page.fpSay(ok); page.fpLoad(); }, e => { page.fpSay(e, true); page.fpLoad(); }, JSON.stringify(req), "places"); }
    Component.onCompleted: { load(); fpLoad(); if (SettingsNav.pageAnchor === "enroll") { SettingsNav.pageAnchor = ""; enrollSoon.start(); } }
    Timer { id: enrollSoon; interval: 1500; onTriggered: if (page.freeFingers.length) page.enroll(page.freeFingers[0]) }
    SetJob { id: fpJob; helper: "hearth-fingerprint" }
    SetJob { id: fpAdmin; helper: page.helper; admin: true }
    Process {
        id: info
        command: ["sh", "-c", "[ -x \"$0\" ] && echo admin=1; [ -e /etc/nimbus/no-autologin ] && echo autologin=0; echo host=$(cat /etc/hostname 2>/dev/null || hostname); sed -n 's/^[[:space:]]*NIMBUS_START_MODE=[\"'\\'']*\\([a-z]*\\).*/mode=\\1/p' \"$1\" 2>/dev/null", page.helper, page.conf]
        stdout: StdioCollector {
            onStreamFinished: {
                page.canAdmin = /^admin=1$/m.test(text);
                page.autologin = !/^autologin=0$/m.test(text);
                const h = text.match(/^host=(.*)$/m); page.hostname = h ? h[1].trim() : "";
                const m = text.match(/^mode=(game|desktop)$/m); page.startMode = m ? m[1] : "game";
            }
        }
    }
    Process {
        id: admin
        property var after: null
        property string input: ""
        // On stdin, then stdin closes (see SetJob.qml: Process.environment can't take a JavaScript object on
        // Hearth's Debian, so changing your password sent nothing and failed).
        stdinEnabled: admin.input !== ""
        onStarted: { if (admin.input !== "") { admin.write(admin.input); admin.input = ""; } }
        stdout: StdioCollector {
            onStreamFinished: {
                const err = text.split("\n").find(l => l.startsWith("error: "));
                const cb = admin.after; admin.after = null; page.busy = "";
                if (err) page.say(err.slice(7), true); else if (cb) cb();
                page.load();
            }
        }
        onExited: code => { if (code === 126 || code === 127) page.say("Not allowed (Hearth's polkit rule is missing).", true); }
    }
    function runAdmin(args, input, what, done) {
        if (admin.running) return;
        busy = what; say("");
        admin.after = done;
        // what the helper needs goes on its stdin, never the command line
        admin.input = input || "";
        admin.command = ["sh", "-c", 'exec pkexec "$@" 2>&1', "sh", helper].concat(args);
        admin.running = true;
    }

    SetHeader { text: "Signing in"; Layout.topMargin: 0 }
    SetSwitch {
        visible: page.canAdmin
        icon: "user"; title: "Sign in automatically"; sub: page.busy === "autologin" ? "Saving…" : "Off shows the login screen at start, with your password"
        checked: page.autologin
        onToggled: v => page.runAdmin(["autologin", v ? "on" : "off"], "", "autologin", () => page.say(v ? "Hearth will sign you in automatically." : "The login screen will show at start."))
    }

    // Hearth's own login screen (1.14, a test): its own block (LoginScreenSwitch.qml), so the Users / Login screen pages can take it as it is.
    LoginScreenSwitch { visible: page.canAdmin }

    SetSwitch {
        icon: "lock"; title: "Ask for my password after sleep"; sub: "Locked when it wakes, on the desktop and in Game Mode. Lock any time with Super+L or the power menu."
        checked: Settings.lockOnWake
        onToggled: v => Settings.lockOnWake = v
    }

    SetHeader { text: "Password"; visible: page.canAdmin }
    SetField { id: oldPw; visible: page.canAdmin; placeholder: "Current password"; secret: true }
    SetField { id: newPw; visible: page.canAdmin; placeholder: "New password (at least 4 characters)"; secret: true }
    SetField { id: newPw2; visible: page.canAdmin; placeholder: "New password again"; secret: true; onSubmitted: pwBtn.activated() }
    SetRow {
        id: pwBtn
        visible: page.canAdmin
        icon: "lock"; title: page.busy === "password" ? "Changing…" : "Change password"; chevron: false
        onActivated: {
            if (newPw.text.length < 4) return page.say("The new password needs at least 4 characters.", true);
            if (newPw.text !== newPw2.text) return page.say("The new passwords don't match.", true);
            page.runAdmin(["password"], JSON.stringify({ old: oldPw.text, new: newPw.text }) + "\n", "password", () => { oldPw.text = newPw.text = newPw2.text = ""; page.say("Password changed."); });
        }
    }

    // ---- Fingerprint (1.14)
    SetHeader { text: "Fingerprint" }
    SetRow {
        interactive: false; chevron: false; icon: "fingerprint"
        title: !page.fp ? "Looking for a fingerprint reader…" : !page.fp.fprintd ? "Fingerprint sign-in isn't on this system" : !page.fp.reader ? "No fingerprint reader found" : page.fp.device || "Fingerprint reader"
        sub: !page.fp ? "" : !page.fp.fprintd ? "It needs fprintd (in Hearth OS's image)" : !page.fp.reader ? "Plug one in, or this PC doesn't have one" : page.fp.fingers.length ? page.fp.fingers.length + (page.fp.fingers.length === 1 ? " finger saved" : " fingers saved") : "No fingers saved yet"
    }
    Repeater {
        model: page.fp && page.fp.reader ? page.fp.fingers : []
        delegate: ColumnLayout {
            id: fr
            required property string modelData
            Layout.fillWidth: true
            spacing: 6
            SetRow {
                icon: "fingerprint"; title: page.fingerNames[fr.modelData] || fr.modelData; chevron: false
                sub: "Saved on the reader"
                onActivated: page.fpConfirm = page.fpConfirm === fr.modelData ? "" : fr.modelData
            }
            SetRow {
                visible: page.fpConfirm === fr.modelData
                Layout.leftMargin: 52
                icon: "trash"; danger: true; chevron: false; title: "Forget this finger"
                onActivated: fpJob.run(["delete", fr.modelData], r => { page.fp = r; page.fpConfirm = ""; page.fpSay("Forgotten."); }, e => page.fpSay(e, true), "", "delete")
            }
        }
    }
    SetDropdown {
        id: fingerPick
        visible: !!page.fp && page.fp.reader && page.freeFingers.length > 0 && fpJob.what !== "enroll"
        icon: "plus"; title: "Add a finger"
        options: page.freeFingers.map(f => ({ value: f, label: page.fingerNames[f] }))
        current: page.freeFingers.length ? page.freeFingers[0] : ""
        onChosen: v => current = v
    }
    SetRow {
        visible: fingerPick.visible
        icon: "fingerprint"; chevron: false; title: "Start"
        sub: "Then touch the reader again and again with that finger until it's done"
        onActivated: page.enroll(fingerPick.current)
    }
    // Adding: the reader's own words and how far it is.
    ColumnLayout {
        visible: fpJob.what === "enroll"
        Layout.fillWidth: true; Layout.leftMargin: 4
        spacing: 8
        Text { text: fpJob.line || "Touch the fingerprint reader"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; Layout.fillWidth: true; wrapMode: Text.WordWrap }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 10; radius: 5; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, fpJob.progress); height: parent.height; radius: 5; color: Theme.accent; Behavior on width { NumberAnimation { duration: 250 } } }
        }
    }
    SetSwitch {
        visible: !!page.fp && page.fp.reader && page.fp.fingers.length > 0
        icon: "lock"; title: "Unlock the lock screen with a finger"
        sub: !page.fp || page.fp.pam.lock ? "Your password works there too, always" : "Needs Hearth's lock screen file (hearth-fingerprint), missing here"
        checked: !!page.fp && page.fp.places.lock
        onToggled: v => page.places({ lock: v }, v ? "Touch the reader to unlock. Your password still works." : "The lock screen asks for your password only.")
    }
    SetSwitch {
        visible: !!page.fp && page.fp.reader && page.fp.fingers.length > 0
        icon: "shield"; title: "Use it for administrator prompts"
        sub: !page.fp || page.fp.pam.polkit ? "When an app asks for your password: touch the reader, or wait a moment and type it" : "Needs Hearth's polkit setup, missing on this system"
        checked: !!page.fp && page.fp.places.polkit
        onToggled: v => page.places({ polkit: v }, v ? "Administrator prompts take your finger too." : "Administrator prompts ask for your password only.")
    }
    Text {
        Layout.fillWidth: true
        visible: !!page.fp && page.fp.reader
        text: "A fingerprint is never the only way in: your password always works. Signing in at the login screen still takes your password (it also unlocks your saved passwords)."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.fpMessage !== ""
        Layout.fillWidth: true; Layout.topMargin: 4
        text: page.fpMessage; color: page.fpBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Moved" }
    SetRow { chevron: true; icon: "power"; title: "Start in Game Mode or Desktop Mode"; sub: "Now in Gaming › Start in"; onActivated: SettingsNav.openPage("gaming") }
    SetRow { chevron: true; icon: "display"; title: "Device name"; sub: "Rename this PC on Home" + (page.hostname ? " · now: " + page.hostname : ""); onActivated: SettingsNav.openPage("home") }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.topMargin: 6
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !page.canAdmin
        Layout.fillWidth: true
        text: "Automatic sign-in and your password are set here on Hearth, the OS."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
