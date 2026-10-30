// Hearth Plasma: the Game Mode button at the end of the taskbar, as on Hearth Classic's (Mantel's Bar.qml "Back to
// Game Mode"). It runs nimbus-mode game, which ends Plasma properly and starts Game Mode in the same session.
import QtQuick
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Plasmoid.icon: "input-gamepad"
    toolTipMainText: "Game Mode"
    toolTipSubText: "Back to Hearth's Game Mode (Super+G)"
    preferredRepresentation: compactRepresentation

    P5Support.DataSource {
        id: exec
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => disconnectSource(source)
    }

    compactRepresentation: MouseArea {
        id: area
        hoverEnabled: true
        Accessible.name: "Game Mode"
        Accessible.role: Accessible.Button
        onClicked: exec.connectSource("nimbus-mode game")
        Rectangle {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.smallSpacing / 2
            radius: Kirigami.Units.smallSpacing * 2
            color: area.containsMouse ? Kirigami.Theme.highlightColor : "transparent"
            opacity: area.containsMouse ? 0.25 : 0
        }
        Kirigami.Icon {
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height) * 0.7
            height: width
            source: "input-gamepad"
            active: area.containsMouse
        }
    }
}
