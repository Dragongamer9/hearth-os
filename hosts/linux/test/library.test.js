'use strict';
// Scanner and helper tests against fake Steam / Heroic folders. Run with: npm test (no Electron needed).

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'nimbus-test-'));
process.env.XDG_DATA_HOME = path.join(tmp, 'data');   // keep test state out of the real ~/.local/share
process.env.XDG_CONFIG_HOME = path.join(tmp, 'home', '.config');

const vdf = require('../src/library/vdf');
const steam = require('../src/library/steam');
const heroic = require('../src/library/heroic');
const xcloud = require('../src/library/xcloud');
const proc = require('../src/library/proc');
const manual = require('../src/library/manual');
const { hasBit, guidePresses, EVENT_SIZE } = require('../src/guide');
const { nmFields, bars } = require('../src/system');
const { newer } = require('../src/updates');

const write = (p, text) => { fs.mkdirSync(path.dirname(p), { recursive: true }); fs.writeFileSync(p, text); };

test('vdf parses nested objects, escapes, comments and case-insensitive lookups', () => {
  const d = vdf.parse('"AppState"\n{\n  // comment\n  "appid" "570"\n  "name" "Dota \\"2\\""\n  "UserConfig" { "language" "english" }\n}');
  assert.equal(vdf.str(d, 'appstate', 'APPID'), '570');
  assert.equal(vdf.str(d, 'AppState', 'name'), 'Dota "2"');
  assert.equal(vdf.str(d, 'AppState', 'userconfig', 'language'), 'english');
  assert.equal(vdf.get(d, 'AppState', 'missing'), undefined);
});

test('steam: finds installed and downloading games across libraries, with stats and art', () => {
  const home = path.join(tmp, 'home');
  const steamDir = path.join(home, '.local', 'share', 'Steam');
  const extra = path.join(tmp, 'games', 'SteamLibrary');
  write(path.join(steamDir, 'steamapps', 'libraryfolders.vdf'),
    `"libraryfolders" { "0" { "path" "${steamDir}" } "1" { "path" "${extra}" } }`);
  write(path.join(steamDir, 'steamapps', 'appmanifest_570.acf'),
    '"AppState" { "appid" "570" "name" "Dota 2" "installdir" "dota 2 beta" "StateFlags" "4" "SizeOnDisk" "1000" }');
  write(path.join(steamDir, 'steamapps', 'appmanifest_1493710.acf'),
    '"AppState" { "appid" "1493710" "name" "Proton Experimental" "installdir" "Proton - Experimental" "StateFlags" "4" }');
  write(path.join(extra, 'steamapps', 'appmanifest_620.acf'),
    '"AppState" { "appid" "620" "name" "Portal 2" "installdir" "Portal 2" "StateFlags" "1026" "BytesToDownload" "200" "BytesDownloaded" "50" }');
  fs.mkdirSync(path.join(steamDir, 'steamapps', 'common', 'dota 2 beta'), { recursive: true });
  // Most recent user: 76561197960265729 → account id 1
  write(path.join(steamDir, 'config', 'loginusers.vdf'), '"users" { "76561197960265729" { "MostRecent" "1" } }');
  write(path.join(steamDir, 'userdata', '1', 'config', 'localconfig.vdf'),
    '"UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "570" { "Playtime" "125" "LastPlayed" "1700000000" } } } } } }');
  write(path.join(steamDir, 'userdata', '1', 'config', 'grid', '570p.png'), 'x');
  write(path.join(steamDir, 'appcache', 'librarycache', '570', 'abc', 'library_hero.jpg'), 'x');

  const games = steam.scan({ home });
  const ids = games.map(g => g.id).sort();
  assert.deepEqual(ids, ['steam:570', 'steam:620']); // Proton is skipped
  const dota = games.find(g => g.id === 'steam:570');
  assert.equal(dota.state, 'installed');
  assert.equal(dota.storeMinutes, 125);
  assert.equal(dota.storeLastPlayed, 1700000000000);
  assert.match(dota.art.cover, /grid\/570p\.png$/);
  assert.match(dota.art.hero, /librarycache\/570\/abc\/library_hero\.jpg$/);
  assert.match(dota.art.logo, /^https:\/\/cdn/);          // nothing local: CDN fallback
  assert.deepEqual(dota.track, { steamAppId: '570' });
  const portal = games.find(g => g.id === 'steam:620');
  assert.equal(portal.installed, false);
  assert.equal(portal.state, 'downloading');
  assert.equal(portal.progress, 0.25);
});

