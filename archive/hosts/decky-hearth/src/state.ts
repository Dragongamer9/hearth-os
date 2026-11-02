// The plugin's live state: the switches, the parts the guard turned off, the shortcuts Hearth made in Steam.
import { useEffect, useState } from "react";
import { DEFAULTS, Settings, makeGuard } from "./model";
import { logError } from "./api";

export let settings: Settings = { ...DEFAULTS };
export let shortcutMap: Record<string, number> = {};
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((f) => { try { f(); } catch { /* a page that went away */ } });

export const guard = makeGuard((part, message) => {
  console.warn(`Hearth: ${part} turned off: ${message}`);
  logError(part, message).catch(() => { /* Decky's backend gone: the console has it */ });
}, notify);

export function setSettingsLocal(s: Settings) { settings = s; notify(); }
export function setShortcutMap(m: Record<string, number>) { shortcutMap = m || {}; notify(); }

/** Re-renders when a switch changes or a part is turned off. */
export function useLive(): Settings {
  const [, bump] = useState(0);
  useEffect(() => {
    const f = () => bump((n) => n + 1);
    listeners.add(f);
    return () => { listeners.delete(f); };
  }, []);
  return settings;
}
