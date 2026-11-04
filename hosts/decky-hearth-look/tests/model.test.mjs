// Hearth Classic's pure parts (src/model.ts). CI: node --experimental-strip-types --test tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  DEFAULTS, REQUIRED, PAUSED_NOTE, normalize, checkSteam, decide, renderTheme, makePatchSet, appIdFor,
  homeFromHearth, homeFromSteam, pickHome, artCandidates, steamArtMethods, played, blipFor,
  PLACES, runPlace, KNOWN_NAVIGATOR, KNOWN_WINDOW, KNOWN_ROUTES, menuPlan, menuIcon, MENU_PAUSED, judgeStructure, judgeBar, menuNote, classKeys, diagnosticsText, isWebRoute, judgeVisible, sendReasons,
} from "../src/model.ts";

const CSS = readFileSync(new URL("../defaults/hearth-classic.css", import.meta.url), "utf8");
const all = (m, k) => `${m}_${k}_x1 ${m}_extra`;

test("settings: off by default, only valid values kept", () => {
  assert.equal(DEFAULTS.look, "steam");
  assert.equal(DEFAULTS.sounds, false);
  assert.deepEqual(normalize(null), DEFAULTS);
  assert.deepEqual(normalize({ look: "classic", sounds: true }), { ...DEFAULTS, look: "classic", sounds: true });
  assert.deepEqual(normalize({ look: "other", sounds: "yes", home: 0 }), DEFAULTS);
});

test("Steam check: everything there means apply", () => {
  const c = checkSteam(all, true);
  assert.deepEqual(c.missing, []);
  assert.deepEqual(decide({ ...DEFAULTS, look: "classic" }, true, c), { state: "on", why: "" });
});

test("fallback: a missing or renamed Steam module pauses the look", () => {
  const renamed = (m, k) => (m === "gamepadDialogClasses" ? undefined : all(m, k));
  const c = checkSteam(renamed, true);
  assert.deepEqual(c.missing, ["gamepadDialogClasses.Field"]);
  const d = decide({ ...DEFAULTS, look: "classic" }, true, c);
  assert.equal(d.state, "paused");
  assert.match(d.why, /gamepadDialogClasses\.Field/);
  // A resolver that throws counts as missing, never as a crash.
  const boom = () => { throw new Error("webpack changed"); };
  assert.equal(checkSteam(boom, true).missing.length, REQUIRED.length);
  // No route patching: paused too (the home can't be put in cleanly).
  assert.deepEqual(checkSteam(all, false).missing, ["routerHook"]);
  // A failure while applying pauses as well.
  assert.equal(decide({ ...DEFAULTS, look: "classic" }, true, checkSteam(all, true), "boom").state, "paused");
  assert.ok(PAUSED_NOTE.includes("Steam changed its layout"));
});

test("off: the Steam look chosen, or not Hearth OS, applies nothing whatever Steam has", () => {
  assert.equal(decide(DEFAULTS, true, checkSteam(all, true)).state, "off");
  assert.equal(decide({ ...DEFAULTS, look: "classic" }, false, checkSteam(all, true)).state, "off");
  assert.equal(decide({ ...DEFAULTS, look: "classic" }, false, null, "boom").state, "off");
});

test("skin: Steam's names filled in, lines with missing names left out", () => {
  const full = renderTheme(CSS, { logo: "data:L", wallpaper: "data:W" }, all);
  assert.ok(!full.css.includes("{{"));
  assert.ok(full.css.includes(".gamepadTabbedPageClasses_TabHeaderRowWrapper_x1{"));
  assert.ok(!full.css.includes("_extra"));          // only the first of several names
  assert.ok(full.css.includes("url(data:W)"));
  assert.equal(full.dropped, 0);
  const none = renderTheme(CSS, { logo: "", wallpaper: "" }, () => undefined);
  assert.ok(!none.css.includes("{{") && !none.css.includes("%WALLPAPER%") && !none.css.includes("%LOGO%"));
  assert.ok(none.css.includes(".gpfocus{"));        // layers 1 and 2 stay
  assert.ok(none.css.includes("--gpColor-Blue:#3fd0b8"));
  assert.ok(none.dropped > 10);
  // Leaving lines out never leaves a comment open (which would swallow the rest of the skin).
  for (const r of [full, none]) assert.equal(r.css.split("/*").length, r.css.split("*/").length);
});

