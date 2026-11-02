// Unit tests of the takeover's pure parts (src/model.ts), run in CI with Node's own test runner and type stripping:
//   node --experimental-strip-types --test archive/hosts/decky-hearth/tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import * as M from "../src/model.ts";

test("settings: defaults, odd values ignored", () => {
  assert.deepEqual(M.normalize(null), M.DEFAULTS);
  const s = M.normalize({ takeover: false, tkHome: "yes", theme: false, evil: true });
  assert.equal(s.takeover, false);
  assert.equal(s.tkHome, true, "a non-boolean keeps the default");
  assert.equal(s.theme, false);
  assert.equal("evil" in s, false);
  assert.equal(M.DEFAULTS.takeover, true, "the takeover is on by default");
  assert.equal(M.DEFAULTS.tkSounds, false, "Hearth's sounds don't double Steam's unless asked");
});

test("parts: master switch, own switch, the guard", () => {
  const s = { ...M.DEFAULTS };
  assert.equal(M.partOn(s, "tkHome"), true);
  assert.equal(M.partOn({ ...s, takeover: false }, "tkHome"), false, "the master switch turns every part off");
  assert.equal(M.partOn({ ...s, tkHome: false }, "tkHome"), false);
  assert.equal(M.partOn(s, "tkHome", new Set(["tkHome"])), false, "a part the guard turned off stays off");
  assert.equal(M.oldRowOn(s), false, "the older row isn't drawn under Hearth's home");
  assert.equal(M.oldRowOn({ ...s, tkHome: false }), true);
  assert.equal(M.oldRowOn({ ...s, tkHome: false, homeRow: false }), false);
  for (const [k] of M.PARTS) assert.equal(typeof M.DEFAULTS[k], "boolean", k);
});

test("guard: a throw turns that part off once, logged, and returns the fallback", () => {
  const logged = [];
  let changes = 0;
  const g = M.makeGuard((part, msg) => logged.push([part, msg]), () => changes++);
  assert.equal(g.run("a", () => 1, 0), 1);
  assert.equal(g.run("a", () => { throw new Error("Steam changed"); }, 0), 0);
  assert.equal(g.run("a", () => 5, 0), 0, "turned off: not run again");
  g.run("a", () => { throw new Error("again"); }, 0);
  assert.deepEqual(logged, [["a", "Error: Steam changed"]], "logged once");
  assert.equal(changes, 1);
  assert.equal(g.run("b", () => 2, 0), 2, "other parts carry on");
  const quiet = M.makeGuard(() => { throw new Error("log broke"); }, () => { throw new Error("listener broke"); });
  assert.equal(quiet.run("c", () => { throw "x"; }, "fallback"), "fallback", "a broken log never throws");
  assert.equal(quiet.broken.has("c"), true);
});

const game = (id, extra = {}) => ({ id, title: id, source: id.split(":")[0], store: "", tab: id.split(":")[0], installed: true, state: "installed", ...extra });

test("home: Hearth's order of shelves", () => {
  assert.deepEqual(M.homeShelves(null), []);
  const h = { hero: null, count: 3, continue: [game("epic:a")], favorites: [game("gog:b")],
              rows: [{ id: "steam", title: "Steam", total: 1, items: [game("steam:1")] }, { id: "gog", title: "GOG", total: 0, items: [] }],
              added: [game("a:1"), game("a:2")] };
  const s = M.homeShelves(h);
  assert.deepEqual(s.map((x) => x.key), ["continue", "fav", "row-steam"], "empty rows and a short Recently added are left out");
  assert.equal(s[0].wide, true);
  h.added.push(game("a:3"));
  assert.equal(M.homeShelves(h).at(-1).key, "added");
});

test("library tabs: Steam's own stay Steam's", () => {
  const t = M.extraLibraryTabs([{ id: "steam", title: "Steam", count: 9 }, { id: "epic", title: "Epic", count: 2 }, { id: "gog", title: "GOG", count: 0 }]);
  assert.deepEqual(t.map((x) => x.id), ["epic"]);
  assert.deepEqual(M.extraLibraryTabs(undefined), []);
});

