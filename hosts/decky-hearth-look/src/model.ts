// Hearth Classic's pure parts (no Steam, no Decky): the settings, the check for Steam's modules, the skin's template,
// the set of patches, and the home screen's data. tests/model.test.mjs runs these in Node.

// ---------------------------------------------------------------- settings

export type Look = "steam" | "classic";
export interface Settings { look: Look; home: boolean; sounds: boolean; wallpaper: boolean; menu: boolean; menuTop: boolean }
/** Off until the person turns it on (Quick Access › Hearth Classic). Never changed by the plugin itself. The parts
 *  (home, wallpaper, the bottom bar menu, the top bar in a game) are on once Hearth Classic is picked; sounds are off. */
export const DEFAULTS: Settings = { look: "steam", home: true, sounds: false, wallpaper: true, menu: true, menuTop: true };

export function normalize(raw: unknown): Settings {
  const r = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const b = (k: keyof Settings) => (typeof r[k] === "boolean" ? (r[k] as boolean) : (DEFAULTS[k] as boolean));
  return { look: r.look === "classic" ? "classic" : "steam", home: b("home"), sounds: b("sounds"), wallpaper: b("wallpaper"), menu: b("menu"), menuTop: b("menuTop") };
}

// ---------------------------------------------------------------- Steam's modules

/** Steam's class names the look can't do without: the Library and home's tabbed page, the dialogs and Quick Access.
 *  If any is missing, Steam changed its layout and Hearth Classic pauses (Steam's normal look) instead of half-applying. */
export const REQUIRED: [string, string][] = [
  ["gamepadTabbedPageClasses", "TabHeaderRowWrapper"],
  ["gamepadDialogClasses", "Field"],
  ["quickAccessMenuClasses", "Title"],
];

export type Resolve = (map: string, key: string) => string | undefined;

export interface SteamCheck { missing: string[]; router: boolean }
/** What of Steam's UI the look needs and can't find (`router`: Decky's route patching, for the home screen). */
export function checkSteam(resolve: Resolve, router: boolean): SteamCheck {
  const missing: string[] = [];
  for (const [m, k] of REQUIRED) {
    let v: string | undefined;
    try { v = resolve(m, k); } catch { v = undefined; }
    if (!v) missing.push(m + "." + k);
  }
  if (!router) missing.push("routerHook");
  return { missing, router };
}

export type State = "off" | "on" | "paused";
export const PAUSED_NOTE = "Hearth Classic paused: Steam changed its layout";

/** What to do: nothing (the Steam look chosen, or not Hearth OS), apply, or pause with the reason. */
export function decide(s: Settings, hearthOs: boolean, check: SteamCheck | null, failure?: string): { state: State; why: string } {
  if (!hearthOs || s.look !== "classic") return { state: "off", why: "" };
  if (failure) return { state: "paused", why: failure };
  if (!check) return { state: "paused", why: "Steam's UI couldn't be checked" };
  if (check.missing.length) return { state: "paused", why: "missing in Steam: " + check.missing.join(", ") };
  return { state: "on", why: "" };
}

// ---------------------------------------------------------------- the skin

const TEMPLATE = /\{\{([A-Za-z0-9_]+):([A-Za-z0-9_]+)\}\}/g;
/** A part of the skin that's all or nothing (the bottom bar menu, the top bar in a game): between these lines. */
const SECTION = /^\/\* @section ([a-z]+) \*\/\s*$/;
const SECTION_END = /^\/\* @end \*\/\s*$/;

export const MENU_PAUSED = "Hearth bottom bar menu paused";

/** Which all-or-nothing parts of the skin go in. Whether the menu really takes the bar is decided on Steam's real menu
 *  (theme.ts startMenuBar, judgeStructure and judgeBar below). */
export function menuPlan(s: Settings): { sections: Record<string, boolean> } {
  return { sections: { menu: s.menu, top: s.menu && s.menuTop } };
}

/** Steam's menu as CSS Loader themes see it: #MainNavMenuContainer > .Panel > * > .Panel.Focusable entries. */
export function judgeStructure(panel: boolean, labels: string[]): { ok: boolean; why: string } {
  if (!panel) return { ok: false, why: "Steam's menu has no .Panel inside #MainNavMenuContainer any more (Steam changed its menu)" };
  if (labels.length < 3) return { ok: false, why: `only ${labels.length} menu entries found (Steam changed its menu)` };
  const empty = labels.filter((l) => !l).length;
  if (empty * 2 > labels.length) return { ok: false, why: `${empty} of ${labels.length} menu entries have no label (Steam changed its menu)` };
  return { ok: true, why: "" };
}

