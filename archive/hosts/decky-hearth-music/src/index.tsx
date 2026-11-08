// Hearth Music (docs/HEARTH-MUSIC.md): plays the theme song of the game you're looking at in Steam Game Mode's
// library, crossfades between games, fades out when you press Play and stays silent while a game runs. The song is
// one you picked (a file, or one found with "Choose music…"), the game's own soundtrack, or Apple Music's or
// Deezer's official 30-second preview (credited here). Never YouTube. Nothing of Steam's screens is changed.
import { ButtonItem, DialogButton, Focusable, Navigation, PanelSection, PanelSectionRow, SliderField, TextField, ToggleField, staticClasses } from "@decky/ui";
import { FileSelectionType, callable, definePlugin, openFilePicker } from "@decky/api";
import { useEffect, useState } from "react";
import { FaMusic } from "react-icons/fa";
import { Choice, ReviewRow, Scan, choiceLine, credit, grouped, nowLine, openLabel, reviewLine, scanLines, silentWhy } from "./model";
import { HearthOnly, hearthOs } from "./hearth-only";
import { State, begin, chooseMusic, cleanTitle, forgetSongs, noMusic, onChange, preview, refresh, reviewList, scanAll, scanCancel, scanStatus,
  searchMusic, setPick, state, stopPreview, updateSettings } from "./music";

const getHome = callable<[], string>("get_home");
const EXTS = ["mp3", "ogg", "opus", "flac", "m4a", "aac", "wav"];

function openLink(url: string) {
  try { (Navigation as any).NavigateToExternalWeb(url); } catch { /* Steam's browser isn't reachable */ }
}

const small = { fontSize: 12, color: "#8d99a8" };
const rowStyle = { display: "flex", alignItems: "center", gap: 6, width: "100%" };
const btnStyle = { minWidth: 0, width: "auto", padding: "6px 10px", flex: "0 0 auto" };

/** A row of the review list or of "Choose music…": two lines of text and its buttons, moved through with the d-pad. */
function Row({ top, line, children }: { top: string; line: string; children: any }) {
  return (
    <PanelSectionRow>
      <Focusable style={rowStyle} {...({ "flow-children": "horizontal" } as any)}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div className={staticClasses.Text} style={{ fontSize: 13.5, fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{top}</div>
          <div className={staticClasses.Text} style={{ ...small, overflow: "hidden", textOverflow: "ellipsis" }}>{line}</div>
        </div>
        {children}
      </Focusable>
    </PanelSectionRow>
  );
}

/** ▶ / ■: hear a song before keeping it; pressing it again (or another ▶) stops it. */
function Hear({ id, what }: { id: string; what: { choice?: string; appid?: number; title?: string } }) {
  const on = state.preview === id;
  return <DialogButton style={btnStyle} onClick={() => (on ? stopPreview() : preview(id, what))}>{on ? "■ Stop" : "▶"}</DialogButton>;
}

/** "Choose music…": searches your soundtracks, Apple Music and Deezer (the search box starts with the game's name
 *  cleaned of ™/®, years and editions), lets you hear each, and keeps the one you choose as this game's pick. */
function Chooser({ appid, title, query, done }: { appid: number; title: string; query?: string; done: (msg: string) => void }) {
  const [q, setQ] = useState(query ?? "");
  const [res, setRes] = useState<Choice[] | null>(null);
  const [note, setNote] = useState("");
  const go = async (query: string) => {
    setNote("Searching…");
    const r = await searchMusic(query).catch(() => null);
    setRes(r?.results || []);
    const later = (r?.later || []).map((x) => (x === "apple" ? "Apple Music" : "Deezer")).join(" and ");
    setNote(!r ? "Couldn't search right now." : later ? `${later} couldn't be asked right now; try again in a minute.` : r.results.length ? "" : "Nothing found: try other words.");
  };
  useEffect(() => {
    if (query !== undefined) { if (query) go(query); return; }
    cleanTitle(title).catch(() => title).then((t) => { const v = String(t || title || ""); setQ(v); if (v) go(v); });
  }, []);
  const choose = async (c: Choice) => {
    stopPreview();
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
        <Row key={c.choice} top={choiceLine(c)} line={c.album || ""}>
          <Hear id={"choice:" + c.choice} what={{ choice: c.choice }} />
          <DialogButton style={btnStyle} onClick={() => choose(c)}>Choose</DialogButton>
        </Row>
      ))}
      <PanelSectionRow><ButtonItem layout="below" onClick={() => { stopPreview(); done(""); }}>Close</ButtonItem></PanelSectionRow>
    </>
  );
}

/** "Scan all games" and its progress (n of N), with Cancel while it runs. */
function ScanControls({ sc, setSc }: { sc: Scan | null; setSc: (s: Scan) => void }) {
  const [count, note] = scanLines(sc);
  const all = !!sc?.running && !!sc?.all;
  return (
    <>
      <PanelSectionRow><div className={staticClasses.Text} style={{ fontWeight: 700 }}>{count}</div></PanelSectionRow>
      {note && <PanelSectionRow><div className={staticClasses.Text} style={small}>{note}</div></PanelSectionRow>}
      {all && sc!.total > 0 && <PanelSectionRow>
        <div style={{ height: 4, borderRadius: 2, background: "#273140" }}><div style={{ height: 4, borderRadius: 2, background: "#1a9fff", width: `${Math.round((sc!.done / sc!.total) * 100)}%` }} /></div>
      </PanelSectionRow>}
      <PanelSectionRow>
        {all ? <ButtonItem layout="below" onClick={() => { scanCancel().then(() => scanStatus().then(setSc)).catch(() => {}); }}
          description="Stops where it is; every song it found so far is kept">Cancel scan</ButtonItem>
          : <ButtonItem layout="below" onClick={() => { scanAll().then(() => scanStatus().then(setSc)).catch(() => {}); }}
            description="Checks every installed game and non-Steam game again with today's matching; your picks are never changed">Scan all games</ButtonItem>}
      </PanelSectionRow>
    </>
  );
}

