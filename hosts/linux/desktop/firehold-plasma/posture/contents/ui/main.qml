// Firehold's Desk (mockup 2): the posture pill "Desk" with its blue dot, then Wi-Fi · battery. Press the pill (or the
// controller's Guide button) to go back to the Couch; everything stays open. It also tells hearth-firehold-plasma the
// Desk came up. Battery and network from Plasma's own sources, as they change (no timer).
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasma5support as P5Support

PlasmoidItem {
    id: root
    preferredRepresentation: fullRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground

    P5Support.DataSource {
        id: sh
        engine: "executable"
        property var cbs: ({})
        onNewData: function (source, data) {
            const cb = sh.cbs[source];
            delete sh.cbs[source];
            sh.disconnectSource(source);
            if (cb) cb(String(data["stdout"] || ""));
        }
        function run(cmd, cb) { sh.cbs[cmd] = cb || null; sh.connectSource(cmd); }
    }
    P5Support.DataSource {
        id: power
        engine: "powermanagement"
        connectedSources: ["Battery"]
    }
    readonly property var battery: power.data["Battery"] || ({})
    readonly property string batteryText: battery["Has Battery"] ? Math.round(battery["Percent"] || 0) + "%" : ""
    property string net: ""
    Component.onCompleted: {
        sh.run("hearth-firehold-plasma ready org.hearthos.firehold.desk");
        sh.run("hearth-firehold-plasma sys", function (t) {
            try { root.net = JSON.parse(t.trim().split("\n").pop()).net || ""; } catch (e) {}
        });
    }

    fullRepresentation: Item {
        Layout.minimumWidth: row.implicitWidth + 12
        Layout.preferredWidth: row.implicitWidth + 12
        Layout.fillHeight: true
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 14
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                height: 26; radius: 13
                width: pillRow.implicitWidth + 24
                color: "#2a201b"
                border.width: 1; border.color: "#3a2d26"
                Row {
                    id: pillRow
                    anchors.centerIn: parent
                    spacing: 7
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 8; height: 8; radius: 4; color: "#5fb4f5" }
                    Text { text: "Desk"; color: "#f6ede6"; font.pixelSize: 14; font.weight: Font.Bold }
                }
                MouseArea { anchors.fill: parent; onClicked: sh.run("hearth-firehold-plasma switch couch home") }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: [root.net, root.batteryText].filter(Boolean).join(" · ")
                visible: text !== ""
                color: "#a3958b"; font.pixelSize: 14
            }
        }
    }
}
