// Hearth Eras' pure parts (no Steam, no Decky): the eras and their presets, the settings, what to do, the skin's
// assembly, the quick-switch buttons, the sounds' recipes and the homes' sections. tests/eras.test.mjs runs these in
// Node. The Hearth Classic era's parts are Hearth Classic's own (src/classic/, exact copies of hosts/decky-hearth-look).
import { PLACES, played, renderTheme } from "./classic/model";
import type { Card, HomeView, Place, Resolve, SteamCheck } from "./classic/model";

// ---------------------------------------------------------------- the eras

export type EraId = "steam" | "channels" | "blades" | "tiles" | "crossbar" | "pocket" | "classic";
/** A preset is an era's colours: CSS variables the skin and the era's home use (--era-*). */
export interface Preset { id: string; name: string; vars: Record<string, string> }
export type SoundKind = "move" | "ok" | "back";
export interface Era { id: EraId; name: string; blurb: string; presets: Preset[] }

const P = (id: string, name: string, bg: string, bg2: string, panel: string, panel2: string, text: string, dim: string, accent: string, ink: string, extra: Record<string, string> = {}): Preset =>
  ({ id, name, vars: { bg, bg2, panel, panel2, text, dim, accent, ink, ...extra } });

/** Hearth's own designs, each evoking a console era by layout, colour and motion; no one else's names, art or sounds. */
export const ERAS: Era[] = [
  { id: "steam", name: "Steam", blurb: "Steam's own look and Home.", presets: [] },
  { id: "channels", name: "Channels", blurb: "A rounded TV-channel grid on silver, with a clock tray and a soft click.", presets: [
    P("silver", "Silver", "#e9edf1", "#d9dfe5", "#f8fafb", "#ffffff", "#36414c", "#7b8793", "#2fa6d6", "#ffffff", { line: "#c7cfd7", page: "#2b333d" }),
    P("snow", "Snow", "#f4f6f8", "#e6ebef", "#ffffff", "#ffffff", "#2f3a44", "#808b96", "#e0773a", "#ffffff", { line: "#d3dae0", page: "#2e3640" }),
    P("night", "Night", "#1d232b", "#151a20", "#2a323c", "#323b46", "#e6ebf0", "#93a0ad", "#58c2ec", "#0b1a22", { line: "#3a4450", page: "#1d232b" }),
  ] },
  { id: "blades", name: "Blades", blurb: "Glossy panels you swipe through sideways.", presets: [
    P("ember", "Ember", "#1a1338", "#0f0b22", "#2a1f57", "#382a70", "#f5f1ff", "#b2a6d9", "#ff8a3d", "#2a1200"),
    P("ocean", "Ocean", "#0e2742", "#081a2d", "#16395e", "#1d4876", "#eef7ff", "#9dbbd8", "#3ec5ff", "#03253a"),
    P("plum", "Plum", "#2a1036", "#1a0923", "#3e1a50", "#4e2265", "#fff0fa", "#cfa6c8", "#ff5fa2", "#3a0019"),
  ] },
  { id: "tiles", name: "Tiles", blurb: "A big-tile dashboard with a hero banner.", presets: [
    P("graphite", "Graphite", "#15171c", "#0e1014", "#22252c", "#2c3039", "#f3f4f6", "#9aa1ad", "#ff6f59", "#2a0a04"),
    P("violet", "Violet", "#141224", "#0c0b17", "#211e3a", "#2b2750", "#f2f0ff", "#a29fc4", "#8f7bff", "#120b3a"),
    P("cobalt", "Cobalt", "#0f1726", "#090f1a", "#18243a", "#20304d", "#eef3fb", "#97a6bf", "#4d8dff", "#071733"),
  ] },
  { id: "crossbar", name: "Crossbar", blurb: "A category bar with lists under it, over slow waves.", presets: [
    P("dusk", "Dusk", "#1b2a6b", "#4a2470", "#22307a", "#2c3b8c", "#ffffff", "#c6cbef", "#ffffff", "#1b2a6b", { wave: "rgba(255,255,255,.16)" }),
    P("sea", "Sea", "#06424f", "#0b6f6a", "#0a5560", "#0d6773", "#ffffff", "#bfe3e6", "#ffffff", "#06424f", { wave: "rgba(255,255,255,.15)" }),
    P("rose", "Rose", "#5a1630", "#9c3d4f", "#6c1d3a", "#7f2746", "#ffffff", "#f0c6cf", "#ffffff", "#5a1630", { wave: "rgba(255,255,255,.15)" }),
  ] },
  { id: "pocket", name: "Pocket", blurb: "A handheld-style icon row over your wallpaper.", presets: [
    P("aurora", "Aurora", "#101a2c", "#0a1120", "rgba(20,30,50,.72)", "rgba(36,52,84,.85)", "#f4f7ff", "#a9b6d3", "#7fd8ff", "#06202c"),
    P("sunset", "Sunset", "#2a1420", "#1a0b14", "rgba(48,20,34,.72)", "rgba(74,32,52,.85)", "#fff4ee", "#e2b6a8", "#ffb35c", "#2a1300"),
    P("mono", "Mono", "#18191b", "#0f1011", "rgba(30,31,34,.72)", "rgba(48,50,54,.85)", "#f5f5f5", "#a8a8a8", "#ffffff", "#111111"),
  ] },
  { id: "classic", name: "Hearth Classic", blurb: "The old Hearth Game Mode home and look.", presets: [
    P("hearth", "Hearth", "#0c1016", "#121821", "#161d27", "#1d2531", "#e9eef4", "#8d99a8", "#3fd0b8", "#06231e"),
  ] },
];
export const ERA_IDS: EraId[] = ERAS.map((e) => e.id);
export const MAX_PRESETS = 3;   // main.py PRESETS
export function eraById(id: string): Era { return ERAS.find((e) => e.id === id) || ERAS[0]; }

