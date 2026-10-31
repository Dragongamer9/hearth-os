pragma Singleton
// Hearth's state for the Qt renderer, and the bridge to the Hearth core: the page side of the channel in
// hosts/hearth-rs/src/winhost.rs (calls, results, events, and what the core asks of the windows). Everything Hearth
// knows and does is the core's (docs/bridge.md lists the calls); this only keeps what the screens show.
import QtQuick

QtObject {
    id: app

    // ------------------------------------------------------------------ what the screens show
    property var hostInfo: ({})
    property var games: []                 // the web page's shape (fromHost)
    property int rev: 0                    // bumped when anything in `games` changed
    property bool libLoaded: false
    property bool libLoading: false
    property string running: ""
    property double runningSince: 0
    property bool paused: false
    property var profile: ({ name: "Player One", accent: "#3fd0b8", background: "default" })
    property var sys: null
    property var perf: null
    property var renderer: null
    property var dl: ({ speeds: [], recent: [] })
    property var accounts: null
    property bool padsAllowed: true        // Game Mode says whether this window is the one on screen (hostScreen)
    property bool pointer: false           // the mouse was used last (hover selects)
    property string connection: "starting" // starting · ready · gone

    // Pages and layers
    property string view: "home"
    property string gameId: ""
    property string missing: ""            // MissingView: what isn't in the Qt renderer yet
    property var history: []
    property var layers: []                // "menu", "qam", "power", "sheet", "search", "launch" (top last)
    property string qamTab: "now"
    property var sheet: null               // {title, sub, items:[{label, icon, run, danger, on}]}
    property string libTab: "all"
    property string libFilter: ""
    property string libSort: "alpha"
    property string query: ""
    property string launchId: ""
    property string launchText: ""
    property bool launchWaiting: false
    property string toastText: ""
    property int toastSeq: 0
    property var notes: []                 // Quick Access › Notifications: what Hearth said (newest first, the last 40)
    property var osd: null

    // The overlay window (Quick Access over games; not on the Handheld Daemon editions, where HHD's app shows it)
    property var overlayWin: null
    property var mainWin: null

    signal libraryChanged()
    signal eventArrived(string name, var data)
    signal overlayEvent(string name, var data)
    signal logLine(string text)
    signal tabStepped(int dir)            // LB / RB: the page (or layer) on top moves between its tabs
    function tabStep(d) { tabStepped(d); }

    readonly property bool linux: true
    function hasFeat(f) { return (hostInfo.features || []).indexOf(f) >= 0; }

    // ------------------------------------------------------------------ timers (QML has no setTimeout)
    property Component timerComp: Component { Timer { repeat: false; running: true } }
    function later(ms, fn) {
        const t = timerComp.createObject(app, { interval: Math.max(0, ms) });
        t.triggered.connect(function () { try { fn(); } finally { t.destroy(); } });
        return t;
    }

    // ------------------------------------------------------------------ the channel
    property int nextId: 1
    property var pending: ({})
    property var artWait: ({})
    property var artCache: ({})
    property int artNext: 1
    property bool evalTold: false

    function raw(obj) { if (typeof coreLink !== "undefined") coreLink.send(JSON.stringify(obj)); }
    /** A message from a page, as the web page's nimbusHost.postMessage. */
    function send(msg, win) { raw({ t: "page", win: win || "main", msg: msg }); }
    function log(text) { hostApp.log(text); send({ type: "log", text: "qt: " + text }); }
    /** A call to the core: a Promise of its answer (docs/bridge.md). */
    function call(method, args, timeoutMs, win) {
        return new Promise(function (resolve, reject) {
            const id = app.nextId++;
            app.pending[id] = { resolve: resolve, reject: reject, method: method, at: Date.now(), timeout: timeoutMs || 30000 };
            app.send({ type: "call", id: id, method: method, args: args || {} }, win);
        });
    }
    property Timer callTimer: Timer {
        interval: 1000; repeat: true; running: true
        onTriggered: {
            const now = Date.now();
            for (const k in app.pending) {
                const p = app.pending[k];
                if (now - p.at > p.timeout) { delete app.pending[k]; app.log("call " + p.method + " timed out"); p.reject(new Error("Hearth didn't answer in time")); }
            }
        }
    }

    // Lines from the core arrive through Main.qml (a Connections there; coreLink.line → App.fromCore).
    property int linesIn: 0
    function fromCore(l) {
        if (linesIn++ === 0) hostApp.log("core: first line in");
        let m;
        try { m = JSON.parse(l); } catch (e) { return; }
        if (!m || typeof m.t !== "string") return;
        switch (m.t) {
        case "msg":
            if (m.to === "main" || m.to === "*") onPageMsg(m.msg, "main");
            if ((m.to === "overlay" || m.to === "*") && overlayWin) onPageMsg(m.msg, "overlay");
            return;
        case "do": return act(m);
        case "ask":
            // File and folder pickers aren't in the Qt renderer yet: "cancelled", and said so.
            toast("Choosing a file isn't in the Qt renderer yet. Open the web renderer for that (Settings › Display › Renderer).");
            raw({ t: "answer", id: m.id, value: null });
            return;
        case "art": {
            const cb = artWait[m.id];
            if (cb) { delete artWait[m.id]; cb(m.path || ""); }
            return;
        }
        }
    }

    function onPageMsg(msg, win) {
        if (!msg) return;
        if (msg.type === "result") {
            const p = pending[msg.id];
            if (!p) return;
            delete pending[msg.id];
            if (msg.ok === false) p.reject(new Error(msg.error || "That didn't work"));
            else p.resolve(msg.value === undefined ? null : msg.value);
            return;
        }
        if (msg.type === "toast") { if (win === "main") toast(msg.text); else overlayEvent("toast", { text: msg.text }); return; }
        if (msg.type === "event") {
            if (win === "overlay") overlayEvent(msg.name, msg.data || {});
            else onEvent(msg.name, msg.data || {});
            return;
        }
        if (msg.type === "restart") return;   // the core restarts itself (main.rs)
    }

    // What the core asks of the windows (winhost.rs "do").
    function act(m) {
        const w = m.win === "overlay" ? overlayWin : m.win === "main" ? mainWin : null;
        switch (m.op) {
        case "show": if (w) w.showFromCore(); break;
        case "hide": if (w) w.hide(); break;
        case "focus": if (w) { w.raise(); w.requestActivate(); } break;
        case "unminimize": if (w && w.visibility === Window.Minimized) w.showFromCore(); break;
        case "destroy": if (m.win === "overlay" && overlayWin) { overlayWin.destroy(); overlayWin = null; reportWin("overlay", null); } break;
        case "eval": if (!evalTold) { evalTold = true; log("the core asked to run a script in the page; the Qt renderer has no scripts (ignored from now on)"); } break;
        case "exit": hostApp.quit(Number(m.code) || 0); break;
        case "restart": hostApp.restartHearth(); break;
        case "zoom": { const p = Number(m.pct); Theme.zoom = (p > 0 && p <= 500 ? p : 100) / 100; break; }
        case "overlay.create": if (mainWin) mainWin.makeOverlay(); break;
        case "hotkey": log("Ctrl+Alt+Q: the Qt renderer leaves it to the core's own Guide button reading"); break;
        case "stream.open":
            raw({ t: "stream.closed" });
            notHere("Xbox Cloud Gaming");
            break;
        case "signin.open":
            raw({ t: "signin.closed" });
            notHere("Signing in to a store");
            break;
        case "storeweb.open":
            raw({ t: "storeweb.closed", store: m.store || "" });
            notHere("The store's own pages");
            break;
        case "stream.close": case "signin.close": case "storeweb.close": break;
        }
    }
    function reportWin(label, w) {
        if (!w) { raw({ t: "win", win: label, exists: false, visible: false, focused: false, title: "" }); return; }
        raw({ t: "win", win: label, exists: true, visible: w.visible, focused: w.active, title: w.title });
    }

    // ------------------------------------------------------------------ art (nimbus-art:// addresses: the core says which file)
    function art(url, cb) {
        if (!url) { cb(""); return; }
        if (/^https?:|^file:|^qrc:/.test(url)) { cb(url); return; }
        if (artCache[url] !== undefined) { cb(artCache[url]); return; }
        const id = artNext++;
        artWait[id] = function (path) { const u = path ? hostApp.fileUrl(path) : ""; app.artCache[url] = u; cb(u); };
        raw({ t: "art", id: id, uri: url });
    }

    // ------------------------------------------------------------------ messages on screen
    function toast(t) { if (!t) return; toastText = String(t); toastSeq++; hostApp.log("toast: " + toastText); notes = [{ text: toastText, at: Date.now() }].concat(notes).slice(0, 40); }
    function notHere(what) {
        toast(what + " isn't in the Qt renderer yet. Select Open in the web renderer to use it.");
        missing = what;
        openSheet({ title: what + " isn't here yet", sub: "The Qt renderer is Experimental and doesn't have this yet. The web renderer has everything: Hearth starts again with it, this once.",
                    items: [{ label: "Open in the web renderer", icon: "globe", run: () => openInWeb() }] });
    }
    function openInWeb() {
        log("Open in the web renderer");
        toast("Starting Hearth again with the web renderer…");
        call("display.renderer", { once: "web" }).then(() => later(400, () => call("app.restart").catch(e => toast(e.message)))).catch(e => toast(e.message));
    }
    function pointerUsed() { pointer = true; }

    // ------------------------------------------------------------------ the library
    function fromHost(d) {
        return { id: d.id, title: d.title || "", src: d.source || "manual", fav: !!d.fav, hidden: !!d.hidden, hrs: (d.minutes || 0) / 60,
                 last: d.lastPlayed || 0, added: d.added || 0, sizeBytes: d.sizeBytes || 0, art: d.art || {}, path: d.path || "",
                 canRemove: !!d.canRemove, inst: d.installed !== false, cloud: !!d.cloud, state: d.state || "installed",
                 progress: d.progress, bytesDone: d.bytesDone, bytesTotal: d.bytesTotal, speed: d.bytesPerSec || 0,
                 canUninstall: !!d.canUninstall, note: d.note || null, installKind: d.installKind || null, system: d.system || null,
                 stage: d.stage || null, beacon: d.beacon || null };
    }
    function g(id) { for (const x of games) if (x.id === id) return x; return null; }
    function isActive(x) { return !!x && ["downloading", "queued", "paused", "updating"].indexOf(x.state) >= 0; }
    function playable(x) { return !!x && (x.inst || x.cloud); }
    function visibleGames() { return games.filter(x => !x.hidden); }
    function sourceName(x) { return x ? (x.system || Theme.sourceNames[x.src] || x.src) : ""; }
    function stateLabel(x) {
        const L = { downloading: "Downloading", queued: "Queued", paused: "Paused", updating: "Updating", installed: "Installed", notInstalled: "Not installed", cloud: "Cloud" };
        const pct = x.progress != null ? Math.round(x.progress * 100) : null;
        return (x.stage || L[x.state] || x.state) + (pct != null && x.state !== "queued" && !/^Stopped/.test(x.stage || "") ? " " + pct + "%" : "");
    }
    function loadLibrary(rescan) {
        libLoading = true;
        return call("library.get", { rescan: !!rescan }, 120000).then(lib => {
            if (lib && lib.games) { games = lib.games.map(fromHost); rev++; libLoaded = true; libraryChanged(); }
            if (lib && lib.runningId) running = lib.runningId;
            if (games.some(isActive)) pollDownloads(true);
        }).catch(e => toast("Couldn't load your library: " + e.message)).then(() => { libLoading = false; });
    }
    property bool dlPolling: false
    function pollDownloads(force) {
        if (dlPolling && !force) return;
        dlPolling = true;
        call("library.downloads").then(r => {
            if (!r) return;
            if (r.speeds) dl = Object.assign({}, dl, { speeds: r.speeds });
            if (r.recent) dl = Object.assign({}, dl, { recent: r.recent });
            let changed = false;
            const list = games.slice();
            for (const d of (r.active || [])) {
                const i = list.findIndex(x => x.id === d.id);
                const n = fromHost(d);
                if (i >= 0) { n.fav = list[i].fav; n.hidden = list[i].hidden; list[i] = n; } else list.push(n);
                changed = true;
            }
            // Finished ones: back to what the library says.
            if (r.changed) { loadLibrary(false); }
            if (changed) { games = list; rev++; }
        }).catch(() => {}).then(() => {
            const busy = games.some(isActive);
            if (busy || view === "downloads") later(view === "downloads" ? 2000 : 10000, () => { app.dlPolling = false; app.pollDownloads(); });
            else dlPolling = false;
        });
    }
    function setGame(id, patch) {
        const list = games.slice(), i = list.findIndex(x => x.id === id);
        if (i < 0) return;
        list[i] = Object.assign({}, list[i], patch);
        games = list; rev++;
    }

    // ------------------------------------------------------------------ start
    function boot() {
        connection = "ready";
        send({ type: "ready" });
        call("host.info").then(i => {
            hostInfo = i || {};
            log("connected: Hearth " + (hostInfo.version || "?") + ", renderer " + (hostInfo.renderer || "?") + ", " + (hostInfo.features || []).length + " features");
        }).catch(e => log("host.info: " + e.message));
        call("profile.get").then(p => { if (p) { profile = p; Theme.applyProfile(p); } }).catch(() => {});
        call("sys.status").then(s => { sys = s; }).catch(() => {});
        call("display.renderer").then(r => { renderer = r; }).catch(() => {});
        loadLibrary(false);
    }

    // ------------------------------------------------------------------ events from the core
    function onEvent(name, data) {
        eventArrived(name, data);
        switch (name) {
        case "library.changed": case "library.art": case "accounts.changed": return loadLibrary(false);
        case "install.done":
            if (data.error === "cancelled") toast("Cancelled the download of " + (data.title || "the game"));
            else if (data.error) toast((data.title || "The game") + " didn't install: " + data.error);
            else toast((data.title || "The game") + " is installed and ready to play");
            return loadLibrary(true);
        case "game.started": return gameStarted(data.id);
        case "game.ready": return gameReady(data.id);
        case "game.exited": return gameEnded(data);
        case "game.timeout": hideLaunch(); return toast("Hearth couldn't see " + ((g(data.id) || {}).title || "the game") + " start. If it's running, it still works; it just won't show here.");
        case "game.slow": if (launchWaiting) launchText = data.firstRun ? "The first start of a Windows game sets up Proton and can take a few minutes." : "Still starting. It comes to the front by itself when it's ready."; return;
        case "status": sys = data; return;
        case "hostScreen": padsAllowed = !data || data.pads !== false; return;
        case "pad": if (padsAllowed && data.down !== false && mainWin) mainWin.hostPad(data.b); return;
        case "power.menu": closeAll(); return openLayer("power");
        case "overlay.unavailable": if (layerTop() === "launch") { openLayer("qam"); } return;
        case "osd": osd = data; osdSeq++; return;
        case "profiles.changed": return call("profile.get").then(p => { if (p) { profile = p; Theme.applyProfile(p); } }).catch(() => {});
        case "open": if (["home", "library", "downloads", "settings"].indexOf(data.view) >= 0) navigate(data.view); return;
        case "watchdog": return toast(data && data.code ? "Hearth got stuck. Report sent: " + data.code : "Hearth got stuck. It's freeing itself.");
        case "net": return toast(data.online ? "Back online" : "You're offline. Installed games and your library still work.");
        }
    }
    property int osdSeq: 0

    // ------------------------------------------------------------------ pages and layers
    function navigate(v, params, push) {
        if (push !== false) history = history.concat([{ view: view, gameId: gameId, key: Nav.focused && Nav.scope === "view" ? Nav.focused.navKey : "" }]);
        const p = params || {};
        if (p.gameId !== undefined) gameId = p.gameId;
        if (p.missing !== undefined) missing = p.missing;
        closeAll();
        view = v;
        log("page: " + v + (v === "game" ? " " + gameId : ""));
        if (v === "downloads") pollDownloads(true);
        later(40, () => { const r = Nav.remembered; delete r.view; Nav.remembered = r; Nav.reset("view"); if (p.focusKey) Nav.focusKey(p.focusKey); });
    }
    function goBack() {
        if (!history.length) return false;
        const h = history[history.length - 1];
        history = history.slice(0, -1);
        view = h.view; gameId = h.gameId;
        later(40, () => { Nav.reset("view", false); if (!h.key || !Nav.focusKey(h.key)) Nav.reset("view"); });
        return true;
    }
    function layerTop() { return layers.length ? layers[layers.length - 1] : ""; }
    function openLayer(name) {
        if (layerTop() === name) return;
        layers = layers.filter(x => x !== name).concat([name]);
        if (name === "qam") call("perf.stats").then(r => { perf = r; if (r && r.runningId !== undefined) running = r.runningId || ""; }).catch(() => {});
        later(30, () => Nav.reset(name));
    }
    function closeLayer(name) {
        const top = name || layerTop();
        if (!top) return;
        layers = layers.filter(x => x !== top);
        // Quick Access closed while a game runs (Hearth was shown over it): the game comes back (gamemode.rs resume_from_main).
        later(30, () => Nav.reset(layerTop() || "view"));
    }
    function closeAll() { if (layers.indexOf("launch") >= 0) layers = ["launch"]; else layers = []; Nav.scope = layerTop() || "view"; }
    function openSheet(s) { sheet = s; layers = layers.filter(x => x !== "sheet").concat(["sheet"]); later(30, () => Nav.reset("sheet", false)); }
    function back() {
        const top = layerTop();
        if (top === "launch") { if (launchWaiting) { launchWaiting = false; hideLaunch(); send({ type: "resume" }); } return true; }
        if (top) { closeLayer(); if (top === "qam" && running) send({ type: "resume" }); return true; }
        return goBack();
    }

    // ------------------------------------------------------------------ games
    function launch(x) {
        if (!x) return;
        if (running === x.id) { closeAll(); send({ type: "resume" }); return; }
        if (running && running !== x.id) { toast("Exit " + ((g(running) || {}).title || "the game that's running") + " first (Quick Access › Now playing)"); return; }
        launchId = x.id;
        launchText = x.cloud ? "Streaming with Xbox Cloud Gaming…" : "Starting via " + sourceName(x) + "…";
        launchWaiting = false;
        closeAll();
        openLayer("launch");
        log("play " + x.id);
        call("game.launch", { id: x.id }, 60000).then(r => {
            if (r && r.tracked === false) { later(2500, () => { hideLaunch(); toast("Started " + x.title); }); return; }
            if (r && r.expired) toast(x.title + " is starting offline: your store sign-in expired, so cloud saves didn't sync.");
            later(25000, () => { if (app.launchId === x.id && app.layers.indexOf("launch") >= 0 && !app.launchWaiting) { hideLaunch(); toast(x.title + " is still starting. The store may be updating it."); } });
        }).catch(e => { hideLaunch(); toast(e.message); });
    }
    function hideLaunch() { if (layers.indexOf("launch") >= 0) { layers = layers.filter(x => x !== "launch"); Nav.reset(layerTop() || "view"); } launchWaiting = false; }
    function gameStarted(id) {
        // Linux: the game loads behind the starting screen and comes to the front once its window is up ("game.ready").
        if (layers.indexOf("launch") >= 0 && running !== id) { running = id; runningSince = Date.now(); launchWaiting = true; launchText = "Getting it ready…"; return; }
        hideLaunch();
        running = id; runningSince = Date.now();
        const x = g(id);
        if (x) { setGame(id, { last: Date.now() }); toast(x.title + " is running. Open Quick Access to exit."); }
    }
    function gameReady(id) { if (launchWaiting && running === id) { launchWaiting = false; hideLaunch(); } }
    function gameEnded(d) {
        if (d && d.id) { const i = games.findIndex(x => x.id === d.id); if (i >= 0) { const list = games.slice(); const n = fromHost(d); n.fav = list[i].fav; list[i] = n; games = list; rev++; } }
        running = ""; paused = false; launchWaiting = false; hideLaunch();
    }
    function exitGame() {
        const x = g(running);
        toast(x ? "Closing " + x.title + "…" : "Closing game…");
        closeAll();
        call("game.close", {}, 20000).then(r => { if (r && r.ok === false) toast("Some of the game's windows wouldn't close."); }).catch(e => toast(e.message));
    }
    function install(x) {
        if (!x) return;
        if (x.note) return toast(x.note + ". Hearth can't install it.");
        if (x.src === "beacon") return notHere("Setting up Beacon games");
        log("install " + x.id);
        call("game.install", { id: x.id }, 60000).then(r => {
            if (r && r.silent) { setGame(x.id, { state: "queued", stage: "Waiting for Steam" }); toast(r.waiting ? x.title + " is queued. Steam starts it when its current download or game is done." : "Steam is downloading " + x.title + " in the background. Follow it under Downloads."); }
            else if (x.installKind === "nimbus") { setGame(x.id, { state: "queued" }); toast("Downloading " + x.title + ". Follow it under Downloads."); }
            else if (x.installKind === "store") toast("The Microsoft Store is open. Install " + x.title + " there; Hearth picks it up when it's done.");
            else toast(sourceName(x) + "'s install window is open: A installs, B cancels.");
            pollDownloads(true);
        }).catch(e => toast(e.message));
    }
    function uninstall(x) {
        if (!x) return;
        openSheet({ title: "Uninstall " + x.title + "?", sub: "Its files are deleted from this PC. Your saves in the cloud stay.", items: [
            { label: "Uninstall", icon: "trash", danger: true, run: () => {
                toast("Deleting " + x.title + "…");
                log("uninstall " + x.id);
                call("game.uninstall", { id: x.id }, 15 * 60000).then(r => {
                    if (r && r.games) { games = r.games.map(fromHost); rev++; } else loadLibrary(true);
                    toast("Deleted " + x.title);
                }).catch(e => toast(e.message));
            } }] });
    }
    function setFav(x) {
        const on = !x.fav;
        setGame(x.id, { fav: on });
        call("library.setFav", { id: x.id, on: on }).catch(e => { setGame(x.id, { fav: !on }); toast(e.message); });
    }
    function dlCall(method, id) { return call(method, { id: id }).then(() => pollDownloads(true)).catch(e => toast(e.message)); }

    // ------------------------------------------------------------------ power
    function power(p) {
        const label = { sleep: "Going to sleep…", restart: "Restarting…", shutdown: "Shutting down…", signout: "Signing out…", desktop: "Switching to Mantel…" }[p] || "";
        closeAll();
        log("power " + p);
        send({ type: "power", action: p });
        toast(label);
    }
}
