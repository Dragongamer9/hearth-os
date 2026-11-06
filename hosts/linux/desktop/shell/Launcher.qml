// App launcher (Mantel 2026: a large centred panel over the darkened desktop): search, a grid of installed apps,
// plus power actions. This is the "Hearth" app menu; Settings › Appearance › Bar and dock › App menu can pick one of
// four others instead (Centred, Tiles, Start screen, Classic: MenuCentred.qml, MenuTiles.qml, MenuStart.qml,
// MenuClassic.qml), drawn by file name over this window with the lists, search and actions below; if one can't
// load, this Hearth menu shows instead. Searching in Apps also finds Settings pages and your games; Recent can hold all three.
// Opens with Super, the Nimbus button on the taskbar, or `quickshell ipc ... call launcher toggle`.
// Type to search, arrows to move, Enter to open, Esc to close.
// Two sections: Apps (every app with a .desktop file) and Games (your installed games from Hearth's library, through
// hearth-games; they start through Hearth, like its Play button). Switch with Tab / Shift+Tab, or go up from the
// top row to the section buttons and press left or right (the D-pad).
// In Apps, Hearth's own apps and the apps you installed come first; the apps that came with the system (everything
// else in /usr/share/applications) are in one "System apps" folder at the end. Searching looks through all of them.
// The apps you opened last are in a Recent row above the list (DeskState.recent). Right-click an app for its own
// actions, Pin to taskbar, Add to desktop, and Uninstall for apps from Flathub (shell.qml).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets

