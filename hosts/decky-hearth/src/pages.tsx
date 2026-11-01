// Hearth's pages inside Steam's Game Mode (the takeover): Hearth's home in place of Steam's Home, and Hearth Store,
// Library, Downloads, Storage and a game's Hearth page as routes of their own. Drawn with @decky/ui's focusable
// parts in Hearth's look (not Hearth's own page in a frame: see docs/STEAM-GAME-MODE.md, "How the pages are made").
import * as DUI from "@decky/ui";
import { DialogButton, Focusable, Navigation } from "@decky/ui";
import { ReactNode, useEffect, useState } from "react";
import { logError, useArt, useAsk } from "./api";
import { gamePage, openApp, playGame, say } from "./actions";
import { DESTS, Game, Home, STORE_TABS, artCandidates, bytes, downloadLine, driveLine, homeShelves, percent, played, validId } from "./model";
import { Boundary, C, Chips, CoverArt, Page, Shelf, blip, go, onFocus } from "./ui";
import { guard } from "./state";

export interface Brand { css: string; logo: string; wallpaper: string }
let brandNow: Brand | null = null;
export function setBrand(b: Brand | null) { brandNow = b; }
export const brand = () => brandNow;

function Btn(props: { label: string; primary?: boolean; onClick: () => void }) {
  return (
    <DialogButton className={"ht-btn" + (props.primary ? " primary" : "")} onClick={() => { blip("ok"); props.onClick(); }} {...({ onGamepadFocus: onFocus } as any)}>
      {props.label}
    </DialogButton>
  );
}

function openDest(id: string) {
  const d = DESTS.find((x) => x.id === id);
  if (!d) return;
  if (d.route) go(d.route); else if (d.app) openApp(d.app);
}

/** Hearth's places, as a row of chips: Store, Library, Downloads, Theater, Mods, Storage, and Steam's own. */
export function DestRow(props: { withSteam?: boolean }) {
  const items: [string, string][] = DESTS.map((d) => [d.id, d.title]);
  if (props.withSteam) items.push(["steam-library", "Steam Library"], ["steam-store", "Steam Store"]);
  return (
    <Chips items={items} value="" onChange={(v) => {
      if (v === "steam-library") go("/library");
      else if (v === "steam-store") go("/store");
      else openDest(v);
    }} />
  );
}

// ---------------------------------------------------------------- the home screen