/** Whether the bar really sits as a bar once the menu is on screen: across most of the window, along the bottom (or
 *  the top in a game), the entries in one row, and the first entry reachable (nothing covering it). */
export function judgeBar(win: { w: number; h: number }, box: { left: number; right: number; top: number; bottom: number },
                         entries: { left: number; top: number; width: number; height: number }[], hit: boolean, top: boolean): { ok: boolean; why: string } {
  if (!(win.w > 0 && win.h > 0)) return { ok: false, why: "no window size" };
  if (win.w < 640) return { ok: false, why: `Steam's menu window is only ${Math.round(win.w)} px wide` };
  if (box.right - box.left < Math.min(win.w * 0.4, 600)) return { ok: false, why: "the bar is too narrow" };
  if (top ? box.top > 160 : box.bottom < win.h - 160) return { ok: false, why: top ? "the bar isn't at the top" : "the bar isn't at the bottom" };
  if (!entries.length || entries.some((e) => !(e.width > 0 && e.height > 0))) return { ok: false, why: "entries without a size" };
  const tops = entries.map((e) => e.top);
  if (Math.max(...tops) - Math.min(...tops) > 24) return { ok: false, why: "the entries aren't in one row" };
  if (entries.some((e) => e.left < -1 || e.left + e.width > win.w + 1)) return { ok: false, why: "entries off the screen" };
  if (!hit) return { ok: false, why: "something covers the entries" };
  return { ok: true, why: "" };
}

/** The panel's note for the menu (empty when all is well or it's off). */
export function menuNote(d: { layout: string; why: string; found: boolean }, s: Settings): string {
  if (!s.menu) return "";
  if (d.layout === "bar" || d.layout === "top") return "";
  if (d.layout === "pending") return "";
  if (d.layout === "vertical") return `${MENU_PAUSED}: ${d.why}`;
  return d.found ? `${MENU_PAUSED}: ${d.why}; Steam's own menu is shown` : "";
}

/** Every Steam class name the skin asks for, resolved or missing (for the diagnostics). */
export function classKeys(css: string, resolve: Resolve): { resolved: string[]; missing: string[] } {
  const seen = new Set<string>();
  const resolved: string[] = [], missing: string[] = [];
  for (const m of (css || "").matchAll(/\{\{([A-Za-z0-9_]+):([A-Za-z0-9_]+)\}\}/g)) {
    const id = m[1] + "." + m[2];
    if (seen.has(id)) continue;
    seen.add(id);
    let v: string | undefined;
    try { v = resolve(m[1], m[2]); } catch { v = undefined; }
    (v ? resolved : missing).push(id);
  }
  return { resolved, missing };
}

/** The diagnostics as plain text (the panel shows it; "Write a Hearth Classic log" saves it). */
export function diagnosticsText(d: { state: string; why: string; settings: Settings; keys: { resolved: string[]; missing: string[] };
  menu: { windows: string[]; found: boolean; entries: number; labels: string[]; layout: string; why: string; game: boolean }; windows: string[]; game: boolean | null }): string {
  const s = d.settings;
  return [
    `Hearth Classic: ${d.state}${d.why ? " (" + d.why + ")" : ""}`,
    `Settings: look ${s.look}, home ${s.home}, wallpaper ${s.wallpaper}, bottom bar ${s.menu}, top bar in a game ${s.menuTop}, sounds ${s.sounds}`,
    `Steam windows: ${d.windows.join(", ") || "none found"}`,
    `Class names: ${d.keys.resolved.length} found, ${d.keys.missing.length} missing${d.keys.missing.length ? ": " + d.keys.missing.join(", ") : ""}`,
    `Menu: ${d.menu.found ? "found" : "not found"} (#MainNavMenuContainer), ${d.menu.entries} entries${d.menu.labels.length ? " [" + d.menu.labels.join(", ") + "]" : ""}`,
    `Menu bar: ${d.menu.layout}${d.menu.why ? " — " + d.menu.why : ""}`,
    `Game running: ${d.game === null ? "can't tell (Router.MainRunningApp unreadable)" : d.game ? "yes" : "no"}`,
  ].join("\n");
}

