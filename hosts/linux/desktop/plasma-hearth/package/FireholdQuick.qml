// Firehold on Plasma's Quick settings (the shared org.hearthos.quicksettings, style "firehold"; mockups
// docs/mockups/firehold-plasma/07-quick-settings-desk.png and 08-quick-settings-couch.png). The same backend as
// Firehold's own (/usr/libexec/hearth/hearth-quick qs / qs-do), read when it opens and after each press, never in
// a loop. On a screen in Desk mode it opens under the bar at the right (07); on a screen in Couch mode it's the
// controller's panel down the right side, with LB / RB tabs and the focus ring (08).
// Keys (the controller sends them): arrows move, Enter (A) toggles, Left / Right (◀ ▶) adjust, Escape (B) closes,
// PageUp / PageDown (LB / RB) switch tabs.
import QtQuick
import QtQuick.Layouts
import "fh.js" as F

FocusScope {
    id: fq
    property bool shown: false
    property var qs: ({})
    property bool couch: false
    property int tab: 0                     // the Couch panel: 0 Quick Settings, 1 Notifications
    property int focusAt: 0
    property bool keys: false               // the focus ring shows once a key or button moved it
    signal openNotifications()
    readonly property string fontName: F.font(Qt.fontFamilies())

    function toggle() { if (shown) close(); else open(); }
    // Desk or Couch: this screen's mode, as the backend says it when it opens (worked out again once it answered).
    function placeFor() {
        const me = thisScreen();
        couch = !!me && me.mode === "couch";
        keys = couch;
        focusAt = -1;
        if (couch) Qt.callLater(() => { const f = fq.focusables(); fq.focusAt = f.length ? fq.order.indexOf(f[0]) : -1; });
    }
    function open() {
        tab = 0;
        placeFor();
        shown = true;
        forceActiveFocus();
        refresh(() => fq.placeFor());
    }
    function close() { shown = false; }
    function refresh(then) {
        Hearth.call(["python3", Hearth.quick, "qs"], (out) => {
            try { const o = JSON.parse(out.trim().split("\n").pop() || "{}"); if (o && typeof o === "object") fq.qs = o; } catch (e) {}
            if (then && fq.shown) then();
        });
    }
    function press(what, arg) {
        Hearth.call(["python3", Hearth.quick, "qs-do", what].concat(arg === undefined ? [] : [String(arg)]), (out) => {
            try { const r = JSON.parse(out.trim().split("\n").pop() || "{}"); if (r.ok === false) SettingsNav.toast(r.why || "That didn't work"); } catch (e) {}
            fq.refresh();
        });
    }
    function thisScreen() {
        const ms = qs.modes || [];
        return ms.find(s => s.name === Hearth.screen.name) || (ms.length === 1 ? ms[0] : null);
    }

    // ---- what the rows say
    readonly property var wifi: qs.wifi || ({})
    readonly property var bt: qs.bt || ({})
    readonly property var dnd: qs.dnd || ({})
    readonly property var night: qs.night || ({})
    readonly property var sinkList: (qs.sinks || [])
    readonly property var sink: sinkList.find(s => s.default) || sinkList[0] || null
    readonly property var bright: (qs.brightness || [])
    readonly property var mainBright: bright.find(b => !/tv/i.test(b.label)) || bright[0] || null
    readonly property var screens: qs.modes || []
    readonly property var tvs: screens.filter(s => s.tv)
    readonly property var devices: qs.devices || []
    readonly property var steps: qs.powerNames || ({ "power-saver": "Power saver", "balanced": "Balanced", "performance": "Performance" })
    readonly property string userName: qs.user || Hearth.env("USER") || "You"
    readonly property bool handheld: /ROG Ally|Steam Deck|Legion Go/.test(qs.device || "")

    function wifiSub() { return wifi.on ? (wifi.ssid ? wifi.ssid + (wifi.strength && !couch ? " · " + wifi.strength : "") : "Not connected") : "Off"; }
    function btSub() {
        if (!bt.on) return "Off";
        if (couch && (bt.names || []).length === 1) { const d = devices.find(x => x.name === bt.names[0]); return bt.names[0] + (d ? " · " + d.percent + "%" : ""); }
        return bt.connected > 0 ? bt.connected + " connected" : "On";
    }
    function dndSub() { return dnd.on ? (dnd.byHand ? "On" : "On · until " + dnd.to) : (dnd.schedule && !couch ? "Off · on at " + dnd.from : "Off"); }
    function nightSub() { return night.on ? (night.until ? "On until " + night.until : "On") : (night.until ? "Off · at " + night.until : "Off"); }
    function screenIcon(s) { return s.tv ? "tv" : "display"; }
    function screenLabel(s) { return s.label + (s.name === Hearth.screen.name && screens.length > 1 ? " (this one)" : ""); }
    function devGlyph(d) { return d.kind === "headset" ? "headphones" : d.kind === "mouse" ? "mouse" : d.kind === "keyboard" ? "kbd" : "pad"; }
    function devName(d) { return d.kind === "pad" ? "Controller " + (d.player || 1) : d.kind === "headset" ? "Headset" : d.kind === "mouse" ? "Mouse" : d.name; }

    // ---- the notifications line (Hearth's own history, the one the bell shows)
    readonly property var notes: { void DeskState.history; return DeskState.history.slice(0, 40); }
    function notesLine() {
        const n = notes.length;
        if (!n) return "";
        const first = notes.slice(0, 2).map(h => h.summary || h.body).filter(x => x);
        return first.join(" · ") + (n > first.length ? " · +" + (n - first.length) : "");
    }

    // ---- keys: the focus moves through the parts in order (the ring of mockup 08)
    property var order: []
    function focusables() { return order.filter(x => x && x.visible); }
    function moveFocus(d) {
        const f = focusables();
        if (!f.length) return;
        keys = true;
        const cur = f.indexOf(focused());
        focusAt = order.indexOf(f[Math.max(0, Math.min(f.length - 1, (cur < 0 ? (d > 0 ? -1 : f.length) : cur) + d))]);
    }
    function focused() { return focusAt >= 0 && focusAt < order.length ? order[focusAt] : null; }
    function isFocused(it) { return keys && !!it && focused() === it; }
    // A on the Mode tile: Desk; held: Away.
    Keys.onReleased: event => {
        if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) && !event.isAutoRepeat && holdA.running) { holdA.stop(); cMode.activate(); event.accepted = true; }
    }
    Keys.onPressed: event => {
        const it = focused();
        if (event.key === Qt.Key_Escape) { close(); event.accepted = true; }
        else if (event.key === Qt.Key_Down || event.key === Qt.Key_Tab) { moveFocus(1); event.accepted = true; }
        else if (event.key === Qt.Key_Up || event.key === Qt.Key_Backtab) { moveFocus(-1); event.accepted = true; }
        else if ((event.key === Qt.Key_Left || event.key === Qt.Key_Right) && it && typeof it.adjust === "function") { keys = true; it.adjust(event.key === Qt.Key_Right ? 1 : -1); event.accepted = true; }
        else if (event.key === Qt.Key_Left || event.key === Qt.Key_Right) { moveFocus(event.key === Qt.Key_Right ? 1 : -1); event.accepted = true; }
        else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) && it === cMode) { if (!event.isAutoRepeat) holdA.restart(); event.accepted = true; }
        else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) && it) { keys = true; it.activate(); event.accepted = true; }
        else if (couch && (event.key === Qt.Key_PageUp || event.key === Qt.Key_PageDown)) { tab = event.key === Qt.Key_PageDown ? 1 : 0; event.accepted = true; }
    }

    // Outside it: closes, as clicking away from a Plasma popup does.
    Rectangle { anchors.fill: parent; color: fq.couch ? "#66000000" : "transparent" }
    MouseArea { anchors.fill: parent; onClicked: fq.close() }

    // =========================================================== Desk (07): a card under the bar, at the right
    Rectangle {
        id: desk
        visible: !fq.couch
        width: 736
        x: parent.width - width - 14
        y: (Hearth.panelEdge === 0 ? Hearth.panelSize : 0) + 9
        height: dcol.implicitHeight + 44
        radius: 18
        color: F.panel
        border.color: F.line
        MouseArea { anchors.fill: parent }     // clicks inside stay inside
        ColumnLayout {
            id: dcol
            x: 22; y: 22
            width: parent.width - 44
            spacing: 12
            // You, this PC, Settings / Lock / Power
            RowLayout {
                Layout.fillWidth: true
                spacing: 14
                Rectangle {
                    Layout.preferredWidth: 50; Layout.preferredHeight: 50; radius: 25
                    gradient: Gradient { GradientStop { position: 0; color: "#ff8c42" } GradientStop { position: 1; color: "#f2621f" } }
                    Text { anchors.centerIn: parent; text: F.initial(fq.userName); color: "#ffffff"; font.pixelSize: 22; font.weight: Font.Bold; font.family: fq.fontName }
                }
                ColumnLayout {
                    spacing: 1
                    Layout.fillWidth: true
                    Text { text: fq.userName; color: F.text; font.pixelSize: 20; font.weight: Font.DemiBold; font.family: fq.fontName }
                    Text { Layout.fillWidth: true; text: fq.qs.line || ""; color: F.muted; font.pixelSize: 14; font.family: fq.fontName; elide: Text.ElideRight }
                }
                FhButton { id: bSet; glyph: "gear"; text: "Settings"; focused: fq.isFocused(bSet); onPressed: { fq.press("settings", ""); fq.close(); } }
                FhButton { id: bLock; glyph: "lock"; text: "Lock"; focused: fq.isFocused(bLock); onPressed: { fq.press("lock"); fq.close(); } }
                FhButton { id: bPow; glyph: "power"; text: "Power ▾"; focused: fq.isFocused(bPow); onPressed: powerMenu.visible = !powerMenu.visible }
            }
            // The six tiles
            GridLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                columns: 3; rowSpacing: 12; columnSpacing: 12
                FhTile { id: tWifi; Layout.fillWidth: true; glyph: "wifi"; title: "Wi-Fi"; sub: fq.wifiSub(); on: !!fq.wifi.on; focused: fq.isFocused(tWifi); onPressed: fq.press("wifi") }
                FhTile { id: tBt; Layout.fillWidth: true; glyph: "bt"; title: "Bluetooth"; sub: fq.btSub(); on: !!fq.bt.on; focused: fq.isFocused(tBt); onPressed: fq.press("bt") }
                FhTile { id: tDnd; Layout.fillWidth: true; glyph: "moon"; title: "Do not disturb"; sub: fq.dndSub(); on: !!fq.dnd.on; focused: fq.isFocused(tDnd); onPressed: fq.press("dnd") }
                FhTile { id: tKbd; Layout.fillWidth: true; glyph: "kbd"; title: "Keyboard"; sub: "On-screen · " + (fq.qs.osk ? "on" : "off"); on: !!fq.qs.osk; focused: fq.isFocused(tKbd); onPressed: fq.press("osk") }
                FhTile { id: tNight; Layout.fillWidth: true; glyph: "sun"; title: "Night light"; sub: fq.nightSub(); on: !!fq.night.on; focused: fq.isFocused(tNight); onPressed: fq.press("night") }
                FhTile { id: tShot; Layout.fillWidth: true; glyph: "cam"; title: "Screenshot"; sub: "Print · to Pictures"; focused: fq.isFocused(tShot); onPressed: { fq.close(); fq.press("screenshot"); } }
            }
            // Modes, each screen on its own (Firehold's Couch, Desk and Away)
            Rectangle {
                Layout.fillWidth: true
                visible: fq.screens.length > 0
                implicitHeight: mcol.implicitHeight + 28
                radius: 12
                color: F.tile
                ColumnLayout {
                    id: mcol
                    x: 16; y: 14; width: parent.width - 32
                    spacing: 6
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "Modes"; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fq.fontName }
                        Item { Layout.fillWidth: true }
                        Text { text: fq.screens.length > 1 ? "each screen on its own" : ""; color: F.muted; font.pixelSize: 14; font.family: fq.fontName }
                    }
                    Repeater {
                        model: fq.screens
                        delegate: RowLayout {
                            id: mrow
                            required property var modelData
                            Layout.fillWidth: true
                            Icon { name: fq.screenIcon(mrow.modelData); size: 15; color: mrow.modelData.tv ? F.desk : F.muted }
                            Text { Layout.fillWidth: true; text: fq.screenLabel(mrow.modelData); color: F.text; font.pixelSize: 15; font.family: fq.fontName; elide: Text.ElideRight }
                            FhSegs {
                                id: mseg
                                items: [{ id: "couch", text: "Couch" }, { id: "desk", text: "Desk", color: F.desk }, { id: "away", text: "Away" }]
                                current: mrow.modelData.mode || "desk"
                                itemWidth: 68
                                focused: fq.isFocused(mseg)
                                onPicked: id => fq.press("mode", mrow.modelData.name + ":" + id)
                                Component.onCompleted: fq.order = fq.order.concat([mseg])
                            }
                        }
                    }
                }
            }
            // Volume, with the output picker
            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                spacing: 8
                Icon { name: "vol"; size: 16; color: F.text }
                Text { text: "Volume"; color: F.text; font.pixelSize: 15; font.family: fq.fontName }
                Item { Layout.fillWidth: true }
                FhSegs {
                    id: outs
                    visible: fq.sinkList.length > 1
                    items: fq.sinkList.slice(0, 3).map(s => ({ id: s.name, text: s.word }))
                    current: fq.sink ? fq.sink.name : ""
                    itemHeight: 30
                    focused: fq.isFocused(outs)
                    onPicked: id => fq.press("output", id)
                }
                Item { Layout.preferredWidth: 150; visible: outs.visible }
                Text { text: fq.sink && fq.sink.volume >= 0 ? fq.sink.volume + "%" : ""; color: F.text; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: fq.fontName }
            }
            FhSlider {
                id: vol
                Layout.fillWidth: true
                value: fq.sink && fq.sink.volume >= 0 ? fq.sink.volume / 100 : 0
                focused: fq.isFocused(vol)
                onMoved: v => fq.press("volume", Math.round(v * 100))
            }
            // Brightness for the monitor; a TV's is on its remote
            RowLayout {
                Layout.fillWidth: true
                visible: !!fq.mainBright
                spacing: 8
                Icon { name: "sun"; size: 16; color: F.text }
                Text { text: "Brightness" + (fq.mainBright && fq.mainBright.label !== "This screen" ? " · " + fq.mainBright.label : ""); color: F.text; font.pixelSize: 15; font.family: fq.fontName }
                Item { Layout.fillWidth: true }
                Text { visible: fq.tvs.length > 0; text: "TV: set on the TV's remote"; color: F.muted; font.pixelSize: 14; font.family: fq.fontName }
                Item { Layout.preferredWidth: 150; visible: fq.tvs.length > 0 }
                Text { text: fq.mainBright ? fq.mainBright.percent + "%" : ""; color: F.text; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: fq.fontName }
            }
            FhSlider {
                id: bri
                Layout.fillWidth: true
                visible: !!fq.mainBright
                value: fq.mainBright ? fq.mainBright.percent / 100 : 0
                focused: fq.isFocused(bri)
                onMoved: v => fq.press("brightness", (fq.mainBright.id || "") + ":" + Math.max(1, Math.round(v * 100)))
            }
            // Power profile
            RowLayout {
                Layout.fillWidth: true
                visible: (fq.qs.power || "") !== ""
                spacing: 8
                Icon { name: "bolt"; size: 16; color: F.warn }
                Text { text: "Power profile"; color: F.text; font.pixelSize: 15; font.family: fq.fontName }
                Item { Layout.fillWidth: true }
                FhSegs {
                    id: prof
                    items: ["power-saver", "balanced", "performance"].map(k => ({ id: k, text: fq.steps[k] || k }))
                    current: fq.qs.power || ""
                    focused: fq.isFocused(prof)
                    onPicked: id => fq.press("power", id)
                }
            }
            // Batteries: the PC's devices (controllers, headset, mouse)
            Rectangle {
                Layout.fillWidth: true
                visible: fq.devices.length > 0
                implicitHeight: bcol.implicitHeight + 28
                radius: 12
                color: F.tile
                ColumnLayout {
                    id: bcol
                    x: 16; y: 14; width: parent.width - 32
                    spacing: 10
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "Batteries"; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fq.fontName }
                        Item { Layout.fillWidth: true }
                        Text { text: (fq.qs.battery === undefined || fq.qs.battery < 0) ? "this PC has none: these are its devices" : ""; color: F.muted; font.pixelSize: 14; font.family: fq.fontName }
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 8
                        Repeater {
                            model: fq.devices
                            delegate: Rectangle {
                                required property var modelData
                                width: chip.implicitWidth + 30; height: 34; radius: 17
                                color: "#3a312c"
                                Row {
                                    id: chip
                                    anchors.centerIn: parent
                                    spacing: 6
                                    Icon { anchors.verticalCenter: parent.verticalCenter; name: fq.devGlyph(parent.parent.modelData); size: 14; color: parent.parent.modelData.low ? F.warn : F.text }
                                    Text { anchors.verticalCenter: parent.verticalCenter; text: fq.devName(parent.parent.modelData) + " · " + parent.parent.modelData.percent + "%"; color: parent.parent.modelData.low ? F.warn : F.text; font.pixelSize: 14; font.weight: Font.DemiBold; font.family: fq.fontName }
                                }
                            }
                        }
                    }
                }
            }
            // Return to Game Mode
            Rectangle {
                id: gm
                Layout.fillWidth: true
                implicitHeight: 68
                radius: 12
                color: "transparent"
                border.color: F.accent
                border.width: 1
                ColumnLayout {
                    anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter; right: goB.left; rightMargin: 12 }
                    spacing: 2
                    Text { text: fq.qs.gameModeOff ? "Game Mode is off on this device" : "Return to Game Mode"; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fq.fontName }
                    Text { Layout.fillWidth: true; text: fq.qs.gameModeOff ? "Turn it on in Settings › System › Startup" : "Steam Game Mode · desktop apps close first, it asks about unsaved work"; color: F.muted; font.pixelSize: 14; font.family: fq.fontName; elide: Text.ElideRight }
                }
                FhButton {
                    id: goB
                    anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter }
                    primary: true; text: fq.qs.gameModeOff ? "Open" : "Go"
                    focused: fq.isFocused(goB)
                    onPressed: { fq.close(); if (fq.qs.gameModeOff) fq.press("settings", "startup"); else fq.press("gamemode"); }
                }
            }
            // Notifications, in a line, and Open all
            RowLayout {
                Layout.fillWidth: true
                visible: fq.notes.length > 0
                spacing: 6
                Icon { name: "bell"; size: 15; color: F.warn }
                Text { text: fq.notes.length + (fq.notes.length === 1 ? " notification" : " notifications"); color: F.text; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: fq.fontName }
                Text { Layout.fillWidth: true; text: "· " + fq.notesLine(); color: F.muted; font.pixelSize: 15; font.family: fq.fontName; elide: Text.ElideRight }
                Text {
                    id: openAll
                    text: "Open all ›"; color: F.accent; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: fq.fontName
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { fq.close(); fq.openNotifications(); } }
                }
            }
        }
        // Power ▾: Sleep, Restart, Shut down, Log out
        Rectangle {
            id: powerMenu
            visible: false
            x: parent.width - width - 22; y: 72
            width: 190; height: pcol.implicitHeight + 12; radius: 12
            color: F.tile; border.color: F.line
            z: 5
            Column {
                id: pcol
                x: 6; y: 6; width: parent.width - 12
                Repeater {
                    model: [{ t: "Sleep", w: "sleep", g: "sleep" }, { t: "Restart", w: "restart", g: "restart" }, { t: "Shut down", w: "shutdown", g: "power" }, { t: "Log out", w: "logout", g: "logout" }]
                    delegate: Rectangle {
                        required property var modelData
                        width: pcol.width; height: 38; radius: 8
                        color: pma.containsMouse ? F.seg : "transparent"
                        Row { x: 12; anchors.verticalCenter: parent.verticalCenter; spacing: 10
                            Icon { name: parent.parent.modelData.g; size: 15; color: F.text }
                            Text { text: parent.parent.modelData.t; color: F.text; font.pixelSize: 15; font.family: fq.fontName } }
                        MouseArea { id: pma; anchors.fill: parent; hoverEnabled: true; onClicked: { powerMenu.visible = false; fq.close(); fq.press(parent.modelData.w); } }
                    }
                }
            }
        }
    }

    // =========================================================== Couch (08): the controller's panel at the right
    Rectangle {
        id: side
        visible: fq.couch
        width: Math.round(parent.width * 0.375)
        anchors { right: parent.right; top: parent.top; bottom: parent.bottom }
        color: F.panel
        Rectangle { width: 1; height: parent.height; color: F.line }
        MouseArea { anchors.fill: parent }
        // LB / RB tabs
        Row {
            x: 40; y: 34
            spacing: 28
            Repeater {
                model: ["Quick Settings", "Notifications · " + fq.notes.length]
                delegate: Column {
                    required property var modelData
                    required property int index
                    spacing: 6
                    Text { text: parent.modelData; color: fq.tab === parent.index ? F.text : F.muted; font.pixelSize: 24; font.weight: Font.DemiBold; font.family: fq.fontName
                        MouseArea { anchors.fill: parent; onClicked: fq.tab = parent.parent.index } }
                    Rectangle { width: parent.width; height: 3; radius: 1.5; color: F.accent; visible: fq.tab === parent.index }
                }
            }
        }
        Rectangle {
            anchors { right: parent.right; rightMargin: 40 }
            y: 38
            width: lbrb.implicitWidth + 18; height: 30; radius: 15; color: "#d9d2cc"
            Text { id: lbrb; anchors.centerIn: parent; text: "LB RB"; color: "#1b1715"; font.pixelSize: 13; font.weight: Font.Bold; font.family: fq.fontName }
        }
        // ---- the Quick Settings tab
        ColumnLayout {
            id: ccol
            visible: fq.tab === 0
            x: 40; y: 94; width: parent.width - 80
            spacing: 12
            RowLayout {
                Layout.fillWidth: true
                spacing: 16
                Rectangle {
                    Layout.preferredWidth: 54; Layout.preferredHeight: 54; radius: 27
                    gradient: Gradient { GradientStop { position: 0; color: "#ff8c42" } GradientStop { position: 1; color: "#f2621f" } }
                    Text { anchors.centerIn: parent; text: F.initial(fq.userName); color: "#ffffff"; font.pixelSize: 22; font.weight: Font.Bold; font.family: fq.fontName }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1
                    Text { text: fq.qs.device || ""; color: F.text; font.pixelSize: 20; font.weight: Font.DemiBold; font.family: fq.fontName }
                    Text { Layout.fillWidth: true; text: fq.qs.battery >= 0 ? ["Battery " + fq.qs.battery + "%", (fq.qs.batteryMore || {}).left || "", (fq.qs.batteryMore || {}).limit ? "charges to " + fq.qs.batteryMore.limit + "%" : ""].filter(x => x).join(" · ") : (fq.qs.line || ""); color: F.muted; font.pixelSize: 15; font.family: fq.fontName; elide: Text.ElideRight }
                }
                Rectangle {
                    visible: fq.qs.battery >= 0
                    Layout.preferredWidth: 120; Layout.preferredHeight: 10; radius: 5; color: F.dot
                    Rectangle { width: parent.width * Math.max(0, Math.min(100, fq.qs.battery || 0)) / 100; height: parent.height; radius: 5; color: fq.qs.battery <= 20 ? F.warn : F.good }
                }
            }
            GridLayout {
                Layout.fillWidth: true
                columns: 2; rowSpacing: 12; columnSpacing: 12
                FhTile { id: cWifi; Layout.fillWidth: true; glyph: "wifi"; title: "Wi-Fi"; sub: fq.wifiSub(); on: !!fq.wifi.on; focused: fq.isFocused(cWifi); onPressed: fq.press("wifi") }
                FhTile { id: cBt; Layout.fillWidth: true; glyph: "bt"; title: "Bluetooth"; sub: fq.btSub(); on: !!fq.bt.on; focused: fq.isFocused(cBt); onPressed: fq.press("bt") }
                FhTile { id: cDnd; Layout.fillWidth: true; glyph: "moon"; title: "Do not disturb"; sub: fq.dndSub(); on: !!fq.dnd.on; focused: fq.isFocused(cDnd); onPressed: fq.press("dnd") }
                FhTile { id: cKbd; Layout.fillWidth: true; glyph: "kbd"; title: "Keyboard"; sub: fq.qs.osk ? "On-screen · on" : "Show on-screen"; on: !!fq.qs.osk; focused: fq.isFocused(cKbd); onPressed: fq.press("osk") }
                FhTile {
                    id: cMode; Layout.fillWidth: true; glyph: "tv"
                    readonly property var me: fq.thisScreen()
                    title: "Mode: " + F.modeName(me ? me.mode : "couch"); sub: "A: Desk · hold A: Away"; on: true
                    focused: fq.isFocused(cMode)
                    onPressed: if (me) { fq.press("mode", me.name + ":desk"); fq.close(); }
                    Timer { id: holdA; interval: 600; onTriggered: if (cMode.me) { fq.press("mode", cMode.me.name + ":away"); fq.close(); } }
                }
                FhTile { id: cNight; Layout.fillWidth: true; glyph: "sun"; title: "Night light"; sub: fq.nightSub(); on: !!fq.night.on; focused: fq.isFocused(cNight); onPressed: fq.press("night") }
            }
            RowLayout {
                Layout.fillWidth: true; Layout.topMargin: 2
                spacing: 8
                Icon { name: "vol"; size: 17; color: F.text }
                Text { text: "Volume" + (fq.sink ? " · " + fq.sink.word : ""); color: F.text; font.pixelSize: 17; font.family: fq.fontName }
                Item { Layout.fillWidth: true }
                Text { text: fq.sink && fq.sink.volume >= 0 ? fq.sink.volume + "%" : ""; color: F.text; font.pixelSize: 17; font.weight: Font.DemiBold; font.family: fq.fontName }
            }
            FhSlider { id: cVol; Layout.fillWidth: true; value: fq.sink && fq.sink.volume >= 0 ? fq.sink.volume / 100 : 0; focused: fq.isFocused(cVol); onMoved: v => fq.press("volume", Math.round(v * 100)) }
            RowLayout {
                Layout.fillWidth: true
                visible: !!fq.mainBright
                spacing: 8
                Icon { name: "sun"; size: 17; color: F.text }
                Text { text: "Brightness"; color: F.text; font.pixelSize: 17; font.family: fq.fontName }
                Item { Layout.fillWidth: true }
                Text { text: fq.mainBright ? fq.mainBright.percent + "%" : ""; color: F.text; font.pixelSize: 17; font.weight: Font.DemiBold; font.family: fq.fontName }
            }
            FhSlider { id: cBri; Layout.fillWidth: true; visible: !!fq.mainBright; value: fq.mainBright ? fq.mainBright.percent / 100 : 0; focused: fq.isFocused(cBri); onMoved: v => fq.press("brightness", (fq.mainBright.id || "") + ":" + Math.max(1, Math.round(v * 100))) }
            // Power, in the device's own steps (◀ ▶ to change)
            Rectangle {
                id: cPow
                Layout.fillWidth: true
                visible: (fq.qs.power || "") !== ""
                implicitHeight: 108
                radius: 14
                color: fq.isFocused(cPow) ? "#2e211b" : F.tile
                border.width: fq.isFocused(cPow) ? 3 : 0
                border.color: F.accent
                function adjust(d) { cSegs.adjust(d); }
                function activate() { cSegs.adjust(1); }
                RowLayout {
                    x: 22; y: 16; width: parent.width - 44
                    Icon { name: "bolt"; size: 17; color: F.warn }
                    Text { text: "Power"; color: F.text; font.pixelSize: 17; font.weight: Font.DemiBold; font.family: fq.fontName }
                    Item { Layout.fillWidth: true }
                    Text { text: "◀ ▶ to change"; color: F.muted; font.pixelSize: 14; font.family: fq.fontName }
                }
                FhSegs {
                    id: cSegs
                    x: 22; y: 52; width: parent.width - 44
                    itemWidth: (width - 8 - 4) / 3
                    items: ["power-saver", "balanced", "performance"].map(k => ({ id: k, text: fq.steps[k] || k }))
                    current: fq.qs.power || ""
                    onPicked: id => fq.press("power", id)
                }
            }
            // Game Mode switched off on this device
            Rectangle {
                id: cGm
                Layout.fillWidth: true
                visible: fq.qs.gameModeOff === true
                implicitHeight: 66; radius: 12; color: F.tile
                border.width: fq.isFocused(cGm) ? 2 : 0; border.color: F.accent
                function activate() { fq.close(); fq.press("settings", "startup"); }
                Rectangle { x: 18; anchors.verticalCenter: parent.verticalCenter; width: 38; height: 38; radius: 8; color: F.dot
                    Icon { anchors.centerIn: parent; name: "pad"; size: 18; color: F.text } }
                Column {
                    x: 70; anchors.verticalCenter: parent.verticalCenter
                    Text { text: "Game Mode is off on this device"; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fq.fontName }
                    Text { text: "Turn it on in Settings › System › Startup"; color: F.muted; font.pixelSize: 14; font.family: fq.fontName }
                }
                FhButton { anchors { right: parent.right; rightMargin: 18; verticalCenter: parent.verticalCenter } text: "Open"; onPressed: cGm.activate() }
            }
            // The controllers: who's player 1, a low battery
            Rectangle {
                Layout.fillWidth: true
                visible: fq.devices.some(d => d.kind === "pad") || fq.handheld
                implicitHeight: 42; radius: 10; color: F.tile
                readonly property var low: fq.devices.find(d => d.kind === "pad" && d.low)
                Row { x: 18; anchors.verticalCenter: parent.verticalCenter; spacing: 8
                    Icon { name: "pad"; size: 15; color: F.text }
                    Text { text: (fq.handheld ? "Built-in controls" : "Controller 1") + " · player 1"; color: F.text; font.pixelSize: 15; font.family: fq.fontName } }
                Text { anchors { right: parent.right; rightMargin: 18; verticalCenter: parent.verticalCenter }
                    text: parent.low ? fq.devName(parent.low) + " · " + parent.low.percent + "%" : ""; color: F.muted; font.pixelSize: 15; font.family: fq.fontName }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                FhButton { id: cSleep; Layout.fillWidth: true; Layout.preferredWidth: 1; h: 40; text: "Sleep"; focused: fq.isFocused(cSleep); onPressed: { fq.close(); fq.press("sleep"); } }
                FhButton { id: cRestart; Layout.fillWidth: true; Layout.preferredWidth: 1; h: 40; text: "Restart"; focused: fq.isFocused(cRestart); onPressed: { fq.close(); fq.press("restart"); } }
                FhButton { id: cOff; Layout.fillWidth: true; Layout.preferredWidth: 1; h: 40; text: "Shut down"; focused: fq.isFocused(cOff); onPressed: { fq.close(); fq.press("shutdown"); } }
                FhButton { id: cSet; Layout.fillWidth: true; Layout.preferredWidth: 1; h: 40; text: "Settings"; focused: fq.isFocused(cSet); onPressed: { fq.close(); fq.press("settings", ""); } }
            }
        }
        // ---- the Notifications tab: Hearth's history, newest first
        ListView {
            visible: fq.tab === 1
            x: 40; y: 100; width: parent.width - 80; height: parent.height - 200
            clip: true
            spacing: 10
            model: fq.notes
            delegate: Rectangle {
                required property var modelData
                width: ListView.view.width; height: 72; radius: 12; color: F.tile
                Rectangle { x: 14; anchors.verticalCenter: parent.verticalCenter; width: 40; height: 40; radius: 10; color: F.tint(parent.modelData.app)
                    Text { anchors.centerIn: parent; text: F.initial(parent.parent.modelData.app); color: "#ffffff"; font.pixelSize: 17; font.weight: Font.Bold; font.family: fq.fontName } }
                Column { x: 68; anchors.verticalCenter: parent.verticalCenter; width: parent.width - 150
                    Text { width: parent.width; text: parent.parent.modelData.summary; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fq.fontName; elide: Text.ElideRight }
                    Text { width: parent.width; text: parent.parent.modelData.app + " · " + parent.parent.modelData.body; color: F.muted; font.pixelSize: 14; font.family: fq.fontName; elide: Text.ElideRight } }
                Text { anchors { right: parent.right; rightMargin: 16; top: parent.top; topMargin: 12 } text: F.ago(parent.modelData.time, true); color: F.muted; font.pixelSize: 13; font.family: fq.fontName }
            }
        }
        // Button hints
        Row {
            anchors { right: parent.right; rightMargin: 40; bottom: parent.bottom; bottomMargin: 24 }
            spacing: 30
            Repeater {
                model: [{ k: "A", c: "#3ecf6e", t: "Toggle" }, { k: "◀▶", c: "#d9d2cc", t: "Adjust" }, { k: "B", c: "#f05a5a", t: "Close" }]
                delegate: Row {
                    required property var modelData
                    spacing: 10
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: Math.max(32, k.implicitWidth + 14); height: 32; radius: 16; color: parent.modelData.c
                        Text { id: k; anchors.centerIn: parent; text: parent.parent.modelData.k; color: "#1b1715"; font.pixelSize: 14; font.weight: Font.Bold; font.family: fq.fontName } }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: parent.modelData.t; color: F.text; font.pixelSize: 20; font.family: fq.fontName }
                }
            }
        }
    }

    // The order the focus goes through (Desk, then Couch)
    Component.onCompleted: order = order.concat([bSet, bLock, bPow, tWifi, tBt, tDnd, tKbd, tNight, tShot, outs, vol, bri, prof, goB,
                                                 cWifi, cBt, cDnd, cKbd, cMode, cNight, cVol, cBri, cPow, cGm, cSleep, cRestart, cOff, cSet])
}
