// Quick Access for Desktop Mode, like the one in Game Mode: volume, brightness, night light, controller
// mouse, Wi-Fi, performance profile, and a few actions. Opens from the taskbar, the Hearth menu, or a
// swipe in from the right edge. Controller: D-pad to move, left/right to change, A to choose, B to close.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower

PanelWindow {
    id: root
    property bool shown: false
    property bool mouseOn: true          // controller mouse (from hearth-pad)
    property bool nightAvailable: false  // wlsunset installed
    signal mouseToggled(bool enable)
    signal keyboardRequested()
    function toggle() { shown = !shown; }

    visible: shown || closing.running
    Timer { id: closing; interval: Theme.calm + 60 }
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-quick"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    onShownChanged: if (shown) { refresh(); r = 0; c = 0; keys.forceActiveFocus(); } else closing.restart()

    // ------------------------------------------------------------------ state read from the system
    PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
    readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
    property int brightness: -1          // percent; -1 = no backlight
    property bool wifiOn: false
    property bool wifiKnown: false
    property string profile: ""          // power-saver | balanced | performance; "" = not available

    function refresh() { brightRead.running = true; wifiRead.running = true; ppdRead.running = true; }
    function run(cmd) { Quickshell.execDetached(cmd); }

    Process {
        id: brightRead
        command: ["brightnessctl", "-m", "-c", "backlight"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim().split(","); root.brightness = f.length >= 4 ? parseInt(f[3]) : -1; } }
        onExited: code => { if (code !== 0) root.brightness = -1; }
    }
    Process {
        id: wifiRead
        command: ["nmcli", "radio", "wifi"]
        stdout: StdioCollector { onStreamFinished: { root.wifiOn = text.trim() === "enabled"; root.wifiKnown = text.trim() !== ""; } }
    }
    Process {
        id: ppdRead
        command: ["powerprofilesctl", "get"]
        stdout: StdioCollector { onStreamFinished: root.profile = text.trim() }
        onExited: code => { if (code !== 0) root.profile = ""; }
    }
    function setBrightness(p) {
        brightness = Math.max(1, Math.min(100, p));
        run(["brightnessctl", "-q", "-c", "backlight", "set", brightness + "%"]);
    }
    function setVolume(v) { if (audio) { audio.muted = false; audio.volume = Math.max(0, Math.min(1, v)); } }
    function setProfile(p) { profile = p; run(["powerprofilesctl", "set", p]); }
    function setWifi(on) { wifiOn = on; run(["nmcli", "radio", "wifi", on ? "on" : "off"]); }

    // ------------------------------------------------------------------ controller / keyboard navigation
    readonly property var navRows: {
        const rows = [["volume"]];
        if (brightness >= 0) rows.push(["brightness"]);
        rows.push(["night", "mouse", "wifi"]);
        if (profile !== "") rows.push(["power-saver", "balanced", "performance"]);
        rows.push(["settings", "keyboard", "game", "sleep"]);
        return rows;
    }
    property int r: 0
    property int c: 0
    readonly property string current: navRows[Math.min(r, navRows.length - 1)][c] || ""

    function move(dr, dc) {
        if (dr) {
            r = Math.max(0, Math.min(navRows.length - 1, r + dr));
            c = Math.min(c, navRows[r].length - 1);
        } else if (current === "volume") {
            setVolume((audio ? audio.volume : 0) + dc * 0.05);
        } else if (current === "brightness") {
            setBrightness(brightness + dc * 5);
        } else {
            c = Math.max(0, Math.min(navRows[r].length - 1, c + dc));
        }
    }
    function activate(id) {
        switch (id) {
        case "volume": if (audio) audio.muted = !audio.muted; break;
        case "night": Settings.nightLight = !Settings.nightLight; break;
        case "mouse": root.mouseToggled(!root.mouseOn); break;
        case "wifi": setWifi(!wifiOn); break;
        case "power-saver": case "balanced": case "performance": setProfile(id); break;
        case "settings": shown = false; run(["sh", "-c", "exec \"${NIMBUS_APP:-nimbus}\" --windowed --settings"]); break;
        case "keyboard": shown = false; root.keyboardRequested(); break;
        case "screenshot": shown = false; run(["sh", "-c", "sleep 0.4; sh \"${NIMBUS_SHARE:-/usr/local/share/nimbus}/desktop/open.sh\" screenshot"]); break;
        case "game": run(["nimbus-mode", "game"]); break;
        case "sleep": shown = false; run(["systemctl", "suspend"]); break;
        }
    }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Up: root.move(-1, 0); break;
            case Qt.Key_Down: root.move(1, 0); break;
            case Qt.Key_Left: root.move(0, -1); break;
            case Qt.Key_Right: root.move(0, 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: root.activate(root.current); break;
            case Qt.Key_Escape: root.shown = false; break;
            default: return;
            }
            event.accepted = true;
        }
    }

    // ------------------------------------------------------------------ look
    MouseArea { anchors.fill: parent; onClicked: root.shown = false }
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.shown ? 0.3 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.calm } }
    }

    Rectangle {
        id: panel
        width: Math.min(400, root.width - 32)
        anchors { top: parent.top; bottom: parent.bottom; topMargin: 12; bottomMargin: 72 }
        x: root.shown ? root.width - width - 12 : root.width + 8
        Behavior on x { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        radius: Theme.radius + 6
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent }   // clicks inside don't close

        Flickable {
            anchors { fill: parent; margins: 18 }
            contentHeight: body.implicitHeight
            clip: true
            ColumnLayout {
                id: body
                width: parent.width
                spacing: 16

                // Header: time and battery
                RowLayout {
                    Layout.fillWidth: true
                    SystemClock { id: clock; precision: SystemClock.Minutes }
                    Text { text: Qt.formatTime(clock.date, "h:mm AP"); color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                    Item { Layout.fillWidth: true }
                    Row {
                        spacing: 6
                        visible: UPower.displayDevice && UPower.displayDevice.isLaptopBattery
                        Icon { anchors.verticalCenter: parent.verticalCenter; name: "battery-good-symbolic"; size: 18 }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: UPower.displayDevice ? Math.round(UPower.displayDevice.percentage * 100) + "%" : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 }
                    }
                }

                SliderRow {
                    navId: "volume"
                    icon: root.audio && root.audio.muted ? "audio-volume-muted-symbolic" : "audio-volume-high-symbolic"
                    label: "Volume"
                    value: root.audio ? (root.audio.muted ? 0 : root.audio.volume) : 0
                    onMoved: v => root.setVolume(v)
                    onIconClicked: root.activate("volume")
                }
                SliderRow {
                    navId: "brightness"
                    visible: root.brightness >= 0
                    icon: "display-brightness-symbolic"
                    label: "Brightness"
                    value: root.brightness / 100
                    onMoved: v => root.setBrightness(Math.round(v * 100))
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: 3
                    columnSpacing: 10
                    Toggle { navId: "night"; icon: "night-light-symbolic"; label: "Night light"; detail: root.nightAvailable ? "Sunset to sunrise" : "Not installed"; lit: Settings.nightLight; enabled: root.nightAvailable }
                    Toggle { navId: "mouse"; icon: "input-gaming-symbolic"; label: "Controller mouse"; detail: root.mouseOn ? "On" : "Off"; lit: root.mouseOn }
                    Toggle { navId: "wifi"; icon: "network-wireless-symbolic"; label: "Wi-Fi"; detail: root.wifiOn ? "On" : "Off"; lit: root.wifiOn; enabled: root.wifiKnown }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.profile !== ""
                    spacing: 8
                    Text { text: "Performance"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Choice { navId: "power-saver"; label: "Battery saver" }
                        Choice { navId: "balanced"; label: "Balanced" }
                        Choice { navId: "performance"; label: "Performance" }
                    }
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: 4
                    columnSpacing: 8
                    Action { navId: "settings"; icon: "emblem-system-symbolic"; label: "Settings" }
                    Action { navId: "keyboard"; icon: "input-keyboard-symbolic"; label: "Keyboard" }
                    Action { navId: "game"; icon: "input-gaming-symbolic"; label: "Game Mode"; accent: true }
                    Action { navId: "sleep"; icon: "weather-clear-night-symbolic"; label: "Sleep" }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks
    component Focusable: Rectangle {
        property string navId
        readonly property bool focused: root.current === navId
        radius: 12
        border.width: 2
        border.color: focused ? Theme.accent : "transparent"
        Behavior on border.color { ColorAnimation { duration: Theme.quick } }
    }

    component SliderRow: Focusable {
        id: s
        property string icon
        property string label
        property real value
        signal moved(real v)
        signal iconClicked()
        Layout.fillWidth: true
        implicitHeight: 64
        color: Theme.panel2
        RowLayout {
            anchors { fill: parent; leftMargin: 14; rightMargin: 16 }
            spacing: 12
            Icon { name: s.icon; size: 22; MouseArea { anchors.fill: parent; onClicked: s.iconClicked() } }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: s.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                    Item { Layout.fillWidth: true }
                    Text { text: Math.round(s.value * 100) + "%"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                }
                Item {
                    id: track
                    Layout.fillWidth: true
                    implicitHeight: 8
                    Rectangle { anchors.fill: parent; radius: 4; color: Theme.panel3 }
                    Rectangle {
                        width: Math.max(8, track.width * Math.max(0, Math.min(1, s.value)))
                        height: parent.height; radius: 4
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop { position: 0; color: Theme.ember }
                            GradientStop { position: 1; color: Theme.flame }
                        }
                    }
                    MouseArea {
                        anchors { fill: parent; topMargin: -14; bottomMargin: -14 }
                        onPressed: m => s.moved(Math.max(0, Math.min(1, m.x / track.width)))
                        onPositionChanged: m => { if (pressed) s.moved(Math.max(0, Math.min(1, m.x / track.width))); }
                    }
                }
            }
        }
    }

    component Toggle: Focusable {
        id: t
        property string icon
        property string label
        property string detail
        property bool lit
        Layout.fillWidth: true
        implicitHeight: 100
        opacity: enabled ? 1 : 0.5
        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22) : Theme.panel2
        Behavior on color { ColorAnimation { duration: Theme.quick } }
        ColumnLayout {
            anchors { fill: parent; margins: 12 }
            spacing: 4
            Icon { name: t.icon; size: 22; color: t.lit ? Theme.accent : Theme.text }
            Item { Layout.fillHeight: true }
            Text { Layout.fillWidth: true; text: t.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; text: t.detail; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
        }
        MouseArea { anchors.fill: parent; enabled: t.enabled; onClicked: root.activate(t.navId) }
    }

    component Choice: Focusable {
        id: ch
        property string label
        readonly property bool lit: root.profile === navId
        Layout.fillWidth: true
        implicitHeight: 44
        color: lit ? Theme.accent : Theme.panel2
        Text { anchors.centerIn: parent; text: ch.label; color: ch.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: ch.lit ? Font.DemiBold : Font.Normal }
        MouseArea { anchors.fill: parent; onClicked: root.activate(ch.navId) }
    }

    component Action: Focusable {
        id: a
        property string icon
        property string label
        property bool accent: false
        Layout.fillWidth: true
        implicitHeight: 76
        color: accent ? Theme.accent : Theme.panel2
        ColumnLayout {
            anchors.centerIn: parent
            spacing: 6
            Icon { Layout.alignment: Qt.AlignHCenter; name: a.icon; size: 22; color: a.accent ? Theme.accentInk : Theme.text }
            Text { Layout.alignment: Qt.AlignHCenter; text: a.label; color: a.accent ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
        }
        MouseArea { anchors.fill: parent; onClicked: root.activate(a.navId) }
    }
}
