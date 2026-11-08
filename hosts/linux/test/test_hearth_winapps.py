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

        def run(cmd, env, cwd, log=None):
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
        r = self.w.start(a, p, [], runner=lambda c, e, d, log=None: 1, clock=lambda: next(t))
        self.assertTrue(r['crashed'])
        self.assertEqual(r['next'], {'name': 'GE-Proton10-34', 'download': True})
        self.assertEqual(self.w.state(a)['next']['name'], 'GE-Proton10-34')
        # A long run that ends with an error isn't a crash.
        t = iter([0, 600])
        self.assertFalse(self.w.start(a, p, [], runner=lambda c, e, d, log=None: 1, clock=lambda: next(t))['crashed'])

    def test_installer_finds_its_program(self):
        p = self.exe('SteamSetup.exe')
        holder = {}

        def run(cmd, env, cwd, log=None):
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


def make_lnk(path, target, args='', cwd='', icon=''):
    """A Windows shortcut (Shell Link format) the way an installer leaves one: LinkInfo with the target's local path,
    then the working folder, arguments and icon as Unicode strings."""
    import struct
    flags = 0x02 | 0x80 | (0x10 if cwd else 0) | (0x20 if args else 0) | (0x40 if icon else 0)
    header = struct.pack('<I16sIIQQQIIIHHII', 0x4C, b'\x01\x14\x02\x00\x00\x00\x00\x00\xc0\x00\x00\x00\x00\x00\x00\x46',
                         flags, 0x20, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0)
    assert len(header) == 76
    base = target.encode('cp1252') + b'\0'
    hsize = 0x1C
    vol = struct.pack('<IIII', 16, 3, 0, 16) + b'\0'
    vol_off = hsize
    base_off = vol_off + len(vol)
    suffix_off = base_off + len(base)
    body = vol + base + b'\0'
    info = struct.pack('<7I', hsize + len(body), hsize, 1, vol_off, base_off, 0, suffix_off) + body
    out = header + info
    for v in (cwd, args, icon):
        if v:
            out += struct.pack('<H', len(v)) + v.encode('utf-16-le')
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb') as f:
        f.write(out)