test("game pages: which Hearth game a Steam app is", () => {
  const map = { "epic:a": 3000000001 };
  assert.equal(M.hearthIdForApp(map, "3000000001"), "epic:a");
  assert.equal(M.hearthIdForApp(map, 570), "steam:570");
  assert.equal(M.hearthIdForApp(map, 3000000002), null, "someone else's shortcut");
  assert.equal(M.hearthIdForApp(map, "x"), null);
  assert.equal(M.hearthIdForApp(map, 0), null);
});

test("words and numbers", () => {
  assert.equal(M.bytes(0), "0 B");
  assert.equal(M.bytes(1500), "1.5 KB");
  assert.equal(M.bytes(512e9), "512 GB");
  assert.equal(M.bytes(null), "");
  assert.equal(M.speed(12.5e6), "12.5 MB/s");
  assert.equal(M.speed(0), "");
  assert.equal(M.percent(0.426), 43);
  assert.equal(M.percent(2), 100);
  assert.equal(M.played(1), "Played 1 minute");
  assert.equal(M.played(180), "Played 3 hours");
  assert.equal(M.played(0), "");
  assert.equal(M.downloadLine({ state: "downloading", progress: 0.42, bytesPerSec: 12.5e6 }), "Downloading · 42% · 12.5 MB/s");
  assert.equal(M.downloadLine({ state: "queued", stage: "Waiting for Steam" }), "Waiting for Steam");
  assert.equal(M.driveLine({ free: 120e9, size: 512e9 }), "120 GB free of 512 GB");
  assert.deepEqual(M.blipFor("move"), [700, 0.035, 0.035], "Hearth's own move sound (ui/index.html blip)");
});

test("ids: the same rule as hearth-core", () => {
  for (const ok of ["epic:a", "emu:snes:A b.sfc"]) assert.equal(M.validId(ok), true, ok);
  for (const bad of ["", "x", 'a:"', "a:\\", "a:\n", null, 5]) assert.equal(M.validId(bad), false, String(bad));
});

test("places and stores: Hearth's", () => {
  assert.deepEqual(M.STORE_TABS.map((t) => t[0]), ["steam", "epic", "gog", "free", "xbox", "battlenet", "recomps"]);
  assert.ok(M.DESTS.every((d) => !!d.route !== !!d.app), "each place is a page or an app");
  assert.ok(M.DESTS.some((d) => d.id === "theater" && d.app === "hearth-app:theater"));
});

test("art: portrait cover first, then Steam's own picture, then wide art; no blanks or repeats", () => {
  assert.deepEqual(M.artCandidates({ id: "gog:1", cover: "/c.webp", wide: "https://w", hero: "https://w" }, "portrait"), ["/c.webp", "https://w"]);
  assert.deepEqual(M.artCandidates({ id: "steam:620", cover: "", wide: null }, "portrait"), ["steam-lib:620:portrait", "steam-lib:620:wide"],
    "a Steam game without Hearth art uses Steam's library cache");
  assert.deepEqual(M.artCandidates({ id: "steam:620", cover: "nimbus-art://cache/abc" }, "portrait")[0], "nimbus-art://cache/abc");
  assert.deepEqual(M.artCandidates({ id: "epic:x", cover: "c", wide: "w", hero: "h" }, "wide"), ["w", "h", "c"]);
  assert.deepEqual(M.artCandidates({ id: "epic:x", cover: "c", wide: "w", hero: "h" }, "hero"), ["h", "w"]);
  assert.deepEqual(M.artCandidates({ id: "battlenet:bnet", cover: "  ", wide: "" }, "portrait"), [], "nothing: the card shows the title");
  assert.deepEqual(M.artCandidates(null, "portrait"), []);
  assert.equal(M.steamAppId("steam:570"), "570");
  assert.equal(M.steamAppId("steam:abc"), "");
  assert.equal(M.steamAppId("gog:570"), "");
});

test("art: web pictures straight, then through the backend; files only through the backend", () => {
  assert.deepEqual(M.artAttempts(["https://a/b.jpg", "/home/me/c.png", "data:image/png;base64,x", "nimbus-art://cache/x"]), [
    { ref: "https://a/b.jpg", via: "direct" }, { ref: "https://a/b.jpg", via: "backend" },
    { ref: "/home/me/c.png", via: "backend" }, { ref: "data:image/png;base64,x", via: "direct" }, { ref: "nimbus-art://cache/x", via: "backend" },
  ]);
  assert.deepEqual(M.artAttempts(["http://plain/x.jpg"]), [{ ref: "http://plain/x.jpg", via: "backend" }], "plain http only from Hearth's cache");
});

