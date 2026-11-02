// Hearth Sync's Steam side: non-Steam shortcuts, their artwork and per-store collections, through the same Steam
// frontend calls other Decky plugins use (SteamClient.Apps, appStore, collectionStore). Every call is guarded: if
// Steam's calls are missing or start throwing the way a changed Steam would, syncing turns itself off with a note
// (until the next start or "Sync now") instead of breaking Steam. Nothing of Steam's screens is changed.
import { callable } from "@decky/api";
import { ART, Applied, Item, Status, groups, missingArt, plan, shouldAct, storeOn, targetKey, SHORTCUT_TYPE } from "./model";

interface Games { ok: boolean; error: string | null; games: Item[]; signature: string }
interface Art { data: string; type: "png" | "jpg" }
export interface Settings { stores: Record<string, boolean> }

const getGames = callable<[force: boolean], Games>("get_games");
const getArt = callable<[ref: string], Art | null>("get_art");
const artError = callable<[reason: string], void>("art_error");
const getMap = callable<[], Record<string, number>>("get_shortcuts");
const setMap = callable<[m: Record<string, number>], Record<string, number>>("set_shortcuts");
const getApplied = callable<[], Record<string, Applied>>("get_applied");
const setApplied = callable<[a: Record<string, Applied>], Record<string, Applied>>("set_applied");
export const getSettings = callable<[], Settings>("get_settings");
export const setSettings = callable<[s: Settings], Settings>("set_settings");
export const getStatus = callable<[], Status>("get_status");
const setStatus = callable<[s: Status], Status>("set_status");
const report = callable<[line: string], void>("report");

const steam = () => (window as any).SteamClient;
const apps = () => (window as any).appStore;
const colls = () => (window as any).collectionStore;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const quote = (s: string) => `"${s}"`;

/** Thrown when Steam no longer works the way Hearth Sync expects. */
class SteamChanged extends Error {}

const NEEDED = ["AddShortcut", "RemoveShortcut", "SetShortcutName", "SetShortcutExe", "SetShortcutStartDir", "SetAppLaunchOptions", "SetCustomArtworkForApp"];

/** Why Steam can't be synced with from here, or "" when it can. */
export function steamMissing(): string {
  try {
    const a = steam()?.Apps;
    if (!a) return "Steam's library can't be changed from here (only in Steam Game Mode).";
    const gone = NEEDED.filter((n) => typeof a[n] !== "function");
    if (gone.length) return `Steam changed (${gone.join(", ")} missing): Hearth Sync turned itself off.`;
    if (typeof apps()?.GetAppOverviewByAppID !== "function") return "Steam changed (its app list can't be read): Hearth Sync turned itself off.";
    return "";
  } catch { return "Steam couldn't be read: Hearth Sync turned itself off."; }
}

/** One Steam call. A TypeError (a call that's gone or changed shape) means Steam changed: syncing stops. */
async function call<T>(what: string, f: () => T | Promise<T>): Promise<T> {
  try { return await f(); } catch (e) {
    if (e instanceof TypeError) throw new SteamChanged(`Steam changed (${what} failed): Hearth Sync turned itself off.`);
    throw e;
  }
}

/** Whether `appId` is still a non-Steam shortcut in Steam (never true for one of Steam's own games). */
export function exists(appId: number | undefined): boolean {
  if (!appId) return false;
  try {
    const o = apps()?.GetAppOverviewByAppID(appId);
    return !!o && (o.app_type === undefined || o.app_type === SHORTCUT_TYPE);
  } catch { return false; }
}

async function setTarget(appId: number, it: Item) {
  const a = steam().Apps;
  await call("SetShortcutName", () => a.SetShortcutName(appId, it.title));
  await call("SetShortcutExe", () => a.SetShortcutExe(appId, quote(it.exe)));
  await call("SetShortcutStartDir", () => a.SetShortcutStartDir(appId, quote(it.startDir)));
  await call("SetAppLaunchOptions", () => a.SetAppLaunchOptions(appId, it.options));
}

