'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { EventEmitter } = require('node:events');
const { Importer } = require('../src/importer');

function fakes({ epicSignedIn = false, result }) {
  const data = {};
  const store = { data, update: f => f(data) };
  const accounts = new EventEmitter();
  accounts.installRoot = '/home/a/Games';
  accounts.epic = { signedIn: epicSignedIn, imported: [], importGame: async (app, dir) => { accounts.epic.imported.push([app, dir]); } };
  const library = { store, accounts, get: async () => [] };
  const calls = [];
  const admin = {
    canAdmin: true,
    run: async (args, opts) => {
      calls.push({ args, input: opts && opts.input });
      if (args[0] === 'import-scan') return { free: 1e11, drives: [] };
      if (opts && opts.onStage) { opts.onStage({ phase: 'start', total: 100, n: 2 }); opts.onStage({ phase: 'copy', i: 0, n: 2, name: 'X', done: 50, size: 100, bps: 1e8, overall: 50, total: 100 }); }
      return result;
    },
  };
  return { admin, library, calls, data, accounts };
}

test('import: scan, then copy registers GOG right away and holds Epic until you sign in', async () => {
  const f = fakes({ result: { done: [{ key: 'a' }, { key: 'b' }], steam: 0, gog: [{ id: '42', path: '/home/a/Games/GOG/W' }], epic: [{ app: 'Fn', path: '/home/a/Games/Epic/F' }] } });
  const imp = new Importer({ admin: f.admin, library: f.library });
  const s = await imp.scan();
  assert.strictEqual(s.scan.free, 1e11);
  imp.start(['a', 'b'], 'copy');
  await new Promise(r => setTimeout(r, 20));
  assert.deepStrictEqual(JSON.parse(f.calls[1].input), { mode: 'copy', items: ['a', 'b'], root: '/home/a/Games' });
  assert.strictEqual(imp.status().state, 'idle');
  assert.strictEqual(f.data.gogInstalls['42'], '/home/a/Games/GOG/W');
  assert.strictEqual(f.data.pendingEpicImports.length, 1);
  assert.match(imp.status().result.notes.join(' '), /Sign in to Epic/);
  assert.strictEqual(imp.status().speeds.length, 1);
  // Signing in to Epic finishes it.
  f.accounts.epic.signedIn = true;
  f.accounts.emit('changed');
  await new Promise(r => setTimeout(r, 20));
  assert.deepStrictEqual(f.accounts.epic.imported, [['Fn', '/home/a/Games/Epic/F']]);
  assert.strictEqual(f.data.pendingEpicImports.length, 0);
});

test('import: nothing picked or busy is refused', async () => {
  const f = fakes({ result: {} });
  const imp = new Importer({ admin: f.admin, library: f.library });
  assert.throws(() => imp.start([], 'copy'), /Pick something/);
  assert.throws(() => imp.start(['a'], 'wipe'), /Copy or move/);
});
