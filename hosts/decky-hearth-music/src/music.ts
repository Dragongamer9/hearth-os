// Hearth Music's loop: twice a second it reads what Steam shows (steam.ts), decides which game's song should play
// (model.ts wanted), waits the delay you set, asks the backend for the song and plays it (player.ts). Play pressed:
// a quick fade out, then silence while the game runs.
import { callable, toaster } from "@decky/api";
import { DEFAULTS, FADE_OUT_PLAY, Seen, Settings, Song, cleanSettings, credit, nowLine, wanted } from "./model";
import { Player } from "./player";
import { collectionsOf, gameTitle, readPath, readRunning, steamMissing, watchFocus, watchPlay } from "./steam";

const findSong = callable<[appid: number, title: string, collections: string[]], Song>("find_song");
const getAudio = callable<[key: string, transcode: boolean], { data: string; mime: string } | null>("audio");
export const getSettings = callable<[], Settings>("get_settings");
const saveSettings = callable<[s: Settings], Settings>("set_settings");
export const setPick = callable<[kind: string, name: string, path: string], boolean>("set_pick");
const report = callable<[line: string], void>("report");

export interface State { settings: Settings; seen: Seen; appid: number; title: string; song: Song | null; off: string; collections: string[] }

const player = new Player();
export const state: State = { settings: { ...DEFAULTS }, seen: { path: "", focused: 0, running: 0, launching: false }, appid: 0, title: "", song: null, off: "", collections: [] };
const listeners = new Set<() => void>();
export const onChange = (f: () => void) => { listeners.add(f); return () => listeners.delete(f); };
const changed = () => listeners.forEach((f) => { try { f(); } catch { /* panel gone */ } });

const songs = new Map<number, { song: Song; at: number }>();   // this session's answers per game
const toasted = new Set<number>();
let want = 0, timer: any = null, launchUntil = 0, seq = 0;

export async function updateSettings(p: Partial<Settings>) {
  state.settings = cleanSettings(await saveSettings(cleanSettings({ ...state.settings, ...p })).catch(() => ({ ...state.settings, ...p })));
  player.setVolume(state.settings.volume);
  if (!state.settings.enabled) player.stop();
  changed();
}

/** Forgets this game's song (after a new pick) and plays the right one again. */
export function refresh(appid: number) { songs.delete(appid); if (want === appid) { want = -1; } tick(); }

async function start(appid: number, my: number) {
  try {
    let hit = songs.get(appid);
    if (!hit || (hit.song.later && Date.now() - hit.at > 20000)) {
      const title = gameTitle(appid);
      hit = { song: await findSong(appid, title, collectionsOf(appid)), at: Date.now() };
      songs.set(appid, hit);
    }
    if (my !== seq) return;
    const song = hit.song;
    state.song = song;
    changed();
    if (!song?.source || !song.key) { player.stop(); return; }
    if (player.playing() === song.key) return;
    let data = await getAudio(song.key, false);
    if (my !== seq || !data) return;
    try { await player.play(song.key, data); } catch (e: any) {
      if (String(e?.message) !== "unsupported") throw e;
      data = await getAudio(song.key, true);   // a format Steam's browser can't play: made into Ogg Opus by the backend
      if (my !== seq || !data) return;
      await player.play(song.key, data);
    }
    if (state.settings.toast && !toasted.has(appid)) {
      toasted.add(appid);
      try { toaster.toast({ title: "Now playing", body: [nowLine(song), credit(song)].filter(Boolean).join("\n"), duration: 4000 } as any); } catch { /* no toasts */ }
    }
  } catch (e) {
    report(`play failed: ${String((e as any)?.message || e).slice(0, 80)}`).catch(() => {});
  }
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
  getSettings().then((s) => { state.settings = cleanSettings(s); player.setVolume(state.settings.volume); changed(); }).catch(() => {});
  const onFocus = (id: number) => { state.seen = { ...state.seen, focused: id }; tick(); };
  let offFocus = watchFocus(onFocus);
  // Steam's window may come a moment after the plugin: tried again until it's there.
  const retry = setInterval(() => { if (!offFocus) offFocus = watchFocus(onFocus); if (offFocus) clearInterval(retry); }, 5000);
  const offPlay = watchPlay(() => { launchUntil = Date.now() + 30000; player.stop(FADE_OUT_PLAY); tick(); });
  const iv = setInterval(tick, 500);
  return () => { clearInterval(iv); clearInterval(retry); clearTimeout(timer); offFocus?.(); offPlay(); player.stop(300); };
}
