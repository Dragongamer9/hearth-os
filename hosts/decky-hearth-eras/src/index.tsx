// Hearth Eras (docs/HEARTH-ERAS.md): Steam Game Mode's look as home screens inspired by past console eras, Hearth's own
// designs. Quick Access › Hearth Eras has the gallery of eras; "Steam" (Steam's own look) is the default and is never
// written by itself. A picked era brings its skin (skin.ts, the way Hearth Classic styled Steam on a device) and its
// home on Steam's Home route (homes.tsx); the extras (sounds, quick switching, loading screen, keyboard, pages) are off
// until turned on. Steam's UI is checked first (checkSteam): if what an era needs is missing, nothing is applied, the
// panel says "Hearth Eras paused: Steam changed its layout" and the reason goes to Decky's log. Every change to Steam is
// in one PatchSet, so switching back, a failure or unloading takes all of it out. Steam's menu window is never resized
// or reshaped (Steam ignores that; Hearth Classic's lesson). Off Hearth OS only the Hearth OS panel shows.
import { ButtonItem, DialogButton, DropdownItem, Focusable, PanelSection, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { callable, definePlugin, routerHook, toaster } from "@decky/api";
import { ReactNode, useEffect, useState } from "react";
import { FaFire } from "react-icons/fa";
import { HearthOnly, hearthOs } from "./hearth-only";
import { checkSteam, classKeys, makePatchSet } from "./classic/model";
import { Boundary, setSounds as setClassicSounds } from "./classic/home";
import type { Brand, EraId, QuickAction, Settings, State } from "./model";
import { DEFAULTS, ERAS, PAUSED_NOTE, applyQuick, buildSkin, decide, diagnosticsText, eraById, normalize, presetOf, quickLabel } from "./model";
import { applySkin, gameRunning, removeSkin, steamClass, windowNames } from "./skin";
import { quick, setEraSounds, startQuickSwitch } from "./extras";
import { EraHome } from "./homes";

const getSettings = callable<[], unknown>("get_settings");
const setSettingsCall = callable<[Partial<Settings>], { ok: boolean; settings?: unknown; error?: string }>("set_settings");
const getBrand = callable<[], Brand | null>("brand");
const report = callable<[string], null>("report");
const writeDiag = callable<[string], { ok: boolean; path?: string }>("diagnostics");
const sendLog = callable<[string, string], { ok: boolean; code?: string; via?: string; skipped?: string; error?: string }>("send_log");

// ---------------------------------------------------------------- the live state

const live = {
  hearth: false, loaded: false, settings: { ...DEFAULTS } as Settings, state: "off" as State, why: "",
  brand: null as Brand | null, dropped: 0,
};
const listeners = new Set<() => void>();
const notify = () => {
  try { (window as any).HearthEras = { state: live.state, why: live.why, era: live.settings.era, quick: { ...quick }, dropped: live.dropped }; } catch { /* fine */ }
  listeners.forEach((f) => { try { f(); } catch { /* a panel that went away */ } });
};
const patches = makePatchSet((m) => { console.warn("Hearth Eras:", m); report(m).catch(() => {}); });
let stopQuick: (() => void) | null = null;
let saidWhy = "";

function useLive() {
  const [, bump] = useState(0);
  useEffect(() => { const f = () => bump((n) => n + 1); listeners.add(f); return () => { listeners.delete(f); }; }, []);
  return live;
}

function diagnostics(): string {
  const css = live.settings.era === "classic" ? (live.brand?.eras?.classic || "") : (live.brand ? live.brand.base + "\n" + (live.brand.eras?.[live.settings.era] || "") : "");
  return diagnosticsText({ state: live.state, why: live.why, settings: live.settings, keys: classKeys(css, steamClass), windows: windowNames(),
    dropped: live.dropped, quick: live.settings.quickSwitch ? (quick.on ? "listening (" + quick.how + ")" : quick.how) + (quick.last ? ", last " + quick.last : "") : "off" });
}

/** Takes the era out of Steam (the skin, the home patch, the sounds). Safe to call any number of times. */
function stopAll() {
  patches.removeAll();
  removeSkin();
  setEraSounds(null);
  setClassicSounds(false);
}

function pause(why: string) {
  stopAll();
  live.state = "paused";
  live.why = why;
  if (why !== saidWhy) {
    saidWhy = why;
    console.warn(`Hearth Eras: ${PAUSED_NOTE} (${why}); Steam's normal look is back`);
    report(`${PAUSED_NOTE} (${why}); Steam's normal look is back`).catch(() => {});
    writeDiag(diagnostics()).catch(() => {});
  }
  notify();
}

/** Quick switching listens whenever it's on (also on Steam's look, to switch to an era), only on Hearth OS. */
function syncQuick() {
  const want = live.hearth && live.settings.quickSwitch;
  if (want && !stopQuick) stopQuick = startQuickSwitch(onQuick, gameRunning);
  if (!want && stopQuick) { stopQuick(); stopQuick = null; }
}

function onQuick(a: QuickAction) {
  const change = applyQuick(live.settings, a);
  if (!change) return;
  save(change).then((err) => {
    try { toaster.toast({ title: "Hearth Eras", body: err || quickLabel(live.settings) }); } catch { /* no toasts here */ }
  });
}

/** Decides and applies: Steam's look, an era, or paused. The saved choice itself is never touched here. */
function refresh(failure?: string) {
  stopAll();
  syncQuick();
  const s = live.settings;
  const on = s.era !== "steam";
  const check = live.hearth && on ? checkSteam(steamClass, !!(routerHook && typeof (routerHook as any).addPatch === "function")) : null;
  const skin = on ? buildSkin(live.brand, s, steamClass) : { css: "", dropped: 0 };
  const d = decide(s, live.hearth, check, failure || (on && !skin.css ? "the era's skin couldn't be read" : undefined));
  if (d.state === "paused") { pause(d.why); return; }
  if (d.state === "off") { live.state = "off"; live.why = ""; live.dropped = 0; notify(); return; }
  try {
    live.dropped = skin.dropped;
    patches.add("skin", removeSkin);
    if (!applySkin(skin.css)) throw new Error("the skin couldn't be put in Steam's window");
    patchHome();
    if (s.sounds) {
      if (s.era === "classic") setClassicSounds(true); else setEraSounds(s.era);
    }
    live.state = "on"; live.why = "";
    notify();
  } catch (e) {
    pause(e instanceof Error ? e.message : String(e));
  }
}

// ---------------------------------------------------------------- the home on Steam's Home route

function HomeSwitch(props: { steam: any }) {
  const l = useLive();
  if (l.state !== "on" || l.settings.era === "steam") return props.steam;
  const b = l.brand!;
  return (
    <Boundary key={l.settings.era} fallback={props.steam} onError={(e) => setTimeout(() => pause("the home screen failed: " + (e instanceof Error ? e.message : String(e)).slice(0, 200)), 0)}>
      <EraHome era={l.settings.era} preset={presetOf(l.settings, l.settings.era)} steam={props.steam} wallpaper={b.wallpaper || ""} logo={b.logo || ""} />
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

/** A tiny picture of each era, drawn here (no images): how its home is laid out, in its first preset's colours. */
function Thumb(props: { era: EraId }) {
  const e = eraById(props.era);
  const v = e.presets[0] ? e.presets[0].vars : { bg: "#1b2838", bg2: "#171a21", panel: "#2a475e", panel2: "#3d4450", accent: "#66c0f4", text: "#ffffff" } as Record<string, string>;
  const box = (s: Record<string, any>) => <div key={parts.length} style={Object.assign({ position: "absolute", borderRadius: 2 }, s) as any} />;
  const parts: ReactNode[] = [];
  if (props.era === "channels") for (let i = 0; i < 8; i++) parts.push(box({ left: 4 + (i % 4) * 13, top: 4 + Math.floor(i / 4) * 10, width: 11, height: 8, borderRadius: 3, background: i === 1 ? v.accent : v.panel2, border: "1px solid " + (v.line || v.dim) }));
  else if (props.era === "blades") for (let i = 0; i < 4; i++) parts.push(box({ left: i < 1 ? 4 + i * 6 : i === 1 ? 11 : 37 + (i - 2) * 7, top: 4, width: i === 1 ? 25 : 5, height: 28, background: i === 1 ? v.panel2 : v.panel, borderLeft: i === 1 ? "2px solid " + v.accent : undefined }));
  else if (props.era === "tiles") { parts.push(box({ left: 0, top: 0, width: 56, height: 15, background: v.panel2 })); parts.push(box({ left: 4, top: 18, width: 18, height: 12, background: v.accent })); for (let i = 0; i < 4; i++) parts.push(box({ left: 24 + i * 8, top: 18, width: 6, height: 12, background: v.panel })); }
  else if (props.era === "crossbar") { for (let i = 0; i < 4; i++) parts.push(box({ left: 10 + i * 11, top: 6, width: 6, height: 6, borderRadius: 3, background: i === 0 ? "#fff" : "rgba(255,255,255,.5)" })); for (let i = 0; i < 3; i++) parts.push(box({ left: 10, top: 16 + i * 6, width: 30, height: 3, background: "rgba(255,255,255," + (i ? ".4" : ".9") + ")" })); }
  else if (props.era === "pocket") { parts.push(box({ left: 4, top: 4, width: 26, height: 14, borderRadius: 4, background: v.panel2 })); for (let i = 0; i < 5; i++) parts.push(box({ left: 4 + i * 10, top: 23, width: 8, height: 8, borderRadius: 3, background: i === 0 ? v.accent : v.panel2 })); }
  else if (props.era === "classic") { parts.push(box({ left: 4, top: 4, width: 24, height: 12, background: v.panel2 })); for (let i = 0; i < 5; i++) parts.push(box({ left: 4 + i * 10, top: 20, width: 8, height: 11, borderRadius: 2, background: i === 0 ? v.accent : v.panel2 })); }
  else for (let i = 0; i < 4; i++) parts.push(box({ left: 4 + i * 13, top: 10, width: 11, height: 16, background: i === 0 ? v.accent : v.panel }));
  const bg = props.era === "crossbar" ? `linear-gradient(135deg,${v.bg},${v.bg2})` : props.era === "channels" ? `repeating-linear-gradient(180deg,${v.bg} 0 2px,${v.bg2} 2px 3px)` : v.bg;
  return <div className="hearth-eras-thumb" style={{ position: "relative", width: 56, height: 34, flex: "none", borderRadius: 5, overflow: "hidden", background: bg, boxShadow: "inset 0 0 0 1px rgba(255,255,255,.12)" }}>{parts}</div>;
}

function Gallery(props: { era: EraId; set: (c: Partial<Settings>) => void }) {
  return (
    <Focusable className="hearth-eras-gallery" style={{ display: "flex", flexDirection: "column", gap: 6 }} {...({ "flow-children": "vertical" } as any)}>
      {ERAS.map((e) => (
        <DialogButton key={e.id} className={"hearth-eras-pick" + (e.id === props.era ? " picked" : "")} data-era={e.id}
          style={{ display: "flex", alignItems: "center", gap: 10, padding: "6px 8px", minWidth: 0, textAlign: "left", border: e.id === props.era ? "2px solid #3fd0b8" : "2px solid transparent" }}
          onClick={() => { if (e.id !== props.era) props.set({ era: e.id }); }}>
          <Thumb era={e.id} />
          <div style={{ minWidth: 0 }}>
            <div style={{ fontSize: 14, fontWeight: 700 }}>{e.name}{e.id === props.era ? "  ✓" : ""}</div>
            <div style={{ fontSize: 11, opacity: 0.75, lineHeight: 1.25, whiteSpace: "normal" }}>{e.blurb}</div>
          </div>
        </DialogButton>
      ))}
    </Focusable>
  );
}

function Diagnostics() {
  useLive();
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState("");
  const text = diagnostics();
  return (
    <PanelSection title="Diagnostics">
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => setOpen(!open)}>{open ? "Hide what Hearth Eras found" : "Show what Hearth Eras found"}</ButtonItem>
      </PanelSectionRow>
      {open && <PanelSectionRow><pre className="hearth-eras-diag" style={{ whiteSpace: "pre-wrap", fontSize: 10.5, lineHeight: 1.35, margin: 0 }}>{text}</pre></PanelSectionRow>}
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => {
          setNote("Sending…");
          writeDiag(text).catch(() => {});
          sendLog("sent by hand from the panel", text).then((r) => setNote(r && r.ok && r.code ? "Sent: code " + r.code + (r.via === "curl" ? " (through the system's curl)" : "") : (r && (r.error || r.skipped)) || "Couldn't send it"))
            .catch(() => setNote("Couldn't send it"));
        }}>Send Hearth Eras log</ButtonItem>
      </PanelSectionRow>
      {note && <PanelSectionRow><div className="hearth-eras-note" style={{ fontSize: 11.5, opacity: 0.8 }}>{note}</div></PanelSectionRow>}
    </PanelSection>
  );
}

function Panel() {
  const l = useLive();
  const [err, setErr] = useState("");
  const s = l.settings;
  const set = (c: Partial<Settings>) => { setErr(""); save(c).then(setErr); };
  if (!l.loaded) return <PanelSection><PanelSectionRow><div className={staticClasses.Text}>Loading…</div></PanelSectionRow></PanelSection>;
  const era = eraById(s.era);
  const preset = presetOf(s, s.era);
  return (
    <>
      <PanelSection title="Eras">
        <PanelSectionRow><Gallery era={s.era} set={set} /></PanelSectionRow>
        {era.presets.length > 1 && preset && (
          <PanelSectionRow>
            <DropdownItem label="Colours" rgOptions={era.presets.map((p, i) => ({ data: i, label: p.name }))} selectedOption={era.presets.indexOf(preset)}
              onChange={(o: any) => set({ presets: { [s.era]: Number(o.data) } })} />
          </PanelSectionRow>
        )}
        {l.state === "paused" && (
          <>
            <PanelSectionRow>
              <div className="hearth-eras-paused" style={{ padding: "8px 10px", borderRadius: 8, background: "rgba(255,181,71,.12)", border: "1px solid rgba(255,181,71,.4)", fontSize: 13, lineHeight: 1.35 }}>
                <div style={{ fontWeight: 700, color: "#ffb547" }}>{PAUSED_NOTE}</div>
                <div style={{ fontSize: 12, marginTop: 2, opacity: 0.85 }}>Steam's normal look is on meanwhile; your choice is kept. {l.why}</div>
              </div>
            </PanelSectionRow>
            <PanelSectionRow><ButtonItem layout="below" onClick={() => refresh()}>Try again</ButtonItem></PanelSectionRow>
          </>
        )}
      </PanelSection>
      <PanelSection title="Extras">
        <PanelSectionRow><ToggleField label="Era sounds" description="Each era's own soft sounds on its home screen (made by Hearth)." checked={s.sounds} onChange={(v: boolean) => set({ sounds: v })} /></PanelSectionRow>
        <PanelSectionRow><ToggleField label="Quick switching" description="Hold L1: D-pad left/right changes the era, L2/R2 its colours (not while a game runs)." checked={s.quickSwitch} onChange={(v: boolean) => set({ quickSwitch: v })} /></PanelSectionRow>
        <PanelSectionRow><ToggleField label="Loading screen style" description="Steam's loading spinner in the era's colours." checked={s.loading} onChange={(v: boolean) => set({ loading: v })} /></PanelSectionRow>
        <PanelSectionRow><ToggleField label="On-screen keyboard skin" description="The keyboard in the era's colours." checked={s.keyboard} onChange={(v: boolean) => set({ keyboard: v })} /></PanelSectionRow>
        <PanelSectionRow><ToggleField label="Styled Library pages" description="The era's background and panels on the Library, game details and search." checked={s.pages} onChange={(v: boolean) => set({ pages: v })} /></PanelSectionRow>
      </PanelSection>
      <Diagnostics />
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
    name: "Hearth Eras",
    titleView: <div className={staticClasses.Title}>Hearth Eras</div>,
    content: <HearthOnly name="Hearth Eras" what="Home screens inspired by past console eras, on Steam Game Mode."><Panel /></HearthOnly>,
    icon: <FaFire />,
    onDismount() {
      gone = true; stopAll();
      if (stopQuick) { stopQuick(); stopQuick = null; }
      live.state = "off"; live.why = ""; notify();
    },
  };
});
