// Hearth Eras' pure parts (src/model.ts). CI: node --experimental-strip-types --import ./tests/ts-resolve.mjs --test tests/eras.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import {
  ERAS, ERA_IDS, DEFAULTS, MAX_PRESETS, PAUSED_NOTE, BTN, normalize, presetOf, decide, buildSkin, presetCss, quickAction, stepEra,
  applyQuick, quickLabel, soundRecipe, sectionsOf, gridGames, subtitle, clock, dateLine, diagnosticsText, eraById,
} from "../src/model.ts";
import { checkSteam } from "../src/classic/model.ts";

const dir = new URL("../defaults/eras/", import.meta.url);
const read = (f) => readFileSync(new URL(f, dir), "utf8");
const brand = { base: read("base.css"), eras: Object.fromEntries(readdirSync(dir).filter((f) => f !== "base.css").map((f) => [f.replace(".css", ""), read(f)])), logo: "data:L", wallpaper: "data:W" };
brand.eras.classic = readFileSync(new URL("../../decky-hearth-look/defaults/hearth-classic.css", import.meta.url), "utf8");
const all = (m, k) => `${m}_${k}`;
const none = () => undefined;

test("the eras: Steam first, every era with presets and a name of Hearth's own", () => {
  assert.deepEqual(ERA_IDS, ["steam", "channels", "blades", "tiles", "crossbar", "pocket", "classic"]);
  assert.equal(ERAS[0].presets.length, 0);
  for (const e of ERAS.slice(1)) {
    assert.ok(e.presets.length >= 1 && e.presets.length <= MAX_PRESETS, e.id);
    assert.ok(e.name && e.blurb, e.id);
    for (const p of e.presets) for (const k of ["bg", "bg2", "panel", "panel2", "text", "dim", "accent", "ink"]) assert.ok(p.vars[k], `${e.id}.${p.id}.${k}`);
    if (e.id !== "classic") assert.ok(brand.eras[e.id], `${e.id} has a skin`);
  }
  assert.equal(eraById("nope").id, "steam");
});

test("settings: Steam's look and every extra off by default; only valid values kept", () => {
  assert.equal(DEFAULTS.era, "steam");
  for (const k of ["sounds", "quickSwitch", "loading", "keyboard", "pages"]) assert.equal(DEFAULTS[k], false, k);
  assert.deepEqual(normalize(null), DEFAULTS);
  assert.deepEqual(normalize({ era: "x", sounds: "yes", presets: { steam: 1, tiles: 5, blades: 2, nope: 0 } }), { ...DEFAULTS, presets: { blades: 2 } });
  assert.deepEqual(normalize({ era: "crossbar", keyboard: true }), { ...DEFAULTS, era: "crossbar", keyboard: true });
  // A preset beyond an era's list reads as its first.
  assert.equal(presetOf(normalize({ presets: { classic: 2 } }), "classic").id, "hearth");
  assert.equal(presetOf(DEFAULTS, "steam"), null);
});

test("decide: off for Steam or off Hearth OS, paused with the reason when Steam changed", () => {
  const s = { ...DEFAULTS, era: "tiles" };
  assert.equal(decide(DEFAULTS, true, null).state, "off");
  assert.equal(decide(s, false, null).state, "off");
  assert.equal(decide(s, true, checkSteam(all, true)).state, "on");
  const d = decide(s, true, checkSteam(none, false));
  assert.equal(d.state, "paused");
  assert.match(d.why, /gamepadTabbedPageClasses\.TabHeaderRowWrapper.*routerHook/);
  assert.equal(decide(s, true, checkSteam(all, true), "boom").why, "boom");
  assert.match(PAUSED_NOTE, /^Hearth Eras paused/);
});

