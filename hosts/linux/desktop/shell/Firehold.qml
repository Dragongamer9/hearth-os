// Firehold (a hidden preview of the 2.0 desktop: one desktop, two ways to sit, games run natively, no separate Game
// Mode). Couch is for the controller and the TV: full-screen tiles (FireholdCouch.qml: Home, Library, Apps, Store and
// Everything open), the Guide button's panel over a game (FireholdGuide.qml) and its performance overlay
// (FireholdPerf.qml). Desk is Mantel as it is: windows, the bar and the dock. A controller press switches to Couch, the
// mouse or the keyboard to Desk (Automatic), and switching never closes anything (FireholdNotice.qml says it). Settings
// › Couch and Desk is FireholdSettings.qml. Works on KWin and labwc today (WinList); what needs Firehold's own
// compositor (firehold-comp) is drawn but says so.
//
// Locked, nothing of it shows or runs: it's only read from ~/.config/nimbus/firehold.json.
// Unlocked by tapping the Hearth OS version in Settings › About seven times (SettingsNav.tapVersion, which writes
// {"firehold": {"unlocked": true}}; the flat key "firehold.unlocked" is read too).
// Loaded by file name from shell.qml, so a problem in any of it only loses Firehold, never Desktop Mode.
//   quickshell ipc -p <shell folder> call firehold state | couch | desk | tab <home|library|apps|store> | open <switcher|guide|settings|home>
//                                                  | press <button> | filter <key> | search <text> | close | library
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "firehold.js" as F

