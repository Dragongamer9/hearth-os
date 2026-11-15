// A pretend Jellyfin (on 127.0.0.1:8096, where Theater looks on this device) and a check of Theater's core against
// it: finding it, Quick Connect, its rows (with where you stopped), search, opening a title in Jellyfin's own player,
// and sync's inbox from another device. Nothing here talks to a real server.
//   node tests/jellyfin.mjs <hearth-theater binary>     (HEARTH_THEATER_DIR: an empty folder for Theater's data)
import http from 'node:http';
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import path from 'node:path';

const PORT = 8096, TOKEN = 'tok-123', USER = 'u1', SERVER = 'srv1';
const H = 10_000_000 * 60; // ticks in a minute
const ep = (id, s, e, x = {}) => ({ Id: id, Type: 'Episode', Name: `Episode ${e}`, SeriesName: 'The Long Road', SeriesId: 'ser1', ParentIndexNumber: s, IndexNumber: e, RunTimeTicks: 60 * H, ...x });
const movie = (id, name, x = {}) => ({ Id: id, Type: 'Movie', Name: name, ProductionYear: 2025, ImageTags: { Primary: 't' }, RunTimeTicks: 120 * H, ...x });
const series = { Id: 'ser1', Type: 'Series', Name: 'The Long Road', RecursiveItemCount: 16, ImageTags: { Primary: 't' } };
let approved = false, asked = 0;
const seen = [];

const server = http.createServer((req, res) => {
  const u = new URL(req.url, 'http://x');
  const p = u.pathname, auth = req.headers['authorization'] || '';
  seen.push(`${req.method} ${p}`);
  const send = (v, code = 200) => { res.writeHead(code, { 'content-type': 'application/json' }); res.end(JSON.stringify(v)); };
  const needToken = () => { if (!auth.includes(`Token="${TOKEN}"`)) { send({}, 401); return true; } return false; };
  if (/^\/Items\/[a-z0-9]+\/Images\/Primary$/.test(p)) { res.writeHead(200, { 'content-type': 'image/svg+xml' }); res.end('<svg xmlns="http://www.w3.org/2000/svg" width="20" height="30"/>'); return; }
  if (p === '/System/Info/Public') return send({ ServerName: 'Den', Version: '10.10.7', Id: SERVER });
  if (p === '/QuickConnect/Initiate' && req.method === 'POST') { if (!auth.startsWith('MediaBrowser Client="Hearth Theater"')) return send({}, 400); return send({ Secret: 'sec-1', Code: '482913' }); }
  if (p === '/QuickConnect/Connect') { asked++; if (asked >= 2) approved = true; return u.searchParams.get('secret') === 'sec-1' ? send({ Authenticated: approved }) : send({}, 404); }
  if (p === '/Users/AuthenticateWithQuickConnect') {
    let body = ''; req.on('data', c => body += c); req.on('end', () => {
      try { if (JSON.parse(body).Secret !== 'sec-1' || !approved) return send({}, 401); } catch { return send({}, 400); }
      send({ AccessToken: TOKEN, ServerId: SERVER, User: { Id: USER, Name: 'Chris' } });
    }); return;
  }
  // Like Jellyfin 10.8: the newer addresses of these lists aren't there (Theater falls back to the older ones).
  if (p === '/UserItems/Resume' || p === '/Items/Latest') return send({}, 404);
  if (needToken()) return;
  if (p === `/Users/${USER}/Items/Resume`) return send({ Items: [ep('e5', 2, 5, { UserData: { PlaybackPositionTicks: 36 * H, PlayedPercentage: 60 } }), movie('m1', 'Iron Valley', { UserData: { PlaybackPositionTicks: 50 * H, PlayedPercentage: 41.6 } })] });
  if (p === '/Shows/NextUp') return send({ Items: [ep('e6', 2, 6)] });
  if (p === `/Users/${USER}/Items/Latest`) return send([movie('m2', 'Northwind'), movie('m3', 'Glass Harbor')]);
  if (p === '/Items') {
    const q = u.searchParams.get('searchTerm');
    if (q) return send({ Items: [movie('m2', 'Northwind')].filter(x => x.Name.toLowerCase().includes(q.toLowerCase())) });
    return send({ Items: u.searchParams.get('IncludeItemTypes') === 'Series' ? [series] : [movie('m1', 'Iron Valley'), movie('m2', 'Northwind'), movie('m3', 'Glass Harbor')] });
  }
  send({}, 404);
});

const fail = [];
const check = (ok, what) => { console.log(`${ok ? 'ok' : 'FAIL'}: ${what}`); if (!ok) fail.push(what); };

