// `hearth-qml --self-test [--shots DIR]` (CI, and the image check): goes through every screen against the core it was
// given (a fake one with sample games in CI, the real one in the image), saves a picture of each, and leaves 0 when
// every step did what it should and no screen had a QML error or warning.
import QtQuick

QtObject {
    id: st
    property var mainWindow: null
    property int step: 0
    property var fails: []
    property var steps: []
    property string dir: hostApp.shotsDir
    property int shotNo: 0
    property bool haveGames: false

    function log(t) { hostApp.log("self-test: " + t); }
    function check(ok, what) { if (!ok) { fails.push(what); log("FAILED: " + what); } else log("ok: " + what); }
    function shot(name, w) {
        if (!dir) return;
        const target = (w || mainWindow).contentItem;
        const n = ("0" + (++shotNo)).slice(-2) + "-" + name + ".png";
        target.grabToImage(r => { const ok = r.saveToFile(st.dir + "/" + n); st.log((ok ? "picture " : "couldn't save ") + n); });
    }
    function first() { const L = App.visibleGames(); return L.find(x => x.inst) || L[0] || null; }
    function dl() { return App.games.find(x => App.isActive(x)) || null; }

    function start() {
        if (dir) hostApp.makeDir(dir);
        steps = [
            [1500, () => { check(App.connection === "ready", "the core answered (host.info, library)"); }],
            [10, () => { if (/fake-core/.test(hostApp.env("HEARTH_CORE_BIN"))) { check(!!App.hostInfo.version, "host.info answered (the channel works both ways)"); } }],
            [2500, () => { if (/fake-core/.test(hostApp.env("HEARTH_CORE_BIN"))) check(App.games.length > 0, "the fake core's library loaded"); haveGames = App.games.length > 0; check(App.view === "home", "Home is the first page"); log(App.games.length + " games"); shot("home"); }],
            [700, () => { Nav.move("right"); Nav.move("right"); }],
            [700, () => { check(!!Nav.focused, "the selection moves with the D-pad"); shot("home-selection"); }],
            [300, () => { App.navigate("library"); }],
            [1200, () => { check(App.view === "library", "Library opens"); shot("library"); }],
            [300, () => { App.tabStep(1); }],
            [900, () => { check(App.libTab === "inst", "LB / RB change the Library tab"); shot("library-installed"); }],
            [300, () => { App.tabStep(-1); }],
            [300, () => { App.libFilter = "unplayed"; }],
            [700, () => { check(App.libFilter === "unplayed", "the Library's Show filter"); shot("library-never-played"); App.libFilter = ""; }],
            [300, () => { App.openLayer("search"); }],
            [700, () => { mainWindow.searchLayer.typeText(haveGames ? first().title.slice(0, 4) : "kart"); }],
            [900, () => { check(App.layerTop() === "search" && App.query.length > 0, "Search types with the on-screen keyboard's field"); shot("search"); }],
            [300, () => {}],
            [300, () => { mainWindow.searchLayer.padButton("b", true); mainWindow.searchLayer.padButton("b", false); }],
            [500, () => { App.closeLayer("search"); App.query = ""; if (haveGames) App.navigate("game", { gameId: first().id }); }],
            [1600, () => { if (haveGames) { check(App.view === "game", "A game's page opens"); shot("game"); } }],
            [300, () => { if (haveGames) mainWindow.gamePage().openSettings(); }],
            [900, () => { if (haveGames) { check(App.layerTop() === "sheet" && App.sheet && App.sheet.items.length >= 3, "Game settings open (game.options)"); shot("game-settings"); } }],
            [300, () => { App.closeLayer("sheet"); }],
            [300, () => { if (haveGames && App.playable(first())) App.launch(first()); }],
            [400, () => { if (haveGames && App.playable(first())) shot("starting"); }],
            [2600, () => { if (haveGames && App.playable(first())) check(App.running === first().id, "Play starts the game (game.started)"); App.openLayer("qam"); }],
            [1000, () => { check(App.layerTop() === "qam", "Quick Access opens"); shot("quick-access"); }],
            [300, () => { App.tabStep(1); }],
            [1600, () => { shot("quick-access-performance"); }],
            [300, () => { App.tabStep(1); }],
            [1500, () => { shot("quick-access-quick-settings"); }],
            [300, () => { App.tabStep(1); }],
            [900, () => { check(App.notes.length > 0, "Quick Access › Notifications keeps what Hearth said"); shot("quick-access-notifications"); }],
            [300, () => { App.closeLayer("qam"); if (App.running) App.exitGame(); }],
            [1500, () => { check(!App.running, "Exit game ends it (game.exited)"); App.navigate("downloads"); }],
            [1500, () => { check(App.view === "downloads", "Downloads opens"); shot("downloads"); }],
            [300, () => { App.openLayer("power"); }],
            [800, () => { check(App.layerTop() === "power", "the power menu opens"); shot("power"); }],
            [300, () => { App.closeLayer("power"); }],
            [300, () => { App.navigate("settings"); }],
            [1200, () => { check(App.view === "settings", "Settings opens"); shot("settings-renderer"); }],
            [300, () => { mainWindow.settingsPage().pickRenderer(); }],
            [900, () => { check(App.layerTop() === "sheet" && App.sheet && App.sheet.items.length >= 3, "Settings › Renderer offers the renderers"); shot("renderer-choices"); }],
            [300, () => { App.closeLayer("sheet"); }],
            [500, () => { App.tabStep(1); }],
            [900, () => { shot("settings-system"); }],
            [300, () => { App.navigate("missing", { missing: "Store" }); }],
            [900, () => { check(App.view === "missing", "a page that isn't here yet says so"); shot("not-here-yet"); }],
            [300, () => { App.openLayer("menu"); }],
            [900, () => { check(App.layerTop() === "menu", "the menu opens"); shot("menu"); }],
            [300, () => { App.closeLayer("menu"); }],
            [300, () => { mainWindow.makeOverlay(); }],
            [800, () => { if (App.overlayWin) { App.overlayWin.show(); App.overlayEvent("overlay.shown", {}); } }],
            [1200, () => { check(!!App.overlayWin && App.overlayWin.shown, "Quick Access over a game (the overlay window)"); if (App.overlayWin) shot("overlay", App.overlayWin); }],
            [300, () => { if (App.overlayWin) App.overlayEvent("overlay.hidden", {}); App.navigate("home"); }],
            [1200, () => finish()]
        ];
        next();
    }
    function next() {
        if (step >= steps.length) return;
        const s = steps[step++];
        App.later(s[0], () => { try { s[1](); } catch (e) { st.check(false, "step " + st.step + " threw: " + e); } st.next(); });
    }
    function finish() {
        const warns = hostApp.warnings();
        for (const w of warns) log("QML: " + w);
        log(fails.length + " failed step(s), " + warns.length + " QML warning(s)");
        App.later(800, () => hostApp.quit(fails.length || warns.length ? 1 : 0));
    }
}
