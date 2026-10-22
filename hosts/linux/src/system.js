'use strict';
// System settings on Linux, through the standard tools a SteamOS-like distro ships:
//   Wi-Fi: nmcli (NetworkManager)       Bluetooth: bluetoothctl (BlueZ)     Radios: rfkill / nmcli
//   Audio: wpctl (PipeWire)             Brightness: brightnessctl          Power modes: powerprofilesctl
//   Battery: /sys/class/power_supply
// Each one reports itself unavailable instead of failing when its tool is missing.

const fs = require('node:fs');
const path = require('node:path');
const { EventEmitter } = require('node:events');
const { run, has, readText } = require('./util');

// ------------------------------------------------------------------ helpers

/** Splits one line of `nmcli -t` output: fields are ':'-separated, with '\:' and '\\' escaped. */
function nmFields(line) {
  const out = [];
  let cur = '';
  for (let i = 0; i < line.length; i++) {
    if (line[i] === '\\' && i + 1 < line.length) { cur += line[++i]; continue; }
    if (line[i] === ':') { out.push(cur); cur = ''; continue; }
    cur += line[i];
  }
  out.push(cur);
  return out;
}
const nmLines = text => text.split('\n').filter(Boolean).map(nmFields);
const bars = signal => (signal >= 75 ? 4 : signal >= 50 ? 3 : signal >= 25 ? 2 : signal > 0 ? 1 : 0);

// ------------------------------------------------------------------ Wi-Fi

class Wifi {
  get available() { return has('nmcli'); }

  async device() {
    const rows = nmLines(await run('nmcli', ['-t', '-f', 'DEVICE,TYPE,STATE', 'device']));
    const w = rows.find(r => r[1] === 'wifi');
    return w ? { name: w[0], state: w[2] } : null;
  }

  async radioOn() {
    if (!this.available) return null;
    if (!(await this.device())) return null;
    return (await run('nmcli', ['radio', 'wifi'])).trim() === 'enabled';
  }

  async setRadio(on) { await run('nmcli', ['radio', 'wifi', on ? 'on' : 'off']); }

  async current() {
    const rows = nmLines(await run('nmcli', ['-t', '-f', 'ACTIVE,SSID,SIGNAL', 'device', 'wifi', 'list', '--rescan', 'no']));
    const a = rows.find(r => r[0] === 'yes');
    return a ? { ssid: a[1] || null, bars: bars(+a[2]) } : { ssid: null, bars: 0 };
  }

  /** The saved Wi-Fi connections: Map of network name (the connection's own SSID, so a renamed connection still
   *  counts) to its connections' UUIDs. */
  async saved() {
    const rows = nmLines(await run('nmcli', ['-t', '-f', 'NAME,UUID,TYPE', 'connection', 'show']));
    const out = new Map();
    for (const [name, uuid, type] of rows) {
      if (!/wireless/.test(type || '') || !uuid) continue;
      let ssid = '';
      // -g escapes '\\' and ':' like -t does.
      try { ssid = (await run('nmcli', ['--escape', 'yes', '-g', '802-11-wireless.ssid', 'connection', 'show', 'uuid', uuid])).replace(/[\r\n]+$/, '').replace(/\\(.)/g, '$1'); } catch { /* the name stands in */ }
      ssid = ssid || name;
      out.set(ssid, [...(out.get(ssid) || []), uuid]);
    }
    return out;
  }

  async scan() {
    if (!(await this.radioOn())) throw new Error('Wi-Fi is off');
    const rows = nmLines(await run('nmcli', ['-t', '-f', 'IN-USE,SSID,SIGNAL,SECURITY', 'device', 'wifi', 'list', '--rescan', 'yes'], { timeout: 30000 }));
    const saved = await this.saved();
    const best = new Map();
    for (const [inUse, ssid, signal, security] of rows) {
      if (!ssid) continue; // hidden networks
      const n = { ssid, bars: bars(+signal), secure: !!security && security !== '--', saved: saved.has(ssid), connected: inUse === '*' };
      const prev = best.get(ssid);
      if (!prev || n.connected || (!prev.connected && n.bars > prev.bars)) best.set(ssid, n);
    }
    return [...best.values()].sort((a, b) => b.connected - a.connected || b.saved - a.saved || b.bars - a.bars);
  }

