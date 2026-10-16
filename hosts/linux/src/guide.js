'use strict';
// Watches controllers for the Guide (Xbox / PS / Home) button, so Nimbus can come back over a running game,
// the way the Steam button does on a Steam Deck. The UI itself reads controllers through the browser's
// Gamepad API, but a hidden window gets no gamepad input, so this reads the kernel's event devices directly.
// Joystick event devices are readable by the logged-in user (systemd's uaccess rule), no extra setup needed.

const fs = require('node:fs');
const path = require('node:path');
const { EventEmitter } = require('node:events');

const EV_KEY = 1;
const BTN_GAMEPAD = 0x130;
const BTN_MODE = 0x13c;
const EVENT_SIZE = 24; // struct input_event on 64-bit: timeval (16) + type (2) + code (2) + value (4)

/** True if a /sys capabilities bitmap ("key" file: hex words, most significant first) has bit `bit` set. */
function hasBit(capText, bit, wordBits = 64) {
  const words = capText.trim().split(/\s+/).reverse();
  const w = words[Math.floor(bit / wordBits)];
  if (!w) return false;
  return (BigInt('0x' + w) >> BigInt(bit % wordBits)) & 1n ? true : false;
}

/** Event devices that are gamepads with a Guide button. */
function gamepadDevices(sysRoot = '/sys/class/input') {
  let names = [];
  try { names = fs.readdirSync(sysRoot).filter(n => /^event\d+$/.test(n)); } catch {}
  return names.filter(n => {
    try {
      const caps = fs.readFileSync(path.join(sysRoot, n, 'device', 'capabilities', 'key'), 'utf8');
      return hasBit(caps, BTN_GAMEPAD) && hasBit(caps, BTN_MODE);
    } catch { return false; }
  }).map(n => '/dev/input/' + n);
}

/** Pulls Guide presses out of a chunk of raw input_event records. */
function guidePresses(buf) {
  let n = 0;
  for (let off = 0; off + EVENT_SIZE <= buf.length; off += EVENT_SIZE) {
    const type = buf.readUInt16LE(off + 16), code = buf.readUInt16LE(off + 18), value = buf.readInt32LE(off + 20);
    if (type === EV_KEY && code === BTN_MODE && value === 1) n++;
  }
  return n;
}

class GuideButton extends EventEmitter {
  start() {
    this.open = new Map();
    this.rescan();
    this.timer = setInterval(() => this.rescan(), 5000); // controllers come and go
  }

  rescan() {
    for (const dev of gamepadDevices()) {
      if (this.open.has(dev)) continue;
      const s = fs.createReadStream(dev, { highWaterMark: EVENT_SIZE * 64 });
      let rest = Buffer.alloc(0);
      s.on('data', chunk => {
        const buf = Buffer.concat([rest, chunk]);
        const whole = buf.length - (buf.length % EVENT_SIZE);
        if (guidePresses(buf.subarray(0, whole))) this.emit('press');
        rest = buf.subarray(whole);
      });
      s.on('error', () => this.open.delete(dev)); // unplugged, or no permission
      s.on('close', () => this.open.delete(dev));
      this.open.set(dev, s);
    }
  }

  stop() {
    clearInterval(this.timer);
    for (const s of (this.open || new Map()).values()) s.destroy();
  }
}

module.exports = { GuideButton, hasBit, gamepadDevices, guidePresses, EVENT_SIZE };
