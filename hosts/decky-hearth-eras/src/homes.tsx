// The eras' home screens on Steam's Home route: Hearth's own designs, each evoking a console era by layout, colour and
// motion. Made of @decky/ui's Focusable and DialogButton, so Steam's own controller focus moves through them. Every
// game opens Steam's own game page (Play stays Steam's), the places open Steam's own pages through its navigator (the
// way Hearth Classic proved on a device), and with nothing to show Steam's own Home stays. The data is Hearth Classic's:
// Hearth's home layout, or Steam's library and Hearth Sync's collections when Hearth's program doesn't answer.
import { DialogButton, Focusable } from "@decky/ui";
import { callable } from "@decky/api";
import { CSSProperties, ReactNode, useEffect, useState } from "react";
import { FaBook, FaClock, FaCog, FaDownload, FaGamepad, FaImages, FaLayerGroup, FaShoppingBag, FaStar, FaThLarge, FaUserFriends } from "react-icons/fa";
import type { Card, HomeView, Place, Shape } from "./classic/model";
import { artCandidates, homeFromHearth, homeFromSteam, pickHome } from "./classic/model";
import { HearthHome, openApp, openPlace, readSteam, steamUrls, useArt } from "./classic/home";
import { play } from "./extras";
import type { EraId, Preset, Section } from "./model";
import { ERA_PLACES, clock, dateLine, gridGames, sectionsOf, subtitle } from "./model";

const askHome = callable<[], { ok: boolean; home?: unknown; shortcuts?: Record<string, unknown>; error?: string }>("home");

// ---------------------------------------------------------------- shared parts

/** The home's data: Hearth's layout (asked again every minute), or Steam's library when Hearth doesn't answer. */
let lastHome: HomeView | null = null;
function useHomeView(): { view: HomeView | null; loading: boolean } {
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

function useNow(): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => { const t = setInterval(() => setNow(new Date()), 30000); return () => clearInterval(t); }, []);
  return now;
}

const H = { "flow-children": "horizontal" } as any;
const V = { "flow-children": "vertical" } as any;

// Steam moves its focus with the controller (onGamepadFocus); the browser's own focus (onFocus) is the same moment in
// the CI stand-in. One sound and one scroll per focus, whichever comes.
let lastEl: unknown = null;
let lastAt = 0;
function focusProps(cb?: () => void): any {
  const f = (e: any) => {
    const el = e?.currentTarget || e?.target;
    const now = Date.now();
    if (el === lastEl && now - lastAt < 150) return;
    lastEl = el; lastAt = now;
    play("move");
    try { el?.scrollIntoView?.({ block: "nearest", inline: "nearest", behavior: "smooth" }); } catch { /* fine */ }
    try { cb?.(); } catch { /* fine */ }
  };
  return { onGamepadFocus: f, onFocus: f };
}

function varsOf(p: Preset | null, wallpaper = ""): CSSProperties {
  const o: Record<string, string> = {};
  for (const [k, v] of Object.entries(p ? p.vars : {})) o["--era-" + k] = v;
  if (wallpaper) o["--era-wall"] = `url("${wallpaper}")`;
  return o as unknown as CSSProperties;
}

const PLACE_ICONS: Record<string, ReactNode> = { library: <FaBook />, store: <FaShoppingBag />, downloads: <FaDownload />, media: <FaImages />, friends: <FaUserFriends />, settings: <FaCog /> };
function sectionIcon(key: string): ReactNode {
  return key === "continue" ? <FaClock /> : key === "fav" ? <FaStar /> : key === "row-steam" ? <FaGamepad /> : key === "places" ? <FaThLarge /> : <FaLayerGroup />;
}

function Art(props: { card: Card; shape: Shape; className?: string }) {
  const c = props.card;
  const src = useArt(artCandidates(c, props.shape, steamUrls(c.appid, props.shape)));
  return src ? <img className={"era-art " + (props.className || "")} src={src} alt="" /> : <div className={"era-noart " + (props.className || "")}><span>{c.title}</span></div>;
}

/** One game: opens Steam's own page. data-hearth-appid lets Hearth Music play its theme when it's focused. */
function Game(props: { card: Card; shape: Shape; className?: string; style?: CSSProperties; onFocusCard?: (c: Card) => void; children?: ReactNode }) {
  const c = props.card;
  return (
    <div className="era-wrap" data-hearth-appid={c.appid} style={{ display: "contents" }}>
      <DialogButton className={"era-game " + (props.className || "")} style={props.style}
        onClick={() => { play("ok"); openApp(c.appid); }} {...focusProps(() => props.onFocusCard?.(c))}>
        <Art card={c} shape={props.shape} />
        {props.children}
      </DialogButton>
    </div>
  );
}

