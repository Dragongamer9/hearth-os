"""Windows apps and the Proton detector (docs/WINDOWS-APPS.md): hearth-winapps, hearth_proton.py, the bundled table,
the image's built-in GE-Proton, and Hearth Games' side (proton_pick.rs)."""
import importlib.machinery
import importlib.util
import json
import os
import re
import shutil
import sys
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')
RULES = os.path.join(ROOT, 'os', 'system_files', 'usr', 'share', 'hearth', 'proton-rules.json')
sys.path.insert(0, SHELL)


def load_helper():
    loader = importlib.machinery.SourceFileLoader('hearth_winapps', os.path.join(SHELL, 'hearth-winapps'))
    spec = importlib.util.spec_from_loader('hearth_winapps', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def read(*p):
    with open(os.path.join(ROOT, *p), encoding='utf-8') as f:
        return f.read()


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-winapps-')
        self.home = os.path.join(self.tmp, 'home')
        self.sys = os.path.join(self.tmp, 'sys')
        os.makedirs(self.home)
        self.env = {k: os.environ.get(k) for k in ('HOME', 'HEARTH_BUILTIN_PROTON_DIR', 'HEARTH_PROTON_RULES', 'XDG_DATA_HOME', 'HEARTH_WINAPPS_BIN', 'XDG_STATE_HOME')}
        os.environ.update({'HOME': self.home, 'HEARTH_BUILTIN_PROTON_DIR': self.sys, 'HEARTH_PROTON_RULES': RULES,
                           'HEARTH_WINAPPS_BIN': '/nonexistent', 'XDG_STATE_HOME': os.path.join(self.tmp, 'state')})
        os.environ.pop('XDG_DATA_HOME', None)
        self.tool(self.sys, 'GE-Proton11-3')
        self.hp = __import__('hearth_proton')
        self.w = load_helper()
        self.w.notify = lambda *a: None

    def tearDown(self):
        for k, v in self.env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        shutil.rmtree(self.tmp, ignore_errors=True)

    def tool(self, d, name):
        os.makedirs(os.path.join(d, name), exist_ok=True)
        with open(os.path.join(d, name, 'proton'), 'w') as f:
            f.write('#!/bin/sh\n')

    def exe(self, name, folder='Downloads'):
        d = os.path.join(self.home, folder)
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, name)
        with open(p, 'wb') as f:
            f.write(b'MZ')
        return p


class Detector(Base):
    def test_order_builtin_then_valve_then_older(self):
        steam = os.path.join(self.home, '.local/share/Steam/steamapps/common')
        self.tool(steam, 'Proton 10.0')
        self.tool(steam, 'Proton - Experimental')
        names = [c['name'] for c in self.hp.candidates('winapp:x', 'thing.exe')]
        self.assertEqual(names[:2], ['GE-Proton11-3', 'Proton 10.0'])
        self.assertEqual(names[2:], ['GE-Proton10-34', 'GE-Proton9-27', 'GE-Proton8-32'], 'older GE-Protons last, as downloads')
        self.assertIsNone(self.hp.candidates('winapp:x')[-1]['path'])

    def test_remembers_what_worked_and_moves_crashes_last(self):
        self.tool(os.path.join(self.home, '.local/share/hearth/proton'), 'GE-Proton9-27')
        self.hp.note_worked('winapp:x', 'GE-Proton9-27')
        self.assertEqual(self.hp.pick('winapp:x')['name'], 'GE-Proton9-27')
        self.hp.note_failed('winapp:x', 'GE-Proton9-27')
        self.assertEqual(self.hp.pick('winapp:x')['name'], 'GE-Proton11-3')
        self.assertEqual(self.hp.candidates('winapp:x')[-1]['name'], 'GE-Proton9-27')
        self.assertEqual(self.hp.next_after('winapp:x', 'GE-Proton11-3')['name'], 'GE-Proton10-34')
        self.assertTrue(os.path.exists(self.hp.worked_path() + '.bak') or os.path.exists(self.hp.worked_path()))

    def test_table_rules_match_by_program_name(self):
        r = self.hp.rule_for('/x/Battle.net-Setup.exe')
        self.assertEqual((r['proton'], r['winver']), ('builtin', 'win10'))
        self.assertIsNone(self.hp.rule_for('/x/notepad.exe'))

    def test_damaged_memory_never_crashes(self):
        os.makedirs(os.path.dirname(self.hp.worked_path()), exist_ok=True)
        with open(self.hp.worked_path(), 'w') as f:
            f.write('{nope')
        self.assertEqual(self.hp.pick('winapp:x')['name'], 'GE-Proton11-3')

    def test_downloads_only_from_the_table_with_its_checksum(self):
        with self.assertRaises(ValueError):
            self.hp.fetch('GE-Proton1-1')

        class R:
            headers = {'Content-Length': '4'}

            def __init__(self):
                self.d = [b'evil', b'']

            def read(self, n):
                return self.d.pop(0)

            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False
        with self.assertRaises(ValueError):
            self.hp.fetch('GE-Proton9-27', opener=lambda url: R())
        self.assertEqual([n for n in os.listdir(self.hp.user_dir()) if not n.startswith('.')], [], 'nothing kept from a bad download')


