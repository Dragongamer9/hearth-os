// CI only (decky-hearth-sync.yml): draws Hearth Sync's real Quick Access panel (src/index.tsx) in a plain browser, with a
// stand-in for Decky (mock/) and made-up games, and saves a picture. Steam isn't involved.
// Usage: HEARTH_ROOT=<repo> node render-panel.mjs OUTDIR   (needs esbuild, react, react-dom, react-icons, playwright next to it)
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const mock = join(root, "hosts/decky-hearth-sync/tests/mock");
const out = process.argv[2] || "shots";
mkdirSync(out, { recursive: true });
const res = await build({
  entryPoints: [join(mock, "entry.jsx")], bundle: true, write: false, format: "iife", jsx: "automatic", target: "es2020",
  alias: { "@decky/ui": join(mock, "stub-ui.jsx"), "@decky/api": join(mock, "stub-api.js"), react: join(here, "node_modules/react"),
           "react-dom": join(here, "node_modules/react-dom"), "react-icons": join(here, "node_modules/react-icons") },
  nodePaths: [join(here, "node_modules")], define: { "process.env.NODE_ENV": '"production"' }, logLevel: "warning",
});
const css = `body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:#0c1016;font-family:"Noto Sans",Arial,sans-serif;color:#e9eef4}
.qam{position:absolute;right:0;top:0;bottom:0;width:460px;display:flex;background:rgba(14,20,27,.98);box-shadow:-8px 0 30px rgba(0,0,0,.5)}
.qam-tabs{width:58px;background:#0b1016;display:flex;flex-direction:column;align-items:center;padding-top:12px;gap:6px}
.qam-tab{width:40px;height:40px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:#8d99a8;font-size:18px}.qam-tab.on{background:#3d4450;color:#fff}
.qam-body{flex:1;display:flex;flex-direction:column;min-width:0}.qam-title{font-size:18px;font-weight:700;padding:16px 16px 8px}
.ScrollPanel{padding:0 12px 20px;overflow:hidden}.qam-section{margin-bottom:6px}.qam-section-title{font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:#8d99a8;padding:8px 4px 4px}
.qam-row{padding:3px 4px}.qam-field{display:flex;flex-direction:column;gap:4px;padding:2px 0}.qam-label{font-size:14px}.qam-desc{font-size:11.5px;color:#8d99a8}.Text{font-size:14px}
.DialogButton{border:none;color:#e9eef4;background:#3d4450;border-radius:4px;font:inherit}.qam-btn{width:100%;padding:9px 12px;font-size:14px}.qam-btn.gpfocus{background:#fff;color:#0e141b}.qam-switch.gpfocus{outline:2px solid #fff;outline-offset:2px}
.qam-line{display:flex;justify-content:space-between;align-items:center}.qam-switch{width:40px;height:22px;border-radius:11px;background:#273140;position:relative}
.qam-switch i{position:absolute;top:3px;left:3px;width:16px;height:16px;border-radius:50%;background:#e9eef4}.qam-switch.on{background:#1a9fff}.qam-switch.on i{left:21px}`;
const page0 = (st) => `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body><div id="root"></div><script>window.HS_STATE=${JSON.stringify(st)};</script><script>${res.outputFiles[0].text}</script></body></html>`;
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
const btn = (label) => page.evaluate((l) => { const b = [...document.querySelectorAll(".qam-btn")].find((x) => x.textContent.includes(l)); if (b) b.click(); return !!b; }, label);
const focus = (label) => page.evaluate((l) => { document.querySelectorAll(".gpfocus").forEach((x) => x.classList.remove("gpfocus"));
  const b = [...document.querySelectorAll(".qam-btn")].find((x) => x.textContent.includes(l))
    || [...document.querySelectorAll(".qam-field")].find((x) => x.textContent.includes(l))?.querySelector(".qam-switch");
  if (b) b.classList.add("gpfocus"); }, label);
// Shows the panel from a section on (the sections above it hidden, as if scrolled down: the import section is below
// the first screenful).
const scrollTo = (title) => page.evaluate((t) => { const secs = [...document.querySelectorAll(".qam-section")];
  const i = secs.findIndex((x) => x.querySelector(".qam-section-title")?.textContent === t);
  secs.forEach((x, j) => { x.style.display = j < i ? "none" : ""; }); }, title);
