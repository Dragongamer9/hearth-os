"""Hearth Files' This PC (docs/HEARTH-FILES.md "This PC" and "Drive letters"): drive letters that stay the same
(hearth_drive_letters.py), paths in letters and back (drive-paths.js, run with node), This PC's drives in hearth-files,
and Windows apps seeing the same letters (map_prefix). Each test has its own throwaway home folder."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
import hearth_drive_letters as hdl  # noqa: E402


def load_files(home):
    os.environ['HOME'] = home
    for k in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_DIRS', 'XDG_CONFIG_DIRS'):
        os.environ.pop(k, None)
    loader = importlib.machinery.SourceFileLoader('hearth_files_pc', str(SHELL / 'hearth-files'))
    spec = importlib.util.spec_from_loader('hearth_files_pc', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.home = tempfile.mkdtemp(prefix='hearth-thispc-')
        os.environ['HOME'] = self.home
        os.environ['XDG_STATE_HOME'] = os.path.join(self.home, '.local', 'state')
        os.environ.pop('XDG_CONFIG_HOME', None)
        self.file = os.path.join(self.home, '.config', 'hearth', 'drive-letters.json')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.home, ignore_errors=True)

    def files_in(self, d):
        try:
            return sorted(os.listdir(d))
        except OSError:
            return []


class Letters(Base):
    def test_c_is_hearth_and_nothing_written_for_it(self):
        self.assertEqual(hdl.assign([{'key': 'system', 'name': 'Hearth', 'path': '/'}]), {})
        self.assertFalse(os.path.exists(self.file), 'opening This PC with only C: must not write anything')
        with self.assertRaises(hdl.Fail):
            hdl.set_letter('uuid:A', 'C')
        with self.assertRaises(hdl.Fail):
            hdl.set_letter('system', 'E')
        self.assertFalse(os.path.exists(self.file))

    def test_first_seen_gets_next_free_and_is_kept(self):
        got = hdl.assign([{'key': 'uuid:A', 'name': 'Games', 'path': '/run/media/me/Games', 'kind': 'drive'},
                          {'key': 'uuid:B', 'name': 'USB', 'path': '/run/media/me/USB', 'kind': 'usb'}])
        self.assertEqual(got, {'uuid:A': 'D', 'uuid:B': 'E'})
        self.assertTrue(os.path.exists(self.file))
        with open(self.file) as f:
            data = json.load(f)
        self.assertEqual(data['drives']['uuid:A']['letter'], 'D')
        self.assertNotIn('system', data['drives'])

    def test_stable_across_order_and_unplugging(self):
        hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}, {'key': 'uuid:B', 'path': '/m/b'}])
        # A unplugged, a new drive comes: it gets F:, never A's D:.
        got = hdl.assign([{'key': 'uuid:C', 'path': '/m/c'}, {'key': 'uuid:B', 'path': '/m/b'}])
        self.assertEqual(got, {'uuid:C': 'F', 'uuid:B': 'E'})
        # A back, in another order and at another mount place: still D:.
        got = hdl.assign([{'key': 'uuid:B', 'path': '/m/b'}, {'key': 'uuid:A', 'path': '/m/a2'}])
        self.assertEqual(got['uuid:A'], 'D')
        self.assertEqual(hdl.load()['uuid:A']['path'], '/m/a2')

    def test_seeing_known_drives_writes_nothing(self):
        hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}])
        before = os.stat(self.file).st_mtime_ns
        listing = self.files_in(os.path.dirname(self.file))
        for _ in range(3):
            self.assertEqual(hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}]), {'uuid:A': 'D'})
        self.assertEqual(os.stat(self.file).st_mtime_ns, before)
        self.assertEqual(self.files_in(os.path.dirname(self.file)), listing)

    def test_user_change_saved_and_never_moved(self):
        hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}, {'key': 'uuid:B', 'path': '/m/b'}])
        self.assertEqual(hdl.set_letter('uuid:A', 'G:')['letter'], 'G')
        with self.assertRaises(hdl.Fail):
            hdl.set_letter('uuid:B', 'G')           # taken
        with self.assertRaises(hdl.Fail):
            hdl.set_letter('uuid:B', 'Z')           # Windows apps' whole system
        got = hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}, {'key': 'uuid:B', 'path': '/m/b'}, {'key': 'uuid:N', 'path': '/m/n'}])
        self.assertEqual(got, {'uuid:A': 'G', 'uuid:B': 'E', 'uuid:N': 'D'})   # D: was freed by your change
        st = hdl.load()
        self.assertTrue(st['uuid:A'].get('user'))
        self.assertEqual(st['uuid:A']['letter'], 'G')

    def test_bad_file_kept_aside_once(self):
        os.makedirs(os.path.dirname(self.file))
        with open(self.file, 'w') as f:
            f.write('{"drives": {"uuid:A": {"letter": "D"')     # cut short (a power cut)
        self.assertEqual(hdl.load(), {})
        kept = [n for n in self.files_in(os.path.dirname(self.file)) if '.broken-' in n]
        self.assertEqual(len(kept), 1, kept)
        with open(os.path.join(os.path.dirname(self.file), kept[0])) as f:
            self.assertIn('"letter": "D"', f.read())
        # Not kept again on every look, and new letters can be saved.
        hdl.load()
        hdl.assign([{'key': 'uuid:B', 'path': '/m/b'}])
        self.assertEqual(len([n for n in self.files_in(os.path.dirname(self.file)) if '.broken-' in n]), 1)
        self.assertEqual(hdl.load()['uuid:B']['letter'], 'D')

    def test_bad_file_put_back_from_bak(self):
        hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}])
        hdl.assign([{'key': 'uuid:B', 'path': '/m/b'}])      # the first version is the .bak now
        with open(self.file, 'w') as f:
            f.write('')
        st = hdl.load()
        self.assertEqual(st['uuid:A']['letter'], 'D')
        self.assertNotIn('uuid:B', st)                        # what .bak had: never defaults over it


NODE = shutil.which('node')
JS = r'''
const fs = require('fs');
const src = fs.readFileSync(process.argv[2], 'utf8').replace(/^\.pragma library\s*$/m, '');
const m = {}; new Function('exports', src + '\nexports.crumbsFor = crumbsFor; exports.toWindows = toWindows; exports.fromWindows = fromWindows; exports.driveFor = driveFor;')(m);
const ctx = { home: '/home/me', user: 'me',
  drives: [{ letter: 'C', path: '/', label: 'Hearth (C:)' }, { letter: 'D', path: '/run/media/me/Games', label: 'Games (D:)' },
           { letter: 'G', path: '/run/user/1000/gvfs/smb-share:server=pc,share=games', label: 'Games (\\\\Gaming PC) (G:)' },
           { letter: 'E', path: '/run/media/me/GamesBackup', label: 'Backup (E:)' }],
  folders: [{ label: 'Documents', path: '/home/me/Documents' }, { label: 'Downloads', path: '/home/me/Downloads' }] };
const out = {};
for (const p of JSON.parse(process.argv[3])) out[p] = { crumbs: m.crumbsFor(p, ctx, 'hearth:thispc'), win: m.toWindows(p, ctx) };
const back = {};
for (const t of JSON.parse(process.argv[4])) back[t] = m.fromWindows(t, ctx);
console.log(JSON.stringify({ out, back }));
'''


@unittest.skipUnless(NODE, 'node is needed to run drive-paths.js')
class Paths(unittest.TestCase):
    def run_js(self, paths, typed):
        with tempfile.NamedTemporaryFile('w', suffix='.js', delete=False) as f:
            f.write(JS)
        try:
            r = subprocess.run([NODE, f.name, str(SHELL / 'drive-paths.js'), json.dumps(paths), json.dumps(typed)],
                               capture_output=True, text=True, timeout=30)
        finally:
            os.unlink(f.name)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout)

    def test_paths_to_letters(self):
        paths = ['/run/media/me/Games/SteamLibrary/steamapps', '/run/media/me/Games', '/run/media/me/GamesBackup/x',
                 '/home/me/Documents/notes.txt', '/home/me/Games', '/usr/share', '/', '/run/user/1000/gvfs/smb-share:server=pc,share=games/Saves']
        r = self.run_js(paths, [])['out']
        labels = lambda p: [c['label'] for c in r[p]['crumbs']] if r[p]['crumbs'] else None
        self.assertEqual(labels(paths[0]), ['This PC', 'Games (D:)', 'SteamLibrary', 'steamapps'])
        self.assertEqual(r[paths[0]]['crumbs'][1]['path'], '/run/media/me/Games')
        self.assertEqual(r[paths[0]]['crumbs'][0]['path'], 'hearth:thispc')
        self.assertEqual(r[paths[0]]['win'], 'D:\\SteamLibrary\\steamapps')
        self.assertEqual(r[paths[1]]['win'], 'D:\\')
        self.assertEqual(labels(paths[2]), ['This PC', 'Backup (E:)', 'x'])          # not D: (a longer name)
        self.assertEqual(labels(paths[3]), ['This PC', 'Documents', 'notes.txt'])
        self.assertEqual(r[paths[3]]['win'], 'C:\\Users\\me\\Documents\\notes.txt')
        self.assertIsNone(r[paths[4]]['crumbs'])                                    # the rest of home: Files' own steps
        self.assertEqual(labels(paths[5]), ['This PC', 'Hearth (C:)', 'usr', 'share'])
        self.assertEqual(r[paths[5]]['win'], 'C:\\usr\\share')
        self.assertEqual(r[paths[6]]['win'], 'C:\\')
        self.assertEqual(r[paths[7]]['win'], 'G:\\Saves')

    def test_letters_to_paths(self):
        typed = ['D:\\Games', 'd:/Games/', 'D:', 'D:\\a\\..\\b', 'C:\\Users\\me\\Documents', 'C:\\usr\\share', 'X:\\nope',
                 '/home/me', 'Documents', 'C:\\Users\\Public']
        b = self.run_js([], typed)['back']
        self.assertEqual(b['D:\\Games']['path'], '/run/media/me/Games/Games')
        self.assertEqual(b['d:/Games/']['path'], '/run/media/me/Games/Games')
        self.assertEqual(b['D:']['path'], '/run/media/me/Games')
        self.assertEqual(b['D:\\a\\..\\b']['path'], '/run/media/me/Games/b')
        self.assertEqual(b['C:\\Users\\me\\Documents']['path'], '/home/me/Documents')
        self.assertEqual(b['C:\\usr\\share']['path'], '/usr/share')
        self.assertIn('error', b['X:\\nope'])
        self.assertIsNone(b['/home/me'])
        self.assertIsNone(b['Documents'])
        self.assertEqual(b['C:\\Users\\Public']['path'], '/Users/Public')

    def test_round_trip(self):
        paths = ['/run/media/me/Games/SteamLibrary', '/home/me/Documents/a b', '/usr/share/fonts']
        r = self.run_js(paths, [])['out']
        b = self.run_js([], [r[p]['win'] for p in paths])['back']
        for p in paths:
            self.assertEqual(b[r[p]['win']]['path'], p)


class ThisPcDrives(Base):
    """hearth-files' This PC drives with a pretend system: what /proc/mounts and lsblk would say."""
    def fake(self, m, mounts, flat, media=()):
        m.mounts = lambda: dict(mounts)
        m.fs_types = lambda: {k: 'ext4' for k in mounts}
        m.lsblk_flat = lambda: list(flat)
        m.media_bases = lambda: list(media)
        m.steam_libraries = lambda: []
        m.free_space = lambda p: (40 * 10**9, 100 * 10**9)

    def test_order_letters_and_labels(self):
        m = load_files(self.home)
        media = os.path.join(self.home, 'media')
        os.makedirs(os.path.join(media, 'STICK'))
        mounts = {'/': 'composefs', '/sysroot': '/dev/nvme0n1p3', '/var': '/dev/nvme0n1p3', '/boot/efi': '/dev/nvme0n1p1',
                  '/var/mnt/games-1': '/dev/sda1', '/data': '/dev/nvme1n1p1', os.path.join(media, 'STICK'): '/dev/sdb1',
                  '/proc': 'proc', '/run/user/1000': 'tmpfs'}
        flat = [{'path': '/dev/sda1', 'uuid': 'GAMES-UUID', 'label': '', 'rm': False, 'tran': 'sata'},
                {'path': '/dev/nvme1n1p1', 'uuid': 'DATA-UUID', 'label': 'Data', 'rm': False, 'tran': 'nvme'},
                {'path': '/dev/sdb1', 'uuid': 'STICK-UUID', 'label': 'STICK', 'rm': True, 'tran': 'usb'}]
        self.fake(m, mounts, flat, [media])
        drives = m.pc_drives()
        self.assertEqual([d['label'] for d in drives], ['Data (D:)', 'Games (E:)', 'STICK (F:)'])
        self.assertEqual([d['kind'] for d in drives], ['drive', 'games', 'usb'])
        self.assertTrue(drives[2]['removable'])
        self.assertFalse(drives[0]['removable'])
        self.assertEqual(drives[1]['key'], 'uuid:GAMES-UUID')
        # Seen again, with the stick at another place: the same letters, and C: is Hearth.
        mounts[os.path.join(media, 'STICK2')] = mounts.pop(os.path.join(media, 'STICK'))
        os.rename(os.path.join(media, 'STICK'), os.path.join(media, 'STICK2'))
        self.fake(m, mounts, flat, [media])
        self.assertEqual([d['letter'] for d in m.pc_drives()], ['D', 'E', 'F'])
        self.assertEqual(m.system_drive()['label'], 'Hearth (C:)')

    def test_no_drives_no_file(self):
        m = load_files(self.home)
        self.fake(m, {'/': 'composefs', '/sysroot': '/dev/nvme0n1p3'}, [])
        os.makedirs(os.path.join(self.home, 'Documents'))
        m.device_places = lambda: []
        out = m.this_pc()
        self.assertEqual([d['label'] for d in out['drives']], ['Hearth (C:)'])
        self.assertEqual([f['label'] for f in out['folders']], ['Documents'])
        self.assertFalse(os.path.exists(self.file))
        self.assertFalse(os.path.exists(os.path.join(self.home, '.config')), 'This PC wrote something: %s'
                         % list(pathlib.Path(self.home, '.config').rglob('*')))

    def test_letter_set_command(self):
        m = load_files(self.home)
        hdl.assign([{'key': 'uuid:A', 'path': '/m/a'}])
        self.assertEqual(m.letter_set('uuid:A', 'K')['letter'], 'K')
        with self.assertRaises(m.Fail):
            m.letter_set('uuid:A', 'C')


