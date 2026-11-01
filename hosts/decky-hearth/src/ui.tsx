// The takeover's building blocks in Hearth's look (ui/index.html: tiles, shelves, the hero, chips), made of
// @decky/ui's Focusable and DialogButton so Steam's own controller navigation moves between them.
import { DialogButton, Focusable, Navigation } from "@decky/ui";
import { Component, ReactNode } from "react";
import { usePic } from "./api";
import { Game, blipFor } from "./model";

export const C = { bg: "#0c1016", bg2: "#121821", panel: "#161d27", panel2: "#1d2531", panel3: "#273140", line: "#2a3443", text: "#e9eef4", dim: "#8d99a8", accent: "#3fd0b8", ink: "#06231e" };

// Hearth's page styles (scoped to .hearth-tk): the tile focus is Hearth's double ring and lift.
export const CSS = `
.hearth-tk{position:absolute;inset:0;overflow-y:auto;overflow-x:hidden;color:${C.text};background:${C.bg};font-family:"Segoe UI Variable Text","Segoe UI",system-ui,Roboto,"Noto Sans",Cantarell,sans-serif}
.hearth-tk h2{font-size:15px;font-weight:600;letter-spacing:.02em;text-transform:uppercase;color:${C.dim};margin:22px 0 10px}
.hearth-tk .ht-shelf{display:flex;gap:14px;overflow-x:auto;overflow-y:visible;padding:6px 4px 10px;scroll-padding:40px}
.hearth-tk .ht-tile{flex:none;position:relative;padding:0 !important;min-width:0 !important;border-radius:10px !important;overflow:hidden;background:${C.panel2} !important;border:none !important;transition:transform .16s cubic-bezier(.2,.8,.2,1),box-shadow .16s}
.hearth-tk .ht-tile.gpfocus{transform:translateY(-3px) scale(1.03);box-shadow:0 0 0 2px ${C.bg},0 0 0 4px ${C.accent},0 8px 30px rgba(63,208,184,.18) !important}
.hearth-tk .ht-tile img{position:absolute;inset:0;width:100%;height:100%;object-fit:cover}
.hearth-tk .ht-name{position:absolute;left:0;right:0;bottom:0;padding:22px 8px 7px;font-size:12px;line-height:1.25;text-align:left;white-space:normal;background:linear-gradient(transparent,rgba(12,16,22,.92));color:${C.text}}
.hearth-tk .ht-badge{position:absolute;left:6px;top:6px;font-size:10px;font-weight:700;padding:2px 6px;border-radius:8px;background:${C.accent};color:${C.ink}}
.hearth-tk .ht-chip{flex:none;min-width:0 !important;width:auto !important;padding:8px 16px !important;border-radius:999px !important;background:${C.panel2} !important;color:${C.text} !important;font-size:14px}
.hearth-tk .ht-chip.on{background:${C.accent} !important;color:${C.ink} !important}
.hearth-tk .ht-card{background:${C.panel};border:1px solid ${C.line};border-radius:12px;padding:14px 16px;margin-bottom:10px}
.hearth-tk .ht-dim{color:${C.dim};font-size:13px}
.hearth-tk .ht-bar{height:6px;border-radius:3px;background:${C.panel3};overflow:hidden;margin-top:8px}
.hearth-tk .ht-bar>i{display:block;height:100%;background:${C.accent}}
.hearth-tk .ht-btn{min-width:0 !important;width:auto !important;padding:10px 20px !important;border-radius:10px !important}
.hearth-tk .ht-btn.primary{background:${C.accent} !important;color:${C.ink} !important;font-weight:700}
`;

// ---- Hearth's sounds (ui/index.html blip, the Soft pack), only on Hearth's pages and only when switched on.
let soundsOn = false;
let actx: AudioContext | null = null;
export function setSounds(on: boolean) { soundsOn = on; }
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

/** Focus: Hearth's sound, and the focused tile scrolled into view (Steam usually does this itself). */
export const onFocus = (e: any) => {
  blip("move");
  try { (e?.currentTarget || e?.target)?.scrollIntoView?.({ block: "nearest", inline: "nearest", behavior: "smooth" }); } catch { /* fine */ }
};

