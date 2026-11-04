// The desktop clock and weather (Mantel 2026, Settings › Appearance › Desktop clock and weather): a big clock, the
// date and Hearth Weather's line at the bottom left of the main screen, drawn on the wallpaper (under every window,
// never in the way of a click). Each look dresses it its own way: Gaming glows, Synthwave is chrome, Cabin is a
// wooden plaque, Glass is thin.
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: root
    SystemClock { id: clock; precision: SystemClock.Minutes }
    property var weather: ({})
    Process {
        id: wx
        running: true
        command: ["hearth-time", "--weather-summary"]
        stdout: StdioCollector { onStreamFinished: { try { root.weather = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { root.weather = {}; } } }
    }
    Timer { interval: 900000; running: true; repeat: true; onTriggered: wx.running = true }

    readonly property string look: Theme.look
    readonly property real k: Math.max(0.6, Math.min(1.3, height / 1080))
    readonly property string timeText: Qt.formatTime(clock.date, Settings.clock24 || look === "gaming" ? "HH:mm" : "h:mm")
    readonly property string dateText: Qt.formatDate(clock.date, "dddd, d MMMM") + (weather.line ? "   ·   " + weather.line : "")

    Rectangle {   // Cabin's wooden plaque
        visible: root.look === "cabin"
        x: col.x - 28 * root.k; y: col.y - 18 * root.k
        width: col.width + 56 * root.k; height: col.height + 36 * root.k
        radius: 10 * root.k
        color: "#5a3a22"
        border.color: "#2e1d10"; border.width: 3
        gradient: Gradient {
            GradientStop { position: 0; color: "#6b4528" }
            GradientStop { position: 0.5; color: "#573620" }
            GradientStop { position: 1; color: "#3f2716" }
        }
        Repeater {
            model: 6
            delegate: Rectangle { required property int index; x: 6; width: parent.width - 12; y: 8 + index * (parent.height - 16) / 6; height: 1; color: "#33000000" }
        }
    }
    Column {
        id: col
        x: 80 * root.k
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Math.max(110, DeskState.bottomInset + 60) * root.k
        spacing: 4 * root.k
        Text {
            text: root.timeText
            color: root.look === "gaming" ? "#e8fbff" : root.look === "synthwave" ? "#ffe9fb" : root.look === "cabin" ? "#ffe2bf" : "#ffffff"
            style: root.look === "gaming" || root.look === "synthwave" ? Text.Outline : Text.Normal
            styleColor: root.look === "gaming" ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.6) : Qt.rgba(Theme.flame.r, Theme.flame.g, Theme.flame.b, 0.7)
            font.family: root.look === "gaming" ? "DejaVu Sans Condensed" : Theme.font
            font.pixelSize: (root.look === "cabin" ? 72 : 150) * root.k
            font.weight: root.look === "glass" ? Font.Light : Font.Bold
            font.italic: root.look === "synthwave"
        }
        Text {
            text: root.look === "gaming" ? root.dateText.toUpperCase() : root.dateText
            color: root.look === "gaming" ? Theme.flame : root.look === "cabin" ? "#f1d3ad" : "#f4f4f4"
            font.family: Theme.font
            font.pixelSize: (root.look === "cabin" ? 18 : 28) * root.k
            font.letterSpacing: root.look === "gaming" ? 6 * root.k : 0
            font.bold: root.look === "gaming"
            style: Text.Raised; styleColor: "#66000000"
        }
    }
}
