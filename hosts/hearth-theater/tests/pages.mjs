// Opens Hearth Theater's page in Chromium with its sample sites (no app: the page answers its own calls), drives it
// with the keyboard, and checks the viewer's controller (ui/viewer-pad.js) with a stand-in controller, failing on
// any script error. It also runs the catalog reader (src/reader.js) on four local test sites (tests/fixtures, served
// by tests/serve.mjs: JSON-LD, Open Graph cards, RSS + sitemap, plain cards), checks the rows it finds, shows them on
// Theater's home screen and picks a card. Screenshots go to shots-pages/.
//   node tests/pages.mjs            (from hosts/hearth-theater; needs the playwright package)
import { chromium } from 'playwright';
import { mkdirSync, readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { serveFixtures, FIXTURES } from './serve.mjs';

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
  if (await p.locator('.grid-tiles .tile[data-id]').count() !== 2) errors.push('add: the new tile is missing from the Anime shelf');
  await p.click('[data-tab="All"]'); await sleep(200);
  await p.click('.tile[data-id="s2"]', { button: 'right' }); await sleep(300);
  await shot(p, 'options');
  await ctx.close();
}

// ---- the catalog reader on the four local test sites
const fx = await serveFixtures(0);
const readerJs = readFileSync(path.join(here, '..', 'src', 'reader.js'), 'utf8');
const notice = (t, m) => console.log(`::notice title=${t}::${String(m).replace(/\n/g, ' ')}`);
async function read(url, mode = 'catalog') {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  await ctx.addInitScript(m => { window.__hearthReaderMode = m; window.__hearthReaderSink = r => { window.__readerResult = r; }; }, mode);
  await ctx.addInitScript(readerJs);
  const p = await ctx.newPage();
  p.on('pageerror', e => errors.push(`reader ${url}: ${e.message}`));
  await p.goto(url);
  await p.waitForFunction(() => window.__readerResult, null, { timeout: 45000 }).catch(() => errors.push(`reader: ${url} never finished`));
  const r = await p.evaluate(() => window.__readerResult);
  await ctx.close();
  return r || { rows: [], items: [], sources: [] };
}
const EXPECT = {
  jsonld: [['Popular movies', 4], ['New episodes', 3], ['', 1]],
  og: [['Popular', 4]],
  rss: [['New videos', 5], ['', 2]],
  cards: [['Trending now', 5], ['Movies', 3], ['Popular', 4]],
};
const R = {};
for (const name of FIXTURES) {
  const r = R[name] = await read(fx.urls[name]);
  const got = (r.rows || []).map(x => [x.name, x.items.length]);
  for (const [n, c] of EXPECT[name]) {
    const row = r.rows?.find(x => x.name === n);
    if (!row || row.items.length !== c) errors.push(`reader ${name}: expected the row "${n || '(From the site)'}" with ${c} titles; got ${JSON.stringify(got)}`);
  }
  if (got.length !== EXPECT[name].length) errors.push(`reader ${name}: expected ${EXPECT[name].length} rows; got ${JSON.stringify(got)}`);
  notice(`Theater reader: ${name} fixture`, `${got.map(([n, c]) => `${n || '(From the site)'}: ${c}`).join(', ')} | sources: ${(r.sources || []).join(', ')} | search: ${r.search || 'none'}`);
}
const first = (name, row) => R[name].rows?.find(r => r.name === row)?.items?.[0] || {};
const check = (ok, what) => { if (!ok) errors.push(what); };
check(first('jsonld', 'Popular movies').title === 'Fixture Movie One' && first('jsonld', 'Popular movies').kind === 'movie' && first('jsonld', 'Popular movies').year === '2019', `reader jsonld: the first movie is wrong: ${JSON.stringify(first('jsonld', 'Popular movies'))}`);
check(/movie-2\.svg$/.test(R.jsonld.rows?.[0]?.items?.[1]?.image || ''), 'reader jsonld: an ImageObject picture was missed');
check(first('jsonld', '').episodes === 24 && first('jsonld', '').kind === 'series', `reader jsonld: the series' episode count is wrong: ${JSON.stringify(first('jsonld', ''))}`);
check(first('jsonld', 'New episodes').kind === 'episode' && /ep-a\.svg$/.test(first('jsonld', 'New episodes').image), 'reader jsonld: the episodes are wrong');
check(first('og', 'Popular').title === 'Alpha (Open Graph)' && /og-alpha\.svg$/.test(first('og', 'Popular').image) && first('og', 'Popular').kind === 'movie', `reader og: the Open Graph card wasn't read: ${JSON.stringify(first('og', 'Popular'))}`);
check(first('rss', 'New videos').title === 'Feed Video 1' && /feed-1\.svg$/.test(first('rss', 'New videos').image), `reader rss: the feed's items are wrong: ${JSON.stringify(first('rss', 'New videos'))}`);
check(first('rss', '').title === 'Sitemap Extra 1' && /extra-1\.svg$/.test(first('rss', '').image), `reader rss: the sitemap's titles are wrong: ${JSON.stringify(first('rss', ''))}`);
check(first('cards', 'Trending now').title === 'Show Number 1' && first('cards', 'Trending now').episodes === 12 && /show-1\.svg$/.test(first('cards', 'Trending now').image), `reader cards: the cards are wrong: ${JSON.stringify(first('cards', 'Trending now'))}`);
check(/film-1\.svg$/.test(first('cards', 'Movies').image), `reader cards: a lazy picture (data-src) was missed: ${JSON.stringify(first('cards', 'Movies'))}`);
check(first('cards', 'Popular').title === 'Popular Pick 1', `reader cards: the browse page (Popular) wasn't read: ${JSON.stringify(first('cards', 'Popular'))}`);
check(R.cards.search === fx.urls.cards + 'search?q={searchTerms}', `reader cards: the search form wasn't found (${R.cards.search})`);
check(!R.og.rows?.some(r => r.items.some(i => /about|privacy|terms|login/.test(i.url))), 'reader: a footer or sign-in link was listed as a title');
const SR = await read(fx.urls.cards + 'search?q=found', 'search');
check(SR.items?.length === 2 && SR.items[0].title === 'Found Show 1', `reader search: expected 2 results; got ${JSON.stringify(SR.items)}`);
notice('Theater reader: search', `${SR.items?.length} results on the cards fixture's search page`);

