'use strict';
// Sign-in options and system updates for Settings › Sign-in and Settings › Updates.
// The parts that need root go through /usr/libexec/nimbus/hearth-admin (Hearth, the OS) with pkexec; a
// polkit rule lets the person at the device run it without a password prompt. Elsewhere only the start
// mode (Game or Desktop Mode, ~/.config/nimbus/session.conf) is available.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');
const { EventEmitter } = require('node:events');

const HELPER = '/usr/libexec/nimbus/hearth-admin';
const NO_AUTOLOGIN = '/etc/nimbus/no-autologin';

const confDir = () => path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'nimbus');

/** Reads NIMBUS_START_MODE from session.conf (shell syntax). */
function readStartMode(file) {
  try {
    const m = fs.readFileSync(file, 'utf8').match(/^\s*NIMBUS_START_MODE=["']?(game|desktop)["']?\s*$/m);
    return m ? m[1] : 'game';
  } catch { return 'game'; }
}

/** Sets NIMBUS_START_MODE in session.conf, keeping everything else in the file. */
function writeStartMode(file, mode) {
  let text = '';
  try { text = fs.readFileSync(file, 'utf8'); } catch {}
  const line = `NIMBUS_START_MODE=${mode}`;
  if (/^\s*NIMBUS_START_MODE=.*$/m.test(text)) text = text.replace(/^\s*NIMBUS_START_MODE=.*$/m, line);
  else text = (text && !text.endsWith('\n') ? text + '\n' : text) + line + '\n';
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
}

/** The JSON line and progress lines hearth-admin prints. */
function parseHelperOutput(out) {
  const lines = out.split('\n').map(l => l.trim()).filter(Boolean);
  const err = lines.find(l => l.startsWith('error: '));
  if (err) return { error: err.slice(7) };
  for (let i = lines.length - 1; i >= 0; i--) {
    if (lines[i].startsWith('{')) { try { return { result: JSON.parse(lines[i]) }; } catch {} }
  }
  return { result: null };
}

class SystemAdmin extends EventEmitter {
  constructor({ helper = HELPER, sessionConf = path.join(confDir(), 'session.conf'), exists = fs.existsSync, spawnFn = spawn } = {}) {
    super();
    this.helper = helper;
    this.sessionConf = sessionConf;
    this.exists = exists;
    this.spawn = spawnFn;
    this.update = { state: 'idle', line: '', progress: null, status: null, error: null, stage: null, phases: null, speeds: [] };
  }

  get canAdmin() { return this.exists(this.helper); }

  /** Runs hearth-admin with pkexec. onLine gets each "log:" line, onProgress each "progress: <pct> <what>",
   *  onStage each "stage: {json}" (detailed update progress). */
  run(args, { input = null, onLine = null, onProgress = null, onStage = null, onSpawn = null, timeout = 30 * 60 * 1000 } = {}) {
    if (!this.canAdmin) return Promise.reject(new Error('This needs Hearth, the OS.'));
    return new Promise((resolve, reject) => {
      const p = this.spawn('pkexec', [this.helper, ...args], { stdio: ['pipe', 'pipe', 'pipe'] });
      if (onSpawn) onSpawn(p);
      let out = '', errText = '', buf = '';
      const timer = setTimeout(() => { p.kill('SIGTERM'); reject(new Error('Timed out')); }, timeout);
      p.stdout.on('data', d => {
        out += d; buf += d;
        let i;
        while ((i = buf.indexOf('\n')) >= 0) {
          const line = buf.slice(0, i).trim(); buf = buf.slice(i + 1);
          if (onLine && line.startsWith('log: ')) onLine(line.slice(5));
          const m = onProgress && line.match(/^progress: ([\d.]+) ?(.*)$/);
          if (m) onProgress(+m[1] / 100, m[2]);
          if (onStage && line.startsWith('stage: ')) { try { onStage(JSON.parse(line.slice(7))); } catch {} }
        }
      });
      p.stderr.on('data', d => { errText += d; });
      p.on('error', e => { clearTimeout(timer); reject(e); });
      p.on('close', code => {
        clearTimeout(timer);
        const { error, result } = parseHelperOutput(out);
        if (error) return reject(new Error(error));
        if (code === 126 || code === 127) return reject(new Error('Not allowed (the polkit rule for Hearth is missing).'));
        if (code !== 0) return reject(new Error(errText.trim().split('\n').pop() || `hearth-admin exited with ${code}`));
        resolve(result);
      });
      if (input != null) p.stdin.end(input); else p.stdin.end();
    });
  }

  // ------------------------------------------------------------------ sign-in
  login() {
    return {
      canAdmin: this.canAdmin,
      autologin: this.canAdmin ? !this.exists(NO_AUTOLOGIN) : null,
      startMode: readStartMode(this.sessionConf),
      hostname: os.hostname(),
      user: os.userInfo().username,
    };
  }
  async setAutologin(on) { await this.run(['autologin', on ? 'on' : 'off']); return this.login(); }
  setStartMode(mode) {
    if (!['game', 'desktop'].includes(mode)) throw new Error('Start mode is game or desktop');
    writeStartMode(this.sessionConf, mode);
    return this.login();
  }
  async setHostname(name) { await this.run(['hostname', String(name || '').trim()]); return this.login(); }
  async changePassword(oldPw, newPw) {
    await this.run(['password'], { input: JSON.stringify({ old: String(oldPw), new: String(newPw) }) + '\n' });
    return { ok: true };
  }

  // ------------------------------------------------------------------ system update
  setUpdate(patch) {
    Object.assign(this.update, patch);
    this.emit('update', this.updateStatus());
  }
  updateStatus() { return { canAdmin: this.canAdmin, ...this.update }; }
  async refreshUpdate() {
    if (!this.canAdmin || ['checking', 'updating'].includes(this.update.state)) return this.updateStatus();
    try { this.setUpdate({ status: await this.run(['update-status'], { timeout: 60000 }), error: null }); }
    catch (e) { this.setUpdate({ error: e.message }); }
    return this.updateStatus();
  }
  async checkUpdate() {
    if (['checking', 'updating'].includes(this.update.state)) return this.updateStatus();
    this.setUpdate({ state: 'checking', line: '', error: null });
    try { this.setUpdate({ state: 'idle', status: await this.run(['update-check'], { onLine: l => this.setUpdate({ line: l }), timeout: 5 * 60000 }) }); }
    catch (e) { this.setUpdate({ state: 'idle', error: e.message }); }
    return this.updateStatus();
  }
  async installUpdate() {
    if (['checking', 'updating'].includes(this.update.state)) return this.updateStatus();
    const started = Date.now();
    this.setUpdate({ state: 'updating', line: 'Starting…', progress: 0, error: null, stage: null, speeds: [], started,
      phases: { download: { pct: 0 }, install: { pct: 0 }, apps: { pct: 0 } } });
    let lastPush = 0;
    const push = (patch, force) => {
      // Many events per second while downloading: pass a few on to the page.
      Object.assign(this.update, patch);
      const now = Date.now();
      if (!force && now - lastPush < 500) return;
      lastPush = now;
      this.emit('update', this.updateStatus());
    };
    const onProgress = (p, what) => push({ progress: p, line: what || this.update.line }, p >= 1);
    const onStage = st => {
      const phases = { ...this.update.phases };
      // Reaching a later phase finishes the earlier ones.
      const order = ['download', 'install', 'apps', 'done'];
      const at = order.indexOf(st.phase);
      order.slice(0, Math.max(0, at)).forEach(k => { if (phases[k]) phases[k] = { ...phases[k], pct: 100, finished: true }; });
      if (phases[st.phase]) phases[st.phase] = { ...phases[st.phase], ...st, started: phases[st.phase].started || Date.now() };
      const speeds = st.phase === 'download' && st.bps != null
        ? [...this.update.speeds.filter(([t]) => Date.now() - t < 5 * 60e3), [Date.now(), st.bps]].slice(-400)
        : this.update.speeds;
      push({ stage: st, phases, speeds }, st.pct === 100 || st.phase !== (this.update.stage || {}).phase);
    };
    try {
      const status = await this.run(['update'], { onLine: l => push({ line: l }, true), onProgress, onStage });
      this.setUpdate({ state: 'idle', line: '', progress: null, status, lastRun: { ok: true, at: Date.now(), took: Date.now() - started } });
    } catch (e) {
      this.setUpdate({ state: 'idle', progress: null, error: e.message, lastRun: { ok: false, at: Date.now() } });
    }
    return this.updateStatus();
  }
}

module.exports = { SystemAdmin, readStartMode, writeStartMode, parseHelperOutput };
