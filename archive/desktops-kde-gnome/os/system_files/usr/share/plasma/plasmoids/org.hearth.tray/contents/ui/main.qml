// KDE Plasma with Hearth inside (1.13, docs/DESKTOPS.md; mockup docs/mockups/next-update/desktops-2-kde-plasma.png):
// Hearth's item in KDE's tray, an orange square by the clock. Its popup: "Hearth", a big "Return to Game Mode", then
// Controller, Keyboard, Hearth notices, Server at home, Hearth update and Switch desktop (Mantel · KDE · GNOME).
// Everything through hearth-desktops (tray: what it shows; act: what each button does).
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    toolTipMainText: "Hearth"
    toolTipSubText: "Return to Game Mode and Hearth's parts"
    Plasmoid.status: PlasmaCore.Types.ActiveStatus
    readonly property string helper: "/usr/share/nimbus/desktop/shell/hearth-desktops"
    readonly property color orange: "#f0a840"
    property var t: ({})
    property bool switching: false

    function lastJson(out, fallback) {
        const lines = String(out || "").trim().split("\n");
        try { return JSON.parse(lines[lines.length - 1]); } catch (e) { return fallback; }
    }
    P5Support.DataSource {
        id: exec
        engine: "executable"
        connectedSources: []
        property var callbacks: ({})
        property int n: 0
        onNewData: (source, data) => {
            const cb = callbacks[source];
            delete callbacks[source];
            disconnectSource(source);
            if (cb) cb(data["stdout"] || "", data["exit code"]);
        }
        function run(cmd, cb) { const src = cmd + " #" + (++n); callbacks[src] = cb || null; connectSource(src); }
    }
    function refresh() { exec.run("python3 " + helper + " tray", out => { const j = lastJson(out, null); if (j) root.t = j; }); }
    function act(a) { exec.run("python3 " + helper + " act " + a, () => {}); root.expanded = false; }
    Component.onCompleted: refresh()
    onExpandedChanged: if (expanded) { switching = false; refresh(); }
    Timer { interval: 60000; repeat: true; running: true; onTriggered: root.refresh() }

    compactRepresentation: MouseArea {
        onClicked: root.expanded = !root.expanded
        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height) * 0.78; height: width; radius: width * 0.22
            color: root.orange
        }
    }

    fullRepresentation: ColumnLayout {
        Layout.preferredWidth: Kirigami.Units.gridUnit * 26
        Layout.minimumWidth: Kirigami.Units.gridUnit * 22
        spacing: Kirigami.Units.largeSpacing
        RowLayout {
            spacing: Kirigami.Units.smallSpacing * 2
            Rectangle { width: Kirigami.Units.gridUnit * 1.3; height: width; radius: width * 0.22; color: root.orange }
            PlasmaComponents.Label { text: "Hearth"; font.weight: Font.DemiBold; font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.25 }
        }
        PlasmaComponents.Button {   // the big one, in KDE's own highlight colour
            Layout.fillWidth: true
            Layout.preferredHeight: Kirigami.Units.gridUnit * 2.6
            text: "🎮  Return to Game Mode"
            font.weight: Font.DemiBold
            highlighted: true
            onClicked: root.act("game")
        }
        GridLayout {
            visible: !root.switching
            Layout.fillWidth: true
            columns: 2; rowSpacing: Kirigami.Units.smallSpacing * 2; columnSpacing: Kirigami.Units.smallSpacing * 2
            Repeater {
                model: [
                    { a: "controller", icon: "🎮", t: "Controller", s: root.t.controller || "" },
                    { a: "keyboard", icon: "⌨", t: "Keyboard", s: root.t.keyboard || "shows when you type" },
                    { a: "notices", icon: "🔔", t: "Hearth notices", s: root.t.notices || "in KDE's notifications" },
                    { a: "server", icon: "🖥", t: "Server at home", s: root.t.server || "" },
                    { a: "update", icon: "⬇", t: "Hearth update", s: root.t.update || "" },
                    { a: "switch", icon: "⇄", t: "Switch desktop", s: root.t.switch || "Mantel · KDE · GNOME" }
                ]
                delegate: PlasmaComponents.ItemDelegate {
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    onClicked: modelData.a === "switch" ? root.switching = true : root.act(modelData.a)
                    contentItem: ColumnLayout {
                        spacing: 2
                        PlasmaComponents.Label { text: modelData.icon + " " + modelData.t }
                        PlasmaComponents.Label { Layout.fillWidth: true; text: modelData.s; opacity: 0.7; font.pointSize: Kirigami.Theme.smallFont.pointSize; wrapMode: Text.WordWrap }
                    }
                    background: Rectangle { radius: Kirigami.Units.smallSpacing * 1.5; color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, parent.hovered ? 0.12 : 0.06) }
                }
            }
        }
        ColumnLayout {   // Switch desktop: the ones that are here
            visible: root.switching
            Layout.fillWidth: true
            PlasmaComponents.Label { text: "Switch desktop"; font.weight: Font.DemiBold }
            Repeater {
                model: root.t.desktops || []
                delegate: PlasmaComponents.ItemDelegate {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData.name + (root.t.now === modelData.id ? "  (using now)" : "")
                    enabled: root.t.now !== modelData.id
                    onClicked: root.act("switch " + modelData.id)
                }
            }
            PlasmaComponents.ItemDelegate { Layout.fillWidth: true; text: "Get KDE Plasma or GNOME… (Settings)"; onClicked: root.act("settings") }
            PlasmaComponents.Button { text: "Back"; onClicked: root.switching = false }
        }
    }
}
