// CI only (decky-hearth-look.yml): runs Hearth Classic's real plugin (src/) in a plain browser over a stand-in of
// Steam's Game Mode (tests/mock/), checks what it does and saves pictures: the home screen, the skin on a Library page
// (before and after), the Quick Access panel (Steam look, Hearth Classic, paused after a Steam change, not Hearth OS),
// unloading, and the old Hearth Game Mode home (site/img/gm-home.webp) beside the new one. Steam itself isn't involved:
// the frame only stands in for Steam's screens, so how it sits in the real Steam needs a device.
// Usage: HEARTH_ROOT=<repo> node render.mjs OUTDIR   (needs esbuild, react, react-dom, react-icons, playwright next to it)
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const plug = join(root, "hosts/decky-hearth-look");
const mock = join(plug, "tests/mock");
const out = process.argv[2] || "shots";
mkdirSync(out, { recursive: true });
const b64 = (p, mime) => `data:${mime};base64,${readFileSync(p).toString("base64")}`;
const res = await build({
  entryPoints: [join(mock, "entry.jsx")], bundle: true, write: false, format: "iife", jsx: "automatic", target: "es2020",
  alias: { "@decky/ui": join(mock, "stub-ui.jsx"), "@decky/api": join(mock, "stub-api.js"), react: join(here, "node_modules/react"),
           "react-dom": join(here, "node_modules/react-dom"), "react-icons": join(here, "node_modules/react-icons") },
  nodePaths: [join(here, "node_modules")], define: { "process.env.NODE_ENV": '"production"' }, logLevel: "warning",
});
const js = res.outputFiles[0].text;
const css = readFileSync(join(plug, "defaults/hearth-classic.css"), "utf8");
const logo = b64(join(root, "site/img/logo.svg"), "image/svg+xml");
const wall = b64(join(root, "os/branding/wallpapers/hearth-embers.jpg"), "image/jpeg");

