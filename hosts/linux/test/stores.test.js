'use strict';
// Store accounts, installs, Steam's binary caches and performance numbers (no network, no Electron).

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'nimbus-stores-'));
process.env.XDG_DATA_HOME = path.join(tmp, 'data');
process.env.XDG_CONFIG_HOME = path.join(tmp, 'config');
const DATA = path.join(tmp, 'data', 'nimbus');

const { InstallQueue, InstallJob } = require('../src/stores/installs');
const tools = require('../src/stores/tools');
const { OwnedCache, secure, jsonIn } = require('../src/stores/common');
const { StoreAccounts } = require('../src/stores');
const { Store } = require('../src/library/store');
const steamcache = require('../src/library/steamcache');
const perf = require('../src/perf');

const write = (p, t) => { fs.mkdirSync(path.dirname(p), { recursive: true }); fs.writeFileSync(p, t); };

test('install progress lines from legendary and gogdl', () => {
  const j = new InstallJob({ gameId: 'epic:x', title: 'X', work: async () => {} });
  j.readLine('[DLManager] INFO: Download size: 1024.00 MiB', false);
  j.readLine('[DLManager] INFO: = Progress: 25.00% (100/400), Running for 00:00:10', false);
  j.readLine('[DLManager] INFO:  + Download\t- 12.50 MiB/s (raw) / 20.00 MiB/s (decompressed)', false);
  assert.equal(j.progress, 0.25);
  assert.equal(j.bytesTotal, 1024 * 1024 * 1024);
  assert.equal(j.bytesDone, 256 * 1024 * 1024);
  assert.equal(j.speed, 12.5 * 1024 * 1024);
  const g = new InstallJob({ gameId: 'gog:1', title: 'G', work: async () => {} });
  g.readLine('[PROGRESS] INFO: = Progress: 50.00 500/1000, Running for: 00:00:01', true);
  assert.deepEqual([g.progress, g.bytesDone, g.bytesTotal], [0.5, 500, 1000]);
});

test('install queue runs one at a time, reports results, cancels', async () => {
  const q = new InstallQueue();
  const done = [];
  q.on('finished', (id, err) => done.push([id, err]));
  let release;
  q.enqueue({ gameId: 'a', title: 'A', work: () => new Promise(r => { release = r; }) });
  q.enqueue({ gameId: 'b', title: 'B', work: async () => { throw new Error('disk full'); } });
  q.enqueue({ gameId: 'c', title: 'C', work: async () => {} });
  assert.throws(() => q.enqueue({ gameId: 'a', title: 'A', work: async () => {} }), /already in the download queue/);
  assert.equal(q.get('b').state, 'queued');
  q.cancel('c');
  release();
  await new Promise(r => setTimeout(r, 50));
  assert.deepEqual(done, [['c', 'cancelled'], ['a', null], ['b', 'disk full']]);
  assert.equal(q.active().length, 0);
});

test('tool error lines and JSON in tool output', () => {
  assert.equal(tools.problem({ out: '', err: '[cli] INFO: x\n[cli] ERROR: Login failed\n', code: 1 }), 'Login failed');
  assert.deepEqual(jsonIn('[cli] INFO: ok\n[{"a":1}]'), [{ a: 1 }]);
  assert.equal(jsonIn('nothing'), null);
});

test('secrets fall back to a private file without a keyring', () => {
  secure.save('t', 'token-123');
  assert.equal(secure.load('t'), 'token-123');
  assert.equal(fs.statSync(path.join(DATA, 'accounts', 't.bin')).mode & 0o777, 0o600);
  secure.remove('t');
  assert.equal(secure.load('t'), null);
});

