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
    color: dash.gBg
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
                try { const v = JSON.parse(text); dash.d = v; if (v.busy) dash.lastBusy = Date.now(); dash.sample(); } catch (e) { console.warn("server mode: dash: " + e); }
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
        // Play sits to the right of the tiles: Right from a row's last tile goes to it, Left from Play to the top row's last.
        if (K === Qt.Key_Right) sel = sel === 0 ? 0 : (sel % cols === 0 || sel === n) ? 0 : Math.min(n, sel + 1);
        else if (K === Qt.Key_Left) sel = sel === 0 ? Math.min(n, cols) : Math.max(1, sel - 1);
        else if (K === Qt.Key_Down) sel = sel === 0 ? Math.min(n, cols) : Math.min(n, sel + cols);
        else if (K === Qt.Key_Up) sel = sel === 0 ? 0 : sel <= cols ? 0 : sel - cols;
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

    // ---------------------------------------------------------------- the gold look (the Hearth AI Command Center's)
    readonly property color gBg: "#030201"
    readonly property color gPanel: "#0c0702"
    readonly property color gPanel2: "#140c03"
    readonly property color gLine: "#2b1c08"
    readonly property color gLine2: "#4a3212"
    readonly property color gText: "#ffe9c7"
    readonly property color gHi: "#fff4df"
    readonly property color gMuted: "#c9a372"
    readonly property color gFaint: "#8a6a3e"
    readonly property color gAcc: "#ffb347"
    readonly property color gInk: "#140c03"
    readonly property color gOk: "#6fe39a"
    readonly property color gWarn: "#ff8a4c"
    readonly property int cols: 5

    // The last hour, one sample every 30 seconds (only on this screen, never saved).
    property var hist: ({ vram: [], storage: [], power: [], net: [] })
    property double lastSample: 0
    function sample() {
        if (!d || !d.live || Date.now() - lastSample < 30000) return;
        lastSample = Date.now();
        const L = d.live, h = hist;
        const push = (k, v) => { const a = (h[k] || []).concat([typeof v === "number" ? v : 0]); h[k] = a.slice(-120); };
        push("vram", L.vram); push("storage", L.storage); push("power", L.power); push("net", (L.rx || 0) + (L.tx || 0));
        hist = Object.assign({}, h);
    }
    function mbps(b) { return b === null || b === undefined ? "—" : (b / 1e6).toFixed(1); }
    readonly property var asking: d && d.who ? d.who.filter(w => (w.what || []).indexOf("AI chat") >= 0).map(w => w.name) : []
    readonly property var talker: d && d.cores ? d.cores.find(c => c.mode === "working" || c.state === "working") || null : null
    readonly property var vramParts: {
        const names = { ai: ["AI chat", "#e8d66e"], pictures: ["ComfyUI", "#ff5fd2"], voices: ["Voices", "#ff9aa8"], media: ["Media", "#7fb2ff"], other: ["Other", "#c9a372"] };
        return (d && d.vram ? d.vram : []).map(v => ({ name: (names[v.who] || names.other)[0], color: (names[v.who] || names.other)[1], gb: v.gb }));
    }

    component GPanel: Rectangle {
        property string title: ""
        default property alias content: inner.data
        radius: 16 * dash.k; color: dash.gPanel; border.color: dash.gLine; border.width: 1
        Rectangle { x: -1; y: -1; width: 16 * dash.k; height: 2 * dash.k; color: dash.gAcc }
        Rectangle { x: -1; y: -1; width: 2 * dash.k; height: 16 * dash.k; color: dash.gAcc }
        Rectangle { x: parent.width - width + 1; y: parent.height - height + 1; width: 16 * dash.k; height: 2 * dash.k; color: dash.gAcc }
        Rectangle { x: parent.width - width + 1; y: parent.height - height + 1; width: 2 * dash.k; height: 16 * dash.k; color: dash.gAcc }
        Text { id: ttl; x: 18 * dash.k; y: 14 * dash.k; text: "◢ " + parent.title.toUpperCase(); color: dash.gMuted; font.family: Theme.font; font.pixelSize: 12.5 * dash.k; font.weight: Font.DemiBold; font.letterSpacing: 2.6 * dash.k }
        Item { id: inner; anchors { left: parent.left; right: parent.right; top: ttl.bottom; bottom: parent.bottom; leftMargin: 18 * dash.k; rightMargin: 18 * dash.k; topMargin: 10 * dash.k; bottomMargin: 14 * dash.k } }
    }
    component Spark: Canvas {
        property var pts: []
        property color line: dash.gAcc
        onPtsChanged: requestPaint()
        onPaint: {
            const c = getContext("2d"); c.reset();
            if (!pts || pts.length < 2) return;
            const mx = Math.max.apply(null, pts.concat([1])), mn = Math.min.apply(null, pts);
            const span = Math.max(mx - mn, mx * 0.1, 0.001);
            c.strokeStyle = "" + line; c.lineWidth = 2 * dash.k; c.lineJoin = "round"; c.beginPath();
            for (let i = 0; i < pts.length; i++) {
                const x = i * width / (pts.length - 1), y = height - 2 - (pts[i] - mn) / span * (height - 4);
                if (i === 0) c.moveTo(x, y); else c.lineTo(x, y);
            }
            c.stroke();
        }
    }
    component Fact: Rectangle {
        property string label: ""
        property string value: ""
        property string sub: ""
        property var pts: []
        property color line: dash.gAcc
        radius: 12 * dash.k; color: dash.gPanel2; border.color: dash.gLine
        Text { id: fl; x: 14 * dash.k; y: 10 * dash.k; text: parent.label; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 14 * dash.k }
        Text { id: fv; x: 14 * dash.k; anchors.top: fl.bottom; anchors.topMargin: 2 * dash.k; text: parent.value; color: dash.gHi; font.family: Theme.font; font.pixelSize: 23 * dash.k; font.weight: Font.DemiBold }
        Text { x: 14 * dash.k; anchors.top: fv.bottom; anchors.topMargin: 2 * dash.k; width: parent.width - 28 * dash.k; text: parent.sub; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 13 * dash.k; elide: Text.ElideRight }
        Spark { anchors { right: parent.right; rightMargin: 14 * dash.k; top: parent.top; topMargin: 28 * dash.k } width: 120 * dash.k; height: 32 * dash.k; pts: parent.pts; line: parent.line }
    }

    // ---------------------------------------------------------------- the dashboard
    Item {
        anchors.fill: parent
        visible: dash.screenState === "on"
        // The dotted grid behind everything.
        Canvas {
            anchors.fill: parent
            onPaint: {
                const c = getContext("2d"); c.reset();
                c.fillStyle = "rgba(255,179,71,0.07)";
                const step = 24 * dash.k;
                for (let y = step / 2; y < height; y += step) for (let x = step / 2; x < width; x += step) c.fillRect(x, y, 1.2, 1.2);
            }
            onWidthChanged: requestPaint(); onHeightChanged: requestPaint()
        }
        // Top line: Server mode, the PC's name, Tailscale and the web page, the date and the clock.
        Rectangle {
            id: top
            anchors { left: parent.left; right: parent.right; top: parent.top }
            height: 78 * dash.k; color: "transparent"
            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: dash.gLine }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 36 * dash.k; anchors.rightMargin: 36 * dash.k
                spacing: 16 * dash.k
                Rectangle { implicitWidth: 42 * dash.k; implicitHeight: 42 * dash.k; radius: 11 * dash.k; color: "#2b1a08"
                    Icon { anchors.centerIn: parent; name: "server"; size: 22 * dash.k; color: Theme.accent } }
                Text { text: "◢ SERVER MODE"; color: dash.gAcc; font.family: Theme.font; font.pixelSize: 14 * dash.k; font.weight: Font.DemiBold; font.letterSpacing: 3.6 * dash.k }
                Text { text: (dash.d ? dash.d.host : "Hearth").toUpperCase(); color: dash.gHi; font.family: Theme.font; font.pixelSize: 24 * dash.k; font.weight: Font.DemiBold; font.letterSpacing: 2.4 * dash.k }
                Item { Layout.fillWidth: true }
                Repeater {
                    model: [["Tailscale", !!(dash.d && dash.tiles.find(t => t.id === "who" && t.state === "running"))], ["Web page", !!(dash.d && dash.d.web && dash.d.web.on)]]
                    delegate: Rectangle {
                        required property var modelData
                        implicitWidth: pt.implicitWidth + 26 * dash.k; implicitHeight: 30 * dash.k; radius: 15 * dash.k; color: "transparent"
                        border.color: modelData[1] ? Qt.rgba(0.435, 0.89, 0.604, 0.45) : dash.gLine2
                        Text { id: pt; anchors.centerIn: parent; text: modelData[0] + (modelData[1] ? " · on" : " · off"); color: modelData[1] ? dash.gOk : dash.gMuted; font.family: Theme.font; font.pixelSize: 14 * dash.k }
                    }
                }
                Text { text: Qt.formatDate(new Date(dash.nowMs), "ddd d MMM"); color: dash.gMuted; font.family: Theme.font; font.pixelSize: 17 * dash.k }
                Text { text: Qt.formatTime(new Date(dash.nowMs), "h:mm AP"); color: dash.gHi; font.family: Theme.font; font.pixelSize: 23 * dash.k; font.weight: Font.DemiBold }
            }
        }

        // ---- the right column: Play, Next, graphics memory by app, Running, Alerts, your phone
        ColumnLayout {
            id: hero
            anchors { right: parent.right; top: top.bottom; bottom: hints.top; rightMargin: 36 * dash.k; topMargin: 14 * dash.k; bottomMargin: 12 * dash.k }
            width: 430 * dash.k
            spacing: 12 * dash.k
            Rectangle {
                id: playCard
                Layout.fillWidth: true; Layout.preferredHeight: 108 * dash.k
                radius: 18 * dash.k; color: dash.gAcc
                border.width: dash.sel === 0 ? 4 * dash.k : 0; border.color: "#ffe2b0"
                Column { anchors.centerIn: parent; spacing: 2 * dash.k
                    Row { anchors.horizontalCenter: parent.horizontalCenter; spacing: 14 * dash.k
                        Icon { anchors.verticalCenter: parent.verticalCenter; name: "pad"; size: 34 * dash.k; color: Theme.accentInk }
                        Text { text: "PLAY"; color: dash.gInk; font.family: Theme.font; font.pixelSize: 36 * dash.k; font.weight: Font.Bold; font.letterSpacing: 5 * dash.k } }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Switch to Game Mode"; color: dash.gInk; font.family: Theme.font; font.pixelSize: 16 * dash.k; font.weight: Font.DemiBold } }
                Text { anchors { right: parent.right; top: parent.top; margins: 14 * dash.k } text: "or press Guide"; color: dash.gInk; font.family: Theme.font; font.pixelSize: 13 * dash.k; font.weight: Font.DemiBold }
                MouseArea { anchors.fill: parent; onClicked: { dash.sel = 0; dash.play(); } }
            }
            GPanel {
                title: "Next"
                Layout.fillWidth: true; Layout.preferredHeight: 104 * dash.k
                Column { spacing: 4 * dash.k
                    Text { text: dash.d && dash.d.next ? dash.d.next.text : "No schedule"; color: dash.gHi; font.family: Theme.font; font.pixelSize: 19 * dash.k; font.weight: Font.DemiBold }
                    Text { text: dash.d && dash.d.next ? "Your " + (new Date(dash.nowMs).getDay() % 6 === 0 ? "weekend" : "weekday") + " schedule" + (dash.d.idle && dash.d.idle.dim > 0 ? " · the screen dims after " + dash.d.idle.dim + " min" : "")
                                                       : (dash.d && dash.d.idle && dash.d.idle.dim > 0 ? "The screen dims after " + dash.d.idle.dim + " min with nothing pressed" : "Make one in Settings › System › Startup")
                          color: dash.gMuted; font.family: Theme.font; font.pixelSize: 14 * dash.k } }
            }
            GPanel {
                title: "Graphics memory by app"
                Layout.fillWidth: true; Layout.preferredHeight: 116 * dash.k
                visible: dash.vramParts.length > 0 && !!dash.d && dash.d.vramTotal > 0
                Column { width: parent.width; spacing: 8 * dash.k
                    Rectangle { width: parent.width; height: 12 * dash.k; radius: 6 * dash.k; color: dash.gLine; clip: true
                        Row { Repeater { model: dash.vramParts
                            delegate: Rectangle { required property var modelData; height: 12 * dash.k; width: dash.d && dash.d.vramTotal > 0 ? parent.parent.width * modelData.gb / dash.d.vramTotal : 0; color: modelData.color } } } }
                    Flow { width: parent.width; spacing: 16 * dash.k
                        Repeater { model: dash.vramParts
                            delegate: Text { required property var modelData; text: "<font color='" + modelData.color + "'>●</font> " + modelData.name + " " + modelData.gb + " GB"; textFormat: Text.StyledText; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 14 * dash.k } }
                        Text { text: dash.d ? "free " + Math.max(0, dash.d.vramTotal - dash.d.vramUsed).toFixed(1) + " GB" : ""; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 14 * dash.k } }
                }
            }
            GPanel {
                title: "Running"
                Layout.fillWidth: true; Layout.preferredHeight: pillFlow.implicitHeight + 60 * dash.k
                Flow {
                    id: pillFlow
                    width: parent.width; spacing: 8 * dash.k
                    Repeater {
                        model: dash.d ? dash.d.pills : []
                        delegate: Rectangle {
                            required property var modelData
                            height: 32 * dash.k; radius: 16 * dash.k; width: pr.implicitWidth + 24 * dash.k
                            readonly property color dc: modelData.state === "running" ? dash.gOk : modelData.state === "off" ? dash.gMuted : dash.gWarn
                            color: "transparent"; border.color: Qt.rgba(dc.r, dc.g, dc.b, 0.4)
                            Row { id: pr; anchors.centerIn: parent; spacing: 7 * dash.k
                                Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 8 * dash.k; height: width; radius: width / 2; color: parent.parent.dc }
                                Text { text: modelData.name; color: dash.gText; font.family: Theme.font; font.pixelSize: 14.5 * dash.k }
                                Text { text: modelData.extra || ""; visible: text !== ""; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 13 * dash.k; anchors.verticalCenter: parent.verticalCenter } }
                        }
                    }
                }
            }
            GPanel {
                id: alerts
                title: "Alerts"
                Layout.fillWidth: true; Layout.preferredHeight: alertCol.implicitHeight + 60 * dash.k
                Column {
                    id: alertCol
                    width: parent.width; spacing: 10 * dash.k
                    Repeater {
                        model: dash.d && dash.d.alerts.length ? dash.d.alerts : [{ level: "ok", text: "Nothing needs a look", sub: "", at: "" }]
                        delegate: RowLayout {
                            required property var modelData
                            width: alertCol.width; spacing: 10 * dash.k
                            Rectangle { Layout.alignment: Qt.AlignTop; Layout.topMargin: 6 * dash.k; implicitWidth: 9 * dash.k; implicitHeight: 9 * dash.k; radius: 5 * dash.k; color: modelData.level === "warn" ? dash.gWarn : modelData.level === "ok" ? dash.gOk : dash.gMuted }
                            Text { Layout.fillWidth: true; text: modelData.text + (modelData.sub ? "<font color='" + dash.gMuted + "'> · " + modelData.sub + "</font>" : ""); textFormat: Text.StyledText; color: dash.gText; font.family: Theme.font; font.pixelSize: 15 * dash.k; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
                            Text { Layout.alignment: Qt.AlignTop; text: modelData.at || ""; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 13.5 * dash.k }
                        }
                    }
                }
            }
            GPanel {
                title: "Your phone"
                Layout.fillWidth: true; Layout.preferredHeight: 128 * dash.k
                visible: !!(dash.d && dash.d.web && dash.d.web.on)
                RowLayout { anchors.fill: parent; spacing: 14 * dash.k
                    Image { visible: !!(dash.d && dash.d.qr); source: dash.d && dash.d.qr ? "file://" + dash.d.qr : ""; Layout.preferredWidth: 72 * dash.k; Layout.preferredHeight: 72 * dash.k; sourceSize.width: 144; sourceSize.height: 144; smooth: false; fillMode: Image.PreserveAspectFit }
                    Column { Layout.fillWidth: true; spacing: 3 * dash.k
                        Text { text: dash.d && dash.d.web ? dash.d.web.url : ""; color: dash.gHi; font.family: Theme.font; font.pixelSize: 16 * dash.k; font.weight: Font.DemiBold }
                        Text { width: parent.width; text: (dash.d && dash.d.qr ? "Scan to open this dashboard on your phone · " : "Open it on your phone · ") + "Tailscale only · your PIN"; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 13.5 * dash.k; wrapMode: Text.WordWrap } }
                }
            }
            Item { Layout.fillHeight: true }
        }

        // ---- the left column: this PC, the AIs, the services
        ColumnLayout {
            anchors { left: parent.left; right: hero.left; top: top.bottom; bottom: hints.top; leftMargin: 36 * dash.k; rightMargin: 22 * dash.k; topMargin: 14 * dash.k; bottomMargin: 12 * dash.k }
            spacing: 12 * dash.k
            GPanel {
                id: heroCard
                title: "This PC · " + (dash.d ? dash.d.host : "")
                Layout.fillWidth: true; Layout.preferredHeight: 178 * dash.k
                readonly property color sc: dash.d && dash.d.status && dash.d.status.level !== "ok" ? dash.gWarn : dash.gOk
                ColumnLayout { anchors.fill: parent; spacing: 10 * dash.k
                    RowLayout { Layout.fillWidth: true; spacing: 12 * dash.k
                        Text { id: statusTitle; text: dash.backInfo && dash.overlay === "back" ? "Coming back" : (dash.d ? dash.d.status.title : "Starting…"); color: heroCard.sc; font.family: Theme.font; font.pixelSize: 34 * dash.k; font.weight: Font.DemiBold }
                        Text { Layout.alignment: Qt.AlignBaseline; text: dash.d ? "· " + dash.d.status.sub : ""; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 21 * dash.k }
                        Item { Layout.fillWidth: true }
                        Text { text: dash.d ? "<b>" + dash.d.devices + (dash.d.devices === 1 ? " device</b> connected" : " devices</b> connected") + (dash.d.uptime >= 86400 ? " · Up <b>" + Math.floor(dash.d.uptime / 86400) + " days</b>" : "") + " · Server mode since <b>" + dash.d.sinceText + "</b>" : ""
                               textFormat: Text.StyledText; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 16 * dash.k }
                    }
                    RowLayout { Layout.fillWidth: true; Layout.fillHeight: true; spacing: 12 * dash.k
                        Fact { Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1; label: dash.d ? dash.d.facts[0][0] : "Graphics memory"; value: dash.d ? dash.d.facts[0][1] : "—"; line: "#e8d66e"; pts: dash.hist.vram
                            sub: dash.d && dash.d.live ? (dash.d.gpuName || "Graphics card") + (dash.d.live.gpuBusy !== null && dash.d.live.gpuBusy !== undefined ? " · " + Math.round(dash.d.live.gpuBusy) + "% busy" : "") + (dash.d.live.gpuTemp ? " · " + Math.round(dash.d.live.gpuTemp) + " °C" : "") : "" }
                        Fact { Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1; label: dash.d ? dash.d.facts[1][0] : "Storage"; value: dash.d ? dash.d.facts[1][1] : "—"; line: dash.gAcc; pts: dash.hist.storage
                            sub: dash.d && dash.d.live && dash.d.live.storage !== null && dash.d.live.storage !== undefined ? dash.d.live.storage + "% used" : "" }
                        Fact { Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1; label: dash.d ? dash.d.facts[2][0] : "Power"; value: dash.d ? dash.d.facts[2][1] : "—"; line: dash.gWarn; pts: dash.hist.power
                            sub: dash.d && dash.d.live ? "processor " + (dash.d.live.cpuTemp ? dash.d.live.cpuTemp + " °C · " : "") + (dash.d.live.cpu !== null && dash.d.live.cpu !== undefined ? Math.round(dash.d.live.cpu) + "% busy" : "") : "" }
                        Fact { Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1; label: "Network"; line: "#7fb2ff"; pts: dash.hist.net
                            value: dash.d && dash.d.live ? "↓" + dash.mbps(dash.d.live.rx) + " ↑" + dash.mbps(dash.d.live.tx) + " MB/s" : "—"
                            sub: dash.d && dash.d.live ? dash.d.live.online + " online on Tailscale" : "" }
                    }
                }
            }
            GPanel {
                title: "The AIs on this PC · the same as in your Command Center"
                Layout.fillWidth: true; Layout.preferredHeight: 214 * dash.k
                visible: !!(dash.d && dash.d.cores && dash.d.cores.length)
                ColumnLayout { anchors.fill: parent; spacing: 6 * dash.k
                    RowLayout { Layout.fillWidth: true; Layout.fillHeight: true; spacing: 0
                        Repeater {
                            model: dash.d && dash.d.cores ? dash.d.cores : []
                            delegate: Column {
                                required property var modelData
                                Layout.fillWidth: true; Layout.preferredWidth: 1
                                spacing: 4 * dash.k
                                ServerCore { anchors.horizontalCenter: parent.horizontalCenter; kind: modelData.id; mode: modelData.state || "ready"; size: 82 * dash.k }
                                Item { width: 1; height: 4 * dash.k }
                                Text { anchors.horizontalCenter: parent.horizontalCenter; text: (modelData.name || "").toUpperCase(); color: modelData.state === "working" ? (({ gpu: "#e8d66e", comfy: "#ff5fd2", voice: "#ff9aa8", listen: "#9cf5d6", hearth: "#ff8a4c" })[modelData.id] || dash.gHi) : dash.gHi; font.family: Theme.font; font.pixelSize: 14 * dash.k; font.weight: Font.DemiBold; font.letterSpacing: 1.6 * dash.k }
                                Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.status || ""; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 13.5 * dash.k }
                            }
                        }
                    }
                    Rectangle { Layout.fillWidth: true; height: 1; color: dash.gLine }
                    RowLayout { Layout.fillWidth: true; spacing: 12 * dash.k
                        Row { spacing: 3 * dash.k; visible: !!dash.talker
                            Repeater { model: [700, 500, 800, 600, 900]
                                delegate: Rectangle { required property var modelData; anchors.verticalCenter: parent.verticalCenter; width: 4 * dash.k; height: 22 * dash.k; radius: 2; color: "#e8d66e"
                                    SequentialAnimation on scale { running: !Settings.reduceMotion && !!dash.talker; loops: Animation.Infinite
                                        NumberAnimation { to: 0.25; duration: modelData / 2 } NumberAnimation { to: 1; duration: modelData / 2 } } } } }
                        Text { Layout.fillWidth: true; elide: Text.ElideRight; textFormat: Text.StyledText; font.family: Theme.font; font.pixelSize: 16 * dash.k; color: dash.gHi
                            text: dash.talker ? "<b>" + dash.talker.name.toUpperCase() + "</b>  " + (dash.talker.id === "gpu" && dash.asking.length ? "answering " + dash.asking.join(", ") : dash.talker.status)
                                              : "<font color='" + dash.gMuted + "'>All quiet: nobody's asking right now</font>" }
                    }
                }
            }
            GPanel {
                title: "Services"
                Layout.fillWidth: true; Layout.fillHeight: true
                GridLayout {
                    id: grid
                    anchors.fill: parent
                    columns: dash.cols; rowSpacing: 12 * dash.k; columnSpacing: 12 * dash.k
                    Repeater {
                        model: dash.tiles
                        delegate: ServerTile {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true; Layout.fillHeight: true
                            Layout.preferredWidth: 1; Layout.preferredHeight: 1
                            t: modelData; k: dash.k * 0.86; focused: dash.sel === index + 1; gold: true
                            MouseArea { anchors.fill: parent; onClicked: { dash.sel = index + 1; dash.overlay = "detail"; } }
                        }
                    }
                }
            }
        }

        // The buttons along the bottom.
        Rectangle {
            id: hints
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: 48 * dash.k
            color: Qt.rgba(0.008, 0.004, 0, 0.92)
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: dash.gLine }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 36 * dash.k; anchors.rightMargin: 36 * dash.k; spacing: 26 * dash.k
                Repeater {
                    model: [["A", "Open"], ["X", "Restart service"], ["Y", "Pause or resume"], ["☰", "Menu"]]
                    delegate: Row { required property var modelData; spacing: 8 * dash.k
                        Rectangle { width: 26 * dash.k; height: width; implicitWidth: 26 * dash.k; implicitHeight: 26 * dash.k; radius: width / 2; border.color: dash.gMuted; color: "transparent"
                            Text { anchors.centerIn: parent; text: modelData[0]; color: dash.gHi; font.pixelSize: 12 * dash.k; font.weight: Font.Bold } }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; color: dash.gText; font.family: Theme.font; font.pixelSize: 15 * dash.k } }
                }
                Item { Layout.fillWidth: true }
                Text { visible: dash.toast !== ""; text: dash.toast; color: dash.gAcc; font.family: Theme.font; font.pixelSize: 15 * dash.k }
                Item { Layout.fillWidth: true }
                Text { text: "Guide  Play (Game Mode)"; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 15 * dash.k }
            }
        }

        // ------------------------------------------------------------ overlays
        Rectangle { anchors.fill: parent; visible: dash.overlay !== "" && dash.overlay !== "back" || !!dash.remote; color: Qt.rgba(0.012, 0.008, 0.004, 0.82) }

        // The countdown: Play / Guide (5 s, here) or the web page / schedule (10 s, the agent's).
        Rectangle {
            id: sw
            visible: dash.overlay === "switch" || !!dash.remote
            anchors.centerIn: parent
            width: 1240 * dash.k; height: swCol.implicitHeight + 72 * dash.k
            radius: 22 * dash.k; color: dash.gPanel; border.color: dash.gLine
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
                                c.strokeStyle = "" + dash.gLine2; c.beginPath(); c.arc(width / 2, height / 2, r, 0, Math.PI * 2); c.stroke();
                                c.strokeStyle = "" + dash.gAcc; c.beginPath(); c.arc(width / 2, height / 2, r, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * p); c.stroke();
                            }
                        }
                        Column { anchors.centerIn: parent
                            Text { anchors.horizontalCenter: parent.horizontalCenter; text: String(sw.secsLeft); color: dash.gText; font.family: Theme.font; font.pixelSize: 72 * dash.k; font.weight: Font.DemiBold }
                            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "seconds"; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 15 * dash.k } }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 8 * dash.k
                        Text { text: (sw.isRemote && dash.remote.what === "power" ? "" : "SWITCHING TO ") + sw.toName + (sw.isRemote ? "  ·  ASKED FOR " + (dash.remote.from === "schedule" ? "BY THE SCHEDULE" : "FROM " + (dash.remote.by || "THE WEB PAGE").toUpperCase()) : ""); color: dash.gAcc; font.family: Theme.font; font.pixelSize: 15 * dash.k; font.weight: Font.DemiBold; font.letterSpacing: 0.4 }
                        Text { text: sw.isRemote && dash.remote.what === "power" ? (dash.remote.to === "sleep" ? "The PC is going to sleep" : "The PC is restarting") : sw.toName === "DESKTOP MODE" ? "Opening Desktop Mode" : "Getting the PC ready to play"; color: dash.gText; font.family: Theme.font; font.pixelSize: 40 * dash.k; font.weight: Font.DemiBold }
                        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Big jobs pause so your game gets the graphics card. They come back by themselves when you return to Server mode."; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 17 * dash.k }
                    }
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2; columnSpacing: 28 * dash.k; rowSpacing: 10 * dash.k
                    Text { text: "Pauses while you play"; color: dash.gText; font.family: Theme.font; font.pixelSize: 16 * dash.k; font.weight: Font.DemiBold; Layout.fillWidth: true }
                    Text { text: "Keeps working"; color: dash.gText; font.family: Theme.font; font.pixelSize: 16 * dash.k; font.weight: Font.DemiBold; Layout.fillWidth: true }
                    ColumnLayout { Layout.alignment: Qt.AlignTop; Layout.fillWidth: true; Layout.preferredWidth: 1; spacing: 10 * dash.k
                        Repeater { model: dash.pauseList; delegate: ServerLine { required property var modelData; Layout.fillWidth: true; k: dash.k; icon: modelData.icon; title: modelData.t; sub: modelData.s; tone: dash.gWarn } }
                        Text { visible: dash.pauseList.length === 0; text: "Nothing: Light mode is off or nothing big is on"; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 15 * dash.k } }
                    ColumnLayout { Layout.alignment: Qt.AlignTop; Layout.fillWidth: true; Layout.preferredWidth: 1; spacing: 10 * dash.k
                        Repeater { model: dash.keepList; delegate: ServerLine { required property var modelData; Layout.fillWidth: true; k: dash.k; icon: modelData.icon; title: modelData.t; sub: modelData.s; tone: dash.gOk } } }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 14 * dash.k
                    Text { text: "The same rules as Light mode · change them in Settings › Home server › Server mode"; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 14 * dash.k; Layout.fillWidth: true; elide: Text.ElideRight }
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
            radius: 22 * dash.k; color: dash.gPanel; border.color: dash.gLine
            ServerDetail { anchors.fill: parent; t: dash.cur; d: dash.d; k: dash.k
                onAction: a => a === "unload" ? dash.act(["ai-unload"], "Model unloaded") : dash.serviceAct(dash.cur, a)
                onClose: dash.overlay = "" }
        }

        // The menu (☰): Game Mode, Desktop Mode, Settings, the screen, sleep and restart.
        Rectangle {
            visible: dash.overlay === "menu" && !dash.remote
            anchors.centerIn: parent
            width: 640 * dash.k; height: menuCol.implicitHeight + 40 * dash.k
            radius: 22 * dash.k; color: dash.gPanel; border.color: dash.gLine
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
                        color: dash.menuSel === index ? Qt.rgba(dash.gAcc.r, dash.gAcc.g, dash.gAcc.b, 0.18) : "transparent"
                        border.width: dash.menuSel === index ? 2 : 0; border.color: dash.gAcc
                        RowLayout { anchors.fill: parent; anchors.leftMargin: 18 * dash.k; spacing: 16 * dash.k
                            Icon { name: modelData.icon; size: 24 * dash.k; color: dash.menuSel === index ? dash.gAcc : dash.gText }
                            Column { Layout.fillWidth: true
                                Text { text: modelData.t; color: dash.gText; font.family: Theme.font; font.pixelSize: 18 * dash.k; font.weight: Font.DemiBold }
                                Text { text: modelData.s; color: dash.gMuted; font.family: Theme.font; font.pixelSize: 13.5 * dash.k } } }
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
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatTime(new Date(dash.nowMs), "h:mm"); color: "#8a6a3e"; font.family: Theme.font; font.pixelSize: 230 * dash.k; font.weight: Font.Light }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatDate(new Date(dash.nowMs), "dddd d MMMM"); color: "#5a4424"; font.family: Theme.font; font.pixelSize: 30 * dash.k }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter; spacing: 34 * dash.k
                Row { spacing: 10 * dash.k
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 10 * dash.k; height: width; implicitWidth: 10 * dash.k; implicitHeight: 10 * dash.k; radius: width / 2; color: dash.d && dash.d.status.level !== "ok" ? "#6b5a2f" : "#2f6b47" }
                    Text { text: dash.d ? dash.d.status.title : ""; color: "#5a4424"; font.family: Theme.font; font.pixelSize: 21 * dash.k } }
                Text { text: dash.d ? dash.d.status.sub : ""; color: "#5a4424"; font.family: Theme.font; font.pixelSize: 21 * dash.k }
                Text { text: dash.d ? dash.d.devices + (dash.d.devices === 1 ? " device" : " devices") : ""; color: "#5a4424"; font.family: Theme.font; font.pixelSize: 21 * dash.k }
                Repeater { model: dash.d ? dash.d.facts.filter(f => f[1] !== "—").slice(0, 2) : []
                    delegate: Text { required property var modelData; text: modelData[0] + " " + modelData[1]; color: "#5a4424"; font.family: Theme.font; font.pixelSize: 21 * dash.k } }
            }
        }
        // The AIs, faint (kind to OLED screens): they keep moving only while one is working.
        Row {
            visible: dash.screenState === "dim" && !!(dash.d && dash.d.cores && dash.d.cores.length)
            anchors { bottom: parent.bottom; bottomMargin: 150 * dash.k; horizontalCenter: parent.horizontalCenter }
            spacing: 70 * dash.k
            opacity: 0.32
            Repeater {
                model: dash.d && dash.d.cores ? dash.d.cores : []
                delegate: ServerCore { required property var modelData; kind: modelData.id; mode: modelData.state === "working" ? "working" : "ready"; size: 64 * dash.k }
            }
        }
        Text { visible: dash.screenState === "dim"; anchors { bottom: parent.bottom; bottomMargin: 56 * dash.k; horizontalCenter: parent.horizontalCenter }
            text: (dash.d ? dash.d.host : "") + " · Server mode · press any button"; color: "#3a2c18"; font.family: Theme.font; font.pixelSize: 16 * dash.k }
    }
}