/** Hearth's icon for one of Steam's main menu entries, from its (English) label; "" for one Hearth has no icon for
 *  (it keeps its label). Only decoration: the entries do what Steam's do. */
export function menuIcon(label: string): string {
  const l = (label || "").trim().toLowerCase();
  const table: [RegExp, string][] = [
    [/^home\b/, "home"], [/^library\b/, "lib"], [/^store\b/, "shop"], [/^media\b/, "media"], [/^downloads?\b/, "dl"],
    [/^friends\b/, "friends"], [/^settings\b/, "gear"], [/^power\b/, "power"], [/^search\b/, "search"],
    [/^(resume|return to game|back to game)/, "pad"], [/^(exit game|close game|quit)/, "close"], [/^(screenshot|take screenshot)/, "shot"],
  ];
  for (const [re, ic] of table) if (re.test(l)) return ic;
  return "";
}

/** The skin with Steam's class names and Hearth's pictures filled in; a line with a name or picture that's missing is
 *  left out (a Steam update makes the look plainer, never broken). Also returns how many lines were left out. */
export function renderTheme(css: string, pics: { logo: string; wallpaper: string }, resolve: Resolve,
                            sections: Record<string, boolean> = {}): { css: string; dropped: number } {
  const cache = new Map<string, string | undefined>();
  const out: string[] = [];
  let dropped = 0;
  let skip = false;   // inside a section that's switched off (or whose names Steam hasn't got): left out whole
  for (const line of (css || "").split("\n")) {
    const sec = SECTION.exec(line);
    if (sec) { skip = sections[sec[1]] !== true; continue; }
    if (SECTION_END.test(line)) { skip = false; continue; }
    if (skip) continue;
    if ((line.includes("%WALLPAPER%") && !pics.wallpaper) || (line.includes("%LOGO%") && !pics.logo)) { dropped++; continue; }
    let missing = false;
    const done = line.replace(TEMPLATE, (_all, a: string, k: string) => {
      const id = a + ":" + k;
      if (!cache.has(id)) { let v: string | undefined; try { v = resolve(a, k); } catch { v = undefined; } cache.set(id, v); }
      const v = cache.get(id);
      if (!v) { missing = true; return ""; }
      // Steam's class map values can hold several names ("a b"): the first is the component's own.
      return v.split(/\s+/)[0];
    });
    if (missing) { dropped++; continue; }
    out.push(done.replace(/%WALLPAPER%/g, pics.wallpaper).replace(/%LOGO%/g, pics.logo));
  }
  return { css: out.join("\n"), dropped };
}

// ---------------------------------------------------------------- patches

/** Every change made to Steam, with its undo. removeAll undoes them all (newest first), even if one undo throws. */
export interface PatchSet { add(name: string, undo: () => void): void; removeAll(): string[]; names(): string[] }
export function makePatchSet(log?: (msg: string) => void): PatchSet {
  let list: { name: string; undo: () => void }[] = [];
  return {
    add(name, undo) { list.push({ name, undo }); },
    removeAll() {
      const done: string[] = [];
      const all = list.reverse();
      list = [];
      for (const p of all) {
        try { p.undo(); done.push(p.name); } catch (e) { try { log && log(`couldn't undo ${p.name}: ${String(e)}`); } catch { /* never throw */ } }
      }
      return done;
    },
    names() { return list.map((p) => p.name); },
  };
}

// ---------------------------------------------------------------- sounds (ui/index.html blip, the Soft pack)

export function blipFor(kind: "move" | "ok" | "back"): [number, number, number] {
  return kind === "ok" ? [880, 0.05, 0.035] : kind === "back" ? [420, 0.05, 0.03] : [700, 0.035, 0.035];
}

// ---------------------------------------------------------------- the home screen's data

/** A game on Hearth Classic's home: always one Steam can open (a Steam game or a shortcut Hearth Sync made). */
export interface Card { appid: number; title: string; store: string; minutes?: number; cover?: string; wide?: string; hero?: string; logo?: string }
export interface Shelf { key: string; title: string; items: Card[]; wide?: boolean }
export interface HomeView { from: "hearth" | "steam"; hero: Card | null; shelves: Shelf[]; count: number }

