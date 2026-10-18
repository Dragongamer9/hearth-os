'use strict';
// Emulated games: ROMs in your ROMs folder, played with RetroArch. Folders are named after the system, the way
// EmuDeck and ES-DE name them (ROMs/snes, ROMs/psx, ...), so an existing EmuDeck set-up shows up as it is.
// Box art comes from the libretro thumbnail project, by the ROM's own (No-Intro / Redump) name.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { has, launchDetached } = require('../util');

const FLATPAK = 'org.libretro.RetroArch';
const THUMBS = 'https://thumbnails.libretro.com/';

/** System folder names → what plays them, which files count, and the thumbnail set's name. */
const SYSTEMS = [
  { dirs: ['nes', 'famicom'], name: 'NES', ext: ['nes', 'fds', 'unf', 'zip', '7z'], cores: ['mesen', 'nestopia', 'fceumm'], thumbs: 'Nintendo - Nintendo Entertainment System' },
  { dirs: ['snes', 'sfc'], name: 'Super Nintendo', ext: ['sfc', 'smc', 'zip', '7z'], cores: ['snes9x', 'bsnes'], thumbs: 'Nintendo - Super Nintendo Entertainment System' },
  { dirs: ['n64'], name: 'Nintendo 64', ext: ['n64', 'z64', 'v64', 'zip', '7z'], cores: ['mupen64plus_next', 'parallel_n64'], thumbs: 'Nintendo - Nintendo 64' },
  { dirs: ['gb'], name: 'Game Boy', ext: ['gb', 'zip', '7z'], cores: ['gambatte', 'sameboy', 'mgba'], thumbs: 'Nintendo - Game Boy' },
  { dirs: ['gbc'], name: 'Game Boy Color', ext: ['gbc', 'zip', '7z'], cores: ['gambatte', 'sameboy', 'mgba'], thumbs: 'Nintendo - Game Boy Color' },
  { dirs: ['gba'], name: 'Game Boy Advance', ext: ['gba', 'zip', '7z'], cores: ['mgba', 'vbam', 'gpsp'], thumbs: 'Nintendo - Game Boy Advance' },
  { dirs: ['nds'], name: 'Nintendo DS', ext: ['nds', 'zip', '7z'], cores: ['melondsds', 'melonds', 'desmume'], thumbs: 'Nintendo - Nintendo DS' },
  { dirs: ['genesis', 'megadrive'], name: 'Genesis / Mega Drive', ext: ['md', 'gen', 'smd', 'bin', 'zip', '7z'], cores: ['genesis_plus_gx', 'picodrive'], thumbs: 'Sega - Mega Drive - Genesis' },
  { dirs: ['mastersystem'], name: 'Master System', ext: ['sms', 'zip', '7z'], cores: ['genesis_plus_gx', 'picodrive'], thumbs: 'Sega - Master System - Mark III' },
  { dirs: ['gamegear'], name: 'Game Gear', ext: ['gg', 'zip', '7z'], cores: ['genesis_plus_gx'], thumbs: 'Sega - Game Gear' },
  { dirs: ['segacd', 'megacd'], name: 'Sega CD', ext: ['chd', 'cue', 'm3u'], cores: ['genesis_plus_gx', 'picodrive'], thumbs: 'Sega - Mega-CD - Sega CD' },
  { dirs: ['saturn'], name: 'Saturn', ext: ['chd', 'cue', 'm3u'], cores: ['mednafen_saturn', 'yabasanshiro'], thumbs: 'Sega - Saturn' },
  { dirs: ['dreamcast'], name: 'Dreamcast', ext: ['chd', 'gdi', 'cdi', 'm3u'], cores: ['flycast'], thumbs: 'Sega - Dreamcast' },
  { dirs: ['psx', 'ps1'], name: 'PlayStation', ext: ['chd', 'cue', 'pbp', 'm3u'], cores: ['swanstation', 'mednafen_psx_hw', 'mednafen_psx', 'pcsx_rearmed'], thumbs: 'Sony - PlayStation' },
  { dirs: ['psp'], name: 'PSP', ext: ['iso', 'cso', 'chd'], cores: ['ppsspp'], thumbs: 'Sony - PlayStation Portable' },
  { dirs: ['pcengine', 'tg16'], name: 'TurboGrafx-16', ext: ['pce', 'zip', '7z'], cores: ['mednafen_pce_fast', 'mednafen_pce'], thumbs: 'NEC - PC Engine - TurboGrafx 16' },
  { dirs: ['atari2600'], name: 'Atari 2600', ext: ['a26', 'bin', 'zip'], cores: ['stella', 'stella2014'], thumbs: 'Atari - 2600' },
  { dirs: ['ngp', 'ngpc'], name: 'Neo Geo Pocket', ext: ['ngp', 'ngc', 'zip'], cores: ['mednafen_ngp'], thumbs: 'SNK - Neo Geo Pocket Color' },
];

