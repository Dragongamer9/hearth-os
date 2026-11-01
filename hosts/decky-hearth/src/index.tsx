// The Hearth Decky plugin (Steam Game Mode, Experimental; docs/STEAM-GAME-MODE.md): Hearth inside Steam's gamepad UI.
//  - Quick Access › Hearth: your games from Hearth's other stores with Play, Sync to Steam library, Hearth's apps, and
//    the way back to Hearth's Game Mode;
//  - Hearth's look over Steam (theme.ts), on by default and switched off with one toggle;
//  - a Hearth row at the top of Steam's home (your Hearth games), also with its own toggle.
import { ButtonItem, DialogButton, Focusable, PanelSection, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { callable, definePlugin, routerHook, toaster } from "@decky/api";
import { useEffect, useState } from "react";
import { FaFire } from "react-icons/fa";
import { applyTheme, removeTheme, Brand } from "./theme";
import { Item, play, removeAll, sync } from "./shortcuts";

interface Games { ok: boolean; error?: string; games: Item[]; links: Item[] }
interface Settings { theme: boolean; homeRow: boolean }

const getGames = callable<[], Games>("get_games");
const getBrand = callable<[], Brand>("get_brand");
const getSettings = callable<[], Settings>("get_settings");
const setSettings = callable<[settings: Settings], Settings>("set_settings");
const switchToHearth = callable<[], { ok: boolean; error?: string }>("switch_to_hearth");
const getArt = callable<[ref: string], { data: string; type: string } | null>("get_art");

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

function say(body: string) { toaster.toast({ title: "Hearth", body }); }
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

function Panel() {
  const games = useGames();
  const [settings, setS] = useState<Settings | null>(null);
  const [busy, setBusy] = useState("");
  useEffect(() => { getSettings().then(setS).catch(() => setS({ theme: true, homeRow: true })); }, []);

  const change = async (k: keyof Settings, v: boolean) => {
    const next = await setSettings({ ...(settings ?? { theme: true, homeRow: true }), [k]: v });
    setS(next);
    if (k === "theme") { if (v && brand) applyTheme(brand); else removeTheme(); }
    if (k === "homeRow") homeRowOn = v;
    if (k === "homeRow") say(v ? "Hearth's row is back on Steam's home" : "Hearth's row is off Steam's home");
  };
  const doSync = async () => {
    const g = await loadGames(true);
    setBusy("Syncing…");
    try {
      const r = await sync([...g.games, ...g.links], (d, t) => setBusy(`Syncing ${d} of ${t}…`));
      say(`Steam library synced: ${r.added} added${r.removed ? `, ${r.removed} removed` : ""}`);
    } catch (e) { say(`Sync didn't work: ${e}`); }
    setBusy("");
  };
  const link = (id: string) => games?.links.find((l) => l.id === id);

  return (
    <>
      <PanelSection title="Hearth">
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={async () => { const r = await switchToHearth(); if (!r.ok) say(r.error ?? "Couldn't switch"); }}
            description="Steam closes and Hearth's own Game Mode comes back">
            Switch to Hearth Game Mode
          </ButtonItem>
        </PanelSectionRow>
        {settings && (
          <>
            <PanelSectionRow>
              <ToggleField label="Hearth look" description="Hearth's colours, focus, wallpaper and logo over Steam"
                checked={settings.theme} onChange={(v) => change("theme", v)} />
            </PanelSectionRow>
            <PanelSectionRow>
              <ToggleField label="Hearth row on Steam's home" description="Your Hearth games at the top of Home (applies next time Home opens)"
                checked={settings.homeRow} onChange={(v) => change("homeRow", v)} />
            </PanelSectionRow>
          </>
        )}
      </PanelSection>
      <PanelSection title="Hearth's apps">
        {[["hearth-app:store", "Store"], ["hearth-app:mods", "Mods"], ["hearth-app:theater", "Theater"]].map(([id, label]) => {
          const l = link(id);
          return l ? (
            <PanelSectionRow key={id}><ButtonItem layout="below" onClick={() => playItem(l)}>{`Hearth ${label}`}</ButtonItem></PanelSectionRow>
          ) : null;
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
          <ButtonItem layout="below" onClick={async () => say(`Removed ${await removeAll()} Hearth shortcuts from Steam`)}>
            Remove Hearth's shortcuts
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>
    </>
  );
}

// ---- Hearth's row on Steam's home: a hero with Hearth's wallpaper and logo, then your Hearth games like Hearth's home.
let homeRowOn = true;
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
function HearthHome(props: { children?: any }) {
  const games = useGames();
  const list = games?.games ?? [];
  const covers = useCovers(list);
  if (!homeRowOn || !list.length) return props.children ?? null;
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
  let patch: any = null;
  (async () => {
    const s = await getSettings().catch(() => ({ theme: true, homeRow: true }));
    homeRowOn = s.homeRow;
    brand = await getBrand().catch(() => null);
    if (s.theme && brand) { try { applyTheme(brand); } catch (e) { console.warn("Hearth: theme", e); } }
    loadGames();
  })();
  try {
    patch = routerHook.addPatch("/library/home", (route: any) => {
      if (route?.children && route.children.type !== HearthHome) route.children = <HearthHome>{route.children}</HearthHome>;
      return route;
    });
  } catch (e) { console.warn("Hearth: home row", e); }
  return {
    name: "Hearth",
    titleView: <div className={staticClasses.Title}>Hearth</div>,
    content: <Panel />,
    icon: <FaFire />,
    onDismount() {
      removeTheme();
      if (patch) { try { routerHook.removePatch("/library/home", patch); } catch { /* already gone */ } }
    },
  };
});
