'use strict';
// How Hearth's apps read the controller (ui/pad/hearth-gamepad.js, docs/CONTROLLER-FOCUS.md), with a fake Gamepad API,
// focus and frames: no presses while the window isn't focused, a press held from before the focus came back doesn't
// count, an error in the app's loop doesn't end it, and frames keep coming when requestAnimationFrame stops.
const test = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const SRC = fs.readFileSync(path.join(__dirname, '../../../ui/pad/hearth-gamepad.js'), 'utf8');

function page() {
  let t = 0;
  const listeners = {}, docListeners = {}, frames = [], intervals = [], body = [];
  const storage = {};
  const pad = { id: 'Xbox Wireless Controller', index: 0, mapping: 'standard', connected: true, timestamp: 0,
    buttons: Array.from({ length: 17 }, () => ({ pressed: false, touched: false, value: 0 })), axes: [0, 0, 0, 0] };
  const env = {
    pads: [], focused: true, hidden: false, rafOn: true,
    press(i, on) { pad.buttons[i] = { pressed: on, touched: on, value: on ? 1 : 0 }; },
    axis(i, v) { pad.axes[i] = v; },
    connect() { env.pads = [pad]; },
  };
  const doc = {
    get hidden() { return env.hidden; },
    hasFocus: () => env.focused,
    addEventListener: (n, f) => { (docListeners[n] = docListeners[n] || []).push(f); },
    createElement: () => ({ style: {}, setAttribute() {}, remove() { const i = body.indexOf(this); if (i >= 0) body.splice(i, 1); } }),
    body: { appendChild: el => body.push(el) },
  };
  const win = {
    document: doc,
    navigator: { getGamepads: () => [null, ...env.pads].slice(1).concat([null]) },
    performance: { now: () => t },
    localStorage: { getItem: k => (k in storage ? storage[k] : null), setItem: (k, v) => { storage[k] = String(v); } },
    requestAnimationFrame: f => { if (!env.rafOn) return 0; frames.push(f); return frames.length; },
    cancelAnimationFrame: () => {},
    setInterval: f => { intervals.push(f); return intervals.length; },
    setTimeout: () => 0,
    addEventListener: (n, f) => { (listeners[n] = listeners[n] || []).push(f); },
    console: { error() { env.errors = (env.errors || 0) + 1; } },
  };
  win.window = win;
  vm.runInNewContext(SRC, win);
  env.G = win.HearthGamepad;
  env.storage = storage;
  env.body = body;
  // One frame: time moves on 16 ms, pending rAF callbacks run (or, with rAF stopped, the 100 ms timer does).
  env.frame = (ms = 16) => {
    t += ms;
    const f = frames.splice(0);
    f.forEach(cb => cb(t));
    if (!f.length) intervals.forEach(cb => cb());
  };
  env.fire = (n) => (listeners[n] || []).forEach(f => f({}));
  env.docFire = (n) => (docListeners[n] || []).forEach(f => f({}));
  return env;
}

// An app loop like the apps' own: counts presses of A (button 0), D-pad down held (13).
function appLoop(env) {
  const seen = { a: 0, down: 0 }; let held = false;
  env.G.loop(() => {
    const pads = env.G.pads();
    const a = pads.some(g => g.buttons[0] && g.buttons[0].pressed);
    if (a && !held) seen.a++;
    held = a;
    if (pads.some(g => g.buttons[13] && g.buttons[13].pressed)) seen.down++;
  });
  return seen;
}

test('pads come through while focused, empty slots left out', () => {
  const env = page();
  env.connect();
  const seen = appLoop(env);
  env.press(0, true); env.frame(); env.press(0, false); env.frame();
  assert.strictEqual(seen.a, 1);
  assert.strictEqual(env.G.pads().length, 1);
});

