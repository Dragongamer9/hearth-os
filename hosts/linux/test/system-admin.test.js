'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { EventEmitter } = require('node:events');
const { SystemAdmin, readStartMode, writeStartMode, parseHelperOutput } = require('../src/system-admin');

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-admin-'));

test('start mode is kept in session.conf without touching other settings', () => {
  const f = path.join(tmp(), 'session.conf');
  assert.strictEqual(readStartMode(f), 'game');
  fs.writeFileSync(f, 'NIMBUS_GAMESCOPE_ARGS="-W 1280"');
  writeStartMode(f, 'desktop');
  assert.strictEqual(fs.readFileSync(f, 'utf8'), 'NIMBUS_GAMESCOPE_ARGS="-W 1280"\nNIMBUS_START_MODE=desktop\n');
  assert.strictEqual(readStartMode(f), 'desktop');
  writeStartMode(f, 'game');
  assert.match(fs.readFileSync(f, 'utf8'), /^NIMBUS_START_MODE=game$/m);
  assert.strictEqual(fs.readFileSync(f, 'utf8').match(/NIMBUS_START_MODE/g).length, 1);
});

test('helper output: result JSON, progress lines, errors', () => {
  assert.deepStrictEqual(parseHelperOutput('log: a\nlog: b\n{"ok":true}\n'), { result: { ok: true } });
  assert.deepStrictEqual(parseHelperOutput('log: a\nerror: the current password is incorrect\n'), { error: 'the current password is incorrect' });
});

function fakeSpawn(lines, code = 0, seen = {}) {
  return (cmd, args) => {
    seen.cmd = cmd; seen.args = args; seen.input = '';
    const p = new EventEmitter();
    p.stdout = new EventEmitter(); p.stderr = new EventEmitter();
    p.stdin = { end: d => { seen.input = d || ''; } };
    p.kill = () => {};
    setImmediate(() => { for (const l of lines) p.stdout.emit('data', l + '\n'); p.emit('close', code); });
    return p;
  };
}

test('changing the password goes through pkexec hearth-admin with the passwords on stdin', async () => {
  const seen = {};
  const a = new SystemAdmin({ helper: '/x/hearth-admin', exists: () => true, spawnFn: fakeSpawn(['{"ok":true}'], 0, seen) });
  await a.changePassword('old pw', 'new pw');
  assert.strictEqual(seen.cmd, 'pkexec');
  assert.deepStrictEqual(seen.args, ['/x/hearth-admin', 'password']);
  assert.deepStrictEqual(JSON.parse(seen.input), { old: 'old pw', new: 'new pw' });
});

test('helper errors come back as readable messages', async () => {
  const a = new SystemAdmin({ exists: () => true, spawnFn: fakeSpawn(['error: the current password is incorrect'], 1) });
  await assert.rejects(a.changePassword('x', 'yyyy'), /current password is incorrect/);
});

test('without Hearth OS only the start mode is offered', async () => {
  const a = new SystemAdmin({ exists: () => false, sessionConf: path.join(tmp(), 's.conf') });
  const l = a.login();
  assert.strictEqual(l.canAdmin, false);
  assert.strictEqual(l.autologin, null);
  await assert.rejects(a.setAutologin(true), /Hearth, the OS/);
  assert.strictEqual(a.setStartMode('desktop').startMode, 'desktop');
});

test('system update reports progress, then the new status', async () => {
  const status = { booted: { version: '44.1' }, staged: { version: '44.2' }, available: null };
  const a = new SystemAdmin({ exists: () => true, spawnFn: fakeSpawn(['log: Downloading the system update…', 'log: Fetching layer 3/9', JSON.stringify(status)]) });
  const seen = [];
  a.on('update', s => seen.push({ state: s.state, line: s.line }));
  const r = await a.installUpdate();
  assert.strictEqual(r.state, 'idle');
  assert.deepStrictEqual(r.status, status);
  assert.ok(seen.some(s => s.state === 'updating' && s.line === 'Fetching layer 3/9'));
});
