// Unit tests of Hearth Music's pure parts (src/model.ts), run in CI with Node's own test runner and type stripping:
//   node --experimental-strip-types --test hosts/decky-hearth-music/tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import * as M from "../src/model.ts";

const seen = (path, extra = {}) => ({ path, focused: 0, running: 0, launching: false, ...extra });
const on = (extra = {}) => ({ ...M.DEFAULTS, ...extra });

test("a game page plays its game; the library only when asked", () => {
  assert.equal(M.pageApp("/library/app/570"), 570);
  assert.equal(M.pageApp("/library/app/570/achievements"), 570);
  assert.equal(M.pageApp("/library/home"), 0);
  assert.equal(M.wanted(seen("/library/app/570"), on()), 570);
  assert.equal(M.wanted(seen("/library/home", { focused: 620 }), on()), 0, "game pages only by default");
  assert.equal(M.wanted(seen("/library/home", { focused: 620 }), on({ where: "library" })), 620);
  assert.equal(M.wanted(seen("/library/app/570", { focused: 620 }), on({ where: "library" })), 570, "the page wins");
  assert.equal(M.wanted(seen("/store", { focused: 620 }), on({ where: "library" })), 0, "not outside the library");
  assert.equal(M.wanted(seen("/library/home"), on({ where: "library" })), 0, "nothing highlighted");
});

test("silent while a game runs, after Play, and when off", () => {
  assert.equal(M.wanted(seen("/library/app/570", { running: 570 }), on()), 0);
  assert.equal(M.wanted(seen("/library/app/570", { launching: true }), on()), 0);
  assert.equal(M.wanted(seen("/library/app/570"), on({ enabled: false })), 0);
  assert.equal(M.silentWhy(seen("/library/app/570", { running: 570 }), on(), null), "Silent while a game runs");
  assert.equal(M.silentWhy(seen("/library/app/570"), on({ enabled: false }), null), "Hearth Music is off");
  assert.equal(M.silentWhy(seen("/library/app/570"), on(), { source: null, why: "no theme song found" }), "Silent: no theme song found");
  assert.equal(M.silentWhy(seen("/library/app/570"), on(), { source: "apple", key: "k" }), "");
  assert.equal(M.silentWhy(seen("/library/home"), on(), null), "Open a game's page to hear its theme");
});

test("fades are smooth and end where they should", () => {
  assert.equal(M.fadeAt(0, 0.5, 0, 1000), 0);
  assert.equal(M.fadeAt(0, 0.5, 1000, 1000), 0.5);
  assert.equal(M.fadeAt(0.5, 0, 2000, 1000), 0);
  const mid = M.fadeAt(0, 1, 500, 1000), out = M.fadeAt(1, 0, 500, 1000);
  assert.ok(Math.abs(mid - Math.SQRT1_2) < 1e-9 && Math.abs(out - Math.SQRT1_2) < 1e-9, "equal power: no dip in a crossfade");
  let last = -1;
  for (let t = 0; t <= 1000; t += 50) { const v = M.fadeAt(0, 0.8, t, 1000); assert.ok(v >= last); last = v; }
  assert.equal(M.fadeAt(0, 2, 5000, 1000), 1, "clamped");
});

test("now playing and Apple's credit", () => {
  const apple = { source: "apple", title: "Elden Ring", artist: "Tsukasa Saitoh", album: "ELDEN RING (Original Soundtrack)" };
  assert.equal(M.nowLine(apple), "Elden Ring — Tsukasa Saitoh");
  assert.equal(M.credit(apple), "Preview courtesy of Apple Music · ELDEN RING (Original Soundtrack)");
  assert.equal(M.credit({ source: "soundtrack", title: "x" }), "");
  assert.equal(M.nowLine({ source: "pick", title: "Mix", via: "collection", name: "RPGs" }), "Mix (your pick for RPGs)");
  assert.equal(M.nowLine({ source: null }), "");
});

test("settings are cleaned", () => {
  assert.deepEqual(M.cleanSettings({ volume: 4, delay: -1, where: "x" }), { ...M.DEFAULTS, volume: 1, delay: 0, where: "pages" });
  assert.deepEqual(M.cleanSettings(null), M.DEFAULTS);
});
