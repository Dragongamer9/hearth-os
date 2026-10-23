// Opens each Hearth Time page in Chromium with its sample data (no app: each page answers its own calls), drives it
// a little with the keyboard, and fails on any script error. Screenshots go to shots-pages/.
//   node tests/pages.mjs            (from hosts/hearth-time; needs the playwright package)
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const ui = process.env.HEARTH_TIME_UI || path.join(here, '..', 'ui');
const out = process.env.HEARTH_TIME_SHOTS || path.join(here, '..', 'shots-pages');
mkdirSync(out, { recursive: true });

const browser = await chromium.launch();
const errors = [];
const sleep = ms => new Promise(r => setTimeout(r, ms));

async function page(name, steps, size = { width: 1280, height: 800 }) {
  const ctx = await browser.newContext({ viewport: size, deviceScaleFactor: 1 });
  const p = await ctx.newPage();
  p.on('pageerror', e => errors.push(`${name}: ${e.message}`));
  p.on('console', m => { if (m.type() === 'error') errors.push(`${name} console: ${m.text()}`); });
  await p.goto(pathToFileURL(path.join(ui, `${name}.html`)).href);
  await sleep(600);
  await p.screenshot({ path: path.join(out, `${name}.png`) });
  for (const [label, fn] of steps) {
    await fn(p);
    await sleep(400);
    await p.screenshot({ path: path.join(out, `${name}-${label}.png`) });
  }
  await ctx.close();
}
const key = k => async p => { for (const x of k.split(' ')) await p.keyboard.press(x); };

await page('clock', [
  ['edit', key('Enter')],
  ['timers', async p => { await p.keyboard.press('Escape'); await p.keyboard.press('2'); }],
  ['new-timer', key('n')],
  ['stopwatch', async p => { await p.keyboard.press('Escape'); await p.keyboard.press('3'); }],
  ['world', key('4')],
  ['add-city', async p => { await p.keyboard.press('n'); await p.keyboard.type('tok'); }],
]);
await page('calendar', [
  ['week', async p => { await p.keyboard.press('w'); }],
  ['day', key('d')],
  ['agenda', key('a')],
  ['month-move', async p => { await p.keyboard.press('m'); await sleep(300); await p.keyboard.press('ArrowRight'); await p.keyboard.press('ArrowDown'); }],
  ['new', key('n')],
  ['menu', async p => { await p.keyboard.press('Escape'); await p.click('#menubtn'); }],
  ['import', async p => { await p.click('[data-m="import"]'); }],
]);
await page('notes', [
  ['open', async p => { await p.click('.nt'); }],
  ['edit', async p => { await p.click('[data-act="mode"]'); await p.keyboard.press('End'); await p.keyboard.type('\n- [ ] typed in the test'); }],
  ['view', async p => { await p.keyboard.press('Escape'); }],
  ['search', async p => { await p.fill('#q', 'wi-fi'); }],
]);
await page('weather', [
  ['place', key('c')],
  ['search', async p => { await p.keyboard.type('Por'); await sleep(700); }],
  ['picked', async p => { await p.click('#res .card'); await sleep(300); }],
]);
await page('weather', [], { width: 820, height: 560 });
await page('ring', []);
await page('clock', [], { width: 800, height: 520 });
await page('calendar', [], { width: 1024, height: 640 });

await browser.close();
if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
console.log('All pages drew without script errors');
