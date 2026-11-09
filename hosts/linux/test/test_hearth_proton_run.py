"""hearth-proton-run (docs/WINDOWS-APPS.md, "Proton: offline and without Steam"): the owner's red flag that Hearth's
built-in Proton works without Steam and 100% offline. umu-run's command line and variables, the built-in GE-Proton when
nothing (or something missing) is picked, no Steam folder needed, nothing on the start path touching the network, and
Proton's own Python (protonfixes) kept offline. Every Hearth call site starts Windows programs with it.

The real thing (GE-Proton running Windows programs with networking off and no Steam folder) is winapps.yml's
end-to-end job (tools/winapps-e2e/e2e.sh)."""
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import unittest
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')
RUN_PY = os.path.join(SHELL, 'hearth_proton_run.py')
sys.path.insert(0, SHELL)

import hearth_proton as hp  # noqa: E402
import hearth_proton_run as hpr  # noqa: E402


def read(*p):
    with open(os.path.join(ROOT, *p), encoding='utf-8') as f:
        return f.read()


FAKE_PROTON = r'''
import json, os, socket, sys, urllib.request
out = {"argv": sys.argv[1:], "env": {k: os.environ.get(k) for k in (
    "UMU_ID", "GAMEID", "SteamAppId", "SteamGameId", "STEAM_COMPAT_APP_ID", "WINEPREFIX", "STEAM_COMPAT_DATA_PATH",
    "STEAM_COMPAT_CLIENT_INSTALL_PATH", "STEAM_COMPAT_INSTALL_PATH", "PROTONPATH", "STORE", "EXE", "PROTON_VERB",
    "UMU_USE_STEAM", "HTTPS_PROXY", "MY_GAME_SETTING")}, "here": os.path.dirname(sys.argv[0])}
# What GE-Proton11-3's protonfixes does: the FSR4 manifest (urllib), check_internet before winetricks (a UDP socket
# to 1.1.1.1:53), a name lookup, and a direct TCP connection.
def tried(f):
    try:
        f()
        return "reached"
    except Exception as e:
        return "blocked: %s" % e
out["urlopen"] = tried(lambda: urllib.request.urlopen("https://loathingkernel.github.io/proton-upscalers/manifest.json", timeout=3))
def udp():
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
        s.settimeout(2)
        s.connect(("1.1.1.1", 53))
out["udp"] = tried(udp)
out["dns"] = tried(lambda: socket.getaddrinfo("github.com", 443))
out["tcp"] = tried(lambda: socket.create_connection(("140.82.112.3", 443), timeout=2))
loop = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
loop.bind(("127.0.0.1", 0)); loop.listen(1)
out["loopback"] = tried(lambda: socket.create_connection(loop.getsockname(), timeout=2).close())
print("FAKE-PROTON " + json.dumps(out), flush=True)
sys.exit(int(os.environ.get("FAKE_EXIT", "0")))
'''


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-proton-run-')
        self.home = os.path.join(self.tmp, 'home')
        self.sys = os.path.join(self.tmp, 'sys')
        os.makedirs(self.home)
        self.saved = {k: os.environ.get(k) for k in ('HOME', 'HEARTH_BUILTIN_PROTON_DIR', 'PROTONPATH', 'GAMEID', 'STORE',
                                                     'WINEPREFIX', 'STEAM_COMPAT_INSTALL_PATH', 'STEAM_COMPAT_CLIENT_INSTALL_PATH')}
        for k in self.saved:
            os.environ.pop(k, None)
        os.environ.update({'HOME': self.home, 'HEARTH_BUILTIN_PROTON_DIR': self.sys})
        self.builtin = self.tool(self.sys, 'GE-Proton11-3')

    def tearDown(self):
        for k, v in self.saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        shutil.rmtree(self.tmp, ignore_errors=True)

    def tool(self, d, name, body='#!/bin/sh\n'):
        os.makedirs(os.path.join(d, name), exist_ok=True)
        with open(os.path.join(d, name, 'proton'), 'w') as f:
            f.write(body)
        return os.path.join(d, name)

    def plan(self, exe='/g/Game/game.exe', args=(), **env):
        e = {'HOME': self.home, 'HEARTH_BUILTIN_PROTON_DIR': self.sys}
        e.update(env)
        saved = dict(os.environ)
        os.environ.clear()
        os.environ.update(e)
        try:
            return hpr.plan(exe, list(args), e, cwd=self.tmp)
        finally:
            os.environ.clear()
            os.environ.update(saved)


