'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { EventEmitter } = require('node:events');
const { RemotePlay } = require('../src/remote-play');

// A pretend Sunshine: answers like its web API does.
function fakeSunshine(state) {
  return (opts, cb) => {
    const req = new EventEmitter(); let body = '';
    req.write = d => { body += d; }; req.destroy = e => req.emit('error', e);
    req.end = () => setImmediate(() => {
      state.calls.push({ path: opts.path, auth: opts.headers.Authorization || null, body: body ? JSON.parse(body) : null });
      const res = new EventEmitter(); res.setEncoding = () => {};
      const ok = auth => state.creds && auth === 'Basic ' + Buffer.from(state.creds).toString('base64');
      let code = 200, out = {};
      if (opts.path === '/api/password') { if (state.creds) code = 401; else { const b = JSON.parse(body); state.creds = `${b.newUsername}:${b.newPassword}`; out = { status: true }; } }
      else if (!ok(opts.headers.Authorization)) code = 401;
      else if (opts.path === '/api/pin') out = { status: JSON.parse(body).pin === '4321' };
      else if (opts.path === '/api/clients/list') out = { named_certs: [{ name: 'Living room TV', uuid: 'u1' }], status: true };
      res.statusCode = code; cb(res); res.emit('data', JSON.stringify(out)); res.emit('end');
    });
    return req;
  };
}

test('remote play: Hearth sets Sunshine up with its own login and pairs Moonlight with the PIN', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rp-'));
  const state = { creds: null, calls: [] };
  const rp = new RemotePlay({ dir, request: fakeSunshine(state) });
  assert.strictEqual(await rp.ensureLogin(), true);
  const saved = JSON.parse(fs.readFileSync(path.join(dir, 'remote-play.json'), 'utf8'));
  assert.strictEqual(state.creds, `hearth:${saved.pass}`);
  assert.ok(saved.pass.length >= 20);
  assert.strictEqual(fs.statSync(path.join(dir, 'remote-play.json')).mode & 0o777, 0o600);
  await assert.rejects(rp.pair('12'), /4 numbers/);
  await assert.rejects(rp.pair('1111', 'TV'), /didn't work/);
  assert.strictEqual(await rp.pair('4321', 'TV'), true);
  assert.deepStrictEqual(state.calls.at(-1).body, { pin: '4321', name: 'TV' });
  const list = await rp.call('GET', '/api/clients/list');
  assert.strictEqual(list.named_certs[0].name, 'Living room TV');
});

test('remote play: Sunshine that already had a login asks for it once', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rp-'));
  const state = { creds: 'me:secret', calls: [] };
  const rp = new RemotePlay({ dir, request: fakeSunshine(state) });
  assert.strictEqual(await rp.ensureLogin(), false);
  await assert.rejects(rp.login('me', 'wrong'), /username and password/);
  assert.strictEqual(await rp.login('me', 'secret'), true);
  assert.strictEqual(await rp.pair('4321', 'Phone'), true);
});