class WindowsApps(Base):
    def prefix(self, files=False):
        p = os.path.join(self.home, 'Games', 'Windows apps', 'Notepad', 'prefix')
        os.makedirs(os.path.join(p, 'dosdevices'))
        os.symlink('../drive_c', os.path.join(p, 'dosdevices', 'c:'))
        os.symlink('/', os.path.join(p, 'dosdevices', 'z:'))
        docs = os.path.join(p, 'drive_c', 'users', 'steamuser', 'Documents')
        os.makedirs(docs)
        os.makedirs(os.path.join(p, 'drive_c', 'users', 'steamuser', 'Music'))
        if files:
            open(os.path.join(p, 'drive_c', 'users', 'steamuser', 'Music', 'saved.txt'), 'w').close()
        for d in ('Documents', 'Music', 'Downloads'):
            os.makedirs(os.path.join(self.home, d), exist_ok=True)
        return p

    def test_letters_and_folders(self):
        hdl.assign([{'key': 'uuid:A', 'path': '/run/media/me/Games'}])
        p = self.prefix(files=True)
        os.symlink('/dev/sr0', os.path.join(p, 'dosdevices', 'e:'))   # Wine's own (say, a disc drive): kept
        hdl.assign([{'key': 'uuid:E', 'path': '/run/media/me/E'}, {'key': 'net:x', 'path': ''}])   # E:, and F: with no place yet
        r = hdl.map_prefix(p)
        dos = os.path.join(p, 'dosdevices')
        self.assertEqual(os.readlink(os.path.join(dos, 'd:')), '/run/media/me/Games')
        self.assertEqual(os.readlink(os.path.join(dos, 'e:')), '/dev/sr0')
        self.assertIn('e:', r['kept'])
        self.assertEqual(os.readlink(os.path.join(dos, 'c:')), '../drive_c')
        self.assertEqual(os.readlink(os.path.join(dos, 'z:')), '/')
        users = os.path.join(p, 'drive_c', 'users', 'steamuser')
        self.assertEqual(os.readlink(os.path.join(users, 'Documents')), os.path.join(self.home, 'Documents'))
        self.assertTrue(os.path.isfile(os.path.join(users, 'Music', 'saved.txt')), 'a folder with files is never touched')
        self.assertEqual(os.readlink(os.path.join(users, 'Downloads')), os.path.join(self.home, 'Downloads'))
        # A changed letter: the old link Hearth made goes, the new one comes; again changes nothing.
        hdl.set_letter('uuid:A', 'K')
        hdl.map_prefix(p)
        self.assertFalse(os.path.lexists(os.path.join(dos, 'd:')))
        self.assertEqual(os.readlink(os.path.join(dos, 'k:')), '/run/media/me/Games')
        marker = os.path.join(p, '.hearth-drives.json')
        before = os.stat(marker).st_mtime_ns
        hdl.map_prefix(p)
        self.assertEqual(os.stat(marker).st_mtime_ns, before)

    def test_new_prefix_untouched(self):
        hdl.assign([{'key': 'uuid:A', 'path': '/run/media/me/Games'}])
        p = os.path.join(self.home, 'fresh')
        os.makedirs(p)
        hdl.map_prefix(p)
        self.assertEqual(os.listdir(p), [], 'a prefix Proton hasn\'t made yet is left for Proton')

    def test_winapps_calls_it(self):
        src = (SHELL / 'hearth-winapps').read_text()
        self.assertIn('hdl.map_prefix(prefix)', src)
        self.assertIn('except Exception', src[src.index('hdl.map_prefix(prefix)'):][:200])


class FilesWindow(unittest.TestCase):
    """FileManager.qml keeps the pages behind Loaders and offers what the owner asked for."""
    def test_pages_and_menus(self):
        q = (SHELL / 'FileManager.qml').read_text()
        for needle in ('setSource("FilesThisPc.qml"', 'setSource("FilesApps.qml"', '"Open Files to: This PC"', '"Change drive letter…"',
                       '"Copy Linux path"', 'import "drive-paths.js" as DrivePaths', 'label: "This PC", path: thisPcPage',
                       'label: "Installed apps", path: appsPage', 'function thispc(): void', 'function installed(): void'):
            self.assertIn(needle, q)
        self.assertNotIn('FilesThisPc {', q, 'the page must load through a Loader, never directly')
        self.assertNotIn('FilesApps {', q)
        self.assertNotIn('WinGlyph {', q)
        apps = (SHELL / 'FilesApps.qml').read_text()
        self.assertIn('"···"', apps)
        self.assertIn('Part of Hearth', apps)


if __name__ == '__main__':
    unittest.main()
