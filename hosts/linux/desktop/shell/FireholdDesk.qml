// Firehold's Desk on the main screen (mockup docs/mockups/firehold Desk, 1920 × 1080): the slim top bar (the app in
// front, the date and time in the middle, the "Desk" posture pill, Wi-Fi and battery) and the dock (Home, the apps
// and games that are open with their dots, Library, Settings). While it's up Mantel's own bar and dock step aside
// (SettingsNav.fireholdDesk, Islands.qml / Dock.qml); they come back the moment this goes (Couch, locked, or a
// problem in this file: Firehold.qml loads it by file name).
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.UPower
import "firehold.js" as F

Scope {
    id: dk
    property var fh
    readonly property var scr: Quickshell.screens.length ? Quickshell.screens[0] : null
    readonly property real k: scr ? Math.min(scr.width / 1920, scr.height / 1080) : 1
    readonly property string font: F.font(Theme.font)
    Component.onCompleted: SettingsNav.fireholdDesk = true
    Component.onDestruction: SettingsNav.fireholdDesk = false

    readonly property var front: WinList.activeToplevel
    readonly property string frontName: {
        if (!front) return "Firehold";
        const g = F.gameFor(front, fh.library);
        if (g) return g.title;
        const e = AppIcons.entryFor(front.appId);
        return e ? e.name : (F.windowName(front, "").name || "Firehold");
    }
    SystemClock { id: clock; precision: SystemClock.Minutes }
    readonly property var batt: UPower.displayDevice
    readonly property string status: [net, batt && batt.isLaptopBattery ? Math.round(batt.percentage * 100) + "%" : ""].filter(Boolean).join(" · ")
    property string net: ""
    Timer { interval: 15000; running: true; repeat: true; triggeredOnStart: true; onTriggered: netProc.running = true }
    Process {
        id: netProc
        command: ["nmcli", "-t", "-f", "TYPE,STATE", "device"]
        stdout: StdioCollector {
            onStreamFinished: {
                let kd = "";
                for (const l of text.split("\n")) { const f = l.split(":"); if (f[1] === "connected" && (f[0] === "wifi" || f[0] === "ethernet")) { kd = f[0]; break; } }
                dk.net = kd === "wifi" ? "Wi-Fi" : kd === "ethernet" ? "Wired" : "Offline";
            }
        }
    }

    component T: Text { color: F.C.text; font.family: F.font(Theme.font); textFormat: Text.PlainText; elide: Text.ElideRight }

    // ------------------------------------------------------------------ the top bar
    PanelWindow {
        screen: dk.scr
        WlrLayershell.layer: WlrLayer.Top
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
        }
        T {
            anchors.centerIn: parent
            text: Qt.formatDate(clock.date, "ddd MMM d") + " · " + Qt.formatTime(clock.date, "h:mm AP")
            font.pixelSize: 15 * dk.k; font.weight: Font.DemiBold
        }
        Row {
            anchors { right: parent.right; rightMargin: 16 * dk.k; verticalCenter: parent.verticalCenter }
            spacing: 14 * dk.k
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: deskT.implicitWidth + 40 * dk.k; height: deskT.implicitHeight + 8 * dk.k; radius: 14 * dk.k
                color: F.C.card; border.width: 1; border.color: F.C.line
                Rectangle { x: 12 * dk.k; anchors.verticalCenter: parent.verticalCenter; width: 8 * dk.k; height: width; radius: width / 2; color: F.C.blue }
                T { id: deskT; x: 28 * dk.k; anchors.verticalCenter: parent.verticalCenter; text: "Desk"; font.pixelSize: 14 * dk.k; font.weight: Font.DemiBold }
                TapHandler { onTapped: dk.fh.settingsShown = true }
            }
            T {
                anchors.verticalCenter: parent.verticalCenter; text: dk.status; color: F.C.muted; font.pixelSize: 14 * dk.k; visible: text !== ""
                TapHandler { onTapped: if (dk.fh.quick) dk.fh.quick.toggle() }
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
            Image { id: ic; anchors.centerIn: parent; width: 34 * db.kk; height: width; sourceSize.width: width; source: db.icon; visible: status === Image.Ready; asynchronous: true }
            T { anchors.centerIn: parent; visible: !ic.visible; text: db.glyph; font.pixelSize: 18 * db.kk; font.weight: Font.Bold }
            TapHandler { onTapped: db.picked() }
            Accessible.role: Accessible.Button
            Accessible.name: db.label
        }
        Rectangle { anchors.horizontalCenter: parent.horizontalCenter; width: 6 * db.kk; height: width; radius: width / 2; color: db.open ? F.C.accent : "transparent" }
    }
}
