// The Hearth taskbar: launcher button, open apps, then tray, volume, network, battery, clock, Quick Access,
// keyboard and a Game Mode button. A floating rounded bar along the bottom of each screen (down the left
// side when the screen is upright); windows don't cover it.
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
    signal quickRequested()
    signal keyboardRequested()

    // Upright screen (a handheld held vertically, a rotated monitor): the bar goes down the left side.
    readonly property bool portrait: screen && screen.height > screen.width
    readonly property int thickness: 60

    anchors { bottom: true; left: true; right: !portrait; top: portrait }
    implicitHeight: portrait ? 0 : thickness
    implicitWidth: portrait ? thickness : 0
    exclusiveZone: thickness
    color: "transparent"
    WlrLayershell.namespace: "hearth-bar"
    WlrLayershell.layer: WlrLayer.Top

    Rectangle {
        id: body
        anchors {
            fill: parent
            leftMargin: bar.portrait ? 6 : 8
            rightMargin: bar.portrait ? 4 : 8
            bottomMargin: bar.portrait ? 8 : 6
            topMargin: bar.portrait ? 8 : 4
        }
        radius: Theme.radius
        color: Theme.panel
        border.color: Theme.line
        border.width: 1

        GridLayout {
            anchors {
                fill: parent
                leftMargin: bar.portrait ? 0 : 8
                rightMargin: bar.portrait ? 0 : 8
                topMargin: bar.portrait ? 8 : 0
                bottomMargin: bar.portrait ? 8 : 0
            }
            flow: bar.portrait ? GridLayout.TopToBottom : GridLayout.LeftToRight
            rows: bar.portrait ? -1 : 1
            columns: bar.portrait ? 1 : -1
            rowSpacing: 6
            columnSpacing: 6

            // Launcher (the Hearth mark)
            BarButton {
                tip: "Apps (Super)"
                onClicked: bar.launcherRequested()
                Image {
                    anchors.centerIn: parent
                    width: 32; height: 32
                    source: Qt.resolvedUrl("images/hearth-logo.png")
                    smooth: true; mipmap: true
                }
            }

            Separator {}

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
                        // Hearth Settings is part of this shell (its app id is Quickshell's): show the Hearth mark.
                        source: (task.modelData.title === "Hearth Settings" || task.modelData.title === "Task Manager" || task.modelData.title === "Files") ? Qt.resolvedUrl("images/hearth-logo.png")
                            : Quickshell.iconPath(task.entry ? task.entry.icon : task.modelData.appId, "application-x-executable")
                    }
                }
            }

            Item { Layout.fillWidth: !bar.portrait; Layout.fillHeight: bar.portrait }

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
                wide: !bar.portrait
                tip: audio && audio.muted ? "Muted" : "Volume " + pct + "%"
                onClicked: if (audio) audio.muted = !audio.muted
                onScrolled: delta => { if (audio) audio.volume = Math.max(0, Math.min(Buttons.maxVolume, audio.volume + (delta > 0 ? 0.05 : -0.05))); }
                StatusLabel {
                    icon: !vol.audio || vol.audio.muted ? "audio-volume-muted-symbolic" : vol.pct > 66 ? "audio-volume-high-symbolic" : vol.pct > 33 ? "audio-volume-medium-symbolic" : "audio-volume-low-symbolic"
                    label: bar.portrait || (vol.audio && vol.audio.muted) ? "" : vol.pct + "%"
                }
            }

            // Network, read from NetworkManager every 10 seconds
            BarButton {
                id: net
                property string kind: ""     // wifi | ethernet | ""
                property string name: ""
                tip: name ? "Connected to " + name : "Not connected"
                onClicked: bar.quickRequested()
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
                wide: !bar.portrait
                tip: pct + "%" + (charging ? ", charging" : "")
                onClicked: bar.quickRequested()
                StatusLabel {
                    icon: batt.charging ? "battery-good-charging-symbolic" : batt.pct > 60 ? "battery-good-symbolic" : batt.pct > 20 ? "battery-low-symbolic" : "battery-caution-symbolic"
                    label: bar.portrait ? "" : batt.pct + "%"
                }
            }

            // Quick Access and the on-screen keyboard (handy on touchscreens)
            BarButton {
                tip: "Quick Access"
                onClicked: bar.quickRequested()
                Icon { anchors.centerIn: parent; name: "open-menu-symbolic"; size: 20 }
            }
            BarButton {
                tip: "Keyboard"
                onClicked: bar.keyboardRequested()
                Icon { anchors.centerIn: parent; name: "input-keyboard-symbolic"; size: 20 }
            }

            // Clock
            SystemClock { id: clock; precision: SystemClock.Minutes }
            Column {
                Layout.leftMargin: bar.portrait ? 0 : 6
                Layout.rightMargin: bar.portrait ? 0 : 6
                Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                Text {
                    anchors.horizontalCenter: bar.portrait ? parent.horizontalCenter : undefined
                    anchors.right: bar.portrait ? undefined : parent.right
                    text: bar.portrait ? Qt.formatTime(clock.date, "h:mm") : Qt.formatTime(clock.date, "h:mm AP")
                    color: Theme.text; font.pixelSize: bar.portrait ? 13 : 14; font.bold: true; font.family: Theme.font
                }
                Text {
                    visible: !bar.portrait
                    anchors.right: parent.right
                    text: Qt.formatDate(clock.date, "ddd, MMM d")
                    color: Theme.muted; font.pixelSize: 11; font.family: Theme.font
                }
            }

            // Back to Game Mode
            Rectangle {
                Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                implicitWidth: bar.portrait ? 44 : gm.implicitWidth + 28
                implicitHeight: bar.portrait ? 44 : 36
                radius: 10
                color: gmArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
                Text { id: gm; visible: !bar.portrait; anchors.centerIn: parent; text: "Game Mode"; color: Theme.accentInk; font.bold: true; font.pixelSize: 13; font.family: Theme.font }
                Icon { visible: bar.portrait; anchors.centerIn: parent; name: "input-gaming-symbolic"; size: 22; color: Theme.accentInk }
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

        implicitWidth: narrow ? 30 : wide ? Math.max(44, (holder.children.length ? holder.children[0].implicitWidth : 0) + 20) : 44
        implicitHeight: narrow && bar.portrait ? 30 : 44
        Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
        Rectangle {
            anchors.fill: parent
            radius: 10
            color: area.containsMouse ? Theme.panel3 : btn.active ? Theme.panel2 : "transparent"
        }
        Item { id: holder; anchors.fill: parent }
        Rectangle { // running-and-focused indicator: under the icon, or beside it on a side bar
            visible: btn.active
            x: bar.portrait ? 1 : (parent.width - width) / 2
            y: bar.portrait ? (parent.height - height) / 2 : parent.height - height - 2
            width: bar.portrait ? 3 : 16
            height: bar.portrait ? 16 : 3
            radius: 2
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
            anchor.rect.x: bar.portrait ? btn.width + 12 : btn.width / 2 - implicitWidth / 2
            anchor.rect.y: bar.portrait ? btn.height / 2 - implicitHeight / 2 : -implicitHeight - 10
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

    component Separator: Rectangle {
        Layout.preferredWidth: bar.portrait ? 28 : 1
        Layout.preferredHeight: bar.portrait ? 1 : 24
        Layout.alignment: Qt.AlignCenter
        color: Theme.line
    }

    component StatusLabel: Row {
        property string icon
        property string label
        anchors.centerIn: parent
        spacing: 6
        Icon { anchors.verticalCenter: parent.verticalCenter; size: 18; name: parent.icon }
        Text { anchors.verticalCenter: parent.verticalCenter; visible: text !== ""; text: parent.label; color: Theme.text; font.pixelSize: 13; font.family: Theme.font }
    }
}
