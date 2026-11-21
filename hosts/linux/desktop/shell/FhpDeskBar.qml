// Firehold on labwc's Desk bar (1.16, mockups firehold-plasma 04, 11, 12; checklist firehold-labwc.md), left to right
// as Firehold on Plasma's: the app in front (its tile and name: About, Settings, Quit), its own menus (Hearth's menu
// source, FhGlobalMenu.qml) or, for an app with none, Hearth's standard ones (the app, Window, Help: deskmenus.js), the
// date and clock (calendar and notifications), the mode pill (Modes: ModesCard), downloads, Hearth Link, the
// controllers, Quick Settings and notifications; the first-run tip naming each part. The menus are the Couch's
// MenuCard.qml, the tray the Couch's TrayButton.qml: the same files as under Plasma. Loaded by FireholdDesk.qml by file
// name (if anything here fails, Firehold's own bar stays). Nothing runs by itself: the bar's cards are asked for once a
// minute only while someone's using the PC (DeskState.idle stops it).
import QtQuick
import Quickshell
import Quickshell.Wayland
import "fhcouch"
import "fhcouch/deskmenus.js" as DM
import "fhcouch/fhp.js" as H

PanelWindow {
    id: bar
    property var fh
    readonly property real k: screen ? Math.min(screen.width / 1920, screen.height / 1080) : 1
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.namespace: "hearth-firehold-desk-bar"
    anchors { top: true; left: true; right: true }
    implicitHeight: 44 * k
    exclusiveZone: 44 * k
    color: Qt.rgba(28 / 255, 21 / 255, 18 / 255, 0.92)
    readonly property string font: Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : "Noto Sans"
    component L: Text { color: "#f2ede8"; font.family: bar.font; font.pixelSize: 15 * bar.k; textFormat: Text.PlainText; elide: Text.ElideRight }
    Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: "#14ffffff" }

    // ---------------------------------------------------------------- the app in front
    readonly property var front: WinList.activeToplevel
    readonly property string appName: {
        if (!front) return "Firehold";
        const e = AppIcons.entryFor(front.appId);
        const n = e ? String(e.name) : String(front.appId || "");
        // A runtime's name (qml, wine, java, python) says nothing: the window's title does (Trains 2010).
        // (no desktop entry: the window's own title, not its app id)
        return !e || /^(qml|qml6|wine.*|java|python3?|.*\.exe)$/i.test(n) || n === "" ? String(front.title || n || "Firehold").split(" — ")[0].split(" - ")[0] : n;
    }
    FhGlobalMenu { id: gm; active: true }
    readonly property var globalMenu: gm
    readonly property bool ownMenus: gm.menus.length > 0
    readonly property var screens: fh.modes.modes.screens || []
    readonly property string frontScreen: front && front.screens && front.screens.length ? String(front.screens[0].name) : ""
    property string open: ""          // "app" | "window" | "help" | "own:<id>"
    property Item anchorItem: null
    property var subRows: []
    function openMenu(which, item) {
        if (open === which) { open = ""; return; }
        anchorItem = item; subRows = [];
        if (which.indexOf("own:") === 0) gm.read(() => { bar.open = which; });
        else open = which;
    }
    readonly property var rows: open === "app" ? DM.appItems(appName) : open === "window" ? DM.windowItems(appName) : open === "help" ? DM.helpItems()
        : open.indexOf("own:") === 0 ? gm.rows((gm.menus.find(m => "own:" + m.id === open) || {}).items) : []

    function wt(keys) { Quickshell.execDetached(["sh", "-c", "sleep 0.2; exec wtype " + keys]); }
    function ipc(args) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call"].concat(args)); }
    function act(a) {
        open = "";
        const w = bar.front;
        if (a.indexOf("id:") === 0) { gm.click(a.slice(3)); return; }
        if (a.indexOf("move:") === 0) {
            // labwc's own Move to output (Super+Shift+← →, rc.xml): toward the screen that was picked
            const names = Quickshell.screens.map(s => s.name), from = names.indexOf(frontScreen), to = names.indexOf(a.slice(5));
            if (w && to >= 0 && to !== from) { w.activate(); wt(to > from ? "-M logo -M shift -k Right -m shift -m logo" : "-M logo -M shift -k Left -m shift -m logo"); }
            return;
        }
        if (a === "about" || a === "settings") Quickshell.execDetached(["hearth-settings-app", "--page", "apps"]);
        else if (a === "folder") {
            const e = w ? AppIcons.entryFor(w.appId) : null;
            const cmd = e && e.command && e.command.length ? String(e.command[0]) : "";
            Quickshell.execDetached(["sh", "-c", 'p=$(command -v "$1" 2>/dev/null) && exec xdg-open "$(dirname "$(readlink -f "$p")")"', "sh", cmd]);
        }
        else if (a === "logs") Quickshell.execDetached(["hearth-send-logs", ("Sent from the Desk bar: " + appName).slice(0, 200)]);
        else if (a === "close") { if (w) w.close(); }
        else if (a === "fullscreen") { if (w) w.fullscreen = !w.fullscreen; }
        else if (a === "maximize") { if (w) w.maximized = !w.maximized; }
        else if (a === "minimize") { if (w) w.minimized = true; }
        else if (a === "above") { if (w) { w.activate(); wt("-M logo -M alt -k t -m alt -m logo"); } }
        else if (a === "snap-left" || a === "snap-right") { if (w) { w.activate(); wt("-M logo -k " + (a === "snap-left" ? "Left" : "Right") + " -m logo"); } }
        else if (a === "screenshot") ipc(["capture", "take", "window"]);
        else if (a === "record") ipc(["record", "start"]);
        else if (a === "kill") ipc(["forceclose", "start"]);
        else if (a === "search") ipc(["searchbar", "key"]);
        else if (a === "shortcuts") Quickshell.execDetached(["hearth-settings-app", "--page", "keyboard"]);
    }

    Row {
        id: left
        x: 14 * bar.k; anchors.verticalCenter: parent.verticalCenter
        spacing: 14 * bar.k
        // The tile: back to the Couch's Home.
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            // (Hearth's own apps in the flame's orange, any other app in green, as the mockups draw them)
            width: 30 * bar.k; height: width; radius: 8 * bar.k
            color: !bar.front || /^(hearth|nimbus|org\.hearthos)/i.test(String(bar.front.appId || "")) || bar.appName.indexOf("Hearth") === 0 ? "#ff7a2f" : "#4f8a5c"
            L { anchors.centerIn: parent; text: bar.appName.replace(/^Hearth\s+/, "").charAt(0).toUpperCase(); color: "#ffffff"; font.weight: Font.ExtraBold; font.pixelSize: 15 * bar.k }
            TapHandler { onTapped: bar.fh.couchHome() }
        }
        Item {
            id: nameBtn
            anchors.verticalCenter: parent.verticalCenter
            width: nameT.implicitWidth; height: 30 * bar.k
            L { id: nameT; anchors.verticalCenter: parent.verticalCenter; text: bar.appName; font.weight: Font.Bold; font.pixelSize: 16 * bar.k }
            TapHandler { onTapped: bar.openMenu("app", nameBtn) }
        }
        Repeater {
            model: bar.ownMenus ? gm.menus.map(m => ["own:" + m.id, m.label]) : (bar.front ? [["app", bar.appName], ["window", "Window"], ["help", "Help"]] : [])
            delegate: Rectangle {
                id: mb
                required property var modelData
                anchors.verticalCenter: parent ? parent.verticalCenter : undefined
                width: mt.implicitWidth + 24 * bar.k; height: 30 * bar.k; radius: 8 * bar.k
                color: bar.open === modelData[0] ? "#2eff7a2f" : "transparent"
                L { id: mt; anchors.centerIn: parent; text: mb.modelData[1]; font.pixelSize: 16 * bar.k }
                TapHandler { onTapped: bar.openMenu(mb.modelData[0], mb) }
            }
        }
    }

    // ---------------------------------------------------------------- the date and clock: calendar and notifications
    SystemClock { id: clock; precision: SystemClock.Minutes }
    L {
        anchors.centerIn: parent
        text: Qt.formatDate(clock.date, "ddd MMM d") + " · " + Qt.formatTime(clock.date, "h:mm AP")
        font.pixelSize: 16 * bar.k; font.weight: Font.DemiBold
        TapHandler { onTapped: bar.fh.toggleList() }
    }

    // ---------------------------------------------------------------- the right side
    readonly property var lm: fh.modes
    property var cards: ({})
    readonly property var dls: cards.downloads || []
    readonly property var running: dls.filter(d => d.state === "running")
    readonly property var pads: (cards.controllers || []).filter(c => c.kind === "pad")
    FhRun { id: runner }
    property bool asking: false
    function askCards(first) {
        if (asking || (!first && DeskState.idle)) return;      // (the first look always: what the bar shows when it comes up)
        asking = true;
        runner.run(first ? ["home", "fast"] : ["home-after", running.length ? "5" : "60", "fast"], function (t) {
            bar.asking = false;
            const o = runner.lastJson(t);
            if (o) bar.cards = o;
            if (bar.visible) bar.askCards(false);
        });
    }
    Connections { target: DeskState; function onIdleChanged() { if (!DeskState.idle) bar.askCards(true); } }
    readonly property string net: NetBtWatch.netKind === "wifi" ? "Wi-Fi" : NetBtWatch.netKind === "ethernet" ? "Wired" : "Offline"
    Row {
        anchors { right: parent.right; rightMargin: 14 * bar.k; verticalCenter: parent.verticalCenter }
        spacing: 6 * bar.k
        Rectangle {
            id: pill
            anchors.verticalCenter: parent.verticalCenter
            width: pillRow.implicitWidth + 26 * bar.k; height: 30 * bar.k; radius: 15 * bar.k
            color: bar.lm.popupOpen ? "#38ff7a2f" : "#12ffffff"
            Row {
                id: pillRow
                anchors.centerIn: parent
                spacing: 8 * bar.k
                Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 9 * bar.k; height: width; radius: width / 2; color: bar.lm.me.mode === "couch" ? "#ff7a2f" : "#5bb8ff" }
                L { anchors.verticalCenter: parent.verticalCenter; text: bar.lm.modeName(bar.lm.me.mode); font.pixelSize: 14 * bar.k; font.weight: Font.DemiBold }
                L { anchors.verticalCenter: parent.verticalCenter; visible: !!bar.lm.tv; text: bar.lm.tv ? "· TV: " + bar.lm.modeName(bar.lm.tv.mode) : ""; color: "#a39d96"; font.pixelSize: 14 * bar.k; font.weight: Font.Medium }
            }
            TapHandler { onTapped: bar.lm.popupOpen = !bar.lm.popupOpen }
        }
        Item { width: 6 * bar.k; height: 1 }
        TrayButton {
            anchors.verticalCenter: parent.verticalCenter
            visible: bar.dls.length > 0
            text: String(bar.running.length || bar.dls.length)
            ring: bar.running.length ? bar.running[0].percent : -1
            onClicked: Quickshell.execDetached(["hearth-settings-app", "--page", "storage"])
        }
        TrayButton { anchors.verticalCenter: parent.verticalCenter; glyph: "link"; onClicked: Quickshell.execDetached(["hearth-settings-app", "--page", "link"]) }
        TrayButton { anchors.verticalCenter: parent.verticalCenter; visible: bar.pads.length > 0; glyph: "pad"; text: bar.pads.length ? bar.pads[0].percent + "%" : "" }
        TrayButton { anchors.verticalCenter: parent.verticalCenter; glyph: "speaker"; text: bar.net; onClicked: bar.fh.toggleQuick() }
        TrayButton { anchors.verticalCenter: parent.verticalCenter; glyph: "bell"; text: DeskState.history.length > 0 ? String(DeskState.history.length) : ""; onClicked: bar.fh.toggleList() }
    }

    // ---------------------------------------------------------------- the menus (the Couch's MenuCard.qml)
    PopupWindow {
        anchor.window: bar
        anchor.rect.x: bar.anchorItem ? bar.anchorItem.mapToItem(null, 0, 0).x - 6 * bar.k : 0
        anchor.rect.y: 44 * bar.k + 4 * bar.k
        visible: bar.open !== "" && bar.rows.length > 0
        implicitWidth: card.width
        implicitHeight: card.height
        color: "transparent"
        MenuCard {
            id: card
            items: bar.rows
            subItems: bar.open === "window" ? DM.moveItems(bar.screens, bar.frontScreen) : bar.subRows
            subOpen: false
            iconFile: Qt.resolvedUrl("FhIcon.qml")
            onPicked: a => bar.act(a)
            onSubRequested: row => { if (String(row[2]).indexOf("sub:") === 0) bar.subRows = gm.rows(gm.sub(gm.menus, String(row[2]).slice(4))); }
        }
        onVisibleChanged: if (!visible) card.subOpen = false
    }
    // Firehold's IPC (the checks' pictures): a menu open by name ("File", "Window", "help"...), "" closes.
    function openNamed(n) {
        if (n === "") { open = ""; return; }
        const which = n === "move" ? "window" : n;
        for (let i = 0; i < left.children.length; i++) {
            const c = left.children[i];
            if (c.modelData && (c.modelData[1] === which || c.modelData[0] === which)) {
                openMenu(c.modelData[0], c);
                if (n === "move") Qt.callLater(() => card.openSub({ y: card.firstSubY, modelData: ["", "", "", "sub"] }));
                return;
            }
        }
    }

    // ---------------------------------------------------------------- the first-run tip (once: state, not a setting)
    property bool tipShown: false
    PopupWindow {
        anchor.window: bar
        anchor.rect.x: bar.width - implicitWidth - 120 * bar.k
        anchor.rect.y: 44 * bar.k + 10 * bar.k
        visible: bar.tipShown
        implicitWidth: tipCard.width
        implicitHeight: tipCard.height
        color: "transparent"
        BarTipCard { id: tipCard }
        TapHandler { onTapped: bar.tipShown = false }
    }
    onOpenChanged: if (open !== "") tipShown = false
    Component.onCompleted: {
        askCards(true);
        runner.run(["tip"], function (t) { if (t.trim() === "show") bar.tipShown = true; });
    }
}
