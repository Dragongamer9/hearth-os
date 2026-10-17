'use strict';
// Installed Steam games on Linux: libraryfolders.vdf → appmanifest_*.acf in every library.
// Play time and last played come from the most recent user's localconfig.vdf.
// Art: the user's custom grid images first, then Steam's local library cache, then Steam's CDN.

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const vdf = require('./vdf');
const { exists, isDir, has, launchDetached, log } = require('../util');

const STEAM_ID64_BASE = 76561197960265728n;
// Runtimes and tools that show up as "apps" but aren't games.
const SKIP_APP_IDS = new Set(['228980', '250820', '1070560', '1391110', '1628350', '1493710', '1826330', '2180100', '2348590']);
const SKIP_PREFIXES = ['Steamworks', 'Proton', 'Steam Linux Runtime', 'SteamVR'];

// appmanifest StateFlags (EAppState)
const FULLY_INSTALLED = 4, UPDATE_RUNNING = 256, UPDATE_PAUSED = 512, UPDATE_STARTED = 1024,
  PREALLOCATING = 524288, DOWNLOADING = 1048576, STAGING = 2097152, COMMITTING = 4194304;
const BUSY = UPDATE_RUNNING | UPDATE_STARTED | PREALLOCATING | DOWNLOADING | STAGING | COMMITTING;

const CDN = 'https://cdn.cloudflare.steamstatic.com/steam/apps/';
const FLATPAK_ID = 'com.valvesoftware.Steam';

/** Every place Steam keeps its data, native or Flatpak. First existing one wins. */
function candidates(home = os.homedir()) {
  return [
    path.join(home, '.local', 'share', 'Steam'),
    path.join(home, '.steam', 'steam'),
    path.join(home, '.steam', 'root'),
    path.join(home, '.var', 'app', FLATPAK_ID, '.local', 'share', 'Steam'),
    path.join(home, '.var', 'app', FLATPAK_ID, 'data', 'Steam'),
  ];
}

// A Steam folder: has a library, the sign-in list, or Steam's own launcher script. (Requiring only steamapps
// made Steam look "not installed" before its first download, or while Steam was updating itself.)
const looksLikeSteam = p => isDir(path.join(p, 'steamapps')) || exists(path.join(p, 'config', 'loginusers.vdf')) || exists(path.join(p, 'steam.sh'));
let lastPath = null;
function steamPath(home) {
  for (const p of candidates(home)) {
    if (looksLikeSteam(p)) {
      let real = p;
      try { real = fs.realpathSync(p); } catch {}
      if (!home) lastPath = real;
      return real;
    }
  }
  return home ? null : lastPath;   // a moment where nothing reads right (Steam rewriting its files) keeps the last answer
}

/** Steam itself is there (the client, native or Flatpak), even if it was never started. */
function clientInstalled() {
  if (has('steam')) return true;
  return exists(path.join('/var/lib/flatpak/app', FLATPAK_ID)) || exists(path.join(os.homedir(), '.local/share/flatpak/app', FLATPAK_ID));
}

const isFlatpak = p => !!p && p.includes(path.join('.var', 'app', FLATPAK_ID));

/** Installed games plus games Steam is installing, downloading or updating. */
function scan({ home } = {}) {
  const games = [];
  const steam = steamPath(home);
  if (!steam) return games;

  const { gridDir, stats } = userData(steam);
  const cache = path.join(steam, 'appcache', 'librarycache');

  for (const lib of libraries(steam)) {
    const apps = path.join(lib, 'steamapps');
    let files;
    try { files = fs.readdirSync(apps).filter(f => /^appmanifest_\d+\.acf$/.test(f)); } catch { continue; }

    for (const f of files) {
      const manifest = path.join(apps, f);
      const acf = vdf.obj(vdf.load(manifest), 'AppState');
      const appId = vdf.str(acf, 'appid');
      const name = vdf.str(acf, 'name');
      const installDir = vdf.str(acf, 'installdir');
      if (!appId || !name || !name.trim() || !installDir) continue;
      if (SKIP_APP_IDS.has(appId) || SKIP_PREFIXES.some(p => name.toLowerCase().startsWith(p.toLowerCase()))) continue;

      const dir = path.join(apps, 'common', installDir);
      const size = +vdf.str(acf, 'SizeOnDisk') || 0;
      const flags = +vdf.str(acf, 'StateFlags') || 0;
      const toDl = +vdf.str(acf, 'BytesToDownload') || 0;
      const dl = +vdf.str(acf, 'BytesDownloaded') || 0;
      const installed = (flags & FULLY_INSTALLED) !== 0;
      const state = installed
        ? ((flags & BUSY) ? 'updating' : 'installed')
        : (flags & UPDATE_PAUSED) ? 'paused'
        : (flags & BUSY) ? 'downloading'
        : 'queued';
      const st = stats[appId] || { minutes: 0, lastPlayed: 0 };

      let added = 0;
      try { added = fs.statSync(isDir(dir) ? dir : manifest).birthtimeMs || fs.statSync(manifest).mtimeMs; } catch {}

      games.push({
        id: 'steam:' + appId,
        title: name,
        source: 'steam',
        installPath: dir,
        sizeBytes: size > 0 ? size : null,
        added: Math.round(added),
        storeMinutes: st.minutes,
        storeLastPlayed: st.lastPlayed,
        launchUri: 'steam://rungameid/' + appId,
        installUri: 'steam://install/' + appId,
        installed,
        state,
        bytesDone: toDl > 0 ? dl : null,
        bytesTotal: toDl > 0 ? toDl : null,
        progress: toDl > 0 ? Math.min(1, Math.max(0, dl / toDl)) : null,
        art: findArt(appId, gridDir, cache),
        // Steam starts every game under its "reaper" with AppId=<id> on the command line.
        track: { steamAppId: appId },
      });
    }
  }
  return games;
}

