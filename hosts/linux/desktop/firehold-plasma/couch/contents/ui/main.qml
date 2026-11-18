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
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as P5Support
import org.kde.taskmanager as TaskManager
import "firehold.js" as F

ContainmentItem {
    id: fh

    readonly property real k: Math.max(0.5, Math.min(width / 1920, height / 1080))
    readonly property string fontName: F.font("Noto Sans")
    readonly property QtObject pal: QtObject {
        readonly property color bg: "#120c09"
        readonly property color bg2: "#2a160c"
        readonly property color text: "#f6ede6"
        readonly property color dim: "#a3958b"
        readonly property color faint: "#6f625a"
        readonly property color card: "#1d1512"
        readonly property color card2: "#2c221d"
        readonly property color line: "#3a2d26"
        readonly property color accent: "#ff8a3d"
        readonly property color gold: "#f6c56b"
        readonly property color green: "#8fe0a8"
        readonly property color red: "#f29a9a"
        readonly property color blue: "#79c0f2"
        readonly property color yellow: "#f5d36a"
    }

    // ------------------------------------------------------------------ what's on screen (runtime only)
    property string tab: "home"            // home | library | apps | store
    property string page: "tabs"           // tabs | switcher (Everything open) | settings
    property bool guideShown: false
    property bool started: false
    readonly property bool front: Window.active
    onFrontChanged: {
        if (!started) return;
        run(["state", front ? "front" : (guideShown ? "guide" : "back")]);
        if (front) { forceActiveFocus(); refreshSys(false); }
    }
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
    readonly property var tabs: ["home", "library", "apps", "store"]
    function key(k) {
        if (k === Qt.Key_F1) { guide(false); return true; }
        if (k === Qt.Key_Menu || k === Qt.Key_F2) { page = page === "settings" ? "tabs" : "settings"; return true; }
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
        run(["ready", "org.hearthos.firehold"]);
    }

    // ------------------------------------------------------------------ drawing
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: fh.pal.bg }
            GradientStop { position: 0.55; color: fh.page === "switcher" ? "#1e0f08" : fh.tab === "home" && fh.page === "tabs" ? "#2a1409" : "#1c0f08" }
            GradientStop { position: 1.0; color: fh.pal.bg }
        }
    }
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "transparent" }
            GradientStop { position: 1.0; color: "#c00b0806" }
        }
    }

    TopBar {
        id: topBar
        couch: fh
        visible: fh.page === "tabs"
        x: 72 * fh.k; y: 44 * fh.k
        width: parent.width - 144 * fh.k
        height: 48 * fh.k
    }
    Home { id: homePage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && fh.tab === "home" }
    Library { id: libraryPage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && fh.tab === "library" }
    AppsPage { id: appsPage; couch: fh; anchors.fill: parent; visible: fh.page === "tabs" && (fh.tab === "apps" || fh.tab === "store") }
    Switcher { id: switcherPage; couch: fh; anchors.fill: parent; visible: fh.page === "switcher" }
    CouchSettings { id: settingsPage; couch: fh; anchors.fill: parent; visible: fh.page === "settings" }
}
