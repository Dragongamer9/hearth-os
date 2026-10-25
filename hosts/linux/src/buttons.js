'use strict';
// The power and volume buttons, and controller remapping (Hearth, Linux).
//   ButtonPrefs  what the power button does, and whether volume can go past 100% (~/.config/nimbus/buttons.json,
//                shared with Desktop Mode's Hearth Settings)
//   Buttons      runs desktop/shell/hearth-buttons, which reads the buttons from the kernel, and passes on presses
//   Controller   button remaps / controller type through InputPlumber (desktop/shell/hearth-controller), with
//                the settings in ~/.config/nimbus/controller.json (also shared with Desktop Mode)

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn, execFile } = require('node:child_process');
const { EventEmitter } = require('node:events');
const { log } = require('./util');

const confDir = () => path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'nimbus');
const POWER_ACTIONS = ['sleep', 'menu', 'shutdown', 'nothing'];
const PREF_DEFAULTS = { press: 'sleep', hold: 'menu', overamp: false };

function readJson(file) { try { return JSON.parse(fs.readFileSync(file, 'utf8')); } catch { return null; } }
function writeJson(file, obj) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const tmp = file + '.tmp';
  fs.writeFileSync(tmp, JSON.stringify(obj, null, 2) + '\n');
  fs.renameSync(tmp, file);
}

class ButtonPrefs {
  constructor(file = path.join(confDir(), 'buttons.json')) { this.file = file; }
  get() {
    const o = { ...PREF_DEFAULTS, ...(readJson(this.file) || {}) };
    return {
      press: POWER_ACTIONS.includes(o.press) ? o.press : PREF_DEFAULTS.press,
      hold: POWER_ACTIONS.includes(o.hold) ? o.hold : PREF_DEFAULTS.hold,
      overamp: o.overamp === true,
    };
  }
  set(patch) {
    const cur = this.get();
    if ('press' in patch && POWER_ACTIONS.includes(patch.press)) cur.press = patch.press;
    if ('hold' in patch && POWER_ACTIONS.includes(patch.hold)) cur.hold = patch.hold;
    if ('overamp' in patch) cur.overamp = !!patch.overamp;
    writeJson(this.file, cur);
    return cur;
  }
  /** Highest volume allowed, in percent. */
  maxVolume() { return this.get().overamp ? 150 : 100; }
}

class Buttons extends EventEmitter {
  constructor(helper) {
    super();
    this.helper = helper;
    this.devices = [];
    this.problems = [];
    this.proc = null;
    this.stopped = false;
  }
  start() {
    if (!fs.existsSync(this.helper)) { this.problems = ['hearth-buttons is missing']; return; }
    this.stopped = false;
    const p = spawn('python3', [this.helper], { stdio: ['ignore', 'pipe', 'pipe'] });
    this.proc = p;
    let buf = '';
    p.stdout.on('data', d => {
      buf += d;
      let i;
      while ((i = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, i); buf = buf.slice(i + 1);
        let m; try { m = JSON.parse(line); } catch { continue; }
        this.onMessage(m);
      }
    });
    p.stderr.on('data', d => log('hearth-buttons: ' + String(d).trim()));
    p.on('error', e => { this.problems = [e.message]; });
    p.on('exit', code => {
      this.proc = null;
      if (this.stopped || code === 2) return;          // 2: python3-evdev missing, retrying won't help
      setTimeout(() => { if (!this.stopped) this.start(); }, 3000);
    });
  }
  onMessage(m) {
    if (m.t === 'devices') { this.devices = m.list || []; this.problems = m.problems || []; log(`buttons: watching ${this.devices.join(', ') || 'nothing'}`); }
    else if (m.t === 'ignored') log(`power button: press ignored ${m.after} s after waking up (the press that woke it)`);
    else if (m.t === 'power') this.emit('power', m.press === 'long' ? 'long' : 'short');
    else if (m.t === 'vol') this.emit('volume', m.d > 0 ? 1 : -1);
    else if (m.t === 'mute') this.emit('mute');
  }
  status() { return { devices: this.devices, problems: this.problems }; }
  stop() { this.stopped = true; if (this.proc) this.proc.kill(); }
}

// ------------------------------------------------------------------ controller remapping (InputPlumber)

// The lists (buttons, what they can become, controller types) and the checks live in hearth-controller, which
// Desktop Mode's Hearth Settings uses too.
class Controller {
  constructor(helper) { this.helper = helper; }
  get available() { return fs.existsSync(this.helper); }
  run(args, timeout = 30000) {
    return new Promise((resolve, reject) => {
      if (!this.available) return reject(new Error('Controller remapping needs Hearth, the OS.'));
      execFile('python3', [this.helper, ...args], { timeout }, (err, stdout) => {
        const out = String(stdout || '').trim().split('\n').pop() || '';
        if (out.startsWith('error: ')) return reject(new Error(out.slice(7)));
        if (err) return reject(new Error(err.killed ? 'InputPlumber took too long' : err.message));
        try { resolve(JSON.parse(out)); } catch { resolve(null); }
      });
    });
  }
  status() { return this.run(['status']); }
  /** One button ("" = back to default), and/or the controller type. */
  async set({ button, to, target } = {}) {
    let r = null;
    if (button !== undefined) r = await this.run(['set', String(button), String(to || '')]);
    if (target !== undefined) r = await this.run(['type', String(target || '')]);
    return r || this.status();
  }
  preset(name) { return this.run(['preset', String(name)]); }
  reset() { return this.run(['reset']); }
}

module.exports = { ButtonPrefs, Buttons, Controller, POWER_ACTIONS };