/** Gives Steam the pictures in `kinds`; returns the ones that were set. Failures are counted in the backend's log. */
async function setArt(appId: number, it: Item, kinds: string[]): Promise<string[]> {
  const done: string[] = [];
  const cache = new Map<string, Art | null>();
  for (const [k, type] of ART) {
    if (!kinds.includes(k)) continue;
    const ref = it[k];
    if (!cache.has(ref)) cache.set(ref, await getArt(ref).catch(() => null));
    const art = cache.get(ref);
    if (!art) continue;   // the backend logged why
    try {
      await call("SetCustomArtworkForApp", () => steam().Apps.SetCustomArtworkForApp(appId, art.data, art.type, type));
      done.push(k);
    } catch (e) {
      if (e instanceof SteamChanged) throw e;
      artError(`${k} refused`).catch(() => {});
    }
  }
  return done;
}

/** Puts each synced game in a collection named after its store. Skipped silently if Steam's collections can't be
 *  reached the usual way. Never removes anything from a collection, never deletes one. */
async function collect(byStore: Record<string, number[]>): Promise<string> {
  try {
    const cs = colls();
    if (!cs || !Array.isArray(cs.userCollections) || typeof cs.NewUnsavedCollection !== "function") return "unavailable";
    let n = 0;
    for (const [name, ids] of Object.entries(byStore)) {
      const overviews = ids.map((id) => apps().GetAppOverviewByAppID(id)).filter(Boolean);
      if (!overviews.length) continue;
      const c = cs.userCollections.find((x: any) => x?.displayName === name);
      if (!c) {
        const made = cs.NewUnsavedCollection(name, undefined, overviews);
        if (made?.Save) { await made.Save(); n++; }
        continue;
      }
      const missing = overviews.filter((o: any) => !(c.apps?.has?.(o.appid)));
      if (missing.length && c.AsDragDropCollection) { c.AsDragDropCollection().AddApps(missing); await c.Save(); n++; }
    }
    return `${n} changed`;
  } catch { return "skipped (Steam's collections didn't take it)"; }
}

let busy = false;
let offNote = "";   // set when Steam changed: syncing stays off until the next start or Sync now
const listeners = new Set<() => void>();
export const onChange = (f: () => void) => { listeners.add(f); return () => listeners.delete(f); };
const notify = () => listeners.forEach((f) => { try { f(); } catch { /* gone */ } });
export const isBusy = () => busy;

/** One sync. `force` (start, Sync now) re-reads Hearth's library and acts; the timer acts only when it changed or
 *  missing pictures are due another try. Returns the new status. */
