// CI only (decky-hearth-link.yml): draws Hearth Link's real Quick Access panel (src/index.tsx) in a plain browser, with
// stand-ins for Decky and the backend (mock/: made-up devices, games, copies and Hearth Storage), and saves pictures of
// every state. Steam and Hearth Link aren't involved.
// Usage: HEARTH_ROOT=<repo> node render-panel.mjs OUTDIR   (needs esbuild, react, react-dom, react-icons, playwright next to it)
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const mock = join(root, "hosts/decky-hearth-link/tests/mock");
const out = process.argv[2] || "shots";
mkdirSync(out, { recursive: true });
const res = await build({
  entryPoints: [join(mock, "entry.jsx")], bundle: true, write: false, format: "iife", jsx: "automatic", target: "es2020",
  alias: { "@decky/ui": join(mock, "stub-ui.jsx"), "@decky/api": join(mock, "stub-api.js"), react: join(here, "node_modules/react"),
           "react-dom": join(here, "node_modules/react-dom"), "react-icons": join(here, "node_modules/react-icons") },
  nodePaths: [join(here, "node_modules")], define: { "process.env.NODE_ENV": '"production"' }, logLevel: "warning",
});
const css = `body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:#0c1016;font-family:"Noto Sans",Arial,sans-serif;color:#e9eef4}
body.tall{height:auto;overflow:visible}body.tall .qam{position:relative;margin-left:auto;min-height:800px}body.tall .ScrollPanel{height:auto;overflow:visible}
body::before{content:"";position:absolute;inset:0 460px 0 0;background:linear-gradient(135deg,#1b2838 0%,#0e141b 60%)}
.qam{position:absolute;right:0;top:0;bottom:0;width:460px;display:flex;background:rgba(14,20,27,.98);box-shadow:-8px 0 30px rgba(0,0,0,.5)}
.qam-tabs{width:58px;background:#0b1016;display:flex;flex-direction:column;align-items:center;padding-top:12px;gap:6px}
.qam-tab{width:40px;height:40px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:#8d99a8;font-size:18px}.qam-tab.on{background:#3d4450;color:#fff}
.qam-body{flex:1;display:flex;flex-direction:column;min-width:0}.qam-title{font-size:18px;font-weight:700;padding:16px 16px 8px}
.ScrollPanel{padding:0 12px 20px;overflow-y:auto;height:750px;box-sizing:border-box;scrollbar-width:none}.qam-section{margin-bottom:6px}.qam-section-title{font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:#8d99a8;padding:8px 4px 4px}
.qam-row{padding:3px 4px}.qam-field{display:flex;flex-direction:column;gap:4px;padding:2px 0}.qam-kv{padding:6px 4px;border-bottom:1px solid #1f2731}.qam-label{font-size:14px}.qam-desc{font-size:11.5px;color:#8d99a8}.Text{font-size:14px}
.qam-side .qam-field{padding:0}.qam-side .qam-btn{width:auto;padding:5px 14px}
.DialogButton{border:none;color:#e9eef4;background:#3d4450;border-radius:4px;font:inherit}.DialogButton:disabled{opacity:.45}.qam-btn{width:100%;padding:9px 12px;font-size:14px;text-align:center}.qam-drop{text-align:left;background:#23262e}
.qam-btn.gpfocus{background:#fff;color:#0e141b;opacity:1}.qam-input{background:#0b1016;border:1px solid #3d4450;border-radius:4px;color:#e9eef4;padding:8px;font-size:18px;letter-spacing:.1em}
.qam-line{display:flex;justify-content:space-between;align-items:center;gap:8px}.qam-bar{height:6px;border-radius:3px;background:#273140;overflow:hidden}.qam-bar i{display:block;height:100%;background:#1a9fff}
.qam-toggle{width:auto;padding:5px 14px;font-size:14px;background:#23262e}.qam-toggle.on{background:#1a9fff;color:#fff}
.qam-bar.ind i{background:linear-gradient(90deg,#1a9fff,#67c1ff)}.qam-prog{padding:6px 0}`;
const page0 = (st) => `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body><div id="root"></div><script>window.HL_STATE=${JSON.stringify(st)};</script><script>${res.outputFiles[0].text}</script></body></html>`;
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
const btn = (label) => page.evaluate((l) => { const b = [...document.querySelectorAll(".qam-btn")].find((x) => x.textContent.includes(l)); if (b) b.click(); return !!b; }, label);
const focus = (label) => page.evaluate((l) => {
  document.querySelectorAll(".gpfocus").forEach((x) => x.classList.remove("gpfocus"));
  const b = [...document.querySelectorAll(".qam-btn")].find((x) => x.textContent.includes(l));
  if (b) { b.classList.add("gpfocus"); b.scrollIntoView({ block: "center" }); } }, label);
