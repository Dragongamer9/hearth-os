// The rest of Hearth's Game Mode in Steam Game Mode (docs/STEAM-GAME-MODE.md "The rest of Hearth's Game Mode"), each
// through Hearth's core service (core_service.rs `aside`, `friends`, `link`, `remote`, from main.py rest_call):
//  - Quick Resume: put the game Steam runs aside (paused, kept in memory), resume or close it (Quick Access › Hearth);
//  - Sunshine: game streaming on or off, the devices paired, pairing Moonlight with its PIN (Quick Access › Hearth);
//  - Hearth Friends (/hearth/friends): who's online and playing what, read only;
//  - Hearth Link and Hearth Storage (/hearth/link): paired devices, copies, storage servers, Sync now and Wake.
// Nothing here signs in, holds an account or keeps a secret; when the core isn't running each part says so.
import { ButtonItem, DialogButton, Focusable, Navigation, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { callable } from "@decky/api";
import { ReactNode, useState } from "react";
import { usePoll } from "./api";
import { openApp, say } from "./actions";
import { useFront } from "./core";
import { asideLine, bytes, friendLine, pinPress, remoteLine } from "./model";
import { Boundary, C, Page, onFocus } from "./ui";
import { Act, Row } from "./manage";
import { guard } from "./state";
import { brand } from "./pages";

export const rest = callable<[cmd: string, op?: string, args?: Record<string, unknown>], any>("hearth_rest");
const V = { "flow-children": "vertical" } as any;
const H = { "flow-children": "horizontal" } as any;

const Line = (p: { children?: ReactNode; dim?: boolean }) => (
  <div className={staticClasses.Text} style={{ fontSize: p.dim ? 12 : 13, color: p.dim ? C.dim : undefined, lineHeight: 1.35 }}>{p.children}</div>
);

async function act(cmd: string, op: string, args: Record<string, unknown>, ok: string, after?: (r: any) => void) {
  const r = await rest(cmd, op, args).catch((e) => ({ error: String(e) }));
  if (r?.error) say(r.error); else if (ok) say(ok);
  after?.(r);
  return r;
}

// ---------------------------------------------------------------- Quick Resume

interface Aside { appid: number; game?: string | null; title?: string; at?: number }

/** Quick Resume in Steam Game Mode: Steam runs one game at a time, so this is the game Steam runs, paused where it is
 *  while you're in Steam's screens; it carries on by itself when you go back to it (Steam's Resume). */
export function QuickResumeBody() {
  const { run } = useFront();
  const [l, again] = usePoll<{ list: Aside[] }>(() => rest("aside", "list", {}), 5000, []);
  const list = l && !l.error && Array.isArray(l.list) ? l.list : [];
  const aside = !!run && list.some((a) => a.appid === run.appid);
  const put = async () => {
    if (!run) return;
    const r = await act("aside", "put", { appid: run.appid, game: run.id || null, title: run.title }, `${run.title || "The game"} is put aside`, again);
    if (r && !r.error) { try { Navigation.Navigate("/library/home"); Navigation.CloseSideMenus(); } catch { /* Steam's own menu still works */ } }
  };
  return (
    <>
      {l?.error && <PanelSectionRow><Line dim>{l.error}</Line></PanelSectionRow>}
      {run && !aside && (
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={put} description="Paused where it is, kept in memory, while you're in Steam's screens; Steam's Resume carries on">
            Put {run.title || "the game"} aside
          </ButtonItem>
        </PanelSectionRow>
      )}
      {!run && !list.length && <PanelSectionRow><Line dim>No game is running.</Line></PanelSectionRow>}
      {list.map((a) => (
        <PanelSectionRow key={a.appid}>
          <Line>{asideLine(a, Date.now())}</Line>
          <Focusable style={{ display: "flex", gap: 6, marginTop: 6 }} {...H}>
            <DialogButton style={{ minWidth: 0, flex: 1 }} onClick={() => act("aside", "resume", { appid: a.appid }, "Carries on", again)}>Resume</DialogButton>
            <CloseButton onClick={() => act("aside", "close", { appid: a.appid }, `${a.title || "The game"} closed`, again)} />
          </Focusable>
        </PanelSectionRow>
      ))}
    </>
  );
}

function CloseButton(props: { onClick: () => void }) {
  const [asking, setAsking] = useState(false);
  return (
    <DialogButton style={{ minWidth: 0, flex: 1 }} onClick={() => { if (!asking) { setAsking(true); setTimeout(() => setAsking(false), 5000); return; } setAsking(false); props.onClick(); }}>
      {asking ? "Press again to close" : "Close"}
    </DialogButton>
  );
}

// ---------------------------------------------------------------- Sunshine

interface Remote { installed?: boolean; on?: boolean; clients?: { name: string }[]; needsLogin?: boolean; addresses?: string[] }

/** Game streaming (Sunshine: Moonlight plays this device elsewhere), the same as Hearth's Settings › Remote play. */
export function StreamingBody() {
  const [s, setS] = useState<Remote | null>(null);
  const [, again] = usePoll<Remote>(() => rest("remote", "status", {}).then((r) => { setS(r); return r; }), 15000, []);
  const [busy, setBusy] = useState(false);
  const [pin, setPin] = useState("");
  const [pairing, setPairing] = useState(false);
  const toggle = async (on: boolean) => { setBusy(true); await act("remote", on ? "on" : "off", {}, on ? "Game streaming is on" : "Game streaming is off", (r) => { if (r && !r.error) setS(r); }); setBusy(false); };
  const pair = async () => { setBusy(true); await act("remote", "pair", { pin, name: "Moonlight" }, "Paired: it can stream this device now", (r) => { if (r && !r.error) { setS(r); setPin(""); setPairing(false); } }); setBusy(false); };
  const err = (s as any)?.error as string | undefined;
  return (
    <>
      <PanelSectionRow><Line dim={!!err}>{err || remoteLine(s)}</Line></PanelSectionRow>
      {s && !err && s.installed && (
        <PanelSectionRow>
          <ToggleField label="Game streaming (Sunshine)" description="Play this device on a TV, phone or PC with Moonlight" checked={!!s.on} disabled={busy} onChange={toggle} />
        </PanelSectionRow>
      )}
      {s?.on && !pairing && (
        <PanelSectionRow><ButtonItem layout="below" onClick={() => { setPairing(true); setPin(""); }} description="Moonlight shows a 4-digit PIN when you pick this device">Pair a device</ButtonItem></PanelSectionRow>
      )}
      {s?.on && pairing && (
        <PanelSectionRow>
          <Line>PIN: <span style={{ fontSize: 20, fontWeight: 700, letterSpacing: ".3em", color: C.accent }}>{(pin + "____").slice(0, 4)}</span></Line>
          <Focusable style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 6, marginTop: 6 }} {...({ "flow-children": "grid" } as any)}>
            {["1", "2", "3", "4", "5", "6", "7", "8", "9", "del", "0", "ok"].map((k) => (
              <DialogButton key={k} style={{ minWidth: 0, padding: "8px 0", background: k === "ok" && pin.length === 4 ? C.accent : undefined, color: k === "ok" && pin.length === 4 ? C.ink : undefined }}
                disabled={busy || (k === "ok" && pin.length !== 4)}
                onClick={() => { if (k === "ok") pair(); else setPin(pinPress(pin, k)); }}>
                {k === "del" ? "⌫" : k === "ok" ? "Pair" : k}
              </DialogButton>
            ))}
          </Focusable>
          <div style={{ marginTop: 6 }}><DialogButton style={{ minWidth: 0, width: "100%" }} onClick={() => { setPairing(false); setPin(""); again(); }}>Cancel</DialogButton></div>
        </PanelSectionRow>
      )}
    </>
  );
}

