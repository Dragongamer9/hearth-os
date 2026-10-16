'use strict';
// Small helpers shared by the Linux host: running system tools, reading files safely, logging.

const { execFile, spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

/** Nimbus's own data folder (~/.local/share/nimbus). */
const DATA_DIR = path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'), 'nimbus');
fs.mkdirSync(DATA_DIR, { recursive: true });

const LOG_PATH = path.join(DATA_DIR, 'bridge.log');
function log(line) {
  const t = new Date().toTimeString().slice(0, 8);
  try { fs.appendFileSync(LOG_PATH, `${t} ${line}\n`); } catch { /* best effort */ }
}

/**
 * Runs a program and resolves with its stdout. Rejects with a readable message
 * (stderr's last line) if it fails, isn't installed, or takes too long.
 */
function run(file, args = [], { timeout = 15000, input } = {}) {
  return new Promise((resolve, reject) => {
    const child = execFile(file, args, { timeout, maxBuffer: 8 * 1024 * 1024, env: { ...process.env, LC_ALL: 'C' } },
      (err, stdout, stderr) => {
        if (!err) return resolve(stdout);
        if (err.code === 'ENOENT') return reject(new Error(`${file} is not installed`));
        const msg = String(stderr || stdout || err.message).trim().split('\n').filter(Boolean).pop() || err.message;
        reject(new Error(msg.replace(/^Error:\s*/i, '')));
      });
    if (input != null) { child.stdin.write(input); child.stdin.end(); }
  });
}

const whichCache = new Map();
/** True if a program is on PATH. */
function has(cmd) {
  if (whichCache.has(cmd)) return whichCache.get(cmd);
  const found = (process.env.PATH || '').split(':').some(dir => {
    try { fs.accessSync(path.join(dir, cmd), fs.constants.X_OK); return true; } catch { return false; }
  });
  whichCache.set(cmd, found);
  return found;
}

/** Starts a program that should outlive this call (a game, a launcher). */
function launchDetached(file, args = [], opts = {}) {
  const child = spawn(file, args, { detached: true, stdio: 'ignore', ...opts });
  child.on('error', e => log(`launch ${file} failed: ${e.message}`));
  child.unref();
  return child;
}

function readJson(file) {
  try { return JSON.parse(fs.readFileSync(file, 'utf8')); } catch { return null; }
}
function readText(file) {
  try { return fs.readFileSync(file, 'utf8'); } catch { return null; }
}
function exists(p) { try { fs.accessSync(p); return true; } catch { return false; } }
function isDir(p) { try { return fs.statSync(p).isDirectory(); } catch { return false; } }

/** Writes a file atomically (temp file + rename) so a crash never leaves it half-written. */
function writeJsonAtomic(file, data) {
  const tmp = file + '.tmp';
  fs.writeFileSync(tmp, JSON.stringify(data, null, 2));
  fs.renameSync(tmp, file);
}

module.exports = { DATA_DIR, LOG_PATH, log, run, has, launchDetached, readJson, readText, exists, isDir, writeJsonAtomic };
