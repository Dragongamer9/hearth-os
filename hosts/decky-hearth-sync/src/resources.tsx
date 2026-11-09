// Quick Access › Resources (docs/RESOURCE-MONITOR.md, the approved GameOverlay mockup): graphics, processor, memory and
// network meters with temperatures, what else is running, and "Pause downloads and backups while I play". The numbers
// come from Resource Monitor's helper on this device (main.py resources_sample), asked once a second only while this
// section is on screen. The pause uses existing pauses only and changes no setting: Steam's own download pause (this
// session) and the home server's "big jobs wait while you play" runtime flag; Light mode (a setting) only turns on when
// the person presses that option.
import { ButtonItem, PanelSection, PanelSectionRow, staticClasses } from "@decky/ui";
import { callable } from "@decky/api";
import { useEffect, useState } from "react";

export interface Brief {
  gpu: { busy: number | null; temp: number | null; name: string; vramUsed: number | null; vramTotal: number | null; watts: number | null } | null;
  cpu: { busy: number | null; temp: number | null };
  mem: { used: number | null; total: number | null; cache: number | null };
  net: { recv: number; send: number; kind: string };
  game: { name: string; cpu: number | null; gpu: number | null; mem: number | null; tag: string } | null;
  others: { name: string; tag: string; color: string; cpu: number | null; mem: number | null; disk: number | null }[];
}
export interface PauseState {
  backups: { available: boolean; count: number; lightMode: boolean | null; resumed: boolean; paused: string[]; waiting: boolean };
  needsOption: boolean;
}

const sampleRes = callable<[], Brief | null>("resources_sample");
const pauseRes = callable<[action: string], PauseState | null>("resources_pause");

const COLORS: Record<string, string> = { game: "#f2c14e", steam: "#66c0f4", hearth: "#ff9a4d", app: "#9a7cff", sys: "#5d7488", home: "#3fb8c9" };
export const gb = (b: number | null | undefined) => (b === null || b === undefined ? "–" : b >= 1073741824 ? `${(b / 1073741824).toFixed(1)} GB` : `${Math.round(b / 1048576)} MB`);
export const rate = (b: number) => (b >= 1048576 ? `${(b / 1048576).toFixed(b >= 104857600 ? 0 : 1)} MB/s` : b >= 1024 ? `${Math.round(b / 1024)} KB/s` : "0 KB/s");
const pct = (n: number | null | undefined) => (n === null || n === undefined ? "–" : `${Math.round(n)}%`);
const temp = (t: number | null | undefined) => (t === null || t === undefined ? "" : `${Math.round(t)} °C`);

/** The four meters from one sample: [label, value, temperature, bar 0-100, colour, note]. */
export function meters(b: Brief): [string, string, string, number, string, string][] {
  const out: [string, string, string, number, string, string][] = [];
  if (b.gpu) out.push(["Graphics", pct(b.gpu.busy), temp(b.gpu.temp), b.gpu.busy || 0, "#ff9a4d",
    [b.gpu.vramTotal ? `Video memory ${gb(b.gpu.vramUsed)} of ${gb(b.gpu.vramTotal)}` : "", b.gpu.watts ? `${Math.round(b.gpu.watts)} W` : ""].filter(Boolean).join(" · ")]);
  out.push(["Processor", pct(b.cpu.busy), temp(b.cpu.temp), b.cpu.busy || 0, "#66c0f4", b.game && b.game.cpu !== null ? `The game uses ${pct(b.game.cpu)}` : ""]);
  out.push(["Memory", gb(b.mem.used), "", b.mem.total ? (100 * (b.mem.used || 0)) / b.mem.total : 0, "#9a7cff",
    b.mem.cache ? `${gb(b.mem.cache)} more is cache the game can have` : ""]);
  const net = b.net.recv + b.net.send;
  out.push(["Network", rate(net), "", Math.min(100, (net / (50 * 1048576)) * 100), "#f2c14e", b.net.kind ? `${b.net.kind} · ${rate(b.net.recv)} down` : "Not connected"]);
  return out;
}

/** Steam's own pause of its downloads (Steam's Downloads page's), this session. False when Steam's call isn't there. */
function steamDownloads(on: boolean): boolean {
  try {
    const d = (window as any).SteamClient?.Downloads;
    if (typeof d?.EnableAllDownloads !== "function") return false;
    d.EnableAllDownloads(on);
    return true;
  } catch { return false; }
}

// Paused from this panel (this session only: nothing is saved; Steam's own pause ends with Steam).
let pausedHere = false;

