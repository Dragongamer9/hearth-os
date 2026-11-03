// What Hearth Music reads from Steam's frontend, every read guarded: the page Steam's gamepad UI shows (its router's
// location), the game highlighted in the library (the focused tile's React props), the running game
// (Router.MainRunningApp, as the older Hearth plugin read it), a game's name and collections (appStore,
// collectionStore, as Hearth Sync uses them), and Play being pressed (SteamClient.Apps.RegisterForGameActionStart).
// If the page and the running game can't be read at all, Steam changed: Hearth Music turns itself off with a note.
import * as DUI from "@decky/ui";

const W = () => window as any;
const router = () => (DUI as any).Router ?? W().SteamUIStore;
const mainWindow = () => router()?.WindowStore?.GamepadUIMainWindowInstance ?? W().SteamUIStore?.WindowStore?.GamepadUIMainWindowInstance;

/** The gamepad UI's current path ("/library/app/570"), or null when it can't be read. */
export function readPath(): string | null {
  try {
    const loc = mainWindow()?.m_history?.location ?? mainWindow()?.m_History?.location;
    return typeof loc?.pathname === "string" ? loc.pathname : null;
  } catch { return null; }
}

/** The running game's app id, 0 when none, null when Steam's state can't be read. */
export function readRunning(): number | null {
  try {
    const r = router();
    if (!r) return null;
    const a = r.MainRunningApp ?? W().SteamUIStore?.MainRunningApp;
    const id = Number(a?.appid || 0);
    return Number.isFinite(id) && id > 0 ? id : 0;
  } catch { return null; }
}

/** Why Hearth Music can't work with this Steam, or "" when it can. */
export function steamMissing(): string {
  if (readPath() === null && readRunning() === null) return "Steam changed (its page and running game can't be read): Hearth Music turned itself off.";
  if (readPath() === null) return "Steam changed (the page it shows can't be read): Hearth Music turned itself off.";
  return "";
}

export function gameTitle(appid: number): string {
  try { return String(W().appStore?.GetAppOverviewByAppID(appid)?.display_name || ""); } catch { return ""; }
}

export function collectionsOf(appid: number): string[] {
  try {
    const cs = W().collectionStore?.userCollections;
    if (!Array.isArray(cs)) return [];
    return cs.filter((c: any) => c?.apps?.has?.(appid)).map((c: any) => String(c.displayName || "")).filter(Boolean).slice(0, 20);
  } catch { return []; }
}

/** The app id a library tile (or something inside it) stands for, from its React props; 0 when none. */
export function appIdOf(el: any): number {
  try {
    for (let e = el, d = 0; e && d < 10; e = e.parentElement, d++) {
      const fk = Object.keys(e).find((k) => k.startsWith("__reactFiber$") || k.startsWith("__reactInternalInstance$"));
      for (let f = fk ? e[fk] : null, n = 0; f && n < 12; f = f.return, n++) {
        const p = f.memoizedProps;
        const id = Number(p?.app?.appid ?? p?.appOverview?.appid ?? p?.overview?.appid ?? p?.appid ?? 0);
        if (Number.isFinite(id) && id > 0) return id;
      }
    }
  } catch { /* not a tile */ }
  return 0;
}

/** Calls `cb(appid)` when the highlighted element in Steam's gamepad UI changes. Returns the way to stop. */
export function watchFocus(cb: (appid: number) => void): (() => void) | null {
  let doc: Document | null = null;
  try { doc = mainWindow()?.BrowserWindow?.document ?? (DUI as any).findSP?.()?.document ?? null; } catch { doc = null; }
  if (!doc) return null;
  const on = (e: Event) => cb(appIdOf(e.target));
  doc.addEventListener("focusin", on, true);
  return () => { try { doc!.removeEventListener("focusin", on, true); } catch { /* gone */ } };
}

/** Calls `cb()` the moment Play is pressed on any game. Returns the way to stop. */
export function watchPlay(cb: () => void): () => void {
  try {
    const r = W().SteamClient?.Apps?.RegisterForGameActionStart?.(() => cb());
    return () => { try { r?.unregister?.(); } catch { /* gone */ } };
  } catch { return () => {}; }
}

/** Every game in Steam's library, for the library scan: Steam games and non-Steam shortcuts (Hearth Sync's store
 *  games among them) with their names, from collectionStore's "all games" and appStore, every read guarded. */
export function libraryGames(): { appid: number; title: string; shortcut: boolean }[] {
  const out = new Map<number, { appid: number; title: string; shortcut: boolean }>();
  const add = (a: any) => {
    try {
      const id = Number(a?.appid || 0), title = String(a?.display_name || "");
      // app_type 1 is a game, 1073741824 a non-Steam shortcut.
      const type = Number(a?.app_type ?? 1);
      if (!(id > 0) || !title || out.has(id) || (type !== 1 && type !== 1073741824)) return;
      out.set(id, { appid: id, title: title.slice(0, 200), shortcut: type === 1073741824 || id >= 2 ** 31 });
    } catch { /* not an app */ }
  };
  const each = (list: any) => {
    try {
      if (!list) return;
      for (const a of (typeof list.values === "function" && !Array.isArray(list) ? list.values() : list)) add(a);
    } catch { /* not a list */ }
  };
  try {
    const cs = W().collectionStore;
    each(cs?.allGamesCollection?.allApps);
    each(cs?.GetCollection?.("type-games")?.allApps);
    each(cs?.GetCollection?.("type-nonsteam")?.allApps);
  } catch { /* no collectionStore */ }
  try { each(W().appStore?.allApps); } catch { /* no appStore */ }
  return [...out.values()].slice(0, 20000);
}
