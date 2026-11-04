// Stage 1, the skin: Hearth's look as one <style> in Steam's UI windows (the gamepad UI's main window, and the
// windows Steam opens for its main menu and Quick Access: CSS Loader calls them "MainMenu" and "QuickAccess"; they're
// popups of the main window, found through g_PopupManager and the gamepad navigation trees), refreshed every few seconds for windows Steam opens later, and taken out of
// every window at once by removeSkin(). Class names come from @decky/ui's class maps (or findClassModule), never
// hard-coded hashes.
import * as DUI from "@decky/ui";
import { isWebRoute, judgeBar, judgeStructure, judgeVisible, menuIcon } from "./model";
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
  // Steam's popups (its menu and Quick Access windows) don't get the main window's page background: on a device the
  // menu window's body painted the whole window dark behind the mirror (1.2.5 log).
  try { doc.documentElement.classList.toggle("hearth-popup", win !== mainWindow()); } catch { /* fine */ }
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
    try { w.document.getElementById(STYLE_ID)?.remove(); w.document.documentElement.classList.remove("hearth-popup"); } catch { /* gone */ }
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
  route: string; anchor: string; selfCheck: string; resizeApis: string; attempts: string;
}
export const menuDiag: MenuDiag = { windows: [], found: false, entries: 0, labels: [], layout: "none", why: "not checked yet", game: false,
  method: "none", before: "", after: "", resizeWith: "", route: "", anchor: "", selfCheck: "", resizeApis: "", attempts: "" };
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

/** The main window's page ("/library/home", "/steamweb" for the Store...), "" when it can't be read. */
export function mainRoute(): string {
  try {
    const m = (DUI as any).Router?.WindowStore?.GamepadUIMainWindowInstance ?? (window as any).SteamUIStore?.WindowStore?.GamepadUIMainWindowInstance;
    const loc = m?.m_history?.location ?? m?.m_History?.location;
    if (typeof loc?.pathname === "string") return loc.pathname;
    return typeof m?.m_locationPathname === "string" ? m.m_locationPathname : "";
  } catch { return ""; }
}

/** Every entry really visible in its window: inside it, not hidden by itself or a parent, not covered. */
let resizeToTried = false;   // window.resizeTo: tried once per plugin start (it does nothing to Steam's menu on a device)
let bvName = "";

/** The BrowserViewPopup of Steam's main menu (the menu window is one of Steam's browser views): the focus controller's
 *  active browser view while the menu has the focus, else one found by name ("MainMenu...") on the main window's
 *  instance or its menu store. null when none is found. */
function browserViewFor(doc: Document): any {
  const cands: any[] = [];
  try {
    const fnc = (DUI as any).getFocusNavController?.() ?? (window as any).GamepadNavTree?.m_context?.m_controller ?? (window as any).FocusNavController;
    if (fnc?.m_activeBrowserView) cands.push(fnc.m_activeBrowserView);
  } catch { /* none */ }
  try {
    const inst = (DUI as any).Router?.WindowStore?.GamepadUIMainWindowInstance ?? (window as any).SteamUIStore?.WindowStore?.GamepadUIMainWindowInstance;
    for (const o of [inst, inst?.MenuStore, inst?.MenuStore?.m_MainMenuStore, inst?.m_MenuStore]) {
      if (!o || typeof o !== "object") continue;
      for (const k of Object.keys(o)) { try { const v = o[k]; if (v && typeof v === "object" && (v.m_browserView || v.m_strName)) cands.push(v); } catch { /* */ } }
    }
  } catch { /* none */ }
  const title = (doc.title || "").toLowerCase();
  for (const c of cands) {
    try {
      const name = String(c.m_strName || "");
      const isMenu = /mainmenu/i.test(name) || (!!title && name.toLowerCase() === title);
      const b = c.m_browserView ?? c.GetBrowser?.();
      if (isMenu && b) { bvName = name; return b; }
    } catch { /* next */ }
  }
  return null;
}

let lastSelfCheck: { label: string; inside: boolean; shown: boolean; hit: boolean; cover: string }[] = [];
let lastMenuDoc: Document | null = null;
let lastAnchorClass = "";

