// One change in the history (1.14): "Accent colour → Ember orange", "you · 2 min ago", and Undo, which puts the old
// value back unless it was changed again since (then it says so and changes nothing).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "settings-changes.js" as Ch

SetRow {
    id: row
    required property var change
    signal undone()
    readonly property var accents: (Theme.palettes.accents || [])
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth_settings_io.py").toString().replace(/^file:\/\//, ""))
    flat: true
    chevron: false
    icon: ""
    title: Ch.line(change, accents)
    sub: (change.who || "you") + " · " + Ch.ago(change.at) + "  (was " + Ch.value(change.key, change.from, accents) + ")"
    trailing: 90
    onActivated: undo.trigger()
    Accessible.name: title + ", " + sub + ". Undo"
    SetButton {
        id: undo
        anchors { right: parent.right; rightMargin: SetStyle.pad; verticalCenter: parent.verticalCenter }
        ghost: true
        text: proc.running ? "…" : "Undo"
        function trigger() { if (!proc.running) { proc.command = ["python3", row.helper, "undo", String(row.change.id)]; proc.running = true; } }
        onActivated: trigger()
    }
    Process {
        id: proc
        stdout: StdioCollector {
            onStreamFinished: {
                const r = text.trim();
                SettingsNav.toast(r === "ok" ? Ch.name(row.change.key) + " is back to " + Ch.value(row.change.key, row.change.from, row.accents)
                    : r === "changed" ? Ch.name(row.change.key) + " was changed again since: nothing undone"
                    : "That change isn't in the history any more");
                row.undone();
            }
        }
    }
}
