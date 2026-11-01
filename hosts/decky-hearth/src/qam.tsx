// Hearth in Steam's Quick Access (the … button) and over the game you're playing:
//  - Hearth's own tabs beside Steam's (Hearth, Hearth Performance, Hearth Controller, Hearth Players: coop.tsx), added through Decky Loader's own
//    Quick Access tab list (window.DeckyPluginLoader.tabsHook: the list Decky adds its plug tab with). Decky has no
//    plugin API for top-level tabs, so this is guarded like every patch: if Decky's list isn't there or throws, the part
//    turns itself off (Quick Access › Hearth › Turned off by itself) and the same sections stay in Decky's Hearth tab;
//  - "This game": the running game's Hearth Input layout, its Proton (games Hearth starts), its mods in Hearth Mods and
//    Hearth's facts, used in the Hearth tab, in Decky's Hearth tab and on the Now playing page (ingame.tsx).
import * as DUI from "@decky/ui";
import { ButtonItem, DialogButton, DropdownItem, Focusable, PanelSection, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { callable } from "@decky/api";
import * as API from "@decky/api";
import { ReactNode, useEffect, useState } from "react";
import { FaFire, FaGamepad, FaTachometerAlt, FaUsers } from "react-icons/fa";
import { GameView, QAM_TABS, Settings, UpdateStatus, inputLine, layoutLabel, modsLine, partOn, protonLabel, qamPlan, tdpLine, updateView } from "./model";
import { ask, setSettings } from "./api";
import { Answer, gameView, inputSet, markShown, modsOf, setProton, tool, useFront } from "./core";
import { guard, setSettingsLocal, settings, useLive } from "./state";
import { Boundary, C, go } from "./ui";
import { openApp, say } from "./actions";
import { GameInfoLine } from "./pages";
import { PlayersBody } from "./coop";

const ACCENT = C.accent, INK = C.ink;
const switchToHearth = callable<[], { ok: boolean; error?: string }>("switch_to_hearth");

export async function changeSetting(k: keyof Settings, v: boolean) {
  const next = await setSettings({ ...settings, [k]: v });
  setSettingsLocal(next);
  return next;
}

const Line = (p: { children?: ReactNode; dim?: boolean }) => (
  <div className={staticClasses.Text} style={{ fontSize: p.dim ? 12 : 13, color: p.dim ? C.dim : undefined, lineHeight: 1.35 }}>{p.children}</div>
);

// ---------------------------------------------------------------- Performance

/** The power mode (Hearth's Quick Access › Performance, through hearth-power), when this device has one. */
export function PowerMode() {
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

/** Hearth Performance: Hearth's power mode, and Hearth's power limit read only. The frame limit and TDP are Steam's
 *  own Performance tab's in Steam Game Mode: Hearth doesn't set them here, so the two never fight. */
export function PerformanceBody() {
  const [p, setP] = useState<any>(null);
  useEffect(() => { ask("power", "", 10000).then(setP); }, []);
  const tdp = tdpLine(p);
  const none = p && !p.error && !p.mode?.supported && !tdp;
  return (
    <>
      <PowerMode />
      {tdp && <PanelSectionRow><Line>{tdp}</Line></PanelSectionRow>}
      {none && <PanelSectionRow><Line dim>This device has no power modes Hearth can set.</Line></PanelSectionRow>}
      <PanelSectionRow><Line dim>Frame limit, refresh rate and TDP: Steam's own Performance tab (Hearth leaves them to Steam here).</Line></PanelSectionRow>
    </>
  );
}

// ---------------------------------------------------------------- This game

const views = new Map<string, Answer<GameView>>();
/** The running game's in-game section (core_service.rs `game`), kept per game and asked again when changed. */
function useGameView(id: string): [Answer<GameView> | null, () => void] {
  const [v, setV] = useState<Answer<GameView> | null>(() => (id ? views.get(id) || null : null));
  const [n, setN] = useState(0);
  useEffect(() => {
    if (!id) { setV(null); return; }
    let live = true;
    gameView(id).then((r) => { if (r && !r.error) views.set(id, r); if (live) setV(r); }).catch((e) => { if (live) setV({ error: String(e) } as any); });
    return () => { live = false; };
  }, [id, n]);
  return [v, () => setN((x) => x + 1)];
}

/** The game's Hearth Input layout: Off, a template, your own; and Hearth's editor. */
export function InputPicker(props: { id: string; v: Answer<GameView>; reload: () => void }) {
  const v = props.v;
  const opts = (v.input?.choices || []).map((c) => ({ data: c.id, label: c.label }));
  return (
    <>
      <PanelSectionRow>
        <DropdownItem label="Hearth Input layout" description="Makes the controller a keyboard and mouse in this game"
          rgOptions={opts} selectedOption={v.input?.layout || "off"}
          onChange={async (o: any) => { const r = await inputSet(props.id, String(o.data)); if (r.error) say(r.error); else say(`Hearth Input: ${r.name || (o.label ?? o.data)}`); props.reload(); }} />
      </PanelSectionRow>
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => openApp("hearth-app:settings")} description="Hearth's layout editor opens in a window">Edit layouts in Hearth</ButtonItem>
      </PanelSectionRow>
    </>
  );
}

function ProtonPicker(props: { id: string; v: Answer<GameView>; reload: () => void }) {
  const v = props.v;
  if (v.kind === "steam") return <PanelSectionRow><Line dim>Proton: Steam's own (the game's Properties › Compatibility).</Line></PanelSectionRow>;
  if (v.kind !== "proton" || !v.protonChoices?.length) return null;
  return (
    <PanelSectionRow>
      <DropdownItem label="Proton (Play through Hearth)" description="Used the next time the game starts"
        rgOptions={v.protonChoices.map((c) => ({ data: c.id, label: c.label }))} selectedOption={v.proton || ""}
        onChange={async (o: any) => { const r = await setProton(props.id, String(o.data)); if (r.error) say(r.error); props.reload(); }} />
    </PanelSectionRow>
  );
}

