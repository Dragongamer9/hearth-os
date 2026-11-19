// Game options (mockup firehold-plasma 03): X on a game's tile opens this over the dimmed Couch, for the controller.
// Play; Play on (each screen with its mode: the game starts there in that screen's mode); Open in a window; Controller
// layout; Pin to Home; Saves; Who's playing (a Hearth profile for each game); Game page; Uninstall… (asks first).
// Pin to Home and Who's playing are Firehold's settings (firehold.json "pins", "players", "playOn"): written only when
// you change one here, a key at a time through hearth_settings_io (red flag 9). Opening this writes nothing.
import QtQuick
import "fhp.js" as H
import "firehold.js" as F

Item {
    id: o
    property QtObject couch
    readonly property real k: couch.k
    readonly property var game: couch.optionsGame
    readonly property var p: game ? ((couch.cards.progress || {})[game.id] || {}) : ({})
    readonly property var screens: couch.screenList
    readonly property var profiles: couch.profiles
    readonly property string playOn: game && couch.cfg.playOn && couch.cfg.playOn[game.id] ? couch.cfg.playOn[game.id] : (screens.length ? screens[0].name : "")
    readonly property int playOnIndex: Math.max(0, screens.map(function (s) { return s.name; }).indexOf(playOn))
    readonly property string who: game && couch.cfg.players && couch.cfg.players[game.id] ? couch.cfg.players[game.id] : (profiles.length ? profiles[0].id : "")
    readonly property int whoIndex: Math.max(0, profiles.map(function (x) { return x.id; }).indexOf(who))
    readonly property bool pinned: !!game && couch.pins.indexOf(game.id) >= 0
    property bool confirmUninstall: false
    property int row: 0
    readonly property var items: ["play", "playOn", "window", "layout", "pin", "saves", "who", "page", "uninstall"]
    onGameChanged: { row = 0; confirmUninstall = false; }

    function whoName(x) { return x ? (x.name || couch.sys.user || "You") : ""; }
    function key(kk) {
        if (confirmUninstall) {
            if (kk === Qt.Key_Return || kk === Qt.Key_Enter) { couch.run(["do", "uninstall", game.id]); confirmUninstall = false; couch.optionsGame = null; }
            else if (kk === Qt.Key_Escape) confirmUninstall = false;
            return true;
        }
        if (kk === Qt.Key_Up) { row = Math.max(0, row - 1); return true; }
        if (kk === Qt.Key_Down) { row = Math.min(items.length - 1, row + 1); return true; }
        if (kk === Qt.Key_Left || kk === Qt.Key_Right) { change(items[row], kk === Qt.Key_Left ? -1 : 1); return true; }
        if (kk === Qt.Key_Return || kk === Qt.Key_Enter) { pick(items[row]); return true; }
        if (kk === Qt.Key_Escape || kk === Qt.Key_X) { couch.optionsGame = null; return true; }
        return true;     // the menu has the controller while it's open
    }
    function change(what, d) {
        if (what === "playOn" && screens.length > 1) setPlayOn((playOnIndex + d + screens.length) % screens.length);
        else if (what === "who" && profiles.length > 1) setWho((whoIndex + d + profiles.length) % profiles.length);
        else if (what === "pin") setPin(d > 0);
    }
    function setPlayOn(i) {
        const m = Object.assign({}, couch.cfg.playOn || {});
        m[game.id] = screens[i].name;
        couch.set("playOn", m);
    }
    function setWho(i) {
        const m = Object.assign({}, couch.cfg.players || {});
        m[game.id] = profiles[i].id;
        couch.set("players", m);
    }
    function setPin(on) {
        const l = couch.pins.filter(function (id) { return id !== o.game.id; });
        if (on) l.unshift(game.id);
        couch.set("pins", l.slice(0, 12));
    }
    function pick(what) {
        if (!game) return;
        if (what === "play" || what === "playOn") { couch.optionsGame = null; couch.play(game, false); }
        else if (what === "window") { couch.optionsGame = null; couch.play(game, true); }
        else if (what === "pin") setPin(!pinned);
        else if (what === "who") setWho((whoIndex + 1) % Math.max(1, profiles.length));
        else if (what === "uninstall") confirmUninstall = true;
        else { couch.optionsGame = null; couch.run(["page", game.id]); }   // Hearth's game page: layout, saves, files
    }
    function sub(what) {
        if (what === "play") return "On the screen you're looking at";
        if (what === "playOn") return "Starts there in that screen's mode";
        if (what === "window") return "On the Desk, next to your other apps";
        if (what === "layout") return p.layout ? p.layout : (game && game.source === "winapp" ? "The game wants a mouse · Hearth's layout for it" : "Steam Input or Hearth's layout for this game");
        if (what === "pin") return "Keeps it at the front of Continue playing";
        if (what === "saves") return p.saves ? p.saves : (game && game.savesBackedUp ? "Backed up by Saved Games" : game && game.cloudSaves ? "Kept by Steam Cloud" : "Not backed up yet");
        if (what === "who") { const x = profiles[whoIndex]; return x ? (whoIndex === 0 ? "Your profile" : whoName(x) + "'s profile") + " · " + (whoIndex === 0 ? "your" : "their") + " own saves and layout" : ""; }
        if (what === "page") return "Saves, screenshots, add-ons, Proton version, files";
        if (what === "uninstall") return "Asks first · saves stay on Hearth Storage";
        return "";
    }
    readonly property var titles: ({ play: "Play", playOn: "Play on", window: "Open in a window", layout: "Controller layout", pin: "Pin to Home",
                                     saves: "Saves", who: "Who's playing", page: "Game page", uninstall: "Uninstall…" })

    visible: !!game
    Rectangle { anchors.fill: parent; color: "#8c080605"; MouseArea { anchors.fill: parent; onClicked: o.couch.optionsGame = null } }
    Rectangle {
        id: panel
        anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom
        width: 700 * o.k
        color: "#1a1513"
        Rectangle { width: 1; height: parent.height; color: "#12ffffff" }
        MouseArea { anchors.fill: parent }
        Item {
            id: head
            x: 48 * o.k; y: 44 * o.k
            width: parent.width - 96 * o.k; height: 124 * o.k
            GameTile { id: htile; couch: o.couch; game: o.game; boxW: 220; boxH: 124; name: ""; height: 124 * o.k }
            Column {
                anchors.left: htile.right; anchors.leftMargin: 20 * o.k
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6 * o.k
                T { couchRef: o.couch; width: parent.width; text: o.game ? o.game.title : ""; font.pixelSize: 30 * o.k; font.weight: Font.ExtraBold }
                T {
                    couchRef: o.couch; width: parent.width; color: H.P.mut; font.pixelSize: 16 * o.k
                    text: o.game ? o.game.store + (o.p.runner ? " · runs with " + o.p.runner : o.game.source === "winapp" ? " · runs with Proton" : o.game.source === "beacon" ? " · through Beacon" : "") : ""
                }
                T {
                    couchRef: o.couch; width: parent.width; color: H.P.mut; font.pixelSize: 16 * o.k
                    visible: !!o.game && o.game.lastPlayed > 0
                    text: o.game ? "Last played" + (o.p.who && o.p.who !== o.couch.sys.user ? " by " + o.p.who : "") + " · " + H.agoShort(o.game.lastPlayed, o.couch.now.getTime()).toLowerCase() + " on this PC" : ""
                }
            }
        }
        Column {
            x: 48 * o.k; y: head.y + head.height + 30 * o.k
            width: parent.width - 96 * o.k
            spacing: 8 * o.k
            Repeater {
                model: o.items
                delegate: Item {
                    required property var modelData
                    required property int index
                    readonly property bool on: o.row === index
                    width: parent.width; height: 80 * o.k
                    Rectangle {
                        id: r
                        anchors.fill: parent
                        radius: 14 * o.k
                        color: parent.on ? "#24ff7a2f" : "#0dffffff"
                        Column {
                            x: 22 * o.k; anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 44 * o.k - right.width
                            spacing: 3 * o.k
                            T { couchRef: o.couch; text: o.titles[modelData]; color: modelData === "uninstall" ? "#ff9b9b" : H.P.tx; font.pixelSize: 18 * o.k; font.weight: Font.DemiBold }
                            T { couchRef: o.couch; width: parent.width; text: o.sub(modelData); color: H.P.mut; font.pixelSize: 15 * o.k }
                        }
                        Row {
                            id: right
                            anchors.right: parent.right; anchors.rightMargin: 22 * o.k
                            anchors.verticalCenter: parent.verticalCenter
                            Rectangle {
                                visible: modelData === "play"
                                width: 32 * o.k; height: width; radius: width / 2; color: H.keyColor("A")
                                T { couchRef: o.couch; anchors.centerIn: parent; text: "A"; color: "#111111"; font.pixelSize: 14 * o.k; font.weight: Font.ExtraBold }
                            }
                            Seg {
                                visible: modelData === "playOn" && o.screens.length > 0
                                couch: o.couch
                                options: o.screens.map(function (s) { return s.label + " · " + s.modeName; })
                                current: o.playOnIndex
                                onPicked: function (i) { o.setPlayOn(i); }
                            }
                            T { visible: ["layout", "saves", "page"].indexOf(modelData) >= 0; couchRef: o.couch; text: "›"; color: H.P.mut; font.pixelSize: 20 * o.k }
                            Toggle { visible: modelData === "pin"; k: o.k; on: o.pinned; onToggled: o.setPin(!o.pinned) }
                            Seg {
                                visible: modelData === "who" && o.profiles.length > 1
                                couch: o.couch
                                options: o.profiles.map(function (x) { return o.whoName(x); })
                                current: o.whoIndex
                                onPicked: function (i) { o.setWho(i); }
                            }
                        }
                        MouseArea { anchors.fill: parent; z: -1; onClicked: { o.row = index; o.pick(modelData); } }
                    }
                    Ring { visible: parent.on; k: o.k; r: 14; gap: 2; anchors.fill: r }
                }
            }
        }
        // Uninstall asks first.
        Rectangle {
            visible: o.confirmUninstall
            anchors.centerIn: parent
            width: parent.width - 96 * o.k; height: 170 * o.k
            radius: 18 * o.k; color: "#2a221f"; border.width: 1; border.color: "#24ffffff"
            Column {
                anchors.centerIn: parent
                spacing: 10 * o.k
                T { couchRef: o.couch; anchors.horizontalCenter: parent.horizontalCenter; text: o.game ? "Uninstall " + o.game.title + "?" : ""; font.pixelSize: 22 * o.k; font.weight: Font.Bold }
                T { couchRef: o.couch; anchors.horizontalCenter: parent.horizontalCenter; text: "Its saves stay on Hearth Storage. A: Uninstall · B: Keep it"; color: H.P.mut; font.pixelSize: 16 * o.k }
            }
        }
        Row {
            anchors.right: parent.right; anchors.rightMargin: 48 * o.k
            anchors.bottom: parent.bottom; anchors.bottomMargin: 26 * o.k
            spacing: 30 * o.k
            Hint { couch: o.couch; key: "A"; label: "Select" }
            Hint { couch: o.couch; key: "◀ ▶"; label: "Change" }
            Hint { couch: o.couch; key: "B"; label: "Back" }
        }
    }
}
