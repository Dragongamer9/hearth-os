'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const po = require('../src/perf-overlay');

test('performance overlay writes mangoapp layouts, off by default', () => {
  const f = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'po-')), 'mangohud.conf');
  const env = { MANGOHUD_CONFIGFILE: f };
  assert.strictEqual(po.supported(env), false);
  fs.writeFileSync(f, 'no_display\n');
  assert.strictEqual(po.supported(env), true);
  assert.strictEqual(po.set(1, env), 1);
  assert.match(fs.readFileSync(f, 'utf8'), /fps_only/);
  assert.strictEqual(po.set(99, env), 4);
  assert.match(fs.readFileSync(f, 'utf8'), /gpu_power/);
  po.set(0, env);
  assert.strictEqual(fs.readFileSync(f, 'utf8'), 'no_display\n');
  assert.throws(() => po.set(1, {}), /Game Mode/);
});