class Plan(Base):
    def test_umu_runs_command_line_with_the_builtin_proton(self):
        cmd, env, said = self.plan('/g/Game/game.exe', ['-skipintro', 'two words'], GAMEID='umu-489830', STORE='gog',
                                   WINEPREFIX=os.path.join(self.tmp, 'pfx'), MY_GAME_SETTING='1')
        self.assertEqual(cmd, ['python3', os.path.realpath(RUN_PY), '--in-proton', self.builtin, 'waitforexitandrun',
                               '/g/Game/game.exe', '-skipintro', 'two words'])
        self.assertEqual((env['UMU_ID'], env['GAMEID'], env['SteamAppId'], env['SteamGameId'], env['STORE']),
                         ('umu-489830', 'umu-489830', '489830', '489830', 'gog'))
        self.assertEqual(env['WINEPREFIX'], os.path.join(self.tmp, 'pfx'))
        self.assertEqual(env['STEAM_COMPAT_DATA_PATH'], env['WINEPREFIX'], "Proton's data folder is the prefix, as umu-run")
        self.assertEqual(env['STEAM_COMPAT_CLIENT_INSTALL_PATH'], '', 'no Steam folder')
        self.assertEqual((env['PROTONPATH'], env['PROTON_VERB'], env['MY_GAME_SETTING']), (self.builtin, 'waitforexitandrun', '1'))
        self.assertIn('no Proton picked', said[0])
        self.assertIn('offline and without Steam', said[1])

    def test_game_ids_as_umu_reads_them(self):
        self.assertEqual(self.plan()[1]['UMU_ID'], 'umu-default')
        self.assertEqual(self.plan()[1]['SteamAppId'], 'default')
        self.assertEqual(self.plan(GAMEID='umu-battlenet')[1]['SteamAppId'], 'battlenet')
        self.assertEqual(self.plan(GAMEID='0')[1]['SteamAppId'], '0')

    def test_default_prefix_as_umu(self):
        self.assertEqual(self.plan()[1]['WINEPREFIX'], os.path.join(os.path.realpath(self.home), 'Games', 'umu', 'umu-default'))
        self.assertEqual(self.plan(GAMEID='umu-70')[1]['WINEPREFIX'], os.path.join(os.path.realpath(self.home), 'Games', 'umu', 'umu-70'))
        self.assertEqual(self.plan(STORE='egs')[1]['WINEPREFIX'], os.path.join(os.path.realpath(self.home), 'Games', 'egs'))
        with self.assertRaises(hpr.Fail):
            self.plan(WINEPREFIX='')

    def test_never_through_steam(self):
        env = self.plan(UMU_USE_STEAM='1')[1]
        self.assertNotIn('UMU_USE_STEAM', env)
        self.assertFalse(os.path.exists(os.path.join(self.home, '.local', 'share', 'Steam')))
        # Steam's own folder when a caller names it (Hearth Mods, for Steam DRM games) passes through.
        self.assertEqual(self.plan(STEAM_COMPAT_CLIENT_INSTALL_PATH='/s')[1]['STEAM_COMPAT_CLIENT_INSTALL_PATH'], '/s')

    def test_program_paths(self):
        g = os.path.join(self.tmp, 'Game')
        os.makedirs(g)
        open(os.path.join(g, 'game.exe'), 'w').close()
        cmd, env, _ = self.plan('Game/game.exe')
        self.assertEqual((cmd[5], env['STEAM_COMPAT_INSTALL_PATH'], env['EXE']), (os.path.join(g, 'game.exe'), g, os.path.join(g, 'game.exe')))
        cmd, env, _ = self.plan('winecfg')
        self.assertEqual(cmd[5], 'winecfg', 'a Windows command stays as it is')
        self.assertEqual(self.plan('/g/x.exe', STEAM_COMPAT_INSTALL_PATH='/g/root')[1]['STEAM_COMPAT_INSTALL_PATH'], '/g/root')

    def test_winetricks_is_ge_protons_own(self):
        os.makedirs(os.path.join(self.builtin, 'protonfixes'))
        open(os.path.join(self.builtin, 'protonfixes', 'winetricks'), 'w').close()
        cmd, env, _ = self.plan('winetricks', ['-q', 'vcrun2022'], PROTON_VERB='run')
        wt = os.path.join(self.builtin, 'protonfixes', 'winetricks')
        self.assertEqual(cmd[3:], [self.builtin, 'waitforexitandrun', wt, '-q', 'vcrun2022'])
        self.assertEqual((env['EXE'], env['PROTON_VERB'], env['WINETRICKS_SUPER_QUIET']), (wt, 'waitforexitandrun', '1'))
        shutil.rmtree(os.path.join(self.builtin, 'protonfixes'))
        with self.assertRaises(hpr.Fail):
            self.plan('winetricks')

    def test_no_proton_at_all_says_so(self):
        shutil.rmtree(self.sys)
        with self.assertRaises(hpr.Fail):
            self.plan()