export async function runSync(force: boolean): Promise<Status | null> {
  if (busy) return null;
  if (!force && offNote) return null;
  busy = true;
  notify();
  const st: Status = await getStatus().catch(() => null as any) || { lastSync: 0, lastArtTry: 0, synced: 0, artMissing: 0, signature: "", problems: [], off: "" };
  try {
    offNote = steamMissing();
    if (offNote) { st.off = offNote; await setStatus(st); report(offNote).catch(() => {}); return st; }
    st.off = "";
    const g = await getGames(force);
    if (!g?.ok) {
      st.problems = [`Hearth's library couldn't be read: ${g?.error || "no answer"}`];
      await setStatus(st);
      return st;
    }
    const now = Date.now();
    if (!force && !shouldAct(g.signature, st, now)) return st;
    const [map, applied, settings] = await Promise.all([getMap(), getApplied(), getSettings()]);
    const stores = settings?.stores || {};
    const p = plan(g.games, map, applied, stores, exists);
    const problems: string[] = [];
    let added = 0, updated = 0, removed = 0, artSet = 0, failed = 0;
    const a = steam().Apps;
    for (const it of p.add) {
      try {
        const appId = await call("AddShortcut", () => a.AddShortcut(it.title, it.exe, it.startDir, it.options));
        if (!appId || typeof appId !== "number") { failed++; continue; }
        map[it.id] = appId;
        await sleep(400);   // Steam names a new shortcut after its program at first
        await setTarget(appId, it);
        applied[it.id] = { t: targetKey(it), art: {} };
        added++;
      } catch (e) { if (e instanceof SteamChanged) throw e; failed++; }
    }
    for (const it of p.update) {
      try { await setTarget(map[it.id], it); applied[it.id] = { ...(applied[it.id] || { art: {} }), t: targetKey(it) }; updated++; }
      catch (e) { if (e instanceof SteamChanged) throw e; failed++; }
    }
    for (const { item, kinds } of p.art) {
      const appId = map[item.id];
      if (!exists(appId)) continue;
      const done = await setArt(appId, item, kinds);
      const rec = applied[item.id] || { t: targetKey(item), art: {} };
      for (const k of done) (rec.art as any)[k] = item[k as keyof Item];
      applied[item.id] = rec;
      artSet += done.length;
      if (item.icon && item.icon.startsWith("/") && p.add.includes(item)) {
        try { a.SetShortcutIcon?.(appId, item.icon); } catch { /* not a picture Steam takes */ }
      }
    }
    for (const id of p.remove) {
      // Only Hearth Sync's own shortcuts are ever in `map`; exists() checked it's still a non-Steam shortcut.
      try { await call("RemoveShortcut", () => a.RemoveShortcut(map[id])); removed++; delete map[id]; delete applied[id]; }
      catch (e) { if (e instanceof SteamChanged) throw e; failed++; }
    }
    for (const id of p.forget) { delete map[id]; delete applied[id]; }
    await setMap(map);
    await setApplied(applied);
    const cols = await collect(groups(g.games, map, stores));
    const synced = Object.keys(map).filter((id) => exists(map[id])).length;
    const missing = missingArt(g.games, map, applied, stores);
    if (failed) problems.push(`${failed} change${failed === 1 ? "" : "s"} Steam didn't take (tried again next sync)`);
    if (missing) problems.push(`${missing} game${missing === 1 ? "" : "s"} still missing a picture (tried again later; see the log)`);
    const skipped = g.games.filter((i) => !storeOn(stores, i.store)).length;
    Object.assign(st, { lastSync: now, synced, signature: failed ? "" : g.signature, problems, artMissing: missing });
    if (p.art.length) st.lastArtTry = now;
    await setStatus(st);
    report(`sync: ${synced} in Steam; added ${added}, updated ${updated}, removed ${removed}, pictures set ${artSet}, still missing art ${missing}, failed ${failed}, turned off by store ${skipped}; collections ${cols}`).catch(() => {});
    return st;
  } catch (e) {
    const note = e instanceof SteamChanged ? e.message : `Sync stopped: ${String((e as any)?.message || e).slice(0, 120)}`;
    if (e instanceof SteamChanged) offNote = note;
    st.off = e instanceof SteamChanged ? note : "";
    st.problems = e instanceof SteamChanged ? [] : [note];
    await setStatus(st).catch(() => {});
    report(note).catch(() => {});
    return st;
  } finally {
    busy = false;
    notify();
  }
}

/** Removes every shortcut Hearth Sync made (nothing else in Steam's library). Returns how many. */
export async function removeAll(): Promise<number> {
  const map = await getMap();
  let n = 0;
  for (const id of Object.keys(map)) {
    if (!exists(map[id])) continue;
    try { steam().Apps.RemoveShortcut(map[id]); n++; } catch { /* gone */ }
  }
  await setMap({});
  await setApplied({});
  const st = await getStatus();
  await setStatus({ ...st, synced: 0, signature: "", artMissing: 0, problems: [] });
  report(`removed all ${n} Hearth Sync shortcuts (from Quick Access)`).catch(() => {});
  notify();
  return n;
}

/** The games Hearth's library has (for the store switches), without syncing. */
export async function listGames(): Promise<Item[]> {
  const g = await getGames(false).catch(() => null);
  return g?.ok ? g.games : [];
}
