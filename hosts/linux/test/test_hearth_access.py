"""Tests for hearth-access (Settings › Accessibility in both modes): settings checks, the labwc magnifier block and
environment lines hearth-system puts in Desktop Mode's labwc folder, the colour filters' 3D LUTs, and what's set on
gamescope's root window in Game Mode (with a fake runner)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LABWC = ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


A = load(SHELL / 'hearth-access', 'hearth_access_test')


class Settings(unittest.TestCase):
    def test_defaults_and_bad_values(self):
        self.assertEqual(A.clean(None), A.DEFAULTS)
        s = A.clean({'filter': 'sepia', 'magnifierZoom': 99, 'gameZoom': 'big', 'textScale': 1.27, 'screenReader': 'yes',
                     'magnifierSize': 'lens', 'highContrast': True})
        self.assertEqual(s['filter'], 'none')
        self.assertEqual(s['magnifierZoom'], 8.0)
        self.assertEqual(s['gameZoom'], 1.0)
        self.assertEqual(s['textScale'], 1.3)
        self.assertFalse(s['screenReader'])
        self.assertEqual(s['magnifierSize'], 'lens')
        self.assertTrue(s['highContrast'])
        self.assertEqual(A.clean({'magnifierStep': True})['magnifierStep'], A.DEFAULTS['magnifierStep'])

    def test_set_saves_and_reports(self):
        with tempfile.TemporaryDirectory() as d, mock.patch.object(A, 'SETTINGS_JSON', os.path.join(d, 'a.json')), \
                mock.patch.object(A, 'mode', return_value='other'), mock.patch.object(A, 'labwc_version', return_value=(0, 9, 2)), \
                mock.patch.object(A, 'orca_running', return_value=False):
            r = A.set_settings(json.dumps({'filter': 'grayscale', 'gameZoom': 2}))
            self.assertEqual(r['settings']['filter'], 'grayscale')
            self.assertEqual(r['settings']['gameZoom'], 2.0)
            self.assertTrue(r['available']['labwcMagnifier'])
            self.assertEqual(json.loads(pathlib.Path(d, 'a.json').read_text())['filter'], 'grayscale')
            with self.assertRaises(A.Fail):
                A.set_settings('[1]')

    def test_old_labwc_has_no_magnifier(self):
        with mock.patch.object(A, 'labwc_version', return_value=(0, 7, 2)):
            self.assertFalse(A.available()['labwcMagnifier'])


class Labwc(unittest.TestCase):
    RC = (LABWC / 'rc.xml').read_text()

    def test_magnifier_block(self):
        rc = A.rc_with_access(self.RC, A.clean({'magnifierSize': 'lens', 'magnifierZoom': 3, 'magnifierStep': 0.25}))
        self.assertIn('<width>640</width>', rc)
        self.assertIn('<initScale>3.00</initScale>', rc)
        self.assertIn('<increment>0.25</increment>', rc)
        # Again: replaced, not added twice.
        rc2 = A.rc_with_access(rc, A.clean({'magnifierSize': 'full'}))
        self.assertEqual(rc2.count('<magnifier>'), 1)
        self.assertIn('<width>-1</width>', rc2)
        import xml.dom.minidom
        xml.dom.minidom.parseString(rc2)

    def test_keys(self):
        for k in ('W-equal', 'W-minus', 'W-Escape', 'W-A-s', 'W-period'):
            self.assertIn('key="%s"' % k, self.RC)

    def test_text_size_scales_titles(self):
        rc = A.rc_with_access(self.RC, A.clean({'textScale': 1.5}))
        self.assertIn('<font place="ActiveWindow"><name>sans</name><size>15</size>', rc)
        self.assertIn('<font place="ActiveWindow"><name>sans</name><size>10</size>', A.rc_with_access(self.RC, A.clean({})))

    def test_environment(self):
        self.assertEqual(A.env_lines(A.clean({})), '')
        e = A.env_lines(A.clean({'textScale': 1.3, 'highContrast': True, 'largePointer': True, 'screenReader': True}))
        for line in ('GDK_DPI_SCALE=1.30', 'QT_FONT_DPI=125', 'GTK_THEME=HighContrastInverse', 'XCURSOR_SIZE=48',
                     'QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1'):
            self.assertIn(line + '\n', e)

    def test_hearth_system_folder_has_it(self):
        system = load(SHELL / 'hearth-system', 'hearth_system_access_test')
        with tempfile.TemporaryDirectory() as d:
            cfg = os.path.join(d, 'cfg')
            os.makedirs(os.path.join(cfg, 'nimbus'))
            with open(os.path.join(cfg, 'nimbus', 'accessibility.json'), 'w') as f:
                json.dump({'magnifierSize': 'lens', 'largePointer': True}, f)
            with mock.patch.dict(os.environ, {'XDG_CONFIG_HOME': cfg}), \
                    mock.patch.object(system, 'LABWC_ENV', os.path.join(cfg, 'labwc', 'environment')), \
                    mock.patch.object(system, 'KEYBOARD_JSON', os.path.join(cfg, 'nimbus', 'keyboard.json')), \
                    mock.patch.object(system, 'POINTER_JSON', os.path.join(cfg, 'nimbus', 'pointer.json')):
                out = system.labwc_config(str(LABWC), os.path.join(d, 'rt'))
            rc = pathlib.Path(out, 'rc.xml').read_text()
            env = pathlib.Path(out, 'environment').read_text()
            self.assertIn('<width>640</width>', rc)
            self.assertTrue(env.rstrip().endswith('XCURSOR_SIZE=48'))


class Filters(unittest.TestCase):
    def test_colours(self):
        self.assertEqual(A.filter_color('invert', 1, 0, 0.25), (0, 1, 0.75))
        r, g, b = A.filter_color('grayscale', 1, 0, 0)
        self.assertAlmostEqual(r, g)
        self.assertAlmostEqual(g, b)
        for name in ('protanopia', 'deuteranopia', 'tritanopia', 'invert-brightness'):
            for c in ((0, 0, 0), (1, 1, 1), (1, 0, 0), (0.2, 0.7, 0.4)):
                out = A.filter_color(name, *c)
                self.assertTrue(all(0 <= x <= 1 for x in out), (name, c, out))
        # Grey stays grey through a colour-blindness filter (nothing to correct).
        r, g, b = A.filter_color('deuteranopia', .5, .5, .5)
        self.assertAlmostEqual(r, .5, places=2)
        self.assertAlmostEqual(g, .5, places=2)

    def test_cube_file(self):
        t = A.cube_text('grayscale', size=5).splitlines()
        self.assertIn('LUT_3D_SIZE 5', t)
        rows = [l for l in t if l and l[0].isdigit()]
        self.assertEqual(len(rows), 125)
        self.assertEqual(rows[-1], '1.000000 1.000000 1.000000')

    def test_looks_prefer_valve(self):
        with tempfile.TemporaryDirectory() as d:
            valve = os.path.join(d, 'valve')
            os.makedirs(valve)
            open(os.path.join(valve, 'cb_prot_g22.cube'), 'w').close()
            open(os.path.join(valve, 'cb_prot_pq.cube'), 'w').close()
            with mock.patch.object(A, 'VALVE_LOOKS', [valve]), mock.patch.object(A, 'LOOKS_DIR', os.path.join(d, 'own')):
                self.assertEqual(A.look_paths('protanopia'), (os.path.join(valve, 'cb_prot_g22.cube'), os.path.join(valve, 'cb_prot_pq.cube')))
                g22, pq = A.look_paths('grayscale')
                self.assertEqual(g22, pq)
                self.assertTrue(g22.startswith(os.path.join(d, 'own')) and os.path.getsize(g22) > 1000)

    def test_game_mode_sets_root_properties(self):
        calls = []
        with tempfile.TemporaryDirectory() as d, mock.patch.object(A, 'run', side_effect=lambda c, **k: (calls.append(c), (0, ''))[1]), \
                mock.patch.object(A, 'VALVE_LOOKS', []), mock.patch.object(A, 'LOOKS_DIR', d), \
                mock.patch.dict(os.environ, {'DISPLAY': ':1'}), mock.patch.object(A.shutil, 'which', return_value='/usr/bin/xprop'):
            self.assertIsNone(A.apply_filter_game('invert'))
            self.assertIsNone(A.apply_zoom_game(2.0))
            self.assertIsNone(A.apply_filter_game('none'))
        self.assertEqual(calls[0][:6], ['xprop', '-root', '-f', 'GAMESCOPE_COLOR_LOOK_G22', '8s', '-set'])
        self.assertIn(['xprop', '-root', '-f', 'STEAM_SCREEN_MAGNIFICATION', '32c', '-set', 'STEAM_SCREEN_MAGNIFICATION', str(2 * 0xFFFF)], calls)
        self.assertIn(['xprop', '-root', '-remove', 'GAMESCOPE_COLOR_LOOK_PQ'], calls)

    def test_desktop_only_inverts(self):
        with mock.patch.object(A, 'stop_gamma'):
            self.assertIn('Game Mode', A.apply_filter_desktop('grayscale'))
            self.assertIsNone(A.apply_filter_desktop('none'))


if __name__ == '__main__':
    unittest.main()
