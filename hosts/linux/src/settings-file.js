'use strict';
// The safe way this host writes a settings file (docs/SETTINGS-PERSISTENCE.md; the same rules and file names as
// desktop/shell/hearth_settings_io.py and hearth-core's settings_file.rs): a temporary file in the same folder,
// fsync, rename; the last good version kept as FILE.bak; a damaged JSON file read back from FILE.bak, never replaced
// with the defaults.

const fs = require('node:fs');
const path = require('node:path');

function validJsonObject(text) { try { const v = JSON.parse(text); return !!v && typeof v === 'object' && !Array.isArray(v); } catch { return false; } }

function atomic(file, data, mode) {
  const dir = path.dirname(file);
  fs.mkdirSync(dir, { recursive: true });
  const tmp = path.join(dir, `.${path.basename(file)}.${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}.tmp`);
  try {
    const fd = fs.openSync(tmp, 'w', mode ?? (() => { try { return fs.statSync(file).mode & 0o7777; } catch { return 0o644; } })());
    try { fs.writeSync(fd, data); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
    fs.renameSync(tmp, file);
  } catch (e) { try { fs.rmSync(tmp, { force: true }); } catch {} throw e; }
  try { const d = fs.openSync(dir, 'r'); try { fs.fsyncSync(d); } finally { fs.closeSync(d); } } catch {}
}

/** Writes TEXT atomically, keeping the current file as FILE.bak when it's good. Unchanged text isn't written. */
function writeSettings(file, text, { json = false, mode } = {}) {
  let cur = null;
  try { cur = fs.readFileSync(file, 'utf8'); } catch {}
  if (cur === text) return false;
  if (cur !== null && cur.trim() && (!json || validJsonObject(cur))) atomic(file + '.bak', cur, 0o600);
  atomic(file, text, mode);
  return true;
}

/** The JSON object in FILE; damaged: FILE.bak's (put back). {} when missing or neither can be read. */
function readSettings(file) {
  for (const f of [file, file + '.bak']) {
    let text;
    try { text = fs.readFileSync(f, 'utf8'); } catch { continue; }
    if (!validJsonObject(text)) continue;
    if (f !== file) { try { atomic(file, text); } catch {} }
    return JSON.parse(text);
  }
  return {};
}

/** Sets these keys in the latest FILE (null removes one); every other key stays. */
function mergeSettings(file, changes) {
  const o = readSettings(file);
  for (const [k, v] of Object.entries(changes)) { if (v === null) delete o[k]; else o[k] = v; }
  writeSettings(file, JSON.stringify(o, null, 2) + '\n', { json: true });
  return o;
}

module.exports = { writeSettings, readSettings, mergeSettings, validJsonObject };
