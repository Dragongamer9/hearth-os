// Settings › Couch and Desk (mockup 6). Every choice is saved in ~/.config/nimbus/firehold.json (shared with Mantel's
// Firehold) a key at a time, only when you change it; opening the page writes nothing. System holds the Desktop Mode
// choice and the ways out (plain KDE Plasma, Game Mode); the other pages are KDE's own (System Settings).
import QtQuick

Item {
    id: st
    property QtObject couch
    readonly property real k: couch.k
    property string section: "couch"
    property int nav: 1                 // the list's row
    property bool inNav: false
    property int item: 0                // 0..2 the posture cards, 3.. the rows
    property string choice: ""
    readonly property var navItems: [["system", "System"], ["couch", "Couch and Desk"], ["kcm_kscreen", "Display"],
        ["gaming", "Gaming"], ["kcm_gamecontroller", "Controllers"], ["kcm_networkmanagement", "Network"],
        ["kcm_pulseaudio", "Sound"], ["kcm_users", "Accounts"], ["kcm_updates", "Updates"]]
    readonly property var cards: [["auto", "Automatic", "Controller → Couch, mouse or keyboard → Desk"],
        ["couch", "Always Couch", "A console: tiles, full screen, Guide button"],
        ["desk", "Always Desk", "A PC: windows, bar and dock"]]
    readonly property var startNames: ({ couch: "Couch", desk: "Desk", last: "Where you left off" })
    readonly property var gameNames: ({ follow: "Follow the posture", fullscreen: "Always full screen", window: "Always in a window" })
    readonly property var rows: section === "system"
        ? [["Desktop Mode", "Firehold (Plasma, test) or plain KDE Plasma, from the next start", choice === "firehold" ? "Firehold (Plasma, test)" : choice === "plasma" ? "KDE Plasma" : "…"],
           ["Plain KDE Plasma now", "Until Desktop Mode starts again. Everything stays open", "Switch"],
           ["Back to Game Mode", "Steam's Game Mode", "Go"]]
        : [["Per screen", "The TV stays Couch while the monitor is Desk", "One posture for every screen on Plasma"],
           ["Start in", "What you see when the PC turns on", startNames[couch.startIn]],
           ["Guide button", "Press once: Quick Access. Twice: everything open", couch.guideButton === "open" ? "Everything open" : "Quick Access"],
           ["Games", "Open full screen on the Couch, in a window on the Desk", gameNames[couch.gamesOpen]],
           ["Big Picture", "Steam's own TV mode isn't needed; Steam is an app like the others", couch.bigPicture ? "On" : "Off"]]
    readonly property int first: section === "system" ? 0 : 3
    readonly property int last: first + rows.length - 1
    onVisibleChanged: if (visible) couch.run(["choice"], function (t) { st.choice = t.trim(); })

    function cycle(list, cur) { return list[(list.indexOf(cur) + 1) % list.length]; }
    function press(i) {
        if (section === "couch" && i < 3) { couch.set("posture", cards[i][0]); return; }
        const r = i - first;
        if (section === "system") {
            if (r === 0) {
                const v = choice === "firehold" ? "plasma" : "firehold";
                couch.run(["choose", v], function () { st.choice = v; });
            } else if (r === 1) couch.run(["do", "plain"]);
            else if (r === 2) couch.run(["do", "gamemode"]);
            return;
        }
        if (r === 1) couch.set("startIn", cycle(["couch", "desk", "last"], couch.startIn));
        else if (r === 2) couch.set("guide", couch.guideButton === "open" ? "quick" : "open");
        else if (r === 3) couch.set("games", cycle(["follow", "fullscreen", "window"], couch.gamesOpen));
        else if (r === 4) couch.set("bigPicture", !couch.bigPicture);
    }
    function pick(n) {
        nav = n;
        const id = navItems[n][0];
        if (id === "system" || id === "couch") { section = id; item = 0; }
        else couch.run(["do", "systemsettings", id === "gaming" ? "" : id]);
    }
    function key(kk) {
        if (kk === Qt.Key_Escape || kk === Qt.Key_Backspace) { couch.page = "tabs"; return true; }
        if (inNav) {
            if (kk === Qt.Key_Up) nav = Math.max(0, nav - 1);
            else if (kk === Qt.Key_Down) nav = Math.min(navItems.length - 1, nav + 1);
            else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) pick(nav);
            else if (kk === Qt.Key_Right) inNav = false;
            else return false;
            return true;
        }
        if (kk === Qt.Key_Left) { if (section === "couch" && item > 0 && item < 3) item -= 1; else inNav = true; }
        else if (kk === Qt.Key_Right) { if (section === "couch" && item < 2) item += 1; }
        else if (kk === Qt.Key_Up) {
            if (section === "couch" && item === 3) item = 0;
            else if (item > first && !(section === "couch" && item < 3)) item -= 1;
        }
        else if (kk === Qt.Key_Down) item = section === "couch" && item < 3 ? 3 : Math.min(last, item + 1);
        else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) press(item);
        else return false;
        return true;
    }

    Rectangle {
        id: win
        anchors.centerIn: parent
        width: 1520 * st.k; height: 940 * st.k; radius: 24 * st.k
        color: "#1a1311"
        border.width: 1; border.color: "#2e231e"
        clip: true
        Rectangle {
            id: side
            width: 340 * st.k; height: parent.height
            radius: parent.radius
            color: "#16100d"
            Rectangle { anchors.right: parent.right; width: parent.radius; height: parent.height; color: parent.color }
            Column {
                x: 19 * st.k; y: 36 * st.k
                spacing: 6 * st.k
                Repeater {
                    model: st.navItems
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool on: (modelData[0] === st.section) || (st.inNav && index === st.nav)
                        width: 303 * st.k; height: 46 * st.k; radius: 10 * st.k
                        color: modelData[0] === st.section ? "#2c221d" : "transparent"
                        border.width: st.inNav && index === st.nav ? 2 * st.k : 0
                        border.color: st.couch.pal.accent
                        T {
                            couchRef: st.couch
                            x: 16 * st.k; anchors.verticalCenter: parent.verticalCenter
                            text: modelData[1]
                            color: parent.modelData[0] === st.section ? st.couch.pal.text : "#b8aaa0"
                            font.pixelSize: 19 * st.k
                            font.weight: parent.modelData[0] === st.section ? Font.Bold : Font.Normal
                        }
                        MouseArea { anchors.fill: parent; onClicked: { st.inNav = false; st.pick(index); } }
                    }
                }
            }
        }
        Item {
            x: side.width + 53 * st.k; y: 0
            width: parent.width - x - 53 * st.k; height: parent.height
            T {
                couchRef: st.couch
                y: 40 * st.k
                text: st.section === "system" ? "System" : "Couch and Desk"
                font.pixelSize: 38 * st.k; font.weight: Font.Bold
            }
            T {
                couchRef: st.couch
                y: 104 * st.k; width: parent.width
                wrapMode: Text.WordWrap; elide: Text.ElideNone
                text: st.section === "system" ? "Firehold on KDE Plasma is a test. If it doesn't start, plain KDE Plasma opens instead and your apps stay open."
                      : "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything."
                color: "#d8cbc1"; font.pixelSize: 19 * st.k; lineHeight: 1.05
            }
            Row {
                id: cardRow
                visible: st.section === "couch"
                y: 170 * st.k
                spacing: 19 * st.k
                Repeater {
                    model: st.cards
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool chosen: st.couch.mode === modelData[0]
                        readonly property bool focused: !st.inNav && st.item === index
                        width: 346 * st.k; height: 240 * st.k; radius: 16 * st.k
                        color: "#1d1613"
                        border.width: chosen || focused ? 2 * st.k : 1
                        border.color: chosen ? st.couch.pal.accent : focused ? "#c98a5e" : "#3a2d26"
                        Item {
                            x: 17 * st.k; y: 17 * st.k
                            width: parent.width - 34 * st.k; height: 130 * st.k
                            Rectangle {
                                visible: index === 0
                                width: parent.width / 2; height: parent.height; radius: 10 * st.k
                                color: "#5b2b1a"
                                Rectangle { anchors.right: parent.right; width: parent.radius; height: parent.height; color: parent.color }
                            }
                            Rectangle {
                                visible: index === 0
                                x: parent.width / 2; width: parent.width / 2; height: parent.height; radius: 10 * st.k
                                color: "#2c2826"
                                Rectangle { width: parent.radius; height: parent.height; color: parent.color }
                            }
                            Rectangle {
                                visible: index === 1
                                anchors.fill: parent; radius: 10 * st.k
                                gradient: Gradient {
                                    orientation: Gradient.Horizontal
                                    GradientStop { position: 0.0; color: "#2b1a2e" }
                                    GradientStop { position: 0.5; color: "#5b2b1a" }
                                    GradientStop { position: 1.0; color: "#a3541e" }
                                }
                            }
                            Rectangle { visible: index === 2; anchors.fill: parent; radius: 10 * st.k; color: "#2c2826" }
                        }
                        T {
                            couchRef: st.couch
                            x: 17 * st.k; y: 158 * st.k
                            text: modelData[1]
                            font.pixelSize: 20 * st.k; font.weight: Font.Bold
                        }
                        T {
                            couchRef: st.couch
                            x: 17 * st.k; y: 190 * st.k; width: parent.width - 34 * st.k
                            wrapMode: Text.WordWrap; elide: Text.ElideNone; maximumLineCount: 2
                            text: modelData[2]
                            color: "#b8aaa0"; font.pixelSize: 15 * st.k
                        }
                        MouseArea { anchors.fill: parent; onClicked: { st.inNav = false; st.item = index; st.press(index); } }
                    }
                }
            }
            Rectangle {
                y: st.section === "couch" ? 435 * st.k : 170 * st.k
                width: parent.width
                height: rowCol.height
                radius: 14 * st.k
                color: "#1c1512"
                Column {
                    id: rowCol
                    width: parent.width
                    Repeater {
                        model: st.rows
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            readonly property bool focused: !st.inNav && st.item === st.first + index
                            width: rowCol.width; height: 77 * st.k
                            color: "transparent"
                            radius: 12 * st.k
                            border.width: focused ? 2 * st.k : 0
                            border.color: st.couch.pal.accent
                            Rectangle { visible: index > 0; width: parent.width; height: 1; color: "#2a201b" }
                            Column {
                                x: 22 * st.k; anchors.verticalCenter: parent.verticalCenter
                                spacing: 3 * st.k
                                T { couchRef: st.couch; text: modelData[0]; font.pixelSize: 19 * st.k; font.weight: Font.Bold }
                                T { couchRef: st.couch; text: modelData[1]; color: "#b8aaa0"; font.pixelSize: 15 * st.k }
                            }
                            Rectangle {
                                anchors.right: parent.right; anchors.rightMargin: 22 * st.k
                                anchors.verticalCenter: parent.verticalCenter
                                height: 40 * st.k; radius: 10 * st.k
                                width: val.implicitWidth + 32 * st.k
                                color: "#2a201b"
                                T { id: val; couchRef: st.couch; anchors.centerIn: parent; text: modelData[2]; font.pixelSize: 17 * st.k; font.weight: Font.Bold }
                            }
                            MouseArea { anchors.fill: parent; onClicked: { st.inNav = false; st.item = st.first + index; st.press(st.first + index); } }
                        }
                    }
                }
            }
        }
    }
}
