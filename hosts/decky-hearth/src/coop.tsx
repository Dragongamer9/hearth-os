// Couch Co-op in Steam Game Mode (docs/STEAM-GAME-MODE.md "Couch co-op in Steam Game Mode", Experimental):
//  - Quick Access › Hearth Players: the controllers connected, in Hearth's player order (hearth-coop `players`, kept
//    by the controller, not the port), moved up and down, and a short rumble to tell which one is which;
//  - the split-screen page (/hearth/coop): games that play split-screen, each played or added to Steam's library as
//    one Steam shortcut running `hearth-coop play --game <id>` (main.py coop_shortcut): Hearth's own Couch Co-op split
//    screen (a nested labwc inside Steam's gamescope, a nested gamescope per player, each player's copy seeing only
//    their own controller), started with the player order.
// Everything goes through Hearth's core service (`coop`), or hearth-coop itself when the core isn't running. Nothing
// here can stop Steam: every answer that fails is shown as a line, and the page is inside a Boundary.
import { ButtonItem, DialogButton, Focusable, PanelSectionRow, staticClasses } from "@decky/ui";
import { callable } from "@decky/api";
import { ReactNode, useEffect, useState } from "react";
import { COOP_LAYOUTS, CoopPad, CoopPlayers, coopLine, coopProblem, movePad } from "./model";
import { Boundary, C, Chips, Page, go } from "./ui";
import { guard } from "./state";
import { brand } from "./pages";
import { say } from "./actions";
import { Item, add, play } from "./shortcuts";
import { Act, Row } from "./manage";

type Ans<T> = T & { error?: string };
const coopRaw = callable<[op: string, pads: string[], pad: string], any>("coop");
const coopTarget = callable<[id: string, players: number, layout: string], { ok: boolean; error?: string; exe?: string; startDir?: string; options?: string }>("coop_target");

async function coop<T = any>(op: string, pads: string[] = [], pad = ""): Promise<Ans<T>> {
  try {
    const r = await coopRaw(op, pads, pad);
    return r && typeof r === "object" ? r : ({ error: "Hearth gave no answer" } as any);
  } catch (e) { return { error: String(e) } as any; }
}

/** The players, asked again every `every` ms while shown (controllers come and go). */
function usePlayers(every: number): [Ans<CoopPlayers> | null, (p: Ans<CoopPlayers>) => void] {
  const [p, setP] = useState<Ans<CoopPlayers> | null>(null);
  useEffect(() => {
    let live = true;
    const f = () => coop<CoopPlayers>("players").then((r) => { if (live) setP(r); });
    f();
    const t = setInterval(f, every);
    return () => { live = false; clearInterval(t); };
  }, []);
  return [p, setP];
}

const Line = (p: { children?: ReactNode; dim?: boolean }) => (
  <div className={staticClasses.Text} style={{ fontSize: p.dim ? 12 : 13, color: p.dim ? C.dim : undefined, lineHeight: 1.35 }}>{p.children}</div>
);

function Badge(props: { pad: CoopPad }) {
  const n = props.pad.player;
  return (
    <div className="hearth-player-badge" style={{ width: 34, height: 34, borderRadius: 17, flex: "none", display: "flex", alignItems: "center", justifyContent: "center",
      fontWeight: 800, fontSize: 13, background: n ? C.accent : C.panel3, color: n ? C.ink : C.dim }}>{n ? `P${n}` : "–"}</div>
  );
}

const small = { minWidth: 0, flex: 1, padding: "6px 4px", fontSize: 13 };

