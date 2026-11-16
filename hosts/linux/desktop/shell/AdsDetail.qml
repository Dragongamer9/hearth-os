// Server mode › Ad blocker, opened (A on its tile; 1.13, mockup docs/mockups/next-update/server-adblock.png): Today
// (% of lookups blocked, ads and trackers stopped, the average answer), Most blocked, Your devices (this PC and your
// devices over Tailscale, each "Using it" / "Paused 1 h" with its switch, and everything else on your Wi-Fi through the
// router) and Never stops the internet. The controller: Up / Down pick a row (the ad blocker itself, or a device), A
// turns it on or off, X allows a site (typed here), Y pauses blocking for an hour (or pauses the picked device for an
// hour), ☰ opens Settings, B goes back. Draw-only: ServerDash runs the actions (hearth-server-mode dns …).
import QtQuick
import QtQuick.Layouts

Item {
    id: det
    property var t: null
    property real k: 1
    property int sel: 0                    // 0: the ad blocker; 1…: the devices
    property bool typing: false            // X: the site box is open
    signal action(var args)
    signal close()
    // The two columns' widths come from the page's width: sized by ratio they kept the layouts busy forever
    readonly property real colsW: Math.max(0, width - 84 * k)
    readonly property var a: t && t.ads ? t.ads : ({})
    readonly property var v: a.view || ({})
    readonly property var today: v.today || ({})
    readonly property var devices: v.devices || []
    readonly property int rows: 1 + devices.length
    function pick(d) { if (!typing) sel = Math.max(0, Math.min(rows - 1, sel + d)); }
    function enter() {
        if (typing) { allow(); return; }
        if (sel === 0) action(["ads", a.on ? "off" : "on"]);
        else { const d = devices[sel - 1]; action(["device", d.id, d.on ? "off" : "on"]); }
    }
    function pause() {
        if (sel === 0) action(["ads-pause", v.pausedUntil ? "0" : "3600"]);
        else action(["device", devices[sel - 1].id, "pause"]);
    }
    function startAllow() { typing = true; site.text = ""; site.forceActiveFocus(); }
    function allow() { const s = site.text.trim(); typing = false; if (s) action(["allow", s]); }
    function back() { if (typing) { typing = false; return true; } return false; }
    onVisibleChanged: { sel = 0; typing = false; }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32 * det.k
        spacing: 20 * det.k
        ServerViewHead {
            k: det.k; icon: "shield"; title: "Ad blocker"
            sub: "AdGuard Home · your home Wi-Fi and your Hearth devices over Tailscale · off until you turn it on"
            chip: det.a.on ? (det.v.pausedUntil ? "Paused" : det.a.running ? "On" : "Starting") : "Off"
            chipColor: det.a.on && !det.v.pausedUntil ? "#6fe39a" : "#c9b79f"
            picked: det.sel === 0 && !det.typing
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: false   // the panels sit right under the title; the space left goes below them
            spacing: 20 * det.k
            ColumnLayout {
                Layout.fillWidth: false; Layout.preferredWidth: Math.floor(det.colsW * 1.2/2.2); Layout.minimumWidth: Math.floor(det.colsW * 1.2/2.2); Layout.maximumWidth: Math.floor(det.colsW * 1.2/2.2); Layout.alignment: Qt.AlignTop; Layout.fillHeight: false
                spacing: 20 * det.k
                ServerPanel {
                    k: det.k; title: "Today"
                    Item {   // the three numbers, a third of the panel each
                        id: todayRow
                        Layout.fillWidth: true
                        implicitHeight: 82 * det.k
                        Repeater {
                            model: [[(det.today.percent || 0) + " %", "of lookups blocked"],
                                    [Number(det.today.blocked || 0).toLocaleString(Qt.locale("en_US"), "f", 0), "ads and trackers stopped"],
                                    [(det.today.ms || 0) + " ms", "average answer"]]
                            delegate: Column {
                                required property var modelData
                                required property int index
                                x: index * todayRow.width / 3; width: todayRow.width / 3
                                Text { text: det.a.on ? modelData[0] : "—"; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 52 * det.k; font.weight: Font.Bold }
                                Text { text: modelData[1]; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 15 * det.k }
                            }
                        }
                    }
                }
                ServerPanel {
                    k: det.k; title: "Most blocked"
                    Repeater {
                        model: det.v.top || []
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Text { Layout.fillWidth: true; text: modelData.name; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 18 * det.k; elide: Text.ElideRight }
                            Text { text: Number(modelData.count).toLocaleString(Qt.locale("en_US"), "f", 0); color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 17 * det.k }
                        }
                    }
                    Text { visible: !(det.v.top || []).length; text: det.a.on ? "Nothing blocked yet today." : "Off."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 16 * det.k }
                }
            }
            ColumnLayout {
                Layout.fillWidth: false; Layout.preferredWidth: Math.floor(det.colsW * 1/2.2); Layout.minimumWidth: Math.floor(det.colsW * 1/2.2); Layout.maximumWidth: Math.floor(det.colsW * 1/2.2); Layout.alignment: Qt.AlignTop; Layout.fillHeight: false
                spacing: 20 * det.k
                ServerPanel {
                    k: det.k; title: "Your devices"
                    Repeater {
                        model: det.devices
                        delegate: ServerSwitch {
                            required property var modelData
                            required property int index
                            k: det.k
                            label: modelData.name
                            chip: modelData.state || ""
                            chipKind: (modelData.state || "").indexOf("Paused") === 0 || (modelData.state || "").indexOf("Set your") === 0 ? "warn" : "ok"
                            on: !!modelData.on
                            picked: det.sel === index + 1 && !det.typing
                            onToggled: { det.sel = index + 1; det.enter(); }
                        }
                    }
                }
                ServerPanel {
                    k: det.k; title: "Never stops the internet"
                    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "If this PC can't be reached, each device goes back to normal lookups by itself within seconds. A site that breaks: allow it from here or your phone."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 16 * det.k }
                }
                ServerPanel {
                    visible: det.typing
                    k: det.k; title: "Allow a site"
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: 48 * det.k; radius: 10 * det.k; color: "#14100c"; border.color: Theme.accent; border.width: 2
                        TextInput {
                            id: site
                            anchors { fill: parent; leftMargin: 14 * det.k; rightMargin: 14 * det.k }
                            verticalAlignment: TextInput.AlignVCenter
                            color: "#fff4df"; font.family: Theme.font; font.pixelSize: 19 * det.k
                            onAccepted: det.allow()
                            Keys.onEscapePressed: det.typing = false
                        }
                        Text { visible: site.text === ""; anchors { left: parent.left; leftMargin: 14 * det.k; verticalCenter: parent.verticalCenter } text: "example.com"; color: "#7a6a58"; font.family: Theme.font; font.pixelSize: 19 * det.k }
                    }
                    Text { text: "A or Enter allows it · B cancels"; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 14 * det.k }
                }
            }
        }
        Item { Layout.fillHeight: true }
        ServerKeys {
            k: det.k
            keys: [["A", "Turn on or off", "enter"], ["X", "Allow a site…", "allow"], ["Y", det.sel === 0 ? (det.v.pausedUntil ? "Block again" : "Pause for 1 hour") : "Pause this device for 1 hour", "pause"], ["☰", "Settings", "settings"]]
            onPressed: x => x === "enter" ? det.enter() : x === "allow" ? det.startAllow() : x === "pause" ? det.pause() : det.action(["settings"])
            onBackPressed: { if (!det.back()) det.close(); }
        }
    }
}
