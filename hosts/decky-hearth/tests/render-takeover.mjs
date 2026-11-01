// CI only (decky-hearth.yml): draws the Hearth takeover's real pages (src/pages.tsx: Hearth's home in place of Steam's
// Home, Hearth Store, Hearth Library) in a plain browser, with stand-ins for Decky (takeover-mock/) and made-up games,
// and saves pictures. Steam isn't involved: this shows the pages Hearth draws, not how they sit in a real Steam.
// Usage: HEARTH_ROOT=<repo> node render-takeover.mjs OUTDIR   (needs esbuild, react, react-dom, playwright next to it)
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const mock = join(root, "hosts/decky-hearth/tests/takeover-mock");
const out = process.argv[2] || "shots";
mkdirSync(out, { recursive: true });
const b64 = (p, mime) => `data:${mime};base64,${readFileSync(p).toString("base64")}`;

const res = await build({
  entryPoints: [join(mock, "entry.jsx")], bundle: true, write: false, format: "iife", jsx: "automatic", target: "es2020",
  // One React for everything (the plugin's own node_modules may hold another, which React can't mix).
  alias: { "@decky/ui": join(mock, "stub-ui.jsx"), "@decky/api": join(mock, "stub-api.js"), react: join(here, "node_modules/react"),
           "react-dom": join(here, "node_modules/react-dom"), "react-icons": join(here, "node_modules/react-icons") },
  nodePaths: [join(here, "node_modules")], define: { "process.env.NODE_ENV": '"production"' }, logLevel: "warning",
});
const js = res.outputFiles[0].text;
const logo = b64(join(root, "site/img/logo.svg"), "image/svg+xml");
const wall = b64(join(root, "os/branding/wallpapers/hearth-embers.jpg"), "image/jpeg");
// Steam's main window is dark and full screen; Hearth's look (defaults/hearth-theme.css) would be on as well.
const theme = readFileSync(join(root, "hosts/decky-hearth/defaults/hearth-theme.css"), "utf8").split("\n").filter((l) => !l.includes("{{") && !l.includes("%")).join("\n");

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
for (const [name, which, focus] of [["takeover-home", "home", 1], ["takeover-store", "store", 8], ["takeover-library", "library", 2], ["takeover-old-row", "row", 2]]) {
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${theme}</style><style>body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:#0c1016}
.DialogButton{border:none;color:#e9eef4;background:rgba(233,238,244,.1);border-radius:6px;font:inherit;cursor:pointer}
/* Hearth's look paints Steam's dialog buttons (hearth-theme.css GamepadDialogContent:Button) like this: what hid 0.6.60's covers */
.DialogButton{background:#1d2531 !important}.DialogButton.gpfocus{background:#3fd0b8 !important}</style></head>
<body><div id="root" style="position:absolute;inset:0"></div><script>window.HEARTH_PAGE=${JSON.stringify(which)};window.HEARTH_LOGO=${JSON.stringify(logo)};window.HEARTH_WALL=${JSON.stringify(wall)};</script><script>${js}</script></body></html>`;
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(which === "row" || which === "home" ? 4000 : 1200);
  // Steam marks the focused control with .gpfocus: one tile as the controller would have it.
  await page.evaluate((n) => { const t = [...document.querySelectorAll(".ht-tile, .hearth-row-card")]; if (t[n]) t[n].classList.add("gpfocus"); }, focus);
  await page.waitForTimeout(300);
  const tiles = await page.locator(".ht-tile, .hearth-row-card").count();
  if (!tiles) errors.push(`${name}: no tiles were drawn`);
  // Every game card shows its title, and the art cases behave: the webp shows, the broken picture and the game with
  // no art are title cards (never blank), Steam's own picture stands in for a Steam game without Hearth art.
  if (which === "home" || which === "row" || which === "library") {
    const cards = await page.evaluate(() => [...document.querySelectorAll(".ht-tile, .hearth-row-card")].filter((t) => t.querySelector(".ht-name")).map((t) => {
      const img = t.querySelector("img.ht-art");
      return { title: t.querySelector(".ht-name").textContent, img: img ? img.src.slice(0, 16) : "", loaded: img ? img.complete && img.naturalWidth > 0 : false };
    }));
    if (!cards.length) errors.push(`${name}: no game cards`);
    for (const c of cards) if (!c.title.trim()) errors.push(`${name}: a card without a title`);
    for (const c of cards) if (c.img && !c.loaded) errors.push(`${name}: ${c.title}: its picture didn't load and no title card took its place`);
    const by = (t) => cards.find((c) => c.title === t);
    const expect = (t, ok, why) => { const c = by(t); if (c && !ok(c)) errors.push(`${name}: ${t}: ${why} (${JSON.stringify(c)})`); };
    expect("Webp Cover Game", (c) => c.img.startsWith("data:image/webp") && c.loaded, "the webp cover should show");
    expect("Broken Picture Game", (c) => !c.img, "a broken picture should leave a title card");
    expect("Battle.net", (c) => !c.img, "no art: a title card");
    expect("Team Fortress 2", (c) => c.loaded, "Steam's own library picture should stand in");
    if (which === "row" && !by("Webp Cover Game")) errors.push(`${name}: the webp game isn't in the row`);
    console.log(`${name} cards: ${cards.map((c) => `${c.title}=${c.img ? (c.loaded ? "art" : "BROKEN") : "title"}`).join(", ")}`);
    console.log(`::notice title=${name} cards::${cards.map((c) => `${c.title}=${c.img ? (c.loaded ? "art" : "BROKEN") : "title"}`).join(", ").slice(0, 900)}`);
  }
  await page.screenshot({ path: join(out, name + ".png") });
  console.log(`drew ${name} (${tiles} tiles)`);
}
// Hearth in Steam's main menu, Quick Access and the in-game section, over static stand-ins for Steam's menu and Quick
// Access frame (styled roughly like Steam's; Hearth's own parts are the real components). A game is "running".
const frame = `.steam-menu{position:absolute;left:0;top:0;bottom:0;width:340px;background:rgba(14,20,27,.97);box-shadow:8px 0 30px rgba(0,0,0,.5);padding:24px 8px;overflow:hidden;font-size:15px;color:#e9eef4}
.steam-menu-logo{font-weight:800;letter-spacing:.2em;color:#8d99a8;padding:0 16px 14px}.steam-menu-entry{padding:9px 16px;border-radius:8px}
.qam{position:absolute;right:0;top:0;bottom:0;width:460px;display:flex;background:rgba(14,20,27,.98);box-shadow:-8px 0 30px rgba(0,0,0,.5);color:#e9eef4}
.qam-tabs{width:58px;background:#0b1016;display:flex;flex-direction:column;align-items:center;padding-top:12px;gap:6px}
.qam-tab{width:40px;height:40px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:#8d99a8;font-size:18px}
.qam-tab.hearth{color:#3fd0b8}.qam-tab.on{background:#3fd0b8;color:#06231e}
.qam-body{flex:1;display:flex;flex-direction:column;min-width:0}.qam-title{font-size:18px;font-weight:700;padding:16px 16px 8px}
.ScrollPanel{padding:0 12px 20px}.qam-section{margin-bottom:10px}.qam-section-title{font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:#8d99a8;padding:10px 4px 6px}
.qam-row{padding:4px 4px}.qam-field{display:flex;flex-direction:column;gap:5px;padding:2px 0}.qam-label{font-size:14px}.qam-desc{font-size:11.5px;color:#8d99a8}
.qam-btn,.qam-drop{width:100%;padding:9px 12px;text-align:center;font-size:14px}.qam-drop{text-align:left;display:flex;justify-content:space-between}
.qam-line{display:flex;justify-content:space-between;align-items:center}.qam-switch{width:40px;height:22px;border-radius:11px;background:#273140;position:relative}
.qam-switch i{position:absolute;top:3px;left:3px;width:16px;height:16px;border-radius:50%;background:#e9eef4}.qam-switch.on{background:#3fd0b8}.qam-switch.on i{left:21px}
.Text{color:#e9eef4}`;
const running = { appid: 3000000001, display_name: "Alan Wake 2" };
for (const [name, which, want, focus] of [
  ["steam-menu", "steam-menu", ["Hearth Home", "Hearth Store", "Hearth Library", "Theater", "Hearth Mods", "Storage", "Downloads", "Switch to Hearth Game Mode", "Desktop Mode", "Alan Wake 2"], ".hearth-menu-item:nth-of-type(1)"],
  ["qam-hearth", "qam-hearth", ["This game", "Alan Wake 2", "Quick Resume", "Put Alan Wake 2 aside", "Hearth Input layout", "Shooter / action", "GE-Proton (newest)", "7 mods, 5 on", "Screenshot", "Switch to Hearth Game Mode"], null],
  ["qam-performance", "qam-perf", ["Power mode", "Balanced", "Power limit: 15 W", "Steam's own Performance tab"], null],
  ["qam-controller", "qam-controller", ["Shooter / action: on in the game (paused while Quick Access is open)", "Hearth Input in Steam Game Mode", "Edit layouts in Hearth"], null],
  ["qam-players", "qam-players", ["Players", "DualSense Wireless Controller", "P1", "P4", "Dragon", "Move up", "Move down", "Rumble", "Waits (4 players at most)", "1 more waits", "Split-screen games"], null],
  ["ingame-hearth", "ingame", ["Plays through Hearth", "Alan Wake 2", "Hearth Input layout", "Proton (Play through Hearth)", "Open Hearth Mods", "Game details in Hearth"], null],
]) {
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${theme}</style><style>body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:#0c1016;font-family:"Segoe UI","Noto Sans",sans-serif}
.DialogButton{border:none;color:#e9eef4;background:rgba(233,238,244,.1);border-radius:6px;font:inherit;cursor:pointer}
.DialogButton{background:#1d2531 !important}.DialogButton.gpfocus{background:#3fd0b8 !important;color:#06231e !important}${frame}</style></head>
<body><div id="root" style="position:absolute;inset:0"></div><script>window.HEARTH_PAGE=${JSON.stringify(which)};window.HEARTH_RUNNING=${JSON.stringify(running)};window.HEARTH_LOGO=${JSON.stringify(logo)};window.HEARTH_WALL=${JSON.stringify(wall)};</script><script>${js}</script></body></html>`;
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(2500);
  if (focus) await page.evaluate((sel) => { const e = document.querySelector(sel); if (e) e.classList.add("gpfocus"); }, focus);
  await page.waitForTimeout(300);
  const text = await page.evaluate(() => document.body.innerText);
  // innerText follows text-transform (Hearth's headings are upper case): compared without case.
  const missing = want.filter((w) => !text.toLowerCase().includes(w.toLowerCase()));
  if (missing.length) errors.push(`${name}: missing ${missing.join(", ")}`);
  if (/couldn't be shown/i.test(text)) errors.push(`${name}: a Hearth part fell back to its error`);
  await page.screenshot({ path: join(out, name + ".png") });
  console.log(`drew ${name}${missing.length ? " (missing " + missing.join(", ") + ")" : ""}`);
}
// Quick Access › Hearth Players › Seats (multi-seat beside Steam Game Mode): the players, then the controllers opened
// with "Assign controllers" (as a controller's A would), and the line shown when multi-seat isn't set up.
for (const [name, want, clicks, off] of [
  ["qam-seats", ["Seats", "Player 1 · Playing here (Steam Game Mode)", "Player 2 · Playing (Hearth Game Mode)", "Player 3 · Stopped", "Stop Player 2", "Start Player 3",
    "hearth-p2 · own account and Steam sign-in", "Streamed (Moonlight)", "Assign controllers", "never sees their Steam sign-in"], [], false],
  ["qam-seats-controllers", ["Hide controllers", "DualSense Wireless Controller", "Xbox Wireless Controller", "P1", "P2", "Stays with Player 1"], ["Assign controllers"], false],
  ["qam-seats-off", ["Multi-seat isn't set up", "Settings › Games › Multi-seat"], [], true],
]) {
  await page.setViewportSize({ width: 1280, height: 1000 });
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${theme}</style><style>body{margin:0;width:1280px;height:1000px;overflow:hidden;position:relative;background:#0c1016;font-family:"Segoe UI","Noto Sans",sans-serif}
.DialogButton{border:none;color:#e9eef4;background:rgba(233,238,244,.1);border-radius:6px;font:inherit;cursor:pointer}
.DialogButton{background:#1d2531 !important}.DialogButton.gpfocus{background:#3fd0b8 !important;color:#06231e !important}${frame}</style></head>
<body><div id="root" style="position:absolute;inset:0"></div><script>window.HEARTH_PAGE="qam-seats";window.HEARTH_SEATS_OFF=${off};window.HEARTH_LOGO=${JSON.stringify(logo)};window.HEARTH_WALL=${JSON.stringify(wall)};</script><script>${js}</script></body></html>`;
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(1500);
  for (const c of clicks) {
    const ok = await page.evaluate((t) => { const b = [...document.querySelectorAll("button")].find((x) => x.textContent.trim().startsWith(t)); if (b) b.click(); return !!b; }, c);
    if (!ok) errors.push(`${name}: no button "${c}" to press`);
    await page.waitForTimeout(1200);
  }
  // The first enabled button focused, as the controller would have it.
  await page.evaluate(() => { const b = [...document.querySelectorAll(".qam-body button")].find((x) => !x.disabled); if (b) b.classList.add("gpfocus"); });
  await page.waitForTimeout(300);
  const text = await page.evaluate(() => document.body.innerText);
  const missing = want.filter((w) => !text.toLowerCase().includes(w.toLowerCase()));
  if (missing.length) errors.push(`${name}: missing ${missing.join(", ")}`);
  if (/couldn't be shown/i.test(text)) errors.push(`${name}: a Hearth part fell back to its error`);
  await page.screenshot({ path: join(out, name + ".png") });
  console.log(`drew ${name}${missing.length ? " (missing " + missing.join(", ") + ")" : ""}`);
}
// Quick Access › Hearth › Quick Resume and Game streaming (rest.tsx): the game put aside (as Put aside's A would),
// and Sunshine's PIN pad with a PIN typed.
for (const [name, want, clicks] of [
  ["qam-quick-resume", ["Quick Resume", "Alan Wake 2 · put aside 4 min ago", "Resume", "Close", "Game streaming", "Streaming is on at 192.168.1.42", "Living room TV", "Pair a device"], ["Put Alan Wake 2 aside"]],
  ["qam-streaming-pin", ["Game streaming (Sunshine)", "PIN", "Pair", "Cancel"], ["Pair a device", "4", "2", "0"]],
]) {
  await page.setViewportSize({ width: 1280, height: 800 });
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${theme}</style><style>body{margin:0;width:1280px;height:800px;overflow:hidden;position:relative;background:#0c1016;font-family:"Segoe UI","Noto Sans",sans-serif}
.DialogButton{border:none;color:#e9eef4;background:rgba(233,238,244,.1);border-radius:6px;font:inherit;cursor:pointer}
.DialogButton{background:#1d2531 !important}.DialogButton.gpfocus{background:#3fd0b8 !important;color:#06231e !important}${frame}</style></head>
<body><div id="root" style="position:absolute;inset:0"></div><script>window.HEARTH_PAGE="qam-rest";window.HEARTH_RUNNING=${JSON.stringify(running)};window.HEARTH_LOGO=${JSON.stringify(logo)};window.HEARTH_WALL=${JSON.stringify(wall)};</script><script>${js}</script></body></html>`;
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(2500);
  for (const c of clicks) {
    const ok = await page.evaluate((t) => { const b = [...document.querySelectorAll("button")].find((x) => x.textContent.trim().startsWith(t)); if (b) b.click(); return !!b; }, c);
    if (!ok) errors.push(`${name}: no button "${c}" to press`);
    await page.waitForTimeout(900);
  }
  await page.evaluate(() => { const b = [...document.querySelectorAll(".qam-body button")].find((x) => !x.disabled); if (b) b.classList.add("gpfocus"); });
  await page.waitForTimeout(300);
  const text = await page.evaluate(() => document.body.innerText);
  const missing = want.filter((w) => !text.toLowerCase().includes(w.toLowerCase()));
  if (missing.length) errors.push(`${name}: missing ${missing.join(", ")}`);
  if (name === "qam-streaming-pin" && !text.includes("420_")) errors.push(`${name}: the PIN typed so far isn't shown`);
  await page.screenshot({ path: join(out, name + ".png") });
  console.log(`drew ${name}${missing.length ? " (missing " + missing.join(", ") + ")" : ""}`);
}
// The pages that change things (manage.tsx) and the Store's details with a download's progress, taller so the whole
// page shows. "click": buttons pressed first (by their text), as a controller's A would.
for (const [name, which, want, clicks, height] of [
  ["takeover-store-detail", "store", ["Cyberpunk 2077: Phantom Liberty", "In your library", "Cancel download", "42%", "GOG page"], ["GOG", "Cyberpunk 2077: Phantom Liberty"], 800],
  ["takeover-mods", "mods", ["Hearth Mods", "Skyrim Special Edition", "SkyUI", "Turn off", "Turn on", "Uninstall", "Immersive Weather", "Install"], [], 1100],
  ["takeover-storage", "storage", ["Storage", "This device", "SD card", "Use this disk for games", "Move to SD card", "Move in Steam's Storage", "Uninstall in Steam", "Uninstall it in Battle.net.", "Stop", "Portal 2", "Delete 2 items"], ["Find things to clean up"], 1900],
  ["takeover-coop", "coop", ["Split-screen", "Dragon", "Everyone (4)", "Side by side", "2 × 2", "Kart Party", "Play split-screen", "Add to Steam library", "Can't play split-screen", "EasyAntiCheat", "Steam Input"], [], 1250],
  ["takeover-recomps", "recomps", ["Hearth Recomps", "Zelda 64: Recompiled", "Play", "Banjo: Recompiled", "Downloading", "Cancel", "Install"], [], 900],
  ["takeover-friends", "friends", ["Hearth Friends", "Sarah", "Playing Starfall Drift", "Mike", "Online", "Away", "Offline", "Chats and requests in Hearth"], [], 800],
  ["takeover-link", "link", ["Hearth Link", "Dragon's Ally", "Living room PC", "Sync now", "Not on right now", "Hades", "4.2 GB of 11.0 GB", "Hearth Storage", "Basement NAS", "1.8 TB free", "Wake it", "Pairing and settings in Hearth"], [], 1100],
]) {
  await page.setViewportSize({ width: 1280, height });
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${theme}</style><style>body{margin:0;width:1280px;height:${height}px;overflow:hidden;position:relative;background:#0c1016;font-family:"Segoe UI","Noto Sans",sans-serif}
.DialogButton{border:none;color:#e9eef4;background:rgba(233,238,244,.1);border-radius:6px;font:inherit;cursor:pointer}
.DialogButton{background:#1d2531 !important}.DialogButton.gpfocus{background:#3fd0b8 !important;color:#06231e !important}</style></head>
<body><div id="root" style="position:absolute;inset:0"></div><script>window.HEARTH_PAGE=${JSON.stringify(which)};window.HEARTH_LOGO=${JSON.stringify(logo)};window.HEARTH_WALL=${JSON.stringify(wall)};</script><script>${js}</script></body></html>`;
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.waitForTimeout(1500);
  for (const c of clicks) {
    const ok = await page.evaluate((t) => { const b = [...document.querySelectorAll("button")].find((x) => x.textContent.trim().startsWith(t)); if (b) b.click(); return !!b; }, c);
    if (!ok) errors.push(`${name}: no button "${c}" to press`);
    await page.waitForTimeout(1500);
  }
  // The first button focused, as Steam would (autoFocus) for the controller.
  await page.evaluate(() => { const b = document.querySelector(".ht-btn.primary") || document.querySelector(".ht-btn"); if (b) b.classList.add("gpfocus"); });
  await page.waitForTimeout(300);
  const text = await page.evaluate(() => document.body.innerText);
  const missing = want.filter((w) => !text.toLowerCase().includes(w.toLowerCase()));
  if (missing.length) errors.push(`${name}: missing ${missing.join(", ")}`);
  if (/couldn't be shown/i.test(text)) errors.push(`${name}: a Hearth part fell back to its error`);
  await page.screenshot({ path: join(out, name + ".png") });
  console.log(`drew ${name}${missing.length ? " (missing " + missing.join(", ") + ")" : ""}`);
}
await browser.close();
if (errors.length) { console.log(`::error title=takeover pictures::${errors.join(" | ").slice(0, 3000)}`); process.exit(1); }
