// CI only (render.mjs): a stand-in for @decky/api (the backend's answers, made up) and Decky's routerHook.
// window.HE_HEARTH false: not Hearth OS. window.HE_SETTINGS: what's saved. HE_EMPTY: no games. No Steam, no network.
const svg = (a, b, shape) => {
  const s = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 300 450'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='${a}'/><stop offset='1' stop-color='${b}'/></linearGradient></defs><rect width='300' height='450' fill='url(#g)'/>${shape}</svg>`;
  return "data:image/svg+xml;base64," + btoa(s);
};
const circle = "<circle cx='210' cy='130' r='70' fill='rgba(255,255,255,.18)'/><path d='M0 360 L120 250 L210 330 L300 260 L300 450 L0 450Z' fill='rgba(0,0,0,.28)'/>";
const ridge = "<path d='M0 300 L80 210 L150 280 L230 170 L300 240 L300 450 L0 450Z' fill='rgba(0,0,0,.3)'/><circle cx='70' cy='90' r='34' fill='rgba(255,240,200,.35)'/>";
const G = (id, title, store, a, b, extra = {}) => ({ id, title, store, cover: svg(a, b, id.length % 2 ? circle : ridge), wide: svg(b, a, ridge), hero: svg(a, b, circle), ...extra });
const games = {
  starfall: G("epic:starfall", "Starfall Drift", "Epic", "#1d4e89", "#f18f01", { minutes: 754 }),
  ember: G("steam:1001", "Ember Knights of Ash", "Steam", "#6a040f", "#ffba08"),
  tides: G("gog:tides", "Tidecaller", "GOG", "#0b525b", "#99e2b4"),
  orbit: G("steam:1002", "Orbit Gardeners", "Steam", "#3a0ca3", "#4cc9f0"),
  kart: G("steam:1003", "Pebble Kart", "Steam", "#2b9348", "#eeef20"),
  forge: G("steam:1004", "Forge & Fable", "Steam", "#432818", "#bb9457"),
  lantern: G("epic:lantern", "Lantern Road", "Epic", "#22223b", "#c9ada7"),
  frost: G("steam:1005", "Frostline Rally", "Steam", "#023e8a", "#caf0f8"),
  moss: G("gog:moss", "Mossheart", "GOG", "#386641", "#a7c957"),
  dune: G("steam:1006", "Dune Couriers", "Steam", "#9c6644", "#ede0d4"),
  hollow: G("epic:hollow", "Hollow Bell", "Epic", "#240046", "#e0aaff"),
  notsynced: G("gog:notsynced", "Not In Steam Yet", "GOG", "#555", "#999"),
};
const g = (...k) => k.map((x) => games[x]);
const HOME = {
  hero: games.starfall,
  continue: g("starfall", "ember", "tides", "orbit", "notsynced"),
  favorites: g("forge", "kart", "lantern", "moss"),
  rows: [
    { id: "steam", title: "Steam", total: 6, items: g("dune", "ember", "forge", "frost", "orbit", "kart") },
    { id: "epic", title: "Epic", total: 3, items: g("hollow", "lantern", "starfall") },
    { id: "gog", title: "GOG", total: 3, items: g("moss", "notsynced", "tides") },
  ],
  added: [], count: 12,
};
const SHORTCUTS = { "epic:starfall": 3000000001, "gog:tides": 3000000002, "epic:lantern": 3000000003, "gog:moss": 3000000004, "epic:hollow": 3000000005 };
let saved = { ...(window.HE_SETTINGS || {}) };
window.HE_REPORTS = [];
window.HE_SAVES = [];
window.HE_TOASTS = [];
const empty = { hero: null, continue: [], favorites: [], rows: [], added: [], count: 0 };
const handlers = {
  hearth_os: () => (window.HE_HEARTH === false ? { ok: false, message: "This plugin is part of Hearth OS. Get Hearth OS at hearthos.fun" } : { ok: true, message: "" }),
  get_settings: () => saved,
  set_settings: (c) => { window.HE_SAVES.push(c); const p = c.presets ? { presets: { ...(saved.presets || {}), ...c.presets } } : {}; saved = { ...saved, ...c, ...p }; return { ok: true, settings: saved }; },
  brand: () => ({ base: window.HE_BASE, eras: window.HE_ERAS, logo: window.HE_LOGO, wallpaper: window.HE_WALL }),
  home: () => (window.HE_EMPTY ? { ok: true, home: empty, shortcuts: {} } : { ok: true, home: HOME, shortcuts: SHORTCUTS }),
  art: () => "",
  report: (l) => { window.HE_REPORTS.push(l); return null; },
  diagnostics: (t) => { (window.HE_DIAGS = window.HE_DIAGS || []).push(t); return { ok: true, path: "~/.local/share/nimbus/hearth-eras.log" }; },
  send_log: (reason, text) => { (window.HE_SENT = window.HE_SENT || []).push({ reason, len: text.length }); return { ok: true, code: "TEST-CODE-01", at: 1762200000 }; },
  last_sent: () => null,
};
export function callable(name) { return async (...a) => (handlers[name] ? handlers[name](...a) : null); }
export function definePlugin(f) { return f; }
window.HE_PATCHES = new Map();
export const routerHook = {
  addPatch(path, fn) { window.HE_PATCHES.set(path, fn); return fn; },
  removePatch(path, fn) { if (window.HE_PATCHES.get(path) === fn) window.HE_PATCHES.delete(path); },
};
export const toaster = { toast(t) { window.HE_TOASTS.push(t); } };
// Steam's focused window and its Navigator (what the places on the home call), recording each call.
// window.HE_NAVIGATOR === "missing": a Steam whose navigator lacks them (each place falls back to the Library).
window.HE_NAVCALLS = [];
const rec = (name) => (...a) => { window.HE_NAVCALLS.push(name + (a.length ? "(" + JSON.stringify(a) + ")" : "")); };
const navigator = window.HE_NAVIGATOR === "missing" ? {} : {
  Downloads: rec("Downloads"), Chat: rec("Chat"), Settings: rec("Settings"), SteamWeb: rec("SteamWeb"),
  Media: { Grid: rec("Media.Grid"), List: rec("Media.List") },
};
window.SteamUIStore = { GetFocusedWindowInstance: () => ({ Navigator: navigator, NavigateToSteamWeb: rec("window.NavigateToSteamWeb") }) };
