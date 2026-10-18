'use strict';
const test = require('node:test');
const assert = require('node:assert');
const col = require('../src/library/collections');

test('collections: make, fill, rename, remove', () => {
  const data = {};
  const store = { data, update(fn) { fn(data); } };
  const a = col.create(store, '  Couch co-op ');
  assert.strictEqual(a.name, 'Couch co-op');
  assert.throws(() => col.create(store, 'couch CO-OP'), /already/);
  assert.throws(() => col.create(store, '  '), /name/);
  col.setGame(store, a.id, 'steam:1', true);
  col.setGame(store, a.id, 'steam:1', true);
  col.setGame(store, a.id, 'gog:2', true);
  assert.deepStrictEqual(col.of(data, 'steam:1'), [a.id]);
  assert.deepStrictEqual(col.summary(data), [{ id: a.id, name: 'Couch co-op', count: 2 }]);
  col.setGame(store, a.id, 'steam:1', false);
  col.rename(store, a.id, 'Party');
  assert.deepStrictEqual(col.summary(data), [{ id: a.id, name: 'Party', count: 1 }]);
  col.remove(store, a.id);
  assert.deepStrictEqual(col.summary(data), []);
});
