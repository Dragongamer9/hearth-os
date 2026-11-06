// A live tile for the Tiles and Start screen app menus (MenuTiles.qml, MenuStart.qml). Sizes: small (1×1),
// medium (2×2), wide (4×2), large (4×4) in `unit`s; yours are saved per user (Launcher.qml: Settings.menuTiles,
// only when you resize or move one). Live data is what Hearth already has (Launcher.qml's `live`, from hearth-appmenu tiles,
// and what's playing): a tile without data is a plain icon tile with its name.
import QtQuick
import QtQuick.Layouts
import Quickshell

Rectangle {
    id: t
    property var menu: null               // Launcher.qml
    property var tile: ({})
    property real unit: 72
    property real gap: 8
    property bool focused: false
    property bool lifted: false           // being moved (drag with the mouse, or Y / Move and the D-pad)
    property bool dropHere: false         // a dragged tile goes here when you let go
    signal picked()
    // Dragging with the mouse: the menu works out where it goes (Launcher.qml groupTiles / saveTileOrder).
    signal dragStarted()
    signal dragMoved(real sx, real sy)    // the pointer on screen
    signal dragEnded()
    z: lifted ? 2 : 0
    scale: lifted ? 1.04 : 1
    opacity: lifted ? 0.92 : 1
    Behavior on scale { enabled: !Settings.reduceMotion; NumberAnimation { duration: 90 } }
    readonly property string size: menu ? menu.tileSize(tile.id, tile.size || "medium") : (tile.size || "medium")
    readonly property int cols: size === "small" ? 1 : size === "medium" ? 2 : 4
    readonly property int rows: size === "small" ? 1 : size === "large" ? 4 : 2
    readonly property bool tiny: size === "small"
    readonly property var live: menu ? menu.live : ({})
    width: cols * unit + (cols - 1) * gap
    height: rows * unit + (rows - 1) * gap
    radius: Theme.r(4)
    clip: true

    function hashColor(s) {
        const pal = ["#d9702a", "#c99316", "#2f7fbf", "#6b4fc8", "#1f9a8a", "#c8414b", "#3f5fa8", "#2f9e57"];
        let h = 0;
        for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
        return pal[h % pal.length];
    }
    readonly property bool homeOk: !!live.home && live.home.ok
    color: tile.kind === "gamemode" ? Theme.accent
        : tile.kind === "weather" ? "#2f6db5" : tile.kind === "calendar" ? "#c8414b"
        : tile.kind === "home" ? (homeOk ? "#2f9e57" : "#c76a1d") : tile.kind === "updates" ? "#3e9b4f"
        : tile.kind === "nowplaying" ? "#6b4fc8" : tile.kind === "game" ? Theme.panel3 : hashColor(tile.id || "")
    readonly property color ink: tile.kind === "gamemode" ? Theme.accentInk : tile.kind === "game" ? "#ffffff" : "#ffffff"

    // Now playing (loaded on its own: MenuNowPlaying.qml)
    Loader { id: np; active: t.tile.kind === "nowplaying"; source: "MenuNowPlaying.qml" }
    readonly property bool music: np.status === Loader.Ready && np.item.has

    // ---- the picture: a game's cover / art, or the app's icon
    Image {
        id: cover
        anchors.fill: parent
        visible: t.tile.kind === "game" && status === Image.Ready
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        sourceSize.width: t.width * 1.5
        source: t.tile.kind === "game" && t.tile.game ? ((t.cols >= 4 && t.rows <= 2 ? t.tile.game.wide : t.tile.game.cover) || t.tile.game.cover || t.tile.game.wide || "") : ""
    }
    Rectangle {   // a shade under the title on pictures
        visible: cover.visible && !t.tiny
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: Math.min(parent.height, 70)
        gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.72) } }
    }
    Text {   // a game without a cover: its first letter
        visible: t.tile.kind === "game" && !cover.visible
        anchors.centerIn: parent
        text: (t.tile.title || "?").charAt(0).toUpperCase()
        color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.min(t.width, t.height) * 0.4
    }
    Rectangle {   // an app's icon on a dark plate, as the mockups
        visible: t.tile.kind === "app"
        anchors.centerIn: parent
        anchors.verticalCenterOffset: t.tiny ? 0 : -t.height * 0.06
        width: Math.min(t.width, t.height) * (t.tiny ? 0.62 : 0.42); height: width
        radius: Theme.r(width * 0.24)
        color: Qt.rgba(0, 0, 0, 0.28)
        AppIcon { anchors.centerIn: parent; size: parent.width * 0.78; icon: t.tile.entry ? (t.tile.entry.icon || "") : ""; appId: t.tile.entry ? (t.tile.entry.id || "") : "" }
    }

    // ---- live content
    ColumnLayout {
        id: body
        visible: !t.tiny && t.tile.kind !== "app" && t.tile.kind !== "game"
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
        spacing: 3
        // Now playing
        RowLayout {
            visible: t.tile.kind === "nowplaying" && t.music
            spacing: 12
            Rectangle {
                implicitWidth: Math.min(t.height - 60, 72); implicitHeight: implicitWidth; radius: Theme.r(8); clip: true
                gradient: Gradient { GradientStop { position: 0; color: Theme.flame } GradientStop { position: 1; color: Theme.accent } }
                Image { anchors.fill: parent; source: t.music ? np.item.art : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text { Layout.fillWidth: true; text: t.music ? np.item.title : ""; color: t.ink; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: t.music ? np.item.artist : ""; color: t.ink; opacity: 0.85; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                RowLayout {
                    spacing: 14
                    Text { text: t.music && np.item.playing ? "❚❚" : "▶"; color: t.ink; font.pixelSize: 13; MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: np.item.toggle() } }
                    Text { text: "▶❚"; color: t.ink; font.pixelSize: 11; MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: np.item.next() } }
                }
            }
        }
        // Weather (Hearth Weather's summary)
        Text {
            visible: t.tile.kind === "weather" && !!t.live.weather
            Layout.fillWidth: true
            text: t.live.weather ? (t.live.weather.line || "") : ""
            wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight
            color: t.ink; font.family: Theme.font; font.pixelSize: t.size === "medium" ? 17 : 22; font.weight: Font.DemiBold
        }
        Text {
            visible: t.tile.kind === "weather" && !!t.live.weather && !!t.live.weather.place
            text: t.live.weather ? (t.live.weather.place || "") : ""
            color: t.ink; opacity: 0.85; font.family: Theme.font; font.pixelSize: 12
        }
        // Calendar (Hearth Calendar's next events)
        Text {
            visible: t.tile.kind === "calendar" && !!t.live.calendar
            text: Qt.formatDate(new Date(), "dddd d")
            color: t.ink; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold
        }
        Repeater {
            model: t.tile.kind === "calendar" && t.live.events ? t.live.events : []
            delegate: Text {
                required property var modelData
                Layout.fillWidth: true
                text: (modelData.allDay ? "" : Qt.formatTime(new Date(modelData.start), Settings.clock24 ? "HH:mm" : "h AP") + " · ") + modelData.title
                elide: Text.ElideRight; color: t.ink; opacity: 0.9; font.family: Theme.font; font.pixelSize: 12
            }
        }
        Text {
            visible: t.tile.kind === "calendar" && !!t.live.calendar && (!t.live.events || t.live.events.length === 0)
            text: "Nothing planned"
            color: t.ink; opacity: 0.85; font.family: Theme.font; font.pixelSize: 12
        }
        // Home server (hearth-home-server quick)
        Text {
            visible: t.tile.kind === "home" && !!t.live.home
            text: t.homeOk ? "● All good" : "● Needs a look"
            color: t.ink; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold
        }
        Text {
            visible: t.tile.kind === "home" && !!t.live.home
            Layout.fillWidth: true
            wrapMode: Text.WordWrap; maximumLineCount: t.rows > 2 ? 6 : 2; elide: Text.ElideRight
            text: !t.live.home ? "" : t.homeOk
                ? t.live.home.shares + (t.live.home.shares === 1 ? " share" : " shares") + " · " + t.live.home.people + (t.live.home.people === 1 ? " person" : " people")
                  + (t.live.home.paused && t.live.home.paused.length ? "\nPaused for your game: " + t.live.home.paused.join(", ") : "")
                : t.live.home.problems.join("\n")
            color: t.ink; opacity: 0.9; font.family: Theme.font; font.pixelSize: 12
        }
        Rectangle {   // space used
            visible: t.tile.kind === "home" && !!t.live.home && t.live.home.total > 0 && t.rows > 1
            Layout.fillWidth: true; Layout.topMargin: 6
            implicitHeight: 5; radius: 2.5; color: Qt.rgba(1, 1, 1, 0.3)
            Rectangle { width: parent.width * (t.live.home && t.live.home.total ? Math.min(1, t.live.home.used / t.live.home.total) : 0); height: parent.height; radius: 2.5; color: "#ffffff" }
        }
        // Updates
        Text {
            visible: t.tile.kind === "updates" && !!t.live.updates
            text: t.live.updates ? t.live.updates.count + (t.live.updates.count === 1 ? " update" : " updates") : ""
            color: t.ink; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold
        }
        Text {
            visible: t.tile.kind === "updates" && !!t.live.updates
            Layout.fillWidth: true; elide: Text.ElideRight
            text: t.live.updates ? t.live.updates.titles.join(", ") : ""
            color: t.ink; opacity: 0.9; font.family: Theme.font; font.pixelSize: 12
        }
    }

    // ---- the plain icon (a tile with no data, Game Mode, a small tile)
    readonly property bool plain: t.tile.kind === "gamemode" || (t.tile.kind === "nowplaying" && !t.music) || (t.tile.kind === "weather" && !t.live.weather)
        || (t.tile.kind === "calendar" && !t.live.calendar) || (t.tile.kind === "home" && !t.live.home) || (t.tile.kind === "updates" && !t.live.updates)
        || (t.tiny && t.tile.kind !== "app" && t.tile.kind !== "game")
    Icon {
        visible: t.plain
        anchors.centerIn: parent
        anchors.verticalCenterOffset: t.tiny ? 0 : -8
        size: Math.min(t.width, t.height) * (t.tiny ? 0.42 : 0.3)
        color: t.tile.kind === "gamemode" ? Theme.accentInk : Theme.text
        name: ({ gamemode: "pad", nowplaying: "vol", weather: "sun", calendar: "calendar", home: "store", updates: "update" })[t.tile.kind] || "grid"
    }

    // ---- the name along the bottom
    Text {
        visible: !t.tiny
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 12; bottomMargin: 10 }
        text: t.tile.kind === "game" ? (t.tile.title || "") : t.tile.kind === "app" ? (t.tile.title || "") : t.tile.kind === "nowplaying" && t.music ? (np.item.app || "Now playing") : (t.tile.title || "")
        elide: Text.ElideRight
        color: t.ink; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
    }
    Text {
        visible: t.tile.kind === "gamemode" && t.cols >= 4
        anchors { right: parent.right; bottom: parent.bottom; margins: 12; bottomMargin: 10 }
        text: "Home button"; color: t.ink; opacity: 0.8; font.family: Theme.font; font.pixelSize: 12
    }

    // ---- focus ring (controller / keyboard) and the mouse
    Rectangle {
        anchors.fill: parent
        color: t.dropHere ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.3) : area.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"
        radius: parent.radius
        border.width: t.lifted || t.dropHere || t.focused ? 3 : 0
        border.color: t.lifted || t.dropHere ? Theme.accent : Theme.text
    }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: dragging ? Qt.ClosedHandCursor : Qt.PointingHandCursor
        // A real mouse drags the tile (the tiles' Flickable doesn't take the drag); a finger still scrolls the tiles
        // (press and hold for the menu, then Move).
        preventStealing: mousePress
        property bool mousePress: false
        property bool dragging: false
        property bool dragged: false      // this press moved a tile: no click after it
        property point down: Qt.point(0, 0)
        onPressed: mouse => { down = Qt.point(mouse.x, mouse.y); dragged = false; mousePress = mouse.button === Qt.LeftButton && mouse.source === Qt.MouseEventNotSynthesized; }
        onPositionChanged: mouse => {
            if (!mousePress || !(mouse.buttons & Qt.LeftButton)) return;
            if (!dragging && Math.hypot(mouse.x - down.x, mouse.y - down.y) > 14) { dragging = true; dragged = true; t.picked(); t.dragStarted(); }
            if (dragging) { const p = mapToItem(null, mouse.x, mouse.y); t.dragMoved(p.x, p.y); }
        }
        onReleased: { mousePress = false; if (dragging) { dragging = false; t.dragEnded(); } }
        onCanceled: { mousePress = false; if (dragging) { dragging = false; t.dragEnded(); } }
        onClicked: mouse => { if (dragged) { dragged = false; return; } t.picked(); if (mouse.button === Qt.RightButton) { if (t.menu) t.menu.tileMenu(t.tile, t); } else if (t.tile.open) t.tile.open(); }
        onPressAndHold: if (!dragging && t.menu) t.menu.tileMenu(t.tile, t)
    }
}
