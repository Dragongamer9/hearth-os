// Hearth's pages that change things inside Steam's Game Mode: Mods (a game's mods on and off, uninstall, install a
// finished download), Storage (Settings › Storage: move a game, uninstall, clean up, use a drive for games) and
// Recomps (install, progress, play). Everything goes through Hearth's core service (api.ts hearthCall / modsCall /
// gamesDisk); when it isn't running the page says so and nothing else happens. Steam's own games are moved,
// installed and uninstalled by Steam (its pages are opened instead): Hearth's way closes Steam, which runs this mode.
import { DialogButton, Focusable } from "@decky/ui";
import { ReactNode, useEffect, useState } from "react";
import { gamesDisk, hearthCall, modsCall, usePoll } from "./api";
import { openApp, playGame, say, steamPage } from "./actions";
import { bytes, percent } from "./model";
import { Boundary, C, Chips, Page, blip, go, onFocus } from "./ui";
import { guard } from "./state";
import { brand } from "./pages";

const H = { "flow-children": "horizontal" } as any;
const V = { "flow-children": "vertical" } as any;

/** A button; `confirm`: the first press asks ("Press again to …"), the second does it. */
export function Act(props: { label: string; primary?: boolean; confirm?: string; disabled?: boolean; autoFocus?: boolean; onClick: () => void }) {
  const [asking, setAsking] = useState(false);
  return (
    <DialogButton className={"ht-btn" + (props.primary ? " primary" : "")} disabled={props.disabled}
      onClick={() => {
        blip("ok");
        if (props.confirm && !asking) { setAsking(true); setTimeout(() => setAsking(false), 5000); return; }
        setAsking(false);
        props.onClick();
      }}
      {...({ onGamepadFocus: onFocus, autoFocus: props.autoFocus } as any)}>
      {asking ? props.confirm : props.label}
    </DialogButton>
  );
}

export function Row(props: { children: ReactNode }) {
  return <Focusable style={{ display: "flex", gap: 10, marginTop: 10, flexWrap: "wrap" }} {...H}>{props.children}</Focusable>;
}

function Bar(props: { p: number | null | undefined }) {
  return <div className="ht-bar"><i style={{ width: `${percent(props.p)}%` }} /></div>;
}

/** The core isn't there (or said no): one clear card. */
function Problem(props: { error: string; app?: string }) {
  return (
    <div className="ht-card">
      {props.error}
      {props.app && <Row><Act label="Open Hearth" onClick={() => openApp(props.app!)} /></Row>}
    </div>
  );
}

/** Runs a change and says how it went (Steam's toast); `after` refreshes the page. */
async function run(what: Promise<any>, ok: string, after?: () => void) {
  const r = await what;
  if (r && r.error) say(r.error); else if (ok) say(ok);
  after?.();
  return r;
}

// ---------------------------------------------------------------- Mods

interface ModGame { id: string; title: string; mods?: number; modsOn?: number; beacon?: boolean }
interface Mod { id: string; name: string; enabled?: boolean; kind?: string; targetLabel?: string; size?: number }
interface ModsSummary { title?: string; mods?: Mod[]; beacon?: boolean }
interface ModDownload { id: string; name: string; state: string; game?: string; size?: number; done?: number }

/** Hearth Mods: a game's mods to turn on and off or uninstall, and finished downloads to install. Finding new mods
 *  (Nexus Mods, signed in there) stays in Hearth Mods' own window. */
export function ModsPage() {
  const [games] = usePoll<{ value: ModGame[] }>(() => modsCall("list"), 0, []);
  const list = games && !games.error && Array.isArray(games.value) ? games.value : [];
  const [pick, setPick] = useState("");
  const cur = pick || (list.find((g) => (g.mods || 0) > 0) || list[0])?.id || "";
  return (
    <Boundary part="mods" onError={guard.fail}>
      <Page title="Hearth Mods" wallpaper={brand()?.wallpaper}>
        <div className="ht-dim">Turn a game's mods on and off, uninstall them, and install mods you downloaded.</div>
        {!games && <div className="ht-dim" style={{ marginTop: 14 }}>Asking Hearth Mods…</div>}
        {games && games.error ? <div style={{ marginTop: 14 }}><Problem error={games.error} /></div> : null}
        {list.length > 0 && <Chips items={list.map((g) => [g.id, `${g.title}${g.mods ? ` ${g.modsOn ?? 0}/${g.mods}` : ""}`] as [string, string])} value={cur} onChange={setPick} />}
        {games && !games.error && !list.length && <div className="ht-card" style={{ marginTop: 14 }}>No games Hearth Mods can mod are installed.</div>}
        {cur && <GameMods key={cur} id={cur} />}
        <Row><Act label="Find mods: open Hearth Mods" onClick={() => openApp("hearth-app:mods")} /></Row>
      </Page>
    </Boundary>
  );
}

