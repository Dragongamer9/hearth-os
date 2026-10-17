'use strict';
// Reads Steam's binary caches (port of the Windows host's Library/SteamCache.cs):
//   appcache/appinfo.vdf     metadata (name, type, OS, art) for every app the client knows about
//   appcache/packageinfo.vdf the packages (licenses) this account owns, with their app ids
// Together they give the games you own, without a Steam Web API key. appinfo v27–v29, packageinfo v27–v28.

const fs = require('node:fs');

class Reader {
  constructor(buf) { this.b = buf; this.i = 0; }
  u32() { const v = this.b.readUInt32LE(this.i); this.i += 4; return v; }
  i32() { const v = this.b.readInt32LE(this.i); this.i += 4; return v; }
  i64() { const v = this.b.readBigInt64LE(this.i); this.i += 8; return v; }
  cstr() { const end = this.b.indexOf(0, this.i); const s = this.b.toString('utf8', this.i, end); this.i = end + 1; return s; }
}

/** Binary KeyValues: 0x00 object, 0x01 string, 0x02 int32, 0x03 float, 0x07 uint64, 0x0A int64, 0x08 end. */
function readKv(r, keys) {
  const d = {};
  for (;;) {
    const type = r.b[r.i++];
    if (type === 0x08) return d;
    let key;
    if (keys) { const k = r.i32(); key = k >= 0 && k < keys.length ? keys[k] : String(k); } else key = r.cstr();
    let value;
    switch (type) {
      case 0x00: value = readKv(r, keys); break;
      case 0x01: value = r.cstr(); break;
      case 0x02: value = r.i32(); break;
      case 0x03: value = r.b.readFloatLE(r.i); r.i += 4; break;
      case 0x07: value = r.b.readBigUInt64LE(r.i); r.i += 8; break;
      case 0x0a: value = r.i64(); break;
      default: throw new Error('Unknown KV type ' + type);
    }
    d[key.toLowerCase()] = value;
  }
}

const child = (d, k) => (d && typeof d[k] === 'object' && d[k] !== null && !Array.isArray(d[k]) ? d[k] : null);

/** Art file names from library_assets_full (newer apps use hashed sub-folders, e.g. "f86c…/library_capsule.jpg"). */
function assets(common) {
  const a = {};
  const english = (node, k) => { const img = child(node, k); return img && typeof img.english === 'string' && img.english ? img.english : null; };
  const full = child(common, 'library_assets_full');
  const pick = (kind, name) => { const v = english(child(full, name), 'image'); if (v) a[kind] = v; };
  pick('cover', 'library_capsule'); pick('hero', 'library_hero'); pick('logo', 'library_logo');
  const header = english(child(full, 'library_header'), 'image') || (child(common, 'header_image') || {}).english;
  if (header) a.wide = header;
  return a;
}

function readAppInfo(path) {
  const result = new Map();
  let buf;
  try { buf = fs.readFileSync(path); } catch { return result; }
  const r = new Reader(buf);
  const magic = r.u32();
  r.u32(); // universe
  let keys = null, headerSkip;
  switch (magic) {
    case 0x07564429: { // v29: key names live in a string table at the end
      const t = new Reader(buf); t.i = Number(r.i64());
      const count = t.u32();
      keys = Array.from({ length: count }, () => t.cstr());
      headerSkip = 4 + 4 + 8 + 20 + 4 + 20;
      break;
    }
    case 0x07564428: headerSkip = 4 + 4 + 8 + 20 + 4 + 20; break;
    case 0x07564427: headerSkip = 4 + 4 + 8 + 20 + 4; break;
    default: throw new Error(`Unknown appinfo.vdf version 0x${magic.toString(16)}`);
  }
  while (r.i + 8 <= buf.length) {
    const appId = r.u32();
    if (appId === 0) break;
    const size = r.u32();
    const end = r.i + size;
    try {
      const e = new Reader(buf); e.i = r.i + headerSkip;
      const root = readKv(e, keys);
      const common = child(child(root, 'appinfo'), 'common');
      if (common && typeof common.name === 'string' && typeof common.type === 'string') {
        result.set(appId, { appId, name: common.name.trim(), type: common.type, osList: common.oslist || null,
          releaseState: common.releasestate || null, assets: assets(common) });
      }
    } catch { /* skip a malformed entry */ }
    r.i = end;
  }
  return result;
}

/** App ids in every owned package except package 0 (Steam's base package that everyone has). */
function readOwnedAppIds(path) {
  const owned = new Set();
  let buf;
  try { buf = fs.readFileSync(path); } catch { return owned; }
  const r = new Reader(buf);
  const magic = r.u32();
  r.u32(); // universe
  if ((magic & 0xffffff00) >>> 0 !== 0x06565500) throw new Error(`Unknown packageinfo.vdf version 0x${magic.toString(16)}`);
  const hasToken = magic >= 0x06565528;
  while (r.i + 4 <= buf.length) {
    const pkg = r.u32();
    if (pkg === 0xffffffff) break;
    r.i += 20 + 4 + (hasToken ? 8 : 0);
    const root = readKv(r, null);
    if (pkg === 0) continue;
    for (const p of Object.values(root)) {
      const ids = child(p, 'appids');
      if (!ids) continue;
      for (const v of Object.values(ids)) if (typeof v === 'number' && v > 0) owned.add(v);
    }
  }
  return owned;
}

module.exports = { readAppInfo, readOwnedAppIds, readKv, Reader };