test("art: the log gets the kind of picture, never its address", () => {
  const kinds = ["https://x/secret", "http://x", "data:image/png;base64,", "nimbus-art://cache/ab", "nimbus-art://file//home/me/a.png",
    "steam-lib:1:wide", "file:///a", "~/a.png", "/home/me/a.png", "weird", ""].map(M.refKind);
  assert.deepEqual(kinds, ["web", "plain-http", "data", "art-cache", "nimbus-file", "steam-library", "file-url", "home-path", "path", "other", "none"]);
});

test("update row: check, update with progress, ready for a restart, errors", () => {
  assert.deepEqual(M.updateView(null), { line: "", button: "Check for Hearth updates", action: "check", poll: false, pct: null });
  assert.equal(M.updateView({ state: "available", message: "An update is available (0.6.61)" }).action, "start");
  const r = M.updateView({ state: "running", pct: 37.6 });
  assert.deepEqual([r.line, r.button, r.poll, r.pct], ["Updating Hearth OS… 38%", "", true, 38], "no button while it runs; asked again");
  assert.equal(M.updateView({ state: "running", pct: 400 }).pct, 100);
  assert.equal(M.updateView({ state: "ready", pct: 100 }).line, "Ready: restart to finish updating");
  assert.equal(M.updateView({ state: "none" }).line, "Hearth OS is up to date");
  assert.equal(M.updateView({ state: "error", message: "another update is already running" }).line, "another update is already running");
  assert.equal(M.updateView({ state: "error" }).button, "Try again");
});

test("main menu: Hearth's places in Hearth's order, Home where the takeover has it, power options", () => {
  const s = { ...M.DEFAULTS };
  const p = M.menuPlaces(s);
  assert.deepEqual(p.map((d) => d.title), ["Hearth Home", "Hearth Store", "Hearth Library", "Theater", "Hearth Mods", "Storage", "Downloads", "Hearth Friends", "Hearth Link"], "Couch Co-op is dropped for now");
  assert.equal(p[0].route, "/library/home", "Hearth's home is Steam's Home with the takeover");
  assert.equal(M.menuPlaces({ ...s, tkHome: false })[0].route, "/hearth/home");
  assert.equal(M.menuPlaces(s, new Set(["tkHome"]))[0].route, "/hearth/home", "turned off by itself: its own page");
  for (const d of p) assert.ok(d.route || d.app, d.id);
  assert.deepEqual(M.MENU_POWER.map((x) => x[0]), ["hearth-desktop"], "no Switch to Hearth Game Mode: Steam Game Mode is Hearth OS's Game Mode");
  assert.equal(M.MULTIPLAYER, false, "couch co-op and multi-seat are hidden");
});

test("the game Steam runs: shortcuts are Hearth's games, Steam's own are steam:<id>", () => {
  const map = { "epic:alan": 3000000001 };
  assert.deepEqual(M.runningGame({ appid: "3000000001", display_name: "Alan Wake 2" }, map), { appid: 3000000001, id: "epic:alan", title: "Alan Wake 2", hearth: true });
  assert.deepEqual(M.runningGame({ appid: 620, display_name: "Portal 2" }, map), { appid: 620, id: "steam:620", title: "Portal 2", hearth: false });
  assert.equal(M.runningGame({ appid: 3000000009, display_name: "Someone else's shortcut" }, map).id, null);
  assert.equal(M.runningGame(null, map), null);
  assert.equal(M.runningGame({ appid: 0 }, map), null);
});

test("front: Hearth Input only gets a game when it's switched on", () => {
  const run = { appid: 5, id: "gog:a", title: "A", hearth: true };
  assert.deepEqual(M.frontMessage(run, M.DEFAULTS, undefined, false), { appid: 5, game: "gog:a", overlay: false });
  assert.equal(M.frontMessage(run, { ...M.DEFAULTS, tkInput: false }, undefined, false).game, "", "off: no game, the mapper lets go");
  assert.equal(M.frontMessage(run, { ...M.DEFAULTS, takeover: false }, undefined, false).game, "");
  assert.equal(M.frontMessage(run, M.DEFAULTS, new Set(["tkInput"]), true).game, "");
  assert.equal(M.frontMessage(null, M.DEFAULTS, undefined, false).game, "");
  assert.equal(M.frontMessage({ ...run, id: null }, M.DEFAULTS, undefined, false).game, "");
});

