// Steam games from other Steam folders (docs/HEARTH-SYNC.md): games installed when this PC used Flathub's Steam, or in
// SteamLibrary folders on other drives Steam isn't using. The backend finds them (hearth-steam-import's scan); bringing
// them in needs Steam closed, so "Bring them in" arms a helper and restarts Steam: while Steam is away the games are
// moved (same drive) or their library folder is added to Steam's list (other drive). Nothing is overwritten.
import { ButtonItem, PanelSection, PanelSectionRow, staticClasses } from "@decky/ui";
import { callable } from "@decky/api";
import { useEffect, useState } from "react";

export interface Found { appid: string; name: string; how: string; source?: string; target?: string; why?: string }
interface Scan { ok: boolean; error?: string; target?: string; games: Found[]; skipped: Found[] }
interface Counts { moved: number; registered: number; libraries: number; skipped: number; failed: number }
interface State { pending: boolean; result: { ok?: boolean; error?: string; counts?: Counts; failed?: Found[] } | null }

const scan = callable<[], Scan>("import_scan");
const state = callable<[], State>("import_state");
const arm = callable<[], boolean>("import_arm");

/** What the result of the last import says, in one line. */
export function resultLine(r: State["result"]): string {
  if (!r) return "";
  if (r.ok === false || !r.counts) return `The import didn't run: ${r.error || "Steam didn't close"}.`;
  const c = r.counts;
  const parts = [`${c.moved} moved`];
  if (c.registered) parts.push(`${c.registered} from ${c.libraries} added library folder${c.libraries === 1 ? "" : "s"}`);
  if (c.skipped) parts.push(`${c.skipped} skipped`);
  if (c.failed) parts.push(`${c.failed} failed`);
  return `Last import: ${parts.join(", ")}.`;
}

/** Steam's own restart (the power menu's Restart Steam). False when Steam's call isn't there. */
function restartSteam(): boolean {
  try {
    const u = (window as any).SteamClient?.User;
    if (typeof u?.StartRestart !== "function") return false;
    u.StartRestart(false);
    return true;
  } catch { return false; }
}

export function ImportSection() {
  const [found, setFound] = useState<Scan | null>(null);
  const [st, setSt] = useState<State | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [msg, setMsg] = useState("");
  const look = () => { scan().then(setFound).catch(() => setFound({ ok: false, error: "couldn't look", games: [], skipped: [] })); state().then(setSt).catch(() => {}); };
  useEffect(look, []);
  const go = async () => {
    if (!confirm) { setConfirm(true); return; }
    setConfirm(false);
    if (!(await arm().catch(() => false))) { setMsg("The importer isn't on this system."); return; }
    setMsg("Steam restarts now; the games are brought in while it's closed.");
    if (!restartSteam()) setMsg("Close Steam (or leave Game Mode) to finish; the games are brought in when it has closed.");
    state().then(setSt).catch(() => {});
  };
  const n = found?.games.length || 0;
  return (
    <PanelSection title="Other Steam folders">
      {!found && <PanelSectionRow><div className={staticClasses.Text}>Looking…</div></PanelSectionRow>}
      {found && !found.ok && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12, color: "#ffb37a" }}>Couldn't look: {found.error}</div></PanelSectionRow>}
      {found?.ok && !n && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>
        No Steam games in other Steam folders{found.skipped.length ? ` (${found.skipped.length} Steam already has)` : ""}.</div></PanelSectionRow>}
      {found?.ok && n > 0 && <PanelSectionRow><div className={staticClasses.Text}>
        {n} game{n === 1 ? "" : "s"} Steam doesn't see yet</div></PanelSectionRow>}
      {(found?.games || []).slice(0, 6).map((g) => (
        <PanelSectionRow key={g.appid}><div className={staticClasses.Text} style={{ fontSize: 12 }}>
          {g.name} <span style={{ color: "#8d99a8" }}>· {g.how === "move" ? "moved (same drive)" : "its drive's library added"}</span></div></PanelSectionRow>
      ))}
      {n > 6 && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12, color: "#8d99a8" }}>and {n - 6} more</div></PanelSectionRow>}
      {(found?.skipped || []).filter((g) => g.why !== "Steam already has it").slice(0, 3).map((g) => (
        <PanelSectionRow key={"s" + g.appid}><div className={staticClasses.Text} style={{ fontSize: 12, color: "#ffb37a" }}>{g.name}: skipped, {g.why}</div></PanelSectionRow>
      ))}
      {st?.pending && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>Waiting for Steam to close to bring them in…</div></PanelSectionRow>}
      {!st?.pending && st?.result && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{resultLine(st.result)}</div></PanelSectionRow>}
      {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{msg}</div></PanelSectionRow>}
      {n > 0 && <PanelSectionRow>
        <ButtonItem layout="below" disabled={!!st?.pending} onClick={go}
          description="Moves them with their Proton saves, or adds their drive's library to Steam; never overwrites anything">
          {confirm ? "Press again: Steam restarts" : `Bring ${n === 1 ? "it" : "them"} into Steam`}
        </ButtonItem>
      </PanelSectionRow>}
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={look}>Look again</ButtonItem>
      </PanelSectionRow>
    </PanelSection>
  );
}