class Installers(Base):
    """Unigine Heaven 4.0 on a laptop (Hearth 1.7.0): an Inno Setup installer whose name doesn't say "setup", a
    launcher .bat with arguments, files that keep their own old dates."""

    def inno(self, name='Unigine_Heaven-4.0.exe'):
        p = self.exe(name)
        with open(p, 'wb') as f:
            f.write(b'MZ' + b'\0' * 200 + b'InnoSetupLdrWindow\0' + b'\0' * 100 + b'Inno Setup Setup Data (6.2.0)')
        return p

    def heaven_install(self, prefix):
        """What Heaven's installer leaves: its files (with 2013 dates), a launcher .bat, Start menu shortcuts."""
        top = os.path.join(prefix, 'drive_c', 'Program Files (x86)', 'Unigine', 'Heaven Benchmark 4.0')
        os.makedirs(os.path.join(top, 'bin'), exist_ok=True)
        for rel, size in (('bin/browser_x86.exe', 300), ('bin/Heaven_x64.exe', 900), ('bin/Heaven_x86.exe', 800),
                          ('unins000.exe', 5000), ('heaven.bat', 60)):
            with open(os.path.join(top, rel), 'wb') as f:
                f.write(b'x' * size)
            os.utime(os.path.join(top, rel), (1370000000, 1370000000))
        menu = os.path.join(prefix, 'drive_c', 'ProgramData', 'Microsoft', 'Windows', 'Start Menu', 'Programs', 'Unigine', 'Heaven Benchmark 4.0')
        base = r'C:\Program Files (x86)\Unigine\Heaven Benchmark 4.0'
        make_lnk(os.path.join(menu, 'Heaven Benchmark 4.0.lnk'), base + r'\bin\browser_x86.exe',
                 args='-config ../data/launcher/launcher.xml', cwd=base + r'\bin', icon=base + r'\bin\browser_x86.exe,0')
        make_lnk(os.path.join(menu, 'Uninstall Heaven Benchmark 4.0.lnk'), base + r'\unins000.exe')
        make_lnk(os.path.join(menu, 'Heaven Benchmark 4.0 Website.lnk'), base + r'\bin\browser_x86.exe', args='http://unigine.com')
        return top

    def test_installer_is_known_by_what_it_is(self):
        p = self.inno()
        self.assertEqual(self.w.installer_kind(p), 'Inno Setup')
        self.assertTrue(self.w.is_installer(p))
        self.assertFalse(self.w.is_installer(self.exe('Heaven.exe')))

    def test_heaven_installs_and_its_shortcut_becomes_the_app(self):
        p = self.inno()
        calls = []

        def run(cmd, env, cwd, log=None):
            calls.append((cmd, env, cwd))
            if p in cmd:
                self.heaven_install(env['WINEPREFIX'])
            return 0
        r = self.w.cmd_open(p, runner=run)
        a = self.w.app(r['id'])
        top = os.path.join(a['prefix'], 'drive_c', 'Program Files (x86)', 'Unigine', 'Heaven Benchmark 4.0')
        self.assertEqual(calls[0][0][3], p, 'the installer ran first')
        self.assertEqual(calls[0][2], os.path.dirname(p), 'in its own folder')
        self.assertEqual(a['exe'], os.path.join(top, 'bin', 'browser_x86.exe'), "the shortcut's program, not the biggest .exe")
        self.assertEqual(a['args'], ['-config', '../data/launcher/launcher.xml'])
        self.assertEqual(a['cwd'], os.path.join(top, 'bin'))
        self.assertTrue(a['installer'])
        self.assertEqual(r['installed'], a['exe'])
        calls.clear()
        self.w.start(a, a['exe'], a['args'], runner=run)
        cmd, env, cwd = calls[-1]
        self.assertEqual(cmd[3:], [a['exe'], '-config', '../data/launcher/launcher.xml'])
        self.assertEqual(cwd, os.path.join(top, 'bin'), "started in the shortcut's folder")

    def test_old_app_that_is_its_installer_is_repaired(self):
        """Hearth 1.7.0 kept Unigine_Heaven-4.0.exe as the program: the next start runs Heaven, not the installer."""
        p = self.inno()
        a = self.w.new_app('Unigine_Heaven-4.0', p, False)
        self.w.save_app(a['dir'], {'exe': p})
        self.heaven_install(a['prefix'])
        calls = []
        r = self.w.cmd_open(p, runner=lambda c, e, d, log=None: calls.append(c) or 0)
        a = self.w.app(r['id'])
        self.assertTrue(a['exe'].endswith('/bin/browser_x86.exe'), a['exe'])
        self.assertEqual(calls[-1][3], a['exe'])
        self.assertEqual(len(self.w.apps()), 1)

    def test_bat_runs_through_cmd(self):
        p = self.exe('heaven.bat', 'Games/Heaven')
        calls = []
        r = self.w.cmd_open(p, runner=lambda c, e, d, log=None: calls.append((c, d)) or 0)
        cmd, cwd = calls[-1]
        self.assertEqual(cmd[3:5], ['cmd', '/c'])
        self.assertEqual(cmd[5], 'Z:' + p.replace('/', '\\'))
        self.assertEqual(cwd, os.path.dirname(p))
        self.assertTrue(r['new'])

    def test_win_and_unix_paths(self):
        a = self.w.new_app('x', '/x.exe', False)
        self.w.ensure_prefix(a['prefix'])
        f = os.path.join(a['prefix'], 'drive_c', 'Program Files', 'A b', 'c.exe')
        os.makedirs(os.path.dirname(f))
        open(f, 'w').close()
        self.assertEqual(self.w.win_path(f, a['prefix']), r'C:\Program Files\A b\c.exe')
        self.assertEqual(self.w.unix_path(a['prefix'], r'c:\program files\a B\C.EXE'), f)
        self.assertEqual(self.w.unix_path(a['prefix'], r'C:\nope\c.exe'), '')
        self.assertEqual(self.w.split_args('-a "b c" d'), ['-a', 'b c', 'd'])


