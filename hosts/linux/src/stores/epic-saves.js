'use strict';
// Epic cloud saves for games Hearth installed: before a game starts, newer saves come down from Epic; after it
// closes, new ones go up (legendary sync-saves). Epic gives each game a save folder written with Windows
// placeholders ({AppData}/Game/Saved); Hearth fills them in for the game's own Wine prefix.

const fs = require('node:fs');
const path = require('node:path');

/** The game's save folder as Epic writes it, or null when it has no cloud saves. */
function template(meta) {
  const c = ((meta && meta.metadata) || meta || {}).customAttributes || {};
  return (c.CloudSaveFolder && c.CloudSaveFolder.value) || null;
}

/** drive_c of a Proton prefix (umu puts it in the prefix itself; Proton proper in pfx/). */
function driveC(prefix) {
  for (const d of [path.join(prefix, 'drive_c'), path.join(prefix, 'pfx', 'drive_c')]) if (fs.existsSync(d)) return d;
  return path.join(prefix, 'drive_c');
}

/** Fills in Epic's placeholders. Null if one is left that Hearth doesn't know. */
function resolve(tpl, { prefix, installDir, accountId }) {
  if (!tpl) return null;
  const user = path.join(driveC(prefix), 'users', 'steamuser');
  const vars = {
    appdata: path.join(user, 'AppData', 'Local'),
    localappdata: path.join(user, 'AppData', 'Local'),
    roamingappdata: path.join(user, 'AppData', 'Roaming'),
    userdir: path.join(user, 'Documents'),
    userprofile: user,
    usersavedgames: path.join(user, 'Saved Games'),
    installdir: installDir || '',
    epicid: accountId || '',
  };
  let missing = false;
  const out = tpl.replace(/\{(\w+)\}/g, (all, k) => {
    const v = vars[k.toLowerCase()];
    if (!v) { missing = true; return all; }
    return v;
  }).replace(/\\/g, '/');
  if (missing || /%\w+%/.test(out)) return null;
  return path.normalize(out);
}

/** Everything legendary needs to sync one game's saves, read from its own files. */
function plan(app, { configDir, prefix }) {
  const read = f => { try { return JSON.parse(fs.readFileSync(path.join(configDir, f), 'utf8')); } catch { return null; } };
  const tpl = template(read(path.join('metadata', `${app}.json`)));
  if (!tpl) return null;
  const inst = (read('installed.json') || {})[app] || {};
  const user = read('user.json') || {};
  const savePath = resolve(tpl, { prefix, installDir: inst.install_path, accountId: user.account_id });
  return savePath ? { app, savePath } : null;
}

/** legendary's arguments for one direction: 'down' before playing, 'up' after. */
function args(p, direction) {
  return ['-y', 'sync-saves', p.app, '--save-path', p.savePath, direction === 'down' ? '--skip-upload' : '--skip-download'];
}

module.exports = { template, resolve, plan, args, driveC };
