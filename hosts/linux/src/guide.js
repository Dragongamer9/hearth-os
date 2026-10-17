'use strict';
// Watches controllers for the Guide (Xbox / PS / Home) button, so Nimbus can come back over a running game,
// the way the Steam button does on a Steam Deck. The UI itself reads controllers through the browser's
// Gamepad API, but a hidden window gets no gamepad input, so this reads the kernel's event devices directly.
// Joystick event devices are readable by the logged-in user (systemd's uaccess rule), no extra setup needed.
// Handhelds sometimes report the Guide button on a separate input device from the sticks, so any device
// that has the button is watched. Holding View (Back/Select) for a second can open it too (optional).

const fs = require('node:fs');
const path = require('node:path');
const { EventEmitter } = require('node:events');

const EV_KEY = 1;
const BTN_GAMEPAD = 0x130;
const BTN_MODE = 0x13c;
const BTN_SELECT = 0x13a;   // View / Back
const HOLD_MS = 1000;
const EVENT_SIZE = 24; // struct input_event on 64-bit: timeval (16) + type (2) + code (2) + value (4)

/** True if a /sys capabilities bitmap ("key" file: hex words, most significant first) has bit `bit` set. */
function hasBit(capText, bit, wordBits = 64) {
  const words = capText.trim().split(/\s+/).reverse();
  const w = words[Math.floor(bit / wordBits)];
  if (!w) return false;
  return (BigInt('0x' + w) >> BigInt(bit % wordBits)) & 1n ? true : false;
}

/** Event devices with a Guide button: {path, name}. */
function gamepadDevices(sysRoot = '/sys/class/input') {
  let names = [];
  try { names = fs.readdirSync(sysRoot).filter(n => /^event\d+$/.test(n)); } catch {}
  const out = [];
  for (const n of names) {
    try {
      const caps = fs.readFileSync(path.join(sysRoot, n, 'device', 'capabilities', 'key'), 'utf8');
      if (!hasBit(caps, BTN_MODE)) continue;
      let name = n;
      try { name = fs.readFileSync(path.join(sysRoot, n, 'device', 'name'), 'utf8').trim() || n; } catch {}
      out.push({ path: '/dev/input/' + n, name, gamepad: hasBit(caps, BTN_GAMEPAD) });
    } catch { /* no capabilities file */ }
  }
  return out;
}

/** Button changes in a chunk of raw input_event records: [{code, value}] for Guide and View. */
function buttonEvents(buf) {
  const out = [];
  for (let off = 0; off + EVENT_SIZE <= buf.length; off += EVENT_SIZE) {
    const type = buf.readUInt16LE(off + 16), code = buf.readUInt16LE(off + 18), value = buf.readInt32LE(off + 20);
    if (type === EV_KEY && (code === BTN_MODE || code === BTN_SELECT)) out.push({ code, value });
  }
  return out;
}
const guidePresses = buf => buttonEvents(buf).filter(e => e.code === BTN_MODE && e.value === 1).length;

class GuideButton extends EventEmitter {
  constructor() {
    super();
    this.holdView = false;   // also open on View held for a second
    this.open = new Map();   // path → {stream, name}
    this.lastPress = 0;
    this.problems = new Map(); // path → why it can't be read
  }

  start() {
    this.rescan();
    this.timer = setInterval(() => this.rescan(), 5000); // controllers come and go
  }

  /** For Settings › Controls: which devices are watched, and when the Guide button last came through. */
  status() {
    return {
      devices: [...this.open.values()].map(d => d.name),
      problems: [...this.problems.entries()].map(([p, why]) => `${p}: ${why}`),
      lastPress: this.lastPress,
    };
  }

  fire(how) {
    this.lastPress = Date.now();
    this.emit('press', how);
  }

  rescan() {
    for (const dev of gamepadDevices()) {
      if (this.open.has(dev.path)) continue;
      const s = fs.createReadStream(dev.path, { highWaterMark: EVENT_SIZE * 64 });
      let rest = Buffer.alloc(0);
      let viewTimer = null;
      s.on('data', chunk => {
        const buf = Buffer.concat([rest, chunk]);
        const whole = buf.length - (buf.length % EVENT_SIZE);
        for (const e of buttonEvents(buf.subarray(0, whole))) {
          if (e.code === BTN_MODE && e.value === 1) this.fire('guide');
          if (e.code === BTN_SELECT) {
            clearTimeout(viewTimer);
            if (e.value === 1 && this.holdView) viewTimer = setTimeout(() => this.fire('view'), HOLD_MS);
          }
        }
        rest = buf.subarray(whole);
      });
      s.on('error', err => { this.problems.set(dev.path, err.code === 'EACCES' ? 'no permission' : err.message); this.open.delete(dev.path); });
      s.on('close', () => this.open.delete(dev.path));
      this.problems.delete(dev.path);
      this.open.set(dev.path, { stream: s, name: dev.name });
    }
  }

  stop() {
    clearInterval(this.timer);
    for (const d of this.open.values()) d.stream.destroy();
  }
}

module.exports = { GuideButton, hasBit, gamepadDevices, guidePresses, buttonEvents, EVENT_SIZE };
