// The Hearth takeover's pure parts (no Steam, no Decky, no imports): settings and their switches, the guard that
// turns a part off when Steam's screens changed under it, and the small helpers the pages use. Tested on their own
// by tests/model.test.mjs (Node, type stripping), so keep this file free of imports and of TypeScript-only syntax
// that emits code (no enums, namespaces or parameter properties).

/** Every switch the plugin keeps (main.py DEFAULT_SETTINGS has the same keys and defaults). */
export interface Settings {
  theme: boolean;        // Hearth's look over Steam
  homeRow: boolean;      // the older Hearth row over Steam's own Home (when the Hearth home is off)
  takeover: boolean;     // the master switch: Hearth takes over Steam's Game Mode
  tkHome: boolean;       // Hearth's home screen in place of Steam's Home
  tkMenu: boolean;       // Hearth's pages in Steam's main menu
  tkLibraryTabs: boolean;// Hearth's stores as tabs in Steam's Library
  tkGamePage: boolean;   // Hearth's facts and Play through Hearth on a game's page
  tkSounds: boolean;     // Hearth's sounds on Hearth's pages
  tkQam: boolean;        // Hearth's tabs in Steam's Quick Access (beside Steam's and Decky's)
  tkInGame: boolean;     // Hearth for the game you're playing (Steam's main menu and Quick Access)
  tkInput: boolean;      // Hearth Input layouts for the game in front (through Hearth's core service)
}

export const DEFAULTS: Settings = {
  theme: true, homeRow: true, takeover: true, tkHome: true, tkMenu: true, tkLibraryTabs: true, tkGamePage: true, tkSounds: false,
  tkQam: true, tkInGame: true, tkInput: true,
};

/** The takeover's parts: what each switch is called in Quick Access. */
export type Part = "tkHome" | "tkMenu" | "tkLibraryTabs" | "tkGamePage" | "tkSounds" | "tkQam" | "tkInGame" | "tkInput";
export const PARTS: [Part, string, string][] = [
  ["tkHome", "Hearth home screen", "Hearth's home in place of Steam's Home"],
  ["tkMenu", "Hearth in Steam's menu", "Hearth's places and power options in Steam's main menu"],
  ["tkLibraryTabs", "Hearth tabs in the Library", "Epic, GOG, Battle.net, Xbox… as tabs in Steam's Library"],
  ["tkGamePage", "Hearth on game pages", "ProtonDB and achievements from Hearth, Play through Hearth"],
  ["tkQam", "Hearth tabs in Quick Access", "Hearth, Hearth Performance and Hearth Controller beside Steam's tabs"],
  ["tkInGame", "Hearth for the game you're playing", "Its Hearth Input layout, mods and Proton in Steam's menu and Quick Access"],
  ["tkInput", "Hearth Input layouts", "A game's Hearth controller layout works while it's in front"],
  ["tkSounds", "Hearth sounds", "Hearth's sounds on Hearth's pages (Steam's own stay)"],
];

/** Settings from the backend, with anything missing or odd set to its default. */
export function normalize(raw: unknown): Settings {
  const out: Settings = { ...DEFAULTS };
  if (raw && typeof raw === "object") {
    for (const k of Object.keys(DEFAULTS) as (keyof Settings)[]) {
      const v = (raw as Record<string, unknown>)[k];
      if (typeof v === "boolean") out[k] = v;
    }
  }
  return out;
}

/** Whether a part of the takeover is on: its own switch, the master switch, and not turned off by the guard. */
export function partOn(s: Settings, part: Part, broken?: Set<string>): boolean {
  return s.takeover && s[part] && !(broken && broken.has(part));
}

/** The older Hearth row on Steam's Home: only when Hearth's home screen isn't taking Home over. */
export function oldRowOn(s: Settings, broken?: Set<string>): boolean {
  return s.homeRow && !partOn(s, "tkHome", broken);
}

export interface Guard {
  /** Runs `fn` for `part`; a throw turns the part off (until the plugin restarts), is logged once, and `fallback` is returned. */
  run<T>(part: string, fn: () => T, fallback: T): T;
  fail(part: string, e: unknown): void;
  broken: Set<string>;
}