/** A page's frame: Hearth's background (the wallpaper, darkened like Hearth's home), its styles, a back button. */
export function Page(props: { title?: string; wallpaper?: string; children?: ReactNode; noBack?: boolean }) {
  return (
    <div className="hearth-tk" style={props.wallpaper ? { background: `linear-gradient(180deg,rgba(12,16,22,.35),rgba(12,16,22,.9) 50%,${C.bg}),url(${props.wallpaper}) center/cover fixed` } : undefined}>
      <style>{CSS}</style>
      <div style={{ padding: "54px 2.8vw 80px" }}>
        {props.title && <div style={{ fontSize: 26, fontWeight: 700, marginBottom: 6 }}>{props.title}</div>}
        {props.children}
      </div>
    </div>
  );
}

/** A game's tile (portrait, or wide for Continue playing). */
export function Tile(props: { game: Game; wide?: boolean; badge?: boolean; onActivate: (g: Game) => void; onMenu?: (g: Game) => void }) {
  const g = props.game;
  const src = usePic(props.wide ? g.wide || g.hero || g.cover : g.cover || g.wide);
  const w = props.wide ? 300 : 132, h = props.wide ? 150 : 188;
  return (
    <DialogButton className="ht-tile" style={{ width: w, height: h }}
      onClick={() => { blip("ok"); props.onActivate(g); }}
      {...({ onGamepadFocus: onFocus, onSecondaryButton: props.onMenu ? () => props.onMenu!(g) : undefined, onSecondaryActionDescription: props.onMenu ? "Hearth info" : undefined } as any)}>
      {src && <img src={src} />}
      {(!src || props.wide) && <div className="ht-name">{g.title}</div>}
      {props.badge && <div className="ht-badge">{g.store}</div>}
    </DialogButton>
  );
}

/** A row of tiles, moved through with left and right. */
export function Shelf(props: { title: string; games: Game[]; wide?: boolean; badge?: boolean; onActivate: (g: Game) => void; onMenu?: (g: Game) => void; extra?: ReactNode }) {
  if (!props.games.length && !props.extra) return null;
  return (
    <>
      <h2>{props.title}</h2>
      <Focusable className="ht-shelf" {...({ "flow-children": "horizontal" } as any)}>
        {props.games.map((g) => <Tile key={g.id} game={g} wide={props.wide} badge={props.badge} onActivate={props.onActivate} onMenu={props.onMenu} />)}
        {props.extra}
      </Focusable>
    </>
  );
}

/** Chips (Hearth's tabs): left and right between them, the chosen one filled. */
export function Chips(props: { items: [string, string][]; value: string; onChange: (v: string) => void }) {
  return (
    <Focusable className="ht-shelf" style={{ gap: 8 }} {...({ "flow-children": "horizontal" } as any)}>
      {props.items.map(([k, label]) => (
        <DialogButton key={k} className={"ht-chip" + (k === props.value ? " on" : "")} onClick={() => { blip("ok"); props.onChange(k); }}
          {...({ onGamepadFocus: onFocus } as any)}>{label}</DialogButton>
      ))}
    </Focusable>
  );
}

export function go(route: string) {
  try { Navigation.Navigate(route); Navigation.CloseSideMenus?.(); } catch (e) { console.warn("Hearth: couldn't open", route, e); }
}

/** Anything that throws inside shows `fallback` (Steam's own screen, for the home) and is reported once. */
export class Boundary extends Component<{ part: string; fallback?: ReactNode; onError: (part: string, e: unknown) => void; children?: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(e: unknown) { try { this.props.onError(this.props.part, e); } catch { /* never throw from here */ } }
  render() {
    if (this.state.failed) return this.props.fallback ?? <div style={{ padding: 60, color: C.dim }}>This Hearth page couldn't be shown. Steam's own screens still work.</div>;
    return this.props.children;
  }
}
