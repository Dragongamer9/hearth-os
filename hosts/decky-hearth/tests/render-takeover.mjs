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
  alias: { "@decky/ui": join(mock, "stub-ui.jsx"), "@decky/api": join(mock, "stub-api.js") },
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
await browser.close();
if (errors.length) { console.log(`::error title=takeover pictures::${errors.join(" | ").slice(0, 3000)}`); process.exit(1); }