// ---- Theater's home screen with those catalogs
{
  const sites = [
    ['f1', 'Fixture Films', 'jsonld', 'Movies'], ['f2', 'Fixture OG', 'og', 'TV'], ['f3', 'Fixture Feed', 'rss', 'Other'], ['f4', 'Fixture Shows', 'cards', 'Anime'],
  ].map(([id, name, fxn, category], i) => ({ id, name, url: fx.urls[fxn], category, favorite: false, opened: 0, added: i + 1, art: '', icon: '', hidden: [] }));
  const cat = r => ({ updated: Date.now(), rows: r.rows, search: r.search || '', signin: false, sources: r.sources || [], error: '' });
  const catalogs = { f1: cat(R.jsonld), f2: cat(R.og), f3: cat(R.rss), f4: cat(R.cards) };
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  await ctx.addInitScript(([s, c, q]) => { window.__demoSites = s; window.__demoCatalogs = c; window.__demoSearch = q; }, [sites, catalogs, { f4: SR.items }]);
  const p = await ctx.newPage();
  p.on('pageerror', e => errors.push(`home: ${e.message}`));
  p.on('console', m => { if (m.type() === 'error' && !/favicon/.test(m.text())) errors.push(`home console: ${m.text()}`); });
  await p.goto(pathToFileURL(path.join(ui, 'index.html')).href);
  await sleep(1200);
  await shot(p, 'home-catalogs');
  const heads = await p.$$eval('section[data-site] h2', hs => hs.map(h => [...h.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent).join('').trim()));
  for (const h of ['Popular movies', 'New episodes', 'From Fixture Films', 'Popular', 'New videos', 'From Fixture Feed', 'Trending now', 'Movies']) {
    check(heads.includes(h), `home: the row "${h}" is missing (rows: ${heads.join(' / ')})`);
  }
  for (const [id, n] of [['f1', 8], ['f2', 4], ['f3', 7], ['f4', 12]]) {
    const c = await p.locator(`section[data-site="${id}"] .ct`).count();
    check(c === n, `home: ${id} should show ${n} cards, shows ${c}`);
  }
  check(await p.locator('section[data-site] .ct img.art').count() >= 30, 'home: the posters are missing');
  notice('Theater home (fixtures)', `rows: ${heads.join(' / ')}`);
  // The controller's way: focus moves from card to card, X (F on a keyboard) makes a favorite.
  await p.locator('section[data-site="f4"] .ct').first().focus();
  await p.keyboard.press('ArrowRight'); await sleep(150);
  check(await p.evaluate(() => document.activeElement?.classList.contains('ct')), 'home: Right on a card should move to the next card');
  await p.keyboard.press('f'); await sleep(300);
  check(await p.locator('.ct .fav').count() >= 1, "home: F (X) didn't make the title a favorite");
  check(await p.locator('.shelf h2', { hasText: 'Favorites' }).count() === 1, 'home: Favorites should show the favorite title');
  await shot(p, 'home-favorite');
  // Picking a card opens that title's page.
  await p.locator('section[data-site="f2"] .ct').first().click(); await sleep(200);
  check(await p.evaluate(() => window.__opened) === fx.urls.og + 'title/alpha', `home: picking a card opened ${await p.evaluate(() => window.__opened)}`);
  // A shelf (LB/RB): Movies has only the JSON-LD site's rows.
  await p.click('[data-tab="Movies"]'); await sleep(300);
  check(await p.locator('section[data-site="f1"]').count() === 3 && await p.locator('section[data-site="f2"]').count() === 0, 'home: the Movies shelf should hold only the Movies site\'s rows');
  await shot(p, 'shelf-movies');
  // A site's rows can be hidden (its options, Y).
  await p.click('[data-tab="All"]'); await sleep(200);
  await p.locator('.tile[data-id="f4"]').last().click({ button: 'right' }); await sleep(200);
  await p.click('[data-o="rows"]'); await sleep(200);
  await shot(p, 'rows-dialog');
  await p.click('[data-row="0"]'); await sleep(200);
  await p.click('[data-row="done"]'); await sleep(300);
  check(await p.locator('section[data-site="f4"]').count() === 2, 'home: hiding a row of the cards site didn\'t hide it');
  // Search across the sites: what they list at once, then each site's own search page.
  await p.click('[data-tab="Search"]'); await sleep(200);
  await p.fill('#q', 'number'); await sleep(2300);
  await shot(p, 'search');
  check(await p.locator('#sres .ct[data-card$="l"]').count() === 8, `search: expected 8 listed titles with "number"; got ${await p.locator('#sres .ct[data-card$="l"]').count()}`);
  check(await p.locator('#sres .ct[data-card$="s"]').count() === 2, "search: the cards site's own search results are missing");
  await p.locator('#sres .ct[data-card$="s"]').first().click(); await sleep(200);
  check(await p.evaluate(() => window.__opened) === fx.urls.cards + 'watch/found-1', 'search: picking a result should open it');
  await ctx.close();
}
fx.close();

