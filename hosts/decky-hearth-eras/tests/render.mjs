// CI only (decky-hearth-eras.yml): runs Hearth Eras' real plugin (src/) in a plain browser over a stand-in of Steam's
// Game Mode (tests/mock/), checks what it does and saves pictures: the Quick Access panel (the gallery), every era's
// home and its skin on a Library page, the extras (keyboard, spinner), quick switching, the fallback to Steam's Home,
// paused after a Steam change, not Hearth OS, and unloading. Steam itself isn't involved: the frame only stands in for
// Steam's screens, so how it sits in the real Steam needs a device.
// Usage: HEARTH_ROOT=<repo> node render.mjs OUTDIR   (needs esbuild, react, react-dom, react-icons, playwright next to it)
import { readFileSync, readdirSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const plug = join(root, "hosts/decky-hearth-eras");
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
const erasDir = join(plug, "defaults/eras");
const base = readFileSync(join(erasDir, "base.css"), "utf8");
const eras = Object.fromEntries(readdirSync(erasDir).filter((f) => f.endsWith(".css") && f !== "base.css").map((f) => [f.slice(0, -4), readFileSync(join(erasDir, f), "utf8")]));
eras.classic = readFileSync(join(root, "hosts/decky-hearth-look/defaults/hearth-classic.css"), "utf8");
const logo = b64(join(root, "site/img/logo.svg"), "image/svg+xml");
const wall = b64(join(root, "os/branding/wallpapers/hearth-embers.jpg"), "image/jpeg");

const frameBase = `body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:var(--gpSystemDarkestGrey,#0e141b);font-family:"Noto Sans",Arial,sans-serif;color:#dcdedf}
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
const frame = frameBase + `
.keyboard{position:absolute;left:0;right:0;bottom:0;height:200px;background:#23262e;display:flex;flex-direction:column;gap:8px;padding:16px;z-index:4}
.krow{display:flex;gap:8px;justify-content:center}.key{width:72px;height:48px;background:#3d4450;border-radius:3px;display:flex;align-items:center;justify-content:center;font-size:18px}
.spinner{position:absolute;left:560px;top:250px;width:72px;height:72px;border-radius:50%;border:7px solid #66c0f4;border-top-color:transparent}
.ScrollPanel{overflow:visible}`;

const browser = await chromium.launch();
const errors = [];
let page = null;
async function freshPage(w = 1280, h = 800) {
  if (page) await page.close();
  page = await browser.newPage({ viewport: { width: w, height: h } });
  page.on("pageerror", (e) => errors.push(String(e)));
}

async function scene(name, opts, check) {
  const vars = { HE_SCENE: opts.scene || "home", HE_SETTINGS: opts.settings || {}, HE_STEAM: opts.steam || "", HE_HEARTH: opts.hearth !== false,
                 HE_QAM: opts.qam !== false, HE_EMPTY: !!opts.empty, HE_RUNNING: !!opts.running, HE_KEYBOARD: !!opts.keyboard, HE_SPINNER: !!opts.spinner,
                 HE_BASE: base, HE_ERAS: eras, HE_LOGO: logo, HE_WALL: wall };
  const audio = `window.HE_AUDIO=0;window.AudioContext=class{constructor(){window.HE_AUDIO++;this.state="running";this.currentTime=0;this.sampleRate=44100;this.destination={}}resume(){}close(){}
createGain(){return{gain:{setValueAtTime(){},exponentialRampToValueAtTime(){}},connect(x){return x}}}createOscillator(){return{frequency:{value:0,setValueAtTime(){},exponentialRampToValueAtTime(){}},connect(x){return x},start(){},stop(){}}}
createBuffer(c,n){return{getChannelData(){return new Float32Array(n)}}}createBufferSource(){return{connect(x){return x},start(){},stop(){}}}createBiquadFilter(){return{frequency:{},Q:{},connect(x){return x}}}};`;
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${frame}</style></head><body><div id="root"></div>
<script>${audio}${Object.entries(vars).map(([k, v]) => `window.${k}=${JSON.stringify(v)};`).join("")}</script><script>${js}</script></body></html>`;
  await freshPage(1280, opts.height || 800);
  await page.setContent(html);
  await page.waitForTimeout(opts.wait || 2500);
  if (opts.focus) await page.evaluate((sel) => { const e = document.querySelector(sel); if (e) { e.focus(); e.classList.add("gpfocus"); } }, opts.focus);
  await page.waitForTimeout(400);
  const text = (await page.evaluate(() => document.body.innerText)).toLowerCase();
  const style = await page.evaluate(() => document.getElementById("hearth-eras")?.textContent || "");
  const missing = (opts.want || []).filter((w) => !text.includes(w.toLowerCase()));
  const present = (opts.not || []).filter((w) => text.includes(w.toLowerCase()));
  if (missing.length) errors.push(`${name}: missing ${missing.join(", ")}`);
  if (present.length) errors.push(`${name}: shouldn't show ${present.join(", ")}`);
  if (opts.skin === true && !style.includes(".gtp_Tab")) errors.push(`${name}: the skin isn't applied`);
  if (opts.skin === false && style) errors.push(`${name}: the skin is applied but shouldn't be`);
  await page.screenshot({ path: join(out, name + ".png") });
  if (check) await check(text, style);
  console.log(`drew ${name}`);
}
const accentOf = (era, i = 0) => ({ channels: ["#2fa6d6", "#e0773a", "#58c2ec"], blades: ["#ff8a3d", "#3ec5ff", "#ff5fa2"], tiles: ["#ff6f59", "#8f7bff", "#4d8dff"],
  crossbar: ["#ffffff"], pocket: ["#7fd8ff", "#ffb35c", "#ffffff"] })[era]?.[i];
const rgb = (hex) => { const n = parseInt(hex.slice(1), 16); return `rgb(${n >> 16}, ${(n >> 8) & 255}, ${n & 255})`; };

// 1. Steam's look (the default): the gallery lists every era, nothing is applied; then the person picks Crossbar.
await scene("panel-steam", { height: 1500, want: ["Eras", "Steam", "Channels", "Blades", "Tiles", "Crossbar", "Pocket", "Hearth Classic", "Extras", "Era sounds", "Quick switching",
  "Loading screen style", "On-screen keyboard skin", "Styled Library pages", "Steam's own Home"], not: ["Graphite", "paused"], skin: false }, async () => {
  if (await page.evaluate(() => window.HE_PATCHES.size)) errors.push("panel-steam: a route patch was added with Steam's look");
  if (await page.evaluate(() => window.HE_SAVES.length)) errors.push("panel-steam: something was saved without the person");
  await page.click('.hearth-eras-pick[data-era="crossbar"]');
  await page.waitForTimeout(2500);
  const st = await page.evaluate(() => ({ style: !!document.getElementById("hearth-eras"), home: !!document.querySelector(".era-crossbar"), saves: window.HE_SAVES }));
  if (!st.style || !st.home || JSON.stringify(st.saves) !== '[{"era":"crossbar"}]') errors.push(`panel-steam: picking Crossbar didn't apply just that (${JSON.stringify(st)})`);
  await page.screenshot({ path: join(out, "panel-crossbar.png") });
});

// 2. Every era: its home on Steam's Home route (first game focused) and its skin on a Library page.
const ERA_WANT = { channels: ["Starfall Drift", "Library", "Store"], blades: ["Recent", "Favorites", "Places"], tiles: ["Starfall Drift", "Places", "Favorites"],
  crossbar: ["Recent", "Places", "Favorites"], pocket: ["Starfall Drift", "Store"], classic: ["Recent games", "Favorites", "Library"] };
for (const era of ["channels", "blades", "tiles", "crossbar", "pocket", "classic"]) {
  const sel = era === "classic" ? ".hc-shelf .hc-tile" : ".era-game";
  await scene(`home-${era}`, { settings: { era }, qam: false, wait: 3200, focus: sel, skin: era === "classic" ? undefined : true, want: ERA_WANT[era], not: ["Not In Steam Yet", "Steam's own Home"] }, async (_t, style) => {
    if (era === "classic" && !style.includes("--hearth-accent")) errors.push("home-classic: Hearth Classic's skin isn't applied");
    const n = await page.locator(sel).count();
    if (n < 3) errors.push(`home-${era}: only ${n} games`);
    const imgs = await page.evaluate((s) => [...document.querySelectorAll(s)].filter((b) => b.querySelector("img")).length, sel);
    if (imgs < Math.min(n, 3)) errors.push(`home-${era}: games without pictures (${imgs} of ${n})`);
    const id = await page.evaluate((s) => window.HE_MUSIC_APPID(document.querySelector(s)), sel);
    if (!(id > 0)) errors.push(`home-${era}: Hearth Music can't tell the focused game (${id})`);
    await page.evaluate((s) => document.querySelector(s).click(), sel);
    const nav = await page.evaluate(() => window.HE_NAV || []);
    if (!nav.some((r) => /^\/library\/app\/\d+$/.test(r))) errors.push(`home-${era}: a game didn't open Steam's page (${nav})`);
    if (era !== "classic") {
      // Blades' and Crossbar's places open when focus reaches their panel or category.
      await page.evaluate(() => { const t = [...document.querySelectorAll(".bl-tab")].pop() || document.querySelector(".xb-cat"); if (t && document.querySelector(".bl-tab, .xb-cat")) { t.focus(); t.click(); } });
      await page.waitForTimeout(300);
      const used = await page.evaluate(() => { const b = [...document.querySelectorAll(".era-place")].find((x) => x.textContent.includes("Store")); if (!b) return "missing"; b.click(); return window.HC_LAST_PLACE; });
      if (used !== "navigator:SteamWeb") errors.push(`home-${era}: Store used ${used}, not Steam's navigator`);
    }
  });
  await scene(`library-${era}`, { scene: "library", qam: false, settings: { era }, skin: era === "classic" ? undefined : true }, async () => {
    const bg = await page.evaluate(() => getComputedStyle(document.querySelector(".gtp_Selected")).backgroundColor);
    const want = era === "classic" ? "rgb(233, 238, 244)" : era === "crossbar" ? "rgba(255, 255, 255, 0.2)" : era === "channels" ? rgb("#2fa6d6") : rgb(accentOf(era));
    if (bg !== want) errors.push(`library-${era}: the selected tab is ${bg}, not ${want}`);
  });
}
// 2b. A different preset changes the colours (Blades in Ocean).
await scene("library-blades-ocean", { scene: "library", qam: false, settings: { era: "blades", presets: { blades: 1 } }, skin: true }, async () => {
  const bg = await page.evaluate(() => getComputedStyle(document.querySelector(".gtp_Selected")).backgroundColor);
  if (bg !== rgb("#3ec5ff")) errors.push(`preset: Blades · Ocean's tab is ${bg}`);
});

// 3. The panel with an era on: the colours picker; the extras off.
await scene("panel-tiles", { height: 1500, settings: { era: "tiles" }, want: ["Colours", "Graphite"], skin: true });

// 4. Extras: the keyboard and the spinner in the era's colours only when switched on.
await scene("extras-off", { scene: "library", qam: false, keyboard: true, spinner: true, settings: { era: "pocket" }, skin: true }, async (_t, style) => {
  if (style.includes("vk_Container") || style.includes(".throbber")) errors.push("extras-off: keyboard or spinner styled while off");
});
await scene("extras-keyboard-loading", { scene: "library", qam: false, keyboard: true, spinner: true, settings: { era: "pocket", keyboard: true, loading: true, pages: true }, skin: true }, async (_t, style) => {
  if (!style.includes(".vk_Container{") || !style.includes(".throbber{")) errors.push("extras: keyboard or spinner not styled");
  const kb = await page.evaluate(() => getComputedStyle(document.querySelector(".vk_Container")).borderTopColor);
  if (kb !== rgb("#7fd8ff")) errors.push(`extras: keyboard edge ${kb}`);
});

// 5. Sounds: none unless switched on (nothing even made), then the era's own.
await scene("sounds-off", { settings: { era: "channels" }, qam: false, focus: ".era-game", skin: true }, async () => {
  await page.evaluate(() => { const g = document.querySelectorAll(".era-game"); g[1].focus(); g[2].focus(); });
  if (await page.evaluate(() => window.HE_AUDIO)) errors.push("sounds-off: an AudioContext was made with sounds off");
});
await scene("sounds-on", { settings: { era: "channels", sounds: true }, qam: false, focus: ".era-game", skin: true }, async () => {
  await page.evaluate(() => { const g = document.querySelectorAll(".era-game"); g[1].focus(); g[2].focus(); });
  if (!(await page.evaluate(() => window.HE_AUDIO))) errors.push("sounds-on: no sound was made");
});

// 6. Quick switching: off by default (nothing listens); on: hold L1 + D-pad right / R2; never while a game runs.
const L1 = 8, R2 = 1, RIGHT = 512, LEFT = 1024;
await scene("quick-off", { qam: false, skin: false }, async () => {
  if (await page.evaluate(() => typeof window.HE_PAD === "function")) errors.push("quick-off: listening while quick switching is off");
});
await scene("quick-switch", { settings: { quickSwitch: true }, wait: 2500, height: 1500, skin: false }, async () => {
  const pad = (b) => page.evaluate((b) => window.HE_PAD && window.HE_PAD([{ unControllerIndex: 0, ulButtons: b }]), b);
  await pad(RIGHT); await pad(0);                        // D-pad alone: nothing
  await pad(L1); await pad(L1 | RIGHT); await page.waitForTimeout(1500);   // -> Channels
  await pad(L1); await pad(L1 | RIGHT); await page.waitForTimeout(1500);   // -> Blades
  await pad(L1); await pad(L1 | R2); await page.waitForTimeout(1500);      // -> Blades · Ocean
  await pad(0);
  const st = await page.evaluate(() => ({ saves: window.HE_SAVES, toasts: window.HE_TOASTS.map((t) => t.body), era: window.HearthEras && window.HearthEras.era }));
  if (JSON.stringify(st.saves) !== '[{"era":"channels"},{"era":"blades"},{"presets":{"blades":1}}]') errors.push(`quick-switch: saved ${JSON.stringify(st.saves)}`);
  if (st.toasts.join("|") !== "Channels · Silver|Blades · Ember|Blades · Ocean") errors.push(`quick-switch: toasts ${st.toasts.join("|")}`);
  console.log(`::notice title=Hearth Eras quick switching::${JSON.stringify(st)}`);
});
await scene("quick-in-game", { settings: { quickSwitch: true, era: "tiles" }, running: true, qam: false, skin: true }, async () => {
  await page.evaluate(([a, b]) => { window.HE_PAD([{ ulButtons: a }]); window.HE_PAD([{ ulButtons: b }]); }, [L1, L1 | LEFT]);
  await page.waitForTimeout(800);
  if (await page.evaluate(() => window.HE_SAVES.length)) errors.push("quick-in-game: switched while a game runs");
});

// 7. Nothing to show: Steam's own Home stays.
await scene("home-fallback-empty", { settings: { era: "tiles" }, empty: true, qam: false, skin: true, want: ["Steam's own Home"] });
// 8. A Steam that changed its modules: paused, Steam's look, the panel says so; the choice stays.
await scene("panel-paused", { height: 1500, settings: { era: "blades" }, steam: "changed", skin: false, want: ["Hearth Eras paused: Steam changed its layout", "Try again", "Steam's own Home"] }, async () => {
  if (await page.evaluate(() => window.HE_SAVES.length)) errors.push("panel-paused: the choice was changed");
});
// 9. Not Hearth OS: the invitation only, nothing applied.
await scene("panel-not-hearth-os", { settings: { era: "blades" }, hearth: false, skin: false, want: ["Get Hearth OS", "Steam's own Home"], not: ["Quick switching", "Try again"] });
// 10. Unloading takes everything out.
await scene("unload", { settings: { era: "pocket", quickSwitch: true }, qam: false, skin: true }, async () => {
  await page.evaluate(() => window.HE_PLUGIN.onDismount());
  await page.waitForTimeout(800);
  const left = await page.evaluate(() => ({ style: !!document.getElementById("hearth-eras"), patches: window.HE_PATCHES.size, pad: typeof window.HE_PAD, text: document.body.innerText }));
  if (left.style || left.patches || left.pad === "function" || !left.text.includes("Steam's own Home")) errors.push(`unload: something stayed (${JSON.stringify({ ...left, text: "" })})`);
});

// 11. The gallery sheet: every era's home and Library side by side.
await freshPage(2600, 2500);
const ERAS_ALL = ["channels", "blades", "tiles", "crossbar", "pocket", "classic"];
const cell = (f, cap) => `<figure style="margin:0"><figcaption style="font-size:22px;margin:0 0 8px">${cap}</figcaption><img src="${b64(join(out, f + ".png"), "image/png")}" style="width:1240px;height:775px;border-radius:10px;display:block"></figure>`;
await page.setContent(`<!doctype html><html><body style="margin:0;background:#0c1016;color:#e9eef4;font-family:'Noto Sans',Arial,sans-serif;display:grid;grid-template-columns:1240px 1240px;gap:30px 40px;padding:30px;box-sizing:border-box">
${ERAS_ALL.slice(0, 3).map((e) => cell("home-" + e, e + " · home")).join("")}</body></html>`);
await page.waitForTimeout(300);
await page.screenshot({ path: join(out, "sheet-1.png"), fullPage: true });
await page.setContent(`<!doctype html><html><body style="margin:0;background:#0c1016;color:#e9eef4;font-family:'Noto Sans',Arial,sans-serif;display:grid;grid-template-columns:1240px 1240px;gap:30px 40px;padding:30px;box-sizing:border-box">
${ERAS_ALL.slice(3).map((e) => cell("home-" + e, e + " · home")).join("")}</body></html>`);
await page.waitForTimeout(300);
await page.screenshot({ path: join(out, "sheet-2.png"), fullPage: true });
writeFileSync(join(out, "README.txt"), "Hearth Eras' real plugin code over a stand-in of Steam's Game Mode (tests/mock). Not Steam itself.\n");
await browser.close();
if (errors.length) { console.log(`::error title=Hearth Eras pictures::${errors.join(" | ").slice(0, 3000)}`); process.exit(1); }
console.log("::notice title=Hearth Eras pictures::all scenes drawn and checked");