test("patches: all removed on unload, newest first, even if one undo throws", () => {
  const routes = new Map();
  const routerHook = { addPatch: (p, f) => { routes.set(p, f); return f; }, removePatch: (p, f) => { if (routes.get(p) === f) routes.delete(p); } };
  const logs = [];
  const ps = makePatchSet((m) => logs.push(m));
  const order = [];
  const h = routerHook.addPatch("/library/home", () => {});
  ps.add("home", () => { order.push("home"); routerHook.removePatch("/library/home", h); });
  ps.add("broken", () => { order.push("broken"); throw new Error("gone"); });
  ps.add("skin", () => order.push("skin"));
  assert.deepEqual(ps.names(), ["home", "broken", "skin"]);
  const done = ps.removeAll();
  assert.deepEqual(order, ["skin", "broken", "home"]);
  assert.deepEqual(done, ["skin", "home"]);
  assert.equal(routes.size, 0);
  assert.equal(logs.length, 1);
  assert.deepEqual(ps.names(), []);
  assert.deepEqual(ps.removeAll(), []);             // twice is fine
});

const HEARTH = {
  hero: { id: "epic:alan", title: "Alan Wake 2", store: "Epic", minutes: 300 },
  continue: [{ id: "epic:alan", title: "Alan Wake 2", store: "Epic" }, { id: "steam:620", title: "Portal 2", store: "Steam" }, { id: "gog:nope", title: "Not synced" }],
  favorites: [{ id: "steam:1245620", title: "ELDEN RING", store: "Steam", cover: "https://x/c.jpg" }],
  rows: [
    { id: "steam", title: "Steam", items: [{ id: "steam:620", title: "Portal 2" }, { id: "steam:1245620", title: "ELDEN RING" }] },
    { id: "epic", title: "Epic", items: [{ id: "epic:alan", title: "Alan Wake 2" }] },
    { id: "gog", title: "GOG", items: [{ id: "gog:nope", title: "Not synced" }] },
    { id: "manual", title: "Added", items: [{ id: "manual:x", title: "My game" }] },
  ],
  added: [], count: 5,
};
const SHORTCUTS = { "epic:alan": 3000000001, "manual:x": "3000000002", "bad:x": -1 };

test("home from Hearth's layout: only games Steam can open, Hearth Sync's names for the rows", () => {
  assert.equal(appIdFor("steam:620", {}), 620);
  assert.equal(appIdFor("epic:alan", SHORTCUTS), 3000000001);
  assert.equal(appIdFor("manual:x", SHORTCUTS), 3000000002);
  assert.equal(appIdFor("bad:x", SHORTCUTS), null);
  assert.equal(appIdFor("gog:nope", SHORTCUTS), null);
  assert.equal(appIdFor("toString", SHORTCUTS), null);
  const v = homeFromHearth(HEARTH, SHORTCUTS);
  assert.equal(v.from, "hearth");
  assert.equal(v.hero.appid, 3000000001);
  assert.equal(v.hero.title, "Alan Wake 2");
  assert.deepEqual(v.shelves.map((s) => s.title), ["Continue playing", "Favorites", "Steam", "Epic", "Added by hand"]);
  assert.deepEqual(v.shelves[0].items.map((c) => c.appid), [3000000001, 620]);   // the unsynced GOG game is left out
  assert.equal(v.shelves[0].wide, true);
  assert.equal(v.shelves[1].items[0].cover, "https://x/c.jpg");
  assert.equal(homeFromHearth(null, null).shelves.length, 0);
  assert.equal(homeFromHearth({ rows: "x", continue: 5 }, {}).shelves.length, 0);
});

test("home from Steam: recent, Favorites, Steam's games and Hearth Sync's collections", () => {
  const apps = [
    { appid: 620, display_name: "Portal 2", app_type: 1, rt_last_time_played: 50 },
    { appid: 1245620, display_name: "ELDEN RING", app_type: 1, rt_last_time_played: 90 },
    { appid: 3000000001, display_name: "Alan Wake 2", app_type: 1073741824, rt_last_time_played: 70 },
    { appid: 3000000003, display_name: "A shortcut of mine", app_type: 1073741824 },
    { appid: 0, display_name: "broken" },
  ];
  const cols = [{ name: "Epic", appids: [3000000001] }, { name: "RPGs", appids: [1245620] }];
  const v = homeFromSteam(apps, cols, [620, 999]);
  assert.equal(v.from, "steam");
  assert.equal(v.hero.appid, 1245620);
  assert.deepEqual(v.shelves.map((s) => s.title), ["Continue playing", "Favorites", "Steam", "Epic"]);
  assert.deepEqual(v.shelves[0].items.map((c) => c.appid), [1245620, 3000000001, 620]);
  assert.equal(v.shelves[0].items[1].store, "Epic");
  assert.deepEqual(v.shelves[2].items.map((c) => c.title), ["ELDEN RING", "Portal 2"]);
  assert.equal(homeFromSteam([], [], []).shelves.length, 0);
});