async function main() {
  const bin = process.argv[2], dir = process.env.HEARTH_THEATER_DIR;
  if (!bin || !dir) { console.error('usage: HEARTH_THEATER_DIR=<dir> node tests/jellyfin.mjs <hearth-theater>'); process.exit(2); }
  await new Promise(r => server.listen(PORT, '127.0.0.1', r));
  mkdirSync(dir, { recursive: true });
  // Another device's site and favorite title, waiting for Theater when it starts (what Hearth Link leaves).
  writeFileSync(path.join(dir, 'sites.json'), '[]');
  writeFileSync(path.join(dir, 'link-inbox.json'), JSON.stringify({ sites: { s9: { id: 's9', name: 'From the Ally', url: 'http://127.0.0.1:9/', category: 'TV', favorite: true, hidden: [], added: 1, kind: '' } },
    titles: { abc: { site: 's9', title: 'A show', url: 'http://127.0.0.1:9/show', favorite: true, opened: 5 } } }));
  const core = spawn(bin, ['--core-stdio'], { stdio: ['pipe', 'pipe', 'inherit'], env: { ...process.env, PATH: '/usr/bin:/bin' } });
  const waiting = new Map(), events = [];
  let buf = '', n = 0;
  core.stdout.on('data', d => {
    buf += d; let i;
    while ((i = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, i); buf = buf.slice(i + 1);
      let v; try { v = JSON.parse(line); } catch { continue; }
      if (v.t === 'reply') { waiting.get(v.id)?.(v); waiting.delete(v.id); } else events.push(v);
    }
  });
  const call = (cmd, args = {}) => new Promise((ok, no) => {
    const id = ++n; waiting.set(id, v => v.ok ? ok(v.value) : no(new Error(v.error)));
    core.stdin.write(JSON.stringify({ t: 'call', id, cmd, args }) + '\n');
    setTimeout(() => no(new Error(`${cmd}: no answer`)), 30000);
  });
  try {
    await new Promise(r => setTimeout(r, 1500));
    let init = await call('init');
    check(init.sites.some(s => s.id === 's9' && s.favorite) && init.titles.some(t => t.title === 'A show' && t.favorite), 'sync: a site and a favorite from another device are put in place at start');
    check(!existsSync(path.join(dir, 'link-inbox.json')), 'sync: the inbox is emptied once applied');
    const found = await call('jellyfin_find');
    const srv = (found.servers || []).find(s => s.url === `http://127.0.0.1:${PORT}/`);
    check(srv && srv.name === 'Den' && srv.version === '10.10' && srv.where === 'on this device', `finds the Jellyfin on this device (${JSON.stringify(found.servers)})`);
    const { code } = await call('jellyfin_connect', { url: srv?.url || `http://127.0.0.1:${PORT}/` });
    check(code === '482913', 'Quick Connect gives the code to approve');
    let w = await call('jellyfin_wait', { url: `http://127.0.0.1:${PORT}`, category: 'Movies' });
    check(!w.site, 'waits until the code is approved');
    w = await call('jellyfin_wait', { url: `http://127.0.0.1:${PORT}`, category: 'Movies' });
    check(w.site?.kind === 'jellyfin' && w.user === 'Chris' && w.site.name === 'Jellyfin · Den', `connected (${JSON.stringify(w)})`);
    const login = JSON.parse(readFileSync(path.join(dir, 'jellyfin', `${w.site.id}.json`), 'utf8'));
    check(login.token === TOKEN, 'the sign-in is kept on this device');
    const cat = await call('catalog_refresh', { id: w.site.id });
    const names = cat.rows.map(r => r.name);
    check(JSON.stringify(names) === JSON.stringify(['Continue watching', 'Next up', 'Recently added', 'Movies', 'Shows']), `its rows (${names})`);
    const cont = cat.rows[0]?.items || [];
    check(cont[0]?.title === 'The Long Road · S2 E5' && cont[0]?.left === '24 min left' && Math.abs(cont[0]?.progress - 0.6) < 1e-6, `Continue watching with where you stopped (${JSON.stringify(cont[0])})`);
    check(cont[1]?.left === '1 h 10 min left', 'a movie\'s time left');
    check(cat.rows[4]?.items[0]?.episodes === 16 && cat.search === 'jellyfin', 'Shows with their episodes, and search');
    const posters = cat.rows.flatMap(r => r.items).filter(i => i.art).length;
    check(posters > 0, `posters saved (${posters})`);
    const s = await call('search_site', { id: w.site.id, query: 'north', gen: 1 });
    check(s.items?.length === 1 && s.items[0].title === 'Northwind', 'search goes to Jellyfin');
    init = await call('init');
    check(init.jellyfin?.[w.site.id] === 'Chris', 'the home screen knows it\'s connected here');
    events.length = 0;
    await call('open_title', { site: w.site.id, item: cont[0] });
    const v = events.find(e => e.t === 'do' && e.op === 'viewer');
    check(v?.url === `http://127.0.0.1:${PORT}/web/index.html#/details?id=e5&serverId=${SERVER}`, `opens in Jellyfin's own player (${v?.url})`);
    let st = await call('sync_set', { on: false });
    check(st.on === false && existsSync(path.join(dir, 'sync-off')), 'sync can be turned off on this device');
    st = await call('sync_set', { on: true });
    check(st.on === true && !existsSync(path.join(dir, 'sync-off')), 'and on again');
    await call('remove', { id: w.site.id });
    check(!existsSync(path.join(dir, 'jellyfin', `${w.site.id}.json`)), 'removing it forgets the sign-in');
  } catch (e) { fail.push(String(e)); console.log('FAIL: ' + e); }
  core.stdin.end(); core.kill(); server.close();
  if (fail.length) { console.log(`::error title=Theater and Jellyfin::${fail.join(' | ').slice(0, 3500)} || asked: ${seen.slice(-30).join(', ')}`); process.exit(1); }
  console.log('::notice title=Theater and Jellyfin::found, Quick Connect, rows, search, the player and sync all work');
  process.exit(0);
}
main();
