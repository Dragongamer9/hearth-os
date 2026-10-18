'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { ArtCache } = require('../src/art-cache');

test('art cache: downloads web art once, then serves it from the device', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'art-'));
  let calls = 0;
  const png = Buffer.alloc(200, 1);
  const fetchFn = async u => { calls++; if (u.includes('broken')) throw new Error('fetch failed');
    return { ok: true, status: 200, headers: { get: () => 'image/png' }, arrayBuffer: async () => png }; };
  const c = new ArtCache(dir, { fetchFn, concurrency: 2 });
  const u = 'https://cdn.example/cover.jpg';
  assert.strictEqual(c.url(u), u);                                   // not yet: web address, queued
  assert.strictEqual(c.url('/home/x/cover.jpg'), '/home/x/cover.jpg'); // local files pass through
  c.url('https://cdn.example/broken.jpg');
  await new Promise(r => setTimeout(r, 50));
  const local = c.url(u);
  assert.match(local, /^nimbus-art:\/\/cache\/[0-9a-f]{24}$/);
  assert.ok(fs.existsSync(c.file(local.split('/').pop())));
  assert.ok(c.file(local.split('/').pop()).endsWith('.png'));
  c.url('https://cdn.example/broken.jpg');                           // failed recently: not tried again yet
  await new Promise(r => setTimeout(r, 20));
  assert.strictEqual(calls, 2);
  assert.strictEqual(new ArtCache(dir, { fetchFn }).url(u), local);  // still there after a restart
  assert.strictEqual(c.file('../etc/passwd'), null);
});
