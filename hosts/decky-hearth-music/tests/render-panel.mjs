// CI only (decky-hearth-music.yml): draws Hearth Music's real Quick Access panel (src/index.tsx, running its real loop,
// src/music.ts) in a plain browser over a static stand-in of Steam's game page or library, with stand-ins for Decky and
// Steam's state (mock/), and saves pictures, including the "Now playing" note. Steam isn't involved; the song is a tone.
// Usage: HEARTH_ROOT=<repo> node render-panel.mjs OUTDIR   (needs esbuild, react, react-dom, react-icons, playwright next to it)
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const mock = join(root, "hosts/decky-hearth-music/tests/mock");
const out = process.argv[2] || "shots";
mkdirSync(out, { recursive: true });
const res = await build({
  entryPoints: [join(mock, "entry.jsx")], bundle: true, write: false, format: "iife", jsx: "automatic", target: "es2020",
  alias: { "@decky/ui": join(mock, "stub-ui.jsx"), "@decky/api": join(mock, "stub-api.js"), react: join(here, "node_modules/react"),
           "react-dom": join(here, "node_modules/react-dom"), "react-icons": join(here, "node_modules/react-icons") },
  nodePaths: [join(here, "node_modules")], define: { "process.env.NODE_ENV": '"production"' }, logLevel: "warning",
});
const css = `body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:#0c1016;font-family:"Noto Sans",Arial,sans-serif;color:#e9eef4}
.qam{position:absolute;right:0;top:0;bottom:0;width:460px;display:flex;background:rgba(14,20,27,.98);box-shadow:-8px 0 30px rgba(0,0,0,.5);z-index:5}
.qam-tabs{width:58px;background:#0b1016;display:flex;flex-direction:column;align-items:center;padding-top:12px;gap:6px}
.qam-tab{width:40px;height:40px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:#8d99a8;font-size:18px}.qam-tab.on{background:#3d4450;color:#fff}
.qam-body{flex:1;display:flex;flex-direction:column;min-width:0}.qam-title{font-size:18px;font-weight:700;padding:16px 16px 8px}
.ScrollPanel{padding:0 12px 20px;overflow:hidden}.qam-section{margin-bottom:6px}.qam-section-title{font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:#8d99a8;padding:8px 4px 4px}
.qam-row{padding:3px 4px}.qam-field{display:flex;flex-direction:column;gap:4px;padding:2px 0}.qam-label{font-size:14px}.qam-desc{font-size:11.5px;color:#8d99a8}.Text{font-size:14px}
.qam-focusable{display:flex}.qam-dbtn{padding:6px 10px;font-size:13px;cursor:pointer}.qam-dbtn.gpfocus{background:#fff;color:#0e141b}
.qam-input{background:#0b1016;border:1px solid #3d4450;border-radius:4px;color:#e9eef4;padding:7px 9px;font:inherit;font-size:14px}
.DialogButton{border:none;color:#e9eef4;background:#3d4450;border-radius:4px;font:inherit}.qam-btn{width:100%;padding:9px 12px;font-size:14px}.qam-btn.gpfocus{background:#fff;color:#0e141b}.qam-switch.gpfocus{outline:2px solid #fff;outline-offset:2px}
.qam-line{display:flex;justify-content:space-between;align-items:center}.qam-switch{width:40px;height:22px;border-radius:11px;background:#273140;position:relative}
.qam-switch i{position:absolute;top:3px;left:3px;width:16px;height:16px;border-radius:50%;background:#e9eef4}.qam-switch.on{background:#1a9fff}.qam-switch.on i{left:21px}
.qam-val{font-size:13px;color:#b8c2cc}.qam-slider{height:4px;border-radius:2px;background:#273140;margin:6px 0 4px}.qam-slider b{display:block;height:4px;border-radius:2px;background:#1a9fff}
.stand{position:absolute;inset:0}.hero{height:430px;position:relative}.hero h1{position:absolute;left:48px;bottom:24px;margin:0;font-size:44px;text-shadow:0 2px 12px #000}
.bar{display:flex;gap:14px;align-items:center;padding:18px 48px;background:#171d25}.play{background:#59bf40;color:#fff;font-weight:700;padding:12px 40px;border-radius:4px;font-size:18px}
.play.f{outline:3px solid #fff}.meta{color:#8b929a;font-size:14px}.top{display:flex;gap:10px;padding:16px 28px;background:#171d25;font-size:15px}.tab{padding:6px 14px;border-radius:4px;color:#8b929a}.tab.on{background:#dcdedf;color:#0e141b}
.grid{display:flex;flex-wrap:wrap;gap:16px;padding:20px 28px}.cap{width:150px;height:225px;border-radius:4px;display:flex;align-items:flex-end;padding:10px;box-sizing:border-box;font-size:15px;box-shadow:0 4px 14px rgba(0,0,0,.4)}
.cap b{text-shadow:0 1px 4px #000}.cap.f{outline:3px solid #fff;transform:scale(1.05)}.note{position:absolute;bottom:10px;left:20px;font-size:11px;color:#67707b}
.toast{position:absolute;right:24px;bottom:24px;width:360px;display:flex;gap:12px;align-items:center;background:#23262e;border-radius:6px;padding:12px 14px;box-shadow:0 6px 24px rgba(0,0,0,.6);z-index:9}
.toast-icon{width:40px;height:40px;border-radius:6px;background:#1a9fff;display:flex;align-items:center;justify-content:center;font-size:22px}.toast-title{font-weight:700;font-size:14px}.toast-body{font-size:12.5px;color:#b8c2cc;white-space:pre-line}`;
const PAL = { 367520: ["#1c2230", "#9fb3c8"], 1245620: ["#3b2f1a", "#c9a85b"], 413150: ["#2d6a4f", "#95d5b2"], 292030: ["#1b263b", "#8d99ae"], 620: ["#283044", "#78a1bb"], 1091500: ["#f7e600", "#00c2d1"] };
const NAMES = { 367520: "Hollow Knight", 1245620: "ELDEN RING", 413150: "Stardew Valley", 292030: "The Witcher 3: Wild Hunt", 620: "Portal 2", 1091500: "Cyberpunk 2077" };
const gamePage = (id, playFocus) => `<div class="stand"><div class="hero" style="background:linear-gradient(160deg,${PAL[id][0]},${PAL[id][1]})"><h1>${NAMES[id]}</h1></div>
<div class="bar"><span class="play${playFocus ? " f" : ""}">${playFocus === "running" ? "Resume" : "Play"}</span><span class="meta">Last played today · 41 hours</span></div>
<div class="note">Static stand-in for Steam's game page (not Steam itself)</div></div>`;
const library = () => `<div class="stand"><div class="top"><span class="tab on">Home</span><span class="tab">Collections</span><span class="tab">All games</span></div>
<div class="grid">${Object.keys(NAMES).map((id) => `<div class="cap" data-app="${id}" style="background:linear-gradient(160deg,${PAL[id][0]},${PAL[id][1]})"><b>${NAMES[id]}</b></div>`).join("")}</div>
<div class="note">Static stand-in for Steam's library (not Steam itself)</div></div>`;
const page0 = (scene, panel, bg) => `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body>${bg}<div id="root"></div>
<script>window.HM_STATE=${JSON.stringify(scene)};window.HM_PANEL=${panel};</script><script>${res.outputFiles[0].text}</script></body></html>`;
const browser = await chromium.launch({ args: ["--autoplay-policy=no-user-gesture-required"] });
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
const focus = (label) => page.evaluate((l) => { document.querySelectorAll(".gpfocus").forEach((x) => x.classList.remove("gpfocus"));
  const b = [...document.querySelectorAll(".qam-btn")].find((x) => x.textContent.includes(l))
    || [...document.querySelectorAll(".qam-field")].find((x) => x.textContent.includes(l))?.querySelector(".qam-switch");
  if (b) b.classList.add("gpfocus"); }, label);