class Apps(Base):
    def runner(self, code=0, took=60):
        calls = []
        t = [0]

        def run(cmd, env, cwd):
            calls.append((cmd, env, cwd))
            return code
        self.clock_vals = iter([0, took] * 10)
        return calls, run

    def test_open_exe_makes_prefix_entry_and_runs_with_builtin(self):
        p = self.exe('Notepad++ 8.6.exe')
        calls, run = self.runner()
        r = self.w.cmd_open(p, runner=run)
        a = self.w.app(r['id'])
        self.assertEqual(a['name'], 'Notepad++')
        self.assertEqual(a['prefix'], os.path.join(self.home, 'Games', 'Windows apps', 'Notepad++', 'prefix'))
        self.assertTrue(os.path.islink(os.path.join(a['prefix'], 'pfx')))
        cmd, env, cwd = calls[-1]
        self.assertEqual(cmd[1:4], [os.path.join(self.sys, 'GE-Proton11-3', 'proton'), 'waitforexitandrun', p])
        self.assertEqual(env['STEAM_COMPAT_DATA_PATH'], a['prefix'])
        desk = open(self.w.desktop_path(a['id'])).read()
        self.assertIn('Exec=/usr/bin/python3 %s run %s' % (os.path.realpath(os.path.join(SHELL, 'hearth-winapps')), a['id']), desk)
        self.assertIn('X-Hearth-WinApp=' + a['id'], desk)
        self.assertEqual(self.hp.remembered('winapp:' + a['id'])['worked'], 'GE-Proton11-3')
        # Opening it again runs the same app, not a second one.
        self.w.cmd_open(p, runner=run)
        self.assertEqual(len(self.w.apps()), 1)

    def test_crash_within_seconds_offers_next(self):
        p = self.exe('game.exe')
        a = self.w.new_app('game', p, False)
        self.w.save_app(a['dir'], {'exe': p})
        a = self.w.load_app(a['dir'])
        t = iter([0, 2])
        r = self.w.start(a, p, [], runner=lambda c, e, d: 1, clock=lambda: next(t))
        self.assertTrue(r['crashed'])
        self.assertEqual(r['next'], {'name': 'GE-Proton10-34', 'download': True})
        self.assertEqual(self.w.state(a)['next']['name'], 'GE-Proton10-34')
        # A long run that ends with an error isn't a crash.
        t = iter([0, 600])
        self.assertFalse(self.w.start(a, p, [], runner=lambda c, e, d: 1, clock=lambda: next(t))['crashed'])

    def test_installer_finds_its_program(self):
        p = self.exe('SteamSetup.exe')
        holder = {}

        def run(cmd, env, cwd):
            pf = os.path.join(env['WINEPREFIX'], 'drive_c', 'Program Files (x86)', 'Steam')
            os.makedirs(pf, exist_ok=True)
            for n, size in (('steam.exe', 5000), ('uninstall.exe', 9000), ('steamerrorreporter.exe', 100)):
                with open(os.path.join(pf, n), 'wb') as f:
                    f.write(b'x' * size)
            holder['cmd'] = cmd
            return 0
        r = self.w.cmd_open(p, runner=run)
        a = self.w.app(r['id'])
        self.assertEqual(a['name'], 'Steam')
        self.assertTrue(a['exe'].endswith('/Steam/steam.exe'), a['exe'])
        self.assertTrue(os.path.isfile(self.w.desktop_path(a['id'])))

    def test_msi_runs_through_msiexec(self):
        p = self.exe('Tool.msi')
        calls, run = self.runner()
        self.w.cmd_open(p, runner=run)
        self.assertEqual(calls[-1][0][3:6], ['msiexec', '/i', p])

    def test_settings_become_proton_env_and_are_kept_safely(self):
        p = self.exe('thing.exe')
        a = self.w.new_app('thing', p, False)
        self.w.cmd_set(a['id'], 'dxvk', 'off')
        self.w.cmd_set(a['id'], 'vkd3d', 'off')
        self.w.cmd_set(a['id'], 'winver', 'win7')
        a = self.w.app(a['id'])
        env = self.w.proton_env(a)
        self.assertEqual((env.get('PROTON_USE_WINED3D'), env.get('PROTON_NO_D3D12')), ('1', '1'))
        self.assertTrue(os.path.exists(os.path.join(a['dir'], 'hearth-app.json.bak')), 'written with hearth_settings_io (keeps .bak)')
        with self.assertRaises(self.w.Fail):
            self.w.cmd_set(a['id'], 'winver', 'win95')
        with self.assertRaises(self.w.Fail):
            self.w.cmd_set(a['id'], 'exe', '/etc/passwd')
        with self.assertRaises(self.w.Fail):
            self.w.cmd_set(a['id'], 'proton', 'Made-Up-Proton')

    def test_windows_version_from_the_table_is_set_once(self):
        p = self.exe('Battle.net-Setup.exe')
        calls, run = self.runner()
        self.w.cmd_open(p, runner=run)
        regs = [c for c, _, _ in calls if 'reg' in c]
        self.assertEqual(len(regs), 1)
        self.assertIn('win10', regs[0])

    def test_steam_and_remove(self):
        p = self.exe('thing.exe')
        calls, run = self.runner()
        i = self.w.cmd_open(p, runner=run)['id']
        self.w.cmd_steam(i, 'on')
        d = json.load(open(self.w.steam_file()))
        self.assertEqual(d['apps'][0]['id'], 'hearth-winapps:' + i)
        self.assertEqual(d['apps'][0]['launch'][-2:], ['run', i])
        a = self.w.app(i)
        self.w.cmd_remove(i)
        self.assertFalse(os.path.exists(a['dir']))
        self.assertFalse(os.path.exists(self.w.desktop_path(i)))
        self.assertEqual(json.load(open(self.w.steam_file()))['apps'], [])
        self.assertTrue(os.path.isfile(p), "the .exe you opened is never deleted")

    def test_names(self):
        self.assertEqual(self.w.clean_name('Battle.net-Setup'), 'Battle.net')
        self.assertEqual(self.w.clean_name('setup'), 'Windows app')
        self.assertTrue(self.w.is_installer('/x/ChromeSetup.exe'))
        self.assertTrue(self.w.is_installer('/x/a.msi'))
        self.assertFalse(self.w.is_installer('/x/game.exe'))


