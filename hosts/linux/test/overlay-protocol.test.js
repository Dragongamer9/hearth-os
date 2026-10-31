'use strict';
// Hearth's Quick Access as Handheld Daemon's overlay app (src/overlay): HHD's stdin/stdout lines, what goes to Hearth
// and to the page, the socket to Hearth, and the art it serves.
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const net = require('node:net');
const { OverlayProtocol, padFor, GRAB_EVERY, PENDING_MS } = require('../src/overlay/protocol');
const { HearthLink, socketPath, splitLines } = require('../src/overlay/link');
const { artFile } = require('../src/overlay/art');

/** A protocol with fake clocks and timers: everything it says to HHD, Hearth and the page is recorded. */
function rig({ connected = true } = {}) {
  const r = { out: [], hearth: [], page: [], host: [], t: 10000, timers: [], repeats: new Set(), connected };
  r.p = new OverlayProtocol({
    out: l => r.out.push(l), toHearth: m => r.hearth.push(m), toPage: m => r.page.push(m), onHost: m => r.host.push(m),
    connected: () => r.connected, now: () => r.t,
    setTimer: (fn, ms) => { const t = { fn, at: r.t + ms }; r.timers.push(t); return t; },
    clearTimer: t => { r.timers = r.timers.filter(x => x !== t); },
    every: (fn, ms) => { const h = { fn, ms }; r.repeats.add(h); return h; },
    stopEvery: h => r.repeats.delete(h),
  });
  r.advance = ms => { r.t += ms; for (const t of r.timers.filter(x => x.at <= r.t)) { r.timers = r.timers.filter(x => x !== t); t.fn(); } };
  r.shown = () => r.p.fromHearth({ type: 'event', name: 'overlay.shown', data: {} });
  r.hidden = () => r.p.fromHearth({ type: 'event', name: 'overlay.hidden', data: {} });
  r.clear = () => { r.out.length = 0; r.hearth.length = 0; r.page.length = 0; };
  return r;
}
const OPEN = { type: 'hhd', action: 'open', how: 'qam' };
const CLOSE = { type: 'overlay', action: 'close' };

test("HHD's action names become the page's controller names", () => {
  for (const b of ['a', 'b', 'x', 'y', 'lb', 'rb', 'up', 'down', 'left', 'right', 'select']) assert.strictEqual(padFor(b), b);
  assert.strictEqual(padFor('mode'), 'guide');
  assert.strictEqual(padFor('x_up'), null);      // X released: nothing in Hearth uses it
  assert.strictEqual(padFor('toString'), null);
  assert.strictEqual(padFor('rt'), null);
});

test('QAM asks Hearth; Quick Access opens (stat + grab) only when Hearth says overlay.shown', () => {
  const r = rig();
  r.p.line('cmd:open_qam');
  assert.deepStrictEqual(r.hearth, [OPEN]);
  assert.deepStrictEqual(r.out, []);
  assert.strictEqual(r.p.state, 'closed');
  r.shown();
  assert.deepStrictEqual(r.out, ['stat:qam', 'grab:enable']);
  assert.deepStrictEqual(r.page.map(m => m.name), ['overlay.shown', 'padHost']);
  assert.strictEqual(r.page[1].data.connected, 1);   // the page mustn't read the controller itself
  assert.strictEqual(r.repeats.size, 1);
  assert.strictEqual([...r.repeats][0].ms, GRAB_EVERY);
  [...r.repeats][0].fn();
  assert.deepStrictEqual(r.out.slice(-1), ['grab:enable']);   // the grab is renewed while open (HHD drops it after 5 s)
});

test('the on-screen keyboard over the game (Guide + X) is shown and holds the controller like Quick Access', () => {
  const r = rig();
  r.p.fromHearth({ type: 'event', name: 'overlay.osk', data: {} });
  assert.deepStrictEqual(r.out, ['stat:qam', 'grab:enable']);
  assert.deepStrictEqual(r.page.map(m => m.name), ['overlay.osk', 'padHost']);
  r.hidden();
  assert.deepStrictEqual(r.out.slice(-2), ['stat:closed', 'grab:disable']);
});

