// Where the takeover changes Steam's own screens. Every change goes through the guard (state.ts guard, model.ts
// makeGuard): if Steam's screens aren't shaped the way a patch expects any more (a Steam update), that part turns
// itself off, says why in Decky's log and Quick Access › Hearth, and Steam's own screen is shown as it is.
// Each wrapper also checks its switch every time it runs, so switching a part off takes effect without a restart.
import * as DUI from "@decky/ui";
import { DialogButton, Focusable, afterPatch, findInReactTree } from "@decky/ui";
import { routerHook } from "@decky/api";
import { ReactNode } from "react";
import { useAsk } from "./api";
import { DESTS, Game, extraLibraryTabs, hearthIdForApp, oldRowOn, partOn } from "./model";
import { Boundary, C, CSS, go, onFocus } from "./ui";
import { GameInfoLine, HearthHome, LibraryGrid } from "./pages";
import { openApp } from "./actions";
import { guard, settings, shortcutMap, useLive } from "./state";

type Unpatch = () => void;

// ---------------------------------------------------------------- 1. Hearth's home in place of Steam's Home

/** Decides what Home shows, every render: Hearth's home, Steam's with the older Hearth row, or Steam's own. */
function HomeSwitch(props: { steam: ReactNode; oldRow: (steam: ReactNode) => ReactNode }) {
  const s = useLive();
  if (partOn(s, "tkHome", guard.broken)) {
    return <Boundary part="tkHome" fallback={props.steam} onError={guard.fail}><HearthHome steam={props.steam} /></Boundary>;
  }
  if (oldRowOn(s, guard.broken)) return <Boundary part="homeRow" fallback={props.steam} onError={guard.fail}>{props.oldRow(props.steam)}</Boundary>;
  return <>{props.steam}</>;
}

export function patchHome(oldRow: (steam: ReactNode) => ReactNode): Unpatch {
  const patch = guard.run("tkHome", () => routerHook.addPatch("/library/home", (route: any) => {
    guard.run("tkHome", () => {
      const kids = route?.children;
      if (kids && kids.type !== HomeSwitch) route.children = <HomeSwitch steam={kids} oldRow={oldRow} />;
    }, undefined);
    return route;
  }), null as any);
  return () => { if (patch) { try { routerHook.removePatch("/library/home", patch); } catch { /* gone */ } } };
}

// ---------------------------------------------------------------- 2. Hearth's pages as routes

export function addRoutes(routes: [string, () => any][]): Unpatch {
  const added: string[] = [];
  for (const [path, Comp] of routes) {
    guard.run("routes", () => { routerHook.addRoute(path, Comp, { exact: true }); added.push(path); }, undefined);
  }
  return () => { for (const p of added) { try { routerHook.removeRoute(p); } catch { /* gone */ } } };
}

// ---------------------------------------------------------------- shared: wrapping one of Steam's components

/** One wrapper per original component (so React keeps its state between renders), calling `after` on its result. */
const wrappers = new WeakMap<object, any>();
const ours = new WeakSet<object>();
function hookType(el: any, part: string, after: (ret: any) => any): boolean {
  const orig = el?.type;
  if (!orig || (typeof orig !== "function" && typeof orig !== "object")) return false;
  if (ours.has(orig)) return true;
  const have = wrappers.get(orig);
  if (have) { el.type = have; return true; }
  const call = (f: any) => function (this: any, ...a: any[]) {
    const r = f.apply(this, a);
    return guard.run(part, () => after(r), r);
  };
  let w: any;
  if (typeof orig === "function" && !orig.prototype?.isReactComponent) { w = Object.assign(call(orig), orig); }
  else if (typeof orig === "object" && typeof orig.type === "function") { w = { ...orig, type: call(orig.type) }; }       // memo
  else if (typeof orig === "object" && typeof orig.render === "function") { w = { ...orig, render: call(orig.render) }; } // forwardRef
  else return false;
  wrappers.set(orig, w); ours.add(w); el.type = w;
  return true;
}

// ---------------------------------------------------------------- 3. Hearth's stores as tabs in Steam's Library

interface LibraryData { tabs: { id: string; title: string; count: number; installed: number; items: Game[] }[] }
let libraryTabs: { id: string; title: string; count: number }[] = [];
function LibraryTabFeeder() {
  // Keeps the tab list fresh (the patch itself can't wait for answers).
  const lib = useAsk<LibraryData>("library", "", 60000);
  if (lib && !(lib as any).error) libraryTabs = extraLibraryTabs(lib.tabs);
  return null;
}

