// Unit tests for the remote's pointer on websites (assets/tv/pointer.js). Run by android-tv.yml: node --test.
const test = require('node:test');
const assert = require('node:assert/strict');
const HP = require('../app/src/main/assets/tv/pointer.js');

// A pretend ViewerActivity (HearthPointerNative) that notes every call, and what the layer draws.
function make(opts) {
  const calls = [], drawn = [];
  const native = new Proxy({}, { get: (_, k) => (...a) => { calls.push([k, ...a]); return null; } });
  const p = new HP.Pointer(Object.assign({ native, width: 1920, height: 1080, view: (_, what) => drawn.push(what) }, opts || {}));
  const of = (k) => calls.filter((c) => c[0] === k);
  return { p, calls, of, drawn };
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

test('OK is a finger at the pointer: a short press clicks', () => {
  const { p, of, drawn } = make();
  p.place(0.25, 0.75);
  p.ok(true, 1000); p.ok(false, 1120);
  assert.deepEqual(of('press'), [['press', 0.25, 0.75]]);
  assert.deepEqual(of('release'), [['release', 0.25, 0.75]]);
  assert.ok(drawn.includes('press') && drawn.includes('click'));
});

test('holding OK is a long-press (the site\'s right-click menu)', () => {
  const { p, of, drawn } = make();
  p.ok(true, 1000); p.ok(true, 1300); // a repeat while held: still one press
  p.ok(false, 1000 + HP.LONG_MS + 50);
  assert.equal(of('press').length, 1); assert.equal(of('release').length, 1);
  assert.equal(drawn[drawn.length - 1], 'long');
  p.ok(false, 3000); // a stray release: nothing
  assert.equal(of('release').length, 1);
});

test('moving while OK is held drags (the move reaches ViewerActivity, which moves the finger)', () => {
  const { p, of } = make();
  p.ok(true, 1000);
  p.key('right', true, 1010); p.tick(1100); p.key('right', false, 1110);
  p.ok(false, 1200);
  assert.ok(of('move').length > 0);
  assert.ok(of('release')[0][1] > 0.5, 'released where it was dragged to');
});

test('Back goes back a page, lifting a press first', () => {
  const { p, of } = make();
  p.ok(true, 0);
  assert.equal(p.back(), 'page');
  assert.equal(of('release').length, 1); assert.equal(of('back').length, 1);
});

test('page mode: the pointer leaves the buttons alone', () => {
  const { p, of } = make({ startOn: false });
  assert.equal(p.key('right', true, 0), false);
  assert.equal(p.ok(true, 0), false);
  assert.equal(p.scrollKey(1), false);
  assert.equal(of('press').length + of('move').length + of('scroll').length, 0);
  p.setMode('pointer');
  assert.equal(p.key('right', true, 0), true);
});

test('scroll keys: half a screen at the pointer', () => {
  const { p, of } = make();
  p.scrollKey(1); p.scrollKey(-1); p.scrollKey(0, 1);
  assert.deepEqual(of('scroll').map((c) => [c[3], c[4]]), [[0, 0.5], [0, -0.5], [0.5, 0]]);
});

test('letting go of everything (Hearth TV in the background) stops the pointer and lifts a press', () => {
  const { p, of } = make();
  p.key('right', true, 0); p.ok(true, 0);
  p.release();
  assert.equal(p.holding(), false); assert.equal(p.okAt, 0);
  assert.equal(of('release').length, 1);
  p.ok(false, 100);
  assert.equal(of('release').length, 1);
});

test('switching mode lifts a press and stops moving', () => {
  const { p, of } = make();
  p.key('up', true, 0); p.ok(true, 0);
  p.setMode('page');
  assert.equal(p.mode, 'page'); assert.equal(p.holding(), false); assert.equal(of('release').length, 1);
});

test('a new screen size keeps the pointer at the same place', () => {
  const { p } = make();
  p.place(0.3, 0.6);
  p.resize(960, 540);
  assert.equal(p.x, 288); assert.equal(p.y, 324);
});

test('the hint says how to switch', () => {
  const pt = HP.hintText('pointer').keys.map((k) => k.join(' ')).join(' · ');
  assert.match(pt, /OK Click/); assert.match(pt, /Hold OK Right-click/); assert.match(pt, /Hold ↩ Page mode/);
  assert.match(HP.hintText('page').keys.map((k) => k.join(' ')).join(' · '), /Hold ↩ Pointer/);
});