// Steam's Game Mode, roughly (its own colours through its --gp variables, which the skin's first layer replaces).
const frame = `body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:var(--gpSystemDarkestGrey,#0e141b);font-family:"Noto Sans",Arial,sans-serif;color:#dcdedf}
.frame{position:absolute;inset:0}.main{position:absolute;left:0;right:0;top:0;bottom:40px;overflow:hidden}
.steam-home,.steam-page{position:absolute;inset:0;background:var(--gpSystemDarkestGrey,#0e141b)}.steam-home-title{padding:60px 28px 0;font-size:22px;font-weight:700}
.gtp_Header{display:flex;gap:8px;padding:56px 28px 10px;background:#0e141b}.gtp_Tab{padding:7px 14px;border-radius:var(--gpCornerSmall,3px);color:#8b929a;font-size:15px}.gtp_Selected{background:#dcdedf;color:#0e141b}
.grid{display:flex;flex-wrap:wrap;gap:16px;padding:20px 28px}.cap{width:150px;height:225px;border-radius:var(--gpCornerSmall,3px)}.cap.gpfocus{outline:3px solid #fff}
.mainmenu{position:absolute;left:0;top:0;bottom:40px;width:300px;background:#0e141b;box-shadow:8px 0 30px rgba(0,0,0,.5);padding:18px 8px;box-sizing:border-box}
.mainmenu-logo{height:34px;margin:0 12px 16px;font-weight:800;letter-spacing:.3em;color:#8b929a;display:flex;align-items:center}.menu-entry{padding:10px 16px;font-size:17px;border-radius:2px}.menu-entry.gpfocus{background:#3d4450}
.qam{position:absolute;right:0;top:0;bottom:40px;width:460px;display:flex;background:var(--gpSystemDarkerGrey,#1a1f27);box-shadow:-8px 0 30px rgba(0,0,0,.5);z-index:5}
.qam-tabs{width:58px;background:#0b1016;display:flex;flex-direction:column;align-items:center;padding-top:12px;gap:6px}
.qam-tab{width:40px;height:40px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:#8d99a8;font-size:18px}.qam-tab.on{background:#3d4450;color:#fff}
.qam-body{flex:1;display:flex;flex-direction:column;min-width:0}.qam-title{font-size:18px;font-weight:700;padding:16px 16px 8px}
.ScrollPanel{padding:0 12px 20px;overflow:hidden}.qam-section{margin-bottom:6px}.qam-section-title{font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:#8d99a8;padding:8px 4px 4px}
.qam-row{padding:3px 4px}.qam-field{display:flex;flex-direction:column;gap:4px;padding:6px 4px}.qam-label{font-size:14px}.qam-desc{font-size:11.5px;color:#8d99a8}.Text{font-size:14px}
.DialogButton{border:none;color:#e9eef4;background:#3d4450;border-radius:2px;font:inherit}.qam-btn,.qam-drop{width:100%;padding:9px 12px;font-size:14px}.qam-drop{display:flex;justify-content:space-between;text-align:left}
.qam-line{display:flex;justify-content:space-between;align-items:center}.qam-switch{width:40px;height:22px;border-radius:11px;background:#273140;position:relative}
.qam-switch i{position:absolute;top:3px;left:3px;width:16px;height:16px;border-radius:50%;background:#e9eef4}.qam-switch.on{background:var(--gpColor-Blue,#1a9fff)}.qam-switch.on i{left:21px}
.lib_Header{padding:16px 28px 0;font-size:18px;font-weight:700;color:#dcdedf}.cap-wrap{border-radius:3px}.cap-wrap.gpfocus{transform:scale(1.04)}
.hero{height:330px;position:relative;background:linear-gradient(135deg,#6a040f,#ffba08)}.ad_Gradient{position:absolute;inset:0;background:linear-gradient(0deg,#0e141b,transparent 70%)}
.hero-title{position:absolute;left:40px;bottom:30px;font-size:42px;font-weight:800;text-shadow:0 2px 12px #000}
.ps_PlayBar{background:#23262e}.bar{display:flex;gap:22px;align-items:center;padding:16px 40px}.play{background:#59bf40;color:#fff;font-weight:700;padding:12px 44px;border-radius:2px;font-size:18px}
.ps_Stat{color:#8b929a;font-size:14px}.menu{width:44px;height:44px;margin-left:auto;font-size:18px}.ad_Content{display:flex;gap:16px;padding:20px 40px}.info{flex:1;background:#23262e;padding:14px 16px;font-size:14px;line-height:1.5}
.mm_Anchor{position:absolute;left:0;top:0;bottom:40px;width:300px;z-index:6;display:flex}.menu-panel{width:300px;height:100%;background:#0e141b;box-shadow:8px 0 30px rgba(0,0,0,.55);display:flex;flex-direction:column}
.mm_NavColumn{display:flex;flex-direction:column;padding:18px 8px}.mm_NavBox{display:flex;flex-direction:column}.mm_Item{padding:10px 16px;font-size:17px;color:#dcdedf;border-radius:2px}.mm_Item.gpfocus{background:#3d4450}
.mm_AppColumn{padding:16px;border-bottom:1px solid #23262e}.mm_GameBg{height:60px;background:linear-gradient(135deg,#6a040f,#ffba08);margin-bottom:8px}.game-logo{font-weight:800;font-size:18px;margin-bottom:6px}
.steam-footer{position:absolute;left:0;right:0;bottom:0;height:40px;display:flex;align-items:center;justify-content:flex-end;padding:0 24px;background:#0e141b;color:#8b929a;font-size:13px;word-spacing:6px}`;

const browser = await chromium.launch();
const errors = [];
// A fresh page for every scene: the plugin keeps module state and timers (the skin's refresh), which setContent on the
// same page would carry over into the next scene.
let page = null;
async function freshPage(w = 1280, h = 800) {
  if (page) await page.close();
  page = await browser.newPage({ viewport: { width: w, height: h } });
  page.on("pageerror", (e) => errors.push(String(e)));
}

/** Presses each place on the Hearth home; what each used (model.ts runPlace). */
async function places() {
  const got = {};
  for (const t of ["Library", "Store", "Downloads", "Media", "Friends", "Settings"]) {
    got[t] = await page.evaluate((t) => { const b = [...document.querySelectorAll(".hc-chip")].find((x) => x.textContent === t); if (!b) return "missing"; b.click(); return window.HC_LAST_PLACE; }, t);
  }
  return got;
}

