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

test('battery: plugged in at a charge limit is not "on battery"; a battery without a level is skipped', () => {
  const fs = require('node:fs'), os = require('node:os'), path = require('node:path');
  const { Power } = require('../src/system');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-batt-'));
  const put = (n, f, v) => { fs.mkdirSync(path.join(root, n), { recursive: true }); fs.writeFileSync(path.join(root, n, f), v); };
  put('BAT0', 'type', 'Battery\n'); put('BAT0', 'status', 'Unknown\n');
  put('BAT1', 'type', 'Battery\n'); put('BAT1', 'capacity', '80\n'); put('BAT1', 'status', 'Not charging\n');
  assert.deepStrictEqual(new Power(root).battery(), { present: true, pct: 80, charging: true });
  put('BAT1', 'status', 'Discharging\n');
  assert.strictEqual(new Power(root).battery().charging, false);
  fs.rmSync(root, { recursive: true, force: true });
});
