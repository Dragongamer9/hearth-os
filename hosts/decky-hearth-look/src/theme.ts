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
// Steam's main menu is drawn in its own popup window (CSS Loader's "MainMenu"): a Focusable with the id
// MainNavMenuContainer, its panel (.Panel) and the entries (.Panel > * > .Panel.Focusable). On a device that window is
// only as wide as Steam's side panel (241 px on the owner's), so a bar can't fit in it as it is. In order:
//  1. "resized": the menu's window is made as wide as the screen and bar-high, at the bottom (the top in a game), with
//     the window's own SteamClient.Window (ResizeTo/MoveTo, what Steam uses for its popups) or window.resizeTo/moveTo;
//     its size is read back, and its old bounds are put back when the bar goes off, pauses or unloads.
//  2. "mirror": if it can't be resized, Hearth's bar is drawn in Steam's main window while the menu is open, and the
//     menu's own panel is made see-through but stays where it is, focused and working: up/down in Steam's menu moves
//     the bar's highlight, A presses Steam's real entry, the Steam button still opens and closes it.
//  3. "vertical": anything else, Steam's own menu layout in Hearth's colours (html.hearth-menu).
// The bar's rules only apply under html.hearth-bar (or the mirror's html.hearth-ghost), set only after the structure is
// really there; a bar that doesn't fit once on screen comes off again. Never a half-styled or unusable menu.

export type MenuMethod = "none" | "pending" | "fits" | "resized" | "mirror" | "vertical";
export interface MenuDiag {
  windows: string[]; found: boolean; entries: number; labels: string[]; layout: "bar" | "top" | "vertical" | "none" | "pending";
  why: string; game: boolean; method: MenuMethod; before: string; after: string; resizeWith: string;
}
export const menuDiag: MenuDiag = { windows: [], found: false, entries: 0, labels: [], layout: "none", why: "not checked yet", game: false,
  method: "none", before: "", after: "", resizeWith: "" };
const CLASSES = ["hearth-bar", "hearth-menu", "hearth-ingame", "hearth-ghost"];
const BAR_H = 150;   // the resized menu window's height (CSS px): the bar, its focused entry and the gaps around it
const MIRROR_ID = "hearth-bar-mirror";

/** Steam's main Big Picture window (where the mirror is drawn). */
function mainWindow(): Window {
  try { const w = (DUI as any).findSP?.(); if (w && w.document) return w; } catch { /* below */ }
  try { const w = (DUI as any).Router?.WindowStore?.GamepadUIMainWindowInstance?.BrowserWindow; if (w && w.document) return w; } catch { /* below */ }
  return window;
}
const size = (w: Window) => { try { return `${Math.round(w.innerWidth)}×${Math.round(w.innerHeight)}`; } catch { return "?"; } };

interface DocState { mode: MenuMethod; orig: { x: number; y: number; width: number; height: number } | null; tries: number; why: string; at: string; busy: boolean }

