'use strict';
// Power mode and power limit (TDP) through desktop/shell/hearth-power, which Desktop Mode's Hearth Settings
// uses too. It picks what the device offers (power-profiles/tuned, SteamOS Manager, PowerStation, or Hearth's
// own way through hearth-admin) and says which in "via".

const fs = require('node:fs');
const path = require('node:path');
const { execFile } = require('node:child_process');

const MODE_TO_PROFILE = { efficiency: 'power-saver', balanced: 'balanced', performance: 'performance' };
const PROFILE_TO_MODE = { 'power-saver': 'efficiency', balanced: 'balanced', performance: 'performance' };

function helperPath() {
  const c = [process.resourcesPath && path.join(process.resourcesPath, 'desktop', 'shell', 'hearth-power'),
    path.join(__dirname, '..', 'desktop', 'shell', 'hearth-power')].filter(Boolean);
  return c.find(p => fs.existsSync(p)) || c[c.length - 1];
}

function call(args, timeout = 30000) {
  return new Promise((resolve, reject) => {
    execFile('python3', [helperPath(), ...args], { timeout }, (err, stdout) => {
      const out = String(stdout || '').trim().split('\n').pop() || '';
      if (out.startsWith('error: ')) return reject(new Error(out.slice(7)));
      if (err) return reject(new Error(err.killed ? 'Took too long' : err.message));
      try { resolve(JSON.parse(out)); } catch { reject(new Error('Unexpected answer from hearth-power')); }
    });
  });
}

let cache = null, cachedAt = 0, pending = null;
/** {mode:{supported, mode, available:[modes], via}, tdp:{supported, watts, min, max, via}} — cached for a few seconds. */
async function status(fresh = false) {
  if (!fresh && cache && Date.now() - cachedAt < 5000) return cache;
  if (pending) return pending;
  pending = call(['status']).then(r => {
    const m = r.mode || {};
    cache = {
      mode: { supported: !!m.supported, mode: PROFILE_TO_MODE[m.active] || 'balanced', available: (m.available || []).map(p => PROFILE_TO_MODE[p]).filter(Boolean), via: m.via || null },
      tdp: r.tdp && r.tdp.supported ? r.tdp : { supported: false },
    };
    cachedAt = Date.now();
    return cache;
  }).catch(() => ({ mode: { supported: false, mode: 'balanced', available: [], via: null }, tdp: { supported: false } }))
    .finally(() => { pending = null; });
  return pending;
}

async function setMode(mode) {
  const p = MODE_TO_PROFILE[mode];
  if (!p) throw new Error('Unknown mode');
  await call(['mode', p]);
  return (await status(true)).mode;
}
async function setTdp(watts) {
  await call(['tdp', String(Math.round(+watts))]);
  return (await status(true)).tdp;
}
function restore() { return call(['restore']).catch(() => null); }

module.exports = { status, setMode, setTdp, restore, MODE_TO_PROFILE, PROFILE_TO_MODE };
