// Unit tests of Hearth Music's pure parts (src/model.ts), run in CI with Node's own test runner and type stripping:
//   node --experimental-strip-types --test hosts/decky-hearth-music/tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import * as M from "../src/model.ts";

const seen = (path, extra = {}) => ({ path, focused: 0, running: 0, launching: false, ...extra });
const on = (extra = {}) => ({ ...M.DEFAULTS, ...extra });

test("a game page plays its game; the library only when asked", () => {
  assert.equal(M.pageApp("/library/app/570"), 570);
  assert.equal(M.pageApp("/library/app/570/achievements"), 570);
  assert.equal(M.pageApp("/library/home"), 0);
  assert.equal(M.wanted(seen("/library/app/570"), on()), 570);
  assert.equal(M.wanted(seen("/library/home", { focused: 620 }), on()), 0, "game pages only by default");
  assert.equal(M.wanted(seen("/library/home", { focused: 620 }), on({ where: "library" })), 620);
  assert.equal(M.wanted(seen("/library/app/570", { focused: 620 }), on({ where: "library" })), 570, "the page wins");
  assert.equal(M.wanted(seen("/store", { focused: 620 }), on({ where: "library" })), 0, "not outside the library");
  assert.equal(M.wanted(seen("/library/home"), on({ where: "library" })), 0, "nothing highlighted");
});

test("silent while a game runs, after Play, and when off", () => {
  assert.equal(M.wanted(seen("/library/app/570", { running: 570 }), on()), 0);
  assert.equal(M.wanted(seen("/library/app/570", { launching: true }), on()), 0);
  assert.equal(M.wanted(seen("/library/app/570"), on({ enabled: false })), 0);
  assert.equal(M.silentWhy(seen("/library/app/570", { running: 570 }), on(), null), "Silent while a game runs");
  assert.equal(M.silentWhy(seen("/library/app/570"), on({ enabled: false }), null), "Hearth Music is off");
  assert.equal(M.silentWhy(seen("/library/app/570"), on(), { source: null, why: "no theme song found" }), "Silent: no theme song found");
  assert.equal(M.silentWhy(seen("/library/app/570"), on(), { source: "apple", key: "k" }), "");
  assert.equal(M.silentWhy(seen("/library/home"), on(), null), "Open a game's page to hear its theme");
});

test("fades are smooth and end where they should", () => {
  assert.equal(M.fadeAt(0, 0.5, 0, 1000), 0);
  assert.equal(M.fadeAt(0, 0.5, 1000, 1000), 0.5);
  assert.equal(M.fadeAt(0.5, 0, 2000, 1000), 0);
  const mid = M.fadeAt(0, 1, 500, 1000), out = M.fadeAt(1, 0, 500, 1000);
  assert.ok(Math.abs(mid - Math.SQRT1_2) < 1e-9 && Math.abs(out - Math.SQRT1_2) < 1e-9, "equal power: no dip in a crossfade");
  let last = -1;
  for (let t = 0; t <= 1000; t += 50) { const v = M.fadeAt(0, 0.8, t, 1000); assert.ok(v >= last); last = v; }
  assert.equal(M.fadeAt(0, 2, 5000, 1000), 1, "clamped");
});

test("now playing and Apple's credit", () => {
  const apple = { source: "apple", title: "Elden Ring", artist: "Tsukasa Saitoh", album: "ELDEN RING (Original Soundtrack)" };
  assert.equal(M.nowLine(apple), "Elden Ring — Tsukasa Saitoh");
  assert.equal(M.credit(apple), "Preview courtesy of Apple Music · ELDEN RING (Original Soundtrack)");
  assert.equal(M.credit({ source: "soundtrack", title: "x" }), "");
  assert.equal(M.nowLine({ source: "pick", title: "Mix", via: "collection", name: "RPGs" }), "Mix (your pick for RPGs)");
  assert.equal(M.nowLine({ source: null }), "");
});

test("settings are cleaned", () => {
  assert.deepEqual(M.cleanSettings({ volume: 4, delay: -1, where: "x" }), { ...M.DEFAULTS, volume: 1, delay: 0, where: "pages" });
  assert.deepEqual(M.cleanSettings(null), M.DEFAULTS);
});

