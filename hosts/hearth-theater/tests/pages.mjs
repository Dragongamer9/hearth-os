// Opens Hearth Theater's page in Chromium with its sample sites (no app: the page answers its own calls), drives it
// with the keyboard, and runs the viewer's script (src/viewer.js) on a test page with a video and a frame, failing on
// any script error. Screenshots go to shots-pages/.
//   node tests/pages.mjs            (from hosts/hearth-theater; needs the playwright package)
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const ui = path.join(here, '..', 'ui');
const out = process.env.HEARTH_THEATER_SHOTS || path.join(here, '..', 'shots-pages');
mkdirSync(out, { recursive: true });
const browser = await chromium.launch(process.env.CHROME ? { executablePath: process.env.CHROME } : {});
const errors = [];
const sleep = ms => new Promise(r => setTimeout(r, ms));
const shot = (p, n) => p.screenshot({ path: path.join(out, `${n}.png`) });

// ---- the wall
{
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const p = await ctx.newPage();
  p.on('pageerror', e => errors.push(`wall: ${e.message}`));
  p.on('console', m => { if (m.type() === 'error' && !/favicon/.test(m.text())) errors.push(`wall console: ${m.text()}`); });
  await p.goto(pathToFileURL(path.join(ui, 'index.html')).href);
  await sleep(500);
  await shot(p, 'wall');
  if (await p.locator('.tile[data-id]').count() < 4) errors.push('wall: the sample tiles are missing');
  await p.keyboard.press('ArrowRight'); await sleep(200);
  await p.click('[data-tab="Anime"]'); await sleep(300);
  await shot(p, 'wall-anime');
  if (await p.locator('.grid-tiles .tile[data-id]').count() !== 1) errors.push('wall: the Anime shelf should have one tile');
  await p.click('#addBtn'); await sleep(300);
  await p.fill('#u', 'example.org/shows'); await sleep(1100);
  await shot(p, 'add');
  if (await p.inputValue('#n') !== 'Sample site') errors.push("add: the name wasn't filled in from the site");
  await p.click('[data-a="save"]'); await sleep(300);
  if (await p.locator('.tile[data-id]').count() !== 2) errors.push('add: the new tile is missing from the Anime shelf');
  await p.click('[data-tab="All"]'); await sleep(200);
  await p.click('.tile[data-id="s2"]', { button: 'right' }); await sleep(300);
  await shot(p, 'options');
  await ctx.close();
}

// ---- the viewer's script on a page with a video in it and one in a frame
{
  const dir = path.join(out, 'viewer-page');
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, 'frame.html'), `<!doctype html><body style="margin:0;background:#123"><video id=v width=320 height=180 style="background:#333"></video><button id=b style="position:absolute;left:10px;top:200px" onclick="parent.postMessage('frame-clicked','*')">in frame</button></body>`);
  writeFileSync(path.join(dir, 'index.html'), `<!doctype html><body style="margin:0;height:3000px;background:#222;color:#fff">
    <button id=b style="position:absolute;left:600px;top:380px;width:120px;height:60px" onclick="window.clicked=(window.clicked||0)+1">click me</button>
    <iframe id=f src="frame.html" style="position:absolute;left:40px;top:40px;width:400px;height:260px;border:0"></iframe>
    <script>addEventListener('message',e=>{ if(e.data==='frame-clicked') window.frameClicked=1; })</script></body>`);
  const js = readFileSync(path.join(here, '..', 'src', 'viewer.js'), 'utf8');
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  await ctx.addInitScript(js);
  const p = await ctx.newPage();
  p.on('pageerror', e => errors.push(`viewer: ${e.message}`));
  await p.goto(pathToFileURL(path.join(dir, 'index.html')).href);
  await sleep(700);
  await shot(p, 'viewer');
  if (!await p.evaluate(() => !!document.querySelector('hearth-theater-ui'))) errors.push("viewer: the pointer layer isn't on the page");
  // A fake controller: press A with the pointer in the middle of the screen (where "click me" is).
  const pad = async (buttons, axes = [0, 0, 0, 0], ms = 120) => {
    await p.evaluate(([b, a]) => {
      const g = { id: 'test', index: 0, connected: true, axes: a, buttons: Array.from({ length: 17 }, (_, i) => ({ pressed: b.includes(i), value: b.includes(i) ? 1 : 0 })) };
      navigator.getGamepads = () => [g];
    }, [buttons, axes]);
    await sleep(ms);
  };
  await pad([0]); await pad([]);
  if (await p.evaluate(() => window.clicked) !== 1) errors.push("viewer: A didn't click under the pointer");
  // Y fills the screen with the biggest player (the frame), Y again puts it back.
  await pad([3]); await pad([]);
  const filled = await p.evaluate(() => getComputedStyle(document.getElementById('f')).position);
  if (filled !== 'fixed') errors.push("viewer: Y didn't fill the screen with the player");
  await shot(p, 'viewer-fill');
  await pad([3]); await pad([]);
  if (await p.evaluate(() => getComputedStyle(document.getElementById('f')).position) === 'fixed') errors.push("viewer: Y again didn't put the player back");
  // The right stick scrolls.
  await pad([], [0, 0, 0, 1], 300); await pad([]);
  if (await p.evaluate(() => scrollY) < 50) errors.push("viewer: the right stick didn't scroll");
  // View goes back to Theater (the viewer asks for hearth-theater://close; here that's just a failed navigation).
  const asked = new Promise(res => p.on('request', r => { if (r.url().startsWith('hearth-theater:')) res(true); }));
  await pad([8]); await pad([]);
  if (!await Promise.race([asked, sleep(1500).then(() => false)])) errors.push("viewer: View didn't ask to go back to Theater");
  await ctx.close();
}

await browser.close();
if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
console.log('Hearth Theater pages: all fine');
