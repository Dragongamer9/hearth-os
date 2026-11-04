// The dashboard's system card (ClockPanel.qml): CPU, GPU, memory and temperature as small bars, read by
// hearth-stats only while the dashboard is open. A value the system doesn't give is left out.
import QtQuick
import QtQuick.Layouts

Item {
    id: card
    Stats { id: stats; running: card.visible }
    Rectangle { anchors.fill: parent; radius: Theme.r(16); color: Theme.panel2 }
    ColumnLayout {
        anchors { fill: parent; margins: 18 }
        spacing: 6
        Text { text: "SYSTEM"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 1.2 }
        Bar { visible: stats.cpu >= 0; label: "CPU"; value: stats.cpu + "%"; frac: stats.cpu / 100 }
        Bar { visible: stats.gpu >= 0; label: "GPU"; value: stats.gpu + "%"; frac: stats.gpu / 100 }
        Bar { visible: stats.ram >= 0 && stats.ramTotal > 0; label: "Memory"; value: stats.ram.toFixed(1) + " / " + Math.round(stats.ramTotal) + " GB"; frac: stats.ramTotal > 0 ? stats.ram / stats.ramTotal : 0 }
        Bar { visible: stats.cpuTemp >= 0; label: "Temp"; value: stats.cpuTemp + " °C"; frac: Math.min(1, stats.cpuTemp / 100) }
        Text { visible: stats.cpu < 0; text: "Reading…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        Item { Layout.fillHeight: true }
    }
    component Bar: ColumnLayout {
        id: b
        property string label
        property string value
        property real frac: 0
        Layout.fillWidth: true
        spacing: 3
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: b.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
            Text { text: b.value; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        }
        Item {
            Layout.fillWidth: true
            implicitHeight: 4
            Rectangle { anchors.fill: parent; radius: 2; color: Theme.panel3 }
            Rectangle { height: parent.height; radius: 2; color: Theme.accent; width: parent.width * Math.max(0, Math.min(1, b.frac)) }
        }
    }
}