async function scene(name, opts, check) {
  const vars = { HC_SCENE: opts.scene || "home", HC_SETTINGS: opts.settings || {}, HC_STEAM: opts.steam || "", HC_HEARTH: opts.hearth !== false,
                 HC_QAM: opts.qam !== false, HC_MENU: !!opts.menu, HC_RUNNING: !!opts.running, HC_NAVIGATOR: opts.navigator || "",
                 HC_CSS: css, HC_LOGO: logo, HC_WALL: wall };
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${frame}</style></head><body><div id="root"></div>
<script>${Object.entries(vars).map(([k, v]) => `window.${k}=${JSON.stringify(v)};`).join("")}</script><script>${js}</script></body></html>`;
  await freshPage();
  await page.setContent(html);
  await page.waitForTimeout(opts.wait || 2500);
  if (opts.focus) await page.evaluate((sel) => { const e = document.querySelector(sel); if (e) e.classList.add("gpfocus"); }, opts.focus);
  await page.waitForTimeout(250);
  const text = (await page.evaluate(() => document.body.innerText)).toLowerCase();
  const style = await page.evaluate(() => document.getElementById("hearth-classic")?.textContent || "");
  const missing = (opts.want || []).filter((w) => !text.includes(w.toLowerCase()));
  const present = (opts.not || []).filter((w) => text.includes(w.toLowerCase()));
  if (missing.length) errors.push(`${name}: missing ${missing.join(", ")}`);
  if (present.length) errors.push(`${name}: shouldn't show ${present.join(", ")}`);
  if (opts.skin === true && !style.includes(".gtp_Header{")) errors.push(`${name}: the skin isn't applied`);
  if (opts.skin === false && style) errors.push(`${name}: the skin is applied but shouldn't be`);
  await page.screenshot({ path: join(out, name + ".png") });
  if (check) await check(text);
  console.log(`drew ${name}`);
}

