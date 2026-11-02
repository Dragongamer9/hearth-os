// Unit tests of Hearth Sync's pure parts (src/model.ts), run in CI with Node's own test runner and type stripping:
//   node --experimental-strip-types --test hosts/decky-hearth-sync/tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import * as M from "../src/model.ts";

const item = (id, store, extra = {}) => ({ id, title: id, source: store, store, storeName: store.toUpperCase(), cloud: false,
  cover: `/c/${id}.png`, wide: `/w/${id}.jpg`, hero: `/w/${id}.jpg`, logo: "", icon: "", exe: "/bin/sh", startDir: "/home/you",
  options: `-c x hearth-steam /usr/bin/nimbus --play-now ${id}`, ...extra });
const done = (it) => ({ t: M.targetKey(it), art: { cover: it.cover, wide: it.wide, hero: it.hero } });

test("new games are added with all their pictures", () => {
  const a = item("epic:a", "epic");
  const p = M.plan([a], {}, {}, {}, () => false);
  assert.deepEqual(p.add.map((i) => i.id), ["epic:a"]);
  assert.deepEqual(p.art[0].kinds, ["cover", "wide", "hero"], "no logo: none asked for");
  assert.deepEqual(p.remove, []);
});

test("nothing to do when nothing changed", () => {
  const a = item("epic:a", "epic");
  const p = M.plan([a], { "epic:a": 5 }, { "epic:a": done(a) }, {}, () => true);
  assert.deepEqual([p.add, p.update, p.remove, p.forget, p.art], [[], [], [], [], []]);
});

test("a changed title or target is updated, a new picture is set", () => {
  const a = item("epic:a", "epic");
  const b = { ...a, title: "New name", logo: "nimbus-art://cache/0123456789abcdef01234567" };
  const p = M.plan([b], { "epic:a": 5 }, { "epic:a": done(a) }, {}, () => true);
  assert.deepEqual(p.update.map((i) => i.id), ["epic:a"]);
  assert.deepEqual(p.art[0].kinds, ["logo"]);
});

test("a picture that failed before is tried again", () => {
  const a = item("gog:b", "gog");
  const p = M.plan([a], { "gog:b": 7 }, { "gog:b": { t: M.targetKey(a), art: { wide: a.wide, hero: a.hero } } }, {}, () => true);
  assert.deepEqual(p.art[0].kinds, ["cover"]);
  assert.equal(M.missingArt([a], { "gog:b": 7 }, { "gog:b": { t: "", art: {} } }, {}), 1);
  assert.equal(M.missingArt([a], { "gog:b": 7 }, { "gog:b": done(a) }, {}), 0);
});

test("only Hearth Sync's own shortcuts are removed: gone games and stores turned off", () => {
  const a = item("epic:a", "epic"), g = item("gog:b", "gog");
  const map = { "epic:a": 5, "gog:b": 6, "epic:gone": 7, "epic:deleted-by-hand": 8 };
  const exists = (id) => id !== 8;
  const p = M.plan([a, g], map, {}, { gog: false }, exists);
  assert.deepEqual(p.remove.sort(), ["epic:gone", "gog:b"]);
  assert.deepEqual(p.forget, ["epic:deleted-by-hand"], "one you removed in Steam is just forgotten");
  assert.ok(!p.remove.includes("epic:a"));
  // Nothing outside the map can ever be in remove.
  for (const id of p.remove) assert.ok(id in map);
});

test("a shortcut removed in Steam is added again", () => {
  const a = item("epic:a", "epic");
  const p = M.plan([a], { "epic:a": 5 }, { "epic:a": done(a) }, {}, () => false);
  assert.deepEqual(p.add.map((i) => i.id), ["epic:a"]);
  assert.deepEqual(p.art[0].kinds, ["cover", "wide", "hero"], "a new shortcut gets every picture again");
});

test("the timer acts only on a change or when missing art is due", () => {
  const st = { lastSync: 1, lastArtTry: 0, synced: 1, artMissing: 0, signature: "s1", problems: [], off: "" };
  assert.equal(M.shouldAct("s1", st, 10), false);
  assert.equal(M.shouldAct("s2", st, 10), true);
  assert.equal(M.shouldAct("", st, 10), false, "no list: nothing done");
  assert.equal(M.shouldAct("s1", { ...st, artMissing: 2, lastArtTry: 0 }, M.ART_RETRY_EVERY - 1), false);
  assert.equal(M.shouldAct("s1", { ...st, artMissing: 2, lastArtTry: 0 }, M.ART_RETRY_EVERY), true);
});

test("collections per store, store list for the switches", () => {
  const items = [item("epic:a", "epic"), item("epic:b", "epic"), item("gog:c", "gog"), item("emu:d", "emu")];
  assert.deepEqual(M.groups(items, { "epic:a": 1, "epic:b": 2, "gog:c": 3 }, { gog: false }), { EPIC: [1, 2] });
  assert.deepEqual(M.storeList(items), [["emu", "EMU", 1], ["epic", "EPIC", 2], ["gog", "GOG", 1]]);
  assert.equal(M.storeOn({}, "epic"), true, "every store on by default");
  assert.equal(M.storeOn({ epic: false }, "epic"), false);
});

test("asset types are Steam's", () => {
  assert.deepEqual(Object.fromEntries(M.ART), { cover: 0, hero: 1, logo: 2, wide: 3 });
  assert.equal(M.ago(0, 5), "never");
  assert.equal(M.ago(0 + 1, 1 + 3 * 60000), "3 minutes ago");
});
