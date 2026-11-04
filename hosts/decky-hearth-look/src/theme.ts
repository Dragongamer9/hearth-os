// Stage 1, the skin: Hearth's look as one <style> in Steam's UI windows (the gamepad UI's main window and the ones
// Steam keeps for menus and the keyboard), refreshed every few seconds for windows Steam opens later, and taken out of
// every window at once by removeSkin(). Class names come from @decky/ui's class maps (or findClassModule), never
// hard-coded hashes.
import * as DUI from "@decky/ui";
import { menuIcon } from "./model";
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

// ---- the menu bars' small helpers (only while their part of the skin is in; each returns its undo)

/** Gives each of Steam's main menu entries Hearth's icon for it (data-hearth-icon, from its label; the skin draws it).
 *  Watches for the menu opening (Steam draws it when the Steam button is pressed). Only an attribute is added: the
 *  entries stay Steam's own. The undo stops watching and takes every attribute away again. */
export function startMenuDecor(itemClass: string): () => void {
  const watched: { doc: Document; obs: MutationObserver }[] = [];
  const tagAll = (doc: Document) => {
    try {
      doc.querySelectorAll("." + itemClass).forEach((el) => {
        const ic = menuIcon(el.textContent || "");
        if (el.getAttribute("data-hearth-icon") !== ic) el.setAttribute("data-hearth-icon", ic);
      });
    } catch { /* a window that went away */ }
  };
  const watch = () => {
    for (const w of steamWindows()) {
      const doc = w.document;
      if (!doc?.body || watched.some((x) => x.doc === doc)) continue;
      let queued = false;
      const obs = new MutationObserver(() => {
        if (queued) return;
        queued = true;
        const run = () => { queued = false; tagAll(doc); };
        try { w.requestAnimationFrame(run); } catch { setTimeout(run, 16); }
      });
      try { obs.observe(doc.body, { childList: true, subtree: true }); watched.push({ doc, obs }); tagAll(doc); } catch { /* not watchable */ }
    }
  };
  watch();
  const t = setInterval(watch, 4000);
  return () => {
    clearInterval(t);
    for (const { doc, obs } of watched) {
      try { obs.disconnect(); doc.querySelectorAll("[data-hearth-icon]").forEach((el) => el.removeAttribute("data-hearth-icon")); } catch { /* gone */ }
    }
    watched.length = 0;
  };
}

/** Whether Steam is running a game now (Router.MainRunningApp, as Hearth Music reads it); null when unreadable. */
export function gameRunning(): boolean | null {
  try {
    const r = (DUI as any).Router ?? (window as any).SteamUIStore;
    if (!r) return null;
    const a = r.MainRunningApp ?? (window as any).SteamUIStore?.MainRunningApp;
    return !!(a && Number(a.appid) > 0);
  } catch { return null; }
}

/** Keeps html.hearth-ingame on Steam's windows while a game runs (the menu is then Hearth's top bar). */
export function startIngameWatch(): () => void {
  const set = () => {
    const on = gameRunning() === true;
    for (const w of steamWindows()) { try { w.document.documentElement.classList.toggle("hearth-ingame", on); } catch { /* gone */ } }
  };
  set();
  const t = setInterval(set, 1000);
  return () => {
    clearInterval(t);
    for (const w of steamWindows()) { try { w.document.documentElement.classList.remove("hearth-ingame"); } catch { /* gone */ } }
  };
}
