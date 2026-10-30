// Quick Access (the web page's renderQam): Notifications (what Hearth told you), Now playing (back to the game, exit
// it, its controller layout), Performance (what the PC is doing, frame-rate limit, power limit, power mode, the
// on-screen overlay) and Quick settings (volume, brightness, Wi-Fi and its networks, Bluetooth and its devices,
// airplane mode). The same panel in Hearth's window and over a game (OverlayWindow.qml).
import QtQuick

Item {
    id: qa
    property string scopeName: "qam"
    property bool overlay: false
    property bool open: App.layers.indexOf("qam") >= 0
    property string tab: "now"
    property var perf: null
    property var im: null            // the running game's controller layout (input.get)
    property var imTables: null
    readonly property var tabs: [["notes", "bell", "Notifications"], ["now", "play", "Now playing"], ["perf", "bolt", "Performance"], ["quick", "sliders", "Quick settings"]]
    property var nets: null          // wifi.scan
    property bool scanning: false
    property var paired: null        // bt.paired
    readonly property var game: { App.rev; return App.g(App.running); }
    signal closeRequested()
    visible: panel.x < qa.width - 1

    function refreshPerf() {
        App.call("perf.stats").then(r => { qa.perf = r; if (r && r.runningId !== undefined && !qa.overlay) App.running = r.runningId || ""; }).catch(() => {});
    }
    function loadInput() {
        if (!game || game.cloud) { im = null; return; }
        if (!imTables) App.call("input.tables", {}, 15000).then(t => { qa.imTables = t; }).catch(() => {});
        App.call("input.get", { game: game.id }, 15000).then(r => { qa.im = r; }).catch(() => { qa.im = null; });
    }
    function imLabel() {
        const c = im; if (!c) return "…";
        if (c.layout === "off") return "Off";
        if (c.layout === "custom") return (c.custom && c.custom.name) || "Custom";
        const t = ((imTables && imTables.templates) || []).find(t => t.id === c.layout);
        return t ? t.name : c.layout;
    }
    function pickInput() {
        if (!game || !imTables || !im) { App.toast("Loading…"); loadInput(); return; }
        const id = game.id;
        const set = layout => App.call("input.set", { game: id, layout: layout }, 15000).then(r => { qa.im = r; App.toast(r.layout === "off" ? "Controller layout off" : "Controller layout: " + qa.imLabel()); }).catch(e => App.toast(e.message));
        const items = [{ label: "Off (the game's own controller support)", icon: im.layout === "off" ? "check" : "pad", on: im.layout === "off", run: () => set("off") }];
        for (const t of imTables.templates || []) items.push({ label: t.name, icon: im.layout === t.id ? "check" : "pad", on: im.layout === t.id, run: () => set(t.id) });
        if (im.custom) items.push({ label: (im.custom.name || "Custom") + " (yours)", icon: im.layout === "custom" ? "check" : "pad", on: im.layout === "custom", run: () => set("custom") });
        App.openSheet({ title: "Controller layout", sub: "A layout makes the controller act as a keyboard and mouse in this game, for games without controller support. Making your own is in the web renderer.", items: items });
    }
    onOpenChanged: if (open) { refreshPerf(); loadInput(); App.call("sys.status").then(s => { App.sys = s; }).catch(() => {}); if (tab === "quick") loadRadios(); }
    onTabChanged: if (open && tab === "quick") loadRadios()

    // ---- Wi-Fi and Bluetooth (the web page's wifiScan / wifiNet / loadPaired)
    function radioOn(kind) { const s = App.sys; return !!(s && (kind === "wifi" ? s.wifi && s.wifi.on : kind === "bt" ? s.bt && s.bt.on : s.airplane)); }
    function loadRadios() {
        if (radioOn("wifi") && !scanning) {
            scanning = true;
            App.call("wifi.scan", {}, 60000).then(l => { qa.nets = (l || []).slice(0, 8); }).catch(() => { if (!qa.nets) qa.nets = []; }).then(() => { qa.scanning = false; });
        }
        if (radioOn("bt")) App.call("bt.paired", {}, 15000).then(l => { qa.paired = l || []; }).catch(() => { qa.paired = []; });
    }
    function radio(kind) {
        const p = kind === "air" ? App.call("airplane.set", { on: !radioOn("air") }) : App.call("radios.set", { kind: kind, on: !radioOn(kind) });
        p.then(s => { if (s) App.sys = s; App.later(1500, () => qa.loadRadios()); }).catch(e => App.toast(e.message));
    }
    function wifiConnect(n) {
        App.toast("Connecting to " + n.ssid + "…");
        App.call("wifi.connect", { ssid: n.ssid, password: null }, 60000).then(r => {
            App.toast(r && r.ok ? "Connected to " + n.ssid : ((r && r.error) || "Couldn't connect"));
        }).catch(e => App.toast(e.message)).then(() => App.call("sys.status")).then(s => { if (s) App.sys = s; qa.loadRadios(); });
    }
    function wifiDo(method, args, done) { App.call(method, args || {}, 30000).then(() => App.toast(done)).catch(e => App.toast(e.message)).then(() => App.call("sys.status")).then(s => { if (s) App.sys = s; qa.loadRadios(); }); }
    function pickNet(n) {
        if (n.connected) return App.openSheet({ title: n.ssid, sub: "Connected", items: [
            { label: "Disconnect", icon: "x", run: () => qa.wifiDo("wifi.disconnect", {}, "Disconnected") },
            { label: "Forget network", icon: "trash", danger: true, run: () => qa.wifiDo("wifi.forget", { ssid: n.ssid }, "Forgot " + n.ssid) }] });
        if (n.saved) return App.openSheet({ title: n.ssid, sub: "Saved network", items: [
            { label: "Connect", icon: "wifi", run: () => qa.wifiConnect(n) },
            { label: "Forget network", icon: "trash", danger: true, run: () => qa.wifiDo("wifi.forget", { ssid: n.ssid }, "Forgot " + n.ssid) }] });
        if (!n.secure) return wifiConnect(n);
        App.notHere("Joining a new Wi-Fi network with a password");
    }
    function pickDevice(d) {
        App.openSheet({ title: d.name || d.id, sub: d.connected ? "Connected" : "Paired", items: [
            { label: "Remove (unpair)", icon: "trash", danger: true, run: () => App.call("bt.unpair", { id: d.id }, 20000).then(() => { App.toast("Removed " + (d.name || d.id)); qa.loadRadios(); }).catch(e => App.toast(e.message)) }] });
    }
    onGameChanged: if (open) loadInput()
    Timer { interval: 1000; repeat: true; running: qa.open && qa.tab === "perf"; onTriggered: qa.refreshPerf() }
    Connections {
        target: App
        function onTabStepped(dir) {
            if (App.layerTop() !== "qam" && !qa.overlay) return;
            if (qa.overlay && !qa.open) return;
            const i = qa.tabs.findIndex(t => t[0] === qa.tab);
            qa.tab = qa.tabs[(i + dir + qa.tabs.length) % qa.tabs.length][0];
            App.later(30, () => Nav.reset(qa.scopeName));
        }
    }

    Rectangle { anchors.fill: parent; color: "#000000"; opacity: qa.open && !qa.overlay ? 0.35 : 0; Behavior on opacity { NumberAnimation { duration: Theme.calm } }
        MouseArea { anchors.fill: parent; enabled: qa.open; onClicked: qa.closeRequested() } }

    Rectangle {
        id: panel
        width: Math.min(qa.width, Theme.u(460))
        height: qa.height
        x: qa.open ? qa.width - width : qa.width
        color: Theme.alpha(Theme.bg2, qa.overlay ? 0.97 : 1)
        Behavior on x { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: 1; color: Theme.line }

        Column {
            id: tabCol
            anchors { right: parent.right; top: parent.top; bottom: parent.bottom; topMargin: Theme.u(16); rightMargin: Theme.u(8) }
            width: Theme.u(52)
            spacing: Theme.u(6)
            Repeater {
                model: qa.tabs
                delegate: Btn {
                    required property var modelData
                    kind: "icon"
                    icon: modelData[1]
                    on: qa.tab === modelData[0]
                    navScope: qa.scopeName
                    navKey: "qt-" + modelData[0]
                    width: Theme.u(48); height: Theme.u(48)
                    onActivated: qa.tab = modelData[0]
                }
            }
        }
        Scroller {
            id: body
            anchors { left: parent.left; right: tabCol.left; top: parent.top; bottom: parent.bottom; leftMargin: Theme.u(20); rightMargin: Theme.u(10) }
            contentHeight: content.height + Theme.u(30)
            Column {
                id: content
                width: body.width
                topPadding: Theme.u(18)
                spacing: Theme.u(8)
                Text { text: (qa.tabs.find(t => t[0] === qa.tab) || ["", "", ""])[2]; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(18); font.weight: Font.Bold; bottomPadding: Theme.u(6) }

                // ---- Notifications
                Column {
                    visible: qa.tab === "notes"
                    width: parent.width
                    spacing: Theme.u(8)
                    Text { visible: App.notes.length === 0; width: parent.width; wrapMode: Text.WordWrap; text: "Nothing new. What Hearth tells you shows up here."; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14) }
                    Repeater {
                        model: qa.tab === "notes" ? App.notes : []
                        delegate: Rectangle {
                            required property var modelData
                            width: content.width
                            height: noteCol.implicitHeight + Theme.u(20)
                            radius: Theme.radius
                            color: Theme.panel
                            Column {
                                id: noteCol
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: Theme.u(12) }
                                spacing: Theme.u(3)
                                Text { width: parent.width; wrapMode: Text.WordWrap; text: modelData.text; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(14) }
                                Text { text: qa.ago(modelData.at); color: Theme.faint; font.family: Theme.font; font.pixelSize: Theme.u(11) }
                            }
                        }
                    }
                    Btn { visible: App.notes.length > 0; width: parent.width; navScope: qa.scopeName; navKey: "n-clear"; icon: "x"; text: "Clear all"; onActivated: App.notes = [] }
                }

                // ---- Now playing
                Column {
                    visible: qa.tab === "now"
                    width: parent.width
                    spacing: Theme.u(8)
                    Rectangle {
                        visible: !!qa.game
                        width: parent.width; height: Theme.u(130); radius: Theme.radius; color: Theme.panel2; clip: true
                        ArtImage { anchors.fill: parent; game: qa.game; kinds: ["hero", "wide", "cover"] }
                        Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0.4; color: "transparent" } GradientStop { position: 1; color: Theme.alpha("#000000", 0.75) } } }
                        Column {
                            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.u(12) }
                            Text { text: qa.game ? qa.game.title : ""; color: "#ffffff"; font.family: Theme.font; font.pixelSize: Theme.u(17); font.weight: Font.Bold; elide: Text.ElideRight; width: parent.width }
                            Text { text: qa.game ? App.sourceName(qa.game) + (App.runningSince ? " · playing for " + Math.max(0, Math.round((Date.now() - App.runningSince) / 60000)) + " min" : " · running") : ""; color: "#dfe6ee"; font.family: Theme.font; font.pixelSize: Theme.u(12) }
                        }
                    }
                    Text { visible: !qa.game; width: parent.width; wrapMode: Text.WordWrap; text: "No game from your library is running."; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14) }
                    Btn { visible: !!qa.game; width: parent.width; navScope: qa.scopeName; navKey: "q-resume"; navFirst: true; kind: "primary"; icon: "play"; text: "Back to game"
                          onActivated: { if (qa.overlay) App.send({ type: "resume" }, "overlay"); else { App.closeAll(); App.send({ type: "resume" }); } } }
                    Btn { visible: qa.overlay; width: parent.width; navScope: qa.scopeName; navKey: "q-home"; icon: "home"; text: "Back to Hearth"; onActivated: App.send({ type: "overlay", action: "home" }, "overlay") }
                    Btn { visible: !!qa.game; width: parent.width; navScope: qa.scopeName; navKey: "q-exit"; kind: "danger"; icon: "stop"; text: "Exit game"; onActivated: { if (qa.overlay) qa.closeRequested(); App.exitGame(); } }
                    Heading { visible: !!qa.game && !qa.game.cloud && !!qa.im; text: "Controller layout" }
                    Btn { visible: !!qa.game && !qa.game.cloud && !!qa.im; width: parent.width; navScope: qa.scopeName; navKey: "q-im"; icon: "pad"; text: qa.imLabel(); sub: "Makes the controller a keyboard and mouse in this game"; onActivated: qa.pickInput() }
                    Btn { width: parent.width; navScope: qa.scopeName; navKey: "q-power"; navFirst: !qa.game; icon: "power"; text: "Power"; onActivated: { if (qa.overlay) App.overlayEvent("power.menu", {}); else { App.closeLayer("qam"); App.openLayer("power"); } } }
                }

                // ---- Performance
                Column {
                    visible: qa.tab === "perf"
                    width: parent.width
                    spacing: Theme.u(10)
                    readonly property var p: qa.perf
                    Text { visible: !parent.p; text: "Reading…"; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14) }
                    Stat { visible: !!parent.p; label: "CPU"; value: parent.p ? parent.p.cpu + "%" : ""; frac: parent.p ? parent.p.cpu / 100 : 0 }
                    Stat { visible: !!parent.p; label: "GPU"; value: parent.p && parent.p.gpu != null ? parent.p.gpu + "%" : "—"; frac: parent.p && parent.p.gpu != null ? parent.p.gpu / 100 : 0
                           sub: parent.p && parent.p.vramUsed != null ? parent.p.vramUsed + " GB video memory" : (parent.p && parent.p.gpu == null ? "Not available on this PC" : "") }
                    Stat { visible: !!parent.p; label: "Memory"; value: parent.p ? parent.p.ramUsed + " / " + parent.p.ramTotal + " GB" : ""; frac: parent.p && parent.p.ramTotal ? parent.p.ramUsed / parent.p.ramTotal : 0 }
                    Stat { visible: !!(parent.p && parent.p.battery && parent.p.battery.present !== false && parent.p.battery.pct != null); label: "Battery"
                           value: parent.p && parent.p.battery ? parent.p.battery.pct + "%" + (parent.p.battery.charging ? " ⚡" : "") : ""; frac: parent.p && parent.p.battery ? parent.p.battery.pct / 100 : 0 }
                    Choices {
                        visible: !!(parent.p && parent.p.canFpsLimit)
                        title: "Frame-rate limit"; keyPrefix: "pf-"; sc: qa.scopeName
                        options: [0, 30, 40, 45, 60, 90, 120].map(v => [v, v ? String(v) : "Off"])
                        current: parent.p ? (parent.p.fpsLimit || 0) : 0
                        onPicked: v => App.call("perf.setFpsLimit", { fps: v }).then(r => { qa.refreshPerf(); App.toast(r && r.fpsLimit ? "Limited to " + r.fpsLimit + " FPS" : "Frame-rate limit off"); }).catch(e => App.toast(e.message))
                    }
                    Choices {
                        visible: !!(parent.p && parent.p.tdp && parent.p.tdp.supported)
                        title: "Power limit (TDP)"; keyPrefix: "pt-"; sc: qa.scopeName
                        options: parent.p && parent.p.tdp ? [5, 8, 10, 12, 15, 18, 20, 25, 30].filter(v => v >= parent.p.tdp.min && v <= parent.p.tdp.max).map(v => [v, v + " W"]) : []
                        current: parent.p && parent.p.tdp ? parent.p.tdp.watts : 0
                        onPicked: v => App.call("perf.setTdp", { watts: v }).then(() => qa.refreshPerf()).catch(e => App.toast(e.message))
                    }
                    Choices {
                        visible: !!(parent.p && parent.p.powerMode)
                        title: "Power"; keyPrefix: "pp-"; sc: qa.scopeName
                        options: [["efficiency", "Efficiency"], ["balanced", "Balanced"], ["performance", "Performance"]].filter(m => !parent.p || !parent.p.powerModes || !parent.p.powerModes.length || parent.p.powerModes.indexOf(m[0]) >= 0)
                        current: parent.p ? parent.p.powerMode : ""
                        onPicked: v => App.call("power.setMode", { mode: v }).then(() => qa.refreshPerf()).catch(e => App.toast(e.message))
                    }
                    Choices {
                        visible: !!(parent.p && parent.p.canOverlay)
                        title: "On-screen overlay"; keyPrefix: "po-"; sc: qa.scopeName
                        options: ["Off", "FPS", "Simple", "Detailed", "Everything"].map((l, i) => [i, l])
                        current: parent.p ? (parent.p.perfOverlay || 0) : 0
                        onPicked: v => App.call("perf.setOverlay", { level: v }).then(() => qa.refreshPerf()).catch(e => App.toast(e.message))
                    }
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.faint; font.family: Theme.font; font.pixelSize: Theme.u(12)
                           text: "Handheld Daemon's own power settings (fan curve, lights) are in the web renderer for now." }
                }

                // ---- Quick settings
                Column {
                    visible: qa.tab === "quick"
                    width: parent.width
                    spacing: Theme.u(12)
                    Slider {
                        navScope: qa.scopeName; title: "Volume"; icon: App.sys && App.sys.audio && App.sys.audio.muted ? "mute" : "vol"; keyName: "q-vol"
                        value: App.sys && App.sys.audio ? App.sys.audio.volume : 50
                        onMovedTo: v => App.call("audio.setVolume", { v: v }).then(() => { if (App.sys && App.sys.audio) App.sys.audio.volume = v; }).catch(e => App.toast(e.message))
                    }
                    Slider {
                        visible: !!(App.sys && App.sys.display && App.sys.display.supported)
                        navScope: qa.scopeName; title: "Brightness"; icon: "sun"; keyName: "q-bright"
                        value: App.sys && App.sys.display ? App.sys.display.brightness : 50
                        onMovedTo: v => App.call("display.setBrightness", { v: v }).catch(e => App.toast(e.message))
                    }
                    Btn { width: parent.width; navScope: qa.scopeName; navKey: "q-mute"; icon: "mute"; text: App.sys && App.sys.audio && App.sys.audio.muted ? "Unmute" : "Mute"
                          onActivated: { const m = !(App.sys && App.sys.audio && App.sys.audio.muted); App.call("audio.setMute", { m: m }).then(() => App.call("sys.status")).then(s => { if (s) App.sys = s; }).catch(e => App.toast(e.message)); } }
                    Btn { visible: !!(App.sys && App.sys.wifi && App.sys.wifi.available !== false); width: parent.width; navScope: qa.scopeName; navKey: "q-wifi"; on: qa.radioOn("wifi")
                          icon: qa.radioOn("wifi") ? "wifi" : "wifiOff"; text: "Wi-Fi: " + (qa.radioOn("wifi") ? "on" : "off"); sub: qa.radioOn("wifi") ? ((App.sys.wifi && App.sys.wifi.ssid) || "Not connected") : ""; onActivated: qa.radio("wifi") }
                    Heading { visible: qa.radioOn("wifi"); text: qa.scanning && !qa.nets ? "Looking for networks…" : "Networks" }
                    Repeater {
                        model: qa.radioOn("wifi") && qa.nets ? qa.nets : []
                        delegate: Btn {
                            required property var modelData
                            required property int index
                            width: content.width; navScope: qa.scopeName; navKey: "qn-" + index; icon: "wifi"; text: modelData.ssid || "Hidden network"
                            sub: modelData.connected ? "Connected" : modelData.saved ? "Saved" : modelData.secure ? "Needs a password" : "Open network"
                            on: !!modelData.connected
                            onActivated: qa.pickNet(modelData)
                        }
                    }
                    Btn { visible: !!(App.sys && App.sys.bt && App.sys.bt.available !== false); width: parent.width; navScope: qa.scopeName; navKey: "q-bt"; on: qa.radioOn("bt"); icon: "bt"
                          text: "Bluetooth: " + (qa.radioOn("bt") ? "on" : "off"); onActivated: qa.radio("bt") }
                    Repeater {
                        model: qa.radioOn("bt") && qa.paired ? qa.paired : []
                        delegate: Btn {
                            required property var modelData
                            required property int index
                            width: content.width; navScope: qa.scopeName; navKey: "qb-" + index; icon: modelData.kind === "pad" || /game|controller/i.test(modelData.kind || "") ? "pad" : "bt"
                            text: modelData.name || modelData.id; sub: modelData.connected ? "Connected" : "Paired"; on: !!modelData.connected
                            onActivated: qa.pickDevice(modelData)
                        }
                    }
                    Btn { width: parent.width; navScope: qa.scopeName; navKey: "q-air"; on: qa.radioOn("air"); icon: "plane"; text: "Airplane mode: " + (qa.radioOn("air") ? "on" : "off"); onActivated: qa.radio("air") }
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.faint; font.family: Theme.font; font.pixelSize: Theme.u(12)
                           text: "Pairing new Bluetooth devices, joining a network with a password, friends and open apps are in the web renderer's Quick Access for now." }
                }
            }
        }
    }

    function ago(ms) {
        const m = Math.floor((Date.now() - ms) / 60000);
        return m < 1 ? "Just now" : m < 60 ? m + " min ago" : Math.floor(m / 60) + " h ago";
    }

    component Stat: Column {
        id: st
        property string label: ""
        property string value: ""
        property string sub: ""
        property real frac: 0
        width: parent ? parent.width : 0
        spacing: Theme.u(5)
        Row {
            width: parent.width
            Text { text: st.label; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13); width: parent.width / 2 }
            Text { text: st.value; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(13); width: parent.width / 2; horizontalAlignment: Text.AlignRight }
        }
        Bar { width: parent.width; value: st.frac }
        Text { visible: st.sub !== ""; text: st.sub; color: Theme.faint; font.family: Theme.font; font.pixelSize: Theme.u(11) }
    }
    component Choices: Column {
        id: ch
        property string title: ""
        property var options: []
        property var current: null
        property string keyPrefix: ""
        property string sc: "qam"
        signal picked(var v)
        width: parent ? parent.width : 0
        spacing: Theme.u(6)
        Text { text: ch.title; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(14); font.weight: Font.DemiBold; topPadding: Theme.u(6) }
        Flow {
            width: parent.width
            spacing: Theme.u(6)
            Repeater {
                model: ch.options
                delegate: Chip {
                    required property var modelData
                    text: modelData[1]
                    on: ch.current === modelData[0]
                    navScope: ch.sc
                    navKey: ch.keyPrefix + modelData[0]
                    onActivated: ch.picked(modelData[0])
                }
            }
        }
    }
    component Slider: Focusable {
        id: sl
        property string title: ""
        property string icon: "vol"
        property string keyName: ""
        property real value: 50
        property real shown: value
        signal movedTo(int v)
        onValueChanged: shown = value
        width: parent ? parent.width : 0
        height: Theme.u(64)
        navKey: keyName
        navMove: function (dir) {
            if (dir !== "left" && dir !== "right") return false;
            sl.shown = Math.max(0, Math.min(100, Math.round(sl.shown + (dir === "right" ? 5 : -5))));
            send.restart();
            return true;
        }
        Timer { id: send; interval: 120; onTriggered: sl.movedTo(sl.shown) }
        Rectangle { anchors.fill: parent; radius: sl.radius; color: sl.isFocused || sl.hovered ? Theme.panel2 : Theme.panel }
        Row {
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(12) }
            spacing: Theme.u(8)
            Icon { name: sl.icon; size: Theme.u(18); color: Theme.dim }
            Text { text: sl.title; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(14) }
        }
        Text { anchors { right: parent.right; top: parent.top; margins: Theme.u(12) } text: Math.round(sl.shown) + "%"; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13) }
        Bar { anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.u(14) } value: sl.shown / 100 }
        MouseArea {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.u(6) }
            height: Theme.u(24)
            onPressed: mouse => { sl.shown = Math.round(Math.max(0, Math.min(1, (mouse.x - Theme.u(8)) / (width - Theme.u(16)))) * 100); send.restart(); }
            onPositionChanged: mouse => { if (pressed) { sl.shown = Math.round(Math.max(0, Math.min(1, (mouse.x - Theme.u(8)) / (width - Theme.u(16)))) * 100); send.restart(); } }
        }
    }
}
