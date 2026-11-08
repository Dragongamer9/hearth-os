// Plays the songs in Steam's own browser (the plugin's page, an HTML audio element), so the sound goes out with
// Steam's and follows Steam Game Mode's volume. Two elements crossfade; each song loops until the next one or silence.
// Apple's previews are streamed from their https address (playUrl); files come from the backend as bytes (play).
// A YouTube video plays in YouTube's own mini player (video.tsx, playYoutube) instead, faded the same way.
import { CROSSFADE, clamp, fadeAt } from "./model";
import { video } from "./video";

interface Track { el: HTMLAudioElement; url: string; blob: boolean; key: string; vol: number; timer: any }

function blobUrl(b64: string, mime: string): string {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return URL.createObjectURL(new Blob([bytes], { type: mime }));
}

function ramp(t: Track, to: number, ms: number, done?: () => void) {
  clearInterval(t.timer);
  const from = t.vol, start = Date.now();
  t.timer = setInterval(() => {
    const el = Date.now() - start;
    t.vol = fadeAt(from, to, el, ms);
    try { t.el.volume = t.vol; } catch { /* gone */ }
    if (el >= ms) { clearInterval(t.timer); done?.(); }
  }, 40);
}

function drop(t: Track) {
  clearInterval(t.timer);
  try { t.el.pause(); t.el.removeAttribute("src"); t.el.load(); } catch { /* gone */ }
  if (t.blob) { try { URL.revokeObjectURL(t.url); } catch { /* gone */ } }
}

export class Player {
  cur: Track | null = null;
  volume = 0.35;

  playing(): string { return this.cur?.key || video.playing(); }

  /** Shows a YouTube video in the mini player (YouTube's own embed) and fades out the song playing; throws when it
   *  can't play there (the caller tells the log; nothing else is tried for YouTube). */
  async playYoutube(key: string, id: string, fadeMs = CROSSFADE): Promise<void> {
    await video.play(key, id, this.volume, fadeMs);
    const old = this.cur;
    this.cur = null;
    if (old) ramp(old, 0, fadeMs, () => drop(old));
  }

  /** Starts a song (crossfading from the one playing). Throws "unsupported" when Steam's browser can't decode it. */
  async play(key: string, data: { data: string; mime: string }, fadeMs = CROSSFADE): Promise<void> {
    return this.start({ el: new Audio(), url: blobUrl(data.data, data.mime), blob: true, key, vol: 0, timer: null }, fadeMs);
  }

  /** Streams a song from its https address (Apple's previews: played, never stored). Throws when Steam's browser
   *  won't load or play it (its page's rules, the network), so the caller can go through the backend instead. */
  async playUrl(key: string, url: string, fadeMs = CROSSFADE): Promise<void> {
    const t: Track = { el: new Audio(), url, blob: false, key, vol: 0, timer: null };
    t.el.preload = "auto";
    return this.start(t, fadeMs);
  }

  private async start(t: Track, fadeMs: number): Promise<void> {
    const el = t.el;
    el.loop = true;
    el.volume = 0;
    el.src = t.url;
    try { await el.play(); } catch (e: any) {
      drop(t);
      throw new Error(e?.name === "NotSupportedError" || el.error?.code === 4 ? "unsupported" : String(e?.name || e));
    }
    const old = this.cur;
    this.cur = t;
    ramp(t, this.volume, fadeMs);
    if (old) ramp(old, 0, fadeMs, () => drop(old));
    video.stop(fadeMs);
  }

  /** Fades out to silence. */
  stop(fadeMs = CROSSFADE) {
    const old = this.cur;
    this.cur = null;
    if (old) ramp(old, 0, fadeMs, () => drop(old));
    video.stop(fadeMs);
  }

  setVolume(v: number) {
    this.volume = clamp(v);
    if (this.cur) ramp(this.cur, this.volume, 250);
    video.setVolume(this.volume);
  }
}
