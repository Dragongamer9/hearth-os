// Couch Home (mockup 1): Continue (the game you played last), Recent, and Open on this PC (the same windows you see
// on the Desk). Rows: 0 the hero's buttons, 1 Recent, 2 the open windows.
import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "firehold.js" as F

Item {
    id: home
    property QtObject couch
    readonly property real k: couch.k
    readonly property var game: couch.continueGame
    readonly property var running: game ? couch.windowFor(game) : null
    readonly property var recent: F.recent(couch.library, 7, game ? game.id : "")
    readonly property var windows: couch.openList.filter(function (o) { return o.kind === "window"; }).slice(0, 6)
    property int row: 0
    property int col: 0
    function count(r) { return r === 0 ? (game ? 2 : 1) : r === 1 ? recent.length : windows.length; }
    function key(kk) {
        if (kk === Qt.Key_Up || kk === Qt.Key_Down) {
            let r = row;
            do { r += kk === Qt.Key_Up ? -1 : 1; } while (r >= 0 && r <= 2 && count(r) === 0);
            if (r >= 0 && r <= 2) { row = r; col = Math.min(col, count(r) - 1); }
            return true;
        }
        if (kk === Qt.Key_Left) { col = Math.max(0, col - 1); return true; }
        if (kk === Qt.Key_Right) { col = Math.min(count(row) - 1, col + 1); return true; }
        if (kk === Qt.Key_Return || kk === Qt.Key_Enter) { activate(row, col); return true; }
        if (kk === Qt.Key_X) {
            const g = row === 0 ? game : row === 1 ? recent[col] : null;
            if (g) couch.run(["page", g.id]);
            return true;
        }
        if (kk === Qt.Key_Y) { couch.tab = "library"; couch.key(Qt.Key_Y); return true; }
        return false;
    }
    function activate(r, c) {
        if (r === 0) { if (game) couch.play(game, c === 1); else couch.tab = "library"; }
        else if (r === 1) couch.play(recent[c], false);
        else if (windows[c]) couch.goTo(windows[c].win, false);
    }
    function sel(r, c) { return row === r && col === c; }

    // ---------------------------------------------------------------- Continue
    Rectangle {
        id: hero
        x: 72 * home.k; y: 136 * home.k
        width: parent.width - 144 * home.k; height: 440 * home.k
        radius: 28 * home.k
        clip: true
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: F.GAME_GRADIENT[0] }
            GradientStop { position: 0.5; color: F.GAME_GRADIENT[1] }
            GradientStop { position: 1.0; color: F.GAME_GRADIENT[2] }
        }
        readonly property string art: home.game ? (home.game.hero || home.game.wide || "") : ""
        Image {
            id: heroArt
            visible: hero.art !== ""
            x: 1073 * home.k; y: 50 * home.k; width: 642 * home.k; height: 343 * home.k
            source: hero.art === "" ? "" : (hero.art.indexOf("/") === 0 ? "file://" + hero.art : hero.art)
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            sourceSize.width: 1284; sourceSize.height: 686
        }
        Shape {
            visible: hero.art === "" || heroArt.status !== Image.Ready
            x: 1073 * home.k; y: 50 * home.k; width: 642 * home.k; height: 343 * home.k
            ShapePath {
                strokeColor: "#80f6ede6"; strokeWidth: 2; fillColor: "transparent"
                strokeStyle: ShapePath.DashLine; dashPattern: [4, 3]
                startX: 16 * home.k; startY: 1
                PathLine { x: 642 * home.k - 16 * home.k; y: 1 }
                PathArc { x: 642 * home.k - 1; y: 16 * home.k; radiusX: 15 * home.k; radiusY: 15 * home.k }
                PathLine { x: 642 * home.k - 1; y: 343 * home.k - 16 * home.k }
                PathArc { x: 642 * home.k - 16 * home.k; y: 343 * home.k - 1; radiusX: 15 * home.k; radiusY: 15 * home.k }
                PathLine { x: 16 * home.k; y: 343 * home.k - 1 }
                PathArc { x: 1; y: 343 * home.k - 16 * home.k; radiusX: 15 * home.k; radiusY: 15 * home.k }
                PathLine { x: 1; y: 16 * home.k }
                PathArc { x: 16 * home.k; y: 1; radiusX: 15 * home.k; radiusY: 15 * home.k }
            }
            T { couchRef: home.couch; anchors.centerIn: parent; text: "Game art"; color: "#c0f6ede6"; font.pixelSize: 19 * home.k }
        }
        Column {
            x: 48 * home.k
            anchors.bottom: parent.bottom; anchors.bottomMargin: 48 * home.k
            spacing: 6 * home.k
            T {
                couchRef: home.couch
                text: home.game ? "CONTINUE · " + F.ago(home.game.lastPlayed || 0, home.couch.now.getTime()) : "WELCOME TO FIREHOLD"
                color: home.couch.pal.gold
                font.pixelSize: 19 * home.k; font.weight: Font.Bold; font.letterSpacing: 1.2 * home.k
            }
            T {
                couchRef: home.couch
                width: 960 * home.k
                text: home.game ? home.game.title : "Your games, all in one place"
                font.pixelSize: 64 * home.k; font.weight: Font.Bold
            }
            T {
                couchRef: home.couch
                width: 960 * home.k
                text: home.game ? [home.game.chapter || "", home.game.cloudSaves ? "Saved on this PC and in the cloud" : (home.game.savesBackedUp ? "Saved on this PC and backed up" : "Saved on this PC"),
                                   "runs natively, no Game Mode"].filter(Boolean).join(" · ") : "Steam, GOG, Epic, emulators and your own games"
                color: "#e6d9cf"
                font.pixelSize: 21 * home.k
            }
            Item { width: 1; height: 18 * home.k }
            Row {
                spacing: 14 * home.k
                Repeater {
                    model: home.game ? [home.running ? "Resume" : "Play", "Open in a window"] : ["Open the Library"]
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool primary: index === 0
                        height: 60 * home.k
                        width: btnText.implicitWidth + 74 * home.k
                        radius: 12 * home.k
                        color: primary ? home.couch.pal.accent : "#2a1a17"
                        border.width: home.sel(0, index) ? 3 * home.k : 0
                        border.color: primary ? "#ffd2a8" : home.couch.pal.accent
                        T {
                            id: btnText
                            couchRef: home.couch
                            anchors.centerIn: parent
                            text: modelData
                            color: parent.primary ? "#2a1206" : home.couch.pal.text
                            font.pixelSize: 22 * home.k; font.weight: Font.Bold
                        }
                        MouseArea { anchors.fill: parent; onClicked: { home.row = 0; home.col = index; home.activate(0, index); } }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- Recent
    T {
        couchRef: home.couch
        x: 72 * home.k; y: 616 * home.k
        text: "Recent"
        font.pixelSize: 23 * home.k; font.weight: Font.Bold
        visible: home.recent.length > 0
    }
    Row {
        x: 72 * home.k; y: 658 * home.k
        spacing: 20 * home.k
        Repeater {
            model: home.recent
            delegate: Item {
                required property var modelData
                required property int index
                width: 228 * home.k; height: 162 * home.k
                Rectangle {
                    id: tile
                    width: parent.width; height: 128 * home.k
                    radius: 12 * home.k
                    color: F.toneFor(modelData.id)
                    clip: true
                    border.width: home.sel(1, index) ? 3 * home.k : 0
                    border.color: home.couch.pal.accent
                    readonly property string art: modelData.wide || modelData.cover || ""
                    Image {
                        anchors.fill: parent; anchors.margins: parent.border.width
                        visible: parent.art !== ""
                        source: parent.art === "" ? "" : (parent.art.indexOf("/") === 0 ? "file://" + parent.art : parent.art)
                        fillMode: Image.PreserveAspectCrop; asynchronous: true
                        sourceSize.width: 456; sourceSize.height: 256
                    }
                    T {
                        couchRef: home.couch
                        x: 10 * home.k; anchors.bottom: parent.bottom; anchors.bottomMargin: 9 * home.k
                        text: modelData.store || ""
                        color: "#d8cbc1"; font.pixelSize: 14 * home.k
                    }
                }
                T {
                    couchRef: home.couch
                    anchors.top: tile.bottom; anchors.topMargin: 10 * home.k
                    width: parent.width
                    text: modelData.title
                    font.pixelSize: 19 * home.k; font.weight: Font.Medium
                }
                MouseArea { anchors.fill: parent; onClicked: { home.row = 1; home.col = index; home.activate(1, index); } }
            }
        }
    }

    // ---------------------------------------------------------------- Open on this PC
    Row {
        x: 72 * home.k; y: 852 * home.k
        spacing: 0
        visible: home.windows.length > 0
        T { id: openTitle; couchRef: home.couch; text: "Open on this PC"; font.pixelSize: 23 * home.k; font.weight: Font.Bold }
        T { couchRef: home.couch; y: openTitle.baselineOffset - baselineOffset; text: " · the same windows you see on the Desk"; color: home.couch.pal.dim; font.pixelSize: 19 * home.k }
    }
    Row {
        x: 72 * home.k; y: 894 * home.k
        spacing: 16 * home.k
        Repeater {
            model: home.windows
            delegate: Rectangle {
                required property var modelData
                required property int index
                height: 74 * home.k
                width: Math.max(160 * home.k, textCol.implicitWidth + 98 * home.k)
                radius: 12 * home.k
                color: "#1a1311"
                border.width: home.sel(2, index) ? 3 * home.k : 1
                border.color: home.sel(2, index) ? home.couch.pal.accent : home.couch.pal.line
                Rectangle {
                    id: ic
                    x: 22 * home.k; anchors.verticalCenter: parent.verticalCenter
                    width: 40 * home.k; height: width; radius: 10 * home.k
                    color: F.toneFor(modelData.name)
                    Kirigami.Icon {
                        anchors.fill: parent; anchors.margins: 6 * home.k
                        source: modelData.win.appId || "application-x-executable"
                        fallback: "application-x-executable"
                    }
                }
                Column {
                    id: textCol
                    anchors.left: ic.right; anchors.leftMargin: 14 * home.k
                    anchors.verticalCenter: parent.verticalCenter
                    T { couchRef: home.couch; text: modelData.name; font.pixelSize: 19 * home.k; font.weight: Font.Bold; width: Math.min(implicitWidth, 300 * home.k) }
                    T { couchRef: home.couch; text: modelData.sub || ""; color: home.couch.pal.dim; font.pixelSize: 15 * home.k; width: Math.min(implicitWidth, 300 * home.k); visible: text !== "" }
                }
                MouseArea { anchors.fill: parent; onClicked: home.couch.goTo(modelData.win, false) }
            }
        }
    }

    // ---------------------------------------------------------------- hints
    Row {
        anchors.right: parent.right; anchors.rightMargin: 72 * home.k
        y: 1032 * home.k
        spacing: 30 * home.k
        Hint { couch: home.couch; key: "A"; label: "Play"; tint: home.couch.pal.green }
        Hint { couch: home.couch; key: "X"; label: "Options"; tint: home.couch.pal.blue }
        Hint { couch: home.couch; key: "Y"; label: "Search"; tint: home.couch.pal.yellow }
        Hint { couch: home.couch; key: "≡"; label: "Guide: Quick Access"; tint: "#a89a90" }
        Hint { couch: home.couch; key: "LB RB"; label: "Tabs"; tint: "#a89a90" }
    }
}
