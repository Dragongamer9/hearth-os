// CI only: a stand-in for Steam's state that Hearth Music reads (src/steam.ts), per scene (window.HM_STATE).
export const S = () => window.HM_STATE || "apple";
const sceneApp = { apple: 1245620, soundtrack: 413150, pick: 292030, game: 620, nomatch: 1091500, running: 1091500, off: 1245620, changed: 0, library: 1245620,
  deezer: 367520, scan: 1091500, choose: 1091500 };
const GAMES = { 1091500: "Cyberpunk 2077", 292030: "The Witcher 3: Wild Hunt", 1245620: "ELDEN RING", 413150: "Stardew Valley", 620: "Portal 2", 367520: "Hollow Knight" };
// Steam's state (steam.ts reads these).
const path = () => (S() === "library" ? "/library/home" : `/library/app/${sceneApp[S()]}`);
window.HM_ROUTER = S() === "changed" ? undefined : {
  get MainRunningApp() { return S() === "running" ? { appid: 1091500, display_name: "Cyberpunk 2077" } : undefined; },
  WindowStore: { GamepadUIMainWindowInstance: { m_history: { location: { get pathname() { return path(); } } }, BrowserWindow: window } },
};
if (S() === "changed") window.SteamUIStore = undefined;
window.appStore = { GetAppOverviewByAppID: (id) => (GAMES[id] ? { appid: id, display_name: GAMES[id] } : null),
  allApps: Object.keys(GAMES).map((id) => ({ appid: Number(id), display_name: GAMES[id], app_type: 1 })) };
window.collectionStore = { userCollections: [{ displayName: "RPGs", apps: new Set([292030, 1245620, 1091500]) }, { displayName: "Favorites", apps: new Set([1245620]) }] };
window.SteamClient = { Apps: { RegisterForGameActionStart: () => ({ unregister() {} }) } };