test("Quick Access tabs: three of Hearth's (Players is dropped with Couch Co-op), added and taken away with the switch", () => {
  assert.deepEqual(M.QAM_TABS.map((t) => t.key), ["hearth", "perf", "controller"]);
  for (const t of M.QAM_TABS) assert.ok(t.id > 999, "never one of Steam's or Decky's ids");
  const ids = M.QAM_TABS.map((t) => t.id);
  assert.deepEqual(M.qamPlan(M.DEFAULTS, undefined, []), { add: ids, remove: [] });
  assert.deepEqual(M.qamPlan(M.DEFAULTS, undefined, ids), { add: [], remove: [] });
  assert.deepEqual(M.qamPlan({ ...M.DEFAULTS, tkQam: false }, undefined, ids), { add: [], remove: ids });
  assert.deepEqual(M.qamPlan(M.DEFAULTS, new Set(["tkQam"]), [ids[0]]), { add: [], remove: [ids[0]] }, "turned off by itself");
});

test("in-game section: layout, Hearth Input, Proton and mods lines", () => {
  const v = { id: "gog:a", title: "A", source: "gog", store: "GOG", kind: "proton", mods: true, proton: "GE-Proton",
    input: { layout: "shooter", name: "Shooter / action", choices: [{ id: "off", label: "Off" }, { id: "shooter", label: "Shooter / action" }] },
    protonChoices: [{ id: "", label: "Default" }, { id: "GE-Proton", label: "GE-Proton (newest)" }] };
  assert.equal(M.layoutLabel(v), "Shooter / action");
  assert.equal(M.layoutLabel({ ...v, input: { ...v.input, layout: "off" } }), "Off");
  assert.equal(M.layoutLabel(null), "");
  assert.equal(M.protonLabel(v), "GE-Proton (newest)");
  assert.equal(M.protonLabel({ ...v, proton: "" }), "Default");
  assert.equal(M.protonLabel({ ...v, kind: "steam" }), "", "Steam's games: Steam's own Proton setting");
  assert.equal(M.inputLine(null, false), "Hearth Input is off in Steam Game Mode");
  assert.equal(M.inputLine({ game: null }, true), "No game of yours is running");
  assert.equal(M.inputLine({ game: "gog:a", layout: null }, true), "No Hearth layout: the game gets the controller");
  assert.equal(M.inputLine({ game: "gog:a", layout: "Shooter", active: true }, true), "Shooter: on");
  assert.equal(M.inputLine({ game: "gog:a", layout: "Shooter", active: false, overlay: true }, true), "Shooter: on in the game (paused while Quick Access is open)");
  assert.equal(M.modsLine({ count: 12, enabled: 9 }), "12 mods, 9 on");
  assert.equal(M.modsLine({ count: 1, enabled: 1 }), "1 mod, 1 on");
  assert.equal(M.modsLine({ count: 0 }), "No mods yet");
  assert.equal(M.modsLine({ beacon: true, count: 3 }), "Beacon manages this game's mods");
  assert.equal(M.tdpLine({ tdp: { supported: true, watts: 15.4, min: 5, max: 30 } }), "Power limit: 15 W (Hearth's, 5–30 W)");
  assert.equal(M.tdpLine({ tdp: { supported: false } }), "");
  assert.equal(M.tdpLine(null), "");
});