// ---------------------------------------------------------------- settings

export interface Settings {
  era: EraId; presets: Record<string, number>;
  sounds: boolean; quickSwitch: boolean; loading: boolean; keyboard: boolean; pages: boolean;
}
/** Steam's look (off) and every extra off until the person turns it on. Never changed by the plugin itself. */
export const DEFAULTS: Settings = { era: "steam", presets: {}, sounds: false, quickSwitch: false, loading: false, keyboard: false, pages: false };
export const EXTRAS: (keyof Settings)[] = ["sounds", "quickSwitch", "loading", "keyboard", "pages"];

export function normalize(raw: unknown): Settings {
  const r = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const out: Settings = { ...DEFAULTS, presets: {} };
  if (typeof r.era === "string" && (ERA_IDS as string[]).includes(r.era)) out.era = r.era as EraId;
  const p = r.presets && typeof r.presets === "object" ? (r.presets as Record<string, unknown>) : {};
  for (const [k, v] of Object.entries(p)) {
    const e = ERAS.find((x) => x.id === k && x.id !== "steam");
    if (e && typeof v === "number" && Number.isInteger(v) && v >= 0 && v < MAX_PRESETS) out.presets[k] = v;
  }
  for (const k of EXTRAS) if (typeof r[k] === "boolean") (out as any)[k] = r[k];
  return out;
}

/** The preset picked for an era (its first when none, or when the saved one is beyond this era's presets). */
export function presetOf(s: Settings, era: EraId): Preset | null {
  const e = eraById(era);
  if (!e.presets.length) return null;
  const i = s.presets[era];
  return e.presets[typeof i === "number" && i >= 0 && i < e.presets.length ? i : 0];
}

// ---------------------------------------------------------------- what to do

export type State = "off" | "on" | "paused";
export const PAUSED_NOTE = "Hearth Eras paused: Steam changed its layout";

/** Nothing (Steam's look, or not Hearth OS), apply, or pause with the reason. */
export function decide(s: Settings, hearthOs: boolean, check: SteamCheck | null, failure?: string): { state: State; why: string } {
  if (!hearthOs || s.era === "steam") return { state: "off", why: "" };
  if (failure) return { state: "paused", why: failure };
  if (!check) return { state: "paused", why: "Steam's UI couldn't be checked" };
  if (check.missing.length) return { state: "paused", why: "missing in Steam: " + check.missing.join(", ") };
  return { state: "on", why: "" };
}

// ---------------------------------------------------------------- the skin

export interface Brand { base: string; eras: Record<string, string>; logo: string; wallpaper: string }

