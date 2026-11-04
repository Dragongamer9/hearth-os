// Two of the extras, each off until the person turns it on: the eras' sounds (made from plain waves as they play, from
// model.ts soundRecipe; no recordings) and quick switching (hold L1: D-pad left/right changes the era, L2/R2 the preset).
import type { EraId, QuickAction, SoundKind } from "./model";
import { quickAction, soundRecipe } from "./model";

// ---- sounds

let soundEra: EraId | null = null;
let actx: AudioContext | null = null;

/** The era whose sounds play on its home (null: none). */
export function setEraSounds(era: EraId | null) {
  soundEra = era;
  if (!era && actx) { try { actx.close(); } catch { /* fine */ } actx = null; }
}
export function soundsOn(): boolean { return soundEra !== null; }

export function play(kind: SoundKind) {
  if (!soundEra) return;
  try {
    const AC = (window as any).AudioContext || (window as any).webkitAudioContext;
    if (!AC) return;
    if (!actx || actx.state === "closed") actx = new AC() as AudioContext;
    const ctx = actx;
    if (ctx.state !== "running") ctx.resume();
    for (const t of soundRecipe(soundEra, kind)) {
      const start = ctx.currentTime + (t.at || 0);
      const g = ctx.createGain();
      g.gain.setValueAtTime(t.v, start);
      g.gain.exponentialRampToValueAtTime(0.0001, start + t.d);
      g.connect(ctx.destination);
      if (t.noise) {
        const n = Math.max(1, Math.floor(ctx.sampleRate * t.d));
        const buf = ctx.createBuffer(1, n, ctx.sampleRate);
        const ch = buf.getChannelData(0);
        for (let i = 0; i < n; i++) ch[i] = Math.random() * 2 - 1;
        const src = ctx.createBufferSource();
        src.buffer = buf;
        const f = ctx.createBiquadFilter();
        f.type = "bandpass"; f.frequency.value = t.noise; f.Q.value = 1.2;
        src.connect(f).connect(g);
        src.start(start); src.stop(start + t.d + 0.01);
      } else {
        const o = ctx.createOscillator();
        o.type = t.wave || "sine";
        o.frequency.setValueAtTime(t.f, start);
        if (t.to) o.frequency.exponentialRampToValueAtTime(t.to, start + t.d);
        o.connect(g);
        o.start(start); o.stop(start + t.d + 0.01);
      }
    }
  } catch { /* no sound, nothing else */ }
}

// ---- quick switching

export interface QuickState { on: boolean; how: string; last: string }
export const quick: QuickState = { on: false, how: "off", last: "" };

/** Listens to Steam's controller state (SteamClient.Input) and calls `act` for each quick action, except while a game
 *  runs (the buttons are the game's then). Returns the way to stop. Where Steam has no such call, nothing listens. */
export function startQuickSwitch(act: (a: QuickAction) => void, inGame: () => boolean | null): () => void {
  const prev = new Map<number, number>();
  let handle: any = null;
  try {
    const input = (window as any).SteamClient?.Input;
    const reg = input?.RegisterForControllerStateChanges;
    if (typeof reg === "function") {
      handle = reg.call(input, (changes: any[]) => {
        try {
          for (const c of Array.isArray(changes) ? changes : [changes]) {
            const id = Number(c?.unControllerIndex ?? 0);
            const now = Number(c?.ulButtons ?? 0);
            const a = quickAction(prev.get(id) || 0, now);
            prev.set(id, now);
            if (a && inGame() !== true) { quick.last = a; act(a); }
          }
        } catch { /* a state Steam changed: ignored */ }
      });
      quick.on = true; quick.how = "SteamClient.Input.RegisterForControllerStateChanges";
    } else {
      quick.on = false; quick.how = "this Steam has no controller state call";
    }
  } catch (e) {
    quick.on = false; quick.how = "couldn't listen: " + String(e).slice(0, 120);
  }
  return () => {
    try { handle?.unregister?.(); } catch { /* gone */ }
    quick.on = false; quick.how = "off";
  };
}