test("the skin: preset variables, base and era rules with Steam's names; extras only when on", () => {
  for (const e of ERA_IDS.slice(1, 6)) {
    const s = { ...DEFAULTS, era: e };
    const { css, dropped } = buildSkin(brand, s, all);
    assert.match(css, /:root\{--era-bg:/, e);
    assert.ok(css.includes(".gamepadTabbedPageClasses_Tab{"), e);
    assert.ok(!css.includes("{{"), e);
    assert.equal(dropped, 0, e);
    assert.ok(!css.includes("VirtualKeyboardContainer"), `${e}: keyboard is off`);
    const on = buildSkin(brand, { ...s, keyboard: true, loading: true, pages: true }, all).css;
    assert.ok(on.includes(".VirtualKeyboardContainer_VirtualKeyboardContainer{"), e);
    assert.ok(on.includes("throbberClasses_Throbber"), e);
    // The pictures go in where an era asks for them.
    if (e === "pocket") assert.ok(css.includes("url(data:W)"));
  }
  // A Steam without some names: those lines are left out, never half-filled.
  const partial = buildSkin(brand, { ...DEFAULTS, era: "blades" }, (m, k) => (m === "gamepadTabbedPageClasses" ? all(m, k) : undefined));
  assert.ok(partial.dropped > 0 && !partial.css.includes("{{") && !partial.css.includes("._"));
  // Steam's look: nothing.
  assert.deepEqual(buildSkin(brand, DEFAULTS, all), { css: "", dropped: 0 });
  assert.equal(buildSkin(null, { ...DEFAULTS, era: "tiles" }, all).css, "");
});

test("the Hearth Classic era uses Hearth Classic's skin without its menu bars", () => {
  const { css } = buildSkin(brand, { ...DEFAULTS, era: "classic" }, all);
  assert.ok(css.includes("--hearth-accent:#3fd0b8"));
  assert.ok(!css.includes("html.hearth-bar #MainNavMenuContainer"));
  assert.ok(!/^html\.hearth-(rail|bar|ghost)/m.test(css));
});

test("preset variables: only safe values", () => {
  assert.equal(presetCss({ id: "x", name: "x", vars: { bg: "#000", bad: "red;}body{x:1", "Bad-Key": "#fff" } }), ":root{--era-bg:#000}");
  assert.equal(presetCss(null), "");
});

test("quick switching: hold L1, then D-pad left/right or L2/R2; a held button acts once", () => {
  assert.equal(quickAction(0, BTN.RIGHT), null);                       // L1 not held
  assert.equal(quickAction(BTN.L1, BTN.L1 | BTN.RIGHT), "era+");
  assert.equal(quickAction(BTN.L1, BTN.L1 | BTN.LEFT), "era-");
  assert.equal(quickAction(BTN.L1, BTN.L1 | BTN.R2), "preset+");
  assert.equal(quickAction(BTN.L1, BTN.L1 | BTN.L2), "preset-");
  assert.equal(quickAction(BTN.L1 | BTN.RIGHT, BTN.L1 | BTN.RIGHT), null);   // still held: nothing more
  assert.equal(quickAction(BTN.RIGHT, BTN.L1 | BTN.RIGHT), null);           // D-pad first, then L1: not a new press
  assert.equal(stepEra("steam", 1), "channels");
  assert.equal(stepEra("steam", -1), "classic");
  assert.equal(stepEra("classic", 1), "steam");
  assert.deepEqual(applyQuick(DEFAULTS, "era+"), { era: "channels" });
  assert.deepEqual(applyQuick({ ...DEFAULTS, era: "tiles" }, "preset+"), { presets: { tiles: 1 } });
  assert.deepEqual(applyQuick({ ...DEFAULTS, era: "tiles" }, "preset-"), { presets: { tiles: 2 } });
  assert.equal(applyQuick(DEFAULTS, "preset+"), null);                    // Steam's look has no presets
  assert.equal(applyQuick({ ...DEFAULTS, era: "classic" }, "preset+"), null);
  assert.equal(quickLabel({ ...DEFAULTS, era: "crossbar", presets: { crossbar: 1 } }), "Crossbar · Sea");
  assert.equal(quickLabel(DEFAULTS), "Steam's look");
});

test("sounds: every era has quiet move, ok and back sounds made of plain waves", () => {
  for (const e of ERA_IDS.slice(1)) for (const k of ["move", "ok", "back"]) {
    const r = soundRecipe(e, k);
    assert.ok(r.length >= 1, `${e}.${k}`);
    for (const t of r) {
      assert.ok(t.v > 0 && t.v <= 0.06, `${e}.${k} volume ${t.v}`);
      assert.ok(t.d > 0 && t.d <= 0.25, `${e}.${k} length ${t.d}`);
      assert.ok(t.noise || (t.f > 100 && t.f < 5000), `${e}.${k} frequency`);
    }
  }
  assert.deepEqual(soundRecipe("steam", "move"), []);
});

test("the homes' data: sections, a grid without repeats, subtitles, the clock", () => {
  const c = (appid, title) => ({ appid, title, store: "Steam", minutes: 90 });
  const view = { from: "hearth", hero: null, count: 4, shelves: [
    { key: "continue", title: "Continue playing", items: [c(1, "A"), c(2, "B")] },
    { key: "fav", title: "Favorites", items: [c(2, "B"), c(3, "C")] },
    { key: "row-epic", title: "Epic", items: [] },
  ] };
  assert.deepEqual(sectionsOf(view).map((s) => s.title), ["Recent", "Favorites"]);
  assert.deepEqual(gridGames(view, 12).map((g) => g.appid), [1, 2, 3]);
  assert.deepEqual(gridGames(view, 2).map((g) => g.appid), [1, 2]);
  assert.deepEqual(sectionsOf(null), []);
  assert.equal(subtitle(c(1, "A")), "Steam · 1.5 h played");
  const d = new Date(2026, 10, 4, 9, 5);
  assert.equal(clock(d), "09:05");
  assert.equal(clock(d, false), "9:05");
  assert.equal(dateLine(d), "Wed 4/11");
});

test("diagnostics name the era, the extras and what's missing", () => {
  const t = diagnosticsText({ state: "paused", why: "missing in Steam: x", settings: { ...DEFAULTS, era: "pocket" }, keys: { resolved: ["a"], missing: ["b.c"] }, windows: ["SP"], dropped: 2, quick: "off" });
  assert.match(t, /Hearth Eras: paused \(missing in Steam: x\)/);
  assert.match(t, /Era: pocket \(aurora\)/);
  assert.match(t, /1 found, 1 missing: b\.c; 2 skin lines left out/);
});
