// Unit tests for the remote's pointer on websites (assets/tv/pointer.js). Run by android-tv.yml: node --test.
const test = require('node:test');
const assert = require('node:assert/strict');
const HP = require('../app/src/main/assets/tv/pointer.js');

// A pretend ViewerActivity (HearthPointerNative) that notes every call, and timers we run by hand.
function make(opts) {
  const calls = [], timers = [];
  const native = new Proxy({}, { get: (_, k) => (...a) => { calls.push([k, ...a]); return null; } });
  const p = new HP.Pointer(Object.assign({
    native, width: 1920, height: 1080,
    later: (fn, ms) => { timers.push({ fn, ms }); return timers.length - 1; },
    cancel: (i) => { if (timers[i]) timers[i].fn = null; }
  }, opts || {}));
  const fire = () => timers.splice(0).forEach((t) => t.fn && t.fn());
  const of = (k) => calls.filter((c) => c[0] === k);
  return { p, calls, of, fire };
}
// Holds [dir] for [ms], one frame every 16 ms.
function hold(p, dir, ms, t0) {
  let t = t0 || 1000;
  p.key(dir, true, t);
  for (const end = t + ms; t < end;) { t += 16; p.tick(t); }
  p.key(dir, false, t);
  return t;
}

test('starts at the centre in pointer mode, or in page mode when the setting is off', () => {
  const { p } = make();
  assert.equal(p.mode, 'pointer'); assert.equal(p.x, 960); assert.equal(p.y, 540);
  assert.equal(make({ startOn: false }).p.mode, 'page');
  assert.equal(make({ startOn: true }).p.mode, 'pointer');
});

test('unknown speed and size fall back to Normal and Medium', () => {
  const { p } = make({ speed: 'warp', size: 'xxl' });
  assert.equal(p.speed, 'normal'); assert.equal(p.size, 'm');
});

test('speed: slow at first, faster when held, and the setting scales it', () => {
  assert.equal(HP.speedAt(0, 'normal'), HP.BASE);
  assert.ok(Math.abs(HP.speedAt(5000, 'normal') - HP.BASE * HP.MAXMUL) < 1e-9);
  let prev = 0;
  for (let ms = 0; ms <= 2000; ms += 100) { const v = HP.speedAt(ms, 'normal'); assert.ok(v >= prev); prev = v; }
  assert.ok(HP.speedAt(800, 'slow') < HP.speedAt(800, 'normal'));
  assert.ok(HP.speedAt(800, 'normal') < HP.speedAt(800, 'fast'));
});

test('a short press is a small, precise step', () => {
  const { p } = make();
  p.key('right', true, 1000); p.key('right', false, 1010);
  const step = p.x - 960;
  assert.ok(step > 2 && step < 12, 'step ' + step);
});

test('holding a button accelerates: the second half second covers more than the first', () => {
  const { p } = make();
  let t = 1000; p.key('right', true, t);
  for (; t < 1500; t += 16) p.tick(t + 16);
  const first = p.x - 960;
  for (; t < 2000; t += 16) p.tick(t + 16);
  const second = p.x - 960 - first;
  p.key('right', false, t);
  assert.ok(second > first * 1.5, `first ${first}, second ${second}`);
});

test('diagonals: two buttons held move both ways', () => {
  const { p } = make();
  p.key('left', true, 1000); p.key('up', true, 1000);
  for (let t = 1016; t < 1400; t += 16) p.tick(t);
  assert.ok(p.x < 950 && p.y < 530);
});

test('the pointer stays on the screen and pushing past an edge scrolls the page', () => {
  const { p, of } = make();
  hold(p, 'down', 3000);
  assert.equal(p.y, 1079);
  const s = of('scroll');
  assert.ok(s.length > 0, 'scrolled');
  const total = s.reduce((a, c) => a + c[4], 0);
  assert.ok(total > 0.5, 'scrolled down ' + total);
  assert.ok(s.every((c) => c[3] === 0), 'not sideways');
  hold(make().p, 'left', 3000); // no throw at the left edge
});

test('hovering reaches the website at most every 33 ms, and once more when the button is let go', () => {
  const { p, of } = make();
  p.key('right', true, 1000);
  for (let t = 1004; t <= 1100; t += 4) p.tick(t);
  const n = of('move').length;
  assert.ok(n >= 3 && n <= 5, 'moves ' + n);
  p.key('right', false, 1101);
  const last = of('move').pop();
  assert.ok(Math.abs(last[1] - p.x / 1920) < 1e-9 && Math.abs(last[2] - p.y / 1080) < 1e-9);
});