function libraries(steam) {
  const seen = new Set([steam]);
  const out = [steam];
  const lf = vdf.obj(vdf.load(path.join(steam, 'steamapps', 'libraryfolders.vdf')), 'libraryfolders');
  if (!lf) return out;
  for (const entry of Object.values(lf)) {
    // New format: "0" { "path" "/mnt/games/SteamLibrary" ... }   Old format: "1" "/mnt/games/SteamLibrary"
    let p = typeof entry === 'object' ? vdf.str(entry, 'path') : entry;
    if (!p) continue;
    try { p = fs.realpathSync(p); } catch { continue; }
    if (isDir(p) && !seen.has(p)) { seen.add(p); out.push(p); }
  }
  return out;
}

/** Custom grid folder and per-app stats for the most recently signed-in Steam user. */
function userData(steam) {
  const stats = {};
  const userdata = path.join(steam, 'userdata');
  if (!isDir(userdata)) return { gridDir: null, stats };

  let accountDir = null;
  const users = vdf.obj(vdf.load(path.join(steam, 'config', 'loginusers.vdf')), 'users');
  if (users) {
    for (const [id64, u] of Object.entries(users)) {
      if (typeof u !== 'object' || vdf.str(u, 'MostRecent') !== '1' || !/^\d+$/.test(id64)) continue;
      const dir = path.join(userdata, (BigInt(id64) - STEAM_ID64_BASE).toString());
      if (isDir(dir)) accountDir = dir;
    }
  }
  if (!accountDir) {
    let best = null, bestTime = 0;
    for (const d of safeReaddir(userdata)) {
      const cfg = path.join(userdata, d, 'config', 'localconfig.vdf');
      try { const t = fs.statSync(cfg).mtimeMs; if (t > bestTime) { best = path.join(userdata, d); bestTime = t; } } catch {}
    }
    accountDir = best;
  }
  if (!accountDir) return { gridDir: null, stats };

  const local = vdf.load(path.join(accountDir, 'config', 'localconfig.vdf'));
  const apps = vdf.obj(local, 'UserLocalConfigStore', 'Software', 'Valve', 'Steam', 'apps');
  if (apps) {
    for (const [appId, a] of Object.entries(apps)) {
      if (typeof a !== 'object') continue;
      const lastSec = +vdf.str(a, 'LastPlayed') || 0;
      stats[appId] = { minutes: +vdf.str(a, 'Playtime') || 0, lastPlayed: lastSec > 0 ? lastSec * 1000 : 0 };
    }
  }
  const grid = path.join(accountDir, 'config', 'grid');
  return { gridDir: isDir(grid) ? grid : null, stats };
}

function safeReaddir(d) { try { return fs.readdirSync(d); } catch { return []; } }

function findInTree(dir, re, depth = 2) {
  for (const name of safeReaddir(dir)) {
    const p = path.join(dir, name);
    if (re.test(name) && !isDir(p)) return p;
  }
  if (depth <= 0) return null;
  for (const name of safeReaddir(dir)) {
    const p = path.join(dir, name);
    if (isDir(p)) { const hit = findInTree(p, re, depth - 1); if (hit) return hit; }
  }
  return null;
}

