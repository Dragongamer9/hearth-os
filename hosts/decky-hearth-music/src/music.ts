// Hearth Music's loop: twice a second it reads what Steam shows (steam.ts), decides which game's song should play
// (model.ts wanted), waits the delay you set, asks the backend for the song and plays it (player.ts). Play pressed:
// a quick fade out, then silence while the game runs.
import { callable, toaster } from "@decky/api";
import { Choice, DEFAULTS, FADE_OUT_PLAY, ReviewRow, Scan, Seen, Settings, Song, YouTubeStatus, cleanSettings, credit, nowLine, wanted } from "./model";
import { Player } from "./player";
import { video } from "./video";
import { collectionsOf, gameTitle, libraryGames, readPath, readRunning, steamMissing, watchFocus, watchPlay } from "./steam";

const findSong = callable<[appid: number, title: string, collections: string[]], Song>("find_song");
const getAudio = callable<[key: string, transcode: boolean], { data: string; mime: string } | null>("audio");
export const getSettings = callable<[], Settings>("get_settings");
const saveSettings = callable<[s: Partial<Settings>], Settings>("set_settings");   // only the keys changed
export const setPick = callable<[kind: string, name: string, path: string], boolean>("set_pick");
const report = callable<[line: string], void>("report");
const scanLibrary = callable<[games: { appid: number; title: string; shortcut: boolean }[], force: boolean], boolean>("scan_library");
const scanPause = callable<[paused: boolean], boolean>("scan_pause");
export const scanStatus = callable<[], Scan>("scan_status");
export const searchMusic = callable<[query: string], { results: Choice[]; later: string[]; note?: string }>("search_music");
export const youtubeStatus = callable<[], YouTubeStatus>("youtube_status");
/** The person's YouTube API key goes to the backend once (kept there in a 0600 file); it never comes back. */
export const setYoutubeKey = callable<[key: string], YouTubeStatus>("set_youtube_key");
export const chooseMusic = callable<[appid: number, choice: string], boolean>("choose_music");
const scanAllGames = callable<[games: { appid: number; title: string; shortcut: boolean }[]], boolean>("scan_all");
export const scanCancel = callable<[], boolean>("scan_cancel");
const reviewGames = callable<[games: { appid: number; title: string; shortcut: boolean }[]], ReviewRow[]>("review");
const choicePreview = callable<[choice: string], { key: string; url?: string; youtube?: string } | null>("choice_preview");
export const noMusic = callable<[appid: number], boolean>("no_music");
export const cleanTitle = callable<[title: string], string>("clean_title");

/** "Scan all games": every installed game and shortcut looked at again with today's matcher (your picks kept). */
export async function scanAll(): Promise<boolean> { return scanAllGames(libraryGames()).catch(() => false); }

/** The review list: every installed game and shortcut, with what it plays. */
export async function reviewList(): Promise<ReviewRow[]> { return reviewGames(libraryGames()).catch(() => []); }

/** Hands Steam's library to the backend's scan: the first time it goes through every game, then new installs and
 *  week-old answers only (the backend skips what it knows). force: "Scan my library". */
export async function scan(force = false): Promise<boolean> {
  const games = libraryGames();
  if (!games.length && !force) return false;
  return scanLibrary(games, force).catch(() => false);
}

export interface State { settings: Settings; seen: Seen; appid: number; title: string; song: Song | null; off: string; collections: string[]; preview: string }

const player = new Player();
export const state: State = { settings: { ...DEFAULTS }, seen: { path: "", focused: 0, running: 0, launching: false }, appid: 0, title: "", song: null, off: "", collections: [], preview: "" };
const listeners = new Set<() => void>();
export const onChange = (f: () => void) => { listeners.add(f); return () => listeners.delete(f); };
const changed = () => listeners.forEach((f) => { try { f(); } catch { /* panel gone */ } });

// This session's answers per game, kept for a few minutes only: a scan, "Scan all games" or a newer matcher may find a
// better answer meanwhile, and the backend (lookups.json, picks.json) is the one place answers live.
const songs = new Map<number, { song: Song; at: number }>();
const SONG_FOR = 5 * 60 * 1000;
/** Forgets every answer of this session (after a scan finished or a fix in the review list). */
export function forgetSongs() { songs.clear(); if (want > 0) { want = -1; tick(); } }
const toasted = new Set<number>();
let want = 0, timer: any = null, launchUntil = 0, seq = 0;

