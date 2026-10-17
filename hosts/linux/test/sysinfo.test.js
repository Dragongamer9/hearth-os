'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { info, osRelease, gpus } = require('../src/sysinfo');

test('About reads the OS, device and graphics from the system', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sysinfo-'));
  const w = (f, t) => { fs.mkdirSync(path.dirname(path.join(root, f)), { recursive: true }); fs.writeFileSync(path.join(root, f), t); };
  w('/etc/os-release', 'NAME="Hearth"\nVERSION="44.20261017"\nPRETTY_NAME="Hearth (based on Bazzite)"\n');
  w('/sys/class/dmi/id/sys_vendor', 'ASUSTeK COMPUTER INC.\n');
  w('/sys/class/dmi/id/product_name', 'ROG Xbox Ally X\n');
  w('/sys/class/drm/card1/device/vendor', '0x1002\n');
  w('/sys/class/drm/card1/device/mem_info_vram_total', String(4 * 2 ** 30));
  assert.strictEqual(osRelease(root).PRETTY_NAME, 'Hearth (based on Bazzite)');
  assert.deepStrictEqual(gpus(root), ['AMD (4 GB)']);
  const i = info({ root });
  assert.strictEqual(i.device, 'ASUS ROG Xbox Ally X');
  assert.strictEqual(i.os, 'Hearth');
  assert.ok(i.memoryBytes > 0 && i.storage.totalBytes > 0);
});
