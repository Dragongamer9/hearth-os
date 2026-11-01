// Hearth's look in Steam's gamepad UI (defaults/hearth-theme.css, which explains its three layers). The CSS goes into
// every Steam UI window this can find (the main gamepad window, Quick Access and menus in it, the keyboard's window),
// again every few seconds so windows Steam opens later get it too, and comes out at once when switched off.
import * as DUI from "@decky/ui";

export interface Brand { css: string; logo: string; wallpaper: string }

const STYLE_ID = "hearth-theme";
const TEMPLATE = /\{\{([A-Za-z0-9_]+):([A-Za-z0-9_]+)\}\}/g;

/** The hashed class name Steam uses for `key`, from the class map that also has `anchor` (undefined: not there). */
export function steamClass(anchor: string, key: string): string | undefined {
  try {
    const find = (DUI as any).findModule as ((f: (m: any) => boolean) => any) | undefined;
    if (!find) return undefined;
    const m = find((mod: any) => mod && typeof mod === "object" && typeof mod[anchor] === "string" && typeof mod[key] === "string");
    return m ? (m[key] as string) : undefined;
  } catch {
    return undefined;
  }
}

/** The theme with Steam's names and Hearth's pictures filled in; lines with a name or picture that's missing are left out. */
export function renderTheme(css: string, brand: Brand, resolve: (anchor: string, key: string) => string | undefined): string {
  const cache = new Map<string, string | undefined>();
  const out: string[] = [];
  for (const line of css.split("\n")) {
    if ((line.includes("%WALLPAPER%") && !brand.wallpaper) || (line.includes("%LOGO%") && !brand.logo)) continue;
    let missing = false;
    const done = line.replace(TEMPLATE, (_all, a: string, k: string) => {
      const id = a + ":" + k;
      if (!cache.has(id)) cache.set(id, resolve(a, k));
      const v = cache.get(id);
      if (!v) { missing = true; return ""; }
      // Steam's class map values can hold several names ("a b"): the first is the component's own.
      return v.split(/\s+/)[0];
    });
    if (missing) continue;
    out.push(done.replace(/%WALLPAPER%/g, brand.wallpaper).replace(/%LOGO%/g, brand.logo));
  }
  return out.join("\n");
}

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

export function applyTheme(brand: Brand) {
  current = renderTheme(brand.css, brand, steamClass);
  const run = () => { for (const w of steamWindows()) { try { put(w, current); } catch { /* a window that went away */ } } };
  run();
  if (!timer) timer = setInterval(run, 4000);
}

export function removeTheme() {
  if (timer) { clearInterval(timer); timer = undefined; }
  current = "";
  for (const w of steamWindows()) {
    try { w.document.getElementById(STYLE_ID)?.remove(); } catch { /* gone */ }
  }
}