function visibleCheck(doc: Document, items: HTMLElement[]): { ok: boolean; why: string } {
  const win = doc.defaultView!;
  const list = items.map((e) => {
    const r = e.getBoundingClientRect();
    let shown = true;
    for (let n: Element | null = e; n && n !== doc.documentElement; n = n.parentElement) {
      const cs = win.getComputedStyle(n);
      if (cs.display === "none" || cs.visibility === "hidden" || parseFloat(cs.opacity) < 0.05) { shown = false; break; }
    }
    let hit = false, cover = "";
    try {
      const el = doc.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      hit = !!el && (el === e || e.contains(el) || el.contains(e));
      if (!hit && el) cover = (el.id ? "#" + el.id : "") + (typeof el.className === "string" && el.className ? "." + el.className.split(/\s+/).slice(0, 2).join(".") : el.tagName.toLowerCase());
    } catch { hit = false; }
    return { label: (e.textContent || "").trim().slice(0, 20), inside: r.width > 0 && r.height > 0 && r.right > 0 && r.left < win.innerWidth && r.bottom > 0 && r.top < win.innerHeight, shown, hit, cover };
  });
  lastSelfCheck = list;
  return judgeVisible(list);
}

// ---- the deep diagnostics (round 6): what's really in Steam's windows while the menu is open, so a device log can
// pinpoint the blank side menu and the blank top dock. Compact one-line outlines, capped.

function outline(el: Element, win: Window): string {
  const r = el.getBoundingClientRect();
  let cs: CSSStyleDeclaration | null = null;
  try { cs = win.getComputedStyle(el); } catch { cs = null; }
  const cls = typeof (el as any).className === "string" ? (el as any).className.trim().split(/\s+/).filter(Boolean).slice(0, 4).join(".") : "";
  const name = el.tagName.toLowerCase() + (el.id ? "#" + el.id : "") + (cls ? "." + cls : "");
  const st = cs ? ` pos=${cs.position} op=${cs.opacity} vis=${cs.visibility} disp=${cs.display} z=${cs.zIndex} bg=${cs.backgroundColor}${cs.backgroundImage && cs.backgroundImage !== "none" ? "+img" : ""}` +
    `${cs.transform && cs.transform !== "none" ? " tf=" + cs.transform.slice(0, 40) : ""} t=${cs.top} b=${cs.bottom} l=${cs.left} w=${cs.width} h=${cs.height}` : "";
  return `${name.slice(0, 120)} [${Math.round(r.left)},${Math.round(r.top)} ${Math.round(r.width)}×${Math.round(r.height)}]${st}`;
}

function mainOutline(main: Window): string[] {
  const out: string[] = [];
  const d = main.document;
  const all = Array.from(d.body ? d.body.querySelectorAll("*") : []);
  const NAMES = /menu|nav|sidebar|side|overlay|backdrop|modal|anchor|scrim|dim/i;
  for (const el of all) {
    if (out.length >= 45) { out.push("… (more left out)"); break; }
    if (el.id === "hearth-bar-mirror" || el.closest("#hearth-bar-mirror")) continue;
    const r = el.getBoundingClientRect();
    const cls = typeof (el as any).className === "string" ? (el as any).className : "";
    const big = r.width >= 150 && r.height >= 300 && r.left < 320 && r.right > 0;
    if ((big && r.width < main.innerWidth * 0.95) || (NAMES.test(cls) && r.width > 0 && r.height > 0)) out.push("  " + outline(el, main));
  }
  return out;
}

/** The deep dump for the log: the main window's elements around the side menu's place, the menu window's container
 *  and its parents, what MainNavMenuAnchor matched, the windows (which one has the focus), the running game, and the
 *  self-check per entry with the entries' boxes. */
