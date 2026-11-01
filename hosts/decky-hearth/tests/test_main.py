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
        self.assertEqual(t['exe'], '/usr/bin/nimbus')
        self.assertEqual(shlex.split(t['options']), ['--play-now', "emu:snes:Mario's World.sfc"])
        f = self.m.shortcut('/usr/bin/nimbus', ['--play-now', 'gog:1'], True)
        self.assertEqual(f['exe'], '/usr/bin/flatpak-spawn')
        self.assertEqual(shlex.split(f['options']), ['--host', '/usr/bin/nimbus', '--play-now', 'gog:1'])

    def test_targets_include_hearths_apps_only_when_installed(self):
        games = self.m.clean_games([{'id': 'gog:1', 'title': 'A', 'source': 'gog'}])
        finder = lambda p: {'nimbus': '/usr/bin/nimbus', 'hearth-mods': '/usr/bin/hearth-mods'}.get(p)
        g, links = self.m.targets(games, False, finder=finder)
        self.assertEqual(shlex.split(g[0]['options']), ['--play-now', 'gog:1'])
        ids = [l['id'] for l in links]
        self.assertIn('hearth-app:mods', ids)
        self.assertIn('hearth-app:store', ids)
        self.assertNotIn('hearth-app:theater', ids)
        store = [l for l in links if l['id'] == 'hearth-app:store'][0]
        self.assertEqual(shlex.split(store['options']), ['--windowed', '--page', 'shop'])

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
        self.assertIsNone(self.m.read_art('https://x/y.webp', fetch=lambda u: b'RIFF0000WEBP'))
        link = os.path.join(home, 'sneaky.png')
        os.symlink(outside, link)
        self.assertIsNone(self.m.read_art(link), 'a link out of home is followed and refused')

    def test_settings_and_shortcuts_are_kept(self):
        p = self.m.Plugin()
        self.assertEqual(run(p.get_settings()), {'theme': True, 'homeRow': True})
        run(p.set_settings({'theme': False, 'evil': 'x'}))
        self.assertEqual(run(p.get_settings()), {'theme': False, 'homeRow': True})
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