  async connect(ssid, password) {
    try {
      const saved = await this.saved();
      if (saved.has(ssid) && !password) await run('nmcli', ['connection', 'up', 'uuid', saved.get(ssid)[0]], { timeout: 45000 });
      else await run('nmcli', ['device', 'wifi', 'connect', ssid, ...(password ? ['password', password] : [])], { timeout: 45000 });
      return { ok: true };
    } catch (e) {
      const msg = /secrets were required|802-1x|psk|password/i.test(e.message) ? 'The password is incorrect' : e.message;
      return { ok: false, error: msg };
    }
  }

  async disconnect() { const d = await this.device(); if (d) await run('nmcli', ['device', 'disconnect', d.name]); }
  async forget(ssid) {
    const uuids = (await this.saved()).get(ssid);
    if (!uuids) return run('nmcli', ['connection', 'delete', 'id', ssid]);
    for (const u of uuids) await run('nmcli', ['connection', 'delete', 'uuid', u]);
  }
}

// ------------------------------------------------------------------ Bluetooth

const BT_KIND = { 'input-gaming': 'Input.Gamepad', 'input-keyboard': 'Input.Keyboard', 'input-mouse': 'Input.Mouse',
  'audio-headset': 'Audio.Headphone', 'audio-headphones': 'Audio.Headphone', 'audio-card': 'Audio.Speaker',
  phone: 'Phone', computer: 'Computer', 'video-display': 'Display.Television' };

class Bluetooth extends EventEmitter {
  get available() { return has('bluetoothctl'); }

  async info(mac) {
    const text = await run('bluetoothctl', ['info', mac]);
    const val = k => { const m = new RegExp(`^\\s*${k}: (.*)$`, 'm').exec(text); return m ? m[1].trim() : ''; };
    return { id: mac, name: val('Alias') || val('Name') || mac, kind: BT_KIND[val('Icon')] || 'Other',
      connected: val('Connected') === 'yes', paired: val('Paired') === 'yes' };
  }

  async powered() {
    if (!this.available) return null;
    const text = await run('bluetoothctl', ['show']).catch(() => '');
    if (!/Controller/.test(text)) return null; // no adapter
    return /Powered: yes/.test(text);
  }

  async setPower(on) {
    if (on && has('rfkill')) await run('rfkill', ['unblock', 'bluetooth']).catch(() => {});
    await run('bluetoothctl', ['power', on ? 'on' : 'off']);
  }

  async devices(filter) {
    const text = await run('bluetoothctl', ['devices', ...(filter ? [filter] : [])]).catch(() => '');
    return [...text.matchAll(/^Device ([0-9A-F:]{17}) /gm)].map(m => m[1]);
  }

  async paired() {
    const macs = await this.devices('Paired');
    return Promise.all(macs.map(m => this.info(m)));
  }

  /** Scans for `seconds`, reporting each new device as it appears ('found'), then 'scanDone'. */
  startScan(seconds = 15) {
    this.stopScan();
    const before = new Set();
    let stopped = false;
    this.devices().then(list => list.forEach(m => before.add(m)));
    const scanner = run('bluetoothctl', ['--timeout', String(seconds), 'scan', 'on'], { timeout: (seconds + 5) * 1000 }).catch(() => {});
    const poll = setInterval(async () => {
      for (const mac of await this.devices()) {
        if (before.has(mac) || stopped) continue;
        before.add(mac);
        const d = await this.info(mac).catch(() => null);
        if (d && !d.paired && d.name !== mac) this.emit('found', d);
      }
    }, 1500);
    this._scan = { stop: () => { stopped = true; clearInterval(poll); } };
    scanner.then(() => { if (!stopped) { this.stopScan(); } });
  }

  stopScan() {
    if (!this._scan) return;
    this._scan.stop();
    this._scan = null;
    run('bluetoothctl', ['scan', 'off']).catch(() => {});
    this.emit('scanDone');
  }

