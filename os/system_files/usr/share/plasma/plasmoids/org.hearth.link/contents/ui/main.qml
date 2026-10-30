// Hearth Plasma: Hearth Link in the system tray, the short version of Hearth Classic's Settings › Hearth Link
// (Mantel's SetLink.qml): this device, the devices paired with it, a pairing code, sync now. Everything goes through
// the same helper Mantel uses, desktop/shell/hearth-link-settings (Hearth Link's own socket, only this user's).
// It sits in the tray's overflow, and comes forward while a pairing code is showing.
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Plasmoid.icon: "hearth-share"
    Plasmoid.status: st && st.pairing ? PlasmaCore.Types.ActiveStatus : PlasmaCore.Types.PassiveStatus
    toolTipMainText: "Hearth Link"
    toolTipSubText: !st ? "Not running" : st.error ? st.error : peers.length + (peers.length === 1 ? " paired device" : " paired devices")

    readonly property string helper: "/usr/share/nimbus/desktop/shell/hearth-link-settings"
    property var st: null
    property string message: ""
    readonly property var peers: st && st.peers ? st.peers : []

    function lastLine(out) { const l = String(out || "").trim().split("\n"); return l[l.length - 1] || ""; }
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
        function run(cmd, cb) {
            const src = cmd + " #" + (++n);
            callbacks[src] = cb || null;
            connectSource(src);
        }
    }
    function refresh() {
        exec.run("python3 " + helper + " status", out => {
            const l = lastLine(out);
            if (l.startsWith("error: ")) { root.st = { error: l.slice(7) }; return; }
            try { root.st = JSON.parse(l); } catch (e) { root.st = null; }
        });
    }
    function act(args, done) {
        exec.run("python3 " + helper + " " + args, out => {
            const l = lastLine(out);
            root.message = l.startsWith("error: ") ? l.slice(7) : (done || "");
            refresh();
        });
    }
    Component.onCompleted: refresh()
    onExpandedChanged: if (root.expanded) { message = ""; refresh(); }
    Timer { interval: root.expanded ? 3000 : 60000; running: true; repeat: true; onTriggered: root.refresh() }

    fullRepresentation: ColumnLayout {
        Layout.preferredWidth: Kirigami.Units.gridUnit * 22
        Layout.minimumWidth: Kirigami.Units.gridUnit * 18
        spacing: Kirigami.Units.smallSpacing

        PlasmaExtras.Heading { level: 3; text: "Hearth Link"; Layout.margins: Kirigami.Units.smallSpacing }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !root.st || !!root.st.error
            text: !root.st ? "Hearth Link isn't running." : (root.st.error || "")
            wrapMode: Text.WordWrap; opacity: 0.7
        }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !!root.st && !!root.st.me
            text: root.st && root.st.me ? "This device: " + root.st.me.name : ""
            font.bold: true; elide: Text.ElideRight
        }
        // A pairing code is showing: type it on the other Hearth device (Settings › Hearth Link, pick this one).
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !!root.st && !!root.st.pairing && !!root.st.pairing.code
            text: root.st && root.st.pairing && root.st.pairing.code ? "Code: " + root.st.pairing.code.slice(0, 3) + " " + root.st.pairing.code.slice(3) : ""
            font.pixelSize: Kirigami.Theme.defaultFont.pixelSize * 1.8; color: Kirigami.Theme.highlightColor
        }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !!root.st && !!root.st.pairing
            text: root.st && root.st.me ? "On the other device: Settings › Hearth Link, pick " + root.st.me.name + " and type this code." : ""
            wrapMode: Text.WordWrap; opacity: 0.7
        }

        PlasmaExtras.Heading { level: 5; text: "Paired devices"; visible: root.peers.length > 0; Layout.topMargin: Kirigami.Units.smallSpacing }
        Repeater {
            model: root.peers
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                Kirigami.Icon { source: modelData.server ? "hearth-disks" : "computer"; Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium; Layout.preferredHeight: Layout.preferredWidth }
                PlasmaComponents.Label { Layout.fillWidth: true; text: modelData.name + (modelData.server ? " · Hearth Storage" : ""); elide: Text.ElideRight }
                PlasmaComponents.Label { text: modelData.online ? "Online" : "Off"; opacity: modelData.online ? 1 : 0.6 }
            }
        }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !!root.st && !!root.st.me && root.peers.length === 0
            text: "No devices paired yet. Pair one to share games, saves and files between your Hearth devices."
            wrapMode: Text.WordWrap; opacity: 0.7
        }
        PlasmaComponents.Label { Layout.fillWidth: true; visible: root.message !== ""; text: root.message; wrapMode: Text.WordWrap; color: Kirigami.Theme.neutralTextColor }

        Kirigami.Separator { Layout.fillWidth: true }
        RowLayout {
            Layout.fillWidth: true
            visible: !!root.st && !!root.st.me
            PlasmaComponents.Button {
                text: root.st && root.st.pairing ? "Stop pairing" : "Pair a device"
                icon.name: "network-connect"
                onClicked: root.act(root.st && root.st.pairing ? "pair-close" : "pair-open")
            }
            PlasmaComponents.Button { text: "Sync now"; icon.name: "view-refresh"; enabled: root.peers.length > 0; onClicked: root.act("sync", "Syncing with your devices") }
            Item { Layout.fillWidth: true }
            PlasmaComponents.ToolButton {
                icon.name: "configure"
                onClicked: { exec.run("hearth-settings link"); root.expanded = false; }
                PlasmaComponents.ToolTip { text: "Hearth Link settings" }
            }
        }
    }
}