class Wiring(unittest.TestCase):
    def test_image_pins_the_builtin_ge_proton_from_the_table(self):
        b = read('os-bases', 'peppermint', 'build.sh')
        ver = re.search(r'^GE_PROTON=(GE-Proton[\d-]+)$', b, re.M).group(1)
        self.assertRegex(re.search(r'^GE_PROTON_SHA256=(\S+)$', b, re.M).group(1), r'^[0-9a-f]{64}$')
        self.assertEqual(json.load(open(RULES))['builtin'], ver, 'the table names the GE-Proton the image has')
        block = b[b.index('GE_PROTON='):b.index('icoutils')]
        self.assertGreaterEqual(block.count('exit 1'), 3, 'a failed download or checksum stops the build')
        self.assertIn('ln -sfn "/usr/share/hearth/proton/$GE_PROTON" "/usr/share/steam/compatibilitytools.d/$GE_PROTON"', block)
        self.assertNotIn('config.vdf', block, "Steam's settings are never touched")
        self.assertIn('/usr/bin/hearth-winapps', b)

    def test_older_ge_are_pinned(self):
        t = json.load(open(RULES))
        for o in t['older_ge']:
            self.assertRegex(o['name'], r'^GE-Proton\d+-\d+$')
            self.assertRegex(o['sha512'], r'^[0-9a-f]{128}$')
        for r in t['rules']:
            self.assertIn(r['proton'], ['builtin', 'valve'] + [o['name'] for o in t['older_ge']])

    def test_handler_and_files_and_rust_agree(self):
        d = read('os', 'system_files', 'usr', 'share', 'applications', 'hearth-winapps.desktop')
        self.assertIn('application/x-ms-dos-executable;', d)
        self.assertIn('application/x-msi;', d)
        self.assertIn('Exec=/usr/bin/hearth-winapps open %f', d)
        fm = read('hosts', 'linux', 'desktop', 'shell', 'FileManager.qml')
        self.assertIn('"hearth-winapps", "open", e.path', fm)
        rs = read('hosts', 'hearth-core', 'src', 'proton_pick.rs')
        self.assertIn('"/usr/share/hearth/proton-rules.json"', rs)
        self.assertIn('.local/share/nimbus/proton-worked.json', rs)
        self.assertIn('{ id: "winapps", file: "SetWinApps.qml"', read('hosts', 'linux', 'desktop', 'shell', 'settings-pages.js'))


if __name__ == '__main__':
    unittest.main()