function findArt(appId, grid, cache) {
  const art = {};
  const pick = (kind, gridName, legacyName, patterns, cdnName) => {
    if (grid) {
      for (const ext of ['.png', '.jpg', '.jpeg', '.webp']) {
        const f = path.join(grid, gridName + ext);
        if (exists(f)) { art[kind] = f; return; }
      }
    }
    const legacy = path.join(cache, legacyName);
    if (exists(legacy)) { art[kind] = legacy; return; }
    // Newer Steam clients: librarycache/<appid>/<hash>/library_600x900.jpg etc.
    const dir = path.join(cache, appId);
    if (isDir(dir)) {
      for (const re of patterns) { const f = findInTree(dir, re); if (f) { art[kind] = f; return; } }
    }
    art[kind] = CDN + appId + '/' + cdnName; // needs internet; the UI falls back to a gradient
  };
  pick('cover', appId + 'p', `${appId}_library_600x900.jpg`, [/^library_600x900.*\.jpg$/, /^library_capsule.*\.jpg$/], 'library_600x900.jpg');
  pick('hero', appId + '_hero', `${appId}_library_hero.jpg`, [/^library_hero.*\.jpg$/], 'library_hero.jpg');
  pick('logo', appId + '_logo', `${appId}_logo.png`, [/^logo.*\.png$/], 'logo.png');
  pick('wide', appId, `${appId}_header.jpg`, [/^header.*\.jpg$/, /^library_header.*\.jpg$/], 'header.jpg');
  return art;
}

// ------------------------------------------------------------------ owned, not installed

const ASSET_CDN = 'https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/';

/**
 * Games the signed-in account owns but hasn't installed, from Steam's local app and license caches.
 * Released games for Windows or Linux (Windows ones run with Proton) are included.
 */
function scanOwned(installedIds, { home } = {}) {
  const games = [];
  const steam = steamPath(home);
  if (!steam) return games;
  const cache = require('./steamcache');
  const appinfo = cache.readAppInfo(path.join(steam, 'appcache', 'appinfo.vdf'));
  const owned = cache.readOwnedAppIds(path.join(steam, 'appcache', 'packageinfo.vdf'));
  const { gridDir, stats } = userData(steam);
  const lib = path.join(steam, 'appcache', 'librarycache');
  for (const id of owned) {
    const app = appinfo.get(id);
    const appId = String(id);
    if (!app || installedIds.has(appId) || SKIP_APP_IDS.has(appId)) continue;
    if (app.type.toLowerCase() !== 'game') continue;
    if (app.osList && !/windows|linux/i.test(app.osList)) continue;
    if (app.releaseState && app.releaseState.toLowerCase() !== 'released') continue;
    if (SKIP_PREFIXES.some(p => app.name.toLowerCase().startsWith(p.toLowerCase()))) continue;
    const st = stats[appId] || { minutes: 0, lastPlayed: 0 };
    const art = findArt(appId, gridDir, lib);
    // Exact CDN paths from Steam's own metadata beat the guessed ones.
    for (const [kind, rel] of Object.entries(app.assets)) if (!art[kind] || /^https?:/.test(art[kind])) art[kind] = ASSET_CDN + appId + '/' + rel;
    games.push({
      id: 'steam:' + appId, title: app.name, source: 'steam', added: 0,
      storeMinutes: st.minutes, storeLastPlayed: st.lastPlayed,
      launchUri: 'steam://rungameid/' + appId, installUri: 'steam://install/' + appId,
      installed: false, state: 'notInstalled', art, track: { steamAppId: appId },
    });
  }
  return games;
}

// ------------------------------------------------------------------ running Steam

/** How to hand a steam:// URL to Steam: native client, Flatpak, or the desktop's URL handler. */
function steamCommand(uri) {
  const p = steamPath();
  if (has('steam') && !isFlatpak(p)) return ['steam', [uri]];
  if (isFlatpak(p) && has('flatpak')) return ['flatpak', ['run', FLATPAK_ID, uri]];
  if (has('steam')) return ['steam', [uri]];
  return ['xdg-open', [uri]];
}

function openUri(uri) {
  const [cmd, args] = steamCommand(uri);
  log(`steam: ${cmd} ${args.join(' ')}`);
  launchDetached(cmd, args);
}

/** True if a Steam client process is running (native "steam" or the Flatpak's). */
function isRunning() {
  for (const pid of safeReaddir('/proc')) {
    if (!/^\d+$/.test(pid)) continue;
    try {
      const comm = fs.readFileSync(`/proc/${pid}/comm`, 'utf8').trim();
      if (comm === 'steam' || comm === 'steamwebhelper') return true;
    } catch {}
  }
  return false;
}

/** Starts Steam hidden (-silent) if it isn't running, and waits until it's up. */
async function ensureRunningSilent() {
  if (!steamPath() || isRunning()) return;
  const [cmd, args] = steamCommand('-silent');
  launchDetached(cmd, args);
  for (let i = 0; i < 60; i++) {
    await new Promise(r => setTimeout(r, 500));
    if (isRunning()) { await new Promise(r => setTimeout(r, 3000)); return; }
  }
}

module.exports = { scan, scanOwned, steamPath, clientInstalled, openUri, isRunning, ensureRunningSilent, candidates };
