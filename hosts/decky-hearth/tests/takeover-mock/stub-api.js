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
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><circle cx="${w * 0.78}" cy="${h * 0.3}" r="${h * 0.42}" fill="rgba(255,255,255,.07)"/>${wide ? "" : `<text x="18" y="${h - 34}" font-family="Segoe UI,Noto Sans,sans-serif" font-size="34" font-weight="800" fill="#fff">${title.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</text>`}</svg>`;
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
  // Bought in the store, downloading through Hearth (the Store's details show its progress).
  G("gog:phantom", "Cyberpunk 2077: Phantom Liberty", "gog", "GOG", "gog", { installed: false, state: "downloading", added: 0 }),
  G("steam:620", "Portal 2", "steam", "Steam", "steam", { added: 4 }),
  G("epic:fortnite", "Fortnite", "epic", "Epic", "epic", { added: 3 }),
  // The cases behind 0.6.60's blank cards: a webp from Hearth's art cache (nimbus-art://cache/…), a picture that
  // doesn't load (the card falls back to its title), a game with no art at all, a Steam game whose only picture is
  // Steam's own (steam-lib:…, Steam's library cache).
  G("gog:webp", "Webp Cover Game", "gog", "GOG", "gog", { fav: true, cover: "nimbus-art://cache/0123456789abcdef01234567", wide: null, hero: null }),
  G("gog:broken", "Broken Picture Game", "gog", "GOG", "gog", { fav: true, cover: "https://broken.invalid/cover.jpg", wide: null, hero: null }),
  G("battlenet:bnet", "Battle.net", "battlenet", "Battle.net", "battlenet", { fav: true, cover: "", wide: "", hero: "" }),
  G("steam:440", "Team Fortress 2", "steam", "Steam", "steam", { fav: true, cover: null, wide: null, hero: null }),
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
  gog: { store: "gog", sections: [
    { id: "featured", title: "Featured", items: [{ ...item("gog", "Cyberpunk 2077: Phantom Liberty", "$29.99"), id: "phantom", url: "https://www.gog.com/en/game/cyberpunk_2077_phantom_liberty", sub: "Expansion" },
      ...["The Witcher 3", "Baldur's Gate 3", "Disco Elysium"].map((t) => item("gog", t, "$39.99"))] },
  ] },
};
const answers = {
  home, library: { tabs: TABS.map(([id, title]) => ({ id, title, count: byTab(id).length, installed: byTab(id).length, items: byTab(id) })).filter((t) => t.count) },
  downloads: { active: [{ id: "gog:phantom", title: "Cyberpunk 2077: Phantom Liberty", store: "GOG", state: "downloading", progress: 0.42, bytesPerSec: 31e6, bytesDone: 21e9, bytesTotal: 50e9 }] },
  storage: { drives: [{ name: "This device", kind: "internal", mount: "/var", size: 512e9, used: 380e9, free: 132e9, readOnly: false, steamLibs: 1 }] },
  info: { protondb: { label: "Gold" }, achievements: { total: 40, done: 12 } },
};
// A real webp, made by the browser (what Hearth's art cache often holds).
function webp(title) {
  const c = document.createElement("canvas");
  c.width = 300; c.height = 450;
  const x = c.getContext("2d");
  const g = x.createLinearGradient(0, 0, 300, 450);
  g.addColorStop(0, "#0b6e4f"); g.addColorStop(1, "#08415c");
  x.fillStyle = g; x.fillRect(0, 0, 300, 450);
  x.fillStyle = "#fff"; x.font = "800 30px sans-serif"; x.fillText("WEBP", 20, 60); x.font = "600 20px sans-serif"; x.fillText(title, 20, 400);
  return c.toDataURL("image/webp");
}
function picture(ref) {
  if (!ref) return "";
  if (ref.startsWith("mock:")) return pic(ref);
  if (ref.startsWith("nimbus-art://cache/")) return webp("from the art cache");
  if (ref.startsWith("steam-lib:")) return pic(`mock:tall:Steam's own ${ref.split(":")[1]}`);
  return "";   // the backend couldn't get it either (the broken one)
}
export const rowGames = games.filter((g) => g.source !== "steam").map((g) => ({ id: g.id, title: g.title, store: g.store, cover: g.cover, wide: g.wide }))
  .sort((a, b) => (a.id.startsWith("gog:") || a.id.startsWith("battlenet:") ? -1 : 1) - (b.id.startsWith("gog:") || b.id.startsWith("battlenet:") ? -1 : 1));
