// One of your features in the bar (Features.qml): what it shows, read every few seconds only while it's on the
// screen; a click does its action. Marked NEW for a while after it's kept.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Rectangle {
    id: fp
    property var feature: null
    property real side: 34
    readonly property var sources: feature ? (feature.shows || []).map(s => s.source).filter(s => s !== "text") : []
    property var values: ({})
    implicitWidth: row.implicitWidth + 24
    implicitHeight: side
    radius: Theme.r(height / 2)
    color: ma.containsMouse ? Theme.panel3 : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.08)
    border.color: Theme.line
    Process {
        id: data
        command: ["python3", Quickshell.shellDir + "/hearth-feature-data"].concat(fp.sources)
        stdout: StdioCollector { onStreamFinished: { try { fp.values = JSON.parse(text.trim() || "{}"); } catch (e) {} } }
    }
    Timer {
        interval: Math.max(2, fp.feature ? fp.feature.every || 30 : 30) * 1000
        running: fp.visible && fp.sources.length > 0 && fp.QsWindow.window && fp.QsWindow.window.visible
        repeat: true; triggeredOnStart: true
        onTriggered: { data.running = false; data.running = true; }
    }
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 10
        Repeater {
            model: fp.feature ? fp.feature.shows || [] : []
            Row {
                required property var modelData
                spacing: 5
                Text { visible: !!modelData.label; text: modelData.label || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
                Text { text: modelData.source === "text" ? "" : (fp.values[modelData.source] || "…"); color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
            }
        }
    }
    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Features.run(fp.feature ? fp.feature.click : null) }
}