// ---------------------------------------------------------------- Hearth Friends

interface Friend { name: string; presence: string; game?: string | null; joinable?: boolean }

/** Hearth Friends (Matrix), read only. Chats, requests and invites stay in Hearth's Game Mode (or any Matrix app);
 *  Steam's friends are in Steam's own Friends & Chat. */
export function FriendsPage() {
  const [f] = usePoll<{ signedIn: boolean; friends: Friend[] }>(() => rest("friends", "", {}), 30000, []);
  const list = f && !f.error && Array.isArray(f.friends) ? f.friends : [];
  return (
    <Boundary part="friends" onError={guard.fail}>
      <Page title="Hearth Friends" wallpaper={brand()?.wallpaper}>
        <div className="ht-dim">Your friends on Hearth Friends and what they're playing. Steam's friends are in Steam's Friends &amp; Chat.</div>
        {!f && <div className="ht-dim" style={{ marginTop: 14 }}>Asking Hearth Friends…</div>}
        {f?.error && <div className="ht-card" style={{ marginTop: 14 }}>{f.error}</div>}
        {f && !f.error && !f.signedIn && <div className="ht-card" style={{ marginTop: 14 }}>Hearth Friends isn't signed in on this device. Sign in from Hearth's Friends (Hearth in Desktop Mode).</div>}
        {f && !f.error && f.signedIn && !list.length && <div className="ht-card" style={{ marginTop: 14 }}>No friends yet. Add them in Hearth's Friends.</div>}
        <Focusable {...V} style={{ marginTop: 14, display: "flex", flexDirection: "column", gap: 8, maxWidth: 720 }}>
          {list.map((x, i) => (
            <DialogButton key={i} className="ht-btn" style={{ textAlign: "left", display: "flex", alignItems: "center", gap: 12 }} {...({ onGamepadFocus: onFocus } as any)}>
              <span style={{ width: 10, height: 10, borderRadius: 5, flex: "none", background: x.game ? C.accent : x.presence === "online" ? "#7fd36b" : C.panel3 }} />
              <span style={{ flex: 1 }}><b>{x.name}</b><br /><span className="ht-dim">{friendLine(x)}</span></span>
            </DialogButton>
          ))}
        </Focusable>
        <Row><Act label="Chats and requests in Hearth" onClick={() => openApp("hearth-app:hearth")} /></Row>
      </Page>
    </Boundary>
  );
}

// ---------------------------------------------------------------- Hearth Link and Hearth Storage

interface LinkStatus {
  running: boolean; installed?: boolean; error?: string; name?: string; playing?: boolean; offers?: number;
  peers?: { id: string; name: string; online: boolean }[];
  copies?: { title: string; state: string; done?: number; total?: number; from?: string }[];
  servers?: { id: string; name: string; free?: number; usable?: number; canWake?: boolean }[];
}

/** Hearth Link (your Hearth devices, kept in sync, games copied between them) and Hearth Storage (a device keeping
 *  games, backups and files for the others): what's there, Sync now, Wake. Pairing and settings stay in Hearth's
 *  Settings › Hearth Link (a window). */
export function LinkPage() {
  const [s, again] = usePoll<LinkStatus>(() => rest("link", "status", {}), 4000, []);
  const ok = s && !s.error && s.running;
  return (
    <Boundary part="link" onError={guard.fail}>
      <Page title="Hearth Link" wallpaper={brand()?.wallpaper}>
        <div className="ht-dim">Your Hearth devices: kept in sync, games copied between them, and Hearth Storage.</div>
        {!s && <div className="ht-dim" style={{ marginTop: 14 }}>Asking Hearth Link…</div>}
        {s && !ok && <div className="ht-card" style={{ marginTop: 14 }}>{s.error || "Hearth Link isn't running on this device."}{s.installed === false ? " It isn't part of this Hearth OS." : ""}</div>}
        {ok && (
          <>
            <div className="ht-dim" style={{ marginTop: 6 }}>This device: {s.name || "Hearth"}{s.playing ? " · quiet while a game runs" : ""}{s.offers ? ` · ${s.offers} game${s.offers === 1 ? "" : "s"} offered (accept in Hearth)` : ""}</div>
            <h2>Devices</h2>
            {!s.peers?.length && <div className="ht-dim">No devices paired yet: pair one in Hearth's Settings › Hearth Link.</div>}
            <Focusable {...V}>
              {(s.peers || []).map((p) => (
                <div key={p.id} className="ht-card">
                  <b>{p.name}</b> <span className="ht-dim">{p.online ? "Online" : "Not on right now"}</span>
                  <Row><Act label="Sync now" disabled={!p.online} onClick={() => act("link", "sync", { peer: p.id }, `Synced with ${p.name}`, again)} /></Row>
                </div>
              ))}
            </Focusable>
            {!!s.copies?.length && <h2>Copies</h2>}
            {(s.copies || []).map((c, i) => (
              <div key={i} className="ht-card"><b>{c.title}</b> <span className="ht-dim">{c.from ? `from ${c.from} · ` : ""}{c.state}{c.total ? ` · ${bytes(c.done)} of ${bytes(c.total)}` : ""}</span></div>
            ))}
            <h2>Hearth Storage</h2>
            {!s.servers?.length && <div className="ht-dim">No Hearth Storage device found. Set one up in Hearth's Settings › Hearth Link.</div>}
            <Focusable {...V}>
              {(s.servers || []).map((v) => (
                <div key={v.id} className="ht-card">
                  <b>{v.name}</b> <span className="ht-dim">{v.free != null ? `${bytes(v.free)} free` : "Asleep or away"}</span>
                  {v.canWake && <Row><Act label="Wake it" onClick={() => act("link", "wake", { server: v.id }, `Waking ${v.name}`, again)} /></Row>}
                </div>
              ))}
            </Focusable>
          </>
        )}
        <Row><Act label="Pairing and settings in Hearth" onClick={() => openApp("hearth-app:settings")} /></Row>
      </Page>
    </Boundary>
  );
}
