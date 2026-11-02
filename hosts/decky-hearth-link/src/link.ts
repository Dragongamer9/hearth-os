// Hearth Link's backend calls (main.py) and the one Steam call the panel makes, guarded.
import { callable } from "@decky/api";
import { Answer, Game, Health } from "./model";

export interface Result<T = any> { ok: boolean; data?: T; error?: string; running?: boolean }

export const getStatus = callable<[], Answer>("status");
export const linkCall = callable<[cmd: string, args: Record<string, unknown>], Result>("call");
export const getStorage = callable<[], any>("storage");
export const startLink = callable<[], Result>("start_link");

export async function games(peer?: string, refresh = false): Promise<Game[]> {
  const r = peer ? await linkCall("games", { peer, refresh }) : await linkCall("local.games", {});
  if (!r.ok) throw new Error(r.error || "That didn't work");
  return (r.data?.games || []) as Game[];
}

export type StorageInfo = {
  running: boolean; error?: string;
  cache: { error?: string; enabled: boolean; using: string; jobs: any[]; available: { game: string; title: string; size: number }[]; updates: { game: string; title: string }[] };
  saves: { error?: string; follow: boolean; server: boolean; games: { title: string; at: number; conflict: boolean; pending: boolean }[] };
  updates: { error?: string; enabled: boolean; fetching: boolean; using: string; cached: { version: string; at: number; verified: number }[] };
  health: Health;
};

/** Steam's own restart (Steam reads a copied game's appmanifest when it starts). Guarded: when Steam has no such call
 *  any more, the panel says how to restart it instead. */
export function steamCanRestart(): boolean {
  const u = (window as any).SteamClient?.User;
  return !!u && typeof u.StartRestart === "function";
}
export function restartSteam(): boolean {
  if (!steamCanRestart()) return false;
  try { (window as any).SteamClient.User.StartRestart(false); return true; } catch { return false; }
}
