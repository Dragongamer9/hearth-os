"""Tests for dwell click (hearth-dwell, Dwell.qml), inverting one window and the typing sounds on Settings ›
Accessibility (1.14): the countdown never clicks twice in one place, never at the place it started, stops when the
pointer moves away or Esc is pressed; every press has its release; the settings are clamped and only read; the KWin
script's cancel action has no key of its own; hearth-access keeps KWin's invert effect for one window and unloads it
when nothing needs it; the page has the rows."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import types
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


D = load(SHELL / 'hearth-dwell', 'hearth_dwell_test')
A = load(SHELL / 'hearth-access', 'hearth_access_dwell_test')


def clicks(evs):
    return [e for e in evs if e['t'] == 'click']


class Countdown(unittest.TestCase):
    def rest(self, dw, start, until, step=40):
        out, t = [], start
        while t <= until:
            out += dw.tick(t)
            t += step
        return out

    def test_not_where_it_started(self):
        dw = D.Dweller(delay=500, tolerance=10)
        dw.move(100, 100, 0)                       # where the pointer was when dwell click was turned on
        self.assertEqual(clicks(self.rest(dw, 0, 5000)), [])

    def test_one_click_after_resting(self):
        dw = D.Dweller(delay=500, tolerance=10, kind='right')
        dw.move(100, 100, 0)
        dw.move(300, 200, 100)
        evs = self.rest(dw, 100, 3000)
        rings = [e for e in evs if e['t'] == 'ring']
        self.assertTrue(rings and rings[0]['p'] < rings[-1]['p'] <= 1)
        self.assertEqual(clicks(evs), [{'t': 'click', 'kind': 'right', 'x': 300, 'y': 200}])
        # Resting on: never a second click in the same place.
        self.assertEqual(clicks(self.rest(dw, 3000, 9000)), [])
        # A wobble within the tolerance doesn't count as moving away.
        dw.move(305, 204, 9000)
        self.assertEqual(clicks(self.rest(dw, 9000, 12000)), [])
        # Away and resting again: the next click.
        dw.move(400, 200, 12000)
        self.assertEqual(len(clicks(self.rest(dw, 12000, 14000))), 1)

    def test_no_ring_before_the_pointer_settles(self):
        dw = D.Dweller(delay=1000, tolerance=10)
        dw.move(0, 0, 0)
        dw.move(50, 50, 1000)
        self.assertEqual(dw.tick(1000 + D.SETTLE_MS - 1), [])
        self.assertEqual(dw.tick(1000 + D.SETTLE_MS + 100)[0]['t'], 'ring')

    def test_moving_away_stops_it(self):
        dw = D.Dweller(delay=1000, tolerance=10)
        dw.move(0, 0, 0)
        dw.move(50, 50, 0)
        self.assertEqual(dw.tick(600)[0]['t'], 'ring')
        self.assertEqual(dw.move(90, 50, 700), [{'t': 'hide'}])
        self.assertFalse(dw.counting)
        # It starts again from nothing at the new place.
        self.assertEqual(clicks(self.rest(dw, 700, 1500)), [])
        self.assertEqual(len(clicks(self.rest(dw, 1500, 2500))), 1)

    def test_esc_stops_it_until_moved(self):
        dw = D.Dweller(delay=1000, tolerance=10)
        dw.move(0, 0, 0)
        dw.move(50, 50, 0)
        dw.tick(600)
        self.assertEqual(dw.cancel(), [{'t': 'hide'}])
        self.assertEqual(clicks(self.rest(dw, 600, 10000)), [])
        dw.move(200, 50, 10000)
        self.assertEqual(len(clicks(self.rest(dw, 10000, 12000))), 1)

    def test_settings_change_the_countdown(self):
        dw = D.Dweller()
        dw.configure(D.clean({'dwellDelay': 2000, 'dwellTolerance': 20, 'dwellClickType': 'double'}))
        self.assertEqual((dw.delay, dw.tolerance, dw.kind), (2000, 20, 'double'))


class Clicks(unittest.TestCase):
    class Dev:
        def __init__(self, fail_press=False):
            self.log, self.fail_press = [], fail_press

        def write(self, t, code, v):
            if v == 1 and self.fail_press:
                raise OSError('gone')
            self.log.append((code, v))

        def syn(self):
            pass

    def test_every_press_is_released(self):
        for kind, code, n in (('left', 272, 1), ('double', 272, 2), ('right', 273, 1), ('middle', 274, 1)):
            dev = self.Dev()
            D.Clicker(dev).click(kind)
            self.assertEqual(dev.log, [(code, 1), (code, 0)] * n, kind)

    def test_release_even_when_the_press_fails(self):
        dev = self.Dev(fail_press=True)
        with self.assertRaises(OSError):
            D.Clicker(dev).click('left')
        self.assertEqual(dev.log, [(272, 0)])


class Settings(unittest.TestCase):
    def test_clamped_and_off_by_default(self):
        self.assertFalse(D.clean(None)['dwellClick'])
        s = D.clean({'dwellClick': 'yes', 'dwellDelay': 5, 'dwellTolerance': 999, 'dwellClickType': 'drag'})
        self.assertEqual(s, dict(D.DEFAULTS, dwellDelay=300, dwellTolerance=40))
        a = A.clean({'dwellClick': True, 'dwellDelay': 99999, 'dwellClickType': 'middle', 'invertWindow': True})
        self.assertTrue(a['dwellClick'] and a['invertWindow'])
        self.assertEqual((a['dwellDelay'], a['dwellClickType']), (3000, 'middle'))
        self.assertFalse(A.clean(None)['dwellClick'] or A.clean(None)['invertWindow'])
        # The two helpers agree on what's allowed.
        self.assertEqual(tuple(A.DWELL_KINDS), D.KINDS)
        for k in D.DEFAULTS:
            self.assertEqual(A.DEFAULTS[k], D.DEFAULTS[k], k)

    def test_reading_writes_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'accessibility.json')
            with open(p, 'w') as f:
                f.write('{"dwellClick": true, "filter": "inv')        # damaged
            self.assertFalse(D.load_settings(p)['dwellClick'])
            # (hearth_settings_io may keep a copy of the damaged file aside; the file itself is never replaced.)
            self.assertNotIn('accessibility.json.tmp', os.listdir(d))
            with open(p) as f:
                self.assertTrue(f.read().endswith('"inv'))
            with open(p, 'w') as f:
                json.dump({'dwellClick': True, 'dwellDelay': 1500, 'filter': 'invert'}, f)
            s = D.load_settings(p)
            self.assertEqual((s['dwellClick'], s['dwellDelay']), (True, 1500))

    def test_saving_one_keeps_the_rest(self):
        with tempfile.TemporaryDirectory() as d, mock.patch.object(A, 'SETTINGS_JSON', os.path.join(d, 'a.json')), \
                mock.patch.object(A, 'mode', return_value='other'), mock.patch.object(A, 'labwc_version', return_value=None), \
                mock.patch.object(A, 'orca_running', return_value=False):
            A.set_settings(json.dumps({'filter': 'grayscale'}))
            r = A.set_settings(json.dumps({'dwellClick': True, 'dwellClickType': 'double'}))
            self.assertEqual((r['settings']['filter'], r['settings']['dwellClick']), ('grayscale', True))
            self.assertIn('dwellKinds', r['available'])
            self.assertFalse(r['available']['dwell'])          # not on KWin here


class Script(unittest.TestCase):
    def test_script(self):
        s = D.SCRIPT
        self.assertIn('workspace.cursorPosChanged.connect(onCursor)', s)
        # Esc is never the action's own key: hearth-dwell lends it only while a ring fills.
        self.assertIn('registerShortcut("HearthDwellCancel", "Hearth: cancel the dwell click", "",', s)
        self.assertIn('callDBus(BUS, OBJ, IFACE, "Pos", x + "," + y)', s)
        self.assertEqual(D.ESC_KEY, 0x01000000)

    def test_not_on_labwc(self):
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}), mock.patch('sys.stdout', new_callable=lambda: open(os.devnull, 'w')):
            self.assertEqual(D.main(['hearth-dwell', 'serve']), 4)

    def test_esc_key_given_back(self):
        calls = []

        class Err(Exception):
            pass

        class Bus:
            def call_sync(self, dest, path, iface, method, args, *rest):
                calls.append((method, args))
        GLib = types.SimpleNamespace(Variant=lambda sig, v: (sig, v), Error=Err)
        Gio = types.SimpleNamespace(DBusCallFlags=types.SimpleNamespace(NONE=0))
        k = D.EscKey(Bus(), Gio, GLib)
        k.hold(True)
        k.hold(True)                                   # once is enough
        k.hold(False)
        self.assertEqual([c[0] for c in calls], ['setForeignShortcutKeys'] * 2)
        self.assertEqual(calls[0][1][1][1], [([D.ESC_KEY, 0, 0, 0],)])
        self.assertEqual(calls[1][1][1][1], [])


class InvertOneWindow(unittest.TestCase):
    def fake_kwin(self, vals, running=False):
        writes, shortcuts = [], []

        class Fail(Exception):
            pass

        def update(name, changes):
            writes.append(changes)
            for g, kv in changes.items():
                for key, v in kv.items():
                    vals.setdefault(g, {})[key] = 'true' if v is True else 'false' if v is False else str(v)
        k = types.SimpleNamespace(
            EFFECTS={'invert': False, 'colorblindnesscorrection': False}, Fail=Fail,
            kc=types.SimpleNamespace(effective=lambda name: (vals, {}, {}),
                                     to_text=lambda v: 'true' if v is True else 'false' if v is False else str(v)),
            write_changes=update, reconfigure=lambda: True, kwin_running=lambda: running,
            shortcut=lambda n: shortcuts.append(n), call=lambda *a: (0,))
        return k, writes, shortcuts

    def test_effect_kept_for_one_window_then_unloaded(self):
        vals = {}
        k, writes, _ = self.fake_kwin(vals)
        with mock.patch.object(A, 'kwin_tool', return_value=k), mock.patch.object(A, 'kwin_version_tuple', return_value=(6, 3)):
            self.assertIsNone(A.apply_filter_kwin('none', 1.0, True))
            self.assertEqual(vals['Plugins']['invertEnabled'], 'true')
            n = len(writes)
            self.assertIsNone(A.apply_filter_kwin('none', 1.0, True))
            self.assertEqual(len(writes), n)                       # nothing written when nothing changed
            self.assertIsNone(A.apply_filter_kwin('deuteranopia', 1.0, True))
            self.assertEqual(vals['Plugins']['invertEnabled'], 'true')
            self.assertIsNone(A.apply_filter_kwin('none', 1.0, False))
            self.assertEqual(vals['Plugins']['invertEnabled'], 'false')

    def test_screen_switched_back_before_unloading(self):
        vals = {'Plugins': {'invertEnabled': 'true'}}
        k, writes, shortcuts = self.fake_kwin(vals, running=True)
        with tempfile.TemporaryDirectory() as d, mock.patch.dict(os.environ, {'XDG_RUNTIME_DIR': d}), \
                mock.patch.object(A, 'kwin_tool', return_value=k), mock.patch.object(A, 'kwin_version_tuple', return_value=(6, 3)), \
                mock.patch.object(A, 'kwin_pid', return_value=42):
            with open(A.kwin_invert_mark(), 'w') as f:
                f.write('42')
            order = []
            k.shortcut = lambda n: order.append(('shortcut', n, vals['Plugins'].get('invertEnabled')))
            self.assertIsNone(A.apply_filter_kwin('none', 1.0, False))
            self.assertEqual(order, [('shortcut', 'invert', 'true')])
            self.assertEqual(vals['Plugins']['invertEnabled'], 'false')

    def test_shortcut_name(self):
        K = load(SHELL / 'hearth-kwin-settings', 'hearth_kwin_settings_dwell_test')
        self.assertEqual(K.SHORTCUTS['invert-window'], 'InvertWindow')


class Page(unittest.TestCase):
    def test_rows(self):
        q = (SHELL / 'SetAccessibility.qml').read_text()
        for bit in ('"Invert one window"', 'invertWindow: v', '"ToggleKeysBeep"', '"StickyKeysBeep"', '"Toggle keys"',
                    'SetHeader { text: "Dwell click" }', 'dwellClick: v', 'dwellDelay:', 'dwellClickType: v', 'dwellTolerance: v'):
            self.assertIn(bit, q)
        shell = (SHELL / 'shell.qml').read_text()
        self.assertIn('Loader { source: "Dwell.qml"', shell)
        d = (SHELL / 'Dwell.qml').read_text()
        self.assertIn('mask: Region {}', d)
        self.assertIn('WlrKeyboardFocus.None', d)
        self.assertIn('"release"', d)
        self.assertTrue(os.access(SHELL / 'hearth-dwell', os.X_OK))


if __name__ == '__main__':
    unittest.main()