  async pair(mac) {
    try {
      await run('bluetoothctl', ['pair', mac], { timeout: 40000 });
      await run('bluetoothctl', ['trust', mac]).catch(() => {});
      await run('bluetoothctl', ['connect', mac], { timeout: 20000 }).catch(() => {});
      return { ok: true };
    } catch (e) { return { ok: false, error: e.message }; }
  }

  async unpair(mac) {
    try { await run('bluetoothctl', ['remove', mac]); return { ok: true }; }
    catch (e) { return { ok: false, error: e.message }; }
  }

  /**
   * "Connect a controller" (first screen on the live USB and at first start): with nothing but the controller,
   * Hearth has to pair it by itself. Bluetooth goes on, scanning keeps going, and any controller in pairing
   * mode that shows up is paired, trusted and connected. Events: 'pad' {state: 'pairing'|'paired'|'failed', name}.
   */
  autoPairPads(on) {
    clearInterval(this._auto && this._auto.poll);
    if (!on) { if (this._auto) run('bluetoothctl', ['scan', 'off']).catch(() => {}); this._auto = null; return; }
    if (!this.available) return;
    const auto = this._auto = { busy: false, tried: new Map() };
    const scan = () => { if (this._auto === auto) run('bluetoothctl', ['--timeout', '120', 'scan', 'on'], { timeout: 125000 }).catch(() => {}).then(() => { if (this._auto === auto) scan(); }); };
    this.setPower(true).catch(() => {}).then(scan);
    auto.poll = setInterval(async () => {
      if (auto.busy || this._auto !== auto) return;
      for (const mac of await this.devices()) {
        const last = auto.tried.get(mac);
        if (last && Date.now() - last < 20000) continue;
        const d = await this.info(mac).catch(() => null);
        if (!d || d.paired || !isGamepad(d)) continue;
        auto.busy = true; auto.tried.set(mac, Date.now());
        this.emit('pad', { state: 'pairing', name: d.name });
        const r = await this.pair(mac);
        this.emit('pad', { state: r.ok ? 'paired' : 'failed', name: d.name });
        auto.busy = false;
        break;
      }
    }, 2000);
  }
}

/** Looks like a game controller: Bluetooth says so, or its name does. */
function isGamepad(d) {
  return d.kind === 'Input.Gamepad' || /controller|gamepad|xbox|dualsense|dualshock|wireless controller|pro controller|joy-?con|8bitdo|stadia|gamesir|flydigi|nimbus|razer kishi|backbone/i.test(d.name || '');
}

// ------------------------------------------------------------------ Audio (PipeWire)

class Audio {
  get available() { return has('wpctl'); }

  async volume() {
    const out = await run('wpctl', ['get-volume', '@DEFAULT_AUDIO_SINK@']);
    const m = /Volume:\s*([\d.]+)/.exec(out);
    return { volume: m ? Math.round(parseFloat(m[1]) * 100) : 0, muted: /MUTED/.test(out) };
  }

  /** Output devices from `wpctl status` (the Sinks list under Audio). The default is marked with '*'. */
  async devices() {
    const text = await run('wpctl', ['status']);
    const lines = text.split('\n');
    const out = [];
    let inAudio = false, inSinks = false;
    for (const line of lines) {
      if (/^Audio/.test(line)) { inAudio = true; continue; }
      if (/^(Video|Settings)/.test(line)) inAudio = false;
      if (!inAudio) continue;
      if (/Sinks:/.test(line)) { inSinks = true; continue; }
      if (/(Sources|Filters|Streams|Sink endpoints|Devices):/.test(line)) { inSinks = false; continue; }
      if (!inSinks) continue;
      const m = /(\*)?\s+(\d+)\.\s+(.+?)(?:\s+\[vol:.*\])?\s*$/.exec(line.replace(/[│├└─]/g, ' '));
      if (m) out.push({ id: m[2], name: m[3].trim(), isDefault: !!m[1] });
    }
    return out;
  }