/** The guard every patch goes through: Steam changing its screens turns that part off and logs why, never Steam's UI. */
export function makeGuard(log: (part: string, message: string) => void, onChange?: () => void): Guard {
  const broken = new Set<string>();
  const fail = (part: string, e: unknown) => {
    const first = !broken.has(part);
    broken.add(part);
    if (first) {
      let msg = "";
      try { msg = e instanceof Error ? `${e.name}: ${e.message}` : String(e); } catch { msg = "unknown error"; }
      try { log(part, msg.slice(0, 500)); } catch { /* logging must never throw */ }
      try { onChange && onChange(); } catch { /* nor this */ }
    }
  };
  return {
    broken, fail,
    run<T>(part: string, fn: () => T, fallback: T): T {
      if (broken.has(part)) return fallback;
      try { return fn(); } catch (e) { fail(part, e); return fallback; }
    },
  };
}

// ---- what the pages show

export interface Game {
  id: string; title: string; source: string; store: string; tab: string; installed: boolean; state: string;
  cloud?: boolean; fav?: boolean; minutes?: number; lastPlayed?: number; added?: number; sizeBytes?: number | null;
  cover?: string | null; wide?: string | null; hero?: string | null; logo?: string | null; icon?: string | null;
}
export interface Home { hero: Game | null; continue: Game[]; favorites: Game[]; rows: { id: string; title: string; total: number; items: Game[] }[]; added: Game[]; count: number }
export interface Shelf { key: string; title: string; items: Game[]; wide?: boolean; total?: number }

/** The home's shelves in Hearth's order: Continue playing (wide), Favorites, a row per store, Recently added. */
export function homeShelves(h: Home | null | undefined): Shelf[] {
  if (!h) return [];
  const out: Shelf[] = [];
  if (h.continue && h.continue.length) out.push({ key: "continue", title: "Continue playing", items: h.continue, wide: true });
  if (h.favorites && h.favorites.length) out.push({ key: "fav", title: "Favorites", items: h.favorites });
  for (const r of h.rows || []) if (r.items && r.items.length) out.push({ key: "row-" + r.id, title: r.title, items: r.items, total: r.total });
  if (h.added && h.added.length > 2) out.push({ key: "added", title: "Recently added", items: h.added });
  return out;
}

/** Hearth's pages and where they open: a route of the plugin's, or one of Hearth's apps as a Steam shortcut. */
export interface Dest { id: string; title: string; route?: string; app?: string }
export const DESTS: Dest[] = [
  { id: "store", title: "Hearth Store", route: "/hearth/store" },
  { id: "library", title: "Hearth Library", route: "/hearth/library" },
  { id: "downloads", title: "Downloads", route: "/hearth/downloads" },
  { id: "theater", title: "Theater", app: "hearth-app:theater" },
  { id: "mods", title: "Hearth Mods", route: "/hearth/mods" },
  { id: "storage", title: "Storage", route: "/hearth/storage" },
  { id: "recomps", title: "Recomps", route: "/hearth/recomps" },
];

/** Hearth's Store's tabs, in its order (ui/index.html SHOP_STORES), plus free games. */
export const STORE_TABS: [string, string][] = [
  ["steam", "Steam"], ["epic", "Epic Games"], ["gog", "GOG"], ["free", "Free games"], ["xbox", "Xbox"], ["battlenet", "Battle.net"], ["recomps", "Recomps"],
];

/** The tabs Hearth adds to Steam's own Library: every store but Steam's (Steam lists those itself). */
export function extraLibraryTabs(tabs: { id: string; title: string; count: number }[] | null | undefined): { id: string; title: string; count: number }[] {
  return (tabs || []).filter((t) => t.id !== "steam" && t.count > 0);
}

/** The Hearth game a Steam app is (a shortcut the plugin made, or a Steam game), else null. */
export function hearthIdForApp(map: Record<string, number>, appId: number | string): string | null {
  const n = Number(appId);
  if (!Number.isFinite(n) || n <= 0) return null;
  for (const [id, a] of Object.entries(map || {})) if (a === n) return id;
  // Steam's own games: their Steam id (a shortcut's id is far above any game's).
  return n < 0x80000000 ? `steam:${n}` : null;
}

