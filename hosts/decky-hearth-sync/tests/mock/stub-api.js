// CI only (render-panel.mjs): a stand-in for @decky/api. The backend answers with made-up games and a status after a
// sync (no Hearth, no Steam, no network).
const G = (id, title, source, store, storeName) => ({ id, title, source, store, storeName, cloud: false, cover: "", wide: "", hero: "", logo: "", icon: "",
  exe: "/bin/sh", startDir: "/home/you", options: "" });
const games = [
  G("epic:alan-wake-2", "Alan Wake 2", "epic", "epic", "Epic"), G("epic:fortnite", "Fortnite", "epic", "epic", "Epic"),
  G("gog:cyberpunk", "Cyberpunk 2077", "gog", "gog", "GOG"), G("gog:witcher3", "The Witcher 3", "gog", "gog", "GOG"),
  G("battlenet:d4", "Diablo IV", "battlenet", "battlenet", "Battle.net"), G("xbox:cloud", "Xbox Cloud Gaming", "xbox", "xbox", "Xbox"),
  G("emu:snes:smw", "Super Mario World", "emu", "emu", "Emulators"), G("recomps:zelda64", "Zelda 64", "recomps", "recomps", "Recomps"),
  G("beacon:bfme2", "BFME II", "beacon", "beacon", "Beacon"), G("manual:arena", "Pixel Arena", "manual", "manual", "Added by hand"),
];
// window.HS_STATE picks what the panel shows: synced (all fine), problem, toggles, syncing, confirm.
const state = () => window.HS_STATE || "synced";
const ago = (m) => Date.now() - m * 60000;
const statusFor = () => ({
  synced: { lastSync: ago(4), lastArtTry: ago(4), synced: 10, artMissing: 0, signature: "s", off: "", problems: [] },
  problem: { lastSync: ago(95), lastArtTry: ago(95), synced: 9, artMissing: 2, signature: "s",
    off: "Steam changed (SetCustomArtworkForApp missing): Hearth Sync turned itself off.",
    problems: ["2 games still missing a picture (tried again later; see the log)", "1 change Steam didn't take (tried again next sync)"] },
  toggles: { lastSync: ago(1), lastArtTry: ago(1), synced: 7, artMissing: 0, signature: "s", off: "", problems: [] },
}[state()] || { lastSync: ago(12), lastArtTry: ago(12), synced: 10, artMissing: 0, signature: "s", off: "", problems: [] });
const settingsFor = () => (state() === "toggles" ? { stores: { recomps: false, emu: false, manual: false } } : { stores: {} });
// The import section (src/import.tsx): games in Flathub Steam's folder and on another drive, or none.
const F = (appid, name, how, why) => ({ appid, name, how, why, source: how === "move" ? "/home/you/.var/app/com.valvesoftware.Steam/.local/share/Steam" : "/run/media/you/Games/SteamLibrary" });
const importScan = () => (["import", "import-confirm", "import-waiting", "import-done"].includes(state())
  ? { ok: true, target: "/home/you/.local/share/Steam", games: [F("1091500", "Cyberpunk 2077", "move"), F("292030", "The Witcher 3: Wild Hunt", "move"),
      F("1245620", "ELDEN RING", "move"), F("413150", "Stardew Valley", "register")],
    skipped: [F("570", "Dota 2", "move", "Steam already has it"), F("1174180", "Red Dead Redemption 2", "move", "a folder with its name is already in /home/you/.local/share/Steam")] }
  : { ok: true, target: "/home/you/.local/share/Steam", games: [], skipped: [F("570", "Dota 2", "move", "Steam already has it")] });
const importState = () => ({ "import-waiting": { pending: true, result: null },
  "import-done": { pending: false, result: { ok: true, counts: { moved: 3, registered: 1, libraries: 1, skipped: 2, failed: 0 } } } }[state()] || { pending: false, result: null });
const handlers = {
  get_games: (force) => (force && state() === "syncing" ? new Promise(() => {}) : { ok: true, error: null, games, signature: "s" }),
  get_status: () => statusFor(), set_status: (s) => s, get_settings: () => settingsFor(), set_settings: (s) => s,
  import_scan: () => importScan(), import_state: () => importState(), import_arm: () => true,
  hearth_os: () => (state() === "not-hearth" ? { ok: false, message: "This plugin is part of Hearth OS. Get Hearth OS at hearthos.fun" } : { ok: true }),
  get_shortcuts: () => ({}), set_shortcuts: (m) => m, get_applied: () => ({}), set_applied: (a) => a, get_art: () => null, art_error: () => null, report: () => null,
};
export function callable(name) { return async (...a) => (handlers[name] ? handlers[name](...a) : null); }
export function definePlugin(f) { return f; }
// Enough of Steam's frontend for Sync now to start (the picture of it syncing).
window.SteamClient = { Apps: Object.fromEntries(["AddShortcut", "RemoveShortcut", "SetShortcutName", "SetShortcutExe", "SetShortcutStartDir",
  "SetAppLaunchOptions", "SetCustomArtworkForApp", "SetShortcutIcon"].map((n) => [n, async () => 1])) };
window.appStore = { GetAppOverviewByAppID: () => null };