  async get() { return { ...(await this.volume()), devices: await this.devices().catch(() => []) }; }
  /** max: 100, or 150 with over-amplification on (Settings › Audio). */
  async setVolume(v, max = 100) { await run('wpctl', ['set-volume', '@DEFAULT_AUDIO_SINK@', (Math.max(0, Math.min(max, v)) / 100).toFixed(2)]); }
  /** The volume buttons: 5% steps, unmuting, never past max. Returns the new {volume, muted}. */
  async step(d, max = 100) {
    const cur = await this.volume();
    const v = Math.max(0, Math.min(max, Math.round((cur.volume + d * 5) / 5) * 5));
    await this.setVolume(v, max);
    if (cur.muted && d > 0) await this.setMute(false);
    return { volume: v, muted: cur.muted && d <= 0 };
  }
  async toggleMute() { const cur = await this.volume(); await this.setMute(!cur.muted); return { volume: cur.volume, muted: !cur.muted }; }
  async setMute(m) { await run('wpctl', ['set-mute', '@DEFAULT_AUDIO_SINK@', m ? '1' : '0']); }
  async setDefault(id) { await run('wpctl', ['set-default', String(id)]); }
}

// ------------------------------------------------------------------ Display brightness

class Display {
  get available() { return has('brightnessctl'); }

  /** Brightness 0–100 of the first backlight, or null if there isn't one (desktops, most TVs). */
  async brightness() {
    if (!this.available) return null;
    const out = await run('brightnessctl', ['-m', '-c', 'backlight', 'info']).catch(() => '');
    const m = /,(\d+)%,/.exec(out);
    return m ? +m[1] : null;
  }
  async setBrightness(v) { await run('brightnessctl', ['-c', 'backlight', 'set', `${Math.max(1, Math.min(100, v))}%`]); }
}

// ------------------------------------------------------------------ Power


class Power {
  constructor(root = '/sys/class/power_supply') { this.root = root; }

  battery() {
    let names = [];
    try { names = fs.readdirSync(this.root); } catch {}
    for (const n of names) {
      const dir = path.join(this.root, n);
      if ((readText(path.join(dir, 'type')) || '').trim() !== 'Battery') continue;
      if ((readText(path.join(dir, 'scope')) || '').trim() === 'Device') continue; // controller/mouse batteries
      const cap = (readText(path.join(dir, 'capacity')) || '').trim();
      const pct = +cap;
      const status = (readText(path.join(dir, 'status')) || '').trim();
      if (!cap || !Number.isFinite(pct)) continue;
      // "Not charging": plugged in but held below full (a charge limit, like the ROG Ally's 80 %), not "on battery".
      return { present: true, pct, charging: status === 'Charging' || status === 'Full' || status === 'Not charging' };
    }
    return { present: false, pct: 0, charging: false };
  }

  // Power mode through hearth-power (power-profiles D-Bus / tuned on Bazzite, powerprofilesctl, or SteamOS Manager).
  async mode() { return (await require('./power-control').status()).mode.mode; }
  async setMode(mode) { await require('./power-control').setMode(mode); }

  async get() {
    const st = await require('./power-control').status();
    return { ...this.battery(), mode: st.mode.mode, modes: st.mode.available, modeSupported: st.mode.supported, tdp: st.tdp };
  }
}

// ------------------------------------------------------------------ Radios / airplane mode

/** Airplane mode: rfkill blocks every radio when available, otherwise Wi-Fi and Bluetooth one by one. */
async function setAirplane(on, wifi, bt) {
  if (has('rfkill')) { await run('rfkill', [on ? 'block' : 'unblock', 'all']); return; }
  if (wifi.available) await wifi.setRadio(!on).catch(() => {});
  if (bt.available) await bt.setPower(!on).catch(() => {});
}

async function airplaneOn() {
  if (!has('rfkill')) return false;
  const out = await run('rfkill', ['-n', '-o', 'SOFT']).catch(() => '');
  const rows = out.split('\n').map(s => s.trim()).filter(Boolean);
  return rows.length > 0 && rows.every(s => s === 'blocked');
}

module.exports = { Wifi, Bluetooth, Audio, Display, Power, setAirplane, airplaneOn, nmFields, bars, isGamepad };