// 1. Steam's look (the default): nothing applied; then the person picks Hearth Classic in the panel.
await scene("panel-steam-look", { want: ["Game Mode look", "Steam's own Home", "Pick Hearth Classic"], not: ["Hearth home screen"], skin: false }, async () => {
  if (await page.evaluate(() => window.HC_PATCHES.size)) errors.push("panel-steam-look: a route patch was added with the Steam look");
  await page.click(".qam-drop");
  await page.waitForTimeout(2500);
  const s = await page.evaluate(() => !!document.getElementById("hearth-classic"));
  const t = await page.evaluate(() => document.body.innerText);
  if (!s || !t.includes("Jump back in")) errors.push("panel-steam-look: picking Hearth Classic didn't apply it");
  await page.screenshot({ path: join(out, "panel-hearth-classic.png") });
});
// 2. Hearth Classic's home on Steam's Home route (no Quick Access), the first tile focused; then unloading.
await scene("home-hearth-classic", { settings: { look: "classic" }, qam: false, wait: 3500, focus: ".hc-shelf .hc-tile", skin: true,
  want: ["Jump back in", "Starfall Drift", "Continue playing", "Favorites", "Steam", "Epic", "GOG", "Library", "Store", "Downloads", "Play", "Details"],
  not: ["Not In Steam Yet", "Steam's own Home"] }, async () => {
  const tiles = await page.locator(".hc-tile").count();
  if (tiles < 15) errors.push(`home-hearth-classic: only ${tiles} tiles`);
  const imgs = await page.evaluate(() => [...document.querySelectorAll(".hc-tile img.hc-art")].filter((i) => i.complete && i.naturalWidth > 0).length);
  if (imgs < tiles) errors.push(`home-hearth-classic: ${tiles - imgs} tiles without a picture`);
  await page.click(".hc-shelf .hc-tile");
  const nav = await page.evaluate(() => window.HC_NAV || []);
  if (!nav.includes("/library/app/3000000001")) errors.push(`home-hearth-classic: the first tile didn't open Steam's page (${nav})`);
  // Hearth Music's own focus reader sees the game of a focused home tile.
  const seen = await page.evaluate(() => window.HC_MUSIC_APPID(document.querySelectorAll(".hc-shelf .hc-tile")[1]));
  if (seen !== 1001) errors.push(`music: Hearth Music's appIdOf read ${seen} for Ember Knights' tile, not 1001`);
  // Every place opens Steam's own page through its navigator.
  const want = { Library: "route:/library", Store: "navigator:SteamWeb", Downloads: "navigator:Downloads", Media: "navigator:Media.Grid", Friends: "navigator:Chat", Settings: "navigator:Settings" };
  const got = await places();
  for (const [k, v] of Object.entries(want)) if (got[k] !== v) errors.push(`places: ${k} used ${got[k]}, not ${v}`);
  const calls = await page.evaluate(() => window.HC_NAVCALLS);
  console.log(`::notice title=Hearth Classic places::${JSON.stringify(got)} calls: ${calls.join(", ").slice(0, 600)}`);
  // Unloading (Decky's onDismount) takes everything out: the skin, the route patch; Steam's Home is back.
  await page.evaluate(() => window.HC_PLUGIN.onDismount());
  await page.waitForTimeout(800);
  const left = await page.evaluate(() => ({ style: !!document.getElementById("hearth-classic"), patches: window.HC_PATCHES.size, text: document.body.innerText }));
  if (left.style || left.patches || !left.text.includes("Steam's own Home")) errors.push(`unload: something stayed (${JSON.stringify({ style: left.style, patches: left.patches })})`);
  else console.log("::notice title=Hearth Classic unload::the skin and the home patch were removed; Steam's Home is back");
});
// 3. The skin on a Library page with the main menu open: Steam's look, then Hearth Classic.
await scene("skin-steam-library", { scene: "library", qam: false, skin: false });
await scene("skin-hearth-classic-library", { scene: "library", qam: false, settings: { look: "classic" }, skin: true }, async () => {
  const bg = await page.evaluate(() => getComputedStyle(document.querySelector(".gtp_Selected")).backgroundColor);
  if (bg !== "rgb(63, 208, 184)") errors.push(`skin: the selected tab isn't Hearth's accent (${bg})`);
});
// 3b. A Steam whose navigator lacks the places' calls: every place still opens a page (the Library), never an empty one.
await scene("places-fallback", { settings: { look: "classic" }, qam: false, navigator: "missing", skin: true }, async () => {
  const got = await places();
  for (const [k, v] of Object.entries(got)) if (!(v === "route:/library" || (k === "Settings" && v === "route:/settings"))) errors.push(`places-fallback: ${k} used ${v}`);
});
// 3c. A game page: Steam's look, then Hearth Classic.
await scene("game-steam", { scene: "game", qam: false, skin: false });
await scene("game-hearth-classic", { scene: "game", qam: false, settings: { look: "classic" }, skin: true }, async () => {
  const bg = await page.evaluate(() => getComputedStyle(document.querySelector(".ps_Play")).backgroundColor);
  if (bg !== "rgb(110, 226, 207)" && bg !== "rgb(63, 208, 184)") errors.push(`game page: Play isn't Hearth's accent (${bg})`);
});
// 3d. The Steam button's menu: Steam's, Hearth's bottom bar, Hearth's top bar in a game, and a Steam without the menu's names.
await scene("menu-steam", { scene: "library", qam: false, menu: true, skin: false });
await scene("menu-bottom-bar", { scene: "library", qam: false, menu: true, settings: { look: "classic" }, skin: true }, async () => {
  const r = await page.evaluate(() => ({ icons: [...document.querySelectorAll(".mm_Item")].map((e) => e.getAttribute("data-hearth-icon")),
    dir: getComputedStyle(document.querySelector(".mm_NavColumn")).flexDirection, top: document.querySelector(".mm_NavColumn").getBoundingClientRect().top,
    ingame: document.documentElement.classList.contains("hearth-ingame") }));
  if (r.dir !== "row" || r.top < 500) errors.push(`menu-bottom-bar: not a bar along the bottom (${r.dir}, top ${r.top})`);
  if (r.icons.join(",") !== "home,lib,shop,media,dl,friends,gear,power") errors.push(`menu-bottom-bar: icons ${r.icons}`);
  if (r.ingame) errors.push("menu-bottom-bar: marked as in a game");
});
await scene("menu-top-bar-in-game", { scene: "game", qam: false, menu: true, running: true, settings: { look: "classic" }, skin: true }, async () => {
  const r = await page.evaluate(() => ({ ingame: document.documentElement.classList.contains("hearth-ingame"), top: document.querySelector(".mm_NavColumn").getBoundingClientRect().top,
    icons: [...document.querySelectorAll(".mm_Item")].map((e) => e.getAttribute("data-hearth-icon")) }));
  if (!r.ingame || r.top > 150) errors.push(`menu-top-bar-in-game: not a bar at the top (${JSON.stringify(r)})`);
  if (!r.icons.includes("pad") || !r.icons.includes("close")) errors.push(`menu-top-bar-in-game: Resume and Exit game have no icons (${r.icons})`);
});
await scene("menu-paused-steam-changed", { scene: "library", menu: true, steam: "menuchanged", settings: { look: "classic" }, skin: true,
  want: ["Hearth bottom bar menu paused"] }, async () => {
  const n = await page.evaluate(() => document.querySelectorAll("[data-hearth-icon]").length);
  if (n) errors.push("menu-paused-steam-changed: menu icons added though the menu's names are missing");
  const dir = await page.evaluate(() => getComputedStyle(document.querySelector(".mm_NavColumn")).flexDirection);
  if (dir !== "column") errors.push("menu-paused-steam-changed: Steam's menu was restyled anyway");
});
// 4. The Quick Access panel with Hearth Classic on.
await scene("panel-hearth-classic-on", { settings: { look: "classic" }, want: ["Hearth home screen", "Hearth wallpaper", "Hearth bottom bar menu", "In a game: Hearth top bar", "Hearth sounds", "Jump back in"], skin: true });
// 5. A Steam update renamed its modules: paused, Steam's normal look and Home, the note, the log line.
await scene("panel-paused", { settings: { look: "classic" }, steam: "changed", skin: false,
  want: ["Hearth Classic paused: Steam changed its layout", "Try again", "Steam's own Home"] }, async () => {
  if (await page.locator(".hearth-classic").count()) errors.push("panel-paused: the Hearth home is drawn while paused");
  const reps = await page.evaluate(() => window.HC_REPORTS);
  if (!reps.some((r) => r.includes("paused"))) errors.push("panel-paused: the pause wasn't logged");
  if (await page.evaluate(() => window.HC_PATCHES.size)) errors.push("panel-paused: a route patch stayed");
});
// 6. Not Hearth OS: the shared invitation, nothing applied even with Hearth Classic saved.
await scene("panel-not-hearth-os", { settings: { look: "classic" }, hearth: false, skin: false, want: ["Get Hearth OS", "Steam's own Home"], not: ["Hearth home screen", "Try again", "Pick Hearth Classic"] });

