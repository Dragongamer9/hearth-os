// CI only (render-takeover.mjs): a stand-in for @decky/api. The backend's calls answer with made-up games in the
// shapes hearth-core's steam_takeover.rs gives (no Hearth, no Steam, no network).
const PALETTE = [["#3a1c71", "#d76d77"], ["#0f2027", "#2c5364"], ["#134e5e", "#71b280"], ["#41295a", "#2f0743"], ["#c04848", "#480048"],
                 ["#1f4037", "#99f2c8"], ["#373b44", "#4286f4"], ["#8e2de2", "#4a00e0"], ["#ff512f", "#6b1d05"], ["#16222a", "#3a6073"]];
function pic(ref) {
  // "mock:<shape>:<title>": a gradient with the title, like a cover or a banner.
  const [, shape, title] = ref.split(":");
  const wide = shape !== "tall";
  const [w, h] = wide ? [460, 215] : [300, 450];
  let n = 0;
  for (const c of title) n = (n * 31 + c.charCodeAt(0)) % 9973;
  const [a, b] = PALETTE[n % PALETTE.length];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><circle cx="${w * 0.78}" cy="${h * 0.3}" r="${h * 0.42}" fill="rgba(255,255,255,.07)"/><text x="${wide ? 22 : 18}" y="${h - (wide ? 26 : 34)}" font-family="Segoe UI,Noto Sans,sans-serif" font-size="${wide ? 30 : 34}" font-weight="800" fill="#fff">${title.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</text></svg>`;
  return "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
}

const G = (id, title, source, store, tab, extra = {}) => ({ id, title, source, store, tab, installed: true, state: "installed", cloud: false, fav: false,
  minutes: 0, lastPlayed: 0, added: 1, sizeBytes: 40e9, cover: `mock:tall:${title}`, wide: `mock:wide:${title}`, hero: `mock:wide:${title}`, logo: null, icon: null, ...extra });
const games = [
  G("epic:alan-wake-2", "Alan Wake 2", "epic", "Epic", "epic", { lastPlayed: 50, minutes: 610, fav: true }),
  G("steam:292030", "The Witcher 3", "steam", "Steam", "steam", { lastPlayed: 40, minutes: 5400 }),
  G("beacon:bfme2", "BFME II", "beacon", "Beacon", "beacon", { lastPlayed: 30, minutes: 900, fav: true }),
  G("gog:cyberpunk", "Cyberpunk 2077", "gog", "GOG", "gog", { lastPlayed: 20, minutes: 3000 }),
  G("battlenet:d4", "Diablo IV", "battlenet", "Battle.net", "battlenet", { lastPlayed: 10, minutes: 1200 }),
  G("steam:1145360", "Hades", "steam", "Steam", "steam", { added: 9 }),
  G("steam:504230", "Celeste", "steam", "Steam", "steam", { added: 8, fav: true }),
  G("xodus:halo", "Halo MCC", "xodus", "Xbox PC", "xbox", { added: 7 }),
  G("recomps:zelda64", "Zelda 64", "recomps", "Recomps", "recomps", { added: 6 }),
  G("emu:snes:smw", "Super Mario World", "emu", "Emulators", "emu", { added: 5 }),
  G("steam:620", "Portal 2", "steam", "Steam", "steam", { added: 4 }),
  G("epic:fortnite", "Fortnite", "epic", "Epic", "epic", { added: 3 }),
];
const byTab = (t) => games.filter((g) => g.tab === t);
const TABS = [["steam", "Steam"], ["epic", "Epic"], ["gog", "GOG"], ["battlenet", "Battle.net"], ["xbox", "Xbox"], ["beacon", "Beacon"], ["recomps", "Recomps"], ["emu", "Emulators"]];
const home = {
  hero: games[0], count: games.length,
  continue: games.filter((g) => g.lastPlayed).sort((a, b) => b.lastPlayed - a.lastPlayed),
  favorites: games.filter((g) => g.fav),
  rows: TABS.map(([id, title]) => ({ id, title, total: byTab(id).length, items: byTab(id) })).filter((r) => r.items.length),
  added: [...games].sort((a, b) => b.added - a.added).slice(0, 8),
};
const item = (store, title, final, discount = 0) => ({ store, id: title, title, wide: `mock:wide:${title}`, tall: null,
  price: { final, base: discount ? "$59.99" : null, discount, free: final === "Free" } });
const stores = {
  steam: { store: "steam", sections: [
    { id: "featured", title: "Featured", items: ["Baldur's Gate 3", "Elden Ring", "Hollow Knight", "Stardew Valley", "Balatro"].map((t, i) => item("steam", t, i % 2 ? "$14.99" : "$59.99")) },
    { id: "specials", title: "Specials", items: ["Hades II", "Dead Cells", "Disco Elysium", "Outer Wilds", "Inscryption"].map((t) => item("steam", t, "$19.79", 67)) },
    { id: "free", title: "Free to play", items: ["Dota 2", "Warframe", "Path of Exile 2", "Apex Legends"].map((t) => item("steam", t, "Free")) },
  ] },
};
const answers = {
  home, library: { tabs: TABS.map(([id, title]) => ({ id, title, count: byTab(id).length, installed: byTab(id).length, items: byTab(id) })).filter((t) => t.count) },
  downloads: { active: [{ id: "gog:x", title: "Cyberpunk 2077: Phantom Liberty", store: "GOG", state: "downloading", progress: 0.42, bytesPerSec: 31e6 }] },
  storage: { drives: [{ name: "This device", kind: "internal", mount: "/var", size: 512e9, used: 380e9, free: 132e9, readOnly: false, steamLibs: 1 }] },
  info: { protondb: { label: "Gold" }, achievements: { total: 40, done: 12 } },
};
const handlers = {
  hearth_data: (what, arg) => (what === "store" ? stores[arg] || stores.steam : answers[what] || { error: "no mock" }),
  get_picture: (ref) => (ref && ref.startsWith("mock:") ? pic(ref) : ""),
  log_error: () => null, get_errors: () => [], get_settings: () => ({}), set_settings: (s) => s, get_shortcuts: () => ({}),
};
export function callable(name) { return async (...a) => (handlers[name] ? handlers[name](...a) : null); }
export const routerHook = { addPatch() { return {}; }, removePatch() {}, addRoute() {}, removeRoute() {} };
export const toaster = { toast() {} };
export function definePlugin(f) { return f; }
