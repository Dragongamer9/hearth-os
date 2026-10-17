// The Nimbus taskbar: launcher button, open apps, then tray, volume, network, battery, clock and a
// Game Mode button. A floating rounded bar along the bottom of each screen; windows don't cover it.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower

PanelWindow {
    id: bar
    signal launcherRequested()

    anchors { bottom: true; left: true; right: true }
    implicitHeight: 60
    exclusiveZone: implicitHeight
    color: "transparent"
    WlrLayershell.namespace: "nimbus-bar"
    WlrLayershell.layer: WlrLayer.Top

    Rectangle {
        id: body
        anchors { fill: parent; leftMargin: 8; rightMargin: 8; bottomMargin: 6; topMargin: 4 }
        radius: Theme.radius
        color: Theme.panel
        border.color: Theme.line
        border.width: 1

        RowLayout {
            anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
            spacing: 6

            // Launcher (the Hearth mark)
            BarButton {
                tip: "Apps (Super)"
                onClicked: bar.launcherRequested()
                Image {
                    anchors.centerIn: parent
                    width: 32; height: 32
                    source: Qt.resolvedUrl("hearth-logo.png")
                    smooth: true; mipmap: true
                }
            }

            Rectangle { width: 1; Layout.fillHeight: true; Layout.topMargin: 12; Layout.bottomMargin: 12; color: Theme.line }

            // Open windows
            Repeater {
                model: ToplevelManager.toplevels
                delegate: BarButton {
                    id: task
                    required property Toplevel modelData
                    readonly property var entry: DesktopEntries.heuristicLookup(modelData.appId)
                    tip: modelData.title || modelData.appId
                    active: modelData.activated
                    // Click: focus it, or minimize it if it's already in front. Middle-click closes.
                    onClicked: modelData.activated ? (modelData.minimized = true) : modelData.activate()
                    onMiddleClicked: modelData.close()
                    IconImage {
                        anchors.centerIn: parent
                        implicitSize: 28
                        source: Quickshell.iconPath(task.entry ? task.entry.icon : task.modelData.appId, "application-x-executable")
                    }
                }
            }

            Item { Layout.fillWidth: true }

            // Tray icons (apps like Discord, Steam)
            Repeater {
                model: SystemTray.items
                delegate: BarButton {
                    id: tray
                    required property SystemTrayItem modelData
                    narrow: true
                    tip: modelData.tooltipTitle || modelData.title
                    onClicked: modelData.activate()
                    onRightClicked: {
                        if (!modelData.hasMenu) return modelData.secondaryActivate();
                        const p = tray.mapToItem(null, 0, 0);
                        modelData.display(bar, p.x, p.y - 8);
                    }
                    IconImage { anchors.centerIn: parent; implicitSize: 20; source: tray.modelData.icon }
                }
            }

            // Volume: click mutes, scroll changes it
            PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
            BarButton {
                id: vol
                readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
                readonly property int pct: audio ? Math.round(audio.volume * 100) : 0
                visible: audio !== null
                wide: true
                tip: audio && audio.muted ? "Muted" : "Volume " + pct + "%"
                onClicked: if (audio) audio.muted = !audio.muted
                onScrolled: delta => { if (audio) audio.volume = Math.max(0, Math.min(1, audio.volume + (delta > 0 ? 0.05 : -0.05))); }
                StatusLabel {
                    icon: !vol.audio || vol.audio.muted ? "audio-volume-muted-symbolic" : vol.pct > 66 ? "audio-volume-high-symbolic" : vol.pct > 33 ? "audio-volume-medium-symbolic" : "audio-volume-low-symbolic"
                    label: vol.audio && vol.audio.muted ? "" : vol.pct + "%"
                }
            }

            // Network, read from NetworkManager every 10 seconds
            BarButton {
                id: net
                property string kind: ""     // wifi | ethernet | ""
                property string name: ""
                wide: true
                tip: name ? "Connected to " + name : "Not connected"
                onClicked: Quickshell.execDetached(["sh", "-c", "command -v nm-connection-editor >/dev/null && exec nm-connection-editor"])
                StatusLabel {
                    icon: net.kind === "wifi" ? "network-wireless-signal-good-symbolic" : net.kind === "ethernet" ? "network-wired-symbolic" : "network-offline-symbolic"
                    label: ""
                }
                Process {
                    id: nmcli
                    command: ["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"]
                    stdout: StdioCollector {
                        onStreamFinished: {
                            let k = "", n = "";
                            for (const line of text.split("\n")) {
                                const f = line.split(":");
                                if (f[1] === "connected" && (f[0] === "wifi" || f[0] === "ethernet") && !k) { k = f[0]; n = f.slice(2).join(":"); }
                            }
                            net.kind = k; net.name = n;
                        }
                    }
                }
                Timer { interval: 10000; running: true; repeat: true; triggeredOnStart: true; onTriggered: nmcli.running = true }
            }

            // Battery (laptops and handhelds only)
            BarButton {
                id: batt
                readonly property var dev: UPower.displayDevice
                readonly property int pct: dev ? Math.round(dev.percentage * 100) : 0
                readonly property bool charging: dev && dev.state === UPowerDeviceState.Charging
                visible: dev && dev.isLaptopBattery
                wide: true
                tip: pct + "%" + (charging ? ", charging" : "")
                StatusLabel {
                    icon: batt.charging ? "battery-good-charging-symbolic" : batt.pct > 60 ? "battery-good-symbolic" : batt.pct > 20 ? "battery-low-symbolic" : "battery-caution-symbolic"
                    label: batt.pct + "%"
                }
            }

            // Clock
            SystemClock { id: clock; precision: SystemClock.Minutes }
            Column {
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Text { anchors.right: parent.right; text: Qt.formatTime(clock.date, "h:mm AP"); color: Theme.text; font.pixelSize: 14; font.bold: true; font.family: Theme.font }
                Text { anchors.right: parent.right; text: Qt.formatDate(clock.date, "ddd, MMM d"); color: Theme.muted; font.pixelSize: 11; font.family: Theme.font }
            }

            // Back to Game Mode
            Rectangle {
                implicitWidth: gm.implicitWidth + 28
                implicitHeight: 36
                radius: 10
                color: gmArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
                Text { id: gm; anchors.centerIn: parent; text: "Game Mode"; color: Theme.accentInk; font.bold: true; font.pixelSize: 13; font.family: Theme.font }
                MouseArea { id: gmArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Quickshell.execDetached(["nimbus-mode", "game"]) }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks

    component BarButton: Item {
        id: btn
        property string tip: ""
        property bool active: false
        property bool narrow: false
        property bool wide: false
        default property alias content: holder.data
        signal clicked()
        signal middleClicked()
        signal rightClicked()
        signal scrolled(int delta)

        implicitWidth: narrow ? 30 : wide ? Math.max(44, holder.childrenRect.width + 16) : 44
        implicitHeight: 44
        Rectangle {
            anchors.fill: parent
            radius: 10
            color: area.containsMouse ? Theme.panel3 : btn.active ? Theme.panel2 : "transparent"
        }
        Item { id: holder; anchors.fill: parent }
        Rectangle { // running-and-focused indicator
            visible: btn.active
            anchors { bottom: parent.bottom; horizontalCenter: parent.horizontalCenter; bottomMargin: 2 }
            width: 16; height: 3; radius: 2
            color: Theme.accent
        }
        MouseArea {
            id: area
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
            onClicked: mouse => {
                if (mouse.button === Qt.MiddleButton) btn.middleClicked();
                else if (mouse.button === Qt.RightButton) btn.rightClicked();
                else btn.clicked();
            }
            onWheel: wheel => btn.scrolled(wheel.angleDelta.y)
        }
        // Hover label above the bar
        PopupWindow {
            visible: area.containsMouse && btn.tip !== ""
            anchor.item: btn
            anchor.rect.x: btn.width / 2 - implicitWidth / 2
            anchor.rect.y: -implicitHeight - 10
            implicitWidth: tipText.implicitWidth + 20
            implicitHeight: tipText.implicitHeight + 12
            color: "transparent"
            Rectangle {
                anchors.fill: parent
                radius: 8
                color: Theme.panel2
                border.color: Theme.line
                Text { id: tipText; anchors.centerIn: parent; text: btn.tip; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
            }
        }
    }

    component StatusLabel: Row {
        property string icon
        property string label
        anchors.centerIn: parent
        spacing: 6
        IconImage { anchors.verticalCenter: parent.verticalCenter; implicitSize: 18; source: Quickshell.iconPath(parent.icon, true) }
        Text { anchors.verticalCenter: parent.verticalCenter; visible: text !== ""; text: parent.label; color: Theme.text; font.pixelSize: 13; font.family: Theme.font }
    }
}
