// Hearth Classic's pure parts (src/model.ts). CI: node --experimental-strip-types --test tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  DEFAULTS, REQUIRED, PAUSED_NOTE, normalize, checkSteam, decide, renderTheme, makePatchSet, appIdFor,
  homeFromHearth, homeFromSteam, pickHome, artCandidates, steamArtMethods, played, blipFor,
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