test('heroic: Epic and GOG installs with titles and art from the store caches', () => {
  const home = path.join(tmp, 'home');
  const h = path.join(home, '.config', 'heroic');
  write(path.join(h, 'legendaryConfig', 'legendary', 'installed.json'), JSON.stringify({
    Quail: { app_name: 'Quail', title: 'Hades', install_path: '/games/Hades', install_size: 1234 },
    QuailDLC: { app_name: 'QuailDLC', title: 'DLC', is_dlc: true },
  }));
  write(path.join(h, 'store_cache', 'legendary_library.json'), JSON.stringify({
    library: [{ app_name: 'Quail', title: 'Hades', art_square: '//cdn/tall.jpg', art_cover: 'https://cdn/wide.jpg' }],
  }));
  write(path.join(h, 'gog_store', 'installed.json'), JSON.stringify({
    installed: [{ appName: '1207658924', install_path: '/games/Witcher', install_size: '2.5 GiB' }],
  }));
  write(path.join(h, 'store_cache', 'gog_library.json'), JSON.stringify({ games: [{ app_name: '1207658924', title: 'The Witcher' }] }));

  const epic = heroic.scanEpic({ home });
  assert.equal(epic.length, 1);
  assert.equal(epic[0].title, 'Hades');
  assert.equal(epic[0].art.cover, 'https://cdn/tall.jpg');
  assert.equal(epic[0].art.wide, 'https://cdn/wide.jpg');
  assert.match(epic[0].launchUri, /^heroic:\/\/launch\?appName=Quail&runner=legendary/);
  const gog = heroic.scanGog({ home });
  assert.equal(gog[0].title, 'The Witcher');
  assert.equal(gog[0].sizeBytes, Math.round(2.5 * 1024 ** 3));
  assert.deepEqual(gog[0].track.dirs, ['/games/Witcher']);
});

test('xcloud: catalog becomes streamable library entries plus the Xbox Cloud Gaming tile', async () => {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push([url, init && init.method]);
    if (url.includes('/sigls/v2')) return { ok: true, json: async () => [{ siglId: 'x' }, { id: 'AAA111' }, { id: 'BBB222' }] };
    return { ok: true, json: async () => ({ Products: {
      AAA111: { ProductTitle: 'Tunic', XCloudOfferings: { CLOUDGAMING: { Programs: ['GPU'] } },
        Image_Poster: { URL: '//store-images/poster.jpg' }, Image_Tile: { URL: 'https://x/tile.jpg' } },
      BBB222: { ProductTitle: 'Console Only', XCloudOfferings: {} },
    } }) };
  };
  const games = await xcloud.scan({ force: true, fetchImpl, env: { LANG: 'en_GB.UTF-8' } });
  assert.match(calls[0][0], /market=GB/);
  assert.equal(calls[1][1], 'POST');
  assert.equal(games[0].id, 'xbox:cloud');
  assert.equal(games.length, 2);
  const tunic = games[1];
  assert.equal(tunic.cloud, true);
  assert.equal(tunic.art.cover, 'https://store-images/poster.jpg');
  assert.equal(tunic.streamUrl, 'https://play.xbox.com/products/AAA111/tunic');
  // A failed refresh falls back to the cached copy
  const cached = await xcloud.scan({ force: true, fetchImpl: async () => { throw new Error('offline'); }, env: { LANG: 'en_GB.UTF-8' } });
  assert.equal(cached.length, 2);
  assert.equal(xcloud.slug('EA SPORTS FC™ 25'), 'ea-sports-fc-25');
});

