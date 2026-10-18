'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { ButtonPrefs, Buttons } = require('../src/buttons');

test('button prefs: defaults, checked values, max volume', () => {
  const file = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'hb-')), 'buttons.json');
  const p = new ButtonPrefs(file);
  assert.deepStrictEqual(p.get(), { press: 'sleep', hold: 'menu', overamp: false });
  assert.strictEqual(p.maxVolume(), 100);
  p.set({ press: 'menu', hold: 'explode', overamp: 1 });
  assert.deepStrictEqual(p.get(), { press: 'menu', hold: 'menu', overamp: true });
  assert.strictEqual(p.maxVolume(), 150);
  fs.writeFileSync(file, '{"press":"nope","overamp":"yes"}');
  assert.deepStrictEqual(p.get(), { press: 'sleep', hold: 'menu', overamp: false });
});

test('buttons: helper messages become events', () => {
  const b = new Buttons('/nonexistent');
  const got = [];
  b.on('power', k => got.push('power:' + k));
  b.on('volume', d => got.push('vol:' + d));
  b.on('mute', () => got.push('mute'));
  b.onMessage({ t: 'devices', list: ['Power Button'], problems: [] });
  b.onMessage({ t: 'power', press: 'short' });
  b.onMessage({ t: 'power', press: 'long' });
  b.onMessage({ t: 'vol', d: -1 });
  b.onMessage({ t: 'mute' });
  assert.deepStrictEqual(got, ['power:short', 'power:long', 'vol:-1', 'mute']);
  assert.deepStrictEqual(b.status(), { devices: ['Power Button'], problems: [] });
});

test('gamescope: top-level windows from xwininfo', () => {
  const { parseChildren } = require('../src/gamescope');
  const out = parseChildren(`
xwininfo: Window id: 0x3a6 (the root window) (has no name)
  Root window id: 0x3a6 (the root window) (has no name)
  Parent window id: 0x0 (none)
     3 children:
     0x200002 "Hearth": ("nimbus" "Nimbus")  1280x800+0+0  +0+0
     0x400001 (has no name): ()  1x1+0+0  +0+0
     0x600003 "Discord - #general": ("discord" "discord")  1200x700+40+50  +40+50
`);
  assert.deepStrictEqual(out, [
    { win: 0x200002, title: 'Hearth', w: 1280, h: 800 },
    { win: 0x600003, title: 'Discord - #general', w: 1200, h: 700 },
  ]);
});

test('steam account: MostRecent, then AutoLoginUser, then newest', () => {
  const { pickUser } = require('../src/accounts');
  const users = { 1: { AccountName: 'old', Timestamp: '100' }, 2: { AccountName: 'Auto', Timestamp: '50' }, 3: { AccountName: 'new', Timestamp: '300' } };
  assert.strictEqual(pickUser(users, null)[0], '3');
  assert.strictEqual(pickUser(users, 'auto')[0], '2');
  assert.strictEqual(pickUser({ ...users, 4: { AccountName: 'x', MostRecent: '1' } }, 'auto')[0], '4');
  assert.strictEqual(pickUser({}, null), null);
});

test('guide: controller presses and directions for the overlay', () => {
  const { padEvents, EVENT_SIZE } = require('../src/guide');
  const ev = list => { const b = Buffer.alloc(EVENT_SIZE * list.length); list.forEach(([t, c, v], i) => { b.writeUInt16LE(t, i * EVENT_SIZE + 16); b.writeUInt16LE(c, i * EVENT_SIZE + 18); b.writeInt32LE(v, i * EVENT_SIZE + 20); }); return b; };
  const st = {};
  assert.deepStrictEqual(padEvents(ev([[1, 0x130, 1], [1, 0x130, 2], [1, 0x130, 0], [1, 0x13c, 1]]), st),
    [{ b: 'a', down: true }, { b: 'a', down: false }, { b: 'guide', down: true }]);
  assert.deepStrictEqual(padEvents(ev([[3, 17, -1], [3, 17, 0], [3, 0, 30000], [3, 0, 15000], [3, 0, 0]]), st),
    [{ b: 'up', down: true }, { b: 'up', down: false }, { b: 'right', down: true }, { b: 'right', down: false }]);
});

test('install queue: pause keeps the job, resume carries on, queue order', async () => {
  const { InstallQueue } = require('../src/stores/installs');
  const q = new InstallQueue();
  const started = [];
  const work = id => async (job, signal) => { started.push(id); await new Promise((res, rej) => { const t = setTimeout(res, 200); signal.addEventListener('abort', () => { clearTimeout(t); rej(new Error('aborted')); }); }); };
  const finished = [];
  q.on('finished', (id, err) => finished.push(id + ':' + (err || 'ok')));
  q.enqueue({ gameId: 'a', title: 'A', work: work('a') });
  q.enqueue({ gameId: 'b', title: 'B', work: work('b') });
  q.enqueue({ gameId: 'c', title: 'C', work: work('c') }, { paused: true });
  await new Promise(r => setTimeout(r, 20));
  assert.ok(q.pause('a'));
  await new Promise(r => setTimeout(r, 20));
  assert.strictEqual(q.get('a').state, 'paused');            // still in the queue
  assert.deepStrictEqual(started, ['a', 'b']);               // b started while a is paused, c stays paused
  q.resume('a'); q.first('a');
  await new Promise(r => setTimeout(r, 500));
  assert.deepStrictEqual(finished, ['b:ok', 'a:ok']);
  assert.strictEqual(q.get('c').state, 'paused');
  q.cancel('c');
  assert.deepStrictEqual(finished, ['b:ok', 'a:ok', 'c:cancelled']);
});
