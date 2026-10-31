// Website check (the Website workflow runs it on every build): serves site/ locally, opens the front page and every
// docs page at phone, tablet and desktop width, and looks for what breaks a page for people:
//   errors (fail the run): script errors, links to pages/files/anchors on the site that don't exist, images that
//     don't load, a page wider than the window (sideways scrolling on a phone), a front page over its weight budget
//   warnings: links to other sites that don't answer, images without alt, buttons/links without a name, text below
//     WCAG AA contrast, more than one h1, heading levels that skip
// and takes screenshots: after-<width>.jpg (the front page, whole), docs-<page>-<width>.jpg, and before-<width>.jpg
// from the site as it is on main when that's given.
//   node tools/site-check.js SITE_DIR [BEFORE_SITE_DIR] [SHOTS_DIR]     (needs playwright, like layout-audit.js)
const { chromium } = require('playwright');
const http = require('http');
const fs = require('fs');
const path = require('path');

const SITE = path.resolve(process.argv[2] || path.join(__dirname, '../site'));
const BEFORE = process.argv[3] && fs.existsSync(process.argv[3]) ? path.resolve(process.argv[3]) : null;
const OUT = path.resolve(process.argv[4] || path.join(__dirname, '../site-shots'));
const WIDTHS = [['phone', 390, 844], ['tablet', 820, 1180], ['desktop', 1440, 900]];
// Page weight budgets for the front page: before scrolling (lazy images not yet loaded), and in all.
const FIRST_MAX = 1200 * 1024, ALL_MAX = 4 * 1024 * 1024;
let weight = '';
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.webp': 'image/webp',
  '.avif': 'image/avif', '.png': 'image/png', '.jpg': 'image/jpeg', '.woff2': 'font/woff2', '.txt': 'text/plain', '.json': 'application/json' };

function serve(root) {
  const srv = http.createServer((req, res) => {
    let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    let f = path.join(root, p);
    if (!f.startsWith(root)) { res.writeHead(403); return res.end(); }
    if (fs.existsSync(f) && fs.statSync(f).isDirectory()) f = path.join(f, 'index.html');
    if (!fs.existsSync(f)) { res.writeHead(404); return res.end('not found'); }
    res.writeHead(200, { 'content-type': TYPES[path.extname(f)] || 'application/octet-stream' });
    fs.createReadStream(f).pipe(res);
  });
  return new Promise(r => srv.listen(0, '127.0.0.1', () => r(srv)));
}

