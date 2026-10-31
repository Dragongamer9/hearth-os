"""Tests for Hearth COSMIC (docs/COSMIC.md): the desktop choice and the fallback in nimbus-session (a broken or missing
COSMIC always ends in Hearth Classic), the desktop choice in hearth-plasma and admin.rs, hearth-pad's COSMIC mode
(CosmicPad), and the defaults made from COSMIC's own (hearth-cosmic-defaults). The session runs for real with
stand-ins for labwc and COSMIC."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'plasma' / 'hearth-plasma'
PAD = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-pad'
DEFAULTS = ROOT / 'hosts' / 'linux' / 'desktop' / 'cosmic' / 'hearth-cosmic-defaults'
ADMIN_RS = ROOT / 'hosts' / 'hearth-core' / 'src' / 'admin.rs'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


FAKE_COSMIC = {
    'crash': '#!/bin/sh\necho "cosmic crashed" >&2\nexit 1\n',
    'hang': '#!/bin/sh\nexec sleep 30\n',
    'ok': '#!/bin/sh\n"$(dirname "$0")/cosmic-comp" & "$(dirname "$0")/cosmic-panel" &\n'
          'echo "XDG_DATA_DIRS=$XDG_DATA_DIRS XDG_CONFIG_DIRS=$XDG_CONFIG_DIRS HEARTH_DESKTOP=$HEARTH_DESKTOP CUR=${XDG_CURRENT_DESKTOP-unset}" > "$HOME/cosmic-env"\n'
          'sleep 4\nexit 0\n',
}


class SessionFallback(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-cosmic-test-')
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        for d in (self.home, self.run_dir, self.bin, self.share):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        os.makedirs(os.path.dirname(self.conf))
        self.write_bin('labwc', '#!/bin/sh\ntouch "$HOME/labwc-ran"\nexit 0\n')

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.bin], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def cosmic(self, kind):
        self.write_bin('start-cosmic', FAKE_COSMIC[kind])
        self.write_bin('cosmic-comp', '#!/bin/sh\nsleep 6\n')
        self.write_bin('cosmic-panel', '#!/bin/sh\nsleep 6\n')

    def installed(self):
        return os.path.isdir('/usr/share/hearth-cosmic/data/cosmic')

    def session(self, conf, *args, extra_env=None):
        with open(self.conf, 'w') as f:
            f.write(conf)
        env = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
               'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent', 'HEARTH_COSMIC_TIMEOUT': '20'}
        env.update(extra_env or {})
        r = subprocess.run(['sh', str(SESSION)] + list(args), env=env, capture_output=True, text=True, timeout=60)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        return r, log.read_text() if log.exists() else ''

    def labwc_ran(self):
        return os.path.exists(os.path.join(self.home, 'labwc-ran'))

    def test_missing_defaults_uses_classic(self):
        # COSMIC's programs are there, Hearth's COSMIC defaults aren't (Peppermint, or an image where they failed).
        if self.installed():
            self.skipTest("Hearth COSMIC's defaults are installed on this machine")
        self.cosmic('ok')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=cosmic\n')
        self.assertIn('cosmic: not installed', log)
        self.assertTrue(self.labwc_ran(), log)

    def test_login_screen_choice_is_saved(self):
        self.cosmic('ok')
        self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n', '--desktop=cosmic')
        self.assertIn('HEARTH_DESKTOP=cosmic', pathlib.Path(self.conf).read_text())
        self.assertNotIn('HEARTH_DESKTOP=plasma', pathlib.Path(self.conf).read_text())

    def test_deepin_choice_is_classic(self):
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=deepin\n')
        self.assertIn('Hearth Classic', log)
        self.assertTrue(self.labwc_ran(), log)

    def fake_installed(self):
        """nimbus-session with the defaults folder pointed at a temporary one (can_cosmic checks it)."""
        data = os.path.join(self.tmp, 'hearth-cosmic', 'data')
        os.makedirs(os.path.join(data, 'cosmic'))
        text = SESSION.read_text().replace('/usr/share/hearth-cosmic', os.path.join(self.tmp, 'hearth-cosmic'))
        p = os.path.join(self.tmp, 'nimbus-session')
        with open(p, 'w') as f:
            f.write(text)
        return p

    def run_fake(self, conf, extra_env=None):
        global SESSION
        old = SESSION
        SESSION = pathlib.Path(self.fake_installed())
        try:
            return self.session(conf, extra_env=extra_env)
        finally:
            SESSION = old

    def test_crash_falls_back_to_classic(self):
        self.cosmic('crash')
        _, log = self.run_fake('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=cosmic\n')
        self.assertIn('falling back to Hearth Classic', log)
        self.assertTrue(self.labwc_ran(), log)
        self.assertTrue(os.path.exists(os.path.join(self.run_dir, 'hearth-cosmic-failed')))
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth-cosmic.pid')))

    def test_no_desktop_in_time_falls_back(self):
        self.cosmic('hang')
        _, log = self.run_fake('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=cosmic\n', extra_env={'HEARTH_COSMIC_TIMEOUT': '2'})
        self.assertIn('no desktop after 2s', log)
        self.assertTrue(self.labwc_ran(), log)

    def test_runs_and_ends_normally(self):
        self.cosmic('ok')
        _, log = self.run_fake('NIMBUS_START_MODE=desktop\nexport HEARTH_DESKTOP="cosmic"\n')
        self.assertIn('desktop mode: Hearth COSMIC', log)
        self.assertIn('cosmic ended (code 0)', log)
        self.assertFalse(self.labwc_ran(), log)
        env = pathlib.Path(self.home, 'cosmic-env').read_text()
        self.assertIn('hearth-cosmic/data:', env)
        self.assertIn('hearth-cosmic/xdg:', env)
        self.assertIn('HEARTH_DESKTOP=cosmic', env)
        self.assertIn('CUR=unset', env)     # start-cosmic sets COSMIC itself

    def test_failed_earlier_skips_cosmic(self):
        self.cosmic('ok')
        pathlib.Path(self.run_dir, 'hearth-cosmic-failed').touch()
        _, log = self.run_fake('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=cosmic\n')
        self.assertIn("cosmic: didn't start earlier", log)
        self.assertTrue(self.labwc_ran(), log)


class DesktopChoice(unittest.TestCase):
    def test_helper_reads_and_refuses(self):
        m = load(HELPER, 'hearth_plasma_c')
        self.assertEqual(m.read_desktop('HEARTH_DESKTOP=cosmic\n'), 'cosmic')
        self.assertEqual(m.read_desktop('HEARTH_DESKTOP=deepin\n'), 'classic')
        d = m.desktop()
        self.assertIn('cosmic', d)
        self.assertIs(d['deepin'], False)
        if not m.cosmic_installed():
            with self.assertRaises(m.Fail):
                m.desktop_set('cosmic')
        with self.assertRaises(m.Fail):
            m.desktop_set('deepin')

    def test_rust_side_knows_cosmic(self):
        rs = ADMIN_RS.read_text()
        self.assertIn('"cosmic" => "cosmic"', rs)
        self.assertIn('pub fn cosmic_installed', rs)
        self.assertIn('"deepin": false', rs)


class FakeOut:
    def __init__(self):
        self.ev = []

    def move(self, dx, dy):
        self.ev.append(('move', dx, dy))

    def wheel(self, v, h):
        self.ev.append(('wheel', v, h))

    def press(self, name, down):
        self.ev.append(('press', name, down))

    def send(self, obj):
        self.ev.append(('send', obj))

    def rumble(self, *a):
        pass

    def combos(self):
        """The key combinations pressed, e.g. ['KEY_LEFTCTRL+KEY_TAB']."""
        out, held, fresh = [], [], False
        for e in self.ev:
            if e[0] != 'press' or not e[1].startswith('KEY_'):
                continue
            if e[2]:
                held.append(e[1])
                fresh = True
            else:
                if fresh and held and held[-1] == e[1]:
                    out.append('+'.join(held))
                fresh = False
                held = [k for k in held if k != e[1]]
        return out


class CosmicPadKeys(unittest.TestCase):
    def setUp(self):
        self.m = load(PAD, 'hearth_pad_c')
        self.out = FakeOut()
        self.pad = self.m.CosmicPad(self.out)
        self.t = 100.0

    def press(self, *names):
        for n in names:
            self.pad.button(n, True, self.t)
        for n in reversed(names):
            self.pad.button(n, False, self.t + 0.05)
        self.t += 1

    def test_files_keys(self):
        self.pad.command({'cmd': 'app', 'id': 'com.system76.CosmicFiles.desktop'})
        self.press('down')
        self.press('a')
        self.press('b')
        self.press('rb')
        self.press('lb')
        self.assertEqual(self.out.combos(), ['KEY_DOWN', 'KEY_ENTER', 'KEY_LEFTALT+KEY_LEFT', 'KEY_LEFTCTRL+KEY_TAB',
                                             'KEY_LEFTCTRL+KEY_LEFTSHIFT+KEY_TAB'])
        self.press('y')
        self.assertEqual(self.out.combos()[-1], 'KEY_LEFTCTRL+KEY_F')
        self.assertIn(('send', {'t': 'osk', 'open': True}), self.out.ev)

    def test_terminal_y_opens_keyboard_only(self):
        self.pad.command({'cmd': 'app', 'id': 'com.system76.CosmicTerm'})
        self.press('y')
        self.assertEqual(self.out.combos(), [])
        self.assertIn(('send', {'t': 'osk', 'open': True}), self.out.ev)

    def test_other_window_lb_switches_windows_and_view_b_closes(self):
        self.press('rb')
        self.assertIn(('press', 'KEY_LEFTALT', True), self.out.ev)
        self.pad.tick(self.t + 5, 0.01)          # the switcher lets go of Alt
        self.assertIn(('press', 'KEY_LEFTALT', False), self.out.ev)
        self.press('view', 'b')
        self.assertIn('KEY_LEFTMETA+KEY_Q', self.out.combos())
        self.assertNotIn(('send', {'t': 'help'}), self.out.ev)

    def test_unknown_app_view_lb_is_tabs_and_view_y_searches(self):
        self.press('view', 'rb')
        self.assertIn('KEY_LEFTCTRL+KEY_TAB', self.out.combos())
        self.press('view', 'y')
        self.assertIn('KEY_LEFTCTRL+KEY_F', self.out.combos())
        self.assertIn(('send', {'t': 'osk', 'open': True}), self.out.ev)

    def test_files_view_rb_switches_windows(self):
        self.pad.command({'cmd': 'app', 'id': 'com.system76.CosmicFiles'})
        self.press('view', 'rb')
        self.assertIn(('press', 'KEY_LEFTALT', True), self.out.ev)

    def test_view_dpad_moves_and_maximizes(self):
        self.press('view', 'up')
        self.press('view', 'left')
        c = self.out.combos()
        self.assertIn('KEY_LEFTMETA+KEY_LEFTCTRL+KEY_UP', c)
        self.assertIn('KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_LEFT', c)

    def test_a_clicks_after_moving_the_pointer(self):
        self.pad.axes(0, 0, 0.9, 0)
        self.pad.tick(self.t, 0.02)
        self.pad.axes(0, 0, 0, 0)
        self.pad.button('a', True, self.t + 0.1)
        self.pad.button('a', False, self.t + 0.2)
        self.assertIn(('press', 'BTN_LEFT', True), self.out.ev)
        self.assertNotIn('KEY_ENTER', self.out.combos())

    def test_triggers_scroll(self):
        self.pad.button('rt', True, self.t)
        for i in range(20):
            self.pad.tick(self.t + i * 0.02, 0.02)
        self.pad.button('rt', False, self.t + 1)
        wheel = [e for e in self.out.ev if e[0] == 'wheel']
        self.assertTrue(wheel and all(e[1] < 0 for e in wheel), wheel)
        self.assertNotIn(('press', 'BTN_LEFT', True), self.out.ev)

    def test_guide_menu_and_osk_chord(self):
        self.pad.button('guide', True, self.t)
        self.assertEqual(self.pad.mode, 'radial')
        self.pad.button('x', True, self.t + 0.05)
        self.assertIn(('send', {'t': 'osk', 'open': True}), self.out.ev)
        self.pad.button('x', False, self.t + 0.1)
        self.pad.button('guide', False, self.t + 0.1)
        self.assertNotEqual(self.pad.mode, 'radial')

    def test_menu_opens_launcher_and_keys_in_menus(self):
        self.press('menu')
        self.assertIn(('send', {'t': 'launcher'}), self.out.ev)
        self.pad.command({'cmd': 'keys', 'on': True})
        self.press('a')
        self.assertEqual(self.out.combos()[-1], 'KEY_ENTER')

    def test_paused_by_fullscreen_game(self):
        self.pad.command({'cmd': 'pause', 'on': True})
        self.press('a', 'b', 'rb')
        self.assertEqual(self.out.combos(), [])

    def test_close_from_menu(self):
        self.pad.command({'cmd': 'key', 'key': 'close'})
        self.assertEqual(self.out.combos(), ['KEY_LEFTMETA+KEY_Q'])

    def test_keys_are_in_the_virtual_keyboard(self):
        used = set(self.m.KEYS_USED)
        for app in self.m.COSMIC_APPS.values():
            for k in ('a', 'b', 'y'):
                self.assertTrue(set(app[k]) <= used, app[k])
        for keys in self.m.COSMIC_SNAP.values():
            self.assertTrue(set(keys) <= used)


class Link(unittest.TestCase):
    def test_socket_link(self):
        import json as _json
        import socket as _socket
        m = load(PAD, 'hearth_pad_link')
        tmp = tempfile.mkdtemp(prefix='hearth-pad-sock-')
        try:
            path = os.path.join(tmp, 's')
            link = m.CosmicLink(path)
            self.assertEqual(os.stat(path).st_mode & 0o077, 0)
            pad = m.CosmicPad(FakeOut())
            pad.mode = 'radial'
            c = _socket.socket(_socket.AF_UNIX, _socket.SOCK_STREAM)
            c.connect(path)
            self.assertEqual(link.read(link.srv.fileno(), pad), [])
            self.assertEqual(pad.mode, 'desk')
            self.assertEqual(_json.loads(c.recv(100).decode().strip()), {'t': 'ready'})
            m.emit({'t': 'toast', 'text': 'x'})
            self.assertIn('toast', c.recv(100).decode())
            c.sendall(b'{"cmd": "app", "id": "com.system76.CosmicTerm"}\n{"cmd"')
            lines = link.read(link.cli.fileno(), pad)
            self.assertEqual(lines, ['{"cmd": "app", "id": "com.system76.CosmicTerm"}'])
            c.close()
            self.assertEqual(link.read(link.cli.fileno(), pad), [])
            self.assertIsNone(link.cli)
            m.emit({'t': 'toast'})        # nobody connected: dropped quietly
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


SAMPLE_ACCENT = '(\n    base: (\n        red: 0.3882353,\n        green: 0.8156863,\n        blue: 0.8745098,\n        alpha: 1.0,\n    ),\n' \
                '    on: (\n        red: 0.0,\n        green: 0.0,\n        blue: 0.0,\n        alpha: 1.0,\n    ),\n)\n'


class Defaults(unittest.TestCase):
    def test_build_from_cosmic_defaults(self):
        m = load(DEFAULTS, 'hearth_cosmic_defaults')
        tmp = tempfile.mkdtemp(prefix='hearth-cosmic-defaults-')
        try:
            src = os.path.join(tmp, 'src')
            for cid, key, text in (
                ('com.system76.CosmicTheme.Dark', 'accent', SAMPLE_ACCENT),
                ('com.system76.CosmicTheme.Dark', 'background', '(base: (red: 0.10588235, green: 0.10588235, blue: 0.10588235, alpha: 1.0))'),
                ('com.system76.CosmicTheme.Dark.Builder', 'bg_color', 'Some((red: 0.10588235, green: 0.10588235, blue: 0.10588235, alpha: 1.0))'),
                ('com.system76.CosmicPanel.Panel', 'name', '"Panel"'),
                ('com.system76.CosmicTheme.Dark', None, '(\n    base: "#63D0DFFF",\n    hover: "#1B1B1BFF",\n)\n'),
                ('com.system76.CosmicPanel.Panel', 'anchor', 'Top'),
                ('com.system76.CosmicSettings.Shortcuts', 'system_actions', '{\n    AppLibrary: "cosmic-app-library",\n    Launcher: "cosmic-launcher",\n}\n'),
            ):
                ver, key = ('v2', 'accent') if key is None else ('v1', key)
                os.makedirs(os.path.join(src, cid, ver), exist_ok=True)
                with open(os.path.join(src, cid, ver, key), 'w') as f:
                    f.write(text)
            dst = os.path.join(tmp, 'dst')
            m.build(src, dst)
            rd = lambda *p: pathlib.Path(dst, *p).read_text()
            self.assertEqual(rd('com.system76.CosmicPanel.Panel', 'v1', 'anchor').strip(), 'Bottom')
            self.assertEqual(rd('com.system76.CosmicPanel.Panel', 'v1', 'name').strip(), '"Panel"')    # copied whole
            self.assertIn('Launcher: "hearth-cosmic launcher"', rd('com.system76.CosmicSettings.Shortcuts', 'v1', 'system_actions'))
            self.assertIn('AppLibrary: "cosmic-app-library"', rd('com.system76.CosmicSettings.Shortcuts', 'v1', 'system_actions'))
            self.assertIn('Spawn("nimbus-mode game")', rd('com.system76.CosmicSettings.Shortcuts', 'v1', 'custom'))
            acc = rd('com.system76.CosmicTheme.Dark', 'v1', 'accent')
            self.assertNotIn('0.8156863', acc)
            r, g, b = m.retint(0.3882353, 0.8156863, 0.8745098)
            self.assertGreater(r, 0.9)
            self.assertLess(b, 0.4)                       # ember, not cyan
            self.assertIn('alpha: 1.0', acc)
            bg = rd('com.system76.CosmicTheme.Dark', 'v1', 'background')
            self.assertIn('red: 0.090', bg)          # #17110e
            self.assertIn('Hearth-fireside', rd('com.system76.CosmicBackground', 'v1', 'all'))
            self.assertIn('red: 1.0000000', rd('com.system76.CosmicTheme.Dark.Builder', 'v1', 'accent'))
            v2 = rd('com.system76.CosmicTheme.Dark', 'v2', 'accent')     # COSMIC 1.10's hex colours
            self.assertNotIn('#63D0DF', v2)
            self.assertIn('FF"', v2)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_retint_keeps_other_colours(self):
        m = load(DEFAULTS, 'hearth_cosmic_defaults2')
        self.assertEqual(m.retint(1.0, 0.627451, 0.5647059), (1.0, 0.627451, 0.5647059))   # bright red stays


if __name__ == '__main__':
    unittest.main()
