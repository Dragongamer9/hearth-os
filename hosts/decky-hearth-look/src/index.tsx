// Hearth Classic (docs/HEARTH-LOOK.md): the old Hearth Game Mode look on top of Steam's Game Mode. Stage 1 is the skin
// (theme.ts), stage 2 the home screen on Steam's Home (home.tsx). Off until the person picks "Hearth Classic" in this
// panel; the plugin never changes that choice itself. At every start (and every change) Steam's UI is checked first
// (model.ts checkSteam): if what the look needs is missing, nothing is applied, the panel says "Hearth Classic paused:
// Steam changed its layout" and the reason goes to Decky's log. Every change to Steam is in one PatchSet, so turning it
// off, a failure or unloading the plugin takes all of it out again. Off Hearth OS only the Hearth OS panel shows.
import { ButtonItem, DropdownItem, PanelSection, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { callable, definePlugin, routerHook } from "@decky/api";
import { useEffect, useState } from "react";
import { FaFire } from "react-icons/fa";
import { HearthOnly, hearthOs } from "./hearth-only";
import { DEFAULTS, PAUSED_NOTE, Settings, State, checkSteam, classKeys, decide, diagnosticsText, makePatchSet, menuNote, menuPlan, normalize, renderTheme } from "./model";
import { MenuDiag, applySkin, gameRunning, menuDiag, removeSkin, startMenuBar, steamClass, windowNames } from "./theme";
import { Boundary, HearthHome, setSounds } from "./home";

const getSettings = callable<[], unknown>("get_settings");
const setSettingsCall = callable<[Partial<Settings>], { ok: boolean; settings?: unknown; error?: string }>("set_settings");
const getBrand = callable<[], { css: string; logo: string; wallpaper: string } | null>("brand");
const report = callable<[string], null>("report");
const writeDiag = callable<[string], { ok: boolean; path?: string }>("diagnostics");

// ---------------------------------------------------------------- the live state

const live = {
  hearth: false, loaded: false, settings: { ...DEFAULTS } as Settings,
  state: "off" as State, why: "", menuNote: "", brand: null as { css: string; logo: string; wallpaper: string } | null,
};
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((f) => { try { f(); } catch { /* a panel that went away */ } });
const patches = makePatchSet((m) => { console.warn("Hearth Classic:", m); report(m).catch(() => {}); });
let saidWhy = "";
let lastDiag = "";
/** Everything a device test needs to tell what Hearth Classic found (the panel's Diagnostics and the log). */
function diagnostics(): string {
  const keys = live.brand ? classKeys(live.brand.css, steamClass) : { resolved: [], missing: [] };
  return diagnosticsText({ state: live.state, why: live.why, settings: live.settings, keys, menu: menuDiag, windows: windowNames(), game: gameRunning() });
}
/** The menu's state changed: the panel's note, and one line in the log (Hearth's Logs app reads it too). */
function onMenu(d: MenuDiag) {
  try { (window as any).HearthClassicMenu = d; } catch { /* fine */ }   // for a look in Steam's console (and the CI pictures)
  live.menuNote = menuNote(d, live.settings);
  const line = `menu: ${d.layout}, method ${d.method}${d.why ? " (" + d.why + ")" : ""}; found ${d.found ? "yes" : "no"}, ${d.entries} entries [${d.labels.join(", ")}], game ${d.game ? "running" : "none"}` +
    (d.before ? `; window ${d.before} -> ${d.after} with ${d.resizeWith}` : "");
  if (line !== lastDiag) { lastDiag = line; writeDiag(line).catch(() => {}); }
  notify();
}

function useLive() {
  const [, bump] = useState(0);
  useEffect(() => { const f = () => bump((n) => n + 1); listeners.add(f); return () => { listeners.delete(f); }; }, []);
  return live;
}

/** Takes every change out of Steam (the skin, the home patch, the sounds). Safe to call any number of times. */
function stopAll() {
  patches.removeAll();
  removeSkin();
  setSounds(false);
}

function pause(why: string) {
  stopAll();
  live.state = "paused";
  live.why = why;
  if (why !== saidWhy) {
    saidWhy = why;
    console.warn(`Hearth Classic: ${PAUSED_NOTE} (${why}); Steam's normal look is back`);
    report(`${PAUSED_NOTE} (${why}); Steam's normal look is back`).catch(() => {});
  }
  notify();
}

/** Decides and applies: Steam's look, Hearth Classic, or paused. The saved choice itself is never touched here. */
function refresh(failure?: string) {
  stopAll();
  const s = live.settings;
  const check = live.hearth && s.look === "classic" ? checkSteam(steamClass, !!(routerHook && typeof (routerHook as any).addPatch === "function")) : null;
  const d = decide(s, live.hearth, check, failure || (s.look === "classic" && !(live.brand && live.brand.css) ? "Hearth's look files couldn't be read" : undefined));
  if (d.state === "paused") { pause(d.why); return; }
  live.menuNote = "";
  if (d.state === "off") { live.state = "off"; live.why = ""; notify(); return; }
  try {
    const b = live.brand!;
    // The menu bars: Steam's real menu is checked in its own window before (and after) the bar goes on (theme.ts).
    const plan = menuPlan(s);
    const { css } = renderTheme(b.css, { logo: b.logo, wallpaper: s.wallpaper ? b.wallpaper : "" }, steamClass, plan.sections);
    patches.add("skin", removeSkin);
    if (!applySkin(css)) throw new Error("the skin couldn't be put in Steam's window");
    if (plan.sections.menu) patches.add("menu bar", startMenuBar({ top: plan.sections.top, onChange: onMenu }));
    if (s.home) patchHome();
    setSounds(s.sounds);
    patches.add("sounds", () => setSounds(false));
    live.state = "on"; live.why = "";
    notify();
  } catch (e) {
    pause(e instanceof Error ? e.message : String(e));
  }
}

// ---------------------------------------------------------------- stage 2: the home on Steam's Home route

function HomeSwitch(props: { steam: any }) {
  const l = useLive();
  if (l.state !== "on" || !l.settings.home) return props.steam;
  const b = l.brand!;
  return (
    <Boundary fallback={props.steam} onError={(e) => setTimeout(() => pause("the home screen failed: " + (e instanceof Error ? e.message : String(e)).slice(0, 200)), 0)}>
      <HearthHome steam={props.steam} wallpaper={l.settings.wallpaper ? b.wallpaper : ""} logo={b.logo} />
    </Boundary>
  );
}

function patchHome() {
  const fn = (route: any) => {
    try {
      const kids = route?.children;
      if (kids && kids.type !== HomeSwitch) route.children = <HomeSwitch steam={kids} />;
    } catch (e) { setTimeout(() => pause("Steam's Home route changed: " + String(e).slice(0, 200)), 0); }
    return route;
  };
  const handle = (routerHook as any).addPatch("/library/home", fn);
  patches.add("home", () => { (routerHook as any).removePatch("/library/home", handle); });
}

// ---------------------------------------------------------------- the panel

async function save(change: Partial<Settings>): Promise<string> {
  try {
    const r = await setSettingsCall(change);
    if (!r || !r.ok) return (r && r.error) || "Couldn't save";
    live.settings = normalize(r.settings);
    refresh();
    return "";
  } catch (e) {
    return "Couldn't save: " + String(e).slice(0, 120);
  }
}

/** What Hearth Classic found in this Steam, and a button that writes it to the log (for a device test). */
function Diagnostics() {
  useLive();
  const [open, setOpen] = useState(false);
  const [saved, setSaved] = useState("");
  const text = diagnostics();
  return (
    <PanelSection title="Diagnostics">
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => setOpen(!open)}>{open ? "Hide what Hearth Classic found" : "Show what Hearth Classic found"}</ButtonItem>
      </PanelSectionRow>
      {open && <PanelSectionRow><pre className="hearth-diag" style={{ whiteSpace: "pre-wrap", fontSize: 10.5, lineHeight: 1.35, color: "#b8bcbf", margin: 0 }}>{text}</pre></PanelSectionRow>}
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => { setSaved(""); writeDiag(text).then((r) => setSaved(r && r.ok ? "Written to the log" + (r.path ? " (" + r.path + ")" : "") : "Couldn't write it")).catch(() => setSaved("Couldn't write it")); }}>
          Write a Hearth Classic log
        </ButtonItem>
      </PanelSectionRow>
      {saved && <PanelSectionRow><div style={{ fontSize: 11, color: "#8d99a8" }}>{saved}</div></PanelSectionRow>}
    </PanelSection>
  );
}

