// Stage 2, the home screen: Hearth Classic's home on Steam's Home route (the old Hearth Game Mode's home, ui/index.html
// viewHome): Hearth's wallpaper and logo, a banner from the focused game, Recent games (the newest big), Hearth's
// places, Favorites and a row per store. Made of @decky/ui's Focusable and DialogButton, so Steam's own controller
// focus moves through it. Every game is one Steam can open, and opens Steam's own page. The data is Hearth's own
// layout (the backend asks `nimbus --steam-mode-data home` once a minute; no second Hearth runs), or Steam's library
// and Hearth Sync's collections when Hearth's program doesn't answer.
import * as DUI from "@decky/ui";
import { DialogButton, Focusable, Navigation } from "@decky/ui";
import { callable } from "@decky/api";
import { Component, ReactNode, useEffect, useState } from "react";
import { Card, HomeView, PLACES, Place, Shape, Shelf, SteamApp, artCandidates, blipFor, homeFromHearth, homeFromSteam, pickHome, played, runPlace, steamArtMethods } from "./model";

const askHome = callable<[], { ok: boolean; home?: unknown; shortcuts?: Record<string, unknown>; error?: string }>("home");
const askArt = callable<[string], string>("art");

export const C = { bg: "#0c1016", bg2: "#121821", panel: "#161d27", panel2: "#1d2531", panel3: "#273140", line: "#2a3443", text: "#e9eef4", dim: "#8d99a8", accent: "#3fd0b8", ink: "#06231e" };

// Scoped to .hearth-classic: the old Game Mode's shelves, tiles (its double-ring focus and lift), chips and buttons.
const CSS = `
.hearth-classic{position:absolute;inset:0;overflow-y:auto;overflow-x:hidden;color:${C.text};background:${C.bg};font-family:"Segoe UI Variable Text","Segoe UI",system-ui,Roboto,"Noto Sans",Cantarell,sans-serif}
.hearth-classic h2{font-size:15px;font-weight:600;letter-spacing:.02em;text-transform:uppercase;color:${C.dim};margin:22px 0 10px}
.hearth-classic .hc-shelf{display:flex;gap:14px;overflow-x:auto;overflow-y:visible;padding:6px 4px 10px;scroll-padding:40px}
.hearth-classic .hc-shelf::-webkit-scrollbar{display:none}
.hearth-classic .hc-tile{flex:none;position:relative;padding:0 !important;min-width:0 !important;border-radius:10px !important;overflow:hidden;background:${C.panel2} !important;border:none !important;transition:transform .16s cubic-bezier(.2,.8,.2,1),box-shadow .16s}
.hearth-classic .hc-tile.gpfocus{transform:scale(1.045);z-index:2;box-shadow:0 0 0 2px ${C.bg},0 0 0 4px ${C.accent},0 8px 30px rgba(63,208,184,.18) !important}
.hearth-classic .hc-chip{flex:none;min-width:0 !important;width:auto !important;padding:8px 16px !important;border-radius:999px !important;background:${C.panel2} !important;color:${C.text} !important;font-size:14px}
.hearth-classic .hc-chip.gpfocus{background:${C.accent} !important;color:${C.ink} !important}
.hearth-classic .hc-btn{min-width:0 !important;width:auto !important;padding:10px 22px !important;border-radius:10px !important;background:${C.panel2} !important;color:${C.text} !important}
.hearth-classic .hc-btn.primary{background:${C.accent} !important;color:${C.ink} !important;font-weight:700}
.hearth-classic .hc-dim{color:${C.dim};font-size:13px}
.hearth-classic .hc-caption{font-size:13px;color:${C.text};margin:2px 0 4px 6px;min-height:18px}
.hearth-classic .hc-places{margin-top:10px}
.hearth-classic .hc-tile.big.gpfocus{transform:scale(1.03)}
`;

