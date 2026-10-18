'use strict';
// Per-game settings (game page › Game settings): which Proton runs it, extra launch options and
// environment variables, MangoHud, GameMode, and AMD FSR. Applied each time Hearth starts the game.
// Steam games keep Steam's own settings (Properties in Steam); Hearth starts those through Steam.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { has } = require('../util');

// fpsLimit and tdp (watts) work for every game, Steam's included: Hearth sets them when the game starts and puts
// the usual ones back when it ends (0 = use Hearth's normal setting).
const DEFAULTS = { proton: '', args: '', env: '', mangohud: false, gamemode: false, fsr: false, fpsLimit: 0, tdp: 0, cloudSaves: true };

/** What kind of launch a game has, which decides the settings it can have. */
function kindOf(g) {
  if (g.cloud) return 'cloud';
  if (g.source === 'steam') return 'steam';
  if (g.source === 'emu') return 'emu';
  if (g.launchVia === 'legendary' || g.windowsExe || (g.source === 'manual' && /\.exe$/i.test(g.exe || ''))) return 'proton';
  if (g.source === 'manual') return 'native';
  return 'external';   // installed and started by Heroic
}

/** Proton versions to choose from: umu's defaults, then any installed ones (Steam, ProtonUp, Bazzite). */
function protonChoices(home = os.homedir()) {
  const out = [
    { id: '', label: 'Default', sub: 'UMU-Proton, the newest, downloaded when needed' },
    { id: 'GE-Proton', label: 'GE-Proton (newest)', sub: 'Community build with extra fixes, downloaded when needed' },
  ];
  const roots = [
    path.join(home, '.local/share/Steam/compatibilitytools.d'),
    path.join(home, '.steam/root/compatibilitytools.d'),
    path.join(home, '.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d'),
    '/usr/share/steam/compatibilitytools.d',
    path.join(home, '.local/share/Steam/steamapps/common'),
    path.join(home, '.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common'),
  ];
  const seen = new Set();
  for (const root of roots) {
    let names = [];
    try { names = fs.readdirSync(root); } catch { continue; }
    for (const n of names.sort().reverse()) {
      const dir = path.join(root, n);
      if (!fs.existsSync(path.join(dir, 'proton'))) continue;     // a Proton build has a 'proton' script
      let real = dir; try { real = fs.realpathSync(dir); } catch {}
      if (seen.has(real) || seen.has(n)) continue;
      seen.add(real); seen.add(n);
      out.push({ id: dir, label: n, sub: root.includes('steamapps') ? 'Installed with Steam' : 'Installed' });
    }
  }
  return out;
}

/** "A=1 B='two words'" → {A:'1', B:'two words'}. Only NAME=value pairs; anything else is ignored. */
function parseEnv(text) {
  const out = {};
  const re = /([A-Za-z_][A-Za-z0-9_]*)=("([^"]*)"|'([^']*)'|(\S*))/g;
  let m;
  while ((m = re.exec(String(text || '')))) out[m[1]] = m[3] ?? m[4] ?? m[5] ?? '';
  return out;
}

/** Splits launch options like a shell would (quotes group words). */
function splitArgs(text) {
  const out = [];
  const re = /"([^"]*)"|'([^']*)'|(\S+)/g;
  let m;
  while ((m = re.exec(String(text || '')))) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

/**
 * Applies settings to a launch: {cmd, args, env} → the same with extra args, environment and wrappers.
 * proton: whether this launch goes through umu/Proton (PROTONPATH, FSR only matter then).
 */
function apply(opts, { cmd, args = [], env = {} }, { proton = false } = {}) {
  const o = { ...DEFAULTS, ...(opts || {}) };
  const e = { ...env, ...parseEnv(o.env) };
  if (proton && o.proton) e.PROTONPATH = o.proton;
  if (proton && o.fsr) e.WINE_FULLSCREEN_FSR = '1';
  if (o.mangohud) e.MANGOHUD = '1';
  let c = cmd, a = [...args, ...splitArgs(o.args)];
  if (o.gamemode && has('gamemoderun')) { a = [c, ...a]; c = 'gamemoderun'; }
  return { cmd: c, args: a, env: e };
}

class GameOptions {
  constructor(store) { this.store = store; }
  get(id) { return { ...DEFAULTS, ...((this.store.data.gameOptions || {})[id] || {}) }; }
  set(id, patch) {
    const clean = {};
    for (const k of Object.keys(DEFAULTS)) if (k in patch) clean[k] = typeof DEFAULTS[k] === 'boolean' ? !!patch[k]
      : typeof DEFAULTS[k] === 'number' ? Math.max(0, Math.min(k === 'tdp' ? 120 : 240, Math.round(+patch[k] || 0)))
      : String(patch[k] || '').slice(0, 2000);
    this.store.update(d => { d.gameOptions = d.gameOptions || {}; d.gameOptions[id] = { ...this.get(id), ...clean }; });
    return this.get(id);
  }
}

module.exports = { GameOptions, kindOf, protonChoices, parseEnv, splitArgs, apply, DEFAULTS };
