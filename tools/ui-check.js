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
  // Settings › Controls › Handheld Daemon: every section, with advanced settings shown and hidden, a choice list,
  // a slider moved, and a change made (the preview's HHD is a ROG Ally's).
  await step('screen hhd', async () => { S.layers = []; navigate('hhd', {}, false); await new Promise(r => setTimeout(r, 600)); if (!HHDP.schema) throw new Error('Handheld Daemon page has no settings'); });
  const hhdSecs = await page.evaluate(() => typeof hhdSections === 'function' && HHDP.schema ? hhdSections() : []);
  if (!hhdSecs.length) errors.push('hhd: no sections');
  for (const adv of [false, true])
    for (const s of hhdSecs) await step(`hhd ${s}${adv ? ' (advanced)' : ''}`, ([s, adv]) => {
      HHDP.adv = adv; HHDP.sec = s; render(null, true);
      if (!document.querySelector('#view [data-k="hh-adv"]')) throw new Error('page not drawn');
    }, [s, adv]);
  await step('hhd pick', () => { HHDP.sec = 'controllers'; render(null, true); hhdAct('pick', 'controllers.rog_ally.controller_mode.mode'); if (!S.layers.includes('sheet')) throw new Error('no choice list'); closeAll(); });
  await step('hhd slider', () => { HHDP.sec = 'controllers'; render(null, true); const el = document.querySelector('#view [data-slider="hhd:controllers.rog_ally.imu_hz"]'); if (!el) throw new Error('no slider'); adjustSlider(el, 1); });
  await step('hhd switch', async () => { await hhdWrite('controllers.rog_ally.imu', false); if (hv('controllers.rog_ally.imu') !== false) throw new Error('change not shown'); });
  await step('hhd power mode', async () => { HHDP.sec = 'tdp'; await hhdWrite('tdp.unified.tdp.mode', 'custom'); if (!document.querySelector('#view [data-slider="hhd:tdp.unified.tdp.custom.tdp"]')) throw new Error('custom power limit not shown'); });
  await step('hhd leave', () => { S.layers = []; navigate('home', {}, false); });
  const tabs = await page.evaluate(() => typeof STABS !== 'undefined' ? STABS.map(t => t[0]) : []);
  for (const t of tabs) await step('settings ' + t, t => { S.settingsTab = t; navigate('settings', {}, false); }, t);
  const layouts = await page.evaluate(() => [...(typeof LAYOUTS !== 'undefined' ? LAYOUTS : []), ...(typeof XLAYOUTS !== 'undefined' ? XLAYOUTS : [])].map(l => l.id));
  for (const id of layouts) await step('layout ' + id, id => { LOOK.layout = id; saveLook(); S.layers = []; navigate('home', {}, false); }, id);
  // Quick Access › Performance, as Hearth OS (?preview=linux): with Handheld Daemon (its own power settings in place
  // of Hearth's), without it (?nohhd: Hearth's power mode and limit, as before), and over a game (the overlay page),
  // where a power change is kept for the game.
  const perfModes = [['hhd', 'preview=linux'], ['no hhd', 'preview=linux&nohhd'], ['overlay hhd', 'preview=linux&overlay'], ['overlay no hhd', 'preview=linux&overlay&nohhd']];
  for (const [name, q] of perfModes) {
    await page.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?' + q);
    await page.waitForTimeout(1200);
    const hhd = !q.includes('nohhd'), overlay = q.includes('overlay');
    await step(`performance (${name})`, async ([hhd, overlay]) => {
      if (overlay) { await overlayShown(); S.running = (GAMES.find(g => !g.cloud) || GAMES[0]).id; }   // a game plays (the preview runs none)
      else { closeAll(); openLayer('qam'); }
      S.qtab = 'perf'; renderQam(); startPerf();
      await new Promise(r => setTimeout(r, 1500));
      const q = document.querySelector('#qam .qbody');
      const has = sel => !!q.querySelector(sel);
      if (!has('[data-act="fpsLimit"]') || !has('[data-act="perfOverlay"]')) throw new Error("Hearth's frame-rate limit or overlay is missing");
      if (hhd) {
        if (!has('.qhhd [data-key="tdp.unified.tdp.mode"]')) throw new Error("Handheld Daemon's power mode isn't shown");
        if (has('[data-act="tdp"]') || has('[data-act="perfPower"]')) throw new Error("Hearth's own power controls show next to Handheld Daemon's");
        if (has('[data-slider="hhd:tdp.unified.fan.manual_edge.st40"]')) throw new Error('a fan curve point shows in Quick Access');
        if (!overlay !== has('[data-act="hhdOpen"]')) throw new Error('the Handheld Daemon settings button is wrong for this page');
      } else {
        if (has('.qhhd')) throw new Error("Handheld Daemon's settings show without it");
        if (!has('[data-act="tdp"]') || !has('[data-act="perfPower"]')) throw new Error("Hearth's power controls are missing");
      }
    }, [hhd, overlay]);
    if (!hhd) continue;
    // The custom mode's limit as a slider (moved with the controller), kept for the game when one plays.
    await step(`performance custom limit (${name})`, async (overlay) => {
      await hhdWrite('tdp.unified.tdp.mode', 'custom');
      const el = document.querySelector('#qam [data-slider="hhd:tdp.unified.tdp.custom.tdp"]');
      if (!el) throw new Error('custom power limit not shown');
      if (overlay && !(PERF.s.gamePower && PERF.s.gamePower.saved)) throw new Error('the change was not kept for the game');
      if (overlay && !document.querySelector('#qam [data-act="gamePowerForget"]')) throw new Error('no way back to the usual power settings');
    }, overlay);
  }
  await browser.close();
  if (errors.length) {
    for (const e of errors) console.log(`::error title=Hearth page::${e}`);
    process.exit(1);
  }
  console.log(`ok: Hearth's page loads and draws ${9 + tabs.length + layouts.length} screens (and ${hhdSecs.length * 2} Handheld Daemon sections, and Quick Access › Performance ${perfModes.length} ways) without errors`);
})();
