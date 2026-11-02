// Hearth Music's pure parts (no Steam, no Decky): what should play, when, and how loud. Unit-tested in
// tests/model.test.mjs.

export interface Settings { enabled: boolean; volume: number; delay: number; where: "pages" | "library"; online: boolean; toast: boolean }
export const DEFAULTS: Settings = { enabled: true, volume: 0.35, delay: 1.5, where: "pages", online: true, toast: true };

/** A song the backend found: where it's from and (for Apple's previews) the credit. */
export interface Song { source: "pick" | "soundtrack" | "game" | "apple" | null; key?: string; title?: string; artist?: string;
  album?: string; link?: string; why?: string; later?: boolean; via?: string; name?: string }

/** What Steam shows right now, as Hearth Music reads it. */
export interface Seen { path: string; focused: number; running: number; launching: boolean }

export const FADE_IN = 1500;
export const CROSSFADE = 1500;
export const FADE_OUT_PLAY = 700;

/** The app id of a game page (/library/app/<appid>, also its sub-pages), or 0. */
export function pageApp(path: string): number {
  const m = /^\/library\/app\/(\d+)(?:\/|$)/.exec(path || "");
  return m ? Number(m[1]) : 0;
}

/** Whether Steam's library (not a game page) is showing. */
export function inLibrary(path: string): boolean {
  return /^\/library(\/(home|collection|tab|all|collections)?|$)/.test(path || "") && !pageApp(path);
}

/** The game whose song should play now, or 0 for silence. */
export function wanted(seen: Seen, s: Settings): number {
  if (!s.enabled || seen.running > 0 || seen.launching) return 0;
  const page = pageApp(seen.path);
  if (page) return page;
  if (s.where === "library" && inLibrary(seen.path) && seen.focused > 0) return seen.focused;
  return 0;
}

/** Why it's silent, in a few words (for the panel), or "" when something plays. */
export function silentWhy(seen: Seen, s: Settings, song: Song | null): string {
  if (!s.enabled) return "Hearth Music is off";
  if (seen.running > 0 || seen.launching) return "Silent while a game runs";
  if (!wanted(seen, s)) return s.where === "library" ? "Highlight a game to hear its theme" : "Open a game's page to hear its theme";
  if (song && !song.source) return song.why ? `Silent: ${song.why}` : "Silent";
  return "";
}

/** The volume at `t` ms into a fade from `from` to `to` over `ms` (equal-power, so a crossfade doesn't dip). */
export function fadeAt(from: number, to: number, t: number, ms: number): number {
  if (ms <= 0 || t >= ms) return clamp(to);
  if (t <= 0) return clamp(from);
  const x = t / ms;
  const g = to > from ? Math.sin((x * Math.PI) / 2) : Math.cos((x * Math.PI) / 2);
  return clamp(to > from ? from + (to - from) * g : to + (from - to) * g);
}

export function clamp(v: number): number { return Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0)); }

/** The line under "Now playing": the song and where it's from. */
export function nowLine(song: Song | null): string {
  if (!song || !song.source) return "";
  const t = song.title || "Theme";
  if (song.source === "apple") return `${t} — ${song.artist || "Unknown artist"}`;
  if (song.source === "pick") return `${t} (your pick${song.via === "collection" ? ` for ${song.name}` : ""})`;
  if (song.source === "soundtrack") return `${t} (the game's soundtrack)`;
  return `${t} (from the game's files)`;
}

/** Apple's credit for a preview, shown with it. */
export function credit(song: Song | null): string {
  return song?.source === "apple" ? `Preview courtesy of Apple Music${song.album ? ` · ${song.album}` : ""}` : "";
}

export function cleanSettings(s: Partial<Settings> | null | undefined): Settings {
  const o = { ...DEFAULTS, ...(s || {}) } as Settings;
  o.volume = clamp(Number(o.volume));
  o.delay = Math.min(10, Math.max(0, Number(o.delay) || 0));
  if (o.where !== "library") o.where = "pages";
  return o;
}
