// The Hearth Decky plugin (Steam Game Mode, Experimental; docs/STEAM-GAME-MODE.md): Hearth takes over Steam's gamepad
// UI as far as Decky allows, with Steam's library, downloads and overlay underneath.
//  - the takeover (patches.tsx, pages.tsx): Hearth's home in place of Steam's Home, Hearth Store / Library /
//    Downloads / Storage and a game's Hearth page as routes, Hearth's stores as tabs in Steam's Library, Hearth's facts
//    on game pages, Hearth's places in Steam's main menu. A master switch and one per part; every patch is guarded;
//  - Quick Access › Hearth: the switches, the power mode, your Hearth games with Play, Sync to Steam library, Hearth's
//    apps, and the way back to Hearth's Game Mode;
//  - Hearth's look over Steam (theme.ts), on by default and switched off with one toggle;
//  - the older Hearth row over Steam's Home (when Hearth's home screen is off).
import { ButtonItem, DialogButton, Focusable, PanelSection, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { callable, definePlugin } from "@decky/api";
import { ReactNode, useEffect, useState } from "react";
import { FaFire } from "react-icons/fa";
import { applyTheme, removeTheme } from "./theme";
import { Item, play, removeAll, sync } from "./shortcuts";
import { PARTS, Settings, partOn } from "./model";
import { ask, getErrors, getSettings, setSettings } from "./api";
import { guard, setSettingsLocal, setShortcutMap, settings as liveSettings, useLive } from "./state";
import { Brand, DownloadsPage, GamePage, LibraryPage, StoragePage, StorePage, setBrand } from "./pages";
import { addRoutes, patchGamePage, patchHome, patchLibraryTabs, patchMainMenu } from "./patches";
import { go, setSounds } from "./ui";
import { say } from "./actions";

interface Games { ok: boolean; error?: string; games: Item[]; links: Item[] }

const getGames = callable<[], Games>("get_games");
const getBrand = callable<[], Brand>("get_brand");
const switchToHearth = callable<[], { ok: boolean; error?: string }>("switch_to_hearth");
const getArt = callable<[ref: string], { data: string; type: string } | null>("get_art");
const getShortcuts = callable<[], Record<string, number>>("get_shortcuts");

const ACCENT = "#3fd0b8", INK = "#06231e", PANEL = "#1d2531", DIM = "#8d99a8";

let cache: Games | null = null;
let brand: Brand | null = null;
const listeners = new Set<() => void>();
async function loadGames(force = false): Promise<Games> {
  if (!cache || force) {
    cache = await getGames().catch((e) => ({ ok: false, error: String(e), games: [], links: [] }));
    listeners.forEach((f) => f());
  }
  return cache;
}

async function playItem(it: Item) {
  try { await play(it); } catch (e) { say(`Couldn't start ${it.title}: ${e}`); }
}

function useGames(): Games | null {
  const [g, setG] = useState<Games | null>(cache);
  useEffect(() => {
    const f = () => setG(cache);
    listeners.add(f);
    if (!cache) loadGames();
    return () => { listeners.delete(f); };
  }, []);
  return g;
}

async function change(k: keyof Settings, v: boolean) {
  const next = await setSettings({ ...liveSettings, [k]: v });
  setSettingsLocal(next);
  setSounds(partOn(next, "tkSounds", guard.broken));
  if (k === "theme") { if (v && brand) applyTheme(brand); else removeTheme(); }
}

/** The power mode (Hearth's Quick Access › Performance, through hearth-power), when this device has one. */
function PowerMode() {
  const [p, setP] = useState<any>(null);
  useEffect(() => { ask("power", "", 0).then(setP); }, []);
  const m = p?.mode;
  if (!m || !m.supported) return null;
  const modes: [string, string][] = ([["efficiency", "Saver"], ["balanced", "Balanced"], ["performance", "Performance"]] as [string, string][])
    .filter(([k]) => !m.available?.length || m.available.includes(k));
  return (
    <PanelSectionRow>
      <div className={staticClasses.Text} style={{ marginBottom: 6 }}>Power mode</div>
      <Focusable style={{ display: "flex", gap: 6 }} {...({ "flow-children": "horizontal" } as any)}>
        {modes.map(([k, label]) => (
          <DialogButton key={k} style={{ minWidth: 0, flex: 1, padding: "8px 4px", background: m.mode === k ? ACCENT : undefined, color: m.mode === k ? INK : undefined }}
            onClick={async () => { const r = await ask("power-mode", k, 0); if (r.error) say(r.error); else setP(r); }}>{label}</DialogButton>
        ))}
      </Focusable>
    </PanelSectionRow>
  );
}

function Panel() {
  const games = useGames();
  const s = useLive();
  const [busy, setBusy] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  useEffect(() => { getErrors().then((e) => setErrors(e || [])).catch(() => {}); }, [guard.broken.size]);

  const doSync = async () => {
    const g = await loadGames(true);
    setBusy("Syncing…");
    try {
      const r = await sync([...g.games, ...g.links], (d, t) => setBusy(`Syncing ${d} of ${t}…`));
      say(`Steam library synced: ${r.added} added${r.removed ? `, ${r.removed} removed` : ""}`);
      setShortcutMap(await getShortcuts().catch(() => ({})));
    } catch (e) { say(`Sync didn't work: ${e}`); }
    setBusy("");
  };
  const link = (id: string) => games?.links.find((l) => l.id === id);
  const nav = (route: string) => () => go(route);

  return (
    <>
      <PanelSection title="Hearth">
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={async () => { const r = await switchToHearth(); if (!r.ok) say(r.error ?? "Couldn't switch"); }}
            description="Steam closes and Hearth's own Game Mode comes back">
            Switch to Hearth Game Mode
          </ButtonItem>
        </PanelSectionRow>
        <PanelSectionRow>
          <ToggleField label="Hearth takeover" description="Hearth's home, pages and tabs in Steam's Game Mode (Steam's library, downloads and overlay stay)"
            checked={s.takeover} onChange={(v) => change("takeover", v)} />
        </PanelSectionRow>
        {s.takeover && PARTS.map(([k, label, desc]) => (
          <PanelSectionRow key={k}>
            <ToggleField label={label} description={guard.broken.has(k) ? "Turned off: Steam changed this screen (see Decky's log)" : desc}
              checked={s[k]} disabled={guard.broken.has(k)} onChange={(v) => change(k, v)} />
          </PanelSectionRow>
        ))}
        <PanelSectionRow>
          <ToggleField label="Hearth look" description="Hearth's colours, focus, wallpaper and logo over Steam"
            checked={s.theme} onChange={(v) => change("theme", v)} />
        </PanelSectionRow>
        {!partOn(s, "tkHome", guard.broken) && (
          <PanelSectionRow>
            <ToggleField label="Hearth row on Steam's home" description="Your Hearth games at the top of Steam's Home"
              checked={s.homeRow} onChange={(v) => change("homeRow", v)} />
          </PanelSectionRow>
        )}
      </PanelSection>
      <PanelSection title="Quick settings">
        <PowerMode />
        <PanelSectionRow>
          <ButtonItem layout="below" description="Steam's layouts: Quick Access › Controller. Hearth Input's open in Hearth's Settings"
            onClick={() => { const l = link("hearth-app:settings"); if (l) playItem(l); else say("Hearth's Settings aren't installed here"); }}>
            Controller layouts
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>
      <PanelSection title="Hearth's places">
        {s.takeover && <PanelSectionRow><ButtonItem layout="below" onClick={nav("/library/home")}>Hearth home</ButtonItem></PanelSectionRow>}
        <PanelSectionRow><ButtonItem layout="below" onClick={nav("/hearth/store")}>Hearth Store</ButtonItem></PanelSectionRow>
        <PanelSectionRow><ButtonItem layout="below" onClick={nav("/hearth/library")}>Hearth Library</ButtonItem></PanelSectionRow>
        <PanelSectionRow><ButtonItem layout="below" onClick={nav("/hearth/downloads")}>Downloads</ButtonItem></PanelSectionRow>
        <PanelSectionRow><ButtonItem layout="below" onClick={nav("/hearth/storage")}>Storage</ButtonItem></PanelSectionRow>
        {[["hearth-app:mods", "Hearth Mods"], ["hearth-app:theater", "Theater"]].map(([id, label]) => {
          const l = link(id);
          return l ? <PanelSectionRow key={id}><ButtonItem layout="below" onClick={() => playItem(l)}>{label}</ButtonItem></PanelSectionRow> : null;
        })}
      </PanelSection>
      <PanelSection title="Your Hearth games">
        <PanelSectionRow>
          <ButtonItem layout="below" disabled={!!busy} onClick={doSync}
            description="Adds them to Steam's library with Hearth's art; they play through Hearth">
            {busy || "Sync to Steam library"}
          </ButtonItem>
        </PanelSectionRow>
        {!games && <PanelSectionRow><div className={staticClasses.Text}>Asking Hearth…</div></PanelSectionRow>}
        {games && !games.ok && <PanelSectionRow><div className={staticClasses.Text}>{games.error}</div></PanelSectionRow>}
        {games && games.ok && !games.games.length && (
          <PanelSectionRow><div className={staticClasses.Text}>No games from Hearth's other stores are installed.</div></PanelSectionRow>
        )}
        {games?.games.slice(0, 40).map((g) => (
          <PanelSectionRow key={g.id}>
            <ButtonItem layout="below" label={g.title} description={g.store} onClick={() => playItem(g)}>Play</ButtonItem>
          </PanelSectionRow>
        ))}
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={() => loadGames(true)}>Refresh list</ButtonItem>
        </PanelSectionRow>
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={async () => { say(`Removed ${await removeAll()} Hearth shortcuts from Steam`); setShortcutMap({}); }}>
            Remove Hearth's shortcuts
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>
      {errors.length > 0 && (
        <PanelSection title="Turned off by itself">
          {errors.slice(-5).map((e, i) => <PanelSectionRow key={i}><div className={staticClasses.Text} style={{ fontSize: 12 }}>{e}</div></PanelSectionRow>)}
        </PanelSection>
      )}
    </>
  );
}

// ---- The older Hearth row over Steam's Home (Hearth's home screen off): a hero with Hearth's wallpaper and logo,
// then your Hearth games like Hearth's home.
// Covers as data: addresses (Steam's window can't load files from the disk), read once each.
const coverCache: Record<string, string> = {};
function useCovers(list: Item[]): Record<string, string> {
  const [, bump] = useState(0);
  useEffect(() => {
    let live = true;
    (async () => {
      for (const g of list.slice(0, 12)) {
        if (coverCache[g.id] !== undefined || !g.cover) continue;
        const a = await getArt(g.cover).catch(() => null);
        coverCache[g.id] = a ? `data:image/${a.type === "png" ? "png" : "jpeg"};base64,${a.data}` : "";
        if (live) bump((n) => n + 1);
      }
    })();
    return () => { live = false; };
  }, [list.length]);
  return coverCache;
}
function HearthRow(props: { children?: ReactNode }) {
  const games = useGames();
  const list = games?.games ?? [];
  const covers = useCovers(list);
  if (!list.length) return <>{props.children}</>;
  const cover = (g: Item) => covers[g.id] || null;
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <div style={{ flex: "none", padding: "18px 2.8vw 10px", background: brand?.wallpaper
          ? `linear-gradient(180deg,rgba(12,16,22,.25),rgba(12,16,22,.95)),url(${brand.wallpaper}) center/cover` : "#121821" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          {brand?.logo && <img src={brand.logo} style={{ width: 30, height: 30 }} />}
          <div style={{ fontSize: 15, fontWeight: 600, letterSpacing: ".02em", textTransform: "uppercase", color: DIM }}>From Hearth</div>
        </div>
        <Focusable {...({ "flow-children": "horizontal" } as any)} style={{ display: "flex", gap: 14, overflow: "hidden" }}>
          {list.slice(0, 12).map((g) => (
            <DialogButton key={g.id} onClick={() => playItem(g)}
              style={{ flex: "none", width: 120, height: 170, minWidth: 0, padding: 0, borderRadius: 10, overflow: "hidden", position: "relative",
                background: cover(g) ? `url(${cover(g)}) center/cover` : PANEL }}>
              {!cover(g) && <div style={{ padding: 10, fontSize: 13, color: "#e9eef4", whiteSpace: "normal" }}>{g.title}</div>}
              <div style={{ position: "absolute", left: 6, bottom: 6, fontSize: 10, fontWeight: 700, padding: "2px 6px", borderRadius: 8, background: ACCENT, color: INK }}>{g.store}</div>
            </DialogButton>
          ))}
        </Focusable>
      </div>
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>{props.children}</div>
    </div>
  );
}

export default definePlugin(() => {
  const undo: (() => void)[] = [];
  (async () => {
    const s = await getSettings();
    setSettingsLocal(s);
    setSounds(partOn(s, "tkSounds", guard.broken));
    setShortcutMap(await getShortcuts().catch(() => ({})));
    brand = await getBrand().catch(() => null);
    setBrand(brand);
    if (s.theme && brand) { try { applyTheme(brand); } catch (e) { console.warn("Hearth: theme", e); } }
    loadGames();
    // Steam's main menu is looked for once Steam's UI has loaded its code (a moment after the plugin starts).
    setTimeout(() => { undo.push(patchMainMenu()); }, 4000);
  })().catch((e) => console.warn("Hearth: start", e));
  // Each of these is guarded inside: one that fails leaves Steam's screen as it is and the others working.
  undo.push(patchHome((steam) => <HearthRow>{steam}</HearthRow>));
  undo.push(addRoutes([
    ["/hearth/store", StorePage], ["/hearth/library", LibraryPage], ["/hearth/downloads", DownloadsPage],
    ["/hearth/storage", StoragePage], ["/hearth/game/:id", GamePage],
  ]));
  undo.push(patchLibraryTabs());
  undo.push(patchGamePage());
  return {
    name: "Hearth",
    titleView: <div className={staticClasses.Title}>Hearth</div>,
    content: <Panel />,
    icon: <FaFire />,
    onDismount() {
      removeTheme();
      for (const u of undo) { try { u(); } catch { /* already gone */ } }
    },
  };
});