export function menuDeepDump(): string {
  const lines: string[] = ["== Deep diagnostics =="];
  try {
    const wins = steamWindows();
    lines.push("Windows: " + wins.map((w) => { let f = false; try { f = w.document.hasFocus(); } catch { /* */ } return `${(w.document.title || "(untitled)").slice(0, 30)} ${size(w)}${f ? " [focused]" : ""}`; }).join(" | "));
    try {
      const a = ((DUI as any).Router ?? (window as any).SteamUIStore)?.MainRunningApp;
      lines.push("Router.MainRunningApp: " + (a ? `appid ${a.appid} "${String(a.display_name || "").slice(0, 40)}"` : "none"));
    } catch { lines.push("Router.MainRunningApp: unreadable"); }
    lines.push(`Window bounds: before ${menuDiag.before || "-"}, after ${menuDiag.after || "-"} (with ${menuDiag.resizeWith || "-"})`);
    const main = mainWindow();
    const cls = (lastAnchorClass || "").split(/\s+/)[0];
    const matches = cls ? Array.from(main.document.querySelectorAll("." + cls)) : [];
    lines.push(`MainNavMenuAnchor: class "${lastAnchorClass || "(none)"}", ${matches.length} match(es) in the main window`);
    for (const m of matches.slice(0, 3)) lines.push("  " + outline(m, main));
    lines.push(`Main window (${size(main)}), elements over the left side or named like a menu/overlay:`);
    lines.push(...mainOutline(main));
    const md = lastMenuDoc;
    if (md && md.defaultView) {
      const mw = md.defaultView;
      let bodyBg = "?";
      try { bodyBg = mw.getComputedStyle(md.body).backgroundColor + " / html " + mw.getComputedStyle(md.documentElement).backgroundColor; } catch { /* */ }
      lines.push(`Menu window "${(md.title || "").slice(0, 30)}": ${size(mw)}, document ${md.documentElement.scrollWidth}×${md.documentElement.scrollHeight}, classes "${md.documentElement.className}", background ${bodyBg}`);
      const c = md.getElementById("MainNavMenuContainer");
      let n: Element | null = c;
      for (let i = 0; n && i < 8; i++, n = n.parentElement) lines.push((i ? "  parent " + i + ": " : "  container: ") + outline(n, mw));
      if (c) {
        const panel = c.querySelector(":scope > .Panel");
        if (panel) lines.push("  panel: " + outline(panel, mw));
        Array.from(c.querySelectorAll(":scope > .Panel > * > .Panel.Focusable")).slice(0, 10).forEach((e, i) => lines.push(`  entry ${i}: ` + outline(e, mw)));
      }
      const others = Array.from(md.body ? md.body.children : []).filter((e) => !e.contains(c));
      if (others.length) { lines.push("  other things in the menu window:"); others.slice(0, 8).forEach((e) => lines.push("    " + outline(e, mw))); }
    } else lines.push("Menu window: not seen open yet");
    lines.push("Self-check per entry: " + (lastSelfCheck.length ? lastSelfCheck.map((e) => `${e.label}:${e.inside ? "in" : "OUT"}/${e.shown ? "shown" : "HIDDEN"}/${e.hit ? "hit" : "COVERED" + (e.cover ? " by " + e.cover : "")}`).join(", ") : "not run"));
  } catch (e) { lines.push("(the deep diagnostics stopped: " + String(e).slice(0, 120) + ")"); }
  return lines.join("\n").slice(0, 40000);
}

interface DocState { mode: MenuMethod; topFailed: string; with?: { how: string; restore(saved: any): void }; orig: any; tries: number; why: string; at: string; busy: boolean }