const show = (title) => page.evaluate((t) => {
  const s = [...document.querySelectorAll(".qam-section-title")].find((x) => x.textContent.startsWith(t));
  if (s) { const p = document.querySelector(".ScrollPanel"); p.scrollTop = s.offsetTop - p.offsetTop - 4; } }, title);
const type = (text) => page.evaluate((t) => { const i = document.querySelector(".qam-input"); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
  set.call(i, t); i.dispatchEvent(new Event("input", { bubbles: true })); }, text);
const wait = (ms) => page.waitForTimeout(ms);
const toggle = (label) => page.evaluate((l) => { const f = [...document.querySelectorAll(".qam-kv")].find((x) => x.querySelector(".qam-label")?.textContent === l);
  const b = f && f.querySelector(".qam-toggle"); if (b) b.click(); return !!b; }, label);
// [picture, state, what to do, what it must show]
const shots = [
  ["hearth-link-devices", "devices", async () => focus("Get a game from Living room PC"),
    ["Hearth Link", "ROG Ally (this device)", "Online · on this network", "Online · over the internet", "Offline · last seen 3 hours ago", "Wake", "Tower · Hearth Storage"]],
  ["hearth-link-get-from-device", "browse", async () => { await btn("Get a game from Living room PC"); await wait(400); await focus("Get Baldur's Gate 3"); },
    ["Get Cyberpunk 2077", "Get Baldur's Gate 3", "150 GB · steam", "Hide Living room PC's games"]],
  ["hearth-link-send-pick", "send", async () => { await show("Send a game"); await btn("Pick a game"); await wait(200); await btn("Pick a device"); await wait(200); await focus("Send"); },
    ["Game on this device", "Hades · 15 GB", "Living room PC"]],
  ["hearth-link-sending", "sending", async () => show("Send a game"), ["Sending Hades to Living room PC", "6.2 GB of 15 GB sent", "can pause or cancel it"]],
  ["hearth-link-send-error", "error", async () => { await show("Send a game"); await btn("Pick a game"); await wait(150); await btn("Pick a device"); await wait(150); await btn("Send"); await wait(400); },
    ["only 80 GB free"]],
  ["hearth-link-getting", "getting", async () => { await show("Copies"); await focus("Pause"); },
    ["Steam Deck wants to send Hollow Knight", "Get it", "No thanks", "Cyberpunk 2077", "Copying · 32 GB of 70 GB · 112 MB/s · 6 min left", "45%", "Paused", "Resume", "Cancel"]],
  ["hearth-link-copied", "done", async () => { await show("Copies"); await focus("Restart Steam to see it"); },
    ["Restart Steam to see it", "after Steam restarts", "Hearth Sync adds it", "Clear finished"]],
  ["hearth-link-storage", "storage", async () => { await show("Hearth Storage"); await focus("Sync now"); },
    ["Hearth Storage", "Tower", "3.2 TB free", "Sync now", "Download once", "2 games to get from Tower · 1 update", "Get Elden Ring", "Baldur's Gate 3", "Saves that follow you", "Hades · synced", "Update cache", "Hearth OS 1.0 kept", "Device health", "4 devices · 4 online", "All good"]],
  ["hearth-link-storage-asleep", "storage-asleep", async () => show("Hearth Storage"),
    ["Asleep or away", "Wake", "Tower is asleep", "battery 14%", "only 40 GB free", "has an update", "last backup 12 days ago", "two different saves"]],
  ["hearth-link-sync-off", "devices", async () => { await show("Sync between your devices"); },
    ["Sync Decky plugins", "Off: plugins and their settings stay", "Saves follow you", "Steam Cloud games already sync through Steam",
     "Sync Hearth settings", "Off: Hearth's settings stay"]],
  ["hearth-link-sync-on", "sync-on", async () => { await show("Sync between your devices"); await focus("Install CSS Loader"); },
    ["never sign-ins", "Install CSS Loader", "Version 2.1.2 · on Living room PC", "Install PowerTools", "on Tower (offline)",
     "Screen modes, sound devices, power and network stay on each device"]],
  ["hearth-link-sync-install", "sync-on", async () => { await show("Sync between your devices"); await btn("Install CSS Loader"); await wait(400); },
    ["Decky asks before it installs CSS Loader"]],
  ["hearth-link-sync-saves-off", "sync-saves-off", async () => { await show("Sync between your devices"); await toggle("Sync Hearth settings"); await wait(400); await show("Sync between your devices"); },
    ["Off: a game's newest save", "Steam Cloud games already sync through Steam", "On: look, theme and colours"]],
  ["hearth-link-pairing-code", "pairing", async () => { await show("Pair a device"); await focus("Stop showing the code"); },
    ["This device's code", "482 913", "4:3", "Internet code", "HRTH-7QK2", "Stop showing the code"]],
  ["hearth-link-pairing-show", "devices", async () => { await show("Pair a device"); await focus("Show this device's code"); }, ["Show this device's code", "Enter an internet code"]],
  ["hearth-link-pairing-join", "pairing-join", async () => { await show("Pair a device"); await btn("Pair with Kitchen laptop"); await wait(200); await type("482913"); await wait(200); await focus("Pair"); await show("Pair a device"); },
    ["Type Kitchen laptop's code below", "6-digit code", "Pair"]],
  ["hearth-link-paired", "paired", async () => show("Pair a device"), ["Paired with Kitchen laptop"]],
  ["hearth-link-playing", "playing", async () => {}, ["A game is running"]],
  ["hearth-link-not-running", "not-running", async () => focus("Start Hearth Link"),
    ["isn't running on this device", "Nothing can be sent", "Start Hearth Link", "42 minutes ago", "Living room PC", "Last seen"]],
  ["hearth-link-not-installed", "not-installed", async () => {}, ["It isn't part of this Hearth OS"]],
  ["hearth-link-home-server", "home-server", async () => show("Home server"),
    ["Home server", "Nvidia-PC", "3 shared folders · 3.1 TB of 6.0 TB used · 2 people", "Last backup 2 hours ago", "91% full",
     "Light mode while you play", "More in Desktop Mode: Settings › Home server"]],
  ["hearth-link-home-server-playing", "home-server-playing", async () => { await show("Home server"); await focus("Let paused jobs run now"); },
    ["Paused for your game: snapshots · backups", "Waiting for your game: Games to backup drive", "Let paused jobs run now", "Only for now: Light mode stays on"]],
  ["hearth-link-home-server-resumed", "home-server-playing", async () => { await show("Home server"); await btn("Let paused jobs run now"); await wait(500); await show("Home server"); },
    ["Paused jobs can run now", "Pause them again while I play"]],
];
for (const [name, st, act, wants] of shots) {
  const html = page0(st);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.setContent(html);
  await wait(900);
  await act();
  await wait(400);
  const text = await page.evaluate(() => document.body.innerText);
  for (const w of wants) if (!text.includes(w)) errors.push(`${name}: the panel doesn't show "${w}"`);
  if (name === "hearth-link-send-pick") {
    const dis = await page.evaluate(() => [...document.querySelectorAll(".qam-btn")].find((b) => b.textContent === "Send")?.disabled);
    if (dis !== false) errors.push(`${name}: Send isn't ready after picking a game and a device`);
  }
  if (name === "hearth-link-sync-off") {
    const on = await page.evaluate(() => [...document.querySelectorAll(".qam-toggle")].map((b) => b.textContent));
    if (on.join() !== "Off,On,Off") errors.push(`${name}: the switches should be Off (plugins), On (saves, as Hearth Link has it), Off (settings): ${on.join()}`);
  }
  if (name === "hearth-link-sync-install") {
    const got = await page.evaluate(() => window.HL_INSTALLED);
    if (!got || got[0] !== "utilities/install_plugin" || !String(got[1]).startsWith("file:///") || got[5] !== 0) errors.push(`${name}: Decky's own install wasn't asked: ${JSON.stringify(got)}`);
  }
  if (name === "hearth-link-not-running") {
    const acts = await page.evaluate(() => [...document.querySelectorAll(".qam-btn")].map((b) => b.textContent));
    if (acts.join() !== "Start Hearth Link") errors.push(`${name}: read only, but has buttons: ${acts.join()}`);
  }
  await page.screenshot({ path: join(out, name + ".png") });
  // The whole panel, top to bottom (Quick Access scrolls; this shows everything at once).
  await page.evaluate(() => document.body.classList.add("tall"));
  const h = await page.evaluate(() => Math.max(800, document.querySelector(".qam").scrollHeight));
  await page.setViewportSize({ width: 1280, height: Math.min(h, 4000) });
  await page.screenshot({ path: join(out, name + "-full.png"), fullPage: true });
  console.log("drew " + name);
}
await browser.close();
if (errors.length) { console.log("::error title=Hearth Link panel::" + errors.join(" | ")); process.exit(1); }