test("pickHome: Hearth's layout first, then Steam's, else Steam's own Home", () => {
  const h = homeFromHearth(HEARTH, SHORTCUTS);
  const s = homeFromSteam([{ appid: 620, display_name: "Portal 2", app_type: 1 }], [], []);
  assert.equal(pickHome(h, s).from, "hearth");
  assert.equal(pickHome(homeFromHearth({}, {}), s).from, "steam");
  assert.equal(pickHome(null, homeFromSteam([], [], [])), null);
});

test("pictures and small helpers", () => {
  const c = { appid: 620, title: "Portal 2", store: "Steam", cover: "https://c", wide: "https://w" };
  assert.deepEqual(artCandidates(c, "portrait", ["/steam/p.jpg", "https://c"]), ["/steam/p.jpg", "https://c", "https://w"]);
  assert.deepEqual(artCandidates(null, "wide", []), []);
  assert.ok(steamArtMethods("portrait")[0].startsWith("GetCustom"));
  assert.equal(played(0), "");
  assert.equal(played(30), "30 min played");
  assert.equal(played(300), "5 h played");
  assert.deepEqual(blipFor("ok"), [880, 0.05, 0.035]);
});

test("settings: the menu bars are on once Hearth Classic is picked, and remembered off", () => {
  assert.equal(DEFAULTS.menu, true);
  assert.equal(DEFAULTS.menuTop, true);
  assert.equal(normalize({ menu: false }).menu, false);
  assert.equal(normalize({ menuTop: "no" }).menuTop, true);
});

test("places: every tile maps to a known navigator call or route, and ends on a page that exists", () => {
  assert.deepEqual(PLACES.map((p) => p.id), ["library", "store", "downloads", "media", "friends", "settings"]);
  for (const p of PLACES) {
    for (const st of p.steps) {
      if (st.kind === "navigator") assert.ok(KNOWN_NAVIGATOR.includes(st.call), `${p.id}: ${st.call}`);
      else if (st.kind === "window") assert.ok(KNOWN_WINDOW.includes(st.call), `${p.id}: ${st.call}`);
      else assert.ok(KNOWN_ROUTES.includes(st.path), `${p.id}: ${st.path}`);
    }
    const last = p.steps[p.steps.length - 1];
    assert.equal(last.kind, "route", `${p.id} must end on a route`);
    // None of the routes that showed empty pages on the device.
    for (const st of p.steps) if (st.kind === "route") assert.ok(!["/downloads", "/media", "/friends", "/store"].includes(st.path));
  }
});

test("places: Steam's navigator first, the next way when one is missing or throws, the Library at worst", () => {
  const calls = [];
  const rec = (n) => (...a) => calls.push([n, a]);
  const win = { Navigator: { Downloads: rec("Downloads"), Chat: rec("Chat"), Settings: rec("Settings"), SteamWeb: rec("SteamWeb"), Media: { Grid: rec("Media.Grid") } },
                NavigateToSteamWeb: rec("window.NavigateToSteamWeb") };
  const routes = [];
  const go = (id) => runPlace(PLACES.find((p) => p.id === id), win, (r) => routes.push(r));
  assert.equal(go("store"), "navigator:SteamWeb");
  assert.deepEqual(calls.at(-1), ["SteamWeb", ["https://store.steampowered.com/"]]);
  assert.equal(go("downloads"), "navigator:Downloads");
  assert.equal(go("media"), "navigator:Media.Grid");
  assert.equal(calls.at(-1)[1][0].state.filter.listSource.type, "recents");
  assert.equal(go("friends"), "navigator:Chat");
  assert.equal(go("settings"), "navigator:Settings");
  assert.equal(go("library"), "route:/library");
  // Missing: the next way. Throwing: the next way too.
  const win2 = { Navigator: { Settings: rec("Settings"), Downloads: () => { throw new Error("gone"); } }, NavigateToSteamWeb: rec("window.NavigateToSteamWeb") };
  assert.equal(runPlace(PLACES[2], win2, (r) => routes.push(r)), "navigator:Settings");
  assert.equal(runPlace(PLACES[1], win2, (r) => routes.push(r)), "window:NavigateToSteamWeb");
  assert.equal(runPlace(PLACES[1], { Navigator: {} }, (r) => routes.push(r)), "route:/steamweb");   // Steam's own Store route
  assert.equal(runPlace(PLACES[3], win2, (r) => routes.push(r)), "route:/library");
  // No window at all: still a page.
  for (const p of PLACES) assert.match(runPlace(p, null, () => {}), /^route:\/(library|settings|steamweb)$/);
});

