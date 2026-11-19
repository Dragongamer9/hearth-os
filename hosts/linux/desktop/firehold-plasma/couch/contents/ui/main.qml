// Firehold's Couch on Plasma (a test, docs/FIREHOLD.md "Firehold on Plasma"): the desktop containment of the shell
// package org.hearthos.firehold, on KDE's own KWin. Home, Library, Apps and Store, Everything open, the Guide and
// Settings › Couch and Desk (docs/mockups/firehold, checklist docs/mockups/checklists/firehold-plasma.md).
//
// Windows come from Plasma's own task model (org.kde.taskmanager, as Plasma's task manager has them), games from
// desktop/shell/hearth-firehold (the same list Mantel's Firehold shows), settings from ~/.config/nimbus/firehold.json
// through hearth-firehold-plasma (hearth_settings_io: a key at a time, atomic, .bak; opening a page writes nothing).
// Performance (the owner's red flag): no Timer and no animation anywhere; the clock is Plasma's time source once a
// minute; window pictures stream only while Everything open is on screen.
import QtQuick
import QtQuick.Window
import QtQuick.Shapes
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as P5Support
import org.kde.taskmanager as TaskManager
import "firehold.js" as F
import "fhp.js" as H

ContainmentItem {
    id: fh

    readonly property real k: Math.max(0.5, Math.min(width / 1920, height / 1080))
    readonly property string fontName: F.font("Noto Sans")
    // The approved mockups' colours (firehold-plasma 01-12; fhp.js P has the rest).
    readonly property QtObject pal: QtObject {
        readonly property color bg: "#0f0b0a"
        readonly property color bg2: "#1d120d"
        readonly property color text: "#f2ede8"
        readonly property color dim: "#a39d96"
        readonly property color faint: "#6f6a66"
        readonly property color card: "#1e1a18"
        readonly property color card2: "#2b2522"
        readonly property color line: "#2e2a28"
        readonly property color accent: "#ff7a2f"
        readonly property color gold: "#ffb36b"
        readonly property color green: "#5fd38d"
        readonly property color red: "#ff6b6b"
        readonly property color blue: "#5bb8ff"
        readonly property color yellow: "#ffd35c"
    }

    // ------------------------------------------------------------------ what's on screen (runtime only)
    property string tab: "home"            // home | library | apps | media | store
    property string page: "tabs"           // tabs | switcher (Everything open) | settings
    property bool guideShown: false
    property bool started: false
    readonly property bool front: Window.active
    onFrontChanged: {
        if (!started) return;
        run(["state", front ? "front" : (guideShown ? "guide" : "back")]);
        if (front) { forceActiveFocus(); refreshSys(false); cardsNow(); }
    }
    onPageChanged: if (started && page === "tabs") askCards()
    onGuideShownChanged: if (started) { run(["state", guideShown ? "guide" : (front ? "front" : "back")]); if (guideShown) refreshSys(true); }

    // ------------------------------------------------------------------ settings (firehold.json, shared with Mantel's Firehold)
    property var cfg: ({})
    readonly property string mode: ["auto", "couch", "desk"].indexOf(cfg.posture) >= 0 ? cfg.posture : "auto"
    readonly property string startIn: ["couch", "desk", "last"].indexOf(cfg.startIn) >= 0 ? cfg.startIn : "desk"
    readonly property string guideButton: ["quick", "open"].indexOf(cfg.guide) >= 0 ? cfg.guide : "quick"
    readonly property string gamesOpen: ["follow", "fullscreen", "window"].indexOf(cfg.games) >= 0 ? cfg.games : "follow"
    readonly property bool bigPicture: cfg.bigPicture === true
    function set(key, value) {
        if (JSON.stringify(cfg[key]) === JSON.stringify(value)) return;
        const c = Object.assign({}, cfg);
        c[key] = value;
        cfg = c;
        run(["set", key, JSON.stringify(value)]);
        if (key === "posture" && value === "desk") run(["switch", "desk", "--why", "pick"]);
    }

    // ------------------------------------------------------------------ the helper (Plasma's executable source)
    P5Support.DataSource {
        id: sh
        engine: "executable"
        property var cbs: ({})
        property int n: 0
        onNewData: function (source, data) {
            const cb = sh.cbs[source];
            delete sh.cbs[source];
            sh.disconnectSource(source);
            if (cb) cb(String(data["stdout"] || ""));
        }
    }
    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    function run(args, cb) {
        sh.n += 1;
        const cmd = "hearth-firehold-plasma " + args.map(q).join(" ") + " #" + sh.n;
        sh.cbs[cmd] = cb || null;
        sh.connectSource(cmd);
    }
    function lastJson(t) {
        const l = String(t || "").trim().split("\n").pop();
        try { return JSON.parse(l); } catch (e) { return null; }
    }

    // ------------------------------------------------------------------ the clock, the bar's status
    P5Support.DataSource {
        id: clock
        engine: "time"
        connectedSources: ["Local"]
        interval: 60000
        intervalAlignment: P5Support.Types.AlignToMinute
    }
    readonly property var now: clock.data["Local"] && clock.data["Local"]["DateTime"] ? clock.data["Local"]["DateTime"] : new Date()
    property var sys: ({})
    property bool sysBusy: false
    function refreshSys(all) {
        if (sysBusy) return;
        sysBusy = true;
        run(all ? ["sys", "all"] : ["sys"], function (t) { fh.sysBusy = false; const o = fh.lastJson(t); if (o) fh.sys = Object.assign({}, fh.sys, o); });
    }

    // ------------------------------------------------------------------ games (hearth-firehold)
    property var library: []
    property var counts: ({})
    property bool libraryLoaded: false
    function take(t, cached) {
        const d = lastJson(t);
        if (!d || !Array.isArray(d.games)) return;
        if (cached && libraryLoaded) return;
        library = d.games;
        counts = d.counts || ({});
        libraryLoaded = true;
    }
    property var apps: []

    // ------------------------------------------------------------------ the Couch home's cards (hearth-firehold-plasma home)
    // Asked for only while the Couch is in front: every 5 s while something downloads, else once a minute (the helper
    // sleeps between looks; no Timer here: the owner's rule on lag). Nothing is asked while a game or the Desk is in front.
    property var cards: ({})
    property bool cardsAsking: false
    property real cardsFullAt: 0
    function takeCards(t, full) {
        const o = lastJson(t);
        if (!o || typeof o !== "object") return;
        cards = full ? o : Object.assign({}, cards, o);
        if (full) cardsFullAt = Date.now();
    }
    function askCards() {
        if (cardsAsking || !front || page !== "tabs") return;
        cardsAsking = true;
        const busy = (cards.downloads || []).some(function (d) { return d.state === "running"; });
        const full = !busy || Date.now() - cardsFullAt > 55000;
        run(["home-after", busy ? "5" : "60"].concat(full ? [] : ["fast"]), function (t) {
            fh.cardsAsking = false;
            fh.takeCards(t, full);
            fh.askCards();
        });
    }
    function cardsNow() {
        run(["home"], function (t) { fh.takeCards(t, true); fh.askCards(); });
    }
    // Firehold's own choices for the home (firehold.json, written only when you change one).
    readonly property var pins: Array.isArray(cfg.pins) ? cfg.pins : []
    readonly property var profiles: {
        const l = (cards.profiles || []).filter(function (p) { return p && p.id; });
        return l.length ? l : [{ id: "main", name: sys.user || "You" }];
    }
    property var optionsGame: null
    function openOptions(g) { if (g) optionsGame = g; }
    property string toastSeen: ""
    property var player: null      // what's playing (MediaCard, Plasma's MPRIS model)

    // ------------------------------------------------------------------ this screen and the others
    readonly property real screenDiagMm: Screen.pixelDensity > 0 ? Math.sqrt(Screen.width * Screen.width + Screen.height * Screen.height) * Screen.devicePixelRatio / Screen.pixelDensity : 0
    readonly property bool screenIsTv: (sys.tvScreens || []).indexOf(Screen.name) >= 0 || H.isTv(Screen.name, Screen.model, screenDiagMm)
    readonly property string screenWord: /^(edp|lvds|dsi)/i.test(Screen.name) ? "screen" : "monitor"
    readonly property var screenList: Qt.application.screens.map(function (s) {
        const d = s.pixelDensity > 0 ? Math.sqrt(s.width * s.width + s.height * s.height) * s.devicePixelRatio / s.pixelDensity : 0;
        const tv = (fh.sys.tvScreens || []).indexOf(s.name) >= 0 || H.isTv(s.name, s.model, d);
        const here = s.name === Screen.name;
        return { name: s.name, label: tv ? "TV" : (/^(edp|lvds|dsi)/i.test(s.name) ? "This screen" : "Monitor"),
                 modeName: here ? "Couch" : "Desk", here: here };
    })

    // ------------------------------------------------------------------ windows (Plasma's task model)
    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        sortMode: TaskManager.TasksModel.SortLastActivated
        filterByVirtualDesktop: false
        filterByScreen: false
        filterByActivity: false
        filterNotMinimized: false
        onCountChanged: fh.readWins()
        onDataChanged: fh.readWins()
    }
    property var wins: []
    property string winsKey: ""
    property var seen: ({})
    property string lastGameId: ""
    function readWins() {
        const R = TaskManager.AbstractTasksModel;
        const out = [];
        for (let i = 0; i < tasks.count; i++) {
            const ix = tasks.makeModelIndex(i);
            if (!tasks.data(ix, R.IsWindow) || tasks.data(ix, R.IsLauncher) || tasks.data(ix, R.IsStartup)) continue;
            const ids = tasks.data(ix, R.WinIdList) || [];
            const uuid = ids.length ? String(ids[0]) : "";
            out.push({ row: i, id: uuid || String(i), uuid: uuid, appId: String(tasks.data(ix, R.AppId) || ""),
                       appName: String(tasks.data(ix, R.AppName) || ""), title: String(tasks.data(ix, Qt.DisplayRole) || ""),
                       activated: !!tasks.data(ix, R.IsActive), minimized: !!tasks.data(ix, R.IsMinimized),
                       fullscreen: !!tasks.data(ix, R.IsFullScreen), pid: Number(tasks.data(ix, R.AppPid) || 0) });
        }
        const key = JSON.stringify(out);
        if (key === winsKey) return;
        winsKey = key;
        const s = {}, t = Date.now();
        for (const w of out) s[w.id] = seen[w.id] || t;
        seen = s;
        wins = out;
    }
    readonly property var openList: F.openItems(wins, library, function (w) { return w.appName; })
    readonly property var playing: F.playingIds(wins, library)
    readonly property var continueGame: F.continueGame(library, playing)
    function gameWins() { return wins.filter(function (w) { return F.isGame(w, fh.library); }); }
    function frontGame() {
        for (const w of wins) if (w.activated && F.isGame(w, library)) return w;
        return null;
    }
    function windowFor(g) {
        if (!g) return null;
        for (const w of wins) { const x = F.gameFor(w, library); if (x && x.id === g.id) return w; }
        return null;
    }
    function idx(w) { return tasks.makeModelIndex(w.row); }

    // Go to a game or window: in front, the Couch steps aside; a game full screen on the Couch (Games: Follow).
    function goTo(w, asWindow) {
        if (!w) return;
        guideShown = false;
        page = "tabs";
        const game = F.isGame(w, library);
        if (game) lastGameId = w.id;
        tasks.requestActivate(idx(w));
        const full = game && !asWindow && gamesOpen !== "window";
        if (game && full !== w.fullscreen) tasks.requestToggleFullScreen(idx(w));
    }
    function toggleFull(w) { if (w) tasks.requestToggleFullScreen(idx(w)); }
    function closeWin(w) { if (w) tasks.requestClose(idx(w)); }
    function play(g, asWindow) {
        if (!g) return;
        const w = windowFor(g);
        if (w) { goTo(w, asWindow); return; }
        if (!g.installed) { run(["page", g.id]); return; }
        run(["play", g.id]);
    }
    function launchApp(a) { if (a) run(["launch", a.id]); }

    // The Couch in front again (KWin's Show Desktop), at Home or a tab.
    function home(t) {
        guideShown = false;
        page = "tabs";
        tab = t || "home";
        if (!front) run(["do", "home"]);
        if (Window.window) Window.window.requestActivate();
        forceActiveFocus();
    }

    // ------------------------------------------------------------------ the Guide
    property string guideGameId: ""
    readonly property var guideWin: {
        for (const w of wins) if (w.id === guideGameId) return w;
        return null;
    }
    function openGuide() {
        const g = frontGame() || wins.filter(function (w) { return w.id === fh.lastGameId; })[0] || gameWins()[0] || null;
        guideGameId = g ? g.id : "";
        guideShown = true;
    }
    function guide(twice) {
        if (twice || guideButton === "open") {
            guideShown = false;
            if (page === "switcher" && front) { page = "tabs"; return; }
            page = "switcher";
            if (!front) run(["do", "home"]);
            if (Window.window) Window.window.requestActivate();
            return;
        }
        if (guideShown) { guideShown = false; return; }
        if (page === "switcher") { page = "tabs"; return; }
        openGuide();
    }
    // The Guide's window is made with the Couch already set (initial properties), so nothing in it reads a null Couch.
    Loader {
        id: guideLoader
        active: fh.guideShown
        property bool fallback: false
        onActiveChanged: if (active) setSource(fallback ? "GuideWindow.qml" : "GuideLayer.qml", { couch: fh })
        onStatusChanged: if (status === Loader.Error && !fallback) { fallback = true; setSource("GuideWindow.qml", { couch: fh }); }
    }

    // Requests from hearth-firehold-plasma ask (the Guide button, Meta+Ctrl+G, the Desk's Home and Library): its
    // listener sleeps until one comes, then the Couch starts it again. Plasma's scripting (the "request" key) too.
    function handle(cmd) {
        if (cmd === "guide") guide(false);
        else if (cmd === "open") guide(true);
        else if (cmd === "home") home("home");
        else if (cmd === "settings") { home("home"); page = "settings"; }
        else if (cmd.indexOf("tab:") === 0) home(cmd.slice(4));
        else if (cmd === "cards") cardsNow();
        // A controller button by name (the same keys hearth-firehold-plasma's controller sends; the checks use it).
        else if (cmd.indexOf("key:") === 0) {
            const m = { up: Qt.Key_Up, down: Qt.Key_Down, left: Qt.Key_Left, right: Qt.Key_Right, a: Qt.Key_Return, b: Qt.Key_Escape,
                        x: Qt.Key_X, y: Qt.Key_Y, lb: Qt.Key_PageUp, rb: Qt.Key_PageDown, lt: Qt.Key_Comma, rt: Qt.Key_Period };
            if (m[cmd.slice(4)] !== undefined) key(m[cmd.slice(4)]);
        }
    }
    property int listenFails: 0
    function listen() {
        run(["listen"], function (t) {
            const l = t.trim();
            if (l === "" || l.indexOf("error") === 0) { fh.listenFails += 1; if (fh.listenFails < 3) fh.listen(); return; }
            fh.listenFails = 0;
            fh.handle(l);
            fh.listen();
        });
    }
    readonly property string request: Plasmoid.configuration.request || ""
    onRequestChanged: if (started && request !== "") handle(request.split("|")[0])

    // ------------------------------------------------------------------ the keys (the controller sends these too)
    focus: true
    Keys.onPressed: function (e) {
        if (fh.key(e.key)) e.accepted = true;
    }
    readonly property var tabs: ["home", "library", "apps", "media", "store"]
    function key(k) {
        if (k === Qt.Key_F1) { guide(false); return true; }
        if (k === Qt.Key_Menu || k === Qt.Key_F2) { page = page === "settings" ? "tabs" : "settings"; return true; }
        if (optionsGame) return optionsPage.key(k);
        if (page === "switcher") return switcherPage.key(k);
        if (page === "settings") return settingsPage.key(k);
        if (k === Qt.Key_PageUp || k === Qt.Key_PageDown) {
            const i = tabs.indexOf(tab) + (k === Qt.Key_PageUp ? -1 : 1);
            if (i >= 0 && i < tabs.length) tab = tabs[i];
            return true;
        }
        if (tab === "home") return homePage.key(k);
        if (tab === "library") return libraryPage.key(k);
        return appsPage.key(k);
    }

    Component.onCompleted: {
        run(["get"], function (t) { const o = fh.lastJson(t); if (o && typeof o === "object") fh.cfg = o; });
        run(["tab"], function (t) {
            const v = t.trim();
            if (fh.tabs.indexOf(v) >= 0) fh.tab = v;
            else if (v === "switcher" || v === "settings") fh.page = v;
        });
        run(["cached"], function (t) { fh.take(t, true); });
        run(["library"], function (t) { fh.take(t, false); });
        run(["apps"], function (t) { const a = fh.lastJson(t); if (Array.isArray(a)) fh.apps = a; });
        refreshSys(false);
        readWins();
        started = true;
        forceActiveFocus();
        run(["state", front ? "front" : "back"]);
        listen();
        cardsNow();
        run(["ready", "org.hearthos.firehold"]);
    }

    // ------------------------------------------------------------------ drawing
    // The mockups' warm background: a glow at the top right over a dark brown fall.
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#1d120d" }
            GradientStop { position: 1.0; color: "#0f0b0a" }
        }
    }
    Item {
        anchors.fill: parent
        clip: true
        Shape {
            x: fh.width * 0.75 - 1200 * fh.k; y: fh.height * 0.10 - 700 * fh.k
            width: 2400 * fh.k; height: 1400 * fh.k
            ShapePath {
                strokeWidth: -1
                fillGradient: RadialGradient {
                    centerX: 1200 * fh.k; centerY: 700 * fh.k; centerRadius: 1200 * fh.k
                    focalX: centerX; focalY: centerY
                    GradientStop { position: 0.0; color: "#3a1a0b" }
                    GradientStop { position: 0.7; color: "#003a1a0b" }
                }
                startX: 0; startY: 0
                PathLine { x: 2400 * fh.k; y: 0 }
                PathLine { x: 2400 * fh.k; y: 1400 * fh.k }
                PathLine { x: 0; y: 1400 * fh.k }
                PathLine { x: 0; y: 0 }
            }
            transform: Scale { origin.x: 1200 * fh.k; origin.y: 700 * fh.k; yScale: 700 / 1200 }
        }
    }

    TopBar {
        id: topBar
        couch: fh
        visible: fh.page === "tabs"
        x: 72 * fh.k; y: 36 * fh.k
        width: parent.width - 144 * fh.k
        height: 48 * fh.k
    }
    Home { id: homePage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && fh.tab === "home" }
    Library { id: libraryPage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && fh.tab === "library" }
    AppsPage { id: appsPage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && (fh.tab === "apps" || fh.tab === "media" || fh.tab === "store") }
    Switcher { id: switcherPage; couch: fh; anchors.fill: parent; visible: fh.page === "switcher" }
    CouchSettings { id: settingsPage; couch: fh; anchors.fill: parent; visible: fh.page === "settings" }
    // Game options (mockup 03): over whatever tab it was opened from.
    GameOptions { id: optionsPage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && fh.optionsGame !== null }
}