/** Hearth's home in place of Steam's Home. Anything wrong (Hearth not answering, a throw): Steam's own Home. */
export function HearthHome(props: { steam: ReactNode }) {
  const home = useAsk<Home>("home", "", 20000, 60000);
  const wall = brandNow?.wallpaper || "";
  const failed = home && (home as any).error ? String((home as any).error) : "";
  // Hearth not answering (not installed, still starting): Steam's own Home this time, said once in the log.
  useEffect(() => { if (failed) logError("tkHome", "Hearth didn't answer, Steam's Home shown: " + failed).catch(() => {}); }, [failed]);
  if (failed) return <>{props.steam}</>;
  if (!home) return <Page wallpaper={wall}><div className="ht-dim">Asking Hearth…</div></Page>;
  const hero = home.hero;
  return (
    <Page wallpaper={wall} noBack>
      <Hero game={hero} />
      <DestRow withSteam />
      {!home.count && <div className="ht-card" style={{ marginTop: 18 }}>No games in Hearth's library yet. Install one in Steam, or open Hearth Store.</div>}
      {homeShelves(home).map((s) => (
        <Shelf key={s.key} title={s.title} games={s.items} wide={s.wide} badge={s.key === "continue" || s.key === "fav" || s.key === "added"}
          onActivate={(g) => (s.key === "continue" ? playGame(g) : gamePage(g))} onMenu={gamePage} />
      ))}
    </Page>
  );
}

function Hero(props: { game: Game | null }) {
  const g = props.game;
  const art = useArt(artCandidates(g, "hero"), "hero").src;
  const logo = useArt(g?.logo ? [g.logo] : [], "logo").src;
  return (
    <div style={{ position: "relative", height: "34vh", minHeight: 200, margin: "-54px -2.8vw 10px", padding: "54px 2.8vw 18px", display: "flex", flexDirection: "column", justifyContent: "flex-end",
                  background: art ? `linear-gradient(90deg,rgba(12,16,22,.92),rgba(12,16,22,.2) 70%),linear-gradient(0deg,${C.bg},transparent 45%),url("${art}") center/cover` : undefined }}>
      <div style={{ position: "absolute", top: 54, left: "2.8vw", display: "flex", alignItems: "center", gap: 10 }}>
        {brandNow?.logo && <img src={brandNow.logo} style={{ width: 28, height: 28 }} />}
        <div style={{ fontSize: 15, fontWeight: 600, letterSpacing: ".02em", textTransform: "uppercase", color: C.dim }}>Hearth</div>
      </div>
      {g && (
        <>
          <div style={{ fontSize: 12, fontWeight: 700, letterSpacing: ".08em", textTransform: "uppercase", color: C.accent }}>Jump back in</div>
          {logo ? <img src={logo} style={{ maxHeight: 90, maxWidth: 380, objectFit: "contain", objectPosition: "left", margin: "6px 0" }} />
                : <div style={{ fontSize: 34, fontWeight: 800, margin: "4px 0" }}>{g.title}</div>}
          <div className="ht-dim" style={{ marginBottom: 12 }}>{[g.store, played(g.minutes)].filter(Boolean).join(" · ")}</div>
          <Focusable style={{ display: "flex", gap: 10 }} {...({ "flow-children": "horizontal" } as any)}>
            <Btn primary label={g.installed ? "Play" : "Open in Hearth"} onClick={() => playGame(g)} />
            <Btn label="Details" onClick={() => gamePage(g)} />
          </Focusable>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------- Store

interface StoreItem { store: string; id: string; title: string; wide?: string | null; tall?: string | null; icon?: string | null; sub?: string; price?: { final?: string; base?: string | null; discount?: number; free?: boolean } }
interface StorePageData { store: string; sections: { id: string; title: string; items: StoreItem[] }[]; inApp?: boolean; stale?: boolean }

function StoreTile(props: { it: StoreItem }) {
  const it = props.it;
  const { src, failed } = useArt([it.wide, it.tall, it.icon].filter((x): x is string => !!x), "store");
  const p: NonNullable<StoreItem["price"]> = it.price || {};
  return (
    <DialogButton className="ht-tile" style={{ width: 276, height: 160 }} onClick={() => { blip("ok"); openApp("hearth-app:store"); say(`Opening Hearth Store for ${it.title}`); }}
      {...({ onGamepadFocus: onFocus } as any)}>
      {src && <img src={src} onError={failed} style={{ height: 112, bottom: "auto", objectFit: it.wide ? "cover" : "contain" }} />}
      <div style={{ position: "absolute", left: 0, right: 0, bottom: 0, height: 48, padding: "6px 10px", background: C.panel, textAlign: "left" }}>
        <div style={{ fontSize: 13, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{it.title}</div>
        <div style={{ fontSize: 12, color: p.free ? C.accent : C.dim }}>
          {p.discount ? <span style={{ background: C.accent, color: C.ink, borderRadius: 6, padding: "0 5px", marginRight: 6, fontWeight: 700 }}>-{p.discount}%</span> : null}
          {it.sub ? `${it.sub} · ` : ""}{p.final || ""}
        </div>
      </div>
    </DialogButton>
  );
}

function StoreSections(props: { store: string }) {
  const d = useAsk<StorePageData>("store", props.store, 10 * 60000);
  if (!d) return <div className="ht-dim" style={{ marginTop: 16 }}>Asking {props.store === "free" ? "the stores" : "the store"}…</div>;
  if ((d as any).error) return <div className="ht-card" style={{ marginTop: 16 }}>Couldn't load this store: {(d as any).error}<div style={{ marginTop: 10 }}><Btn label="Open Hearth Store" onClick={() => openApp("hearth-app:store")} /></div></div>;
  if (d.inApp) return (
    <div className="ht-card" style={{ marginTop: 16 }}>
      Battle.net's shop is Blizzard's own app. Hearth Store opens it, signed in with your Battle.net account there.
      <div style={{ marginTop: 10 }}><Btn primary label="Open Hearth Store" onClick={() => openApp("hearth-app:store")} /></div>
    </div>
  );
  return (
    <>
      {d.stale && <div className="ht-dim" style={{ marginTop: 8 }}>Offline: showing what the store sent last time.</div>}
      {d.sections.map((s) => (
        <div key={s.id}>
          <h2>{s.title}</h2>
          <Focusable className="ht-shelf" {...({ "flow-children": "horizontal" } as any)}>
            {s.items.slice(0, 24).map((it) => <StoreTile key={it.store + it.id} it={it} />)}
          </Focusable>
        </div>
      ))}
    </>
  );
}

/** Hearth Store: Hearth's store tabs (Steam, Epic, GOG, free games, Xbox, Battle.net, Recomps) to browse; buying,
 *  claiming and installing open Hearth Store itself. */
export function StorePage() {
  const [store, setStore] = useState("steam");
  return (
    <Boundary part="store" onError={guard.fail}>
      <Page title="Hearth Store" wallpaper={brandNow?.wallpaper}>
        <div className="ht-dim">Browse every store Hearth has. Press a game to open it in Hearth Store (buy, claim, install).</div>
        <Chips items={STORE_TABS} value={store} onChange={setStore} />
        <StoreSections key={store} store={store} />
        <div style={{ marginTop: 22 }}><Btn label="Open Hearth Store" onClick={() => openApp("hearth-app:store")} /></div>
      </Page>
    </Boundary>
  );
}

// ---------------------------------------------------------------- Library

interface LibraryData { tabs: { id: string; title: string; count: number; installed: number; items: Game[] }[] }

/** A store's games as rows of tiles (rows move up and down, tiles left and right). */
export function LibraryGrid(props: { tab: string; perRow?: number }) {
  const lib = useAsk<LibraryData>("library", "", 30000);
  if (!lib) return <div className="ht-dim" style={{ padding: 20 }}>Asking Hearth…</div>;
  if ((lib as any).error) return <div className="ht-card">Hearth didn't answer: {(lib as any).error}</div>;
  const t = lib.tabs.find((x) => x.id === props.tab);
  if (!t) return <div className="ht-dim" style={{ padding: 20 }}>No games here.</div>;
  const n = props.perRow || 7, rows: Game[][] = [];
  for (let i = 0; i < t.items.length; i += n) rows.push(t.items.slice(i, i + n));
  return (
    <Focusable {...({ "flow-children": "vertical" } as any)}>
      <div className="ht-dim" style={{ margin: "6px 4px" }}>{t.count} game{t.count === 1 ? "" : "s"}, {t.installed} installed</div>
      {rows.map((r, i) => (
        <Focusable key={i} className="ht-shelf" style={{ overflowX: "visible" }} {...({ "flow-children": "horizontal" } as any)}>
          {r.map((g) => <TileFor key={g.id} g={g} />)}
        </Focusable>
      ))}
    </Focusable>
  );
}
function TileFor(props: { g: Game }) {
  const g = props.g;
  return (
    <DialogButton className="ht-tile" style={{ width: 132, height: 188, opacity: g.installed ? 1 : 0.55 }} onClick={() => { blip("ok"); gamePage(g); }}
      {...({ onGamepadFocus: onFocus } as any)}>
      <CoverArt game={g} shape="portrait" badge="none" where="library" />
    </DialogButton>
  );
}

/** Hearth Library: a tab per store, like Hearth's library. */
export function LibraryPage() {
  const lib = useAsk<LibraryData>("library", "", 30000);
  const tabs: [string, string][] = lib && !(lib as any).error ? lib.tabs.map((t) => [t.id, `${t.title} ${t.count}`] as [string, string]) : [];
  const [tab, setTab] = useState("");
  const cur = tab || (tabs[0] ? tabs[0][0] : "");
  return (
    <Boundary part="library" onError={guard.fail}>
      <Page title="Hearth Library" wallpaper={brandNow?.wallpaper}>
        {tabs.length > 0 && <Chips items={tabs} value={cur} onChange={setTab} />}
        {cur ? <LibraryGrid tab={cur} /> : <div className="ht-dim">{lib && (lib as any).error ? `Hearth didn't answer: ${(lib as any).error}` : lib ? "No games in Hearth's library yet." : "Asking Hearth…"}</div>}
      </Page>
    </Boundary>
  );
}

// ---------------------------------------------------------------- Downloads

interface DownloadsData { active: { id: string; title: string; store: string; state: string; stage?: string | null; progress?: number | null; bytesPerSec?: number | null }[] }

/** Downloads: Hearth's queue (Epic, GOG, Steam games Hearth started, Hearth's own Steam client's), every 2 seconds. */
export function DownloadsPage() {
  const d = useAsk<DownloadsData>("downloads", "", 0, 2000);
  return (
    <Boundary part="downloads" onError={guard.fail}>
      <Page title="Downloads" wallpaper={brandNow?.wallpaper}>
        <div className="ht-dim" style={{ marginBottom: 14 }}>Hearth's downloads. Steam's own are in Steam's Downloads.</div>
        {!d && <div className="ht-dim">Asking Hearth…</div>}
        {d && (d as any).error && <div className="ht-card">Hearth didn't answer: {(d as any).error}</div>}
        {d && !(d as any).error && !d.active.length && <div className="ht-card">Nothing is downloading in Hearth.</div>}
        {d && !(d as any).error && d.active.map((x) => (
          <div key={x.id} className="ht-card">
            <div style={{ display: "flex", justifyContent: "space-between" }}><b>{x.title}</b><span className="ht-dim">{x.store}</span></div>
            <div className="ht-dim">{downloadLine(x)}</div>
            <div className="ht-bar"><i style={{ width: `${percent(x.progress)}%` }} /></div>
          </div>
        ))}
        <Focusable style={{ display: "flex", gap: 10, marginTop: 10 }} {...({ "flow-children": "horizontal" } as any)}>
          <Btn label="Steam's downloads" onClick={() => go("/downloads")} />
          <Btn label="Manage in Hearth" onClick={() => openApp("hearth-app:hearth")} />
        </Focusable>
      </Page>
    </Boundary>
  );
}

// ---------------------------------------------------------------- Storage

interface StorageData { drives: { name: string; kind: string; mount: string; size: number; used: number; free: number; readOnly: boolean; steamLibs: number }[] }

/** Settings › Storage, the summary: each drive and how full it is. Moving and cleaning up open Hearth's Settings. */
export function StoragePage() {
  const d = useAsk<StorageData>("storage", "", 60000);
  return (
    <Boundary part="storage" onError={guard.fail}>
      <Page title="Storage" wallpaper={brandNow?.wallpaper}>
        {!d && <div className="ht-dim">Asking Hearth…</div>}
        {d && (d as any).error && <div className="ht-card">Hearth didn't answer: {(d as any).error}</div>}
        {d && !(d as any).error && d.drives.map((x) => (
          <div key={x.mount} className="ht-card">
            <div style={{ display: "flex", justifyContent: "space-between" }}><b>{x.name}</b><span className="ht-dim">{driveLine(x)}</span></div>
            <div className="ht-dim">{x.mount}{x.steamLibs ? " · Steam library" : ""}{x.readOnly ? " · read only" : ""}</div>
            <div className="ht-bar"><i style={{ width: `${x.size ? Math.round((x.used / x.size) * 100) : 0}%` }} /></div>
          </div>
        ))}
        <div style={{ marginTop: 10 }}><Btn label="Move games, clean up: open Hearth Settings" onClick={() => openApp("hearth-app:settings")} /></div>
      </Page>
    </Boundary>
  );
}

// ---------------------------------------------------------------- a game's Hearth page

interface Info { protondb?: { tier?: string; label?: string; text?: string; reports?: number } | null; achievements?: { total: number; done: number } | null }

export function GameInfoLine(props: { id: string }) {
  const info = useAsk<Info>("info", props.id, 3600000);
  if (!info || (info as any).error) return null;
  const bits: string[] = [];
  if (info.protondb?.label) bits.push(`ProtonDB: ${info.protondb.label}`);
  if (info.achievements?.total) bits.push(`${info.achievements.done} of ${info.achievements.total} achievements`);
  return bits.length ? <div className="ht-dim">{bits.join(" · ")}</div> : null;
}

export function GamePage() {
  let id = "";
  // Steam's router's useParams when @decky/ui has it (the same one every render), else the address.
  const useParams = (DUI as any).useParams as (() => { id?: string }) | undefined;
  const params: { id?: string } = useParams ? useParams() : {};
  try { id = decodeURIComponent(params.id || ""); } catch { id = ""; }
  if (!id) { try { id = decodeURIComponent((location.pathname.split("/hearth/game/")[1] || "")); } catch { id = ""; } }
  const lib = useAsk<LibraryData>("library", "", 30000);
  const g = lib && !(lib as any).error ? lib.tabs.flatMap((t) => t.items).find((x) => x.id === id) : undefined;
  const art = useArt(artCandidates(g, "hero"), "game page").src;
  return (
    <Boundary part="gamePage" onError={guard.fail}>
      <Page wallpaper={art || brandNow?.wallpaper}>
        {!validId(id) && <div className="ht-card">That isn't a game Hearth knows.</div>}
        {validId(id) && !g && <div className="ht-dim">{lib ? "That game isn't in Hearth's library anymore." : "Asking Hearth…"}</div>}
        {g && (
          <>
            <div style={{ fontSize: 13, fontWeight: 700, letterSpacing: ".08em", textTransform: "uppercase", color: C.accent, marginTop: "16vh" }}>{g.store}</div>
            <div style={{ fontSize: 36, fontWeight: 800, margin: "4px 0 6px" }}>{g.title}</div>
            <div className="ht-dim">{[g.installed ? "Installed" : "Not installed", played(g.minutes), g.sizeBytes ? bytes(g.sizeBytes) : ""].filter(Boolean).join(" · ")}</div>
            <GameInfoLine id={g.id} />
            <Focusable style={{ display: "flex", gap: 10, marginTop: 16 }} {...({ "flow-children": "horizontal" } as any)}>
              <Btn primary label={g.installed ? "Play through Hearth" : "Install in Hearth"} onClick={() => playGame(g)} />
              <Btn label="Open in Hearth" onClick={() => openApp("hearth-app:hearth")} />
              <Btn label="Back" onClick={() => { try { Navigation.NavigateBack(); } catch { go("/library/home"); } }} />
            </Focusable>
          </>
        )}
      </Page>
    </Boundary>
  );
}