class Resolve(Base):
    def test_protonpath_names_and_folders(self):
        steam_ct = os.path.join(self.home, '.local', 'share', 'Steam', 'compatibilitytools.d')
        newer = self.tool(os.path.join(self.home, '.local', 'share', 'hearth', 'proton'), 'GE-Proton12-1')
        older = self.tool(steam_ct, 'GE-Proton9-27')
        umu = self.tool(steam_ct, 'UMU-Proton-9.0-3.2')
        valve = self.tool(os.path.join(self.home, '.local', 'share', 'Steam', 'steamapps', 'common'), 'Proton 10.0')
        cases = {
            None: self.builtin, '': self.builtin,
            'GE-Proton11-3': self.builtin, 'builtin': self.builtin,
            'GE-Proton': newer, 'GE-Latest': newer,           # the newest GE-Proton here: a downloaded newer one wins
            'GE-Proton9-27': older, 'UMU-Proton': umu, 'UMU-Proton-9.0-3.2': umu,
            'Proton 10.0': valve, valve: valve, valve + '/': valve,
            'Made-Up-Proton': self.builtin,                   # not here: the built-in one, never a download
            '/gone/GE-Proton9-27': older,                     # a moved folder: the same name elsewhere
            '/gone/Nothing': self.builtin,
            '../evil': self.builtin,
        }
        for want, path in cases.items():
            self.assertEqual(hp.resolve(want)[1], path, want)
        self.assertIn("isn't on this device", hp.resolve('Made-Up-Proton')[2])
        self.assertEqual(hp.resolve('Made-Up-Proton')[0], 'GE-Proton11-3')

    def test_umu_proton_not_here_means_builtin(self):
        self.assertEqual(hp.resolve('UMU-Proton')[1], self.builtin)

    def test_print_proton(self):
        r = subprocess.run([sys.executable, RUN_PY, '--print-proton'], capture_output=True, text=True,
                           env=dict(os.environ, PROTONPATH='Made-Up'))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.split('\t')[:2], ['GE-Proton11-3', self.builtin])


class Prefix(Base):
    def test_prepared_as_umu_prepares_it(self):
        p = os.path.join(self.tmp, 'prefix')
        hpr.prepare_prefix(p)
        self.assertEqual(os.readlink(os.path.join(p, 'pfx')), '.')
        for f in ('shadercache', 'gstreamer-1.0', 'tracked_files'):
            self.assertTrue(os.path.exists(os.path.join(p, f)), f)
        users = os.path.join(p, 'drive_c', 'users')
        self.assertTrue(os.path.isdir(os.path.join(users, 'steamuser')))
        import pwd
        me = pwd.getpwuid(os.getuid()).pw_name
        if me != 'steamuser':
            self.assertEqual(os.readlink(os.path.join(users, me)), 'steamuser')
        hpr.prepare_prefix(p)  # again: nothing changes, nothing breaks
        self.assertEqual(os.readlink(os.path.join(p, 'pfx')), '.')

    def test_a_steam_style_prefix_is_left_alone(self):
        p = os.path.join(self.tmp, 'compat')
        os.makedirs(os.path.join(p, 'pfx', 'drive_c'))
        hpr.prepare_prefix(p)
        self.assertFalse(os.path.islink(os.path.join(p, 'pfx')))
        self.assertTrue(os.path.isdir(os.path.join(p, 'pfx', 'drive_c')))

    def test_a_moved_prefix_link_is_fixed(self):
        p = os.path.join(self.tmp, 'moved')
        os.makedirs(p)
        os.symlink('/old/place', os.path.join(p, 'pfx'))
        hpr.prepare_prefix(p)
        self.assertEqual(os.readlink(os.path.join(p, 'pfx')), '.')