/** Hearth Sync's collection names (hosts/decky-hearth-sync/main.py STORES), in Hearth's order: its rows on the home. */
export const SYNC_COLLECTIONS: [string, string][] = [
  ["epic", "Epic"], ["gog", "GOG"], ["battlenet", "Battle.net"], ["xbox", "Xbox"], ["beacon", "Beacon"], ["recomps", "Recomps"],
  ["emu", "Emulators"], ["manual", "Added by hand"], ["ea", "EA"], ["ubisoft", "Ubisoft"], ["amazon", "Amazon"], ["itch", "itch.io"],
  ["heroic", "Heroic"], ["lutris", "Lutris"], ["mods", "Modded games"], ["other", "Other games"],
];

/** How a place opens in Steam, tried in order until one works:
 *  - "navigator": Steam's own window navigator (SteamUIStore's focused window `.Navigator`, the calls Steam's main
 *    menu makes; names from @decky/ui's SteamWindowNavigator types): it throws or is missing when Steam changed;
 *  - "window": a call on that window itself (NavigateToSteamWeb);
 *  - "route": Navigation.Navigate to one of Steam's routes known to show a page (always last: never an empty page). */
export type NavStep = { kind: "navigator"; call: string; args?: unknown[] } | { kind: "window"; call: string; args?: unknown[] } | { kind: "route"; path: string };
export interface Place { id: string; title: string; steps: NavStep[] }
export const STORE_URL = "https://store.steampowered.com/";
const MEDIA_STATE = { state: { filter: { listSource: { type: "recents" }, mediaType: "all" } } };
/** The navigator calls Steam has (decky-frontend-lib globals/stores/steamuistore/window.ts SteamWindowNavigator) and
 *  the routes known to show a page; tests/model.test.mjs checks every place against these. */
export const KNOWN_NAVIGATOR = ["Downloads", "Chat", "Media.Grid", "Media.List", "Settings", "SteamWeb", "LibraryTab", "Home"];
export const KNOWN_WINDOW = ["NavigateToSteamWeb"];
export const KNOWN_ROUTES = ["/library", "/library/home", "/settings", "/steamweb"];
export const PLACES: Place[] = [
  { id: "library", title: "Library", steps: [{ kind: "route", path: "/library" }] },
  { id: "store", title: "Store", steps: [{ kind: "navigator", call: "SteamWeb", args: [STORE_URL] }, { kind: "window", call: "NavigateToSteamWeb", args: [STORE_URL] }, { kind: "route", path: "/steamweb" }] },
  { id: "downloads", title: "Downloads", steps: [{ kind: "navigator", call: "Downloads" }, { kind: "navigator", call: "Settings", args: ["Downloads"] }, { kind: "route", path: "/library" }] },
  { id: "media", title: "Media", steps: [{ kind: "navigator", call: "Media.Grid", args: [MEDIA_STATE] }, { kind: "navigator", call: "Media.List", args: [MEDIA_STATE] }, { kind: "route", path: "/library" }] },
  { id: "friends", title: "Friends", steps: [{ kind: "navigator", call: "Chat" }, { kind: "navigator", call: "Settings", args: ["Friends"] }, { kind: "route", path: "/library" }] },
  { id: "settings", title: "Settings", steps: [{ kind: "navigator", call: "Settings" }, { kind: "route", path: "/settings" }] },
];

/** Opens a place: the first step that's there and doesn't throw. Returns what was used ("navigator:Downloads",
 *  "route:/library"). `win` is Steam's focused window instance (or null), `navigate` Navigation.Navigate. */
export function runPlace(place: Place, win: any, navigate: (route: string) => void): string {
  const nav = win ? (win.Navigator ?? win.m_Navigator ?? null) : null;
  for (const st of place.steps) {
    try {
      if (st.kind === "route") { navigate(st.path); return "route:" + st.path; }
      const base = st.kind === "navigator" ? nav : win;
      if (!base) continue;
      const parts = st.call.split(".");
      let owner: any = base;
      for (const p of parts.slice(0, -1)) owner = owner ? owner[p] : undefined;
      const fn = owner ? owner[parts[parts.length - 1]] : undefined;
      if (typeof fn !== "function") continue;
      fn.apply(owner, st.args || []);
      return st.kind + ":" + st.call;
    } catch { /* this way failed: the next */ }
  }
  navigate("/library");
  return "route:/library";
}

