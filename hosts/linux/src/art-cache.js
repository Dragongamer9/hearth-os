'use strict';
// Game art (and store profile pictures) kept on the device, so the library looks the same offline.
// Art that comes from the internet (Epic, GOG, Xbox, Steam's CDN for games Steam hasn't cached) is downloaded
// once into ~/.local/share/nimbus/art-cache/ and served from there as nimbus-art://cache/<name>. Until a
// picture is downloaded the UI uses its web address as before; downloads run quietly in the background, a few
// at a time, and start again whenever the device is back online.

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { EventEmitter } = require('node:events');
const { DATA_DIR, log } = require('./util');

const EXTS = ['.jpg', '.png', '.webp', '.jpeg', '.gif', '.avif'];
const TYPES = { 'image/jpeg': '.jpg', 'image/png': '.png', 'image/webp': '.webp', 'image/gif': '.gif', 'image/avif': '.avif' };
const isRemote = u => typeof u === 'string' && /^https?:\/\//i.test(u);

class ArtCache extends EventEmitter {
  constructor(dir = path.join(DATA_DIR, 'art-cache'), { fetchFn = (...a) => fetch(...a), concurrency = 3 } = {}) {
    super();
    this.dir = dir;
    this.fetch = fetchFn;
    this.concurrency = concurrency;
    this.queue = [];
    this.queued = new Set();
    this.failed = new Map();   // url → time it last failed (tried again after a while)
    this.active = 0;
    this.index = new Map();    // name (hash) → file
    try { fs.mkdirSync(dir, { recursive: true }); } catch {}
    this.scan();
  }

  static nameOf(url) { return crypto.createHash('sha1').update(url).digest('hex').slice(0, 24); }

  scan() {
    this.index.clear();
    let files = [];
    try { files = fs.readdirSync(this.dir); } catch {}
    for (const f of files) {
      const ext = path.extname(f);
      if (EXTS.includes(ext.toLowerCase())) this.index.set(path.basename(f, ext), path.join(this.dir, f));
    }
  }

  /** Local file for a cached picture by its name, or null. */
  file(name) { return /^[0-9a-f]{24}$/.test(name || '') ? this.index.get(name) || null : null; }

  /** What the UI should load for this picture: the cached copy if there is one; otherwise the web address
   *  (and it's queued for download). Local paths and other values pass through. */
  url(u) {
    if (!isRemote(u)) return u;
    const name = ArtCache.nameOf(u);
    if (this.index.has(name)) return `nimbus-art://cache/${name}`;
    this.want(u);
    return u;
  }

  want(u) {
    if (!isRemote(u) || this.queued.has(u)) return;
    const failedAt = this.failed.get(u);
    if (failedAt && Date.now() - failedAt < 6 * 3600 * 1000) return;
    this.queued.add(u);
    this.queue.push(u);
    this.pump();
  }

  /** Try everything that failed again (called when the network comes back). */
  retry() {
    const again = [...this.failed.keys()];
    this.failed.clear();
    for (const u of again) this.want(u);
  }

  pump() {
    while (this.active < this.concurrency && this.queue.length) {
      const u = this.queue.shift();
      this.active++;
      this.download(u).then(ok => {
        if (ok) { this.got = (this.got || 0) + 1; this.changedSoon(); }
      }).finally(() => {
        this.active--;
        this.queued.delete(u);
        this.pump();
      });
    }
  }

  changedSoon() {
    // The library reloads on 'changed': once when the queue empties, and at most once a minute while a big
    // batch is still downloading (more often made scrolling stutter).
    clearTimeout(this.changedTimer);
    const now = Date.now();
    if (this.queue.length && now - (this.lastChanged || 0) < 60000) {
      this.changedTimer = setTimeout(() => this.changedSoon(), 60000 - (now - (this.lastChanged || 0)));
      return;
    }
    this.changedTimer = setTimeout(() => { this.lastChanged = Date.now(); this.emit('changed'); }, this.queue.length ? 0 : 1500);
  }

  async download(u) {
    const name = ArtCache.nameOf(u);
    if (this.index.has(name)) return false;
    const ctl = new AbortController();
    const timer = setTimeout(() => ctl.abort(), 30000);
    try {
      const res = await this.fetch(u, { signal: ctl.signal });
      if (!res.ok) throw new Error('HTTP ' + res.status);
      const type = String(res.headers.get('content-type') || '').split(';')[0].trim().toLowerCase();
      let ext = TYPES[type] || path.extname(new URL(u).pathname).toLowerCase();
      if (!EXTS.includes(ext)) {
        if (type && !type.startsWith('image/')) throw new Error('not a picture: ' + type);
        ext = '.jpg';
      }
      const buf = Buffer.from(await res.arrayBuffer());
      if (buf.length < 64) throw new Error('empty');
      const file = path.join(this.dir, name + ext);
      fs.writeFileSync(file + '.part', buf);
      fs.renameSync(file + '.part', file);
      this.index.set(name, file);
      return true;
    } catch (e) {
      this.failed.set(u, Date.now());
      if (!/abort|fetch failed|ENOTFOUND|EAI_AGAIN|ECONNREFUSED|network/i.test(String(e && (e.cause && e.cause.code || e.message)))) log(`art cache: ${u}: ${e.message}`);
      return false;
    } finally { clearTimeout(timer); }
  }

  status() {
    let bytes = 0;
    for (const f of this.index.values()) { try { bytes += fs.statSync(f).size; } catch {} }
    return { pictures: this.index.size, bytes, waiting: this.queue.length + this.active, failed: this.failed.size };
  }

  clear() {
    for (const f of this.index.values()) { try { fs.rmSync(f, { force: true }); } catch {} }
    this.index.clear();
  }
}

let shared = null;
/** The one cache the app uses. */
function artCache() { if (!shared) shared = new ArtCache(); return shared; }

module.exports = { ArtCache, artCache, isRemote };
