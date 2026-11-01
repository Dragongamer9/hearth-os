// CI only (decky-hearth.yml): draws a static stand-in for Steam's gamepad UI (home, library tabs, a game's play bar,
// Quick Access with the Hearth panel, the keyboard) using Steam's own CSS variables, focus class and component names,
// first as Steam looks, then with Hearth's theme, and saves both as pictures. Real Steam isn't involved: this shows
// what the theme's rules do, not that Steam's current names still match (that needs a device).
// Usage: HEARTH_ROOT=<repo> node render-mock.mjs OUTDIR   (needs playwright; CI copies it next to tools/node_modules)
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = process.env.HEARTH_ROOT || join(here, "..", "..", "..");
const out = process.argv[2] || "shots";
mkdirSync(out, { recursive: true });
const b64 = (p, mime) => `data:${mime};base64,${readFileSync(p).toString("base64")}`;
const logo = b64(join(root, "site/img/logo.svg"), "image/svg+xml");
const wall = b64(join(root, "os/branding/wallpapers/hearth-embers.jpg"), "image/jpeg");
// The same filling-in as src/theme.ts renderTheme, with the mock's readable names standing in for Steam's hashed ones.
const theme = readFileSync(join(root, "hosts/decky-hearth/defaults/hearth-theme.css"), "utf8").split("\n")
  .map((l) => l.replace(/\{\{([A-Za-z0-9_]+):([A-Za-z0-9_]+)\}\}/g, (_a, _m, k) => k).replace(/%WALLPAPER%/g, wall).replace(/%LOGO%/g, logo))
  .join("\n");

