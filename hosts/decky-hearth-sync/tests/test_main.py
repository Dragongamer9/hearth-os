"""Tests for Hearth Sync's backend (hosts/decky-hearth-sync/main.py). No Steam, no Decky: a stand-in `decky` module
and a fake Hearth app."""
import sys
sys.dont_write_bytecode = True
import asyncio
import base64
import importlib.util
import json
import os
import pathlib
import shlex
import shutil
import subprocess
import tempfile
import types
import unittest

HERE = pathlib.Path(__file__).resolve().parents[1]
LOG = []


def load(tmp):
    fake = types.ModuleType('decky')
    fake.DECKY_USER_HOME = os.path.join(tmp, 'home')
    fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(tmp, 'homebrew', 'settings', 'decky-hearth-sync')
    fake.DECKY_PLUGIN_DIR = os.path.join(tmp, 'plugin')
    fake.logger = types.SimpleNamespace(info=LOG.append)
    sys.modules['decky'] = fake
    spec = importlib.util.spec_from_file_location('hearth_sync_main', HERE / 'main.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def run(coro):
    return asyncio.new_event_loop().run_until_complete(coro)


PNG = b'\x89PNG\r\n\x1a\n' + b'0' * 64
JPG = b'\xff\xd8\xff\xe0' + b'0' * 64
RAW = [
    {'id': 'steam:570', 'title': 'Dota 2', 'source': 'steam', 'cover': '/x.png'},
    {'id': 'epic:fn', 'title': 'Fortnite', 'source': 'epic', 'cover': 'https://cdn.example/fn.jpg', 'wide': None, 'hero': '/h.jpg', 'logo': '/l.png'},
    {'id': 'gog:cp', 'title': 'Cyberpunk 2077', 'source': 'gog', 'cover': 'nimbus-art://cache/0123456789abcdef01234567', 'wide': '/w.jpg'},
    {'id': 'xbox:cloud', 'title': 'Xbox Cloud Gaming', 'source': 'xbox', 'cloud': True},
    {'id': 'emu:snes:smw', 'title': 'Super Mario World', 'source': 'emu'},
    {'id': 'recomps:zelda64', 'title': 'Zelda 64', 'source': 'recomps'},
    {'id': 'beacon:bfme2', 'title': 'BFME II', 'source': 'beacon'},
    {'id': 'manual:x', 'title': '  My Game ', 'source': 'manual'},
    {'id': 'odd:y', 'title': 'Odd', 'source': 'weird'},
    {'id': 'bad', 'title': 'no colon'}, {'id': 'a"b:c', 'title': 'quote'}, 'not a dict',
    {'id': 'epic:fn', 'title': 'duplicate', 'source': 'epic'},
]


class Backend(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-sync-')
        os.makedirs(os.path.join(self.tmp, 'home'))
        self.m = load(self.tmp)
        LOG.clear()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_only_non_steam_games_with_their_store_and_target(self):
        g = self.m.clean_games(RAW, '/usr/bin/nimbus')
        ids = [x['id'] for x in g]
        self.assertNotIn('steam:570', ids)
        self.assertNotIn('bad', ids)
        self.assertEqual(len(ids), len(set(ids)), 'no duplicates')
        by = {x['id']: x for x in g}
        self.assertEqual(by['epic:fn']['store'], 'epic')
        self.assertEqual(by['epic:fn']['storeName'], 'Epic')
        self.assertEqual(by['gog:cp']['storeName'], 'GOG')
        self.assertEqual(by['emu:snes:smw']['storeName'], 'Emulators')
        self.assertEqual(by['recomps:zelda64']['storeName'], 'Recomps')
        self.assertEqual(by['beacon:bfme2']['storeName'], 'Beacon')
        self.assertEqual(by['manual:x']['storeName'], 'Added by hand')
        self.assertEqual(by['manual:x']['title'], 'My Game')
        self.assertEqual(by['odd:y']['store'], 'other')
        # Pictures: the wide one stands in for a missing hero and the other way round.
        self.assertEqual(by['epic:fn']['wide'], '/h.jpg')
        self.assertEqual(by['epic:fn']['hero'], '/h.jpg')
        self.assertEqual(by['epic:fn']['logo'], '/l.png')
        self.assertEqual(by['gog:cp']['hero'], '/w.jpg')
        # Launch: Hearth's launcher through the wrapper that cleans Steam's environment.
        t = by['epic:fn']
        self.assertEqual(t['exe'], '/bin/sh')
        args = shlex.split(t['options'])
        self.assertEqual(args[:2], ['-c', self.m.WRAPPER])
        self.assertEqual(args[2:], ['hearth-steam', '/usr/bin/nimbus', '--play-now', 'epic:fn'])
        self.assertIn('-u LD_PRELOAD', self.m.WRAPPER)
        self.assertEqual(shlex.split(by['xbox:cloud']['options'])[3:], ['/usr/bin/nimbus', '--windowed', '--play', 'xbox:cloud'])
        self.assertEqual(ids, sorted(ids, key=lambda i: (by[i]['title'].lower(), i)), 'A to Z')

    def test_signature_follows_changes(self):
        a = self.m.clean_games(RAW)
        s = self.m.signature(a)
        self.assertEqual(s, self.m.signature(self.m.clean_games(RAW)))
        raw2 = json.loads(json.dumps([r for r in RAW if isinstance(r, dict)]))
        raw2[2]['cover'] = '/new.png'
        self.assertNotEqual(s, self.m.signature(self.m.clean_games(raw2)))

    def test_list_runs_hearth_once_then_only_when_its_files_change(self):
        calls = []

        def fake_run(cmd, **kw):
            calls.append(cmd)
            return subprocess.CompletedProcess(cmd, 0, 'log line\n' + json.dumps(RAW) + '\n', '')
        finder = lambda n: '/usr/bin/nimbus'
        r = self.m.list_games(run=fake_run, now=1000, finder=finder)
        self.assertTrue(r['ok'])
        self.assertEqual(calls[0][1:], ['--list-games', '--with-cloud'])
        self.m.list_games(run=fake_run, now=1100, finder=finder)
        self.assertEqual(len(calls), 1, 'nothing changed: no new scan')
        self.m.list_games(run=fake_run, now=1100, force=True, finder=finder)
        self.assertEqual(len(calls), 2, 'Sync now always reads again')
        self.m.list_games(run=fake_run, now=1100 + self.m.RESCAN_EVERY, finder=finder)
        self.assertEqual(len(calls), 3, 'a full read every so often')
        lib = os.path.join(self.tmp, 'home', '.local', 'share', 'nimbus')
        os.makedirs(lib)
        open(os.path.join(lib, 'library.json'), 'w').write('{}')
        self.m.list_games(run=fake_run, now=1100 + self.m.RESCAN_EVERY + 1, finder=finder)
        self.assertEqual(len(calls), 4, "Hearth's library file changed")

    def test_list_errors(self):
        self.assertFalse(self.m.list_games(finder=lambda n: None, force=True)['ok'])
        bad = lambda cmd, **kw: subprocess.CompletedProcess(cmd, 1, 'garbage', 'boom /home/secret/path')
        r = self.m.list_games(run=bad, force=True, finder=lambda n: '/usr/bin/nimbus')
        self.assertFalse(r['ok'])
        self.assertNotIn('/home/secret', r['error'], 'no paths in what the panel shows')

    def test_map_and_applied_are_cleaned(self):
        m = run(self.m.Plugin().set_shortcuts({'epic:a': 123, 'bad': 5, 'gog:b': -1, 'gog:c': True, 'gog:d': 'x', 'gog:e': 2 ** 40}))
        self.assertEqual(m, {'epic:a': 123})
        a = run(self.m.Plugin().set_applied({'epic:a': {'t': 'x', 'art': {'cover': '/c.png', 'evil': 'y'}}, 'bad': {}}))
        self.assertEqual(a, {'epic:a': {'t': 'x', 'art': {'cover': '/c.png'}}})
        self.assertEqual(run(self.m.Plugin().get_applied()), a)

    def test_store_switches_default_on(self):
        p = self.m.Plugin()
        self.assertEqual(run(p.get_settings()), {'stores': {}})
        s = run(p.set_settings({'stores': {'epic': False, 'gog': 'yes', '': True}, 'other': 1}))
        self.assertEqual(s, {'stores': {'epic': False}})

    def test_takes_over_the_old_hearth_plugins_game_shortcuts_once(self):
        old = os.path.join(self.tmp, 'homebrew', 'settings', 'decky-hearth')
        os.makedirs(old)
        json.dump({'epic:fn': 111, 'hearth-app:mods': 222, 'coop:steam:600': 333, 'gog:cp': 444}, open(os.path.join(old, 'shortcuts.json'), 'w'))
        p = self.m.Plugin()
        self.assertEqual(run(p.get_shortcuts()), {'epic:fn': 111, 'gog:cp': 444}, "its links and split-screen shortcuts stay the old plugin's")
        run(p.set_shortcuts({}))
        self.assertEqual(run(p.get_shortcuts()), {}, 'only once: removed ones never come back')
        self.assertTrue(run(p.get_status())['adopted'])
        run(p.set_status({'synced': 3, 'adopted': False}))
        self.assertTrue(run(p.get_status())['adopted'], 'the panel can\'t undo it')

    def test_status_is_cleaned(self):
        s = run(self.m.Plugin().set_status({'synced': 4, 'lastSync': 5, 'problems': ['a', 3, 'b' * 500], 'off': 'x', 'artMissing': -2}))
        self.assertEqual(s['synced'], 4)
        self.assertEqual(s['problems'], ['a', 'b' * 200])
        self.assertEqual(s['artMissing'], 0)

    def test_art_from_every_kind_of_reference(self):
        h = os.path.join(self.tmp, 'home')
        cache = os.path.join(h, '.local', 'share', 'nimbus', 'art-cache')
        os.makedirs(cache)
        open(os.path.join(cache, '0123456789abcdef01234567.png'), 'wb').write(PNG)
        url = 'https://cdn.example/fn.jpg'
        open(os.path.join(cache, self.m.cache_name(url) + '.jpg'), 'wb').write(JPG)
        open(os.path.join(h, 'cover.jpg'), 'wb').write(JPG)
        self.assertEqual(self.m.read_art('nimbus-art://cache/0123456789abcdef01234567')['type'], 'png')
        self.assertEqual(base64.b64decode(self.m.read_art(url, fetch=lambda u: self.fail('cached'))['data']), JPG)
        self.assertEqual(self.m.read_art(os.path.join(h, 'cover.jpg'))['type'], 'jpg')
        self.assertEqual(self.m.read_art('https://cdn.example/new.png', fetch=lambda u: PNG)['type'], 'png')
        self.assertIsNone(self.m.read_art('/etc/passwd'))
        self.assertIsNone(self.m.read_art('http://plain.example/x.png'))
        webp = b'RIFF\0\0\0\0WEBP' + b'0' * 32
        self.assertEqual(self.m.read_art('https://x.example/a.webp', fetch=lambda u: webp, converters=[lambda d: PNG])['type'], 'png')
        self.assertIsNone(self.m.read_art('https://x.example/a.webp', fetch=lambda u: webp, converters=[]))

    def test_art_failures_logged_by_reason_never_paths(self):
        self.m.read_art('/home/someone/secret.png')
        self.m.read_art('https://secret.example/x.png', fetch=lambda u: (_ for _ in ()).throw(OSError('no')))
        run(self.m.Plugin().art_error('cover refused'))
        text = '\n'.join(LOG)
        self.assertIn('art: not shown', text)
        self.assertIn('steam: cover refused', text)
        self.assertNotIn('secret', text)

    def w(self, rel, text):
        p = os.path.join(self.tmp, 'home', rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text if isinstance(text, str) else json.dumps(text))
        return p

    def test_minigalaxy_linux_and_windows_games(self):
        h = os.path.join(self.tmp, 'home')
        self.w('.config/minigalaxy/config.json', {'install_dir': os.path.join(h, 'Games/GOG')})
        self.w('Games/GOG/Celeste/gameinfo', 'Celeste\n1.4\n\nen\n1428288153\n')
        self.w('Games/GOG/Celeste/start.sh', '#!/bin/sh\n')
        self.w('Games/GOG/Witcher/goggame-1207664663.info', {'gameId': '1207664663', 'name': 'The Witcher', 'playTasks': [
            {'isPrimary': False, 'path': 'other.exe'}, {'isPrimary': True, 'path': 'System\\witcher.exe', 'workingDir': 'System', 'arguments': '-fast'}]})
        self.w('Games/GOG/Witcher/unins000.exe', '')
        self.w('Games/GOG/Dosgame/gameinfo', 'Old\n1\n\nen\n55\n')
        self.w('.cache/minigalaxy/covers/1428288153.jpg', 'x')
        g = {x['id']: x for x in self.m.minigalaxy_games(which=lambda n: '/usr/bin/wine')}
        self.assertEqual(sorted(g), ['gog:1207664663', 'gog:1428288153'])
        self.assertEqual(g['gog:1428288153']['launch'], [os.path.join(h, 'Games/GOG/Celeste/start.sh')])
        self.assertTrue(g['gog:1428288153']['cover'].endswith('covers/1428288153.jpg'))
        wl = g['gog:1207664663']['launch']
        self.assertEqual(wl[:3], ['env', 'WINEPREFIX=' + os.path.join(h, 'Games/GOG/Witcher/prefix'), 'wine'])
        self.assertEqual(wl[3:], ['start', '/b', '/wait', '/d', 'c:\\game\\System', 'c:\\game\\System\\witcher.exe', '-fast'])
        self.assertIn('Minigalaxy: no start.sh (DOSBox/ScummVM games start in Minigalaxy)', self.m.SKIPPED)
        t = self.m.direct(wl, '/g')
        self.assertEqual(shlex.split(t['options'])[3:], wl)
        self.assertNotIn('LD_PRELOAD', self.m.DIRECT, "Steam's overlay stays for these")

    def test_heroic_games_launch_through_heroic(self):
        self.w('.config/heroic/legendaryConfig/legendary/installed.json', {'Fortnite': {'app_name': 'Fortnite', 'title': 'Fortnite', 'install_path': '/nope'}})
        self.w('.config/heroic/store_cache/legendary_library.json', {'library': [{'app_name': 'Fortnite', 'art_square': 'https://x/c.jpg', 'art_cover': 'https://x/w.jpg'}]})
        self.w('.config/heroic/gog_store/installed.json', {'installed': [{'appName': '1207664663', 'install_path': '/nope2'}]})
        g = {x['id']: x for x in self.m.heroic_games()}
        self.assertEqual(sorted(g), ['epic:Fortnite', 'gog:1207664663'])
        self.assertEqual(g['epic:Fortnite']['launch'], ['xdg-open', 'heroic://launch?appName=Fortnite&runner=legendary'])
        self.assertEqual(g['epic:Fortnite']['cover'], 'https://x/c.jpg')
        self.assertEqual(g['gog:1207664663']['launch'][1], 'heroic://launch?appName=1207664663&runner=gog')

    def test_epic_games_launcher_only_with_a_link_handler(self):
        inst = os.path.join(self.tmp, 'home', 'Games', 'Epic', 'Hades')
        os.makedirs(inst)
        self.w('.local/share/com.epicgames.eglv2/Manifests/A1.item', {'AppName': 'Min', 'DisplayName': 'Hades', 'InstallLocation': inst})
        self.w('.local/share/com.epicgames.eglv2/Manifests/A2.item', {'AppName': 'Half', 'DisplayName': 'Half', 'InstallLocation': inst, 'bIsIncompleteInstall': True})
        self.w('.local/share/com.epicgames.eglv2/compat/runtime/x.json', {'AppName': 'NotAGame', 'InstallLocation': inst})
        self.assertEqual(self.m.egl_games(handler=lambda: False), [])
        g = self.m.egl_games(handler=lambda: True)
        self.assertEqual([x['id'] for x in g], ['epic:Min'])
        self.assertEqual(g[0]['launch'], ['xdg-open', 'com.epicgames.launcher://apps/Min?action=launch&silent=true'])

    def test_same_game_twice_is_one_shortcut_newest_install_wins(self):
        hearth = self.m.clean_games([{'id': 'gog:1', 'title': 'Game', 'source': 'gog', 'cover': '/hearth-cover.png'}])
        old = self.m._game('gog:1', 'Game', 'gog', ['/g/start.sh'], '/g', 100, 'minigalaxy', {'wide': '/w.jpg'})
        new = self.m._game('gog:1', 'Game', 'gog', ['xdg-open', 'heroic://launch?appName=1&runner=gog'], '/h', 200, 'heroic')
        out = self.m.merge(hearth, [old, new])
        self.assertEqual(len(out), 1)
        self.assertEqual(out[0]['via'], 'heroic')
        self.assertIn('heroic://', out[0]['options'])
        self.assertEqual(out[0]['cover'], '/hearth-cover.png', 'pictures the winner lacks come from the others')
        self.assertEqual(out[0]['wide'], '/w.jpg')
        self.assertEqual(self.m.merge(hearth, [])[0]['via'], 'hearth')

    def test_plugin_json_and_version(self):
        meta = json.load(open(HERE / 'plugin.json'))
        self.assertEqual(meta['name'], 'Hearth Sync')
        mine = json.load(open(HERE / 'package.json'))['version']
        other = json.load(open(HERE.parents[1] / 'archive' / 'hosts' / 'decky-hearth' / 'package.json'))['version']
        self.assertEqual(mine, other, 'the same version as Hearth\'s other packages')

    def test_frontend_takes_nothing_over(self):
        src = ''.join(open(p).read() for p in (HERE / 'src').glob('*.ts*'))
        for word in ('routerHook', 'afterPatch', 'addRoute', 'addPatch', 'findModule', 'insertAdjacent', 'createElement("style'):
            self.assertNotIn(word, src, 'Hearth Sync never changes Steam\'s screens')
        self.assertIn('RemoveShortcut', src)
        self.assertNotIn('Login', src)


if __name__ == '__main__':
    unittest.main()
