'use strict';
// Runs a Windows game on Linux with Proton through umu-run, the way Heroic and Lutris do.
// Each game gets its own Wine prefix in ~/.local/share/nimbus/prefixes/<store>-<id>, so one game's
// settings or broken install never affects another. umu downloads Proton itself the first time.

const fs = require('node:fs');
const path = require('node:path');
const tools = require('./tools');
const { DATA_DIR, launchDetached, log } = require('../util');

const PREFIXES = path.join(DATA_DIR, 'prefixes');
const STORE_CODE = { epic: 'egs', gog: 'gog' }; // umu's names, used to apply known per-game fixes

/** Starts exe with Proton. Resolves with the child process. */
async function launch({ store, id, exe, args = [], cwd, env = {} }) {
  const umu = await tools.ensure(tools.TOOLS.umu);
  const prefix = path.join(PREFIXES, `${store}-${String(id).replace(/[^\w.-]/g, '_')}`);
  fs.mkdirSync(prefix, { recursive: true });
  log(`proton: ${store}:${id} ${exe}`);
  return launchDetached(umu, [exe, ...args], {
    cwd: cwd || path.dirname(exe),
    env: {
      ...process.env, ...env,
      WINEPREFIX: prefix,
      GAMEID: process.env.NIMBUS_GAMEID || '0',
      STORE: STORE_CODE[store] || 'none',
      PROTON_VERB: 'waitforexitandrun',
    },
  });
}

/** Removes a game's Wine prefix (after uninstalling it). */
function removePrefix(store, id) {
  const prefix = path.join(PREFIXES, `${store}-${String(id).replace(/[^\w.-]/g, '_')}`);
  if (prefix.startsWith(PREFIXES + path.sep)) fs.rmSync(prefix, { recursive: true, force: true });
}

module.exports = { launch, removePrefix, PREFIXES };