// [picture, state, what to do, what it must show]
const shots = [
  ["hearth-sync-panel", "synced", async () => focus("Sync now"), ["Hearth Sync", "10 games synced", "4 minutes ago", "Sync now", "Epic", "GOG", "Battle.net", "Emulators", "Recomps", "Remove all Hearth Sync shortcuts"]],
  ["hearth-sync-panel-syncing", "syncing", async () => { await btn("Sync now"); await page.waitForTimeout(500); await focus("Syncing"); }, ["Syncing…"]],
  ["hearth-sync-panel-problem", "problem", async () => focus("Sync now"), ["turned itself off", "missing a picture", "Steam didn't take", "2 hours ago"]],
  ["hearth-sync-panel-stores", "toggles", async () => focus("Recomps"), ["7 games synced", "Recomps", "Added by hand"]],
  ["hearth-sync-panel-import", "import", async () => { await scrollTo("Other Steam folders"); await focus("Bring them into Steam"); }, ["4 games Steam doesn't see yet", "Cyberpunk 2077", "moved (same drive)", "its drive's library added", "Red Dead Redemption 2: skipped", "Bring them into Steam"]],
  ["hearth-sync-panel-import-confirm", "import-confirm", async () => { await btn("Bring them"); await page.waitForTimeout(300); await scrollTo("Other Steam folders"); await focus("Press again"); }, ["Press again: Steam restarts"]],
  ["hearth-sync-panel-import-waiting", "import-waiting", async () => { await scrollTo("Other Steam folders"); }, ["Waiting for Steam to close"]],
  ["hearth-sync-panel-import-done", "import-done", async () => { await scrollTo("Other Steam folders"); }, ["Last import: 3 moved, 1 from 1 added library folder, 2 skipped."]],
  ["hearth-sync-panel-import-none", "synced", async () => { await scrollTo("Other Steam folders"); }, ["No Steam games in other Steam folders (1 Steam already has)"]],
  ["hearth-sync-panel-not-hearth", "not-hearth", async () => focus("Get Hearth OS"), ["part of Hearth OS", "Steam Game Mode, like SteamOS", "Get Hearth OS", "Free and open source"]],
  // Which Game Mode (src/gamemode.tsx, docs/HEARTH-GAME-MODE.md): Steam Game Mode in use by default; picking Hearth Game
  // Mode offers Switch now; when Hearth Game Mode didn't start, why and Try again.
  ["hearth-sync-panel-game-mode", "synced", async () => { await scrollTo("Game Mode"); await focus("Hearth Game Mode"); }, ["Game Mode", "✓ Steam Game Mode (in use, default)", "Hearth Game Mode", "Applies the next time Game Mode starts"]],
  ["hearth-sync-panel-game-mode-picked", "synced", async () => { await btn("Hearth Game Mode"); await page.waitForTimeout(300); await scrollTo("Game Mode"); await focus("Switch to Hearth"); }, ["✓ Hearth Game Mode (in use)", "Switch to Hearth Game Mode now"]],
  ["hearth-sync-panel-game-mode-fallback", "gamemode-fallback", async () => { await scrollTo("Game Mode"); await focus("Try Hearth"); }, ["Hearth Game Mode didn't start", "3 times within 2 minutes", "Your choice is still Hearth Game Mode", "Try Hearth Game Mode again"]],
  ["hearth-sync-panel-steam-game", "steam-game", async () => { await scrollTo("Game Mode"); await focus("Back to Hearth"); }, ["Back to Hearth Game Mode", "You're here for one Steam game"]],
  ["hearth-sync-panel-remove-confirm", "synced", async () => { await btn("Remove all"); await page.waitForTimeout(300); await focus("Press again"); }, ["Press again to remove them", "your other non-Steam games stay"]],
];
for (const [name, st, act, wants] of shots) {
  const html = page0(st);
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(1200);
  await act();
  await page.waitForTimeout(300);
  const text = await page.evaluate(() => document.body.innerText);
  for (const w of wants) if (!text.includes(w)) errors.push(`${name}: the panel doesn't show "${w}"`);
  if (st === "toggles") {
    const off = await page.evaluate(() => [...document.querySelectorAll(".qam-field")].filter((f) => f.querySelector(".qam-switch") && !f.querySelector(".qam-switch.on")).map((f) => f.querySelector(".qam-label").textContent));
    if (off.join(",") !== "Added by hand,Emulators,Recomps") errors.push(`${name}: switches off: ${off.join(",")}`);
  }
  await page.screenshot({ path: join(out, name + ".png") });
  console.log("drew " + name);
}

