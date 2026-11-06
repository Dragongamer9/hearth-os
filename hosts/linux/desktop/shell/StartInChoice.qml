// "Start in": what this PC opens when it turns on or you sign in (Game Mode or Desktop Mode). Its one real home is
// Gaming; Home's Recommended settings shows the same choice. Moved here from Settings › Sign-in as it was: read from
// and written to ~/.config/nimbus/session.conf (NIMBUS_START_MODE), only when you pick one.
import QtQuick
import Quickshell
import Quickshell.Io

SetChoice {
    id: start
    readonly property string conf: Quickshell.env("HOME") + "/.config/nimbus/session.conf"
    property string startMode: "game"
    icon: "power"
    title: "Start in"
    sub: "What this PC opens when it turns on or you sign in"
    options: [{ value: "game", label: "Game Mode" }, { value: "desktop", label: "Desktop Mode" }]
    current: start.startMode
    onChosen: v => start.setStartMode(v)
    Process {
        running: true
        command: ["sh", "-c", "sed -n 's/^[[:space:]]*NIMBUS_START_MODE=[\"'\\'']*\\([a-z]*\\).*/mode=\\1/p' \"$0\" 2>/dev/null", start.conf]
        stdout: StdioCollector { onStreamFinished: { const m = text.match(/^mode=(game|desktop)$/m); start.startMode = m ? m[1] : "game"; } }
    }
    function setStartMode(m) {
        startMode = m;
        Quickshell.execDetached(["sh", "-c", 'mkdir -p "$(dirname "$0")"; touch "$0"; if grep -q "^[[:space:]]*NIMBUS_START_MODE=" "$0"; then sed -i "s/^[[:space:]]*NIMBUS_START_MODE=.*/NIMBUS_START_MODE=$1/" "$0"; else echo "NIMBUS_START_MODE=$1" >> "$0"; fi', conf, m]);
    }
}
