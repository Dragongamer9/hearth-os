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
.dlg-page{display:flex}.dlg-side{width:260px;padding:60px 12px;background:#0e141b}.dlg-tab{padding:10px 14px;color:#8b929a;font-size:15px}.dlg-tab.on{color:#fff;background:#23262e}
.dlg{flex:1;padding:56px 32px;background:var(--gpSystemDarkerGrey,#1a1f27)}.dlg-title{font-size:24px;font-weight:700;margin-bottom:16px}.dlg-row{display:flex;justify-content:space-between;align-items:center;padding:14px 16px;border-bottom:1px solid #23262e}
.slider{width:220px;height:6px;background:#3d4450;border-radius:3px}.slider b{display:block;height:6px;background:var(--gpColor-Blue,#1a9fff);border-radius:3px}.dlg-btns{display:flex;gap:12px;justify-content:flex-end;margin-top:20px}.dlg-btns .gd_Button{padding:10px 26px}
.qam-toggle.on .qam-switch{background:var(--gpColor-Blue,#1a9fff)}.qam-toggle.on .qam-switch i{left:21px}
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
                 HC_QAM: opts.qam !== false, HC_MENU: opts.menu || "", HC_MENUWIDTH: opts.menuWidth || 0, HC_MENURESIZE: opts.resize || "", HC_ROUTE: opts.route || "", HC_COVER: !!opts.cover, HC_RUNNING: !!opts.running, HC_NAVIGATOR: opts.navigator || "",
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
  if (!s || !t.toLowerCase().includes("recent games")) errors.push("panel-steam-look: picking Hearth Classic didn't apply it");
  await page.screenshot({ path: join(out, "panel-hearth-classic.png") });
});
// 2. Hearth Classic's home on Steam's Home route (no Quick Access), the first tile focused; then unloading.
await scene("home-hearth-classic", { settings: { look: "classic" }, qam: false, wait: 3500, focus: ".hc-shelf .hc-tile", skin: true,
  want: ["Home", "Recent games", "Starfall Drift", "Favorites", "Steam", "Epic", "GOG", "Library", "Store", "Downloads"],
  not: ["Not In Steam Yet", "Steam's own Home"] }, async () => {
  const tiles = await page.locator(".hc-tile").count();
  if (tiles < 15) errors.push(`home-hearth-classic: only ${tiles} tiles`);
  const imgs = await page.evaluate(() => [...document.querySelectorAll(".hc-tile img.hc-art")].filter((i) => i.complete && i.naturalWidth > 0).length);
  if (imgs < tiles) errors.push(`home-hearth-classic: ${tiles - imgs} tiles without a picture`);
  await page.click(".hc-shelf .hc-tile");
  const nav = await page.evaluate(() => window.HC_NAV || []);
  if (!nav.includes("/library/app/3000000001")) errors.push(`home-hearth-classic: the first tile didn't open Steam's page (${nav})`);
  // No game music from the home itself: ThemeDeck plays music on Steam's game pages (docs/THEMEDECK.md).
  if (await page.locator("[data-hearth-appid]").count()) errors.push("home-hearth-classic: tiles still marked for Hearth Music");
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
  if (bg !== "rgb(233, 238, 244)") errors.push(`skin: the selected tab isn't Hearth's light pill (${bg})`);
});
// 3b. A Steam whose navigator lacks the places' calls: every place still opens a page (the Library), never an empty one.
await scene("places-fallback", { settings: { look: "classic" }, qam: false, navigator: "missing", skin: true }, async () => {
  const got = await places();
  // The Store still has the window's own NavigateToSteamWeb in this Steam; the rest fall back to a page.
  const ok = { Library: "route:/library", Store: "window:NavigateToSteamWeb", Downloads: "route:/library", Media: "route:/library", Friends: "route:/library", Settings: "route:/settings" };
  for (const [k, v] of Object.entries(got)) if (ok[k] !== v) errors.push(`places-fallback: ${k} used ${v}`);
});
// 3c. A game page: Steam's look, then Hearth Classic.
await scene("game-steam", { scene: "game", qam: false, skin: false });
await scene("game-hearth-classic", { scene: "game", qam: false, settings: { look: "classic" }, skin: true }, async () => {
  const bg = await page.evaluate(() => getComputedStyle(document.querySelector(".ps_Play")).backgroundColor);
  if (bg !== "rgb(110, 226, 207)" && bg !== "rgb(63, 208, 184)") errors.push(`game page: Play isn't Hearth's accent (${bg})`);
});
// 3d. The Steam button's menu, in its own window (an iframe standing in for Steam's MainMenu popup): Steam's, Hearth's
// bottom bar, Hearth's top bar in a game, a narrow menu window (the bar doesn't fit: Hearth's colours on Steam's layout),
// and a Steam that changed its menu (Steam's own menu, untouched).
const menuState = () => page.evaluate(() => {
  const f = document.getElementById("menuwin"), d = f.contentDocument, html = d.documentElement;
  const items = [...d.querySelectorAll("#MainNavMenuContainer .Focusable.mm_entry")];
  const r = items.map((e) => e.getBoundingClientRect()), fr = f.getBoundingClientRect();
  const mir = document.getElementById("hearth-bar-mirror");
  const anchor = document.querySelector(".mm_Anchor");
  return { cls: html.className, icons: items.map((e) => e.getAttribute("data-hearth-icon")), tops: r.map((x) => Math.round(x.top)),
           win: { left: Math.round(fr.left), top: Math.round(fr.top), width: Math.round(fr.width), height: Math.round(fr.height) },
           style: !!d.getElementById("hearth-classic"), dir: getComputedStyle(items[0].parentElement.parentElement).flexDirection,
           panelOpacity: getComputedStyle(items[0].parentElement.parentElement).opacity, anchorOpacity: anchor ? getComputedStyle(anchor).opacity : "",
           diag: window.HearthClassicMenu || null,
           mirror: mir && mir.style.display !== "none" ? { cls: mir.className, n: mir.querySelectorAll(".hbm-item").length,
             on: [...mir.querySelectorAll(".hbm-item")].findIndex((x) => x.classList.contains("on")), top: Math.round(mir.querySelector(".hbm-bar").getBoundingClientRect().top) } : null };
});
// The owner's device: Steam's menu window is a 241 px popup at the left. A: its window resized to a bar.
await scene("menu-steam", { scene: "library", qam: false, menu: "steam", menuWidth: 241, skin: false });
await scene("menu-bottom-bar", { scene: "library", qam: false, menu: "steam", menuWidth: 241, resize: "steamclient", settings: { look: "classic" }, skin: true, wait: 3500 }, async () => {
  const m = await menuState();
  console.log(`::notice title=menu-bottom-bar diag::${JSON.stringify(await page.evaluate(() => window.HearthClassicMenu)).slice(0, 1500)}`);
  if (!m.style) errors.push("menu-bottom-bar: the skin didn't reach the menu's own window");
  if (m.win.width !== 1280 || m.win.height !== 150 || m.win.top !== 650) errors.push(`menu-bottom-bar: the menu window wasn't made a bar at the bottom (${JSON.stringify(m.win)})`);
  if (!/hearth-bar/.test(m.cls) || m.dir !== "row" || Math.max(...m.tops) - Math.min(...m.tops) > 24) errors.push(`menu-bottom-bar: not a bar (${m.cls}, ${m.dir}, ${m.tops})`);
  if (m.icons.join(",") !== "home,lib,shop,friends,media,dl,gear,power") errors.push(`menu-bottom-bar: icons ${m.icons}`);
  if (m.anchorOpacity !== "0") errors.push(`menu-bottom-bar: the main window's empty menu place is still showing (${m.anchorOpacity})`);
  if (!m.diag || !/^passed/.test(m.diag.selfCheck)) errors.push(`menu-bottom-bar: self-check ${m.diag && m.diag.selfCheck}`);
  if (!m.diag || !/^SteamClient\.Window \(CSS px, scale 1\)$/.test(m.diag.resizeWith)) errors.push(`menu-bottom-bar: resized with ${m.diag && m.diag.resizeWith}`);
  // Unloading puts the menu window back where Steam had it.
  await page.evaluate(() => window.HC_PLUGIN.onDismount());
  await page.waitForTimeout(600);
  const back = await menuState();
  if (back.anchorOpacity !== "1") errors.push("menu-bottom-bar: unloading didn't bring back the main window's menu place");
  if (back.win.width !== 241 || back.win.top !== 0 || /hearth-/.test(back.cls)) errors.push(`menu-bottom-bar: unloading didn't restore Steam's menu window (${JSON.stringify(back.win)}, ${back.cls})`);
});
await scene("menu-top-bar-in-game", { scene: "game", qam: false, menu: "steam", menuWidth: 241, resize: "steamclient", running: true, settings: { look: "classic" }, skin: true, wait: 3500 }, async () => {
  const m = await menuState();
  if (m.win.width !== 1280 || m.win.top !== 0 || !/hearth-ingame/.test(m.cls) || !/hearth-bar/.test(m.cls)) errors.push(`menu-top-bar-in-game: not a bar at the top (${JSON.stringify(m.win)}, ${m.cls})`);
});
// The Store (a web page above Steam's other windows): Steam's menu in its own place, in Hearth's colours.
await scene("menu-in-store", { scene: "library", qam: false, menu: "steam", menuWidth: 241, resize: "steamclient", route: "/steamweb", settings: { look: "classic" }, skin: true, wait: 3500 }, async () => {
  // Steam's menu window shows above the Store: with a working resize, the real bar there too.
  const m = await menuState();
  if (m.win.width !== 1280 || m.win.top !== 650 || !/hearth-bar/.test(m.cls) || m.mirror) errors.push(`menu-in-store: expected the resized bar on the Store (${JSON.stringify(m.win)}, ${m.cls})`);
});
await scene("menu-rail-on-store", { scene: "library", qam: false, menu: "steam", menuWidth: 241, route: "/steamweb", settings: { look: "classic" }, skin: true, wait: 3500 }, async () => {
  // No resize on the Store: the mirror would be under the page, so the side rail in Steam's menu window.
  const m = await menuState();
  if (m.win.width !== 241 || !/hearth-rail/.test(m.cls) || /hearth-(bar|ghost|menu)/.test(m.cls) || m.mirror) errors.push(`menu-rail-on-store: expected the side rail (${JSON.stringify(m.win)}, ${m.cls})`);
  if (!m.diag || !/web page/.test(m.diag.why) || m.diag.layout !== "rail") errors.push(`menu-rail-on-store: ${m.diag && m.diag.why}`);
  if (m.icons.slice(0, 3).join(",") !== "home,lib,shop") errors.push(`menu-rail-on-store: icons ${m.icons}`);
});
// In a game, something covers the resized top dock (a blank strip on the device): the self-check puts Steam's window
// back and shows Steam's menu in Hearth's colours instead.
await scene("menu-top-covered-in-game", { scene: "game", qam: false, menu: "steam", menuWidth: 241, resize: "steamclient", running: true, cover: true, settings: { look: "classic" }, skin: true, wait: 4000 }, async () => {
  const m = await menuState();
  if (m.win.width !== 241 || m.win.top !== 0 || !/hearth-menu/.test(m.cls) || /hearth-bar/.test(m.cls)) errors.push(`menu-top-covered-in-game: not back to Steam's menu (${JSON.stringify(m.win)}, ${m.cls})`);
  if (!m.diag || !/top dock/.test(m.diag.why) || !/cover/.test(m.diag.why)) errors.push(`menu-top-covered-in-game: ${m.diag && m.diag.why}`);
});
// B: a menu window that can't be resized: Hearth's bar in the main window, following Steam's see-through menu.
await scene("menu-mirror-bottom", { scene: "library", qam: false, menu: "steam", menuWidth: 241, settings: { look: "classic", autoSend: true }, skin: true, wait: 3500 }, async () => {
  const m = await menuState();
  // "Send diagnostics to Hearth automatically" on: the mirror (and the first opening) go to Hearth, with the deep dump.
  const sends = await page.evaluate(() => window.HC_SENT || []);
  const keys = sends.filter((x) => x.auto).map((x) => x.key.split(":")[0]);
  for (const k of ["first-open", "mirror", "resize-refused"]) if (!keys.includes(k)) errors.push(`auto-send: no "${k}" send (${keys})`);
  const deep = (await page.evaluate(() => (window.HC_DIAGS || []).filter((t) => t.includes("== Deep diagnostics ==")).pop() || ""));
  for (const w of ['MainNavMenuAnchor: class "mm_Anchor", 1 match', "container: div#MainNavMenuContainer", "Main window (1280×800)", "Menu window \"MainMenu_uid2\"", "Router.MainRunningApp: none", "Windows: "])
    if (!deep.includes(w)) errors.push(`deep diagnostics: missing ${w}`);
  console.log(`::notice title=deep diagnostics (mirror)::${deep.slice(0, 3000).replace(/\n/g, " | ")}`);
  if (!m.mirror || m.mirror.n !== 8 || m.mirror.on !== 1 || m.mirror.top < 600) errors.push(`menu-mirror-bottom: no mirror bar at the bottom (${JSON.stringify(m.mirror)})`);
  if (!/hearth-ghost/.test(m.cls) || m.panelOpacity !== "0") errors.push(`menu-mirror-bottom: Steam's menu isn't see-through (${m.cls}, ${m.panelOpacity})`);
  // The device's blank side menu: the menu window's body painted dark by the skin. Now see-through.
  const bodyBg = await page.evaluate(() => { const d = document.getElementById("menuwin").contentDocument; return getComputedStyle(d.body).backgroundColor; });
  if (bodyBg !== "rgba(0, 0, 0, 0)") errors.push(`menu-mirror-bottom: the menu window's body is painted (${bodyBg})`);
  if (!m.diag || !/window\.resizeTo: asked/.test(m.diag.why)) errors.push(`menu-mirror-bottom: the refused resize isn't explained (${m.diag && m.diag.why})`);
  if (m.win.width !== 241) errors.push("menu-mirror-bottom: Steam's menu window was moved");
  // Down in Steam's menu: the highlight follows.
  await page.evaluate(() => { const d = document.getElementById("menuwin").contentDocument; const it = [...d.querySelectorAll(".mm_entry")]; it.forEach((e) => e.classList.remove("gpfocus")); it[4].classList.add("gpfocus"); });
  await page.waitForTimeout(500);
  const m2 = await menuState();
  if (!m2.mirror || m2.mirror.on !== 4) errors.push(`menu-mirror-bottom: the highlight didn't follow Steam's focus (${JSON.stringify(m2.mirror)})`);
  await page.screenshot({ path: join(out, "menu-mirror-bottom-moved.png") });
});
await scene("menu-rail-in-game", { scene: "game", qam: false, menu: "steam", menuWidth: 241, running: true, settings: { look: "classic" }, skin: true, wait: 4000 }, async () => {
  // Steam keeps its menu window at 241 px and the main window isn't on screen over a game: Hearth's side rail inside
  // Steam's menu window, the game's card first, every entry visible, the window untouched, resize tried once at most.
  const m = await menuState();
  const r = await page.evaluate(() => {
    const d = document.getElementById("menuwin").contentDocument;
    const panel = d.querySelector("#MainNavMenuContainer > .Panel"), pr = panel.getBoundingClientRect();
    const items = [...d.querySelectorAll(".mm_entry")];
    return { panel: [Math.round(pr.left), Math.round(pr.top), Math.round(pr.right), Math.round(pr.bottom)], game: items[0].getAttribute("data-hearth-game"),
             inside: items.every((e) => { const x = e.getBoundingClientRect(); return x.left >= 0 && x.right <= 241 && x.top >= 0 && x.bottom <= 760 && x.width > 0; }),
             radius: getComputedStyle(panel).borderRadius, calls: window.HC_RESIZE_CALLS };
  });
  if (m.mirror || !/hearth-rail/.test(m.cls) || /hearth-(bar|ghost|menu)/.test(m.cls) || m.win.width !== 241) errors.push(`menu-rail-in-game: expected the side rail (${m.cls}, ${JSON.stringify(m.win)})`);
  if (r.game !== "1" || !r.inside || r.panel[2] > 241 || r.panel[3] > 760 || r.radius !== "22px") errors.push(`menu-rail-in-game: ${JSON.stringify(r)}`);
  // window.resizeTo: one try and its undo, once per start; never again (no flicker, no repeated calls).
  if (r.calls > 2) errors.push(`menu-rail-in-game: window.resizeTo called ${r.calls} times`);
  if (!m.diag || m.diag.layout !== "rail" || !/the side rail is used in games and on web pages/.test(m.diag.why) || !/^passed/.test(m.diag.selfCheck)) errors.push(`menu-rail-in-game: diag ${JSON.stringify(m.diag && { layout: m.diag.layout, why: m.diag.why, selfCheck: m.diag.selfCheck })}`);
});
await scene("menu-rail-off-in-game", { scene: "game", qam: false, menu: "steam", menuWidth: 241, running: true, settings: { look: "classic", rail: false }, skin: true, wait: 3500 }, async () => {
  const m = await menuState();
  if (!/hearth-menu/.test(m.cls) || /hearth-rail/.test(m.cls) || m.mirror) errors.push(`menu-rail-off-in-game: expected Steam's menu in Hearth's colours (${m.cls})`);
});
await scene("menu-steam-changed", { scene: "library", menu: "changed", menuWidth: 241, settings: { look: "classic" }, skin: true,
  want: ["Hearth bottom bar menu paused", "no .Panel"] }, async () => {
  const m = await menuState();
  if (/hearth-(bar|menu|ghost)/.test(m.cls) || m.icons.some(Boolean) || m.mirror) errors.push(`menu-steam-changed: Steam's changed menu was touched (${m.cls})`);
});
// The diagnostics in the panel, after the menu was found.
await scene("panel-diagnostics", { scene: "library", menu: "steam", menuWidth: 241, resize: "browserview", settings: { look: "classic" }, skin: true, wait: 3500 }, async () => {
  await page.evaluate(() => { const b = [...document.querySelectorAll("button")].find((x) => x.textContent.startsWith("Show what Hearth Classic found")); b && b.click(); });
  await page.waitForTimeout(600);
  await page.evaluate(() => { const m = document.getElementById("menuwin"); if (m) m.style.display = "none"; });
  const t = await page.evaluate(() => document.body.innerText);
  for (const w of ["MainMenu_uid2", "Menu: found", "8 entries", "Menu bar: bar", "Steam's menu window resized", "before 241×760", "after 1280×150", "BrowserView.SetBounds", "Class names:", "Send diagnostics to Hearth automatically"]) if (!t.includes(w)) errors.push(`panel-diagnostics: missing ${w}`);
  // Send by hand: the code shows as "Last sent".
  await page.evaluate(() => { const b = [...document.querySelectorAll("button")].find((x) => x.textContent.startsWith("Send Hearth Classic log now")); b && b.click(); });
  await page.waitForTimeout(800);
  const t2 = await page.evaluate(() => document.body.innerText);
  if (!t2.includes("Last sent:") || !t2.includes("code TEST-CODE-01")) errors.push("panel-diagnostics: no Last sent line after sending");
  const man = await page.evaluate(() => (window.HC_SENT || []).filter((x) => !x.auto));
  if (!man.length || man[0].len < 500) errors.push(`panel-diagnostics: the manual send carried no deep diagnostics (${JSON.stringify(man)})`);
  // A send that fails both ways: the panel says why, in the service's and TLS's own words.
  await page.evaluate(() => { window.HC_SENDFAIL = true; const b = [...document.querySelectorAll("button")].find((x) => x.textContent.startsWith("Send Hearth Classic log now")); b && b.click(); });
  await page.waitForTimeout(800);
  const t3 = await page.evaluate(() => document.body.innerText);
  if (!t3.includes("certificate verify failed") || !t3.includes("through curl: HTTP 429: busy, try again later")) errors.push("panel-diagnostics: the send's error isn't shown");
  // The picture: the found-text folded again, Quick Access scrolled down to the Diagnostics section.
  await page.evaluate(() => { const b = [...document.querySelectorAll("button")].find((x) => x.textContent.startsWith("Hide what Hearth Classic found")); b && b.click(); });
  await page.waitForTimeout(300);
  await page.evaluate(() => { const p = document.querySelector(".ScrollPanel"); if (p) { p.style.overflowY = "auto"; p.style.flex = "1"; p.style.minHeight = "0"; p.scrollTop = p.scrollHeight; } });
  await page.waitForTimeout(300);
  await page.screenshot({ path: join(out, "panel-diagnostics.png") });
});
// 3e. A settings dialog (Steam's rows, a toggle, a dropdown, buttons): Steam's look, then Hearth Classic.
await scene("dialog-steam", { scene: "dialog", qam: false, skin: false });
await scene("dialog-hearth-classic", { scene: "dialog", qam: false, settings: { look: "classic" }, skin: true });
// 4. The Quick Access panel with Hearth Classic on.
await scene("panel-hearth-classic-on", { settings: { look: "classic" }, want: ["Hearth home screen", "Hearth wallpaper", "Hearth bottom bar menu", "In games and the Store: Hearth side rail", "Hearth sounds", "Recent games"], skin: true });
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

// 7. The audit pairs: the old Hearth Game Mode (site/img/gm-*.webp, drawn from ui/index.html by tools/site-shots.py)
// beside Hearth Classic in the stand-in, one per screen.
await freshPage(2600, 900);
const PAIRS = [["home", "gm-home", "home-hearth-classic"], ["library", "gm-library", "skin-hearth-classic-library"], ["game-page", "gm-game", "game-hearth-classic"],
  ["quick-access", "gm-qa", "panel-hearth-classic-on"], ["menu-bottom-bar", "gm-dock", "menu-bottom-bar"], ["in-game-top-bar", "gm-overlay", "menu-top-bar-in-game"], ["menu-bottom-bar-mirror", "gm-dock", "menu-mirror-bottom"], ["side-rail-in-game", "gm-overlay", "menu-rail-in-game"], ["side-rail-on-store", "gm-dock", "menu-rail-on-store"],
  ["dialogs", "gm-settings", "dialog-hearth-classic"]];
for (const [name, old, mine] of PAIRS) {
  const a = b64(join(root, "site/img", old + ".webp"), "image/webp"), c = b64(join(out, mine + ".png"), "image/png");
  await page.setContent(`<!doctype html><html><body style="margin:0;background:#0c1016;color:#e9eef4;font-family:'Noto Sans',Arial,sans-serif;display:flex;gap:40px;padding:30px;box-sizing:border-box">
<figure style="margin:0"><figcaption style="font-size:22px;margin-bottom:12px">Hearth Game Mode (${old}.webp)</figcaption><img src="${a}" style="width:1240px;height:775px;object-fit:cover;object-position:top;border-radius:10px"></figure>
<figure style="margin:0"><figcaption style="font-size:22px;margin-bottom:12px">Hearth Classic (${mine}, stand-in for Steam)</figcaption><img src="${c}" style="width:1240px;height:775px;border-radius:10px"></figure></body></html>`);
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(out, "pair-" + name + ".png") });
}
writeFileSync(join(out, "README.txt"), "Hearth Classic's real plugin code over a stand-in of Steam's Game Mode (tests/mock). Not Steam itself.\n");
await browser.close();
if (errors.length) { console.log(`::error title=Hearth Classic pictures::${errors.join(" | ").slice(0, 3000)}`); process.exit(1); }
console.log("::notice title=Hearth Classic pictures::all scenes drawn and checked");
