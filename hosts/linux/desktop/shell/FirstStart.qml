// Build anything board 7: the first start after a fresh install, full screen, once. "How do you want your desktop?":
// Hearth OS Mantel (the bar, the dock, Quick Access, Game Mode; ready now) or Build your own desktop (the ready-made ones,
// or start from nothing). Either way it's Hearth underneath, and Settings › Your desktop changes it any time.
// Shown by itself only on a new install (desktop.json made for one, Settings.defaultsGen 2) that hasn't chosen yet;
// `quickshell ipc call screen firstStart` (and Settings › Your desktop › Show the first-start choice) shows it any time.
// Mouse, keyboard, touch or controller: ◀ ▶ (or Tab) move, A / Enter choose. Closing it without choosing keeps Hearth OS
// Mantel and doesn't ask again.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: fs
    property bool shown: false
    property var settingsApp: null
    property var editor: null
    // 0 = Hearth OS Mantel; 1-6 = a ready-made desktop; 7 = start from nothing
    property int sel: 0
    readonly property var tiles: ["console", "iconrow", "cockpit", "cabin", "retro"].map(id => ScreenLayout.readyMade.find(r => r.id === id))
    visible: shown
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-first-start"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "#140d09"

    function open() { sel = 0; shown = true; }
    // a choice made anywhere else (Settings, the screen's IPC) closes it
    Connections { target: ScreenLayout; function onChosenChanged() { if (ScreenLayout.chosen) fs.shown = false; } }
    function choose(i) {
        if (i === 0) ScreenLayout.useMantel();
        else if (i <= tiles.length) ScreenLayout.use(tiles[i - 1].id);
        else { ScreenLayout.markChosen(); if (editor) editor.open({ grid: { cols: 24, rows: 14 }, background: { kind: "gradient", from: "#1a1410", to: "#0c0907" }, look: {}, blocks: [] }, "Start from nothing"); }
        shown = false;
    }
    function later() { ScreenLayout.markChosen(); shown = false; }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient { GradientStop { position: 0; color: "#120c08" } GradientStop { position: 1; color: "#2a1508" } }
    }
    Item {
        anchors.fill: parent
        focus: fs.shown
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Left || e.key === Qt.Key_Backtab) { fs.sel = Math.max(0, fs.sel - 1); e.accepted = true; }
            else if (e.key === Qt.Key_Right || e.key === Qt.Key_Tab) { fs.sel = Math.min(fs.tiles.length + 1, fs.sel + 1); e.accepted = true; }
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) { fs.choose(fs.sel); e.accepted = true; }
            else if (e.key === Qt.Key_Escape) { fs.later(); e.accepted = true; }
        }
    }

    Column {
        anchors { top: parent.top; topMargin: parent.height * 0.1; horizontalCenter: parent.horizontalCenter }
        spacing: 12
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "WELCOME TO HEARTH OS"; color: "#ff7a2f"; font.family: Theme.font; font.pixelSize: 16; font.bold: true; font.letterSpacing: 2 }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "How do you want your desktop?"; color: "white"; font.family: Theme.font; font.pixelSize: 54; font.bold: true }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Either way it's all Hearth underneath, and you can change your mind any time in Settings › Your desktop."; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 19 }
    }

    Row {
        id: cards
        anchors.centerIn: parent
        anchors.verticalCenterOffset: parent.height * 0.08
        spacing: 40
        readonly property real cw: Math.min(780, (fs.width - 260) / 2)

        // ---- Hearth OS Mantel
        Rectangle {
            width: cards.cw; height: cw * 0.7
            readonly property real cw: cards.cw
            radius: 22
            color: "#221a16"
            border.width: fs.sel === 0 ? 3 : 1
            border.color: fs.sel === 0 ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.1)
            clip: true
            Rectangle {
                id: mpic
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 3 }
                height: parent.height * 0.5; radius: 20
                gradient: Gradient { GradientStop { position: 0; color: "#2a1a12" } GradientStop { position: 1; color: "#6a3412" } }
                Rectangle {
                    anchors { top: parent.top; left: parent.left; right: parent.right; margins: 18 }
                    height: 32; radius: 10; color: Qt.rgba(0, 0, 0, 0.45)
                    Text { anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter } text: "Files  ·  Games"; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 13 }
                    Rectangle {
                        anchors { right: parent.right; rightMargin: 60; verticalCenter: parent.verticalCenter }
                        width: 88; height: 22; radius: 11; color: "#ff7a2f"
                        Text { anchors.centerIn: parent; text: "Game Mode"; color: "#1b1006"; font.family: Theme.font; font.pixelSize: 12; font.bold: true }
                    }
                    Text { anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter } text: Qt.formatTime(new Date(), Settings.clock24 ? "HH:mm" : "h:mm"); color: "white"; font.family: Theme.font; font.pixelSize: 13 }
                }
                Rectangle { anchors.centerIn: parent; width: parent.width * 0.5; height: parent.height * 0.42; radius: 8; color: "#241c18"; border.color: Qt.rgba(1, 1, 1, 0.08) }
                Row {
                    anchors { bottom: parent.bottom; bottomMargin: 14; horizontalCenter: parent.horizontalCenter }
                    spacing: 10
                    Repeater { model: ["#ff8a3d", "#5b8cff", "#5fd18a", "#f2c14e", "#b07cff"]; delegate: Rectangle { required property var modelData; width: 34; height: 34; radius: 8; color: modelData } }
                }
            }
            Column {
                anchors { top: mpic.bottom; left: parent.left; right: parent.right; margins: 34 }
                spacing: 14
                Row {
                    spacing: 14
                    Text { text: "Hearth OS Mantel"; color: "white"; font.family: Theme.font; font.pixelSize: 30; font.bold: true }
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 92; height: 26; radius: 13; color: "transparent"; border.color: "#2f6b45"; Text { anchors.centerIn: parent; text: "Ready now"; color: "#7fe0a3"; font.family: Theme.font; font.pixelSize: 13 } }
                }
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Hearth's own desktop, set up and ready: the bar, the dock, Quick Access and Game Mode one press away. Pick a look later if you like."; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 16 }
                Rectangle {
                    width: 250; height: 48; radius: 6; color: "#ff7a2f"
                    Text { anchors.centerIn: parent; text: "Use Hearth OS Mantel"; color: "#1b1006"; font.family: Theme.font; font.pixelSize: 17; font.bold: true }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: fs.choose(0) }
                }
            }
            MouseArea { anchors.fill: parent; z: -1; onClicked: fs.sel = 0 }
        }

        // ---- Build your own desktop
        Rectangle {
            width: cards.cw; height: cw * 0.7
            readonly property real cw: cards.cw
            radius: 22
            color: "#1d1a19"
            border.width: fs.sel > 0 ? 2 : 1
            border.color: fs.sel > 0 ? Qt.rgba(1, 1, 1, 0.35) : Qt.rgba(1, 1, 1, 0.1)
            clip: true
            Grid {
                id: gal
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
                columns: 3; spacing: 8
                readonly property real tw: (width - 16) / 3
                Repeater {
                    model: fs.tiles.length + 1
                    delegate: Rectangle {
                        required property int index
                        readonly property bool nothing: index === fs.tiles.length
                        readonly property var tl: nothing ? null : fs.tiles[index]
                        width: gal.tw; height: index < 3 ? gal.tw * 0.47 : gal.tw * 0.63
                        radius: 6
                        gradient: nothing ? null : g
                        color: nothing ? "transparent" : "transparent"
                        border.width: fs.sel === index + 1 ? 3 : (nothing ? 2 : 0)
                        border.color: fs.sel === index + 1 ? "white" : "#ff7a2f"
                        Gradient { id: g; GradientStop { position: 0; color: tl ? tl.from : "black" } GradientStop { position: 1; color: tl ? tl.to : "black" } }
                        Text { visible: !nothing; anchors { left: parent.left; bottom: parent.bottom; margins: 12 } text: tl ? tl.name : ""; color: tl && tl.id === "notebook" ? "#2b2b2b" : "white"; font.family: Theme.font; font.pixelSize: 13; font.bold: true }
                        Text { visible: nothing; anchors.centerIn: parent; text: "+\nStart from nothing"; horizontalAlignment: Text.AlignHCenter; color: "#ff7a2f"; font.family: Theme.font; font.pixelSize: 15 }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { fs.sel = index + 1; } onDoubleClicked: fs.choose(index + 1) }
                    }
                }
            }
            Column {
                anchors { top: gal.bottom; left: parent.left; right: parent.right; margins: 34 }
                spacing: 14
                Text { text: "Build your own desktop"; color: "white"; font.family: Theme.font; font.pixelSize: 30; font.bold: true }
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Say it, sketch it, or start from one of these and change anything: where things go, how they look, sound and move. If Hearth hasn't got a part, your AI makes it."; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 16 }
                Row {
                    spacing: 12
                    Rectangle {
                        width: 170; height: 48; radius: 6; color: Qt.rgba(1, 1, 1, 0.06); border.color: Qt.rgba(1, 1, 1, 0.18)
                        Text { anchors.centerIn: parent; text: fs.sel > 0 && fs.sel <= fs.tiles.length ? "Use this one" : "Build my own"; color: "white"; font.family: Theme.font; font.pixelSize: 17 }
                        MouseArea {
                            anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (fs.sel > 0 && fs.sel <= fs.tiles.length) { fs.choose(fs.sel); return; }
                                ScreenLayout.markChosen(); fs.shown = false;
                                if (fs.editor) fs.editor.openReady(fs.tiles[0].id);
                            }
                        }
                    }
                    Rectangle {
                        width: 160; height: 48; radius: 6; color: Qt.rgba(1, 1, 1, 0.06); border.color: Qt.rgba(1, 1, 1, 0.18)
                        Row { anchors.centerIn: parent; spacing: 8; Icon { name: "mic"; size: 18; color: "white"; anchors.verticalCenter: parent.verticalCenter } Text { text: "Describe it"; color: "white"; font.family: Theme.font; font.pixelSize: 17 } }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { ScreenLayout.markChosen(); fs.shown = false; if (fs.editor) fs.editor.describeNew(); } }
                    }
                }
            }
        }
    }

    Row {
        anchors { bottom: parent.bottom; bottomMargin: 50; horizontalCenter: parent.horizontalCenter }
        spacing: 30
        Row { spacing: 8; Rectangle { width: 26; height: 26; radius: 13; color: "#3fae62"; Text { anchors.centerIn: parent; text: "A"; color: "white"; font.bold: true; font.pixelSize: 13 } } Text { text: "Choose"; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 16; anchors.verticalCenter: parent.verticalCenter } }
        Row { spacing: 8; Rectangle { width: 40; height: 26; radius: 13; color: "#d8d8d8"; Text { anchors.centerIn: parent; text: "◀ ▶"; color: "#222"; font.pixelSize: 11 } } Text { text: "Move"; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 16; anchors.verticalCenter: parent.verticalCenter } }
        Text { text: "Mouse, keyboard, touch or controller"; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 16 }
        Text { text: "Not now"; color: "#8a8079"; font.family: Theme.font; font.pixelSize: 16; MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: fs.later() } }
    }
}
