// What the takeover asks Hearth (main.py → `nimbus --steam-mode-data`, hearth-core steam_takeover.rs), kept a
// little while so moving between pages is instant, and Hearth's pictures as addresses Steam's window can load.
import { callable } from "@decky/api";
import { useEffect, useState } from "react";
import { Settings, normalize } from "./model";

export type Answer<T> = T & { error?: string };

const data = callable<[what: string, arg: string], any>("hearth_data");
const picture = callable<[ref: string], string>("get_picture");
const getSettingsRaw = callable<[], unknown>("get_settings");
const setSettingsRaw = callable<[settings: Settings], unknown>("set_settings");
export const logError = callable<[part: string, message: string], void>("log_error");
export const getErrors = callable<[], string[]>("get_errors");

export async function getSettings(): Promise<Settings> { return normalize(await getSettingsRaw().catch(() => null)); }
export async function setSettings(s: Settings): Promise<Settings> { return normalize(await setSettingsRaw(s)); }

const kept = new Map<string, { at: number; v: any }>();
/** One of Hearth's answers; `maxAge` ms old at most (0: always ask). Never throws: {error} instead. */
export async function ask<T = any>(what: string, arg = "", maxAge = 30000): Promise<Answer<T>> {
  const key = what + "\u0000" + arg;
  const k = kept.get(key);
  if (k && Date.now() - k.at < maxAge) return k.v;
  let v: any;
  try { v = await data(what, arg); } catch (e) { v = { error: String(e) }; }
  if (!v || typeof v !== "object") v = { error: "Hearth gave no answer" };
  if (!v.error) kept.set(key, { at: Date.now(), v });
  return v;
}

/** An answer for a page: null while it's asked, then the answer (or {error}); `every` ms: asked again while shown. */
export function useAsk<T = any>(what: string, arg = "", maxAge = 30000, every = 0): Answer<T> | null {
  const [v, setV] = useState<Answer<T> | null>(() => {
    const k = kept.get(what + "\u0000" + arg);
    return k ? k.v : null;
  });
  useEffect(() => {
    let live = true;
    const go = (age: number) => ask<T>(what, arg, age).then((r) => { if (live) setV(r); });
    go(maxAge);
    const t = every ? setInterval(() => go(0), every) : undefined;
    return () => { live = false; if (t) clearInterval(t); };
  }, [what, arg]);
  return v;
}

// Pictures: web pictures load as they are; files on this device come as data: addresses (Steam's window can't
// open files), read once each.
const pics = new Map<string, string>();
const waiting = new Map<string, Promise<string>>();
export function picNow(ref: string | null | undefined): string {
  if (!ref) return "";
  if (/^https:\/\//.test(ref)) return ref;
  return pics.get(ref) || "";
}
export function loadPic(ref: string): Promise<string> {
  if (!ref || /^https:\/\//.test(ref)) return Promise.resolve(ref || "");
  const have = pics.get(ref);
  if (have !== undefined) return Promise.resolve(have);
  let p = waiting.get(ref);
  if (!p) {
    p = picture(ref).catch(() => "").then((u) => { pics.set(ref, u || ""); waiting.delete(ref); return u || ""; });
    waiting.set(ref, p);
  }
  return p;
}
/** A picture's address for an <img> or background, filled in once it's read. */
export function usePic(ref: string | null | undefined): string {
  const [u, setU] = useState(() => picNow(ref));
  useEffect(() => {
    let live = true;
    if (ref && !picNow(ref)) loadPic(ref).then((x) => { if (live) setU(x); });
    else setU(picNow(ref));
    return () => { live = false; };
  }, [ref]);
  return u;
}