// Scrolls the panel so the section holding `text` shows (the real Quick Access panel scrolls the same way).
const reveal = (text) => page.evaluate((t) => { const sp = document.querySelector(".ScrollPanel"); sp.style.flex = "1"; sp.style.minHeight = "0";
  const el = [...document.querySelectorAll(".qam-section")].find((x) => x.textContent.includes(t)); if (el) sp.scrollTop = el.offsetTop - sp.offsetTop - 8; }, text);
const click = (label) => page.evaluate((l) => { const b = [...document.querySelectorAll(".qam-btn")].find((x) => x.textContent.includes(l)); if (b) b.click(); }, label);
// A small button inside a row (the review list's ▶ / Fix, a choice's Choose), found by the row's text.
const rowBtn = (rowText, label, press) => page.evaluate(([r, l, p]) => { document.querySelectorAll(".gpfocus").forEach((x) => x.classList.remove("gpfocus"));
  const row = [...document.querySelectorAll(".qam-focusable")].find((x) => x.textContent.includes(r));
  const b = row && [...row.querySelectorAll(".qam-dbtn")].find((x) => x.textContent.includes(l)); if (b) { b.classList.add("gpfocus"); if (p) b.click(); } }, [rowText, label, press]);
const revealText = (text) => page.evaluate((t) => { const sp = document.querySelector(".ScrollPanel"); sp.style.flex = "1"; sp.style.minHeight = "0";
  const el = [...document.querySelectorAll(".qam-row,.qam-section-title")].find((x) => x.textContent.includes(t)); if (el) sp.scrollTop = el.offsetTop - sp.offsetTop - 8; }, text);