test('accounts: installed Epic/GOG games, not-installed list, install routing', () => {
  // legendary's records and a GOG install
  write(path.join(DATA, 'legendary', 'user.json'), JSON.stringify({ displayName: 'DeckPilot' }));
  const epicDir = path.join(tmp, 'Games', 'Epic', 'Hades');
  fs.mkdirSync(epicDir, { recursive: true });
  write(path.join(DATA, 'legendary', 'installed.json'), JSON.stringify({ Quail: { title: 'Hades', install_path: epicDir, install_size: 10 } }));
  const gogDir = path.join(tmp, 'Games', 'GOG', 'Witcher');
  write(path.join(gogDir, 'goggame-1207658924.info'), JSON.stringify({ name: 'The Witcher', playTasks: [{ isPrimary: true, path: 'bin\\witcher.exe', type: 'FileTask' }] }));
  const store = new Store(path.join(tmp, 'library.json'));
  store.update(d => { d.gogInstalls = { 1207658924: gogDir }; });
  const acc = new StoreAccounts(store);
  acc.epic.owned = new OwnedCache({ fetched: 1, games: [
    { id: 'Quail', title: 'Hades', art: { cover: 'https://x/c.jpg' } },
    { id: 'Fortnite', title: 'Fortnite', art: {} },
    { id: 'EAgame', title: 'EA Game', art: {}, note: 'Installs through the EA app' }] });

  const epic = acc.installed('epic');
  assert.equal(epic[0].id, 'epic:Quail');
  assert.equal(epic[0].art.cover, 'https://x/c.jpg');
  assert.equal(epic[0].launchVia, 'legendary');
  const gog = acc.installed('gog');
  assert.equal(gog[0].title, 'The Witcher');
  assert.equal(gog[0].windowsExe, path.join(gogDir, 'bin/witcher.exe'));

  const missing = acc.notInstalled('epic', new Set(['epic:Quail']));
  assert.deepEqual(missing.map(g => [g.id, g.installUri]), [['epic:Fortnite', 'nimbus-epic:Fortnite'], ['epic:EAgame', null]]);
  assert.equal(acc.status({ epic: 1 }, { name: null, owned: 0 })[1].name, 'DeckPilot');
  assert.equal(acc.installRoot, path.join(os.homedir(), 'Games'));
  assert.throws(() => acc.startInstall({ id: 'gog:2', title: 'G', installUri: 'nimbus-gog:2' }), /Sign in to GOG first/);
  const job = acc.startInstall(missing[0]);
  assert.equal(job.gameId, 'epic:Fortnite');
  acc.installs.cancel('epic:Fortnite');
});

// ------------------------------------------------------------------ Steam binary caches

function kvString(k, v) { return Buffer.concat([Buffer.from([0x01]), Buffer.from(k + '\0'), Buffer.from(v + '\0')]); }
function kvInt(k, v) { const b = Buffer.alloc(4); b.writeInt32LE(v); return Buffer.concat([Buffer.from([0x02]), Buffer.from(k + '\0'), b]); }
function kvObj(k, parts) { return Buffer.concat([Buffer.from([0x00]), Buffer.from(k + '\0'), ...parts, Buffer.from([0x08])]); }

test('Steam packageinfo.vdf (v28): owned app ids, package 0 ignored', () => {
  const u32 = n => { const b = Buffer.alloc(4); b.writeUInt32LE(n >>> 0); return b; };
  const pkg = (id, apps) => Buffer.concat([u32(id), Buffer.alloc(20 + 4 + 8),
    kvObj(String(id), [kvObj('appids', apps.map((a, i) => kvInt(String(i), a)))]), Buffer.from([0x08])]);
  const file = path.join(tmp, 'packageinfo.vdf');
  fs.writeFileSync(file, Buffer.concat([u32(0x06565528), u32(1), pkg(0, [7]), pkg(100, [570, 620]), pkg(200, [440]), u32(0xffffffff)]));
  assert.deepEqual([...steamcache.readOwnedAppIds(file)].sort((a, b) => a - b), [440, 570, 620]);
});

test('Steam appinfo.vdf (v28): name, type, OS and art', () => {
  const u32 = n => { const b = Buffer.alloc(4); b.writeUInt32LE(n >>> 0); return b; };
  const entry = (id, kv) => {
    const body = Buffer.concat([Buffer.alloc(4 + 4 + 8 + 20 + 4 + 20), kv, Buffer.from([0x08])]);
    return Buffer.concat([u32(id), u32(body.length), body]);
  };
  const common = kvObj('appinfo', [kvObj('common', [
    kvString('name', ' Dota 2 '), kvString('type', 'Game'), kvString('oslist', 'windows,linux'),
    kvObj('library_assets_full', [kvObj('library_capsule', [kvObj('image', [kvString('english', 'abc/library_capsule.jpg')])])]),
  ])]);
  const file = path.join(tmp, 'appinfo.vdf');
  fs.writeFileSync(file, Buffer.concat([u32(0x07564428), u32(1), entry(570, common), u32(0)]));
  const info = steamcache.readAppInfo(file).get(570);
  assert.equal(info.name, 'Dota 2');
  assert.equal(info.type, 'Game');
  assert.equal(info.osList, 'windows,linux');
  assert.equal(info.assets.cover, 'abc/library_capsule.jpg');
});

test('performance: CPU share between samples and memory', () => {
  perf.cpuPercent('cpu  100 0 100 800 0 0 0 0 0 0\n');
  assert.equal(perf.cpuPercent('cpu  150 0 150 900 0 0 0 0 0 0\n'), 50);
  assert.deepEqual(perf.memory('MemTotal:       16384000 kB\nMemAvailable:    8192000 kB\n'), { ramUsed: 7.8, ramTotal: 15.6 });
  assert.deepEqual(perf.gpu(path.join(tmp, 'no-drm')), { gpu: null, vramUsed: null });
});