function ModsRow(props: { id: string; v: Answer<GameView> }) {
  const [m, setM] = useState<any>(null);
  useEffect(() => { let live = true; if (props.v.mods) modsOf(props.id).then((r) => { if (live) setM(r); }).catch(() => {}); return () => { live = false; }; }, [props.id, props.v.mods]);
  if (!props.v.mods) return null;
  return (
    <PanelSectionRow>
      <ButtonItem layout="below" label="Hearth Mods" description={m ? (m.error ? m.error : modsLine(m)) : "Asking Hearth Mods…"} onClick={() => openApp("hearth-app:mods")}>
        Open Hearth Mods
      </ButtonItem>
    </PanelSectionRow>
  );
}

/** Hearth for the game you're playing. `idle`: say so when nothing runs (the Hearth tab), else draw nothing. */
export function ThisGame(props: { idle?: boolean; header?: boolean }) {
  const s = useLive();
  const { run, core } = useFront();
  const id = run?.id || "";
  const [v, reload] = useGameView(id);
  if (!partOn(s, "tkInGame", guard.broken)) return null;
  if (!run) return props.idle ? <PanelSectionRow><Line dim>No game is running.</Line></PanelSectionRow> : null;
  const body = !id ? <PanelSectionRow><Line dim>Not one of Hearth's games or a Steam game Hearth knows.</Line></PanelSectionRow>
    : !v ? <PanelSectionRow><Line dim>Asking Hearth…</Line></PanelSectionRow>
    : v.error ? <PanelSectionRow><Line dim>{core === false ? "Hearth's core isn't running, so the in-game settings aren't here this time." : v.error}</Line></PanelSectionRow>
    : (
      <>
        <InputPicker id={id} v={v} reload={reload} />
        <ProtonPicker id={id} v={v} reload={reload} />
        <ModsRow id={id} v={v} />
        <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}><GameInfoLine id={id} /></div></PanelSectionRow>
        <PanelSectionRow><ButtonItem layout="below" onClick={() => go("/hearth/now")}>Hearth's page for this game</ButtonItem></PanelSectionRow>
      </>
    );
  return (
    <Boundary part="tkInGame" fallback={null} onError={guard.fail}>
      {props.header !== false && (
        <PanelSectionRow>
          <div style={{ display: "flex", flexDirection: "column", gap: 2, padding: "4px 0" }}>
            <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: ".08em", textTransform: "uppercase", color: ACCENT }}>{run.hearth ? "Plays through Hearth" : "Now playing"}</div>
            <div style={{ fontSize: 16, fontWeight: 700, color: C.text }}>{run.title || v?.title || "This game"}</div>
            {v && !v.error && <div style={{ fontSize: 12, color: C.dim }}>{v.store} · Hearth Input: {layoutLabel(v)}{protonLabel(v) ? ` · ${protonLabel(v)}` : ""}</div>}
          </div>
        </PanelSectionRow>
      )}
      {body}
    </Boundary>
  );
}