test("Deezer's credit and link, also for a pick chosen from it", () => {
  const dz = { source: "deezer", from: "deezer", title: "Hollow Knight", artist: "Christopher Larkin", album: "Hollow Knight (Original Soundtrack)", link: "https://www.deezer.com/track/1" };
  assert.equal(M.nowLine(dz), "Hollow Knight — Christopher Larkin");
  assert.equal(M.credit(dz), "Preview from Deezer · Hollow Knight (Original Soundtrack)");
  assert.equal(M.openLabel(dz), "Open in Deezer");
  const pick = { source: "pick", from: "deezer", title: "In the Blood", artist: "Darren Korb", via: "game", link: "https://www.deezer.com/track/2" };
  assert.equal(M.nowLine(pick), "In the Blood — Darren Korb (your pick)");
  assert.equal(M.credit(pick), "Preview from Deezer");
  assert.equal(M.credit({ source: "pick", from: "apple", title: "x" }), "Preview courtesy of Apple Music");
  assert.equal(M.openLabel({ source: "apple", link: "https://music.apple.com/x" }), "Open in Apple Music");
  assert.equal(M.openLabel({ source: "pick", title: "mine" }), "");
  assert.equal(M.credit({ source: "pick", title: "mine" }), "");
});

test("the library scan's lines", () => {
  const s = { running: true, paused: false, total: 210, done: 142, matched: 143, waiting: "", last: 0 };
  assert.deepEqual(M.scanLines(s), ["Matched 143 of 210", "Looking through your library: 143 of 210"]);
  assert.equal(M.scanLines({ ...s, waiting: "game" })[1], "Paused while a game runs");
  assert.equal(M.scanLines({ ...s, paused: true })[1], "Paused while a game runs");
  assert.equal(M.scanLines({ ...s, waiting: "slow" })[1], "Waiting a moment: the music service asked to slow down");
  assert.equal(M.scanLines({ ...s, running: false, last: 1 })[0], "Matched 143 of 210");
  assert.deepEqual(M.scanLines(null), ["Not scanned yet", ""]);
});

test("Choose music lines, and the Deezer switch defaults on and stays as set", () => {
  assert.equal(M.choiceLine({ choice: "a", from: "deezer", title: "In the Blood", artist: "Darren Korb", album: "" }), "In the Blood — Darren Korb · Deezer preview");
  assert.equal(M.choiceLine({ choice: "b", from: "soundtrack", title: "02 - Main Theme", artist: "", album: "Hades OST" }), "02 - Main Theme · your soundtrack");
  assert.equal(M.cleanSettings({}).deezer, true);
  assert.equal(M.cleanSettings({ deezer: false }).deezer, false);
  assert.equal(M.cleanSettings({ deezer: false, volume: 0.9 }).deezer, false);
});

test("Scan all games: n of N, cancelled, and the review list's lines and groups", () => {
  const s = { running: true, paused: false, total: 40, done: 11, matched: 9, waiting: "", last: 0, all: true, current: "Celeste" };
  assert.equal(M.scanLines(s)[1], "Checking every game: 12 of 40 · Celeste");
  assert.equal(M.scanLines({ ...s, waiting: "slow" })[1], "Checking every game: 12 of 40 · Celeste (waiting: the music service asked to slow down)");
  assert.equal(M.scanLines({ ...s, paused: true })[1], "Paused while a game runs");
  assert.equal(M.scanLines({ ...s, running: false, all: false, cancelled: true })[1], "Scan cancelled after 11 of 40; what it found is kept");
  const row = (title, group, song, extra = {}) => ({ appid: "1", title, kind: "steam", query: title, group, song, ...extra });
  assert.equal(M.reviewLine(row("Control", "auto", { title: "Control Theme", artist: "Petri Alanko", album: "CONTROL (Original Soundtrack)", from: "apple" })),
    "Control Theme — Petri Alanko · CONTROL (Original Soundtrack) · Apple Music");
  assert.equal(M.reviewLine(row("INSIDE", "none", null, { checked: true })), "Nothing found that is surely this game's music");
  assert.equal(M.reviewLine(row("New", "none", null)), "Not checked yet");
  assert.equal(M.reviewLine(row("Celeste", "pick", { title: "", artist: "", album: "", from: "none" })), "No music (your choice)");
  assert.equal(M.reviewLine(row("Stardew", "auto", { title: "01 Overture", artist: "", album: "", from: "soundtrack" })), "01 Overture · your soundtrack");
  const g = M.grouped([row("b", "auto", null), row("a", "auto", null), row("c", "pick", null)]);
  assert.deepEqual(g.map(([k, l, r]) => [k, l, r.map((x) => x.title)]), [["none", "No music found", []], ["auto", "Found automatically", ["a", "b"]], ["pick", "Your picks", ["c"]]]);
});

