// Hearth Music (docs/HEARTH-MUSIC.md): plays the theme song of the game you're looking at in Steam Game Mode's
// library, crossfades between games, fades out when you press Play and stays silent while a game runs. The song is
// one you picked, the game's own soundtrack, or Apple Music's official 30-second preview (credited here). Never
// YouTube. Nothing of Steam's screens is changed.
import { ButtonItem, Navigation, PanelSection, PanelSectionRow, SliderField, ToggleField, staticClasses } from "@decky/ui";
import { FileSelectionType, callable, definePlugin, openFilePicker } from "@decky/api";
import { useEffect, useState } from "react";
import { FaMusic } from "react-icons/fa";
import { credit, nowLine, silentWhy } from "./model";
import { State, begin, onChange, refresh, setPick, state, updateSettings } from "./music";

const getHome = callable<[], string>("get_home");
const EXTS = ["mp3", "ogg", "opus", "flac", "m4a", "aac", "wav"];

function openLink(url: string) {
  try { (Navigation as any).NavigateToExternalWeb(url); } catch { /* Steam's browser isn't reachable */ }
}

function Panel() {
  const [s, setS] = useState<State>({ ...state });
  const [msg, setMsg] = useState("");
  useEffect(() => onChange(() => setS({ ...state })) as any, []);
  const st = s.settings;
  const why = silentWhy(s.seen, st, s.song);
  const pick = async (kind: "app" | "collection", name: string, label: string) => {
    setMsg("");
    try {
      const home = await getHome().catch(() => "/home");
      const r: any = await openFilePicker(FileSelectionType.FILE, home, true, true, undefined, EXTS, false, false);
      const path = r?.realpath || r?.path;
      if (!path) return;
      setMsg((await setPick(kind, name, path)) ? `Picked for ${label}.` : "That isn't an audio file.");
      refresh(s.appid);
    } catch { /* closed */ }
  };
  const unpick = async () => { await setPick("app", String(s.appid), ""); setMsg("Back to the automatic song."); refresh(s.appid); };
  return (
    <>
      <PanelSection title="Now playing">
        {s.off && <PanelSectionRow><div className={staticClasses.Text} style={{ color: "#ffb37a" }}>{s.off}</div></PanelSectionRow>}
        {!s.off && !why && s.song?.source && <>
          <PanelSectionRow><div className={staticClasses.Text} style={{ fontWeight: 700 }}>{s.title || "This game"}</div></PanelSectionRow>
          <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 13 }}>{nowLine(s.song)}</div></PanelSectionRow>
          {credit(s.song) && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 11.5, color: "#8d99a8" }}>{credit(s.song)}</div></PanelSectionRow>}
          {s.song.link && <PanelSectionRow><ButtonItem layout="below" onClick={() => openLink(s.song!.link!)}>Open in Apple Music</ButtonItem></PanelSectionRow>}
        </>}
        {!s.off && (why || !s.song?.source) && <PanelSectionRow><div className={staticClasses.Text}>
          {why || (s.appid ? `Finding ${s.title || "the game"}'s theme…` : "")}</div></PanelSectionRow>}
      </PanelSection>
      <PanelSection title="Settings">
        <PanelSectionRow>
          <ToggleField label="Hearth Music" description="Theme songs while you browse" checked={st.enabled} onChange={(v: boolean) => updateSettings({ enabled: v })} />
        </PanelSectionRow>
        <PanelSectionRow>
          <SliderField label="Volume" value={Math.round(st.volume * 100)} min={0} max={100} step={5} showValue disabled={!st.enabled}
            onChange={(v: number) => updateSettings({ volume: v / 100 })} />
        </PanelSectionRow>
        <PanelSectionRow>
          <SliderField label="Wait before playing" value={st.delay} min={0} max={5} step={0.5} showValue valueSuffix=" s" disabled={!st.enabled}
            onChange={(v: number) => updateSettings({ delay: v })} />
        </PanelSectionRow>
        <PanelSectionRow>
          <ToggleField label="In the library too" description={st.where === "library" ? "Plays for the highlighted game in the library and on game pages" : "Only on a game's page"}
            checked={st.where === "library"} disabled={!st.enabled} onChange={(v: boolean) => updateSettings({ where: v ? "library" : "pages" })} />
        </PanelSectionRow>
        <PanelSectionRow>
          <ToggleField label="Apple Music previews" description="When a game has no song of its own: Apple's official 30-second preview of its theme"
            checked={st.online} disabled={!st.enabled} onChange={(v: boolean) => updateSettings({ online: v })} />
        </PanelSectionRow>
        <PanelSectionRow>
          <ToggleField label="Now playing note" description="A short note when a new song starts" checked={st.toast} disabled={!st.enabled}
            onChange={(v: boolean) => updateSettings({ toast: v })} />
        </PanelSectionRow>
      </PanelSection>
      {s.appid > 0 && <PanelSection title={`Song for ${s.title || "this game"}`}>
        {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{msg}</div></PanelSectionRow>}
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={() => pick("app", String(s.appid), s.title || "this game")} description="An audio file of yours, for this game">Pick a song…</ButtonItem>
        </PanelSectionRow>
        {s.collections.slice(0, 2).map((c) => (
          <PanelSectionRow key={c}>
            <ButtonItem layout="below" onClick={() => pick("collection", c, `the ${c} collection`)} description={`For every game in ${c} that has no pick of its own`}>
              Pick a song for {c}…
            </ButtonItem>
          </PanelSectionRow>
        ))}
        {s.song?.source === "pick" && s.song.via !== "collection" && <PanelSectionRow>
          <ButtonItem layout="below" onClick={unpick}>Use the automatic song</ButtonItem>
        </PanelSectionRow>}
      </PanelSection>}
    </>
  );
}

export default definePlugin(() => {
  const end = begin();
  return {
    name: "Hearth Music",
    titleView: <div className={staticClasses.Title}>Hearth Music</div>,
    content: <Panel />,
    icon: <FaMusic />,
    onDismount() { end(); },
  };
});
