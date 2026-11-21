// Modes (mockup firehold-plasma 04): the card itself, every screen drawn as placed, each named, with its mode; Couch /
// Desk / Away for each; Both Couch, Both Desk, Away now; the Automatic and Away-after switches (Firehold's settings,
// written only when you flip one); the keys. Switching a screen is live state, never a setting. Drawn by Plasma's popup
// under the Desk bar's mode pill (posture/ModesPopup.qml) and by Firehold on labwc's (desktop/shell/FhModesPopup.qml,
// 1.16): one card on both. bar: modes, cfg, myScreen, modeName(), setMode(), setSetting(), helper(), popupOpen.
import QtQuick
import QtQuick.Window
import "fhp.js" as H

Rectangle {
    id: mp
    property QtObject bar: null
    readonly property QtObject ui: QtObject { property real k: 1; property string fontName: Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : "Noto Sans" }
    readonly property var screens: bar ? bar.modes.screens : []
    readonly property bool many: screens.length > 1
    function qscreen(name) {
        const l = Qt.application.screens;
        for (let i = 0; i < l.length; i++) if (l[i].name === name) return l[i];
        return null;
    }
    function connector(n) { const m = /^([A-Za-z]+)-?[A-Za-z]*-?(\d+)$/.exec(n || ""); return m ? (m[1].toUpperCase() === "HDMI" ? "HDMI " + m[2] : m[1].toUpperCase() === "DP" ? "DisplayPort " + m[2] : n) : n; }
    function label(s) {
        if (s.label && s.label !== "TV" && s.label !== "Monitor") return s.label;
        const q = qscreen(s.name);
        const made = q ? [q.manufacturer, q.model].filter(function (x) { return x && x !== "Unknown"; }).join(" ") : "";
        return made || (s.tv ? "TV" : "Monitor");
    }
    function whyText(s) {
        if (!s.why) return s.mode === "couch" ? "Couch: tiles, full screen" : s.mode === "away" ? "Away: nobody here" : "Desk: bar, windows, dock";
        const w = { pad: "Started Couch when the controller woke it", mouse: "Desk since you moved the mouse", keys: "Desk since you typed",
                    pick: "You picked " + bar.modeName(s.mode), idle: "Away: nobody was here", start: bar.modeName(s.mode) + " since the PC started" };
        return w[s.why] || bar.modeName(s.mode);
    }

    width: 800
    height: col.implicitHeight + 52
    radius: 14
    color: "#221e1c"
    border.width: 1; border.color: "#1affffff"
    focus: true
    Keys.onEscapePressed: mp.bar.popupOpen = false
    Column {
        id: col
        x: 26; y: 26
        width: parent.width - 52
        spacing: 0
        Item {
            width: parent.width; height: head.implicitHeight
            Column {
                id: head
                T { couchRef: mp.ui; text: "Modes"; font.pixelSize: 24; font.weight: Font.ExtraBold }
                Item { width: 1; height: 4 }
                T { couchRef: mp.ui; text: "Each screen has its own. Switching never closes anything."; color: H.P.mut; font.pixelSize: 15 }
            }
            Rectangle {
                anchors.right: parent.right; anchors.top: parent.top
                width: st.implicitWidth + 28; height: st.implicitHeight + 16; radius: 10; color: H.P.nb
                T { id: st; couchRef: mp.ui; anchors.centerIn: parent; text: "Settings ›"; font.pixelSize: 14; font.weight: Font.DemiBold }
                MouseArea { anchors.fill: parent; onClicked: { mp.bar.helper(["do", "settings-page", "couch-desk-away"]); mp.bar.popupOpen = false; } }
            }
        }
        Item { width: 1; height: 22 }
        // Each screen as placed (left to right), its mode in the middle and its name under it.
        Rectangle {
            width: parent.width; height: 258
            radius: 14; color: "#08ffffff"
            Row {
                anchors.centerIn: parent
                spacing: 18
                Repeater {
                    model: mp.screens.slice().sort(function (a, b) { const qa = mp.qscreen(a.name), qb = mp.qscreen(b.name); return (qa ? qa.virtualX : 0) - (qb ? qb.virtualX : 0); })
                    delegate: Column {
                        required property var modelData
                        readonly property bool big: modelData.tv
                        anchors.bottom: parent.bottom
                        spacing: 8
                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: parent.big ? 300 : 220; height: parent.big ? 170 : 124
                            radius: parent.big ? 10 : 8
                            border.width: 3
                            border.color: modelData.mode === "couch" ? H.P.ac : modelData.mode === "desk" ? H.P.blue : "#6f6a66"
                            gradient: Gradient {
                                orientation: Gradient.Horizontal
                                GradientStop { position: 0; color: modelData.mode === "couch" ? "#3a1d10" : modelData.mode === "desk" ? "#1d1f26" : "#141110" }
                                GradientStop { position: 1; color: modelData.mode === "couch" ? "#1a1210" : modelData.mode === "desk" ? "#1d1f26" : "#0a0807" }
                            }
                            Column {
                                anchors.centerIn: parent
                                T { couchRef: mp.ui; anchors.horizontalCenter: parent.horizontalCenter; text: mp.bar.modeName(modelData.mode); font.pixelSize: modelData.tv ? 20 : 18; font.weight: Font.Bold }
                                T { couchRef: mp.ui; anchors.horizontalCenter: parent.horizontalCenter; text: modelData.mode === "couch" ? "tiles · full screen" : modelData.mode === "desk" ? "bar · windows · dock" : "clock · what keeps working"; color: H.P.mut; font.pixelSize: 14 }
                            }
                            MouseArea { anchors.fill: parent; onClicked: mp.bar.setMode(modelData.name, modelData.mode === "couch" ? "desk" : "couch") }
                        }
                        T {
                            couchRef: mp.ui; anchors.horizontalCenter: parent.horizontalCenter
                            text: mp.label(modelData) + (modelData.tv ? " · " + (/(HDMI|DisplayPort|DVI) \d+/.exec(modelData.spec || "") || [mp.connector(modelData.name)])[0] : modelData.name === mp.bar.myScreen ? " · the mouse is here" : "")
                            font.pixelSize: 14
                        }
                    }
                }
            }
        }
        Item { width: 1; height: 18 }
        Column {
            width: parent.width
            spacing: 10
            Repeater {
                model: mp.screens
                delegate: Rectangle {
                    required property var modelData
                    width: parent.width; height: 80; radius: 14; color: "#2a2522"
                    Rectangle {
                        id: ico
                        x: 22; anchors.verticalCenter: parent.verticalCenter
                        width: 40; height: 40; radius: 11; color: modelData.tv ? "#3a2a22" : "#222a36"
                        Glyph { anchors.centerIn: parent; name: modelData.tv ? "tv" : "monitor"; size: 20; color: modelData.tv ? "#7fc7ff" : "#e8e2dc" }
                    }
                    Column {
                        anchors.left: ico.right; anchors.leftMargin: 14; anchors.verticalCenter: parent.verticalCenter
                        T { couchRef: mp.ui; text: mp.label(modelData) + (modelData.name === mp.bar.myScreen ? " · this screen" : ""); font.pixelSize: 18; font.weight: Font.DemiBold }
                        T { couchRef: mp.ui; text: mp.whyText(modelData); color: H.P.mut; font.pixelSize: 15 }
                    }
                    Seg {
                        anchors.right: parent.right; anchors.rightMargin: 22; anchors.verticalCenter: parent.verticalCenter
                        couch: mp.ui
                        options: ["Couch", "Desk", "Away"]
                        current: ["couch", "desk", "away"].indexOf(modelData.mode)
                        onColor: modelData.mode === "desk" ? H.P.blue : H.P.ac
                        onPicked: function (i) { mp.bar.setMode(modelData.name, ["couch", "desk", "away"][i]); }
                    }
                }
            }
        }
        Item { width: 1; height: 16 }
        Row {
            width: parent.width
            spacing: 10
            Repeater {
                model: [["Both Couch", "couch"], ["Both Desk", "desk"], [mp.many ? "Away now (both)" : "Away now", "away"]]
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: (parent.width - 20) / 3; height: 44; radius: 10
                    color: index === 2 ? H.P.ac : H.P.nb
                    T { couchRef: mp.ui; anchors.centerIn: parent; text: mp.many || index === 2 ? modelData[0] : modelData[0].replace("Both ", ""); color: index === 2 ? H.P.ink : H.P.tx; font.pixelSize: 14; font.weight: Font.DemiBold }
                    MouseArea { anchors.fill: parent; onClicked: { mp.bar.setMode("all", modelData[1]); if (index === 2) mp.bar.popupOpen = false; } }
                }
            }
        }
        Item { width: 1; height: 18 }
        Rectangle { width: parent.width; height: 1; color: "#14ffffff" }
        Item { width: 1; height: 12 }
        Repeater {
            model: [["auto", mp.many ? "Automatic: the controller brings the TV to Couch, the mouse brings this screen to Desk" : "Automatic: the controller brings the Couch, the mouse brings the Desk"],
                    ["away", "Away after " + (mp.bar.cfg.awayAfterOn || 20) + " min with nobody at " + (mp.many ? "either screen" : "the screen")]]
            delegate: Item {
                required property var modelData
                required property int index
                width: col.width; height: 42
                T { couchRef: mp.ui; anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; font.pixelSize: 15; width: parent.width - 70 }
                Toggle {
                    anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                    on: modelData[0] === "auto" ? (mp.bar.cfg.posture || "auto") === "auto" : (typeof mp.bar.cfg.awayAfter === "number" ? mp.bar.cfg.awayAfter > 0 : true)
                    onToggled: {
                        if (modelData[0] === "auto") mp.bar.setSetting("posture", on ? "manual" : "auto");
                        else mp.bar.setSetting("awayAfter", on ? 0 : (mp.bar.cfg.awayAfterOn || 20));
                    }
                }
            }
        }
        Item { width: 1; height: 8 }
        T { couchRef: mp.ui; text: "Keys: Super+Shift+C Couch · Super+Shift+D Desk · Super+Shift+A Away · Guide button: Couch on the " + (mp.screens.some(function (s) { return s.tv; }) ? "TV" : "screen"); color: H.P.dim; font.pixelSize: 13 }
    }
}
