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

const newer = (a, b) => {
  const pa = String(a).split('.').map(n => parseInt(n, 10) || 0), pb = String(b).split('.').map(n => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) > (pb[i] || 0);
  return false;
};

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
      const r = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`,
        { headers: { accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(20000) });
      if (r.status === 404) { this.set('upToDate', { lastChecked: Date.now() }); return this.status(); }
      if (!r.ok) throw new Error(`GitHub answered ${r.status}`);
      const rel = await r.json();
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

module.exports = { Updates, newer };
