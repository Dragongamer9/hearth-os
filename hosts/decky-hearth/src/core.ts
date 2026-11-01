// Hearth's core service in Steam Game Mode (hearth-core core_service.rs, through main.py): the game Steam is running,
// told to Hearth's core every 2 s (so Hearth Input holds the controller only for a game with a Hearth layout that's in
// front), and the in-game section's calls (the game's layout, Proton, mods), Hearth's screenshot and recorder.
import * as DUI from "@decky/ui";
import { callable } from "@decky/api";
import { useEffect, useState } from "react";
import { GameView, Running, frontMessage, runningGame } from "./model";
import { guard, settings, shortcutMap } from "./state";

export type Answer<T> = T & { error?: string };
const coreStatus = callable<[], { running: boolean; version: string }>("core_status");
const front = callable<[appid: number, game: string, overlay: boolean], any>("hearth_front");
export const gameView = callable<[game: string], Answer<GameView>>("hearth_game");
export const inputSet = callable<[game: string, layout: string], Answer<{ layout: string; name?: string }>>("hearth_input_set");
export const setProton = callable<[game: string, proton: string], Answer<{ proton: string }>>("hearth_set_proton");
export const modsOf = callable<[game: string], Answer<{ count: number; enabled: number; mods: { name: string; enabled: boolean }[]; beacon: boolean }>>("hearth_mods");
export const tool = callable<[what: "screenshot" | "record"], Answer<any>>("hearth_tool");
export const switchToDesktop = callable<[], { ok: boolean; error?: string }>("switch_to_desktop");

/** Steam's running game, as Steam's own UI keeps it (Decky's Router.MainRunningApp), or null. */
export function readRunningApp(): { appid?: number | string; display_name?: string } | null {
  try {
    const r = (DUI as any).Router;
    const a = r?.MainRunningApp ?? (window as any).SteamUIStore?.MainRunningApp;
    return a && a.appid ? { appid: a.appid, display_name: a.display_name } : null;
  } catch { return null; }
}

export function running(): Running | null { return runningGame(readRunningApp(), shortcutMap); }

// ---- Quick Access open (Hearth's tabs say so while they're shown): Hearth Input lets go meanwhile.
const shown = new Set<string>();
export function markShown(key: string, on: boolean) { if (on) shown.add(key); else shown.delete(key); }

// ---- the heartbeat
let timer: any = null;
let lastFront: any = null;
let sentNone = false;
let coreUp: boolean | null = null;
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((f) => { try { f(); } catch { /* gone */ } });

async function tick() {
  const run = running();
  const msg = frontMessage(run, settings, guard.broken, shown.size > 0);
  if (!msg.game && sentNone && !run) return;   // nothing running, already said so
  sentNone = !msg.game;
  try {
    const r = await front(msg.appid, msg.game, msg.overlay);
    const up = !(r && r.error);
    if (up !== coreUp) { coreUp = up; notify(); }
    const was = JSON.stringify(lastFront);
    lastFront = { ...(up ? r : {}), running: run };
    if (JSON.stringify(lastFront) !== was) notify();
  } catch { /* Decky's backend busy: next time */ }
}

export function startFront(): () => void {
  coreStatus().then((s) => { coreUp = !!s?.running; notify(); }).catch(() => {});
  if (!timer) {
    timer = setInterval(() => { guard.run("tkInput", () => { tick(); }, undefined); }, 2000);
    setTimeout(() => { guard.run("tkInput", () => { tick(); }, undefined); }, 300);
  }
  return () => { if (timer) clearInterval(timer); timer = null; };
}

/** Re-renders with the game Steam runs, what Hearth Input is doing, and whether Hearth's core runs. */
export function useFront(): { run: Running | null; front: any; core: boolean | null } {
  const [, bump] = useState(0);
  useEffect(() => {
    const f = () => bump((n) => n + 1);
    listeners.add(f);
    f();   // anything that changed before this was shown
    const t = setInterval(f, 3000);   // the running game changes without the heartbeat changing anything
    return () => { listeners.delete(f); clearInterval(t); };
  }, []);
  return { run: running(), front: lastFront, core: coreUp };
}
