"""Mantel on KWin: typing, the lock screen and screen sharing (docs/KWIN-INPUT.md).

hearth-type (wtype's arguments; Hearth's input method for text, a uinput keyboard in the active layout for keys),
hearth_screencast.py (the ScreenCast / Screenshot portal backend's pieces), and the wiring: the on-screen keyboard,
clipboard paste and the magnifier type through hearth-type on KWin, Lock.qml hands locking to KWin's locker there
(never a WlSessionLock that covers nothing), KWin never locks by itself, and hearth.portal offers the new portals.
Run: python3 -m unittest discover -s hosts/linux/test -p 'test_kwin_input.py'"""
import importlib.machinery
import importlib.util
import json
import os
import socket
import stat
import struct
import sys
import tempfile
import threading
import time
import unittest
import zlib

sys.dont_write_bytecode = True
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')
sys.path.insert(0, SHELL)


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, path)
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


ht = load('hearth_type', os.path.join(SHELL, 'hearth-type'))
import hearth_screencast as hs  # noqa: E402


def read(*p):
    with open(os.path.join(ROOT, *p), encoding='utf-8') as f:
        return f.read()


def evs(steps):
    out = []
    for kind, val in steps:
        if kind == 'ev':
            out.extend(val)
    return out


class FakeIM(threading.Thread):
    """Answers like hearth-kwin-im: S, T (commits into a pretend text field), K."""

    def __init__(self, path, active=True):
        threading.Thread.__init__(self, daemon=True)
        self.path, self.active, self.committed = path, active, []
        self.srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.srv.bind(path)
        self.srv.listen(4)

    def run(self):
        while True:
            try:
                c, _ = self.srv.accept()
            except OSError:
                return
            buf = b''
            with c:
                while True:
                    d = c.recv(4096)
                    if not d:
                        break
                    buf += d
                    while b'\n' in buf:
                        line, buf = buf.split(b'\n', 1)
                        line = line.decode()
                        if line == 'S':
                            c.sendall(('active %d purpose 0 hint 0\n' % self.active).encode())
                        elif line.startswith('T '):
                            if self.active:
                                import urllib.parse
                                self.committed.append(urllib.parse.unquote(line[2:]))
                                c.sendall(b'ok\n')
                            else:
                                c.sendall(b'no\n')


class TypeArgs(unittest.TestCase):
    def test_wtype_arguments(self):
        a = ht.parse_args(['-M', 'ctrl', '-k', 'v', '-m', 'ctrl'])
        self.assertEqual(a, [('mod', 'ctrl', True), ('key', 'v', 'tap'), ('mod', 'ctrl', False)])
        self.assertEqual(ht.parse_args(['--', '-k', 'x']), [('text', '-k'), ('text', 'x')])
        self.assertEqual(ht.parse_args(['hello', '-s', '50']), [('text', 'hello'), ('sleep', 50)])
        # Keyboard.qml's paste: a bare "v" with Ctrl held, as wtype takes it.
        self.assertEqual(ht.parse_args(['-M', 'ctrl', 'v', '-m', 'ctrl'])[1], ('text', 'v'))
        for bad in (['-M', 'hyper'], ['-k'], ['-s', 'x'], ['--bogus']):
            with self.assertRaises(ht.Fail):
                ht.parse_args(bad)

    def test_keys_through_the_layout(self):
        self.assertEqual(ht.key_codes('Return'), ['KEY_ENTER'])
        self.assertEqual(ht.key_codes('Page_Down'), ['KEY_PAGEDOWN'])
        self.assertEqual(ht.key_codes('F5'), ['KEY_F5'])
        self.assertEqual(ht.key_codes('equal'), ['KEY_EQUAL'])
        self.assertEqual(ht.key_codes('z', 'us'), ['KEY_Z'])
        self.assertEqual(ht.key_codes('z', 'de'), ['KEY_Y'])       # QWERTZ
        self.assertEqual(ht.key_codes('a', 'fr'), ['KEY_Q'])       # AZERTY
        self.assertIsNone(ht.key_codes('NoSuchKey'))

    def test_characters(self):
        self.assertEqual(ht.char_keys('A', 'us'), ['KEY_LEFTSHIFT', 'KEY_A'])
        self.assertEqual(ht.char_keys('@', 'us'), ['KEY_LEFTSHIFT', 'KEY_2'])
        self.assertEqual(ht.char_keys('\n'), ['KEY_ENTER'])
        self.assertIsNone(ht.char_keys('🔥', 'us'))
        self.assertIsNone(ht.char_keys('é', 'us'))
        # German has its own @ (AltGr+Q) and ö.
        self.assertEqual(ht.char_keys('@', 'de')[-1], 'KEY_Q')
        self.assertIn('KEY_RIGHTALT', ht.char_keys('@', 'de'))
        self.assertIsNotNone(ht.char_keys('ö', 'de'))
        self.assertEqual(ht.layout_table('de(nodeadkeys)')[0], 'de')
        self.assertEqual(ht.layout_table('xx')[0], 'us')

    def test_every_key_is_on_the_virtual_keyboard(self):
        codes = set(ht.all_codes())
        for layout in ht.KEYMAP['layouts']:
            for ch in list(ht.KEYMAP['layouts'][layout]) + list('abcdefghijklmnopqrstuvwxyz'):
                if ch.startswith('_'):
                    continue
                keys = ht.char_keys(ch, layout)
                if keys:
                    self.assertTrue(set(keys) <= codes, (layout, ch, keys))
        for k in ht.NAMED.values():
            self.assertIn(k, codes)


