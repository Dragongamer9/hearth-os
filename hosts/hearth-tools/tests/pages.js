// Opens every Hearth Tools page in Chromium on its own sample data (the pages' preview mode, no Tauri), clicks
// through each tab, moves around with the arrow keys, and fails on any script error. Saves a screenshot of
// each tab in hosts/hearth-tools/shots/pages/. Run: node tests/pages.js (with playwright installed).
const path = require('path');
const fs = require('fs');
const http = require('http');
const { chromium } = require('playwright');

const UI = path.join(__dirname, '..', 'ui');
const OUT = path.join(__dirname, '..', 'shots', 'pages');
const APPS = ['disk-usage', 'system-info', 'logs', 'network', 'passwords', 'printers', 'welcome', 'benchmark'].filter(a => !process.env.ONLY || process.env.ONLY.split(',').includes(a));

// The pages fetch help.json, which needs http (not file://): a tiny server for ui/.
function serve() {
  const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.woff2': 'font/woff2', '.svg': 'image/svg+xml' };
  const srv = http.createServer((req, res) => {
    const f = path.join(UI, path.normalize(decodeURIComponent(req.url.split('?')[0])).replace(/^([/\\])+/, ''));
    if (!f.startsWith(UI) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'content-type': types[path.extname(f)] || 'application/octet-stream' }); fs.createReadStream(f).pipe(res);
  });
  return new Promise(r => srv.listen(0, '127.0.0.1', () => r(srv)));
}

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const srv = await serve();
  const base = `http://127.0.0.1:${srv.address().port}/`;
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', e => errors.push(`${page.url().split('/').pop()}: ${e.message}`));
  page.on('console', m => { if (m.type() === 'error' && !/favicon|ERR_FILE_NOT_FOUND/.test(m.text())) errors.push(`${page.url().split('/').pop()}: console: ${m.text()}`); });
  for (const app of APPS) {
    await page.goto(base + app + '.html');
    await page.waitForTimeout(900);
    const tabs = await page.$$eval('[data-tab]', els => els.map(e => e.dataset.tab));
    const list = tabs.length ? tabs : [''];
    for (const t of list) {
      if (t) { await page.click(`[data-tab="${t}"]`); await page.waitForTimeout(700); }
      await page.keyboard.press('ArrowDown'); await page.keyboard.press('ArrowDown'); await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(200);
      await page.screenshot({ path: path.join(OUT, `${app}${t ? '-' + t : ''}.png`) });
      const blank = await page.evaluate(() => (document.querySelector('#main')?.innerText || '').trim().length);
      if (!blank) errors.push(`${app}${t ? ' / ' + t : ''}: the page is empty`);
    }
    await page.keyboard.press('Escape');
    if (app === 'benchmark') {
      // A Quick test on the sample data: the on-battery warning, the live view, then B (Escape) stops it.
      await page.click('[data-tab="run"]'); await page.waitForTimeout(300);
      await page.click('[data-run="full"]'); await page.waitForTimeout(300);
      if (!(await page.$('.dlg'))) errors.push('benchmark: no on-battery warning before a Full test');
      await page.keyboard.press('Escape'); await page.waitForTimeout(200);
      await page.click('[data-run="quick"]'); await page.waitForTimeout(1500);
      await page.screenshot({ path: path.join(OUT, 'benchmark-running.png') });
      if (!(await page.$('#stop'))) errors.push('benchmark: the live view didn\'t show');
      await page.keyboard.press('Escape'); await page.waitForTimeout(700);
      if (await page.$('#stop')) errors.push('benchmark: B didn\'t stop the run');
      await page.click('[data-run="quick"]'); await page.waitForTimeout(6500);
      if (!(await page.$('.score'))) errors.push('benchmark: no results after a run');
      await page.screenshot({ path: path.join(OUT, 'benchmark-done.png') });
    }
  }
  await browser.close();
  srv.close();
  if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
  console.log(`all ${APPS.length} pages drew without errors; screenshots in ${OUT}`);
})().catch(e => { console.error(e); process.exit(1); });
