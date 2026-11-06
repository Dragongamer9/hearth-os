// "Start in": what this PC opens when it turns on or you sign in (Game Mode, Desktop Mode, or Server mode once it's on:
// docs/SERVER-MODE.md). Its homes are Gaming and System › Startup (SetStartup.qml, with Server mode's schedule); Home's
// Recommended settings shows the same choice. Read from and written to ~/.config/nimbus/session.conf (NIMBUS_START_MODE),
// only when you pick one, through hearth_settings_io's conf-set (only that line; atomic, .bak, under the lock).
import QtQuick
import Quickshell
import Quickshell.Io

SetChoice {
    id: start
    readonly property string conf: Quickshell.env("HOME") + "/.config/nimbus/session.conf"
    readonly property string io: decodeURIComponent(Qt.resolvedUrl("hearth_settings_io.py").toString().replace(/^file:\/\//, ""))
    readonly property string server: decodeURIComponent(Qt.resolvedUrl("hearth-server-mode").toString().replace(/^file:\/\//, ""))
    property string startMode: "game"
    property bool serverOn: false
    icon: "power"
    title: "Start in"
    sub: "What this PC opens when it turns on or you sign in"
    options: [{ value: "game", label: "Game Mode" }, { value: "desktop", label: "Desktop Mode" }].concat(serverOn || startMode === "server" ? [{ value: "server", label: "Server mode" }] : [])
    current: start.startMode
    onChosen: v => start.setStartMode(v)
    Process {
        running: true
        command: ["sh", "-c", "sed -n 's/^[[:space:]]*\\(export[[:space:]]*\\)\\{0,1\\}NIMBUS_START_MODE=[\"'\\'']*\\([a-z]*\\).*/mode=\\2/p' \"$0\" 2>/dev/null | tail -n 1; python3 \"$1\" session-check 2>/dev/null | sed 's/^/server=/'", start.conf, start.server]
        stdout: StdioCollector { onStreamFinished: {
            const m = text.match(/^mode=(game|desktop|server)$/m); start.startMode = m ? m[1] : "game";
            start.serverOn = /^server=on$/m.test(text);
        } }
    }
    function setStartMode(m) {
        if (["game", "desktop", "server"].indexOf(m) < 0) return;
        startMode = m;
        Quickshell.execDetached(["python3", io, "conf-set", conf, "NIMBUS_START_MODE", m]);
    }
}
