// Stage 1, the skin: Hearth's look as one <style> in Steam's UI windows (the gamepad UI's main window, and the
// windows Steam opens for its main menu and Quick Access: CSS Loader calls them "MainMenu" and "QuickAccess"; they're
// popups of the main window, found through g_PopupManager and the gamepad navigation trees), refreshed every few seconds for windows Steam opens later, and taken out of
// every window at once by removeSkin(). Class names come from @decky/ui's class maps (or findClassModule), never
// hard-coded hashes.
import * as DUI from "@decky/ui";
import { judgeBar, judgeStructure, menuIcon } from "./model";
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

/** Steam's UI windows: the gamepad UI's main window, its other windows, and its popups (the main menu's and Quick
 *  Access's own windows among them). Every source guarded: what can't be read is just left out. */
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

/** The windows' titles, for the diagnostics ("SP", "MainMenu_uid2", "QuickAccess_uid2", ...). */
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

// ---- the menu as Hearth's bar (only while its part of the skin is in; returns its undo)
//
// Steam's main menu is drawn in its own window (CSS Loader's "MainMenu"): a Focusable with the id MainNavMenuContainer,
// its panel (.Panel) and the entries (.Panel > * > .Panel.Focusable), the structure CSS Loader themes style. The bar's
// rules only apply under html.hearth-bar, which is set on that window only when this structure is really there, and
// taken away again when, once the menu is on screen, the bar doesn't fit or the entries can't be reached: then the
// menu keeps Steam's layout with Hearth's colours (html.hearth-menu). Never a half-styled or unusable menu.

export interface MenuDiag { windows: string[]; found: boolean; entries: number; labels: string[]; layout: "bar" | "top" | "vertical" | "none" | "pending"; why: string; game: boolean }
export const menuDiag: MenuDiag = { windows: [], found: false, entries: 0, labels: [], layout: "none", why: "not checked yet", game: false };
const CLASSES = ["hearth-bar", "hearth-menu", "hearth-ingame"];

export function startMenuBar(opts: { top: boolean; onChange?: (d: MenuDiag) => void }): () => void {
  const watched: { doc: Document; obs: MutationObserver }[] = [];
  const failed = new WeakMap<Document, string>();   // windows where the bar didn't fit: Hearth's colours only
  let last = "";
  const report = () => {
    const k = JSON.stringify(menuDiag);
    if (k !== last) { last = k; try { opts.onChange && opts.onChange(menuDiag); } catch { /* never throw */ } }
  };
  const clear = (html: HTMLElement) => { for (const c of CLASSES) html.classList.remove(c); };
  const checkDoc = (doc: Document): boolean => {
    const html = doc.documentElement;
    const c = doc.getElementById("MainNavMenuContainer");
    if (!c) { clear(html); return false; }
    const panel = c.querySelector(":scope > .Panel");
    const items = panel ? Array.from(panel.querySelectorAll(":scope > * > .Panel.Focusable")) as HTMLElement[] : [];
    const labels = items.map((e) => (e.textContent || "").trim());
    menuDiag.found = true; menuDiag.entries = items.length; menuDiag.labels = labels.slice(0, 12);
    const st = judgeStructure(!!panel, labels);
    if (!st.ok) { clear(html); menuDiag.layout = "none"; menuDiag.why = st.why; return true; }
    items.forEach((el, i) => { const ic = menuIcon(labels[i]); if (el.getAttribute("data-hearth-icon") !== ic) el.setAttribute("data-hearth-icon", ic); });
    const game = opts.top && gameRunning() === true;
    menuDiag.game = game;
    html.classList.toggle("hearth-ingame", game);
    const why = failed.get(doc);
    if (why) { html.classList.remove("hearth-bar"); html.classList.add("hearth-menu"); menuDiag.layout = "vertical"; menuDiag.why = why; return true; }
    html.classList.add("hearth-bar"); html.classList.remove("hearth-menu");
    // Judged only while the menu is on screen (closed, its boxes are empty or off screen).
    const win = doc.defaultView;
    const box = c.getBoundingClientRect();
    const rects = items.map((e) => e.getBoundingClientRect());
    const visible = !!win && box.width > 0 && box.height > 0 && rects.every((r) => r.width > 0 && r.height > 0) && doc.visibilityState !== "hidden";
    if (!visible) { menuDiag.layout = "pending"; menuDiag.why = "Steam's menu found; the bar is checked when the menu opens"; return true; }
    let hit = false;
    try {
      const r = rects[0], el = doc.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      hit = !!el && (el === items[0] || items[0].contains(el) || el.contains(items[0]));
    } catch { hit = false; }
    const j = judgeBar({ w: win!.innerWidth, h: win!.innerHeight }, { left: box.left, right: box.right, top: box.top, bottom: box.bottom },
      rects.map((r) => ({ left: r.left, top: r.top, width: r.width, height: r.height })), hit, game);
    if (!j.ok) {
      failed.set(doc, "the bar didn't fit Steam's menu window (" + j.why + "): Hearth's colours on Steam's own menu");
      html.classList.remove("hearth-bar"); html.classList.add("hearth-menu");
      menuDiag.layout = "vertical"; menuDiag.why = failed.get(doc)!;
    } else { menuDiag.layout = game ? "top" : "bar"; menuDiag.why = ""; }
    return true;
  };
  const checkAll = () => {
    menuDiag.windows = windowNames();
    let any = false;
    for (const w of steamWindows()) { try { if (checkDoc(w.document)) any = true; } catch { /* a window that went away */ } }
    if (!any) { menuDiag.found = false; menuDiag.entries = 0; menuDiag.labels = []; menuDiag.layout = "none"; menuDiag.why = "Steam's menu (#MainNavMenuContainer) isn't in any of Steam's windows yet (it appears when the menu first opens)"; }
    report();
  };
  const watch = () => {
    for (const w of steamWindows()) {
      const doc = w.document;
      if (!doc?.body || watched.some((x) => x.doc === doc)) continue;
      let queued = false;
      const obs = new MutationObserver(() => {
        if (queued) return;
        queued = true;
        const run = () => { queued = false; try { checkDoc(doc); report(); } catch { /* gone */ } };
        try { w.requestAnimationFrame(run); } catch { setTimeout(run, 16); }
      });
      try { obs.observe(doc.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class"] }); watched.push({ doc, obs }); } catch { /* not watchable */ }
    }
  };
  watch();
  checkAll();
  const t = setInterval(() => { watch(); checkAll(); }, 1000);
  return () => {
    clearInterval(t);
    for (const { doc, obs } of watched) {
      try {
        obs.disconnect();
        clear(doc.documentElement);
        doc.querySelectorAll("[data-hearth-icon]").forEach((el) => el.removeAttribute("data-hearth-icon"));
      } catch { /* gone */ }
    }
    watched.length = 0;
    menuDiag.layout = "none"; menuDiag.why = "off";
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
