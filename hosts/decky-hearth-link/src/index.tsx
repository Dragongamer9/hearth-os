// Hearth Link (docs/HEARTH-LINK-PLUGIN.md): your Hearth devices, game copies between them, Hearth Storage and pairing,
// in Steam Game Mode's Quick Access. Everything goes to Hearth Link on this device (main.py, an allow-list of its
// commands). Nothing of Steam's screens is changed; the only Steam call is its own restart, offered after a Steam game
// was copied here, and only while Steam has it.
import { ButtonItem, DropdownItem, Field, PanelSection, PanelSectionRow, ProgressBarWithInfo, TextField, staticClasses } from "@decky/ui";
import { definePlugin } from "@decky/api";
import { ReactNode, useEffect, useRef, useState } from "react";
import { FaNetworkWired } from "react-icons/fa";
import {
  Answer, Down, Game, Job, POLL, STORAGE_POLL, Status, afterCopy, active, ago, bytes, canWake, deviceLine, healthSummary, hasStorage,
  jobActions, jobLine, lanCodeOk, left, netCodeOk, percent, sendable, serverOf, showCode,
} from "./model";
import { StorageInfo, games, getStatus, getStorage, linkCall, restartSteam, startLink, steamCanRestart } from "./link";

const warn = { color: "#ffb37a" };
const dim = { fontSize: 12, color: "#8d99a8" };
const Line = (p: { children: ReactNode; style?: any }) => <PanelSectionRow><div className={staticClasses.Text} style={p.style}>{p.children}</div></PanelSectionRow>;

function Copy(p: { j: Job; act: (cmd: string, args: any, done?: string) => void }) {
  const { j } = p;
  const after = j.state === "done" ? afterCopy(j) : null;
  return (
    <>
      <PanelSectionRow>
        <ProgressBarWithInfo label={j.title} nProgress={percent(j)} sOperationText={jobLine(j)} indeterminate={j.state === "preparing" || j.state === "comparing"}
          description={j.from ? `From ${j.from}` : undefined} />
      </PanelSectionRow>
      {after && <Line style={dim}>{after.text}</Line>}
      {after?.steam && (steamCanRestart()
        ? <PanelSectionRow><ButtonItem layout="below" onClick={() => restartSteam()}>Restart Steam to see it</ButtonItem></PanelSectionRow>
        : <Line style={dim}>Restart Steam from its Power menu to see it.</Line>)}
      {jobActions(j).map((a) => (
        <PanelSectionRow key={a}>
          <ButtonItem layout="below" onClick={() => p.act(a === "clear" ? "transfer.clear" : `transfer.${a}`, a === "clear" ? {} : { id: j.id })}>
            {{ pause: "Pause", resume: "Resume", cancel: "Cancel", clear: "Clear finished" }[a]}
          </ButtonItem>
        </PanelSectionRow>
      ))}
    </>
  );
}

function NotRunning(p: { s: Down; again: () => void; now: number }) {
  const [msg, setMsg] = useState("");
  const last = p.s.last?.status;
  return (
    <>
      <PanelSection title="Hearth Link">
        <Line style={warn}>{p.s.error}{p.s.installed ? "" : ". It isn't part of this Hearth OS."}</Line>
        <Line style={dim}>Nothing can be sent, fetched or paired until it runs. What's below is what this device last saw.</Line>
        {p.s.installed && (
          <PanelSectionRow>
            <ButtonItem layout="below" onClick={async () => { const r = await startLink(); setMsg(r.ok ? "Starting Hearth Link…" : r.error || ""); setTimeout(p.again, 2500); }}>
              Start Hearth Link
            </ButtonItem>
          </PanelSectionRow>
        )}
        {msg && <Line style={dim}>{msg}</Line>}
      </PanelSection>
      {last?.peers && (
        <PanelSection title="Devices (last seen)">
          <Line style={dim}>As this device saw them {ago(p.s.last!.at, p.now)}</Line>
          {last.me && <Field label={`${last.me.name} (this device)`} description="Hearth Link is off here" />}
          {last.peers.map((d) => <Field key={d.id} label={d.name} description={`Last seen ${ago(d.lastSeen, p.now)}`} />)}
          {!last.peers.length && <Line style={dim}>No other devices were paired.</Line>}
        </PanelSection>
      )}
    </>
  );
}

