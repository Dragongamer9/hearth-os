'use strict';
// Updates from GitHub Releases, same as Windows. A release carries Nimbus-linux-x86_64.AppImage (and .sha256).
// Running as an AppImage, Nimbus downloads the new one next to itself, checks it, swaps the file and restarts.
// The previous AppImage is kept as <name>.old and restored by the session script if the new one keeps crashing.
// Running from source (npm start) it only reports what's available.

const fs = require('node:fs');
const crypto = require('node:crypto');
const { EventEmitter } = require('node:events');
const { log } = require('./util');

const REPO = 'Dragongamer9/nimbus';
const ASSET = /^Nimbus-linux-x86_64\.AppImage$/;

// Versions look like 0.4.0 (a full release) or 0.4.0-linux.13 (preview 13 on the way to the next release).
// A full release is newer than every preview of the same version, and older than previews built after it
// (which carry its version plus a preview number).
function parseVersion(v) {
  const m = /^v?(\d+(?:\.\d+)*)(?:-linux\.(\d+))?/.exec(String(v || ''));
  if (!m) return { parts: [0], preview: null };
  return { parts: m[1].split('.').map(Number), preview: m[2] != null ? +m[2] : null };
}
const newer = (a, b) => {
  const A = parseVersion(a), B = parseVersion(b);
  for (let i = 0; i < Math.max(A.parts.length, B.parts.length); i++) {
    if ((A.parts[i] || 0) !== (B.parts[i] || 0)) return (A.parts[i] || 0) > (B.parts[i] || 0);
  }
  if (A.preview === B.preview) return false;
  // Same base version: a preview is built on top of the release with that number, so it counts as newer.
  if (A.preview == null) return false;
  if (B.preview == null) return true;
  return A.preview > B.preview;
};
const isPreview = v => parseVersion(v).preview != null;

// Nimbus was renumbered to 0.0.x at 0.0.5. Releases from before that (v0.3.0 to v0.5.x, including Linux
// previews and the v0.4.0-linux.100 bridge that carried 0.0.5 to existing installs) use the old numbers,
// which look newer than 0.0.x; ignore them so they're never offered as updates.
const LEGACY_TAG = /^v?0\.[3-5]\./;

class Updates extends EventEmitter {
  constructor({ version, appImage = process.env.APPIMAGE, autoCheck = () => true } = {}) {
    super();
    this.current = version;
    this.appImage = appImage || null;
    this.autoCheck = autoCheck;
    this.state = 'idle';
    this.latest = null;
    this.progress = null;
    this.error = null;
    this.lastChecked = 0;
    this.ready = null; // path of the verified download
  }

  status() {
    return { current: this.current, state: this.state, latest: this.latest, progress: this.progress, error: this.error,
      canInstallHere: !!this.appImage, installFolder: this.appImage || 'the Nimbus AppImage', autoCheck: this.autoCheck(),
      lastChecked: this.lastChecked };
  }

  set(state, extra = {}) { Object.assign(this, { state, ...extra }); this.emit('changed', this.status()); }

  async check() {
    if (['downloading', 'ready'].includes(this.state)) return this.status();
    this.set('checking', { error: null });
    try {
      // Every recent release that has a Linux AppImage. Preview builds also take newer previews;
      // full releases only take full releases.
      const r = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=30`,
        { headers: { accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(20000) });
      if (!r.ok) throw new Error(`GitHub answered ${r.status}`);
      const candidates = (await r.json())
        .filter(x => !x.draft && !LEGACY_TAG.test(x.tag_name) && (x.assets || []).some(a => ASSET.test(a.name)))
        .filter(x => isPreview(this.current) || !x.prerelease);
      const rel = candidates.reduce((best, x) => (!best || newer(x.tag_name, best.tag_name) ? x : best), null);
      if (!rel) { this.latest = null; this.set('upToDate', { lastChecked: Date.now() }); return this.status(); }
      const version = String(rel.tag_name || '').replace(/^v/, '');
      const asset = (rel.assets || []).find(a => ASSET.test(a.name));
      const sha = (rel.assets || []).find(a => asset && a.name === asset.name + '.sha256');
      this.latest = asset ? { version, tag: rel.tag_name, notes: rel.body || null, zipUrl: asset.browser_download_url,
        shaUrl: sha ? sha.browser_download_url : null, size: asset.size, published: rel.published_at } : null;
      this.set(this.latest && newer(version, this.current) ? 'available' : 'upToDate', { lastChecked: Date.now() });
    } catch (e) {
      this.set('error', { error: "Couldn't check for updates: " + e.message, lastChecked: Date.now() });
    }
    return this.status();
  }

  async download() {
    if (!this.appImage || !this.latest || this.state !== 'available') return this.status();
    const target = this.appImage + '.new';
    this.set('downloading', { progress: 0 });
    try {
      const r = await fetch(this.latest.zipUrl);
      if (!r.ok || !r.body) throw new Error(`download failed (${r.status})`);
      const total = +r.headers.get('content-length') || this.latest.size || 0;
      const out = fs.createWriteStream(target, { mode: 0o755 });
      const hash = crypto.createHash('sha256');
      let done = 0, lastEmit = 0;
      for await (const chunk of r.body) {
        hash.update(chunk);
        if (!out.write(chunk)) await new Promise(res => out.once('drain', res));
        done += chunk.length;
        if (total && Date.now() - lastEmit > 500) { lastEmit = Date.now(); this.set('downloading', { progress: done / total }); }
      }
      await new Promise((res, rej) => out.end(err => (err ? rej(err) : res())));
      if (this.latest.shaUrl) {
        const want = (await (await fetch(this.latest.shaUrl)).text()).trim().split(/\s+/)[0].toLowerCase();
        if (want && want !== hash.digest('hex')) throw new Error('the download is damaged (checksum mismatch)');
      }
      this.ready = target;
      this.set('ready', { progress: 1 });
    } catch (e) {
      try { fs.unlinkSync(target); } catch {}
      this.set('error', { error: 'Update failed: ' + e.message, progress: null });
    }
    return this.status();
  }

  /** Swaps the AppImage (keeping the old one as .old). The caller restarts Nimbus afterwards. */
  install() {
    if (!this.ready || !this.appImage) throw new Error('No update is ready');
    fs.copyFileSync(this.appImage, this.appImage + '.old');
    fs.renameSync(this.ready, this.appImage);
    log(`update: installed ${this.latest && this.latest.version}`);
  }
}

module.exports = { Updates, newer, parseVersion, LEGACY_TAG };