test('controller lines go to the page only while open', () => {
  const r = rig();
  r.p.line('action:a');
  assert.deepStrictEqual(r.page, []);
  r.p.line('cmd:open_qam'); r.shown(); r.clear();
  for (const l of ['action:a', 'action:down', 'action:mode', 'action:x_up', 'action:b', 'action:bogus', '', 'junk']) r.p.line(l);
  assert.deepStrictEqual(r.page.map(m => [m.name, m.data.b]), [['pad', 'a'], ['pad', 'down'], ['pad', 'guide'], ['pad', 'b']]);
});

test("closing: QAM again asks Hearth to close; overlay.hidden ends it and gives the controller back", () => {
  const r = rig();
  r.p.line('cmd:open_qam'); r.shown(); r.advance(1000); r.clear();
  r.p.line('cmd:open_qam');
  assert.deepStrictEqual(r.hearth, [CLOSE]);
  assert.strictEqual(r.p.state, 'qam');   // still open until Hearth says so
  r.hidden();
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
  assert.strictEqual(r.repeats.size, 0);
  assert.deepStrictEqual(r.page.map(m => m.name), ['overlay.hidden']);
  r.hidden();   // told again (Hearth resyncs): nothing more for HHD
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
});

test('a double press: the first press opens, the second one is not a second toggle', () => {
  const r = rig();
  r.p.line('cmd:open_qam_if_closed');
  assert.deepStrictEqual(r.hearth, [OPEN]);
  r.shown(); r.advance(1000);
  r.p.line('cmd:open_qam');
  assert.deepStrictEqual(r.hearth, [OPEN]);   // no close
  assert.strictEqual(r.p.state, 'qam');
  r.p.line('cmd:open_qam');                   // a later press closes
  assert.deepStrictEqual(r.hearth, [OPEN, CLOSE]);
  // With Hearth in front, Hearth opens its own Quick Access and the overlay stays closed: the second press still
  // mustn't ask again (that would close Hearth's).
  const s = rig();
  s.p.line('cmd:open_qam_if_closed'); s.p.line('cmd:open_qam');
  assert.deepStrictEqual(s.hearth, [OPEN]);
});

test('expanded (the ROG Ally\'s Armoury Crate button) opens and closes Quick Access the same way', () => {
  const r = rig();
  r.p.line('cmd:open_expanded');
  assert.deepStrictEqual(r.hearth, [{ type: 'hhd', action: 'open', how: 'expanded' }]);
  r.shown();
  assert.deepStrictEqual(r.out, ['stat:expanded', 'grab:enable']);
  r.advance(1000);
  r.p.line('cmd:open_expanded');
  assert.deepStrictEqual(r.hearth.slice(-1), [CLOSE]);
});

test('a second open right after opening (Guide + B) does not close it again', () => {
  const r = rig();
  r.shown();   // Guide opened it through Hearth
  assert.deepStrictEqual(r.out, ['stat:qam', 'grab:enable']);
  r.advance(200);
  r.p.line('cmd:open_expanded');
  assert.deepStrictEqual(r.hearth, []);
  r.advance(800);
  r.p.line('cmd:open_expanded');
  assert.deepStrictEqual(r.hearth, [CLOSE]);
});

test('cmd:close asks Hearth; cmd:close_now closes at once too', () => {
  const r = rig();
  r.shown(); r.clear();
  r.p.line('cmd:close');
  assert.deepStrictEqual(r.hearth, [CLOSE]);
  assert.deepStrictEqual(r.out, []);
  r.p.line('cmd:close_now');
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
  assert.deepStrictEqual(r.hearth, [CLOSE, CLOSE]);
  r.p.line('cmd:mute'); r.p.line('cmd:open_notification');
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
});

test('without Hearth: no open, and anything open closes here', () => {
  const r = rig({ connected: false });
  r.p.line('cmd:open_qam');
  assert.deepStrictEqual(r.hearth, []);
  assert.strictEqual(r.p.state, 'closed');
  r.connected = true; r.shown(); r.connected = false; r.clear();
  r.p.line('cmd:close');
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
  r.connected = true; r.shown(); r.clear();
  r.p.disconnected();   // Hearth went away while open
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
});