function romRoots(home = os.homedir(), user = os.userInfo().username) {
  const roots = [path.join(home, 'ROMs'), path.join(home, 'roms'), path.join(home, 'Emulation', 'roms')];
  // EmuDeck on an SD card or USB drive.
  for (const m of [`/run/media/${user}`, '/run/media']) {
    let names = []; try { names = fs.readdirSync(m); } catch {}
    for (const n of names) roots.push(path.join(m, n, 'Emulation', 'roms'));
  }
  return [...new Set(roots)].filter(r => { try { return fs.statSync(r).isDirectory(); } catch { return false; } });
}

/** "Legend of Zelda, The - A Link to the Past (USA) [!]" → "The Legend of Zelda - A Link to the Past". */
function cleanTitle(file) {
  let t = path.basename(file).replace(/\.[^.]+$/, '');
  t = t.replace(/\s*[([][^)\]]*[)\]]/g, '').replace(/_/g, ' ').replace(/\s+/g, ' ').trim();
  const m = t.match(/^(.*?), (The|A|An)(\b.*)$/);
  if (m) t = `${m[2]} ${m[1]}${m[3]}`;
  return t || path.basename(file);
}
/** The libretro thumbnail file name for a ROM (their rule: &*\/:`<>?\| become _). */
function thumbName(file) { return path.basename(file).replace(/\.[^.]+$/, '').replace(/[&*/:`<>?\\|]/g, '_'); }
function thumbUrl(sys, file, kind) { return THUMBS + encodeURIComponent(sys.thumbs) + '/' + kind + '/' + encodeURIComponent(thumbName(file)) + '.png'; }

function scanDir(dir, sys, depth = 2) {
  let entries = [];
  try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { return []; }
  const files = entries.filter(e => e.isFile()).map(e => e.name);
  // Discs listed in a playlist (.m3u), and .bin tracks of a .cue, aren't games on their own.
  const inPlaylist = new Set();
  for (const f of files) if (/\.m3u$/i.test(f)) { try { for (const l of fs.readFileSync(path.join(dir, f), 'utf8').split(/\r?\n/)) if (l.trim()) inPlaylist.add(path.basename(l.trim()).toLowerCase()); } catch {} }
  const hasCue = files.some(f => /\.cue$/i.test(f));
  const out = [];
  for (const f of files) {
    const ext = (f.split('.').pop() || '').toLowerCase();
    if (!sys.ext.includes(ext) || inPlaylist.has(f.toLowerCase())) continue;
    if (ext === 'bin' && hasCue && sys.ext.includes('cue')) continue;
    out.push(path.join(dir, f));
  }
  if (depth > 0) for (const e of entries) if (e.isDirectory() && !e.name.startsWith('.') && !/^(media|images|videos|manuals|saves|states)$/i.test(e.name)) out.push(...scanDir(path.join(dir, e.name), sys, depth - 1));
  return out;
}

/** Every ROM in the ROMs folders, as library games. */
function scan({ roots = romRoots() } = {}) {
  const games = [];
  const seen = new Set();
  for (const root of roots) {
    let dirs = [];
    try { dirs = fs.readdirSync(root, { withFileTypes: true }).filter(d => d.isDirectory()).map(d => d.name); } catch { continue; }
    for (const d of dirs) {
      const sys = SYSTEMS.find(s => s.dirs.includes(d.toLowerCase()));
      if (!sys) continue;
      for (const file of scanDir(path.join(root, d), sys)) {
        const id = 'emu:' + sys.dirs[0] + ':' + path.relative(path.join(root, d), file);
        if (seen.has(id)) continue;
        seen.add(id);
        let added = 0; try { added = fs.statSync(file).mtimeMs; } catch {}
        games.push({
          id, title: cleanTitle(file), source: 'emu', system: sys.name, systemKey: sys.dirs[0],
          rom: file, installPath: file, installed: true, state: 'installed', added,
          art: { cover: thumbUrl(sys, file, 'Named_Boxarts'), wide: thumbUrl(sys, file, 'Named_Snaps'), hero: thumbUrl(sys, file, 'Named_Snaps') },
        });
      }
    }
  }
  return games;
}

// ---------------------------------------------------------------- RetroArch

function retroarch(home = os.homedir()) {
  if (has('retroarch')) return { cmd: 'retroarch', pre: [], cores: [path.join(home, '.config/retroarch/cores'), '/usr/lib64/libretro', '/usr/lib/libretro', '/usr/lib/x86_64-linux-gnu/libretro'] };
  const fp = [path.join(home, '.local/share/flatpak/app', FLATPAK), path.join('/var/lib/flatpak/app', FLATPAK)].some(p => fs.existsSync(p));
  if (fp && has('flatpak')) return { cmd: 'flatpak', pre: ['run', FLATPAK], cores: [path.join(home, '.var/app', FLATPAK, 'config/retroarch/cores')] };
  return null;
}
function findCore(sys, dirs) {
  for (const c of sys.cores) for (const d of dirs) { const f = path.join(d, `${c}_libretro.so`); if (fs.existsSync(f)) return f; }
  return null;
}
function systemOf(g) { return SYSTEMS.find(s => s.dirs[0] === g.systemKey); }

function launch(g) {
  const ra = retroarch();
  if (!ra) throw new Error('Emulated games play with RetroArch. Install it in Settings › Library sources › Emulators.');
  const sys = systemOf(g);
  const core = sys && findCore(sys, ra.cores);
  if (!core) throw new Error(`RetroArch needs a ${sys ? sys.name : ''} core: open RetroArch › Online Updater › Core Downloader and get ${sys ? sys.cores[0].replace(/_/g, ' ') : 'one'}.`);
  if (!fs.existsSync(g.rom)) throw new Error(`The game file is gone: ${g.rom}`);
  return launchDetached(ra.cmd, [...ra.pre, '-f', '-L', core, g.rom], { cwd: path.dirname(g.rom) });
}

function status() {
  const ra = retroarch();
  return { retroarch: !!ra, via: ra ? (ra.cmd === 'flatpak' ? 'flatpak' : 'system') : null, roots: romRoots(), folder: path.join(os.homedir(), 'ROMs'),
    systems: SYSTEMS.map(s => ({ dir: s.dirs[0], name: s.name })) };
}
/** Makes ~/ROMs with a folder for each system, so you know where to put games. */
function makeFolders() {
  const root = path.join(os.homedir(), 'ROMs');
  for (const s of SYSTEMS) fs.mkdirSync(path.join(root, s.dirs[0]), { recursive: true });
  return root;
}
async function installRetroArch() {
  const { run } = require('../util');
  if (!has('flatpak')) throw new Error('Flatpak is needed to install RetroArch');
  await run('flatpak', ['install', '--user', '-y', '--noninteractive', 'flathub', FLATPAK], { timeout: 20 * 60 * 1000 });
  return true;
}

module.exports = { SYSTEMS, scan, cleanTitle, thumbName, thumbUrl, romRoots, launch, status, makeFolders, installRetroArch, findCore };