const str = (v: unknown) => (typeof v === "string" && v.trim() && v.length < 4096 ? v.trim() : undefined);
const num = (v: unknown) => (typeof v === "number" && isFinite(v) && v > 0 ? v : 0);
const okApp = (n: unknown): n is number => typeof n === "number" && Number.isInteger(n) && n > 0 && n < 2 ** 32;

/** Steam's app id for a game in Hearth's library: steam:<n> is Steam's own, others through Hearth Sync's shortcuts. */
export function appIdFor(id: unknown, shortcuts: Record<string, unknown>): number | null {
  if (typeof id !== "string") return null;
  const m = /^steam:(\d{1,10})$/.exec(id);
  if (m) { const n = Number(m[1]); return okApp(n) ? n : null; }
  const v = shortcuts && Object.prototype.hasOwnProperty.call(shortcuts, id) ? shortcuts[id] : null;
  const n = typeof v === "string" && /^\d+$/.test(v) ? Number(v) : v;
  return okApp(n) ? n : null;
}

function cardOf(g: any, shortcuts: Record<string, unknown>): Card | null {
  if (!g || typeof g !== "object") return null;
  const appid = appIdFor(g.id, shortcuts);
  if (appid === null) return null;   // not in Steam (Hearth Sync hasn't added it, or its store is switched off)
  return { appid, title: str(g.title) || String(g.id), store: str(g.store) || "", minutes: num(g.minutes),
           cover: str(g.cover), wide: str(g.wide), hero: str(g.hero), logo: str(g.logo) };
}

function cards(list: unknown, shortcuts: Record<string, unknown>, max: number): Card[] {
  const out: Card[] = [];
  const seen = new Set<number>();
  for (const g of Array.isArray(list) ? list : []) {
    const c = cardOf(g, shortcuts);
    if (c && !seen.has(c.appid)) { seen.add(c.appid); out.push(c); if (out.length >= max) break; }
  }
  return out;
}

/** The home from Hearth's own layout (`nimbus --steam-mode-data home`, hosts/hearth-core steam_takeover.rs home_layout):
 *  Jump back in, Continue playing, Favorites, a row per store (Steam's, then Hearth Sync's collections' names),
 *  Recently added. Only games Steam can open: Steam's, and the shortcuts Hearth Sync made (its shortcuts.json). */
export function homeFromHearth(raw: any, shortcuts: Record<string, unknown>): HomeView {
  const sc = shortcuts && typeof shortcuts === "object" ? shortcuts : {};
  const h = raw && typeof raw === "object" ? raw : {};
  const shelves: Shelf[] = [];
  const cont = cards(h.continue, sc, 10);
  const favs = cards(h.favorites, sc, 30);
  if (cont.length) shelves.push({ key: "continue", title: "Continue playing", items: cont, wide: true });
  if (favs.length) shelves.push({ key: "fav", title: "Favorites", items: favs });
  const names = new Map(SYNC_COLLECTIONS);
  let count = 0;
  for (const r of Array.isArray(h.rows) ? h.rows : []) {
    const items = cards(r && r.items, sc, 30);
    if (!items.length) continue;
    count += items.length;
    const id = str(r.id) || "other";
    const title = id === "steam" ? "Steam" : names.get(id) || str(r.title) || "Other games";
    shelves.push({ key: "row-" + id, title, items });
  }
  const added = cards(h.added, sc, 12);
  if (added.length > 2) shelves.push({ key: "added", title: "Recently added", items: added });
  const hero = cardOf(h.hero, sc) || cont[0] || added[0] || null;
  return { from: "hearth", hero, shelves, count: Math.max(count, cont.length, favs.length) };
}

export interface SteamApp { appid: number; display_name?: string; rt_last_time_played?: number; app_type?: number; minutes_playtime_forever?: number }

/** The home from Steam alone (when Hearth's program doesn't answer): Steam's recent games, its Favorites, Steam's
 *  games, and a row per Hearth Sync collection (Steam collections named after a store). */