function Pairing(p: { s: Status; act: (cmd: string, args: any, done?: string) => Promise<boolean>; now: number }) {
  const { s } = p;
  const [target, setTarget] = useState<string>("");
  const [code, setCode] = useState("");
  const pr = s.pairing;
  const pairing = s.nearby.filter((n) => n.pairing);
  const join = async () => {
    const c = code.replace(/\s/g, "");
    if (await p.act("pair.join", target === "net" ? { code: code.trim() } : { id: target, code: c }, "Paired")) { setCode(""); setTarget(""); }
  };
  return (
    <PanelSection title="Pair a device">
      {s.pairEvent && <Line style={s.pairEvent.ok ? undefined : warn}>{s.pairEvent.ok ? `Paired with ${s.pairEvent.name}` : s.pairEvent.error || "Didn't pair"}</Line>}
      {s.joining && <Line style={dim}>{({ looking: "Looking up the code…", found: `Found ${s.joining.name}…`, connecting: `Connecting to ${s.joining.name}…` } as Record<string, string>)[s.joining.state] || `Pairing with ${s.joining.name}…`}</Line>}
      {pr ? (
        <>
          <Field label="This device's code" description={`Type it on the other Hearth device · ${left(pr.until, p.now)} left`}>
            <div style={{ fontSize: 30, fontWeight: 700, letterSpacing: "0.12em" }}>{showCode(pr.code)}</div>
          </Field>
          {pr.netCode && <Field label="Internet code" description={`For a device somewhere else · ${left(pr.netUntil, p.now)} left`}><div style={{ fontSize: 20, fontWeight: 700 }}>{pr.netCode}</div></Field>}
          {!pr.netCode && s.shortCodes && <PanelSectionRow><ButtonItem layout="below" onClick={() => p.act("pair.open", { internet: true })}>Also allow a device somewhere else</ButtonItem></PanelSectionRow>}
          <PanelSectionRow><ButtonItem layout="below" onClick={() => p.act("pair.close", {})}>Stop showing the code</ButtonItem></PanelSectionRow>
        </>
      ) : (
        <PanelSectionRow><ButtonItem layout="below" description="Shows a 6-digit code to type on the new device" onClick={() => p.act("pair.open", {})}>Show this device's code</ButtonItem></PanelSectionRow>
      )}
      {pairing.map((n) => (
        <PanelSectionRow key={n.id}>
          <ButtonItem layout="below" description="It's showing a code: pick it, then type that code" onClick={() => { setTarget(n.id); setCode(""); }}>
            {target === n.id ? `Type ${n.name}'s code below` : `Pair with ${n.name}`}
          </ButtonItem>
        </PanelSectionRow>
      ))}
      {s.shortCodes && target !== "net" && <PanelSectionRow><ButtonItem layout="below" onClick={() => { setTarget("net"); setCode(""); }}>Enter an internet code</ButtonItem></PanelSectionRow>}
      {target && (
        <>
          <PanelSectionRow>
            <TextField label={target === "net" ? "Internet code" : "6-digit code"} value={code} onChange={(e: any) => setCode(e.target.value)} />
          </PanelSectionRow>
          <PanelSectionRow>
            <ButtonItem layout="below" disabled={target === "net" ? !netCodeOk(code) : !lanCodeOk(code)} onClick={join}>Pair</ButtonItem>
          </PanelSectionRow>
        </>
      )}
      {!pr && !pairing.length && !target && <Line style={dim}>A device to pair with shows up here once it shows its code (Settings › Hearth Link on it).</Line>}
    </PanelSection>
  );
}