// ---- Hearth's sounds (ui/index.html blip, the Soft pack): only on this home, only when switched on.
let soundsOn = false;
let actx: AudioContext | null = null;
export function setSounds(on: boolean) { soundsOn = on; if (!on && actx) { try { actx.close(); } catch { /* fine */ } actx = null; } }
export function blip(kind: "move" | "ok" | "back") {
  if (!soundsOn) return;
  try {
    const AC = (window as any).AudioContext || (window as any).webkitAudioContext;
    if (!AC) return;
    if (!actx || actx.state === "closed") actx = new AC() as AudioContext;
    if (actx.state !== "running") actx.resume();
    const [f, d, v] = blipFor(kind);
    const o = actx.createOscillator(), g = actx.createGain();
    o.type = "sine"; o.frequency.value = f;
    g.gain.setValueAtTime(v, actx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.0001, actx.currentTime + d);
    o.connect(g).connect(actx.destination); o.start(); o.stop(actx.currentTime + d + 0.01);
  } catch { /* no sound, nothing else */ }
}
const onFocus = (e: any) => {
  blip("move");
  try { (e?.currentTarget || e?.target)?.scrollIntoView?.({ block: "nearest", inline: "nearest", behavior: "smooth" }); } catch { /* fine */ }
};

// ---- Steam's side (guarded: a Steam that changed just gives fewer pictures or the Hearth layout only)

function overview(appid: number): any {
  try { return (window as any).appStore?.GetAppOverviewByAppID?.(appid) ?? null; } catch { return null; }
}

export function steamUrls(appid: number, shape: Shape): string[] {
  const as = (window as any).appStore, ov = overview(appid);
  if (!as || !ov) return [];
  const out: string[] = [];
  for (const m of steamArtMethods(shape)) {
    try {
      const r = typeof as[m] === "function" ? as[m](ov) : null;
      for (const u of Array.isArray(r) ? r : [r]) if (typeof u === "string" && u && !out.includes(u)) out.push(u);
    } catch { /* not in this Steam */ }
  }
  return out;
}

/** Steam's library as plain data for homeFromSteam: its games, its collections (Hearth Sync's among them), Favorites. */
export function readSteam(): { apps: SteamApp[]; collections: { name: string; appids: number[] }[]; favorites: number[] } {
  const cs = (window as any).collectionStore, as = (window as any).appStore;
  const ids = (c: any): number[] => {
    try {
      if (c?.apps) return Array.from(typeof c.apps.keys === "function" ? c.apps.keys() : c.apps).map(Number).filter((n) => n > 0);
      if (Array.isArray(c?.allApps)) return c.allApps.map((a: any) => Number(a?.appid)).filter((n: number) => n > 0);
    } catch { /* fine */ }
    return [];
  };
  let apps: SteamApp[] = [];
  try {
    const list = cs?.allGamesCollection?.allApps ?? as?.allApps ?? [];
    apps = (Array.isArray(list) ? list : []).map((a: any) => ({ appid: Number(a?.appid), display_name: a?.display_name, rt_last_time_played: Number(a?.rt_last_time_played) || 0,
      app_type: Number(a?.app_type), minutes_playtime_forever: Number(a?.minutes_playtime_forever) || 0 }));
  } catch { apps = []; }
  let collections: { name: string; appids: number[] }[] = [];
  try { collections = (cs?.userCollections ?? []).map((c: any) => ({ name: String(c?.displayName ?? ""), appids: ids(c) })); } catch { collections = []; }
  let favorites: number[] = [];
  try { favorites = ids(cs?.GetCollection?.("favorite")); } catch { favorites = []; }
  return { apps, collections, favorites };
}

export function openApp(appid: number) {
  try { Navigation.Navigate(`/library/app/${appid}`); } catch (e) { console.warn("Hearth Classic: couldn't open", appid, e); }
}
function playApp(appid: number) {
  try {
    const ov = overview(appid);
    const run = (window as any).SteamClient?.Apps?.RunGame;
    if (typeof run === "function" && ov) { run(String(ov.m_gameid ?? appid), "", -1, 100); return; }
  } catch { /* fall through to the page */ }
  openApp(appid);
}
/** Steam's focused window instance (its Navigator opens Steam's own pages), or null. */
function steamWindow(): any {
  try { const w = (window as any).SteamUIStore?.GetFocusedWindowInstance?.(); if (w) return w; } catch { /* fall back */ }
  try { return (DUI as any).Router?.WindowStore?.GamepadUIMainWindowInstance ?? null; } catch { return null; }
}
/** Opens a place through Steam's own navigator (model.ts PLACES); never an empty page (the last step is the Library). */
export function openPlace(p: Place): string {
  const used = runPlace(p, steamWindow(), (r) => { try { Navigation.Navigate(r); } catch (e) { console.warn("Hearth Classic: couldn't open", r, e); } });
  (window as any).HC_LAST_PLACE = used;
  return used;
}

// ---- pictures

