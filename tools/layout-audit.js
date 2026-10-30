// Layout audit: draws every experimental and keyboard & mouse layout (Home and Library), and the Dragonfire look
// (its Home and Settings › Updates & Recovery), Settings › Hearth Storage in both looks (a server, three devices with
// their health, a save to pick, downloads & updates), at 1280×800, 1920×1080 and 1280×720 with 0, 1, the sample library (~25) and 500 games, takes a screenshot of each, and looks for:
// script errors, controls pushed off the screen, clipped or overlapping text, low-contrast text, a lost selection,
// and controls the D-pad can't reach. Writes shots/<size>/<layout>-<w>x<h>.jpg and shots/report.txt.
//   node tools/layout-audit.js [filter]     (needs playwright, like ui-check.js)
const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

const OUT = path.resolve(__dirname, '../shots');
const VIEWS = [[1280, 800], [1920, 1080], [1280, 720]];
const SIZES = [['n25', 0], ['n500', 476], ['n1', 0], ['n0', 0]];
const only = process.argv[2] || '';

// Runs in the page: what's wrong with what's on screen now.
function inspect(opts) {
  const W = innerWidth, H = innerHeight, out = [];
  const view = document.querySelector('#view');
  const vis = el => { const r = el.getBoundingClientRect(); if (r.width < 2 || r.height < 2) return null; const s = getComputedStyle(el); if (s.visibility === 'hidden' || +s.opacity === 0) return null; return r; };
  const scroller = el => { for (let p = el.parentElement; p && p !== document.body; p = p.parentElement) { const s = getComputedStyle(p); if (/(auto|scroll)/.test(s.overflowX + s.overflowY)) return p; } return null; };
  const clipper = el => { for (let p = el.parentElement; p && p !== document.body; p = p.parentElement) { const s = getComputedStyle(p); if (s.overflow !== 'visible') return p; } return null; };
  const name = el => (el.dataset && el.dataset.k) || (el.className && String(el.className).split(' ')[0]) || el.tagName.toLowerCase();
  // 1. The selection is somewhere on the page.
  if (opts.games && !(focused && document.contains(focused))) out.push('no selection after drawing');
  // 2. Controls off the screen with no way to scroll to them.
  const offs = [];
  for (const el of view.querySelectorAll('[data-f]')) {
    const r = vis(el); if (!r) continue;
    if (r.right > W + 2 || r.left < -2 || r.bottom > H + 2 || r.top < -2) {
      const sc = scroller(el); if (sc) continue;
      offs.push(name(el));
    }
  }
  if (offs.length) out.push(`${offs.length} controls off screen, nothing scrolls to them: ${offs.slice(0, 6).join(', ')}`);
  if (document.documentElement.scrollWidth > W + 2) {
    const wide = [...document.querySelectorAll('body *')].filter(e => { const r = e.getBoundingClientRect(); return r.width && r.right > W + 2 && !(e.parentElement && e.parentElement.getBoundingClientRect().right > W + 2); }).slice(0, 4)
      .map(e => (e.id ? '#' + e.id : '') + '.' + String(e.className || e.tagName).split(' ')[0] + ' in ' + (e.parentElement.id || String(e.parentElement.className).split(' ')[0]));
    out.push(`the page is ${document.documentElement.scrollWidth - W}px wider than the window: ${wide.join(', ')}`);
  }
  // 2b. Layouts that fill the screen (.lfit): inside the screen, their content inside them, the tabs still in sight.
  const lf = view.querySelector('.lfit');
  if (lf) {
    const r = lf.getBoundingClientRect(), vr = view.getBoundingClientRect();
    if (r.bottom > vr.bottom + 4) { const ch = []; for (let e = lf; e; e = e.offsetParent) ch.push((e.id || String(e.className).split(' ')[0]) + ':' + e.offsetTop); out.push(`fills ${Math.round(r.bottom - vr.bottom)}px past the bottom of the screen (height ${lf.style.height}, view ${view.clientHeight}/${view.offsetTop}, ${ch.join(' > ')})`); }
    if (lf.scrollHeight > lf.clientHeight + 4 && getComputedStyle(lf).overflowY === 'hidden') out.push(`its content is ${lf.scrollHeight - lf.clientHeight}px taller than the space it has`);
    const head = view.querySelector('.libhead'); if (head && head.getBoundingClientRect().bottom < vr.top + 4) out.push('the tabs scrolled out of sight');
  }
  // 2c. Controls hidden behind the footer (an info bar or strip drawn over it, or under it).
  const foot = document.querySelector('#footer'), fr = foot && foot.getBoundingClientRect();
  if (fr && fr.height) {
    const top = document.elementFromPoint(Math.min(W - 2, fr.right - 40), fr.top + fr.height / 2);
    if (top && !foot.contains(top)) out.push('something is drawn over the footer: ' + name(top));
  }
  // 3. Text: clipped without an ellipsis, overlapping other text, low contrast.
  const texts = [];
  const walker = document.createTreeWalker(view, NodeFilter.SHOW_TEXT, { acceptNode: n => n.textContent.trim().length > 1 ? 1 : 2 });
  for (let n; (n = walker.nextNode()) && texts.length < 600;) {
    const el = n.parentElement; if (!el || el.closest('svg')) continue;
    const range = document.createRange(); range.selectNodeContents(n);
    const r = range.getBoundingClientRect(); if (r.width < 2 || r.height < 2 || r.bottom < 0 || r.top > H || r.right < 0 || r.left > W) continue;
    const s = getComputedStyle(el); if (s.visibility === 'hidden' || +s.opacity < 0.05) continue;
    // What's left of it inside every box that clips it (its own, when it's cut off with an ellipsis, and every
    // scrolling list it's in): only that part can overlap anything.
    let v = {left: r.left, right: r.right, top: r.top, bottom: r.bottom};
    for (let p = el; p && p !== document.body; p = p.parentElement) {
      const ps = p === el ? s : getComputedStyle(p); if (ps.overflow === 'visible' && ps.overflowX === 'visible' && ps.overflowY === 'visible') continue;
      const cr = p.getBoundingClientRect(); v = {left: Math.max(v.left, cr.left), right: Math.min(v.right, cr.right), top: Math.max(v.top, cr.top), bottom: Math.min(v.bottom, cr.bottom)};
    }
    if (v.right - v.left < 2 || v.bottom - v.top < 2) continue;
    texts.push({ el, r: {left: v.left, right: v.right, top: v.top, bottom: v.bottom, width: v.right - v.left, height: v.bottom - v.top}, t: n.textContent.trim().slice(0, 28), s });
  }
  const clipped = [];
  for (const { el, t, s } of texts) {
    if (s.textOverflow === 'ellipsis' || s.webkitLineClamp !== 'none' && s.webkitLineClamp) continue;
    if ((s.overflow === 'hidden' || s.overflowX === 'hidden') && el.scrollWidth > el.clientWidth + 3 && el.clientWidth > 0) clipped.push(t);
  }
  if (clipped.length) out.push(`${clipped.length} texts cut off without …: ${clipped.slice(0, 5).map(x => JSON.stringify(x)).join(', ')}`);
  const over = [];
  for (let i = 0; i < texts.length && over.length < 8; i++) for (let j = i + 1; j < texts.length; j++) {
    const a = texts[i], b = texts[j]; if (a.el === b.el || a.el.contains(b.el) || b.el.contains(a.el)) continue;
    const x = Math.min(a.r.right, b.r.right) - Math.max(a.r.left, b.r.left), y = Math.min(a.r.bottom, b.r.bottom) - Math.max(a.r.top, b.r.top);
    if (x <= 2 || y <= 2) continue;
    const small = Math.min(a.r.width * a.r.height, b.r.width * b.r.height);
    if (x * y > small * 0.3) { over.push(`${JSON.stringify(a.t)} / ${JSON.stringify(b.t)}`); break; }
  }
  if (over.length) out.push(`${over.length}+ texts overlap: ${over.slice(0, 4).join('; ')}`);
  const rgb = c => { const m = /rgba?\(([^)]+)\)/.exec(c); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const lum = c => { const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; }; return .2126 * f(c.r) + .7152 * f(c.g) + .0722 * f(c.b); };
  const low = [];
  for (const { el, t, s } of texts) {
    const fg = rgb(s.color); if (!fg || fg.a < .3) continue;
    let bg = null, img = false;
    for (let p = el; p; p = p.parentElement) { const ps = getComputedStyle(p); if (ps.backgroundImage !== 'none' || p.tagName === 'IMG' || p.querySelector(':scope > img, :scope > .art')) { img = true; break; } const b = rgb(ps.backgroundColor); if (b && b.a > .85) { bg = b; break; } }
    if (img || !bg) continue;
    const L1 = lum(fg), L2 = lum(bg), ratio = (Math.max(L1, L2) + .05) / (Math.min(L1, L2) + .05);
    if (ratio < 3) low.push(`${JSON.stringify(t)} ${ratio.toFixed(1)}:1`);
  }
  if (low.length) out.push(`${low.length} texts with low contrast: ${low.slice(0, 4).join(', ')}`);
  return out;
}

