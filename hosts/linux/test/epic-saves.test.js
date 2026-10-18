'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const es = require('../src/stores/epic-saves');

test('Epic save folders are filled in for the game\'s own prefix', () => {
  const prefix = '/p/epic-Anemone';
  assert.strictEqual(es.resolve('{AppData}/Anemone/Saved/SaveGames', { prefix }),
    '/p/epic-Anemone/drive_c/users/steamuser/AppData/Local/Anemone/Saved/SaveGames');
  assert.strictEqual(es.resolve('{UserDir}\\My Games\\Foo\\{EpicID}', { prefix, accountId: 'abc' }),
    '/p/epic-Anemone/drive_c/users/steamuser/Documents/My Games/Foo/abc');
  assert.strictEqual(es.resolve('{InstallDir}/Saves', { prefix, installDir: '/games/Foo' }), '/games/Foo/Saves');
  assert.strictEqual(es.resolve('{Mystery}/x', { prefix }), null);
  assert.strictEqual(es.resolve('{EpicID}/x', { prefix }), null);   // no account id: don't guess
});

test('a sync plan from legendary\'s own files, and its arguments', () => {
  const cfg = fs.mkdtempSync(path.join(os.tmpdir(), 'lgd-'));
  fs.mkdirSync(path.join(cfg, 'metadata'));
  fs.writeFileSync(path.join(cfg, 'metadata', 'Anemone.json'), JSON.stringify({ metadata: { customAttributes: { CloudSaveFolder: { type: 'STRING', value: '{AppData}/Anemone/Saved' } } } }));
  fs.writeFileSync(path.join(cfg, 'metadata', 'NoSaves.json'), JSON.stringify({ metadata: { customAttributes: {} } }));
  fs.writeFileSync(path.join(cfg, 'installed.json'), JSON.stringify({ Anemone: { install_path: '/g/A' } }));
  fs.writeFileSync(path.join(cfg, 'user.json'), JSON.stringify({ account_id: 'acc1' }));
  const p = es.plan('Anemone', { configDir: cfg, prefix: '/p/x' });
  assert.deepStrictEqual(p, { app: 'Anemone', savePath: '/p/x/drive_c/users/steamuser/AppData/Local/Anemone/Saved' });
  assert.strictEqual(es.plan('NoSaves', { configDir: cfg, prefix: '/p/x' }), null);
  assert.deepStrictEqual(es.args(p, 'down'), ['-y', 'sync-saves', 'Anemone', '--save-path', p.savePath, '--skip-upload']);
  assert.deepStrictEqual(es.args(p, 'up').slice(-1), ['--skip-download']);
});