class Logs(Base):
    def app_(self):
        p = self.exe('game.exe')
        a = self.w.new_app('game', p, False)
        self.w.save_app(a['dir'], {'exe': p})
        return self.w.load_app(a['dir']), p

    def fake_proton(self, body):
        d = os.path.join(self.sys, 'GE-Proton11-3')
        with open(os.path.join(d, 'proton'), 'w') as f:
            f.write('import os, sys, time\n' + body)

    def test_every_start_is_logged_with_proton_output(self):
        a, p = self.app_()
        self.fake_proton('print("proton says hi", flush=True)\nsys.stderr.write("err: wine broke\\n")\n'
                         'print("env", os.environ.get("UMU_ID"), repr(os.environ.get("STEAM_COMPAT_CLIENT_INSTALL_PATH")))\n'
                         'sys.exit(3)\n')
        r = self.w.start(a, p, [])
        self.assertEqual(r['code'], 3)
        self.assertTrue(r['crashed'])
        log = open(self.w.last_log(a)).read()
        for want in ('Proton: GE-Proton11-3', 'Command: python3 ', 'waitforexitandrun ' + p, 'proton says hi',
                     'err: wine broke', "env 0 ''", 'with code 3'):
            self.assertIn(want, log)
        bl = open(os.path.join(self.home, '.local', 'share', 'nimbus', 'bridge.log')).read()
        self.assertRegex(bl, r'\d+ \[winapps\] windows apps: game started with GE-Proton11-3, closed after \d+\.\d s with code 3, closed right away')
        st = self.w.state(a)
        self.assertEqual((st['code'], st['log']), (3, self.w.last_log(a)))
        self.assertIn('err: wine broke', self.w.cmd_log(a['id'])['tail'])

    def test_waits_for_what_the_start_left_running(self):
        """A launcher that starts the real program and exits: the log keeps the program's output, the time is its."""
        a, p = self.app_()
        self.fake_proton('import subprocess\n'
                         'subprocess.Popen([sys.executable, "-c", "import time; time.sleep(1); print(\'game output\', flush=True)"])\n'
                         'print("launcher done", flush=True)\n')
        r = self.w.start(a, p, [])
        log = open(self.w.last_log(a)).read()
        self.assertIn('game output', log)
        self.assertGreaterEqual(r['seconds'], 0.9)
        self.assertEqual(r['code'], 0)

    def test_keeps_the_last_five_starts(self):
        a, p = self.app_()
        for i in range(7):
            self.w.start(a, p, [], runner=lambda c, e, d, log=None, i=i: log.output(b'run %d\n' % i) or 0)
        names = sorted(os.listdir(self.w.logs_dir(a)))
        self.assertEqual(names, ['last.1.log', 'last.2.log', 'last.3.log', 'last.4.log', 'last.log'])
        self.assertIn('run 6', open(self.w.last_log(a)).read())
        self.assertIn('run 2', open(os.path.join(self.w.logs_dir(a), 'last.4.log')).read())

    def test_detailed_proton_log_is_for_one_start(self):
        a, p = self.app_()
        os.environ.pop('PROTON_LOG', None)
        envs = []
        run = lambda c, e, d, log=None: envs.append(e) or 0  # noqa: E731
        self.w.cmd_detailed(a['id'], 'on')
        self.assertTrue(self.w.public(a)['detailedNext'])
        self.w.start(a, p, [], runner=run)
        self.assertEqual((envs[-1].get('PROTON_LOG'), envs[-1].get('PROTON_LOG_DIR')), ('1', self.w.logs_dir(a)))
        self.assertFalse(self.w.public(a)['detailedNext'], 'used up by that start')
        self.assertIn('Detailed Proton log', open(self.w.last_log(a)).read())
        self.w.start(a, p, [], runner=run)
        self.assertNotIn('PROTON_LOG', envs[-1])
        self.assertNotIn('PROTON_LOG', os.environ)

    def test_proton_starts_like_umu_without_steam(self):
        a, p = self.app_()
        env = self.w.proton_env(a, p)
        self.assertEqual((env['UMU_ID'], env['STEAM_COMPAT_CLIENT_INSTALL_PATH'], env['STEAM_COMPAT_INSTALL_PATH']),
                         ('0', '', os.path.dirname(p)))
        self.assertNotIn('UMU_USE_STEAM', env)

    def test_report_for_send_logs(self):
        a, p = self.app_()
        self.w.start(a, p, [], runner=lambda c, e, d, log=None: log.output(b'wine: could not load kernel32.dll\n') or 5)
        t = self.w.cmd_report()
        for want in ('Built-in GE-Proton: GE-Proton11-3', '--- game (game)', 'state: a program opened directly',
                     'with code 5', 'could not load kernel32.dll', '--- Vulkan', 'Steam folders: none'):
            self.assertIn(want, t)


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

    def test_send_logs_has_a_windows_apps_section(self):
        rs = read('hosts', 'hearth-core', 'src', 'logs.rs')
        self.assertIn('section("Windows apps"', rs)
        self.assertIn('shell/hearth-winapps', rs)

    def test_icons_from_files_show_in_mantel(self):
        """A menu entry's Icon= that is a picture file (Windows apps' extracted icons) is drawn from that file, not
        looked for as icons/<path>-light.svg."""
        q = read('hosts', 'linux', 'desktop', 'shell', 'Icon.qml')
        self.assertIn('AppIcons.fromIcon', q)
        self.assertIn('IconData.icons[root.file]', q)

    def test_settings_page_shows_logs(self):
        q = read('hosts', 'linux', 'desktop', 'shell', 'SetWinApps.qml')
        for want in ('"detailed"', 'Show log', 'Detailed Proton log for the next start'):
            self.assertIn(want, q)


if __name__ == '__main__':
    unittest.main()