function canShow(url: string): Promise<boolean> {
  return new Promise((ok) => {
    try {
      const img = new Image();
      const t = setTimeout(() => ok(false), 8000);
      img.onload = () => { clearTimeout(t); ok(img.naturalWidth > 0); };
      img.onerror = () => { clearTimeout(t); ok(false); };
      img.src = url;
    } catch { ok(false); }
  });
}
const direct = (r: string) => /^(https?:|data:image\/|\/)/i.test(r) && !r.startsWith("//") && !/^\/(home|var|usr|run|mnt|media|tmp)\//.test(r);

/** The first picture that really shows, tried in order: Steam's addresses and Hearth's web pictures straight, files on
 *  this device through the backend. "" when none does (the card then shows its title on Hearth's panel colour). */
export function useArt(refs: string[]): string {
  const key = refs.join("\u0000");
  const [s, set] = useState<{ key: string; src: string }>({ key: "", src: "" });
  useEffect(() => {
    let live = true;
    (async () => {
      for (const r of refs) {
        let url = r;
        if (!direct(r)) { try { url = await askArt(r); } catch { url = ""; } }
        if (!live) return;
        if (url && await canShow(url)) { if (live) set({ key, src: url }); return; }
      }
      if (live) set({ key, src: "" });
    })();
    return () => { live = false; };
  }, [key]);
  return s.key === key ? s.src : "";
}

// ---- the parts