class Offline(Base):
    def test_start_path_never_touches_the_network(self):
        """plan, prepare_prefix and resolve with every way to the network made to fail the test."""
        def boom(*a, **k):
            raise AssertionError('the start path used the network')
        patches = [(socket.socket, 'connect'), (socket.socket, 'connect_ex'), (socket, 'getaddrinfo'),
                   (socket, 'create_connection'), (urllib.request.OpenerDirector, 'open'), (urllib.request, 'urlopen')]
        saved = [(o, n, getattr(o, n)) for o, n in patches]
        for o, n in patches:
            setattr(o, n, boom)
        try:
            cmd, env, _ = self.plan(GAMEID='umu-1', PROTONPATH='GE-Proton')
            hpr.prepare_prefix(env['WINEPREFIX'])
            hp.resolve('UMU-Proton')
            hp.resolve('Made-Up')
        finally:
            for o, n, f in saved:
                setattr(o, n, f)

    def test_hearth_proton_imports_no_network_module(self):
        code = 'import sys; sys.path.insert(0, %r); import hearth_proton, hearth_proton_run; print("urllib.request" in sys.modules, "http.client" in sys.modules)' % SHELL
        r = subprocess.run([sys.executable, '-I', '-c', code], capture_output=True, text=True)
        self.assertEqual(r.stdout.split(), ['False', 'False'], r.stderr)

    def run_start(self, extra_env=None, args=('/g/game.exe', '-a')):
        self.tool(self.sys, 'GE-Proton11-3', FAKE_PROTON)
        env = {'HOME': self.home, 'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'HEARTH_BUILTIN_PROTON_DIR': self.sys,
               'GAMEID': 'umu-489830', 'STORE': 'gog', 'WINEPREFIX': os.path.join(self.tmp, 'prefix'),
               'HTTPS_PROXY': 'http://127.0.0.1:9', 'https_proxy': 'http://127.0.0.1:9', 'MY_GAME_SETTING': 'kept'}
        env.update(extra_env or {})
        r = subprocess.run([sys.executable, RUN_PY] + list(args), capture_output=True, text=True, env=env, timeout=60)
        line = [l for l in r.stdout.splitlines() if l.startswith('FAKE-PROTON ')]
        self.assertTrue(line, 'Proton never ran: %s %s' % (r.stdout, r.stderr))
        return r, json.loads(line[-1][len('FAKE-PROTON '):])

    def test_proton_runs_with_its_network_off_and_no_steam(self):
        r, out = self.run_start({'FAKE_EXIT': '3'})
        self.assertEqual(r.returncode, 3, "Proton's exit code comes back")
        self.assertEqual(out['argv'], ['waitforexitandrun', '/g/game.exe', '-a'])
        self.assertEqual(os.path.realpath(out['here']), os.path.realpath(self.builtin))
        e = out['env']
        self.assertEqual((e['UMU_ID'], e['SteamAppId'], e['STEAM_COMPAT_CLIENT_INSTALL_PATH'], e['STORE'], e['MY_GAME_SETTING']),
                         ('umu-489830', '489830', '', 'gog', 'kept'))
        self.assertEqual(e['HTTPS_PROXY'], 'http://127.0.0.1:9', 'the game keeps its environment')
        for k in ('urlopen', 'udp', 'dns', 'tcp'):
            self.assertTrue(out[k].startswith('blocked'), '%s: %s' % (k, out[k]))
            self.assertIn('offline', out[k].lower(), out[k])
        self.assertEqual(out['loopback'], 'reached', 'this PC itself is still reachable')
        self.assertIn('offline and without Steam', r.stderr)
        self.assertFalse(os.path.exists(os.path.join(self.home, '.local', 'share', 'Steam')), 'no Steam folder made')
        self.assertFalse(os.path.exists(os.path.join(self.home, '.steam')))
        self.assertEqual(os.readlink(os.path.join(self.tmp, 'prefix', 'pfx')), '.')

    def test_a_missing_pick_still_starts_offline(self):
        r, out = self.run_start({'PROTONPATH': 'UMU-Proton'})
        self.assertEqual(os.path.realpath(out['here']), os.path.realpath(self.builtin))
        self.assertIn('built into Hearth OS', r.stderr)

    def test_usage(self):
        r = subprocess.run([sys.executable, RUN_PY], capture_output=True, text=True)
        self.assertEqual(r.returncode, 2)
        self.assertIn('usage', r.stderr)