/** Steam's tab list with Hearth's stores added after Steam's own (once each). */
function withHearthTabs(tabs: any[]): any[] {
  if (!partOn(settings, "tkLibraryTabs", guard.broken) || !Array.isArray(tabs)) return tabs;
  const mine = libraryTabs.filter((t) => !tabs.some((x) => x?.id === "hearth-" + t.id)).map((t) => ({
    id: "hearth-" + t.id, title: t.title,
    content: <Boundary part="tkLibraryTabs" onError={guard.fail}><div className="hearth-tk" style={{ position: "relative", inset: "auto", background: "transparent", padding: "8px 2.8vw" }}><style>{CSS}</style><LibraryGrid tab={t.id} /></div></Boundary>,
    footer: {}, renderTabAddon: () => <span style={{ color: C.dim }}>{t.count}</span>,
  }));
  return mine.length ? [...tabs, ...mine] : tabs;
}

/** Looks through what a Library component drew for Steam's tab list (props.tabs and props.activeTab); one level
 *  further down when it isn't there yet (Steam's Library draws it a component or two deep). Steam's tabs not found
 *  in 30 of the Library's renders: the part turns off (its wrappers then just pass Steam's screen through). */
let misses = 0;
/** Steam's Library's own tabs (their ids, which TabMaster also goes by): never another page's tabs. */
const LIBRARY_TAB_IDS = /^(GreatOnDeck|AllGames|Installed|Favorites|Collections|DesktopApps|NonSteam|Soundtracks|Hidden|LocalFiles)$/;
const isLibraryTabs = (tabs: any[]) => tabs.some((t) => typeof t?.id === "string" && LIBRARY_TAB_IDS.test(t.id));
const hookable = (t: any) => typeof t === "function" || (t && typeof t === "object" && (typeof t.type === "function" || typeof t.render === "function"));
function findTabs(ret: any, depth: number): any {
  const node = findInReactTree(ret, (x: any) => Array.isArray(x?.props?.tabs) && x?.props?.activeTab !== undefined && isLibraryTabs(x.props.tabs));
  if (node) { misses = 0; node.props.tabs = withHearthTabs(node.props.tabs); return ret; }
  if (depth < 1) {
    const child = findInReactTree(ret, (x: any) => x?.props && hookable(x.type) && !ours.has(x.type));
    if (child && hookType(child, "tkLibraryTabs", (r) => findTabs(r, depth + 1))) return ret;
  }
  if (++misses > 30) throw new Error("Steam's Library tabs weren't found (Steam changed its Library)");
  return ret;
}

export function patchLibraryTabs(): Unpatch {
  const patch = guard.run("tkLibraryTabs", () => routerHook.addPatch("/library", (route: any) => {
    guard.run("tkLibraryTabs", () => {
      const el = route?.children;
      if (!el || !partOn(settings, "tkLibraryTabs", guard.broken)) return;
      hookType(el, "tkLibraryTabs", (r) => findTabs(r, 0));
      route.children = <><LibraryTabFeeder />{el}</>;
    }, undefined);
    return route;
  }), null as any);
  return () => { if (patch) { try { routerHook.removePatch("/library", patch); } catch { /* gone */ } } };
}

// ---------------------------------------------------------------- 4. Hearth on a game's page

interface Info { protondb?: { label?: string } | null; achievements?: { total: number; done: number } | null }
function HearthBadge(props: { id: string; mine: boolean }) {
  const info = useAsk<Info>("info", props.id, 3600000);
  const ok = info && !(info as any).error;
  const has = ok && (info!.protondb?.label || info!.achievements?.total);
  if (!props.mine && !has) return null;
  return (
    <div style={{ position: "absolute", top: 64, right: 28, zIndex: 20, maxWidth: 360, padding: "8px 12px", borderRadius: 10,
                  background: "rgba(12,16,22,.82)", border: `1px solid ${C.line}`, color: C.text, fontSize: 13, pointerEvents: "none" }}>
      <div style={{ color: C.accent, fontWeight: 700, fontSize: 11, letterSpacing: ".08em", textTransform: "uppercase" }}>{props.mine ? "Plays through Hearth" : "Hearth"}</div>
      {has && <GameInfoLine id={props.id} />}
    </div>
  );
}

