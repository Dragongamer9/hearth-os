// Stage 1, the skin: Hearth's look as one <style> in Steam's UI windows (the gamepad UI's main window and the ones
// Steam keeps for menus and the keyboard), refreshed every few seconds for windows Steam opens later, and taken out of
// every window at once by removeSkin(). Class names come from @decky/ui's class maps (or findClassModule), never
// hard-coded hashes.
import * as DUI from "@decky/ui";
import type { Resolve } from "./model";

const STYLE_ID = "hearth-classic";

/** Steam's class name for `key`: from @decky/ui's map called `map` (gamepadTabbedPageClasses, ...), else from the
 *  class module findClassModule finds with both `map` and `key`. undefined: Steam hasn't got it (any more). */
export const steamClass: Resolve = (map, key) => {
  try {
    const known = (DUI as any)[map];
    if (known && typeof known === "object") return typeof known[key] === "string" ? known[key] : undefined;
    const fcm = (DUI as any).findClassModule as ((f: (m: any) => boolean) => any) | undefined;
    const m = fcm ? fcm((mod: any) => mod && typeof mod[map] === "string" && typeof mod[key] === "string") : undefined;
    return m && typeof m[key] === "string" ? m[key] : undefined;
  } catch {
    return undefined;
  }
};

/** Steam's UI windows: the gamepad UI's main window and the others Steam keeps (keyboard, overlays). */
export function steamWindows(): Window[] {
  const found = new Set<Window>();
  const add = (w: any) => { if (w && w.document && typeof w.document.createElement === "function") found.add(w as Window); };
  try { add((DUI as any).findSP?.()); } catch { /* not in this @decky/ui */ }
  try {
    const store = (DUI as any).Router?.WindowStore;
    add(store?.GamepadUIMainWindowInstance?.BrowserWindow);
    for (const w of store?.SteamUIWindows ?? []) add(w?.BrowserWindow ?? w?.m_BrowserWindow);
  } catch { /* Steam changed its window store: the main window below still gets it */ }
  if (!found.size) add(window);
  return [...found];
}

let timer: ReturnType<typeof setInterval> | undefined;
let current = "";

function put(win: Window, css: string) {
  const doc = win.document;
  let el = doc.getElementById(STYLE_ID) as HTMLStyleElement | null;
  if (!el) {
    el = doc.createElement("style");
    el.id = STYLE_ID;
    (doc.head || doc.documentElement).appendChild(el);
  }
  if (el.textContent !== css) el.textContent = css;
}

/** Puts the skin in. Returns false when it couldn't be put in Steam's main window (the caller then pauses). */
export function applySkin(css: string): boolean {
  current = css;
  let ok = false;
  const run = () => {
    for (const w of steamWindows()) { try { put(w, current); ok = true; } catch { /* a window that went away */ } }
  };
  run();
  if (!timer) timer = setInterval(run, 4000);
  return ok;
}

/** Takes the skin out of every window and stops refreshing it. Safe to call any number of times. */
export function removeSkin() {
  if (timer) { clearInterval(timer); timer = undefined; }
  current = "";
  for (const w of steamWindows()) {
    try { w.document.getElementById(STYLE_ID)?.remove(); } catch { /* gone */ }
  }
}
