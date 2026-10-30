// Hearth Plasma: Quick Access, ported from Hearth Classic's (Mantel's QuickAccess.qml): volume, brightness, Wi-Fi,
// the power mode and a few actions (Settings, screenshot, Game Mode, sleep). Read and changed through
// desktop/plasma/hearth-plasma quick / quick-set, which use the same tools as Mantel (wpctl, brightnessctl, nmcli,
// hearth-power).
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.plasma.plasmoid
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root
    Plasmoid.icon: "hearth-settings"
    toolTipMainText: "Quick Access"
    toolTipSubText: "Volume, brightness, Wi-Fi and the power mode"

    readonly property string helper: "/usr/share/nimbus/desktop/plasma/hearth-plasma"
    property var st: ({ volume: null, muted: false, brightness: null, wifi: null, profile: "" })

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
        function run(cmd, cb) {
            const src = cmd + " #" + (++n);
            callbacks[src] = cb || null;
            connectSource(src);
        }
    }
    function refresh() { exec.run("python3 " + helper + " quick", out => { const s = lastJson(out, null); if (s) root.st = s; }); }
    function set(key, val) { exec.run("python3 " + helper + " quick-set " + key + " " + val, () => refresh()); }
    function action(cmd) { exec.run(cmd); root.expanded = false; }
    onExpandedChanged: if (expanded) refresh()
    Component.onCompleted: refresh()

    fullRepresentation: ColumnLayout {
        Layout.preferredWidth: Kirigami.Units.gridUnit * 22
        Layout.minimumWidth: Kirigami.Units.gridUnit * 18
        spacing: Kirigami.Units.largeSpacing

        PlasmaExtras.Heading { level: 3; text: "Quick Access"; Layout.margins: Kirigami.Units.smallSpacing }

        // Volume: the icon mutes, the slider sets it (up to 150 %, as in Mantel).
        RowLayout {
            Layout.fillWidth: true
            visible: root.st.volume !== null
            PlasmaComponents.ToolButton {
                icon.name: root.st.muted ? "audio-volume-muted" : "audio-volume-high"
                onClicked: root.set("mute", "toggle")
                PlasmaComponents.ToolTip { text: root.st.muted ? "Unmute" : "Mute" }
            }
            PlasmaComponents.Slider {
                id: vol
                Layout.fillWidth: true
                from: 0; to: 150; stepSize: 5
                value: root.st.volume || 0
                onMoved: volTimer.restart()
                Timer { id: volTimer; interval: 120; onTriggered: root.set("volume", Math.round(vol.value)) }
            }
            PlasmaComponents.Label { text: Math.round(vol.value) + "%"; Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5 }
        }
        // Brightness (screens with a backlight only).
        RowLayout {
            Layout.fillWidth: true
            visible: root.st.brightness !== null
            Kirigami.Icon { source: "brightness-high"; Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium; Layout.preferredHeight: Layout.preferredWidth; Layout.margins: Kirigami.Units.smallSpacing }
            PlasmaComponents.Slider {
                id: bright
                Layout.fillWidth: true
                from: 1; to: 100; stepSize: 5
                value: root.st.brightness || 50
                onMoved: brightTimer.restart()
                Timer { id: brightTimer; interval: 120; onTriggered: root.set("brightness", Math.round(bright.value)) }
            }
            PlasmaComponents.Label { text: Math.round(bright.value) + "%"; Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5 }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.st.wifi !== null
            PlasmaComponents.Switch {
                text: "Wi-Fi"
                checked: !!root.st.wifi
                onToggled: root.set("wifi", checked ? "on" : "off")
            }
        }

        // The power mode (power-profiles-daemon, SteamOS Manager or Handheld Daemon, through hearth-power).
        PlasmaComponents.Label { visible: root.st.profile !== ""; text: "Power mode"; opacity: 0.7 }
        RowLayout {
            Layout.fillWidth: true
            visible: root.st.profile !== ""
            Repeater {
                model: [{ id: "power-saver", label: "Saver" }, { id: "balanced", label: "Balanced" }, { id: "performance", label: "Performance" }]
                delegate: PlasmaComponents.Button {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData.label
                    checkable: true
                    checked: root.st.profile === modelData.id
                    onClicked: root.set("profile", modelData.id)
                }
            }
        }

        Kirigami.Separator { Layout.fillWidth: true }
        GridLayout {
            Layout.fillWidth: true
            columns: 4
            PlasmaComponents.ToolButton { Layout.fillWidth: true; display: QQC2.AbstractButton.TextUnderIcon; icon.name: "hearth-settings"; text: "Settings"; onClicked: root.action("hearth-settings") }
            PlasmaComponents.ToolButton { Layout.fillWidth: true; display: QQC2.AbstractButton.TextUnderIcon; icon.name: "camera-photo"; text: "Screenshot"; onClicked: root.action("sleep 0.5; spectacle -r -b -n 2>/dev/null || spectacle -r") }
            PlasmaComponents.ToolButton { Layout.fillWidth: true; display: QQC2.AbstractButton.TextUnderIcon; icon.name: "input-gamepad"; text: "Game Mode"; onClicked: root.action("nimbus-mode game") }
            PlasmaComponents.ToolButton { Layout.fillWidth: true; display: QQC2.AbstractButton.TextUnderIcon; icon.name: "system-suspend"; text: "Sleep"; onClicked: root.action("systemctl suspend") }
        }
    }
}
