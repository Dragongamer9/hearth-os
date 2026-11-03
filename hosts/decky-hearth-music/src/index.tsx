// Hearth Music (docs/HEARTH-MUSIC.md): plays the theme song of the game you're looking at in Steam Game Mode's
// library, crossfades between games, fades out when you press Play and stays silent while a game runs. The song is
// one you picked (a file, or one found with "Choose music…"), the game's own soundtrack, or Apple Music's or
// Deezer's official 30-second preview (credited here). Never YouTube. Nothing of Steam's screens is changed.
import { ButtonItem, Navigation, PanelSection, PanelSectionRow, SliderField, TextField, ToggleField, staticClasses } from "@decky/ui";
import { FileSelectionType, callable, definePlugin, openFilePicker } from "@decky/api";
import { useEffect, useState } from "react";
import { FaMusic } from "react-icons/fa";
import { Choice, Scan, choiceLine, credit, nowLine, openLabel, scanLines, silentWhy } from "./model";
import { HearthOnly, hearthOs } from "./hearth-only";
import { State, begin, chooseMusic, onChange, refresh, scan, scanStatus, searchMusic, setPick, state, updateSettings } from "./music";

const getHome = callable<[], string>("get_home");
const EXTS = ["mp3", "ogg", "opus", "flac", "m4a", "aac", "wav"];

function openLink(url: string) {
  try { (Navigation as any).NavigateToExternalWeb(url); } catch { /* Steam's browser isn't reachable */ }
}

/** "Choose music…": searches your soundtracks, Apple Music and Deezer, and keeps the one you choose as this game's pick. */
function Chooser({ appid, title, done }: { appid: number; title: string; done: (msg: string) => void }) {
  const [q, setQ] = useState(title ? `${title} soundtrack` : "");
  const [res, setRes] = useState<Choice[] | null>(null);
  const [note, setNote] = useState("");
  const go = async (query: string) => {
    setNote("Searching…");
    const r = await searchMusic(query).catch(() => null);
    setRes(r?.results || []);
    const later = (r?.later || []).map((x) => (x === "apple" ? "Apple Music" : "Deezer")).join(" and ");
    setNote(!r ? "Couldn't search right now." : later ? `${later} couldn't be asked right now; try again in a minute.` : r.results.length ? "" : "Nothing found: try other words.");
  };
  useEffect(() => { if (q) go(q); }, []);
  const choose = async (c: Choice) => {
    const ok = await chooseMusic(appid, c.choice).catch(() => false);
    done(ok ? `Chosen for ${title || "this game"}: ${c.title}.` : "That couldn't be kept; search again.");
    if (ok) refresh(appid);
  };
  return (
    <>
      <PanelSectionRow>
        <TextField label="Search" value={q} onChange={(e: any) => setQ(String(e?.target?.value ?? ""))} />
      </PanelSectionRow>
      <PanelSectionRow><ButtonItem layout="below" onClick={() => go(q)}>Search</ButtonItem></PanelSectionRow>
      {note && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{note}</div></PanelSectionRow>}
      {(res || []).slice(0, 20).map((c) => (
        <PanelSectionRow key={c.choice}>
          <ButtonItem layout="below" onClick={() => choose(c)} description={c.album || undefined}>{choiceLine(c)}</ButtonItem>
        </PanelSectionRow>
      ))}
      <PanelSectionRow><ButtonItem layout="below" onClick={() => done("")}>Close</ButtonItem></PanelSectionRow>
    </>
  );
}

function Panel() {
  const [s, setS] = useState<State>({ ...state });
  const [msg, setMsg] = useState("");
  const [sc, setSc] = useState<Scan | null>(null);
  const [choosing, setChoosing] = useState(0);
  useEffect(() => onChange(() => setS({ ...state })) as any, []);
  useEffect(() => {
    const get = () => scanStatus().then(setSc).catch(() => {});
    get();
    const iv = setInterval(get, 2000);
    return () => clearInterval(iv);
  }, []);
  const [scanCount, scanNote] = scanLines(sc);
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
          {openLabel(s.song) && <PanelSectionRow><ButtonItem layout="below" onClick={() => openLink(s.song!.link!)}>{openLabel(s.song)}</ButtonItem></PanelSectionRow>}
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
          <ToggleField label="Use Deezer previews" description="When Apple Music has no good match: Deezer's 30-second preview (streamed, never stored)"
            checked={st.deezer} disabled={!st.enabled} onChange={(v: boolean) => updateSettings({ deezer: v })} />
        </PanelSectionRow>
        <PanelSectionRow>
          <ToggleField label="Now playing note" description="A short note when a new song starts" checked={st.toast} disabled={!st.enabled}
            onChange={(v: boolean) => updateSettings({ toast: v })} />
        </PanelSectionRow>
      </PanelSection>
      {s.appid > 0 && <PanelSection title={`Song for ${s.title || "this game"}`}>
        {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{msg}</div></PanelSectionRow>}
        {choosing === s.appid ? <Chooser appid={s.appid} title={s.title} done={(m) => { setChoosing(0); setMsg(m); }} /> : <PanelSectionRow>
          <ButtonItem layout="below" onClick={() => { setMsg(""); setChoosing(s.appid); }}
            description="Search your soundtracks, Apple Music and Deezer; your choice always wins over the automatic song">Choose music…</ButtonItem>
        </PanelSectionRow>}
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
      <PanelSection title="Your library">
        <PanelSectionRow><div className={staticClasses.Text} style={{ fontWeight: 700 }}>{scanCount}</div></PanelSectionRow>
        {scanNote && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12, color: "#8d99a8" }}>{scanNote}</div></PanelSectionRow>}
        <PanelSectionRow>
          <ButtonItem layout="below" disabled={!!sc?.running} onClick={() => { scan(true).then(() => scanStatus().then(setSc)).catch(() => {}); }}
            description="Finds a song for every game in the background, so new games start right away; your picks are kept">Scan my library</ButtonItem>
        </PanelSectionRow>
      </PanelSection>
    </>
  );
}

export default definePlugin(() => {
  // Only on Hearth OS (hearth-only.tsx); anywhere else nothing starts.
  let end: () => void = () => {};
  let gone = false;
  hearthOs().then((ok) => { if (ok && !gone) end = begin(); });
  return {
    name: "Hearth Music",
    titleView: <div className={staticClasses.Title}>Hearth Music</div>,
    content: <HearthOnly name="Hearth Music" what="Your games' soundtracks while you browse your library."><Panel /></HearthOnly>,
    icon: <FaMusic />,
    onDismount() { gone = true; end(); },
  };
});
