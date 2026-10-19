"""Tests for os/.../hearth-import: finding games and files on drive layouts like Windows, SteamOS, Bazzite and
an SD card (fake folders, no real drives or mounting), the manifest fix-up and copying."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-import'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_import', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_import', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    m.du = lambda p, timeout=20: 1234   # no real sizes needed
    return m


def write(p, text=''):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write(text)


def manifest(steamapps, appid, name, installdir, flags=4, size=5000):
    write(os.path.join(steamapps, f'appmanifest_{appid}.acf'),
          f'"AppState"\n{{\n\t"appid"\t\t"{appid}"\n\t"name"\t\t"{name}"\n\t"StateFlags"\t\t"{flags}"\n'
          f'\t"installdir"\t\t"{installdir}"\n\t"SizeOnDisk"\t\t"{size}"\n\t"LauncherPath"\t\t"C:\\\\Program Files (x86)\\\\Steam\\\\steam.exe"\n}}\n')
    os.makedirs(os.path.join(steamapps, 'common', installdir), exist_ok=True)
    write(os.path.join(steamapps, 'common', installdir, 'game.exe'), 'x' * 100)


class Layouts(unittest.TestCase):
    def setUp(self):
        self.hi = load()
        self.root = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_windows_drive(self):
        mnt = os.path.join(self.root, 'c')
        os.makedirs(os.path.join(mnt, 'Windows', 'System32'))
        sa = os.path.join(mnt, 'Program Files (x86)', 'Steam', 'steamapps')
        manifest(sa, '1145360', 'Hades', 'Hades')
        manifest(sa, '228980', 'Steamworks Common Redistributables', 'Steamworks Shared')   # skipped
        manifest(sa, '620', 'Portal 2', 'Portal 2', flags=1026)                            # mid-update: skipped
        # Epic: a game on this drive and one on D:
        d = os.path.join(self.root, 'd')
        os.makedirs(os.path.join(mnt, 'Program Files', 'Epic Games', 'Control'))
        os.makedirs(os.path.join(d, 'Epic', 'Rocket League'))
        mdir = os.path.join(mnt, 'ProgramData', 'Epic', 'EpicGamesLauncher', 'Data', 'Manifests')
        write(os.path.join(mdir, 'a.item'), json.dumps({'AppName': 'Calluna', 'DisplayName': 'Control', 'InstallLocation': 'C:\\Program Files\\Epic Games\\Control', 'InstallSize': 42}))
        write(os.path.join(mdir, 'b.item'), json.dumps({'AppName': 'Sugar', 'DisplayName': 'Rocket League', 'InstallLocation': 'D:\\Epic\\Rocket League', 'InstallSize': 7}))
        write(os.path.join(mdir, 'c.item'), json.dumps({'AppName': 'Half', 'DisplayName': 'Half done', 'InstallLocation': 'C:\\X', 'bIsIncompleteInstall': True}))
        # GOG
        write(os.path.join(mnt, 'GOG Games', 'Witcher 3', 'goggame-1207664663.info'), json.dumps({'gameId': '1207664663', 'name': 'The Witcher 3'}))
        write(os.path.join(mnt, 'GOG Games', 'Witcher 3 DLC', 'goggame-1.info'), json.dumps({'gameId': '1', 'rootGameId': '1207664663', 'name': 'DLC'}))
        # Personal files
        write(os.path.join(mnt, 'Users', 'Sam', 'Documents', 'notes.txt'), 'hi')
        write(os.path.join(mnt, 'Users', 'Public', 'Documents', 'x.txt'), 'hi')

        homes = self.hi.homes(mnt)
        kind, name = self.hi.describe(mnt, {'removable': False, 'fs': 'ntfs'}, homes)
        self.assertEqual((kind, name), ('windows', 'Windows'))
        libs = self.hi.steam_libraries(mnt, kind, homes)
        games = [g for sa in libs for g in self.hi.steam_games(sa, 'nvme0n1p3')]
        self.assertEqual([g['name'] for g in games], ['Hades'])
        self.assertEqual(games[0]['size'], 5000)
        self.assertEqual(games[0]['key'], 'steam:nvme0n1p3:1145360')
        epic = self.hi.epic_games(mnt, kind, homes, [mnt, d], 'x')
        self.assertEqual(sorted(e['name'] for e in epic), ['Control', 'Rocket League'])
        self.assertTrue(any(e['path'] == os.path.join(d, 'Epic', 'Rocket League') for e in epic))
        gog = self.hi.gog_games(mnt, kind, homes, 'x')
        self.assertEqual([(g['id'], g['name']) for g in gog], [('1207664663', 'The Witcher 3')])
        files = self.hi.personal_files(kind, mnt, homes, 'x')
        self.assertEqual([(f['user'], f['name']) for f in files], [('Sam', 'Documents')])

    def test_steamos_home_partition(self):
        # SteamOS keeps /home on its own partition: deck/.local/share/Steam, plus Proton data.
        mnt = os.path.join(self.root, 'home')
        sa = os.path.join(mnt, 'deck', '.local', 'share', 'Steam', 'steamapps')
        manifest(sa, '1091500', 'Cyberpunk 2077', 'Cyberpunk 2077')
        os.makedirs(os.path.join(sa, 'compatdata', '1091500', 'pfx'))
        os.symlink('/home/deck/.local/share/Steam', os.path.join(mnt, 'deck', '.steam-link'))
        write(os.path.join(mnt, 'deck', 'Pictures', 'shot.png'), 'png')
        homes = self.hi.homes(mnt)
        self.assertEqual([os.path.basename(h) for h in homes], ['deck'])
        kind, name = self.hi.describe(mnt, {'removable': False, 'fs': 'ext4'}, homes)
        self.assertEqual((kind, name), ('linux', 'SteamOS'))
        games = [g for s in self.hi.steam_libraries(mnt, kind, homes) for g in self.hi.steam_games(s, 'p8')]
        self.assertEqual(len(games), 1)
        self.assertTrue(games[0]['compat'].endswith(os.path.join('compatdata', '1091500')))

    def test_bazzite_btrfs_and_heroic(self):
        # Fedora Atomic / Bazzite on btrfs, mounted at the top level: <subvolume>/... and /var/home.
        mnt = os.path.join(self.root, 'btrfs')
        home = os.path.join(mnt, 'var', 'home', 'alex')
        write(os.path.join(mnt, 'root', 'etc', 'os-release'), 'NAME=Bazzite\n')
        manifest(os.path.join(home, '.var', 'app', 'com.valvesoftware.Steam', '.local', 'share', 'Steam', 'steamapps'), '570', 'Dota 2', 'dota 2 beta')
        game = os.path.join(home, 'Games', 'Heroic', 'Hollow')
        os.makedirs(game)
        write(os.path.join(home, '.config', 'heroic', 'legendaryConfig', 'legendary', 'installed.json'),
              json.dumps({'Hollow': {'title': 'Hollow Knight', 'install_path': '/var/home/alex/Games/Heroic/Hollow', 'install_size': 9}}))
        homes = self.hi.homes(mnt)
        self.assertIn(os.path.realpath(home), homes)
        kind, _ = self.hi.describe(mnt, {'removable': False, 'fs': 'btrfs'}, homes)
        self.assertEqual(kind, 'linux')
        games = [g for s in self.hi.steam_libraries(mnt, kind, homes) for g in self.hi.steam_games(s, 'x')]
        self.assertEqual([g['name'] for g in games], ['Dota 2'])
        epic = self.hi.epic_games(mnt, kind, homes, [], 'x')
        self.assertEqual([(e['app'], e['name']) for e in epic], [('Hollow', 'Hollow Knight')])

    def test_lutris_games(self):
        import sqlite3
        mnt = os.path.join(self.root, 'ext')
        home = os.path.join(mnt, 'home', 'sam')
        game = os.path.join(home, 'Games', 'overwatch')
        os.makedirs(os.path.join(game, 'drive_c'))
        os.makedirs(os.path.join(home, '.local', 'share', 'lutris'))
        con = sqlite3.connect(os.path.join(home, '.local', 'share', 'lutris', 'pga.db'))
        con.execute('CREATE TABLE games (name TEXT, slug TEXT, runner TEXT, directory TEXT, configpath TEXT, installed INTEGER)')
        con.execute("INSERT INTO games VALUES ('Overwatch', 'overwatch', 'wine', '/home/sam/Games/overwatch', 'overwatch-123', 1)")
        con.execute("INSERT INTO games VALUES ('Gone', 'gone', 'wine', '/home/sam/Games/gone', 'gone-1', 0)")
        con.commit(); con.close()
        write(os.path.join(home, '.config', 'lutris', 'games', 'overwatch-123.yml'), 'game:\n  exe: /home/sam/Games/overwatch/drive_c/ow.exe\n')
        homes = self.hi.homes(mnt)
        got = self.hi.lutris_games(mnt, homes, 'x')
        self.assertEqual([(g['name'], g['exe']) for g in got], [('Overwatch', '/home/sam/Games/overwatch/drive_c/ow.exe')])
        self.assertEqual(got[0]['path'], os.path.realpath(game) if os.path.realpath(game) == got[0]['path'] else game)

    def test_sd_card_library(self):
        mnt = os.path.join(self.root, 'sd')
        manifest(os.path.join(mnt, 'steamapps'), '413150', 'Stardew Valley', 'Stardew Valley')
        homes = self.hi.homes(mnt)
        kind, name = self.hi.describe(mnt, {'removable': True, 'fs': 'ext4'}, homes)
        self.assertEqual((kind, name), ('data', 'SD card'))
        games = [g for s in self.hi.steam_libraries(mnt, kind, homes) for g in self.hi.steam_games(s, 'mmc')]
        self.assertEqual([g['name'] for g in games], ['Stardew Valley'])


class Copying(unittest.TestCase):
    def setUp(self):
        self.hi = load()
        self.root = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_manifest_marked_installed_without_windows_path(self):
        src, dst = os.path.join(self.root, 'a.acf'), os.path.join(self.root, 'b.acf')
        manifest(self.root, '1', 'X', 'X', flags=6)
        os.rename(os.path.join(self.root, 'appmanifest_1.acf'), src)
        self.hi.fix_manifest(src, dst, 'X')
        st = self.hi.ci(self.hi.vdf_load(dst), 'AppState')
        self.assertEqual(st['StateFlags'], '4')
        self.assertNotIn('LauncherPath', st)

    @unittest.skipUnless(shutil.which('rsync'), 'rsync not installed')
    def test_rsync_reports_progress(self):
        src = os.path.join(self.root, 'src')
        write(os.path.join(src, 'big.bin'), 'x' * 300000)
        write(os.path.join(src, 'sub', 'a.txt'), 'hello')
        seen = []
        self.hi.rsync(src, os.path.join(self.root, 'dst'), lambda d, b: seen.append(d))
        self.assertTrue(os.path.exists(os.path.join(self.root, 'dst', 'sub', 'a.txt')))
        self.assertTrue(seen and seen[-1] >= 300000)

    def test_under(self):
        self.assertTrue(self.hi.under('/home/a/Games', '/home/a'))
        self.assertFalse(self.hi.under('/home/ab', '/home/a'))


def fake_rsync(src, dst, on_bytes):
    """rsync without rsync (it may not be installed where the tests run): copies src/ into dst/."""
    shutil.copytree(src, dst, symlinks=True, dirs_exist_ok=True)
    on_bytes(1, 1.0)


class Moving(unittest.TestCase):
    """Move (Beta Experimental): copy, check every byte, register, and only then delete the original."""

    def setUp(self):
        self.hi = load()
        self.root = tempfile.mkdtemp()
        self.mnt = os.path.join(self.root, 'windows')
        self.sa = os.path.join(self.mnt, 'Program Files (x86)', 'Steam', 'steamapps')
        manifest(self.sa, '1145360', 'Hades', 'Hades')
        game = os.path.join(self.sa, 'common', 'Hades')
        write(os.path.join(game, 'Content', 'big.pak'), 'p' * 50000)
        write(os.path.join(game, 'Content', 'a.txt'), 'hello')
        os.symlink('Content/a.txt', os.path.join(game, 'link.txt'))
        manifest(self.sa, '620', 'Portal 2', 'Portal 2')
        self.home = os.path.join(self.root, 'home')
        self.steamapps = os.path.join(self.home, '.local', 'share', 'Steam', 'steamapps')
        os.makedirs(self.steamapps)
        self.user = type('U', (), {'pw_dir': self.home, 'pw_uid': os.getuid(), 'pw_gid': os.getgid(), 'pw_name': 'me'})()
        games = self.hi.steam_games(self.sa, 'sda1')
        self.drive = {'id': 'sda1', 'dev': '/dev/sda1', 'name': 'Windows', 'fs': 'ntfs', 'mount': self.mnt, 'ours': True,
                      'steam': games, 'epic': [], 'gog': [], 'lutris': [], 'files': []}
        self.hades = next(g for g in games if g['appid'] == '1145360')
        self.portal = next(g for g in games if g['appid'] == '620')
        self.hi.rsync = fake_rsync
        self.hi.chown_tree = lambda p, user: None
        self.writable = []
        self.hi.make_writable = lambda d, mnt, state: self.writable.append(mnt)
        self.hi.current.update(stop=False, partial=[], flag=None)
        self.hi.stage = lambda **kw: None

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def move(self, items, mode='move'):
        return self.hi.do_items(items, mode, self.user, os.path.join(self.home, 'Games'), self.steamapps, [self.drive])

    def test_move_copies_checks_registers_then_deletes(self):
        r = self.move([('steam', self.drive, self.hades)])
        dst = os.path.join(self.steamapps, 'common', 'Hades')
        with open(os.path.join(dst, 'Content', 'big.pak')) as f:
            self.assertEqual(f.read(), 'p' * 50000)
        self.assertTrue(os.path.islink(os.path.join(dst, 'link.txt')))
        self.assertTrue(os.path.exists(os.path.join(self.steamapps, 'appmanifest_1145360.acf')))
        self.assertFalse(os.path.exists(self.hades['path']))                                   # original gone
        self.assertFalse(os.path.exists(self.hades['manifest']))                               # and Steam's record of it
        self.assertFalse(any(n.startswith('.hearth-moved-') for n in os.listdir(os.path.join(self.sa, 'common'))))
        self.assertTrue(os.path.exists(self.portal['path']))                                   # other games untouched
        self.assertEqual([x['name'] for x in r['moved']], ['Hades'])
        self.assertEqual(r['steam'], 1)
        self.assertEqual(r['freed'][0]['drive'], 'sda1')
        self.assertGreater(r['freed'][0]['bytes'], 0)
        self.assertEqual(self.writable, [self.mnt])

    def test_bad_copy_keeps_the_original_and_removes_the_copy(self):
        def corrupt(src, dst, on_bytes):
            fake_rsync(src, dst, on_bytes)
            p = os.path.join(dst, 'Content', 'big.pak')
            if os.path.exists(p):
                with open(p, 'r+') as f:
                    f.seek(30000)
                    f.write('X')
        self.hi.rsync = corrupt
        r = self.move([('steam', self.drive, self.hades)])
        self.assertTrue(os.path.exists(os.path.join(self.hades['path'], 'Content', 'big.pak')))
        self.assertTrue(os.path.exists(self.hades['manifest']))
        self.assertFalse(os.path.exists(os.path.join(self.steamapps, 'common', 'Hades')))
        self.assertFalse(os.path.exists(os.path.join(self.steamapps, 'appmanifest_1145360.acf')))   # never registered
        self.assertEqual(len(r['failed']), 1)
        self.assertIn("didn't match", r['failed'][0]['why'])
        self.assertEqual(self.writable, [])

    def test_missing_file_in_copy(self):
        def lose(src, dst, on_bytes):
            fake_rsync(src, dst, on_bytes)
            if os.path.exists(os.path.join(dst, 'Content', 'a.txt')):
                os.remove(os.path.join(dst, 'Content', 'a.txt'))
        self.hi.rsync = lose
        r = self.move([('steam', self.drive, self.hades)])
        self.assertTrue(os.path.exists(self.hades['path']))
        self.assertEqual(len(r['failed']), 1)

    def test_stop_during_copy_leaves_original_and_removes_partial(self):
        def half(src, dst, on_bytes):
            os.makedirs(os.path.join(dst, 'Content'), exist_ok=True)
            write(os.path.join(dst, 'Content', 'big.pak'), 'p' * 100)
            self.hi.current['stop'] = True
            raise RuntimeError('rsync was stopped')
        self.hi.rsync = half
        r = self.move([('steam', self.drive, self.hades), ('steam', self.drive, self.portal)])
        self.assertTrue(r.get('stopped'))
        self.assertFalse(os.path.exists(os.path.join(self.steamapps, 'common', 'Hades')))
        self.assertTrue(os.path.exists(os.path.join(self.hades['path'], 'Content', 'big.pak')))
        self.assertTrue(os.path.exists(self.hades['manifest']))
        self.assertFalse(os.path.exists(os.path.join(self.steamapps, 'common', 'Portal 2')))   # the next one never started
        self.assertEqual(r['done'], [])

    def test_stop_during_check(self):
        flag = os.path.join(self.root, 'cancel')
        self.hi.current['flag'] = flag
        orig = self.hi.same_bytes

        def slow(a, b, on_bytes=None, chunk=4 * 1024 * 1024):
            write(flag, '')
            return orig(a, b, on_bytes, chunk)
        self.hi.same_bytes = slow
        r = self.move([('steam', self.drive, self.hades)])
        self.assertTrue(r.get('stopped'))
        self.assertTrue(os.path.exists(self.hades['path']))
        self.assertFalse(os.path.exists(os.path.join(self.steamapps, 'common', 'Hades')))

    def test_windows_not_shut_down_keeps_the_original(self):
        def refuse(d, mnt, state):
            raise self.hi.Keep("Windows wasn't fully shut down (Fast Startup or hibernation)")
        self.hi.make_writable = refuse
        r = self.move([('steam', self.drive, self.hades)])
        self.assertTrue(os.path.exists(self.hades['path']))
        self.assertTrue(os.path.exists(self.hades['manifest']))
        self.assertTrue(os.path.exists(os.path.join(self.steamapps, 'appmanifest_1145360.acf')))   # the copy is good and registered
        self.assertEqual(len(r['kept']), 1)
        self.assertIn('Fast Startup', r['kept'][0]['why'])
        self.assertEqual(r['freed'], [])

    def test_copy_mode_never_deletes(self):
        r = self.move([('steam', self.drive, self.hades)], mode='copy')
        self.assertTrue(os.path.exists(self.hades['path']))
        self.assertEqual(r['moved'], [])
        self.assertEqual(self.writable, [])

    def test_folder_holding_other_games_is_never_deleted(self):
        games = os.path.join(self.mnt, 'Games')
        write(os.path.join(games, 'One', 'one.exe'), 'x')
        write(os.path.join(games, 'Two', 'two.exe'), 'y')
        lut = {'key': 'lutris:sda1:all', 'name': 'All', 'slug': 'all', 'size': 2, 'path': games, 'records': []}
        other = {'key': 'gog:sda1:2', 'id': '2', 'name': 'Two', 'size': 1, 'path': os.path.join(games, 'Two')}
        self.drive['lutris'], self.drive['gog'] = [lut], [other]
        r = self.move([('lutris', self.drive, lut)])
        self.assertTrue(os.path.exists(os.path.join(games, 'Two', 'two.exe')))
        self.assertEqual(len(r['kept']), 1)

    def test_launcher_records(self):
        cfg = os.path.join(self.root, 'installed.json')
        write(cfg, json.dumps({'Sugar': {'title': 'RL'}, 'Other': {}}))
        self.hi.remove_record({'type': 'json-key', 'path': cfg, 'key': 'Sugar'}, {})
        with open(cfg) as f:
            self.assertEqual(json.load(f), {'Other': {}})
        import sqlite3
        db = os.path.join(self.root, 'pga.db')
        con = sqlite3.connect(db)
        con.execute('CREATE TABLE games (slug TEXT, installed INTEGER)')
        con.execute("INSERT INTO games VALUES ('a', 1), ('b', 1)")
        con.commit()
        con.close()
        self.hi.remove_record({'type': 'lutris', 'path': db, 'slug': 'a'}, {})
        con = sqlite3.connect(db)
        self.assertEqual(sorted(con.execute('SELECT slug, installed FROM games').fetchall()), [('a', 0), ('b', 1)])
        con.close()


class Checking(unittest.TestCase):
    def setUp(self):
        self.hi = load()
        self.hi.current.update(stop=False, flag=None)
        self.root = tempfile.mkdtemp()
        self.a, self.b = os.path.join(self.root, 'a'), os.path.join(self.root, 'b')
        write(os.path.join(self.a, 'x', 'f.bin'), 'abc' * 1000)
        write(os.path.join(self.a, 'g.txt'), 'g')
        os.symlink('g.txt', os.path.join(self.a, 'l'))
        shutil.copytree(self.a, self.b, symlinks=True)

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_identical(self):
        self.assertEqual(self.hi.verify_tree(self.a, self.b), (3, 3001))

    def test_differences_are_caught(self):
        V = self.hi.VerifyError
        with open(os.path.join(self.b, 'x', 'f.bin'), 'w') as f:
            f.write('abc' * 999 + 'abd')                         # same size, one byte different
        with self.assertRaisesRegex(V, 'different'):
            self.hi.verify_tree(self.a, self.b)
        shutil.rmtree(self.b)
        shutil.copytree(self.a, self.b, symlinks=True)
        os.remove(os.path.join(self.b, 'l'))
        os.symlink('x', os.path.join(self.b, 'l'))
        with self.assertRaisesRegex(V, 'link'):
            self.hi.verify_tree(self.a, self.b)
        shutil.rmtree(self.b)
        shutil.copytree(self.a, self.b, symlinks=True)
        with open(os.path.join(self.b, 'g.txt'), 'a') as f:
            f.write('more')
        with self.assertRaisesRegex(V, 'size'):
            self.hi.verify_tree(self.a, self.b)
        shutil.rmtree(os.path.join(self.b, 'x'))
        with self.assertRaises(V):
            self.hi.verify_tree(self.a, self.b)

    @unittest.skipUnless(hasattr(os, 'mkfifo'), 'no fifos here')
    def test_special_files_refused(self):
        os.mkfifo(os.path.join(self.a, 'pipe'))
        with self.assertRaisesRegex(self.hi.VerifyError, 'special'):
            self.hi.verify_tree(self.a, self.b)

    def test_safe_to_delete(self):
        hi = self.hi
        mnt = self.root
        self.assertTrue(hi.safe_to_delete(self.a, mnt))
        self.assertFalse(hi.safe_to_delete(mnt, mnt))                                      # the drive itself
        self.assertFalse(hi.safe_to_delete(os.path.join(self.a, 'l'), mnt))                # a link
        self.assertFalse(hi.safe_to_delete(self.a, mnt, [os.path.join(self.a, 'x')]))       # holds something protected
        os.makedirs(os.path.join(mnt, 'Windows'))
        self.assertFalse(hi.safe_to_delete(os.path.join(mnt, 'Windows'), mnt))
        self.assertFalse(hi.safe_to_delete(self.a, os.path.join(mnt, 'b')))                # not on that drive

    def test_make_writable_refuses_hibernated_windows(self):
        hi = self.hi
        ran = []
        orig = hi.subprocess.run
        hi.subprocess.run = lambda *a, **k: ran.append(a) or orig(['true'])
        hi.is_readonly = lambda m: True
        try:
            with self.assertRaisesRegex(hi.Keep, 'Fast Startup'):
                hi.make_writable({'fs': 'ntfs', 'dev': '/dev/x', 'ours': True, 'forced': True}, self.root, {'rw': set(), 'ours': set()})
            with self.assertRaisesRegex(hi.Keep, 'another app'):
                hi.make_writable({'fs': 'ntfs', 'dev': '/dev/x', 'ours': False}, self.root, {'rw': set(), 'ours': set()})
            self.assertEqual(ran, [])                                                     # never even tried to mount it
            # A hibernated Windows found by the probe: back to read-only, original kept
            hi.ntfs_writable = lambda dev: (False, "Windows wasn't fully shut down")
            with self.assertRaisesRegex(hi.Keep, 'fully shut down'):
                hi.make_writable({'fs': 'ntfs', 'dev': '/dev/x', 'ours': True, 'mountOpts': ['-t', 'ntfs3', '-o', 'ro']}, self.root, {'rw': set(), 'ours': set()})
            self.assertFalse(any('rw' in c[0] for c in ran))
        finally:
            hi.subprocess.run = orig


if __name__ == '__main__':
    unittest.main()
