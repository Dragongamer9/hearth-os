'use strict';
// Hearth's own installer, the Hearth app's side: talks to /usr/libexec/nimbus/hearth-install (the engine,
// shared with Mantel's install window) through pkexec. Only on the live USB.
//   disks()       the drives Hearth can go on
//   check(part)   next to Windows (no part) or the Linux on `part` (SteamOS, ...): how much room it can give,
//                 what's in the way; the result's `target` describes it either way
//   start(plan)   installs; progress comes as 'changed' events ({state, stage, pct, text, ...})
//   restart()     restarts into the installed Hearth

const fs = require('node:fs');
const { spawn } = require('node:child_process');
const { EventEmitter } = require('node:events');

const ENGINE = '/usr/libexec/nimbus/hearth-install';
const PREP = '/usr/libexec/nimbus/hearth-live-prep';

/** Splits a helper's output into its stage lines, result and error. */
function parseLine(line) {
  line = line.trim();
  let m;
  if ((m = line.match(/^stage: (\{.*\})$/))) { try { return { stage: JSON.parse(m[1]) }; } catch { return null; } }
  if ((m = line.match(/^result: (\{.*\})$/))) { try { return { result: JSON.parse(m[1]) }; } catch { return null; } }
  if (line.startsWith('error: ')) return { error: line.slice(7) };
  return null;
}

class OsInstaller extends EventEmitter {
  constructor({ spawnFn = spawn, exists = fs.existsSync, live = () => fs.existsSync('/etc/hearth-live') } = {}) {
    super();
    this.spawn = spawnFn;
    this.exists = exists;
    this.live = live;
    this.state = { state: 'idle', stage: null, pct: 0, text: '', error: null, result: null, log: [] };
  }

  get available() { return this.live() && this.exists(ENGINE); }
  status() { return { available: this.available, ...this.state, log: undefined }; }

  /** Runs a helper with pkexec; resolves its JSON output (the last line that is JSON). */
  helper(cmd, args, input) {
    return new Promise((resolve, reject) => {
      const p = this.spawn('pkexec', [cmd, ...args], { stdio: ['pipe', 'pipe', 'pipe'] });
      let out = '';
      p.stdout.on('data', d => { out += d; });
      p.stderr.on('data', d => { out += d; });
      p.on('error', reject);
      p.on('close', code => {
        const lines = out.split('\n').map(l => l.trim()).filter(Boolean);
        const err = lines.find(l => l.startsWith('error: '));
        if (err) return reject(new Error(err.slice(7)));
        const j = [...lines].reverse().find(l => l.startsWith('{'));
        if (code !== 0 || !j) return reject(new Error(lines.at(-1) || `${cmd} failed`));
        try { resolve(JSON.parse(j)); } catch (e) { reject(e); }
      });
      if (input != null) p.stdin.end(input); else p.stdin.end();
    });
  }

  async disks() {
    if (!this.available) throw new Error('The installer is on the Hearth USB stick.');
    return this.helper(ENGINE, ['disks']);
  }

  async check(part = null) {
    if (!this.available) throw new Error('The installer is on the Hearth USB stick.');
    if (part) {
      if (!/^\/dev\/[\w/-]+$/.test(part)) throw new Error('Unknown partition');
      return this.helper(ENGINE, ['check', part]);
    }
    const r = await this.helper(PREP, ['check']);
    return { ...r, target: r.windows ? { ...r.windows, kind: 'windows', name: 'Windows', part: r.windows.dev } : null };
  }

  set(patch) { Object.assign(this.state, patch); this.emit('changed', this.status()); }

  start(plan) {
    if (!this.available) throw new Error('The installer is on the Hearth USB stick.');
    if (this.state.state === 'running') throw new Error('Hearth is already installing.');
    this.set({ state: 'running', stage: 'start', pct: 0, text: 'Starting', error: null, result: null, log: [], plan: { mode: plan.mode, disk: plan.disk } });
    const p = this.spawn('pkexec', [ENGINE, 'run'], { stdio: ['pipe', 'pipe', 'pipe'] });
    let buf = '';
    const onData = d => {
      buf += d;
      let i;
      while ((i = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, i); buf = buf.slice(i + 1);
        this.state.log = [...this.state.log, line].slice(-60);
        const r = parseLine(line);
        if (!r) continue;
        if (r.stage) this.set({ stage: r.stage.stage, pct: r.stage.pct, text: r.stage.text, left: r.stage.left ?? null, bytes: r.stage.bytes ?? null });
        if (r.result) this.state.result = r.result;
        if (r.error) this.state.error = r.error;
      }
    };
    p.stdout.on('data', onData);
    p.stderr.on('data', onData);
    p.on('error', e => this.set({ state: 'failed', error: e.message }));
    p.on('close', code => {
      if (this.state.state !== 'running') return;
      if (code === 0 && this.state.result) this.set({ state: 'done', pct: 100 });
      else this.set({ state: 'failed', error: this.state.error || (code === 126 || code === 127 ? 'Permission to install was refused.' : 'The install stopped.') });
    });
    p.stdin.end(JSON.stringify(plan));
    return this.status();
  }

  restart() {
    spawn('systemctl', ['reboot'], { detached: true, stdio: 'ignore' }).unref();
    return true;
  }
}

module.exports = { OsInstaller, parseLine };