PanelWindow {
    id: root
    property bool shown: false
    property string tab: "apps"                // "apps" or "games"
    property bool tabFocus: false              // the section buttons have the keyboard / controller focus
    property var games: []                     // hearth-games: [{id, title, source, cover, wide, icon}]
    property bool gamesLoading: false
    property double gamesAt: 0                 // when Hearth was last asked (it takes a moment)
    property bool folderOpen: false            // the System apps folder is open
    property var systemIds: ({})               // desktop ids that came with the system image
    property var hiddenIds: ({})               // not for this desktop (OnlyShowIn/NotShowIn: docs/APP-KINDS.md)
    readonly property string gamesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-games").toString().replace(/^file:\/\//, ""))
    signal settingsRequested()
    signal pageRequested(string page)          // a Settings page found by the search (or in Recent)
    // Right-click, press and hold, or the Menu key (X on the controller): Open / Pin to taskbar (shell.qml).
    signal menuRequested(var entry, real x, real y)
    property var popMenu: null                 // PopMenu (shell.qml): the live tiles' Resize menu, jump lists
    function toggle() { shown = !shown; }
    // The Tiles menu's search field in the bar (Islands.qml, as in the mockup): opens the menu with the search ready;
    // what you type shows there (menuQuery). barSearch: the bar on the main screen shows that field (shell.qml).
    property bool barSearch: false
    readonly property string menuQuery: shown && styled && typeof styleLoader.item.query === "string" ? styleLoader.item.query : ""
    function openMenuSearch() {
        if (!shown) shown = true;
        if (styled && typeof styleLoader.item.focusSearch === "function") styleLoader.item.focusSearch();
    }
    // Y on the controller (Pad.qml) asks here first: the Tiles and Start screen menus use it to move the tile with
    // the focus ring. False: nothing took it (the on-screen keyboard opens as usual).
    function padY() { return shown && styled && typeof styleLoader.item.padY === "function" && styleLoader.item.padY() === true; }
    function openTab(t) { setTab(t); shown = true; }
    function setTab(t) {
        if (tab === t) return;
        tab = t;
        folderOpen = false;
        grid.currentIndex = 0;
        if (t === "games") loadGames();
    }
    function nextTab() { setTab(tab === "apps" ? "games" : "apps"); }
    function menuFor(i) {
        if (tab !== "apps") return;            // games: just Play (Enter)
        const e = matches[i];
        if (e && (e.folder || e.page !== undefined || e.title !== undefined)) return;   // the folder, Settings pages, games: just Open
        const cell = grid.itemAtIndex(i);
        if (!e) return;
        const p = cell ? cell.mapToItem(null, cell.width / 2, cell.height / 2) : Qt.point(width / 2, height / 2);
        menuRequested(e, p.x, p.y);
    }

    visible: shown
    onShownChanged: {
        if (!shown) { fileHits = []; return; }
        search.text = ""; folderOpen = false; grid.currentIndex = 0; tabFocus = false; loadGames(); systemList.running = true;
        if (styled) { styleLoader.item.opened(); readRecentFiles(); if (styleLoader.item.wantsLive) readLive(); }
        else search.forceActiveFocus();
    }
    function closeFolder() { folderOpen = false; grid.currentIndex = Math.max(0, mainApps.length); }

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-launcher"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: styled ? styleLoader.item.scrim : Qt.rgba(0.02, 0.02, 0.03, 0.72)

    // Every app with a .desktop file, A–Z, filtered by what you type.
    // Not the Hearth app itself (nimbus.desktop, "Hearth"): that's the archived Game Mode in a window, hidden everywhere
    // people look (docs/SETTINGS-MOVE.md); HEARTH_LEGACY_GAME_MODE=1 (the developer switch) shows it again.
    readonly property bool legacyGame: Quickshell.env("HEARTH_LEGACY_GAME_MODE") === "1"
    readonly property var apps: Array.from(DesktopEntries.applications.values)
        .filter(a => !a.noDisplay && !root.hiddenIds[a.id] && !/^(qt5ct|qt6ct)$/.test(a.id || "") && (root.legacyGame || !/^nimbus(\.desktop)?$/.test(a.id || "")))
        .sort((a, b) => a.name.localeCompare(b.name))
    // Hearth's own apps (and Beacon, Xodus Nexus, and Bazaar, the app store Hearth OS picks): never in the folder.
    function ours(a) { return /^(hearth|nimbus|beacon|xodus-nexus$|io\.github\.kolunmi\.Bazaar$)/i.test(a.id || "") || /^Hearth\b/.test(a.name || ""); }
    function systemApp(a) { return !ours(a) && root.systemIds[a.id] === true; }
    readonly property var mainApps: apps.filter(a => !systemApp(a))
    readonly property var sysApps: apps.filter(a => systemApp(a))
    readonly property var folderTile: ({ folder: true, id: "", name: "System apps", icon: "folder", genericName: "", comment: "", keywords: [] })
    readonly property var matches: {
        const q = search.text.trim().toLowerCase();
        if (tab === "games") {
            if (!q) return games;
            return games.filter(g => (g.title || "").toLowerCase().includes(q))
                .sort((a, b) => (b.title.toLowerCase().startsWith(q) ? 1 : 0) - (a.title.toLowerCase().startsWith(q) ? 1 : 0));
        }
        if (!q) return folderOpen ? sysApps : (sysApps.length ? mainApps.concat([folderTile]) : mainApps);
        const hit = a => [a.name, a.genericName, a.comment, Array.from(a.keywords || []).join(" ")].join(" ").toLowerCase().includes(q);
        // Names that start with the query first; then Settings pages and games that match.
        const found = apps.filter(hit).sort((a, b) => (b.name.toLowerCase().startsWith(q) ? 1 : 0) - (a.name.toLowerCase().startsWith(q) ? 1 : 0));
        const pages = settingsPages.filter(p => (p.name + " " + p.words).toLowerCase().includes(q));
        const gs = games.filter(g => (g.title || "").toLowerCase().includes(q)).slice(0, 8);
        return found.concat(pages).concat(gs);
    }

    // Settings pages for the search: [{page, name, icon, words}] from Hearth Settings' list (SettingsApp.qml).
    readonly property var settingsPages: {
        const app = SettingsNav.settingsApp;
        if (!app || !app.pages) return [];
        const kw = app.keywords || {};
        return app.pages.map(p => ({ page: p.id, name: p.label, icon: p.icon, words: kw[p.id] || "" }));
    }
    function launch(entry) {
        if (!entry) return;
        if (entry.folder) { folderOpen = true; grid.currentIndex = 0; search.forceActiveFocus(); return; }
        if (entry.page !== undefined) { DeskState.noteLaunch("page:" + entry.page); root.pageRequested(entry.page); }
        else if (entry.title !== undefined) { DeskState.noteLaunch("game:" + entry.id); Quickshell.execDetached(["python3", gamesHelper, "play", entry.id]); }
        else { entry.execute(); DeskState.noteLaunch(entry.id); }
        shown = false;
    }
    // Recent: apps, games ("game:<id>") and Settings pages ("page:<id>"), newest first.
    function recentEntry(id) {
        if (id.startsWith("page:")) return settingsPages.find(p => p.page === id.slice(5)) || null;
        if (id.startsWith("game:")) return games.find(g => "game:" + g.id === id) || null;
        return apps.find(a => a.id === id) || null;
    }

    // ==================================================================== the other app menus (Theme.appMenu)
    // Everything below is shared by the four styles: they draw it their own way.
    readonly property string style: Theme.appMenu
    readonly property var styleFiles: ({ centred: "MenuCentred.qml", tiles: "MenuTiles.qml", start: "MenuStart.qml", classic: "MenuClassic.qml" })
    readonly property bool styled: style !== "hearth" && styleLoader.status === Loader.Ready && styleLoader.item !== null
    readonly property string menuHelper: decodeURIComponent(Qt.resolvedUrl("hearth-appmenu").toString().replace(/^file:\/\//, ""))
    readonly property string userName: Quickshell.env("USER") || "You"
    function close() { shown = false; }
    // Where a style's panel goes: next to the app menu button, wherever the bar is (Settings › Appearance › Bar
    // layout). "centre": centred over the dock or bottom bar (under a top bar with your apps); "corner": the corner
    // with the flame (bottom left over a bottom bar, top left under a top bar, beside a left or right bar).
    function panelPos(w, h, how) {
        const L = DeskState.leftInset, R = DeskState.rightInset, T = DeskState.topInset, B = DeskState.bottomInset;
        const e = DeskState.classicBar ? "bottom" : DeskState.barEdge, W = width, H = height;
        let x = how === "centre" ? L + (W - L - R - w) / 2 : (e === "right" ? W - R - w - 10 : L + 10);
        let y;
        if (e === "bottom") y = H - B - h - 10;
        else if (e === "top") y = how === "centre" && B > 0 ? H - B - h - 10 : T + 10;
        else y = how === "centre" ? T + (H - T - B - h) / 2 : T + 10;
        x = Math.max(L + 4, Math.min(W - R - w - 4, x));
        y = Math.max(T + 4, Math.min(H - B - h - 4, y));
        return Qt.point(Math.round(x), Math.round(y));
    }
    // The power menu (the power button in Centred / Tiles / Start screen, Classic's arrow): Game Mode first.
    function powerMenu(item, above) {
        if (!popMenu || !item) return;
        const p = item.mapToItem(null, 0, above ? 0 : item.height);
        popMenu.popup(screen, p.x, above ? p.y - 4 : p.y + 4, { title: "Power", above: !!above }, [
            { label: "Game Mode", icon: "pad", run: () => root.power("game") },
            { label: "Lock", icon: "lock", run: () => root.power("lock") },
            { label: "Sleep", icon: "sleep", run: () => root.power("sleep") },
            { label: "Restart", icon: "restart", run: () => root.power("restart") },
            { label: "Shut down", icon: "power", run: () => root.power("shutdown") },
            { label: "Log out", icon: "logout", run: () => root.power("logout") }
        ]);
    }

    // Every app A–Z, yours and the system's together (All apps in the styles), and grouped by first letter.
    readonly property var allApps: apps
    function letterOf(a) { const c = (a.name || "?").charAt(0).toUpperCase(); return /[A-Z]/.test(c) ? c : "#"; }
    readonly property var letters: { const seen = []; for (const a of allApps) { const l = letterOf(a); if (seen.indexOf(l) < 0) seen.push(l); } return seen; }
    // Pinned (Centred's grid): the apps pinned to the dock or bar first, then your other apps (Hearth's own first).
    readonly property var pinnedFirst: {
        const out = [], seen = {};
        for (const id of DeskState.pins) { const e = apps.find(a => a.id === id); if (e && !seen[e.id]) { seen[e.id] = true; out.push(e); } }
        for (const e of mainApps.filter(a => ours(a)).concat(mainApps.filter(a => !ours(a)))) if (!seen[e.id]) { seen[e.id] = true; out.push(e); }
        return out;
    }
    // Frequent (Classic, Tiles' Most used): what you opened from the menu lately, then the pinned apps.
    readonly property var frequent: {
        const out = [], seen = {};
        for (const id of DeskState.recent) { if (id.indexOf(":") >= 0) continue; const e = apps.find(a => a.id === id); if (e && !seen[e.id]) { seen[e.id] = true; out.push(e); } }
        for (const e of pinnedFirst) { if (out.length >= 10) break; if (!seen[e.id]) { seen[e.id] = true; out.push(e); } }
        return out.slice(0, 10);
    }
    // Recommended (Centred): recently opened apps, games and Settings pages, and recent files, newest first.
    readonly property var recommended: {
        const r = DeskState.recent.map(id => recentEntry(id)).filter(Boolean).slice(0, 4).map(e => ({ kind: e.page !== undefined ? "page" : e.title !== undefined ? "game" : "app", item: e }));
        const f = recentFiles.slice(0, 6 - Math.min(r.length, 3)).map(x => ({ kind: "file", item: x }));
        return r.slice(0, 6 - f.length).concat(f).slice(0, 6);
    }

    // ---- recent files (hearth-appmenu recent: the freedesktop recently-used list), for Recommended and jump lists
    property var recentFiles: []
    function readRecentFiles() { if (!recentProc.running) recentProc.running = true; }
    Process {
        id: recentProc
        command: ["python3", root.menuHelper, "recent", "60"]
        stdout: StdioCollector { onStreamFinished: { try { const r = JSON.parse(text.trim().split("\n").pop() || "[]"); if (Array.isArray(r)) root.recentFiles = r; } catch (e) {} } }
    }
    // An app's recent files (its own name, desktop id or program in the list), newest first.
    function recentFor(entry) {
        if (!entry) return [];
        const id = String(entry.id || "").toLowerCase().replace(/\.desktop$/, ""), nm = String(entry.name || "").toLowerCase();
        const prog = String((entry.execString || "").trim().split(/\s+/).filter(w => !/^(env|flatpak|run|--?\S+|\S+=\S*)$/.test(w))[0] || "").split("/").pop().toLowerCase();
        return recentFiles.filter(f => (f.apps || []).some(a => { const x = a.toLowerCase(); return x === nm || x === id || id.endsWith("." + x) || x.endsWith("." + id); })
            || (prog !== "" && (f.execs || []).some(x => x.toLowerCase() === prog))).slice(0, 8);
    }
    function hasJumpList(entry) { return !!entry && (recentFor(entry).length > 0 || (entry.actions && entry.actions.length > 0)); }
    // A jump list: recent files where the app has them, then its own actions (.desktop file).
    function jumpList(entry) {
        const out = [];
        for (const f of recentFor(entry)) out.push({ kind: "file", label: f.name, sub: f.folder, icon: f.dir ? "folder" : "file", run: () => root.openFile(f.path) });
        const acts = entry && entry.actions ? Array.from(entry.actions).slice(0, 6) : [];
        for (const a of acts) out.push({ kind: "action", label: a.name, sub: "", icon: "open", run: () => { a.execute(); root.shown = false; } });
        return out;
    }
    function openFile(path) { Quickshell.execDetached(["python3", DeskState.filesHelper, "open", path]); shown = false; }
    function openFolder(path) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "files", "open", path]); shown = false; }

    // ---- search (every style): apps, games, Settings pages and files in your home folder
    property var fileHits: []
    property string fileQuery: ""
    function findFiles(q) {
        fileQuery = q.trim();
        if (fileQuery.length < 2) { fileHits = []; return; }
        fileTimer.restart();
    }
    Timer {
        id: fileTimer
        interval: 280
        onTriggered: { filesProc.running = false; filesProc.command = ["python3", root.menuHelper, "files", root.fileQuery]; filesProc.running = true; }
    }
    Process {
        id: filesProc
        stdout: StdioCollector { onStreamFinished: { try { const r = JSON.parse(text.trim().split("\n").pop() || "[]"); if (Array.isArray(r)) root.fileHits = r; } catch (e) {} } }
    }
    // {best, apps, games, pages, files} for a query; filter: all | apps | games | pages | files.
    function searchAll(q, filter) {
        q = (q || "").trim().toLowerCase();
        const out = { best: null, apps: [], games: [], pages: [], files: [] };
        if (!q) return out;
        const hit = a => [a.name, a.genericName, a.comment, Array.from(a.keywords || []).join(" ")].join(" ").toLowerCase().includes(q);
        const starts = s => (s || "").toLowerCase().startsWith(q) ? 1 : 0;
        const word = s => (s || "").toLowerCase().split(/[\s()._-]+/).some(w => w.startsWith(q));
        const f = filter || "all";
        if (f === "all" || f === "apps") out.apps = apps.filter(hit).sort((a, b) => starts(b.name) - starts(a.name)).slice(0, f === "all" ? 6 : 40);
        if (f === "all" || f === "games") out.games = games.filter(g => (g.title || "").toLowerCase().includes(q)).sort((a, b) => starts(b.title) - starts(a.title)).slice(0, f === "all" ? 4 : 40);
        if (f === "all" || f === "pages") out.pages = settingsPages.filter(p => (p.name + " " + p.words).toLowerCase().includes(q)).slice(0, f === "all" ? 4 : 40);
        if (f === "all" || f === "files") out.files = fileHits.slice(0, f === "all" ? 5 : 24);
        // Best match: a name that starts with what you typed (an app, then a game, then a page), else the first app.
        const pick = out.apps.find(a => starts(a.name)) ? { kind: "app", item: out.apps.find(a => starts(a.name)) }
            : out.games.find(g => starts(g.title)) ? { kind: "game", item: out.games.find(g => starts(g.title)) }
            : out.pages.find(p => starts(p.name)) ? { kind: "page", item: out.pages.find(p => starts(p.name)) }
            : out.apps.find(a => word(a.name)) ? { kind: "app", item: out.apps.find(a => word(a.name)) }
            : out.apps.length ? { kind: "app", item: out.apps[0] } : out.games.length ? { kind: "game", item: out.games[0] }
            : out.pages.length ? { kind: "page", item: out.pages[0] } : out.files.length ? { kind: "file", item: out.files[0] } : null;
        out.best = pick;
        return out;
    }
    // A result as one flat row list (for the keyboard / controller): [{kind, item, group}], the best match first.
    function flatResults(r) {
        const rows = [];
        if (r.best) rows.push({ kind: r.best.kind, item: r.best.item, group: "Best match" });
        const skip = r.best ? r.best.item : null;
        for (const a of r.apps) if (a !== skip) rows.push({ kind: "app", item: a, group: "Apps" });
        for (const g of r.games) if (g !== skip) rows.push({ kind: "game", item: g, group: "Games" });
        for (const p of r.pages) if (p !== skip) rows.push({ kind: "page", item: p, group: "Settings" });
        for (const f of r.files) if (f !== skip) rows.push({ kind: "file", item: f, group: "Files" });
        return rows;
    }
    function openRow(row) {
        if (!row) return;
        if (row.kind === "file") { if (row.item.dir) openFolder(row.item.path); else openFile(row.item.path); }
        else launch(row.item);
    }
    function rowTitle(row) { return !row ? "" : row.kind === "game" ? row.item.title : row.kind === "file" ? row.item.name : row.item.name || ""; }
    function rowSub(row) {
        if (!row) return "";
        if (row.kind === "app") return "App" + (row.item.genericName ? " · " + row.item.genericName : row.item.comment ? " · " + row.item.comment : "");
        if (row.kind === "game") return "Game" + (row.item.source ? " · " + row.item.source : "");
        if (row.kind === "page") return "Settings page";
        return (row.item.dir ? "Folder" : "File") + " · " + (row.item.folder || "");
    }
    // The menu for a row or tile (X on the controller, right-click): an app's (shell.qml), others just Open.
    function rowMenu(row, item, extra) {
        if (!row || !item) return;
        const p = item.mapToItem(null, item.width / 2, item.height / 2);
        if (row.kind === "app" && !extra) { menuRequested(row.item, p.x, p.y); return; }
        if (!popMenu) return;
        const list = [{ label: "Open", icon: "open", run: () => openRow(row) }];
        if (row.kind === "app") for (const j of jumpList(row.item)) list.push({ label: j.label, icon: j.icon, run: j.run });
        if (row.kind === "file") list.push({ label: "Open the folder it's in", icon: "folder", run: () => openFolder(row.item.path.replace(/\/[^/]*$/, "") || "/") });
        if (extra && extra.length) { list.push({ separator: true }); for (const e of extra) list.push(e); }
        popMenu.popup(screen, p.x, p.y, { title: rowTitle(row) }, list);
    }
    function power(kind) {
        shown = false;
        const cmds = { game: ["nimbus-mode", "game"], sleep: ["python3", Buttons.helper, "sleep", "Desktop Mode: menu"], restart: ["systemctl", "reboot"],
            shutdown: ["systemctl", "poweroff"], logout: ["nimbus-mode", "logout"], lock: ["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "lock", "lock"] };
        if (kind === "settings") settingsRequested();
        else if (cmds[kind]) Quickshell.execDetached(cmds[kind]);
    }

    // ---- live tiles (Tiles and Start screen): data Hearth already has (hearth-appmenu tiles), read while open
    property var live: ({})
    function readLive() { if (!liveProc.running) liveProc.running = true; }
    Process {
        id: liveProc
        command: ["python3", root.menuHelper, "tiles"]
        stdout: StdioCollector { onStreamFinished: { try { const r = JSON.parse(text.trim().split("\n").pop() || "{}"); if (r && typeof r === "object") root.live = r; } catch (e) {} } }
    }
    Timer { interval: 60000; repeat: true; running: root.shown && root.styled && !!styleLoader.item && styleLoader.item.wantsLive; onTriggered: root.readLive() }
    // Your tile sizes, the tiles you took off and the order you moved them into (Settings.menuTiles, saved per user,
    // only when you change one: {sizes, hidden, added, order: [tile id, ...]}).
    readonly property var tileSettings: (Settings.menuTiles && typeof Settings.menuTiles === "object" && !Array.isArray(Settings.menuTiles)) ? Settings.menuTiles : ({})
    function tileSize(id, dflt) { const s = (tileSettings.sizes || {})[id]; return ["small", "medium", "wide", "large"].indexOf(s) >= 0 ? s : dflt; }
    function tileHidden(id) { return (tileSettings.hidden || []).indexOf(id) >= 0; }
    function setTileSize(id, size) {
        const t = JSON.parse(JSON.stringify(tileSettings));
        t.sizes = Object.assign({}, t.sizes || {});
        t.sizes[id] = size;
        Settings.menuTiles = t;
    }
    function setTileHidden(id, hide) {
        const t = JSON.parse(JSON.stringify(tileSettings));
        const h = (t.hidden || []).filter(x => x !== id);
        if (hide) h.push(id);
        t.hidden = h;
        if (!hide) t.added = (t.added || []).filter(x => x !== id).concat([id]);
        Settings.menuTiles = t;
    }
    // Where you moved the tiles to: tiles in `order` come first in their group, in that order; the rest after them
    // in Hearth's own order (so a new tile shows up at the end of its group).
    function tileRank(id, dflt) { const i = (tileSettings.order || []).indexOf(id); return i >= 0 ? i : 100000 + dflt; }
    // A group's tiles, in a move's preview order while you move one ({group, ids}), else as saved.
    function groupTiles(group, preview) {
        const list = tiles.filter(t => t.group === group);
        if (!preview || preview.group !== group) return list;
        const at = id => { const i = preview.ids.indexOf(id); return i >= 0 ? i : 100000; };
        return list.map((t, i) => ({ t: t, k: at(t.id) * 1000 + i })).sort((a, b) => a.k - b.k).map(x => x.t);
    }
    // Your move, dropped: the group's new order is saved (only if it changed). The order of other groups stays.
    function saveTileOrder(group, ids) {
        const now = tiles.filter(t => t.group === group).map(t => t.id);
        if (!Array.isArray(ids) || ids.length !== now.length || ids.every((id, i) => id === now[i])) return false;
        if (ids.some(id => now.indexOf(id) < 0)) return false;
        const t = JSON.parse(JSON.stringify(tileSettings));
        t.order = (t.order || []).filter(id => ids.indexOf(id) < 0).concat(ids);
        Settings.menuTiles = t;
        return true;
    }
    // The Resize / Move / Take off menu for a tile.
    function tileMenu(tile, item) {
        if (!popMenu || !tile) return;
        const p = item.mapToItem(null, item.width / 2, item.height / 2);
        const cur = tileSize(tile.id, tile.size);
        const list = [];
        if (tile.open) list.push({ label: "Open", icon: "open", run: () => tile.open() });
        if (tile.entry) for (const j of jumpList(tile.entry)) list.push({ label: j.label, icon: j.icon, run: j.run });
        if (list.length) list.push({ separator: true });
        for (const sz of [["small", "Small"], ["medium", "Medium"], ["wide", "Wide"], ["large", "Large"]])
            list.push({ label: sz[1], icon: "grid", checked: cur === sz[0] ? true : undefined, run: () => root.setTileSize(tile.id, sz[0]) });
        list.push({ separator: true });
        list.push({ label: "Take off the menu", icon: "unpin", run: () => root.setTileHidden(tile.id, true) });
        if (styled && typeof styleLoader.item.startMove === "function")
            list.push({ label: "Move", icon: "edit", run: () => styleLoader.item.startMove(tile.id) });
        popMenu.popup(screen, p.x, p.y, { title: tile.title || "Tile" }, list);
    }
    // The tiles: {id, kind, title, icon, entry, game, size (default), group, open()}. A tile with no data is a plain
    // icon tile. Groups: Play (games, Game Mode, updates), Today (now playing, weather, calendar), Home server, Apps.
    readonly property var tiles: {
        const out = [];
        const add = (t) => { if (!tileHidden(t.id)) out.push(t); };
        const gs = games.slice(0, 5);
        gs.forEach((g, i) => add({ id: "game:" + g.id, kind: "game", group: "Play", title: g.title, game: g, size: i === 0 ? "large" : "medium", open: () => root.launch(g) }));
        add({ id: "gamemode", kind: "gamemode", group: "Play", title: "Game Mode", size: "wide", open: () => root.power("game") });
        if (live.updates) add({ id: "updates", kind: "updates", group: "Play", title: "Updates", size: "wide", open: () => root.power("game") });
        add({ id: "nowplaying", kind: "nowplaying", group: "Today", title: "Now playing", size: "wide", open: () => {} });
        add({ id: "weather", kind: "weather", group: "Today", title: "Weather", size: "medium", open: () => { root.shown = false; Quickshell.execDetached(["hearth-weather"]); } });
        add({ id: "calendar", kind: "calendar", group: "Today", title: "Calendar", size: "medium", open: () => { root.shown = false; Quickshell.execDetached(["hearth-calendar"]); } });
        if (live.home) add({ id: "home", kind: "home", group: "Home server", title: "Home server", size: "wide", open: () => { root.shown = false; root.pageRequested("hs-overview"); } });
        const extra = (tileSettings.added || []).map(id => apps.find(a => a.id === id)).filter(Boolean);
        const seen = {};
        pinnedFirst.slice(0, 10).concat(extra).forEach((e, i) => {
            if (seen[e.id]) return;
            seen[e.id] = true;
            add({ id: "app:" + e.id, kind: "app", group: "Apps", title: e.name, entry: e, size: i < 2 ? "medium" : "small", open: () => root.launch(e) });
        });
        return out.map((t, i) => ({ t: t, k: tileRank(t.id, i) })).sort((a, b) => a.k - b.k).map(x => x.t);
    }
    readonly property var tileGroups: ["Play", "Today", "Home server", "Apps"].filter(g => tiles.some(t => t.group === g))

    // ---- the Games section: what Hearth said last at once, then Hearth's answer now (at most once a minute)
    function loadGames() {
        if (games.length === 0 && !gamesCached.running) gamesCached.running = true;
        if (gamesList.running || Date.now() - gamesAt < 60000) return;
        gamesAt = Date.now();
        gamesLoading = true;
        gamesList.running = true;
    }
    function takeGames(text, fresh) {
        const lines = text.trim().split("\n");
        try {
            const g = JSON.parse(lines[lines.length - 1] || "[]");
            if (Array.isArray(g) && (fresh || games.length === 0) && JSON.stringify(g) !== JSON.stringify(games)) games = g;
        } catch (e) {}
    }
    // The apps that came with the system: the image's own .desktop files. Apps added with apt on the Debian editions
    // (listed by Hearth's package layer in /var/lib/hearth/layer/added-apps) count as yours, not the system's.
    function takeSystem(text) {
        const parts = text.split("\n---\n");
        const added = {};
        for (const l of (parts[1] || "").split("\n")) { const id = l.trim().replace(/\.desktop$/, ""); if (id) added[id] = true; }
        const ids = {};
        for (const l of (parts[0] || "").split("\n")) {
            const id = l.trim().replace(/\.desktop$/, "");
            if (id && l.trim().endsWith(".desktop") && !added[id]) ids[id] = true;
        }
        systemIds = ids;
        // One app of each kind per desktop: what hearth-app-kinds says this desktop doesn't show.
        const hid = {};
        for (const l of (parts[2] || "").split("\n")) { const id = l.trim(); if (id) hid[id] = true; }
        hiddenIds = hid;
    }
    Process {
        id: systemList
        running: true
        command: ["sh", "-c", "ls -1 /usr/share/applications 2>/dev/null; printf '\\n---\\n'; cat /var/lib/hearth/layer/added-apps 2>/dev/null; printf '\\n---\\n'; python3 /usr/libexec/hearth-app-kinds hidden 2>/dev/null; true"]
        stdout: StdioCollector { onStreamFinished: root.takeSystem(text) }
    }
    Process {
        id: gamesCached
        command: ["python3", root.gamesHelper, "cached"]
        stdout: StdioCollector { onStreamFinished: root.takeGames(text, false) }
    }
    Process {
        id: gamesList
        command: ["python3", root.gamesHelper, "list"]
        stdout: StdioCollector { onStreamFinished: { root.takeGames(text, true); root.gamesLoading = false; } }
    }

    MouseArea { anchors.fill: parent; onClicked: root.shown = false } // click outside closes

    Image {   // a warm glow behind the panel
        visible: !root.styled
        anchors.centerIn: parent
        width: parent.width * 1.1; height: parent.height
        source: Qt.resolvedUrl("images/ember-glow.png")
        opacity: Theme.dark ? 0.18 : 0.1
        smooth: true
    }
    // The app menu you picked (not Hearth): drawn by its own file over this window. If it can't load, the Hearth
    // menu below shows instead (and says so in the log).
    Loader {
        id: styleLoader
        anchors.fill: parent
        z: 2
        active: root.style !== "hearth" && root.styleFiles[root.style] !== undefined
        source: active ? root.styleFiles[root.style] : ""
        onLoaded: { item.menu = root; if (root.shown) { item.opened(); root.readRecentFiles(); if (item.wantsLive) root.readLive(); } }
        onStatusChanged: if (status === Loader.Error) console.warn("Mantel: the " + root.style + " app menu couldn't load; showing the Hearth menu")
    }
    Item {
        id: box
        visible: !root.styled
        readonly property int topRoom: Math.max(DeskState.topInset, 20)
        readonly property int bottomRoom: Math.max(DeskState.bottomInset, 20)
        readonly property int leftRoom: DeskState.leftInset
        readonly property int rightRoom: DeskState.rightInset
        width: Math.min(parent.width - 60 - leftRoom - rightRoom, 1240)
        height: Math.min(parent.height - topRoom - bottomRoom - 30, 860)
        x: leftRoom + (parent.width - leftRoom - rightRoom - width) / 2
        y: topRoom + Math.max(10, (parent.height - topRoom - bottomRoom - height) / 2)
        MouseArea { anchors.fill: parent } // clicks inside don't close
        Rectangle { anchors.fill: parent; radius: Theme.r(26); color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.96); border.color: Theme.line; visible: frame.status !== Loader.Ready }
        Loader { id: frame; anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.r(26)); } }

        ColumnLayout {
            anchors { fill: parent; margins: 30 }
            spacing: 16

            // Sections
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text { text: root.tab === "games" ? "Games" : "Apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold; Layout.rightMargin: 10 }
                TabButton { label: "Apps"; key: "apps" }
                TabButton { label: "Games"; key: "games" }
                // In the System apps folder: where you are, and the way back.
                Rectangle {
                    visible: root.folderOpen && root.tab === "apps" && !search.text
                    implicitWidth: backText.implicitWidth + 28
                    implicitHeight: 38
                    radius: Theme.r(10)
                    color: backArea.containsMouse ? Theme.panel3 : Theme.panel2
                    Text { id: backText; anchors.centerIn: parent; text: "‹ System apps"; color: Theme.text; font.pixelSize: 14; font.family: Theme.font }
                    MouseArea { id: backArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { root.closeFolder(); search.forceActiveFocus(); } }
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: root.tabFocus ? "◀ ▶ switch · ▼ back to the list" : (root.folderOpen && !search.text ? "Esc goes back · Tab switches" : "Tab switches")
                    color: Theme.muted
                    font.pixelSize: 12
                    font.family: Theme.font
                }
            }

            // Search
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 60
                radius: Theme.r(30)
                color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.8)
                border.width: 2
                border.color: search.activeFocus && !root.tabFocus ? Theme.accent : Theme.line
                Icon { x: 20; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 22; color: Theme.accent }
                Text { anchors { right: parent.right; rightMargin: 24; verticalCenter: parent.verticalCenter } visible: !search.text; text: "Just start typing"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                TextInput {
                    id: search
                    anchors { fill: parent; leftMargin: 56; rightMargin: 160 }
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.text
                    selectionColor: Theme.accent
                    font.pixelSize: 19
                    font.family: Theme.font
                    onTextChanged: grid.currentIndex = 0
                    Keys.onEscapePressed: { if (root.tabFocus) root.tabFocus = false; else if (root.folderOpen && !search.text) root.closeFolder(); else root.shown = false; }
                    Keys.onMenuPressed: root.menuFor(grid.currentIndex)
                    Keys.onReturnPressed: { if (root.tabFocus) root.tabFocus = false; else root.launch(root.matches[grid.currentIndex]); }
                    Keys.onEnterPressed: { if (root.tabFocus) root.tabFocus = false; else root.launch(root.matches[grid.currentIndex]); }
                    Keys.onTabPressed: root.nextTab()
                    Keys.onBacktabPressed: root.nextTab()
                    Keys.onDownPressed: { if (root.tabFocus) root.tabFocus = false; else grid.moveCurrentIndexDown(); }
                    Keys.onUpPressed: {
                        // From the top row up to the section buttons.
                        const cols = Math.max(1, Math.floor(grid.width / grid.cellWidth));
                        if (grid.currentIndex < cols) root.tabFocus = true;
                        else grid.moveCurrentIndexUp();
                    }
                    Keys.onRightPressed: event => {
                        if (root.tabFocus) root.setTab("games");
                        else if (cursorPosition === text.length) grid.moveCurrentIndexRight();
                        else event.accepted = false;
                    }
                    Keys.onLeftPressed: event => {
                        if (root.tabFocus) root.setTab("apps");
                        else if (cursorPosition === text.length && text === "") grid.moveCurrentIndexLeft();
                        else event.accepted = false;
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: !search.text
                        text: root.tab === "games" ? "Search games" : "Search apps, games and Settings pages"
                        color: Theme.muted
                        font: search.font
                    }
                }
            }

            // Recently opened apps, games and Settings pages (the newest first), above the list when nothing is typed.
            Text {
                visible: recentRow.visible
                text: "RECENT"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 1.3
            }
            RowLayout {
                id: recentRow
                readonly property var entries: DeskState.recent.map(id => root.recentEntry(id)).filter(Boolean).slice(0, 3)
                Layout.fillWidth: true
                visible: root.tab === "apps" && !search.text && !root.folderOpen && entries.length > 0
                spacing: 12
                Repeater {
                    model: recentRow.visible ? recentRow.entries : []
                    delegate: Rectangle {
                        id: chip
                        required property var modelData
                        readonly property bool isPage: modelData.page !== undefined
                        readonly property bool isGame: modelData.title !== undefined
                        Layout.fillWidth: true
                        Layout.maximumWidth: 380
                        implicitHeight: 72
                        radius: Theme.r(16)
                        color: chipArea.containsMouse ? Theme.panel3 : Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.85)
                        border.color: Theme.line
                        RowLayout {
                            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                            spacing: 14
                            Rectangle {
                                implicitWidth: 44; implicitHeight: 44; radius: Theme.r(12)
                                color: chip.isPage || chip.isGame ? Theme.panel3 : "transparent"
                                AppIcon { anchors.centerIn: parent; visible: !chip.isPage && !chip.isGame; size: 40; icon: chip.modelData.icon || ""; appId: chip.modelData.id || "" }
                                Icon { anchors.centerIn: parent; visible: chip.isPage || chip.isGame; name: chip.isGame ? "pad" : (chip.modelData.icon || "gear"); size: 22 }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Text { Layout.fillWidth: true; text: chip.isGame ? chip.modelData.title : chip.modelData.name; color: Theme.text; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: Theme.font; elide: Text.ElideRight }
                                Text { Layout.fillWidth: true; text: chip.isGame ? "Game" : chip.isPage ? "Settings page" : "App"; color: Theme.muted; font.pixelSize: 12; font.family: Theme.font }
                            }
                        }
                        MouseArea {
                            id: chipArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: Qt.PointingHandCursor
                            onClicked: mouse => {
                                if (mouse.button === Qt.RightButton && !chip.isPage && !chip.isGame) { const p = chip.mapToItem(null, mouse.x, mouse.y); root.menuRequested(chip.modelData, p.x, p.y); }
                                else root.launch(chip.modelData);
                            }
                        }
                    }
                }
                Item { Layout.fillWidth: true }
            }
            Text {
                visible: root.tab === "apps" && !search.text
                text: root.folderOpen ? "SYSTEM APPS" : "ALL APPS"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 1.3
            }

            // Apps / games
            GridView {
                id: grid
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.matches
                cellWidth: root.tab === "games" ? Math.floor(width / Math.max(1, Math.floor(width / 150))) : Math.floor(width / Math.max(1, Math.floor(width / 140)))
                cellHeight: root.tab === "games" ? 236 : 128
                keyNavigationEnabled: true
                highlightFollowsCurrentItem: true
                highlight: Rectangle { radius: Theme.r(14); color: Theme.panel3; border.color: root.tabFocus ? Theme.line : Theme.accent }
                delegate: Item {
                    id: cell
                    required property var modelData
                    required property int index
                    // Games are plain objects with a title; Settings pages ({page, name, icon}) come from the search; apps are desktop entries.
                    readonly property bool isPage: cell.modelData.page !== undefined
                    readonly property bool isGame: cell.modelData.title !== undefined && root.tab === "games"
                    readonly property bool gameHit: cell.modelData.title !== undefined && root.tab !== "games"
                    readonly property string label: (cell.modelData.title !== undefined ? cell.modelData.title : cell.modelData.name) || ""
                    width: grid.cellWidth
                    height: grid.cellHeight
                    Rectangle {
                        anchors { fill: parent; margins: 4 }
                        radius: Theme.r(14)
                        color: hover.containsMouse && grid.currentIndex !== cell.index ? Theme.panel2 : "transparent"
                    }
                    Column {
                        anchors.centerIn: parent
                        spacing: 8
                        AppIcon {
                            visible: !cell.isGame && !cell.isPage && !cell.gameHit
                            anchors.horizontalCenter: parent.horizontalCenter
                            size: 64
                            icon: cell.isGame || cell.isPage || cell.gameHit ? "" : (cell.modelData.icon || "")
                            appId: cell.isGame || cell.isPage || cell.gameHit ? "" : (cell.modelData.id || "")
                        }
                        // A Settings page or a game found by the search: Hearth's own icon in a tile.
                        Rectangle {
                            visible: cell.isPage || cell.gameHit
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 64; height: 64; radius: Theme.r(18)
                            color: Theme.panel3
                            Icon { anchors.centerIn: parent; name: cell.gameHit ? "pad" : (cell.modelData.icon || "gear"); size: 28 }
                        }
                        // The game's cover (its first letter until the picture is in, or without one).
                        Rectangle {
                            visible: cell.isGame
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 120
                            height: 168
                            radius: Theme.r(10)
                            color: Theme.panel3
                            Text {
                                anchors.centerIn: parent
                                visible: cover.status !== Image.Ready
                                text: cell.label.charAt(0).toUpperCase()
                                color: Theme.muted
                                font.pixelSize: 40
                                font.family: Theme.font
                            }
                            Image {
                                id: cover
                                anchors.fill: parent
                                visible: status === Image.Ready
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                                cache: true
                                sourceSize.width: 240
                                source: cell.isGame ? (cell.modelData.cover || cell.modelData.wide || cell.modelData.icon || "") : ""
                            }
                        }
                        Text {
                            width: grid.cellWidth - 16
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                            maximumLineCount: 2
                            wrapMode: Text.Wrap
                            text: cell.label + (cell.isPage ? "\nSettings" : cell.gameHit ? "\nGame" : "")
                            color: Theme.text
                            font.pixelSize: 13
                            font.family: Theme.font
                        }
                    }
                    // Pinned to the taskbar
                    Icon {
                        visible: !cell.isGame && !cell.isPage && !cell.gameHit && !cell.modelData.folder && DeskState.isPinned(cell.modelData.id)
                        anchors { top: parent.top; right: parent.right; margins: 12 }
                        name: "pin"; size: 14; color: Theme.muted
                    }
                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: mouse => {
                            root.tabFocus = false;
                            if (mouse.button === Qt.RightButton) { grid.currentIndex = cell.index; root.menuFor(cell.index); }
                            else root.launch(cell.modelData);
                        }
                        onPressAndHold: { grid.currentIndex = cell.index; root.menuFor(cell.index); }
                    }
                }
                Text {
                    anchors.centerIn: parent
                    width: parent.width - 40
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    visible: grid.count === 0
                    text: search.text ? "No " + (root.tab === "games" ? "games" : "apps") + " match \"" + search.text + "\""
                        : root.tab === "games" ? (root.gamesLoading ? "Finding your games…" : "No games installed yet. Games you install in Hearth show up here.")
                        : "No apps"
                    color: Theme.muted
                    font.pixelSize: 14
                    font.family: Theme.font
                }
            }

            Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.line }
            // Power: Game Mode first and in the accent
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                ActionButton { label: "Game Mode"; icon: "pad"; primary: true; command: ["nimbus-mode", "game"] }
                ActionButton { label: "Settings"; icon: "gear"; command: [] }
                ActionButton { label: "Sleep"; icon: "sleep"; command: ["python3", Buttons.helper, "sleep", "Desktop Mode: menu"] }
                ActionButton { label: "Restart"; icon: "restart"; command: ["systemctl", "reboot"] }
                ActionButton { label: "Shut down"; icon: "power"; command: ["systemctl", "poweroff"] }
                ActionButton { label: "Log out"; icon: "logout"; command: ["nimbus-mode", "logout"] }
                Item { Layout.fillWidth: true }
                Repeater {
                    model: root.width > 1500 ? [{ b: "A", c: "#3fae5a", t: "Open" }, { b: "X", c: "#3b7de0", t: "Options" }, { b: "LB", c: "#6c7480", t: "Tab" }] : []
                    delegate: Row {
                        required property var modelData
                        spacing: 6
                        Rectangle { width: 24; height: 24; radius: 12; color: modelData.c; Text { anchors.centerIn: parent; text: modelData.b; color: "white"; font.pixelSize: modelData.b.length > 1 ? 9 : 11; font.bold: true } }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: modelData.t; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
            }
        }
    }

    // A section button (Apps, Games): lit when it's the one shown, ringed when the D-pad is on the buttons.
    component TabButton: Rectangle {
        id: tb
        property string label
        property string key
        readonly property bool current: root.tab === tb.key
        implicitWidth: tbText.implicitWidth + 44
        implicitHeight: 44
        radius: Theme.r(14)
        color: tb.current ? Theme.accent : (tbArea.containsMouse ? Theme.panel3 : Theme.panel2)
        border.width: root.tabFocus && tb.current ? 2 : 0
        border.color: Theme.text
        Text { id: tbText; anchors.centerIn: parent; text: tb.label; color: tb.current ? Theme.accentInk : Theme.text; font.pixelSize: 14; font.bold: tb.current; font.family: Theme.font }
        MouseArea {
            id: tbArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { root.setTab(tb.key); search.forceActiveFocus(); }
        }
    }

    component ActionButton: Rectangle {
        id: ab
        property string label
        property string icon: ""
        property var command: []
        property bool primary: false
        implicitWidth: abRow.implicitWidth + 36
        implicitHeight: 52
        radius: Theme.r(14)
        color: primary ? (abArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent) : (abArea.containsMouse ? Theme.panel3 : Theme.panel2)
        border.color: primary ? "transparent" : Theme.line
        Row {
            id: abRow
            anchors.centerIn: parent
            spacing: 10
            Icon { visible: ab.icon !== ""; anchors.verticalCenter: parent.verticalCenter; name: ab.icon; size: 20; color: ab.primary ? Theme.accentInk : Theme.text }
            Text { id: abText; anchors.verticalCenter: parent.verticalCenter; text: ab.label; color: ab.primary ? Theme.accentInk : Theme.text; font.pixelSize: 15; font.bold: ab.primary; font.family: Theme.font }
        }
        MouseArea {
            id: abArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { root.shown = false; if (ab.command.length) Quickshell.execDetached(ab.command); else root.settingsRequested(); }
        }
    }
}
