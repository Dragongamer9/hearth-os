'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const emu = require('../src/library/emulators');

test('ROM titles and libretro thumbnail names', () => {
  assert.strictEqual(emu.cleanTitle('Legend of Zelda, The - A Link to the Past (USA) [!].sfc'), 'The Legend of Zelda - A Link to the Past');
  assert.strictEqual(emu.cleanTitle('Super Mario World (USA).sfc'), 'Super Mario World');
  assert.strictEqual(emu.thumbName('Sonic & Knuckles (World).md'), 'Sonic _ Knuckles (World)');
  const snes = emu.SYSTEMS.find(s => s.dirs[0] === 'snes');
  assert.strictEqual(emu.thumbUrl(snes, '/r/Super Mario World (USA).sfc', 'Named_Boxarts'),
    'https://thumbnails.libretro.com/Nintendo%20-%20Super%20Nintendo%20Entertainment%20System/Named_Boxarts/Super%20Mario%20World%20(USA).png');
});

test('scanning a ROMs folder: systems by folder name, playlists and .cue tracks counted once', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'roms-'));
  const put = (rel, text = 'x') => { const f = path.join(root, rel); fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, text); };
  put('snes/Super Mario World (USA).sfc');
  put('snes/readme.txt');
  put('psx/Final Fantasy VII (USA) (Disc 1).chd');
  put('psx/Final Fantasy VII (USA) (Disc 2).chd');
  put('psx/Final Fantasy VII (USA).m3u', 'Final Fantasy VII (USA) (Disc 1).chd\nFinal Fantasy VII (USA) (Disc 2).chd\n');
  put('psx/Crash Bandicoot (USA).cue');
  put('psx/Crash Bandicoot (USA).bin');
  put('gba/Collections/Metroid Fusion (USA).gba');
  put('gba/media/Metroid Fusion (USA).gba');   // EmuDeck's art folder, not a game
  put('unknown/thing.bin');
  const games = emu.scan({ roots: [root] });
  assert.deepStrictEqual(games.map(g => g.title).sort(), ['Crash Bandicoot', 'Final Fantasy VII', 'Metroid Fusion', 'Super Mario World']);
  const ff = games.find(g => g.title === 'Final Fantasy VII');
  assert.match(ff.rom, /\.m3u$/);
  assert.strictEqual(ff.system, 'PlayStation');
  assert.strictEqual(ff.id, 'emu:psx:Final Fantasy VII (USA).m3u');
  assert.ok(games.every(g => g.source === 'emu' && g.installed));
});

test('finds the first core RetroArch has for a system', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cores-'));
  const psx = emu.SYSTEMS.find(s => s.dirs[0] === 'psx');
  assert.strictEqual(emu.findCore(psx, [dir]), null);
  fs.writeFileSync(path.join(dir, 'pcsx_rearmed_libretro.so'), '');
  fs.writeFileSync(path.join(dir, 'swanstation_libretro.so'), '');
  assert.strictEqual(emu.findCore(psx, [dir]), path.join(dir, 'swanstation_libretro.so'));
});
