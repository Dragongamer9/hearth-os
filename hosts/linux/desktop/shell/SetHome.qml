// Hearth Settings › Home: where Settings opens. This PC (its wallpaper, name and Rename), Wi-Fi, Bluetooth, Updates and
// Hearth Link at a glance, the settings people change first, your Bluetooth devices, storage, the home server and your
// look. Everything here only reads until you press something; each card opens the page that is its real home.
// Rename (the device name's home, moved from Sign-in): hearth-admin hostname, through pkexec (Hearth's polkit rule).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Home"
    readonly property bool wide: width >= 1080
    readonly property string admin: "/usr/libexec/nimbus/hearth-admin"
    readonly property bool live: !!SettingsNav.settingsApp && SettingsNav.settingsApp.live

    // ---------------------------------------------------------------- what this page reads
    property var info: ({})
    property bool canAdmin: false
    property string ssid: ""
    property bool wired: false
    property string btText: ""
    property var btDevices: []        // [{mac, name, connected, battery}]
    property var updates: null        // hearth-admin update-status
    property var link: null           // hearth-link-settings status
    property var hs: null             // hearth-home-server status
    property var disk: null           // diskspace.status
    readonly property var hsCfg: hs ? hs.settings : null
    function shownVersion(v) { const m = /^(\d+)\.(\d+)\.0$/.exec(v || ""); return m && Number(m[1]) >= 1 ? m[1] + "." + m[2] : (v || ""); }
    function size(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : b >= 1e9 ? Math.round(b / 1e9) + " GB" : Math.round((b || 0) / 1e6) + " MB"; }
    function refresh() { wifi.running = true; bt.running = true; }
    Component.onCompleted: {
        about.running = true;
        refresh();
        linkJob.run(["status"], r => page.link = r, () => page.link = null);
        hsJob.run(["status"], r => page.hs = r, () => page.hs = null);
        core.call("diskspace.status", {}, v => page.disk = v, () => page.disk = null);
    }
    Timer { interval: 10000; repeat: true; running: page.visible; onTriggered: page.refresh() }
    HearthCall { id: core }
    SetJob { id: linkJob; helper: "hearth-link-settings" }
    SetJob { id: hsJob; helper: "hearth-home-server" }
    SetJob { id: rename; helper: page.admin; admin: true }
    SetJob { id: upd; helper: page.admin; admin: true }
    Process {
        id: about
        command: ["sh", "-c", "echo \"host=$(cat /etc/hostname 2>/dev/null || hostname)\"; echo \"hver=$(cat /usr/share/nimbus/version 2>/dev/null)\"; [ -x \"$0\" ] && echo admin=1", page.admin]
        stdout: StdioCollector {
            onStreamFinished: {
                const o = {};
                for (const l of text.split("\n")) { const i = l.indexOf("="); if (i > 0) o[l.slice(0, i)] = l.slice(i + 1).trim(); }
                page.info = o;
                const had = page.canAdmin;
                page.canAdmin = o.admin === "1";
                if (page.canAdmin && !had) upd.run(["update-status"], r => page.updates = r, () => page.updates = { error: true });
            }
        }
    }
    Process {
        id: wifi
        command: ["sh", "-c", "nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | sed -n 's/^yes:/ssid=/p' | head -1; nmcli -t -f TYPE,STATE dev 2>/dev/null | grep -q '^ethernet:connected' && echo wired=1"]
        stdout: StdioCollector { onStreamFinished: { const m = /^ssid=(.*)$/m.exec(text); page.ssid = m ? m[1].trim() : ""; page.wired = /^wired=1$/m.test(text); } }
    }
    Process {
        id: bt
        command: ["sh", "-c", "bluetoothctl --timeout 3 show 2>/dev/null | grep -q 'Powered: yes' && echo on || echo off; echo ---; bluetoothctl --timeout 3 devices Paired 2>/dev/null; echo ---; "
            + "bluetoothctl --timeout 3 devices Connected 2>/dev/null | while read -r _ mac _; do b=$(bluetoothctl --timeout 2 info \"$mac\" 2>/dev/null | sed -n 's/.*Battery Percentage:.*(\\([0-9]*\\)).*/\\1/p'); echo \"$mac $b\"; done"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("---\n");
                const on = (parts[0] || "").trim() === "on";
                const conn = {};
                for (const l of (parts[2] || "").split("\n")) { const m = /^([0-9A-F:]{17}) ?(\d*)$/i.exec(l.trim()); if (m) conn[m[1]] = m[2] || "-"; }
                const devs = (parts[1] || "").split("\n").map(l => /^Device ([0-9A-F:]{17}) (.*)$/i.exec(l.trim())).filter(Boolean)
                    .map(m => ({ mac: m[1], name: m[2], connected: conn[m[1]] !== undefined, battery: conn[m[1]] && conn[m[1]] !== "-" ? conn[m[1]] : "" }))
                    .sort((a, b) => (b.connected ? 1 : 0) - (a.connected ? 1 : 0));
                page.btText = on ? "On" + (Object.keys(conn).length ? " · " + Object.keys(conn).length + " connected" : "") : "Off";
                page.replaceList("btDevices", devs.slice(0, 3));
            }
        }
    }
    function iconFor(name) {
        const n = (name || "").toLowerCase();
        return /controller|gamepad|xbox|dualsense|dualshock|joy-?con|8bitdo/.test(n) ? "pad"
             : /headphone|headset|buds|airpods|wh-|wf-|earbud|speaker|soundbar/.test(n) ? "headphones"
             : /keyboard|keychron/.test(n) ? "kbd" : /mouse/.test(n) ? "mouse" : "bt";
    }
    readonly property string updText: !canAdmin ? "Open to check" : !updates ? "Checking…" : updates.error ? "Open to check"
        : updates.staged ? "Restart to finish" : updates.available ? "Hearth " + shownVersion(updates.available.version || "") + " is ready" : "Up to date"
    readonly property string linkText: {
        if (!link || link.error) return "Pair your devices";
        const peers = link.peers || [];
        if (!peers.length) return "No devices paired yet";
        const on = peers.filter(p => p.online).length;
        return on + (on === 1 ? " device online" : " devices online");
    }
    readonly property var hsAlert: {
        if (!hs || !hsCfg) return null;
        const failed = (hs.backups || []).filter(j => j.state && j.state.last && j.state.last.ok === false);
        if (failed.length) return { title: "A backup didn't finish", sub: failed.map(j => j.name).join(", ") + ": check it in Backups & sync.", page: "hs-backups" };
        const a = hs.alerts ? (hs.alerts.active || []) : [];
        if (a.length) return { title: a[0].text || "Something needs a look", sub: a.length > 1 ? (a.length - 1) + " more in Alerts & power" : "Open Alerts & power", page: "hs-alerts" };
        return null;
    }
    readonly property var hsOn: hs ? (hs.shares || []).filter(s => s.on !== false) : []
    readonly property var hsTs: hs ? (hs.tailscale || {}) : ({})
    // The wallpaper as it is now (the look's own when you haven't picked one).
    readonly property string wall: Theme.wall
    readonly property var scenes: ["anim:neon", "anim:zen", "anim:glass", "anim:synth"]
    function preview(f) {
        if (f === "" || scenes.indexOf(f) >= 0) return "";
        if (f.startsWith("anim:")) return Qt.resolvedUrl("images/wall-" + f.slice(5) + ".jpg");
        if (f.startsWith("builtin:")) return Qt.resolvedUrl("images/wallpapers/" + f.slice(8) + ".jpg");
        return "file://" + f.split("/").map(encodeURIComponent).join("/");
    }
    readonly property var accents: (Theme.palettes.accents || [])
    readonly property string accentName: !Theme.validAccent ? Theme.lookAccentName + " accent"
        : Settings.accentFrom === "wallpaper" ? "Accent from your wallpaper" : ((accents.find(a => a.color === Settings.accent) || { name: "Custom" }).name + " accent")

    // ---------------------------------------------------------------- the live USB: install first
    SetBanner {
        visible: page.live
        Layout.bottomMargin: 10
        kind: "info"; icon: "update"
        title: "Hearth OS is running from the USB stick"
        sub: "Install it on this PC, next to Windows or on its own. Nothing changes until you say so."
        action: "Install Hearth"
        onActivated: SettingsNav.openPage("install")
    }

    // ---------------------------------------------------------------- this PC, and the four tiles
    GridLayout {
        Layout.fillWidth: true
        columns: page.wide ? 2 : 1
        columnSpacing: 16; rowSpacing: 16
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredWidth: 5
            Layout.alignment: Qt.AlignTop
            implicitHeight: Math.max(164, deviceCol.implicitHeight + 40)
            radius: SetStyle.bigRadius
            color: SetStyle.card
            border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
            RowLayout {
                anchors { fill: parent; margins: 20 }
                spacing: 22
                // The monitor with your wallpaper on it.
                Item {
                    visible: page.width > 700
                    Layout.preferredWidth: 196; Layout.preferredHeight: 136
                    Rectangle {
                        id: screenPic
                        width: 196; height: 124
                        radius: 8
                        color: "#0b0f15"
                        Rectangle {
                            anchors { fill: parent; margins: 5 }
                            radius: 4
                            clip: true
                            color: Theme.bg2
                            Image { anchors.fill: parent; source: page.preview(page.wall); fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400; visible: status === Image.Ready }
                            Loader { anchors.fill: parent; active: page.scenes.indexOf(page.wall) >= 0; source: "Scene.qml"; onLoaded: item.name = Qt.binding(() => page.wall.slice(5)) }
                            Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } visible: page.wall === ""; width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.7 }
                            // a sketch of the bar and a window, in your colours
                            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 6; color: Theme.panel; opacity: 0.92 }
                            Rectangle {
                                x: parent.width * 0.18; y: parent.height * 0.2; width: parent.width * 0.64; height: parent.height * 0.62; radius: 3; color: Theme.bg; opacity: 0.88
                                Rectangle { width: parent.width * 0.26; height: parent.height; radius: 3; color: Theme.panel }
                                Rectangle { x: parent.width * 0.32; y: 8; width: parent.width * 0.6; height: 5; radius: 2; color: Theme.accent; opacity: 0.8 }
                            }
                        }
                    }
                    Rectangle { anchors.horizontalCenter: screenPic.horizontalCenter; y: screenPic.height; width: 40; height: 8; radius: 2; color: "#0b0f15" }
                }
                ColumnLayout {
                    id: deviceCol
                    Layout.fillWidth: true
                    spacing: 4
                    Text { Layout.fillWidth: true; text: page.info.host || "This PC"; color: Theme.text; font.family: Theme.font; font.pixelSize: 24; font.weight: Font.DemiBold; elide: Text.ElideRight }
                    Text {
                        Layout.fillWidth: true; Layout.bottomMargin: 10
                        text: [page.info.hver && page.info.hver !== "0.0.0" ? "Hearth OS " + page.shownVersion(page.info.hver) : "Hearth OS", "Desktop Mode", Theme.lookLabel + " look"].join(" · ")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                    }
                    SetButton {
                        visible: page.canAdmin && !nameField.visible
                        icon: "edit"; text: rename.busy ? "Renaming…" : "Rename"
                        onActivated: { nameField.visible = true; nameField.focusInput(); }
                    }
                    SetField {
                        id: nameField
                        visible: false
                        placeholder: "A new name for " + (page.info.host || "this PC") + ", then Enter"
                        onSubmitted: t => {
                            const n = t.trim();
                            if (!n) { nameField.visible = false; return; }
                            rename.run(["hostname", n], () => { nameField.text = ""; nameField.visible = false; SettingsNav.toast("Renamed to " + n); about.running = true; },
                                       e => SettingsNav.toast(e));
                        }
                    }
                    Text { visible: !page.canAdmin; Layout.fillWidth: true; text: "Its name is set on Hearth OS."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
            }
        }
        GridLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 4
            columns: 2
            columnSpacing: 10; rowSpacing: 10
            Tile { icon: "wifi"; label: "Wi-Fi"; status: page.ssid || (page.wired ? "Wired" : "Not connected"); target: "wifi" }
            Tile { icon: "bt"; label: "Bluetooth"; status: page.btText || "…"; target: "bluetooth" }
            Tile { icon: "update"; label: "Updates"; status: page.updText; target: "updates"; warn: !!page.updates && (!!page.updates.available || !!page.updates.staged) }
            Tile { icon: "desktops"; label: "Hearth Link"; status: page.linkText; target: "link" }
        }
    }

    // ---------------------------------------------------------------- two columns of cards
    GridLayout {
        Layout.fillWidth: true
        Layout.topMargin: 16
        columns: page.wide ? 2 : 1
        columnSpacing: 16; rowSpacing: 16
        ColumnLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            spacing: 16
            HomeCard {
                icon: "sparkle"; title: "Recommended settings"; sub: "Settings people often change first"
                linkText: "See more"; linkTarget: "system"
                SetSwitch { flat: true; icon: "moon"; title: "Night light"; sub: "Warmer colours from sunset to sunrise"; checked: Settings.nightLight; onToggled: v => Settings.nightLight = v }
                SetSwitch {
                    flat: true; visible: !!page.hsCfg
                    icon: "pad"; title: "Light mode while you play"; sub: "Big home server jobs wait until your game ends"
                    checked: !!page.hsCfg && page.hsCfg.lightMode !== false
                    onToggled: v => hsJob.run(["set", "lightMode", v ? "on" : "off"], () => hsJob.run(["status"], r => page.hs = r, () => {}), e => SettingsNav.toast(e))
                }
                SetSwitch { flat: true; icon: "bell-off"; title: "Do not disturb"; sub: "Hold notifications until you turn it off"; checked: DeskState.dnd; onToggled: v => DeskState.setDnd(v) }
                StartInChoice { flat: true; sub: "What this PC opens when it turns on" }
            }
            HomeCard {
                icon: "bt"; title: "Bluetooth devices"; linkText: "See all"; linkTarget: "bluetooth"
                Repeater {
                    model: page.btDevices
                    delegate: SetRow { chevron: false;
                        required property var modelData
                        flat: true
                        icon: page.iconFor(modelData.name); title: modelData.name
                        sub: modelData.connected ? "Connected" : "Not connected"
                        value: modelData.battery ? "Battery " + modelData.battery + "%" : ""
                        onActivated: SettingsNav.openPage("bluetooth")
                    }
                }
                SetRow {
                    flat: true; visible: page.btDevices.length === 0
                    icon: "plus"; title: "Pair a device"; sub: page.btText === "Off" ? "Bluetooth is off" : "Headphones, controllers, keyboards"
                    chevron: true
                    onActivated: SettingsNav.openPage("bluetooth")
                }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            spacing: 16
            HomeCard {
                icon: "drive"; title: "Storage"; linkText: "Manage"; linkTarget: "storage"
                Repeater {
                    model: page.disk ? (page.disk.drives || []).filter(d => d.size > 0).slice(0, 3) : []
                    delegate: ColumnLayout {
                        id: drive
                        required property var modelData
                        readonly property real frac: modelData.size ? modelData.used / modelData.size : 0
                        Layout.fillWidth: true
                        Layout.leftMargin: 12; Layout.rightMargin: 12; Layout.topMargin: 8; Layout.bottomMargin: 6
                        spacing: 6
                        RowLayout {
                            Layout.fillWidth: true
                            Text { Layout.fillWidth: true; text: drive.modelData.name || drive.modelData.mount; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
                            Text { text: page.size(drive.modelData.used) + " of " + page.size(drive.modelData.size) + " used"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: SetStyle.card2
                            Rectangle { width: parent.width * Math.max(0, Math.min(1, drive.frac)); height: parent.height; radius: 4; color: drive.frac > 0.85 ? SetStyle.warn : Theme.accent }
                        }
                    }
                }
                SetRow { flat: true; icon: "trash"; title: "Clean up"; sub: "Old shader caches and finished downloads"; chevron: true; onActivated: SettingsNav.openPage("storage") }
            }
            // Your devices (SetDevices.qml): your paired devices from Hearth Link's status (already read here).
            HomeCard {
                visible: !!page.link && !page.link.error && (page.link.peers || []).length > 0
                icon: "cpu"; title: "Your devices"; linkText: "Open"; linkTarget: "devices"
                Repeater {
                    model: page.link ? (page.link.peers || []).slice(0, 3) : []
                    delegate: SetRow { chevron: true;
                        required property var modelData
                        flat: true
                        icon: "desktops"; title: modelData.name
                        sub: modelData.online ? "Online" : "Offline or asleep"
                        onActivated: SettingsNav.openPage("devices")
                    }
                }
            }
            HomeCard {
                visible: !!page.hsCfg
                icon: "server"; title: "Home server"; linkText: "Open"; linkTarget: "hs-overview"
                SetRow { chevron: false;
                    flat: true; visible: !!page.hsAlert
                    icon: "warn"; title: page.hsAlert ? page.hsAlert.title : ""; sub: page.hsAlert ? page.hsAlert.sub : ""
                    value: "Fix it"
                    onActivated: if (page.hsAlert) SettingsNav.openPage(page.hsAlert.page)
                }
                SetRow { chevron: true;
                    flat: true
                    icon: "folder"
                    title: (page.hsOn.length === 1 ? "1 shared folder" : page.hsOn.length + " shared folders") + (page.hsTs.state === "Running" ? " · Tailscale connected" : "")
                    sub: page.hsOn.length ? page.hsOn.map(s => s.name).slice(0, 3).join(", ") + (page.hsTs.state === "Running" ? " · " + (page.hsTs.peers || []).filter(p => p.online).length + " of your devices online" : "") : "Nothing is shared until you share it"
                    chip: page.hsOn.length ? "Running" : ""
                    onActivated: SettingsNav.openPage(page.hsOn.length ? "hs-overview" : "hs-shares")
                }
            }
            HomeCard {
                icon: "palette"; title: "Your look"; linkText: "Personalize"; linkTarget: "appearance"
                SetRow { chevron: false;
                    flat: true
                    title: Theme.lookLabel + " · " + (Settings.sunset ? "Sunset" : Theme.dark ? "Dark" : "Light") + " · " + page.accentName
                    sub: "Eight looks to choose from. Your wallpaper and accent stay yours."
                    trailing: 170
                    onActivated: SettingsNav.openPage("appearance")
                    Row {
                        anchors { right: parent.right; rightMargin: SetStyle.pad; verticalCenter: parent.verticalCenter }
                        spacing: 8
                        Repeater {
                            model: page.accents.slice(0, 5)
                            delegate: Rectangle {
                                id: swatch
                                required property var modelData
                                width: 26; height: 26; radius: 13
                                color: modelData.color
                                border.width: Qt.colorEqual(Theme.accent, modelData.color) ? 2 : 0
                                border.color: Theme.text
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { Settings.accentFrom = ""; Settings.accent = swatch.modelData.color; } }
                            }
                        }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- parts
    component Tile: SetBase {
        id: tile
        property string icon
        property string label
        property string status
        property string target
        property bool warn: false
        Layout.fillWidth: true
        implicitHeight: 76
        radius: SetStyle.bigRadius
        onActivated: SettingsNav.openPage(target)
        Accessible.name: label + ": " + status
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 14 }
            spacing: 14
            Rectangle {
                Layout.preferredWidth: 40; Layout.preferredHeight: 40; radius: Math.max(6, Theme.r(10))
                color: SetStyle.tint(Theme.accent, 0.18)
                Icon { anchors.centerIn: parent; name: tile.icon; size: 22; color: Theme.accent }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text { Layout.fillWidth: true; text: tile.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: tile.status; color: tile.warn ? SetStyle.warn : Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
            }
        }
    }
    component HomeCard: Rectangle {
        id: hc
        property string icon
        property string title
        property string sub
        property string linkText
        property string linkTarget
        default property alias rows: hcRows.data
        Layout.fillWidth: true
        implicitHeight: hcCol.implicitHeight + 28
        radius: SetStyle.bigRadius
        color: SetStyle.card
        border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
        ColumnLayout {
            id: hcCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14; leftMargin: 6; rightMargin: 6 }
            spacing: 2
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 12; Layout.rightMargin: 6
                spacing: 10
                Icon { name: hc.icon; size: 20; color: Theme.text }
                Text { Layout.fillWidth: true; text: hc.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight }
                SetBase {
                    visible: hc.linkText !== ""
                    flat: true
                    implicitWidth: linkLabel.implicitWidth + 16; implicitHeight: 28
                    onActivated: SettingsNav.openPage(hc.linkTarget)
                    Accessible.name: hc.linkText
                    Text { id: linkLabel; anchors.centerIn: parent; text: hc.linkText; color: Theme.accent; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.Medium }
                }
            }
            Text { visible: hc.sub !== ""; Layout.fillWidth: true; Layout.leftMargin: 12; Layout.bottomMargin: 4; text: hc.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            ColumnLayout { id: hcRows; Layout.fillWidth: true; spacing: 0 }
        }
    }
}
