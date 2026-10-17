// Hearth Settings › Sign-in: start in Game or Desktop Mode, sign in automatically, password and the
// device's name. The parts that need root go through hearth-admin (pkexec; Hearth's polkit rule lets the
// person at the device do this without a prompt).
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
    Component.onCompleted: load()
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
        // stdin through the environment (readable only by you), not the command line
        admin.environment = { HEARTH_INPUT: input || "" };
        admin.command = ["sh", "-c", 'printf "%s" "$HEARTH_INPUT" | exec pkexec "$@" 2>&1', "sh", helper].concat(args);
        admin.running = true;
    }
    function setStartMode(m) {
        startMode = m;
        Quickshell.execDetached(["sh", "-c", 'mkdir -p "$(dirname "$0")"; touch "$0"; if grep -q "^[[:space:]]*NIMBUS_START_MODE=" "$0"; then sed -i "s/^[[:space:]]*NIMBUS_START_MODE=.*/NIMBUS_START_MODE=$1/" "$0"; else echo "NIMBUS_START_MODE=$1" >> "$0"; fi', conf, m]);
    }

    SetChoice {
        title: "Start in"; sub: "What you see after turning on"
        options: [{ value: "game", label: "Game Mode" }, { value: "desktop", label: "Desktop Mode" }]
        current: page.startMode
        onChosen: v => page.setStartMode(v)
    }
    SetSwitch {
        visible: page.canAdmin
        icon: "user"; title: "Sign in automatically"; sub: page.busy === "autologin" ? "Saving…" : "Off shows the login screen at start, with your password"
        checked: page.autologin
        onToggled: v => page.runAdmin(["autologin", v ? "on" : "off"], "", "autologin", () => page.say(v ? "Hearth will sign you in automatically." : "The login screen will show at start."))
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

    SetHeader { text: "Device name"; visible: page.canAdmin }
    SetField { id: host; visible: page.canAdmin; placeholder: page.hostname || "Device name"; onSubmitted: hostBtn.activated() }
    SetRow {
        id: hostBtn
        visible: page.canAdmin
        icon: "display"; title: page.busy === "hostname" ? "Saving…" : "Rename"; sub: "Now: " + page.hostname; chevron: false
        onActivated: { if (host.text.trim()) page.runAdmin(["hostname", host.text.trim()], "", "hostname", () => { host.text = ""; page.say("Renamed."); }); }
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.topMargin: 6
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !page.canAdmin
        Layout.fillWidth: true
        text: "Automatic sign-in, password and device name are set here on Hearth, the OS."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