export function ResourcesSection() {
  const [b, setB] = useState<Brief | null>(null);
  const [p, setP] = useState<PauseState | null>(null);
  const [paused, setPaused] = useState(pausedHere);
  const [msg, setMsg] = useState("");
  useEffect(() => {
    let live = true;
    let misses = 0;
    const tick = async () => {
      const r = await sampleRes().catch(() => null);
      if (!live) return;
      if (r) { misses = 0; setB(r); } else if (++misses >= 3) setB(null);
    };
    tick();
    const t = setInterval(tick, 1000);
    pauseRes("status").then((x) => { if (live) setP(x); }).catch(() => {});
    return () => { live = false; clearInterval(t); };
  }, []);
  if (!b) return null;   // nothing yet, or Resource Monitor's helper isn't on this system: no section
  const pause = async () => {
    const steam = steamDownloads(paused);   // resuming turns them back on
    const r = await pauseRes(paused ? "off" : "on").catch(() => null);
    pausedHere = !paused;
    setPaused(pausedHere);
    if (r) setP(r);
    const what = [steam ? "Steam downloads" : "", r && r.backups.available ? "the home server's backups" : ""].filter(Boolean).join(" and ");
    setMsg(pausedHere ? (what ? `Paused ${what} until you resume or restart.` : "Nothing to pause right now: Steam and Hearth already wait while you play.")
                      : (what ? `${what.charAt(0).toUpperCase() + what.slice(1)} can run again.` : ""));
  };
  const lightOn = async () => {
    const r = await pauseRes("light-on").catch(() => null);
    if (r) setP(r);
    setMsg("Light mode is on: the home server's big jobs wait whenever you play. Turn it off in Settings › Home server.");
  };
  return (
    <PanelSection title="Resources">
      {b && meters(b).map(([label, value, t, bar, color, note], i) => (
        <PanelSectionRow key={label}>
          <div style={{ padding: "8px 10px", borderRadius: "8px", background: i === 0 ? "#2a475e" : "#213347", boxShadow: i === 0 ? "0 0 0 2px #66c0f4" : "none", display: "flex", flexDirection: "column", gap: "6px" }}>
            <div style={{ display: "flex", alignItems: "baseline", gap: "8px" }}>
              <b style={{ flex: 1, fontWeight: 600 }}>{label}</b><span style={{ fontWeight: 700 }}>{value}</span>
              {t && <span style={{ fontSize: "12px", color: "#7fe0a4" }}>{t}</span>}
            </div>
            <span style={{ height: "8px", borderRadius: "4px", background: "#2f4a63", display: "block" }}>
              <span style={{ display: "block", height: "8px", borderRadius: "4px", width: `${Math.max(0, Math.min(100, bar))}%`, background: color }} />
            </span>
            {note && <span style={{ fontSize: "12px", color: "#9fb2c3" }}>{note}</span>}
          </div>
        </PanelSectionRow>
      ))}
      {b && b.others.length > 0 && (
        <PanelSectionRow>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ fontSize: "11px", fontWeight: 600, letterSpacing: ".04em", color: "#9fb2c3" }}>ALSO RUNNING</span>
            {b.others.map((o) => (
              <div key={o.name} style={{ display: "flex", alignItems: "center", gap: "10px", padding: "3px 2px" }}>
                <span style={{ width: "16px", height: "16px", borderRadius: "4px", background: COLORS[o.color] || COLORS.app, flex: "none" }} />
                <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{o.name}{o.tag ? ` · ${o.tag}` : ""}</span>
                <span style={{ color: "#9fb2c3", fontSize: "12px" }}>{o.cpu ? pct(o.cpu) + " · " : ""}{gb(o.mem)}</span>
              </div>
            ))}
          </div>
        </PanelSectionRow>
      )}
      {b && (
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={pause}
            description={paused ? "Paused until you resume or restart. No setting was changed." : "This session only: Steam's downloads and the home server's backups wait. No setting changes."}>
            {paused ? "Resume downloads and backups" : "Pause downloads and backups while I play"}
          </ButtonItem>
        </PanelSectionRow>
      )}
      {b && p && p.needsOption && (
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={lightOn} description="Light mode, a setting: the home server's backups and big jobs always wait while you play">
            Always wait while I play
          </ButtonItem>
        </PanelSectionRow>
      )}
      {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: "12px" }}>{msg}</div></PanelSectionRow>}
      {b && <PanelSectionRow><div style={{ fontSize: "11px", color: "#8b929a", textAlign: "center" }}>The full Resource Monitor is in Desktop Mode (Mantel)</div></PanelSectionRow>}
    </PanelSection>
  );
}
