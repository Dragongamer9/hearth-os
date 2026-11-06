// Quick settings for Desktop Mode (Mantel 2026; was Quick Access): a header with your battery and time left and
// Settings / Lock / Power, a big Return to Game Mode button, tiles (Wi-Fi, Bluetooth, Night light, Controller mouse,
// Do not disturb, Keyboard; Wi-Fi and Bluetooth with a chevron to their lists), pill sliders for volume (its chevron
// picks the output) and brightness, the performance profile, Handheld settings and the weather. Sleep is in Power.
// Opens from the status island, the Hearth menu, ☰, or a swipe in from the right edge.
// Controller: D-pad to move, left/right to change, A to choose, B to close.
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
    signal settingsRequested()
    signal pageRequested(string page)    // a page of Hearth Settings (Handheld: Handheld Daemon's settings)
    signal lockRequested()
    signal powerRequested()
    function toggle() { shown = !shown; }

    visible: shown || closing.running
    Timer { id: closing; interval: Theme.calm + 60 }
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-quick"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    onShownChanged: if (shown) { refresh(); r = 1; c = 0; keys.forceActiveFocus(); } else closing.restart()   // (row 1: Return to Game Mode)

    // ------------------------------------------------------------------ state read from the system
    PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
    readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
    property int brightness: -1          // percent; -1 = no backlight
    property bool wifiOn: false
    property bool wifiKnown: false
    property string profile: ""          // power-saver | balanced | performance; "" = not available

    function refresh() { brightRead.running = true; wifiRead.running = true; ppdRead.running = true; weatherRead.running = true; hhdRead.running = true; btRead.running = true; }
    // Bluetooth (BlueZ through bluetoothctl, as the bar and Settings › Bluetooth): on/off and what's connected.
    property bool btPresent: false
    property bool btPowered: false
    property var btConnected: []
    Process {
        id: btRead
        command: ["sh", "-c", "command -v bluetoothctl >/dev/null || exit 3; bluetoothctl --timeout 3 show 2>/dev/null | grep -E '^Controller|Powered:'; echo ---; bluetoothctl --timeout 3 devices Connected 2>/dev/null"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("---");
                root.btPresent = /Controller/.test(parts[0] || "");
                root.btPowered = /Powered:\s*yes/.test(parts[0] || "");
                root.btConnected = (parts[1] || "").split("\n").map(l => l.match(/^Device\s+\S+\s+(.+)$/)).filter(Boolean).map(m => m[1].trim());
            }
        }
    }
    function setBt(on) { btPowered = on; btSet.command = ["bluetoothctl", "power", on ? "on" : "off"]; btSet.running = true; }
    Process { id: btSet; onExited: btRead.running = true }
    // Battery and time left (UPower).
    readonly property var battDev: UPower.displayDevice
    readonly property bool hasBatt: battDev && battDev.isLaptopBattery
    function hm(sec) { const m = Math.round(sec / 60); return m >= 60 ? Math.floor(m / 60) + " h " + (m % 60) + " min" : m + " min"; }
    readonly property string battLine: {
        if (!hasBatt) return "";
        const pct = Math.round(battDev.percentage * 100) + "%";
        if (battDev.state === UPowerDeviceState.Charging) return pct + " · charging" + (battDev.timeToFull > 0 ? ", full in " + hm(battDev.timeToFull) : "");
        return pct + (battDev.timeToEmpty > 0 ? " · about " + hm(battDev.timeToEmpty) + " left" : "");
    }
    // Handheld Daemon running (its API socket, or its token for the local port): a way to its settings (Settings › Handheld).
    property bool hhdOn: false
    Process { id: hhdRead; command: ["sh", "-c", "test -S /run/hhd/api || test -s /tmp/hhd/token"]; onExited: code => root.hhdOn = code === 0 }
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
    readonly property string powerHelper: decodeURIComponent(Qt.resolvedUrl("hearth-power").toString().replace(/^file:\/\//, ""))
    Process {
        id: ppdRead
        command: ["python3", root.powerHelper, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { const r = JSON.parse(text.trim().split("\n").pop()); root.profile = r.mode && r.mode.supported ? r.mode.active : ""; }
                catch (e) { root.profile = ""; }
            }
        }
    }
    // ---- Hearth Weather (hosts/hearth-time): the weather in a few words for the place chosen in the Weather app
    // (`hearth-time --weather-summary`: {} when no place is chosen or the app isn't installed). Opens Weather.
    property var weather: ({})
    Process {
        id: weatherRead
        running: true
        command: ["hearth-time", "--weather-summary"]
        stdout: StdioCollector { onStreamFinished: { try { root.weather = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { root.weather = {}; } } }
    }
    // ---- end of Hearth Weather
    function setBrightness(p) {
        brightness = Math.max(1, Math.min(100, p));
        run(["brightnessctl", "-q", "-c", "backlight", "set", brightness + "%"]);
    }
    function setVolume(v) { if (audio) { audio.muted = false; audio.volume = Math.max(0, Math.min(Buttons.maxVolume, v)); } }
    function setProfile(p) { profile = p; run(["python3", powerHelper, "mode", p]); }
    function setWifi(on) { wifiOn = on; run(["nmcli", "radio", "wifi", on ? "on" : "off"]); }

    // ------------------------------------------------------------------ controller / keyboard navigation
    readonly property var navRows: {
        const rows = [["settings", "lock", "power"], ["game"], ["wifi", "bluetooth"], ["night", "mouse"], ["dnd", "keyboard"], ["volume"]];
        if (brightness >= 0) rows.push(["brightness"]);
        if (profile !== "") rows.push(["power-saver", "balanced", "performance"]);
        const last = [];
        if (hhdOn) last.push("handheld");
        if (weather.line) last.push("weather");   // Hearth Weather
        if (last.length) rows.push(last);
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
        case "bluetooth": if (btPresent) setBt(!btPowered); else { shown = false; root.pageRequested("bluetooth"); } break;
        case "dnd": DeskState.setDnd(!DeskState.dnd); break;
        case "lock": shown = false; root.lockRequested(); break;
        case "power": shown = false; root.powerRequested(); break;
        case "friends": shown = false; if (SettingsNav.discord) SettingsNav.discord.toggle(-1); break;
        case "power-saver": case "balanced": case "performance": setProfile(id); break;
        case "settings": shown = false; root.settingsRequested(); break;
        case "handheld": shown = false; root.pageRequested("handheld"); break;
        case "keyboard": shown = false; root.keyboardRequested(); break;
        case "screenshot": shown = false; run(["sh", "-c", "sleep 0.4; sh \"${NIMBUS_SHARE:-/usr/local/share/nimbus}/desktop/open.sh\" screenshot"]); break;
        case "game": run(["nimbus-mode", "game"]); break;
        case "sleep": shown = false; Buttons.sleep("Quick Access"); break;
        case "weather": shown = false; run(["hearth-weather"]); break;   // Hearth Weather
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
            case Qt.Key_Menu: if (root.current === "wifi" || root.current === "bluetooth") { root.shown = false; root.pageRequested(root.current); } else if (root.current === "volume") { root.shown = false; root.pageRequested("sound"); } break;
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

    Item {
        id: panel
        width: Math.min(460, root.width - 32)
        // Next to the main bar, wherever it is (Settings › Appearance › Bar layout): from the top right under a top
        // bar, from the bottom right over a bottom bar, beside a left or right bar.
        readonly property bool fromLeft: DeskState.barEdge === "left" && !DeskState.classicBar
        readonly property bool fromBottom: DeskState.barEdge === "bottom" && !DeskState.classicBar
        readonly property int topY: DeskState.topInset > 0 ? DeskState.topInset + 2 : 12
        readonly property int room: root.height - topY - DeskState.bottomInset - 12
        height: Math.min(room, body.implicitHeight + 40)
        y: fromBottom ? root.height - DeskState.bottomInset - height - 2 : topY
        x: fromLeft ? (root.shown ? DeskState.leftInset + 12 : -width - 8)
            : (root.shown ? root.width - width - 12 - DeskState.rightInset : root.width + 8)
        Behavior on x { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        MouseArea { anchors.fill: parent }   // clicks inside don't close
        // The look's panel (Frame.qml, loaded on its own); a plain one underneath in case it can't load.
        Rectangle { anchors.fill: parent; radius: Theme.radius + 8; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, Math.max(0.94, Theme.glassAlpha)); border.color: Theme.line; visible: frame.status !== Loader.Ready }
        Loader { id: frame; anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.radius + 8); } }

        Flickable {
            anchors { fill: parent; margins: 20 }
            contentHeight: body.implicitHeight
            clip: true
            ColumnLayout {
                id: body
                width: parent.width
                spacing: 12

                // ---- header: you, the battery and time left, Settings / Lock / Power
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Rectangle {
                        implicitWidth: 46; implicitHeight: 46; radius: 23
                        color: Theme.accent
                        Icon { anchors.centerIn: parent; name: "user"; size: 22; color: Theme.accentInk }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        Text { Layout.fillWidth: true; text: Quickshell.env("USER") || "You"; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                        Text { Layout.fillWidth: true; text: root.battLine || Qt.formatDate(new Date(), "dddd, d MMMM"); elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }
                    RoundBtn { navId: "friends"; icon: "chat"; visible: root.screen ? root.screen.width <= 1440 : false; tip: "Discord" }
                    RoundBtn { navId: "settings"; icon: "gear"; tip: "Settings" }
                    RoundBtn { navId: "lock"; icon: "lock"; tip: "Lock" }
                    RoundBtn { navId: "power"; icon: "power"; tip: "Power (sleep, restart, shut down)" }
                }

                // ---- Return to Game Mode: first and biggest
                Focusable {
                    navId: "game"
                    Layout.fillWidth: true
                    implicitHeight: 72
                    color: gma.containsMouse ? Qt.lighter(Theme.accent, 1.08) : Theme.accent
                    border.color: focused ? Theme.text : "transparent"
                    RowLayout {
                        anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                        spacing: 14
                        Icon { name: "pad"; size: 28; color: Theme.accentInk }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text { text: "Return to Game Mode"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.Bold }
                            Text { text: "Your apps stay open in Desktop Mode"; color: Theme.accentInk; opacity: 0.8; font.family: Theme.font; font.pixelSize: 12 }
                        }
                        Icon { name: "forward"; size: 20; color: Theme.accentInk }
                    }
                    MouseArea { id: gma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.activate("game") }
                }

                // ---- tiles
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: 10
                    rowSpacing: 10
                    Tile { navId: "wifi"; icon: "wifi"; label: "Wi-Fi"; detail: !root.wifiKnown ? "Not available" : root.wifiOn ? "On" : "Off"; lit: root.wifiOn; enabled: root.wifiKnown; page: "wifi" }
                    Tile { navId: "bluetooth"; icon: "bt"; label: "Bluetooth"; detail: !root.btPresent ? "No adapter" : !root.btPowered ? "Off" : root.btConnected.length ? root.btConnected.join(", ") : "On"; lit: root.btPresent && root.btPowered; page: "bluetooth" }
                    Tile { navId: "night"; icon: "moon"; label: "Night light"; detail: root.nightAvailable ? "Sunset to sunrise" : "Not installed"; lit: Settings.nightLight; enabled: root.nightAvailable }
                    Tile { navId: "mouse"; icon: "mouse"; label: "Controller mouse"; detail: root.mouseOn ? "On" : "Off"; lit: root.mouseOn }
                    Tile { navId: "dnd"; icon: "bell-off"; label: "Do not disturb"; detail: DeskState.dnd ? "On" : "Off"; lit: DeskState.dnd }
                    Tile { navId: "keyboard"; icon: "kbd"; label: "Keyboard"; detail: "On-screen" }
                }

                // ---- sliders
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Pill {
                        navId: "volume"
                        icon: !root.audio || root.audio.muted ? "vol-mute" : "vol"
                        value: root.audio ? (root.audio.muted ? 0 : root.audio.volume) : 0
                        maxValue: Buttons.maxVolume
                        onMoved: v => root.setVolume(v)
                        onIconClicked: root.activate("volume")
                    }
                    RoundBtn { navId: "volume-out"; icon: "forward"; tip: "Sound output (Settings › Sound)"; onPicked: { root.shown = false; root.pageRequested("sound"); } }
                }
                Pill {
                    navId: "brightness"
                    visible: root.brightness >= 0
                    icon: "sun"
                    value: root.brightness / 100
                    onMoved: v => root.setBrightness(Math.round(v * 100))
                }

                // ---- performance profile
                Text { visible: root.profile !== ""; text: "POWER PROFILE"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 1.2 }
                Rectangle {
                    Layout.fillWidth: true
                    visible: root.profile !== ""
                    implicitHeight: 50
                    radius: Theme.r(14)
                    color: Theme.panel2
                    RowLayout {
                        anchors { fill: parent; margins: 4 }
                        spacing: 4
                        Choice { navId: "power-saver"; label: "Battery saver" }
                        Choice { navId: "balanced"; label: "Balanced" }
                        Choice { navId: "performance"; label: "Performance" }
                    }
                }

                // ---- Handheld settings and the weather
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    visible: root.hhdOn || !!root.weather.line
                    Focusable {
                        navId: "handheld"
                        visible: root.hhdOn
                        Layout.fillWidth: true
                        implicitHeight: 52
                        color: hha.containsMouse ? Theme.panel3 : Theme.panel2
                        RowLayout {
                            anchors { fill: parent; leftMargin: 16; rightMargin: 14 }
                            Icon { name: "pad"; size: 20 }
                            Text { Layout.fillWidth: true; text: "Handheld settings"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; elide: Text.ElideRight }
                            Icon { name: "forward"; size: 16; color: Theme.muted }
                        }
                        MouseArea { id: hha; anchors.fill: parent; hoverEnabled: true; onClicked: root.activate("handheld") }
                    }
                    // ---- Hearth Weather: "18° Partly cloudy" for the chosen place; opens Weather.
                    Focusable {
                        navId: "weather"
                        visible: !!root.weather.line
                        Layout.fillWidth: !root.hhdOn
                        implicitWidth: wrow.implicitWidth + 28
                        implicitHeight: 52
                        color: wma.containsMouse ? Theme.panel3 : Theme.panel2
                        Row {
                            id: wrow
                            anchors.centerIn: parent
                            spacing: 8
                            Icon { anchors.verticalCenter: parent.verticalCenter; name: root.weather.day === false ? "moon" : "sun"; size: 18 }
                            Text { anchors.verticalCenter: parent.verticalCenter; text: root.weather.line || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                            Text { anchors.verticalCenter: parent.verticalCenter; visible: !root.hhdOn; text: root.weather.place || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        }
                        MouseArea { id: wma; anchors.fill: parent; hoverEnabled: true; onClicked: root.activate("weather") }
                    }
                    // ---- end of Hearth Weather
                }

                // ---- controller hints
                Row {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 18
                    Repeater {
                        model: [{ b: "A", c: "#3fae5a", t: "Choose" }, { b: "B", c: "#d9483b", t: "Close" }, { b: "◂▸", c: "#6c7480", t: "Adjust" }]
                        delegate: Row {
                            required property var modelData
                            spacing: 6
                            Rectangle { width: 22; height: 22; radius: 11; color: modelData.c; Text { anchors.centerIn: parent; text: modelData.b; color: "white"; font.pixelSize: modelData.b.length > 1 ? 9 : 11; font.bold: true } }
                            Text { anchors.verticalCenter: parent.verticalCenter; text: modelData.t; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks
    component Focusable: Rectangle {
        property string navId
        readonly property bool focused: root.current === navId
        radius: Theme.r(14)
        border.width: 2
        border.color: focused ? Theme.accent : "transparent"
        Behavior on border.color { ColorAnimation { duration: Theme.quick } }
    }

    // A 52 px pill slider: grab it anywhere; the icon inside mutes (volume).
    component Pill: Focusable {
        id: s
        property string icon
        property real value
        property real maxValue: 1          // volume can go to 1.5 with over-amplification on
        signal moved(real v)
        signal iconClicked()
        Layout.fillWidth: true
        implicitHeight: 52
        radius: Theme.r(26)
        color: Theme.panel2
        clip: true
        readonly property real frac: Math.max(0, Math.min(1, value / maxValue))
        Rectangle {
            x: 0; y: 0; height: parent.height
            width: Math.max(parent.height, parent.width * s.frac)
            radius: parent.radius
            color: Theme.accent
        }
        Icon { x: 18; anchors.verticalCenter: parent.verticalCenter; name: s.icon; size: 20; color: s.frac * s.width > 50 ? Theme.accentInk : Theme.text }
        Text { anchors { right: parent.right; rightMargin: 18; verticalCenter: parent.verticalCenter } text: Math.round(s.value * 100) + "%"; color: s.frac > 0.86 ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        MouseArea {
            anchors.fill: parent
            onPressed: m => { if (m.x < 50 && s.navId === "volume") { s.iconClicked(); return; } s.moved(s.maxValue * Math.max(0, Math.min(1, m.x / s.width))); }
            onPositionChanged: m => { if (pressed && m.x >= 0) s.moved(s.maxValue * Math.max(0, Math.min(1, m.x / s.width))); }
        }
    }

    // A 76 px tile: icon, name and state; lit when on. Wi-Fi and Bluetooth have a chevron to their lists.
    component Tile: Focusable {
        id: t
        property string icon
        property string label
        property string detail
        property bool lit
        property string page: ""
        Layout.fillWidth: true
        Layout.preferredWidth: 200
        implicitHeight: 76
        opacity: enabled ? 1 : 0.5
        color: lit ? Theme.accent : ta.containsMouse ? Theme.panel3 : Theme.panel2
        Behavior on color { ColorAnimation { duration: Theme.quick } }
        RowLayout {
            anchors { fill: parent; leftMargin: 14; rightMargin: t.page ? 6 : 14 }
            spacing: 12
            Rectangle {
                implicitWidth: 40; implicitHeight: 40; radius: Theme.r(12)
                color: t.lit ? Qt.rgba(1, 1, 1, 0.22) : Theme.panel3
                Icon { anchors.centerIn: parent; name: t.icon; size: 20; color: t.lit ? Theme.accentInk : Theme.text }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text { Layout.fillWidth: true; text: t.label; color: t.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: t.detail; color: t.lit ? Theme.accentInk : Theme.muted; opacity: t.lit ? 0.8 : 1; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
            }
        }
        MouseArea { id: ta; anchors.fill: parent; hoverEnabled: true; enabled: t.enabled; onClicked: root.activate(t.navId) }
        Rectangle {   // the chevron to the list (Settings › Wi-Fi / Bluetooth)
            visible: t.page !== ""
            anchors { right: parent.right; verticalCenter: parent.verticalCenter; rightMargin: 6 }
            width: 32; height: 52; radius: Theme.r(10)
            color: cha.containsMouse ? Qt.rgba(0, 0, 0, 0.15) : "transparent"
            Icon { anchors.centerIn: parent; name: "forward"; size: 16; color: t.lit ? Theme.accentInk : Theme.muted }
            MouseArea { id: cha; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { root.shown = false; root.pageRequested(t.page); } }
        }
    }

    component Choice: Focusable {
        id: ch
        property string label
        readonly property bool lit: root.profile === navId
        Layout.fillWidth: true
        implicitHeight: 42
        radius: Theme.r(11)
        color: lit ? Theme.panel3 : "transparent"
        Text { anchors.centerIn: parent; text: ch.label; color: ch.lit ? Theme.text : Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: ch.lit ? Font.DemiBold : Font.Normal }
        MouseArea { anchors.fill: parent; onClicked: root.activate(ch.navId) }
    }

    component RoundBtn: Focusable {
        id: rb
        property string icon
        property string tip: ""
        signal picked()
        implicitWidth: 44; implicitHeight: 44
        radius: 22
        color: rba.containsMouse ? Theme.panel3 : Theme.panel2
        Icon { anchors.centerIn: parent; name: rb.icon; size: 19 }
        MouseArea { id: rba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { rb.picked(); if (rb.navId !== "volume-out") root.activate(rb.navId); } }
    }
}
