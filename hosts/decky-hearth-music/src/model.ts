// Hearth Music's pure parts (no Steam, no Decky): what should play, when, and how loud. Unit-tested in
// tests/model.test.mjs.

export interface Settings { enabled: boolean; volume: number; delay: number; where: "pages" | "library"; online: boolean; toast: boolean; deezer: boolean; youtube: boolean }
export const DEFAULTS: Settings = { enabled: true, volume: 0.35, delay: 1.5, where: "pages", online: true, toast: true, deezer: true, youtube: true };

/** A song the backend found: where it's from and (for Apple's and Deezer's previews, `from`) the credit. */
export interface Song { source: "pick" | "soundtrack" | "game" | "apple" | "deezer" | "youtube" | null; key?: string; url?: string; youtube?: string;
  title?: string; artist?: string; album?: string; link?: string; why?: string; later?: boolean; via?: string; name?: string;
  from?: "apple" | "deezer" | "youtube"; pick?: "none" }

/** The YouTube mini player: the person's key kept or not (never the key), and whether YouTube can be asked now. */
export interface YouTubeStatus { key: boolean; state: "ok" | "key" | "bad_key" | "quota" | "slow"; used: number; daily: number; auto: number; saved?: boolean }

/** The library scan, as the backend tells it. */
export interface Scan { running: boolean; paused: boolean; total: number; done: number; matched: number; waiting: string; last: number;
  all?: boolean; current?: string; cancelled?: boolean }

/** One game in the review list (the backend's review()): what it plays and why. */
export interface ReviewSong { title: string; artist: string; album: string; from: "apple" | "deezer" | "youtube" | "soundtrack" | "game" | "file" | "none" }
export interface ReviewRow { appid: string; title: string; kind: string; query: string; group: "none" | "auto" | "pick"; song: ReviewSong | null; checked?: boolean }

export const GROUPS: [ReviewRow["group"], string][] = [["none", "No music found"], ["auto", "Found automatically"], ["pick", "Your picks"]];

/** A song "Choose music…" found: one of your soundtracks' tracks, or an Apple or Deezer preview. */
export interface Choice { choice: string; from: "soundtrack" | "apple" | "deezer" | "youtube"; title: string; artist: string; album: string }

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
  if (song.source === "apple" || song.source === "deezer") return `${t} — ${song.artist || "Unknown artist"}`;
  if (song.source === "youtube") return song.artist ? `${t} — ${song.artist}` : t;
  if (song.source === "pick") return `${t}${song.from && song.artist ? ` — ${song.artist}` : ""} (your pick${song.via === "collection" ? ` for ${song.name}` : ""})`;
  if (song.source === "soundtrack") return `${t} (the game's soundtrack)`;
  return `${t} (from the game's files)`;
}

/** Where an online preview or video is from ("apple", "deezer", "youtube"), also for a pick chosen from them; "" otherwise. */
export function previewFrom(song: Song | null): "apple" | "deezer" | "youtube" | "" {
  if (!song) return "";
  if (song.from === "apple" || song.from === "deezer" || song.from === "youtube") return song.from;
  return song.source === "apple" || song.source === "deezer" || song.source === "youtube" ? song.source : "";
}

/** The credit for a preview, shown with it: Apple's, or Deezer's. */
export function credit(song: Song | null): string {
  const f = previewFrom(song);
  const album = song?.album ? ` · ${song.album}` : "";
  if (f === "apple") return `Preview courtesy of Apple Music${album}`;
  if (f === "deezer") return `Preview from Deezer${album}`;
  if (f === "youtube") return "Video on YouTube, in YouTube's player";
  return "";
}

/** The label of the button that opens the preview's track where it's from. */
export function openLabel(song: Song | null): string {
  const f = previewFrom(song);
  return song?.link ? (f === "deezer" ? "Open in Deezer" : f === "apple" ? "Open in Apple Music" : f === "youtube" ? "Open on YouTube" : "") : "";
}

/** The library scan in the panel: [the count, what it's doing]. */
export function scanLines(s: Scan | null): [string, string] {
  if (!s || (!s.total && !s.running)) return ["Not scanned yet", ""];
  const count = `Matched ${s.matched} of ${s.total}`;
  if (!s.running && s.cancelled) return [count, `Scan cancelled after ${s.done} of ${s.total}; what it found is kept`];
  if (s.running && s.all && !(s.waiting === "game" || s.paused)) {
    const n = Math.min(s.done + 1, s.total);
    const wait = s.waiting === "slow" ? " (waiting: the music service asked to slow down)" : s.waiting === "network" ? " (waiting for the network)" : "";
    return [count, `Checking every game: ${n} of ${s.total}${s.current ? ` · ${s.current}` : ""}${wait}`];
  }
  if (!s.running) return [count, s.last ? "Done: new games are matched as they appear, older answers refreshed weekly" : ""];
  if (s.waiting === "game" || s.paused) return [count, "Paused while a game runs"];
  if (s.waiting === "slow") return [count, "Waiting a moment: the music service asked to slow down"];
  if (s.waiting === "network") return [count, "Waiting for the network"];
  return [count, `Looking through your library: ${Math.min(s.done + 1, s.total)} of ${s.total}`];
}

