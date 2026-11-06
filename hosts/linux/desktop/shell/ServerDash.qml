// Server mode's dashboard (docs/SERVER-MODE.md): the one full-screen window of the Server mode session (server-mode.qml).
// Status, Play (Game Mode), every service as a tile, alerts; a tile opens to its details; Play or Guide counts down 5
// seconds (A switches now, B cancels) showing what pauses and what keeps working (Light mode's own rules); a switch asked
// for from the web page or the schedule counts down 10 seconds here (B cancels it). Back from Game Mode, "Back in Server
// mode" says what's coming back. With nothing pressed the screen dims to a clock, then goes black (and off with wlopm).
// Everything goes through hearth-server-mode; nothing here writes a setting.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: dash
    property var settings: null                 // Hearth Settings (SettingsApp.qml), opened from the menu
    signal padSend(var msg)
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-server-mode").toString().replace(/^file:\/\//, ""))
    readonly property string run: Quickshell.env("XDG_RUNTIME_DIR") || "/tmp"
    property var d: null
    property int sel: 0                         // 0: Play, 1..8: the tiles
    property string overlay: ""                 // "" | "switch" | "detail" | "menu" | "back"
    property int menuSel: 0
    property int detailSel: 0
    property int count: 5                       // the Play countdown
    property var remote: null                   // the web page's / schedule's countdown (countdown.json)
    property var backInfo: null
    property string toast: ""
    property double lastInput: Date.now()
    property double lastBusy: Date.now()
    property bool idlePowerOn: false
    property string screenState: "on"           // on | dim | off
    readonly property real k: Math.max(0.55, Math.min(width / 1920, height / 1080))
    readonly property var tiles: d && d.tiles ? d.tiles : []
    readonly property var cur: sel > 0 && sel <= tiles.length ? tiles[sel - 1] : null

    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    color: Theme.bg
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-server-mode"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    // ---------------------------------------------------------------- data
    function refresh() { if (!reader.running) reader.running = true; }
    Process {
        id: reader
        command: ["python3", dash.helper, "dash"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { const v = JSON.parse(text); dash.d = v; if (v.busy) dash.lastBusy = Date.now(); } catch (e) { console.warn("server mode: dash: " + e); }
            }
        }
    }
    Timer { interval: 4000; running: true; repeat: true; triggeredOnStart: true; onTriggered: dash.refresh() }
    // The countdown asked for from the web page or the schedule (the agent counts; this shows it and can cancel it).
    FileView {
        id: cdFile
        path: dash.run + "/hearth-server/countdown.json"
        onLoaded: { try { const c = JSON.parse(text()); dash.remote = c && c.action ? c : null; if (dash.remote) dash.wake(); } catch (e) { dash.remote = null; } }
        onLoadFailed: dash.remote = null
    }
    property double nowMs: Date.now()
    Timer { interval: 1000; running: true; repeat: true; onTriggered: { dash.nowMs = Date.now(); cdFile.reload(); dash.idleTick(); } }
    readonly property int remoteLeft: remote ? Math.max(0, Math.ceil(remote.deadline - nowMs / 1000)) : 0

    function act(args, ok) {
        actor.okText = ok || "";
        actor.command = ["python3", helper].concat(args);
        actor.running = true;
    }
    Process {
        id: actor
        property string okText: ""
        stdout: StdioCollector {
            onStreamFinished: {
                const t = text.trim();
                if (t.startsWith("error: ")) dash.say(t.slice(7));
                else if (actor.okText !== "") dash.say(actor.okText);
                dash.refresh();
            }
        }
    }
    function say(t) { toast = t; toastTimer.restart(); }
    Timer { id: toastTimer; interval: 4500; onTriggered: dash.toast = "" }

    Component.onCompleted: {
        // nimbus-session waits for this mark: without it, labwc is closed and Game Mode opens.
        Quickshell.execDetached(["sh", "-c", ': >"$0"; mkdir -p -m 700 "$1"; [ -e "$1/countdown.json" ] || echo "{}" >"$1/countdown.json"', dash.run + "/hearth-server-dash-up", dash.run + "/hearth-server"]);
        backReader.running = true;
    }
    Process {
        id: backReader
        command: ["python3", dash.helper, "back"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const v = JSON.parse(text);
                    if (v.back) {
                        dash.backInfo = v; dash.overlay = "back"; backTimer.restart();
                        Quickshell.execDetached(["python3", dash.helper, "nudge"]);   // Light mode resumes now, not in a minute
                    }
                } catch (e) {}
            }
        }
    }
    Timer { id: backTimer; interval: 90000; onTriggered: if (dash.overlay === "back") dash.overlay = "" }

    // ---------------------------------------------------------------- idle: dim, black, lower power limits
    function wake() {
        lastInput = Date.now();
        if (screenState !== "on") {
            if (screenState === "off") Quickshell.execDetached(["sh", "-c", 'command -v wlopm >/dev/null && wlopm --on "*"']);
            screenState = "on";
        }
    }
    function idleTick() {
        if (!d || !d.idle) return;
        const idleMin = (Date.now() - lastInput) / 60000;
        if (remote || overlay === "switch") return;
        if (d.idle.off > 0 && idleMin >= d.idle.off && screenState !== "off") {
            screenState = "off";
            Quickshell.execDetached(["sh", "-c", 'command -v wlopm >/dev/null && wlopm --off "*"']);
        } else if (d.idle.dim > 0 && idleMin >= d.idle.dim && screenState === "on") {
            screenState = "dim";
        }
        const quietMin = (Date.now() - lastBusy) / 60000;
        if (d.idle.idlePower && !idlePowerOn && quietMin >= d.idle.idleMinutes && !d.busy) { idlePowerOn = true; act(["idle-power", "on"]); }
        else if (idlePowerOn && d.busy) { idlePowerOn = false; act(["idle-power", "off"]); }
    }

    // ---------------------------------------------------------------- switching
    function play() {
        if (overlay === "switch") return;
        overlay = "switch"; count = 5; countTimer.restart();
    }
    Timer {
        id: countTimer
        interval: 1000; repeat: true
        onTriggered: { dash.count -= 1; if (dash.count <= 0) { stop(); dash.switchNow(); } }
    }
    function switchNow() {
        countTimer.stop();
        say("Switching to Game Mode…");
        act(["go", "game"]);
    }
    function cancelSwitch() { countTimer.stop(); overlay = ""; }
    function goMode(m) { overlay = ""; say(m === "desktop" ? "Opening Desktop Mode…" : "Switching…"); act(["go", m]); }
    function remoteNow() { if (remote && remote.action) { const a = remote.action; act(["cancel"]); if (a.kind === "mode") act(["go", a.to]); else act(["power", a.what]); } }
    function remoteCancel() { act(["cancel"], "Cancelled"); remote = null; }

    readonly property var pauseList: {
        const p = d && d.pauses ? d.pauses : {};
        const o = [];
        if (p.picturesOn && p.pictures === "stop") o.push({ icon: "image", t: "Pictures stop", s: "After the current picture. The pictures waiting stay in the queue." });
        if (p.aiOn && p.ai === "small") o.push({ icon: "chat", t: "AI chat switches to " + p.small, s: "On the processor, so your devices still get answers. " + p.big + " loads again when you're back." });
        else if (p.aiOn && p.ai === "unload") o.push({ icon: "chat", t: "AI chat unloads its model", s: "Frees graphics memory. The next question loads it again." });
        else if (p.aiOn && p.ai === "stop") o.push({ icon: "chat", t: "AI chat stops", s: "Questions wait until you're back." });
        if (p.voicesOn && p.voices === "pause") o.push({ icon: "wave", t: "Voices pause", s: "Kokoro starts again when you're back." });
        if (p.lightMode) o.push({ icon: "cloud", t: "Backups wait", s: "A backup that's due starts when you stop playing." });
        if (p.mediaOn && p.media === "pause") o.push({ icon: "film", t: "Media pauses", s: "Jellyfin starts again when you're back." });
        return o;
    }
    readonly property var keepList: {
        const p = d && d.pauses ? d.pauses : {};
        const o = [{ icon: "folder", t: "Shared folders", s: "They stay open on your devices." }];
        if (p.mediaOn && p.media !== "pause") o.push({ icon: "film", t: "Media", s: "Watching keeps going. Converting uses the processor." });
        o.push({ icon: "tailscale", t: "Tailscale and the web page", s: "You can still check on the PC from your phone." });
        o.push({ icon: "desktops", t: "Remote desktop", s: "Stream your game to another device if you like." });
        if (p.aiOn && p.ai === "keep") o.push({ icon: "chat", t: "AI chat", s: "Keeps its model loaded on the graphics card." });
        return o;
    }

    // ---------------------------------------------------------------- the controller and the keyboard
    readonly property var menuItems: [
        { icon: "pad", t: "Game Mode", s: "Play: Steam on the big screen (5-second countdown)", a: () => { dash.overlay = ""; dash.play(); } },
        { icon: "desktops", t: "Desktop Mode", s: "The Mantel desktop with windows, files and apps", a: () => dash.goMode("desktop") },
        { icon: "gear", t: "Server mode settings", s: "Power when idle, what pauses, the web page", a: () => { dash.overlay = ""; if (dash.settings) dash.settings.open("hs-server"); } },
        { icon: "gear", t: "Hearth Settings", s: "Everything else", a: () => { dash.overlay = ""; if (dash.settings) dash.settings.open(""); } },
        { icon: "moon", t: "Screen off", s: "Any button turns it on again", a: () => { dash.overlay = ""; dash.screenState = "off"; Quickshell.execDetached(["sh", "-c", 'command -v wlopm >/dev/null && wlopm --off "*"']); } },
        { icon: "sleep", t: "Sleep", s: "Your devices can't reach the PC while it sleeps", a: () => { dash.overlay = ""; dash.act(["power", "sleep"]); } },
        { icon: "restart", t: "Restart the PC", s: "Server mode comes back if it's your Start in mode", a: () => { dash.overlay = ""; dash.act(["power", "restart"]); } },
        { icon: "power", t: "Turn off", s: "Everything stops until you turn it on", a: () => { dash.overlay = ""; dash.act(["power", "off"]); } }
    ]
    function serviceAct(t, a) {
        if (!t || !t.actions || t.actions.indexOf(a === "restart" ? "restart" : "pause") < 0) { say("Nothing to " + a + " here"); return; }
        if (a === "pause") a = t.state === "paused-you" ? "resume" : "pause";
        act(["service", a, t.id], t.title + ": " + (a === "restart" ? "restarting" : a === "pause" ? "paused" : "resuming"));
    }
    function key(e) {
        const wasAsleep = screenState !== "on";
        wake();
        if (wasAsleep) { e.accepted = true; return; }
        const K = e.key;
        const enter = K === Qt.Key_Return || K === Qt.Key_Enter || K === Qt.Key_Space;
        const back = K === Qt.Key_Escape || K === Qt.Key_Backspace;
        const x = K === Qt.Key_Menu || K === Qt.Key_X || K === Qt.Key_R;
        const y = K === Qt.Key_Y || K === Qt.Key_P;
        const menu = K === Qt.Key_M || K === Qt.Key_F1;
        const playKey = K === Qt.Key_G || K === Qt.Key_Home || K === Qt.Key_F12;
        e.accepted = true;
        if (remote) { if (enter) remoteNow(); else if (back) remoteCancel(); return; }
        if (overlay === "switch") { if (enter) switchNow(); else if (back) cancelSwitch(); return; }
        if (overlay === "back") { overlay = ""; return; }
        if (overlay === "menu") {
            if (K === Qt.Key_Down) menuSel = (menuSel + 1) % menuItems.length;
            else if (K === Qt.Key_Up) menuSel = (menuSel + menuItems.length - 1) % menuItems.length;
            else if (enter) menuItems[menuSel].a();
            else if (back || menu) overlay = "";
            return;
        }
        if (overlay === "detail") {
            if (back) overlay = "";
            else if (x) serviceAct(cur, "restart");
            else if (y) serviceAct(cur, "pause");
            else if (enter && cur && cur.id === "ai") act(["ai-unload"]);
            return;
        }
        if (playKey) return play();
        if (menu) { overlay = "menu"; menuSel = 0; return; }
        if (enter) { if (sel === 0) play(); else if (cur) { overlay = "detail"; } return; }
        if (x) return serviceAct(cur, "restart");
        if (y) return serviceAct(cur, "pause");
        const n = tiles.length;
        if (K === Qt.Key_Right) sel = sel === 0 ? 0 : Math.min(n, sel + 1);
        else if (K === Qt.Key_Left) sel = sel === 0 ? 4 : Math.max(1, sel - 1);
        else if (K === Qt.Key_Down) sel = sel === 0 ? 4 : Math.min(n, sel + 4);
        else if (K === Qt.Key_Up) sel = sel <= 4 ? 0 : sel - 4;
        else e.accepted = false;
    }
    // From hearth-pad (server-mode.qml): Guide is Play, ☰ the menu, Y pause / resume.
    function pad(ev) {
        const wasAsleep = screenState !== "on";
        wake();
        if (wasAsleep) return;
        if (ev === "guide") { if (remote) remoteCancel(); else if (overlay === "switch") cancelSwitch(); else play(); }
        else if (ev === "menu") { overlay = overlay === "menu" ? "" : "menu"; menuSel = 0; }
        else if (ev === "y") { if (overlay === "" || overlay === "detail") serviceAct(cur, "pause"); }
    }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: e => dash.key(e)
    }
    MouseArea { anchors.fill: parent; hoverEnabled: true; onPositionChanged: dash.wake(); onClicked: dash.wake(); z: -1 }

    // ---------------------------------------------------------------- the dashboard
    Item {
        anchors.fill: parent
        visible: dash.screenState === "on"
        // Top line: the PC's name, Server mode, the clock.
        RowLayout {
            id: top
            anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 48 * dash.k; rightMargin: 48 * dash.k; topMargin: 20 * dash.k }
            height: 34 * dash.k
            spacing: 14 * dash.k
            Icon { name: "server"; size: 26 * dash.k; color: Theme.accent }
            Text { text: dash.d ? dash.d.host : "Hearth"; color: Theme.text; font.family: Theme.font; font.pixelSize: 21 * dash.k; font.weight: Font.DemiBold }
            Rectangle {
                implicitWidth: modeRow.implicitWidth + 24 * dash.k; height: 30 * dash.k; radius: 15 * dash.k
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
                Row { id: modeRow; anchors.centerIn: parent; spacing: 6 * dash.k
                    Icon { name: "server"; size: 15 * dash.k; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: "Server mode"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14 * dash.k; font.weight: Font.DemiBold } }
            }
            Item { Layout.fillWidth: true }
            Icon { name: "tailscale"; size: 20 * dash.k; color: Theme.muted }
            Icon { name: "wired"; size: 20 * dash.k; color: Theme.muted }
            Text { text: Qt.formatTime(new Date(dash.nowMs), "h:mm AP"); color: Theme.text; font.family: Theme.font; font.pixelSize: 19 * dash.k; font.weight: Font.DemiBold }
            Text { text: Qt.formatDate(new Date(dash.nowMs), "ddd d MMM"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 16 * dash.k }
        }
        // Status and Play.
        RowLayout {
            id: hero
            anchors { left: parent.left; right: parent.right; top: top.bottom; leftMargin: 48 * dash.k; rightMargin: 48 * dash.k; topMargin: 22 * dash.k }
            height: 168 * dash.k
            spacing: 14 * dash.k
            Rectangle {
                id: heroCard
                Layout.fillWidth: true; Layout.fillHeight: true
                radius: 18 * dash.k
                readonly property color sc: dash.d && dash.d.status && dash.d.status.level !== "ok" ? SetStyle.warn : SetStyle.ok
                gradient: Gradient { orientation: Gradient.Horizontal
                    GradientStop { position: 0; color: Qt.rgba(heroCard.sc.r, heroCard.sc.g, heroCard.sc.b, 0.16) }
                    GradientStop { position: 0.6; color: SetStyle.card } }
                border.color: Theme.line
                RowLayout {
                    anchors.fill: parent; anchors.margins: 26 * dash.k
                    spacing: 26 * dash.k
                    Rectangle {
                        width: 104 * dash.k; height: width; implicitWidth: 104 * dash.k; implicitHeight: 104 * dash.k; radius: width / 2
                        color: Qt.rgba(heroCard.sc.r, heroCard.sc.g, heroCard.sc.b, 0.18)
                        Icon { anchors.centerIn: parent; name: dash.d && dash.d.status && dash.d.status.level !== "ok" ? "warn" : "check"; size: 54 * dash.k; color: heroCard.sc }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 6 * dash.k
                        Row { spacing: 12 * dash.k
                            Text { id: statusTitle; text: dash.backInfo && dash.overlay === "back" ? "Coming back" : (dash.d ? dash.d.status.title : "Starting…"); color: Theme.text; font.family: Theme.font; font.pixelSize: 44 * dash.k; font.weight: Font.Bold }
                            Text { anchors.baseline: statusTitle.baseline; text: dash.d ? "· " + dash.d.status.sub : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 40 * dash.k } }
                        Row { spacing: 22 * dash.k
                            Text { text: dash.d ? "<b>" + dash.d.devices + (dash.d.devices === 1 ? " device</b> connected" : " devices</b> connected") : ""; textFormat: Text.StyledText; color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 * dash.k }
                            Text { text: dash.d ? "Up <b>" + Math.floor(dash.d.uptime / 86400) + " days</b>" : ""; textFormat: Text.StyledText; color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 * dash.k; visible: !!dash.d && dash.d.uptime >= 86400 }
                            Text { text: dash.d ? "Server mode since <b>" + dash.d.sinceText + "</b>" : ""; textFormat: Text.StyledText; color: Theme.muted; font.family: Theme.font; font.pixelSize: 18 * dash.k } }
                    }
                    Repeater {
                        model: dash.d ? dash.d.facts : []
                        delegate: Column {
                            required property var modelData
                            spacing: 2 * dash.k
                            Text { text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * dash.k }
                            Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 22 * dash.k; font.weight: Font.DemiBold }
                        }
                    }
                }
            }
            Rectangle {
                id: playCard
                Layout.preferredWidth: 436 * dash.k; Layout.fillHeight: true
                radius: 18 * dash.k
                color: Theme.accent
                border.width: dash.sel === 0 ? 4 * dash.k : 0
                border.color: Qt.lighter(Theme.accent, 1.4)
                RowLayout {
                    anchors.fill: parent; anchors.margins: 30 * dash.k; spacing: 22 * dash.k
                    Rectangle { width: 84 * dash.k; height: width; implicitWidth: 84 * dash.k; implicitHeight: 84 * dash.k; radius: width / 2; color: Qt.rgba(0, 0, 0, 0.12)
                        Icon { anchors.centerIn: parent; name: "pad"; size: 44 * dash.k; color: Theme.accentInk } }
                    ColumnLayout { spacing: 2 * dash.k; Layout.fillWidth: true
                        Text { text: "Play"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 38 * dash.k; font.weight: Font.Bold }
                        Text { text: "Switch to Game Mode"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 16 * dash.k } }
                }
                Text { anchors { right: parent.right; top: parent.top; margins: 18 * dash.k } text: "or press Guide"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 13 * dash.k; font.weight: Font.DemiBold }
                MouseArea { anchors.fill: parent; onClicked: { dash.sel = 0; dash.play(); } }
            }
        }
        // Every service in a word.
        Flow {
            id: pills
            anchors { left: parent.left; right: parent.right; top: hero.bottom; leftMargin: 48 * dash.k; rightMargin: 48 * dash.k; topMargin: 16 * dash.k }
            spacing: 10 * dash.k
            Repeater {
                model: dash.d ? dash.d.pills : []
                delegate: Rectangle {
                    required property var modelData
                    height: 44 * dash.k; radius: 22 * dash.k
                    width: pillRow.implicitWidth + 32 * dash.k
                    color: SetStyle.card; border.color: Theme.line
                    Row { id: pillRow; anchors.centerIn: parent; spacing: 10 * dash.k
                        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 9 * dash.k; height: width; implicitWidth: 9 * dash.k; implicitHeight: 9 * dash.k; radius: width / 2
                            color: modelData.state === "running" ? SetStyle.ok : modelData.state === "off" ? Theme.muted : SetStyle.warn }
                        Text { text: modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 * dash.k }
                        Text { text: modelData.extra || ""; visible: text !== ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 * dash.k; anchors.verticalCenter: parent.verticalCenter } }
                }
            }
        }
        // The tiles: four across, two down.
        GridLayout {
            id: grid
            anchors { left: parent.left; right: parent.right; top: pills.bottom; bottom: alerts.top; leftMargin: 48 * dash.k; rightMargin: 48 * dash.k; topMargin: 16 * dash.k; bottomMargin: 16 * dash.k }
            columns: 4; rowSpacing: 16 * dash.k; columnSpacing: 16 * dash.k
            Repeater {
                model: dash.tiles
                delegate: ServerTile {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true; Layout.fillHeight: true
                    Layout.preferredWidth: 1; Layout.preferredHeight: 1
                    t: modelData; k: dash.k; focused: dash.sel === index + 1
                    MouseArea { anchors.fill: parent; onClicked: { dash.sel = index + 1; dash.overlay = "detail"; } }
                }
            }
        }
        // Alerts: up to three.
        RowLayout {
            id: alerts
            anchors { left: parent.left; right: parent.right; bottom: hints.top; leftMargin: 48 * dash.k; rightMargin: 48 * dash.k; bottomMargin: 14 * dash.k }
            height: 62 * dash.k
            spacing: 14 * dash.k
            Repeater {
                model: dash.d && dash.d.alerts.length ? dash.d.alerts : [{ level: "ok", text: "Nothing needs a look", sub: "", at: "" }]
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1
                    radius: 12 * dash.k
                    readonly property color c: modelData.level === "warn" ? SetStyle.warn : modelData.level === "ok" ? SetStyle.ok : Theme.accent
                    color: modelData.level === "warn" ? Qt.rgba(c.r, c.g, c.b, 0.12) : SetStyle.card
                    border.color: modelData.level === "warn" ? Qt.rgba(c.r, c.g, c.b, 0.45) : Theme.line
                    RowLayout { anchors.fill: parent; anchors.leftMargin: 20 * dash.k; anchors.rightMargin: 20 * dash.k; spacing: 14 * dash.k
                        Icon { name: modelData.level === "warn" ? "warn" : modelData.level === "ok" ? "check" : "globe"; size: 22 * dash.k; color: parent.parent.c }
                        Text { Layout.fillWidth: true; text: modelData.text + (modelData.sub ? "<font color='" + Theme.muted + "'> · " + modelData.sub + "</font>" : ""); textFormat: Text.StyledText; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 * dash.k; elide: Text.ElideRight }
                        Text { text: modelData.at || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 * dash.k } }
                }
            }
        }
        // The buttons along the bottom.
        Rectangle {
            id: hints
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: 52 * dash.k
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.9)
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 48 * dash.k; anchors.rightMargin: 48 * dash.k; spacing: 26 * dash.k
                Repeater {
                    model: [["A", "Open"], ["X", "Restart service"], ["Y", "Pause or resume"], ["☰", "Settings"]]
                    delegate: Row { required property var modelData; spacing: 8 * dash.k
                        Rectangle { width: 24 * dash.k; height: width; implicitWidth: 24 * dash.k; implicitHeight: 24 * dash.k; radius: width / 2; border.color: Theme.muted; color: "transparent"
                            Text { anchors.centerIn: parent; text: modelData[0]; color: Theme.text; font.pixelSize: 12 * dash.k; font.weight: Font.Bold } }
                        Text { text: modelData[1]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * dash.k } }
                }
                Item { Layout.fillWidth: true }
                Text { visible: dash.toast !== ""; text: dash.toast; color: Theme.flame; font.family: Theme.font; font.pixelSize: 15 * dash.k }
                Item { Layout.fillWidth: true }
                Text { text: "Guide  Play (Game Mode)"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * dash.k }
            }
        }

        // ------------------------------------------------------------ overlays
        Rectangle { anchors.fill: parent; visible: dash.overlay !== "" && dash.overlay !== "back" || !!dash.remote; color: Qt.rgba(0.02, 0.035, 0.055, 0.78) }

        // The countdown: Play / Guide (5 s, here) or the web page / schedule (10 s, the agent's).
        Rectangle {
            id: sw
            visible: dash.overlay === "switch" || !!dash.remote
            anchors.centerIn: parent
            width: 1240 * dash.k; height: swCol.implicitHeight + 72 * dash.k
            radius: 22 * dash.k; color: Theme.panel; border.color: Theme.line
            readonly property bool isRemote: !!dash.remote
            readonly property int secsLeft: isRemote ? dash.remoteLeft : dash.count
            readonly property int total: isRemote ? (dash.remote.total || 10) : 5
            readonly property string toName: isRemote ? (dash.remote.what === "power" ? (dash.remote.to === "sleep" ? "SLEEP" : "RESTART") : dash.remote.to === "desktop" ? "DESKTOP MODE" : "GAME MODE") : "GAME MODE"
            ColumnLayout {
                id: swCol
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 40 * dash.k }
                spacing: 26 * dash.k
                RowLayout {
                    spacing: 40 * dash.k
                    Item {
                        width: 200 * dash.k; height: width; implicitWidth: 200 * dash.k; implicitHeight: 200 * dash.k
                        Canvas {
                            id: ring
                            anchors.fill: parent
                            property real p: sw.total > 0 ? sw.secsLeft / sw.total : 0
                            onPChanged: requestPaint()
                            onPaint: {
                                const c = getContext("2d"); c.reset();
                                const r = width / 2 - 8 * dash.k;
                                c.lineWidth = 12 * dash.k; c.lineCap = "round";
                                c.strokeStyle = "" + SetStyle.card2; c.beginPath(); c.arc(width / 2, height / 2, r, 0, Math.PI * 2); c.stroke();
                                c.strokeStyle = "" + Theme.accent; c.beginPath(); c.arc(width / 2, height / 2, r, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * p); c.stroke();
                            }
                        }
                        Column { anchors.centerIn: parent
                            Text { anchors.horizontalCenter: parent.horizontalCenter; text: String(sw.secsLeft); color: Theme.text; font.family: Theme.font; font.pixelSize: 72 * dash.k; font.weight: Font.DemiBold }
                            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "seconds"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * dash.k } }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 8 * dash.k
                        Text { text: (sw.isRemote && dash.remote.what === "power" ? "" : "SWITCHING TO ") + sw.toName + (sw.isRemote ? "  ·  ASKED FOR " + (dash.remote.from === "schedule" ? "BY THE SCHEDULE" : "FROM " + (dash.remote.by || "THE WEB PAGE").toUpperCase()) : ""); color: Theme.accent; font.family: Theme.font; font.pixelSize: 15 * dash.k; font.weight: Font.DemiBold; font.letterSpacing: 0.4 }
                        Text { text: sw.isRemote && dash.remote.what === "power" ? (dash.remote.to === "sleep" ? "The PC is going to sleep" : "The PC is restarting") : sw.toName === "DESKTOP MODE" ? "Opening Desktop Mode" : "Getting the PC ready to play"; color: Theme.text; font.family: Theme.font; font.pixelSize: 40 * dash.k; font.weight: Font.DemiBold }
                        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Big jobs pause so your game gets the graphics card. They come back by themselves when you return to Server mode."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 17 * dash.k }
                    }
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2; columnSpacing: 28 * dash.k; rowSpacing: 10 * dash.k
                    Text { text: "Pauses while you play"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 * dash.k; font.weight: Font.DemiBold; Layout.fillWidth: true }
                    Text { text: "Keeps working"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 * dash.k; font.weight: Font.DemiBold; Layout.fillWidth: true }
                    ColumnLayout { Layout.alignment: Qt.AlignTop; Layout.fillWidth: true; Layout.preferredWidth: 1; spacing: 10 * dash.k
                        Repeater { model: dash.pauseList; delegate: ServerLine { required property var modelData; Layout.fillWidth: true; k: dash.k; icon: modelData.icon; title: modelData.t; sub: modelData.s; tone: SetStyle.warn } }
                        Text { visible: dash.pauseList.length === 0; text: "Nothing: Light mode is off or nothing big is on"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * dash.k } }
                    ColumnLayout { Layout.alignment: Qt.AlignTop; Layout.fillWidth: true; Layout.preferredWidth: 1; spacing: 10 * dash.k
                        Repeater { model: dash.keepList; delegate: ServerLine { required property var modelData; Layout.fillWidth: true; k: dash.k; icon: modelData.icon; title: modelData.t; sub: modelData.s; tone: SetStyle.ok } } }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 14 * dash.k
                    Text { text: "The same rules as Light mode · change them in Settings › Home server › Server mode"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 * dash.k; Layout.fillWidth: true; elide: Text.ElideRight }
                    ServerButton { k: dash.k; label: "Cancel"; keyName: "B"; onClicked: sw.isRemote ? dash.remoteCancel() : dash.cancelSwitch() }
                    ServerButton { k: dash.k; label: sw.isRemote && dash.remote.what === "power" ? "Now" : "Switch now"; keyName: "A"; primary: true; onClicked: sw.isRemote ? dash.remoteNow() : dash.switchNow() }
                }
            }
        }

        // A tile opened: its details and what you can do.
        Rectangle {
            visible: dash.overlay === "detail" && !!dash.cur && !dash.remote
            anchors.centerIn: parent
            width: 1620 * dash.k; height: 860 * dash.k
            radius: 22 * dash.k; color: Theme.panel; border.color: Theme.line
            ServerDetail { anchors.fill: parent; t: dash.cur; d: dash.d; k: dash.k
                onAction: a => a === "unload" ? dash.act(["ai-unload"], "Model unloaded") : dash.serviceAct(dash.cur, a)
                onClose: dash.overlay = "" }
        }

        // The menu (☰): Game Mode, Desktop Mode, Settings, the screen, sleep and restart.
        Rectangle {
            visible: dash.overlay === "menu" && !dash.remote
            anchors.centerIn: parent
            width: 640 * dash.k; height: menuCol.implicitHeight + 40 * dash.k
            radius: 22 * dash.k; color: Theme.panel; border.color: Theme.line
            Column {
                id: menuCol
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 * dash.k }
                spacing: 6 * dash.k
                Repeater {
                    model: dash.menuItems
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        width: menuCol.width; height: 66 * dash.k; radius: 12 * dash.k
                        color: dash.menuSel === index ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18) : "transparent"
                        border.width: dash.menuSel === index ? 2 : 0; border.color: Theme.accent
                        RowLayout { anchors.fill: parent; anchors.leftMargin: 18 * dash.k; spacing: 16 * dash.k
                            Icon { name: modelData.icon; size: 24 * dash.k; color: dash.menuSel === index ? Theme.accent : Theme.text }
                            Column { Layout.fillWidth: true
                                Text { text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 18 * dash.k; font.weight: Font.DemiBold }
                                Text { text: modelData.s; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13.5 * dash.k } } }
                        MouseArea { anchors.fill: parent; onClicked: modelData.a() }
                    }
                }
            }
        }

        // Back in Server mode: what's coming back.
        ServerBack {
            visible: dash.overlay === "back" && !dash.remote
            anchors { right: parent.right; top: hero.top; rightMargin: 48 * dash.k }
            width: 436 * dash.k
            k: dash.k; info: dash.backInfo; d: dash.d
        }
    }

    // ---------------------------------------------------------------- the idle screen (dim), then black
    Rectangle {
        anchors.fill: parent
        visible: dash.screenState !== "on"
        color: "#000000"
        Column {
            visible: dash.screenState === "dim"
            anchors.centerIn: parent
            spacing: 26 * dash.k
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatTime(new Date(dash.nowMs), "h:mm"); color: "#4a535d"; font.family: Theme.font; font.pixelSize: 230 * dash.k; font.weight: Font.Light }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatDate(new Date(dash.nowMs), "dddd d MMMM"); color: "#3d454e"; font.family: Theme.font; font.pixelSize: 30 * dash.k }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter; spacing: 34 * dash.k
                Row { spacing: 10 * dash.k
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 10 * dash.k; height: width; implicitWidth: 10 * dash.k; implicitHeight: 10 * dash.k; radius: width / 2; color: dash.d && dash.d.status.level !== "ok" ? "#6b5a2f" : "#2f6b47" }
                    Text { text: dash.d ? dash.d.status.title : ""; color: "#39424b"; font.family: Theme.font; font.pixelSize: 21 * dash.k } }
                Text { text: dash.d ? dash.d.status.sub : ""; color: "#39424b"; font.family: Theme.font; font.pixelSize: 21 * dash.k }
                Text { text: dash.d ? dash.d.devices + (dash.d.devices === 1 ? " device" : " devices") : ""; color: "#39424b"; font.family: Theme.font; font.pixelSize: 21 * dash.k }
                Repeater { model: dash.d ? dash.d.facts.filter(f => f[1] !== "—").slice(0, 2) : []
                    delegate: Text { required property var modelData; text: modelData[0] + " " + modelData[1]; color: "#39424b"; font.family: Theme.font; font.pixelSize: 21 * dash.k } }
            }
        }
        Text { visible: dash.screenState === "dim"; anchors { bottom: parent.bottom; bottomMargin: 56 * dash.k; horizontalCenter: parent.horizontalCenter }
            text: (dash.d ? dash.d.host : "") + " · Server mode · press any button"; color: "#2b3138"; font.family: Theme.font; font.pixelSize: 16 * dash.k }
    }
}