const LOOKS = [{ data: "classic", label: "Hearth Classic" }, { data: "steam", label: "Steam" }];

function Panel() {
  const l = useLive();
  const [err, setErr] = useState("");
  const s = l.settings;
  const set = (c: Partial<Settings>) => { setErr(""); save(c).then(setErr); };
  if (!l.loaded) return <PanelSection><PanelSectionRow><div className={staticClasses.Text}>Loading…</div></PanelSectionRow></PanelSection>;
  const classic = s.look === "classic";
  return (
    <>
      <PanelSection title="Look">
        <PanelSectionRow>
          <DropdownItem label="Game Mode look" rgOptions={LOOKS} selectedOption={s.look}
            description={classic ? "Hearth's colours, cards, focus glow and home screen over Steam." : "Steam's own look. Pick Hearth Classic for the old Hearth Game Mode look."}
            onChange={(o: any) => set({ look: o.data === "classic" ? "classic" : "steam" })} />
        </PanelSectionRow>
        {l.state === "paused" && (
          <>
            <PanelSectionRow>
              <div className="hearth-paused" style={{ padding: "8px 10px", borderRadius: 8, background: "rgba(255,181,71,.12)", border: "1px solid rgba(255,181,71,.4)", fontSize: 13, lineHeight: 1.35 }}>
                <div style={{ fontWeight: 700, color: "#ffb547" }}>{PAUSED_NOTE}</div>
                <div style={{ color: "#b8bcbf", fontSize: 12, marginTop: 2 }}>Steam's normal look is on meanwhile; your choice is kept. {l.why}</div>
              </div>
            </PanelSectionRow>
            <PanelSectionRow><ButtonItem layout="below" onClick={() => refresh()}>Try again</ButtonItem></PanelSectionRow>
          </>
        )}
      </PanelSection>
      {classic && (
        <PanelSection title="Hearth Classic">
          <PanelSectionRow><ToggleField label="Hearth home screen" description="The old Hearth home on Steam's Home: your games, Hearth's places, a row per store." checked={s.home} onChange={(v: boolean) => set({ home: v })} /></PanelSectionRow>
          <PanelSectionRow><ToggleField label="Hearth wallpaper" description="Behind the home and the Library." checked={s.wallpaper} onChange={(v: boolean) => set({ wallpaper: v })} /></PanelSectionRow>
          <PanelSectionRow><ToggleField label="Hearth bottom bar menu" description="The Steam button's menu as Hearth's bar along the bottom." checked={s.menu} onChange={(v: boolean) => set({ menu: v })} /></PanelSectionRow>
          {s.menu && <PanelSectionRow><ToggleField label="In a game: Hearth top bar" description="While a game runs, the menu is Hearth's bar at the top instead." checked={s.menuTop} onChange={(v: boolean) => set({ menuTop: v })} /></PanelSectionRow>}
          {l.state === "on" && s.menu && l.menuNote && <PanelSectionRow><div className="hearth-menu-note" style={{ fontSize: 12, color: "#ffb547", lineHeight: 1.35 }}>{l.menuNote}</div></PanelSectionRow>}
          <PanelSectionRow><ToggleField label="Hearth sounds" description="Hearth's soft sounds when moving on the home screen." checked={s.sounds} onChange={(v: boolean) => set({ sounds: v })} /></PanelSectionRow>
        </PanelSection>
      )}
      {classic && <Diagnostics />}
      {err && <PanelSection><PanelSectionRow><div style={{ color: "#ff6b6b", fontSize: 12 }}>{err}</div></PanelSectionRow></PanelSection>}
    </>
  );
}

export default definePlugin(() => {
  let gone = false;
  hearthOs().then(async (ok) => {
    if (!ok || gone) return;   // not Hearth OS: nothing is applied, the panel is the Hearth OS invitation
    live.hearth = true;
    try { live.settings = normalize(await getSettings()); } catch { live.settings = { ...DEFAULTS }; }
    try { live.brand = await getBrand(); } catch { live.brand = null; }
    live.loaded = true;
    if (!gone) refresh();
  });
  return {
    name: "Hearth Classic",
    titleView: <div className={staticClasses.Title}>Hearth Classic</div>,
    content: <HearthOnly name="Hearth Classic" what="The old Hearth Game Mode look on Steam Game Mode."><Panel /></HearthOnly>,
    icon: <FaFire />,
    onDismount() { gone = true; stopAll(); live.state = "off"; live.why = ""; notify(); },
  };
});

