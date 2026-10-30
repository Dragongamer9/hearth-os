// Hearth Plasma: Hearth Storage in the system tray, the short version of Hearth Classic's Settings › Hearth Link ›
// Hearth Storage (Mantel's SetLink.qml): this device as the home server or not, your devices' health, shared
// folders, back up now. Through the same helper as Mantel, desktop/shell/hearth-link-settings (status, devices,
// storage-set, backup-now). Browsing, moving games and media stay in Game Mode and Hearth Files.
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
    Plasmoid.icon: "hearth-disks"
    Plasmoid.status: PlasmaCore.Types.PassiveStatus
    toolTipMainText: "Hearth Storage"
    toolTipSubText: !stc ? "Hearth Link isn't running" : stc.server ? "This device is your Hearth Storage" : "Using your Hearth Storage"

    readonly property string helper: "/usr/share/nimbus/desktop/shell/hearth-link-settings"
    property var st: null
    property var devs: null
    property string message: ""
    readonly property var stc: st && st.storage ? (st.storage.config || {}) : null
    readonly property var devices: devs && devs.devices ? devs.devices : []

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
            try { root.st = l.startsWith("error: ") ? null : JSON.parse(l); } catch (e) { root.st = null; }
            if (root.stc) exec.run("python3 " + helper + " devices", o2 => { try { const d = JSON.parse(lastLine(o2)); if (d) root.devs = d; } catch (e) {} });
        });
    }
    function act(args, done) {
        exec.run("python3 " + helper + " " + args, out => {
            const l = lastLine(out);
            root.message = l.startsWith("error: ") ? l.slice(7) : (done || "");
            refresh();
        });
    }
    function bytes(n) {
        const u = ["B", "KB", "MB", "GB", "TB"]; let i = 0; let v = Number(n) || 0;
        while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; }
        return (i ? v.toFixed(1) : v) + " " + u[i];
    }
    function devLine(d) {
        const b = d.battery;
        return [d.me ? "This device" : d.online ? "Online" : "Off",
                b && b.percent !== undefined && b.percent !== null ? "Battery " + b.percent + "%" : "",
                d.free !== undefined && d.free !== null ? bytes(d.free) + " free" : "",
                d.lastBackup ? "Backed up " + new Date(d.lastBackup).toLocaleDateString() : ""].filter(x => x).join(" · ");
    }
    Component.onCompleted: refresh()
    onExpandedChanged: if (expanded) { message = ""; refresh(); }
    Timer { interval: root.expanded ? 10000 : 120000; running: true; repeat: true; onTriggered: root.refresh() }

    fullRepresentation: ColumnLayout {
        Layout.preferredWidth: Kirigami.Units.gridUnit * 24
        Layout.minimumWidth: Kirigami.Units.gridUnit * 18
        spacing: Kirigami.Units.smallSpacing

        PlasmaExtras.Heading { level: 3; text: "Hearth Storage"; Layout.margins: Kirigami.Units.smallSpacing }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !root.stc
            text: "Hearth Storage works through Hearth Link, which isn't running."
            wrapMode: Text.WordWrap; opacity: 0.7
        }
        PlasmaComponents.Switch {
            visible: !!root.stc
            text: "Use this device as Hearth Storage"
            checked: !!root.stc && !!root.stc.server
            onToggled: root.act("storage-set server " + (checked ? "on" : "off"))
        }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !!root.stc
            text: "A home server for your paired Hearth devices: shared folders, games kept for them, their backups and saves. Only paired devices; nothing is opened to the internet."
            wrapMode: Text.WordWrap; opacity: 0.7; font: Kirigami.Theme.smallFont
        }

        PlasmaExtras.Heading { level: 5; text: "Servers & devices"; visible: root.devices.length > 0; Layout.topMargin: Kirigami.Units.smallSpacing }
        PlasmaComponents.Label {
            Layout.fillWidth: true
            visible: !!root.devs && !!root.devs.serverOffline
            text: (root.devs && root.devs.serverName ? root.devs.serverName : "Your Hearth Storage") + " isn't answering (asleep or away)."
            wrapMode: Text.WordWrap; opacity: 0.7
        }
        Repeater {
            model: root.devices
            delegate: ColumnLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: 0
                PlasmaComponents.Label { Layout.fillWidth: true; text: modelData.name + (modelData.server ? " · Hearth Storage" : ""); elide: Text.ElideRight }
                PlasmaComponents.Label { Layout.fillWidth: true; text: root.devLine(modelData); opacity: 0.7; font: Kirigami.Theme.smallFont; elide: Text.ElideRight }
            }
        }

        PlasmaExtras.Heading { level: 5; text: "Shared folders"; visible: !!root.stc && !!root.stc.server && (root.stc.shares || []).length > 0 }
        Repeater {
            model: root.stc && root.stc.server ? (root.stc.shares || []) : []
            delegate: PlasmaComponents.Label {
                required property var modelData
                Layout.fillWidth: true
                text: modelData.name + "  ·  " + modelData.path
                elide: Text.ElideMiddle
            }
        }
        PlasmaComponents.Label { Layout.fillWidth: true; visible: root.message !== ""; text: root.message; wrapMode: Text.WordWrap; color: Kirigami.Theme.neutralTextColor }

        Kirigami.Separator { Layout.fillWidth: true }
        RowLayout {
            Layout.fillWidth: true
            visible: !!root.stc
            PlasmaComponents.Button {
                text: "Back up now"; icon.name: "document-save"
                visible: !!root.stc && !root.stc.server
                onClicked: root.act("backup-now", "Backing up to your Hearth Storage")
            }
            Item { Layout.fillWidth: true }
            PlasmaComponents.ToolButton {
                icon.name: "configure"
                onClicked: { exec.run("hearth-settings link"); root.expanded = false; }
                PlasmaComponents.ToolTip { text: "Hearth Storage settings" }
            }
        }
    }
}
