'use strict';
// Hearth's web browser (Aloha Browser), through desktop/shell/hearth-browser: it downloads Aloha's official
// package on the device, installs it in the home folder and makes it the default browser.

const fs = require('node:fs');
const path = require('node:path');
const { spawn } = require('node:child_process');
const { EventEmitter } = require('node:events');

function helperPath() {
  const c = [process.resourcesPath && path.join(process.resourcesPath, 'desktop', 'shell', 'hearth-browser'),
    path.join(__dirname, '..', 'desktop', 'shell', 'hearth-browser')].filter(Boolean);
  return c.find(p => fs.existsSync(p)) || c[c.length - 1];
}

class Browser extends EventEmitter {
  constructor(helper = helperPath()) {
    super();
    this.helper = helper;
    this.job = { state: 'idle', progress: null, line: '', error: null };
    this.last = null;
  }
  run(args, onProgress) {
    return new Promise((resolve, reject) => {
      const p = spawn('python3', [this.helper, ...args], { stdio: ['ignore', 'pipe', 'pipe'] });
      let buf = '', result = null, err = null;
      p.stdout.on('data', d => {
        buf += d;
        let i;
        while ((i = buf.indexOf('\n')) >= 0) {
          const line = buf.slice(0, i).trim(); buf = buf.slice(i + 1);
          const m = line.match(/^progress: (\d+) ?(.*)$/);
          if (m && onProgress) onProgress(+m[1] / 100, m[2]);
          else if (line.startsWith('error: ')) err = line.slice(7);
          else if (line.startsWith('{')) { try { result = JSON.parse(line); } catch {} }
        }
      });
      p.on('error', e => reject(e));
      p.on('close', code => (err || code ? reject(new Error(err || `hearth-browser exited with ${code}`)) : resolve(result)));
    });
  }
  async status() {
    if (!fs.existsSync(this.helper)) return { available: false, ...this.job };
    if (this.job.state === 'idle') this.last = await this.run(['status']).catch(() => this.last);
    return { available: true, ...(this.last || {}), ...this.job };
  }
  set(patch) { Object.assign(this.job, patch); this.emit('changed', { available: true, ...(this.last || {}), ...this.job }); }
  /** install | update | default | remove; long ones report progress through 'changed'. */
  async act(what) {
    if (!['install', 'update', 'default', 'remove'].includes(what)) throw new Error('Unknown action');
    if (this.job.state !== 'idle') return this.status();
    this.set({ state: what, progress: what === 'install' || what === 'update' ? 0 : null, line: '', error: null });
    try { this.last = await this.run([what], (p, line) => this.set({ progress: p, line })); this.set({ state: 'idle', progress: null, line: '' }); }
    catch (e) { this.set({ state: 'idle', progress: null, line: '', error: e.message }); }
    return this.status();
  }
}

module.exports = { Browser };
