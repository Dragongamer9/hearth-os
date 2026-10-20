'use strict';
// Game art for the overlay page, the way the Rust host serves it (hosts/hearth-rs/src/main.rs, art()):
//   nimbus-art://file/<percent-encoded path>  a picture on the disk (Steam's covers, your own pictures)
//   nimbus-art://cache/<name>                 web art Hearth downloaded (~/.local/share/nimbus/art-cache/<name>.<ext>)
// Pictures only: anything else is "not found".

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const PICTURES = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif']);

function dataDir(env = process.env) {
  return path.join(env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'), 'nimbus');
}

function isPicture(file) {
  if (!PICTURES.has(path.extname(file).slice(1).toLowerCase())) return false;
  try { return fs.statSync(file).isFile(); } catch { return false; }
}

/** The file behind a nimbus-art:// address, or null. */
function artFile(url, data = dataDir()) {
  const u = String(url || '').split('?')[0].split('#')[0];
  const cache = u.split('://cache/')[1];
  if (cache !== undefined) {
    if (!/^[0-9a-f]{24}$/i.test(cache)) return null;
    const dir = path.join(data, 'art-cache');
    let names = [];
    try { names = fs.readdirSync(dir); } catch { return null; }
    const hit = names.find(n => n.startsWith(cache + '.'));
    return hit && isPicture(path.join(dir, hit)) ? path.join(dir, hit) : null;
  }
  const raw = u.split('://file/')[1];
  if (raw === undefined) return null;
  let p;
  try { p = decodeURIComponent(raw); } catch { return null; }
  p = path.normalize(p);
  if (!path.isAbsolute(p)) return null;
  return isPicture(p) ? p : null;
}

module.exports = { artFile, dataDir };