test("players: moving a controller, the tab's lines", () => {
  assert.deepEqual(M.movePad(["a", "b", "c"], 1, -1), ["b", "a", "c"]);
  assert.deepEqual(M.movePad(["a", "b", "c"], 1, 1), ["a", "c", "b"]);
  assert.deepEqual(M.movePad(["a", "b"], 0, -1), ["a", "b"], "the first can't go up");
  assert.deepEqual(M.movePad(["a", "b"], 1, 1), ["a", "b"], "the last can't go down");
  const pad = (id, player) => ({ id, name: id, label: id, kind: "physical", player, playerName: "", rumble: true });
  assert.equal(M.coopLine(null), "Looking for controllers…");
  assert.match(M.coopLine({ pads: [pad("a", 1)], max: 4, running: false }), /connect another/);
  assert.match(M.coopLine({ pads: [pad("a", 1), pad("b", 2)], max: 4, running: false }), /^2 players/);
  assert.match(M.coopLine({ pads: ["a", "b", "c", "d", "e"].map((x, i) => pad(x, i < 4 ? i + 1 : null)), max: 4, running: false }), /1 more waits/);
  assert.equal(M.coopProblem({ pads: [], max: 4, running: false, lastError: "x", lastErrorAt: 1000 }, 2000), "x");
  assert.equal(M.coopProblem({ pads: [], max: 4, running: false, lastError: "x", lastErrorAt: 1000 }, 1000 + 7200e3), "", "old news");
  assert.equal(M.coopProblem({ pads: [], max: 4, running: true, lastError: "x" }, 0), "");
});

test("seats: words, actions, who can get a controller", () => {
  const st = { configured: true, enabled: true, seats: [
    { seat: 1, user: "bob", mode: "nested", state: "active" },
    { seat: 2, user: "hearth-p2", mode: "nested", state: "running" },
    { seat: 3, user: "hearth-p3", mode: "stream", state: "stopped" },
    { seat: 4, user: "hearth-p4", mode: "gpu", state: "failed" }] };
  assert.equal(M.seatState(st.seats[0]), "Playing here (Steam Game Mode)");
  assert.equal(M.seatState(st.seats[1]), "Playing (Hearth Game Mode)");
  assert.equal(M.seatAction(st.seats[0]), null, "player 1 is never started or stopped here");
  assert.equal(M.seatAction(st.seats[1]), "stop");
  assert.equal(M.seatAction(st.seats[2]), "start");
  assert.equal(M.seatAction(st.seats[3]), "start");
  assert.deepEqual(M.seatTargets(st), [1, 2, 4], "streamed players' controllers come from their device");
  assert.deepEqual(M.seatTargets({ configured: false, enabled: false, seats: [] }), []);
  assert.match(M.seatAccount(st.seats[2]), /hearth-p3 · own account/);
  assert.match(M.seatsLine(st), /^1 of 3 other players playing/);
  assert.match(M.seatsLine({ configured: false, enabled: false, seats: [] }), /isn't set up/);
  assert.equal(M.seatsLine({ error: "x" }), "x");
  assert.match(M.seatDone("start", 2, { restart_game_mode: true }), /next time Steam Game Mode starts/);
  assert.equal(M.seatDone("start", 3, {}), "Player 3 is starting");
});

test("the rest of Hearth's Game Mode: Quick Resume, Sunshine's PIN and line, Hearth Friends", () => {
  assert.equal(M.asideLine({ title: "Portal 2", at: 1000 }, 1000 + 3 * 60000), "Portal 2 · put aside 3 min ago");
  assert.equal(M.asideLine({ title: "Portal 2", at: 1000 }, 1000), "Portal 2 · put aside just now");
  assert.equal(M.asideLine({}, 5), "A game");
  let pin = "";
  for (const k of ["1", "x", "2", "3", "4", "5"]) pin = M.pinPress(pin, k);
  assert.equal(pin, "1234", "4 digits at most, digits only");
  assert.equal(M.pinPress(pin, "del"), "123");
  assert.equal(M.pinPress("", "del"), "");
  assert.match(M.remoteLine(null), /Asking/);
  assert.match(M.remoteLine({ installed: false }), /isn't on this system/);
  assert.equal(M.remoteLine({ installed: true, on: false }), "Game streaming is off.");
  assert.equal(M.remoteLine({ installed: true, on: true, clients: [{ name: "TV" }], addresses: ["192.168.1.4"] }), "Streaming is on at 192.168.1.4: 1 device paired (TV).");
  assert.equal(M.friendLine({ presence: "online", game: "Hades", joinable: true }), "Playing Hades · can be joined (in Hearth)");
  assert.equal(M.friendLine({ presence: "online" }), "Online");
  assert.equal(M.friendLine({ presence: "offline" }), "Offline");
});
