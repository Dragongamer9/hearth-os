// Release check: Hearth's page (ui/index.html) must load and draw every screen without a script error.
// A single error can leave the app blank, so a release that fails this isn't published.
//   npm i --no-save playwright@1.48.2 && npx playwright install chromium && node tools/ui-check.js
const { chromium } = require('playwright');
const path = require('path');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error' && !/Failed to load resource|net::ERR/.test(m.text())) errors.push('console: ' + m.text()); });
  await page.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
  await page.waitForTimeout(1500);
  const step = async (what, fn, arg) => {
    try { await page.evaluate(fn, arg); await page.waitForTimeout(250); }
    catch (e) { errors.push(`${what}: ${e.message.split('\n')[0]}`); }
  };
  // Every screen, every Settings page, every home layout.
  for (const v of ['home', 'library', 'downloads', 'friends', 'shots', 'stores'])
    await step('screen ' + v, v => { S.layers = []; navigate(v, {}, false); }, v);
  // A game's settings, and its controller layout editor (Hearth Input).
  await step('screen gameset', () => { S.layers = []; navigate('gameset', {game: (GAMES.find(g => !g.cloud) || GAMES[0]).id}, false); });
  await step('screen inputmap', async () => {
    const g = GAMES.find(x => !x.cloud) || GAMES[0];
    imLoadTables(); imLoad(g.id, true);
    await new Promise(r => setTimeout(r, 200));
    imEdit(g.id);
  });
  const tabs = await page.evaluate(() => typeof STABS !== 'undefined' ? STABS.map(t => t[0]) : []);
  for (const t of tabs) await step('settings ' + t, t => { S.settingsTab = t; navigate('settings', {}, false); }, t);
  const layouts = await page.evaluate(() => [...(typeof LAYOUTS !== 'undefined' ? LAYOUTS : []), ...(typeof XLAYOUTS !== 'undefined' ? XLAYOUTS : [])].map(l => l.id));
  for (const id of layouts) await step('layout ' + id, id => { LOOK.layout = id; saveLook(); S.layers = []; navigate('home', {}, false); }, id);
  await browser.close();
  if (errors.length) {
    for (const e of errors) console.log(`::error title=Hearth page::${e}`);
    process.exit(1);
  }
  console.log(`ok: Hearth's page loads and draws ${8 + tabs.length + layouts.length} screens without errors`);
})();
