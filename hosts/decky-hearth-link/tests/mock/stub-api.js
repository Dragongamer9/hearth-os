// CI only (render-panel.mjs): a stand-in for @decky/api. The backend (main.py) answers with made-up devices, games,
// copies and Hearth Storage, in the short form main.py gives the panel (no Hearth Link, no Steam, no network).
// window.HL_STATE picks what the panel shows (see STATES).
const st = () => window.HL_STATE || "devices";
const now = Date.now();
const min = (m) => now - m * 60000;
const id = (c) => c.repeat(64);
const DESK = id("b"), TOWER = id("c"), DECK = id("d"), LAPTOP = id("e");
const peers = [
  { id: DESK, name: "Living room PC", online: true, lan: true, lastSeen: min(0) },
  { id: DECK, name: "Steam Deck", online: true, lan: false, lastSeen: min(0) },
  { id: TOWER, name: "Tower", online: false, lan: null, lastSeen: min(180) },
];
const servers = [{ id: TOWER, name: "Tower", free: 3.2e12, usable: 4e12, seen: min(180), canWake: true }];
const job = (x) => ({ id: "0123456789abcdef", peer: DESK, game: "steam:1091500", title: "Cyberpunk 2077", store: "steam", state: "copying",
  total: 70e9, done: 31.5e9, speed: 112e6, eta: 344, error: "", from: "Living room PC", created: min(5), relay: false, ...x });