export function bytes(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n) || n < 0) return "";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0, v = n;
  while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; }
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${u[i]}`;
}

export function speed(bps: number | null | undefined): string {
  return bps && bps > 0 ? `${bytes(bps)}/s` : "";
}

export function percent(p: number | null | undefined): number {
  return p == null || !Number.isFinite(p) ? 0 : Math.max(0, Math.min(100, Math.round(p * 100)));
}

/** "Played 3 hours", "Played 12 minutes", or "" (never played). */
export function played(minutes: number | null | undefined): string {
  if (!minutes || minutes <= 0) return "";
  if (minutes < 60) return `Played ${minutes} minute${minutes === 1 ? "" : "s"}`;
  const h = Math.round(minutes / 60);
  return `Played ${h} hour${h === 1 ? "" : "s"}`;
}

/** A download's line: "Downloading · 42% · 12.5 MB/s", or Steam's own stage. */
export function downloadLine(d: { state?: string; stage?: string | null; progress?: number | null; bytesPerSec?: number | null }): string {
  const state = (d.state || "").replace(/^./, (c) => c.toUpperCase());
  const parts = [d.stage || state];
  if (d.progress != null) parts.push(`${percent(d.progress)}%`);
  const s = speed(d.bytesPerSec);
  if (s) parts.push(s);
  return parts.filter(Boolean).join(" · ");
}

/** A drive's line: "120 GB free of 512 GB". */
export function driveLine(d: { free?: number; size?: number }): string {
  return d.size ? `${bytes(d.free || 0)} free of ${bytes(d.size)}` : "";
}

/** Hearth's sound for a move (ui/index.html blip, its Soft pack): frequency, length, volume. */
export function blipFor(kind: "move" | "ok" | "back"): [number, number, number] {
  return kind === "ok" ? [880, 0.05, 0.035] : kind === "back" ? [420, 0.05, 0.03] : [700, 0.035, 0.035];
}

/** An id is safe to hand to the backend (the same rule as hearth-core desktop_games::valid_id). */
export function validId(id: unknown): id is string {
  return typeof id === "string" && id.length > 0 && id.length <= 200 && id.includes(":") && !/["\\\u0000-\u001f\u007f]/.test(id);
}

// ---- game pictures (the shelves, the old Hearth row, the hero): which pictures to try, in order

/** A Steam game's app id from Hearth's id ("steam:620" → "620"), else "". */
export function steamAppId(id: string | null | undefined): string {
  const m = /^steam:(\d+)$/.exec(id || "");
  return m ? m[1] : "";
}

export type Shape = "portrait" | "wide" | "hero";

/** The pictures to try for a card, best first: a portrait cover for the shelves (then Steam's own library picture for
 *  Steam's games, then wide art, cropped), wide art for wide cards and the hero. steam-lib:<appid>:<kind> is Steam's
 *  picture in its library cache on this device (main.py steam_art_file). No duplicates, no empty ones. */
export function artCandidates(g: { id?: string; cover?: string | null; wide?: string | null; hero?: string | null } | null | undefined, shape: Shape): string[] {
  if (!g) return [];
  const app = steamAppId(g.id);
  const lib = (k: string) => (app ? `steam-lib:${app}:${k}` : "");
  const order = shape === "portrait" ? [g.cover, lib("portrait"), g.wide, g.hero, lib("wide")]
    : shape === "wide" ? [g.wide, g.hero, lib("wide"), lib("hero"), g.cover, lib("portrait")]
    : [g.hero, g.wide, lib("hero"), lib("wide")];
  const out: string[] = [];
  for (const r of order) if (typeof r === "string" && r.trim() && !out.includes(r.trim())) out.push(r.trim());
  return out;
}

/** One way of loading a picture: straight into Steam's window (web and data: addresses), or read by the plugin's
 *  backend (files on this device, Hearth's nimbus-art:// addresses, Steam's library cache; and a web picture that
 *  didn't load straight, which the backend takes from Hearth's art cache or downloads). */
export interface Attempt { ref: string; via: "direct" | "backend" }
export function artAttempts(refs: string[]): Attempt[] {
  const out: Attempt[] = [];
  for (const ref of refs) {
    if (/^data:image\//i.test(ref)) out.push({ ref, via: "direct" });
    else if (/^https:\/\//i.test(ref)) { out.push({ ref, via: "direct" }); out.push({ ref, via: "backend" }); }
    else out.push({ ref, via: "backend" });
  }
  return out;
}

/** What kind of reference a picture is, for the plugin's log (never the address or path itself). */
export function refKind(ref: string | null | undefined): string {
  const r = ref || "";
  if (!r) return "none";
  if (/^https:\/\//i.test(r)) return "web";
  if (/^http:\/\//i.test(r)) return "plain-http";
  if (/^data:/i.test(r)) return "data";
  if (r.startsWith("nimbus-art://cache/")) return "art-cache";
  if (r.startsWith("nimbus-art://file/")) return "nimbus-file";
  if (r.startsWith("steam-lib:")) return "steam-library";
  if (r.startsWith("file://")) return "file-url";
  if (r.startsWith("~")) return "home-path";
  if (r.startsWith("/")) return "path";
  return "other";
}

// ---- Quick Access › Hearth › Update Hearth OS (main.py update_check / update_start / update_status)

export interface UpdateStatus { state: string; pct?: number | null; message?: string; version?: string }
/** What the update row shows: its line, its button (or "" for none while it runs) and whether to keep asking. */
export function updateView(u: UpdateStatus | null | undefined): { line: string; button: string; action: "check" | "start" | ""; poll: boolean; pct: number | null } {
  const s = u?.state || "idle";
  const pct = typeof u?.pct === "number" && Number.isFinite(u.pct) ? Math.max(0, Math.min(100, Math.round(u.pct))) : null;
  switch (s) {
    case "checking": return { line: "Looking for an update…", button: "", action: "", poll: true, pct: null };
    case "available": return { line: u?.message || "An update is available", button: "Update Hearth OS", action: "start", poll: false, pct: null };
    case "running": return { line: `Updating Hearth OS… ${pct ?? 0}%`, button: "", action: "", poll: true, pct: pct ?? 0 };
    case "ready": return { line: u?.message || "Ready: restart to finish updating", button: "Check again", action: "check", poll: false, pct: 100 };
    case "none": return { line: "Hearth OS is up to date", button: "Check for Hearth updates", action: "check", poll: false, pct: null };
    case "error": return { line: u?.message || "The update failed", button: "Try again", action: "check", poll: false, pct: null };
    default: return { line: "", button: "Check for Hearth updates", action: "check", poll: false, pct: null };
  }
}

// ---- Hearth in Steam's main menu (the Xbox / Steam button)

/** Hearth's places in Steam's main menu, in Hearth's order. Hearth's home is Steam's Home when the takeover has it,
 *  else a page of its own. */
export function menuPlaces(s: Settings, broken?: Set<string>): Dest[] {
  const home: Dest = { id: "home", title: "Hearth Home", route: partOn(s, "tkHome", broken) ? "/library/home" : "/hearth/home" };
  const by = (id: string) => DESTS.find((d) => d.id === id)!;
  return [home, by("store"), by("library"), by("theater"), by("mods"), by("storage"), by("downloads")];
}

/** Hearth's power options beside Steam's: back to Hearth's Game Mode, or Hearth's Desktop Mode. */
export const MENU_POWER: [string, string, string][] = [
  ["hearth-game", "Switch to Hearth Game Mode", "Steam closes and Hearth's own Game Mode comes back"],
  ["hearth-desktop", "Desktop Mode", "Hearth's Desktop Mode (Steam closes)"],
];

// ---- The game Steam is running (Steam's own game, or one of Hearth's played through a shortcut)

export interface Running { appid: number; id: string | null; title: string; hearth: boolean }

/** The running app Steam reports (Router.MainRunningApp: {appid, display_name}) as Hearth sees it, or null. */
export function runningGame(app: { appid?: number | string; display_name?: string } | null | undefined, map: Record<string, number>): Running | null {
  if (!app) return null;
  const appid = Number(app.appid);
  if (!Number.isFinite(appid) || appid <= 0) return null;
  const id = hearthIdForApp(map, appid);
  return { appid, id, title: String(app.display_name || ""), hearth: !!id && !id.startsWith("steam:") };
}

/** What Hearth's core is told every 2 s (core_service.rs `front`): no game when Hearth Input is switched off, so the
 *  mapper never holds the controller then. */
export function frontMessage(run: Running | null, s: Settings, broken: Set<string> | undefined, overlay: boolean): { appid: number; game: string; overlay: boolean } {
  if (!run || !run.id || !partOn(s, "tkInput", broken)) return { appid: run?.appid || 0, game: "", overlay };
  return { appid: run.appid, game: run.id, overlay };
}

// ---- Hearth's tabs in Steam's Quick Access (added through Decky's own tab list, the way Decky adds its tab)

/** Ids far from Steam's own tabs (small numbers) and Decky's (999). */
export const QAM_TABS: { id: number; key: "hearth" | "perf" | "controller" | "players"; title: string }[] = [
  { id: 4801, key: "hearth", title: "Hearth" },
  { id: 4802, key: "perf", title: "Hearth Performance" },
  { id: 4803, key: "controller", title: "Hearth Controller" },
  { id: 4804, key: "players", title: "Hearth Players" },
];

/** Which of Hearth's Quick Access tabs to add or take away so the list matches the switches. */
export function qamPlan(s: Settings, broken: Set<string> | undefined, have: number[]): { add: number[]; remove: number[] } {
  const want = partOn(s, "tkQam", broken) ? QAM_TABS.map((t) => t.id) : [];
  return { add: want.filter((i) => !have.includes(i)), remove: have.filter((i) => !want.includes(i)) };
}

// ---- The in-game section (core_service.rs `game`)

export interface Choice { id: string; label: string }
export interface GameView {
  id: string; title: string; source: string; store: string; kind: string;
  input: { layout: string; name?: string | null; choices: Choice[] };
  proton: string; protonChoices: { id: string; label: string; sub?: string }[]; mods: boolean;
}

/** The layout line: "Shooter / action", "Your own layout", or "Off". */
export function layoutLabel(v: GameView | null | undefined): string {
  if (!v) return "";
  const l = v.input?.layout || "off";
  if (l === "off") return "Off";
  return v.input.choices?.find((c) => c.id === l)?.label || v.input.name || l;
}

/** What the Controller tab says about Hearth Input right now (core_service.rs `front`'s answer). */
export function inputLine(f: { game?: string | null; front?: boolean; overlay?: boolean; layout?: string | null; active?: boolean } | null | undefined, on: boolean): string {
  if (!on) return "Hearth Input is off in Steam Game Mode";
  if (!f || !f.game) return "No game of yours is running";
  if (!f.layout) return "No Hearth layout: the game gets the controller";
  if (f.active) return `${f.layout}: on`;
  return f.overlay ? `${f.layout}: on in the game (paused while Quick Access is open)` : `${f.layout}: paused while Steam is in front`;
}

/** The Proton line for a game Hearth starts (its choice, or the default). */
export function protonLabel(v: GameView | null | undefined): string {
  if (!v || v.kind !== "proton") return "";
  return v.protonChoices?.find((c) => c.id === (v.proton || ""))?.label || v.proton || "Default";
}

/** The mods line: "12 mods, 9 on", "No mods yet". */
export function modsLine(m: { count?: number; enabled?: number; beacon?: boolean } | null | undefined): string {
  if (!m) return "";
  if (m.beacon) return "Beacon manages this game's mods";
  if (!m.count) return "No mods yet";
  return `${m.count} mod${m.count === 1 ? "" : "s"}, ${m.enabled || 0} on`;
}

// ---- Quick Access › Hearth Performance: Hearth's power settings, without fighting Steam's own Performance tab

/** The power limit as a read-only line ("Power limit: 15 W (Hearth's)"); Steam's own Performance tab sets the frame
 *  limit and TDP in Steam Game Mode, so Hearth shows its limit and leaves it there. */
export function tdpLine(p: { tdp?: { supported?: boolean; watts?: number; min?: number; max?: number } } | null | undefined): string {
  const t = p?.tdp;
  if (!t || !t.supported || typeof t.watts !== "number") return "";
  return `Power limit: ${Math.round(t.watts)} W (Hearth's, ${t.min ?? "?"}–${t.max ?? "?"} W)`;
}