function TileArt(props: { card: Card; shape: Shape; badge?: boolean }) {
  const c = props.card;
  const src = useArt(artCandidates(c, props.shape, steamUrls(c.appid, props.shape)));
  return (
    <>
      {!src && <div style={{ position: "absolute", inset: 0, background: `linear-gradient(160deg,${C.panel3},${C.panel2} 55%,${C.panel})` }} />}
      {src && <img className="hc-art" src={src} alt="" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover", display: "block" }} />}
      <div className="hc-name" style={{ position: "absolute", left: 0, right: 0, bottom: 0, padding: src ? "26px 8px 7px" : "8px 8px 9px", fontSize: src ? 12 : 13,
           fontWeight: src ? 500 : 600, lineHeight: 1.25, textAlign: "left", whiteSpace: "normal", color: C.text, textShadow: "0 1px 2px rgba(0,0,0,.8)",
           background: src ? "linear-gradient(transparent,rgba(12,16,22,.92))" : "transparent" }}>{c.title}</div>
      {props.badge && c.store && <div className="hc-badge" style={{ position: "absolute", right: 7, top: 7, fontSize: 10, fontWeight: 700, letterSpacing: ".06em", textTransform: "uppercase",
           padding: "3px 7px", borderRadius: 6, background: "rgba(12,16,22,.72)", color: C.text }}>{c.store}</div>}
    </>
  );
}

// The old Game Mode's home rows (ui/index.html viewHome): "Recent games" with the newest one big and wide, then
// portrait cards; store badges top right; the focused game named under its row.
type FocusCard = (c: Card) => void;
function Tile(props: { card: Card; big?: boolean; badge?: boolean; onFocusCard?: FocusCard }) {
  const w = props.big ? 330 : 137, h = 206;
  return (
    <div className="hc-tile-wrap" style={{ display: "contents" }}>
      <DialogButton className={"hc-tile" + (props.big ? " big" : "")} style={{ width: w, height: h }} onClick={() => { blip("ok"); openApp(props.card.appid); }}
        {...({ onGamepadFocus: (e: any) => { onFocus(e); props.onFocusCard?.(props.card); } } as any)}>
        <TileArt card={props.card} shape={props.big ? "wide" : "portrait"} badge={props.badge} />
      </DialogButton>
    </div>
  );
}

function ShelfRow(props: { shelf: Shelf; focused: Card | null; onFocusCard: FocusCard }) {
  const s = props.shelf;
  const recent = s.key === "continue";
  const here = props.focused && s.items.some((c) => c.appid === props.focused!.appid) ? props.focused : null;
  return (
    <>
      <h2>{recent ? "Recent games" : s.title}</h2>
      <Focusable className="hc-shelf" {...({ "flow-children": "horizontal" } as any)}>
        {s.items.map((c, i) => <Tile key={c.appid} card={c} big={recent && i === 0} badge onFocusCard={props.onFocusCard} />)}
      </Focusable>
      {recent && <div className="hc-caption">{here ? [here.title, played(here.minutes)].filter(Boolean).join(" · ") : "\u00a0"}</div>}
    </>
  );
}

function Places() {
  return (
    <Focusable className="hc-shelf hc-places" style={{ gap: 8 }} {...({ "flow-children": "horizontal" } as any)}>
      {PLACES.map((p) => (
        <DialogButton key={p.id} className="hc-chip" onClick={() => { blip("ok"); openPlace(p); }} {...({ onGamepadFocus: onFocus } as any)}>{p.title}</DialogButton>
      ))}
    </Focusable>
  );
}

/** The old home's banner (ui/index.html .hero): the focused game's art, soft and dark, fading into the page, with
 *  Hearth's logo and the page's name where the old top bar had them. */
function Banner(props: { card: Card | null; logo: string }) {
  const c = props.card;
  const art = useArt(artCandidates(c, "hero", c ? steamUrls(c.appid, "hero") : []));
  return (
    <div className="hc-banner" style={{ position: "relative", height: "27vh", minHeight: 170, margin: "-54px -2.8vw 0", overflow: "hidden" }}>
      {art && <div style={{ position: "absolute", inset: "-30px", background: `url("${art}") center/cover`, filter: "blur(28px) saturate(1.2)", opacity: 0.55 }} />}
      <div style={{ position: "absolute", inset: 0, background: `linear-gradient(180deg,rgba(12,16,22,.2),${C.bg})` }} />
      <div style={{ position: "absolute", top: 54, left: "2.8vw", display: "flex", alignItems: "center", gap: 10 }}>
        {props.logo && <img src={props.logo} style={{ width: 26, height: 26 }} />}
        <div style={{ fontSize: 16, fontWeight: 600, color: C.text }}>Home</div>
      </div>
    </div>
  );
}

/** The home's data: Hearth's layout (asked again every minute), or Steam's library when Hearth doesn't answer. */
let lastHome: HomeView | null = null;
function useHome(): { view: HomeView | null; loading: boolean } {
  const [view, setView] = useState<HomeView | null>(lastHome);
  const [loading, setLoading] = useState(lastHome === null);
  useEffect(() => {
    let live = true;
    const load = async () => {
      let hearth: HomeView | null = null;
      try {
        const r = await askHome();
        if (r && r.ok) hearth = homeFromHearth(r.home, r.shortcuts || {});
      } catch { hearth = null; }
      let steam: HomeView | null = null;
      if (!hearth || !hearth.shelves.length) { const s = readSteam(); steam = homeFromSteam(s.apps, s.collections, s.favorites); }
      const v = pickHome(hearth, steam);
      lastHome = v;
      if (live) { setView(v); setLoading(false); }
    };
    load();
    const t = setInterval(load, 60000);
    return () => { live = false; clearInterval(t); };
  }, []);
  return { view, loading };
}

/** Hearth Classic's home in place of Steam's. Nothing to show: Steam's own Home. */
export function HearthHome(props: { steam: ReactNode; wallpaper: string; logo: string }) {
  const { view, loading } = useHome();
  const [picked, setFocused] = useState<Card | null>(null);
  if (!loading && !view) return <>{props.steam}</>;
  const focused = picked || (view ? view.hero : null);
  const bg = props.wallpaper ? { background: `linear-gradient(180deg,rgba(12,16,22,.35),rgba(12,16,22,.9) 50%,${C.bg}),url("${props.wallpaper}") center/cover fixed` } : undefined;
  return (
    <div className="hearth-classic" style={bg}>
      <style>{CSS}</style>
      <div style={{ padding: "54px 2.8vw 80px" }}>
        <Banner card={focused} logo={props.logo} />
        {loading && <div className="hc-dim" style={{ marginTop: 18 }}>Getting your games…</div>}
        {view && view.shelves.slice(0, 1).map((s) => <ShelfRow key={s.key} shelf={s} focused={focused} onFocusCard={setFocused} />)}
        <Places />
        {view && view.shelves.slice(1).map((s) => <ShelfRow key={s.key} shelf={s} focused={focused} onFocusCard={setFocused} />)}
      </div>
    </div>
  );
}

/** Anything that throws inside shows `fallback` (Steam's own Home) and reports it once (Hearth Classic then pauses). */
export class Boundary extends Component<{ fallback: ReactNode; onError: (e: unknown) => void; children?: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(e: unknown) { try { this.props.onError(e); } catch { /* never throw from here */ } }
  render() { return this.state.failed ? this.props.fallback : this.props.children; }
}
