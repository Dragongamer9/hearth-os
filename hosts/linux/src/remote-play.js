'use strict';
// Remote play (Settings › Remote play), two ways:
//  - Stream FROM this device: Sunshine (it comes with Bazzite, so with Hearth OS) runs as your user service; play
//    on a TV, phone or PC with the free Moonlight app. Pairing: Moonlight shows a 4-digit PIN, you type it here.
//    Hearth gives Sunshine a username and a long random password the first time (kept in its data folder,
//    readable only by you), so nobody has to visit Sunshine's web page.
//  - Play ON this device from another PC: the Moonlight app (Flathub), opened or installed from here.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const crypto = require('node:crypto');
const { run, has, DATA_DIR, log, launchDetached } = require('./util');

const UNITS = ['sunshine.service', 'app-dev.lizardbyte.app.Sunshine.service'];
const API = { host: '127.0.0.1', port: 47990 };
const MOONLIGHT = 'com.moonlight_stream.Moonlight';
const credsFile = (dir = DATA_DIR) => path.join(dir, 'remote-play.json');

function readCreds(dir) { try { const c = JSON.parse(fs.readFileSync(credsFile(dir), 'utf8')); return c.user && c.pass ? c : null; } catch { return null; } }
function writeCreds(c, dir) { const f = credsFile(dir); fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, JSON.stringify(c), { mode: 0o600 }); fs.chmodSync(f, 0o600); }

/** One call to Sunshine's own web API on this device (its certificate is its own, made on this device). */
function api(method, p, body, creds, request = https.request) {
  return new Promise((resolve, reject) => {
    const data = body ? JSON.stringify(body) : null;
    const headers = { 'Content-Type': 'application/json' };
    if (data) headers['Content-Length'] = Buffer.byteLength(data);
    if (creds) headers.Authorization = 'Basic ' + Buffer.from(`${creds.user}:${creds.pass}`).toString('base64');
    const req = request({ ...API, method, path: p, headers, rejectUnauthorized: false, timeout: 6000 }, res => {
      let out = ''; res.setEncoding('utf8'); res.on('data', c => { out += c; });
      res.on('end', () => {
        if (res.statusCode === 401) return reject(Object.assign(new Error('Sunshine wants its username and password'), { code: 'AUTH' }));
        if (res.statusCode >= 400) return reject(new Error(`Sunshine said ${res.statusCode}`));
        try { resolve(out ? JSON.parse(out) : {}); } catch { resolve({}); }
      });
    });
    req.on('timeout', () => req.destroy(new Error("Sunshine isn't answering")));
    req.on('error', e => reject(e.code === 'ECONNREFUSED' ? new Error("Sunshine isn't running") : e));
    if (data) req.write(data);
    req.end();
  });
}

class RemotePlay {
  constructor({ dir = DATA_DIR, request } = {}) { this.dir = dir; this.request = request; this.unit = null; }
  call(method, p, body, creds = readCreds(this.dir)) { return api(method, p, body, creds, this.request); }

  async findUnit() {
    if (this.unit) return this.unit;
    for (const u of UNITS) {
      const out = await run('systemctl', ['--user', 'list-unit-files', u, '--no-legend']).catch(() => '');
      if (out.trim()) return (this.unit = u);
    }
    return null;
  }
  async running() { const u = await this.findUnit(); if (!u) return false; return run('systemctl', ['--user', 'is-active', u]).then(o => o.trim() === 'active', () => false); }

  async status() {
    const unit = await this.findUnit();
    const on = await this.running();
    let clients = [], needsLogin = false;
    if (on) {
      try { clients = ((await this.call('GET', '/api/clients/list')).named_certs || []).map(c => ({ name: c.name, uuid: c.uuid })); }
      catch (e) { if (e.code === 'AUTH') needsLogin = true; }
    }
    return { installed: !!unit || has('sunshine'), on, clients, needsLogin, addresses: addresses(),
      moonlight: await moonlightInstalled() };
  }

  async setOn(on) {
    const u = await this.findUnit();
    if (!u) throw new Error("Sunshine isn't on this system. It comes with Hearth OS.");
    await run('systemctl', ['--user', on ? 'enable' : 'disable', '--now', u], { timeout: 30000 });
    if (on) { await waitFor(() => api('GET', '/api/clients/list', null, null, this.request).then(() => true, e => e.code === 'AUTH'), 15000); await this.ensureLogin(); }
    log(`remote play: sunshine ${on ? 'on' : 'off'}`);
    return this.status();
  }

  /** The first time: gives Sunshine a username and password only Hearth knows. */
  async ensureLogin() {
    if (readCreds(this.dir)) return true;
    const c = { user: 'hearth', pass: crypto.randomBytes(18).toString('base64url') };
    try {
      const r = await this.call('POST', '/api/password', { newUsername: c.user, newPassword: c.pass, confirmNewPassword: c.pass }, null);
      if (r.status === false) return false;
      writeCreds(c, this.dir);
      return true;
    } catch { return false; }   // it already has one (set up before Hearth): login() asks for it
  }
  /** Sunshine was set up before, with your own username and password: Hearth keeps them to pair devices. */
  async login(user, pass) {
    const c = { user: String(user || '').trim(), pass: String(pass || '') };
    if (!c.user || !c.pass) throw new Error('Type the username and password');
    await this.call('GET', '/api/clients/list', null, c);
    writeCreds(c, this.dir);
    return true;
  }
  async pair(pin, name) {
    if (!/^\d{4}$/.test(String(pin))) throw new Error('The PIN is the 4 numbers Moonlight shows');
    await this.ensureLogin();
    const r = await this.call('POST', '/api/pin', { pin: String(pin), name: String(name || 'Moonlight').slice(0, 40) });
    if (r.status === false) throw new Error("That PIN didn't work. In Moonlight, pick this device again for a new one.");
    return true;
  }
  async unpair(uuid) { await this.call('POST', '/api/clients/unpair', { uuid: String(uuid) }); return true; }
}

function addresses() {
  const out = [];
  for (const [name, list] of Object.entries(os.networkInterfaces())) {
    if (/^(lo|docker|veth|virbr|br-)/.test(name)) continue;
    for (const a of list || []) if (a.family === 'IPv4' && !a.internal) out.push(a.address);
  }
  return out;
}
async function waitFor(fn, ms) { for (const t = Date.now(); Date.now() - t < ms;) { if (await fn()) return true; await new Promise(r => setTimeout(r, 500)); } return false; }

// ---------------------------------------------------------------- Moonlight (play another PC's games here)
function moonlightInstalled() { return has('flatpak') ? run('flatpak', ['info', MOONLIGHT]).then(() => true, () => false) : Promise.resolve(false); }
function openMoonlight() { launchDetached('flatpak', ['run', MOONLIGHT]); return true; }
async function installMoonlight() {
  if (!has('flatpak')) throw new Error('Flatpak is needed to install Moonlight');
  await run('flatpak', ['install', '--user', '-y', '--noninteractive', 'flathub', MOONLIGHT], { timeout: 15 * 60 * 1000 });
  return true;
}

module.exports = { RemotePlay, api, readCreds, writeCreds, addresses, openMoonlight, installMoonlight, moonlightInstalled };