test('a press while Hearth is away goes to it, late, when it connects (not if it was long ago, or closed again)', () => {
  const r = rig({ connected: false });
  r.p.line('cmd:open_qam');
  assert.deepStrictEqual(r.hearth, []);
  r.connected = true; r.p.hearthConnected();
  assert.deepStrictEqual(r.hearth, [{ ...OPEN, late: true }]);
  r.p.hearthConnected();   // only once
  assert.strictEqual(r.hearth.length, 1);
  r.clear(); r.connected = false;
  r.p.line('cmd:open_expanded');
  r.advance(PENDING_MS + 1);
  r.connected = true; r.p.hearthConnected();
  assert.deepStrictEqual(r.hearth, []);   // too long ago
  r.connected = false;
  r.p.line('cmd:open_qam'); r.p.line('cmd:close');
  r.connected = true; r.p.hearthConnected();
  assert.deepStrictEqual(r.hearth, []);   // closed again meanwhile
});

test('the volume bar shows over the game without taking the controller, then hides', () => {
  const r = rig();
  r.p.fromHearth({ type: 'event', name: 'osd', data: { kind: 'vol', value: 40, ms: 1000 } });
  assert.deepStrictEqual(r.out, ['stat:notification']);
  assert.deepStrictEqual(r.page.map(m => m.name), ['osd']);
  r.advance(800);
  r.p.fromHearth({ type: 'event', name: 'osd', data: { kind: 'vol', value: 45, ms: 1000 } });   // pressed again: longer
  r.advance(800);
  assert.strictEqual(r.p.state, 'notification');
  r.advance(600);
  assert.deepStrictEqual(r.out, ['stat:notification', 'stat:closed']);
  // Opening Quick Access while it shows: the controller is taken then.
  r.p.fromHearth({ type: 'event', name: 'osd', data: {} });
  r.shown();
  assert.deepStrictEqual(r.out.slice(-2), ['stat:qam', 'grab:enable']);
  r.advance(5000);
  assert.strictEqual(r.p.state, 'qam');   // the volume bar's timer doesn't close Quick Access
  // With Quick Access open the volume bar is just shown in it.
  r.clear();
  r.p.fromHearth({ type: 'event', name: 'osd', data: {} });
  assert.deepStrictEqual(r.out, []);
});

test("Hearth's other messages pass to the page; padHost always says the host reads the controller", () => {
  const r = rig();
  r.p.fromHearth({ type: 'result', id: 3, ok: true, value: 1 });
  r.p.fromHearth({ type: 'event', name: 'padHost', data: { connected: 0 } });
  r.p.fromHearth({ type: 'event', name: 'status', data: { battery: 50 } });
  r.p.fromHearth({ type: 'overlay-host', what: 'opacity', value: 0 });
  r.p.fromHearth(null); r.p.fromHearth('x');
  assert.deepStrictEqual(r.page, [{ type: 'result', id: 3, ok: true, value: 1 }, { type: 'event', name: 'padHost', data: { connected: 1 } }, { type: 'event', name: 'status', data: { battery: 50 } }]);
  assert.deepStrictEqual(r.host, [{ type: 'overlay-host', what: 'opacity', value: 0 }]);
});

test('Take a screenshot: out of sight for the picture (HHD hides it), then back, still open', () => {
  const r = rig();
  r.shown(); r.clear();
  r.p.fromHearth({ type: 'overlay-host', what: 'opacity', value: 0 });   // what Hearth sends (main.js passes it to peek)
  r.p.peek(true);
  assert.deepStrictEqual(r.out, ['stat:closed', 'grab:disable']);
  assert.strictEqual(r.repeats.size, 0);
  assert.strictEqual(r.p.state, 'qam');
  r.p.peek(true);
  assert.strictEqual(r.out.length, 2);
  r.p.peek(false);
  assert.deepStrictEqual(r.out.slice(2), ['stat:qam', 'grab:enable']);
  assert.strictEqual(r.repeats.size, 1);
  // Closed meanwhile: back means nothing.
  r.p.peek(true); r.hidden(); r.clear();
  r.p.peek(false);
  assert.deepStrictEqual(r.out, []);
  assert.strictEqual(r.repeats.size, 0);
  // Nothing open: nothing to hide.
  r.p.peek(true);
  assert.deepStrictEqual(r.out, []);
});

test("the socket's place: /run/user/<uid> first (HHD gives no XDG_RUNTIME_DIR)", () => {
  assert.strictEqual(socketPath({ XDG_RUNTIME_DIR: '/x' }, 1000, () => true), '/run/user/1000/hearth/overlay.sock');
  assert.strictEqual(socketPath({ XDG_RUNTIME_DIR: '/x' }, 1000, () => false), '/x/hearth/overlay.sock');
  assert.strictEqual(socketPath({}, 1000, () => false), '/run/user/1000/hearth/overlay.sock');
  assert.deepStrictEqual(splitLines('a\nb\nc'), [['a', 'b'], 'c']);
});

