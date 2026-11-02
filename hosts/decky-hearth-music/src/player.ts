// Plays the songs in Steam's own browser (the plugin's page, an HTML audio element), so the sound goes out with
// Steam's and follows Steam Game Mode's volume. Two elements crossfade; each song loops until the next one or silence.
import { CROSSFADE, clamp, fadeAt } from "./model";

interface Track { el: HTMLAudioElement; url: string; key: string; vol: number; timer: any }

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
  try { URL.revokeObjectURL(t.url); } catch { /* gone */ }
}

export class Player {
  cur: Track | null = null;
  volume = 0.35;

  playing(): string { return this.cur?.key || ""; }

  /** Starts a song (crossfading from the one playing). Throws "unsupported" when Steam's browser can't decode it. */
  async play(key: string, data: { data: string; mime: string }, fadeMs = CROSSFADE): Promise<void> {
    const el = new Audio();
    const t: Track = { el, url: blobUrl(data.data, data.mime), key, vol: 0, timer: null };
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
  }

  /** Fades out to silence. */
  stop(fadeMs = CROSSFADE) {
    const old = this.cur;
    this.cur = null;
    if (old) ramp(old, 0, fadeMs, () => drop(old));
  }

  setVolume(v: number) {
    this.volume = clamp(v);
    if (this.cur) ramp(this.cur, this.volume, 250);
  }
}