function Storage(p: { s: Status; info: StorageInfo | null; act: (cmd: string, args: any, done?: string) => Promise<boolean>; now: number }) {
  const { s, info } = p;
  const h = info?.health && !info.health.error ? healthSummary(info.health, p.now) : null;
  return (
    <PanelSection title="Hearth Storage">
      {s.storage.server && <Line>This device is Hearth Storage</Line>}
      {s.storage.servers.map((v) => {
        const peer = s.peers.find((x) => x.id === v.id);
        const on = !!peer?.online;
        return (
          <Field key={v.id} label={v.name} description={`${on ? "Online" : "Asleep or away"}${v.free ? ` · ${bytes(v.free)} free` : ""}${v.seen ? ` · checked ${ago(v.seen, p.now)}` : ""}`}>
            {!on && v.canWake && <ButtonItem onClick={() => p.act("storage.wake", { server: v.id }, `Waking ${v.name}`)}>Wake</ButtonItem>}
          </Field>
        );
      })}
      <PanelSectionRow>
        <ButtonItem layout="below" description={`Your Hearth data on every device · last change ${ago(s.lastChange, p.now)}`} onClick={() => p.act("sync.now", {}, "Synced")}>Sync now</ButtonItem>
      </PanelSectionRow>
      {!info && <Line style={dim}>Asking Hearth Storage…</Line>}
      {info && (
        <>
          <Field label="Download once" description={info.cache.error || (info.cache.enabled
            ? `${info.cache.available.length} game${info.cache.available.length === 1 ? "" : "s"} to get from ${info.cache.using || "Hearth Storage"}${info.cache.updates.length ? ` · ${info.cache.updates.length} update${info.cache.updates.length === 1 ? "" : "s"}` : ""}`
            : "Off (Settings › Hearth Link › Hearth Storage)")} />
          {[...s.storage.jobs, ...info.cache.jobs.filter((j) => !s.storage.jobs.some((x) => x.id === j.id))].filter(active).map((j) => (
            <PanelSectionRow key={j.id}><ProgressBarWithInfo label={j.title} nProgress={percent(j)} sOperationText={jobLine(j)} /></PanelSectionRow>
          ))}
          {info.cache.available.slice(0, 5).map((g) => (
            <PanelSectionRow key={g.game}>
              <ButtonItem layout="below" description={g.size ? bytes(g.size) : undefined} onClick={() => p.act("storage.cache.get", { game: g.game }, `Getting ${g.title} from Hearth Storage`)}>Get {g.title}</ButtonItem>
            </PanelSectionRow>
          ))}
          <Field label="Saves that follow you" description={info.saves.error || (info.saves.follow ? (info.saves.games.length ? `${info.saves.games.length} game${info.saves.games.length === 1 ? "" : "s"}` : "No saves synced yet") : "Off (Settings › Hearth Link › Hearth Storage)")} />
          {info.saves.follow && info.saves.games.slice(0, 6).map((g, i) => (
            <Line key={i} style={g.conflict ? { ...dim, ...warn } : dim}>{g.title} · {g.conflict ? "two different saves: pick one in Hearth" : g.pending ? "waiting to sync" : `synced ${ago(g.at, p.now)}`}</Line>
          ))}
          <Field label="Update cache" description={info.updates.error || (info.updates.enabled
            ? `${info.updates.fetching ? "Fetching an update · " : ""}${info.updates.cached.length ? `Hearth OS ${info.updates.cached[0].version || "update"} kept` : "Nothing kept yet"}${info.updates.using ? ` · from ${info.updates.using}` : ""}`
            : "Off")} />
          <Field label="Device health" description={info.health.error || h?.line || ""} />
          {h?.warnings.slice(0, 5).map((w, i) => <Line key={i} style={{ ...dim, ...warn }}>{w}</Line>)}
          {h && !h.warnings.length && <Line style={dim}>All good</Line>}
        </>
      )}
    </PanelSection>
  );
}