// Highlights a library tile the way Steam does: a focus event from an element whose React props carry the game.
const focusTile = (id) => page.evaluate((a) => { const el = document.querySelector(`.cap[data-app="${a}"]`); el.classList.add("f");
  el.__reactFiber$stand = { memoizedProps: { app: { appid: Number(a) } }, return: null }; el.tabIndex = 0; el.focus(); }, id);
// [picture, scene, panel shown, background, what to do, what it must show]
const shots = [
  ["hearth-music-panel-apple", "apple", true, gamePage(1245620), async () => focus("Open in Apple Music"), ["ELDEN RING", "Elden Ring — Tsukasa Saitoh", "Preview courtesy of Apple Music", "Open in Apple Music", "Volume", "Wait before playing", "In the library too", "Pick a song for RPGs", "Pick a song for Favorites"]],
  ["hearth-music-panel-soundtrack", "soundtrack", true, gamePage(413150), async () => focus("Pick a song"), ["01 Stardew Valley Overture (the game's soundtrack)"]],
  ["hearth-music-panel-pick", "pick", true, gamePage(292030), async () => focus("Pick a song for RPGs"), ["My Witcher Mix (your pick for RPGs)"]],
  ["hearth-music-panel-gamefiles", "game", true, gamePage(620), async () => focus("Hearth Music"), ["main_theme (from the game's files)"]],
  ["hearth-music-panel-nomatch", "nomatch", true, gamePage(1091500), async () => focus("Apple Music previews"), ["Silent: no theme song found"]],
  ["hearth-music-panel-running", "running", true, gamePage(1091500, "running"), async () => {}, ["Silent while a game runs"]],
  ["hearth-music-panel-off", "off", true, gamePage(1245620), async () => focus("Hearth Music"), ["Hearth Music is off"]],
  ["hearth-music-panel-steam-changed", "changed", true, gamePage(1245620), async () => {}, ["Hearth Music turned itself off"]],
  ["hearth-music-panel-library", "library", true, library(), async () => { await focusTile(1245620); await page.waitForTimeout(1200); }, ["Elden Ring — Tsukasa Saitoh", "Plays for the highlighted game in the library"]],
  ["hearth-music-panel-deezer", "deezer", true, gamePage(367520), async () => focus("Open in Deezer"), ["Hollow Knight — Christopher Larkin", "Preview from Deezer · Hollow Knight (Original Soundtrack)", "Open in Deezer", "Use Deezer previews"]],
  ["hearth-music-panel-scan", "scan", true, gamePage(1091500), async () => { await reveal("Your library"); }, ["Matched 143 of 210", "Looking through your library: 151 of 210", "Scan all games"]],
  ["hearth-music-panel-scan-done", "nomatch", true, gamePage(1091500), async () => { await reveal("Your library"); await focus("Scan all games"); }, ["Matched 198 of 210", "Done: new games are matched as they appear", "Choose music…", "Scan all games", "Review music for every game"]],
  ["hearth-music-scan-all-running", "scanall", true, gamePage(1091500), async () => { await reveal("Your library"); await focus("Cancel scan"); }, ["Checking every game: 24 of 64 · INSIDE", "Cancel scan"]],
  ["hearth-music-review-list", "review", true, library(), async () => { await click("Review music for every game"); await page.waitForTimeout(600); await rowBtn("INSIDE", "Fix"); },
    ["Music for every game", "No music found (2)", "Found automatically (4)", "Your picks (2)", "Nothing found that is surely this game's music",
     "Control Theme — Petri Alanko · CONTROL (Original Soundtrack) · Apple Music", "Hollow Knight — Christopher Larkin · Hollow Knight (Original Soundtrack) · Deezer",
     "01 Stardew Valley Overture · your soundtrack", "No music (your choice)", "Scan all games"]],
  ["hearth-music-review-list-picks", "review", true, library(), async () => { await click("Review music for every game"); await page.waitForTimeout(600); await revealText("Found automatically"); await rowBtn("Control Ultimate Edition", "▶", true); await page.waitForTimeout(600); },
    ["■ Stop", "Your picks (2)", "The Fields of Ard Skellig — Marcin Przybyłowicz"]],
  ["hearth-music-review-fix", "fix", true, library(), async () => { await click("Review music for every game"); await page.waitForTimeout(600); await rowBtn("INSIDE", "Fix", true); await page.waitForTimeout(400); await focus("Choose music…"); },
    ["INSIDE", "Choose music…", "No music for this game", "Back to the list"]],
  ["hearth-music-review-picker", "fixchoose", true, library(), async () => { await click("Review music for every game"); await page.waitForTimeout(600); await rowBtn("Control Ultimate Edition", "Fix", true); await page.waitForTimeout(400); await click("Choose music…"); await page.waitForTimeout(600); await rowBtn("Cyberpunk 2077 Main Theme", "▶", true); await page.waitForTimeout(500); },
    ["Control Ultimate Edition", "Search", "■ Stop", "Choose", "Apple Music preview", "Deezer preview", "Close"]],
  ["hearth-music-panel-choose", "choose", true, gamePage(1091500), async () => { await click("Choose music…"); await page.waitForTimeout(500); await reveal("Song for"); await focus("Cyberpunk 2077 Main Theme"); },
    ["Search", "01 - V · your soundtrack", "Cyberpunk 2077 Main Theme — Marcin Przybyłowicz · Apple Music preview", "Never Fade Away — P.T. Adamczyk, Olga Jankowska · Deezer preview", "Close"]],
  ["hearth-music-toast-deezer", "deezer", false, gamePage(367520, true), async () => {}, ["Now playing", "Hollow Knight — Christopher Larkin", "Preview from Deezer"]],
  ["hearth-music-toast-gamepage", "apple", false, gamePage(1245620, true), async () => {}, ["Now playing", "Elden Ring — Tsukasa Saitoh", "Preview courtesy of Apple Music · ELDEN RING (Original Soundtrack)"]],
  ["hearth-music-toast-library", "library", false, library(), async () => { await focusTile(1245620); await page.waitForTimeout(1200); }, ["Now playing", "Elden Ring — Tsukasa Saitoh"]],
  ["hearth-music-toast-soundtrack", "soundtrack", false, gamePage(413150, true), async () => {}, ["Now playing", "01 Stardew Valley Overture (the game's soundtrack)"]],
];
for (const [name, scene, panel, bg, act, wants] of shots) {
  const html = page0(scene, panel, bg);
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(1800);
  await act();
  await page.waitForTimeout(400);
  const text = await page.evaluate(() => document.body.innerText);
  // (section titles are drawn in capitals, as Steam does, so innerText has them that way)
  for (const w of wants) if (!text.toLowerCase().includes(w.toLowerCase())) errors.push(`${name}: doesn't show "${w}"`);
  await page.screenshot({ path: join(out, name + ".png") });
  console.log("drew " + name);
}
await browser.close();
if (errors.length) { console.log("::error title=Hearth Music panel::" + errors.join(" | ")); process.exit(1); }