/** A line for one "Choose music…" result. */
export function choiceLine(c: Choice): string {
  const where = c.from === "apple" ? "Apple Music preview" : c.from === "deezer" ? "Deezer preview" : c.from === "youtube" ? "YouTube video" : "your soundtrack";
  return `${c.title}${c.artist ? ` — ${c.artist}` : ""} · ${where}`;
}

export function cleanSettings(s: Partial<Settings> | null | undefined): Settings {
  const o = { ...DEFAULTS, ...(s || {}) } as Settings;
  o.volume = clamp(Number(o.volume));
  o.delay = Math.min(10, Math.max(0, Number(o.delay) || 0));
  if (o.where !== "library") o.where = "pages";
  o.deezer = o.deezer !== false;
  o.youtube = o.youtube !== false;
  return o;
}

/** Where a review row's song is from, in a few words. */
export function sourceLabel(f: ReviewSong["from"] | undefined): string {
  return f === "apple" ? "Apple Music" : f === "deezer" ? "Deezer" : f === "youtube" ? "YouTube" : f === "soundtrack" ? "your soundtrack" : f === "game" ? "the game's files"
    : f === "file" ? "your file" : f === "none" ? "" : "";
}

/** The song line of a review row: "Track — Artist · Album · Apple Music", or why there is none. */
export function reviewLine(r: ReviewRow): string {
  const s = r.song;
  if (!s) return r.checked ? "Nothing found that is surely this game's music" : "Not checked yet";
  if (s.from === "none") return "No music (your choice)";
  return [s.title + (s.artist ? ` — ${s.artist}` : ""), s.album, sourceLabel(s.from)].filter(Boolean).join(" · ");
}

/** The review list in its groups, each alphabetical: [group, heading, rows][]. */
export function grouped(rows: ReviewRow[]): [ReviewRow["group"], string, ReviewRow[]][] {
  return GROUPS.map(([g, label]) => [g, label, rows.filter((r) => r.group === g).sort((a, b) => a.title.localeCompare(b.title))] as [ReviewRow["group"], string, ReviewRow[]]);
}

// ---- The YouTube mini player (src/video.tsx): YouTube's own embed, visible, at least 200x200, its controls kept.

/** The mini player's size in CSS px: YouTube asks for at least 200x200; small, so YouTube picks a low quality. */
export const MINI = { width: 384, height: 216 };
export const YT_EMBED_ORIGIN = "https://www.youtube-nocookie.com";
const YT_ID = /^[A-Za-z0-9_-]{11}$/;

/** The official embed's address for a video (YouTube's own controls and branding; looped; the JS API on so the
 *  plugin can set the volume and fade it like the previews), or "" for anything that isn't a video id. ORIGIN is the
 *  page's own (Steam's https://steamloopback.host), so YouTube knows where it is embedded. */
export function youtubeEmbedUrl(id: string, origin: string): string {
  if (!YT_ID.test(String(id || ""))) return "";
  const q = new URLSearchParams({ autoplay: "1", controls: "1", enablejsapi: "1", loop: "1", playlist: id, playsinline: "1",
    rel: "0", iv_load_policy: "3" });
  if (/^https:\/\/[^/]+$/.test(origin || "")) q.set("origin", origin);
  return `${YT_EMBED_ORIGIN}/embed/${id}?${q.toString()}`;
}

/** The volume YouTube's player takes (0-100) for the plugin's (0-1). */
export function ytVolume(v: number): number { return Math.round(clamp(v) * 100); }

/** A message from YouTube's embed (postMessage, JSON): {event, info}, or null when it isn't one of its messages. */
export function ytMessage(origin: string, data: unknown): { event: string; info: any } | null {
  if (origin !== YT_EMBED_ORIGIN && origin !== "https://www.youtube.com") return null;
  let d: any = data;
  if (typeof d === "string") { try { d = JSON.parse(d); } catch { return null; } }
  if (!d || typeof d !== "object" || typeof d.event !== "string") return null;
  return { event: d.event, info: d.info };
}

/** What a YouTube player error code means, for the log (2: bad id, 5: HTML5 player, 100: gone, 101/150: its owner
 *  doesn't allow embedding, 153: YouTube couldn't tell where it is embedded, the page sent no Referer). */
export function ytErrorLine(code: unknown): string {
  const c = Number(code);
  if (c === 101 || c === 150) return "its owner doesn't allow it to play outside YouTube";
  if (c === 100) return "the video is gone or private";
  if (c === 153 || c === 152) return "YouTube's player couldn't tell where it is embedded (error " + c + ")";
  if (c === 2) return "not a video";
  if (c === 5) return "Steam's browser can't play it";
  return "error " + String(code);
}

/** The panel's line about the YouTube key. */
export function youtubeLine(st: YouTubeStatus | null): string {
  if (!st || !st.key) return "No key: no automatic YouTube search. You can still paste a YouTube link into Choose music… for any game.";
  if (st.state === "bad_key") return "YouTube didn't accept this key. Paste another one.";
  if (st.state === "quota") return "Today's YouTube searches are used up; they go on after midnight Pacific time.";
  if (st.state === "slow") return "YouTube asked to wait a moment.";
  return `Key saved. Games with no other song are searched on YouTube (${st.used} of ${st.daily} units used today).`;
}