// Runs in the page: links, images and accessibility problems on what's drawn now.
function inspect() {
  const out = { links: [], warn: [], err: [] };
  const W = innerWidth;
  if (document.documentElement.scrollWidth > W + 1) {
    const wide = [...document.querySelectorAll('body *')].filter(e => { const r = e.getBoundingClientRect(); const s = getComputedStyle(e);
      return r.width && r.right > W + 1 && s.position !== 'fixed' && !(e.parentElement && e.parentElement.getBoundingClientRect().right > W + 1); })
      .slice(0, 4).map(e => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (e.className && typeof e.className === 'string' ? '.' + e.className.split(' ')[0] : ''));
    out.err.push(`the page is ${document.documentElement.scrollWidth - W}px wider than the window: ${wide.join(', ')}`);
  }
  for (const a of document.querySelectorAll('a[href]')) out.links.push(a.getAttribute('href'));
  for (const img of document.querySelectorAll('img')) {
    if (!img.hasAttribute('alt')) out.warn.push(`image without alt: ${img.getAttribute('src')}`);
    if (img.complete && img.naturalWidth === 0 && img.loading !== 'lazy') out.err.push(`image didn't load: ${img.getAttribute('src')}`);
  }
  const named = el => (el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || el.textContent.trim() || el.getAttribute('title') || (el.querySelector('img[alt]:not([alt=""])') ? 'img' : ''));
  for (const el of document.querySelectorAll('a[href], button')) {
    const s = getComputedStyle(el); if (s.display === 'none' || s.visibility === 'hidden' || el.closest('[hidden]')) continue;
    if (!named(el)) out.warn.push(`${el.tagName.toLowerCase()} without a name: ${el.outerHTML.slice(0, 80)}`);
  }
  const h1 = document.querySelectorAll('h1').length; if (h1 !== 1) out.warn.push(`${h1} h1 headings`);
  let last = 0; for (const h of document.querySelectorAll('h1,h2,h3,h4')) { const n = +h.tagName[1]; if (last && n > last + 1) out.warn.push(`heading jumps from h${last} to h${n}: "${h.textContent.trim().slice(0, 40)}"`); last = n; }
  // Contrast: each element with its own text against the first solid background behind it.
  const rgb = c => { const m = c.match(/[\d.]+/g); return m ? m.map(Number) : [0, 0, 0, 0]; };
  const lum = ([r, g, b]) => { const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : Math.pow((v + .055) / 1.055, 2.4); }; return .2126 * f(r) + .7152 * f(g) + .0722 * f(b); };
  const bgOf = el => { for (let p = el; p; p = p.parentElement) { const s = getComputedStyle(p); if (s.backgroundImage !== 'none' && p !== document.body && p !== document.documentElement) return null; const c = rgb(s.backgroundColor); if (c.length < 4 || c[3] > .8) return c; } return [255, 255, 255]; };
  const low = [];
  for (const el of document.querySelectorAll('body *')) {
    if (low.length > 8) break;
    if (![...el.childNodes].some(n => n.nodeType === 3 && n.textContent.trim().length > 1)) continue;
    const r = el.getBoundingClientRect(); if (!r.width || !r.height) continue;
    const s = getComputedStyle(el); if (s.visibility === 'hidden' || +s.opacity < .1 || el.closest('[aria-hidden="true"]')) continue;
    const bg = bgOf(el); if (!bg) continue;
    const fg = rgb(s.color); if (fg.length > 3 && fg[3] < .99) continue;
    const L1 = lum(fg), L2 = lum(bg), ratio = (Math.max(L1, L2) + .05) / (Math.min(L1, L2) + .05);
    const big = parseFloat(s.fontSize) >= 24 || (parseFloat(s.fontSize) >= 18.6 && +s.fontWeight >= 700);
    if (ratio < (big ? 3 : 4.5)) low.push(`"${el.textContent.trim().slice(0, 30)}" ${ratio.toFixed(2)}:1`);
  }
  if (low.length) out.warn.push(`low contrast: ${low.join(', ')}`);
  return out;
}

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const after = await serve(SITE), base = `http://127.0.0.1:${after.address().port}`;
  const browser = await chromium.launch();
  const pages = ['index.html', ...fs.readdirSync(path.join(SITE, 'docs')).filter(f => f.endsWith('.html')).map(f => 'docs/' + f)];
  const errors = [], warns = [], external = new Set(), checked = new Set();
  for (const [wname, w, h] of WIDTHS) {
    for (const scheme of ['dark', 'light']) {
      if (scheme === 'light' && wname === 'tablet') continue;
      const ctx = await browser.newContext({ viewport: { width: w, height: h }, colorScheme: scheme, reducedMotion: wname === 'tablet' ? 'reduce' : 'no-preference' });
      const page = await ctx.newPage();
      const errs = [];
      page.on('pageerror', e => errs.push('script error: ' + e.message.split('\n')[0]));
      page.on('requestfailed', r => { if (r.url().startsWith(base)) errs.push('failed to load ' + r.url().slice(base.length)); });
      page.on('response', r => { if (r.url().startsWith(base) && r.status() >= 400) errs.push(`${r.status()} for ${r.url().slice(base.length)}`); });
      // Requests to other sites (the live player counter, votes) are answered empty: the check works offline.
      await page.route(u => !u.href.startsWith(base), r => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
      for (const p of pages) {
        if (scheme === 'light' && p !== 'index.html') continue;
        errs.length = 0;
        await page.goto(`${base}/${p}`, { waitUntil: 'networkidle' });
        // Page weight (the front page, at desktop width in dark): what loads first, and everything once scrolled.
        const bytes = () => page.evaluate(() => performance.getEntriesByType('navigation').concat(performance.getEntriesByType('resource'))
          .reduce((n, e) => n + (e.encodedBodySize || 0), 0));
        const weigh = p === 'index.html' && wname === 'desktop' && scheme === 'dark';
        const first = weigh ? await bytes() : 0;
        // Scroll through once so lazy images load, then back up.
        await page.evaluate(async () => { for (let y = 0; y < document.body.scrollHeight; y += innerHeight * .8) { scrollTo(0, y); await new Promise(r => setTimeout(r, 60)); } scrollTo(0, 0); });
        await page.waitForTimeout(400);
        if (weigh) {
          const all = await bytes(), kb = n => Math.round(n / 1024);
          weight = `front page: ${kb(first)} KB at first, ${kb(all)} KB once scrolled through (budgets ${kb(FIRST_MAX)} / ${kb(ALL_MAX)} KB)`;
          if (first > FIRST_MAX) errors.push(`index.html: ${kb(first)} KB loads before scrolling, over the ${kb(FIRST_MAX)} KB budget`);
          if (all > ALL_MAX) errors.push(`index.html: ${kb(all)} KB in all, over the ${kb(ALL_MAX)} KB budget`);
        }
        const r = await page.evaluate(inspect);
        const tag = `${p} ${wname}${scheme === 'light' ? ' light' : ''}`;
        errs.concat(r.err).forEach(e => errors.push(`${tag}: ${e}`));
        r.warn.forEach(e => warns.push(`${tag}: ${e}`));
        // Links: pages, files and anchors on the site must exist; other sites are asked once below.
        for (const href of r.links) {
          if (/^(mailto:|javascript:)/.test(href)) continue;
          const u = new URL(href, `${base}/${p}`);
          if (u.origin !== base) { external.add(u.href.split('#')[0]); continue; }
          const key = u.pathname + u.hash; if (checked.has(key)) continue; checked.add(key);
          let f = path.join(SITE, decodeURIComponent(u.pathname)); if (u.pathname.endsWith('/')) f = path.join(f, 'index.html');
          if (!fs.existsSync(f)) { errors.push(`${p}: broken link ${href}`); continue; }
          if (u.hash.length > 1 && f.endsWith('.html')) {
            const id = decodeURIComponent(u.hash.slice(1));
            if (!new RegExp(`id=["']${id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}["']`).test(fs.readFileSync(f, 'utf8'))) errors.push(`${p}: link to a missing anchor ${href}`);
          }
        }
        const name = p === 'index.html' ? `after-${wname}${scheme === 'light' ? '-light' : ''}` : `docs-${path.basename(p, '.html')}-${wname}`;
        if (p === 'index.html' || wname !== 'tablet') await page.screenshot({ path: path.join(OUT, name + '.jpg'), fullPage: p === 'index.html', type: 'jpeg', quality: 78 });
        if (p === 'index.html') await page.screenshot({ path: path.join(OUT, name + '-top.png') });
      }
      await ctx.close();
    }
  }
  // The same front page on main, for comparison.
  if (BEFORE && fs.existsSync(path.join(BEFORE, 'index.html'))) {
    const before = await serve(BEFORE), bb = `http://127.0.0.1:${before.address().port}`;
    for (const [wname, w, h] of WIDTHS) {
      const ctx = await browser.newContext({ viewport: { width: w, height: h }, colorScheme: 'dark' });
      const page = await ctx.newPage();
      await page.route(u => !u.href.startsWith(bb), r => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
      await page.goto(`${bb}/index.html`, { waitUntil: 'networkidle' }).catch(() => {});
      await page.evaluate(async () => { for (let y = 0; y < document.body.scrollHeight; y += innerHeight * .8) { scrollTo(0, y); await new Promise(r => setTimeout(r, 60)); } scrollTo(0, 0); });
      await page.waitForTimeout(400);
      await page.screenshot({ path: path.join(OUT, `before-${wname}.jpg`), fullPage: true, type: 'jpeg', quality: 78 });
      await page.screenshot({ path: path.join(OUT, `before-${wname}-top.png`) });
      await ctx.close();
    }
    before.close();
  }
  await browser.close();
  after.close();
  // Other sites: one HEAD (then GET) each; these only warn, since a site being down isn't this site's fault.
  for (const u of external) {
    let ok = false, why = '';
    for (const method of ['HEAD', 'GET']) {
      try {
        const r = await fetch(u, { method, redirect: 'follow', signal: AbortSignal.timeout(15000), headers: { 'user-agent': 'hearthos.fun link check' } });
        if (r.status < 400 || r.status === 429 || r.status === 403) { ok = true; break; } why = String(r.status);
      } catch (e) { why = e.message; }
    }
    if (!ok) warns.push(`link to another site doesn't answer (${why}): ${u}`);
  }
  const uniq = a => [...new Set(a)];
  const E = uniq(errors), Wn = uniq(warns);
  const lines = [`${E.length} errors, ${Wn.length} warnings, ${pages.length} pages at ${WIDTHS.map(x => x[1]).join('/')} px, ${external.size} links to other sites${weight ? '; ' + weight : ''}`, ...E.map(e => 'ERROR ' + e), ...Wn.map(w => 'warn  ' + w)];
  fs.writeFileSync(path.join(OUT, 'report.txt'), lines.join('\n') + '\n');
  console.log(lines.join('\n'));
  if (process.env.GITHUB_ACTIONS) {
    console.log(`::notice title=Website check::${lines[0]}`);
    E.slice(0, 20).forEach(e => console.log(`::error title=Website::${e}`));
    Wn.slice(0, 20).forEach(w => console.log(`::warning title=Website::${w}`));
  }
  process.exit(E.length ? 1 : 0);
})();
