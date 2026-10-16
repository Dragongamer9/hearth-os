'use strict';
// End-to-end through the bridge without Electron: messages in, results and events out,
// including launching a real (tiny) game and watching it start and exit.

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'nimbus-bridge-'));
process.env.XDG_DATA_HOME = path.join(tmp, 'data');

const { Library } = require('../src/library');
const { Store } = require('../src/library/store');
const { Bridge } = require('../src/bridge');
const { Updates } = require('../src/updates');

function setup() {
  const store = new Store(path.join(tmp, 'library.json'));
  store.update(d => { d.sources = { steam: false, epic: false, gog: false, xbox: false }; d.steamBackground = false; });
  const opened = [];
  const stream = { open: (url, title) => opened.push([url, title]), close: () => library.markExited(), isOpen: false };
  const library = new Library({ store, stream });
  const out = [];
  const game = path.join(tmp, 'Tiny Game', 'tiny.sh');
  fs.mkdirSync(path.dirname(game), { recursive: true });
  fs.writeFileSync(game, '#!/bin/sh\nsleep 3\n', { mode: 0o755 });
  const bridge = new Bridge({
    post: m => out.push(m), library, updates: new Updates({ version: '0.3.1' }),
    platform: {
      info: () => ({ platform: 'linux' }), pickGameFile: async () => game,
      screenInfo: () => ({ resolution: '1280 × 800', scale: '100%' }),
      session: { status: () => ({ running: false }) }, installUpdate() {}, returnToDesktop() {},
    },
  });
  return { bridge, library, out, opened };
}

const call = async (bridge, out, method, args = {}) => {
  const id = Math.floor(Math.random() * 1e9);
  await bridge.handle({ type: 'call', id, method, args });
  const r = out.find(m => m.type === 'result' && m.id === id);
  if (!r.ok) throw new Error(r.error);
  return r.value;
};
const waitFor = async (pred, ms = 15000) => {
  const until = Date.now() + ms;
  while (Date.now() < until) { if (pred()) return; await new Promise(r => setTimeout(r, 100)); }
  throw new Error('timed out');
};

test('bridge: add, launch, watch and record a manual game', async () => {
  const { bridge, library, out } = setup();
  try {
    assert.deepEqual(await call(bridge, out, 'host.info'), { platform: 'linux' });

    const added = await call(bridge, out, 'library.addManual');
    assert.equal(added.title, 'tiny');
    assert.equal(added.canRemove, true);

    await call(bridge, out, 'library.setFav', { id: added.id, on: true });
    const lib = await call(bridge, out, 'library.get');
    assert.equal(lib.games.length, 1);
    assert.equal(lib.games[0].fav, true);

    const r = await call(bridge, out, 'game.launch', { id: added.id });
    assert.deepEqual(r, { tracked: true });
    await waitFor(() => out.some(m => m.name === 'game.started'));
    assert.equal(library.runningId, added.id);
    await assert.rejects(call(bridge, out, 'game.launch', { id: added.id }), /already running/);
    await waitFor(() => out.some(m => m.name === 'game.exited'));
    assert.equal(library.runningId, null);
    assert.ok(library.store.data.stats[added.id].lastPlayed > 0);

    await assert.rejects(call(bridge, out, 'nope'), /Unknown method/);
    const st = await call(bridge, out, 'sys.status');           // works even with no network/audio tools
    assert.equal(typeof st.wifi.available, 'boolean');
    assert.ok(st.battery);
  } finally { bridge.dispose(); }
});

test('bridge: closing a running game stops it', async () => {
  const { bridge, library, out } = setup();
  try {
    const g = (await call(bridge, out, 'library.get', { rescan: true })).games[0];
    fs.writeFileSync(path.join(tmp, 'Tiny Game', 'tiny.sh'), '#!/bin/sh\nsleep 600\n', { mode: 0o755 });
    await call(bridge, out, 'game.launch', { id: g.id });
    await waitFor(() => out.some(m => m.name === 'game.started'));
    const r = await call(bridge, out, 'game.close');
    assert.deepEqual(r, { ok: true });
    await waitFor(() => out.some(m => m.name === 'game.exited'));
  } finally { bridge.dispose(); }
});

test('bridge: a cloud game opens the stream window and ends with it', async () => {
  const { bridge, library, out, opened } = setup();
  try {
    library.games.set('xbox:AAA', { id: 'xbox:AAA', title: 'Tunic', source: 'xbox', cloud: true, installed: false,
      state: 'cloud', art: {}, streamUrl: 'https://play.xbox.com/products/AAA/tunic' });
    await call(bridge, out, 'game.launch', { id: 'xbox:AAA' });
    assert.deepEqual(opened, [['https://play.xbox.com/products/AAA/tunic', 'Tunic']]);
    assert.ok(out.some(m => m.name === 'game.started' && m.data.id === 'xbox:AAA'));
    await call(bridge, out, 'game.close');
    assert.ok(out.some(m => m.name === 'game.exited' && m.data.id === 'xbox:AAA'));
  } finally { bridge.dispose(); }
});

test('bridge: profile and accounts', async () => {
  const { bridge, out } = setup();
  try {
    const p = await call(bridge, out, 'profile.set', { name: '  Deck Pilot  ', accent: '#ff5d5d', status: 'Playing' });
    assert.equal(p.name, 'Deck Pilot');
    assert.equal(p.accent, '#ff5d5d');
    assert.equal((await call(bridge, out, 'profile.get')).status, 'Playing');
    await call(bridge, out, 'profile.set', { accent: 'not a color' });
    assert.equal((await call(bridge, out, 'profile.get')).accent, '#ff5d5d');
    const acc = await call(bridge, out, 'accounts.status');
    assert.deepEqual(acc.accounts.map(a => a.store), ['steam', 'epic', 'gog', 'xbox']);
    await assert.rejects(call(bridge, out, 'game.uninstall', { id: 'x' }), /Steam or Heroic/);
  } finally { bridge.dispose(); }
});