// ---- Couch Co-op in Steam Game Mode (hearth-coop `players`, core_service.rs `coop`)

export interface CoopPad { id: string; name: string; label: string; kind: string; player: number | null; playerName: string; rumble: boolean | null }
export interface CoopPlayers { pads: CoopPad[]; max: number; running: boolean; game?: { id?: string; title?: string } | null; lastError?: string | null; lastErrorAt?: number | null }
export const COOP_LAYOUTS: [string, string][] = [["auto", "Auto"], ["vertical", "Side by side"], ["horizontal", "Top and bottom"], ["grid", "2 × 2"]];

/** The order with the controller at `i` moved one place up (-1) or down (+1); unchanged at either end. */
export function movePad(ids: string[], i: number, dir: -1 | 1): string[] {
  const out = ids.slice(), j = i + dir;
  if (i < 0 || i >= out.length || j < 0 || j >= out.length) return out;
  [out[i], out[j]] = [out[j], out[i]];
  return out;
}

/** The Players tab's first line: how many play split-screen with the controllers connected now. */
export function coopLine(p: CoopPlayers | null): string {
  if (!p) return "Looking for controllers…";
  const n = p.pads?.length || 0;
  if (!n) return "No controllers connected.";
  if (n === 1) return "One controller connected: connect another to play split-screen.";
  const playing = Math.min(n, p.max || 4);
  return `${playing} players. In split-screen games each player gets the controller beside their number.` + (n > playing ? ` ${n - playing} more wait${n - playing === 1 ? "s" : ""}.` : "");
}

/** A failed split-screen start worth showing: the last hour's, while nothing runs. */
export function coopProblem(p: CoopPlayers | null, now: number): string {
  if (!p || p.running || !p.lastError) return "";
  return !p.lastErrorAt || now - p.lastErrorAt < 3600e3 ? p.lastError : "";
}
