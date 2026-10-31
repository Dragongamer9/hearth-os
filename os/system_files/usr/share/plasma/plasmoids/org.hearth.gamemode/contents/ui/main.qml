// Hearth Plasma: the Game Mode button at the end of the taskbar, as on Hearth Classic's (Mantel's Bar.qml "Back to
// Game Mode"). It runs nimbus-mode game, which ends Plasma properly and starts Game Mode in the same session.
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Plasmoid.icon: "input-gamepad"
    toolTipMainText: "Game Mode"
    toolTipSubText: "Back to Hearth's Game Mode (Super+G)"
    // Shown as it is on the panel (no popup): the pill below is the whole widget.
    preferredRepresentation: fullRepresentation

    P5Support.DataSource {
        id: exec
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => disconnectSource(source)
    }
    function go() { exec.connectSource("nimbus-mode game"); }
    // From the keyboard or the controller: the taskbar's keyboard focus (View + A on the controller, Meta+Alt+P)
    // reaches it with the arrows; Enter / Space (A) or its own shortcut runs it.
    Connections {
        target: Plasmoid
        function onActivated() { root.go(); }
    }

    // Mantel's pill: "Game Mode" on the accent colour.
    fullRepresentation: MouseArea {
        id: area
        hoverEnabled: true
        Accessible.name: "Game Mode"
        Accessible.role: Accessible.Button
        Layout.minimumWidth: label.implicitWidth + Kirigami.Units.largeSpacing * 2
        Layout.preferredWidth: Layout.minimumWidth
        Layout.minimumHeight: Kirigami.Units.gridUnit
        Layout.maximumWidth: Layout.minimumWidth
        onClicked: root.go()
        activeFocusOnTab: true
        Keys.onReturnPressed: root.go()
        Keys.onEnterPressed: root.go()
        Keys.onSpacePressed: root.go()
        Rectangle {
            anchors.centerIn: parent
            width: parent.width
            height: Math.min(parent.height - Kirigami.Units.smallSpacing, Kirigami.Units.gridUnit * 1.8)
            radius: Kirigami.Units.smallSpacing * 2
            color: Kirigami.Theme.highlightColor
            opacity: area.containsMouse ? 0.85 : 1
            border.width: area.activeFocus ? 2 : 0
            border.color: Kirigami.Theme.textColor
        }
        PlasmaComponents.Label {
            id: label
            anchors.centerIn: parent
            text: "Game Mode"
            font.bold: true
            color: Kirigami.Theme.highlightedTextColor
        }
    }
}
