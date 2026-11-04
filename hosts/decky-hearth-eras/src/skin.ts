// The skin as one <style> in every one of Steam's UI windows (the gamepad UI's main window, and the popups Steam opens
// for Quick Access, its menu and the keyboard), refreshed every few seconds for windows Steam opens later and taken out
// of every window at once by removeSkin(). The way Hearth Classic did it (hosts/decky-hearth-look/src/theme.ts), which
// worked on a device. Only styles go in: no window is moved, resized or reshaped (Steam ignores that for its menu).
import * as DUI from "@decky/ui";
import type { Resolve } from "./classic/model";

const STYLE_ID = "hearth-eras";

/** Steam's class name for `key`: from @decky/ui's map called `map`, else from the class module findClassModule finds
 *  with both `map` and `key`. undefined: Steam hasn't got it (any more). */
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

function mainWindow(): Window {
  try { return (DUI as any).Router?.WindowStore?.GamepadUIMainWindowInstance?.BrowserWindow || window; } catch { return window; }
}

/** Steam's UI windows; every source guarded (what can't be read is left out). */
export function steamWindows(): Window[] {
  const found = new Set<Window>();
  const add = (w: any) => { try { if (w && w.document && typeof w.document.createElement === "function") found.add(w as Window); } catch { /* closed */ } };
  try { add((DUI as any).findSP?.()); } catch { /* not in this @decky/ui */ }
  try {
    const store = (DUI as any).Router?.WindowStore;
    add(store?.GamepadUIMainWindowInstance?.BrowserWindow);
    for (const w of store?.SteamUIWindows ?? []) add(w?.BrowserWindow ?? w?.m_BrowserWindow);
  } catch { /* Steam changed its window store */ }
  try {
    const pm = (window as any).g_PopupManager;
    const pops = typeof pm?.GetPopups === "function" ? pm.GetPopups() : pm?.m_mapPopups ? [...pm.m_mapPopups.values()] : [];
    for (const p of pops || []) add(p?.m_popup ?? p?.window);
  } catch { /* no popup manager */ }
  try {
    const trees = (DUI as any).getGamepadNavigationTrees?.() ?? [];
    for (const t of trees || []) add(t?.Root?.Element?.ownerDocument?.defaultView);
  } catch { /* no navigation trees */ }
  if (!found.size) add(window);
  return [...found];
}

export function windowNames(): string[] {
  return steamWindows().map((w) => { try { return String(w.document.title || "(untitled)").slice(0, 40); } catch { return "(closed)"; } });
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
  // Popups don't take the main window's page background (Hearth Classic's skin keys on this; 1.2.5 log).
  try { doc.documentElement.classList.toggle("hearth-popup", win !== mainWindow()); } catch { /* fine */ }
}

/** Puts the skin in. false when it couldn't be put in any of Steam's windows (the caller then pauses). */
export function applySkin(css: string): boolean {
  current = css;
  let ok = false;
  const run = () => { for (const w of steamWindows()) { try { put(w, current); ok = true; } catch { /* went away */ } } };
  run();
  if (!timer) timer = setInterval(run, 4000);
  return ok;
}

/** Takes the skin out of every window and stops refreshing it. Safe to call any number of times. */
export function removeSkin() {
  if (timer) { clearInterval(timer); timer = undefined; }
  current = "";
  for (const w of steamWindows()) {
    try { w.document.getElementById(STYLE_ID)?.remove(); w.document.documentElement.classList.remove("hearth-popup"); } catch { /* gone */ }
  }
}

/** Whether Steam is running a game now (Router.MainRunningApp); null when unreadable. */
export function gameRunning(): boolean | null {
  try {
    const r = (DUI as any).Router ?? (window as any).SteamUIStore;
    if (!r) return null;
    const a = r.MainRunningApp ?? (window as any).SteamUIStore?.MainRunningApp;
    return !!(a && Number(a.appid) > 0);
  } catch { return null; }
}
