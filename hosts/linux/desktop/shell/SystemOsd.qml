// The small messages KDE's OSD shows, written for Hearth (1.14), the same on KWin and labwc: Caps Lock and Num Lock
// on or off (the keyboards' LEDs, hearth-notifyd), the keyboard layout as it switches (KWin's layouts; on labwc the
// ones Settings › Keyboard lists), the microphone muted or on, and "an app wants the microphone, but it's muted"
// (Pipewire: a recording stream while the default microphone is muted). Like the volume bar (VolumeOsd.qml): at the
// top, clicks go through, never takes the keyboard. Also plays the system sounds for these events (Sounds.qml):
// drives plugged in and out, the charger, the volume keys, Caps Lock.
// Loaded by shell.qml by file name: a problem here only loses these messages.
//   quickshell ipc -p <shell folder> call osd show caps|num|layout|mic|micwanted [on|off|text]
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.UPower

PanelWindow {
    id: root
    property string icon: "kbd"
    property string big: ""                // a short word in a box (the layout: "DE")
    property string line: ""
    property string detail: ""
    property bool alert: false
    property bool showing: false
    visible: showing || card.opacity > 0
    color: "transparent"
    anchors { top: true }
    margins.top: Math.max(28, DeskState.topInset + 16)
    implicitWidth: Math.max(300, row.implicitWidth + 48)
    implicitHeight: Math.max(64, row.implicitHeight + 20)
    exclusiveZone: 0
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-osd"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    mask: Region {}                        // clicks go through

    function show(ic, text, sub, bigText, isAlert, ms) {
        icon = ic; line = text; detail = sub || ""; big = bigText || ""; alert = !!isAlert;
        showing = true; hide.interval = ms || 1500; hide.restart();
    }
    Timer { id: hide; interval: 1500; onTriggered: root.showing = false }

    Connections {
        target: NotifyHub
        function onCapsToggled(on) { root.show("kbd", on ? "Caps Lock on" : "Caps Lock off", "", on ? "A" : "a"); Sounds.play("bell"); }
        function onNumToggled(on) { root.show("kbd", on ? "Num Lock on" : "Num Lock off", "", on ? "1" : ""); }
        function onLayoutSwitched(name, shortName) { root.show("kbd", name, "Keyboard layout", shortName); }
        function onMicMutedChanged() { if (NotifyHub.started && NotifyHub.mic) root.show("mic", NotifyHub.micMuted ? "Microphone muted" : "Microphone on", "", "", NotifyHub.micMuted); }
        function onMicAppsChanged() {
            const now = NotifyHub.micApps;
            const fresh = now.filter(a => root.lastMic.indexOf(a) < 0);
            root.lastMic = now;
            if (NotifyHub.started && NotifyHub.micMuted && fresh.length)
                root.show("mic", "The microphone is muted", fresh.join(", ") + (fresh.length === 1 ? " wants to use it. " : " want to use it. ") + "Press the mic key or the tray's camera and mic button to unmute", "", true, 5000);
        }
        // Drives plugged in and out (the tray's disks): their sounds
        function onDisksChanged() {
            const n = NotifyHub.disks.length;
            if (NotifyHub.started && root.lastDisks >= 0 && n !== root.lastDisks) Sounds.play(n > root.lastDisks ? "device-added" : "device-removed");
            root.lastDisks = n;
        }
    }
    property var lastMic: []
    property int lastDisks: -1
    // The charger (UPower) and the volume keys (Buttons.qml's bar): their sounds
    readonly property bool onBattery: UPower.onBattery
    property bool powerKnown: false
    onOnBatteryChanged: if (powerKnown && UPower.displayDevice && UPower.displayDevice.isLaptopBattery) Sounds.play(onBattery ? "power-unplug" : "power-plug")
    Timer { running: true; interval: 4000; onTriggered: root.powerKnown = true }
    Connections { target: Buttons; function onOsd(v, m) { if (!volClick.running) Sounds.play("audio-volume-change"); volClick.restart(); } }
    Timer { id: volClick; interval: 120 }

    Rectangle {
        id: card
        anchors.fill: parent
        radius: Math.min(height / 2, 32)
        color: Theme.glass
        border.color: root.alert ? Theme.danger : Theme.line
        border.width: root.alert ? 2 : 1
        opacity: root.showing ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        RowLayout {
            id: row
            anchors { fill: parent; leftMargin: 20; rightMargin: 24 }
            spacing: 14
            Rectangle {
                visible: root.big !== ""
                implicitWidth: Math.max(34, bigText.implicitWidth + 14); implicitHeight: 34
                radius: Theme.r(8)
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2)
                border.color: Theme.accent
                Text { id: bigText; anchors.centerIn: parent; text: root.big; color: Theme.accent; font.family: Theme.font; font.pixelSize: 17; font.bold: true }
            }
            Icon { visible: root.big === ""; name: root.icon; size: 22; color: root.alert ? Theme.danger : Theme.text }
            ColumnLayout {
                spacing: 1
                Text { text: root.line; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Text { visible: text !== ""; Layout.maximumWidth: 520; text: root.detail; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
    }

    IpcHandler {
        target: "osd"
        function show(kind: string, value: string): void {
            if (kind === "caps") NotifyHub.capsToggled(value !== "off");
            else if (kind === "num") NotifyHub.numToggled(value !== "off");
            else if (kind === "layout") NotifyHub.layoutSwitched(value || "German", (value || "de").slice(0, 2).toUpperCase());
            else if (kind === "mic") root.show("mic", value === "off" ? "Microphone on" : "Microphone muted", "", "", value !== "off");
            else if (kind === "micwanted") root.show("mic", "The microphone is muted", (value || "Hearth Call") + " wants to use it. Press the mic key or the tray's camera and mic button to unmute", "", true, 5000);
        }
    }
}
