"""Tests for hearth-steam-import (os/system_files/usr/bin/hearth-steam-import, shipped in Hearth Sync as steam_import.py)
and Hearth Sync's import calls. Fake Steam trees in a temporary home; no Steam, nothing outside the temporary folder."""
import sys
sys.dont_write_bytecode = True
import asyncio
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import tempfile
import types
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
TOOL = ROOT / 'os/system_files/usr/bin/hearth-steam-import'


def load_tool():
    loader = importlib.machinery.SourceFileLoader('hearth_steam_import_t', str(TOOL))
    spec = importlib.util.spec_from_loader('hearth_steam_import_t', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def write(p, text=''):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write(text)


def manifest(lib, appid, name, installdir, game=True):
    write(os.path.join(lib, 'steamapps', 'appmanifest_%s.acf' % appid),
          '"AppState"\n{\n\t"appid"\t\t"%s"\n\t"name"\t\t"%s"\n\t"installdir"\t\t"%s"\n}\n' % (appid, name, installdir))
    if game:
        write(os.path.join(lib, 'steamapps', 'common', installdir, 'game.bin'), 'x')


class SteamImport(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-import-')
        self.home = os.path.join(self.tmp, 'home')
        self.mnt = os.path.join(self.tmp, 'mnt')
        os.makedirs(self.mnt)
        self.old = dict(os.environ)
        os.environ['HEARTH_STEAM_IMPORT_HOME'] = self.home
        os.environ['HEARTH_STEAM_IMPORT_MOUNTS'] = self.mnt
        self.m = load_tool()
        self.native = os.path.join(self.home, '.local/share/Steam')
        write(os.path.join(self.native, 'steam.sh'), '#!/bin/sh')
        write(os.path.join(self.native, 'steamapps', 'libraryfolders.vdf'),
              '"libraryfolders"\n{\n\t"0"\n\t{\n\t\t"path"\t\t"%s"\n\t\t"apps"\n\t\t{\n\t\t\t"570"\t\t"1"\n\t\t}\n\t}\n}\n' % self.native)
        manifest(self.native, '570', 'Dota 2', 'dota 2 beta')
        self.flat = os.path.join(self.home, '.var/app/com.valvesoftware.Steam/.local/share/Steam')
        write(os.path.join(self.flat, 'steam.sh'), '#!/bin/sh')
        manifest(self.flat, '1091500', 'Cyberpunk 2077', 'Cyberpunk 2077')
        write(os.path.join(self.flat, 'steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/save.dat'), 'save')
        write(os.path.join(self.flat, 'steamapps/shadercache/1091500/cache.bin'), 'c')
        manifest(self.flat, '570', 'Dota 2', 'dota 2 beta')            # Steam already has it
        manifest(self.flat, '999', 'Gone Game', 'Gone', game=False)    # manifest without its folder: not a game

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_vdf_round_trip(self):
        text = '// comment\n"libraryfolders"\n{\n\t"0"\n\t{\n\t\t"path"\t\t"/a \\"b\\""\n\t\t"apps" { "1" "2" }\n\t}\n}\n'
        d = self.m.vdf_loads(text)
        self.assertEqual(d['libraryfolders']['0']['path'], '/a "b"')
        self.assertEqual(self.m.vdf_loads(self.m.vdf_dumps(d)), d)
        with self.assertRaises(ValueError):
            self.m.vdf_loads('"a" { "b" }')

    def test_scan_finds_flatpak_games_and_skips_what_steam_has(self):
        p = self.m.scan()
        self.assertEqual(p['target'], os.path.realpath(self.native))
        self.assertEqual([g['appid'] for g in p['games']], ['1091500'])
        self.assertEqual(p['games'][0]['how'], 'move')
        self.assertEqual([(g['appid'], g['why']) for g in p['skipped']], [('570', 'Steam already has it')])

    def test_moves_game_prefix_and_shadercache_on_the_same_drive(self):
        res = self.m.run_import(running=lambda: False)
        self.assertEqual(res['counts']['moved'], 1)
        n = os.path.join(self.native, 'steamapps')
        self.assertTrue(os.path.isfile(os.path.join(n, 'appmanifest_1091500.acf')))
        self.assertTrue(os.path.isfile(os.path.join(n, 'common/Cyberpunk 2077/game.bin')))
        self.assertEqual(open(os.path.join(n, 'compatdata/1091500/pfx/drive_c/users/steamuser/save.dat')).read(), 'save')
        self.assertTrue(os.path.isfile(os.path.join(n, 'shadercache/1091500/cache.bin')))
        self.assertFalse(os.path.exists(os.path.join(self.flat, 'steamapps/common/Cyberpunk 2077')))
        self.assertIn('1 moved', open(os.path.join(self.home, '.local/state/hearth/steam-import.log')).read())
        self.assertEqual(self.m.scan()['games'], [], 'a second run finds nothing more')

    def test_never_overwrites(self):
        write(os.path.join(self.native, 'steamapps/common/Cyberpunk 2077/mine.txt'), 'keep')
        p = self.m.scan()
        self.assertEqual(p['games'], [])
        self.assertIn('already in', [g for g in p['skipped'] if g['appid'] == '1091500'][0]['why'])
        self.m.run_import(running=lambda: False)
        self.assertTrue(os.path.isfile(os.path.join(self.flat, 'steamapps/appmanifest_1091500.acf')), 'nothing moved')
        self.assertEqual(open(os.path.join(self.native, 'steamapps/common/Cyberpunk 2077/mine.txt')).read(), 'keep')

    def test_existing_prefix_is_kept_and_said(self):
        write(os.path.join(self.native, 'steamapps/compatdata/1091500/new.txt'), 'new')
        res = self.m.run_import(running=lambda: False)
        self.assertEqual(res['counts']['moved'], 1)
        self.assertTrue(any('compatdata kept at the old place' in n for n in res['notes']))
        self.assertTrue(os.path.isfile(os.path.join(self.flat, 'steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/save.dat')))
        self.assertEqual(os.listdir(os.path.join(self.native, 'steamapps/compatdata/1091500')), ['new.txt'])

    def test_a_failed_move_is_undone(self):
        real_rename = os.rename
        calls = []

        def flaky(a, b):
            calls.append(a)
            if a.endswith('appmanifest_1091500.acf'):
                raise PermissionError(13, 'Permission denied')
            return real_rename(a, b)
        os.rename = flaky
        try:
            res = self.m.run_import(running=lambda: False)
        finally:
            os.rename = real_rename
        self.assertEqual(res['counts']['failed'], 1)
        self.assertTrue(os.path.isfile(os.path.join(self.flat, 'steamapps/common/Cyberpunk 2077/game.bin')), 'moved back')
        self.assertTrue(os.path.isfile(os.path.join(self.flat, 'steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/save.dat')))
        self.assertFalse(os.path.exists(os.path.join(self.native, 'steamapps/common/Cyberpunk 2077')))

    def test_refuses_while_steam_runs(self):
        with self.assertRaises(RuntimeError):
            self.m.run_import(running=lambda: True)
        self.assertTrue(os.path.isfile(os.path.join(self.flat, 'steamapps/appmanifest_1091500.acf')))

    def test_steam_running_reads_proc(self):
        proc = os.path.join(self.tmp, 'proc')
        write(os.path.join(proc, '12', 'comm'), 'bash\n')
        self.assertFalse(self.m.steam_running(proc))
        write(os.path.join(proc, '34', 'comm'), 'steam\n')
        self.assertTrue(self.m.steam_running(proc))

    def test_other_drive_library_is_registered_not_copied(self):
        lib = os.path.join(self.mnt, 'Games', 'SteamLibrary')
        manifest(lib, '413150', 'Stardew Valley', 'Stardew Valley')
        dev = os.stat(os.path.join(self.native, 'steamapps')).st_dev
        real_stat = os.stat

        class S:
            def __init__(self, st, dev):
                self.st_dev, self.st_mode = dev, st.st_mode
        os.stat = lambda p, *a, **k: S(real_stat(p, *a, **k), dev + 1) if str(p).startswith(lib) else real_stat(p, *a, **k)
        try:
            p = self.m.scan()
            self.assertEqual({g['appid']: g['how'] for g in p['games']}, {'1091500': 'move', '413150': 'register'})
            res = self.m.run_import(running=lambda: False)
        finally:
            os.stat = real_stat
        self.assertEqual(res['counts']['registered'], 1)
        self.assertTrue(os.path.isfile(os.path.join(lib, 'steamapps/common/Stardew Valley/game.bin')), 'left where it is')
        vdf = self.m.read_vdf(os.path.join(self.native, 'steamapps/libraryfolders.vdf'))
        paths = [v['path'] for v in vdf['libraryfolders'].values()]
        self.assertEqual(paths, [self.native, os.path.realpath(lib)])
        self.assertEqual(vdf['libraryfolders']['1']['apps'], {'413150': '0'})
        self.assertEqual(vdf['libraryfolders']['0']['apps'], {'570': '1'}, 'the rest is kept')
        self.assertTrue(os.path.isfile(os.path.join(self.native, 'steamapps/libraryfolders.vdf.hearth-bak')))
        self.assertEqual(self.m.scan()['games'], [], 'registered once')

    def test_a_whole_steam_install_on_another_drive_isnt_registered(self):
        other = os.path.join(self.mnt, 'Old', 'Steam')
        write(os.path.join(other, 'steam.sh'), '#!/bin/sh')
        manifest(other, '22380', 'Fallout: New Vegas', 'Fallout New Vegas')
        dev = os.stat(os.path.join(self.native, 'steamapps')).st_dev
        real_stat = os.stat

        class S:
            def __init__(self, st, dev):
                self.st_dev, self.st_mode = dev, st.st_mode
        os.stat = lambda p, *a, **k: S(real_stat(p, *a, **k), dev + 1) if str(p).startswith(other) else real_stat(p, *a, **k)
        try:
            p = self.m.scan()
        finally:
            os.stat = real_stat
        sk = [g for g in p['skipped'] if g['appid'] == '22380']
        self.assertTrue(sk and 'by hand' in sk[0]['why'])

    def test_cli_json_and_refusal(self):
        import io
        import contextlib
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(self.m.main(['x', '--json']), 0)
        self.assertEqual([g['appid'] for g in json.loads(out.getvalue())['games']], ['1091500'])
        self.m.steam_running = lambda proc='/proc': True
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.assertEqual(self.m.main(['x', '--yes']), 1)
        self.assertIn('Steam is running', err.getvalue())
        self.assertEqual(self.m.main(['x', '--bogus']), 2)


class HearthSyncImport(unittest.TestCase):
    """Hearth Sync's backend calls for the import (main.py import_scan / arm_import / import_state)."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-sync-import-')
        self.old = dict(os.environ)
        fake = types.ModuleType('decky')
        fake.DECKY_USER_HOME = os.path.join(self.tmp, 'home')
        fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(self.tmp, 'settings')
        fake.logger = types.SimpleNamespace(info=lambda m: None)
        sys.modules['decky'] = fake
        spec = importlib.util.spec_from_file_location('hearth_sync_main_i', ROOT / 'hosts/decky-hearth-sync/main.py')
        self.m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.m)
        self.m.IMPORT_PROGRAMS = (str(TOOL),)
        os.environ['HEARTH_STEAM_IMPORT_MOUNTS'] = ''
        flat = os.path.join(fake.DECKY_USER_HOME, '.var/app/com.valvesoftware.Steam/.local/share/Steam')
        manifest(flat, '1091500', 'Cyberpunk 2077', 'Cyberpunk 2077')
        os.makedirs(os.path.join(fake.DECKY_USER_HOME, '.local/share/Steam/steamapps'))

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_scan_through_the_plugin(self):
        r = asyncio.new_event_loop().run_until_complete(self.m.Plugin().import_scan())
        self.assertTrue(r['ok'])
        self.assertEqual([(g['appid'], g['how']) for g in r['games']], [('1091500', 'move')])
        self.assertNotIn('manifest', r['games'][0])

    def test_arm_starts_the_waiting_helper(self):
        seen = {}

        class P:
            pid = 999999

        def popen(cmd, **kw):
            seen['cmd'], seen['kw'] = cmd, kw
            return P()
        self.assertTrue(self.m.arm_import(popen=popen))
        self.assertEqual(seen['cmd'][-3:], ['--yes', '--wait', '--json'])
        self.assertIn(str(TOOL), seen['cmd'])
        self.assertTrue(seen['kw']['start_new_session'])
        self.assertEqual(seen['kw']['env']['HEARTH_STEAM_IMPORT_HOME'], os.path.join(self.tmp, 'home'))
        st = self.m.import_state()
        self.assertFalse(st['pending'], 'pid 999999 is not running')
        self.assertIsNone(st['result'])
        self.m.save_json('import.json', {'ok': True, 'counts': {'moved': 1}})
        self.assertEqual(self.m.import_state()['result']['counts']['moved'], 1)


if __name__ == '__main__':
    unittest.main()
