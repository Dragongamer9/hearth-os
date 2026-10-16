'use strict';
// Xbox Cloud Gaming (Game Pass streaming). Linux can't install Xbox app games, but it can stream them.
//
// The catalog comes from Microsoft's public, anonymous Game Pass catalog endpoints (no sign-in):
//   GET  catalog.gamepass.com/sigls/v2?id=<All games SIGL>   → product ids behind play.xbox.com/gallery/all-games
//   POST catalog.gamepass.com/v3/products?hydration=...      → title and art for those ids
// It's cached on disk for a day so the library loads instantly and works offline.
// Games stream in a Nimbus window (see stream.js); the account and Game Pass Ultimate are the user's own.

const path = require('node:path');
const { DATA_DIR, readJson, writeJsonAtomic, log } = require('../util');

const BASE = 'https://catalog.gamepass.com';
const ALL_GAMES_SIGL = '1bf84c2b-0643-4591-893f-d9edb703f692';
const HYDRATION = 'RemoteHighSapphire0';
const CACHE = path.join(DATA_DIR, 'xcloud-catalog.json');
const MAX_AGE = 24 * 60 * 60 * 1000;
const GALLERY = 'https://play.xbox.com/gallery/all-games';

/** Market and language from the system locale (en_GB.UTF-8 → GB, en-gb); US/en-us otherwise. */
function locale(env = process.env) {
  const m = /^([a-z]{2})_([A-Z]{2})/.exec(env.LC_ALL || env.LANG || '');
  return m ? { market: m[2], language: `${m[1]}-${m[2].toLowerCase()}` } : { market: 'US', language: 'en-us' };
}

function slug(title) {
  const s = String(title || '').toLowerCase().normalize('NFD').replace(/[^a-z0-9\s-]/g, '').replace(/[\s-]+/g, '-').replace(/^-|-$/g, '');
  return s || '_';
}

const playUrl = (id, title) => `https://play.xbox.com/products/${id}/${slug(title)}`;

function img(p, ...keys) {
  for (const k of keys) {
    const u = p && p[k] && p[k].URL;
    if (u) return u.startsWith('//') ? 'https:' + u : u;
  }
  return null;
}

/** Turns one /v3/products document into a catalog entry, or null if it isn't streamable. */
function fromProduct(id, p) {
  if (!p || !p.ProductTitle) return null;
  if (p.XCloudOfferings && !p.XCloudOfferings.CLOUDGAMING) return null;
  return {
    id, title: p.ProductTitle,
    cover: img(p, 'Image_Poster', 'Image_BoxArt', 'Image_Tile'),
    wide: img(p, 'Image_Tile', 'Image_Hero', 'Image_SuperHero'),
    hero: img(p, 'Image_Hero', 'Image_SuperHero', 'Image_Tile'),
    logo: img(p, 'Image_Logo'),
  };
}

async function fetchJson(url, init, fetchImpl) {
  const r = await fetchImpl(url, { ...init, signal: AbortSignal.timeout(20000) });
  if (!r.ok) throw new Error(`${new URL(url).pathname} → HTTP ${r.status}`);
  return r.json();
}

async function fetchCatalog({ fetchImpl = fetch, env } = {}) {
  const { market, language } = locale(env);
  const q = new URLSearchParams({ id: ALL_GAMES_SIGL, market, language });
  const sigl = await fetchJson(`${BASE}/sigls/v2?${q}`, {}, fetchImpl);
  const ids = [...new Set((Array.isArray(sigl) ? sigl : []).map(x => x && x.id).filter(x => typeof x === 'string'))];

  const out = [];
  const url = `${BASE}/v3/products?${new URLSearchParams({ market, language, hydration: HYDRATION })}`;
  for (let i = 0; i < ids.length; i += 50) {
    const body = await fetchJson(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'ms-cv': 'cv', 'calling-app-name': 'Nimbus', 'calling-app-version': '1' },
      body: JSON.stringify({ Products: ids.slice(i, i + 50) }),
    }, fetchImpl);
    for (const [id, p] of Object.entries((body && body.Products) || {})) {
      const e = fromProduct(id, p);
      if (e) out.push(e);
    }
  }
  return { market, fetched: Date.now(), games: out };
}

/** Cached catalog, refreshed when older than a day. A failed refresh keeps the old copy. */
async function catalog({ force = false, fetchImpl, env } = {}) {
  const cached = readJson(CACHE);
  const { market } = locale(env);
  const fresh = cached && cached.market === market && Date.now() - cached.fetched < MAX_AGE;
  if (fresh && !force) return cached.games;
  try {
    const c = await fetchCatalog({ fetchImpl, env });
    if (c.games.length) writeJsonAtomic(CACHE, c);
    log(`xcloud: catalog ${c.games.length} games (${market})`);
    return c.games;
  } catch (e) {
    log('xcloud: catalog refresh failed: ' + e.message);
    return cached ? cached.games : [];
  }
}

/** Library entries: one "Xbox Cloud Gaming" tile (opens the catalog) plus every cloud game. */
async function scan(opts) {
  const games = (await catalog(opts)).map(e => ({
    id: 'xbox:' + e.id,
    title: e.title,
    source: 'xbox',
    cloud: true, installed: false, state: 'cloud',
    added: 0,
    art: Object.fromEntries(Object.entries({ cover: e.cover, wide: e.wide, hero: e.hero, logo: e.logo }).filter(([, v]) => v)),
    streamUrl: playUrl(e.id, e.title),
  }));
  games.unshift({
    id: 'xbox:cloud', title: 'Xbox Cloud Gaming', source: 'xbox',
    cloud: true, installed: true, state: 'installed', added: 0, art: {},
    streamUrl: GALLERY,
  });
  return games;
}

module.exports = { scan, catalog, fetchCatalog, fromProduct, locale, slug, playUrl, GALLERY };
