'use strict';
// Extra facts on a game's page (Linux):
//   protonDb(game)       how well a Windows game runs on Linux, from ProtonDB's community reports
//                        (Platinum, Gold, Silver, Bronze, Borked, or Native). Steam games by their app id;
//                        Epic, GOG and your own games by finding the same game on Steam first.
//   achievements(appid)  your Steam achievements, read from Steam's own files on this device.
// Answers are kept for a few days (cache/game-info.json) so pages open instantly and offline.

const fs = require('node:fs');
const path = require('node:path');
const { DATA_DIR, readJson, writeJsonAtomic, log } = require('./util');

const CACHE = path.join(DATA_DIR, 'cache', 'game-info.json');
const DAY = 864e5;
let cache = null;
const load = () => (cache ||= readJson(CACHE) || {});
const keep = (k, v) => { load()[k] = { ...v, at: Date.now() }; try { fs.mkdirSync(path.dirname(CACHE), { recursive: true }); writeJsonAtomic(CACHE, cache); } catch {} return v; };
const fresh = (k, days) => { const e = load()[k]; return e && Date.now() - e.at < days * DAY ? e : null; };

const TIERS = {
  native: { label: 'Native', text: 'Made for Linux' },
  platinum: { label: 'Platinum', text: 'Runs perfectly' },
  gold: { label: 'Gold', text: 'Runs well after a tweak or two' },
  silver: { label: 'Silver', text: 'Runs, with some problems' },
  bronze: { label: 'Bronze', text: 'Runs, but badly' },
  borked: { label: 'Borked', text: "Doesn't run" },
  pending: { label: 'No rating yet', text: 'Not enough reports' },
};

const norm = s => String(s || '').toLowerCase().replace(/[®™©:'’.,!?\-–—_()\[\]]/g, ' ').replace(/\b(the|edition|goty|game of the year|definitive|enhanced|remastered|deluxe|complete)\b/g, ' ').replace(/\s+/g, ' ').trim();

/** The Steam app id of a game sold elsewhere, by its name (Steam's store search). */
async function steamAppIdFor(title, fetchFn = fetch) {
  const k = 'appid:' + norm(title);
  const c = fresh(k, 30);
  if (c) return c.appid;
  const r = await fetchFn(`https://store.steampowered.com/api/storesearch/?term=${encodeURIComponent(title)}&l=english&cc=US`);
  if (!r.ok) throw new Error('Steam search ' + r.status);
  const items = (await r.json()).items || [];
  const want = norm(title);
  const hit = items.find(i => norm(i.name) === want) || items.find(i => norm(i.name).startsWith(want) || want.startsWith(norm(i.name)));
  return keep(k, { appid: hit ? hit.id : 0 }).appid;
}

async function protonDb(g, fetchFn = fetch) {
  if (!g) return null;
  let appid = /^steam:(\d+)$/.test(g.id || '') ? +g.id.slice(6) : 0;
  try {
    if (!appid && g.title && g.source !== 'xbox' && !g.cloud) appid = await steamAppIdFor(g.title, fetchFn);
    if (!appid) return null;
    const k = 'pdb:' + appid;
    const c = fresh(k, 3);
    if (c) return c.tier ? { appid, ...c } : null;
    const r = await fetchFn(`https://www.protondb.com/api/v1/reports/summaries/${appid}.json`);
    if (r.status === 404) { keep(k, { tier: null }); return null; }
    if (!r.ok) throw new Error('ProtonDB ' + r.status);
    const j = await r.json();
    const tier = String(j.tier || j.trendingTier || 'pending').toLowerCase();
    const out = { tier, ...(TIERS[tier] || TIERS.pending), reports: j.total || 0, url: `https://www.protondb.com/app/${appid}` };
    keep(k, out);
    return { appid, ...out };
  } catch (e) {
    log('protondb: ' + e.message);
    const old = load()['pdb:' + appid];   // offline: the last answer, however old
    return old && old.tier ? { appid, ...old } : null;
  }
}

// ------------------------------------------------------------------ Steam achievements

/** Steam keeps each game's achievement list (UserGameStatsSchema_<app>.bin) and your progress
 *  (UserGameStats_<account>_<app>.bin) in appcache/stats once you've played it on this device. */
function readAchievements(statsDir, appid, accountId) {
  const { readKv, Reader } = require('./library/steamcache');
  const kv = f => { try { return readKv(new Reader(fs.readFileSync(f)), null); } catch { return null; } };
  const schema = kv(path.join(statsDir, `UserGameStatsSchema_${appid}.bin`));
  if (!schema) return null;
  const root = schema[String(appid)] || Object.values(schema)[0] || {};
  const user = kv(path.join(statsDir, `UserGameStats_${accountId}_${appid}.bin`)) || {};
  const ucache = user.cache || user;
  const list = [];
  for (const [statId, st] of Object.entries(root.stats || {})) {
    if (!st || typeof st !== 'object' || !st.bits) continue;
    const mine = ucache[statId] || {};
    const data = typeof mine.data === 'number' ? mine.data >>> 0 : 0;
    const times = mine.achievementtimes || {};
    for (const [bit, a] of Object.entries(st.bits)) {
      if (!a || typeof a !== 'object') continue;
      const d = a.display || {};
      const text = v => (v && typeof v === 'object' ? (v.english || Object.values(v)[0] || '') : v || '');
      const done = ((data >>> (+bit)) & 1) === 1;
      list.push({
        id: a.name, name: text(d.name) || a.name, desc: text(d.desc), hidden: d.hidden === '1' || d.hidden === 1,
        done, at: done && times[bit] ? times[bit] * 1000 : null,
        icon: d.icon ? `https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/${appid}/${done ? d.icon : (d.icon_gray || d.icon)}` : null,
      });
    }
  }
  if (!list.length) return null;
  return { total: list.length, done: list.filter(a => a.done).length, list: list.sort((a, b) => (b.done - a.done) || ((b.at || 0) - (a.at || 0))) };
}

/** Your achievements for a Steam game (the account signed in to Steam on this device). */
function achievements(appid) {
  const steam = require('./library/steam');
  const dir = steam.steamPath();
  if (!dir || !appid) return null;
  let id64 = null;
  try {
    const users = fs.readFileSync(path.join(dir, 'config', 'loginusers.vdf'), 'utf8');
    const m = [...users.matchAll(/"(\d{17})"\s*\{([^{}]*)\}/g)];
    const hit = m.find(x => /"MostRecent"\s+"1"/.test(x[2])) || m[0];
    id64 = hit ? BigInt(hit[1]) : null;
  } catch {}
  if (!id64) return null;
  const account = Number(id64 - 76561197960265728n);
  return readAchievements(path.join(dir, 'appcache', 'stats'), appid, account);
}

module.exports = { protonDb, steamAppIdFor, norm, TIERS, readAchievements, achievements };