test('nothing runs while the pointer stands still', () => {
  const frames = [];
  const { p } = make({ frame: (f) => frames.push(f) });
  assert.equal(frames.length, 0);
  assert.equal(p.tick(2000), false);
  p.key('up', true, 1000);
  assert.equal(frames.length, 1);
  p.key('down', true, 1001); // already running: no second loop
  assert.equal(frames.length, 1);
  p.key('up', false, 1002); p.key('down', false, 1003);
  frames.shift()(1020);      // the loop sees nothing held and stops
  assert.equal(frames.length, 0);
  assert.equal(p.running, false);
});

test('OK clicks at the pointer', () => {
  const { p, of, fire } = make();
  p.place(0.25, 0.75);
  p.ok(true, 1000); p.ok(false, 1100);
  fire();
  assert.deepEqual(of('tap'), [['tap', 0.25, 0.75]]);
  assert.equal(p.menu, null);
});

test('holding OK opens the menu (no click); OK there picks, ▲▼ move', () => {
  const { p, of, fire } = make();
  p.ok(true, 1000); fire(); p.ok(false, 1700);
  assert.equal(p.menu, 0); assert.equal(of('tap').length, 0);
  // ▲▼ move in the menu, the pointer stays put
  const x = p.x;
  p.key('down', true, 1800); p.key('down', false, 1810);
  assert.equal(p.menu, 1); assert.equal(p.x, x);
  p.ok(true, 1900); p.ok(false, 1950);
  assert.equal(p.menu, null);
  assert.equal(of('rightClick').length, 1);
});

test('the menu: long-press, and page mode', () => {
  let { p, of, fire } = make();
  p.ok(true, 0); fire(); p.ok(false, 700);
  p.ok(true, 800); p.ok(false, 850);
  assert.equal(of('longPress').length, 1);
  ({ p, of, fire } = make());
  p.ok(true, 0); fire(); p.ok(false, 700);
  p.key('up', true, 750); p.key('up', true, 751); // held: no repeat step
  p.key('up', false, 760); p.key('up', true, 770); p.key('up', false, 780);
  assert.equal(HP.MENU[p.menu].id, 'page');
  p.ok(true, 800); p.ok(false, 850);
  assert.equal(p.mode, 'page');
  assert.deepEqual(of('setMode'), [['setMode', 'page']]);
});

test('Back closes the menu first, then goes back a page', () => {
  const { p, of, fire } = make();
  p.ok(true, 0); fire(); p.ok(false, 700);
  assert.equal(p.back(), 'menu'); assert.equal(p.menu, null); assert.equal(of('back').length, 0);
  assert.equal(p.back(), 'page'); assert.equal(of('back').length, 1);
});

test('page mode: the pointer leaves the buttons alone', () => {
  const { p, of } = make({ startOn: false });
  assert.equal(p.key('right', true, 0), false);
  assert.equal(p.ok(true, 0), false);
  assert.equal(p.scrollKey(1), false);
  assert.equal(of('tap').length + of('move').length + of('scroll').length, 0);
  p.setMode('pointer');
  assert.equal(p.key('right', true, 0), true);
});

test('scroll keys: half a screen at the pointer', () => {
  const { p, of } = make();
  p.scrollKey(1); p.scrollKey(-1); p.scrollKey(0, 1);
  assert.deepEqual(of('scroll').map((c) => [c[3], c[4]]), [[0, 0.5], [0, -0.5], [0.5, 0]]);
});

test('letting go of everything (Hearth TV in the background) stops the pointer', () => {
  const { p, fire, of } = make();
  p.key('right', true, 0); p.ok(true, 0);
  p.release(); fire();
  assert.equal(p.holding(), false); assert.equal(p.menu, null);
  p.ok(false, 100);
  assert.equal(of('tap').length, 0);
});

test('a new screen size keeps the pointer at the same place', () => {
  const { p } = make();
  p.place(0.3, 0.6);
  p.resize(960, 540);
  assert.equal(p.x, 288); assert.equal(p.y, 324);
});

test('the hint says how to switch', () => {
  assert.match(HP.hintText('pointer').keys.map((k) => k.join(' ')).join(' '), /Hold OK.*page mode/);
  assert.match(HP.hintText('page').keys.map((k) => k.join(' ')).join(' '), /Hold OK Pointer/);
});