// ---- the viewer's controller (ui/viewer-pad.js): with a site open, the controller drives the real pointer and the
// viewer (electron/main.js), and the home screen leaves it alone. theaterHost is stood in for, recording what's asked.
{
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  await ctx.addInitScript(() => {
    window.__sent = [];
    let fn = null;
    window.theaterHost = { viewer: (op, a) => __sent.push(['viewer', op, a || {}]), ptr: m => __sent.push(['ptr', m.op, m]), on: f => { fn = f; } };
    window.__viewerEvent = m => fn && fn(m);
    window.__pads = [];
    navigator.getGamepads = () => window.__pads;
  });
  const p = await ctx.newPage();
  p.on('pageerror', e => errors.push(`viewer pad: ${e.message}`));
  await p.goto(pathToFileURL(path.join(ui, 'index.html')).href);
  await sleep(500);
  const pad = async (buttons, axes = [0, 0, 0, 0], ms = 120) => {
    await p.evaluate(([b, a]) => {
      window.__pads = [{ id: 'test', index: 0, connected: true, axes: a, buttons: Array.from({ length: 17 }, (_, i) => ({ pressed: b.includes(i), value: b.includes(i) ? 1 : 0 })) }];
    }, [buttons, axes]);
    await sleep(ms);
  };
  const sent = () => p.evaluate(() => window.__sent.splice(0));
  // A held while the viewer opens (the A that chose the tile) doesn't click.
  await pad([0]);
  await p.evaluate(() => window.__viewerEvent({ type: 'open' }));
  await sleep(100); await pad([]); await sent();
  check(await p.evaluate(() => H.viewing === true), 'viewer pad: the home screen should leave the controller to the viewer');
  // Left stick: the pointer moves right and down.
  await pad([], [1, 0.6, 0, 0], 300); await pad([]);
  let got = await sent();
  const moves = got.filter(x => x[1] === 'move');
  check(moves.length > 3 && moves.reduce((t, x) => t + x[2].dx, 0) > 100 && moves.reduce((t, x) => t + x[2].dy, 0) > 20, `viewer pad: the left stick didn't move the pointer: ${JSON.stringify(moves.slice(0, 3))}`);
  // A: the real left button, down and then up (so holding drags).
  await pad([0]); await pad([]);
  got = await sent();
  check(JSON.stringify(got.map(x => [x[1], x[2].button, x[2].down])) === JSON.stringify([['button', 'left', true], ['button', 'left', false]]), `viewer pad: A should press and let go of the left button: ${JSON.stringify(got)}`);
  // The right stick scrolls with the wheel; D-pad down too.
  await pad([], [0, 0, 0, 1], 250); await pad([13]); await pad([]);
  got = await sent();
  check(got.filter(x => x[1] === 'wheel').reduce((t, x) => t + x[2].dy, 0) > 400, `viewer pad: the right stick / D-pad didn't scroll: ${JSON.stringify(got)}`);
  // X play/pause, Y full screen, ◀ ▶ skip, B back, LB/RB, Menu hints, View back to Theater.
  for (const b of [2, 3, 14, 15, 1, 4, 5, 9, 8]) { await pad([b]); await pad([]); }
  got = (await sent()).map(x => x[1] === 'media' ? `media:${x[2].do}${x[2].s ? ':' + x[2].s : ''}` : x[1]);
  const want = ['media:toggle', 'media:fill', 'media:seek:-10', 'media:seek:10', 'back', 'back', 'forward', 'hints', 'close'];
  check(JSON.stringify(got) === JSON.stringify(want), `viewer pad: buttons asked for ${JSON.stringify(got)}, expected ${JSON.stringify(want)}`);
  // RT: Hearth's on-screen keyboard docks under the site and types into the field clicked there.
  await pad([7]); await pad([]);
  got = await sent();
  const osk = got.find(x => x[1] === 'osk');
  check(osk && osk[2].on === true && osk[2].h > 100 && await p.evaluate(() => window.HearthOSK && HearthOSK.isOpen()), `viewer pad: RT didn't open the keyboard: ${JSON.stringify(got)}`);
  await shot(p, 'viewer-keyboard');
  await p.evaluate(() => HearthOSK.close()); await sleep(100);
  got = await sent();
  check(got.some(x => x[1] === 'osk' && x[2].on === false), `viewer pad: closing the keyboard should give the site its room back: ${JSON.stringify(got)}`);
  // Closed: the controller is the home screen's again, and nothing goes to the pointer.
  await p.evaluate(() => window.__viewerEvent({ type: 'closed' }));
  await pad([0], [1, 0, 0, 0], 200); await pad([]);
  check((await sent()).length === 0 && await p.evaluate(() => H.viewing === false), 'viewer pad: after the viewer closed, the controller still drove the pointer');
  notice('Theater viewer controller', 'pointer, A (left button down/up), scrolling, X/Y/◀▶, B/LB/RB, Menu, View and RT (keyboard) asked for the right things');
  await ctx.close();
}

await browser.close();
if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
console.log('Hearth Theater pages: all fine');
