'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { FocusOwner } = require('../src/focus');
const sleep = ms => new Promise(r => setTimeout(r, ms));

test('changes run one at a time, in order', async () => {
  const f = new FocusOwner(), seen = [];
  const a = f.run('a', async () => { await sleep(20); seen.push('a'); }, { soft: true });
  const b = f.run('b', async () => { seen.push('b'); });
  assert.deepStrictEqual(await Promise.all([a, b]), [false, true]);   // a was soft: b, asked later, wins
  assert.deepStrictEqual(seen, ['b']);
});

test('a soft hand-back is dropped when something is chosen meanwhile', async () => {
  const f = new FocusOwner(), seen = [];
  const back = f.run('hand back', () => seen.push('old app'), { delay: 30, soft: true });
  f.claim();   // "Back to Hearth" pressed
  assert.strictEqual(await back, false);
  assert.deepStrictEqual(seen, []);
});

test('a soft hand-back runs when nothing else happened', async () => {
  const f = new FocusOwner(), seen = [];
  assert.strictEqual(await f.run('hand back', () => seen.push('game'), { delay: 10, soft: true }), true);
  assert.deepStrictEqual(seen, ['game']);
});

test('a long game search stops once something newer is asked for', async () => {
  const f = new FocusOwner();
  let tries = 0;
  const search = f.run('game', async still => { while (still() && tries < 50) { tries++; await sleep(5); } });
  await sleep(22);
  await f.run('hearth', () => {});
  await search;
  assert.ok(tries < 10, `kept searching ${tries} times`);
});

test('a failing change does not block the next one', async () => {
  const f = new FocusOwner();
  assert.strictEqual(await f.run('bad', () => { throw new Error('xprop missing'); }), false);
  assert.strictEqual(await f.run('good', () => {}), true);
});