// Runs in the page: Settings › Hearth Storage with a server, three devices (one behind on updates, one whose start
// isn't confirmed), a save to pick, downloads & updates, and Advanced open, in the look asked for.
function storageMock(look) {
  const me = 'a'.repeat(64), srv = 'b'.repeat(64), ally = 'c'.repeat(64), now = Date.now();
  const st = { running: true, me: { id: me, name: 'Deck' }, settings: {}, peers: [{ id: srv, name: 'Basement PC', online: true }, { id: ally, name: 'ROG Ally', online: false, lastSeen: now - 864e5 }],
    transfers: [], offers: [], sending: [], signInHints: [],
    storage: { config: { server: false, folder: '', free: 2e11, shares: [], archive: true, backups: true, media: true, away: false, smb: false, sleepMinutes: 0, backupTo: srv,
                         backupSaves: true, backupShots: true, savesFollow: true, backupHours: 6, lastBackup: now - 36e5,
                         archived: { 'steam:620': { server: srv, title: 'Portal 2', size: 1.2e10, state: 'archived', key: 'aaaaaaaa-steam-620' } } },
               servers: [{ id: srv, name: 'Basement PC', shares: [{ id: 'c0ffee000001', name: 'Films' }, { id: 'c0ffee000002', name: 'Music' }], archive: true, backups: true, media: true, free: 2e12, usable: true, at: now, canWake: true }],
               jobs: [], user: 'me' } };
  window.call = async (m) => {
    if (m === 'link.status') return st;
    if (m === 'link.storage.servers') return st.storage;
    if (m === 'link.storage.updates.status') throw new Error('link.storage.updates.status isn\'t a Hearth Link call');
    return {};
  };
  LK.s = st; closeAll(); S.history = [];
  STG.devices = { server: srv, serverName: 'Basement PC', devices: [
    { id: me, name: 'Deck', me: true, online: true, kind: 'handheld', battery: { percent: 64, charging: false }, free: 2e11, total: 5e11, lastBackup: now - 36e5, version: '0.6.56', upToDate: true, newest: '0.6.56', boot: { good: true } },
    { id: srv, name: 'Basement PC', server: true, online: true, kind: 'desktop', battery: null, free: 2e12, total: 4e12, version: '0.6.56', upToDate: true, newest: '0.6.56', boot: { good: true } },
    { id: ally, name: 'ROG Ally', online: false, lastSeen: now - 864e5, kind: 'handheld', battery: { percent: 12, charging: false }, free: 3e10, total: 5e11, lastBackup: now - 3 * 864e5, version: '0.6.52', upToDate: false, newest: '0.6.56', boot: { good: false } }] };
  STG.saves = { follow: true, server: srv, games: [{ id: 'gog:1207658924', title: 'Heroes of Might and Magic 3', conflict: { server: { fromName: 'ROG Ally', at: now - 72e5 }, local: { at: now - 864e5 } } }] };
  STG.cache = { enabled: true }; STG.updates = { na: true }; STG.adv = true;
  LOOK.style = look; LOOK.layout = 'shelf'; LOOK.libLayout = ''; saveLook();
  S.settingsTab = 'storage'; navigate('settings', {}, false); render(null, true);
  window.__auditView = 'settings';
}