/** Quick Access › Hearth Players. */
export function PlayersBody() {
  const [p, setP] = usePlayers(3000);
  const [busy, setBusy] = useState(false);
  const pads = p && !p.error && Array.isArray(p.pads) ? p.pads : [];
  const move = async (i: number, dir: -1 | 1) => {
    setBusy(true);
    const r = await coop<CoopPlayers>("order", movePad(pads.map((d) => d.id), i, dir));
    setBusy(false);
    if (r.error) say(r.error); else setP(r);
  };
  const identify = async (d: CoopPad) => {
    const r = await coop<{ rumbled: boolean; why?: string }>("identify", [], d.id);
    if (r.error) say(r.error);
    else if (!r.rumbled) say(r.why || `${d.name} didn't rumble`);
  };
  const problem = coopProblem(p && !p.error ? p : null, Date.now());
  return (
    <>
      <PanelSectionRow><Line>{p?.error ? p.error : coopLine(p)}</Line></PanelSectionRow>
      {pads.map((d, i) => (
        <PanelSectionRow key={d.id}>
          <div className="hearth-player" style={{ display: "flex", flexDirection: "column", gap: 6, padding: "6px 0" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <Badge pad={d} />
              <div style={{ minWidth: 0 }}>
                <div style={{ fontSize: 14, fontWeight: 600, color: C.text, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{d.name}</div>
                <div style={{ fontSize: 11.5, color: C.dim }}>
                  {d.player ? (d.playerName || `Player ${d.player}`) : "Waits (4 players at most)"}{d.kind === "inputplumber" ? " · through InputPlumber" : ""}
                </div>
              </div>
            </div>
            <Focusable style={{ display: "flex", gap: 6 }} {...({ "flow-children": "horizontal" } as any)}>
              <DialogButton style={small} disabled={busy || i === 0} onClick={() => move(i, -1)}>Move up</DialogButton>
              <DialogButton style={small} disabled={busy || i === pads.length - 1} onClick={() => move(i, 1)}>Move down</DialogButton>
              <DialogButton style={small} disabled={d.rumble === false} onClick={() => identify(d)}>Rumble</DialogButton>
            </Focusable>
          </div>
        </PanelSectionRow>
      ))}
      {p?.running && <PanelSectionRow><Line dim>A split-screen game is running: a new order is used the next time one starts.</Line></PanelSectionRow>}
      {problem && <PanelSectionRow><Line dim>Last split-screen start: {problem}</Line></PanelSectionRow>}
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => go("/hearth/coop")} description="A part of the screen and a controller for each player, started as one Steam game">
          Split-screen games
        </ButtonItem>
      </PanelSectionRow>
    </>
  );
}

interface CoopGame { id: string; title: string; source?: string; canPlay: boolean; why?: string; run?: string }
const SOURCES: Record<string, string> = { steam: "Steam", epic: "Epic", gog: "GOG", manual: "Added to Hearth" };

/** The split-screen page: players, how many and how the screen is shared, then the games. */
export function CoopPage() {
  const [p] = usePlayers(4000);
  const [g, setG] = useState<Ans<{ games: CoopGame[] }> | null>(null);
  const [count, setCount] = useState("0");
  const [layout, setLayout] = useState("auto");
  useEffect(() => { let live = true; coop<{ games: CoopGame[] }>("games").then((r) => { if (live) setG(r); }); return () => { live = false; }; }, []);
  const pads = p && !p.error && Array.isArray(p.pads) ? p.pads : [];
  const playing = pads.filter((d) => d.player);
  const games = g && !g.error && Array.isArray(g.games) ? g.games : [];
  const yes = games.filter((x) => x.canPlay), no = games.filter((x) => !x.canPlay);
  const counts: [string, string][] = [["0", playing.length >= 2 ? `Everyone (${playing.length})` : "Everyone"], ["2", "2 players"], ["3", "3 players"], ["4", "4 players"]];
  const item = async (x: CoopGame): Promise<Item | null> => {
    const t = await coopTarget(x.id, Number(count), layout).catch((e) => ({ ok: false, error: String(e) } as any));
    if (!t.ok || !t.exe) { say(t.error || "Hearth can't play that split-screen here"); return null; }
    return { id: "coop:" + x.id, title: `${x.title} (Split-screen)`, source: "coop", store: "Split-screen", cover: "", wide: "", icon: "",
             exe: t.exe, startDir: t.startDir || "", options: t.options || "" };
  };
  const start = async (x: CoopGame) => {
    const want = Number(count) || playing.length;
    if (playing.length < 2 || want > pads.length) { say(`Connect ${Math.max(2, want)} controllers first`); return; }
    const it = await item(x);
    if (it) { try { await play(it); } catch (e) { say(`Couldn't start ${x.title}: ${e}`); } }
  };
  const addIt = async (x: CoopGame) => {
    const it = await item(x);
    if (it) { try { await add(it); say(`${it.title} is in Steam's library`); } catch (e) { say(`Couldn't add it: ${e}`); } }
  };
  return (
    <Boundary part="coop" onError={guard.fail}>
      <Page title="Split-screen" wallpaper={brand()?.wallpaper}>
        <div className="ht-dim">Hearth's Couch Co-op in Steam Game Mode (Experimental): each player gets a part of the screen, their own controller and their
          own copy of the game, all started as one game in Steam. Steam's overlay and Exit game work on the whole split screen.</div>
        <h2>Players</h2>
        <div className="ht-card">
          {!p && <div className="ht-dim">Looking for controllers…</div>}
          {p?.error && <div>{p.error}</div>}
          {p && !p.error && (
            <>
              <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
                {pads.map((d) => (
                  <div key={d.id} style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
                    <Badge pad={d} />
                    <div style={{ fontSize: 13 }}><div style={{ fontWeight: 600 }}>{d.player ? (d.playerName || `Player ${d.player}`) : "Waits"}</div><div className="ht-dim" style={{ fontSize: 12 }}>{d.name}</div></div>
                  </div>
                ))}
              </div>
              <div className="ht-dim" style={{ marginTop: 10 }}>{coopLine(p)} Change the order in Quick Access › Hearth Players.</div>
            </>
          )}
        </div>
        <h2>How many play</h2>
        <Chips items={counts} value={count} onChange={setCount} />
        <h2>Screen</h2>
        <Chips items={COOP_LAYOUTS} value={layout} onChange={setLayout} />
        <h2>Games</h2>
        {!g && <div className="ht-dim">Looking at your games…</div>}
        {g?.error && <div className="ht-card">{g.error}</div>}
        {g && !g.error && !yes.length && <div className="ht-card">None of your installed games can play split-screen this way yet.</div>}
        {yes.map((x, i) => (
          <div className="ht-card" key={x.id}>
            <div style={{ fontWeight: 600, fontSize: 16 }}>{x.title}</div>
            <div className="ht-dim">{SOURCES[x.source || ""] || x.source || ""}{x.run === "windows" ? " · Windows game (Proton)" : ""}</div>
            <Row>
              <Act primary autoFocus={i === 0} label="Play split-screen" onClick={() => start(x)} />
              <Act label="Add to Steam library" onClick={() => addIt(x)} />
            </Row>
          </div>
        ))}
        {no.length > 0 && <h2>Can't play split-screen</h2>}
        {no.slice(0, 12).map((x) => (
          <div className="ht-card" key={x.id}><div style={{ fontWeight: 600 }}>{x.title}</div><div className="ht-dim">{x.why}</div></div>
        ))}
        <div className="ht-dim" style={{ marginTop: 14 }}>Set the game's Steam Input to off (its controller settings in Steam), so Steam doesn't also move a mouse or
          keyboard in the split screen.</div>
      </Page>
    </Boundary>
  );
}