// Steam's own look, roughly (its variables' usual values), for the "before" picture.
const steam = `
:root{--gpSystemDarkestGrey:#0e141b;--gpSystemDarkerGrey:#23262e;--gpSystemDarkGrey:#3d4450;--gpSystemGrey:#67707b;--gpSystemLightGrey:#8b929a;--gpSystemLighterGrey:#b8bcbf;--gpSystemLightestGrey:#dcdedf;--gpColor-Blue:#1a9fff;--gpColor-Green:#59bf40;--gpCornerSmall:2px;--gpCornerMedium:4px;--gpCornerLarge:6px;--gpBackground-LightSoft:rgba(255,255,255,.1)}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:"Motiva Sans",Arial,sans-serif;background:var(--gpSystemDarkestGrey);color:var(--gpSystemLightestGrey);width:1280px;height:800px;overflow:hidden;position:relative}
.GamepadTabbedPage{position:absolute;inset:0 380px 0 0;background:var(--gpSystemDarkestGrey)}
.TabHeaderRowWrapper{display:flex;gap:8px;padding:14px 24px;background:var(--gpSystemDarkerGrey)}
.Tab{padding:6px 14px;border-radius:var(--gpCornerMedium);color:var(--gpSystemLightGrey)}
.Tab.Selected{background:var(--gpSystemLightestGrey);color:var(--gpSystemDarkestGrey)}
h3{margin:22px 24px 10px;font-size:14px;letter-spacing:.04em;text-transform:uppercase;color:var(--gpSystemLightGrey)}
.row{display:flex;gap:14px;padding:0 24px}
.cap{width:120px;height:170px;border-radius:var(--gpCornerMedium);background:var(--gpSystemDarkGrey);display:flex;align-items:flex-end;padding:8px;font-size:12px}
.TopCapsule{margin:18px 24px 0;height:150px;border-radius:var(--gpCornerLarge);background:linear-gradient(135deg,#2b3a55,#141b26);display:flex;align-items:flex-end;padding:16px;font-size:28px;font-weight:700}
.PlayBar{position:absolute;left:24px;right:24px;bottom:24px;height:76px;border-radius:var(--gpCornerLarge);display:flex;align-items:center;gap:16px;padding:0 16px;background:var(--gpSystemDarkerGrey)}
.InnerContainer{flex:1;height:40px;border-radius:var(--gpCornerMedium);padding:10px;color:var(--gpSystemLightGrey);font-size:13px}
.PlayButton{padding:12px 34px;border-radius:var(--gpCornerMedium);background:var(--gpColor-Green);color:#fff;font-weight:700}
.QuickAccessMenu{position:absolute;top:0;right:0;bottom:0;width:380px;display:flex;background:var(--gpSystemDarkerGrey)}
.Tabs{width:56px;display:flex;flex-direction:column;gap:6px;padding:10px 6px;background:var(--gpSystemDarkestGrey)}
.Tabs .Tab{height:44px;padding:0}
.TabContentColumn{flex:1;padding:14px}
.Title{font-size:18px;font-weight:700;margin-bottom:12px}
.Field{padding:12px;border-radius:var(--gpCornerMedium);margin-bottom:6px;background:var(--gpBackground-LightSoft)}
.FieldDescription{font-size:12px;color:var(--gpSystemLightGrey);margin-top:3px}
.Button{display:block;text-align:center;padding:10px;margin-bottom:6px;border-radius:var(--gpCornerMedium);background:var(--gpSystemDarkGrey)}
.gpfocus{box-shadow:0 0 0 2px #fff}
.VirtualKeyboardContainer{position:absolute;left:0;right:380px;bottom:0;height:200px;padding:12px;display:grid;grid-template-columns:repeat(10,1fr);gap:6px;background:var(--gpSystemDarkerGrey)}
.Key{display:grid;place-items:center;border-radius:var(--gpCornerSmall);background:var(--gpSystemDarkGrey);font-size:15px}
.BasicFooter{position:absolute;left:0;right:380px;bottom:0;height:0}
`;
const caps = ["Fortnite", "The Witcher 3", "BFME II", "Diablo IV", "Halo MCC", "Celeste", "Hades"];
const body = (kbd) => `
<div class="GamepadTabbedPage">
  <div class="TabHeaderRowWrapper"><span class="Tab Selected">Home</span><span class="Tab">Library</span><span class="Tab">Collections</span><span class="Tab">Non-Steam</span></div>
  <div class="TopCapsule">The Witcher 3</div>
  <h3>Recent games</h3>
  <div class="row">${caps.map((c, i) => `<div class="cap${i === 1 ? " gpfocus" : ""}">${c}</div>`).join("")}</div>
  ${kbd ? `<div class="VirtualKeyboardContainer">${"QWERTYUIOPASDFGHJKL;ZXCVBNM,./".split("").map((k, i) => `<span class="Key${i === 13 ? " gpfocus" : ""}">${k}</span>`).join("")}</div>`
        : `<div class="PlayBar"><span class="PlayButton gpfocus">Play</span><div class="InnerContainer">Last played today · 12 hours played · via Hearth</div></div>`}
</div>
<div class="QuickAccessMenu"><div class="Tabs"><span class="Tab"></span><span class="Tab Selected"></span><span class="Tab"></span></div>
  <div class="TabContentColumn"><div class="Title">Hearth</div>
    <div class="Field gpfocus">Switch to Hearth Game Mode<div class="FieldDescription">Steam closes and Hearth's own Game Mode comes back</div></div>
    <div class="Field">Hearth look<div class="FieldDescription">Hearth's colours, focus, wallpaper and logo over Steam</div></div>
    <span class="Button">Sync to Steam library</span><span class="Button">Fortnite · Play</span><span class="Button">BFME II · Play</span>
  </div></div>`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
for (const [name, css, kbd] of [["steam-plain", "", false], ["steam-hearth", theme, false], ["steam-hearth-keyboard", theme, true]]) {
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>${steam}</style><style id="hearth-theme">${css}</style></head><body>${body(kbd)}</body></html>`;
  writeFileSync(join(out, name + ".html"), html);
  await page.setContent(html);
  await page.screenshot({ path: join(out, name + ".png") });
  console.log("drew " + name);
}
await browser.close();
