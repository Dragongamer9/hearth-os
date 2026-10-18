'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { apply, parseEnv, splitArgs, protonChoices, kindOf, GameOptions } = require('../src/library/game-options');

test('environment variables and launch options are read like a shell would', () => {
  assert.deepStrictEqual(parseEnv('DXVK_HUD=fps PROTON_LOG=1 NAME="two words" junk'), { DXVK_HUD: 'fps', PROTON_LOG: '1', NAME: 'two words' });
  assert.deepStrictEqual(splitArgs('-skipintro -windowed "-name=Big Boss"'), ['-skipintro', '-windowed', '-name=Big Boss']);
});

test('settings change how a Proton game starts', () => {
  const r = apply({ proton: '/p/GE-Proton10-4', args: '-dx11', env: 'A=1', mangohud: true, fsr: true },
    { cmd: 'umu-run', args: ['game.exe'], env: { X: 'y' } }, { proton: true });
  assert.strictEqual(r.cmd, 'umu-run');
  assert.deepStrictEqual(r.args, ['game.exe', '-dx11']);
  assert.deepStrictEqual(r.env, { X: 'y', A: '1', PROTONPATH: '/p/GE-Proton10-4', WINE_FULLSCREEN_FSR: '1', MANGOHUD: '1' });
});

test('Proton-only settings do nothing for Linux games', () => {
  const r = apply({ proton: 'GE-Proton', fsr: true }, { cmd: '/games/thing', args: [] }, { proton: false });
  assert.deepStrictEqual(r.env, {});
});

test('installed Proton versions are offered after the defaults', () => {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), 'proton-'));
  const ge = path.join(home, '.local/share/Steam/compatibilitytools.d/GE-Proton10-4');
  fs.mkdirSync(ge, { recursive: true }); fs.writeFileSync(path.join(ge, 'proton'), '');
  fs.mkdirSync(path.join(home, '.local/share/Steam/compatibilitytools.d/not-proton'), { recursive: true });
  const c = protonChoices(home);
  assert.deepStrictEqual(c.map(x => x.label), ['Default', 'GE-Proton (newest)', 'GE-Proton10-4']);
  assert.strictEqual(c[2].id, ge);
});

test('which games get which settings', () => {
  assert.strictEqual(kindOf({ source: 'steam' }), 'steam');
  assert.strictEqual(kindOf({ source: 'epic', launchVia: 'legendary' }), 'proton');
  assert.strictEqual(kindOf({ source: 'manual', exe: '/g/setup.EXE' }), 'proton');
  assert.strictEqual(kindOf({ source: 'manual', exe: '/g/game.AppImage' }), 'native');
  assert.strictEqual(kindOf({ source: 'xbox', cloud: true }), 'cloud');
  assert.strictEqual(kindOf({ source: 'gog' }), 'external');
});

test('settings are kept per game, only known keys', () => {
  const data = {};
  const o = new GameOptions({ data, update(fn) { fn(data); } });
  o.set('epic:x', { mangohud: 1, proton: 'GE-Proton', bogus: 'no' });
  assert.deepStrictEqual(o.get('epic:x'), { proton: 'GE-Proton', args: '', env: '', mangohud: true, gamemode: false, fsr: false, fpsLimit: 0, tdp: 0, cloudSaves: true });
  assert.strictEqual(o.get('other').mangohud, false);
  o.set('epic:x', { fpsLimit: '45', tdp: 999 });
  assert.strictEqual(o.get('epic:x').fpsLimit, 45);
  assert.strictEqual(o.get('epic:x').tdp, 120);
});