export async function updateSettings(p: Partial<Settings>) {
  // Only what changed goes to the backend, merged into the latest file (another device's change through Hearth Link's
  // "Sync Hearth settings" is never written back over); it answers with every setting as it is now.
  state.settings = cleanSettings({ ...state.settings, ...p, ...(await saveSettings(p).catch(() => ({}))) });
  player.setVolume(state.settings.volume);
  if (!state.settings.enabled) player.stop();
  changed();
}

/** Forgets this game's song (after a new pick) and plays the right one again. */
export function refresh(appid: number) { songs.delete(appid); if (want === appid) { want = -1; } tick(); }

async function start(appid: number, my: number) {
  try {
    let hit = songs.get(appid);
    if (!hit || Date.now() - hit.at > SONG_FOR || (hit.song.later && Date.now() - hit.at > 20000)) {
      const title = gameTitle(appid);
      hit = { song: await findSong(appid, title, collectionsOf(appid)), at: Date.now() };
      songs.set(appid, hit);
    }
    if (my !== seq) return;
    const song = hit.song;
    state.song = song;
    changed();
    if (state.preview) return;   // you're hearing a song in the review list: it plays until you stop it
    if (!song?.source || !song.key) { player.stop(); return; }
    if (player.playing() === song.key) return;
    if (!(await playKey(song.key, song.url, () => my === seq, song.youtube))) {
      if (song.youtube && my === seq) { state.song = { ...song, source: null, why: "YouTube's player couldn't play it here" }; changed(); }
      return;
    }
    if (state.settings.toast && !toasted.has(appid)) {
      toasted.add(appid);
      try { toaster.toast({ title: "Now playing", body: [nowLine(song), credit(song)].filter(Boolean).join("\n"), duration: 4000 } as any); } catch { /* no toasts */ }
    }
  } catch (e) {
    report(`play failed: ${String((e as any)?.message || e).slice(0, 80)}`).catch(() => {});
  }
}

/** Plays a song the backend chose (its key, and an https address to stream when there is one); false when it went
 *  stale (still() turned false) or couldn't be had. */
async function playKey(key: string, url: string | undefined, still: () => boolean, youtube?: string): Promise<boolean> {
  if (youtube) {
    // YouTube's own player in the mini player (video.tsx): streamed by YouTube, nothing passes through the backend.
    try { await player.playYoutube(key, youtube); } catch (e: any) {
      if (still()) {
        report(`YouTube mini player: ${String(e?.message || e).slice(0, 120)}`).catch(() => {});
        player.stop(400);   // never the previous game's song over this one
      }
      return false;
    }
    return still();
  }
  let streamed = false;
  if (url) {
    // Apple's or Deezer's preview: streamed straight from its address. If Steam's browser refuses it, the backend
    // passes it on from memory below (never stored).
    try { await player.playUrl(key, url); streamed = true; } catch (e: any) {
      report(`preview stream refused (${String(e?.message || e).slice(0, 40)}): through the backend`).catch(() => {});
    }
    if (!still()) return false;
  }
  if (!streamed) {
    let data = await getAudio(key, false);
    if (!still() || !data) return false;
    try { await player.play(key, data); } catch (e: any) {
      if (String(e?.message) !== "unsupported") throw e;
      data = await getAudio(key, true);   // a format Steam's browser can't play: made into Ogg Opus by the backend
      if (!still() || !data) return false;
      await player.play(key, data);
    }
  }
  return true;
}

let previewSeq = 0;
/** Plays one song to hear it (the review list's ▶, a "Choose music…" result): `id` names what is playing, the game's
 *  own song pauses meanwhile. A choice ("apple:123") or a game of the list ({appid, title}). False when it can't play. */
export async function preview(id: string, what: { choice?: string; appid?: number; title?: string }): Promise<boolean> {
  const my = ++previewSeq;
  state.preview = id;
  changed();
  try {
    let key = "", url: string | undefined, yt: string | undefined;
    if (what.choice) {
      const c = await choicePreview(what.choice);
      if (c?.key) { key = c.key; url = c.url; yt = c.youtube; }
    } else if (what.appid) {
      const s = await findSong(what.appid, what.title || gameTitle(what.appid), collectionsOf(what.appid));
      if (s?.key) { key = s.key; url = s.url; yt = s.youtube; }
    }
    if (my !== previewSeq) return false;
    if (!key) { stopPreview(); return false; }
    return await playKey(key, url, () => my === previewSeq, yt);
  } catch (e) {
    report(`preview failed: ${String((e as any)?.message || e).slice(0, 80)}`).catch(() => {});
    if (my === previewSeq) stopPreview();
    return false;
  }
}