export function homeFromSteam(apps: SteamApp[], collections: { name: string; appids: number[] }[], favorites: number[]): HomeView {
  const byId = new Map<number, SteamApp>();
  for (const a of Array.isArray(apps) ? apps : []) if (a && okApp(a.appid) && str(a.display_name)) byId.set(a.appid, a);
  const card = (a: SteamApp, store: string): Card => ({ appid: a.appid, title: str(a.display_name)!, store, minutes: num(a.minutes_playtime_forever) });
  const az = (l: SteamApp[]) => [...l].sort((x, y) => x.display_name!.toLowerCase().localeCompare(y.display_name!.toLowerCase()) || x.appid - y.appid);
  const recent = [...byId.values()].filter((a) => num(a.rt_last_time_played) > 0)
    .sort((x, y) => num(y.rt_last_time_played) - num(x.rt_last_time_played) || x.appid - y.appid).slice(0, 10);
  const shelves: Shelf[] = [];
  const syncNames = new Set(SYNC_COLLECTIONS.map(([, n]) => n));
  const inSync = new Map<number, string>();
  for (const c of Array.isArray(collections) ? collections : []) {
    if (!c || !syncNames.has(c.name)) continue;
    for (const id of c.appids || []) if (byId.has(id) && !inSync.has(id)) inSync.set(id, c.name);
  }
  const storeOf = (id: number) => inSync.get(id) || (byId.get(id)!.app_type === 1 ? "Steam" : "");
  if (recent.length) shelves.push({ key: "continue", title: "Continue playing", items: recent.map((a) => card(a, storeOf(a.appid))), wide: true });
  const favs = az((favorites || []).map((id) => byId.get(id)).filter((a): a is SteamApp => !!a)).slice(0, 30);
  if (favs.length) shelves.push({ key: "fav", title: "Favorites", items: favs.map((a) => card(a, storeOf(a.appid))) });
  const steam = az([...byId.values()].filter((a) => a.app_type === 1 && !inSync.has(a.appid)));
  if (steam.length) shelves.push({ key: "row-steam", title: "Steam", items: steam.slice(0, 30).map((a) => card(a, "Steam")) });
  let count = steam.length;
  for (const [id, name] of SYNC_COLLECTIONS) {
    const items = az([...inSync.entries()].filter(([, n]) => n === name).map(([a]) => byId.get(a)!));
    if (!items.length) continue;
    count += items.length;
    shelves.push({ key: "row-" + id, title: name, items: items.slice(0, 30).map((a) => card(a, name)) });
  }
  return { from: "steam", hero: shelves[0] && shelves[0].key === "continue" ? shelves[0].items[0] : null, shelves, count };
}

/** Hearth's layout when it has games, else Steam's; null: nothing to show (Steam's own Home stays). */
export function pickHome(hearth: HomeView | null, steam: HomeView | null): HomeView | null {
  if (hearth && hearth.shelves.length) return hearth;
  if (steam && steam.shelves.length) return steam;
  return null;
}

export function played(minutes: number | undefined): string {
  const m = num(minutes);
  if (!m) return "";
  if (m < 60) return `${Math.round(m)} min played`;
  const h = m / 60;
  return `${h < 10 ? Math.round(h * 10) / 10 : Math.round(h)} h played`;
}

export type Shape = "portrait" | "wide" | "hero" | "logo";
/** Steam's appStore picture methods to try for a shape, best first (Hearth Sync sets a shortcut's custom pictures). */
export function steamArtMethods(shape: Shape): string[] {
  return shape === "portrait" ? ["GetCustomVerticalCapsuleURLs", "GetVerticalCapsuleURLs"]
    : shape === "wide" ? ["GetCustomLandcapeImageURLs", "GetLandscapeImageURLs", "GetCachedLandscapeImageURLs"]
    : shape === "hero" ? ["GetCustomHeroImageURLs", "GetHeroImageURLs", "GetCustomLandcapeImageURLs", "GetLandscapeImageURLs"]
    : ["GetCustomLogoImageURLs", "GetLogoImageURLs"];
}

/** The pictures to try for a card, best first: Steam's own (what Steam shows for it), then Hearth's. No duplicates. */
export function artCandidates(c: Card | null, shape: Shape, steamUrls: string[]): string[] {
  if (!c) return [];
  const mine = shape === "portrait" ? [c.cover, c.wide] : shape === "wide" ? [c.wide, c.hero, c.cover] : shape === "hero" ? [c.hero, c.wide] : [c.logo];
  const out: string[] = [];
  for (const r of [...(steamUrls || []), ...mine]) if (typeof r === "string" && r.trim() && !out.includes(r.trim())) out.push(r.trim());
  return out;
}
