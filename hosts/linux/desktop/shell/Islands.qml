// Mantel 2026: the top of the screen. Floating islands (Hearth, Gaming, Glass, Synthwave, Cabin), one full-width
// bar (Steam-style; Zen's is thin) or Console's big status row (theme/palettes.json "islandStyle", or your own
// Settings › Appearance › Bar and dock choice):
//   left   the Hearth mark (opens the launcher), the desktops, the window in front (icon + title)
//   (Gaming) a performance island: FPS (only when MangoHud logs it), CPU, GPU, memory, temperatures
//   centre the clock pill: grows to show what's playing, and briefly the newest notification; click for the
//          notification centre and dashboard
//   right  the status island: tray, Friends, keyboard, Wi-Fi, Bluetooth, volume, battery, notifications (click: Quick
//          settings; right-click menus; middle-click mutes; scroll changes the volume), then the Game Mode button,
//          always shown, in every look, one press away.
// Loaded by shell.qml through a Loader: if this file can't load, the classic taskbar (Bar.qml) comes up instead.
// Handheld screens (1440 px wide or less) keep only the essentials here; the rest is in Quick settings.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.SystemTray
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower

PanelWindow {
    id: top
    signal launcherRequested()
    signal quickRequested()
    signal keyboardRequested()
    signal calendarRequested()
    signal settingsRequested(string page)
    property var menu: null                // PopMenu
    property bool primary: false
    property bool quickOpen: false         // Quick settings is open: the status island is ringed
    property bool calendarOpen: false      // the dashboard is open: the pill is ringed

    readonly property string style: Theme.islandStyle          // floating | bar | shelf
    readonly property bool floating: style === "floating"
    readonly property bool shelf: style === "shelf"
    readonly property bool compact: screen && screen.width <= 1440 && !shelf
    readonly property real k: shelf ? 1.4 : 1
    readonly property int islandH: shelf ? 72 : Theme.thin ? 32 : style === "bar" ? 44 : compact ? 48 : 52
    readonly property int gapTop: floating ? 10 : shelf ? 18 : 0
    readonly property int band: islandH + gapTop + (floating ? 6 : shelf ? 10 : 0)
    readonly property bool gaming: Theme.look === "gaming"

    anchors { top: true; left: true; right: true }
    implicitHeight: band
    exclusiveZone: band
    color: "transparent"
    WlrLayershell.namespace: "hearth-islands"
    WlrLayershell.layer: WlrLayer.Top
    function claim() { if (primary) { DeskState.topInset = band; DeskState.classicBar = false; } }
    onBandChanged: claim()
    onPrimaryChanged: claim()
    Component.onCompleted: claim()

    // Menus open just under the bar.
    function menuPoint(item) {
        const p = item.mapToItem(null, item.width / 2, item.height);
        return Qt.point(Math.max(8, Math.min((screen ? screen.width : 1920) - 300, p.x - 134)), top.band + 2);
    }
    function statusMenu(btn, title, list) {
        if (!menu) return;
        const p = menuPoint(btn);
        menu.popup(screen, p.x, p.y, { title: title, above: false }, list);
    }
    function soundMenu(btn) {
        const sink = Pipewire.defaultAudioSink;
        const outs = Pipewire.nodes ? Pipewire.nodes.values.filter(n => n.audio && n.isSink && !n.isStream) : [];
        const list = outs.slice(0, 8).map(n => ({
            label: n.description || n.nickname || n.name || "Speakers", checked: n === sink ? true : undefined, icon: "vol",
            run: () => { Pipewire.preferredDefaultAudioSink = n; }
        }));
        if (list.length) list.push({ separator: true });
        const a = sink ? sink.audio : null;
        if (a) list.push({ label: a.muted ? "Unmute" : "Mute", icon: a.muted ? "vol" : "vol-mute", run: () => { a.muted = !a.muted; } });
        list.push({ label: "Sound settings", icon: "gear", run: () => top.settingsRequested("sound") });
        statusMenu(btn, "Sound output", list);
    }

    // ------------------------------------------------------------------ system state (as the classic bar)
    PwObjectTracker { objects: [Pipewire.defaultAudioSink] }
    readonly property var audio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
    readonly property int volPct: audio ? Math.round(audio.volume * 100) : 0
    readonly property var batt: UPower.displayDevice
    readonly property bool hasBatt: batt && batt.isLaptopBattery
    readonly property int battPct: batt ? Math.round(batt.percentage * 100) : 0
    readonly property bool charging: batt && batt.state === UPowerDeviceState.Charging
    property string netKind: ""
    property string netName: ""
    Process {
        id: nmcli
        command: ["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"]
        stdout: StdioCollector {
            onStreamFinished: {
                let k = "", n = "";
                for (const line of text.split("\n")) {
                    const f = line.split(":");
                    if (f[1] === "connected" && (f[0] === "wifi" || f[0] === "ethernet") && !k) { k = f[0]; n = f.slice(2).join(":").replace(/\\(.)/g, "$1"); }
                }
                top.netKind = k; top.netName = n;
            }
        }
    }
    Timer { interval: 10000; running: true; repeat: true; triggeredOnStart: true; onTriggered: nmcli.running = true }
    property bool btPresent: false
    property bool btPowered: false
    property var btConnected: []
    Process {
        id: btProc
        command: ["sh", "-c", "command -v bluetoothctl >/dev/null || exit 3; bluetoothctl --timeout 3 show 2>/dev/null | grep -E '^Controller|Powered:'; echo ---; bluetoothctl --timeout 3 devices Connected 2>/dev/null"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("---");
                top.btPresent = /Controller/.test(parts[0] || "");
                top.btPowered = /Powered:\s*yes/.test(parts[0] || "");
                top.btConnected = (parts[1] || "").split("\n").map(l => l.match(/^Device\s+\S+\s+(.+)$/)).filter(Boolean).map(m => m[1].trim());
            }
        }
    }
    Process { id: btSet; onExited: btProc.running = true }
    Timer { interval: 15000; running: true; repeat: true; triggeredOnStart: true; onTriggered: if (!btProc.running) btProc.running = true }
    readonly property var front: ToplevelManager.activeToplevel
    SystemClock { id: clock; precision: SystemClock.Minutes }

    // The newest notification, shown in the pill for a few seconds (not with Do not disturb).
    property var flash: null
    property int seenSeq: -1
    Connections {
        target: DeskState
        function onHistoryChanged() {
            const n = DeskState.history[0];
            if (!n || n.key === top.seenSeq) return;
            top.seenSeq = n.key;
            if (DeskState.dnd || DeskState.isMuted(n.app)) return;
            top.flash = n;
            flashTimer.restart();
        }
    }
    Timer { id: flashTimer; interval: 5000; onTriggered: top.flash = null }

    // ------------------------------------------------------------------ the band
    Item {
        id: bandItem
        anchors { fill: parent; topMargin: top.gapTop; leftMargin: top.floating ? 12 : top.shelf ? 40 : 0; rightMargin: top.floating ? 12 : top.shelf ? 40 : 0; bottomMargin: top.band - top.gapTop - top.islandH }

        // One full-width bar (Steam-style, Zen): a flat strip attached to the top edge.
        Rectangle {
            anchors.fill: parent
            visible: top.style === "bar"
            color: Theme.glass
            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.line }
            Loader { anchors.fill: parent; active: Theme.frame === "frost" && top.style === "bar"; source: "Frame.qml" }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: top.style === "bar" ? 8 : 0
            anchors.rightMargin: top.style === "bar" ? 6 : 0
            spacing: top.floating ? 10 : 6

            // ---- left island: the mark, the desktops, the window in front
            Island {
                id: leftIsland
                Layout.preferredWidth: leftRow.implicitWidth + (top.floating ? 16 : 4)
                RowLayout {
                    id: leftRow
                    anchors.centerIn: parent
                    spacing: 8 * top.k
                    IslandButton {
                        tip: "Apps (Super)"
                        onClicked: top.launcherRequested()
                        Image { anchors.centerIn: parent; width: 26 * top.k; height: width; source: Qt.resolvedUrl("images/hearth-logo.png"); smooth: true; mipmap: true }
                    }
                    Text {   // Console: the big clock sits next to the mark
                        visible: top.shelf
                        text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : "h:mm")
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 38; font.weight: Font.DemiBold
                        Text { anchors { left: parent.right; leftMargin: 6; baseline: parent.baseline } visible: !Settings.clock24; text: Qt.formatTime(clock.date, "AP"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 }
                    }
                    Loader {
                        id: desks
                        visible: !top.shelf && status === Loader.Ready && !!item && item.wanted
                        active: !top.shelf
                        source: "DeskSwitcher.qml"
                        onLoaded: { item.island = true; item.pillSize = Qt.binding(() => top.islandH > 40 ? 24 : 20); }
                    }
                    RowLayout {   // the window in front
                        visible: !top.compact && !top.shelf && top.front !== null && (top.front.title || "") !== ""
                        spacing: 8
                        Rectangle { visible: desks.visible; implicitWidth: 1; implicitHeight: 22; color: Theme.line }
                        AppIcon { size: 20; appId: top.front ? (top.front.appId || "") : ""; icon: "" }
                        Text {
                            Layout.maximumWidth: 260
                            text: top.front ? (top.front.title || "") : ""
                            elide: Text.ElideRight
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                            font.capitalization: top.gaming ? Font.AllUppercase : Font.MixedCase
                            font.letterSpacing: top.gaming ? 1.5 : 0
                        }
                    }
                }
            }

            // ---- Gaming: the performance island
            Island {
                visible: Theme.perfIsland && !top.compact && perf.status === Loader.Ready
                Layout.preferredWidth: perf.item ? perf.item.implicitWidth + 24 : 0
                Loader { id: perf; anchors.centerIn: parent; active: Theme.perfIsland && !top.compact; source: "PerfIsland.qml" }
            }

            Item { Layout.fillWidth: true }

            // ---- the status island
            Island {
                id: statusIsland
                lit: top.quickOpen
                Layout.preferredWidth: statusRow.implicitWidth + (top.floating ? 16 : 4)
                MouseArea { anchors.fill: parent; onClicked: top.quickRequested() }   // the island's gaps open Quick settings too
                RowLayout {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: top.shelf ? 12 : 2
                    Repeater {
                        model: top.compact && !Theme.hints ? [] : SystemTray.items
                        delegate: IslandButton {
                            id: tray
                            required property SystemTrayItem modelData
                            narrow: !top.shelf
                            tip: (modelData.tooltipTitle || modelData.title || modelData.id || "")
                                + (modelData.tooltipDescription ? "\n" + modelData.tooltipDescription.replace(/<[^>]*>/g, "") : "")
                            onClicked: { if (modelData.onlyMenu && modelData.hasMenu) showMenu(); else modelData.activate(); }
                            onMiddleClicked: modelData.secondaryActivate()
                            onRightClicked: { if (modelData.hasMenu) showMenu(); else modelData.secondaryActivate(); }
                            onScrolled: delta => modelData.scroll(delta, false)
                            function showMenu() { const p = tray.mapToItem(null, 0, tray.height); modelData.display(top, p.x, p.y + 8); }
                            IconImage { anchors.centerIn: parent; implicitSize: 20 * top.k; source: tray.modelData.icon }
                        }
                    }
                    IslandButton {   // Friends
                        visible: !top.compact
                        tip: SettingsNav.friendsUnread > 0 ? "Friends · " + SettingsNav.friendsUnread + " unread" : "Friends"
                        badge: SettingsNav.friendsUnread
                        onClicked: if (SettingsNav.friends) SettingsNav.friends.toggle()
                        Icon { anchors.centerIn: parent; name: "user"; size: 20 * top.k }
                    }
                    IslandButton {   // on-screen keyboard
                        visible: !top.compact
                        tip: "Keyboard"
                        onClicked: top.keyboardRequested()
                        Icon { anchors.centerIn: parent; name: "kbd"; size: 20 * top.k }
                    }
                    IslandButton {   // network
                        id: net
                        tip: top.netName ? "Connected to " + top.netName : "Not connected"
                        onClicked: top.quickRequested()
                        onRightClicked: top.statusMenu(net, net.tip, [
                            { label: "Wi-Fi settings", icon: "wifi", run: () => top.settingsRequested("wifi") },
                            { label: "Quick settings", icon: "menu", run: () => top.quickRequested() }
                        ])
                        Icon { anchors.centerIn: parent; size: 19 * top.k; name: top.netKind === "wifi" ? "wifi" : top.netKind === "ethernet" ? "wired" : "offline" }
                    }
                    IslandButton {   // Bluetooth
                        id: bt
                        visible: top.btPresent && !top.compact
                        tip: !top.btPowered ? "Bluetooth is off" : top.btConnected.length ? "Connected: " + top.btConnected.join(", ") : "Bluetooth is on"
                        onClicked: top.quickRequested()
                        onRightClicked: top.statusMenu(bt, bt.tip, [
                            { label: top.btPowered ? "Turn Bluetooth off" : "Turn Bluetooth on", icon: "bt", run: () => { btSet.command = ["bluetoothctl", "power", top.btPowered ? "off" : "on"]; btSet.running = true; } },
                            { label: "Bluetooth settings", icon: "gear", run: () => top.settingsRequested("bluetooth") }
                        ])
                        Icon { anchors.centerIn: parent; name: "bt"; size: 18 * top.k; color: !top.btPowered ? Theme.muted : top.btConnected.length ? Theme.accent : Theme.text }
                    }
                    IslandButton {   // volume
                        id: vol
                        visible: top.audio !== null
                        tip: (top.audio && top.audio.muted ? "Muted" : "Volume " + top.volPct + "%") + "\nScroll to change, middle-click to mute"
                        onClicked: top.quickRequested()
                        onMiddleClicked: if (top.audio) top.audio.muted = !top.audio.muted
                        onRightClicked: top.soundMenu(vol)
                        onScrolled: delta => { if (top.audio) top.audio.volume = Math.max(0, Math.min(Buttons.maxVolume, top.audio.volume + (delta > 0 ? 0.05 : -0.05))); }
                        Icon { anchors.centerIn: parent; size: 19 * top.k; name: !top.audio || top.audio.muted ? "vol-mute" : top.volPct > 33 ? "vol" : "vol-low" }
                    }
                    IslandButton {   // battery
                        id: battBtn
                        visible: top.hasBatt
                        wideLabel: top.battPct + "%"
                        tip: top.battPct + "%" + (top.charging ? ", charging" : "")
                        onClicked: top.quickRequested()
                        onRightClicked: top.statusMenu(battBtn, battBtn.tip, [{ label: "Power settings", icon: "power", run: () => top.settingsRequested("power") }])
                        Icon { anchors.centerIn: parent; size: 19 * top.k; name: top.charging ? "batt-charging" : top.battPct > 20 ? "batt" : "batt-low" }
                    }
                    IslandButton {   // notifications
                        visible: !top.compact || DeskState.unseen > 0
                        tip: DeskState.dnd ? "Do not disturb is on" : DeskState.unseen > 0 ? DeskState.unseen + (DeskState.unseen === 1 ? " new notification" : " new notifications") : "Notifications"
                        badge: DeskState.dnd ? 0 : DeskState.unseen
                        onClicked: top.calendarRequested()
                        onRightClicked: DeskState.setDnd(!DeskState.dnd)
                        Icon { anchors.centerIn: parent; name: DeskState.dnd ? "bell-off" : "bell"; size: 19 * top.k; color: DeskState.dnd ? Theme.muted : Theme.text }
                    }
                }
            }

            // ---- Game Mode: always here, in every look, the biggest target in the bar
            GameModeButton { Layout.preferredHeight: top.style === "bar" ? top.islandH - 8 : top.islandH }
        }

        // ---- the centre pill: the clock, what's playing, the newest notification
        Island {
            id: pill
            visible: !top.shelf
            lit: top.calendarOpen
            anchors.horizontalCenter: parent.horizontalCenter
            height: parent.height
            width: pillRow.implicitWidth + 28
            rad: Theme.r(height / 2)
            Behavior on width { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: top.calendarRequested() }
            RowLayout {
                id: pillRow
                anchors.centerIn: parent
                spacing: 12
                // the newest notification, for a few seconds
                RowLayout {
                    visible: top.flash !== null
                    spacing: 8
                    AppIcon { size: 22; genericFallback: false; visible: hasIcon; icon: top.flash ? (top.flash.icon || "") : ""; appId: top.flash ? (top.flash.entry || "") : "" }
                    Text {
                        Layout.maximumWidth: top.compact ? 200 : 320
                        text: top.flash ? [top.flash.app, top.flash.summary].filter(Boolean).join(" · ") : ""
                        elide: Text.ElideRight
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                    }
                }
                Loader {
                    id: media
                    visible: top.flash === null && status === Loader.Ready && item.playing
                    source: "MediaPill.qml"
                    onLoaded: item.compact = Qt.binding(() => top.compact)
                }
                Rectangle { visible: media.visible; implicitWidth: 1; implicitHeight: 22; color: Theme.line }
                Text {
                    text: Qt.formatTime(clock.date, Settings.clock24 ? "HH:mm" : "h:mm AP")
                    color: Theme.text
                    font.family: top.gaming || Theme.look === "synthwave" ? "DejaVu Sans Mono" : Theme.font
                    font.pixelSize: Theme.thin ? 13 : 15; font.weight: Font.DemiBold
                    font.letterSpacing: top.gaming ? 1.5 : 0
                }
                Text {
                    visible: !media.visible && top.flash === null && !top.compact
                    text: Qt.formatDate(clock.date, "ddd d MMM")
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
                Rectangle {   // play / pause
                    visible: media.visible
                    implicitWidth: top.islandH - 14; implicitHeight: implicitWidth
                    radius: Theme.r(implicitWidth / 2)
                    color: ppArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
                    Text { anchors.centerIn: parent; text: "❚❚"; color: Theme.accentInk; font.pixelSize: 12 }
                    MouseArea { id: ppArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: if (media.item) media.item.toggle() }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks
    // An island: the look's frame (Frame.qml) when floating; nothing of its own inside a full-width bar.
    component Island: Item {
        property bool lit: false
        property real rad: top.gaming ? 0 : Theme.r(top.islandH / 2.4)
        Layout.fillHeight: true
        Layout.preferredHeight: top.islandH
        Frame { anchors.fill: parent; flat: !top.floating; lit: parent.lit; rad: parent.rad }
    }

    // Back to Game Mode: dressed per look, never hidden.
    component GameModeButton: Item {
        id: gmb
        objectName: "gameModeButton"
        implicitWidth: gmRow.implicitWidth + (top.shelf ? 56 : 36)
        Layout.alignment: Qt.AlignVCenter
        readonly property bool white: Theme.look === "glass"
        Repeater {   // Cabin: an ember glow around it
            model: Theme.look === "cabin" ? 3 : 0
            delegate: Rectangle {
                required property int index
                anchors.fill: parent; anchors.margins: -(index + 1) * 3
                radius: Theme.r(gmb.height / 2) + (index + 1) * 3
                color: "transparent"; border.width: 3
                border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25 - index * 0.07)
            }
        }
        Rectangle {
            anchors.fill: parent
            visible: !top.gaming
            radius: top.style === "bar" ? Theme.r(6) : Theme.r(height / 2)
            color: gmb.white ? (gmArea.containsMouse ? "#ffffff" : "#f2f6ff") : gmArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent
        }
        Rectangle {   // Synthwave: pink chrome
            anchors.fill: parent
            visible: Theme.look === "synthwave"
            radius: Theme.r(height / 2)
            opacity: gmArea.containsMouse ? 0.85 : 1
            gradient: Gradient { GradientStop { position: 0; color: "#ff9bd5" } GradientStop { position: 0.48; color: "#ff3fa4" } GradientStop { position: 0.52; color: "#c81d7e" } GradientStop { position: 1; color: "#ff5fb4" } }
        }
        Canvas {   // Gaming: a cut-corner button, cyan to magenta
            id: gmCanvas
            anchors.fill: parent
            visible: top.gaming
            readonly property string key: Theme.accent + "/" + Theme.flame + "/" + width + "x" + height + "/" + gmArea.containsMouse
            onKeyChanged: requestPaint()
            onVisibleChanged: if (visible) requestPaint()
            onPaint: {
                const c = getContext("2d"), w = width, h = height, k = Math.min(12, h / 3);
                c.reset();
                if (w <= 0 || h <= 0) return;
                const g = c.createLinearGradient(0, 0, w, 0);
                g.addColorStop(0, Theme.css(Theme.accent, 1)); g.addColorStop(1, Theme.css(Theme.flame, 1));
                c.beginPath(); c.moveTo(k, 0); c.lineTo(w, 0); c.lineTo(w, h - k); c.lineTo(w - k, h); c.lineTo(0, h); c.lineTo(0, k); c.closePath();
                c.fillStyle = g; c.fill();
                if (gmArea.containsMouse) { c.fillStyle = "rgba(255,255,255,0.15)"; c.fill(); }
            }
        }
        RowLayout {
            id: gmRow
            anchors.centerIn: parent
            spacing: 8
            Icon { name: "pad"; size: 20 * top.k; color: gmb.white ? "#17203a" : Theme.accentInk }
            Text {
                text: top.gaming ? "GAME MODE" : "Game Mode"
                color: gmb.white ? "#17203a" : Theme.accentInk
                font.family: Theme.font; font.bold: true; font.pixelSize: (Theme.thin ? 12 : 14) * top.k
                font.letterSpacing: top.gaming ? 2 : 0
            }
        }
        MouseArea { id: gmArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Quickshell.execDetached(["nimbus-mode", "game"]) }
    }

    component IslandButton: Item {
        id: btn
        property string tip: ""
        property bool narrow: false
        property int badge: 0
        property string wideLabel: ""
        default property alias content: holder.data
        signal clicked()
        signal middleClicked()
        signal rightClicked()
        signal scrolled(int delta)
        readonly property real side: top.shelf ? 60 : Math.max(26, top.islandH - 12)
        implicitWidth: wideLabel !== "" ? side + wl.implicitWidth + 4 : narrow ? side * 0.8 : side
        implicitHeight: side
        Layout.alignment: Qt.AlignVCenter
        Rectangle {
            anchors.fill: parent
            radius: top.shelf ? height / 2 : Theme.r(10)
            color: area.containsMouse ? Theme.panel3 : top.shelf ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.6) : "transparent"
        }
        Item { id: holder; width: btn.side; height: btn.side }
        Text {
            id: wl
            visible: btn.wideLabel !== ""
            anchors { left: holder.right; verticalCenter: parent.verticalCenter; leftMargin: -6 }
            text: btn.wideLabel; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 * top.k
        }
        Rectangle {
            visible: btn.badge > 0
            x: btn.side / 2 + 2; y: 3
            width: Math.max(16, badgeText.implicitWidth + 8); height: 16; radius: 8
            color: Theme.accent
            border.color: Theme.panel; border.width: 1
            Text { id: badgeText; anchors.centerIn: parent; text: btn.badge > 9 ? "9+" : String(btn.badge); color: Theme.accentInk; font.pixelSize: 10; font.bold: true; font.family: Theme.font }
        }
        MouseArea {
            id: area
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
            onClicked: mouse => {
                if (mouse.button === Qt.MiddleButton) btn.middleClicked();
                else if (mouse.button === Qt.RightButton) btn.rightClicked();
                else btn.clicked();
            }
            onWheel: wheel => btn.scrolled(wheel.angleDelta.y)
        }
        PopupWindow {
            visible: area.containsMouse && btn.tip !== ""
            anchor.item: btn
            anchor.rect.x: btn.width / 2 - implicitWidth / 2
            anchor.rect.y: btn.height + 12
            implicitWidth: tipText.implicitWidth + 20
            implicitHeight: tipText.implicitHeight + 12
            color: "transparent"
            Rectangle {
                anchors.fill: parent
                radius: Theme.r(8)
                color: Theme.panel2
                border.color: Theme.line
                Text { id: tipText; anchors.centerIn: parent; text: btn.tip; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
            }
        }
    }
}