const base = () => ({
  running: true, me: { id: id("a"), name: "ROG Ally", kind: "handheld" }, peers, nearby: [], pairing: null, shortCodes: true, joining: null,
  pairEvent: null, transfers: [], offers: [], sending: [], playing: false, lastChange: min(12),
  storage: { server: false, savesFollow: true, servers, jobs: [] },
});
const STATES = {
  devices: () => base(),
  browse: () => base(),
  send: () => base(),
  sending: () => ({ ...base(), sending: [{ to: "Living room PC", title: "Hades", bytes: 6.2e9 }] }),
  getting: () => ({ ...base(), transfers: [job({}), job({ id: "1111111111111111", game: "gog:celeste", title: "Celeste", store: "gog", state: "paused", total: 1.2e9, done: 0.3e9, speed: 0, eta: null, from: "Steam Deck", relay: true })],
    offers: [{ id: "2222222222222222", from: "Steam Deck", title: "Hollow Knight", store: "gog", size: 9.1e9 }] }),
  done: () => ({ ...base(), transfers: [job({ state: "done", done: 70e9, speed: 0, eta: null }), job({ id: "3333333333333333", game: "epic:alan-wake-2", title: "Alan Wake 2", store: "epic", state: "done", total: 90e9, done: 90e9, speed: 0, eta: null })] }),
  storage: () => ({ ...base(), peers: peers.map((p) => (p.id === TOWER ? { ...p, online: true, lan: true, lastSeen: min(0) } : p)),
    storage: { server: false, savesFollow: true, servers: [{ ...servers[0], seen: min(1) }],
      jobs: [job({ id: "4444444444444444", title: "Baldur's Gate 3", game: "steam:1086940", from: "Tower", total: 150e9, done: 102e9 })] } }),
  "storage-asleep": () => base(),
  pairing: () => ({ ...base(), pairing: { code: "482913", until: now + 272000, netCode: "HRTH-7QK2", netUntil: now + 590000 } }),
  "pairing-join": () => ({ ...base(), nearby: [{ id: LAPTOP, name: "Kitchen laptop", pairing: true }], joining: null }),
  paired: () => ({ ...base(), pairEvent: { name: "Kitchen laptop", ok: true, error: "" } }),
  error: () => base(),
  "sync-on": () => base(),
  "sync-saves-off": () => ({ ...base(), storage: { ...base().storage, savesFollow: false } }),
  playing: () => ({ ...base(), playing: true }),
  "not-running": () => ({ running: false, installed: true, error: "Hearth Link isn't running on this device",
    last: { at: min(42), status: { me: base().me, peers, storage: base().storage } } }),
  "not-installed": () => ({ running: false, installed: false, error: "Hearth Link isn't running on this device", last: null }),
};
const mine = [
  { id: "steam:1145360", title: "Hades", store: "steam", size: 15e9 }, { id: "steam:620", title: "Portal 2", store: "steam", size: 12e9 },
  { id: "gog:cyberpunk", title: "Cyberpunk 2077", store: "gog", size: 70e9 }, { id: "epic:fortnite", title: "Fortnite", store: "epic", size: 32e9 },
  { id: "share:media:films", title: "films", store: "share", size: 1 },
];
const theirs = [
  { id: "steam:1091500", title: "Cyberpunk 2077", store: "steam", size: 70e9 }, { id: "steam:1086940", title: "Baldur's Gate 3", store: "steam", size: 150e9 },
  { id: "gog:witcher3", title: "The Witcher 3", store: "gog", size: 50e9 }, { id: "emu:snes:smw", title: "Super Mario World", store: "emu", size: 0.5e6 },
  { id: "beacon:bfme2", title: "BFME II", store: "beacon", size: 6e9 },
];
const storageInfo = () => {
  const asleep = st() === "storage-asleep";
  return {
    running: true,
    cache: { enabled: true, using: "Tower", jobs: [], available: asleep ? [] : [{ game: "steam:1245620", title: "Elden Ring", size: 60e9 }, { game: "steam:990080", title: "Hogwarts Legacy", size: 85e9 }],
             updates: asleep ? [] : [{ game: "steam:570", title: "Dota 2" }] },
    saves: { follow: true, server: true, games: [{ title: "Hades", at: min(3), conflict: false, pending: false }, { title: "Celeste", at: min(70), conflict: asleep, pending: false },
      { title: "Elden Ring", at: min(60 * 26), conflict: false, pending: asleep }] },
    updates: { enabled: true, fetching: !asleep, using: "Tower", cached: [{ version: "1.0", at: min(600), verified: 2 }] },
    health: { serverName: "Tower", serverOffline: asleep, devices: [
      { name: "ROG Ally", me: true, online: true, lastSeen: now, free: 210e9, total: 1e12, battery: asleep ? 14 : 76, version: "1.0", upToDate: true, lastBackup: min(300), server: false },
      { name: "Living room PC", me: false, online: true, lastSeen: now, free: asleep ? 40e9 : 900e9, total: 2e12, battery: null, version: "1.0", upToDate: true, lastBackup: min(200), server: false },
      { name: "Steam Deck", me: false, online: true, lastSeen: now, free: 80e9, total: 512e9, battery: 58, version: asleep ? "0.9" : "1.0", upToDate: !asleep, lastBackup: asleep ? min(60 * 24 * 12) : min(400), server: false },
      { name: "Tower", me: false, online: !asleep, lastSeen: min(asleep ? 180 : 0), free: 3.2e12, total: 4e12, battery: null, version: "1.0", upToDate: true, lastBackup: null, server: true }] },
  };
};
// The sync switches (main.py sync_info / set_sync / fetch_plugin): off unless the state turns them on.
const sw = { plugins: st() === "sync-on", settings: st() === "sync-on" };
const handlers = {
  sync_info: () => ({ switches: { ...sw, plugins: sw.plugins || st() === "sync-on", settings: sw.settings || st() === "sync-on" },
    available: st() === "sync-on" || sw.plugins ? [
      { folder: "SDH-CssLoader", name: "CSS Loader", version: "2.1.2", from: DESK, fromName: "Living room PC", online: true },
      { folder: "PowerTools", name: "PowerTools", version: "2.0.3", from: TOWER, fromName: "Tower", online: false }] : [] }),
  set_sync: (name, on) => { sw[name] = on; return { ok: true, switches: { ...sw } }; },
  fetch_plugin: (folder) => ({ ok: true, artifact: `file:///home/you/homebrew/settings/decky-hearth-link/incoming/${folder}.zip`, name: "CSS Loader", version: "2.1.2", hash: "a".repeat(64) }),
  status: () => STATES[st()](),
  storage: () => storageInfo(),
  start_link: () => ({ ok: true }),
  call: (cmd, args) => {
    if (cmd === "local.games") return { ok: true, data: { games: mine } };
    if (cmd === "games") return { ok: true, data: { games: theirs } };
    if (st() === "error" && cmd === "send") return { ok: false, error: "Steam Deck has only 80 GB free: it needs 32 GB more" };
    return { ok: true, data: null };
  },
};
export function callable(name) { return async (...a) => (handlers[name] ? handlers[name](...a) : null); }
export function definePlugin(f) { return f; }
// Steam's restart, for the "Restart Steam to see it" button after a Steam game was copied.
window.SteamClient = { User: { StartRestart: () => {} } };
// Decky Loader's own install call (it would show its "Install …?" prompt).
window.DeckyBackend = { call: async (route, ...a) => { window.HL_INSTALLED = [route, ...a]; return null; } };
