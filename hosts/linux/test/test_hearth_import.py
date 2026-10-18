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


if __name__ == '__main__':
    unittest.main()