/** Stops the song you were hearing; the game's own song (if any) comes back. */
export function stopPreview() {
  previewSeq++;
  if (!state.preview) return;
  state.preview = "";
  player.stop(400);
  if (want > 0) { const w = want; want = -1; songs.delete(w); }
  changed();
  tick();
}

function tick() {
  if (state.off) return;
  const path = readPath();
  const running = readRunning();
  if (path === null && running === null) {
    state.off = steamMissing() || "Steam changed: Hearth Music turned itself off.";
    player.stop(400);
    report(state.off).catch(() => {});
    changed();
    return;
  }
  const run = running || 0;
  if (run) launchUntil = 0;
  const seen: Seen = { ...state.seen, path: path || "", running: run, launching: Date.now() < launchUntil };
  const busy = run > 0 || seen.launching;
  if (busy !== (state.seen.running > 0 || state.seen.launching)) scanPause(busy).catch(() => {});   // the scan waits while a game runs
  const w = wanted(seen, state.settings);
  const moved = seen.path !== state.seen.path || seen.running !== state.seen.running || seen.launching !== state.seen.launching;
  state.seen = seen;
  if (w === want) { if (moved) changed(); return; }
  const was = want;
  want = w;
  seq++;
  clearTimeout(timer);
  state.appid = w;
  state.title = w ? gameTitle(w) : "";
  state.collections = w ? collectionsOf(w) : [];
  if (!w) {
    player.stop(run || seen.launching ? FADE_OUT_PLAY : undefined);
    state.song = null;
  } else {
    // Crossfading from one game to the next waits the delay too, so flicking through the library doesn't start
    // every song; the one playing carries on meanwhile.
    const my = seq;
    timer = setTimeout(() => start(w, my), was === -1 ? 0 : state.settings.delay * 1000);
  }
  changed();
}

/** Starts the loop; returns the way to stop it. */
export function begin(): () => void {
  // YouTube's player stopped with an error after it had started (the video was removed, embedding turned off...):
  // the mini player closed itself; the log says why.
  video.onFail = (line) => { report(`YouTube mini player: ${line.slice(0, 120)}`).catch(() => {}); };
  getSettings().then((s) => { state.settings = cleanSettings(s); player.setVolume(state.settings.volume); changed(); }).catch(() => {});
  const onFocus = (id: number) => { state.seen = { ...state.seen, focused: id }; tick(); };
  let offFocus = watchFocus(onFocus);
  // Steam's window may come a moment after the plugin: tried again until it's there.
  const retry = setInterval(() => { if (!offFocus) offFocus = watchFocus(onFocus); if (offFocus) clearInterval(retry); }, 5000);
  const offPlay = watchPlay(() => { launchUntil = Date.now() + 30000; player.stop(FADE_OUT_PLAY); tick(); });
  const iv = setInterval(tick, 500);
  // The library scan: as soon as Steam's library can be read, then every 15 minutes for new installs.
  let scanned = false;
  const tryScan = () => { scan(false).then((ok) => { if (ok) scanned = true; }).catch(() => {}); };
  const scanIv = setInterval(() => { if (!scanned) tryScan(); }, 10000);
  const scanLater = setInterval(tryScan, 15 * 60 * 1000);
  const first = setTimeout(tryScan, 3000);
  // When a scan finishes, this session's answers are forgotten, so what it found plays from then on.
  let wasRunning = false;
  const scanWatch = setInterval(() => {
    scanStatus().then((st) => { if (wasRunning && !st?.running) forgetSongs(); wasRunning = !!st?.running; }).catch(() => {});
  }, 15000);
  return () => { clearTimeout(first); clearInterval(scanWatch); previewSeq++; state.preview = ""; clearInterval(iv); clearInterval(retry); clearInterval(scanIv); clearInterval(scanLater); clearTimeout(timer); offFocus?.(); offPlay(); player.stop(300); };
}
