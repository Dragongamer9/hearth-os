'use strict';
// Shared pieces for the store accounts: the cached list of games on an account, a small secrets store,
// and JSON helpers. Same shapes as the Windows host (Stores/StoreCommon.cs).

const fs = require('node:fs');
const path = require('node:path');
const { DATA_DIR, readJson, writeJsonAtomic } = require('../util');

const CACHE_DIR = path.join(DATA_DIR, 'cache');

/** Games on a store account: { fetched (unix ms), games: [{id, title, art, added, lastPlayed, note}] }. */
class OwnedCache {
  constructor(data = {}) { this.fetched = data.fetched || 0; this.games = data.games || []; }
  static load(name) { return new OwnedCache(readJson(path.join(CACHE_DIR, name)) || {}); }
  save(name) { fs.mkdirSync(CACHE_DIR, { recursive: true }); writeJsonAtomic(path.join(CACHE_DIR, name), { fetched: this.fetched, games: this.games }); }
  static remove(name) { fs.rmSync(path.join(CACHE_DIR, name), { force: true }); }
}

/**
 * Small secrets (the Xbox sign-in token). Encrypted with the desktop's keyring through Electron's safeStorage
 * when it's available; otherwise kept in a file only you can read (mode 600).
 */
const secure = {
  impl: null, // set by main.js: { isEncryptionAvailable(), encryptString(s), decryptString(buf) }
  dir: path.join(DATA_DIR, 'accounts'),
  file(name) { return path.join(this.dir, name + '.bin'); },
  save(name, value) {
    fs.mkdirSync(this.dir, { recursive: true, mode: 0o700 });
    const enc = this.impl && this.impl.isEncryptionAvailable();
    const data = enc ? Buffer.concat([Buffer.from('E1'), this.impl.encryptString(value)]) : Buffer.concat([Buffer.from('P1'), Buffer.from(value, 'utf8')]);
    fs.writeFileSync(this.file(name) + '.tmp', data, { mode: 0o600 });
    fs.renameSync(this.file(name) + '.tmp', this.file(name));
  },
  load(name) {
    try {
      const b = fs.readFileSync(this.file(name));
      const tag = b.subarray(0, 2).toString();
      if (tag === 'P1') return b.subarray(2).toString('utf8');
      if (tag === 'E1' && this.impl) return this.impl.decryptString(b.subarray(2));
    } catch {}
    return null;
  },
  remove(name) { fs.rmSync(this.file(name), { force: true }); },
};

const https = url => (!url ? url : url.startsWith('//') ? 'https:' + url : url.replace(/^http:\/\//, 'https://'));
const ms = iso => { const t = Date.parse(iso || ''); return Number.isFinite(t) ? t : 0; };

/** First JSON value in text that may have log lines around it (legendary prints both). */
function jsonIn(text, open = '[') {
  // Log lines like "[cli] INFO: …" also start with a bracket: try each candidate until one parses.
  for (let i = text.indexOf(open); i >= 0; i = text.indexOf(open, i + 1)) {
    try { return JSON.parse(text.slice(i)); } catch { /* keep looking */ }
  }
  return null;
}

module.exports = { OwnedCache, secure, https, ms, jsonIn, CACHE_DIR };
