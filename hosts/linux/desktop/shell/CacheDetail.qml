// Server mode › Game download cache, opened (A on its tile; 1.13, mockup docs/mockups/next-update/server-cache.png):
// Saved this week (GB not downloaded again, what came from the internet once and what went to your devices from here,
// the days as bars), Right now (who's downloading what, from the cache or from the internet once, and how fast),
// Stores (Steam, Epic Games, Battle.net, Xbox / Microsoft Store, each with its switch), Space (which drive, how much
// of how much, the oldest games leave first) and If this PC isn't there. The controller: Up / Down pick a row (the
// cache itself, or a store), A turns it on or off, X empties the cache (asked twice), Y opens Settings, B goes back.
// Draw-only: ServerDash runs the actions (hearth-server-mode dns …).
import QtQuick
import QtQuick.Layouts

Item {
    id: det
    property var t: null
    property real k: 1
    property int sel: 0                    // 0: the cache itself; 1…: the stores
    property bool askEmpty: false          // X once: "Press X again to empty it"
    signal action(var args)
    signal close()
    // The two columns' widths come from the page's width: sized by ratio they kept the layouts busy forever
    readonly property real colsW: Math.max(0, width - 84 * k)
    readonly property var c: t && t.cache ? t.cache : ({})
    readonly property var v: c.view || ({})
    readonly property var storeIds: ["steam", "epic", "blizzard", "xbox"]
    readonly property int rows: 1 + storeIds.length
    function gb(b) { b = b || 0; return b >= 1e12 ? (Math.round(b / 1e11) / 10) + " TB" : b >= 10e9 ? Math.round(b / 1e9) + " GB" : (Math.round(b / 1e8) / 10) + " GB"; }
    function rate(r) { return r >= 1e6 ? Math.round(r / 1e6) + " MB/s" : Math.round(r / 1e3) + " KB/s"; }
    function pick(d) { sel = Math.max(0, Math.min(rows - 1, sel + d)); askEmpty = false; }
    function enter() {
        askEmpty = false;
        if (sel === 0) action(["cache", c.on ? "off" : "on"]);
        else { const s = storeIds[sel - 1]; action(["store", s, (c.stores || {})[s] ? "off" : "on"]); }
    }
    function empty() { if (askEmpty) { askEmpty = false; action(["cache-empty", "sure"]); } else askEmpty = true; }
    onVisibleChanged: { sel = 0; askEmpty = false; }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32 * det.k
        spacing: 20 * det.k
        ServerViewHead {
            k: det.k; icon: "update"; title: "Game download cache"
            sub: "LANCache on this PC · on your home network (and Tailscale) · off until you turn it on"
            chip: det.c.on ? (det.c.running ? "On" : "Starting") : "Off"
            chipColor: det.c.on ? "#6fe39a" : "#c9b79f"
            picked: det.sel === 0
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 20 * det.k
            ColumnLayout {   // left
                Layout.fillWidth: false; Layout.preferredWidth: Math.floor(det.colsW * 1.2/2.2); Layout.minimumWidth: Math.floor(det.colsW * 1.2/2.2); Layout.maximumWidth: Math.floor(det.colsW * 1.2/2.2); Layout.alignment: Qt.AlignTop
                spacing: 20 * det.k
                ServerPanel {
                    k: det.k; title: "Saved this week"
                    Row {
                        spacing: 12 * det.k
                        Text { id: big; text: det.gb(det.v.savedWeek); color: "#fff4df"; font.family: Theme.font; font.pixelSize: 54 * det.k; font.weight: Font.Bold }
                        Text { anchors.baseline: big.baseline; text: "not downloaded again"; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 21 * det.k }
                    }
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: "Your internet only gave each game once: " + det.gb(det.v.internetWeek) + " came from the internet, " + det.gb(det.v.devicesWeek) + " went to your devices from here."
                        color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 15 * det.k
                    }
                    RowLayout {   // the days
                        Layout.fillWidth: true; Layout.preferredHeight: 170 * det.k
                        spacing: 10 * det.k
                        id: days
                        readonly property real most: Math.max(1, Math.max.apply(null, (det.v.bars || []).map(b => b.saved || 0).concat([0])))
                        Repeater {
                            model: det.v.bars || []
                            delegate: ColumnLayout {
                                required property var modelData
                                Layout.fillWidth: true; Layout.fillHeight: true
                                spacing: 4 * det.k
                                Item { Layout.fillHeight: true }
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: Math.max(4 * det.k, 130 * det.k * (modelData.saved || 0) / days.most)
                                    radius: 3 * det.k
                                    gradient: Gradient { GradientStop { position: 0; color: "#f2994a" } GradientStop { position: 1; color: "#b8611f" } }
                                }
                                Text { text: modelData.day; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 14 * det.k }
                            }
                        }
                    }
                }
                ServerPanel {
                    k: det.k; title: "Right now"
                    Repeater {
                        model: det.v.now || []
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Text { Layout.fillWidth: true; text: modelData.device + " · " + modelData.what; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 18 * det.k; elide: Text.ElideRight }
                            Text { text: modelData.from + " · " + det.rate(modelData.rate); color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 17 * det.k }
                        }
                    }
                    Text { visible: !(det.v.now || []).length; text: det.c.on ? "Nobody is downloading right now." : "Off: your devices download from the internet as usual."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 16 * det.k }
                }
            }
            ColumnLayout {   // right
                Layout.fillWidth: false; Layout.preferredWidth: Math.floor(det.colsW * 1/2.2); Layout.minimumWidth: Math.floor(det.colsW * 1/2.2); Layout.maximumWidth: Math.floor(det.colsW * 1/2.2); Layout.alignment: Qt.AlignTop
                spacing: 20 * det.k
                ServerPanel {
                    k: det.k; title: "Stores"
                    Repeater {
                        model: det.storeIds
                        delegate: ServerSwitch {
                            required property string modelData
                            required property int index
                            k: det.k
                            label: (det.c.storeNames || {})[modelData] || modelData
                            on: !!(det.c.stores || {})[modelData]
                            picked: det.sel === index + 1
                            onToggled: { det.sel = index + 1; det.enter(); }
                        }
                    }
                }
                ServerPanel {
                    k: det.k; title: "Space"
                    Text { Layout.fillWidth: true; textFormat: Text.StyledText; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 18 * det.k; elide: Text.ElideRight
                        text: (det.v.drive || "This PC") + " · <b>" + det.gb(det.v.used) + "</b> of " + det.gb((det.c.sizeGB || 0) * 1e9) + " for the cache" }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: 10 * det.k; radius: 5 * det.k; color: "#14100c"
                        Rectangle { width: parent.width * Math.min(1, (det.v.used || 0) / Math.max(1, (det.c.sizeGB || 1) * 1e9)); height: parent.height; radius: parent.radius; color: "#f2994a" }
                    }
                    Text { text: "The oldest games leave the cache first when it's full."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 15 * det.k }
                }
                ServerPanel {
                    k: det.k; title: "If this PC isn't there"
                    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Your devices download from the internet as normal. Nothing waits for the server, and nothing breaks."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 16 * det.k }
                }
                Text { visible: det.askEmpty; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Press X again to empty the cache (your devices download each game from the internet once more)."; color: "#f0c060"; font.family: Theme.font; font.pixelSize: 16 * det.k }
            }
        }
        ServerKeys {
            k: det.k
            keys: [["A", "Turn on or off", "enter"], ["X", "Empty the cache (asks twice)", "empty"], ["Y", "Settings", "settings"]]
            onPressed: a => a === "enter" ? det.enter() : a === "empty" ? det.empty() : det.action(["settings"])
            onBackPressed: det.close()
        }
    }
}
