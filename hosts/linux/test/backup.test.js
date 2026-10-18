'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const backup = require('../src/backup');

function put(f, text) { fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, text); }

test('backup keeps settings and play data, skips caches and (by default) sign-ins, and restores them', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'bk-'));
  const data = path.join(root, 'data'), conf = path.join(root, 'conf'), dest = path.join(root, 'Backups');
  put(path.join(data, 'library.json'), '{"favorites":["steam:1"]}');
  put(path.join(data, 'profiles.json'), '{"active":"main"}');
  put(path.join(data, 'profiles/p1/library.json'), '{"kid":true}');
  put(path.join(data, 'profiles/p1/accounts/epic.json'), 'token');
  put(path.join(data, 'accounts/xbox.json'), 'token');
  put(path.join(data, 'cache/big.bin'), 'x'.repeat(1000));
  put(path.join(data, 'bridge.log'), 'log');
  put(path.join(conf, 'session.conf'), 'MODE=game');

  const r = await backup.create({ dest, dataDir: data, confDir: conf, now: new Date('2026-10-18T12:30:00Z'), version: '0.0.23' });
  assert.match(path.basename(r.file), /^hearth-backup-2026-10-18-12-30\.tar\.gz$/);
  assert.deepStrictEqual(backup.list([dest]).map(b => b.name), [path.basename(r.file)]);
  const meta = await backup.info(r.file);
  assert.strictEqual(meta.version, '0.0.23');
  assert.strictEqual(meta.signins, false);

  const names = (await require('../src/util').run('tar', ['-tzf', r.file])).split('\n');
  assert.ok(names.includes('data/library.json'));
  assert.ok(names.includes('data/profiles/p1/library.json'));
  assert.ok(names.includes('config/session.conf'));
  assert.ok(!names.some(n => /cache|bridge\.log|accounts/.test(n)), names.join(' '));

  // Change things, then restore: the backup's files come back, sign-ins on the device stay.
  fs.writeFileSync(path.join(data, 'library.json'), '{"favorites":[]}');
  await backup.restore(r.file, { dataDir: data, confDir: conf, now: new Date('2026-10-19T08:00:00Z') });
  assert.strictEqual(fs.readFileSync(path.join(data, 'library.json'), 'utf8'), '{"favorites":["steam:1"]}');
  assert.strictEqual(fs.readFileSync(path.join(data, 'accounts/xbox.json'), 'utf8'), 'token');
  assert.ok(fs.existsSync(path.join(data, 'before-restore/2026-10-19-08-00-00/library.json')));

  const withSignins = await backup.create({ dest, dataDir: data, confDir: conf, signins: true, now: new Date('2026-10-18T13:00:00Z') });
  const n2 = (await require('../src/util').run('tar', ['-tzf', withSignins.file])).split('\n');
  assert.ok(n2.includes('data/accounts/xbox.json'));
  assert.ok(!n2.some(n => n.includes('before-restore')));
  await assert.rejects(backup.info(path.join(root, 'nope.tar.gz')));
});