/** One game of the review list, to fix: choose its music, no music, or back to automatic. */
function Fix({ row, done }: { row: ReviewRow; done: (msg: string) => void }) {
  const appid = Number(row.appid);
  const [choosing, setChoosing] = useState(false);
  const none = async () => { stopPreview(); const ok = await noMusic(appid).catch(() => false); refresh(appid); done(ok ? `${row.title}: no music, as you chose.` : "That couldn't be kept."); };
  const auto = async () => { stopPreview(); await setPick("app", row.appid, "").catch(() => false); refresh(appid); done(`${row.title}: back to the automatic song.`); };
  return (
    <PanelSection title={row.title}>
      <Row top="Now" line={reviewLine(row)}>{row.song && row.song.from !== "none" ? <Hear id={"game:" + row.appid} what={{ appid, title: row.title }} /> : null}</Row>
      {choosing ? <Chooser appid={appid} title={row.title} query={row.query} done={(m) => { setChoosing(false); if (m) { forgetSongs(); done(m); } }} /> : <>
        <PanelSectionRow><ButtonItem layout="below" onClick={() => setChoosing(true)}
          description="Search Apple Music, Deezer and your soundtracks, hear them, and keep the right one">Choose music…</ButtonItem></PanelSectionRow>
        {!(row.group === "pick" && row.song?.from === "none") && <PanelSectionRow>
          <ButtonItem layout="below" onClick={none} description="Kept as your pick: this game stays silent">No music for this game</ButtonItem></PanelSectionRow>}
        {row.group === "pick" && <PanelSectionRow>
          <ButtonItem layout="below" onClick={auto} description="Forget your pick; Hearth Music finds the song again">Back to automatic</ButtonItem></PanelSectionRow>}
        <PanelSectionRow><ButtonItem layout="below" onClick={() => { stopPreview(); done(""); }}>Back to the list</ButtonItem></PanelSectionRow>
      </>}
    </PanelSection>
  );
}

/** Music for every game: the review list in its groups (No music found, Found automatically, Your picks). */
function Review({ sc, setSc, close }: { sc: Scan | null; setSc: (s: Scan) => void; close: () => void }) {
  const [rows, setRows] = useState<ReviewRow[] | null>(null);
  const [fix, setFix] = useState<ReviewRow | null>(null);
  const [msg, setMsg] = useState("");
  const load = () => { reviewList().then(setRows).catch(() => setRows([])); };
  useEffect(load, [sc?.done, sc?.running]);
  if (fix) return <Fix row={fix} done={(m) => { setFix(null); setMsg(m); load(); }} />;
  return (
    <>
      <PanelSection title="Music for every game">
        {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{msg}</div></PanelSectionRow>}
        <ScanControls sc={sc} setSc={setSc} />
        <PanelSectionRow><ButtonItem layout="below" onClick={() => { stopPreview(); close(); }}>Back</ButtonItem></PanelSectionRow>
      </PanelSection>
      {rows === null ? <PanelSection><PanelSectionRow><div className={staticClasses.Text}>Reading your library…</div></PanelSectionRow></PanelSection>
        : grouped(rows).map(([g, label, list]) => (
          <PanelSection key={g} title={`${label} (${list.length})`}>
            {!list.length && <PanelSectionRow><div className={staticClasses.Text} style={small}>None</div></PanelSectionRow>}
            {list.map((r) => (
              <Row key={r.appid} top={r.title} line={reviewLine(r)}>
                {r.song && r.song.from !== "none" ? <Hear id={"game:" + r.appid} what={{ appid: Number(r.appid), title: r.title }} /> : null}
                <DialogButton style={btnStyle} onClick={() => { stopPreview(); setMsg(""); setFix(r); }}>Fix</DialogButton>
              </Row>
            ))}
          </PanelSection>
        ))}
    </>
  );
}

function Panel() {
  const [s, setS] = useState<State>({ ...state });
  const [msg, setMsg] = useState("");
  const [sc, setSc] = useState<Scan | null>(null);
  const [choosing, setChoosing] = useState(0);
  const [view, setView] = useState<"main" | "review">("main");
  useEffect(() => onChange(() => setS({ ...state })) as any, []);
  useEffect(() => {
    const get = () => scanStatus().then(setSc).catch(() => {});
    get();
    const iv = setInterval(get, 2000);
    return () => clearInterval(iv);
  }, []);
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
  if (view === "review") return <Review sc={sc} setSc={setSc} close={() => setView("main")} />;
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
        {((s.song?.source === "pick" && s.song.via !== "collection") || s.song?.pick === "none") && <PanelSectionRow>
          <ButtonItem layout="below" onClick={unpick}>Use the automatic song</ButtonItem>
        </PanelSectionRow>}
      </PanelSection>}
      <PanelSection title="Your library">
        <ScanControls sc={sc} setSc={setSc} />
        <PanelSectionRow>
          <ButtonItem layout="below" onClick={() => setView("review")}
            description="Every game with its song: fix any that's wrong, choose one, or choose no music">Review music for every game</ButtonItem>
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
