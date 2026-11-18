// Firehold's Desk (mockup 2): the accent tile at the top left and the focused app's name (Plasma's own task model,
// as its task manager has it). The tile goes to the Couch's Home; KDE's global menu widget next to it has the app's
// menus (File, Edit, View).
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasma5support as P5Support
import org.kde.taskmanager as TaskManager

PlasmoidItem {
    id: root
    preferredRepresentation: fullRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground

    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        filterByVirtualDesktop: false
        filterByScreen: false
        filterByActivity: false
    }
    readonly property var active: tasks.activeTask
    readonly property string appName: active && active.valid ? String(tasks.data(active, TaskManager.AbstractTasksModel.AppName) || "") : ""
    P5Support.DataSource { id: sh; engine: "executable"; onNewData: function (source) { sh.disconnectSource(source); } }

    fullRepresentation: Item {
        Layout.minimumWidth: row.implicitWidth + 8
        Layout.preferredWidth: row.implicitWidth + 8
        Layout.fillHeight: true
        Row {
            id: row
            anchors.verticalCenter: parent.verticalCenter
            x: 4
            spacing: 14
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 30; height: 30; radius: 7
                color: "#ff8a3d"
                MouseArea { anchors.fill: parent; onClicked: sh.connectSource("hearth-firehold-plasma switch couch home") }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.appName || "Firehold"
                color: Kirigami.Theme.textColor; font.pixelSize: 15; font.weight: Font.Bold
            }
        }
    }
}