// Hearth in Steam's menus, Quick Access and in-game: what Hearth's core service answers (core_service.rs) for a
// running Alan Wake 2 played through Hearth.
answers.power = { mode: { supported: true, mode: "balanced", available: ["efficiency", "balanced", "performance"] }, tdp: { supported: true, watts: 15, min: 5, max: 30 } };
const gameView = { id: "epic:alan-wake-2", title: "Alan Wake 2", source: "epic", store: "Epic", kind: "proton", mods: true, proton: "GE-Proton",
  input: { layout: "shooter", name: "Shooter / action", choices: [{ id: "off", label: "Off (the game's own controls)" }, { id: "shooter", label: "Shooter / action" },
    { id: "pointclick", label: "Point-and-click / strategy" }, { id: "retro", label: "Retro / arrow keys" }, { id: "mouse", label: "Mouse only" }, { id: "rts", label: "Strategy (RTS)" }] },
  protonChoices: [{ id: "", label: "Default" }, { id: "GE-Proton", label: "GE-Proton (newest)" }, { id: "Proton 9.0", label: "Proton 9.0" }] };
// The Steam pages' changes (manage.tsx, the Store's details): what Hearth's core answers through `call` / `mods-call`.
const GB = 1e9;
const calls = {
  "diskspace.status": () => ({
    drives: [{ id: "/var", name: "This device", kind: "internal", mount: "/var", fs: "btrfs", size: 512 * GB, used: 380 * GB, free: 132 * GB, readOnly: false, games: true, steamLibs: ["/var/home/you/.local/share/Steam"] },
             { id: "/run/media/you/sd", name: "SD card", kind: "sd", mount: "/run/media/you/sd", fs: "ext4", size: 256 * GB, used: 61 * GB, free: 195 * GB, readOnly: false, games: true, steamLibs: [] }],
    games: [{ id: "steam:292030", title: "The Witcher 3", source: "steam", store: "Steam", drive: "/var", size: 52 * GB, canMove: true, uninstall: "game" },
            { id: "epic:alan-wake-2", title: "Alan Wake 2", source: "epic", store: "Epic", drive: "/var", size: 88 * GB, canMove: true, uninstall: "game" },
            { id: "gog:cyberpunk", title: "Cyberpunk 2077", source: "gog", store: "GOG", drive: "/run/media/you/sd", size: 61 * GB, canMove: true, uninstall: "game" },
            { id: "recomp:zelda64", title: "Zelda 64: Recompiled", source: "recomps", store: "Recomps", drive: "/var", size: 0.3 * GB, canMove: true, uninstall: "recomp" },
            { id: "battlenet:d4", title: "Diablo IV", source: "battlenet", store: "Battle.net", drive: "/var", size: 90 * GB, canMove: false, moveWhy: "Move it with Battle.net", uninstall: null, uninstallWhy: "Uninstall it in Battle.net." }],
    job: { state: "moving", phase: "copy", title: "Hades", done: 6.2 * GB, total: 9.8 * GB }, scan: { state: "idle" } }),
  "diskspace.cleanList": () => ({ items: [
    { key: "shader:620", kind: "Shader cache", title: "Portal 2", where: "This device", bytes: 1.4 * GB, selectable: true, on: true },
    { key: "mesa", kind: "Mesa shader cache", title: "Graphics driver caches", where: "This device", bytes: 0.9 * GB, selectable: true, on: true },
    { key: "prefix:epic-old", kind: "Proton prefix", title: "Old Epic game", where: "This device", bytes: 2.2 * GB, selectable: true, on: false, confirm: true, saves: ["Documents/My Games/Old"], why: "has saves" },
  ] }),
  "recomps.list": () => ({ consoles: { n64: "Nintendo 64", gamecube: "GameCube", x360: "Xbox 360" }, recomps: [
    { id: "zelda64", title: "Zelda 64: Recompiled", console: "n64", needs: "Majora's Mask (USA) ROM", installed: true, version: "1.2.0", copy: "/home/you/roms/mm.z64" },
    { id: "banjo", title: "Banjo: Recompiled", console: "n64", needs: "Banjo-Kazooie (USA, v1.0) ROM", installed: false, job: { busy: true, stage: "Downloading BanjoRecompiled-Linux.zip…", progress: 0.63 } },
    { id: "sonic-unleashed", title: "Unleashed Recompiled", console: "x360", needs: "Sonic Unleashed (your own copy)", installed: false },
    { id: "sm64", title: "Super Mario 64 (sm64ex)", console: "n64", needs: "Super Mario 64 (USA) ROM", installed: false },
  ] }),
};
const modsCalls = {
  list: () => ({ value: [{ id: "steam:489830", title: "Skyrim Special Edition", mods: 4, modsOn: 3 }, { id: "gog:cyberpunk", title: "Cyberpunk 2077", mods: 2, modsOn: 2 }] }),
  mods_of: (a) => ({ title: "Skyrim Special Edition", beacon: false, mods: [
    { id: "skyui", name: "SkyUI", enabled: true, targetLabel: "Data folder", size: 3.1e6 },
    { id: "hdtex", name: "HD Textures", enabled: true, targetLabel: "Data folder", size: 2.4e9 },
    { id: "sep", name: "— Gameplay —", kind: "separator" },
    { id: "lanterns", name: "Sky Lanterns", enabled: false, targetLabel: "Data folder", size: 2.1e7 },
    { id: "fixes", name: "Community Fixes", enabled: true, targetLabel: "Game folder", size: 4e5 }] }),
  downloads_list: () => ({ items: [{ id: "d1", name: "Immersive Weather · Main file", state: "done", game: "steam:489830", size: 5.6e7 }] }),
};
// Couch Co-op in Steam Game Mode (coop.tsx): hearth-coop's players in the saved order, and the games it can split.
const coopPads = [
  { id: "event30", name: "DualSense Wireless Controller", label: "DualSense Wireless Controller", kind: "physical", player: 1, playerName: "Dragon", rumble: true },
  { id: "event20", name: "ROG Ally controller", label: "ROG Ally controller (InputPlumber)", kind: "inputplumber", player: 2, playerName: "Player 2", rumble: true },
  { id: "event34", name: "Xbox Wireless Controller", label: "Xbox Wireless Controller", kind: "physical", player: 3, playerName: "Player 3", rumble: true },
  { id: "event36", name: "8BitDo Pro 2", label: "8BitDo Pro 2", kind: "physical", player: 4, playerName: "Player 4", rumble: true },
  { id: "event38", name: "Retro USB Pad", label: "Retro USB Pad", kind: "physical", player: null, playerName: "", rumble: false },
];
const coopGames = { games: [
  { id: "steam:600", title: "Kart Party", source: "steam", canPlay: true, run: "native" },
  { id: "gog:dungeon-pals", title: "Dungeon Pals", source: "gog", canPlay: true, run: "windows" },
  { id: "manual:arena", title: "Pixel Arena", source: "manual", canPlay: true, run: "native" },
  { id: "steam:500", title: "Shooty", source: "steam", canPlay: false, why: "Shooty uses EasyAntiCheat, which doesn't allow several copies or LAN play." },
] };
const coopOps = {
  players: () => ({ pads: coopPads, max: 4, running: false, lastError: null }),
  games: () => coopGames,
  order: (pads) => ({ pads: pads.map((id, i) => ({ ...coopPads.find((p) => p.id === id), player: i < 4 ? i + 1 : null })), max: 4, running: false }),
  identify: () => ({ rumbled: true }),
};
const handlers = {
  coop: (op, pads, pad) => (coopOps[op] ? coopOps[op](pads, pad) : { error: "no mock" }),
  coop_target: () => ({ ok: true, exe: "/bin/sh", startDir: "/home/you", options: "-c '…' hearth-steam-coop /usr/libexec/nimbus/hearth-coop play --game steam:600" }),
  hearth_call: (m, a) => (calls[m] ? calls[m](a) : { ok: true }),
  hearth_mods_call: (c, a) => (modsCalls[c] ? modsCalls[c](a) : {}),
  hearth_games_disk: (m) => ({ installRoot: m + "/Games", drive: "SD card" }),
  hearth_data: (what, arg) => (what === "store" ? stores[arg] || stores.steam : answers[what] || { error: "no mock" }),
  core_status: () => ({ running: true, version: "0.0.0" }),
  hearth_front: (appid, game, overlay) => ({ game: game || null, front: !overlay, overlay, layout: game ? "Shooter / action" : null, active: !!game && !overlay }),
  hearth_game: () => gameView, hearth_input_set: (g, l) => ({ layout: l }), hearth_set_proton: (g, p) => ({ proton: p }),
  hearth_mods: () => ({ count: 7, enabled: 5, beacon: false, mods: [{ name: "Ultra textures", enabled: true }] }),
  hearth_tool: () => ({}), update_status: () => ({ state: "none" }), get_steam_default: () => false, get_link: () => null,
  get_picture: picture, art_error: () => null,
  log_error: () => null, get_errors: () => [], get_settings: () => ({}), set_settings: (s) => s, get_shortcuts: () => ({}),
};
export function callable(name) { return async (...a) => (handlers[name] ? handlers[name](...a) : null); }
export const routerHook = { addPatch() { return {}; }, removePatch() {}, addRoute() {}, removeRoute() {} };
export const toaster = { toast() {} };
export const useQuickAccessVisible = () => true;
export function definePlugin(f) { return f; }
