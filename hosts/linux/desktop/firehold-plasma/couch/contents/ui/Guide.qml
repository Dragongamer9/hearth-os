// The Guide (mockup 3): over the game, on the right. The game's art, name and how long you've played; Screenshot,
// Record, Friends, Keyboard, Volume, Brightness; the game rows; and Put it in a window, Home, Quit game.
// On KDE's own KWin some rows are KWin's own settings (VRR, tearing), the overlay is KWin's Show FPS effect, and
// Frame limit and Resolution scaling say plainly that Plasma doesn't have them.
import QtQuick
import org.kde.kirigami as Kirigami
import "firehold.js" as F

Rectangle {
    id: gd
    property QtObject couch
    readonly property real k: couch ? couch.k : 1
    readonly property var win: couch ? couch.guideWin : null
    readonly property var game: win && couch ? F.gameFor(win, couch.library) : null
    readonly property var sys: couch ? couch.sys : ({})
    property int sel: 0
    color: "#1c1411"
    focus: true

    readonly property string powerName: ({ "power-saver": "Power saver", "balanced": "Balanced", "performance": "Performance" })[sys.power] || "—"
    readonly property var tiles: [
        ["Screenshot", "Saved to Pictures"], ["Record", "The screen"], ["Friends", "Steam"],
        ["Keyboard", "On-screen"], ["Volume", sys.volume >= 0 ? sys.volume + "%" : "—"], ["Brightness", sys.brightness >= 0 ? sys.brightness + "%" : "—"]]
    readonly property var rows: [
        ["Frame limit", "Steadier, cooler, longer battery", "Not on Plasma", false],
        ["Resolution scaling", "Firehold upscales the game itself", "Not on Plasma", false],
        ["VRR / FreeSync", "Smooth when it dips below 60", sys.vrr || "—", sys.vrr === "On"],
        ["Allow tearing", "Lowest delay for fast games", sys.tearing === false ? "Off" : "On", false],
        ["Power", "Handhelds and laptops", powerName + (sys.watts >= 0 ? " · " + sys.watts + " W" : ""), false],
        ["Performance overlay", "Top-left corner", sys.overlay === "basic" || sys.overlay === "full" ? "Basic" : "Off", false]]
    readonly property var buttons: [["Put it in a window on the Desk", false], ["Home (the game keeps running)", false], ["Quit game", true]]

    function act(i, dir) {
        const c = couch;
        if (!c) return;
        const after = function () { c.refreshSys(true); };
        if (i < 6) {
            if (i === 0) { c.run(["do", "screenshot"]); c.guideShown = false; }
            else if (i === 1) c.run(["do", "record"]);
            else if (i === 2) { c.run(["do", "friends"]); c.guideShown = false; }
            else if (i === 3) c.run(["do", "osk"]);
            else if (i === 4) c.run(["do", dir < 0 ? "volume-down" : "volume-up"], after);
            else if (i === 5) c.run(["do", dir < 0 ? "brightness-down" : "brightness-up"], after);
        } else if (i < 12) {
            const r = i - 6;
            if (r === 2) c.run(["do", "vrr"], after);
            else if (r === 3) c.run(["do", "tearing"], after);
            else if (r === 4) c.run(["do", "power", dir < 0 ? "down" : "up"], after);
            else if (r === 5) c.run(["do", "overlay"], after);
        } else {
            const b = i - 12;
            if (b === 0) {
                if (win && win.fullscreen) c.toggleFull(win);
                c.guideShown = false;
                c.run(["switch", "desk", "--why", "guide"]);
            } else if (b === 1) c.home("home");
            else if (b === 2) { if (win) c.closeWin(win); c.guideShown = false; }
        }
    }
    Keys.onPressed: function (e) {
        const kk = e.key;
        e.accepted = true;
        if (kk === Qt.Key_Escape || kk === Qt.Key_F1 || kk === Qt.Key_Backspace) couch.guideShown = false;
        else if (kk === Qt.Key_Up) sel = sel < 3 ? sel : sel < 6 ? sel - 3 : sel === 6 ? 3 : sel - 1;
        else if (kk === Qt.Key_Down) sel = sel < 3 ? sel + 3 : sel < 6 ? 6 : Math.min(14, sel + 1);
        else if (kk === Qt.Key_Left) { if (sel === 4 || sel === 5) act(sel, -1); else if (sel < 6) { if (sel % 3 > 0) sel -= 1; } else if (sel < 12) act(sel, -1); }
        else if (kk === Qt.Key_Right) { if (sel === 4 || sel === 5) act(sel, 1); else if (sel < 6) { if (sel % 3 < 2) sel += 1; } else if (sel < 12) act(sel, 1); }
        else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) act(sel, 1);
        else e.accepted = false;
    }

    Rectangle { width: 1; height: parent.height; color: "#2e231e" }

    // ---------------------------------------------------------------- the game
    Rectangle {
        id: art
        x: 33 * gd.k; y: 32 * gd.k
        width: 56 * gd.k; height: width; radius: 10 * gd.k
        clip: true
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#5b2b1a" }
            GradientStop { position: 1.0; color: "#a3541e" }
        }
        readonly property string pic: gd.game ? (gd.game.icon || gd.game.cover || "") : ""
        Image {
            anchors.fill: parent
            visible: parent.pic !== ""
            source: parent.pic === "" ? "" : (parent.pic.indexOf("/") === 0 ? "file://" + parent.pic : parent.pic)
            fillMode: Image.PreserveAspectCrop; asynchronous: true
            sourceSize.width: 112; sourceSize.height: 112
        }
    }
    Column {
        anchors.left: art.right; anchors.leftMargin: 16 * gd.k
        anchors.verticalCenter: art.verticalCenter
        spacing: 2 * gd.k
        T { couchRef: gd.couch; text: gd.game ? gd.game.title : gd.win ? gd.win.title : "Quick Access"; font.pixelSize: 24 * gd.k; font.weight: Font.Bold; width: 420 * gd.k }
        T {
            couchRef: gd.couch
            width: 420 * gd.k
            text: gd.win ? "Playing " + F.duration(gd.couch.now.getTime() - (gd.couch.seen[gd.win.id] || gd.couch.now.getTime())) + " · native, no Game Mode" : "Nothing is playing"
            color: "#b8aaa0"; font.pixelSize: 16 * gd.k
        }
    }

    // ---------------------------------------------------------------- the tiles
    Grid {
        x: 33 * gd.k; y: 108 * gd.k
        columns: 3; spacing: 11 * gd.k; rowSpacing: 10 * gd.k
        Repeater {
            model: gd.tiles
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: 158 * gd.k; height: 78 * gd.k; radius: 12 * gd.k
                color: "#251c18"
                border.width: gd.sel === index ? 2 * gd.k : 1
                border.color: gd.sel === index ? gd.couch.pal.accent : "#33281f"
                Column {
                    anchors.centerIn: parent
                    spacing: 4 * gd.k
                    T { couchRef: gd.couch; anchors.horizontalCenter: parent.horizontalCenter; text: modelData[0]; font.pixelSize: 17 * gd.k; font.weight: Font.Bold }
                    T { couchRef: gd.couch; anchors.horizontalCenter: parent.horizontalCenter; text: modelData[1]; color: "#b8aaa0"; font.pixelSize: 13 * gd.k }
                }
                MouseArea { anchors.fill: parent; onClicked: { gd.sel = index; gd.act(index, 1); } }
            }
        }
    }

    // ---------------------------------------------------------------- the rows
    Rectangle {
        x: 33 * gd.k; y: 295 * gd.k
        width: 495 * gd.k; height: rowCol.height
        radius: 12 * gd.k
        color: "#1f1714"
        Column {
            id: rowCol
            width: parent.width
            Repeater {
                model: gd.rows
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: rowCol.width; height: 71 * gd.k
                    radius: 10 * gd.k
                    color: "transparent"
                    border.width: gd.sel === 6 + index ? 2 * gd.k : 0
                    border.color: gd.couch.pal.accent
                    Rectangle { visible: index > 0; x: 18 * gd.k; width: parent.width - 36 * gd.k; height: 1; color: "#2a201b" }
                    Column {
                        x: 18 * gd.k; anchors.verticalCenter: parent.verticalCenter
                        spacing: 2 * gd.k
                        T { couchRef: gd.couch; text: modelData[0]; font.pixelSize: 17 * gd.k; font.weight: Font.Bold }
                        T { couchRef: gd.couch; text: modelData[1]; color: "#b8aaa0"; font.pixelSize: 14 * gd.k }
                    }
                    Rectangle {
                        anchors.right: parent.right; anchors.rightMargin: 18 * gd.k
                        anchors.verticalCenter: parent.verticalCenter
                        height: 34 * gd.k; radius: 8 * gd.k
                        width: chip.implicitWidth + 28 * gd.k
                        color: "#2a201b"
                        T {
                            id: chip
                            couchRef: gd.couch
                            anchors.centerIn: parent
                            text: "‹ " + modelData[2] + " ›"
                            color: modelData[3] ? gd.couch.pal.green : gd.couch.pal.text
                            font.pixelSize: 15 * gd.k; font.weight: Font.Bold
                        }
                    }
                    MouseArea { anchors.fill: parent; onClicked: { gd.sel = 6 + index; gd.act(6 + index, 1); } }
                }
            }
        }
    }

    // ---------------------------------------------------------------- the buttons
    Column {
        x: 33 * gd.k
        anchors.bottom: parent.bottom; anchors.bottomMargin: 32 * gd.k
        spacing: 10 * gd.k
        Repeater {
            model: gd.buttons
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: 495 * gd.k; height: 54 * gd.k; radius: 12 * gd.k
                color: modelData[1] ? "transparent" : "#2e241f"
                border.width: modelData[1] ? 2 * gd.k : (gd.sel === 12 + index ? 2 * gd.k : 0)
                border.color: modelData[1] ? (gd.sel === 12 + index ? "#ffb4b4" : gd.couch.pal.red) : gd.couch.pal.accent
                T {
                    couchRef: gd.couch
                    anchors.centerIn: parent
                    text: modelData[0]
                    color: modelData[1] ? gd.couch.pal.red : gd.couch.pal.text
                    font.pixelSize: 18 * gd.k; font.weight: Font.Bold
                }
                MouseArea { anchors.fill: parent; onClicked: { gd.sel = 12 + index; gd.act(12 + index, 1); } }
            }
        }
    }
}
