// Which Game Mode (docs/HEARTH-GAME-MODE.md, Hearth 1.10): Steam Game Mode (this one, the default) or Hearth Game Mode,
// picked here too (the same choice as Settings › Gaming in Mantel and Hearth Game Mode's Settings). Hearth OS's own
// helper keeps it (main.py game_mode_*: hearth-game-mode, one line of session.conf, only when you pick one); it applies
// the next time Game Mode starts, and "Switch now" asks the session (nimbus-mode), which closes Steam properly first.
// When Hearth Game Mode couldn't start (the session opened Steam Game Mode instead), it says why here and once as a
// toast, with "Try Hearth Game Mode again". While Steam Game Mode is open for one Steam game started from Hearth Game Mode,
// "Back to Hearth Game Mode" is first. Nothing of Steam's screens is changed.
import { ButtonItem, PanelSection, PanelSectionRow, staticClasses } from "@decky/ui";
import { callable, toaster } from "@decky/api";
import { useEffect, useState } from "react";

export interface GameModeState {
  choice?: "steam" | "hearth"; saved?: boolean; hearth?: { ok: boolean; why: string }; steam?: { ok: boolean; why: string };
  fallback?: string; paused?: string; error?: string;
  // A Steam game opened from Hearth Game Mode (its app id): Steam Game Mode is here for that one game.
  steamGame?: string;
}
const getGm = callable<[], GameModeState>("game_mode_get");
const setGm = callable<[choice: string], GameModeState>("game_mode_set");
const switchGm = callable<[to: string], boolean>("game_mode_switch");
const noticeGm = callable<[], { text?: string }>("game_mode_notice");

const NAME = { steam: "Steam Game Mode", hearth: "Hearth Game Mode" } as const;

// Once per plugin load: the session fell back to Steam Game Mode because Hearth Game Mode didn't start.
export function gameModeNotice() {
  noticeGm().then((r) => {
    if (r && r.text) {
      try { toaster.toast({ title: "Hearth Game Mode didn't start", body: `${r.text}. Steam Game Mode opened instead; Hearth Sync › Game Mode tries again.` }); }
      catch { /* Decky has no toaster */ }
    }
  }).catch(() => {});
}

export function GameModeSection() {
  const [st, setSt] = useState<GameModeState | null>(null);
  const [picked, setPicked] = useState("");
  const [msg, setMsg] = useState("");
  useEffect(() => { getGm().then(setSt).catch(() => setSt({ error: "Couldn't read the Game Mode choice" })); }, []);
  if (!st) return null;
  if (st.error && !st.choice) {
    return <PanelSection title="Game Mode"><PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{st.error}</div></PanelSectionRow></PanelSection>;
  }
  const choice = st.choice || "steam";
  const pick = async (v: "steam" | "hearth") => {
    setMsg("");
    if (v === choice && st.saved) return;
    const r = await setGm(v).catch(() => ({ error: "That didn't work" } as GameModeState));
    if (r.error) { setMsg(r.error); return; }
    setSt(r); setPicked(v);
  };
  const go = async (to: string) => { const ok = await switchGm(to).catch(() => false); if (!ok) setMsg("Couldn't switch from here. Use Steam's Power › Switch to Desktop, then Game Mode."); };
  const row = (v: "steam" | "hearth", desc: string) => (
    <PanelSectionRow key={v}>
      <ButtonItem layout="below" onClick={() => pick(v)} description={(st[v] && st[v]!.ok === false) ? `Can't start here: ${st[v]!.why}` : desc}>
        {choice === v ? `✓ ${NAME[v]}` : NAME[v]}{choice === v ? (v === "steam" && !st.saved ? " (in use, default)" : " (in use)") : ""}
      </ButtonItem>
    </PanelSectionRow>
  );
  return (
    <PanelSection title="Game Mode">
      {st.steamGame && (
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={() => go("hearth")} description="You're here for one Steam game. Hearth Game Mode comes back by itself when it ends; this goes now (Steam closes properly first).">
            Back to Hearth Game Mode
          </ButtonItem>
        </PanelSectionRow>
      )}
      <PanelSectionRow>
        <div className={staticClasses.Text} style={{ fontSize: 12, color: "#b8bcbf" }}>Which Game Mode this PC goes into. Applies the next time Game Mode starts.</div>
      </PanelSectionRow>
      {row("steam", "Steam's own big-screen mode, this one, with Hearth Sync")}
      {row("hearth", "Hearth's own screens: every store in one library. Steam games still play through Steam.")}
      {picked && picked !== "steam" && (
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={() => go("hearth")} description="Steam closes properly first">Switch to Hearth Game Mode now</ButtonItem>
        </PanelSectionRow>
      )}
      {choice === "hearth" && !picked && (st.fallback || st.paused) && (
        <>
          <PanelSectionRow>
            <div className={staticClasses.Text} style={{ fontSize: 12, color: "#ffb37a" }}>
              {st.paused ? `Hearth Game Mode is paused: it failed twice (${st.paused}).` : `Hearth Game Mode didn't start: ${st.fallback}.`} Your choice is still Hearth Game Mode.
            </div>
          </PanelSectionRow>
          <PanelSectionRow>
            <ButtonItem layout="below" onClick={() => go("hearth")}>Try Hearth Game Mode again</ButtonItem>
          </PanelSectionRow>
        </>
      )}
      {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12, color: "#ffb37a" }}>{msg}</div></PanelSectionRow>}
    </PanelSection>
  );
}
