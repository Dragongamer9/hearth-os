// Hearth's games as non-Steam shortcuts in Steam's library (Steam's own frontend calls, as other Decky plugins use
// them): each runs `nimbus --play-now <id>`, so the game starts with Hearth's launch logic and Steam tracks it.
import { callable } from "@decky/api";

export interface Item {
  id: string; title: string; source: string; store: string;
  cover: string; wide: string; icon: string;
  exe: string; startDir: string; options: string;
}
interface Art { data: string; type: "png" | "jpg" }

const getArt = callable<[ref: string], Art | null>("get_art");
const getMap = callable<[], Record<string, number>>("get_shortcuts");
const setMap = callable<[mapping: Record<string, number>], Record<string, number>>("set_shortcuts");

// ELibraryAssetType: Capsule 0, Hero 1, Logo 2, Header 3, Icon 4.
const CAPSULE = 0, HERO = 1, HEADER = 3;

const steam = () => (window as any).SteamClient;
const apps = () => (window as any).appStore;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const quote = (s: string) => `"${s}"`;

function exists(appId: number | undefined): boolean {
  if (!appId) return false;
  try { return !!apps()?.GetAppOverviewByAppID(appId); } catch { return false; }
}

async function setArt(appId: number, ref: string, types: number[]) {
  if (!ref) return;
  const art = await getArt(ref).catch(() => null);
  if (!art) return;
  for (const t of types) {
    try { await steam().Apps.SetCustomArtworkForApp(appId, art.data, art.type, t); } catch { /* Steam refused this picture */ }
  }
}

/** The shortcut for `item`, made or brought up to date. Returns Steam's app id. */
export async function ensure(item: Item, map: Record<string, number>, withArt: boolean): Promise<number> {
  const s = steam();
  if (!s?.Apps?.AddShortcut) throw new Error("Steam's library can't be changed from here (only in Steam Game Mode)");
  let appId = map[item.id];
  const fresh = !exists(appId);
  if (fresh) {
    appId = await s.Apps.AddShortcut(item.title, item.exe, item.startDir, item.options);
    map[item.id] = appId;
    await sleep(400);   // Steam names a new shortcut after its program at first
  }
  s.Apps.SetShortcutName(appId, item.title);
  s.Apps.SetShortcutExe(appId, quote(item.exe));
  s.Apps.SetShortcutStartDir(appId, quote(item.startDir));
  s.Apps.SetAppLaunchOptions(appId, item.options);
  if (withArt && fresh) {
    await setArt(appId, item.cover, [CAPSULE]);
    await setArt(appId, item.wide, [HERO, HEADER]);
    if (item.icon && item.icon.startsWith("/")) { try { s.Apps.SetShortcutIcon(appId, item.icon); } catch { /* not a picture Steam takes */ } }
  }
  return appId;
}

/** Every Hearth game and link in Steam's library; Hearth's shortcuts for games that are gone are removed. */
export async function sync(items: Item[], onStep?: (done: number, total: number) => void): Promise<{ added: number; removed: number }> {
  const map = { ...(await getMap()) };
  const before = new Set(Object.keys(map).filter((k) => exists(map[k])));
  let added = 0, removed = 0, done = 0;
  for (const it of items) {
    const had = before.has(it.id);
    try { await ensure(it, map, true); if (!had) added++; } catch (e) { console.warn("Hearth: couldn't add", it.title, e); }
    onStep?.(++done, items.length);
  }
  const keep = new Set(items.map((i) => i.id));
  for (const id of Object.keys(map)) {
    if (keep.has(id)) continue;
    if (exists(map[id])) { try { steam().Apps.RemoveShortcut(map[id]); removed++; } catch { /* already gone */ } }
    delete map[id];
  }
  await setMap(map);
  return { added, removed };
}

/** Removes every shortcut the Hearth plugin made (nothing else in Steam's library). */
export async function removeAll(): Promise<number> {
  const map = await getMap();
  let n = 0;
  for (const id of Object.keys(map)) {
    if (exists(map[id])) { try { steam().Apps.RemoveShortcut(map[id]); n++; } catch { /* gone */ } }
  }
  await setMap({});
  return n;
}

/** Plays through Steam (so the game gets focus, the overlay and Steam's play time): the shortcut is made if needed. */
export async function play(item: Item): Promise<void> {
  const map = { ...(await getMap()) };
  const had = exists(map[item.id]);
  const appId = await ensure(item, map, !had);
  if (!had) await setMap(map);
  const overview = apps()?.GetAppOverviewByAppID(appId);
  const gameId = String(overview?.m_gameid ?? appId);
  steam().Apps.RunGame(gameId, "", -1, 100);
}