/** "This game" as a section of Decky's Hearth tab: only while a game runs. */
export function ThisGameSection() {
  const s = useLive();
  const { run } = useFront();
  if (!run || !partOn(s, "tkInGame", guard.broken)) return null;
  return <PanelSection title="This game"><ThisGame /></PanelSection>;
}

// ---------------------------------------------------------------- Controller

export function ControllerBody() {
  const s = useLive();
  const { run, front } = useFront();
  const id = run?.id || "";
  const [v, reload] = useGameView(id);
  const on = partOn(s, "tkInput", guard.broken);
  return (
    <>
      <PanelSectionRow><Line>{inputLine(front, on)}</Line></PanelSectionRow>
      <PanelSectionRow>
        <ToggleField label="Hearth Input in Steam Game Mode" description="Steam's own layouts: Quick Access › Controller. Use one or the other for a game"
          checked={s.tkInput} disabled={guard.broken.has("tkInput")} onChange={(x) => changeSetting("tkInput", x)} />
      </PanelSectionRow>
      {id && v && !v.error && <InputPicker id={id} v={v} reload={reload} />}
      {!id && <PanelSectionRow><ButtonItem layout="below" onClick={() => openApp("hearth-app:settings")}>Hearth Input layouts (Hearth's Settings)</ButtonItem></PanelSectionRow>}
    </>
  );
}

// ---------------------------------------------------------------- Hearth

/** Steam's keyboard (Hearth's Guide + X in Hearth's Game Mode): opened through Steam where Steam lets it, else how. */
function openKeyboard() {
  const SC = (window as any).SteamClient;
  for (const f of [SC?.Input?.OpenVirtualKeyboard, SC?.Input?.ShowVirtualKeyboard]) {
    if (typeof f === "function") { try { f.call(SC.Input); return; } catch { /* the next */ } }
  }
  say("Steam's keyboard: hold the Steam button and press X");
}

async function runTool(what: "screenshot" | "record") {
  const r = await tool(what).catch((e) => ({ error: String(e) }));
  if (r?.error) say(r.error);
  else if (what === "screenshot") say("Screenshot saved in Pictures › Screenshots › Hearth");
  else say(r?.recording === false ? "Recording saved" : "Recording… press Record again to stop");
}

const updCheck = callable<[], UpdateStatus>("update_check");
const updStart = callable<[], UpdateStatus>("update_start");
const updStatus = callable<[], UpdateStatus>("update_status");

/** Update Hearth OS: the same updater as Steam's Settings › System › Check for updates (hearth-steamos-update). It
 *  downloads and stages the update; the device never restarts by itself (restart from Steam's power menu). */
export function UpdateRow() {
  const [u, setU] = useState<UpdateStatus | null>(null);
  useEffect(() => { updStatus().then(setU).catch(() => {}); }, []);
  const v = updateView(u);
  useEffect(() => {
    if (!v.poll) return;
    const t = setInterval(() => { updStatus().then(setU).catch(() => {}); }, 1500);
    return () => clearInterval(t);
  }, [v.poll]);
  const act = async () => {
    if (v.action === "check") { setU({ state: "checking" }); setU(await updCheck().catch((e) => ({ state: "error", message: String(e) }))); }
    else if (v.action === "start") setU(await updStart().catch((e) => ({ state: "error", message: String(e) })));
  };
  return (
    <PanelSectionRow>
      {v.line && <div className={staticClasses.Text} style={{ fontSize: 12, marginBottom: 6 }}>{v.line}</div>}
      {v.pct != null && v.pct < 100 && <div style={{ height: 6, borderRadius: 3, background: C.panel3, overflow: "hidden", marginBottom: 6 }}><div style={{ width: `${v.pct}%`, height: "100%", background: ACCENT }} /></div>}
      {v.button && <ButtonItem layout="below" onClick={act} description="Downloads Hearth OS's update; it finishes when you restart, and the version before stays">{v.button}</ButtonItem>}
    </PanelSectionRow>
  );
}

export function SwitchButton() {
  return (
    <PanelSectionRow>
      <ButtonItem layout="below" onClick={async () => { const r = await switchToHearth(); if (!r.ok) say(r.error ?? "Couldn't switch"); }}
        description="Steam closes and Hearth's own Game Mode comes back">
        Switch to Hearth Game Mode
      </ButtonItem>
    </PanelSectionRow>
  );
}

export function ToolsBody() {
  return (
    <>
      <PanelSectionRow><ButtonItem layout="below" onClick={openKeyboard} description="Steam's on-screen keyboard (Steam + X)">Keyboard</ButtonItem></PanelSectionRow>
      <PanelSectionRow><ButtonItem layout="below" onClick={() => runTool("screenshot")} description="Hearth's screenshot: Pictures › Screenshots › Hearth">Screenshot</ButtonItem></PanelSectionRow>
      <PanelSectionRow><ButtonItem layout="below" onClick={() => runTool("record")} description="Hearth Recorder: start or stop recording">Record</ButtonItem></PanelSectionRow>
    </>
  );
}

function HearthBody() {
  return (
    <>
      <PanelSection title="This game"><ThisGame idle /></PanelSection>
      <PanelSection title="Hearth tools"><ToolsBody /></PanelSection>
      <PanelSection title="Hearth"><SwitchButton /></PanelSection>
      <PanelSection title="Hearth OS updates"><UpdateRow /></PanelSection>
    </>
  );
}

// ---------------------------------------------------------------- the tabs

/** Marks Quick Access as open while a Hearth tab is shown (Hearth Input lets go of the controller meanwhile). */
function Shown(props: { k: string; children?: ReactNode }) {
  const useVisible = (API as any).useQuickAccessVisible as (() => boolean) | undefined;
  let visible = true;
  try { if (useVisible) visible = useVisible() !== false; } catch { visible = true; }
  useEffect(() => { markShown(props.k, visible); return () => markShown(props.k, false); }, [visible]);
  return <>{props.children}</>;
}

/** One of Hearth's Quick Access tabs, in Steam's scrolling panel. */
export function QamPanel(props: { which: "hearth" | "perf" | "controller" | "players" }) {
  const scroll = (DUI as any).scrollClasses || (DUI as any).scrollPanelClasses || {};
  const cls = [(staticClasses as any).TabGroupPanel, scroll.ScrollPanel, scroll.ScrollY].filter(Boolean).join(" ");
  return (
    <Shown k={"qam-" + props.which}>
      <div className={cls} style={{ height: "100%", overflowY: "auto" }}>
        <Boundary part="tkQam" onError={guard.fail}>
          {props.which === "hearth" && <HearthBody />}
          {props.which === "perf" && <PanelSection title="Hearth Performance"><PerformanceBody /></PanelSection>}
          {props.which === "controller" && <PanelSection title="Hearth Controller"><ControllerBody /></PanelSection>}
          {props.which === "players" && <PanelSection title="Players"><PlayersBody /></PanelSection>}
        </Boundary>
      </div>
    </Shown>
  );
}

const ICONS = { hearth: <FaFire />, perf: <FaTachometerAlt />, controller: <FaGamepad />, players: <FaUsers /> };
let added: number[] = [];

function tabsHook(): any {
  const hook = (window as any).DeckyPluginLoader?.tabsHook;
  if (!hook || typeof hook.add !== "function" || typeof hook.removeById !== "function") throw new Error("Decky's Quick Access tab list isn't reachable (Decky changed)");
  return hook;
}

export function removeQamTabs() {
  let hook: any = null;
  try { hook = tabsHook(); } catch { added = []; return; }
  for (const id of added) { try { hook.removeById(id); } catch { /* gone */ } }
  added = [];
}

/** Adds or takes away Hearth's Quick Access tabs to match the switches. A throw: the part turns itself off and the
 *  tabs already added are taken away (Decky's own Hearth tab still has the same sections). */
export function syncQamTabs() {
  if (guard.broken.has("tkQam")) { removeQamTabs(); return; }
  try {
    const hook = tabsHook();
    const plan = qamPlan(settings, guard.broken, added);
    for (const id of plan.remove) { try { hook.removeById(id); } catch { /* gone */ } added = added.filter((x) => x !== id); }
    for (const id of plan.add) {
      const t = QAM_TABS.find((x) => x.id === id)!;
      hook.add({ id, title: t.title, icon: ICONS[t.key], content: <QamPanel which={t.key} /> });
      added.push(id);
    }
  } catch (e) {
    removeQamTabs();
    guard.fail("tkQam", e);
  }
}
