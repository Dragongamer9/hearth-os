// Opens Hearth Theater's page in Chromium with its sample sites (no app: the page answers its own calls), drives it
// with the keyboard, and runs the viewer's script (src/viewer.js) on a test page with a video and a frame, failing on
// any script error. It also runs the catalog reader (src/reader.js) on four local test sites (tests/fixtures, served
// by tests/serve.mjs: JSON-LD, Open Graph cards, RSS + sitemap, plain cards), checks the rows it finds, shows them on
// Theater's home screen and picks a card. Screenshots go to shots-pages/.
//   node tests/pages.mjs            (from hosts/hearth-theater; needs the playwright package)
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
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