class TypePlan(unittest.TestCase):
    def test_keys_only(self):
        steps, skipped = ht.plan(ht.parse_args(['-M', 'ctrl', '-k', 'v', '-m', 'ctrl']), 'us')
        self.assertEqual(evs(steps), [('KEY_LEFTCTRL', 1), ('KEY_V', 1), ('KEY_V', 0), ('KEY_LEFTCTRL', 0)])
        self.assertEqual(skipped, '')

    def test_text_without_input_method(self):
        steps, skipped = ht.plan([('text', 'Hi é🔥')], 'us')
        e = evs(steps)
        self.assertEqual(e[:4], [('KEY_LEFTSHIFT', 1), ('KEY_H', 1), ('KEY_H', 0), ('KEY_LEFTSHIFT', 0)])
        self.assertEqual(skipped, 'é🔥')
        # Every key goes down and comes up again.
        downs = [k for k, v in e if v == 1]
        ups = [k for k, v in e if v == 0]
        self.assertEqual(sorted(downs), sorted(ups))

    def test_text_with_input_method(self):
        steps, skipped = ht.plan([('text', 'é🔥\nok')], 'us', im_ok=True)
        self.assertEqual([s for s in steps if s[0] == 'im?'], [('im?', 'é🔥'), ('im?', 'ok')])
        self.assertIn(('ev', [('KEY_ENTER', 1), ('KEY_ENTER', 0)]), steps)
        self.assertEqual(skipped, '')

    def test_modifier_held_text_goes_as_keys(self):
        steps, _ = ht.plan(ht.parse_args(['-M', 'ctrl', 'v', '-m', 'ctrl']), 'us', im_ok=True)
        self.assertFalse([s for s in steps if s[0] == 'im?'])
        self.assertEqual(evs(steps), [('KEY_LEFTCTRL', 1), ('KEY_V', 1), ('KEY_V', 0), ('KEY_LEFTCTRL', 0)])