test('the link to Hearth: lines of JSON both ways, and it connects again when Hearth comes back', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-ovl-'));
  const file = path.join(dir, 'overlay.sock');
  const got = [];
  let conn = null;
  const listen = () => new Promise(ok => {
    const srv = net.createServer(c => { conn = c; c.setEncoding('utf8'); let buf = ''; c.on('data', d => { buf += d; const [ls, rest] = splitLines(buf); buf = rest; ls.forEach(l => got.push(JSON.parse(l))); }); });
    srv.listen(file, () => ok(srv));
  });
  const link = new HearthLink(file, { retryMs: 30 });
  const events = [];
  link.on('connect', () => events.push('connect'));
  link.on('disconnect', () => events.push('disconnect'));
  link.on('message', m => events.push(m));
  assert.strictEqual(link.send({ a: 1 }), false);
  link.start();
  await new Promise(r => setTimeout(r, 100));   // nobody listening yet: it keeps trying
  let srv = await listen();
  for (let i = 0; i < 50 && !link.connected; i++) await new Promise(r => setTimeout(r, 20));
  assert.ok(link.connected);
  assert.strictEqual(link.send({ type: 'hhd', action: 'open' }), true);
  conn.write('{"type":"event","name":"overlay.shown"}\n{"type":"res');
  conn.write('ult","id":1}\nnot json\n');
  for (let i = 0; i < 50 && got.length < 1; i++) await new Promise(r => setTimeout(r, 20));
  await new Promise(r => setTimeout(r, 50));
  assert.deepStrictEqual(got, [{ type: 'hhd', action: 'open' }]);
  assert.deepStrictEqual(events.filter(e => typeof e === 'object'), [{ type: 'event', name: 'overlay.shown' }, { type: 'result', id: 1 }]);
  // Hearth restarts.
  conn.destroy(); await new Promise(r => srv.close(r));
  for (let i = 0; i < 50 && link.connected; i++) await new Promise(r => setTimeout(r, 20));
  assert.ok(!link.connected);
  srv = await listen();
  for (let i = 0; i < 50 && !link.connected; i++) await new Promise(r => setTimeout(r, 20));
  assert.ok(link.connected);
  assert.deepStrictEqual(events.filter(e => typeof e === 'string'), ['connect', 'disconnect', 'connect']);
  link.stop(); await new Promise(r => srv.close(r));
  fs.rmSync(dir, { recursive: true, force: true });
});

test('art: pictures on the disk and in the art cache, nothing else', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-art-'));
  const pic = path.join(dir, 'a b', '620p.png');
  fs.mkdirSync(path.dirname(pic)); fs.writeFileSync(pic, 'png');
  fs.writeFileSync(path.join(dir, 'secret.txt'), 'no');
  const data = path.join(dir, 'data');
  fs.mkdirSync(path.join(data, 'art-cache'), { recursive: true });
  const name = 'abcdef0123456789abcdef01';
  fs.writeFileSync(path.join(data, 'art-cache', name + '.jpg'), 'jpg');
  const enc = p => 'nimbus-art://file/' + p.split('/').map(encodeURIComponent).join('/');
  assert.strictEqual(artFile(enc(pic), data), pic);
  assert.strictEqual(artFile(enc(pic) + '?v=2', data), pic);
  assert.strictEqual(artFile(enc(path.join(dir, 'secret.txt')), data), null);
  assert.strictEqual(artFile(enc(path.join(dir, 'missing.png')), data), null);
  assert.strictEqual(artFile('nimbus-art://file/' + encodeURIComponent(dir), data), null);   // a folder
  assert.strictEqual(artFile(`nimbus-art://cache/${name}`, data), path.join(data, 'art-cache', name + '.jpg'));
  assert.strictEqual(artFile('nimbus-art://cache/../../etc/passwd', data), null);
  assert.strictEqual(artFile('nimbus-art://cache/abc', data), null);
  assert.strictEqual(artFile('nimbus-art://other/x.png', data), null);
  assert.strictEqual(artFile('nimbus-art://file/%E0%A4%A', data), null);   // broken encoding
  fs.rmSync(dir, { recursive: true, force: true });
});
