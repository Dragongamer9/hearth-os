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

    def test_hearth_mods_add_to_steam_entries(self):
        exe = '/usr/bin/hearth-mods'
        good = {'id': 'hearth-mods:modded:steam:489830', 'title': 'Skyrim Special Edition (Modded)', 'game': 'steam:489830', 'kind': 'modded',
                'launch': [exe, '--play-modded', 'steam:489830'], 'cwd': '/home/you', 'cover': '/art/c.jpg', 'hero': '/art/h.jpg', 'added': 1700000000000}
        mo2 = dict(good, id='hearth-mods:mo2:steam:489830', kind='mo2', title='Skyrim Special Edition — Mod Organizer 2', launch=[exe, '--run-mo2', 'skyrimse'])
        self.w('.local/share/nimbus/mods/steam-shortcuts.json', [good, mo2,
               dict(good, id='hearth-mods:modded:x:1', launch=['/bin/sh', '-c', 'evil']), dict(good, id='other:1'), dict(good, kind='nope', id='hearth-mods:nope:x:2'), 'junk'])
        g = self.m.mods_entries(which=lambda p: p == exe)
        self.assertEqual([x['id'] for x in g], [good['id'], mo2['id']], 'only entries that run hearth-mods')
        self.assertEqual((g[0]['source'], g[0]['launch'], g[0]['cover'], g[0]['via']), ('mods', good['launch'], '/art/c.jpg', 'hearth-mods'))
        out = self.m.merge([], g)
        self.assertEqual(out[0]['storeName'], 'Modded games')
        self.assertEqual(shlex.split(out[0]['options'])[3:], [exe, '--play-modded', 'steam:489830'])
        self.assertEqual(out[0]['wide'], '/art/h.jpg')
        self.assertIn(self.m.mods_file(), [os.path.join(self.tmp, 'home', '.local/share/nimbus/mods/steam-shortcuts.json')])

    def test_windows_apps_add_to_steam_entries(self):
        exe = '/usr/bin/hearth-winapps'
        good = {'id': 'hearth-winapps:notepad-plus-plus', 'title': 'Notepad++', 'launch': [exe, 'run', 'notepad-plus-plus'],
                'cwd': '/home/you/Games/Windows apps/Notepad++', 'icon': '/home/you/.local/share/icons/hearth-winapps/notepad-plus-plus.png', 'added': 1700000000000}
        self.w('.local/share/nimbus/winapps/steam-shortcuts.json', {'apps': [good,
               dict(good, id='hearth-winapps:x', launch=['/bin/sh', '-c', 'evil']), dict(good, id='other:1'),
               dict(good, id='hearth-winapps:y', launch=[exe, 'open', '/tmp/x.exe']), 'junk']})
        g = self.m.winapps_entries(which=lambda p: p == exe)
        self.assertEqual([x['id'] for x in g], [good['id']], 'only entries that run hearth-winapps run <id>')
        self.assertEqual((g[0]['source'], g[0]['launch'], g[0]['icon'], g[0]['via']), ('winapps', good['launch'], good['icon'], 'hearth-winapps'))
        out = self.m.merge([], g)
        self.assertEqual(out[0]['storeName'], 'Windows apps')
        self.assertEqual(shlex.split(out[0]['options'])[3:], [exe, 'run', 'notepad-plus-plus'])

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

    # ---- What each shortcut really runs (the GOG-games-don't-start-in-Steam-Game-Mode report, Hearth 1.3/1.4).
    def steam_start(self, item, env_extra=None):
        """Starts a shortcut the way Steam does: its exe and launch options through a shell, in its start folder."""
        env = {'HOME': os.path.join(self.tmp, 'home'), 'PATH': '/usr/bin:/bin', 'LD_LIBRARY_PATH': '/steam/runtime/lib',
               'LD_PRELOAD': '/steam/ubuntu12_64/gameoverlayrenderer.so', 'SteamGameId': '10205416876054413312'}
        env.update(env_extra or {})
        return subprocess.run(['/bin/sh', '-c', '"%s" %s' % (item['exe'], item['options'])], cwd=item['startDir'], env=env,
                              capture_output=True, text=True, timeout=20)

    def recorder(self, name):
        """A stand-in program (in a folder with a space and an apostrophe) that writes down how it was started."""
        d = os.path.join(self.tmp, "Hero's Hour dir", 'bin')
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\nout="$HOME/started.json"\npython3 -c \'import json,os,sys; json.dump({"argv": sys.argv[1:], "cwd": os.getcwd(), '
                    '"ld": os.environ.get("LD_LIBRARY_PATH"), "pre": os.environ.get("LD_PRELOAD"), "aside": os.environ.get("HEARTH_STEAM_LD_PRELOAD")}, '
                    'open(sys.argv[0] if False else os.environ["HOME"] + "/started.json", "w"))\' "$@"\n')
        os.chmod(p, 0o755)
        return p

    def started(self):
        with open(os.path.join(self.tmp, 'home', 'started.json')) as f:
            return json.load(f)

    def test_gog_windows_game_shortcut_runs_hearth_natively(self):
        nimbus = self.recorder('nimbus')
        g = {x['id']: x for x in self.m.clean_games([{'id': 'gog:1724969043', 'title': 'Tomb Raider', 'source': 'gog'}], nimbus)}
        t = g['gog:1724969043']
        # The shortcut is /bin/sh (no Windows program for Steam to give Proton to), started in your home folder.
        self.assertEqual(t['exe'], '/bin/sh')
        self.assertEqual(t['startDir'], os.path.join(self.tmp, 'home'))
        r = self.steam_start(t)
        self.assertEqual(r.returncode, 0, r.stderr)
        s = self.started()
        self.assertEqual(s['argv'], ['--play-now', 'gog:1724969043'], 'Hearth gets the game id intact through a path with spaces and quotes')
        self.assertIsNone(s['ld'], "Steam's runtime libraries are taken out")
        self.assertIsNone(s['pre'])
        self.assertEqual(s['aside'], '/steam/ubuntu12_64/gameoverlayrenderer.so', "Steam's overlay is set aside for the game")

    def test_minigalaxy_linux_game_with_spaces_and_wine_game(self):
        h = os.path.join(self.tmp, 'home')
        self.w('.config/minigalaxy/config.json', {'install_dir': os.path.join(h, 'GOG Games')})
        self.w("GOG Games/Hero's Hour/gameinfo", "Hero's Hour\n1.0\n\nen\n1547269859\n")
        start = self.w("GOG Games/Hero's Hour/start.sh", '')
        os.remove(start)
        os.symlink(self.recorder('start.sh'), start)
        self.w('GOG Games/Fallout 3/goggame-1454315831.info', {'gameId': '1454315831', 'name': 'Fallout 3', 'playTasks': [
            {'isPrimary': True, 'path': 'FalloutLauncher.exe', 'workingDir': ''}]})
        self.w('GOG Games/Fallout 3/unins000.exe', '')
        wine = self.recorder('wine')
        games = self.m.merge([], self.m.minigalaxy_games(which=lambda n: wine if n == 'wine' else None))
        by = {x['id']: x for x in games}
        lin, win = by['gog:1547269859'], by['gog:1454315831']
        self.assertEqual(lin['startDir'], os.path.join(h, "GOG Games/Hero's Hour"))
        r = self.steam_start(lin)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.started()['cwd'], os.path.join(h, "GOG Games/Hero's Hour"), 'started in its own folder, spaces and all')
        self.assertEqual(self.started()['pre'], '/steam/ubuntu12_64/gameoverlayrenderer.so', "Steam's overlay stays for other launchers' games")
        self.assertIsNone(self.started()['ld'])
        # A Windows game Minigalaxy installed: its own prefix and wine, as Minigalaxy starts it (wine here is the stand-in).
        argv = shlex.split(win['options'])[3:]
        self.assertEqual(argv[:3], ['env', 'WINEPREFIX=' + os.path.join(h, 'GOG Games/Fallout 3/prefix'), 'wine'])
        r = self.steam_start(win, {'PATH': os.path.dirname(wine) + ':/usr/bin:/bin'})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.started()['argv'], ['start', '/b', '/wait', '/d', 'c:\\game\\.', 'c:\\game\\FalloutLauncher.exe'])

    def test_epic_game_through_heroic(self):
        self.w('.config/heroic/legendaryConfig/legendary/installed.json', {'Fortnite': {'app_name': 'Fortnite', 'title': 'Fortnite', 'install_path': '/nope'}})
        e = self.m.merge([], self.m.heroic_games())[0]
        self.assertEqual(e['exe'], '/bin/sh')
        self.assertEqual(shlex.split(e['options'])[3:], ['xdg-open', 'heroic://launch?appName=Fortnite&runner=legendary'])

    def test_shortcut_started_inside_steams_container_says_why(self):
        """Regression: Steam set to run a Hearth shortcut with a compatibility tool runs it inside its container, where
        Hearth's program isn't. That start used to end with nothing in Hearth's log; now it says why."""
        os.makedirs(os.path.join(self.tmp, 'home', '.local', 'share', 'nimbus'))
        t = self.m.clean_games([{'id': 'gog:1097893768', 'title': 'Neverwinter Nights', 'source': 'gog'}], '/nonexistent/usr/bin/nimbus')[0]
        r = self.steam_start(t)
        self.assertEqual(r.returncode, 127)
        with open(os.path.join(self.tmp, 'home', '.local', 'share', 'nimbus', 'bridge.log')) as f:
            line = f.read()
        self.assertIn('[hearth-steam] play-now: gog:1097893768: not started', line)
        self.assertIn('Compatibility', line)

    def test_compatibility_tool_on_a_hearth_shortcut_is_found_and_left_alone(self):
        vdf = """"InstallConfigStore"
{
\t"Software"
\t{
\t\t"Valve"
\t\t{
\t\t\t"Steam"
\t\t\t{
\t\t\t\t"CompatToolMapping"
\t\t\t\t{
\t\t\t\t\t"2376133780"
\t\t\t\t\t{
\t\t\t\t\t\t"name"\t\t"proton_experimental"
\t\t\t\t\t\t"config"\t\t""
\t\t\t\t\t\t"priority"\t\t"250"
\t\t\t\t\t}
\t\t\t\t\t"3000000000"
\t\t\t\t\t{
\t\t\t\t\t\t"name"\t\t"proton_9"
\t\t\t\t\t}
\t\t\t\t\t"4241974495"
\t\t\t\t\t{
\t\t\t\t\t\t"name"\t\t""
\t\t\t\t\t}
\t\t\t\t}
\t\t\t}
\t\t}
\t}
}
"""
        p = self.w('.steam/debian-installation/config/config.vdf', vdf)
        before = open(p).read()
        mine = {'gog:1097893768': 2376133780, 'gog:1724969043': 4241974495, 'epic:fn': 123}
        self.assertEqual(self.m.forced_compat(mine), [{'id': 'gog:1097893768', 'tool': 'proton_experimental'}],
                         "only Hearth Sync's own shortcuts, only with a tool named; 3000000000 is someone else's")
        self.assertEqual(open(p).read(), before, "Steam's settings are never changed")
        self.m.save_json('shortcuts.json', mine)
        self.m.save_json('status.json', {'adopted': True})
        r = run(self.m.Plugin().compat_check())
        self.assertEqual([x['id'] for x in r], ['gog:1097893768'])
        self.assertTrue(any('compatibility tool' in str(l) for l in LOG))
        self.assertEqual(self.m.forced_compat(mine, home_dir=os.path.join(self.tmp, 'nobody')), [])


if __name__ == '__main__':
    unittest.main()
