'use strict';
// Epic and GOG games installed with Heroic Games Launcher (native or Flatpak).
// Epic: legendaryConfig/legendary/installed.json.  GOG: gog_store/installed.json.
// Titles and art come from Heroic's store caches. Games launch through Heroic's heroic:// protocol,
// so Heroic's own Wine/Proton settings for each game apply.

const path = require('node:path');
const os = require('node:os');
const fs = require('node:fs');
const { readJson, isDir, has, launchDetached, log } = require('../util');

const FLATPAK_ID = 'com.heroicgameslauncher.hgl';

function configDirs(home = os.homedir()) {
  return [
    { dir: path.join(process.env.XDG_CONFIG_HOME || path.join(home, '.config'), 'heroic'), flatpak: false },
    { dir: path.join(home, '.var', 'app', FLATPAK_ID, 'config', 'heroic'), flatpak: true },
  ].filter(c => isDir(c.dir));
}

/** Heroic's library caches list games as {app_name, title, art_*}; the file shape has changed between versions. */
function libraryIndex(file) {
  const j = readJson(file);
  const list = Array.isArray(j) ? j : (j && (j.library || j.games)) || [];
  const map = new Map();
  for (const g of list) if (g && (g.app_name || g.appName)) map.set(g.app_name || g.appName, g);
  return map;
}

function art(meta) {
  if (!meta) return {};
  const a = {};
  // Heroic's art_square is the tall cover; art_cover is the wide banner.
  if (meta.art_square) a.cover = meta.art_square;
  if (meta.art_cover) a.wide = meta.art_cover;
  if (meta.art_background || meta.art_cover) a.hero = meta.art_background || meta.art_cover;
  if (meta.art_logo) a.logo = meta.art_logo;
  for (const k of Object.keys(a)) if (a[k].startsWith('//')) a[k] = 'https:' + a[k];
  return a;
}

function parseSize(v) {
  if (typeof v === 'number') return v;
  const m = /^([\d.]+)\s*(B|KB|KiB|MB|MiB|GB|GiB|TB|TiB)$/i.exec(String(v || '').trim());
  if (!m) return null;
  const pow = { B: 0, KB: 1, KIB: 1, MB: 2, MIB: 2, GB: 3, GIB: 3, TB: 4, TIB: 4 }[m[2].toUpperCase()];
  return Math.round(parseFloat(m[1]) * 1024 ** pow);
}

function added(dir) {
  try { const s = fs.statSync(dir); return Math.round(s.birthtimeMs || s.mtimeMs); } catch { return 0; }
}

function launchUri(appName, runner) {
  return `heroic://launch?appName=${encodeURIComponent(appName)}&runner=${runner}&gui=false`;
}

function scanEpic({ home } = {}) {
  const games = [];
  for (const { dir, flatpak } of configDirs(home)) {
    const installed = readJson(path.join(dir, 'legendaryConfig', 'legendary', 'installed.json')) || {};
    const lib = libraryIndex(path.join(dir, 'store_cache', 'legendary_library.json'));
    for (const [appName, g] of Object.entries(installed)) {
      if (!g || g.is_dlc) continue;
      const meta = lib.get(appName);
      const title = g.title || (meta && meta.title) || appName;
      games.push({
        id: 'epic:' + appName, title, source: 'epic',
        installPath: g.install_path || null,
        sizeBytes: parseSize(g.install_size),
        added: added(g.install_path),
        launchUri: launchUri(appName, 'legendary'), launchVia: { heroicFlatpak: flatpak },
        installed: true, state: 'installed',
        art: art(meta),
        track: { dirs: g.install_path ? [g.install_path] : [] },
      });
    }
  }
  return dedupe(games);
}

function scanGog({ home } = {}) {
  const games = [];
  for (const { dir, flatpak } of configDirs(home)) {
    const j = readJson(path.join(dir, 'gog_store', 'installed.json'));
    const list = Array.isArray(j) ? j : (j && j.installed) || [];
    const lib = libraryIndex(path.join(dir, 'store_cache', 'gog_library.json'));
    for (const g of list) {
      const appName = g && (g.appName || g.app_name);
      if (!appName || g.is_dlc) continue;
      const meta = lib.get(appName);
      games.push({
        id: 'gog:' + appName, title: (meta && meta.title) || g.title || appName, source: 'gog',
        installPath: g.install_path || null,
        sizeBytes: parseSize(g.install_size),
        added: added(g.install_path),
        launchUri: launchUri(appName, 'gog'), launchVia: { heroicFlatpak: flatpak },
        installed: true, state: 'installed',
        art: art(meta),
        track: { dirs: g.install_path ? [g.install_path] : [] },
      });
    }
  }
  return dedupe(games);
}

function dedupe(games) {
  const seen = new Set();
  return games.filter(g => !seen.has(g.id) && seen.add(g.id));
}

function openUri(uri, { heroicFlatpak } = {}) {
  let cmd, args;
  if (heroicFlatpak && has('flatpak')) [cmd, args] = ['flatpak', ['run', FLATPAK_ID, uri]];
  else if (has('heroic')) [cmd, args] = ['heroic', [uri]];
  else [cmd, args] = ['xdg-open', [uri]];
  log(`heroic: ${cmd} ${args.join(' ')}`);
  launchDetached(cmd, args);
}

module.exports = { scanEpic, scanGog, openUri, parseSize };
