// CI only (render-panel.mjs): a stand-in for @decky/api and for Steam's state. window.HM_STATE picks the scene:
// apple, soundtrack, pick, game, nomatch, running, off, changed, library, deezer, scan, choose. No Steam, no network: Apple's preview
// address doesn't resolve, so the panel goes through the backend's stream fallback, whose "song" is a tone made here.
import { S } from "./steam-state.js";
const songs = {
  apple: { source: "apple", key: "k1", url: "https://preview.invalid/a.m4a", title: "Elden Ring", artist: "Tsukasa Saitoh", album: "ELDEN RING (Original Soundtrack)", link: "https://music.apple.com/us/album/elden-ring/1" },
  library: { source: "apple", key: "k1", url: "https://preview.invalid/a.m4a", title: "Elden Ring", artist: "Tsukasa Saitoh", album: "ELDEN RING (Original Soundtrack)", link: "https://music.apple.com/us/album/elden-ring/1" },
  soundtrack: { source: "soundtrack", key: "k2", title: "01 Stardew Valley Overture" },
  pick: { source: "pick", key: "k3", title: "My Witcher Mix", via: "collection", name: "RPGs" },
  game: { source: "game", key: "k4", title: "main_theme" },
  nomatch: { source: null, why: "no theme song found" },
  scan: { source: null, why: "no theme song found" },
  choose: { source: null, why: "no theme song found" },
  deezer: { source: "deezer", from: "deezer", key: "k5", url: "https://preview.invalid/b.mp3", title: "Hollow Knight", artist: "Christopher Larkin", album: "Hollow Knight (Original Soundtrack)", link: "https://www.deezer.com/track/1" },
};
const scanning = { running: true, paused: false, total: 210, done: 150, matched: 143, waiting: "", last: 0 };
const scanned = { running: false, paused: false, total: 210, done: 210, matched: 198, waiting: "", last: 1 };
const found = { later: [], results: [
  { choice: "s1", from: "soundtrack", title: "01 - V", artist: "", album: "Cyberpunk 2077 Original Score" },
  { choice: "apple:1", from: "apple", title: "Cyberpunk 2077 Main Theme", artist: "Marcin Przybyłowicz", album: "Cyberpunk 2077 (Original Score)" },
  { choice: "apple:2", from: "apple", title: "The Rebel Path", artist: "P.T. Adamczyk", album: "Cyberpunk 2077 (Original Score)" },
  { choice: "deezer:3", from: "deezer", title: "Never Fade Away", artist: "P.T. Adamczyk, Olga Jankowska", album: "Cyberpunk 2077 (Original Score)" },
] };
function tone() {   // 1 s of a quiet 440 Hz tone, as a WAV, base64
  const rate = 8000, n = rate, buf = new ArrayBuffer(44 + n * 2), v = new DataView(buf);
  const w = (o, s) => [...s].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)));
  w(0, "RIFF"); v.setUint32(4, 36 + n * 2, true); w(8, "WAVEfmt "); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true);
  v.setUint32(24, rate, true); v.setUint32(28, rate * 2, true); v.setUint16(32, 2, true); v.setUint16(34, 16, true); w(36, "data"); v.setUint32(40, n * 2, true);
  for (let i = 0; i < n; i++) v.setInt16(44 + i * 2, Math.sin((i / rate) * 2 * Math.PI * 440) * 2000, true);
  let s = ""; new Uint8Array(buf).forEach((b) => { s += String.fromCharCode(b); }); return btoa(s);
}
const settings = () => ({ enabled: S() !== "off", volume: 0.35, delay: 0, where: S() === "library" ? "library" : "pages", online: true, toast: true });
const handlers = {
  get_settings: () => settings(), set_settings: (s) => s, find_song: () => songs[S()] || { source: null, why: "no theme song found" },
  audio: () => ({ data: tone(), mime: "audio/wav" }), set_pick: () => true, get_picks: () => ({}), get_home: () => "/home/you", report: () => null,
  scan_library: () => true, scan_pause: () => true, scan_status: () => (S() === "scan" ? scanning : scanned),
  search_music: () => found, choose_music: () => true,
};
export function callable(name) { return async (...a) => (handlers[name] ? handlers[name](...a) : null); }
export function definePlugin(f) { return f; }
export const FileSelectionType = { FILE: 0, FOLDER: 1 };
export async function openFilePicker() { return { path: "/home/you/Music/theme.mp3", realpath: "/home/you/Music/theme.mp3" }; }
// Steam's toast, drawn the way Steam Game Mode shows one (bottom right).
export const toaster = { toast: (t) => {
  const d = document.createElement("div");
  d.className = "toast";
  d.innerHTML = `<div class="toast-icon">♪</div><div><div class="toast-title"></div><div class="toast-body"></div></div>`;
  d.querySelector(".toast-title").textContent = t.title;
  d.querySelector(".toast-body").textContent = t.body;
  document.body.appendChild(d);
} };
