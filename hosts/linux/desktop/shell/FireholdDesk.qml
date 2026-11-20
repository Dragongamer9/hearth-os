// Firehold's Desk on the main screen (mockup docs/mockups/firehold Desk, 1920 × 1080): the slim top bar (the app in
// front, the date and time in the middle, the "Desk" posture pill, Wi-Fi and battery) and the dock (Home, the apps
// and games that are open with their dots, Library, Settings). While it's up Mantel's own bar and dock step aside
// (SettingsNav.fireholdDesk, Islands.qml / Dock.qml); they come back the moment this goes (Couch, locked, or a
// problem in this file: Firehold.qml loads it by file name).
import QtQuick
import QtQuick.Window
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.UPower
import "firehold.js" as F

Scope {
    id: dk
    property var fh
    readonly property var scr: fh.deskScreen || (Quickshell.screens.length ? Quickshell.screens[0] : null)
    readonly property real k: scr ? Math.min(scr.width / 1920, scr.height / 1080) : 1
    readonly property string font: F.font(Theme.font)
    Component.onCompleted: SettingsNav.fireholdDesk = true
    Component.onDestruction: SettingsNav.fireholdDesk = false

    readonly property var front: WinList.activeToplevel
    readonly property string frontName: {
        if (!front) return "Firehold";
        if (F.ownWindow(front)) return F.windowName(front, "").name;     // Hearth's own windows: Hearth Files, Settings...
        const g = F.gameFor(front, fh.library);
        if (g) return g.title;
        const e = AppIcons.entryFor(front.appId);
        return e ? e.name : (F.windowName(front, "").name || "Firehold");
    }
    SystemClock { id: clock; precision: SystemClock.Minutes }
    readonly property var batt: UPower.displayDevice
    readonly property string status: [net, batt && batt.isLaptopBattery ? Math.round(batt.percentage * 100) + "%" : ""].filter(Boolean).join(" · ")
    // 1.16: from NetBtWatch.qml, when the network changes (no polling).
    readonly property string net: NetBtWatch.netKind === "wifi" ? "Wi-Fi" : NetBtWatch.netKind === "ethernet" ? "Wired" : "Offline"

    component T: Text { color: F.C.text; font.family: F.font(Theme.font); textFormat: Text.PlainText; elide: Text.ElideRight }

    // ------------------------------------------------------------------ the top bar
    // The app's menus: File, Edit, View, with what every app understands, sent to the window in front (its own
    // shortcuts, typed by hearth-type; Close and Full screen through the window list). The bar never takes the keyboard,
    // so the keys go to that window.
    readonly property var menus: ({
        "File": [["New window", ["-M", "ctrl", "-k", "n", "-m", "ctrl"]], ["Close window", "close"], ["Quit", ["-M", "ctrl", "-k", "q", "-m", "ctrl"]]],
        "Edit": [["Undo", ["-M", "ctrl", "-k", "z", "-m", "ctrl"]], ["Redo", ["-M", "ctrl", "-M", "shift", "-k", "z", "-m", "shift", "-m", "ctrl"]],
                 ["Cut", ["-M", "ctrl", "-k", "x", "-m", "ctrl"]], ["Copy", ["-M", "ctrl", "-k", "c", "-m", "ctrl"]],
                 ["Paste", ["-M", "ctrl", "-k", "v", "-m", "ctrl"]], ["Select all", ["-M", "ctrl", "-k", "a", "-m", "ctrl"]]],
        "View": [["Full screen", "full"], ["Zoom in", ["-M", "ctrl", "-k", "plus", "-m", "ctrl"]], ["Zoom out", ["-M", "ctrl", "-k", "minus", "-m", "ctrl"]],
                 ["Actual size", ["-M", "ctrl", "-k", "0", "-m", "ctrl"]]]
    })
    function pick(item) {
        const w = dk.front, what = item[1];
        dk.fh.deskMenu = "";
        if (!w) return;
        if (what === "close") w.close();
        else if (what === "full") w.fullscreen = !w.fullscreen;
        else Quickshell.execDetached(["python3", Quickshell.shellDir + "/hearth-type"].concat(what));
    }
    PanelWindow {
        id: bar
        screen: dk.scr
        WlrLayershell.layer: WlrLayer.Top
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        WlrLayershell.namespace: "hearth-firehold-desk-bar"
        anchors { top: true; left: true; right: true }
        implicitHeight: 44 * dk.k
        exclusiveZone: 44 * dk.k
        color: Qt.rgba(28 / 255, 21 / 255, 18 / 255, 0.9)
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: F.C.line }
        Row {
            x: 16 * dk.k; anchors.verticalCenter: parent.verticalCenter
            spacing: 16 * dk.k
            Rectangle {
                width: 32 * dk.k; height: width; radius: 9 * dk.k; color: F.C.accent
                anchors.verticalCenter: parent.verticalCenter
                TapHandler { onTapped: dk.fh.couchHome() }
            }
            T { anchors.verticalCenter: parent.verticalCenter; text: dk.frontName; font.pixelSize: 15 * dk.k; font.weight: Font.Bold; width: Math.min(implicitWidth, 520 * dk.k) }
            Repeater {
                model: ["File", "Edit", "View"]
                delegate: T {
                    id: mt
                    required property string modelData
                    anchors.verticalCenter: parent ? parent.verticalCenter : undefined
                    text: modelData; font.pixelSize: 14 * dk.k
                    color: dk.fh.deskMenu === modelData ? F.C.text : F.C.muted
                    TapHandler {
                        onTapped: dk.fh.deskMenu = dk.fh.deskMenu === mt.modelData ? "" : mt.modelData
                    }
                    Component.onCompleted: { const m = Object.assign({}, dk.menuItems); m[modelData] = mt; dk.menuItems = m; }
                }
            }
        }
        T {
            anchors.centerIn: parent
            text: Qt.formatDate(clock.date, "ddd MMM d") + " · " + Qt.formatTime(clock.date, "h:mm AP")
            font.pixelSize: 15 * dk.k; font.weight: Font.DemiBold
        }
        Row {
            anchors { right: parent.right; rightMargin: 16 * dk.k; verticalCenter: parent.verticalCenter }
            spacing: 14 * dk.k
            // The mode pill (mockup firehold-plasma 04 on labwc: "Desk · TV: Couch", the Modes card under it); without
            // the live modes (KWin, Firehold's compositor) Couch and Desk settings as before.
            Rectangle {
                id: pill
                readonly property var lm: dk.fh.sharedCouch ? dk.fh.modes : null
                anchors.verticalCenter: parent.verticalCenter
                width: pillRow.implicitWidth + 26 * dk.k; height: 30 * dk.k; radius: 15 * dk.k
                color: pill.lm && pill.lm.popupOpen ? "#38ff7a2f" : F.C.card; border.width: 1; border.color: F.C.line
                Row {
                    id: pillRow
                    anchors.centerIn: parent
                    spacing: 8 * dk.k
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 9 * dk.k; height: width; radius: width / 2; color: pill.lm && pill.lm.me.mode === "couch" ? F.C.accent : F.C.blue }
                    T { id: deskT; anchors.verticalCenter: parent.verticalCenter; text: pill.lm ? pill.lm.modeName(pill.lm.me.mode) : "Desk"; font.pixelSize: 14 * dk.k; font.weight: Font.DemiBold }
                    T { anchors.verticalCenter: parent.verticalCenter; visible: !!pill.lm && !!pill.lm.tv; text: pill.lm && pill.lm.tv ? "· TV: " + pill.lm.modeName(pill.lm.tv.mode) : ""; color: F.C.muted; font.pixelSize: 14 * dk.k; font.weight: Font.Medium }
                }
                TapHandler { onTapped: { if (pill.lm) pill.lm.popupOpen = !pill.lm.popupOpen; else dk.fh.settingsShown = true; } }
            }
            T {
                anchors.verticalCenter: parent.verticalCenter; text: dk.status; color: F.C.muted; font.pixelSize: 14 * dk.k; visible: text !== ""
                TapHandler { onTapped: dk.fh.toggleQuick() }
            }
        }
    }

    // The Modes card (the Couch's own ModesCard.qml, as under Plasma) under the pill, at the bar's right.
    PopupWindow {
        id: modesPop
        anchor.window: bar
        anchor.rect.x: bar.width - implicitWidth - 140 * dk.k
        anchor.rect.y: 44 * dk.k + 10 * dk.k
        visible: dk.fh.sharedCouch && dk.fh.modes.popupOpen && modesCard.status === Loader.Ready
        implicitWidth: modesCard.item ? modesCard.item.width : 800
        implicitHeight: modesCard.item ? modesCard.item.height : 600
        color: "transparent"
        Loader {
            id: modesCard
            active: dk.fh.sharedCouch && dk.fh.modes.popupOpen
            Component.onCompleted: setSource("fhcouch/ModesCard.qml", { bar: dk.fh.modes })
        }
    }
    property var menuItems: ({})
    // The open menu, under its name.
    PopupWindow {
        anchor.window: bar
        anchor.rect.x: dk.menuItems[dk.fh.deskMenu] ? dk.menuItems[dk.fh.deskMenu].mapToItem(null, 0, 0).x - 12 * dk.k : 0
        anchor.rect.y: 44 * dk.k
        visible: dk.fh.deskMenu !== "" && !!dk.menus[dk.fh.deskMenu]
        implicitWidth: 240 * dk.k
        implicitHeight: menuCol.implicitHeight + 16 * dk.k
        color: "transparent"
        Rectangle {
            anchors.fill: parent; radius: 12 * dk.k; color: F.C.card; border.width: 1; border.color: F.C.line
            Column {
                id: menuCol
                x: 8 * dk.k; y: 8 * dk.k; width: parent.width - 16 * dk.k
                Repeater {
                    model: dk.menus[dk.fh.deskMenu] || []
                    delegate: Rectangle {
                        id: mi
                        required property var modelData
                        width: parent.width; height: 34 * dk.k; radius: 8 * dk.k
                        color: hov.hovered ? F.C.card2 : "transparent"
                        T { x: 12 * dk.k; anchors.verticalCenter: parent.verticalCenter; text: mi.modelData[0]; font.pixelSize: 14 * dk.k }
                        HoverHandler { id: hov }
                        TapHandler { onTapped: dk.pick(mi.modelData) }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ the dock
    // Home, then one button for each app or game that's open (a game: ▶ on ember), then Library and Settings.
    readonly property var running: {
        const out = [], seen = {};
        for (const o of fh.openList) {
            const key = o.kind === "game" ? "game:" + o.name : String(o.win.appId || o.name);
            if (seen[key]) continue;
            seen[key] = true;
            out.push({ key: key, game: o.kind === "game", name: o.name, win: o.win,
                       icon: o.kind === "game" ? "" : AppIcons.find("", o.win.appId || "") });
        }
        return out.slice(0, 10);
    }
    PanelWindow {
        screen: dk.scr
        WlrLayershell.layer: WlrLayer.Top
        WlrLayershell.namespace: "hearth-firehold-desk-dock"
        anchors { bottom: true }
        margins { bottom: 22 * dk.k }
        exclusiveZone: dockBox.height + 22 * dk.k
        implicitWidth: dockBox.width
        implicitHeight: dockBox.height
        color: "transparent"
        Rectangle {
            id: dockBox
            width: dockRow.implicitWidth + 28 * dk.k; height: dockRow.implicitHeight + 20 * dk.k; radius: 22 * dk.k
            color: Qt.rgba(28 / 255, 21 / 255, 18 / 255, 0.92); border.width: 1; border.color: F.C.line
            Row {
                id: dockRow
                anchors.centerIn: parent
                spacing: 10 * dk.k
                DockButton { kk: dk.k; glyph: "H"; tint: F.C.accent; label: "Home"; onPicked: dk.fh.couchHome() }
                Repeater {
                    model: dk.running
                    delegate: DockButton {
                        required property var modelData
                        kk: dk.k
                        glyph: modelData.game ? "▶" : modelData.name.charAt(0).toUpperCase()
                        icon: modelData.icon
                        tint: modelData.game ? "#5b2b1a" : F.C.card2
                        label: modelData.name
                        open: true
                        onPicked: { if (modelData.win.minimized) modelData.win.minimized = false; modelData.win.activate(); }
                    }
                }
                DockButton { kk: dk.k; glyph: "L"; tint: F.C.card2; label: "Library"; onPicked: dk.fh.couchTab("library") }
                DockButton { kk: dk.k; glyph: "⚙"; tint: F.C.card2; label: "Settings"; onPicked: { if (dk.fh.settingsApp) dk.fh.settingsApp.open(""); } }
            }
        }
    }
    component DockButton: Column {
        id: db
        property string glyph: ""
        property string icon: ""
        property color tint: F.C.card2
        property string label: ""
        property bool open: false
        signal picked()
        property real kk: 1
        spacing: 5 * kk
        Rectangle {
            width: 56 * db.kk; height: width; radius: 15 * db.kk; color: db.tint
            Image {
                id: ic; anchors.centerIn: parent; width: 34 * db.kk; height: width; source: db.icon; visible: status === Image.Ready; asynchronous: true
                // Drawn at the screen's own pixels (the Ally at 2.1: never a small icon stretched).
                sourceSize.width: Math.ceil(width * Screen.devicePixelRatio); sourceSize.height: Math.ceil(height * Screen.devicePixelRatio); smooth: true; mipmap: true
            }
            T { anchors.centerIn: parent; visible: !ic.visible; text: db.glyph; font.pixelSize: 18 * db.kk; font.weight: Font.Bold }
            TapHandler { onTapped: db.picked() }
            Accessible.role: Accessible.Button
            Accessible.name: db.label
        }
        Rectangle { anchors.horizontalCenter: parent.horizontalCenter; width: 6 * db.kk; height: width; radius: width / 2; color: db.open ? F.C.accent : "transparent" }
    }
}
