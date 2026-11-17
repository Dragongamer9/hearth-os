// Battery widget: how full, charging or not, and the time left (UPower, as the bar's battery). A PC without a battery
// says so. HEARTH_WIDGETS_FIXTURES/battery.json stands in for one in the Desktop Mode test. In the bar: the percentage.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower

WidgetBase {
    id: w
    readonly property var dev: UPower.displayDevice
    property var fix: null
    FileView { path: w.fixtures ? w.fixtures + "/battery.json" : ""; blockLoading: true; onLoaded: { try { w.fix = JSON.parse(text()); } catch (e) {} } }
    readonly property bool has: fix !== null || (dev !== null && dev.isLaptopBattery === true)
    readonly property int pct: fix ? fix.percent : dev ? Math.round(dev.percentage * 100) : 0
    readonly property bool charging: fix ? fix.charging === true : dev !== null && dev.state === UPowerDeviceState.Charging
    readonly property bool full: !fix && dev !== null && dev.state === UPowerDeviceState.FullyCharged
    readonly property real secs: fix ? (fix.seconds || 0) : dev ? (charging ? dev.timeToFull : dev.timeToEmpty) : 0
    readonly property color level: pct <= 15 && !charging ? Theme.danger : pct <= 30 && !charging ? "#ffb454" : "#6fd08c"
    implicitWidth: compact ? barRow.implicitWidth : 260
    implicitHeight: compact ? 24 : 150
    function hm(s) { const m = Math.round(s / 60); return m >= 60 ? Math.floor(m / 60) + " h " + (m % 60) + " min" : m + " min"; }

    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4
        Icon { name: w.charging ? "batt-charging" : w.pct <= 15 ? "batt-low" : "batt"; size: 16; color: Theme.text }
        Text { text: w.has ? w.pct + "%" : "No battery"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
    }
    RowLayout {
        visible: !w.compact
        anchors { fill: parent; margins: 16 }
        spacing: 16
        Item {   // the battery, drawn
            Layout.preferredWidth: 70 * w.k; Layout.preferredHeight: 36 * w.k
            Rectangle { id: shell; anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: parent.width - 6; radius: 6; color: "transparent"; border.width: 3; border.color: Theme.text }
            Rectangle { anchors { left: shell.right; verticalCenter: parent.verticalCenter } width: 5; height: parent.height * 0.4; radius: 2; color: Theme.text }
            Rectangle { x: 6; y: 6; height: parent.height - 12; width: Math.max(0, (shell.width - 12) * w.pct / 100); radius: 3; color: w.level; visible: w.has }
            Icon { anchors.centerIn: shell; visible: w.charging; name: "bolt"; size: 20 * w.k; color: Theme.text }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text { text: w.has ? w.pct + " %" : "No battery"; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(30 * w.k); font.weight: Font.Bold }
            Text {
                Layout.fillWidth: true
                text: !w.has ? "This PC runs on mains power" : w.full ? "Fully charged" : w.charging ? (w.secs > 0 ? "Charging · full in " + w.hm(w.secs) : "Charging") : (w.secs > 0 ? w.hm(w.secs) + " left" : "On battery")
                color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(13 * w.k); wrapMode: Text.WordWrap
            }
        }
    }
}