export function startMenuBar(opts: { top: boolean; anchorClass?: string; onChange?: (d: MenuDiag) => void }): () => void {
  const anchorClass = opts.anchorClass || "";
  lastAnchorClass = anchorClass;
  const watched: { doc: Document; obs: MutationObserver }[] = [];
  const states = new Map<Document, DocState>();
  let last = "";
  const report = () => {
    const k = JSON.stringify(menuDiag);
    if (k !== last) { last = k; try { opts.onChange && opts.onChange(menuDiag); } catch { /* never throw */ } }
  };
  const clear = (html: HTMLElement) => { for (const c of CLASSES) html.classList.remove(c); };
  const stateOf = (doc: Document) => { let st = states.get(doc); if (!st) { st = { mode: "pending", topFailed: "", orig: null, tries: 0, why: "", at: "", busy: false }; states.set(doc, st); } return st; };

  // ---- 1. resizing the menu's own window
  // On the owner's device (1.2.5 logs) the menu window has no SteamClient.Window and window.resizeTo does nothing. The
  // menu is one of Steam's browser views (its body holds #browserview_target): its BrowserViewPopup has SetBounds and
  // GetBounds, which Steam itself uses to place it. Tried in order: that, SteamClient.Window, and window.resizeTo once
  // per plugin start (it never worked on a device; tried once so the log can say so).
  interface Bounds { x: number; y: number; width: number; height: number }
  /** One way of resizing: what it is, how to remember the window's place, put it at a place (in its own units),
   *  and put the remembered place back. */
  interface Resizer { how: string; units: "css" | "device" | "bounds"; save(): Promise<any>; set(b: Bounds): void; restore(saved: any): void }
  function resizers(win: any, doc: Document): Resizer[] {
    const out: Resizer[] = [];
    const bv = browserViewFor(doc);
    if (bv && typeof bv.SetBounds === "function") out.push({ how: "BrowserView.SetBounds", units: "bounds",
      save: async () => { try { const b = bv.GetBounds?.(); return b && b.width > 0 ? { x: b.x, y: b.y, width: b.width, height: b.height } : null; } catch { return null; } },
      set: (b) => bv.SetBounds(b.x, b.y, b.width, b.height),
      restore: (o) => { if (o) bv.SetBounds(o.x, o.y, o.width, o.height); } });
    // The menu window's own SteamClient (that window's context, not the main window's). On the owner's device its
    // ResizeTo refused (w, h, false) with "requires 3 arguments: <x: double> <y: double> ...": the third must be a
    // number there. Tried: CSS pixels with the screen's scale, device pixels with 1, then the booleans; each checked
    // by the window's real size afterwards. Its place is remembered with GetWindowRestoreDetails and put back with
    // RestoreWindowSizeAndPosition (whatever units it uses).
    const sw = (() => { try { return win.SteamClient?.Window ?? null; } catch { return null; } })();
    if (sw && typeof sw.ResizeTo === "function") {
      const dpr = mainWindow().devicePixelRatio || 1;
      const variants: [string, "css" | "device", number | boolean][] = [
        [`CSS px, scale ${Math.round(dpr * 100) / 100}`, "css", dpr], ["device px, scale 1", "device", 1], ["CSS px, true", "css", true], ["device px, false", "device", false]];
      for (const [label, units, third] of variants) out.push({ how: `SteamClient.Window (${label})`, units,
        save: async () => {
          try { if (typeof sw.GetWindowRestoreDetails === "function") { const d = await sw.GetWindowRestoreDetails(); if (typeof d === "string" && d) return { details: d }; } } catch { /* below */ }
          try { const d = typeof sw.GetWindowDimensions === "function" ? await sw.GetWindowDimensions() : null; if (d && d.width > 0) return { dims: d }; } catch { /* below */ }
          return null;
        },
        set: (b) => { sw.ResizeTo(b.width, b.height, third); if (typeof sw.MoveTo === "function") sw.MoveTo(b.x, b.y, third); },
        restore: (o) => {
          if (!o) return;
          if (o.details && typeof sw.RestoreWindowSizeAndPosition === "function") { sw.RestoreWindowSizeAndPosition(o.details); return; }
          if (o.dims) { sw.ResizeTo(o.dims.width, o.dims.height, third); if (typeof sw.MoveTo === "function") sw.MoveTo(o.dims.x, o.dims.y, third); }
        } });
    }
    if (!resizeToTried) out.push({ how: "window.resizeTo", units: "css",
      save: async () => ({ x: win.screenX || 0, y: win.screenY || 0, width: win.outerWidth || win.innerWidth, height: win.outerHeight || win.innerHeight }),
      set: (b) => { resizeToTried = true; win.resizeTo(b.width, b.height); win.moveTo(b.x, b.y); },
      restore: (o) => { if (o) { win.resizeTo(o.width, o.height); win.moveTo(o.x, o.y); } } });
    menuDiag.resizeApis = `BrowserView ${bv ? "found" + (bvName ? " (" + bvName + ")" : "") : "not found"}, SteamClient.Window ${sw ? "yes (ResizeTo, MoveTo " + (typeof sw.MoveTo === "function" ? "yes" : "no") + ", restore details " + (typeof sw.GetWindowRestoreDetails === "function" ? "yes" : "no") + ")" : "no"}, window.resizeTo ${resizeToTried ? "tried before" : "untried"}`;
    return out;
  }
  async function tryResize(doc: Document, game: boolean) {
    const st = stateOf(doc);
    const win: any = doc.defaultView;
    if (!win || st.busy) return;
    st.busy = true;
    try {
      const main = mainWindow();
      const dpr = main.devicePixelRatio || 1;
      menuDiag.before = `${size(win)} (main window ${size(main)}, scale ${Math.round(dpr * 100) / 100})`;
      const tried: string[] = [];
      let done = false;
      for (const r of resizers(win, doc)) {
        const saved = await r.save();
        // The place in this way's units: CSS pixels of the main window, device pixels, or the browser view's own
        // bounds (their unit worked out from what the window is now).
        const k = r.units === "device" ? dpr : r.units === "bounds" ? (saved && saved.width > 0 && win.innerWidth > 1 ? saved.width / win.innerWidth : dpr) : 1;
        const ox = r.units === "bounds" ? 0 : Math.round((main.screenX || 0) * (r.units === "device" ? 1 : 1));
        const oy = r.units === "bounds" ? 0 : Math.round(main.screenY || 0);
        const W = Math.round(main.innerWidth * k), H = Math.round(BAR_H * k), MH = Math.round(main.innerHeight * k);
        const target = { x: ox, y: game ? oy : oy + MH - H, width: W, height: H };
        try { r.set(target); } catch (e) { tried.push(`${r.how}: ${String(e).slice(0, 300)}`); continue; }
        await new Promise((res) => setTimeout(res, 500));
        const ok = win.innerWidth >= Math.min(640, main.innerWidth * 0.9) && Math.abs(win.innerWidth - main.innerWidth) <= main.innerWidth * 0.1;
        if (ok) {
          st.orig = saved; st.with = r;
          menuDiag.resizeWith = r.how; menuDiag.after = size(win); done = true;
          break;
        }
        tried.push(`${r.how}: asked ${target.width}×${target.height} at ${target.x},${target.y}, got ${size(win)}`);
        try { r.restore(saved); } catch (e) { tried.push(`${r.how} restore: ${String(e).slice(0, 200)}`); }
      }
      st.tries++;
      if (done) { st.mode = "resized"; st.at = game ? "top" : "bottom"; st.why = ""; st.tries = 0; }
      else {
        menuDiag.resizeWith = tried.join("; ") || "nothing (no resize call here)"; menuDiag.after = size(win);
        st.mode = "mirror"; st.why = `Steam's menu window couldn't be resized (${menuDiag.before}; ${menuDiag.resizeWith})`;
      }
      try { writeAttempts(tried, done ? menuDiag.resizeWith : ""); } catch { /* fine */ }
    } catch (e) {
      restore(doc); st.mode = "mirror"; st.why = "resizing failed: " + String(e).slice(0, 300);
    } finally { st.busy = false; }
    checkAll();
  }
  /** Every resize attempt, in full, into the log (through the menu callback's diagnostics line). */
  function writeAttempts(tried: string[], worked: string) {
    menuDiag.attempts = tried.concat(worked ? ["worked: " + worked] : []).join(" | ").slice(0, 3000);
  }
  function restore(doc: Document) {
    const st = states.get(doc);
    if (!st || !st.with || st.orig === undefined) return;
    try { st.with.restore(st.orig); } catch { /* gone */ }
    st.orig = undefined; st.with = undefined;
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
  // Steam's main window keeps an empty place for its side menu while the menu is open: hidden while Hearth's bar is up.
  function setMainOn(on: boolean) {
    try {
      const d = mainWindow().document;
      d.documentElement.classList.toggle("hearth-bar-on", on);
      const cls = (anchorClass || "").split(/\s+/)[0];
      const a = cls ? d.querySelector("." + cls) as HTMLElement | null : null;
      if (a) { const r = a.getBoundingClientRect(); menuDiag.anchor = `found, ${Math.round(r.width)}×${Math.round(r.height)} at ${Math.round(r.left)},${Math.round(r.top)}${on ? ", hidden" : ""}`; }
      else menuDiag.anchor = cls ? "not in the main window" : "no class name for it";
    } catch { /* gone */ }
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
    let box = c.getBoundingClientRect();
    let rects = items.map((e) => e.getBoundingClientRect());
    const onScreen = !!win && box.width > 0 && box.height > 0 && rects.some((r) => r.width > 0 && r.height > 0) && doc.visibilityState !== "hidden";
    const focusedIdx = items.findIndex((e) => e.classList.contains("gpfocus") || e.classList.contains("gpfocuswithin"));
    let active = focusedIdx >= 0;
    try { active = active || doc.hasFocus(); } catch { /* keep */ }
    const route = mainRoute();
    menuDiag.route = route || "?";
    if (onScreen && active) lastMenuDoc = doc;
    const colours = (why: string) => {
      restore(doc); hideMirror(); setMainOn(false);
      html.classList.remove("hearth-bar", "hearth-ghost"); html.classList.add("hearth-menu");
      setDiag("vertical", "vertical", why);
      return true;
    };
    // A web page (the Store) is drawn above Steam's main window, so the mirror (drawn there) would be under it; Steam's
    // menu window itself shows above the Store (the owner's Store log), so a resized bar can be used there too.
    const web = isWebRoute(route);
    // In a game, the top dock gave a blank strip on a device: if its self-check failed once, Steam's menu in Hearth's colours.
    if (game && st.topFailed) return colours(st.topFailed);

    if (st.mode === "vertical") return colours(st.why);
    // The mirror is drawn in Steam's main window, which isn't on screen over a game: in a game, Steam's menu in
    // Hearth's colours instead (never a blank strip).
    if (st.mode === "mirror" && web) {
      return colours(`a web page is showing (${route}) and Steam's menu window can't be resized: Steam's menu in Hearth's colours (the mirror would be under the page)`);
    }
    if (st.mode === "mirror" && game) {
      return colours(/couldn't be resized/.test(st.why) ? "Steam doesn't allow the top bar during games on this device (its menu window can't be resized): Steam's menu in Hearth's colours"
        : "in a game Steam's menu is shown in Hearth's colours (the mirror can't be drawn over a game)");
    }
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
      setMainOn(open);
      setDiag(game ? "top" : "bar", "mirror", st.why);
      return true;
    }
    // "fits" / "resized" / "pending": the bar goes in the menu's own window, once it's wide enough for it (a narrow
    // window keeps Steam's layout in Hearth's colours meanwhile, so a bar is never squeezed into it).
    const wide = !!win && win.innerWidth >= 640;
    html.classList.toggle("hearth-bar", wide); html.classList.toggle("hearth-menu", !wide); html.classList.remove("hearth-ghost");
    if (!onScreen || !rects.every((r) => r.width > 0 && r.height > 0)) { setMainOn(false); setDiag("pending", st.mode === "resized" ? "resized" : "pending", "Steam's menu found; the bar is checked when the menu opens"); return true; }
    // Too narrow, or resized for the other place (a game started or ended, or the menu opened over a game): resize it.
    const want = game ? "top" : "bottom";
    if (!wide || (st.mode === "resized" && st.at !== want)) {
      setMainOn(false);
      // Only the menu that's open (focused) is resized: never a window Steam keeps but isn't showing.
      if (!active) { setDiag("pending", "pending", "waiting for the menu to open"); return true; }
      if (st.tries < 6) { setDiag("pending", "pending", "making Steam's menu window as wide as the screen"); tryResize(doc, game); return true; }
      st.mode = "mirror"; st.why = "Steam keeps the menu window narrow"; restore(doc); return checkDoc(doc);
    }
    // Measured again now that the bar's class is on (the boxes above were taken before it).
    box = c.getBoundingClientRect();
    rects = items.map((e) => e.getBoundingClientRect());
    let hit = false;
    try {
      const r = rects[0], el = doc.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      hit = !!el && (el === items[0] || items[0].contains(el) || el.contains(items[0]));
    } catch { hit = false; }
    const measured = size(win!);
    const j = judgeBar({ w: win!.innerWidth, h: win!.innerHeight }, { left: box.left, right: box.right, top: box.top, bottom: box.bottom },
      rects.map((r) => ({ left: r.left, top: r.top, width: r.width, height: r.height })), hit, game);
    if (!j.ok && game) {
      html.classList.remove("hearth-bar");
      st.topFailed = "the top dock's check failed (" + j.why + "): Steam's menu in Hearth's colours in a game";
      menuDiag.selfCheck = "failed: " + j.why;
      return colours(st.topFailed);
    }
    if (!j.ok) {
      restore(doc);
      st.mode = "mirror";
      st.why = `the bar didn't fit Steam's menu window (${j.why}; bar ${Math.round(box.width)}×${Math.round(box.height)} at ${Math.round(box.left)},${Math.round(box.top)} in ${size(win!)} (${measured}), classes ${html.className}, skin ${(doc.getElementById(STYLE_ID)?.textContent || "").includes("html.hearth-bar #MainNavMenuContainer{") ? "has the bar" : "without the bar"})`;
      html.classList.remove("hearth-bar");
      return checkDoc(doc);
    }
    // The self-check: every entry really visible in the resized window. If not, never a blank bar: Steam's window goes
    // back where it was, and the mirror (or, in a game, Steam's menu in Hearth's colours) takes over.
    const vis = visibleCheck(doc, items);
    menuDiag.selfCheck = vis.ok ? `passed (${items.length} entries visible in ${size(win!)})` : "failed: " + vis.why;
    if (!vis.ok) {
      html.classList.remove("hearth-bar");
      if (game) { st.topFailed = "the top dock's self-check failed (" + vis.why + "): Steam's menu in Hearth's colours in a game"; return colours(st.topFailed); }
      restore(doc); st.mode = "mirror"; st.why = "the bar's self-check failed (" + vis.why + ")";
      return checkDoc(doc);
    }
    if (st.mode === "pending") st.mode = "fits";
    setMainOn(true);
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
    try { mainWindow().document.documentElement.classList.remove("hearth-bar-on"); } catch { /* gone */ }
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