class Wiring(unittest.TestCase):
    """Every Hearth place that starts a Windows program with Proton uses hearth-proton-run (umu-run is only the
    fallback where the image has no hearth-proton-run), and the image ships and checks it."""

    def test_image_ships_it(self):
        w = read('os', 'system_files', 'usr', 'bin', 'hearth-proton-run')
        self.assertIn('desktop/shell/hearth_proton_run.py', w)
        self.assertTrue(os.access(os.path.join(ROOT, 'os', 'system_files', 'usr', 'bin', 'hearth-proton-run'), os.X_OK))
        b = read('os-bases', 'peppermint', 'build.sh')
        self.assertIn('/usr/bin/hearth-proton-run', b)
        self.assertIn('hearth-proton-run --print-proton', b)

    def test_rust_call_sites(self):
        core = os.path.join(ROOT, 'hosts', 'hearth-core', 'src')
        gaming = os.path.join(ROOT, 'hosts', 'hearth-gaming', 'src')
        allowed_umu = {'proton_run.rs', 'launch.rs', 'tools.rs'}
        faugus = {'battlenet.rs'}  # Faugus Launcher's own umu-run (Faugus is left alone)
        for base in (core, gaming):
            for dirpath, _, files in os.walk(base):
                for f in files:
                    if not f.endswith('.rs'):
                        continue
                    text = open(os.path.join(dirpath, f), encoding='utf-8').read()
                    code = '\n'.join(l.split('//')[0] for l in text.split('#[cfg(test)]')[0].splitlines())
                    if f not in allowed_umu:
                        self.assertNotIn('tools::UMU', code, '%s downloads umu-launcher' % f)
                        self.assertNotRegex(code, r'(has|on_path)\("umu-run"\)', '%s looks for umu-run instead of hearth-proton-run' % f)
                        if f not in faugus:
                            self.assertNotRegex(code, r'"umu-run"', '%s starts umu-run' % f)
        for f, want in (('stores.rs', 'proton_run::command()'), ('manual.rs', 'proton_run::ready()'),
                        ('steam_engine.rs', 'proton_run::ready()'), ('recomps.rs', 'proton_run::exec_name'),
                        ('battlenet.rs', 'proton_run::command()')):
            self.assertIn(want, read('hosts', 'hearth-core', 'src', f), f)
        self.assertIn('offline_runner()', read('hosts', 'hearth-gaming', 'src', 'launch.rs'))
        self.assertNotRegex(read('hosts', 'hearth-core', 'src', 'proton_pick.rs').split('#[cfg(test)]')[0],
                            r'"[^"]*downloaded when needed', 'Default never means a download')

    def test_js_call_sites(self):
        self.assertIn('tools.protonRunner()', read('hosts', 'linux', 'src', 'stores', 'proton.js'))
        self.assertIn('offlineRunner()', read('hosts', 'linux', 'src', 'library', 'manual.js'))
        self.assertNotIn('downloaded when needed', read('hosts', 'linux', 'src', 'library', 'game-options.js'))

    def test_the_rule_is_written_down(self):
        self.assertIn("Hearth OS's built-in Proton must work without Steam and 100% offline", read('docs', 'OFFLINE-PROTON.md'))
        self.assertIn('hearth-proton-run', read('docs', 'ROADMAP.md'))
        self.assertIn('hearth-proton-run', read('docs', 'WINDOWS-APPS.md'))
        self.assertIn('hearth-proton-run', read('docs', 'STEAM.md'))

    def test_the_network_off_check_runs(self):
        e2e = read('tools', 'winapps-e2e', 'e2e.sh')
        self.assertIn('offline()', e2e)
        self.assertIn('--network=none', read('.github', 'workflows', 'winapps.yml'))
        self.assertIn('--network=none', read('.github', 'workflows', 'base-peppermint.yml'))


if __name__ == '__main__':
    unittest.main()
