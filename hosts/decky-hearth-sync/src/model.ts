// Hearth Sync's pure parts (no Steam, no Decky): what to add, update, remove and which pictures to (re)try. Unit-tested
// in CI with Node's type stripping (tests/model.test.mjs), so plain TypeScript only (no enums).

export interface Item {
  id: string; title: string; source: string; store: string; storeName: string; cloud: boolean;
  cover: string; wide: string; hero: string; logo: string; icon: string;
  exe: string; startDir: string; options: string;
}
export type Kind = "cover" | "wide" | "hero" | "logo";
export interface Applied { t: string; art: Partial<Record<Kind, string>> }
export interface Status {
  lastSync: number; lastArtTry: number; synced: number; artMissing: number; signature: string; problems: string[]; off: string;
}

// Steam's ELibraryAssetType: Capsule 0 (portrait cover), Hero 1, Logo 2, Header 3 (wide).
export const ART: [Kind, number][] = [["cover", 0], ["wide", 3], ["hero", 1], ["logo", 2]];
export const CHECK_EVERY = 5 * 60 * 1000;      // the timer's cheap check
export const ART_RETRY_EVERY = 30 * 60 * 1000;  // missing pictures are tried again at most this often
// Steam's app type for a non-Steam shortcut (appStore overviews' app_type).
export const SHORTCUT_TYPE = 1073741824;

export const targetKey = (i: Item): string => JSON.stringify([i.title, i.exe, i.startDir, i.options]);
export const storeOn = (stores: Record<string, boolean>, key: string): boolean => stores[key] !== false;

/** The pictures `it` has that Steam hasn't been given yet (all of them for a new shortcut). */
export function artToDo(it: Item, applied: Applied | undefined): Kind[] {
  return ART.map(([k]) => k).filter((k) => !!it[k] && applied?.art?.[k] !== it[k]);
}

export interface Plan { add: Item[]; update: Item[]; remove: string[]; forget: string[]; art: { item: Item; kinds: Kind[] }[] }

/** What a sync does. Only ids in `map` (Hearth Sync's own shortcuts) can be removed; `exists` says whether Steam still
 *  has that app id as a non-Steam shortcut (a shortcut removed by hand is just forgotten, then added again). */
export function plan(items: Item[], map: Record<string, number>, applied: Record<string, Applied>,
                     stores: Record<string, boolean>, exists: (appId: number) => boolean): Plan {
  const p: Plan = { add: [], update: [], remove: [], forget: [], art: [] };
  const wanted = items.filter((i) => storeOn(stores, i.store));
  const keep = new Set(wanted.map((i) => i.id));
  for (const it of wanted) {
    const has = map[it.id] !== undefined && exists(map[it.id]);
    if (!has) p.add.push(it);
    else if (applied[it.id]?.t !== targetKey(it)) p.update.push(it);
    const kinds = artToDo(it, has ? applied[it.id] : undefined);
    if (kinds.length) p.art.push({ item: it, kinds });
  }
  for (const id of Object.keys(map)) {
    if (keep.has(id)) continue;
    if (exists(map[id])) p.remove.push(id); else p.forget.push(id);
  }
  return p;
}

/** How many synced games still miss a picture Hearth has for them. */
export function missingArt(items: Item[], map: Record<string, number>, applied: Record<string, Applied>, stores: Record<string, boolean>): number {
  return items.filter((i) => map[i.id] !== undefined && storeOn(stores, i.store) && artToDo(i, applied[i.id]).length > 0).length;
}

/** Whether the timer's check should act: Hearth's library changed, or missing pictures are due another try. */
export function shouldAct(sig: string, st: Status, now: number): boolean {
  if (!sig) return false;
  if (sig !== st.signature) return true;
  return st.artMissing > 0 && now - st.lastArtTry >= ART_RETRY_EVERY;
}

/** Steam collection name -> app ids, for the synced games. */
export function groups(items: Item[], map: Record<string, number>, stores: Record<string, boolean>): Record<string, number[]> {
  const out: Record<string, number[]> = {};
  for (const it of items) {
    if (!storeOn(stores, it.store) || !map[it.id]) continue;
    (out[it.storeName] ||= []).push(map[it.id]);
  }
  return out;
}

/** The stores in Hearth's list, with how many games each has: [switch, name, count], A to Z by name. */
export function storeList(items: Item[]): [string, string, number][] {
  const m = new Map<string, [string, string, number]>();
  for (const it of items) {
    const e = m.get(it.store) || [it.store, it.storeName, 0];
    e[2]++;
    m.set(it.store, e);
  }
  return [...m.values()].sort((a, b) => a[1].localeCompare(b[1]));
}

/** "12 minutes ago" style for the panel. */
export function ago(then: number, now: number): string {
  if (!then) return "never";
  const s = Math.max(0, Math.round((now - then) / 1000));
  if (s < 60) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} minute${m === 1 ? "" : "s"} ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} hour${h === 1 ? "" : "s"} ago`;
  const d = Math.round(h / 24);
  return `${d} day${d === 1 ? "" : "s"} ago`;
}