// Steam's library with Hearth Sync's shortcuts, over a static stand-in (styled roughly like Steam's gamepad library; the
// covers are made-up gradients with the titles, as Hearth's art would give). Shows where the shortcuts and the
// per-store collections end up, not that Steam draws them this way.
const PAL = [["#3a1c71", "#d76d77"], ["#0f2027", "#2c5364"], ["#134e5e", "#71b280"], ["#41295a", "#2f0743"], ["#c04848", "#480048"], ["#1f4037", "#99f2c8"], ["#373b44", "#4286f4"], ["#8e2de2", "#4a00e0"], ["#ff512f", "#6b1d05"], ["#16222a", "#3a6073"]];
const cover = (t, i) => `<div class="cap" style="background:linear-gradient(160deg,${PAL[i % 10][0]},${PAL[i % 10][1]})"><b>${t}</b></div>`;
const lib = [["Alan Wake 2", "Epic"], ["Fortnite", "Epic"], ["Cyberpunk 2077", "GOG"], ["The Witcher", "GOG"], ["Celeste", "GOG"], ["Diablo IV", "Battle.net"],
  ["Xbox Cloud Gaming", "Xbox"], ["Super Mario World", "Emulators"], ["Zelda 64", "Recomps"], ["BFME II", "Beacon"], ["Pixel Arena", "Added by hand"]];
const cols = [...new Set(lib.map((g) => g[1]))];
const steamLib = (tab) => `<!doctype html><html><head><meta charset="utf-8"><style>
body{margin:0;width:1280px;height:800px;overflow:hidden;background:#0e141b;color:#dcdedf;font-family:"Noto Sans",Arial,sans-serif}
.top{display:flex;gap:10px;padding:16px 28px;background:#171d25;font-size:15px}.tab{padding:6px 14px;border-radius:4px;color:#8b929a}.tab.on{background:#dcdedf;color:#0e141b}
h2{font-size:13px;letter-spacing:.06em;text-transform:uppercase;color:#8b929a;margin:20px 28px 10px}.grid{display:flex;flex-wrap:wrap;gap:16px;padding:0 28px}
.cap{width:150px;height:225px;border-radius:4px;display:flex;align-items:flex-end;padding:10px;box-sizing:border-box;font-size:15px;box-shadow:0 4px 14px rgba(0,0,0,.4)}
.cap b{text-shadow:0 1px 4px #000}.cap.f{outline:3px solid #fff}.col{width:220px;height:124px;border-radius:6px;background:#23262e;display:flex;flex-direction:column;justify-content:flex-end;padding:12px;box-sizing:border-box}
.col small{color:#8b929a}.note{position:absolute;bottom:10px;right:20px;font-size:11px;color:#67707b}</style></head><body>
<div class="top"><span class="tab">Great on Deck</span><span class="tab">All games</span><span class="tab${tab === "nonsteam" ? " on" : ""}">Non-Steam</span><span class="tab${tab === "collections" ? " on" : ""}">Collections</span></div>
${tab === "nonsteam" ? `<h2>Non-Steam · ${lib.length} games from Hearth Sync</h2><div class="grid">${lib.map((g, i) => cover(g[0], i).replace('class="cap"', i === 2 ? 'class="cap f"' : 'class="cap"')).join("")}</div>`
  : `<h2>Your collections</h2><div class="grid">${cols.map((c) => `<div class="col"><b>${c}</b><small>${lib.filter((g) => g[1] === c).length} games</small></div>`).join("")}</div>
     <h2>GOG</h2><div class="grid">${lib.filter((g) => g[1] === "GOG").map((g, i) => cover(g[0], i + 2)).join("")}</div>`}
<div class="note">Static stand-in for Steam's library (not Steam itself)</div></body></html>`;
for (const tab of ["nonsteam", "collections"]) {
  await page.setContent(steamLib(tab));
  await page.screenshot({ path: join(out, `steam-library-${tab}.png`) });
  console.log("drew steam-library-" + tab);
}
await browser.close();
if (errors.length) { console.log("::error title=Hearth Sync panel::" + errors.join(" | ")); process.exit(1); }
