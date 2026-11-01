// What the takeover asks Hearth (main.py → `nimbus --steam-mode-data`, hearth-core steam_takeover.rs), kept a
// little while so moving between pages is instant, and Hearth's pictures as addresses Steam's window can load.
import { callable } from "@decky/api";
import { useEffect, useState } from "react";
import { Attempt, Settings, artAttempts, normalize, refKind } from "./model";

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

// Pictures: web and data: addresses load as they are; everything else (files on this device, Hearth's nimbus-art://
// addresses, Steam's library cache, and a web picture that didn't load straight) is read by the backend and comes back
// as a data: address with the right type (Steam's window can't open files), read once each.
const pics = new Map<string, string>();
const waiting = new Map<string, Promise<string>>();
const artError = callable<[where: string, reason: string], void>("art_error");
/** A picture that didn't show, for the plugin's log: where and what kind (never the address). */
export function noteArtFailure(where: string, a: Attempt | null, why: string) {
  try { artError(where, `${a ? refKind(a.ref) + "/" + a.via : "none"} ${why}`).catch(() => {}); } catch { /* logging never breaks a page */ }
}
export function picNow(ref: string | null | undefined): string {
  if (!ref) return "";
  if (/^(https:\/\/|data:image\/)/i.test(ref)) return ref;
  return pics.get(ref) || "";
}
export function loadPic(ref: string, viaBackend = false): Promise<string> {
  if (!ref) return Promise.resolve("");
  if (!viaBackend && /^(https:\/\/|data:image\/)/i.test(ref)) return Promise.resolve(ref);
  const have = pics.get(ref);
  if (have !== undefined) return Promise.resolve(have);
  let p = waiting.get(ref);
  if (!p) {
    p = picture(ref).catch(() => "").then((u) => { const v = typeof u === "string" ? u : ""; pics.set(ref, v); waiting.delete(ref); return v; });
    waiting.set(ref, p);
  }
  return p;
}
/** Whether Steam's window can show an address (it's loaded once off screen first). */
const shown = new Map<string, Promise<boolean>>();
function canShow(url: string): Promise<boolean> {
  let p = shown.get(url);
  if (!p) {
    p = new Promise<boolean>((res) => {
      try {
        const im = new Image();
        const t = setTimeout(() => res(false), 15000);
        im.onload = () => { clearTimeout(t); res(im.naturalWidth > 0); };
        im.onerror = () => { clearTimeout(t); res(false); };
        im.src = url;
      } catch { res(false); }
    });
    shown.set(url, p);
  }
  return p;
}
/** The first of a card's pictures (artCandidates) that really shows, tried in order (artAttempts); "" when none does
 *  (the card then shows the title). `failed()` moves on when the shown <img> errors after all. */
export function useArt(refs: string[], where: string): { src: string; failed: () => void } {
  const key = refs.join("\u0000");
  const [state, setState] = useState<{ key: string; i: number; src: string }>({ key, i: 0, src: "" });
  const attempts = artAttempts(refs);
  const i = state.key === key ? state.i : 0;
  useEffect(() => {
    let live = true;
    (async () => {
      for (let n = i; n < attempts.length; n++) {
        const a = attempts[n];
        const url = await loadPic(a.ref, a.via === "backend");
        if (!live) return;
        if (url && await canShow(url)) { if (live) setState({ key, i: n, src: url }); return; }
        if (!live) return;
        // The backend logs its own reasons; a picture the window wouldn't show is logged here.
        if (url) noteArtFailure(where, a, "didn't load");
      }
      if (live) setState({ key, i: attempts.length, src: "" });
    })();
    return () => { live = false; };
  }, [key, i]);
  const src = state.key === key ? state.src : "";
  return { src, failed: () => { noteArtFailure(where, attempts[i] || null, "img error"); setState({ key, i: i + 1, src: "" }); } };
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
