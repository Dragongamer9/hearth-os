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