/** The preset's colours as CSS variables (--era-bg, --era-accent, ...). Only safe characters get through. */
export function presetCss(p: Preset | null): string {
  if (!p) return "";
  const safe = (v: string) => /^[#(),.%\w\s-]+$/.test(v);
  const vars = Object.entries(p.vars).filter(([k, v]) => /^[a-z0-9]+$/.test(k) && safe(v)).map(([k, v]) => `--era-${k}:${v}`);
  return `:root{${vars.join(";")}}`;
}

/** The whole skin for an era: Hearth Classic's own skin (its menu sections always left out: Eras never touches Steam's
 *  menu window), or the preset's variables, the shared base and the era's own rules, with the extras' sections. Lines
 *  naming a Steam class Steam doesn't have are left out (renderTheme). "" for Steam's look or a missing skin. */
export function buildSkin(b: Brand | null, s: Settings, resolve: Resolve): { css: string; dropped: number } {
  if (!b || s.era === "steam") return { css: "", dropped: 0 };
  const pics = { logo: b.logo || "", wallpaper: b.wallpaper || "" };
  const extras = { loading: s.loading, keyboard: s.keyboard, pages: s.pages };
  if (s.era === "classic") {
    const css = (b.eras && b.eras.classic) || "";
    if (!css) return { css: "", dropped: 0 };
    return renderTheme(css, pics, resolve, {});   // no sections: menu, top and rail stay out
  }
  const own = (b.eras && b.eras[s.era]) || "";
  if (!own || !b.base) return { css: "", dropped: 0 };
  const r = renderTheme(b.base + "\n" + own, pics, resolve, extras);
  return { css: `/* Hearth Eras: ${s.era} */\n${presetCss(presetOf(s, s.era))}\n${r.css}`, dropped: r.dropped };
}

// ---------------------------------------------------------------- quick switching (hold L1: D-pad left/right, L2/R2)

/** Steam's controller state bits (ulButtons in SteamClient.Input's controller state changes). */
export const BTN = { R2: 1 << 0, L2: 1 << 1, R1: 1 << 2, L1: 1 << 3, Y: 1 << 4, B: 1 << 5, X: 1 << 6, A: 1 << 7,
  UP: 1 << 8, RIGHT: 1 << 9, LEFT: 1 << 10, DOWN: 1 << 11 };
export type QuickAction = "era+" | "era-" | "preset+" | "preset-" | null;

/** What a change of the buttons asks for: only while L1 is held, only on a button just pressed (not held). */
export function quickAction(prev: number, now: number): QuickAction {
  if (!(now & BTN.L1)) return null;
  const pressed = now & ~prev;
  if (pressed & BTN.RIGHT) return "era+";
  if (pressed & BTN.LEFT) return "era-";
  if (pressed & BTN.R2) return "preset+";
  if (pressed & BTN.L2) return "preset-";
  return null;
}

/** The next or previous era (Steam's look is one of them, so it's always one press away), round the list. */
export function stepEra(cur: EraId, dir: 1 | -1): EraId {
  const i = Math.max(0, ERA_IDS.indexOf(cur));
  return ERA_IDS[(i + dir + ERA_IDS.length) % ERA_IDS.length];
}

/** The settings change a quick action makes (null: nothing to change, e.g. a preset step on Steam's look). */
export function applyQuick(s: Settings, a: QuickAction): Partial<Settings> | null {
  if (a === "era+" || a === "era-") return { era: stepEra(s.era, a === "era+" ? 1 : -1) };
  if (a === "preset+" || a === "preset-") {
    const e = eraById(s.era);
    if (e.presets.length < 2) return null;
    const cur = e.presets.indexOf(presetOf(s, s.era)!);
    const n = (cur + (a === "preset+" ? 1 : -1) + e.presets.length) % e.presets.length;
    return { presets: { [s.era]: n } };
  }
  return null;
}

/** What the toast says after a quick switch. */
export function quickLabel(s: Settings): string {
  const e = eraById(s.era);
  const p = presetOf(s, s.era);
  return e.id === "steam" ? "Steam's look" : p && e.presets.length > 1 ? `${e.name} · ${p.name}` : e.name;
}

// ---------------------------------------------------------------- sounds (generated here, Hearth's own)

/** One tone: a wave at `f` Hz (gliding to `to`), starting `at` s in, `d` s long, at volume `v`; or a burst of filtered
 *  noise (`noise`: the filter's frequency). */
export interface Tone { f: number; to?: number; d: number; v: number; at?: number; wave?: OscillatorType; noise?: number }

/** Each era's own little sounds, made from plain waves when they play; none is a recording of anything. */
export function soundRecipe(era: EraId, kind: SoundKind): Tone[] {
  const r: Record<string, Record<SoundKind, Tone[]>> = {
    channels: { move: [{ f: 0, noise: 3200, d: 0.018, v: 0.05 }, { f: 1560, d: 0.03, v: 0.02, wave: "sine" }],
                ok: [{ f: 990, d: 0.06, v: 0.035, wave: "sine" }, { f: 1320, d: 0.08, v: 0.03, at: 0.05, wave: "sine" }],
                back: [{ f: 760, to: 520, d: 0.08, v: 0.03, wave: "sine" }] },
    blades: { move: [{ f: 0, noise: 900, d: 0.09, v: 0.035 }, { f: 300, to: 520, d: 0.08, v: 0.015, wave: "triangle" }],
              ok: [{ f: 440, d: 0.07, v: 0.03, wave: "triangle" }, { f: 660, d: 0.1, v: 0.025, at: 0.04, wave: "triangle" }],
              back: [{ f: 520, to: 300, d: 0.1, v: 0.03, wave: "triangle" }] },
    tiles: { move: [{ f: 1200, d: 0.02, v: 0.025, wave: "square" }],
             ok: [{ f: 600, d: 0.05, v: 0.025, wave: "square" }, { f: 900, d: 0.06, v: 0.02, at: 0.035, wave: "square" }],
             back: [{ f: 500, to: 380, d: 0.06, v: 0.022, wave: "square" }] },
    crossbar: { move: [{ f: 2100, d: 0.05, v: 0.018, wave: "sine" }, { f: 3150, d: 0.04, v: 0.008, wave: "sine" }],
                ok: [{ f: 1046, d: 0.14, v: 0.025, wave: "sine" }, { f: 1568, d: 0.18, v: 0.018, at: 0.03, wave: "sine" }],
                back: [{ f: 1318, to: 880, d: 0.12, v: 0.02, wave: "sine" }] },
    pocket: { move: [{ f: 620, to: 880, d: 0.045, v: 0.03, wave: "sine" }],
              ok: [{ f: 523, to: 784, d: 0.07, v: 0.03, wave: "sine" }, { f: 1046, d: 0.06, v: 0.015, at: 0.06, wave: "sine" }],
              back: [{ f: 784, to: 523, d: 0.07, v: 0.028, wave: "sine" }] },
    classic: { move: [{ f: 700, d: 0.035, v: 0.035, wave: "sine" }], ok: [{ f: 880, d: 0.05, v: 0.035, wave: "sine" }],
               back: [{ f: 420, d: 0.05, v: 0.03, wave: "sine" }] },
  };
  return (r[era] && r[era][kind]) || [];
}

// ---------------------------------------------------------------- the homes' data

export interface Section { key: string; title: string; items: Card[] }

/** The home's rows as sections (Recent first), at most `max` games each. */
export function sectionsOf(view: HomeView | null, max = 30): Section[] {
  if (!view) return [];
  return view.shelves.filter((s) => s.items.length).map((s) => ({ key: s.key, title: s.key === "continue" ? "Recent" : s.title, items: s.items.slice(0, max) }));
}

/** Up to `n` different games for a grid: Recent, then Favorites, then each store row. */
export function gridGames(view: HomeView | null, n: number): Card[] {
  const out: Card[] = [];
  const seen = new Set<number>();
  for (const s of view ? view.shelves : []) for (const c of s.items) {
    if (out.length >= n) return out;
    if (!seen.has(c.appid)) { seen.add(c.appid); out.push(c); }
  }
  return out;
}

/** A game's second line: its store and play time. */
export function subtitle(c: Card | null): string {
  if (!c) return "";
  return [c.store, played(c.minutes)].filter(Boolean).join(" · ");
}

/** The places every era offers (Steam's own pages: model PLACES). */
export const ERA_PLACES: Place[] = PLACES;

export function clock(d: Date, h24 = true): string {
  const h = d.getHours(), m = d.getMinutes();
  const mm = (m < 10 ? "0" : "") + m;
  return h24 ? `${h < 10 ? "0" : ""}${h}:${mm}` : `${((h + 11) % 12) + 1}:${mm}`;
}
const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
export function dateLine(d: Date): string { return `${DAYS[d.getDay()]} ${d.getDate()}/${d.getMonth() + 1}`; }

/** What the panel's "Show what Hearth Eras found" says. */
export function diagnosticsText(d: { state: string; why: string; settings: Settings; keys: { resolved: string[]; missing: string[] }; windows: string[]; dropped: number; quick: string }): string {
  const s = d.settings;
  const p = presetOf(s, s.era);
  return [
    `Hearth Eras: ${d.state}${d.why ? " (" + d.why + ")" : ""}`,
    `Era: ${s.era}${p ? " (" + p.id + ")" : ""}; sounds ${s.sounds}, loading screen ${s.loading}, keyboard ${s.keyboard}, pages ${s.pages}, quick switching ${s.quickSwitch}`,
    `Steam windows: ${d.windows.join(", ") || "none found"}`,
    `Class names: ${d.keys.resolved.length} found, ${d.keys.missing.length} missing${d.keys.missing.length ? ": " + d.keys.missing.join(", ") : ""}; ${d.dropped} skin lines left out`,
    `Quick switching: ${d.quick}`,
  ].join("\n");
}
