// Colour picker widget (1.14): Pick, then click anywhere on the screen (KWin's own picker under KWin, slurp + grim on
// labwc: hearth-widgets pick-colour). The colour shows big with its hex, RGB and HSL; click one to copy it. The last
// eight you picked are kept (widgets.json, only when you pick one). In the bar: a swatch of the last one; click picks.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "widget-tools.js" as T

WidgetBase {
    id: w
    readonly property var picked: list("colours") || []
    readonly property string cur: picked.length ? picked[picked.length - 1] : "#ff8a3d"
    implicitWidth: compact ? barRow.implicitWidth : 300
    implicitHeight: compact ? 24 : 250
    property string err: ""
    function pick() { if (!proc.running) { err = ""; proc.running = true; } }
    function choose(hex) { set("colours", picked.filter(x => x !== hex).concat([hex]).slice(-8)); }
    function copy(t) { Quickshell.execDetached(["wl-copy", "--", t]); SettingsNav.toast("Copied " + t); }
    Process {
        id: proc
        command: ["python3", w.helper, "pick-colour"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { const r = JSON.parse(text.trim().split("\n").pop()); if (r.hex) w.choose(String(r.hex).toLowerCase()); else w.err = r.error || ""; }
                catch (e) { w.err = "Couldn't pick a colour"; }
            }
        }
    }

    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6
        Rectangle { width: 18; height: 18; radius: 5; color: w.cur; border.color: Theme.line }
        Text { text: w.cur.toUpperCase(); color: Theme.text; font.family: "DejaVu Sans Mono"; font.pixelSize: 13 }
        MouseArea { anchors.fill: parent; onClicked: w.pick() }
    }

    ColumnLayout {
        visible: !w.compact
        anchors { fill: parent; margins: 14 * w.k }
        spacing: 8 * w.k
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Rectangle {
                Layout.preferredWidth: 74 * w.k; Layout.preferredHeight: 74 * w.k
                radius: Theme.r(14); color: w.cur; border.color: Theme.line; border.width: 1
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Repeater {
                    model: T.formats(w.cur).slice(0, 3)
                    delegate: Text {
                        required property string modelData
                        Layout.fillWidth: true
                        text: modelData; elide: Text.ElideRight
                        color: fa.containsMouse ? Theme.accent : Theme.text
                        font.family: "DejaVu Sans Mono"; font.pixelSize: Math.round(13 * w.k)
                        MouseArea { id: fa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: w.copy(parent.modelData) }
                    }
                }
            }
        }
        WBtn { Layout.fillWidth: true; icon: "palette"; text: proc.running ? "Click on the screen…" : "Pick a colour"; on: true; size: 32 * w.k; onClicked: w.pick() }
        Text { visible: w.err !== ""; Layout.fillWidth: true; text: w.err; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: w.picked.slice().reverse()
                delegate: Rectangle {
                    required property string modelData
                    width: 26 * w.k; height: 26 * w.k; radius: Theme.r(7)
                    color: modelData
                    border.width: modelData === w.cur ? 2 : 1
                    border.color: modelData === w.cur ? Theme.accent : Theme.line
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: w.copy(parent.modelData.toUpperCase()) }
                }
            }
        }
        Item { Layout.fillHeight: true }
    }
}