test("the YouTube mini player: the official embed only, at least 200x200, YouTube's controls kept", () => {
  const u = new URL(M.youtubeEmbedUrl("dQw4w9WgXcQ", "https://steamloopback.host"));
  assert.equal(u.origin, "https://www.youtube-nocookie.com");
  assert.equal(u.pathname, "/embed/dQw4w9WgXcQ");
  assert.equal(u.searchParams.get("controls"), "1", "YouTube's own controls");
  assert.equal(u.searchParams.get("enablejsapi"), "1");
  assert.equal(u.searchParams.get("origin"), "https://steamloopback.host", "YouTube is told where it's embedded");
  assert.equal(u.searchParams.get("loop"), "1");
  assert.equal(u.searchParams.get("playlist"), "dQw4w9WgXcQ", "looping one video needs it as its playlist");
  assert.equal(M.youtubeEmbedUrl("dQw4w9WgXcQ", "steam://x").includes("origin="), false, "only an https origin");
  assert.equal(M.youtubeEmbedUrl("bad", "https://steamloopback.host"), "");
  assert.equal(M.youtubeEmbedUrl("dQw4w9WgXcQ\"><script>", ""), "");
  assert.ok(M.MINI.width >= 200 && M.MINI.height >= 200, "YouTube asks for at least 200x200");
  assert.equal(M.ytVolume(0.35), 35);
  assert.equal(M.ytVolume(2), 100);
});

test("YouTube's player messages: only from its own origin", () => {
  assert.deepEqual(M.ytMessage("https://www.youtube-nocookie.com", '{"event":"onReady","info":null}'), { event: "onReady", info: null });
  assert.deepEqual(M.ytMessage("https://www.youtube-nocookie.com", { event: "onError", info: 150 }), { event: "onError", info: 150 });
  assert.equal(M.ytMessage("https://evil.example", '{"event":"onReady"}'), null);
  assert.equal(M.ytMessage("https://www.youtube-nocookie.com", "not json"), null);
  assert.match(M.ytErrorLine(150), /outside YouTube/);
  assert.match(M.ytErrorLine(153), /153/);
});

test("a YouTube song in the panel, and its setting", () => {
  const song = { source: "youtube", youtube: "dQw4w9WgXcQ", key: "yt:dQw4w9WgXcQ", title: "Hades OST - Main Theme", artist: "Supergiant Games", link: "https://www.youtube.com/watch?v=dQw4w9WgXcQ" };
  assert.equal(M.nowLine(song), "Hades OST - Main Theme — Supergiant Games");
  assert.equal(M.credit(song), "Video on YouTube, in YouTube's player");
  assert.equal(M.openLabel(song), "Open on YouTube");
  assert.equal(M.credit({ source: "pick", from: "youtube", title: "x" }), "Video on YouTube, in YouTube's player");
  assert.equal(M.choiceLine({ choice: "youtube:x", from: "youtube", title: "Hades - Main Theme", artist: "Supergiant Games", album: "" }), "Hades - Main Theme — Supergiant Games · YouTube video");
  assert.equal(M.sourceLabel("youtube"), "YouTube");
  assert.equal(M.DEFAULTS.youtube, true);
  assert.equal(M.cleanSettings({}).youtube, true);
  assert.equal(M.cleanSettings({ youtube: false }).youtube, false, "remembered off");
  assert.equal(M.cleanSettings({ youtube: false, volume: 0.6 }).volume, 0.6);
  assert.match(M.youtubeLine(null), /paste a YouTube link/);
  assert.match(M.youtubeLine({ key: true, state: "quota", used: 8000, daily: 10000, auto: 8000 }), /midnight Pacific/);
  assert.match(M.youtubeLine({ key: true, state: "ok", used: 300, daily: 10000, auto: 8000 }), /300 of 10000/);
});