// Runs in the page: presses up/down/left/right from every control it can reach (breadth first); the ones it
// never gets to can't be reached with a controller.
function explore() {
  const root = window.root();
  const all = () => [...root.querySelectorAll('[data-f]')].filter(el => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0 && !el.disabled; });
  const key = el => el.dataset.k || el.textContent.trim().slice(0, 30);
  const start = focused && root.contains(focused) ? focused : all()[0];
  if (!start) return { total: 0, missed: [] };
  const seen = new Set([key(start)]), queue = [key(start)];
  let steps = 0;
  while (queue.length && steps < 2500) {
    const k = queue.shift();
    for (const dir of ['up', 'down', 'left', 'right']) {
      const el = all().find(e => key(e) === k); if (!el) break;
      setFocus(el, true); steps++;
      try { moveNow(dir); } catch (e) { return { error: e.message }; }
      if (S.view !== opts_view()) return { error: 'moving changed the screen to ' + S.view };
      if (focused && root.contains(focused)) { const nk = key(focused); if (!seen.has(nk)) { seen.add(nk); queue.push(nk); } }
    }
  }
  const missed = [...new Set(all().map(key).filter(k => !seen.has(k)))];
  return { total: all().length, missed };
  function opts_view() { return window.__auditView; }
}

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch();
  const report = [], counts = {};
  let errorsTotal = 0;
  for (const [size, stress] of SIZES) {
    const page = await browser.newPage({ viewport: { width: VIEWS[0][0], height: VIEWS[0][1] } });
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', m => { if (m.type() === 'error' && !/Failed to load resource|net::ERR/.test(m.text())) errors.push('console: ' + m.text()); });
    await page.goto('file://' + path.resolve(__dirname, '../ui/index.html') + (stress ? `?stress=${stress}` : ''));
    await page.waitForTimeout(1500);
    await page.evaluate(n => {
      HOSTINFO.features = ['quickResume', 'friends', 'screenshots', 'profiles'];
      if (n === 'n0') GAMES.length = 0;
      if (n === 'n1') GAMES.splice(1);
      if (n === 'n0' || n === 'n1') { QR.list.length = 0; NOTIFS = []; }
      try { localStorage.clear(); } catch (e) {}
    }, size);
    const sets = await page.evaluate(() => ({
      home: [...(typeof XLAYOUTS !== 'undefined' ? XLAYOUTS : []), ...(typeof MK_LAYOUTS !== 'undefined' ? MK_LAYOUTS : [])].map(l => l.id),
      lib: [...(typeof XLIB_LAYOUTS !== 'undefined' ? XLIB_LAYOUTS : []), ...(typeof MKLIB_LAYOUTS !== 'undefined' ? MKLIB_LAYOUTS : [])].map(l => l.id),
    }));
    fs.mkdirSync(path.join(OUT, size), { recursive: true });
    // Looks ("look-<id>"): drawn with that look on, the layouts above with the default one.
    const jobs = [...sets.home.map(id => ['home', id]), ...sets.lib.map(id => ['library', id]), ['home', 'look-dragonfire'], ['settings', 'look-dragonfire-updates']]
      .filter(([, id]) => !only || id.includes(only));
    for (const [view, id] of jobs) {
      for (const [w, h] of VIEWS) {
        await page.setViewportSize({ width: w, height: h });
        const before = errors.length;
        try {
          await page.evaluate(([view, id]) => {
            window.__auditView = view;
            closeAll(); S.history = []; S.tab = 'all'; S.lfilter = ''; S.sort = 'recent';
            LOOK.style = 'classic';
            if (id.startsWith('look-')) { LOOK.style = id.split('-')[1]; LOOK.layout = 'shelf'; LOOK.libLayout = ''; }
            else if (view === 'home') { LOOK.layout = id; } else { LOOK.layout = 'shelf'; LOOK.libLayout = id; }
            saveLook(); navigate(view, view === 'settings' ? { settingsTab: 'updates' } : {}, false);
          }, [view, id]);
          await page.waitForTimeout(350);
        } catch (e) { errors.push(`draw: ${e.message.split('\n')[0]}`); }
        const tag = `${size} ${view} ${id} ${w}x${h}`;
        const probs = await page.evaluate(inspect, { games: size !== 'n0' }).catch(e => ['inspect failed: ' + e.message.split('\n')[0]]);
        // Mouse: hovering the first game card and right-clicking it must not throw.
        try {
          const box = await page.evaluate(() => { const e = document.querySelector('#view [data-f][data-id]'); if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.left + Math.min(20, r.width / 2), y: r.top + Math.min(20, r.height / 2) }; });
          if (box && box.x > 0 && box.y > 0 && box.x < w && box.y < h) { await page.mouse.move(box.x - 30, box.y); await page.mouse.move(box.x, box.y, { steps: 3 }); }
        } catch (e) { probs.push('hover failed: ' + e.message.split('\n')[0]); }
        await page.screenshot({ path: path.join(OUT, size, `${view}-${id}-${w}x${h}.jpg`), type: 'jpeg', quality: 55 });
        if (w === 1280 && h === 800 && (size === 'n25' || size === 'n1')) {
          const r = await page.evaluate(explore).catch(e => ({ error: e.message.split('\n')[0] }));
          if (r.error) probs.push('controller: ' + r.error);
          else if (r.missed.length) probs.push(`controller can't reach ${r.missed.length} of ${r.total}: ${r.missed.slice(0, 8).join(', ')}`);
        }
        await page.mouse.move(1, h - 1);
        for (const e of errors.slice(before)) probs.unshift('SCRIPT ERROR ' + e);
        errorsTotal += errors.length - before;
        if (probs.length) { report.push(`${tag}\n  - ${probs.join('\n  - ')}`); counts[id] = (counts[id] || 0) + probs.length; }
      }
    }
    await page.close();
    // Settings › Hearth Storage (Hearth OS only, so drawn as Hearth OS): Classic and Dragonfire, Advanced open.
    const sjobs = ['look-classic-storage', 'look-dragonfire-storage'].filter(id => (!only || id.includes(only)) && size === 'n25');
    if (sjobs.length) {
      const sp = await browser.newPage({ viewport: { width: VIEWS[0][0], height: VIEWS[0][1] } });
      const serrs = [];
      sp.on('pageerror', e => serrs.push(e.message));
      await sp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux');
      await sp.waitForTimeout(1500);
      for (const id of sjobs) {
        for (const [w, h] of VIEWS) {
          await sp.setViewportSize({ width: w, height: h });
          const before = serrs.length;
          try { await sp.evaluate(storageMock, id.split('-')[1]); await sp.waitForTimeout(350); } catch (e) { serrs.push(`draw: ${e.message.split('\n')[0]}`); }
          const tag = `${size} settings ${id} ${w}x${h}`;
          const probs = await sp.evaluate(inspect, { games: true }).catch(e => ['inspect failed: ' + e.message.split('\n')[0]]);
          await sp.screenshot({ path: path.join(OUT, size, `settings-${id}-${w}x${h}.jpg`), type: 'jpeg', quality: 70 });
          if (w === 1280 && h === 800) {
            const r = await sp.evaluate(explore).catch(e => ({ error: e.message.split('\n')[0] }));
            if (r.error) probs.push('controller: ' + r.error);
            else if (r.missed.length) probs.push(`controller can't reach ${r.missed.length} of ${r.total}: ${r.missed.slice(0, 8).join(', ')}`);
          }
          for (const e of serrs.slice(before)) probs.unshift('SCRIPT ERROR ' + e);
          errorsTotal += serrs.length - before;
          if (probs.length) { report.push(`${tag}\n  - ${probs.join('\n  - ')}`); counts[id] = (counts[id] || 0) + probs.length; }
        }
      }
      await sp.close();
    }
  }
  await browser.close();
  const sum = Object.entries(counts).sort((a, b) => b[1] - a[1]).map(([k, v]) => `${k}: ${v}`).join(', ');
  fs.writeFileSync(path.join(OUT, 'report.txt'), `Layout audit: ${report.length} renders with problems, ${errorsTotal} script errors\n${sum}\n\n${report.join('\n')}\n`);
  console.log(`${report.length} renders with problems, ${errorsTotal} script errors`);
  console.log(sum);
  process.exit(errorsTotal ? 1 : 0);
})();