test('nothing while the window is not focused (a frozen held D-pad does not keep moving)', () => {
  const env = page();
  env.connect();
  const seen = appLoop(env);
  env.press(13, true); env.frame();
  assert.strictEqual(seen.down, 1);
  // A notification / Quick Access takes the focus while the D-pad is down: WebKit freezes the state as it was.
  env.focused = false; env.fire('blur');
  for (let i = 0; i < 20; i++) env.frame();
  assert.strictEqual(seen.down, 1, 'no repeats while unfocused');
  assert.deepStrictEqual(env.G.pads(), []);
});

test('a press held from before the focus came back does not count until let go', () => {
  const env = page();
  env.connect();
  const seen = appLoop(env);
  env.focused = false; env.fire('blur'); env.frame();
  env.press(0, true);                       // A closes Quick Access (Mantel's), still down when the app is back
  env.focused = true; env.fire('focus');
  env.frame(); env.frame();
  assert.strictEqual(seen.a, 0, 'the A that closed Quick Access does not press in the app');
  env.press(0, false); env.frame();
  env.press(0, true); env.frame();          // a new press does
  assert.strictEqual(seen.a, 1);
});

test('a stick held across the focus change counts as centred until it is', () => {
  const env = page();
  env.connect();
  env.G.loop(() => {});
  env.focused = false; env.fire('blur'); env.frame();
  env.axis(1, 0.9);
  env.focused = true; env.fire('focus');
  assert.strictEqual(env.G.pads()[0].axes[1], 0);
  env.axis(1, 0.1); env.G.pads();
  env.axis(1, 0.9);
  assert.strictEqual(env.G.pads()[0].axes[1], 0.9);
});

test('a hidden page gets nothing', () => {
  const env = page();
  env.connect();
  env.hidden = true;
  assert.deepStrictEqual(env.G.pads(), []);
  env.hidden = false; env.docFire('visibilitychange');
  assert.strictEqual(env.G.pads().length, 1);
});

test('a WebKit whose hasFocus() says no without any blur does not lock the controller out', () => {
  const env = page();
  env.connect();
  env.focused = false;                      // never blurred
  assert.strictEqual(env.G.active(), true);
  assert.strictEqual(env.G.pads().length, 1);
});

test('an error in the app loop does not end it', () => {
  const env = page();
  env.connect();
  let n = 0;
  env.G.loop(() => { n++; if (n === 2) throw new Error('boom'); });
  for (let i = 0; i < 5; i++) env.frame();
  assert.strictEqual(n, 5);
  assert.strictEqual(env.errors, 1);
});

test('frames keep coming from the timer when requestAnimationFrame stops', () => {
  const env = page();
  env.connect();
  let n = 0;
  env.G.loop(() => { n++; });
  env.frame();
  env.rafOn = false;
  for (let i = 0; i < 5; i++) env.frame(300);
  assert.ok(n >= 5, `frames: ${n}`);
});

test('the same loop is only added once (no double presses)', () => {
  const env = page();
  env.connect();
  let n = 0;
  const f = () => { n++; };
  env.G.loop(f); env.G.loop(f);
  env.frame();
  assert.strictEqual(n, 1);
});

test('the "press any button" note shows once where a controller was used before, and goes with a press', () => {
  const env = page();
  env.storage['hearth-pad-seen'] = '1';
  env.G.loop(() => env.G.pads());
  env.frame(); assert.strictEqual(env.body.length, 0, 'not straight away');
  env.frame(1600);
  assert.strictEqual(env.body.length, 1);
  assert.match(env.body[0].textContent, /Press any button/);
  env.connect(); env.press(0, true); env.frame();
  assert.strictEqual(env.body.length, 0);
  env.frame(5000);
  assert.strictEqual(env.body.length, 0, 'only once');
});

test('no note in an app the controller was never used in', () => {
  const env = page();
  env.G.loop(() => env.G.pads());
  env.frame(); env.frame(3000);
  assert.strictEqual(env.body.length, 0);
});
