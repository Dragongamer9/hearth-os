"""Tests for Discord replacing Hearth Friends (Matrix) in 1.6 (docs/DISCORD.md): desktop/shell/hearth-discord (the
chooser's saved pick, installing only what he picked, each client's launch and placement by the bar on every edge, the
window size kept as state, a friend's invite), Mantel's Discord quick window (toggle, hide without quitting), and the
Matrix service gone from the session and the image without anything failing when it's missing."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import tempfile
import time
import unittest
from unittest import mock

REPO = pathlib.Path(__file__).resolve().parents[3]
SHELL = REPO / 'hosts' / 'linux' / 'desktop' / 'shell'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_discord', str(SHELL / 'hearth-discord'))
    spec = importlib.util.spec_from_loader('hearth_discord', loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


hd = load()


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-discord-')
        self.saved = {k: getattr(hd, k) for k in ('SETTINGS', 'SIZE_STATE', 'INVITE', 'WEB_PROFILE', 'DORION_ROOT', 'DORION_DESKTOP', 'DATA', 'CONFIG')}
        hd.SETTINGS = os.path.join(self.tmp, 'config', 'nimbus', 'discord.json')
        hd.SIZE_STATE = os.path.join(self.tmp, 'state', 'nimbus', 'discord-window.json')
        hd.INVITE = os.path.join(self.tmp, 'run', 'hearth', 'discord-invite.json')
        hd.WEB_PROFILE = os.path.join(self.tmp, 'web')
        hd.DORION_ROOT = os.path.join(self.tmp, 'opt', 'dorion')
        hd.DORION_DESKTOP = os.path.join(self.tmp, 'apps', 'hearth-dorion.desktop')
        hd.DATA = os.path.join(self.tmp, 'data')
        hd.CONFIG = os.path.join(self.tmp, 'config')
        self.env = mock.patch.dict(os.environ, {'XDG_STATE_HOME': os.path.join(self.tmp, 'state')})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        for k, v in self.saved.items():
            setattr(hd, k, v)
        shutil.rmtree(self.tmp, ignore_errors=True)


class Chooser(Base):
    def test_nothing_is_saved_until_he_picks(self):
        with mock.patch.object(hd, 'installed', return_value={'official': False, 'web': True, 'dorion': False}), \
             mock.patch.object(hd, 'running', return_value=False):
            st = hd.status()
        self.assertFalse(st['chosen'])
        self.assertIsNone(st['client'])
        self.assertEqual(st['opens'], 'web')
        self.assertFalse(os.path.exists(hd.SETTINGS), 'reading the status never writes a setting')

    def test_his_pick_is_merged_and_other_keys_stay(self):
        os.makedirs(os.path.dirname(hd.SETTINGS))
        with open(hd.SETTINGS, 'w') as f:
            json.dump({'enabled': False, 'appId': '123456789012345678'}, f)
        with mock.patch.object(hd, 'installed', return_value={'official': True, 'web': True, 'dorion': False}), \
             mock.patch.object(hd, 'running', return_value=False):
            st = hd.choose('dorion')
        saved = json.load(open(hd.SETTINGS))
        self.assertEqual(saved, {'enabled': False, 'appId': '123456789012345678', 'client': 'dorion'})
        self.assertEqual(st['client'], 'dorion')
        self.assertEqual(st['opens'], 'official', 'Dorion picked but not installed yet: the app opens meanwhile')
        with self.assertRaises(hd.Fail):
            hd.choose('vesktop')

    def test_pick_client_order(self):
        self.assertEqual(hd.pick_client('dorion', {'official': True, 'web': True, 'dorion': True}), 'dorion')
        self.assertEqual(hd.pick_client('official', {'official': False, 'web': True, 'dorion': True}), 'web')
        self.assertEqual(hd.pick_client(None, {'official': False, 'web': False, 'dorion': True}), None)

    def test_switches_and_app_id(self):
        apps = os.path.join(hd.DATA, 'applications')
        mimeapps = os.path.join(hd.CONFIG, 'mimeapps.list')
        os.makedirs(hd.CONFIG, exist_ok=True)
        with open(mimeapps, 'w') as f:
            f.write('[Default Applications]\ntext/html=hearth-browser.desktop\n')
        with mock.patch.object(hd, 'installed', return_value={'official': True, 'web': True, 'dorion': False}), \
             mock.patch.object(hd, 'running', return_value=False):
            hd.set_key('enabled', 'off')
            hd.set_key('appId', '123456789012345678')
            with self.assertRaises(hd.Fail):
                hd.set_key('appId', '12ab')
            with self.assertRaises(hd.Fail):
                hd.set_key('client', 'web')
        saved = json.load(open(hd.SETTINGS))
        self.assertEqual(saved, {'enabled': False, 'appId': '123456789012345678'})
        entry = open(os.path.join(apps, 'hearth-discord-123456789012345678.desktop')).read()
        self.assertIn('MimeType=x-scheme-handler/discord-123456789012345678;', entry)
        self.assertIn('join --wait %u', entry)
        m = open(mimeapps).read()
        self.assertIn('text/html=hearth-browser.desktop', m, 'the other default apps stay')
        self.assertIn('x-scheme-handler/discord-123456789012345678=hearth-discord-123456789012345678.desktop', m)
        hd.register_join_handler('123456789012345678', apps, mimeapps)
        self.assertEqual(open(mimeapps).read().count('x-scheme-handler/discord-'), 1)


class Install(Base):
    def test_web_needs_nothing_and_official_uses_flathub(self):
        with mock.patch.object(hd, 'firefox', return_value='/usr/bin/firefox'):
            self.assertEqual(hd.install('web'), {'installed': True})
        calls = []
        with mock.patch.object(hd, 'which', return_value='/usr/bin/flatpak'), \
             mock.patch.object(hd, 'run', side_effect=lambda c, *a, **k: calls.append(c) or (0, '', '')), \
             mock.patch.object(hd, 'progress'):
            hd.install('official')
        self.assertEqual(calls, [['flatpak', 'install', '--user', '-y', '--noninteractive', 'flathub', 'com.discordapp.Discord']])

    def release(self, data, digest=True):
        sha = hashlib.sha256(data).hexdigest()
        a = {'name': 'Dorion_6.13.1_amd64.deb', 'browser_download_url': 'https://github.com/SpikeHD/Dorion/releases/download/v6.13.1/Dorion_6.13.1_amd64.deb'}
        if digest:
            a['digest'] = 'sha256:' + sha
        return {'tag_name': 'v6.13.1', 'assets': [{'name': 'Dorion_6.13.1_arm64.deb', 'digest': 'sha256:' + '0' * 64,
                                                    'browser_download_url': 'https://github.com/SpikeHD/Dorion/releases/download/v6.13.1/Dorion_6.13.1_arm64.deb'}, a]}

    def test_dorion_asset_needs_a_published_checksum(self):
        name, url, sha = hd.pick_dorion_asset(self.release(b'x'), 'amd64')
        self.assertEqual(name, 'Dorion_6.13.1_amd64.deb')
        self.assertEqual(sha, hashlib.sha256(b'x').hexdigest())
        with self.assertRaises(hd.Fail):
            hd.pick_dorion_asset(self.release(b'x', digest=False), 'amd64')
        bad = self.release(b'x')
        bad['assets'][1]['browser_download_url'] = 'https://evil.example/Dorion_6.13.1_amd64.deb'
        with self.assertRaises(hd.Fail):
            hd.pick_dorion_asset(bad, 'amd64')
        with self.assertRaises(hd.Fail):
            hd.pick_dorion_asset(self.release(b'x'), 'riscv64')

    def test_dorion_download_that_doesnt_match_is_never_unpacked(self):
        rel = self.release(b'the real package')

        def fetch(url, accept='application/json'):
            return json.dumps(rel).encode() if 'api.github.com' in url else b'something else'
        ran = []
        with mock.patch.object(hd, 'which', return_value='/usr/bin/dpkg-deb'), mock.patch.object(hd, 'deb_arch', return_value='amd64'), \
             mock.patch.object(hd, 'run', side_effect=lambda c, *a, **k: ran.append(c) or (0, '', '')), mock.patch.object(hd, 'progress'):
            with self.assertRaises(hd.Fail) as e:
                hd.install_dorion(fetch, hd.DORION_ROOT, hd.DORION_DESKTOP)
        self.assertIn('checksum', str(e.exception))
        self.assertEqual(ran, [], 'nothing was unpacked')
        self.assertFalse(os.path.exists(hd.DORION_ROOT))

    def test_dorion_good_download_is_unpacked_in_home(self):
        rel = self.release(b'the real package')

        def fetch(url, accept='application/json'):
            return json.dumps(rel).encode() if 'api.github.com' in url else b'the real package'

        def fake_run(c, *a, **k):
            if c[:2] == ['dpkg-deb', '-x']:
                os.makedirs(os.path.join(c[3], 'usr', 'bin'))
                open(os.path.join(c[3], 'usr', 'bin', 'dorion'), 'w').close()
            return 0, '', ''
        with mock.patch.object(hd, 'which', return_value='/usr/bin/dpkg-deb'), mock.patch.object(hd, 'deb_arch', return_value='amd64'), \
             mock.patch.object(hd, 'run', side_effect=fake_run), mock.patch.object(hd, 'progress'):
            r = hd.install_dorion(fetch, hd.DORION_ROOT, hd.DORION_DESKTOP)
        self.assertEqual(r['version'], 'v6.13.1')
        self.assertTrue(os.path.exists(os.path.join(hd.DORION_ROOT, 'usr', 'bin', 'dorion')))
        self.assertIn('GDK_BACKEND=x11', open(hd.DORION_DESKTOP).read())


class Windows(Base):
    def test_each_client_starts_under_xwayland(self):
        cmd, env = hd.launch_command('official', have_flatpak=True)
        self.assertEqual(cmd, ['flatpak', 'run', 'com.discordapp.Discord'])
        self.assertEqual(hd.launch_command('official', have_flatpak=False)[0], ['discord'])
        with mock.patch.object(hd, 'firefox', return_value='/usr/lib/hearth-browser/firefox/firefox'):
            cmd, env = hd.launch_command('web', (500, 700))
        self.assertEqual(cmd[0], '/usr/lib/hearth-browser/firefox/firefox')
        self.assertIn('https://discord.com/app', cmd)
        self.assertEqual(cmd[cmd.index('--class') + 1], hd.WEB_CLASS)
        self.assertEqual(cmd[cmd.index('--profile') + 1], hd.WEB_PROFILE, 'its own profile, never his browser\'s')
        self.assertEqual((cmd[cmd.index('--width') + 1], cmd[cmd.index('--height') + 1]), ('500', '700'))
        self.assertEqual(env, {'MOZ_ENABLE_WAYLAND': '0'})
        with mock.patch.object(hd, 'dorion_bin', return_value='/x/dorion'):
            self.assertEqual(hd.launch_command('dorion'), (['/x/dorion'], {'GDK_BACKEND': 'x11'}))

    def test_web_profile_hides_browser_bars_only(self):
        hd.ensure_web_profile(hd.WEB_PROFILE)
        css = open(os.path.join(hd.WEB_PROFILE, 'chrome', 'userChrome.css')).read()
        self.assertIn('#nav-bar', css)
        self.assertNotIn('discord', css.lower(), 'nothing of Discord\'s page is touched')

    def test_geometry_on_every_edge(self):
        screen = (1920, 1080, 0, 0)
        b = hd.geometry('bottom', screen, 48, -1, (420, 640))
        self.assertEqual(b, {'x': 1920 - 420 - 8, 'y': 1080 - 48 - 640 - 8, 'w': 420, 'h': 640})
        t = hd.geometry('top', screen, 40, 1000, (420, 640))
        self.assertEqual((t['x'], t['y']), (1000 - 210, 48))
        l = hd.geometry('left', screen, 64, 300, (420, 640))
        self.assertEqual((l['x'], l['y']), (72, 8), 'centred on the button but kept on the screen')
        r = hd.geometry('right', screen, 64, -1, (420, 640))
        self.assertEqual((r['x'], r['y']), (1920 - 64 - 420 - 8, 1080 - 640 - 8))
        # A second screen to the right, a small handheld screen, a size bigger than the screen.
        self.assertEqual(hd.geometry('bottom', (1280, 800, 1920, 0), 48, -1, (420, 640))['x'], 1920 + 1280 - 420 - 8)
        small = hd.geometry('bottom', (800, 600, 0, 0), 48, -1, (2000, 2000))
        self.assertEqual((small['w'], small['h']), (784, 584))
        self.assertGreaterEqual(small['y'], 0)
        for edge in ('top', 'bottom', 'left', 'right'):
            g = hd.geometry(edge, screen, 48, 5000, (420, 640))
            self.assertTrue(0 <= g['x'] <= 1920 - g['w'] and 0 <= g['y'] <= 1080 - g['h'], edge)

    def test_size_is_state_not_a_setting(self):
        self.assertEqual(hd.saved_size(hd.SIZE_STATE), [420, 640])
        hd.save_size(500, 100, hd.SIZE_STATE)
        self.assertEqual(hd.saved_size(hd.SIZE_STATE), [500, hd.MIN_H])
        self.assertFalse(os.path.exists(hd.SETTINGS))
        self.assertIn('state', hd.SIZE_STATE)

    def test_place_and_remember_with_a_stand_in_xdotool(self):
        calls = []

        def fake_run(c, *a, **k):
            calls.append(c)
            if c[1] == 'search':
                return (0, '4194305\n4194309\n', '') if c[-1] == '^discord$' else (1, '', '')
            if c[1] == 'getwindowgeometry':
                return 0, 'WINDOW=4194309\nX=10\nY=20\nWIDTH=480\nHEIGHT=700\n', ''
            return 0, '', ''
        with mock.patch.object(hd, 'run', side_effect=fake_run):
            self.assertTrue(hd.place_window('official', {'x': 1, 'y': 2, 'w': 420, 'h': 640}, '/usr/bin/xdotool'))
            self.assertIn(['/usr/bin/xdotool', 'windowsize', '4194309', '420', '640'], calls)
            self.assertIn(['/usr/bin/xdotool', 'windowmove', '4194309', '1', '2'], calls)
            self.assertEqual(hd.remember('official', '/usr/bin/xdotool'), [480, 700])
            self.assertFalse(hd.place_window('web', {'x': 1, 'y': 2, 'w': 3, 'h': 4}, '/usr/bin/xdotool'))
        self.assertEqual(hd.saved_size(hd.SIZE_STATE), [480, 700])
        self.assertFalse(any(c[1] in ('windowkill', 'windowclose', 'key', 'type') for c in calls), 'only placed, never closed or typed into')


class Joining(Base):
    games = [{'id': 'epic:fn', 'title': 'Fortnite'}, {'id': 'gog:celeste', 'title': 'Celeste'}]

    def test_secret_matches_the_cores_format(self):
        # hosts/hearth-core/src/discord.rs secrets_round_trip_and_fit makes exactly this one.
        s = hd.parse_secret('h1|steam:620|100.101.102.103|0420|1790000000|Portal 2')
        self.assertEqual(s, {'id': 'steam:620', 'addr': '100.101.102.103', 'pin': '0420', 'expires': 1790000000, 'title': 'Portal 2'})
        self.assertIsNone(hd.parse_secret('h1|steam:1|1.2.3.4;rm|1234|0|x'))
        self.assertIsNone(hd.parse_secret('h1|steam:1|1.2.3.4|12a4|0|x'))
        self.assertIsNone(hd.parse_secret('h2|steam:1|||0|x'))
        self.assertIsNone(hd.parse_secret('x' * 200))
        self.assertEqual(hd.secret_from_link('https://hearthos.fun/join#h1%7Cgog:1%7C%7C%7C0%7CCeleste'), 'h1|gog:1|||0|Celeste')
        self.assertEqual(hd.secret_from_link('hearth://join/h1%7Cgog:1%7C%7C%7C0%7CCeleste'), 'h1|gog:1|||0|Celeste')
        self.assertIsNone(hd.secret_from_link('https://example.org/join#h1'))

    def test_plans(self):
        now = 1790000000
        pt = hd.parse_secret('h1|epic:fn|192.168.1.20|1234|%d|Fortnite' % (now + 300))
        self.assertEqual(hd.plan_join(pt, self.games, True, now), ('stream', '192.168.1.20', '1234'))
        self.assertEqual(hd.plan_join(pt, self.games, False, now), ('page', 'epic:fn'), 'no Moonlight: the game page')
        old = hd.parse_secret('h1|epic:fn|192.168.1.20|1234|%d|Fortnite' % (now - 1))
        self.assertEqual(hd.plan_join(old, self.games, True, now)[0], 'page', 'an expired PIN is never used')
        by_title = hd.parse_secret('h1|gog:other-id|||0|celeste')
        self.assertEqual(hd.plan_join(by_title, self.games, True, now), ('page', 'gog:celeste'))
        steam = hd.parse_secret('h1|steam:620|||0|Portal 2')
        self.assertEqual(hd.plan_join(steam, self.games, True, now), ('steam', '620'))
        self.assertEqual(hd.plan_join(hd.parse_secret('h1|epic:zz|||0|Zed'), self.games, True, now), ('missing', 'Zed'))

    def test_join_runs_only_safe_commands(self):
        spawned = []
        with mock.patch.object(hd, 'spawn', side_effect=lambda c, env=None: spawned.append(c)), mock.patch.object(hd, 'notify'):
            hd.do_join('h1|gog:celeste|||0|Celeste', self.games, False)
            hd.do_join('h1|steam:620|||0|Portal 2', self.games, False)
        self.assertEqual(spawned, [['nimbus', '--windowed', '--game-page', 'gog:celeste'], ['xdg-open', 'steam://store/620']])
        with self.assertRaises(hd.Fail):
            hd.do_join('not an invite', self.games, False)

    def test_invite_file_expiry(self):
        os.makedirs(os.path.dirname(hd.INVITE))
        with open(hd.INVITE, 'w') as f:
            json.dump({'title': 'Celeste', 'pin': '1234', 'expires': 100, 'playTogether': True, 'link': 'https://hearthos.fun/join#x'}, f)
        v = hd.read_invite(hd.INVITE, now=200)
        self.assertEqual((v['pin'], v['playTogether']), ('', False))
        self.assertEqual(hd.read_invite(hd.INVITE, now=50)['pin'], '1234')
        self.assertIsNone(hd.read_invite(os.path.join(self.tmp, 'none.json')))

    def test_waits_for_a_join_from_a_fake_discord(self):
        import socket
        import threading
        path = os.path.join(self.tmp, 'discord-ipc-0')
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        srv.bind(path)
        srv.listen(1)

        def serve():
            c, _ = srv.accept()
            op, hello = hd._read_frame(c)
            assert op == 0 and hello['client_id'] == '123456789012345678'
            c.sendall(hd._frame(1, {'cmd': 'DISPATCH', 'evt': 'READY'}))
            op, sub = hd._read_frame(c)
            assert sub['cmd'] == 'SUBSCRIBE' and sub['evt'] == 'ACTIVITY_JOIN'
            c.sendall(hd._frame(3, {'ping': 1}))
            hd._read_frame(c)
            c.sendall(hd._frame(1, {'cmd': 'DISPATCH', 'evt': 'ACTIVITY_JOIN', 'data': {'secret': 'h1|gog:celeste|||0|Celeste'}}))
            time.sleep(0.2)
            c.close()
        t = threading.Thread(target=serve)
        t.start()
        try:
            self.assertEqual(hd.wait_for_join('123456789012345678', 10, [path]), 'h1|gog:celeste|||0|Celeste')
        finally:
            t.join(5)
            srv.close()


class MantelAndImage(unittest.TestCase):
    def read(self, p):
        return (REPO / p).read_text(encoding='utf-8')

    def test_quick_window_hides_without_quitting(self):
        q = self.read('hosts/linux/desktop/shell/Discord.qml')
        self.assertIn('minimized = true', q, 'hiding minimizes Discord (notifications keep working)')
        self.assertNotRegex(q, r'\.close\(\)\s*;?\s*//?.*discord|win\.close\(\)|toplevel\.close\(\)', 'never closes Discord')
        self.assertIn('target: "discord"', q)
        self.assertIn('target: "friends"', q, 'older keybinds and links to Friends open Discord')
        for c in ('"official"', '"web"', '"dorion"'):
            self.assertIn(c, q)
        self.assertIn('unofficial', q.lower(), 'the Dorion card says plainly it is unofficial')
        self.assertIn('Theme.barEdge', q)

    def test_toggle_logic(self):
        q = self.read('hosts/linux/desktop/shell/Discord.qml')
        # The decision is one pure function so it reads plainly: chooser first, then install, then show/hide.
        m = re.search(r'function decide\((.*?)\) \{(.*?)\n    \}', q, re.S)
        self.assertIsNotNone(m)
        body = m.group(2)
        for word in ('"choose"', '"install"', '"hide"', '"show"', '"start"'):
            self.assertIn(word, body)

    def test_bar_button_is_discord(self):
        for f in ('Bar.qml', 'Islands.qml', 'QuickAccess.qml'):
            t = self.read('hosts/linux/desktop/shell/' + f)
            self.assertNotIn('SettingsNav.friends.toggle', t, f)
            self.assertIn('SettingsNav.discord', t, f)
        self.assertIn('Loader { source: "Discord.qml" }', self.read('hosts/linux/desktop/shell/shell.qml'))
        self.assertFalse((SHELL / 'Friends.qml').exists())

    def test_matrix_service_is_gone_from_session_and_image(self):
        for f in ('os-bases/peppermint/build.sh', 'os-bases/anatase/build.sh', '.github/workflows/base-peppermint.yml', '.github/workflows/base-anatase.yml'):
            t = self.read(f)
            self.assertNotIn('ExecStart=/usr/bin/hearth-friends', t, f)
            self.assertNotIn('enable hearth-friends.service', t, f)
            self.assertNotIn('cd hosts/hearth-friends', t, f)
        self.assertFalse((REPO / 'hosts' / 'hearth-friends').exists())
        self.assertTrue((REPO / 'archive' / 'hosts' / 'hearth-friends' / 'Cargo.toml').exists())
        self.assertFalse((REPO / 'hosts' / 'hearth-core' / 'src' / 'matrix_friends.rs').exists())
        for p in (REPO / 'os').rglob('*'):
            if p.is_file() and p.suffix in ('', '.sh', '.service', '.desktop', '.target') and p.stat().st_size < 400000:
                try:
                    t = p.read_text(encoding='utf-8')
                except UnicodeDecodeError:
                    continue
                self.assertNotIn('hearth-friends', t, str(p))
        self.assertNotIn('hearth-friends', self.read('hosts/linux/desktop/labwc/autostart'))
        self.assertIn('Hearth Friends (Matrix)', self.read('docs/ARCHIVED.md'))
        self.assertTrue((REPO / 'docs' / 'archive' / 'FRIENDS.md').exists())

    def test_core_never_reads_matrix_settings_or_user_data(self):
        core = self.read('hosts/hearth-core/src/discord.rs') + self.read('hosts/linux/desktop/shell/hearth-discord')
        self.assertNotIn('friends/store', core)
        self.assertNotRegex(core, r'(?i)user.?token|password\s*=|Authorization')
        self.assertIn('SUBSCRIBE', core)
        self.assertIn('ACTIVITY_JOIN', core)


if __name__ == '__main__':
    unittest.main()