function PlaceButton(props: { place: Place; className?: string; label?: boolean; onFocusPlace?: (p: Place) => void }) {
  const p = props.place;
  return (
    <DialogButton className={"era-place " + (props.className || "")} onClick={() => { play("ok"); openPlace(p); }} {...focusProps(() => props.onFocusPlace?.(p))}>
      <span className="era-icon">{PLACE_ICONS[p.id] || <FaThLarge />}</span>
      {props.label !== false && <span className="era-place-name">{p.title}</span>}
    </DialogButton>
  );
}

const BASE = `
.era-home{position:absolute;inset:0;overflow:hidden;font-family:var(--era-font,"Noto Sans",system-ui,sans-serif);color:var(--era-text)}
.era-home *{box-sizing:border-box}
.era-home .era-game,.era-home .era-place{position:relative;padding:0 !important;min-width:0 !important;border:none !important;overflow:hidden;cursor:pointer}
.era-home .era-art{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;display:block}
.era-home .era-noart{position:absolute;inset:0;display:flex;align-items:flex-end;padding:10px;font-size:14px;font-weight:700;line-height:1.2;text-align:left;background:linear-gradient(150deg,var(--era-panel2),var(--era-panel));color:var(--era-text)}
.era-home .era-icon{display:inline-flex}
.era-home .era-loading{position:absolute;left:0;right:0;top:46%;text-align:center;font-size:16px;color:var(--era-dim)}
`;

// ---------------------------------------------------------------- Channels: a rounded TV-channel grid on silver

const CHANNELS = `
.era-channels{background:repeating-linear-gradient(180deg,var(--era-bg) 0 3px,var(--era-bg2) 3px 4px);font-family:"Nunito","Noto Sans",system-ui,sans-serif}
.era-channels .ch-grid{position:absolute;left:5vw;right:5vw;top:76px;bottom:178px;display:flex;flex-direction:column;gap:2.4vh}
.era-channels .ch-row{display:flex;gap:1.8vw;flex:1;min-height:0}
.era-channels .era-game,.era-channels .ch-empty{flex:1 1 0;height:100%;border-radius:24px !important;background:var(--era-panel2) !important;border:3px solid var(--era-line) !important;box-shadow:inset 0 2px 8px rgba(0,0,0,.06),0 2px 0 rgba(255,255,255,.7);transition:transform .22s cubic-bezier(.2,.8,.2,1),box-shadow .22s}
.era-channels .era-game .era-art{border-radius:20px}
.era-channels .ch-empty{background:repeating-linear-gradient(135deg,rgba(255,255,255,.4) 0 10px,rgba(255,255,255,.15) 10px 20px) !important;opacity:.7}
.era-channels .era-game.gpfocus{transform:scale(1.06);z-index:2;border-color:var(--era-accent) !important;box-shadow:0 0 0 4px rgba(255,255,255,.95),0 0 0 7px var(--era-accent),0 12px 30px rgba(47,166,214,.3) !important}
.era-channels .ch-caption{position:absolute;left:0;right:0;bottom:122px;text-align:center;font-size:20px;font-weight:700;color:var(--era-text);min-height:28px}
.era-channels .ch-caption small{display:block;font-size:13px;font-weight:600;color:var(--era-dim)}
.era-channels .ch-tray{position:absolute;left:0;right:0;bottom:0;height:108px;display:flex;align-items:center;gap:22px;padding:0 4vw;background:linear-gradient(180deg,var(--era-panel),var(--era-bg2));border-top:2px solid var(--era-line);box-shadow:0 -8px 30px rgba(0,0,0,.06)}
.era-channels .ch-clock{font-size:44px;font-weight:300;letter-spacing:.05em;color:var(--era-dim);line-height:1}
.era-channels .ch-date{font-size:15px;font-weight:700;color:var(--era-dim);letter-spacing:.06em;text-transform:uppercase}
.era-channels .ch-places{margin-left:auto;display:flex;gap:10px}
.era-channels .era-place{display:flex !important;align-items:center;gap:8px;width:auto !important;padding:11px 18px !important;border-radius:999px !important;background:var(--era-panel2) !important;color:var(--era-text) !important;border:2px solid var(--era-line) !important;font-size:15px;font-weight:700}
.era-channels .era-place.gpfocus{background:var(--era-accent) !important;color:#fff !important;border-color:transparent !important;box-shadow:0 6px 18px rgba(47,166,214,.35) !important}
`;

