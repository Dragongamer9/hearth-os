// Settings › Accounts › Sign-in options: Hearth's own login screen (1.14, a test). A block of its own, so any page
// can hold it (SetSignin.qml today; the Users / Login screen pages later): the switch, what this start used and why,
// "Turn on again" after it paused itself, and Preview (the screen in a window here, signing in off).
// The switch runs `pkexec hearth-login-screen on|off` (an argument array; its polkit rule allows only those two
// words), which writes /etc/hearth/login-screen.json through hearth_settings_io. It takes effect at the next start;
// SDDM stays installed and comes back by itself whenever Hearth's doesn't work (hearth-login-screen explains how).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

ColumnLayout {
    id: block
    Layout.fillWidth: true
    spacing: 8
    readonly property string helper: "/usr/libexec/nimbus/hearth-login-screen"
    property var st: ({})
    property bool there: false
    property string busy: ""
    property string message: ""
    property bool bad: false
    readonly property bool on: !!st.on

    function load() { if (!status.running) status.running = true; }
    Component.onCompleted: load()
    Process {
        id: status
        command: ["sh", "-c", "[ -x \"$0\" ] || exit 3; exec \"$0\" status", block.helper]
        stdout: StdioCollector {
            onStreamFinished: {
                try { block.st = JSON.parse(text); block.there = true; } catch (e) { block.there = false; }
            }
        }
    }
    Process {
        id: setter
        property bool want: false
        stdout: StdioCollector {
            onStreamFinished: {
                const err = text.split("\n").find(l => l.startsWith("error: "));
                block.busy = "";
                if (err) { block.message = err.slice(7); block.bad = true; }
                else if (text.indexOf("\"ok\": true") >= 0) {
                    block.bad = false;
                    block.message = setter.want ? "Hearth's login screen shows from the next start. If it doesn't work, the usual one comes back by itself."
                        : "The usual login screen (SDDM) shows from the next start.";
                }
                block.load();
            }
        }
        onExited: code => { if (code === 126 || code === 127) { block.busy = ""; block.bad = true; block.message = "Not allowed (Hearth's polkit rule is missing)."; } }
    }
    function set(v) {
        if (setter.running) return;
        busy = "set"; message = ""; setter.want = v;
        setter.command = ["pkexec", helper, v ? "on" : "off"];
        setter.running = true;
    }
    Process { id: preview; command: [block.helper, "preview"] }

    SetHeader { text: "Login screen (test)"; visible: block.there }
    SetBanner {
        visible: block.there && block.on && !!block.st.tripped
        kind: "warn"
        title: "Hearth's login screen paused itself"
        sub: "It didn't work twice in a row" + (block.st.last && block.st.last.why ? " (" + block.st.last.why + ")" : "") + ", so the usual login screen shows at every start. Your setting is still on."
        action: "Turn on again"
        onActivated: block.set(true)
    }
    SetSwitch {
        visible: block.there
        icon: "user"
        title: "Hearth's own login screen"
        sub: block.busy === "set" ? "Saving…"
            : !block.on ? "A test, off by default. The usual login screen (SDDM) stays and comes back by itself if Hearth's doesn't work."
            : block.st.thisStart === "greetd" ? "On: in use at this start."
            : block.st.fellBack ? "On, but it didn't work at this start (" + (block.st.why || "") + "): the usual one was used."
            : !block.st.canStart ? "On, but it can't start here: " + (block.st.cannot || "something is missing") + "."
            : "On: shows from the next start."
        checked: block.on
        onToggled: v => block.set(v)
    }
    SetRow {
        visible: block.there
        icon: "display"
        title: "Preview"
        sub: "Hearth's login screen in a window, to see it (signing in is off there)"
        chevron: true
        onActivated: preview.running = true
    }
    Text {
        visible: block.message !== ""
        Layout.fillWidth: true
        text: block.message
        color: block.bad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
