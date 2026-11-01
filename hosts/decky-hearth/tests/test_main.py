"""Tests for the Hearth Decky plugin's backend (hosts/decky-hearth/main.py) and its theme file. No Steam, no Decky:
a stand-in `decky` module, and a fake Hearth app."""
import sys
sys.dont_write_bytecode = True
import asyncio
import base64
import importlib.util
import json
import os
import pathlib
import re
import shlex
import shutil
import tempfile
import types
import unittest

HERE = pathlib.Path(__file__).resolve().parents[1]
ROOT = HERE.parents[1]


def load(tmp):
    fake = types.ModuleType('decky')
    fake.DECKY_USER_HOME = os.path.join(tmp, 'home')
    fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(tmp, 'settings')
    fake.DECKY_PLUGIN_DIR = os.path.join(tmp, 'plugin')
    fake.logger = types.SimpleNamespace(info=lambda *a: None)
    sys.modules['decky'] = fake
    spec = importlib.util.spec_from_file_location('hearth_decky_main', HERE / 'main.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def run(coro):
    return asyncio.new_event_loop().run_until_complete(coro)


PNG = b'\x89PNG\r\n\x1a\n' + b'0' * 64
JPG = b'\xff\xd8\xff\xe0' + b'0' * 64
WEBP = b'RIFF\x20\x00\x00\x00WEBPVP8 ' + b'0' * 32
GIF = b'GIF89a' + b'0' * 32
AVIF = b'\x00\x00\x00\x1cftypavif' + b'0' * 32


def real_png(w=4, h=6):
    """A real (tiny) PNG, made here: zlib and the PNG chunk layout."""
    import struct, zlib
    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    raw = b''.join(b'\x00' + bytes([200, 60, 40]) * w for _ in range(h))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')


class Backend(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='decky-hearth-test-')
        os.makedirs(os.path.join(self.tmp, 'home'))
        self.old = dict(os.environ)
        self.m = load(self.tmp)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_steam_games_are_left_out_and_sorted(self):
        raw = [{'id': 'steam:570', 'title': 'Dota 2', 'source': 'steam'},
               {'id': 'epic:fn', 'title': 'fortnite', 'source': 'epic', 'cover': '/c.png'},
               {'id': 'beacon:bfme2', 'title': 'BFME II', 'source': 'beacon'},
               {'id': 'bad', 'title': 'no colon'}, {'id': 'gog:"x', 'title': 'quote'}, 'junk',
               {'id': 'manual:1', 'title': '  ', 'source': 'manual'}]
        out = self.m.clean_games(raw)
        self.assertEqual([g['id'] for g in out], ['beacon:bfme2', 'epic:fn', 'manual:1'])
        self.assertEqual(out[1]['store'], 'Epic Games')
        self.assertEqual(out[2]['title'], 'manual:1')
        self.assertEqual(self.m.clean_games('nope'), [])

    def test_shortcut_runs_hearth_play_now_quoted(self):
        t = self.m.shortcut('/usr/bin/nimbus', ['--play-now', "emu:snes:Mario's World.sfc"], False)
        self.assertEqual(t['exe'], '/bin/sh', 'a wrapper that cleans Steam\'s environment first')
        self.assertEqual(shlex.split(t['options']), ['-c', self.m.WRAPPER, 'hearth-steam', '/usr/bin/nimbus', '--play-now', "emu:snes:Mario's World.sfc"])
        self.assertEqual(self.m.shortcut('/usr/bin/nimbus', ['--play-now', 'gog:1'], True), self.m.shortcut('/usr/bin/nimbus', ['--play-now', 'gog:1'], False),
                         "the same shortcut for both Steams: the wrapper decides at every launch")
        self.assertIn('[ -e /.flatpak-info ]', self.m.WRAPPER)
        self.assertIn('flatpak-spawn --host /usr/bin/env -u LD_PRELOAD -u LD_LIBRARY_PATH "$@"', self.m.WRAPPER)

    def test_shortcut_cleans_steams_environment_when_run(self):
        """Runs the shortcut the way Steam does (its exe and options through a shell, in Steam's environment) with a
        stand-in for Hearth that writes down what it got."""
        out = os.path.join(self.tmp, 'got')
        fake = os.path.join(self.tmp, 'nimbus')
        with open(fake, 'w') as f:
            f.write('#!/bin/sh\n{ printf "%%s\\n" "$@"; echo "LDLP=${LD_LIBRARY_PATH-unset}"; echo "PRE=${LD_PRELOAD-unset}"; '
                    'echo "SET_ASIDE=$HEARTH_STEAM_LD_PRELOAD"; echo "GAMEID=$SteamGameId"; } > %s\n' % out)
        os.chmod(fake, 0o755)
        t = self.m.shortcut(fake, ['--play-now', "emu:snes:Mario's World.sfc"])
        # LD_PRELOAD empty here: a real overlay library would be loaded into the test's own shell too.
        env = dict(os.environ, LD_LIBRARY_PATH='/steam/ubuntu12_32/steam-runtime/lib', LD_PRELOAD='', SteamGameId='123')
        import subprocess
        subprocess.run(['/bin/sh', '-c', '%s %s' % (shlex.quote(t['exe']), t['options'])], env=env, check=True)
        got = open(out).read().splitlines()
        self.assertEqual(got[:2], ['--play-now', "emu:snes:Mario's World.sfc"], 'arguments arrive whole')
        self.assertIn('LDLP=unset', got, "Steam's runtime libraries are gone before Hearth starts")
        self.assertIn('PRE=unset', got)
        self.assertIn('GAMEID=123', got, "Steam's id for the shortcut stays")

    def test_targets_include_hearths_apps_only_when_installed(self):
        games = self.m.clean_games([{'id': 'gog:1', 'title': 'A', 'source': 'gog'}])
        finder = lambda p: {'nimbus': '/usr/bin/nimbus', 'hearth-mods': '/usr/bin/hearth-mods'}.get(p)
        g, links = self.m.targets(games, False, finder=finder)
        self.assertEqual(shlex.split(g[0]['options'])[3:], ['/usr/bin/nimbus', '--play-now', 'gog:1'])
        ids = [l['id'] for l in links]
        self.assertIn('hearth-app:mods', ids)
        self.assertIn('hearth-app:store', ids)
        self.assertNotIn('hearth-app:theater', ids)
        store = [l for l in links if l['id'] == 'hearth-app:store'][0]
        self.assertEqual(shlex.split(store['options'])[4:], ['--windowed', '--page', 'shop'])

    def test_list_games_asks_hearth(self):
        app = os.path.join(self.tmp, 'nimbus')
        with open(app, 'w') as f:
            f.write('#!/bin/sh\n[ "$1" = --list-games ] || exit 3\necho \'[{"id":"epic:a","title":"A","source":"epic"},{"id":"steam:1","title":"S","source":"steam"}]\'\n')
        os.chmod(app, 0o755)
        self.m.find_program = lambda name: app if name == 'nimbus' else None
        r = self.m.list_games()
        self.assertTrue(r['ok'], r)
        self.assertEqual([g['id'] for g in r['games']], ['epic:a'])
        self.m.find_program = lambda name: None
        self.assertFalse(self.m.list_games()['ok'])

    def test_clean_env_drops_deckys_library_path(self):
        os.environ['LD_LIBRARY_PATH'] = '/tmp/_MEI1234'
        os.environ.pop('LD_LIBRARY_PATH_ORIG', None)
        os.environ.pop('XDG_RUNTIME_DIR', None)
        env = self.m.clean_env()
        self.assertNotIn('LD_LIBRARY_PATH', env)
        self.assertTrue(env['XDG_RUNTIME_DIR'].startswith('/run/user/'))
        os.environ['LD_LIBRARY_PATH_ORIG'] = '/opt/lib'
        self.assertEqual(self.m.clean_env()['LD_LIBRARY_PATH'], '/opt/lib')

    def test_flatpak_steam_is_seen_from_its_environment(self):
        proc = os.path.join(self.tmp, 'proc')
        for pid, comm, env in (('10', 'bash', b'FLATPAK_ID=com.valvesoftware.Steam\0'), ('11', 'steam', b'HOME=/h\0')):
            os.makedirs(os.path.join(proc, pid))
            open(os.path.join(proc, pid, 'comm'), 'w').write(comm + '\n')
            open(os.path.join(proc, pid, 'environ'), 'wb').write(env)
        self.assertFalse(self.m.steam_is_flatpak(proc))
        open(os.path.join(proc, '11', 'environ'), 'wb').write(b'FLATPAK_ID=com.valvesoftware.Steam\0')
        self.assertTrue(self.m.steam_is_flatpak(proc))

    def test_art_only_from_allowed_places(self):
        home = os.path.join(self.tmp, 'home')
        pic = os.path.join(home, 'cover.png')
        open(pic, 'wb').write(PNG)
        a = self.m.read_art(pic)
        self.assertEqual(a['type'], 'png')
        self.assertEqual(base64.b64decode(a['data']), PNG)
        self.assertEqual(self.m.read_art('file://' + pic)['type'], 'png')
        outside = os.path.join(self.tmp, 'secret.png')
        open(outside, 'wb').write(PNG)
        self.assertIsNone(self.m.read_art(outside))
        self.assertIsNone(self.m.read_art('/etc/shadow'))
        notpic = os.path.join(home, 'notes.png')
        open(notpic, 'wb').write(b'hello')
        self.assertIsNone(self.m.read_art(notpic))
        self.assertEqual(self.m.read_art('https://x/y.jpg', fetch=lambda u: JPG)['type'], 'jpg')
        self.assertIsNone(self.m.read_art('http://x/y.jpg', fetch=lambda u: JPG), 'plain http is refused')
        self.assertIsNone(self.m.read_art('https://x/y.webp', fetch=lambda u: WEBP, converters=[]),
                          'a webp nothing here can convert is skipped (Steam takes png and jpg only)')
        link = os.path.join(home, 'sneaky.png')
        os.symlink(outside, link)
        self.assertIsNone(self.m.read_art(link), 'a link out of home is followed and refused')

    def test_settings_and_shortcuts_are_kept(self):
        p = self.m.Plugin()
        self.assertEqual(run(p.get_settings()), self.m.DEFAULT_SETTINGS)
        self.assertTrue(self.m.DEFAULT_SETTINGS['takeover'], 'the takeover is on by default')
        run(p.set_settings({'theme': False, 'evil': 'x', 'tkHome': 0}))
        s = run(p.get_settings())
        self.assertFalse(s['theme'])
        self.assertFalse(s['tkHome'])
        self.assertTrue(s['homeRow'])
        self.assertNotIn('evil', s)
        self.assertEqual(run(p.set_shortcuts({'gog:1': 123, 'bad': -1, 'x': 'y'})), {'gog:1': 123})
        self.assertEqual(run(p.get_shortcuts()), {'gog:1': 123})

    def test_brand_reads_theme_logo_and_wallpaper(self):
        pdir = os.path.join(self.tmp, 'plugin')
        os.makedirs(os.path.join(pdir, 'assets'))
        shutil.copy(HERE / 'defaults' / 'hearth-theme.css', pdir)
        shutil.copy(ROOT / 'site' / 'img' / 'logo.svg', os.path.join(pdir, 'assets', 'logo.svg'))
        open(os.path.join(pdir, 'assets', 'wallpaper.jpg'), 'wb').write(JPG)
        b = self.m.brand(pdir)
        self.assertIn('--gpColor-Blue:#3fd0b8', b['css'])
        self.assertTrue(b['logo'].startswith('data:image/svg+xml;base64,'))
        self.assertTrue(b['wallpaper'].startswith('data:image/jpeg;base64,'))

    def test_startup_movie_never_replaces_yours(self):
        pdir, home = os.path.join(self.tmp, 'plugin'), os.path.join(self.tmp, 'home')
        self.assertIsNone(self.m.startup_movie(pdir, home), 'nothing shipped, nothing done')
        os.makedirs(os.path.join(pdir, 'assets'))
        open(os.path.join(pdir, 'assets', 'hearth-startup.webm'), 'wb').write(b'webm')
        dst = self.m.startup_movie(pdir, home)
        self.assertTrue(dst.endswith('.steam/root/config/uioverrides/movies/deck_startup.webm'))
        open(dst, 'wb').write(b'mine')
        self.assertIsNone(self.m.startup_movie(pdir, home))
        self.assertEqual(open(dst, 'rb').read(), b'mine')

    def test_switch_back_runs_nimbus_mode_game(self):
        out = os.path.join(self.tmp, 'called')
        prog = os.path.join(self.tmp, 'nimbus-mode')
        open(prog, 'w').write('#!/bin/sh\necho "$@ $XDG_RUNTIME_DIR" > %s\n' % out)
        os.chmod(prog, 0o755)
        self.m.find_program = lambda name: prog if name == 'nimbus-mode' else None
        self.assertTrue(run(self.m.Plugin().switch_to_hearth())['ok'])
        for _ in range(50):
            if os.path.exists(out) and open(out).read():
                break
            import time
            time.sleep(0.05)
        self.assertTrue(open(out).read().startswith('game '))

    def test_ids_match_hearths_rules(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'desktop_games.rs').read_text()
        self.assertIn("c.is_control() || c == '\"' || c == '\\\\'", rs)
        for ok in ('epic:a', 'emu:snes:A b.sfc'):
            self.assertTrue(self.m.valid_id(ok))
        for bad in ('', 'x', 'a:"', 'a:\\', 'a:\n', None):
            self.assertFalse(self.m.valid_id(bad))


class Takeover(unittest.TestCase):
    """The takeover's backend calls: questions for Hearth, pictures, the parts that turned themselves off."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='decky-hearth-tk-')
        os.makedirs(os.path.join(self.tmp, 'home'))
        self.m = load(self.tmp)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_hearth(self, body):
        app = os.path.join(self.tmp, 'nimbus')
        with open(app, 'w') as f:
            f.write('#!/bin/sh\n' + body)
        os.chmod(app, 0o755)
        self.m.find_program = lambda name: app if name == 'nimbus' else None
        return app

    def test_only_known_questions_are_asked(self):
        a = self.m.data_args
        self.assertEqual(a('home', ''), ['--steam-mode-data', 'home'])
        self.assertEqual(a('store', 'epic'), ['--steam-mode-data', 'store', 'epic'])
        self.assertIsNone(a('store', 'evil; rm -rf'))
        self.assertIsNone(a('store', ''))
        self.assertEqual(a('info', 'gog:1'), ['--steam-mode-data', 'info', 'gog:1'])
        self.assertIsNone(a('info', 'no colon'))
        self.assertEqual(a('power-mode', 'performance'), ['--steam-mode-data', 'power-mode', 'performance'])
        self.assertIsNone(a('power-mode', 'turbo'))
        self.assertIsNone(a('uninstall', 'gog:1'), 'nothing that changes the library')
        self.assertEqual(set(self.m.STORE_PAGES), {'steam', 'gog', 'epic', 'xbox', 'free', 'recomps', 'battlenet'})
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'steam_takeover.rs').read_text()
        for what in self.m.DATA:
            self.assertIn('"%s"' % what, rs, 'hearth-core answers ' + what)
        for st in self.m.STORE_PAGES:
            self.assertIn('"%s"' % st, rs)

    def test_hearth_is_asked_and_answers_are_checked(self):
        self.fake_hearth('[ "$1" = --steam-mode-data ] || exit 3\necho "warming up"\necho "{\\"what\\":\\"$2\\",\\"arg\\":\\"$3\\"}"\n')
        self.assertEqual(self.m.hearth_data('store', 'gog'), {'what': 'store', 'arg': 'gog'})
        self.assertIn('error', self.m.hearth_data('store', 'nope'))
        self.fake_hearth('echo "[1,2]"\necho broken >&2\n')
        self.assertEqual(self.m.hearth_data('home'), {'error': 'broken'})
        self.fake_hearth('echo \'{"error":"Unknown store"}\'\nexit 1\n')
        self.assertEqual(self.m.hearth_data('library'), {'error': 'Unknown store'})
        self.m.find_program = lambda name: None
        self.assertIn('error', self.m.hearth_data('home'))
        p = self.m.Plugin()
        self.assertIn('error', run(p.hearth_data('home')))

    def test_pictures_only_from_allowed_places(self):
        home = os.path.join(self.tmp, 'home')
        png = os.path.join(home, 'a.png')
        open(png, 'wb').write(PNG)
        self.assertTrue(self.m.picture_url(png).startswith('data:image/png;base64,'))
        webp = os.path.join(home, 'a.webp')
        open(webp, 'wb').write(b'RIFF0000WEBPVP8 ')
        self.assertTrue(self.m.picture_url(webp).startswith('data:image/webp;base64,'))
        fake = os.path.join(home, 'fake.jpg')
        open(fake, 'wb').write(b'#!/bin/sh')
        self.assertEqual(self.m.picture_url(fake), '')
        outside = os.path.join(self.tmp, 'x.png')
        open(outside, 'wb').write(PNG)
        self.assertEqual(self.m.picture_url(outside), '')
        self.assertEqual(self.m.picture_url('/etc/passwd'), '')
        self.assertTrue(self.m.picture_url('https://x/y.png', fetch=lambda u: PNG).startswith('data:image/png;base64,'),
                        'a web picture the page could not load is fetched by the backend')
        os.symlink(outside, os.path.join(home, 'l.png'))
        self.assertEqual(self.m.picture_url(os.path.join(home, 'l.png')), '')

    def test_targets_and_links(self):
        self.fake_hearth('exit 0\n')
        self.m.steam_is_flatpak = lambda proc='/proc': False
        p = self.m.Plugin()
        t = run(p.get_target('epic:a'))
        self.assertTrue(t['ok'])
        self.assertEqual(shlex.split(t['options'])[4:], ['--play-now', 'epic:a'])
        self.assertFalse(run(p.get_target('steam:570'))['ok'], "Steam's games play from Steam")
        self.assertFalse(run(p.get_target('bad'))['ok'])
        h = run(p.get_link('hearth-app:hearth'))
        self.assertEqual(shlex.split(h['options'])[4:], ['--windowed'])
        self.assertEqual(shlex.split(run(p.get_link('hearth-app:settings'))['options'])[4:], ['--windowed', '--settings'])
        self.assertIsNone(run(p.get_link('hearth-app:nope')))

    def test_parts_that_turned_off_are_logged_and_cleared_at_start(self):
        p = self.m.Plugin()
        for i in range(60):
            run(p.log_error('tkMenu', 'not found %d\nsecond line' % i))
        e = run(p.get_errors())
        self.assertEqual(len(e), self.m.ERRORS_KEPT)
        self.assertEqual(e[-1], 'tkMenu: not found 59 second line')
        run(p._main())
        self.assertEqual(run(p.get_errors()), [])

    def test_settings_match_the_frontend(self):
        ts = (HERE / 'src' / 'model.ts').read_text()
        block = re.search(r'export const DEFAULTS: Settings = \{(.*?)\};', ts, re.S).group(1)
        front = {k: v == 'true' for k, v in re.findall(r'(\w+): (true|false)', block)}
        self.assertEqual(front, self.m.DEFAULT_SETTINGS)


class CoreService(unittest.TestCase):
    """Hearth's core service (nimbus --core-service) from the plugin's side: a stand-in listening on a socket in a
    private folder, the checks before and after connecting, the fallback to the command line."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='decky-hearth-core-')
        os.makedirs(os.path.join(self.tmp, 'home'))
        self.m = load(self.tmp)
        self.dir = os.path.join(self.tmp, 'run', 'hearth')
        os.makedirs(self.dir, mode=0o700)
        os.chmod(self.dir, 0o700)
        self.path = os.path.join(self.dir, 'core.sock')
        self.seen = []
        self.server = None

    def tearDown(self):
        if self.server:
            self.server.close()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def serve(self, reply):
        import socket, threading
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        srv.bind(self.path)
        os.chmod(self.path, 0o600)
        srv.listen(4)
        self.server = srv

        def loop():
            while True:
                try:
                    c, _ = srv.accept()
                except OSError:
                    return
                with c:
                    line = c.makefile('rb').readline()
                    req = json.loads(line)
                    self.seen.append(req)
                    c.sendall((json.dumps(reply(req)) + '\n').encode())
        threading.Thread(target=loop, daemon=True).start()

    def test_commands_match_hearth_core(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'core_service.rs').read_text()
        cmds = re.search(r'pub const COMMANDS: &\[&str\] = &\[(.*?)\];', rs).group(1)
        self.assertEqual(tuple(re.findall(r'"([^"]+)"', cmds)), self.m.CORE_COMMANDS)
        data = re.search(r'pub const DATA: &\[&str\] = &\[(.*?)\];', rs).group(1)
        self.assertEqual(set(re.findall(r'"([^"]+)"', data)), set(self.m.DATA))

    def test_coop_matches_hearth_core_and_falls_back_to_the_helper(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'core_service.rs').read_text()
        ops = re.search(r'pub const COOP_OPS: &\[&str\] = &\[(.*?)\];', rs).group(1)
        self.assertEqual(tuple(re.findall(r'"([^"]+)"', ops)), self.m.COOP_OPS)
        self.assertEqual(self.m.coop_call('start', ['event1', 'event2']), {'error': "That isn't something the Hearth plugin asks"})
        self.assertIsNone(self.m.coop_args('identify', pad='../../dev/sda'))
        self.assertIsNone(self.m.coop_args('order', ['js0']))
        seen = []
        core = lambda cmd, a: seen.append((cmd, a)) or {'ok': True, 'data': {'pads': []}}
        self.assertEqual(self.m.coop_call('order', ['event30', 'bad;rm', 'event20'], core=core), {'pads': []})
        self.assertEqual(seen, [('coop', {'op': 'order', 'pads': ['event30', 'event20'], 'pad': ''})])
        # No core: hearth-coop itself answers (same helper), its "error:" line becomes {'error'}.
        helper = os.path.join(self.tmp, 'hearth-coop')
        with open(helper, 'w') as f:
            f.write('#!/bin/sh\n')
        calls = []

        class P:
            def __init__(self, out):
                self.stdout, self.stderr = out, ''
        run = lambda argv, **k: calls.append(argv) or P('progress: 5 x\n{"pads": [{"id": "event30", "player": 1}]}\n')
        r = self.m.coop_call('players', core=lambda c, a: None, run=run, helper=helper)
        self.assertEqual(r['pads'][0]['player'], 1)
        self.assertEqual(calls[0], [helper, 'players'])
        run = lambda argv, **k: P('error: That controller is no longer connected.\n')
        self.assertEqual(self.m.coop_call('identify', pad='event9', core=lambda c, a: None, run=run, helper=helper),
                         {'error': 'That controller is no longer connected.'})
        self.assertIn('error', self.m.coop_call('players', core=lambda c, a: None, helper=os.path.join(self.tmp, 'none')))

    def test_seats_match_hearth_core_and_only_read_without_it(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'core_service.rs').read_text()
        ops = re.search(r'pub const SEATS_OPS: &\[&str\] = &\[(.*?)\];', rs).group(1)
        self.assertEqual(tuple(re.findall(r'"([^"]+)"', ops)), self.m.SEATS_OPS)
        admin = (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-admin').read_text()
        self.assertIn(self.m.SEAT_PAD.pattern, admin, 'the same pad ids as hearth-admin')
        bad = {'error': "That isn't something the Hearth plugin asks"}
        for args in (('setup',), ('remove',), ('start', 1), ('stop', 5), ('start', True), ('assign', 2, 'event7'), ('assign', 0, 'usb:1-2'),
                     ('assign', 2, 'usb:1-2;rm')):
            self.assertEqual(self.m.seats_call(*args, core=lambda c, a: self.fail('asked')), bad, args)
        seen = []
        core = lambda cmd, a: seen.append((cmd, a)) or {'ok': True, 'data': {'ok': True}}
        self.assertEqual(self.m.seats_call('assign', 3, 'uniq:aa:bb:cc:dd:ee:ff', core=core), {'ok': True})
        self.assertEqual(self.m.seats_call('stop', 2, 'usb:1-2', core=core), {'ok': True})
        self.assertEqual(seen, [('seats', {'op': 'assign', 'seat': 3, 'pad': 'uniq:aa:bb:cc:dd:ee:ff'}),
                                ('seats', {'op': 'stop', 'seat': 2, 'pad': ''})])
        # No core: reads go to hearth-seats itself; changes never (polkit wouldn't answer for the plugin).
        helper = os.path.join(self.tmp, 'hearth-seats')
        open(helper, 'w').close()
        calls = []

        class P:
            def __init__(self, out):
                self.stdout, self.stderr = out, ''
        run = lambda argv, **k: calls.append(argv) or P('{"configured": true, "seats": []}\n')
        self.assertTrue(self.m.seats_call('status', core=lambda c, a: None, run=run, helper=helper)['configured'])
        self.assertEqual(calls, [['python3', helper, 'status']])
        self.assertEqual(self.m.seats_call('start', 2, core=lambda c, a: None, run=run, helper=helper), {'error': self.m.NO_CORE_SEATS})
        self.assertEqual(len(calls), 1)
        self.assertIn('error', self.m.seats_call('pads', core=lambda c, a: None, helper=os.path.join(self.tmp, 'none')))

    def test_quick_resume_friends_link_and_sunshine_match_hearth_core(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'core_service.rs').read_text()
        for const, mine in (('ASIDE_OPS', self.m.ASIDE_OPS), ('LINK_OPS', self.m.LINK_OPS), ('REMOTE_OPS', self.m.REMOTE_OPS)):
            ops = re.search(r'pub const %s: &\[&str\] = &\[(.*?)\];' % const, rs).group(1)
            self.assertEqual(tuple(re.findall(r'"([^"]+)"', ops)), mine, const)
        bad = {'error': "That isn't something the Hearth plugin asks"}
        never = lambda c, a: self.fail('asked')
        for cmd, op, a in (('aside', 'put', {}), ('aside', 'put', {'appid': True}), ('aside', 'kill', {'appid': 5}), ('link', 'unpair', {'peer': 'a'}),
                           ('link', 'sync', {'peer': '../x'}), ('remote', 'login', {}), ('remote', 'pair', {'pin': '12345'}), ('seats', 'status', {})):
            self.assertEqual(self.m.rest_call(cmd, op, a, core=never), bad, (cmd, op, a))
        seen = []
        core = lambda c, a: seen.append((c, a)) or {'ok': True, 'data': {'list': []}}
        self.assertEqual(self.m.rest_call('aside', 'put', {'appid': 620, 'game': 'steam:620', 'title': 'Portal\n2'}, core=core), {'list': []})
        self.assertEqual(self.m.rest_call('remote', 'pair', {'pin': '0420', 'name': 7}, core=core), {'list': []})
        self.assertEqual(self.m.rest_call('friends', '', None, core=core), {'list': []})
        self.assertEqual(seen, [('aside', {'op': 'put', 'appid': 620, 'game': 'steam:620', 'title': 'Portal2'}),
                                ('remote', {'op': 'pair', 'pin': '0420', 'name': ''}), ('friends', {})])
        self.assertEqual(self.m.rest_call('link', 'status', core=lambda c, a: None), {'error': self.m.NO_CORE})

    def test_coop_shortcut(self):
        t = self.m.coop_shortcut("gog:it's", 3, 'grid')
        self.assertEqual(t['exe'], '/bin/sh')
        self.assertEqual(shlex.split(t['options']), ['-c', self.m.COOP_WRAPPER, 'hearth-steam-coop', self.m.COOP, 'play', '--game', "gog:it's",
                                                     '--players', '3', '--layout', 'grid'])
        self.assertEqual(shlex.split(self.m.coop_shortcut('steam:400')['options'])[-3:], ['play', '--game', 'steam:400'])
        self.assertIsNone(self.m.coop_shortcut('steam:400', 2, 'sideways'))
        self.assertIsNone(self.m.coop_shortcut('no-colon'))
        self.assertIn('--env=SteamAppId=', self.m.COOP_WRAPPER)
        self.assertIn('-u LD_PRELOAD -u LD_LIBRARY_PATH', self.m.COOP_WRAPPER)

    def test_page_calls_match_hearth_core_and_need_it_running(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'core_service.rs').read_text()
        for const, mine in (('BRIDGE_CALLS', self.m.BRIDGE_CALLS), ('MODS_CALLS', self.m.MODS_CALLS)):
            body = re.search(r'pub const %s: &\[\(&str, &\[&str\]\)\] = &\[(.*?)\];' % const, rs, re.S).group(1)
            self.assertEqual(tuple(re.findall(r'\("([^"]+)"', body)), mine, const)
        seen = []
        ok = lambda c, a: (seen.append((c, a)), {'ok': True, 'data': {'done': True}})[1]
        self.assertEqual(self.m.page_call('call', 'game.install', {'id': 'gog:1'}, core=ok), {'done': True})
        self.assertEqual(seen[-1], ('call', {'method': 'game.install', 'args': {'id': 'gog:1'}}))
        self.assertEqual(self.m.page_call('mods-call', 'toggle', {'id': 'gog:1', 'm': 'a', 'on': True}, core=ok), {'done': True})
        self.assertEqual(seen[-1][1]['call'], 'toggle')
        n = len(seen)
        self.assertIn('error', self.m.page_call('call', 'accounts.signIn', {}, core=ok))
        self.assertIn('error', self.m.page_call('mods-call', 'nexus_sign_in', {}, core=ok))
        self.assertEqual(len(seen), n, 'not on the list: never sent')
        self.assertIn("isn't running", self.m.page_call('call', 'recomps.list', {}, core=lambda c, a: None)['error'])

    def test_answers_from_the_core_and_the_command_line_when_it_is_not_running(self):
        self.assertIsNone(self.m.core_call('ping', path=self.path), 'not running: None')
        self.serve(lambda r: {'id': r['id'], 'ok': True, 'data': {'service': 'hearth-core', 'version': '9', 'asked': r}})
        r = self.m.core_call('ping', path=self.path)
        self.assertTrue(r['ok'])
        self.assertEqual(self.seen[-1]['cmd'], 'ping')
        self.assertEqual(self.m.core_call('exec', path=self.path)['ok'], False, 'not on the list: never sent')
        self.assertEqual(self.seen[-1]['cmd'], 'ping')
        # hearth_data asks the core first, with the question checked.
        d = self.m.hearth_data('store', 'epic', core=lambda c, a, t: self.m.core_call(c, a, t, path=self.path))
        self.assertEqual(d['asked']['what'], 'store')
        self.assertEqual(d['asked']['arg'], 'epic')
        self.assertIn('error', self.m.hearth_data('store', 'evil', core=lambda *a: self.fail('never asked')))
        # The core not running: the command line, as before.
        app = os.path.join(self.tmp, 'nimbus')
        with open(app, 'w') as f:
            f.write('#!/bin/sh\necho \'{"from":"cli"}\'\n')
        os.chmod(app, 0o755)
        self.m.find_program = lambda name: app
        self.assertEqual(self.m.hearth_data('home', core=lambda *a: None), {'from': 'cli'})
        self.assertEqual(self.m.core_result({'ok': False, 'error': 'nope'}), {'error': 'nope'})
        self.assertIn('error', self.m.core_result(None))

    def test_a_socket_that_is_not_this_users_own_is_never_used(self):
        self.serve(lambda r: {'ok': True, 'data': {}})
        self.assertTrue(self.m.core_safe(self.path))
        os.chmod(self.dir, 0o755)
        self.assertFalse(self.m.core_safe(self.path), 'a folder others can write to')
        self.assertIsNone(self.m.core_call('ping', path=self.path))
        os.chmod(self.dir, 0o700)
        self.assertFalse(self.m.core_safe(self.path, uid=os.getuid() + 1), "another user's socket")
        plain = os.path.join(self.dir, 'plain')
        open(plain, 'w').close()
        self.assertFalse(self.m.core_safe(plain), 'not a socket')

    def test_plugin_calls_check_what_they_send(self):
        self.serve(lambda r: {'id': r['id'], 'ok': True, 'data': {'cmd': r['cmd'], 'game': r.get('game'), 'appid': r.get('appid'),
                                                                   'overlay': r.get('overlay'), 'layout': r.get('layout')}})
        self.m.core_socket_path = lambda: self.path
        p = self.m.Plugin()
        f = run(p.hearth_front(3000000000, 'epic:a', 1))
        self.assertEqual((f['game'], f['appid'], f['overlay']), ('epic:a', 3000000000, True))
        f = run(p.hearth_front('x', 'bad id', 0))
        self.assertEqual((f['game'], f['appid']), (None, 0), 'a bad id is no game')
        self.assertEqual(run(p.hearth_input_set('gog:1', 'shooter'))['layout'], 'shooter')
        self.assertIn('error', run(p.hearth_input_set('nope', 'shooter')))
        self.assertIn('error', run(p.hearth_tool('rm')))
        self.assertEqual(run(p.hearth_tool('screenshot'))['cmd'], 'screenshot')
        self.assertTrue(run(p.core_status())['running'])
        self.assertIn('error', run(p.hearth_game('no colon')))

    def test_settings_have_the_new_parts(self):
        for k in ('tkQam', 'tkInGame', 'tkInput'):
            self.assertIn(k, self.m.DEFAULT_SETTINGS)


class Art(unittest.TestCase):
    """Every kind of picture reference Hearth gives (desktop_games.rs art_for_desktop, artcache.rs, library.rs art_url)
    becomes a picture, safely, in every format the cache keeps (0.6.60's Hearth row showed blank cards)."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='decky-hearth-art-')
        self.home = os.path.join(self.tmp, 'home')
        self.cache = os.path.join(self.home, '.local', 'share', 'nimbus', 'art-cache')
        os.makedirs(self.cache)
        self.m = load(self.tmp)
        self.logged = []
        sys.modules['decky'].logger = types.SimpleNamespace(info=self.logged.append)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def put(self, rel, data):
        p = os.path.join(self.home, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        open(p, 'wb').write(data)
        return p

    def test_every_kind_of_reference(self):
        pic = self.put('Pictures/my cover.png', PNG)
        name = self.m.cache_name('https://images.gog-statics.com/abc.jpg')
        self.put('.local/share/nimbus/art-cache/%s.webp' % name, WEBP)
        other = 'a' * 24
        self.put('.local/share/nimbus/art-cache/' + other, JPG)   # no extension
        cases = {
            pic: 'image/png',
            'file://' + pic.replace(' ', '%20'): 'image/png',
            'file://localhost' + pic.replace(' ', '%20'): 'image/png',
            '~/Pictures/my cover.png': 'image/png',
            'nimbus-art://file/' + pic.replace(' ', '%20') + '?v=3': 'image/png',
            'nimbus-art://file//' + pic.lstrip('/').replace(' ', '%20'): 'image/png',
            'nimbus-art://cache/' + name: 'image/webp',
            'nimbus-art://cache/' + other: 'image/jpeg',
            'https://images.gog-statics.com/abc.jpg': 'image/webp',   # the cached copy, no network
        }
        for ref, mime in cases.items():
            self.assertTrue(self.m.picture_url(ref, fetch=self.fail).startswith('data:%s;base64,' % mime), ref)
        self.assertEqual(self.m.picture_url('https://x/new.gif', fetch=lambda u: GIF)[:21], 'data:image/gif;base64')
        self.assertEqual(self.m.picture_url('https://x/new.avif', fetch=lambda u: AVIF)[:22], 'data:image/avif;base64')

    def test_refused_and_logged_without_paths(self):
        outside = os.path.join(self.tmp, 'secret.png')
        open(outside, 'wb').write(PNG)
        for ref in (outside, 'file://' + outside, 'nimbus-art://file/' + outside, '/etc/passwd', 'nimbus-art://cache/../../x',
                    'nimbus-art://cache/' + 'b' * 24, 'http://x/y.png', 'relative/y.png', '', self.put('notes.png', b'hello')):
            self.assertEqual(self.m.picture_url(ref, fetch=lambda u: PNG), '', ref)
        os.symlink(outside, os.path.join(self.cache, 'c' * 24 + '.png'))
        self.assertEqual(self.m.picture_url('nimbus-art://cache/' + 'c' * 24), '', 'a link out of the cache is refused')
        self.assertEqual(self.m.picture_url('https://x/y.png', fetch=lambda u: (_ for _ in ()).throw(OSError('down'))), '')
        text = '\n'.join(self.logged)
        self.assertIn('outside the allowed folders', text)
        self.assertIn('not a picture', text)
        self.assertIn('download failed (OSError)', text)
        self.assertNotIn(self.tmp, text, 'no paths in the log')
        self.assertNotIn('x/y.png', text, 'no addresses in the log')
        self.assertGreaterEqual(self.m.ART_FAILS['page art: outside the allowed folders'], 3)

    def test_steam_library_cache(self):
        self.put('.local/share/Steam/appcache/librarycache/620_library_600x900.jpg', JPG)
        self.put('.var/app/com.valvesoftware.Steam/.local/share/Steam/appcache/librarycache/570/9f8e/library_600x900.jpg', JPG)
        self.put('.steam/steam/appcache/librarycache/440/header.jpg', PNG)
        self.assertTrue(self.m.picture_url('steam-lib:620:portrait').startswith('data:image/jpeg'))
        self.assertTrue(self.m.picture_url('steam-lib:570:portrait').startswith('data:image/jpeg'), "Flathub's Steam, newer layout")
        self.assertTrue(self.m.picture_url('steam-lib:440:wide').startswith('data:image/png'))
        self.assertEqual(self.m.picture_url('steam-lib:440:portrait'), '')
        self.assertEqual(self.m.picture_url('steam-lib:../../etc:portrait'), '')
        self.assertEqual(self.m.picture_url('steam-lib:620:passwd'), '')

    def test_steam_shortcut_art_is_png_or_jpg(self):
        self.assertEqual(self.m.read_art('https://x/a.jpg', fetch=lambda u: JPG)['type'], 'jpg')
        conv = []
        def fake(data):
            conv.append(data[:4])
            return PNG
        a = self.m.read_art('https://x/a.webp', fetch=lambda u: WEBP, converters=[lambda d: None, fake])
        self.assertEqual((a['type'], base64.b64decode(a['data'])), ('png', PNG), 'webp converted to PNG for Steam')
        self.assertEqual(conv, [b'RIFF'])
        self.assertIsNone(self.m.read_art('https://x/a.avif', fetch=lambda u: AVIF, converters=[lambda d: b'not png']))
        self.assertIn('avif and nothing here converts it', '\n'.join(self.logged))
        self.assertEqual(self.m.to_png(GIF, converters=[lambda d: (_ for _ in ()).throw(RuntimeError()), lambda d: PNG]), PNG,
                         'a converter that throws is skipped')

    def test_real_conversion_when_a_converter_is_here(self):
        src = real_png()
        webp = None
        try:
            from PIL import Image  # noqa: F401
            import io
            out = io.BytesIO()
            Image.open(io.BytesIO(src)).save(out, 'WEBP')
            webp = out.getvalue()
        except Exception:  # noqa: BLE001
            for tool, args in (('magick', ['png:-', 'webp:-']), ('convert', ['png:-', 'webp:-']),
                               ('ffmpeg', ['-loglevel', 'error', '-i', 'pipe:0', '-f', 'webp', 'pipe:1'])):
                if shutil.which(tool):
                    import subprocess
                    r = subprocess.run([tool] + args, input=src, capture_output=True)
                    if r.returncode == 0 and r.stdout[8:12] == b'WEBP':
                        webp = r.stdout
                        break
        if not webp:
            self.skipTest('no Pillow, ImageMagick or ffmpeg here to make a webp')
        self.m.find_program = lambda n: shutil.which(n)
        png = self.m.to_png(webp)
        self.assertIsNotNone(png, 'Pillow, magick, convert or ffmpeg turned the webp into PNG')
        self.assertTrue(png.startswith(b'\x89PNG'))
        self.assertEqual(self.m.kind_of(webp), 'webp')
        self.assertEqual(self.m.kind_of(real_png()), 'png')



class Update(unittest.TestCase):
    """Quick Access › Hearth › Update Hearth OS: hearth-steamos-update's answers (a stand-in for it here)."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='decky-hearth-upd-')
        os.makedirs(os.path.join(self.tmp, 'home'))
        self.m = load(self.tmp)
        self.prog = os.path.join(self.tmp, 'hearth-steamos-update')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake(self, body):
        with open(self.prog, 'w') as f:
            f.write('#!/bin/sh\n' + body)
        os.chmod(self.prog, 0o755)

    def test_check(self):
        self.fake('echo "Update available: 0.6.61"; exit 0\n')
        r = self.m.update_check(prog=self.prog)
        self.assertEqual((r['state'], r['version']), ('available', '0.6.61'))
        self.fake('echo "Hearth OS is up to date" >&2; exit 7\n')
        self.assertEqual(self.m.update_check(prog=self.prog)['state'], 'none')
        self.fake('echo "Hearth OS: checking for updates failed: no network" >&2; exit 1\n')
        r = self.m.update_check(prog=self.prog)
        self.assertEqual(r['state'], 'error')
        self.assertIn('no network', r['message'])
        self.m.STEAMOS_UPDATE = os.path.join(self.tmp, 'missing')
        self.assertIn("isn't installed", self.m.update_check()['message'])

    def wait(self):
        import time
        for _ in range(100):
            if self.m.UPDATE['state'] != 'running':
                return
            time.sleep(0.05)

    def test_apply_with_progress(self):
        self.fake('echo 0%; echo 37%; echo "Hearth OS: the update is ready." >&2; echo 100%; exit 0\n')
        self.assertTrue(self.m.update_apply(prog=self.prog))
        self.wait()
        self.assertEqual((self.m.UPDATE['state'], self.m.UPDATE['pct']), ('ready', 100))
        self.assertIn('restart', self.m.UPDATE['message'])
        self.fake('echo 5%; sleep 0.5; echo "Hearth OS: the update failed: another update is already running" >&2; exit 1\n')
        self.assertTrue(self.m.update_apply(prog=self.prog))
        self.assertFalse(self.m.update_apply(prog=self.prog), 'one update at a time')
        self.wait()
        self.assertEqual(self.m.UPDATE['state'], 'error')
        self.assertIn('another update is already running', self.m.UPDATE['message'])
        self.fake('exit 7\n')
        self.m.update_apply(prog=self.prog)
        self.wait()
        self.assertEqual(self.m.UPDATE['state'], 'none')
        self.assertEqual([self.m.progress_of(x) for x in ('42%', ' 7%\n', 'x%', '42', '')], [42, 7, None, None, None])


class Theme(unittest.TestCase):
    CSS = (HERE / 'defaults' / 'hearth-theme.css').read_text()

    def test_uses_hearths_real_colours(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        root = re.search(r':root\{(.*?)\}', ui, re.S).group(1)
        for var in ('--bg', '--panel', '--panel2', '--text', '--dim', '--accent', '--accentInk'):
            value = re.search(var + r':(#[0-9a-fA-F]{6})', root).group(1).lower()
            self.assertIn(value, self.CSS.lower(), '%s %s' % (var, value))

    def test_layer_three_is_one_rule_per_line(self):
        for line in self.CSS.splitlines():
            if '{{' in line:
                self.assertRegex(line, r'^[^{]*\{\{[A-Za-z0-9_]+:[A-Za-z0-9_]+\}\}.*\{[^{}]+\}$', line)
                self.assertEqual(line.count('{{'), line.count('}}'), line)

    def test_stable_layers_need_no_class_names(self):
        stable = [l for l in self.CSS.splitlines() if '{{' not in l and '{' in l and not l.startswith('/*')]
        self.assertTrue(any('.gpfocus' in l for l in stable))
        self.assertTrue(any('--gpColor-Blue' in l for l in stable))
        self.assertEqual(self.CSS.count('{') - 2 * self.CSS.count('{{'), self.CSS.count('}') - 2 * self.CSS.count('}}'))

    def test_css_loader_theme_manifest(self):
        t = json.loads((HERE / 'defaults' / 'css-loader' / 'Hearth' / 'theme.json').read_text())
        self.assertEqual(t['name'], 'Hearth')
        self.assertIn('shared.css', t['inject'])


if __name__ == '__main__':
    unittest.main()