function Channels(props: { view: HomeView }) {
  const now = useNow();
  const games = gridGames(props.view, 12);
  const [focused, setFocused] = useState<Card | null>(null);
  const rows = [0, 1, 2].map((r) => Array.from({ length: 4 }, (_, i) => games[r * 4 + i] || null));
  return (
    <div className="era-home era-channels">
      <style>{CHANNELS}</style>
      <Focusable className="ch-grid" {...V}>
        {rows.map((row, r) => (
          <Focusable key={r} className="ch-row" {...H}>
            {row.map((c, i) => c ? <Game key={c.appid} card={c} shape="wide" onFocusCard={setFocused} /> : <div key={"e" + i} className="ch-empty" />)}
          </Focusable>
        ))}
      </Focusable>
      <div className="ch-caption">{focused ? <>{focused.title}<small>{subtitle(focused)}</small></> : " "}</div>
      <div className="ch-tray">
        <div><div className="ch-clock">{clock(now)}</div><div className="ch-date">{dateLine(now)}</div></div>
        <Focusable className="ch-places" {...H}>{ERA_PLACES.map((p) => <PlaceButton key={p.id} place={p} />)}</Focusable>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- Blades: glossy panels side by side

const BLADES = `
.era-blades{background:radial-gradient(120% 90% at 85% 0%,var(--era-panel2),var(--era-bg) 55%,var(--era-bg2))}
.era-blades .bl-row{position:absolute;left:3vw;right:3vw;top:72px;bottom:6vh;display:flex;gap:8px}
.era-blades .bl-blade{display:flex;flex:0 0 66px;min-width:0;border-radius:6px;overflow:hidden;background:linear-gradient(180deg,rgba(255,255,255,.16),rgba(255,255,255,0) 38%),var(--era-panel);box-shadow:inset 0 1px 0 rgba(255,255,255,.3),0 14px 34px rgba(0,0,0,.35);clip-path:polygon(16px 0,100% 0,calc(100% - 16px) 100%,0 100%);transition:flex .38s cubic-bezier(.2,.8,.2,1),background .3s}
.era-blades .bl-blade.on{flex:1 1 auto;background:linear-gradient(180deg,rgba(255,255,255,.2),rgba(255,255,255,0) 30%),var(--era-panel2)}
.era-blades .bl-tab{flex:none;width:66px !important;height:100%;padding:0 !important;min-width:0 !important;background:transparent !important;border:none !important;color:var(--era-dim) !important;writing-mode:vertical-rl;transform:rotate(180deg);font-size:19px;font-weight:800;letter-spacing:.14em;text-transform:uppercase;display:flex !important;align-items:center;justify-content:center;gap:10px}
.era-blades .bl-tab.gpfocus,.era-blades .bl-blade.on .bl-tab{color:var(--era-accent) !important;box-shadow:none !important;text-shadow:0 0 14px var(--era-accent)}
.era-blades .bl-body{flex:1;min-width:0;padding:26px 30px 20px 4px;display:flex;flex-direction:column;overflow:hidden}
.era-blades .bl-title{font-size:34px;font-weight:800;letter-spacing:.02em;color:var(--era-text)}
.era-blades .bl-sub{font-size:14px;color:var(--era-dim);margin:2px 0 16px;min-height:18px}
.era-blades .bl-grid{display:flex;flex-direction:column;gap:14px;overflow-y:auto;padding:4px}
.era-blades .bl-line{display:flex;gap:14px}
.era-blades .era-game{width:250px;height:140px;flex:none;border-radius:4px !important;background:var(--era-panel) !important;box-shadow:inset 0 1px 0 rgba(255,255,255,.25)}
.era-blades .era-game.gpfocus{box-shadow:0 0 0 3px var(--era-accent),0 0 26px rgba(255,255,255,.18) !important;transform:translateY(-3px)}
.era-blades .era-place{display:flex !important;align-items:center;gap:14px;width:250px !important;height:64px;padding:0 18px !important;border-radius:4px !important;background:var(--era-panel) !important;color:var(--era-text) !important;font-size:18px;font-weight:700}
.era-blades .era-place.gpfocus{background:var(--era-accent) !important;color:var(--era-ink) !important}
.era-blades .era-place .era-icon{font-size:22px}
`;

function chunk<T>(l: T[], n: number): T[][] { const out: T[][] = []; for (let i = 0; i < l.length; i += n) out.push(l.slice(i, i + n)); return out; }

function Blades(props: { view: HomeView }) {
  const secs: (Section | null)[] = [...sectionsOf(props.view, 12), null];   // null: the places' blade
  const [on, setOn] = useState(0);
  const [focused, setFocused] = useState<Card | null>(null);
  return (
    <div className="era-home era-blades">
      <style>{BLADES}</style>
      <Focusable className="bl-row" {...H}>
        {secs.map((s, i) => {
          const title = s ? s.title : "Places";
          return (
            <div key={s ? s.key : "places"} className={"bl-blade" + (i === on ? " on" : "")}>
              <DialogButton className="bl-tab" onClick={() => setOn(i)} {...focusProps(() => setOn(i))}>
                <span>{title}</span>
              </DialogButton>
              {i === on && (
                <div className="bl-body">
                  <div className="bl-title">{title}</div>
                  <div className="bl-sub">{s ? (focused && s.items.some((c) => c.appid === focused.appid) ? `${focused.title} · ${subtitle(focused)}` : `${s.items.length} games`) : "Steam's own pages"}</div>
                  <Focusable className="bl-grid" {...V}>
                    {s ? chunk(s.items, 3).map((line, n) => (
                      <Focusable key={n} className="bl-line" {...H}>{line.map((c) => <Game key={c.appid} card={c} shape="wide" onFocusCard={setFocused} />)}</Focusable>
                    )) : chunk(ERA_PLACES, 3).map((line, n) => (
                      <Focusable key={n} className="bl-line" {...H}>{line.map((p) => <PlaceButton key={p.id} place={p} />)}</Focusable>
                    ))}
                  </Focusable>
                </div>
              )}
            </div>
          );
        })}
      </Focusable>
    </div>
  );
}

// ---------------------------------------------------------------- Tiles: a big-tile dashboard with a hero banner

const TILES = `
.era-tiles{background:var(--era-bg);overflow-y:auto}
.era-tiles .tl-hero{position:relative;height:46vh;min-height:250px;overflow:hidden}
.era-tiles .tl-hero .era-art,.era-tiles .tl-hero .era-noart{object-position:center 25%}
.era-tiles .tl-hero:after{content:"";position:absolute;inset:0;background:linear-gradient(0deg,var(--era-bg) 0%,rgba(0,0,0,0) 55%),linear-gradient(90deg,var(--era-bg) 0%,rgba(0,0,0,.2) 45%,rgba(0,0,0,0) 70%)}
.era-tiles .tl-info{position:absolute;left:4vw;bottom:84px;z-index:1;max-width:46vw}
.era-tiles .tl-kicker{font-size:13px;font-weight:700;letter-spacing:.12em;text-transform:uppercase;color:var(--era-accent)}
.era-tiles .tl-title{font-size:40px;font-weight:800;line-height:1.1;margin-top:4px;text-shadow:0 2px 14px rgba(0,0,0,.6)}
.era-tiles .tl-sub{font-size:15px;color:var(--era-dim);margin-top:6px}
.era-tiles .tl-rows{position:relative;margin-top:-70px;padding:0 4vw 70px;z-index:1}
.era-tiles .tl-h{font-size:15px;font-weight:700;color:var(--era-dim);margin:18px 0 8px}
.era-tiles .tl-strip{display:flex;gap:12px;overflow-x:auto;padding:6px 4px 10px}
.era-tiles .tl-strip::-webkit-scrollbar{display:none}
.era-tiles .era-game{flex:none;width:136px;height:204px;border-radius:6px !important;background:var(--era-panel) !important;transition:transform .16s,box-shadow .16s}
.era-tiles .era-game.big{width:384px;height:204px}
.era-tiles .era-game.gpfocus{transform:scale(1.05);z-index:2;box-shadow:0 0 0 3px var(--era-text),0 12px 30px rgba(0,0,0,.55) !important}
.era-tiles .era-place{flex:none;display:flex !important;flex-direction:column;align-items:flex-start;justify-content:space-between;width:150px !important;height:96px;padding:14px !important;border-radius:6px !important;background:var(--era-panel2) !important;color:var(--era-text) !important;font-size:15px;font-weight:700;text-align:left}
.era-tiles .era-place .era-icon{font-size:24px;color:var(--era-accent)}
.era-tiles .era-place.gpfocus{background:var(--era-accent) !important;color:var(--era-ink) !important;box-shadow:0 0 0 3px var(--era-text) !important}
.era-tiles .era-place.gpfocus .era-icon{color:var(--era-ink)}
`;

function Tiles(props: { view: HomeView }) {
  const secs = sectionsOf(props.view, 20);
  const [focused, setFocused] = useState<Card | null>(null);
  const hero = focused || props.view.hero || (secs[0] && secs[0].items[0]) || null;
  return (
    <div className="era-home era-tiles">
      <style>{TILES}</style>
      <div className="tl-hero">
        {hero && <Art key={hero.appid} card={hero} shape="hero" />}
        {hero && <div className="tl-info"><div className="tl-kicker">{focused ? "Selected" : "Jump back in"}</div><div className="tl-title">{hero.title}</div><div className="tl-sub">{subtitle(hero)}</div></div>}
      </div>
      <Focusable className="tl-rows" {...V}>
        {secs.slice(0, 1).map((s) => (
          <div key={s.key}><div className="tl-h">{s.title}</div>
            <Focusable className="tl-strip" {...H}>{s.items.map((c, i) => <Game key={c.appid} card={c} shape={i === 0 ? "wide" : "portrait"} className={i === 0 ? "big" : ""} onFocusCard={setFocused} />)}</Focusable></div>
        ))}
        <div className="tl-h">Places</div>
        <Focusable className="tl-strip" {...H}>{ERA_PLACES.map((p) => <PlaceButton key={p.id} place={p} />)}</Focusable>
        {secs.slice(1).map((s) => (
          <div key={s.key}><div className="tl-h">{s.title}</div>
            <Focusable className="tl-strip" {...H}>{s.items.map((c) => <Game key={c.appid} card={c} shape="portrait" onFocusCard={setFocused} />)}</Focusable></div>
        ))}
      </Focusable>
    </div>
  );
}

// ---------------------------------------------------------------- Crossbar: a category bar, a list under it, slow waves

const CROSSBAR = `
.era-crossbar{background:linear-gradient(135deg,var(--era-bg),var(--era-bg2));color:#fff}
.era-crossbar .xb-waves{position:absolute;left:0;right:0;top:38%;height:320px;pointer-events:none;overflow:hidden}
.era-crossbar .xb-waves svg{position:absolute;left:0;top:0;width:200%;height:100%;animation:xb-drift 48s linear infinite}
.era-crossbar .xb-waves svg.b{top:36px;animation-duration:76s;animation-direction:reverse;opacity:.8}
@keyframes xb-drift{from{transform:translateX(0)}to{transform:translateX(-50%)}}
.era-crossbar .xb-clock{position:absolute;right:4vw;top:64px;font-size:18px;opacity:.85;text-align:right}
.era-crossbar .xb-cats{position:absolute;top:17vh;left:18vw;display:flex;gap:26px;transition:transform .32s cubic-bezier(.2,.8,.2,1)}
.era-crossbar .xb-cat{flex:none;width:112px !important;height:112px;min-width:0 !important;padding:0 !important;background:transparent !important;border:none !important;color:rgba(255,255,255,.62) !important;display:flex !important;flex-direction:column;align-items:center;justify-content:center;gap:10px;font-size:14px;transition:opacity .2s}
.era-crossbar .xb-cat svg{width:42px;height:42px;transition:transform .25s}
.era-crossbar .xb-cat.on{color:#fff !important}
.era-crossbar .xb-cat.on svg{transform:scale(1.18);filter:drop-shadow(0 0 12px rgba(255,255,255,.7))}
.era-crossbar .xb-cat.gpfocus{box-shadow:none !important}
.era-crossbar .xb-list{position:absolute;left:calc(18vw - 8px);top:calc(17vh + 128px);bottom:30px;width:58vw;display:flex;flex-direction:column;gap:4px;overflow-y:auto;padding:4px}
.era-crossbar .xb-list::-webkit-scrollbar{display:none}
.era-crossbar .xb-item{flex:none;display:flex !important;align-items:center;gap:18px;width:100% !important;padding:6px 14px !important;border-radius:10px !important;background:transparent !important;color:#fff !important;text-align:left}
.era-crossbar .xb-item .era-art,.era-crossbar .xb-item .era-noart{position:static;width:128px;height:60px;border-radius:6px;flex:none}
.era-crossbar .xb-item .era-noart{font-size:11px;padding:6px}
.era-crossbar .xb-item .era-icon{width:60px;height:60px;align-items:center;justify-content:center;font-size:30px;flex:none}
.era-crossbar .xb-t{font-size:21px;line-height:1.2}
.era-crossbar .xb-s{font-size:13px;opacity:.72}
.era-crossbar .xb-item.gpfocus{background:rgba(255,255,255,.14) !important;box-shadow:0 0 24px rgba(255,255,255,.16) !important}
.era-crossbar .xb-item:not(.gpfocus) .xb-s{opacity:.5}
`;

function wave(amp: number, y: number, color: string, width: number): string {
  let d = `M0 ${y}`;
  for (let x = 0; x <= 2400; x += 150) d += ` Q${x + 75} ${(x / 150) % 2 ? y - amp : y + amp} ${x + 150} ${y}`;
  return `<path d="${d}" fill="none" stroke="${color}" stroke-width="${width}"/>`;
}
const WAVES_A = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2400 300" preserveAspectRatio="none">${wave(46, 150, "rgba(255,255,255,.22)", 3)}${wave(30, 170, "rgba(255,255,255,.12)", 2)}</svg>`;
const WAVES_B = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2400 300" preserveAspectRatio="none">${wave(60, 140, "rgba(255,255,255,.10)", 6)}${wave(22, 190, "rgba(255,255,255,.14)", 1.5)}</svg>`;

function XbGame(props: { card: Card }) {
  const c = props.card;
  return (
    <div className="era-wrap" data-hearth-appid={c.appid} style={{ display: "contents" }}>
      <DialogButton className="era-game xb-item" onClick={() => { play("ok"); openApp(c.appid); }} {...focusProps()}>
        <Art card={c} shape="wide" />
        <div><div className="xb-t">{c.title}</div><div className="xb-s">{subtitle(c) || " "}</div></div>
      </DialogButton>
    </div>
  );
}

function Crossbar(props: { view: HomeView }) {
  const now = useNow();
  const secs: (Section | null)[] = [null, ...sectionsOf(props.view, 30)];   // null: the places
  const [on, setOn] = useState(Math.min(1, secs.length - 1));
  const cur = secs[on];
  return (
    <div className="era-home era-crossbar">
      <style>{CROSSBAR}</style>
      <div className="xb-waves" dangerouslySetInnerHTML={{ __html: WAVES_A + WAVES_B.replace("<svg ", '<svg class="b" ') }} />
      <div className="xb-clock">{dateLine(now)}<br />{clock(now)}</div>
      <Focusable className="xb-cats" style={{ transform: `translateX(${-on * 138}px)` }} {...H}>
        {secs.map((s, i) => (
          <DialogButton key={s ? s.key : "places"} className={"xb-cat" + (i === on ? " on" : "")} onClick={() => setOn(i)} {...focusProps(() => setOn(i))}>
            {sectionIcon(s ? s.key : "places")}<span>{s ? s.title : "Places"}</span>
          </DialogButton>
        ))}
      </Focusable>
      <Focusable key={on} className="xb-list" {...V}>
        {cur ? cur.items.map((c) => <XbGame key={c.appid} card={c} />)
          : ERA_PLACES.map((p) => (
            <DialogButton key={p.id} className="era-place xb-item" onClick={() => { play("ok"); openPlace(p); }} {...focusProps()}>
              <span className="era-icon">{PLACE_ICONS[p.id]}</span><div><div className="xb-t">{p.title}</div><div className="xb-s">Steam's {p.title}</div></div>
            </DialogButton>
          ))}
      </Focusable>
    </div>
  );
}

// ---------------------------------------------------------------- Pocket: a handheld-style icon row over the wallpaper

const POCKET = `
.era-pocket{background:linear-gradient(160deg,rgba(0,0,0,.15),var(--era-bg) 85%),var(--era-wall,none) center/cover,var(--era-bg)}
.era-pocket .pk-clock{position:absolute;right:4vw;top:62px;font-size:22px;font-weight:600;text-shadow:0 1px 6px rgba(0,0,0,.5)}
.era-pocket .pk-card{position:absolute;left:4vw;top:72px;width:min(560px,48vw);border-radius:28px;overflow:hidden;background:var(--era-panel);backdrop-filter:blur(16px);box-shadow:0 18px 50px rgba(0,0,0,.4);border:1px solid rgba(255,255,255,.12)}
.era-pocket .pk-art{position:relative;height:min(250px,30vh)}
.era-pocket .pk-text{padding:14px 20px 18px}
.era-pocket .pk-title{font-size:24px;font-weight:800}
.era-pocket .pk-sub{font-size:14px;color:var(--era-dim);margin-top:3px;min-height:18px}
.era-pocket .pk-row{position:absolute;left:0;right:0;bottom:118px;display:flex;gap:20px;overflow-x:auto;padding:16px 4vw}
.era-pocket .pk-row::-webkit-scrollbar{display:none}
.era-pocket .era-game{flex:none;width:108px;height:108px;border-radius:28px !important;background:var(--era-panel2) !important;box-shadow:0 8px 20px rgba(0,0,0,.35);transition:transform .2s cubic-bezier(.2,.8,.2,1)}
.era-pocket .era-game.gpfocus{transform:scale(1.14) translateY(-6px);z-index:2;box-shadow:0 0 0 3px var(--era-accent),0 14px 30px rgba(0,0,0,.5) !important}
.era-pocket .pk-dock{position:absolute;left:4vw;bottom:34px;display:flex;gap:12px}
.era-pocket .era-place{display:flex !important;align-items:center;gap:8px;width:auto !important;height:52px;padding:0 18px !important;border-radius:26px !important;background:var(--era-panel2) !important;color:var(--era-text) !important;font-size:15px;font-weight:700;backdrop-filter:blur(10px)}
.era-pocket .era-place.gpfocus{background:var(--era-accent) !important;color:var(--era-ink) !important}
`;

function Pocket(props: { view: HomeView }) {
  const now = useNow();
  const games = gridGames(props.view, 24);
  const [focused, setFocused] = useState<Card | null>(null);
  const shown = focused || games[0] || null;
  return (
    <div className="era-home era-pocket">
      <style>{POCKET}</style>
      <div className="pk-clock">{clock(now)}</div>
      {shown && (
        <div className="pk-card">
          <div className="pk-art"><Art key={shown.appid} card={shown} shape="wide" /></div>
          <div className="pk-text"><div className="pk-title">{shown.title}</div><div className="pk-sub">{subtitle(shown)}</div></div>
        </div>
      )}
      <Focusable className="pk-row" {...H}>{games.map((c) => <Game key={c.appid} card={c} shape="portrait" onFocusCard={setFocused} />)}</Focusable>
      <Focusable className="pk-dock" {...H}>{ERA_PLACES.map((p) => <PlaceButton key={p.id} place={p} />)}</Focusable>
    </div>
  );
}

// ---------------------------------------------------------------- the switch

/** The picked era's home in place of Steam's. Nothing to show: Steam's own Home. Hearth Classic: Hearth Classic's own. */
export function EraHome(props: { era: EraId; preset: Preset | null; steam: ReactNode; wallpaper: string; logo: string }) {
  if (props.era === "classic") return <HearthHome steam={props.steam} wallpaper={props.wallpaper} logo={props.logo} />;
  return <EraHomeOf {...props} />;
}

function EraHomeOf(props: { era: EraId; preset: Preset | null; steam: ReactNode; wallpaper: string; logo: string }) {
  const { view, loading } = useHomeView();
  if (props.era === "steam" || (!loading && (!view || !view.shelves.length))) return <>{props.steam}</>;
  const body = !view ? <div className="era-loading">Getting your games…</div>
    : props.era === "channels" ? <Channels view={view} />
    : props.era === "blades" ? <Blades view={view} />
    : props.era === "tiles" ? <Tiles view={view} />
    : props.era === "crossbar" ? <Crossbar view={view} />
    : <Pocket view={view} />;
  return (
    <div className={"hearth-eras era-" + props.era + "-root"} data-era={props.era} style={{ position: "absolute", inset: 0, ...varsOf(props.preset, props.era === "pocket" ? props.wallpaper : "") }}>
      <style>{BASE}</style>
      {body}
    </div>
  );
}
