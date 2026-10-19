// Bug finder: on every screen, can the controller reach everything? Starting from where the focus lands, it
// presses up/down/left/right from every reachable control (breadth first) and lists the controls it never got
// to, and any script errors. A control nobody can reach with the D-pad is a bug (like Shut down being skipped).
//   node tools/ui-nav-check.js            (needs playwright, like ui-check.js)
const { chromium } = require('playwright');
const path = require('path');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
  await page.waitForTimeout(1500);
  await page.evaluate(() => { HOSTINFO.features = ['quickResume', 'friends', 'screenshots', 'sendLogs', 'xodus', 'profiles']; });

  // Explores the controls of whatever is open now; returns the unreachable ones.
  const explore = () => page.evaluate(() => {
    const root = typeof window.root === 'function' ? window.root() : document;
    const all = () => [...root.querySelectorAll('[data-f]')].filter(el => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0 && !el.disabled; });
    const key = el => el.dataset.k || el.textContent.trim().slice(0, 30);
    const start = focused && root.contains(focused) ? focused : all()[0];
    if (!start) return { total: 0, missed: [] };
    const seen = new Set([key(start)]), queue = [key(start)];
    let steps = 0;
    while (queue.length && steps < 4000) {
      const k = queue.shift();
      for (const dir of ['up', 'down', 'left', 'right']) {
        const el = all().find(e => key(e) === k); if (!el) break;
        setFocus(el, true); steps++;
        try { moveNow(dir); } catch (e) { return { error: e.message }; }
        if (focused && root.contains(focused)) { const nk = key(focused); if (!seen.has(nk)) { seen.add(nk); queue.push(nk); } }
      }
    }
    const missed = all().map(key).filter(k => !seen.has(k));
    return { total: all().length, missed: [...new Set(missed)] };
  });

  const report = [];
  const check = async (what, open) => {
    try { await page.evaluate(open); await page.waitForTimeout(200); } catch (e) { errors.push(`${what}: ${e.message.split('\n')[0]}`); return; }
    const r = await explore();
    if (r.error) errors.push(`${what}: ${r.error}`);
    else if (r.missed.length) report.push(`${what}: can't reach ${r.missed.length} of ${r.total}: ${r.missed.slice(0, 12).join(', ')}`);
  };
  for (const v of ['home', 'library', 'downloads', 'friends', 'shots', 'stores'])
    await check('screen ' + v, `(() => { closeAll(); S.history = []; navigate('${v}', {}, false); })()`);
  const tabs = await page.evaluate(() => STABS.map(t => t[0]));
  for (const t of tabs) await check('settings ' + t, `(() => { closeAll(); S.settingsTab = '${t}'; navigate('settings', {}, false); })()`);
  const layouts = await page.evaluate(() => [...LAYOUTS, ...(typeof XLAYOUTS !== 'undefined' ? XLAYOUTS : [])].map(l => l.id));
  for (const id of layouts) await check('layout ' + id, `(() => { closeAll(); LOOK.layout = '${id}'; saveLook(); navigate('home', {}, false); })()`);
  await page.evaluate(() => { LOOK.layout = 'shelf'; saveLook(); });
  for (const layer of ['menu', 'power', 'qam', 'search'])
    await check('layer ' + layer, `(() => { closeAll(); navigate('home', {}, false); openLayer('${layer}'); })()`);
  await check('game page', `(() => { closeAll(); navigate('game', { game: GAMES[0].id }, false); })()`);

  await browser.close();
  for (const e of errors) console.log('ERROR ' + e);
  for (const r of report) console.log('UNREACHABLE ' + r);
  console.log(`${errors.length} errors, ${report.length} screens with controls the D-pad can't reach`);
  process.exit(errors.length ? 1 : 0);
})();
