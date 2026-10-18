'use strict';
const test = require('node:test');
const assert = require('node:assert');

test('which Bluetooth devices count as controllers (paired automatically on the first screen)', () => {
  const { isGamepad } = require('../src/system');
  assert.ok(isGamepad({ kind: 'Input.Gamepad', name: 'Mystery pad' }));
  for (const n of ['Xbox Wireless Controller', 'DualSense Wireless Controller', 'Pro Controller', '8BitDo Ultimate 2C', 'Joy-Con (L)'])
    assert.ok(isGamepad({ kind: 'Other', name: n }), n);
  for (const n of ['JBL Flip 5', 'Logitech K380', 'Galaxy S24'])
    assert.ok(!isGamepad({ kind: 'Other', name: n }), n);
});