Scope {
    id: fhs

    // ------------------------------------------------------------------ wired from shell.qml
    property var pad: null
    property var quick: null
    property var settingsApp: null
    property var keyboard: null
    function wire(o) { for (const k in o) fhs[k] = o[k]; }

    // ------------------------------------------------------------------ settings (~/.config/nimbus/firehold.json)
    // One source of truth, merged a key at a time through SettingsWriter; opening any page writes nothing; a damaged
    // file is put back from its .bak and nothing is written over it.
    readonly property string cfgPath: Quickshell.env("HOME") + "/.config/nimbus/firehold.json"
    property var cfg: ({})
    property bool cfgRead: false
    SettingsWriter { id: writer; path: fhs.cfgPath; history: false }
    FileView {
        id: cfgFile
        path: fhs.cfgPath
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") writer.repair();
            } else {
                const keep = {};
                for (const k in fhs.cfg) if (writer.owns(k)) keep[k] = fhs.cfg[k];
                fhs.cfg = Object.assign({}, o, keep);
            }
            fhs.cfgRead = true;
        }
        onLoadFailed: fhs.cfgRead = true
    }
    function set(key, value) {
        if (JSON.stringify(cfg[key]) === JSON.stringify(value)) return;
        const c = Object.assign({}, cfg);
        c[key] = value;
        cfg = c;
        const o = {};
        o[key] = value;
        writer.merge(o);
    }
    readonly property bool unlocked: (cfg.firehold && typeof cfg.firehold === "object" && cfg.firehold.unlocked === true) || cfg["firehold.unlocked"] === true
    readonly property string mode: ["auto", "couch", "desk"].indexOf(cfg.posture) >= 0 ? cfg.posture : "auto"
    readonly property var perScreen: cfg.perScreen && typeof cfg.perScreen === "object" ? cfg.perScreen : ({})
    readonly property string startIn: ["couch", "desk", "last"].indexOf(cfg.startIn) >= 0 ? cfg.startIn : "desk"
    readonly property string guideButton: ["quick", "open"].indexOf(cfg.guide) >= 0 ? cfg.guide : "quick"
    readonly property string gamesOpen: ["follow", "fullscreen", "window"].indexOf(cfg.games) >= 0 ? cfg.games : "follow"
    readonly property bool bigPicture: cfg.bigPicture === true
    readonly property string overlay: ["off", "basic", "full"].indexOf(cfg.overlay) >= 0 ? cfg.overlay : "off"

    // ------------------------------------------------------------------ posture (runtime: never saved but "last")
    property string posture: "desk"
    property bool started: false
    onCfgReadChanged: if (cfgRead && !started) begin()
    onUnlockedChanged: if (started) { if (!unlocked) { posture = "desk"; closeAll(); } else begin(); }
    function begin() {
        started = true;
        if (!unlocked) return;
        posture = F.startPosture(startIn, mode, lastPosture.text().trim());
        loadLibrary(true);
    }
    // "Start in: Where you left off": the posture you were in, as state (not a setting) in $XDG_RUNTIME_DIR's
    // neighbour ~/.local/state, written only when you switch.
    FileView { id: lastPosture; path: Quickshell.env("HOME") + "/.local/state/nimbus/firehold-posture"; blockLoading: true; printErrors: false }
    onPostureChanged: if (unlocked && started) Quickshell.execDetached(["sh", "-c", 'mkdir -p "$(dirname "$1")" && printf %s "$2" > "$1.tmp" && mv -f "$1.tmp" "$1"', "sh", lastPosture.path, posture])
    onModeChanged: if (unlocked && started && mode !== "auto") switchTo(mode, "")

    function postureOf(scr) { return F.screenPosture(perScreen, scr ? scr.name : "", posture); }
    function postureOfName(n) { return F.screenPosture(perScreen, n, posture); }
    readonly property bool anyCouch: unlocked && (posture === "couch" || Object.keys(perScreen).some(k => perScreen[k] === "couch"))

    // why: "pad" | "mouse" | "keys" | "" (a choice in Settings, or the Guide panel's "Put it in a window").
    function switchTo(p, why) {
        if (!unlocked || p === posture) return;
        posture = p;
        if (p === "desk") {
            guideShown = false;
            view = "home";
            if (gamesOpen === "follow") for (const w of games()) if (w.fullscreen) w.fullscreen = false;
            if (why === "mouse" || why === "keys") notice("Switched to Desk", (why === "mouse" ? "You moved the mouse." : "You typed on the keyboard.") + " Everything stayed open. Press the Guide button to go back to the Couch.");
        } else {
            const g = frontGame();
            view = g ? "app" : "home";
            if (g && gamesOpen !== "window" && !g.fullscreen) g.fullscreen = true;
            if (bigPicture) Quickshell.execDetached(["sh", "-c", "pgrep -x steam >/dev/null && exec xdg-open steam://open/bigpicture"]);
            // The press that brought you here (Guide, on the Desk) also opened the Hearth menu: the Couch instead.
            if (why === "pad") radialAway.restart();
        }
    }
    Timer { id: radialAway; interval: 160; onTriggered: if (fhs.pad && fhs.pad.radial && fhs.pad.radial.open) fhs.pad.radial.open = false }
    function input(kind) {
        if (!unlocked || !started) return;
        switchTo(F.nextPosture(mode, posture, kind), kind);
    }
    // The controller (hearth-pad's "used", at most once a second): Automatic goes to Couch.
    Connections { target: SettingsNav; function onPadAtChanged() { if (SettingsNav.padAt > 0) fhs.input("pad"); } }
    // A real mouse, touchpad or keyboard (hearth-posture reads them; never Hearth's own virtual pointer).
    Process {
        id: watcher
        command: ["python3", decodeURIComponent(Qt.resolvedUrl("hearth-posture").toString().replace(/^file:\/\//, "")), "watch"]
        running: fhs.unlocked && fhs.mode === "auto"
        stdout: SplitParser { onRead: line => { let m; try { m = JSON.parse(line); } catch (e) { return; } if (m.t === "mouse" || m.t === "keys") fhs.input(m.t); } }
        onExited: code => { if (code !== 2 && fhs.unlocked && fhs.mode === "auto") watchAgain.restart(); }
    }
    Timer { id: watchAgain; interval: 5000; onTriggered: watcher.running = fhs.unlocked && fhs.mode === "auto" }

    // ------------------------------------------------------------------ what's on screen
    // view: "home" (Firehold's own screens are up on Couch screens) or "app" (a game or window you went to is in front).
    property int redraws: 0
    // The windows you went to, most recent first (KWin raises a window you activate): how Firehold's title bars know
    // which window is above which (FireholdFrames.qml). Runtime only.
    property var recent: []
    property string frameLimit: "Off"          // the Guide's frame limit (firehold-comp), for a game window's title line
    property string deskMenu: ""               // the Desk bar's open menu (File, Edit, View), "" for none
    property string view: "home"
    property string tab: "home"          // home | library | apps | store
    property string page: "tabs"         // tabs | switcher (Everything open)
    property bool guideShown: false
    property bool settingsShown: false
    property string libFilter: "all"
    property string libQuery: ""
    readonly property bool couchUp: anyCouch && view === "home"
    Binding { target: SettingsNav; property: "fireholdShown"; value: fhs.couchUp || fhs.guideShown || fhs.settingsShown }
    Binding { target: SettingsNav; property: "fireholdCouch"; value: fhs.anyCouch }
    Component.onCompleted: SettingsNav.fireholdPad = (b, down) => fhs.padButton(b, down)
    Component.onDestruction: { SettingsNav.fireholdPad = null; SettingsNav.fireholdShown = false; SettingsNav.fireholdCouch = false; }

    function closeAll() { guideShown = false; settingsShown = false; page = "tabs"; view = "home"; }

    // ------------------------------------------------------------------ windows and games
    readonly property var wins: WinList.toplevels ? WinList.toplevels.values : []
    function entryName(w) { const e = AppIcons.entryFor(w.appId); return e ? e.name : ""; }
    readonly property var openList: F.openItems(wins, library, fhs.entryName)
    readonly property var playing: F.playingIds(wins, library)
    function games() { return wins.filter(w => F.isGame(w, library)); }
    function frontGame() {
        const a = WinList.activeToplevel;
        return a && F.isGame(a, library) ? a : null;
    }
    // When each window first showed up (the Guide panel's "Playing 1 h 12 min").
    property var seen: ({})
    onWinsChanged: {
        const s = Object.assign({}, seen), now = Date.now(), keep = {}, fresh = [];
        for (const w of wins) { const k = WinList.widOf(w) || (w.appId + "|" + w.title); keep[k] = s[k] || now; if (!s[k]) fresh.push(w); }
        seen = keep;
        // A game that just opened while you're on the Couch: in front, full screen (Games: Follow the posture).
        if (anyCouch && view === "home" && started) {
            const g = frontGame();
            if (g && !g.minimized && fresh.indexOf(g) >= 0) {
                view = "app";
                if (gamesOpen !== "window" && !launchWindowed && !g.fullscreen) g.fullscreen = true;
                launchWindowed = false;
            }
        }
    }
    function since(w) { const k = WinList.widOf(w) || (w.appId + "|" + w.title); return seen[k] || Date.now(); }
    Connections {
        target: WinList
        function onActiveToplevelChanged() {
            if (!fhs.unlocked || !fhs.started) return;
            const a = WinList.activeToplevel;
            const id = a ? WinList.widOf(a) : "";
            if (id) fhs.recent = [id].concat(fhs.recent.filter(x => x !== id)).slice(0, 50);
            // Games: Always full screen, on the Desk too.
            if (a && F.isGame(a, fhs.library) && fhs.posture === "desk" && fhs.gamesOpen === "fullscreen" && !a.fullscreen) a.fullscreen = true;
        }
    }

    // Go to a game or window: in front, the Couch steps aside (full screen on the Couch, as it was on the Desk).
    function goTo(w, asWindow) {
        if (!w) return;
        page = "tabs";
        guideShown = false;
        view = "app";
        if (w.minimized) w.minimized = false;
        const game = F.isGame(w, library);
        if (game) lastGame = w;
        if (game && posture === "couch") {
            const full = asWindow ? false : gamesOpen !== "window";
            if (w.fullscreen !== full) w.fullscreen = full;
        }
        w.activate();
    }
    function toggleFull(w) { if (w) w.fullscreen = !w.fullscreen; }
    function closeWin(w) { if (w) w.close(); }
    // The game the Guide panel is about: the one in front, else the one you went to last, else any that's running.
    property var guideWin: null
    function openGuide() { guideWin = frontGame() || (lastGame && wins.indexOf(lastGame) >= 0 ? lastGame : null) || games()[0] || null; guideShown = guideWin !== null; }
    property var lastGame: null
    function home() { page = "tabs"; tab = "home"; guideShown = false; view = "home"; }
    // From the Desk (its dock's Home and Library, the bar's tile): the Couch at that tab.
    function couchHome() { couchTab("home"); }
    function couchTab(t) { if (!unlocked) return; switchTo("couch", ""); home(); tab = t; }

    // ------------------------------------------------------------------ Everything open's window pictures (KWin)
    // KWin takes a picture of each window (hearth-firehold pictures → hearth-screens shot --window) every 3 seconds
    // while Everything open is on screen; labwc can't, so there the cards stay their colour. Runtime files only.
    property var pictures: ({})
    property int pictureTick: 0
    readonly property string pictureDir: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-firehold/pictures"
    readonly property bool picturesWanted: unlocked && WinList.kwin && couchUp && page === "switcher"
    Timer { interval: 3000; repeat: true; triggeredOnStart: true; running: fhs.picturesWanted; onTriggered: if (!picProc.running) { picProc.command = ["python3", fhs.libHelper, "pictures", fhs.pictureDir].concat(fhs.openList.map(o => WinList.widOf(o.win)).filter(i => i !== "")); picProc.running = true; } }
    Process {
        id: picProc
        stdout: StdioCollector {
            onStreamFinished: { try { fhs.pictures = JSON.parse(text.trim().split("\n").pop()) || ({}); fhs.pictureTick++; } catch (e) {} }
        }
    }
    function pictureOf(w) { const id = WinList.widOf(w); return id && pictures[id] ? "file://" + pictures[id] + "?" + pictureTick : ""; }

    // ------------------------------------------------------------------ the library (hearth-firehold)
    property var library: []
    property var counts: ({})
    property bool libraryLoaded: false
    property double libraryAt: 0
    readonly property string libHelper: decodeURIComponent(Qt.resolvedUrl("hearth-firehold").toString().replace(/^file:\/\//, ""))
    function loadLibrary(fromCache) {
        if (!unlocked) return;
        if (fromCache && !cacheProc.running && !libraryLoaded) cacheProc.running = true;
        if (!libProc.running) { libProc.running = true; libraryAt = Date.now(); }
    }
    function takeLibrary(text) {
        let d;
        try { d = JSON.parse(text.trim().split("\n").pop()); } catch (e) { return; }
        if (!d || !Array.isArray(d.games)) return;
        library = d.games;
        counts = d.counts || ({});
        libraryLoaded = true;
    }
    Process { id: cacheProc; command: ["python3", fhs.libHelper, "cached"]; stdout: StdioCollector { onStreamFinished: if (!fhs.libraryLoaded) fhs.takeLibrary(text) } }
    Process { id: libProc; command: ["python3", fhs.libHelper, "library"]; stdout: StdioCollector { onStreamFinished: fhs.takeLibrary(text) } }
    // Again when the Couch comes up (at most every 2 minutes), so a game installed meanwhile is there.
    onCouchUpChanged: if (couchUp && Date.now() - libraryAt > 120000) loadLibrary(false)
    readonly property var continueGame: F.continueGame(library, playing)
    function windowFor(g) {
        if (!g) return null;
        for (const w of wins) { const x = F.gameFor(w, library); if (x && x.id === g.id) return w; }
        return null;
    }
    function play(g, asWindow) {
        if (!g) return;
        const w = windowFor(g);
        if (w) { goTo(w, asWindow); return; }
        if (!g.installed) { run(["page", g.id], "Opening " + g.title + " in Hearth to install it"); return; }
        launchWindowed = !!asWindow;
        run(["play", g.id], "Starting " + g.title);
    }
    property bool launchWindowed: false
    function run(args, msg) {
        playProc.command = ["python3", libHelper].concat(args);
        playProc.running = true;
        if (msg) SettingsNav.toast(msg);
    }
    Process {
        id: playProc
        stdout: StdioCollector { onStreamFinished: { const l = text.trim().split("\n").pop() || ""; if (l.startsWith("error: ")) SettingsNav.toast(l.slice(7)); } }
    }
    function launchApp(entry) { if (entry) { view = "app"; entry.execute(); } }

    // ------------------------------------------------------------------ the controller
    // Guide: once (Quick Access, or Everything open as Settings says), twice quickly: Everything open.
    property int guidePresses: 0
    Timer { id: guideWait; interval: 380; onTriggered: { const n = fhs.guidePresses; fhs.guidePresses = 0; fhs.guide(n >= 2); } }
    function guide(twice) {
        if (!unlocked) return;
        if (posture !== "couch" && !anyCouch) switchTo("couch", "pad");
        if (twice || guideButton === "open") { guideShown = false; view = "home"; page = page === "switcher" ? "tabs" : "switcher"; return; }
        if (guideShown) { guideShown = false; return; }
        if (page === "switcher") { page = "tabs"; return; }
        if (view === "app") {
            const g = frontGame();
            if (g) { openGuide(); return; }
            home();
            return;
        }
        if (quick) quick.toggle();
    }
    // Every other button goes to what's in front: the Guide panel or the Couch screen on the main screen.
    property var activeCouch: null
    property var guidePanel: null
    property var settingsPanel: null
    function padButton(b, down) {
        if (!unlocked) return;
        if (b === "guide") { if (down) { guidePresses++; guideWait.restart(); } return; }
        if (!down) return;
        if (settingsShown && settingsPanel) { settingsPanel.press(b); return; }
        if (guideShown && guidePanel) { guidePanel.press(b); return; }
        if (couchUp && activeCouch) activeCouch.press(b);
    }

    // ------------------------------------------------------------------ the notice ("Switched to Desk")
    property string noticeTitle: ""
    property string noticeText: ""
    property bool noticeShown: false
    function notice(t, s) { noticeTitle = t; noticeText = s; noticeShown = true; noticeTimer.restart(); }
    Timer { id: noticeTimer; interval: 6000; onTriggered: fhs.noticeShown = false }

    // ------------------------------------------------------------------ the screens
    Variants {
        model: fhs.unlocked ? Quickshell.screens : []
        delegate: Component {
            FireholdCouch {
                required property var modelData
                screen: modelData
                scrName: modelData.name
                fh: fhs
            }
        }
    }
    LazyLoader {
        active: fhs.unlocked && fhs.guideShown
        FireholdGuide { fh: fhs }
    }
    LazyLoader {
        active: fhs.unlocked && fhs.noticeShown
        FireholdNotice { fh: fhs }
    }
    LazyLoader {
        active: fhs.unlocked && fhs.overlay !== "off" && fhs.frontGame() !== null
        FireholdPerf { fh: fhs }
    }
    // The Desk: Firehold's bar and dock (Mantel's step aside meanwhile), and a game in a window's frame (KWin).
    LazyLoader {
        active: fhs.unlocked && fhs.started && fhs.posture === "desk"
        FireholdDesk { fh: fhs }
    }
    // Every window's title bar in Firehold's look (KWin tells where the windows are).
    LazyLoader {
        active: fhs.unlocked && fhs.started && fhs.posture === "desk" && WinList.kwin
        FireholdFrames { fh: fhs }
    }
    LazyLoader {
        active: fhs.unlocked && fhs.settingsShown
        FireholdSettings { fh: fhs }
    }

    // ------------------------------------------------------------------ for checks (firehold-shell.yml) and keybinds
    IpcHandler {
        target: "firehold"
        function state(): string {
            return JSON.stringify({ unlocked: fhs.unlocked, posture: fhs.posture, mode: fhs.mode, view: fhs.view, tab: fhs.tab, page: fhs.page,
                                    guide: fhs.guideShown, settings: fhs.settingsShown, couchUp: fhs.couchUp, games: fhs.library.length,
                                    counts: fhs.counts, playing: fhs.playing, open: fhs.openList.map(o => ({ kind: o.kind, name: o.name, sub: o.sub })),
                                    notice: fhs.noticeShown ? fhs.noticeTitle : "", filter: fhs.libFilter, query: fhs.libQuery });
        }
        function couch(): void { if (fhs.unlocked) { fhs.switchTo("couch", "pad"); fhs.home(); } }
        function desk(): void { fhs.switchTo("desk", ""); }
        // As if a real mouse moved (Automatic) or a key was typed: checks of the Desk switch.
        function input(kind: string): void { fhs.input(kind); }
        function tab(name: string): void { if (fhs.unlocked && ["home", "library", "apps", "store"].indexOf(name) >= 0) { fhs.home(); fhs.tab = name; } }
        function open(what: string): void {
            if (!fhs.unlocked) return;
            if (what === "switcher") { fhs.guideShown = false; fhs.view = "home"; fhs.page = "switcher"; }
            else if (what === "guide") fhs.openGuide();
            else if (what === "settings") fhs.settingsShown = true;
            else if (what === "home") fhs.home();
        }
        // Checks: go to the window with this title, as A on its card does.
        function go(title: string): bool {
            const w = fhs.wins.find(t => t.title === title);
            if (!w) return false;
            fhs.goTo(w, false);
            return true;
        }
        function press(button: string): void { fhs.padButton(button, true); fhs.padButton(button, false); }
        function filter(key: string): void { fhs.libFilter = key; }
        function search(text: string): void { fhs.libQuery = text; }
        function library(): void { fhs.loadLibrary(false); }
        function close(): void { fhs.closeAll(); fhs.deskMenu = ""; }
        // Checks: the Desk bar's File, Edit or View menu open ("" closes it).
        function menu(name: string): void { fhs.deskMenu = name; }
        // Checks: every Couch screen drawn again whole (CI's software renderer under labwc headless can leave a partial
        // repaint shifted in a screenshot; a real screen draws whole frames).
        function redraw(): void { fhs.redraws++; }
    }
}
