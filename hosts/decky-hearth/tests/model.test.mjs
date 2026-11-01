// Unit tests of the takeover's pure parts (src/model.ts), run in CI with Node's own test runner and type stripping:
//   node --experimental-strip-types --test hosts/decky-hearth/tests/model.test.mjs
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