function GameMods(props: { id: string }) {
  const [sum, again] = usePoll<ModsSummary>(() => modsCall("mods_of", { id: props.id }), 0, [props.id]);
  const [dls, againDl] = usePoll<{ items?: ModDownload[] }>(() => modsCall("downloads_list"), 5000, []);
  const [busy, setBusy] = useState("");
  if (!sum) return <div className="ht-dim" style={{ marginTop: 14 }}>Asking Hearth Mods…</div>;
  if (sum.error) return <div style={{ marginTop: 14 }}><Problem error={sum.error} /></div>;
  const mods = (sum.mods || []).filter((m) => m.kind !== "separator");
  const ready = (dls && !dls.error && dls.items ? dls.items : []).filter((d) => d.state === "done" && (!d.game || d.game === props.id));
  const change = async (key: string, p: Promise<any>, ok: string) => { setBusy(key); await run(p, ok, again); setBusy(""); };
  return (
    <Focusable {...V}>
      <h2>Installed mods ({mods.filter((m) => m.enabled).length} of {mods.length} on)</h2>
      {sum.beacon && <div className="ht-card">Beacon manages this game's mods. Open Beacon to change them.</div>}
      {!mods.length && !sum.beacon && <div className="ht-card">No mods for this game yet.</div>}
      {mods.map((m) => (
        <div key={m.id} className="ht-card">
          <div style={{ display: "flex", justifyContent: "space-between" }}><b>{m.name}</b><span className="ht-dim">{m.enabled ? "On" : "Off"}{m.size ? ` · ${bytes(m.size)}` : ""}</span></div>
          {m.targetLabel && <div className="ht-dim">{m.targetLabel}</div>}
          <Row>
            <Act primary={!m.enabled} disabled={busy === m.id || sum.beacon} label={m.enabled ? "Turn off" : "Turn on"}
              onClick={() => change(m.id, modsCall("toggle", { id: props.id, m: m.id, on: !m.enabled }), "")} />
            <Act disabled={busy === m.id || sum.beacon} label="Uninstall" confirm="Press again to uninstall"
              onClick={() => change(m.id, modsCall("remove_mod", { id: props.id, m: m.id }), `${m.name} uninstalled`)} />
          </Row>
        </div>
      ))}
      {ready.length > 0 && <h2>Downloaded, ready to install</h2>}
      {ready.map((d) => (
        <div key={d.id} className="ht-card">
          <div style={{ display: "flex", justifyContent: "space-between" }}><b>{d.name}</b><span className="ht-dim">{d.size ? bytes(d.size) : ""}</span></div>
          <Row>
            <Act primary disabled={busy === d.id} label={busy === d.id ? "Installing…" : "Install"} onClick={async () => {
              setBusy(d.id);
              const r = await modsCall<any>("install_download", { id: d.id, gameId: props.id });
              setBusy("");
              if (r.error) say(r.error);
              else if (r.fomod) say(`${d.name} has installer choices: finish it in Hearth Mods`);
              else say(`${d.name} installed and turned on`);
              again(); againDl();
            }} />
          </Row>
        </div>
      ))}
    </Focusable>
  );
}

// ---------------------------------------------------------------- Storage

interface DsDrive { id: string; name: string; kind: string; mount: string; fs?: string; size: number; used: number; free: number; readOnly: boolean; games?: boolean; steamLibs?: string[] }
interface DsGame { id: string; title: string; source: string; store: string; drive?: string | null; size?: number | null; canMove: boolean; moveWhy?: string | null;
                   uninstall?: string | null; uninstallWhy?: string | null; running?: boolean }
interface DsJob { state?: string; phase?: string; done?: number; total?: number; error?: string; title?: string }
interface DsStatus { drives: DsDrive[]; games: DsGame[]; job?: DsJob | null; scan?: { state?: string } }
interface CleanItem { key: string; kind: string; title: string; where?: string; bytes: number; selectable: boolean; on: boolean; confirm?: boolean; why?: string | null; saves?: string[] }

/** Settings › Storage inside Steam: drives, games by size (move, uninstall), clean up, and a drive for new games. */
export function StorageManagePage() {
  const [job, setJob] = useState(false);
  const [st, again] = usePoll<DsStatus>(() => hearthCall("diskspace.status", { scan: true }), job ? 2000 : 20000, []);
  const moving = !!st && !st.error && st.job?.state === "moving";
  useEffect(() => { if (moving !== job) setJob(moving); }, [moving]);
  const drives = st && !st.error ? st.drives || [] : [];
  const games = st && !st.error ? [...(st.games || [])].sort((a, b) => (b.size || 0) - (a.size || 0)) : [];
  const nameOf = (mount?: string | null) => drives.find((d) => d.mount === mount)?.name || "";
  return (
    <Boundary part="storage" onError={guard.fail}>
      <Page title="Storage" wallpaper={brand()?.wallpaper}>
        {!st && <div className="ht-dim">Asking Hearth…</div>}
        {st && st.error ? <Problem error={st.error} app="hearth-app:settings" /> : null}
        {st && !st.error && st.job && st.job.state ? <MoveJob job={st.job} after={again} /> : null}
        {drives.length > 0 && <h2>Drives</h2>}
        <Focusable {...V}>
          {drives.map((d) => (
            <div key={d.mount} className="ht-card">
              <div style={{ display: "flex", justifyContent: "space-between" }}><b>{d.name}</b><span className="ht-dim">{bytes(d.free)} free of {bytes(d.size)}</span></div>
              <div className="ht-dim">{d.mount}{d.fs ? ` · ${d.fs}` : ""}{d.steamLibs?.length ? " · Steam library" : ""}{d.readOnly ? " · read only" : ""}</div>
              <div className="ht-bar"><i style={{ width: `${d.size ? Math.round((d.used / d.size) * 100) : 0}%` }} /></div>
              {!d.readOnly && (
                <Row>
                  <Act label="Use this disk for games" onClick={async () => {
                    const r = await gamesDisk(d.mount);
                    say(r.error || `New Epic and GOG games go to ${r.installRoot}. For Steam games, choose it in Steam's Storage settings.`);
                  }} />
                </Row>
              )}
            </div>
          ))}
        </Focusable>
        {st && !st.error && <h2>Games by size</h2>}
        <Focusable {...V}>
          {games.map((g) => {
            const steam = g.source === "steam";
            const targets = drives.filter((d) => !d.readOnly && d.mount !== g.drive);
            return (
              <div key={g.id} className="ht-card">
                <div style={{ display: "flex", justifyContent: "space-between" }}><b>{g.title}</b><span className="ht-dim">{g.size ? bytes(g.size) : "size not known yet"}</span></div>
                <div className="ht-dim">{[g.store, nameOf(g.drive), g.running ? "running" : ""].filter(Boolean).join(" · ")}{!steam && !g.canMove && g.moveWhy ? ` · ${g.moveWhy}` : ""}</div>
                <Row>
                  {steam && <Act label="Move in Steam's Storage" onClick={() => go("/settings/storage")} />}
                  {!steam && g.canMove && targets.map((d) => (
                    <Act key={d.mount} disabled={moving} label={`Move to ${d.name}`} confirm={`Press again to move to ${d.name}`}
                      onClick={() => run(hearthCall("diskspace.move", { id: g.id, to: d.mount }), `Moving ${g.title} to ${d.name}`, again)} />
                  ))}
                  {steam && <Act label="Uninstall in Steam" onClick={() => steamPage(g.id.slice(6))} />}
                  {!steam && g.uninstall && (
                    <Act label="Uninstall" confirm={`Press again to uninstall ${g.title}`} disabled={g.running || moving}
                      onClick={() => run(g.uninstall === "recomp" ? hearthCall("recomps.uninstall", { id: g.id.replace(/^recomps?:/, ""), keep: true })
                                                                  : hearthCall("game.uninstall", { id: g.id }), `${g.title} uninstalled`, again)} />
                  )}
                  {!steam && !g.uninstall && g.uninstallWhy && <span className="ht-dim" style={{ alignSelf: "center" }}>{g.uninstallWhy}</span>}
                </Row>
              </div>
            );
          })}
        </Focusable>
        {st && !st.error && <CleanUp after={again} />}
        <Row><Act label="Steam's Storage settings" onClick={() => go("/settings/storage")} /></Row>
      </Page>
    </Boundary>
  );
}

function MoveJob(props: { job: DsJob; after: () => void }) {
  const j = props.job;
  const p = j.total ? (j.done || 0) / j.total : null;
  const line = j.state === "moving" ? `Moving${j.title ? ` ${j.title}` : ""}: ${j.phase || ""}` : j.state === "done" ? "Moved" : j.state === "stopped" ? "Move stopped: the game stayed where it was" : j.error || j.state || "";
  return (
    <div className="ht-card">
      <b>{line}</b>
      {j.state === "moving" && <Bar p={p} />}
      {j.state === "moving" && <Row><Act label="Stop" onClick={() => run(hearthCall("diskspace.cancel"), "Stopping the move", props.after)} /></Row>}
    </div>
  );
}

function CleanUp(props: { after: () => void }) {
  const [items, setItems] = useState<CleanItem[] | null>(null);
  const [err, setErr] = useState("");
  const [on, setOn] = useState<Record<string, boolean>>({});
  const [busy, setBusy] = useState(false);
  const find = async () => {
    setBusy(true);
    const r = await hearthCall<{ items: CleanItem[] }>("diskspace.cleanList");
    setBusy(false);
    if (r.error) { setErr(r.error); return; }
    setErr("");
    setItems(r.items || []);
    setOn(Object.fromEntries((r.items || []).map((i) => [i.key, !!i.on && i.selectable])));
  };
  const chosen = (items || []).filter((i) => on[i.key]);
  const total = chosen.reduce((n, i) => n + (i.bytes || 0), 0);
  return (
    <>
      <h2>Clean up</h2>
      {err && <div className="ht-card">{err}</div>}
      {!items && <Row><Act label={busy ? "Looking…" : "Find things to clean up"} disabled={busy} onClick={find} /></Row>}
      {items && !items.length && <div className="ht-card">Nothing to clean up.</div>}
      <Focusable {...V}>
        {(items || []).map((i) => (
          <div key={i.key} className="ht-card">
            <div style={{ display: "flex", justifyContent: "space-between" }}><b>{i.title}</b><span className="ht-dim">{bytes(i.bytes)}</span></div>
            <div className="ht-dim">{[i.kind, i.where, i.saves?.length ? `saves: ${i.saves.join(", ")}` : "", i.why || ""].filter(Boolean).join(" · ")}</div>
            {i.selectable && <Row><Act primary={!!on[i.key]} label={on[i.key] ? "Will be deleted" : "Keep"} onClick={() => setOn({ ...on, [i.key]: !on[i.key] })} /></Row>}
          </div>
        ))}
      </Focusable>
      {items && items.length > 0 && (
        <Row>
          <Act primary disabled={!chosen.length || busy} label={`Delete ${chosen.length} item${chosen.length === 1 ? "" : "s"} (${bytes(total)})`}
            confirm={`Press again to delete ${bytes(total)}`} onClick={async () => {
              setBusy(true);
              const keys = chosen.map((i) => i.key);
              const r = await hearthCall<{ freed: number; failed: { title: string; why: string }[] }>("diskspace.clean", { keys, confirmed: chosen.filter((i) => i.confirm).map((i) => i.key) });
              setBusy(false);
              say(r.error || `Freed ${bytes(r.freed || 0)}${r.failed?.length ? `; kept ${r.failed.length} (${r.failed[0].why})` : ""}`);
              setItems(null);
              props.after();
            }} />
        </Row>
      )}
    </>
  );
}

// ---------------------------------------------------------------- Recomps

interface Recomp { id: string; title: string; console?: string; needs?: string; installed?: boolean; version?: string; copy?: string | null; hidden?: boolean;
                   job?: { busy?: boolean; stage?: string; progress?: number | null; error?: string | null; done?: boolean } | null }
interface RecompList { recomps?: Recomp[]; consoles?: Record<string, string>; limitedUntil?: number | null }

/** Hearth Recomps: native PC ports of console games; install (from the project's own releases), progress, play. Your
 *  own copy of the game, when one is needed, is chosen in Hearth (no file picker inside Steam). */
export function RecompsPage() {
  const [busy, setBusy] = useState(false);
  const [l, again] = usePoll<RecompList>(() => hearthCall("recomps.list"), busy ? 1500 : 30000, []);
  const list = l && !l.error ? (l.recomps || []).filter((r) => !r.hidden) : [];
  const anyBusy = list.some((r) => r.job?.busy);
  useEffect(() => { if (anyBusy !== busy) setBusy(anyBusy); }, [anyBusy]);
  const installed = list.filter((r) => r.installed || r.job?.busy);
  const rest = list.filter((r) => !r.installed && !r.job?.busy);
  const consoles: Record<string, string> = (l && !l.error && l.consoles) || {};
  const card = (r: Recomp) => <RecompCard key={r.id} r={r} console={consoles[r.console || ""] || r.console || ""} after={again} />;
  return (
    <Boundary part="recomps" onError={guard.fail}>
      <Page title="Hearth Recomps" wallpaper={brand()?.wallpaper}>
        <div className="ht-dim">Console games rebuilt as native PC games. Some need your own copy of the game.</div>
        {!l && <div className="ht-dim" style={{ marginTop: 14 }}>Asking Hearth…</div>}
        {l && l.error ? <div style={{ marginTop: 14 }}><Problem error={l.error} app="hearth-app:hearth" /></div> : null}
        {l?.limitedUntil ? <div className="ht-dim" style={{ marginTop: 8 }}>GitHub's hourly limit was reached: installs wait until it resets.</div> : null}
        {installed.length > 0 && <h2>Installed</h2>}
        <Focusable {...V}>{installed.map(card)}</Focusable>
        {rest.length > 0 && <h2>Available</h2>}
        <Focusable {...V}>{rest.map(card)}</Focusable>
      </Page>
    </Boundary>
  );
}

function RecompCard(props: { r: Recomp; console: string; after: () => void }) {
  const r = props.r, j = r.job;
  const needsCopy = !!r.installed && !!r.needs && !r.copy;
  const game = { id: `recomp:${r.id}`, title: r.title, source: "recomps", store: "Recomps", installed: true } as any;
  return (
    <div className="ht-card">
      <div style={{ display: "flex", justifyContent: "space-between" }}><b>{r.title}</b><span className="ht-dim">{[props.console, r.installed && r.version ? r.version : ""].filter(Boolean).join(" · ")}</span></div>
      {r.needs && <div className="ht-dim">Needs: {r.needs}</div>}
      {j?.busy && <><div className="ht-dim">{j.stage || "Installing…"}</div><Bar p={j.progress ?? null} /></>}
      {j?.error && !j.busy && <div className="ht-dim" style={{ color: C.text }}>Couldn't install: {j.error}</div>}
      {needsCopy && <div className="ht-dim">Choose your copy of the game in Hearth before playing (no file picker inside Steam).</div>}
      <Row>
        {!r.installed && !j?.busy && <Act primary label="Install" onClick={() => run(hearthCall("recomps.install", { id: r.id }), `Installing ${r.title}`, props.after)} />}
        {j?.busy && <Act label="Cancel" onClick={() => run(hearthCall("recomps.cancel", { id: r.id }), "", props.after)} />}
        {r.installed && !j?.busy && !needsCopy && <Act primary label="Play" onClick={() => playGame(game)} />}
        {needsCopy && <Act primary label="Choose the game in Hearth" onClick={() => openApp("hearth-app:hearth")} />}
        {r.installed && !j?.busy && <Act label="Uninstall" confirm="Press again (saves are backed up)" onClick={() => run(hearthCall("recomps.uninstall", { id: r.id, keep: true }), `${r.title} uninstalled`, props.after)} />}
      </Row>
    </div>
  );
}
