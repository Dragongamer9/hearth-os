// Firehold's Desk bar: the mode pill (mockup firehold-plasma 04: "● Desk · TV: Couch", this screen's mode and the TV's)
// and Wi-Fi · battery. Press the pill: Modes (ModesPopup.qml), each screen's Couch, Desk or Away. On a screen whose
// mode is Couch or Away while another is Desk, this applet (on that screen's bar) shows the Couch or Away over the
// Desk as a full-screen layer (CouchLayer.qml, the Couch's own files copied in by install.sh). It also tells
// hearth-firehold-plasma the Desk came up. Battery from Plasma's own source; the modes when the helper says they
// changed (its "modes" key, through Plasma's scripting): no timer.
import QtQuick
import QtQml
import QtQuick.Layouts
import QtQuick.Window
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasma5support as P5Support

PlasmoidItem {
    id: root
    preferredRepresentation: fullRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground

    P5Support.DataSource {
        id: sh
        engine: "executable"
        property var cbs: ({})
        property int n: 0
        onNewData: function (source, data) {
            const cb = sh.cbs[source];
            delete sh.cbs[source];
            sh.disconnectSource(source);
            if (cb) cb(String(data["stdout"] || ""));
        }
        function run(cmd, cb) { sh.n += 1; cmd += " #" + sh.n; sh.cbs[cmd] = cb || null; sh.connectSource(cmd); }
    }
    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    function helper(args, cb) { sh.run("hearth-firehold-plasma " + args.map(q).join(" "), cb); }
    function lastJson(t) { try { return JSON.parse(String(t || "").trim().split("\n").pop()); } catch (e) { return null; } }

    P5Support.DataSource {
        id: power
        engine: "powermanagement"
        connectedSources: ["Battery"]
    }
    readonly property var battery: power.data["Battery"] || ({})
    readonly property string batteryText: battery["Has Battery"] ? Math.round(battery["Percent"] || 0) + "%" : ""
    property string net: ""
    property var cfg: ({})

    // ---------------------------------------------------------------- the modes (live state, never a setting)
    property var modes: ({ screens: [] })
    function readModes() { helper(["modes"], function (t) { const o = root.lastJson(t); if (o && Array.isArray(o.screens)) root.modes = o; }); }
    readonly property string myScreen: Screen.name
    readonly property var me: {
        for (const s of modes.screens) if (s.name === myScreen) return s;
        return { name: myScreen, mode: "desk", tv: false, why: "" };
    }
    readonly property var tv: {
        for (const s of modes.screens) if (s.tv && s.name !== myScreen) return s;
        return null;
    }
    function modeName(m) { return m === "couch" ? "Couch" : m === "away" ? "Away" : "Desk"; }
    readonly property string modesPing: Plasmoid.configuration.modes || ""
    onModesPingChanged: {
        const parts = modesPing.split("|");
        const w = parts[0];
        if (w === "identify" || w === "identify-off") identifyOn = w === "identify";
        else if (w === "popup" || w === "popup-off") { if (!parts[2] || parts[2] === myScreen) popupOpen = w === "popup"; }
        else readModes();
    }
    property bool identifyOn: false
    Loader { active: root.identifyOn; source: "IdentifyLayer.qml"; onLoaded: item.bar = root }
    readonly property string modeKey: Plasmoid.configuration.key || ""
    onModeKeyChanged: {
        const p = modeKey.split("|");
        if (p.length === 3 && p[1] === myScreen && ["couch", "desk", "away"].indexOf(p[0]) >= 0) setMode(myScreen, p[0]);
    }
    function setMode(target, m) { helper(["mode", target, m, "--why", "pick"], function () { root.readModes(); }); }
    function setSetting(key, value) {
        const c = Object.assign({}, cfg); c[key] = value; cfg = c;
        helper(["set", key, JSON.stringify(value)]);
    }

    property bool popupOpen: false
    property int notifCount: 0      // the shared notifications applet's count, once it's there
    property Item pillItem: null
    Loader {
        active: root.popupOpen
        source: "ModesPopup.qml"
        onLoaded: item.bar = root
    }
    // Each Couch or Away screen's page over the Desk (the bar is on one screen; it opens them on every screen).
    Instantiator {
        model: root.modes.screens.filter(function (s) { return s.mode === "couch" || s.mode === "away"; }).map(function (s) { return s.name; })
        delegate: Loader {
            required property var modelData
            source: "CouchLayer.qml"
            onLoaded: { item.bar = root; item.screenName = modelData; item.show(); }
        }
    }

    // ---------------------------------------------------------------- the bar's right side (mockup 11): downloads,
    // Hearth Link, the controllers, Quick Settings and notifications. The cards come from hearth-firehold-plasma home
    // (it sleeps between looks: every 5 s while something downloads, else once a minute).
    property var cards: ({})
    readonly property var dls: (cards.downloads || [])
    readonly property var running: dls.filter(function (d) { return d.state === "running"; })
    readonly property var pads: (cards.controllers || []).filter(function (c) { return c.kind === "pad"; })
    function askCards(first) {
        const busy = running.length > 0;
        helper(first ? ["home", "fast"] : ["home-after", busy ? "5" : "60", "fast"], function (t) {
            const o = root.lastJson(t);
            if (o) root.cards = o;
            root.askCards(false);
        });
    }
    // Quick Settings and notifications are the shared Hearth applets when they're there, else Hearth's own Settings.
    function openShared(id, fallbackPage) {
        helper(["do", "open-applet", id], function (t) {
            const o = root.lastJson(t);
            if (o && o.missing) helper(["do", "settings-page", fallbackPage]);
        });
    }
    // The first time the Desk comes up: what each part of the bar is (the tip, once; state, not a setting).
    property bool tipShown: false
    Loader { active: root.tipShown; source: "BarTip.qml"; onLoaded: item.bar = root }

    Component.onCompleted: {
        helper(["ready", "org.hearthos.firehold.desk"]);
        askCards(true);
        helper(["tip"], function (t) { if (t.trim() === "show") root.tipShown = true; });
        helper(["sys"], function (t) { const o = root.lastJson(t); if (o) root.net = o.net || ""; });
        helper(["get"], function (t) { const o = root.lastJson(t); if (o && typeof o === "object") root.cfg = o; });
        readModes();
    }

    fullRepresentation: Item {
        Layout.minimumWidth: row.implicitWidth + 12
        Layout.preferredWidth: row.implicitWidth + 12
        Layout.fillHeight: true
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 12
            Rectangle {
                id: pill
                Component.onCompleted: root.pillItem = pill
                anchors.verticalCenter: parent.verticalCenter
                height: 30; radius: 15
                width: pillRow.implicitWidth + 26
                color: root.popupOpen ? "#38ff7a2f" : "#12ffffff"
                Row {
                    id: pillRow
                    anchors.centerIn: parent
                    spacing: 8
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 9; height: 9; radius: 4.5; color: root.me.mode === "couch" ? "#ff7a2f" : "#5bb8ff" }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: root.modeName(root.me.mode); color: "#f2ede8"; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Text { anchors.verticalCenter: parent.verticalCenter; visible: !!root.tv; text: root.tv ? "· TV: " + root.modeName(root.tv.mode) : ""; color: "#6f6a66"; font.pixelSize: 14; font.weight: Font.Medium }
                }
                MouseArea { anchors.fill: parent; onClicked: root.popupOpen = !root.popupOpen }
            }
            Row {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6
                readonly property var ui: QtObject { property real k: 1; property string fontName: Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : "Noto Sans" }
                Tray {
                    visible: root.dls.length > 0
                    text: String(root.running.length || root.dls.length)
                    ring: root.running.length ? root.running[0].percent : -1
                    onClicked: root.helper(["do", "settings-page", "storage"])
                }
                Tray { glyph: "link"; onClicked: root.helper(["do", "settings-page", "link"]) }
                Tray { visible: root.pads.length > 0; glyph: "pad"; text: root.pads.length ? root.pads[0].percent + "%" : "" }
                Tray {
                    glyph: "speaker"
                    text: root.net || (root.batteryText !== "" ? root.batteryText : "")
                    onClicked: root.openShared("org.hearthos.quicksettings", "quick")
                }
                Tray { glyph: "bell"; text: root.notifCount > 0 ? String(root.notifCount) : ""; onClicked: root.openShared("org.hearthos.notifications", "notifications") }
            }
        }
    }
}