function Panel() {
  const [s, setS] = useState<Answer | null>(null);
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [now, setNow] = useState(Date.now());
  const [msg, setMsg] = useState<{ t: string; bad?: boolean } | null>(null);
  const [mine, setMine] = useState<Game[] | null>(null);
  const [sendGame, setSendGame] = useState("");
  const [sendTo, setSendTo] = useState("");
  const [peer, setPeer] = useState("");
  const [theirs, setTheirs] = useState<Game[] | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  const again = () => getStatus().then((v) => alive.current && setS(v)).catch(() => {});
  const storageAgain = () => getStorage().then((v) => alive.current && setInfo(v?.running ? v : null)).catch(() => {});
  useEffect(() => {
    again();
    const t = setInterval(() => { again(); setNow(Date.now()); }, POLL);
    return () => { alive.current = false; clearInterval(t); };
  }, []);
  const up = s?.running === true;
  const storageOn = up && hasStorage(s as Status);
  useEffect(() => {
    if (!storageOn) return;
    storageAgain();
    const t = setInterval(storageAgain, STORAGE_POLL);
    return () => clearInterval(t);
  }, [storageOn]);
  useEffect(() => { if (up && mine === null) games().then(setMine).catch(() => setMine([])); }, [up]);

  const act = async (cmd: string, args: any, done?: string): Promise<boolean> => {
    setBusy(true); setMsg(null);
    const r = await linkCall(cmd, args).catch((e) => ({ ok: false, error: String(e) }));
    setBusy(false);
    if (r.ok) { if (done) setMsg({ t: done }); } else setMsg({ t: r.error || "That didn't work", bad: true });
    again();
    if (cmd.startsWith("storage.")) storageAgain();
    return r.ok;
  };
  const browse = async (id: string) => {
    if (peer === id) { setPeer(""); setTheirs(null); return; }
    setPeer(id); setTheirs(null);
    try { setTheirs(await games(id)); } catch (e: any) { setTheirs([]); setMsg({ t: e?.message || "Couldn't ask that device", bad: true }); }
  };

  if (!s) return <PanelSection><Line>Asking Hearth Link…</Line></PanelSection>;
  if (!s.running) return <NotRunning s={s} again={again} now={now} />;
  const servers = s.storage.servers;
  const online = s.peers.filter((d) => d.online);
  const list = sendable(mine || []);
  const copies = s.transfers;
  return (
    <>
      {msg && <PanelSection><Line style={msg.bad ? warn : undefined}>{msg.t}</Line></PanelSection>}
      {s.playing && <PanelSection><Line style={dim}>A game is running: Hearth Link keeps quiet until it closes.</Line></PanelSection>}
      <PanelSection title="Devices">
        <Field label={`${s.me.name} (this device)`} description={`Hearth Link is on${s.storage.server ? " · Hearth Storage" : ""}`} />
        {s.peers.map((d) => (
          <div key={d.id}>
            <Field label={`${d.name}${serverOf(d, servers) ? " · Hearth Storage" : ""}`} description={deviceLine(d, now)}>
              {canWake(d, servers) && <ButtonItem onClick={() => act("storage.wake", { server: d.id }, `Waking ${d.name}`)}>Wake</ButtonItem>}
            </Field>
            {d.online && (
              <PanelSectionRow>
                <ButtonItem layout="below" onClick={() => browse(d.id)}>{peer === d.id ? `Hide ${d.name}'s games` : `Get a game from ${d.name}`}</ButtonItem>
              </PanelSectionRow>
            )}
            {peer === d.id && (
              theirs === null ? <Line style={dim}>Asking {d.name}…</Line>
                : !theirs.length ? <Line style={dim}>No games to get from {d.name}.</Line>
                  : theirs.slice(0, 12).map((g) => (
                    <PanelSectionRow key={g.id}>
                      <ButtonItem layout="below" disabled={busy} description={`${g.size ? bytes(g.size) + " · " : ""}${g.store}`}
                        onClick={() => act("get", { peer: d.id, game: g.id, title: g.title }, `Getting ${g.title} from ${d.name}`)}>Get {g.title}</ButtonItem>
                    </PanelSectionRow>
                  ))
            )}
          </div>
        ))}
        {!s.peers.length && <Line style={dim}>No other Hearth devices yet: pair one below.</Line>}
      </PanelSection>

      <PanelSection title="Send a game">
        {!online.length ? <Line style={dim}>None of your other devices is online.</Line> : (
          <>
            <PanelSectionRow>
              <DropdownItem label="Game on this device" strDefaultLabel={mine === null ? "Reading…" : list.length ? "Pick a game" : "No games installed"}
                rgOptions={list.map((g) => ({ data: g.id, label: `${g.title}${g.size ? ` · ${bytes(g.size)}` : ""}` }))}
                selectedOption={sendGame || null} onChange={(o: any) => setSendGame(o.data)} />
            </PanelSectionRow>
            <PanelSectionRow>
              <DropdownItem label="To" strDefaultLabel="Pick a device" rgOptions={online.map((d) => ({ data: d.id, label: d.name }))}
                selectedOption={sendTo || null} onChange={(o: any) => setSendTo(o.data)} />
            </PanelSectionRow>
            <PanelSectionRow>
              <ButtonItem layout="below" disabled={busy || !sendGame || !online.some((d) => d.id === sendTo)}
                description="The other device copies it from this one, straight over your network (or the internet)"
                onClick={() => act("send", { peer: sendTo, game: sendGame }, `Sent: ${online.find((d) => d.id === sendTo)?.name} copies it once it says yes`)}>Send</ButtonItem>
            </PanelSectionRow>
          </>
        )}
        {s.sending.map((x, i) => {
          const size = (mine || []).find((g) => g.title === x.title)?.size || 0;
          return (
            <PanelSectionRow key={i}>
              <ProgressBarWithInfo label={`Sending ${x.title} to ${x.to}`} nProgress={size ? Math.min(100, Math.round((x.bytes * 100) / size)) : 0} indeterminate={!size}
                sOperationText={`${bytes(x.bytes)}${size ? ` of ${bytes(size)}` : ""} sent`} description={`${x.to} can pause or cancel it`} />
            </PanelSectionRow>
          );
        })}
      </PanelSection>

      {(copies.length > 0 || s.offers.length > 0) && (
        <PanelSection title="Copies">
          {s.offers.map((o) => (
            <div key={o.id}>
              <Field label={`${o.from} wants to send ${o.title}`} description={o.size ? bytes(o.size) : undefined} />
              <PanelSectionRow><ButtonItem layout="below" onClick={() => act("offer.accept", { id: o.id }, `Getting ${o.title}`)}>Get it</ButtonItem></PanelSectionRow>
              <PanelSectionRow><ButtonItem layout="below" onClick={() => act("offer.decline", { id: o.id })}>No thanks</ButtonItem></PanelSectionRow>
            </div>
          ))}
          {copies.map((j) => <Copy key={j.id} j={j} act={act} />)}
        </PanelSection>
      )}

      {storageOn && <Storage s={s} info={info} act={act} now={now} />}
      <Pairing s={s} act={act} now={now} />
    </>
  );
}

export default definePlugin(() => ({
  name: "Hearth Link",
  titleView: <div className={staticClasses.Title}>Hearth Link</div>,
  content: <Panel />,
  icon: <FaNetworkWired />,
  onDismount() {},
}));