// 7. The old Hearth Game Mode home beside Hearth Classic's.
await freshPage(2600, 900);
const oldShot = b64(join(root, "site/img/gm-home.webp"), "image/webp");
const newShot = b64(join(out, "home-hearth-classic.png"), "image/png");
await page.setContent(`<!doctype html><html><body style="margin:0;background:#0c1016;color:#e9eef4;font-family:'Noto Sans',Arial,sans-serif;display:flex;gap:40px;padding:30px;box-sizing:border-box">
<figure style="margin:0"><figcaption style="font-size:22px;margin-bottom:12px">The old Hearth Game Mode (site/img/gm-home.webp)</figcaption><img src="${oldShot}" style="width:1240px;height:775px;object-fit:cover;border-radius:10px"></figure>
<figure style="margin:0"><figcaption style="font-size:22px;margin-bottom:12px">Hearth Classic on Steam's Home (stand-in frame, CI)</figcaption><img src="${newShot}" style="width:1240px;height:775px;border-radius:10px"></figure></body></html>`);
await page.waitForTimeout(500);
await page.screenshot({ path: join(out, "compare-old-and-new-home.png") });
writeFileSync(join(out, "README.txt"), "Hearth Classic's real plugin code over a stand-in of Steam's Game Mode (tests/mock). Not Steam itself.\n");
await browser.close();
if (errors.length) { console.log(`::error title=Hearth Classic pictures::${errors.join(" | ").slice(0, 3000)}`); process.exit(1); }
console.log("::notice title=Hearth Classic pictures::all scenes drawn and checked");
