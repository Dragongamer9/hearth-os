'use strict';
// Talking to gamescope (Game Mode's compositor) through X11 root-window properties, the way Steam does:
//   GAMESCOPE_FOCUSABLE_WINDOWS     what can be shown: (window, app id, pid) triples, set by gamescope
//   GAMESCOPE_FOCUSED_WINDOW        what's in front now, set by gamescope
//   GAMESCOPECTRL_BASELAYER_WINDOW  which window to put in front, set by us (removed = gamescope decides)
//   GAMESCOPECTRL_BASELAYER_APPID   same by Steam app id (games Steam starts carry their app id)
//   GAMESCOPE_FPS_LIMIT             frame-rate limit, 0 = none
// Used for: bringing a game to the front when it starts or is resumed, Hearth itself when it should be in
// front, and the overlay's list of open apps. Needs xprop; everything here is best effort.

const fs = require('node:fs');
const { run, has, log } = require('./util');

const available = () => has('xprop') && !!process.env.DISPLAY;

/** Numbers in a CARDINAL/WINDOW property of the root window (or a window). */
async function cardinals(name, id = null) {
  const out = await run('xprop', id ? ['-id', String(id), name] : ['-root', name], { timeout: 3000 }).catch(() => '');
  const m = out.match(/=\s*(.+)$/m);
  if (!m) return [];
  return m[1].split(',').map(s => s.trim()).map(s => (s.startsWith('0x') ? parseInt(s, 16) : parseInt(s, 10))).filter(Number.isFinite);
}
async function strProp(id, name) {
  const out = await run('xprop', ['-id', String(id), name], { timeout: 3000 }).catch(() => '');
  const m = out.match(/=\s*"(.*)"/);
  return m ? m[1] : '';
}
const setRoot = (name, v) => run('xprop', ['-root', '-f', name, '32c', '-set', name, String(v)], { timeout: 3000 });
const removeRoot = name => run('xprop', ['-root', '-remove', name], { timeout: 3000 }).catch(() => {});

/** [{win, appid, pid}] */
async function focusableWindows() {
  const n = await cardinals('GAMESCOPE_FOCUSABLE_WINDOWS');
  const out = [];
  for (let i = 0; i + 2 < n.length; i += 3) out.push({ win: n[i], appid: n[i + 1], pid: n[i + 2] });
  return out;
}

function procName(pid) {
  try { return fs.readFileSync(`/proc/${pid}/comm`, 'utf8').trim(); } catch { return ''; }
}

/** Open apps for the overlay: [{id, pid, title, app, focused}], one per process (its first window). */
async function apps() {
  const [wins, focused] = await Promise.all([focusableWindows(), cardinals('GAMESCOPE_FOCUSED_WINDOW')]);
  const out = [], seen = new Set();
  for (const w of wins) {
    if (!w.pid || seen.has(w.pid)) continue;
    seen.add(w.pid);
    const title = (await strProp(w.win, '_NET_WM_NAME')) || (await strProp(w.win, 'WM_NAME'));
    out.push({ id: w.win, pid: w.pid, appid: w.appid, title: title || procName(w.pid) || 'App', app: procName(w.pid), focused: focused[0] === w.win });
  }
  return out;
}

async function focusWindow(win) { await setRoot('GAMESCOPECTRL_BASELAYER_WINDOW', win); }
async function focusAppId(appid) { await setRoot('GAMESCOPECTRL_BASELAYER_APPID', appid); }
async function clearFocus() { await removeRoot('GAMESCOPECTRL_BASELAYER_WINDOW'); await removeRoot('GAMESCOPECTRL_BASELAYER_APPID'); }

/**
 * Brings a game to the front once its window exists: tries for a while (games take time to open a window),
 * matching the game's processes, or its Steam app id. Resolves true when it found one.
 */
async function focusGame({ pids = () => [], steamAppId = 0, exclude = [], tries = 20, every = 1500 } = {}) {
  if (!available()) return false;
  if (steamAppId) await focusAppId(steamAppId).catch(() => {});
  for (let i = 0; i < tries; i++) {
    const want = new Set(pids());
    const wins = (await focusableWindows()).filter(w => !exclude.includes(w.pid));
    const hit = wins.find(w => want.has(w.pid)) || (steamAppId ? wins.find(w => w.appid === steamAppId) : null);
    if (hit) { await focusWindow(hit.win).catch(() => {}); log(`gamescope: game window ${hit.win} in front`); return true; }
    await new Promise(r => setTimeout(r, every));
  }
  return false;
}

module.exports = { available, apps, focusWindow, focusAppId, clearFocus, focusGame, focusableWindows, setRoot, cardinals };
