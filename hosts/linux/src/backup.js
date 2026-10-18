'use strict';
// Backup and restore (Settings › System › Backup): Hearth's settings, profiles, favorites, collections, play time,
// non-library games and controller set-up in one file, on a USB drive or in your Backups folder. Sign-ins
// (store tokens) only if you ask for them. Not games, art or caches: those come back on their own.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { run, DATA_DIR } = require('./util');

const CONF_DIR = path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'nimbus');
/** Never in a backup: big, or made again on their own. */
const SKIP = ['cache', 'tools', 'prefixes', 'art-cache', 'nimbus-dev', 'session', 'xcloud-catalog.json', 'before-restore'];
/** Sign-ins: only with "Include sign-ins" (anyone with the file could use them). */
const SIGNINS = ['accounts', 'legendary', 'gog', 'remote-play.json'];
const NAME = /^hearth-backup-[\w.-]+\.tar\.gz$/;

function destinations(user = os.userInfo().username, home = os.homedir()) {
  const out = [];
  for (const root of [`/run/media/${user}`, '/media/' + user]) {
    let names = [];
    try { names = fs.readdirSync(root); } catch {}
    for (const n of names) { const p = path.join(root, n); try { if (fs.statSync(p).isDirectory()) out.push({ path: p, label: n, removable: true }); } catch {} }
  }
  out.push({ path: path.join(home, 'Backups'), label: 'Backups folder (on this device)', removable: false });
  return out;
}

/** Backups found in these folders (and their Hearth Backups subfolder), newest first. */
function list(dirs) {
  const out = [];
  for (const d of dirs) for (const dir of [d, path.join(d, 'Hearth Backups')]) {
    let names = [];
    try { names = fs.readdirSync(dir); } catch {}
    for (const n of names) if (NAME.test(n)) { try { const st = fs.statSync(path.join(dir, n)); out.push({ file: path.join(dir, n), name: n, bytes: st.size, at: st.mtimeMs }); } catch {} }
  }
  return out.sort((a, b) => b.at - a.at);
}

/** tar's exclusions for Hearth's data folder (names as tar sees them there: "./cache"). */
function excludes(signins) {
  const ex = ['--no-anchored', '--exclude=*.log', '--exclude=*.tmp', '--exclude=*.part', '--anchored'];
  for (const n of SKIP) ex.push(`--exclude=./${n}`);
  ex.push('--exclude=./profiles/*/cache');
  if (!signins) for (const n of SIGNINS) ex.push(`--exclude=./${n}`, `--exclude=./profiles/*/${n}`);
  return ex;
}

async function create({ dest, signins = false, dataDir = DATA_DIR, confDir = CONF_DIR, now = new Date(), version = '' } = {}) {
  if (!dest) throw new Error('Pick where to save the backup');
  const dir = /\/(run\/)?media\//.test(dest) ? path.join(dest, 'Hearth Backups') : dest;
  fs.mkdirSync(dir, { recursive: true });
  const stamp = now.toISOString().slice(0, 16).replace(/[T:]/g, '-');
  const file = path.join(dir, `hearth-backup-${stamp}.tar.gz`);
  const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-backup-'));
  const tarFile = path.join(stage, 'backup.tar');
  const long = { timeout: 10 * 60 * 1000 };
  try {
    fs.writeFileSync(path.join(stage, 'hearth-backup.json'), JSON.stringify({ made: now.toISOString(), version, signins: !!signins, host: os.hostname() }));
    await run('tar', ['-cf', tarFile, '-C', stage, 'hearth-backup.json'], long);
    // Each folder goes in under its own name in the backup: data/... and config/...
    if (fs.existsSync(dataDir)) await run('tar', ['-rf', tarFile, '-C', dataDir, ...excludes(signins), '--transform=s,^\\.,data,', '.'], long);
    if (fs.existsSync(confDir)) await run('tar', ['-rf', tarFile, '-C', confDir, '--transform=s,^\\.,config,', '.'], long);
    await run('gzip', ['-6', tarFile], long);
    fs.copyFileSync(tarFile + '.gz', file + '.part');
    fs.renameSync(file + '.part', file);
    try { await run('sync', [], { timeout: 60000 }); } catch {}   // safe to unplug the USB drive
    return { file, bytes: fs.statSync(file).size };
  } catch (e) {
    fs.rmSync(file + '.part', { force: true });
    throw e;
  } finally { fs.rmSync(stage, { recursive: true, force: true }); }
}

async function info(file) {
  const names = (await run('tar', ['-tzf', file], { timeout: 60000 })).split('\n').filter(Boolean);
  if (!names.includes('hearth-backup.json')) throw new Error("That isn't a Hearth backup");
  const meta = JSON.parse(await run('tar', ['-xzOf', file, 'hearth-backup.json']));
  return { ...meta, files: names.length };
}

/**
 * Puts a backup back. What it replaces is kept in data/before-restore/<time>/ first (in case it was the wrong
 * backup). Sign-ins on this device stay when the backup has none. Hearth must restart afterwards.
 */
async function restore(file, { dataDir = DATA_DIR, confDir = CONF_DIR, now = new Date() } = {}) {
  const meta = await info(file);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-restore-'));
  try {
    await run('tar', ['-xzf', file, '-C', tmp, '--no-same-owner'], { timeout: 10 * 60 * 1000 });
    const keep = path.join(dataDir, 'before-restore', now.toISOString().slice(0, 19).replace(/[T:]/g, '-'));
    const swap = (from, to, keepDir) => {
      let names = [];
      try { names = fs.readdirSync(from); } catch { return; }
      fs.mkdirSync(to, { recursive: true });
      for (const n of names) {
        const dst = path.join(to, n);
        if (fs.existsSync(dst)) { fs.mkdirSync(keepDir, { recursive: true }); fs.renameSync(dst, path.join(keepDir, n)); }
        fs.renameSync(path.join(from, n), dst);
      }
    };
    swap(path.join(tmp, 'data'), dataDir, keep);
    swap(path.join(tmp, 'config'), confDir, path.join(keep, 'config'));
    return meta;
  } finally { fs.rmSync(tmp, { recursive: true, force: true }); }
}

module.exports = { destinations, list, create, restore, info, SKIP, SIGNINS, CONF_DIR };