test("menu bars: switches, the real menu's structure, and the bar only when it really sits as a bar", () => {
  const classic = { ...DEFAULTS, look: "classic" };
  assert.deepEqual(menuPlan(classic).sections, { menu: true, top: true });
  assert.deepEqual(menuPlan({ ...classic, menuTop: false }).sections, { menu: true, top: false });
  assert.deepEqual(menuPlan({ ...classic, menu: false }).sections, { menu: false, top: false });
  const L = ["Home", "Library", "Store", "Friends & Chat", "Media", "Downloads", "Settings", "Power"];
  assert.equal(judgeStructure(true, L).ok, true);
  assert.match(judgeStructure(false, L).why, /no \.Panel/);
  assert.match(judgeStructure(true, ["Home", "Library"]).why, /only 2/);
  assert.match(judgeStructure(true, ["", "", "", "Home"]).why, /no label/);
  const win = { w: 1280, h: 800 };
  const row = (top) => L.map((_, i) => ({ left: 250 + i * 90, top, width: 80, height: 68 }));
  assert.deepEqual(judgeBar(win, { left: 230, right: 1050, top: 690, bottom: 778 }, row(700), true, false), { ok: true, why: "" });
  assert.equal(judgeBar(win, { left: 230, right: 1050, top: 22, bottom: 110 }, row(32), true, true).ok, true);
  // The cases a device could show: a narrow menu window, the bar not at the bottom, entries in a column, covered.
  assert.match(judgeBar({ w: 320, h: 800 }, { left: 0, right: 320, top: 0, bottom: 800 }, row(700), true, false).why, /320 px wide/);
  assert.match(judgeBar(win, { left: 230, right: 1050, top: 100, bottom: 190 }, row(110), true, false).why, /bottom/);
  assert.match(judgeBar(win, { left: 230, right: 1050, top: 690, bottom: 778 }, L.map((_, i) => ({ left: 250, top: 100 + i * 50, width: 80, height: 40 })), true, false).why, /one row/);
  assert.match(judgeBar(win, { left: 230, right: 1050, top: 690, bottom: 778 }, row(700), false, false).why, /covers/);
  assert.match(judgeBar(win, { left: 230, right: 1050, top: 690, bottom: 778 }, [{ left: 0, top: 0, width: 0, height: 0 }], true, false).why, /size/);
  // The panel's note.
  assert.equal(menuNote({ layout: "bar", why: "", found: true }, classic), "");
  assert.equal(menuNote({ layout: "pending", why: "x", found: true }, classic), "");
  assert.ok(menuNote({ layout: "vertical", why: "the bar didn't fit", found: true }, classic).startsWith(MENU_PAUSED));
  assert.ok(menuNote({ layout: "none", why: "only 2", found: true }, classic).includes("Steam's own menu"));
  assert.equal(menuNote({ layout: "none", why: "x", found: true }, { ...classic, menu: false }), "");
  // The skin leaves a section out whole when it's off, and puts it in when it's on.
  const on = renderTheme(CSS, { logo: "L", wallpaper: "W" }, all, { menu: true, top: true }).css;
  const off = renderTheme(CSS, { logo: "L", wallpaper: "W" }, all, { menu: false, top: false }).css;
  assert.ok(on.includes("html.hearth-bar #MainNavMenuContainer > .Panel > * > .Panel.Focusable{") && on.includes("html.hearth-bar.hearth-ingame"));
  const rules = off.replace(/\/\*[\s\S]*?\*\//g, "");
  assert.ok(!rules.includes("MainNavMenuContainer") && !rules.includes("hearth-ingame") && !rules.includes("data-hearth-icon"));
  // Every bar rule is scoped to html.hearth-bar / html.hearth-menu (set only after the real menu is found).
  for (const line of on.split("\n")) if (line.includes("MainNavMenuContainer") && !line.startsWith("/*") && !line.startsWith("   ")) assert.match(line, /^html\.hearth-(bar|menu|ghost)/, line);
  for (const r of [on, off]) assert.equal(r.split("/*").length, r.split("*/").length);
});

test("diagnostics: class names found and missing, the menu, the windows", () => {
  const k = classKeys(CSS, (m, key) => (m === "gamepadDialogClasses" ? undefined : all(m, key)));
  assert.ok(k.resolved.includes("gamepadTabbedPageClasses.TabHeaderRowWrapper"));
  assert.ok(k.missing.includes("gamepadDialogClasses.Field"));
  assert.equal(new Set([...k.resolved, ...k.missing]).size, k.resolved.length + k.missing.length);
  const t = diagnosticsText({ state: "on", why: "", settings: { ...DEFAULTS, look: "classic" }, keys: k,
    menu: { windows: [], found: true, entries: 8, labels: ["Home", "Library"], layout: "vertical", why: "the bar didn't fit", game: false,
            method: "mirror", before: "241×800 (main window 1280×800)", after: "241×800", resizeWith: "SteamClient.Window" },
    windows: ["SP", "MainMenu_uid2"], game: null });
  for (const w of ["Hearth Classic: on", "MainMenu_uid2", "missing: ", "Menu: found", "8 entries", "Menu bar: vertical", "method: Hearth's bar drawn", "before 241×800", "SteamClient.Window", "can't tell"]) assert.ok(t.includes(w), w);
});

test("menu icons from Steam's labels", () => {
  assert.equal(menuIcon(" Library "), "lib");
  assert.equal(menuIcon("Store"), "shop");
  assert.equal(menuIcon("Friends & Chat"), "friends");
  assert.equal(menuIcon("Exit game"), "close");
  assert.equal(menuIcon("Resume"), "pad");
  assert.equal(menuIcon("Bibliothek"), "");
  const css = CSS;
  for (const ic of ["home", "lib", "shop", "media", "dl", "friends", "gear", "power", "search", "pad", "close", "shot"]) assert.ok(css.includes(`[data-hearth-icon="${ic}"]`), ic);
});

test("web pages and the self-check after resizing", () => {
  assert.ok(isWebRoute("/steamweb") && isWebRoute("/steamweb/app/620") && isWebRoute("/externalweb"));
  assert.ok(!isWebRoute("/library/home") && !isWebRoute("/settings") && !isWebRoute("") && !isWebRoute("/steamwebfoo"));
  const e = (o) => ({ label: "Home", inside: true, shown: true, hit: true, cover: "", ...o });
  assert.deepEqual(judgeVisible([e(), e({ label: "Library" })]), { ok: true, why: "" });
  assert.match(judgeVisible([]).why, /no entries/);
  assert.match(judgeVisible([e(), e({ label: "Store", inside: false })]).why, /outside the window \(Store\)/);
  assert.match(judgeVisible([e({ shown: false })]).why, /hidden/);
  assert.match(judgeVisible([e({ hit: false, cover: ".GameBackground" })]).why, /covered by \.GameBackground/);
});

test("auto-send reasons", () => {
  assert.equal(DEFAULTS.autoSend, false);
  const k = (d, first = false) => sendReasons({ found: true, layout: "bar", method: "resized", why: "", selfCheck: "passed", ...d }, first).map(([key]) => key.split(":")[0]);
  assert.deepEqual(k({}), []);
  assert.deepEqual(k({}, true), ["first-open"]);
  assert.deepEqual(k({ method: "mirror", why: "Steam's menu window couldn't be resized (241×760 → 241×760)" }), ["mirror", "resize-refused"]);
  assert.deepEqual(k({ layout: "vertical", method: "vertical", why: "the top dock's self-check failed", selfCheck: "failed: 8 entries covered" }), ["fallback", "selfcheck"]);
  assert.deepEqual(k({ layout: "none", method: "none", why: "only 2 menu entries found" }), ["paused"]);
  assert.deepEqual(k({ found: false, layout: "none", method: "none", why: "not found" }, true), []);
});