test('proc: finds a Steam game by AppId and includes its children', () => {
  const root = path.join(tmp, 'proc');
  const mk = (pid, ppid, cmd, cwd) => {
    write(path.join(root, String(pid), 'stat'), `${pid} (weird ) name) S ${ppid} 1 1`);
    write(path.join(root, String(pid), 'cmdline'), cmd.join('\0') + '\0');
    if (cwd) fs.symlinkSync(cwd, path.join(root, String(pid), 'cwd'));
  };
  mk(100, 1, ['reaper', 'SteamLaunch', 'AppId=570', '--', 'proton'], null);
  mk(101, 100, ['wine64-preloader', 'Z:\\games\\dota\\dota2.exe'], null);
  mk(102, 101, ['child'], null);
  mk(200, 1, ['/games/Witcher/witcher'], '/games/Witcher');
  mk(300, 1, ['bash'], '/home/me');
  const procs = proc.snapshot(root);
  assert.equal(procs.find(p => p.pid === 100).ppid, 1);
  assert.deepEqual(proc.gameProcesses({ steamAppId: '570' }, procs).sort(), [100, 101, 102]);
  assert.deepEqual(proc.gameProcesses({ dirs: ['/games/Witcher'] }, procs), [200]);
  assert.deepEqual(proc.gameProcesses({ dirs: ['/games/dota'] }, procs).sort(), [101, 102]); // Wine Z:\ path
  assert.deepEqual(proc.gameProcesses({ dirs: ['/games/Witch'] }, procs), []);                // prefix isn't a folder match
  // A launcher started by Wine's services (not a child of the game) belongs to the game by its Wine prefix.
  const withLauncher = [
    { pid: 300, ppid: 1, cmdline: ['/games/Fifa/fifa.exe'], cwd: '/games/Fifa', prefix: '/pfx/fifa' },
    { pid: 301, ppid: 2, cmdline: ['C:\\Program Files\\EA\\EADesktop.exe'], cwd: '/pfx/fifa/drive_c', prefix: '/pfx/fifa' },
    { pid: 302, ppid: 2, cmdline: ['C:\\other.exe'], cwd: '/x', prefix: '/pfx/other' },
    { pid: 303, ppid: 2, cmdline: ['C:\\shared.exe'], cwd: '/x', prefix: '/home/u/.wine' },
  ];
  assert.deepEqual(proc.gameProcesses({ dirs: ['/games/Fifa'] }, withLauncher).sort(), [300, 301]);
});

test('manual: .desktop Exec lines and titles', () => {
  assert.deepEqual(manual.execArgv('env FOO=1 "/opt/My Game/run" --full %U'), ['env', 'FOO=1', '/opt/My Game/run', '--full']);
  const f = path.join(tmp, 'g.desktop');
  write(f, '[Desktop Entry]\nName=Celeste\nExec=/opt/celeste/Celeste %f\n[Desktop Action x]\nName=Other\n');
  assert.equal(manual.titleFor(f), 'Celeste');
  assert.equal(manual.titleFor('/games/Hollow_Knight.exe'), 'Hollow Knight');
  assert.equal(manual.kindOf('/a/b.EXE'), 'windows');
});

test('guide: capability bitmaps and raw input events', () => {
  // A typical Xbox pad's key bitmap: BTN_GAMEPAD (0x130) .. BTN_THUMBR (0x13e) set in word 4.
  const caps = '7cdb000000000000 0 0 0 0';
  assert.equal(hasBit(caps, 0x130), true);
  assert.equal(hasBit(caps, 0x13c), true);
  assert.equal(hasBit(caps, 0x100), false);
  const ev = (type, code, value) => { const b = Buffer.alloc(EVENT_SIZE); b.writeUInt16LE(type, 16); b.writeUInt16LE(code, 18); b.writeInt32LE(value, 20); return b; };
  assert.equal(guidePresses(Buffer.concat([ev(1, 0x13c, 1), ev(0, 0, 0), ev(1, 0x13c, 0), ev(1, 0x130, 1)])), 1);
});

test('system: nmcli field splitting and signal bars', () => {
  assert.deepEqual(nmFields('*:Cafe\\: Free:72:WPA2'), ['*', 'Cafe: Free', '72', 'WPA2']);
  assert.deepEqual([bars(90), bars(60), bars(30), bars(5), bars(0)], [4, 3, 2, 1, 0]);
});

test('updates: version comparison, including previews', () => {
  assert.equal(newer('0.3.10', '0.3.9'), true);
  assert.equal(newer('0.3.1', '0.3.1'), false);
  assert.equal(newer('1.0', '0.9.9'), true);
  assert.equal(newer('v0.4.0-linux.15', '0.4.0-linux.13'), true);
  assert.equal(newer('0.4.0-linux.13', '0.4.0-linux.15'), false);
  assert.equal(newer('0.4.0-linux.13', '0.4.0'), true);    // previews are built on top of 0.4.0
  assert.equal(newer('0.4.1', '0.4.0-linux.13'), true);    // the next full release beats them
  assert.equal(newer('v0.4.0', '0.4.0-linux.13'), false);
});