export function startMenuBar(opts: { top: boolean; onChange?: (d: MenuDiag) => void }): () => void {
  const watched: { doc: Document; obs: MutationObserver }[] = [];
  const states = new Map<Document, DocState>();
  let last = "";
  const report = () => {
    const k = JSON.stringify(menuDiag);
    if (k !== last) { last = k; try { opts.onChange && opts.onChange(menuDiag); } catch { /* never throw */ } }
  };
  const clear = (html: HTMLElement) => { for (const c of CLASSES) html.classList.remove(c); };
  const stateOf = (doc: Document) => { let st = states.get(doc); if (!st) { st = { mode: "pending", orig: null, tries: 0, why: "", at: "", busy: false }; states.set(doc, st); } return st; };

  // ---- 1. resizing the menu's own window
  const steamWin = (win: any) => { try { return win.SteamClient?.Window ?? null; } catch { return null; } };
  async function boundsOf(win: any) {
    const sw = steamWin(win);
    try { const d = sw && typeof sw.GetWindowDimensions === "function" ? await sw.GetWindowDimensions() : null; if (d && d.width > 0) return { x: d.x, y: d.y, width: d.width, height: d.height }; } catch { /* below */ }
    return { x: win.screenX || 0, y: win.screenY || 0, width: win.outerWidth || win.innerWidth, height: win.outerHeight || win.innerHeight };
  }
  function place(win: any, x: number, y: number, w: number, h: number): string {
    const sw = steamWin(win);
    if (sw && typeof sw.ResizeTo === "function") {
      try { sw.ResizeTo(w, h, false); if (typeof sw.MoveTo === "function") sw.MoveTo(x, y); return "SteamClient.Window"; } catch { /* below */ }
    }
    try { win.resizeTo(w, h); win.moveTo(x, y); return "window.resizeTo"; } catch { return ""; }
  }
  async function tryResize(doc: Document, game: boolean) {
    const st = stateOf(doc);
    const win: any = doc.defaultView;
    if (!win || st.busy) return;
    st.busy = true;
    try {
      if (!st.orig) st.orig = await boundsOf(win);
      const main = mainWindow();
      const dpr = main.devicePixelRatio || 1;
      // Physical pixels, like Steam's own bounds: the whole width of the main window, bar-high, at its bottom (top in a game).
      const mx = (main.screenX || 0), my = (main.screenY || 0);
      const W = Math.round(main.innerWidth * dpr), H = Math.round(BAR_H * dpr), MH = Math.round(main.innerHeight * dpr);
      menuDiag.before = `${size(win)} (main window ${size(main)})`;
      const how = place(win, mx, game ? my : my + MH - H, W, H);
      menuDiag.resizeWith = how || "nothing (no resize call works here)";
      await new Promise((r) => setTimeout(r, 450));
      menuDiag.after = size(win);
      st.tries++;
      if (how && win.innerWidth >= Math.min(640, main.innerWidth * 0.9)) { st.mode = "resized"; st.at = game ? "top" : "bottom"; st.why = ""; st.tries = 0; }
      else { restore(doc); st.mode = "mirror"; st.why = `Steam's menu window couldn't be resized (${menuDiag.before} → ${menuDiag.after} with ${menuDiag.resizeWith})`; }
    } catch (e) {
      restore(doc); st.mode = "mirror"; st.why = "resizing failed: " + String(e).slice(0, 120);
    } finally { st.busy = false; }
    checkAll();
  }
  function restore(doc: Document) {
    const st = states.get(doc);
    const win: any = doc.defaultView;
    if (!st || !st.orig || !win) return;
    try { place(win, st.orig.x, st.orig.y, st.orig.width, st.orig.height); } catch { /* gone */ }
    st.orig = null;
  }

  // ---- 2. the mirror: Hearth's bar in the main window, following Steam's (see-through) menu
  function mirror(labels: string[], icons: string[], focused: number, open: boolean, game: boolean) {
    const main = mainWindow();
    const d = main.document;
    let el = d.getElementById(MIRROR_ID);
    if (!open) { if (el) el.style.display = "none"; return; }
    if (!el) {
      el = d.createElement("div");
      el.id = MIRROR_ID;
      el.setAttribute("aria-hidden", "true");
      (d.body || d.documentElement).appendChild(el);
    }
    el.className = game ? "top" : "bottom";
    el.style.display = "";
    const key = labels.join("|") + "#" + icons.join("|");
    if (el.getAttribute("data-key") !== key) {
      el.setAttribute("data-key", key);
      el.textContent = "";
      const bar = d.createElement("div");
      bar.className = "hbm-bar";
      labels.forEach((l, i) => {
        const it = d.createElement("div");
        it.className = "hbm-item";
        it.setAttribute("data-hearth-icon", icons[i] || "");
        const t = d.createElement("span");
        t.textContent = l;
        it.appendChild(t);
        bar.appendChild(it);
      });
      el.appendChild(bar);
    }
    el.querySelectorAll(".hbm-item").forEach((it, i) => it.classList.toggle("on", i === focused));
  }
  function hideMirror() { try { const el = mainWindow().document.getElementById(MIRROR_ID); if (el) el.remove(); } catch { /* gone */ } }

  // ---- checking a window
  const checkDoc = (doc: Document): boolean => {
    const html = doc.documentElement;
    const c = doc.getElementById("MainNavMenuContainer");
    if (!c) { clear(html); return false; }
    const panel = c.querySelector(":scope > .Panel");
    const items = panel ? Array.from(panel.querySelectorAll(":scope > * > .Panel.Focusable")) as HTMLElement[] : [];
    const labels = items.map((e) => (e.textContent || "").trim());
    menuDiag.found = true; menuDiag.entries = items.length; menuDiag.labels = labels.slice(0, 12);
    const stj = judgeStructure(!!panel, labels);
    if (!stj.ok) { clear(html); restore(doc); menuDiag.layout = "none"; menuDiag.method = "none"; menuDiag.why = stj.why; return true; }
    const icons = labels.map((l) => menuIcon(l));
    items.forEach((el, i) => { if (el.getAttribute("data-hearth-icon") !== icons[i]) el.setAttribute("data-hearth-icon", icons[i]); });
    const game = opts.top && gameRunning() === true;
    menuDiag.game = game;
    html.classList.toggle("hearth-ingame", game);
    const st = stateOf(doc);
    const win = doc.defaultView;
    const box = c.getBoundingClientRect();
    const rects = items.map((e) => e.getBoundingClientRect());
    const onScreen = !!win && box.width > 0 && box.height > 0 && rects.some((r) => r.width > 0 && r.height > 0) && doc.visibilityState !== "hidden";
    const focusedIdx = items.findIndex((e) => e.classList.contains("gpfocus") || e.classList.contains("gpfocuswithin"));

    if (st.mode === "vertical") { html.classList.remove("hearth-bar", "hearth-ghost"); html.classList.add("hearth-menu"); hideMirror(); setDiag("vertical", "vertical", st.why); return true; }
    if (st.mode === "mirror") {
      // Steam's menu stays where it is, see-through; Hearth's bar is drawn in the main window while it's open.
      // Open: the menu's window is on screen and has the focus (closing the menu gives the focus back to the main window).
      let open = false;
      try { open = onScreen && (doc.hasFocus() || focusedIdx >= 0); } catch { open = false; }
      try { mirror(labels, icons, focusedIdx, open, game); }
      catch (e) {
        st.mode = "vertical"; st.why = "the mirror bar couldn't be drawn: " + String(e).slice(0, 100);
        hideMirror(); html.classList.remove("hearth-bar", "hearth-ghost"); html.classList.add("hearth-menu"); setDiag("vertical", "vertical", st.why); return true;
      }
      html.classList.remove("hearth-bar", "hearth-menu"); html.classList.add("hearth-ghost");
      setDiag(game ? "top" : "bar", "mirror", st.why);
      return true;
    }
    // "fits" / "resized" / "pending": the bar goes in the menu's own window, once it's wide enough for it (a narrow
    // window keeps Steam's layout in Hearth's colours meanwhile, so a bar is never squeezed into it).
    const wide = !!win && win.innerWidth >= 640;
    html.classList.toggle("hearth-bar", wide); html.classList.toggle("hearth-menu", !wide); html.classList.remove("hearth-ghost");
    if (!onScreen || !rects.every((r) => r.width > 0 && r.height > 0)) { setDiag("pending", st.mode === "resized" ? "resized" : "pending", "Steam's menu found; the bar is checked when the menu opens"); return true; }
    // Too narrow, or resized for the other place (a game started or ended, or the menu opened over a game): resize it.
    const want = game ? "top" : "bottom";
    if (!wide || (st.mode === "resized" && st.at !== want)) {
      if (st.tries < 6) { setDiag("pending", "pending", "making Steam's menu window as wide as the screen"); tryResize(doc, game); return true; }
      st.mode = "mirror"; st.why = "Steam keeps the menu window narrow"; restore(doc); return checkDoc(doc);
    }
    let hit = false;
    try {
      const r = rects[0], el = doc.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      hit = !!el && (el === items[0] || items[0].contains(el) || el.contains(items[0]));
    } catch { hit = false; }
    const j = judgeBar({ w: win!.innerWidth, h: win!.innerHeight }, { left: box.left, right: box.right, top: box.top, bottom: box.bottom },
      rects.map((r) => ({ left: r.left, top: r.top, width: r.width, height: r.height })), hit, game);
    if (!j.ok) {
      restore(doc);
      st.mode = "mirror";
      st.why = `the bar didn't fit Steam's menu window (${j.why}; bar ${Math.round(box.width)}×${Math.round(box.height)} at ${Math.round(box.left)},${Math.round(box.top)} in ${size(win!)}, classes ${html.className})`;
      html.classList.remove("hearth-bar");
      return checkDoc(doc);
    }
    if (st.mode === "pending") st.mode = "fits";
    setDiag(game ? "top" : "bar", st.mode, "");
    return true;
  };
  function setDiag(layout: MenuDiag["layout"], method: MenuMethod, why: string) { menuDiag.layout = layout; menuDiag.method = method; menuDiag.why = why; }
  const checkAll = () => {
    menuDiag.windows = windowNames();
    let any = false;
    for (const w of steamWindows()) { try { if (checkDoc(w.document)) any = true; } catch { /* a window that went away */ } }
    if (!any) { menuDiag.found = false; menuDiag.entries = 0; menuDiag.labels = []; setDiag("none", "none", "Steam's menu (#MainNavMenuContainer) isn't in any of Steam's windows yet (it appears when the menu first opens)"); hideMirror(); }
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
        restore(doc);
        clear(doc.documentElement);
        doc.querySelectorAll("[data-hearth-icon]").forEach((el) => el.removeAttribute("data-hearth-icon"));
      } catch { /* gone */ }
    }
    for (const doc of states.keys()) { try { restore(doc); clear(doc.documentElement); } catch { /* gone */ } }
    states.clear();
    hideMirror();
    watched.length = 0;
    setDiag("none", "none", "off");
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
