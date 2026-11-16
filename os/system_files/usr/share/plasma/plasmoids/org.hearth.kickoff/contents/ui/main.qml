// KDE Plasma with Hearth inside (1.13, docs/DESKTOPS.md; mockup docs/mockups/next-update/desktops-2-kde-plasma.png):
// KDE's start menu in KDE's own look, listing only KDE's apps (tagged KDE) and the apps you installed (tagged
// installed): Hearth's own apps stay in Mantel (hearth-desktops apps kde, the same rule as everywhere: Settings › Your
// desktop › Only show each desktop's own apps). Your name and Search at the top, Favorites, All Applications and the
// categories on the left, and along the foot Shut down, Restart, Switch desktop… and Return to Game Mode.
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Plasmoid.icon: "start-here-kde"
    toolTipMainText: "Application Menu"
    switchWidth: Kirigami.Units.gridUnit * 20
    switchHeight: Kirigami.Units.gridUnit * 20
    readonly property string helper: "/usr/share/nimbus/desktop/shell/hearth-desktops"
    property var apps: []
    property var favorites: []
    property var categories: []
    property string section: "Favorites"
    property string query: ""
    property string user: ""
    property bool switching: false
    property var desktops: []

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
    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    function refresh() {
        exec.run("python3 " + helper + " apps kde", out => {
            const j = lastJson(out, null);
            if (j) { root.apps = j.apps || []; root.favorites = j.favorites || []; root.categories = j.categories || []; }
        });
        exec.run("python3 " + helper + " tray", out => { const j = lastJson(out, null); if (j) root.desktops = j.desktops || []; });
        exec.run("id -un", out => root.user = String(out).trim());
    }
    function open(a) { exec.run("gtk-launch " + q(a.id.replace(/\.desktop$/, "")) + " || kioclient exec " + q("/usr/share/applications/" + a.id), () => {}); root.expanded = false; }
    function run(cmd) { exec.run(cmd, () => {}); root.expanded = false; }
    Component.onCompleted: refresh()
    onExpandedChanged: if (expanded) { query = ""; section = "Favorites"; switching = false; refresh(); search.forceActiveFocus(); }

    readonly property var shown: {
        const t = query.trim().toLowerCase();
        if (t) return apps.filter(a => (a.name + " " + a.keywords + " " + a.comment).toLowerCase().indexOf(t) >= 0);
        if (section === "Favorites") return favorites.map(id => apps.find(a => a.id === id)).filter(a => !!a);
        if (section === "All Applications") return apps;
        return apps.filter(a => a.category === section);
    }

    compactRepresentation: MouseArea {
        onClicked: root.expanded = !root.expanded
        Kirigami.Icon { anchors.fill: parent; source: "start-here-kde"; active: parent.containsMouse }
        hoverEnabled: true
    }

    fullRepresentation: ColumnLayout {
        Layout.preferredWidth: Kirigami.Units.gridUnit * 34
        Layout.preferredHeight: Kirigami.Units.gridUnit * 30
        spacing: 0
        RowLayout {   // you, and Search
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.largeSpacing
            Rectangle {
                width: Kirigami.Units.gridUnit * 2; height: width; radius: width / 2; color: "#5b8a6a"
                PlasmaComponents.Label { anchors.centerIn: parent; text: (root.user || "?").charAt(0).toUpperCase(); font.weight: Font.Bold; color: "white" }
            }
            PlasmaComponents.Label { text: root.user; font.weight: Font.DemiBold }
            PlasmaExtras.SearchField {
                id: search
                Layout.fillWidth: true
                onTextChanged: root.query = text
                Keys.onReturnPressed: if (root.shown.length) root.open(root.shown[0])
            }
        }
        Kirigami.Separator { Layout.fillWidth: true }
        RowLayout {
            visible: !root.switching
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0
            ColumnLayout {   // Favorites, All Applications, the categories
                Layout.preferredWidth: Kirigami.Units.gridUnit * 10
                Layout.fillHeight: true
                Layout.margins: Kirigami.Units.smallSpacing * 2
                spacing: 2
                Repeater {
                    model: ["Favorites", "All Applications"].concat(root.categories)
                    delegate: PlasmaComponents.ItemDelegate {
                        required property string modelData
                        Layout.fillWidth: true
                        text: (modelData === "Favorites" ? "★ " : "") + modelData
                        highlighted: root.section === modelData && root.query === ""
                        onClicked: { root.section = modelData; search.text = ""; }
                        onHoveredChanged: if (hovered) root.section = modelData
                    }
                }
                Item { Layout.fillHeight: true }
            }
            Kirigami.Separator { Layout.fillHeight: true }
            QQC2.ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                ListView {
                    model: root.shown
                    clip: true
                    delegate: PlasmaComponents.ItemDelegate {
                        required property var modelData
                        width: ListView.view.width
                        onClicked: root.open(modelData)
                        contentItem: RowLayout {
                            spacing: Kirigami.Units.largeSpacing
                            Kirigami.Icon { source: modelData.icon || "application-x-executable"; Layout.preferredWidth: Kirigami.Units.iconSizes.medium; Layout.preferredHeight: Layout.preferredWidth }
                            PlasmaComponents.Label { Layout.fillWidth: true; text: modelData.name; elide: Text.ElideRight }
                            PlasmaComponents.Label { text: modelData.tag; opacity: 0.6; font.pointSize: Kirigami.Theme.smallFont.pointSize }
                        }
                    }
                }
            }
        }
        ColumnLayout {   // Switch desktop…
            visible: root.switching
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Kirigami.Units.largeSpacing
            PlasmaComponents.Label { text: "Switch desktop"; font.weight: Font.DemiBold }
            Repeater {
                model: root.desktops
                delegate: PlasmaComponents.ItemDelegate {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData.name + (modelData.id === "kde" ? "  (using now)" : "")
                    enabled: modelData.id !== "kde"
                    onClicked: root.run("python3 " + root.helper + " act switch " + modelData.id)
                }
            }
            PlasmaComponents.ItemDelegate { Layout.fillWidth: true; text: "Get GNOME, or remove one… (Settings)"; onClicked: root.run("python3 " + root.helper + " act settings") }
            Item { Layout.fillHeight: true }
            PlasmaComponents.Button { text: "Back"; onClicked: root.switching = false }
        }
        Kirigami.Separator { Layout.fillWidth: true }
        RowLayout {   // the foot
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.smallSpacing * 2
            PlasmaComponents.ToolButton { icon.name: "system-shutdown"; text: "Shut down"
                onClicked: root.run("qdbus6 org.kde.Shutdown /Shutdown org.kde.Shutdown.logoutAndShutdown || systemctl poweroff") }
            PlasmaComponents.ToolButton { icon.name: "system-reboot"; text: "Restart"
                onClicked: root.run("qdbus6 org.kde.Shutdown /Shutdown org.kde.Shutdown.logoutAndReboot || systemctl reboot") }
            PlasmaComponents.ToolButton { text: "⇄ Switch desktop…"; onClicked: root.switching = true }
            Item { Layout.fillWidth: true }
            PlasmaComponents.ToolButton { text: "🎮 Return to Game Mode"; onClicked: root.run("python3 " + root.helper + " act game") }
        }
    }
}
