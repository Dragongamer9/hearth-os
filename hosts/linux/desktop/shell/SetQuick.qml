// Settings › Quick (1.14, the new Settings, mockup settings-B1): the controller look's first tab, big tiles for the
// settings people change most. Light mode while you play (wide), Wi-Fi, Bluetooth, Brightness and Volume (sliders),
// Start in (wide), Updates, Power, Night light, Do not disturb. X (Pin to Quick) picks which tiles are here; the pick
// is kept in ~/.local/share/nimbus/settings-quick.json (merged like every settings file; missing = all of them).
// Every tile is the same setting as on its own page: nothing here has a second copy of a value.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

SetPage {
    id: page
    title: "Quick"
    readonly property var allTiles: ["lightmode", "wifi", "bt", "brightness", "volume", "startin", "updates", "power", "night", "dnd"]
    readonly property var names: ({ lightmode: "Light mode while you play", wifi: "Wi-Fi", bt: "Bluetooth", brightness: "Brightness",
        volume: "Volume", startin: "Start in", updates: "Updates", power: "Power", night: "Night light", dnd: "Do not disturb" })
    property var pinned: allTiles
    property bool picking: false
    readonly property string admin: "/usr/libexec/nimbus/hearth-admin"
    readonly property string powerHelper: decodeURIComponent(Qt.resolvedUrl("hearth-power").toString().replace(/^file:\/\//, ""))
    function shown(id) { return pinned.indexOf(id) >= 0 && (id !== "lightmode" || !!hsCfg) && (id !== "brightness" || brightness >= 0); }
    // X on the controller (the frame's xAction): Pin to Quick.
    readonly property string xLabel: "Pin to Quick"
    function xAction() { picking = !picking; if (picking) Qt.callLater(() => pickFirst.forceActiveFocus()); }
    function togglePin(id) {
        const p = pinned.indexOf(id) >= 0 ? pinned.filter(x => x !== id) : allTiles.filter(x => x === id || pinned.indexOf(x) >= 0);
        pinned = p;
        writer.merge({ tiles: p });
    }

    // ---------------------------------------------------------------- what the tiles read
    SettingsWriter { id: writer; path: Quickshell.env("HOME") + "/.local/share/nimbus/settings-quick.json" }
    FileView {
        path: Quickshell.env("HOME") + "/.local/share/nimbus/settings-quick.json"
        onLoaded: { try { const o = JSON.parse(text()); if (Array.isArray(o.tiles) && !writer.owns("tiles")) page.pinned = o.tiles.filter(t => page.allTiles.indexOf(t) >= 0); } catch (e) {} }
    }
    property string ssid: ""
    property bool wifiOn: true
    property string btText: ""
    property bool btOn: false
    property int brightness: -1
    property var hs: null
    readonly property var hsCfg: hs ? hs.settings : null
    property var updates: null
    property string profile: ""
    PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
    readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
    readonly property string sinkName: Pipewire.defaultAudioSink ? (Pipewire.defaultAudioSink.nickname || Pipewire.defaultAudioSink.description || "Sound") : "Sound"
    SetJob { id: hsJob; helper: "hearth-home-server" }
    SetJob { id: upd; helper: page.admin; admin: true }
    Component.onCompleted: {
        read.running = true;
        hsJob.run(["status"], r => page.hs = r, () => page.hs = null);
        upd.run(["update-status"], r => page.updates = r, () => page.updates = { error: true });
        power.running = true;
    }
    Timer { interval: 8000; repeat: true; running: page.visible; onTriggered: if (!read.running) read.running = true }
    Process {
        id: read
        command: ["sh", "-c", "nmcli -t -f ACTIVE,SSID,SIGNAL dev wifi 2>/dev/null | sed -n 's/^yes:/ssid=/p' | head -1; nmcli radio wifi 2>/dev/null | sed 's/^/radio=/'; "
            + "bluetoothctl --timeout 3 show 2>/dev/null | grep -q 'Powered: yes' && echo bt=on || echo bt=off; "
            + "bluetoothctl --timeout 3 devices Connected 2>/dev/null | sed -n 's/^Device [0-9A-F:]* /dev=/p'; brightnessctl -m -c backlight 2>/dev/null | sed 's/^/bl=/'"]
        stdout: StdioCollector {
            onStreamFinished: {
                const s = /^ssid=(.*)$/m.exec(text);
                if (s) { const f = s[1].split(":"); const sig = Number(f[f.length - 1]); f.pop(); page.ssid = f.join(":") + (sig >= 70 ? " · strong" : sig >= 40 ? " · good" : sig > 0 ? " · weak" : ""); } else page.ssid = "";
                page.wifiOn = !/^radio=disabled/m.test(text);
                page.btOn = /^bt=on$/m.test(text);
                const devs = [...text.matchAll(/^dev=(.*)$/mg)].map(m => m[1].trim());
                page.btText = !page.btOn ? "Off" : devs.length ? devs.slice(0, 2).join(", ") + (devs.length > 2 ? " +" + (devs.length - 2) : "") : "On · nothing connected";
                const b = /^bl=(.*)$/m.exec(text);
                const f = b ? b[1].split(",") : [];
                page.brightness = f.length >= 4 ? parseInt(f[3]) : -1;
            }
        }
    }
    Process {
        id: power
        command: ["python3", page.powerHelper, "status"]
        stdout: StdioCollector { onStreamFinished: { try { const r = JSON.parse(text.trim().split("\n").pop()); page.profile = r.mode && r.mode.supported ? r.mode.active : ""; } catch (e) { page.profile = ""; } } }
    }
    function run(cmd) { Quickshell.execDetached(cmd); }
    readonly property string updText: !updates ? "Checking…" : updates.error ? "Open to check" : updates.staged ? "Restart to finish"
        : updates.available ? "Hearth " + (updates.available.version || "") + " is ready" : "Up to date · Hearth OS " + (updates.current || updates.version || "").replace(/\.0$/, "")
    readonly property string sleepText: Settings.sleepMinutes === 0 ? "sleeps never" : "sleeps after " + (Settings.sleepMinutes >= 60 ? Settings.sleepMinutes / 60 + " h" : Settings.sleepMinutes + " min")
    readonly property string profileText: profile === "power-saver" ? "Power saver" : profile === "performance" ? "Performance" : profile === "balanced" ? "Balanced" : "Power"

    // ---------------------------------------------------------------- Pin to Quick (X)
    ColumnLayout {
        visible: page.picking
        Layout.fillWidth: true
        Layout.bottomMargin: 16
        spacing: 8
        Text { Layout.fillWidth: true; text: "Pin to Quick: pick what's on this tab (A pins or unpins, X when you're done)"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
        Repeater {
            model: page.allTiles
            delegate: SetSwitch {
                id: pinRow
                required property string modelData
                required property int index
                Component.onCompleted: if (index === 0) page.pickFirstItem = pinRow
                title: page.names[modelData]
                checked: page.pinned.indexOf(modelData) >= 0
                onToggled: page.togglePin(modelData)
            }
        }
    }
    property var pickFirstItem: null
    readonly property var pickFirst: pickFirstItem || page

    // ---------------------------------------------------------------- the tiles
    GridLayout {
        visible: !page.picking
        Layout.fillWidth: true
        columns: 4
        columnSpacing: 20; rowSpacing: 20
        Tile {
            id: t0; tid: "lightmode"; Layout.columnSpan: 2; icon: "pad"; title: "Light mode while you play"
            sub: "Big home server jobs wait until your game ends"
            on: !!page.hsCfg && page.hsCfg.lightMode !== false; hasSwitch: true; badge: on ? "On" : "Off"
            onActivated: hsJob.run(["set", "lightMode", on ? "off" : "on"], () => hsJob.run(["status"], r => page.hs = r, () => {}), e => SettingsNav.toast(e))
        }
        Tile { tid: "wifi"; icon: "wifi"; title: "Wi-Fi"; sub: page.wifiOn ? (page.ssid || "Not connected") : "Off"; badge: page.wifiOn ? "On" : "Off"; onActivated: SettingsNav.openPage("wifi") }
        Tile { tid: "bt"; icon: "headphones"; title: "Bluetooth"; sub: page.btText || "…"; badge: page.btOn ? "On" : "Off"; onActivated: SettingsNav.openPage("bluetooth") }
        Tile {
            tid: "brightness"; icon: "sun"; title: "Brightness"; sub: page.brightness + "%"; slider: true; level: page.brightness / 100
            onMoved: v => { page.brightness = Math.max(1, Math.round(v * 100)); page.run(["brightnessctl", "-q", "-c", "backlight", "set", page.brightness + "%"]); }
            onActivated: SettingsNav.openPage("display")
        }
        Tile {
            tid: "volume"; icon: "vol"; title: "Volume"; sub: page.sinkName + " · " + Math.round((page.audio ? page.audio.volume : 0) * 100) + "%"
            slider: true; level: page.audio ? Math.min(1, page.audio.volume) : 0
            onMoved: v => { if (page.audio) { page.audio.muted = false; page.audio.volume = Math.max(0, Math.min(1, v)); } }
            onActivated: SettingsNav.openPage("sound")
        }
        Item {
            visible: page.shown("startin")
            Layout.columnSpan: 2; Layout.fillWidth: true
            implicitHeight: 186
            Rectangle { anchors.fill: parent; radius: SetStyle.bigRadius; color: SetStyle.card; border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05) }
            Text { x: 26; y: 24; text: "Start in"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
            Text { anchors { right: parent.right; rightMargin: 26 } y: 28; text: "What this PC opens when it turns on"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
            StartInChoice { anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 10 } flat: true; title: ""; sub: ""; icon: "" }
        }
        Tile { tid: "updates"; icon: "update"; title: "Updates"; sub: page.updText; onActivated: SettingsNav.openPage("updates") }
        Tile { tid: "power"; icon: "batt"; title: "Power"; sub: page.profileText + " · " + page.sleepText; onActivated: SettingsNav.openPage("power") }
        Tile { tid: "night"; icon: "moon"; title: "Night light"; sub: Settings.nightLight ? "On" : "Off"; hasSwitch: true; on: Settings.nightLight; onActivated: Settings.nightLight = !Settings.nightLight }
        Tile { tid: "dnd"; icon: "bell-off"; title: "Do not disturb"; sub: DeskState.dnd ? "On" : "Off"; hasSwitch: true; on: DeskState.dnd; onActivated: DeskState.setDnd(!DeskState.dnd) }
    }

    // One big tile: its icon, name and state; a switch, an On/Off badge or a slider (changed with left and right).
    component Tile: SetBase {
        id: tile
        property string tid
        property string icon
        property string title
        property string sub
        property string badge
        property bool hasSwitch: false
        property bool on: false
        property bool slider: false
        property real level: 0
        signal moved(real v)
        visible: page.shown(tid)
        Layout.fillWidth: true
        Layout.preferredWidth: 1
        implicitHeight: 186
        radius: SetStyle.bigRadius
        usesLeftRight: slider
        onAdjusted: d => tile.moved(Math.max(0, Math.min(1, tile.level + d * 0.05)))
        // The focused tile: lifted a little, with a warm tint (B1).
        color: activeFocus ? SetStyle.mix(SetStyle.card, Theme.accent, 0.12) : SetStyle.card
        transform: Translate { y: tile.activeFocus ? -3 : 0 }
        Accessible.name: title + ": " + sub
        Icon { x: 26; y: 24; name: tile.icon; size: 44; color: Theme.accent }
        SetChip { visible: tile.badge !== "" && !tile.hasSwitch; anchors { right: parent.right; top: parent.top; margins: 22 } text: tile.badge; kind: tile.badge === "On" ? "ok" : "off" }
        // A wide tile with a switch (Light mode while you play) also says On / Off under its name (B1).
        SetChip { visible: tile.badge !== "" && tile.hasSwitch; anchors { left: parent.left; bottom: parent.bottom; margins: 26; bottomMargin: 22 } text: tile.badge; kind: tile.on ? "ok" : "off" }
        Rectangle {
            visible: tile.hasSwitch
            anchors { right: parent.right; top: parent.top; margins: 22 }
            width: 54; height: 30; radius: 15
            color: tile.on ? Theme.accent : SetStyle.card2
            Rectangle { width: 22; height: 22; radius: 11; y: 4; x: tile.on ? parent.width - width - 4 : 4; color: tile.on ? Theme.accentInk : Theme.muted; Behavior on x { NumberAnimation { duration: Theme.quick } } }
        }
        Column {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 26; bottomMargin: tile.slider ? 54 : tile.hasSwitch && tile.badge !== "" ? 70 : 24 }
            spacing: 4
            Text { width: parent.width; text: tile.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 21; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { width: parent.width; text: tile.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15; elide: Text.ElideRight }
        }
        Item {
            visible: tile.slider
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 26; bottomMargin: 22 }
            height: 22
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 8; radius: 4; color: SetStyle.card2 }
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width * Math.max(0, Math.min(1, tile.level)); height: 8; radius: 4; color: Theme.accent }
            Rectangle { x: parent.width * Math.max(0, Math.min(1, tile.level)) - 11; anchors.verticalCenter: parent.verticalCenter; width: 22; height: 22; radius: 11; color: Theme.text }
            MouseArea { anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.PointingHandCursor; onPressed: m => tile.moved(m.x / width); onPositionChanged: m => { if (pressed) tile.moved(Math.max(0, Math.min(1, m.x / width))); } }
        }
    }
}
