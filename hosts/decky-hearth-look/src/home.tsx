// Stage 2, the home screen: Hearth Classic's home on Steam's Home route (the old Hearth Game Mode's home, ui/index.html
// viewHome, as the archived takeover drew it): Hearth's wallpaper and logo, Jump back in, Hearth's places, Continue
// playing, Favorites and a row per store. Made of @decky/ui's Focusable and DialogButton, so Steam's own controller
// focus moves through it. Every game is one Steam can open, and opens Steam's own page. The data is Hearth's own
// layout (the backend asks `nimbus --steam-mode-data home` once a minute; no second Hearth runs), or Steam's library
// and Hearth Sync's collections when Hearth's program doesn't answer.
import { DialogButton, Focusable, Navigation } from "@decky/ui";
import { callable } from "@decky/api";
import { Component, ReactNode, useEffect, useState } from "react";
import { Card, HomeView, PLACES, Shape, Shelf, SteamApp, artCandidates, blipFor, homeFromHearth, homeFromSteam, pickHome, played, steamArtMethods } from "./model";

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
function openPlace(route: string, id: string) {
  try {
    const nav = Navigation as any;
    if (id === "store" && typeof nav.NavigateToStorePage === "function") { nav.NavigateToStorePage(); return; }
    nav.Navigate(route);
  } catch (e) { console.warn("Hearth Classic: couldn't open", route, e); }
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

function Tile(props: { card: Card; wide?: boolean; badge?: boolean }) {
  const w = props.wide ? 300 : 132, h = props.wide ? 150 : 188;
  return (
    <DialogButton className="hc-tile" style={{ width: w, height: h }} onClick={() => { blip("ok"); openApp(props.card.appid); }}
      {...({ onGamepadFocus: onFocus } as any)}>
      <TileArt card={props.card} shape={props.wide ? "wide" : "portrait"} badge={props.badge} />
    </DialogButton>
  );
}

function ShelfRow(props: { shelf: Shelf }) {
  const s = props.shelf;
  return (
    <>
      <h2>{s.title}</h2>
      <Focusable className="hc-shelf" {...({ "flow-children": "horizontal" } as any)}>
        {s.items.map((c) => <Tile key={c.appid} card={c} wide={s.wide} badge={s.key === "continue" || s.key === "fav" || s.key === "added"} />)}
      </Focusable>
    </>
  );
}

function Places() {
  return (
    <Focusable className="hc-shelf" style={{ gap: 8 }} {...({ "flow-children": "horizontal" } as any)}>
      {PLACES.map((p) => (
        <DialogButton key={p.id} className="hc-chip" onClick={() => { blip("ok"); openPlace(p.route, p.id); }} {...({ onGamepadFocus: onFocus } as any)}>{p.title}</DialogButton>
      ))}
    </Focusable>
  );
}

function Hero(props: { card: Card | null; logo: string }) {
  const c = props.card;
  const art = useArt(artCandidates(c, "hero", c ? steamUrls(c.appid, "hero") : []));
  const logo = useArt(artCandidates(c, "logo", c ? steamUrls(c.appid, "logo") : []));
  return (
    <div style={{ position: "relative", height: "36vh", minHeight: 210, margin: "-54px -2.8vw 10px", padding: "54px 2.8vw 18px", display: "flex", flexDirection: "column", justifyContent: "flex-end",
                  background: art ? `linear-gradient(90deg,rgba(12,16,22,.92),rgba(12,16,22,.2) 70%),linear-gradient(0deg,${C.bg},transparent 45%),url("${art}") center/cover` : undefined }}>
      <div style={{ position: "absolute", top: 54, left: "2.8vw", display: "flex", alignItems: "center", gap: 10 }}>
        {props.logo && <img src={props.logo} style={{ width: 28, height: 28 }} />}
        <div style={{ fontSize: 15, fontWeight: 600, letterSpacing: ".02em", textTransform: "uppercase", color: C.dim }}>Hearth</div>
      </div>
      {c && (
        <>
          <div style={{ fontSize: 12, fontWeight: 700, letterSpacing: ".08em", textTransform: "uppercase", color: C.accent }}>Jump back in</div>
          {logo ? <img src={logo} style={{ maxHeight: 90, maxWidth: 380, objectFit: "contain", objectPosition: "left", margin: "6px 0" }} />
                : <div style={{ fontSize: 34, fontWeight: 800, margin: "4px 0" }}>{c.title}</div>}
          <div className="hc-dim" style={{ marginBottom: 12 }}>{[c.store, played(c.minutes)].filter(Boolean).join(" · ")}</div>
          <Focusable style={{ display: "flex", gap: 10 }} {...({ "flow-children": "horizontal" } as any)}>
            <DialogButton className="hc-btn primary" onClick={() => { blip("ok"); playApp(c.appid); }} {...({ onGamepadFocus: onFocus, autoFocus: true } as any)}>Play</DialogButton>
            <DialogButton className="hc-btn" onClick={() => { blip("ok"); openApp(c.appid); }} {...({ onGamepadFocus: onFocus } as any)}>Details</DialogButton>
          </Focusable>
        </>
      )}
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
  if (!loading && !view) return <>{props.steam}</>;
  const bg = props.wallpaper ? { background: `linear-gradient(180deg,rgba(12,16,22,.35),rgba(12,16,22,.9) 50%,${C.bg}),url("${props.wallpaper}") center/cover fixed` } : undefined;
  return (
    <div className="hearth-classic" style={bg}>
      <style>{CSS}</style>
      <div style={{ padding: "54px 2.8vw 80px" }}>
        <Hero card={view ? view.hero : null} logo={props.logo} />
        <Places />
        {loading && <div className="hc-dim" style={{ marginTop: 18 }}>Getting your games…</div>}
        {view && view.shelves.map((s) => <ShelfRow key={s.key} shelf={s} />)}
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