function GameBadge(props: { steam: ReactNode }) {
  const s = useLive();
  const useParams = (DUI as any).useParams as (() => { appid?: string }) | undefined;
  const params: { appid?: string } = useParams ? useParams() : {};
  const id = partOn(s, "tkGamePage", guard.broken) ? hearthIdForApp(shortcutMap, params.appid || "") : null;
  return (
    <>
      {props.steam}
      {id && <Boundary part="tkGamePage" fallback={null} onError={guard.fail}><HearthBadge id={id} mine={!id.startsWith("steam:")} /></Boundary>}
    </>
  );
}

export function patchGamePage(): Unpatch {
  const patch = guard.run("tkGamePage", () => routerHook.addPatch("/library/app/:appid", (route: any) => {
    guard.run("tkGamePage", () => {
      const kids = route?.children;
      if (kids && kids.type !== GameBadge) route.children = <GameBadge steam={kids} />;
    }, undefined);
    return route;
  }), null as any);
  return () => { if (patch) { try { routerHook.removePatch("/library/app/:appid", patch); } catch { /* gone */ } } };
}

// ---------------------------------------------------------------- 5. Hearth in Steam's main menu

function HearthMenuItems() {
  const s = useLive();
  if (!partOn(s, "tkMenu", guard.broken)) return null;
  return (
    <Boundary part="tkMenu" fallback={null} onError={guard.fail}>
      <Focusable {...({ "flow-children": "vertical" } as any)} style={{ marginTop: 8, paddingTop: 8, borderTop: `1px solid ${C.line}` }}>
        <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: ".08em", textTransform: "uppercase", color: C.accent, padding: "0 16px 4px" }}>Hearth</div>
        {DESTS.map((d) => (
          <DialogButton key={d.id} style={{ display: "block", width: "100%", textAlign: "left", background: "transparent", padding: "8px 16px", minWidth: 0 }}
            onClick={() => { if (d.route) go(d.route); else if (d.app) openApp(d.app); }} {...({ onGamepadFocus: onFocus } as any)}>
            {d.title}
          </DialogButton>
        ))}
      </Focusable>
    </Boundary>
  );
}

/** Steam's main menu component, found by the localisation keys of its entries (stable across many Steam versions
 *  even when its code names change). Null when there's no such component any more. */
function findMainMenu(): { mod: any; key: string } | null {
  const find = (DUI as any).findModule as ((f: (m: any) => boolean) => any) | undefined;
  if (!find) return null;
  let found: { mod: any; key: string } | null = null;
  find((m: any) => {
    if (found || !m || typeof m !== "object") return false;
    for (const k of Object.keys(m)) {
      try {
        const f = m[k];
        const src = typeof f === "function" ? Function.prototype.toString.call(f) : typeof f?.type === "function" ? Function.prototype.toString.call(f.type) : "";
        if (src.includes("#MainMenu_Library") && src.includes("#MainMenu_Store")) { found = { mod: m, key: k }; return true; }
      } catch { /* not readable */ }
    }
    return false;
  });
  return found;
}

/** The main menu's list: the deepest array of three or more of Steam's menu entries (elements with a label or route). */
function addToMenu(ret: any): any {
  const list = findInReactTree(ret, (x: any) => Array.isArray(x) && x.filter((e: any) => e?.props && (e.props.label || e.props.route || e.props.to)).length >= 3);
  if (!list) throw new Error("Steam's main menu has no entries list any more");
  if (!list.some((e: any) => e?.key === "hearth-menu")) list.push(<HearthMenuItems key="hearth-menu" />);
  return ret;
}

export function patchMainMenu(): Unpatch {
  const undo: { unpatch?: () => void } = {};
  guard.run("tkMenu", () => {
    const where = findMainMenu();
    if (!where) throw new Error("Steam's main menu wasn't found (Steam changed it)");
    const m = where.mod, k = where.key;
    const p: any = typeof m[k] === "function" ? afterPatch(m, k, (_a: any[], ret: any) => guard.run("tkMenu", () => addToMenu(ret), ret))
                                                : afterPatch(m[k], "type", (_a: any[], ret: any) => guard.run("tkMenu", () => addToMenu(ret), ret));
    undo.unpatch = () => p?.unpatch?.();
  }, undefined);
  return () => { try { undo.unpatch?.(); } catch { /* gone */ } };
}
