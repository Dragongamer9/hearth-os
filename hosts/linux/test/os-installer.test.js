'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { EventEmitter } = require('node:events');
const { OsInstaller, parseLine } = require('../src/os-installer');

function fakeSpawn(script) {
  const calls = [];
  const fn = (cmd, args) => {
    const p = new EventEmitter();
    p.stdout = new EventEmitter(); p.stderr = new EventEmitter();
    let input = '';
    p.stdin = { end: s => { input = s || ''; setImmediate(() => { const { out, code } = script(args, input); for (const l of out) p.stdout.emit('data', l + '\n'); p.emit('close', code); }); } };
    calls.push({ cmd, args });
    return p;
  };
  fn.calls = calls;
  return fn;
}
const opts = spawnFn => ({ spawnFn, exists: () => true, live: () => true });

test('parses stage, result and error lines', () => {
  assert.deepStrictEqual(parseLine('stage: {"stage":"download","pct":40,"text":"Downloading"}').stage.pct, 40);
  assert.deepStrictEqual(parseLine('result: {"ok":true}').result, { ok: true });
  assert.strictEqual(parseLine('error: no room').error, 'no room');
  assert.strictEqual(parseLine('Fetching layer sha256:abc'), null);
});

test('lists drives through pkexec', async () => {
  const sp = fakeSpawn(() => ({ out: ['{"disks":[{"dev":"/dev/sda"}]}'], code: 0 }));
  const inst = new OsInstaller(opts(sp));
  assert.strictEqual((await inst.disks()).disks[0].dev, '/dev/sda');
  assert.deepStrictEqual(sp.calls[0], { cmd: 'pkexec', args: ['/usr/libexec/nimbus/hearth-install', 'disks'] });
});

test('an install reports progress and finishes', async () => {
  let gotPlan = null;
  const sp = fakeSpawn((args, input) => { gotPlan = JSON.parse(input); return { out: [
    'stage: {"stage":"disk","pct":1,"text":"Erasing the drive"}',
    'stage: {"stage":"download","pct":50,"text":"Downloading Hearth OS","left":120}',
    'result: {"ok":true}'], code: 0 }; });
  const inst = new OsInstaller(opts(sp));
  const seen = [];
  inst.on('changed', s => seen.push(s.pct));
  const done = new Promise(r => inst.on('changed', s => s.state !== 'running' && r(s)));
  inst.start({ mode: 'whole', disk: '/dev/sda', user: { name: 'sam', password: 'x' } });
  const end = await done;
  assert.strictEqual(end.state, 'done');
  assert.strictEqual(gotPlan.disk, '/dev/sda');
  assert.ok(seen.includes(50));
  assert.strictEqual(end.log, undefined);   // the raw log stays out of events
});

test('a failed install keeps the engine\'s message', async () => {
  const sp = fakeSpawn(() => ({ out: ['stage: {"stage":"disk","pct":1,"text":"x"}', "error: couldn't download Hearth OS"], code: 1 }));
  const inst = new OsInstaller(opts(sp));
  const done = new Promise(r => inst.on('changed', s => s.state !== 'running' && r(s)));
  inst.start({ mode: 'whole', disk: '/dev/sda' });
  const end = await done;
  assert.strictEqual(end.state, 'failed');
  assert.match(end.error, /download/);
});

test('not offered off the live USB', async () => {
  const inst = new OsInstaller({ spawnFn: fakeSpawn(() => ({ out: [], code: 0 })), exists: () => true, live: () => false });
  assert.strictEqual(inst.status().available, false);
  await assert.rejects(inst.disks());
});