class TypeRoutes(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = dict(os.environ)
        os.environ['XDG_RUNTIME_DIR'] = self.tmp
        os.environ['HEARTH_TYPE_LAYOUT'] = 'us'
        os.makedirs(os.path.join(self.tmp, 'hearth'), mode=0o700)
        self.sent = []
        self.orig = ht.service_request
        ht.service_request = lambda req, start=True: (self.sent.append(req), {'ok': True})[1]

    def tearDown(self):
        ht.service_request = self.orig
        os.environ.clear()
        os.environ.update(self.env)

    def test_route(self):
        os.environ['HEARTH_WM'] = 'kwin'
        os.environ.pop('HEARTH_TYPE_ROUTE', None)
        self.assertEqual(ht.route(), 'kwin')
        os.environ['HEARTH_TYPE_ROUTE'] = 'wtype'
        self.assertEqual(ht.route(), 'wtype')

    def test_input_method_takes_text(self):
        im = FakeIM(ht.im_sock_path(), active=True)
        im.start()
        ht.run_kwin(ht.parse_args(['--', 'Grüße 🔥']))
        self.assertEqual(im.committed, ['Grüße 🔥'])
        self.assertEqual(self.sent, [])

    def test_no_text_field_falls_back_to_keys(self):
        im = FakeIM(ht.im_sock_path(), active=False)
        im.start()
        ht.run_kwin(ht.parse_args(['--', 'ab']))
        self.assertEqual(im.committed, [])
        self.assertEqual(sum(len(r['ev']) for r in self.sent), 4)

    def test_no_input_method_at_all(self):
        ht.run_kwin(ht.parse_args(['-k', 'Return']))
        self.assertEqual(self.sent, [{'ev': [('KEY_ENTER', 1), ('KEY_ENTER', 0)]}])

    def test_percent_escape(self):
        self.assertEqual(ht.pct('a%b\nc'), 'a%25b%0Ac')


class ScreenCast(unittest.TestCase):
    def test_outputs(self):
        outs, v = hs.parse_outputs('output DP-2\tDell U2720Q\t1920\t0\t3840\t2160\t2\n'
                                   'output Virtual-1\tVirtual\t0\t0\t1920\t1080\t1\nscreencast 5\n')
        self.assertEqual([o['name'] for o in outs], ['Virtual-1', 'DP-2'])
        self.assertEqual(v, 5)
        self.assertEqual(outs[1]['w'], 3840)

    def test_png(self):
        w, h, stride = 3, 2, 16
        rows = []
        for y in range(h):
            rows.append(bytes([10, 20, 30, 255] * w) + b'\0' * (stride - w * 4))   # B G R A
        png = hs.to_png('%d %d %d 5 1' % (w, h, stride), b''.join(rows))
        self.assertTrue(png.startswith(b'\x89PNG'))
        idat = png[png.index(b'IDAT') + 4:]
        raw = zlib.decompress(idat[:struct.unpack('>I', png[png.index(b'IDAT') - 4:png.index(b'IDAT')])[0]])
        self.assertEqual(raw[:5], b'\x00' + bytes([30, 20, 10, 255]))     # R G B A
        with self.assertRaises(ValueError):
            hs.to_png('3 2 16 99 1', b''.join(rows))
        with self.assertRaises(ValueError):
            hs.to_png('3 2 16 5 1', b'short')

    def test_restore(self):
        outs = [{'name': 'DP-1', 'x': 0, 'y': 0, 'w': 1920, 'h': 1080}]
        self.assertEqual(hs.restore_target(('hearth', 1, {'screen': 'DP-1'}), outs), {'kind': 'screen', 'name': 'DP-1'})
        self.assertIsNone(hs.restore_target(('hearth', 1, {'screen': 'HDMI-A-1'}), outs))
        self.assertIsNone(hs.restore_target(('kde', 1, {'screen': 'DP-1'}), outs))
        self.assertIsNone(hs.restore_target(None, outs))

    def test_results(self):
        class S:
            node = 77
        outs = [{'name': 'DP-1', 'x': 0, 'y': 0, 'w': 1920, 'h': 1080}]
        r = hs.stream_results([S()], {'kind': 'screen', 'name': 'DP-1'}, outs, 2)
        node, props = r['streams'][0]
        self.assertEqual(node, 77)
        self.assertEqual(props['source_type'], ('u', hs.MONITOR))
        self.assertEqual(props['size'], ('(ii)', (1920, 1080)))
        self.assertEqual(r['persist_mode'], ('u', 2))
        w = hs.stream_results([S()], {'kind': 'window', 'id': '{abc}'}, outs, 2)
        self.assertEqual(w['streams'][0][1]['source_type'], ('u', hs.WINDOW))
        self.assertNotIn('restore_data', w)
        try:
            import gi
            gi.require_version('GLib', '2.0')
            from gi.repository import GLib
        except (ImportError, ValueError):
            return
        v = GLib.Variant('a{sv}', hs.results_variant(GLib, r))
        self.assertEqual(v['streams'][0][0], 77)
        self.assertEqual(v['restore_data'][0], 'hearth')

    def test_request(self):
        outs = [{'name': 'DP-1', 'desc': 'Main', 'x': 0, 'y': 0, 'w': 1920, 'h': 1080}]
        wins = [{'id': '{1}', 'title': 'Firefox', 'app': 'firefox', 'desktop': 'firefox', 'minimized': False}]
        r = hs.build_request('cast', 'com.discordapp.Discord', hs.MONITOR, False, outs, wins, '/nonexistent', False)
        self.assertEqual([s['name'] for s in r['screens']], ['DP-1'])
        self.assertEqual(r['windows'], [])
        r = hs.build_request('cast', '', hs.MONITOR | hs.WINDOW, False, outs, wins, '/nonexistent', False)
        self.assertEqual(r['windows'][0]['title'], 'Firefox')
        self.assertFalse(r['desktop'])
        self.assertTrue(hs.build_request('shot', '', 0, False, outs, wins, '/x', False)['desktop'])
        self.assertEqual(hs.cursor_mode(4), 4)
        self.assertEqual(hs.cursor_mode(0), hs.CURSOR_EMBEDDED)

    def test_stream_helper(self):
        d = tempfile.mkdtemp()
        fake = os.path.join(d, 'cast')
        with open(fake, 'w') as f:
            f.write('#!/bin/sh\necho "node 42"\nread x\necho closed\n')
        os.chmod(fake, 0o755)
        os.environ['HEARTH_KWIN_CAST'] = fake
        try:
            s = hs.Stream(('output', 'DP-1'), 2)
            self.assertEqual(s.wait_node(3), 42)
            s.stop()
            self.assertIsNotNone(s.proc.poll())
            with open(fake, 'w') as f:
                f.write('#!/bin/sh\necho "failed no screen called DP-9"\nexit 1\n')
            s = hs.Stream(('output', 'DP-9'), 2)
            self.assertIsNone(s.wait_node(3))
            self.assertIn('DP-9', s.error)
        finally:
            del os.environ['HEARTH_KWIN_CAST']


class Wiring(unittest.TestCase):
    def test_keyboard_types_through_hearth_type_on_kwin(self):
        kb = read('hosts/linux/desktop/shell/Keyboard.qml')
        self.assertIn('HEARTH_WM', kb)
        self.assertIn('"hearth-type"', kb)
        self.assertNotIn('push(["wtype"', kb)
        self.assertIn('typeCmd.concat(["-M", "ctrl", "v", "-m", "ctrl"])', kb)

    def test_paste_and_magnifier(self):
        self.assertIn("os.environ.get('HEARTH_WM') == 'kwin'", read('hosts/linux/desktop/shell/hearth-clipboard'))
        self.assertIn("'hearth-type'", read('hosts/linux/desktop/shell/hearth-access'))
        self.assertIn('HEARTH_TYPE', read('hosts/hearth-emoji/src/lib.rs'))
        self.assertIn('hearth-type', read('hosts/hearth-connect/src/desktop.rs'))

    def test_lock_never_covers_nothing(self):
        lock = read('hosts/linux/desktop/shell/Lock.qml')
        # On KWin the Wayland lock is off (KWin has no ext-session-lock-v1) and KWin's locker locks instead,
        # checked, with a loud message when it didn't.
        self.assertIn('locked: root.locked && !root.onKwin', lock)
        self.assertIn('org.freedesktop.ScreenSaver.Lock', lock)
        self.assertIn('GetActive', lock)
        self.assertIn('"critical"', lock)
        rc = read('hosts/linux/desktop/kwin/xdg/kscreenlockerrc')
        for line in ('Autolock=false', 'LockOnResume=false', 'Theme=org.hearth.mantel-lock'):
            self.assertIn(line, rc)
        meta = json.loads(read('hosts/linux/desktop/kwin-lock/metadata.json'))
        self.assertEqual(meta['KPlugin']['Id'], 'org.hearth.mantel-lock')
        qml = read('hosts/linux/desktop/kwin-lock/contents/lockscreen/LockScreen.qml')
        self.assertIn('authenticator.respond', qml)
        self.assertIn('Qt.quit()', qml)
        self.assertNotIn('import org.kde', qml)

    def test_portal_files(self):
        p = read('os/system_files/usr/share/xdg-desktop-portal/portals/hearth.portal')
        for i in ('FileChooser', 'ScreenCast', 'Screenshot'):
            self.assertIn('org.freedesktop.impl.portal.%s;' % i, p)
        self.assertIn('hearth_screencast', read('hosts/linux/desktop/shell/hearth-portal'))
        cast = read('os/system_files/usr/share/applications/hearth-kwin-cast.desktop')
        self.assertIn('Exec=/usr/libexec/hearth/hearth-kwin-cast\n', cast)
        self.assertIn('X-KDE-Wayland-Interfaces=zkde_screencast_unstable_v1', cast)
        self.assertIn('X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2', cast)
        self.assertIn('Exec=/usr/libexec/hearth/hearth-kwin-im\n',
                      read('os/system_files/usr/share/applications/hearth-kwin-im.desktop'))


if __name__ == '__main__':
    unittest.main()
