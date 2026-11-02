"""Tests for Mantel's looks (Settings › Appearance): theme/hearth-theme turns Hearth's colour files into the chosen
look (Hearth or Steam-style, dark or light, any accent), and hearth-system gives labwc that look's window colours,
corner radius and font."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
DESK = ROOT / 'hosts' / 'linux' / 'desktop'
THEME = DESK / 'theme'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def run_theme(home, desktop):
    os.makedirs(os.path.join(home, '.local', 'share', 'nimbus'), exist_ok=True)
    with open(os.path.join(home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
        json.dump(desktop, f)
    env = dict(os.environ, HOME=home, XDG_RUNTIME_DIR=os.path.join(home, 'rt'))
    env.pop('XDG_CONFIG_HOME', None); env.pop('XDG_DATA_HOME', None); env.pop('LABWC_PID', None)
    out = subprocess.run([sys.executable, str(THEME / 'hearth-theme')], env=env, capture_output=True, text=True, check=True).stdout
    return dict(line.split('=', 1) for line in out.split()), pathlib.Path(home, '.config', 'nimbus', 'theme')


class HearthTheme(unittest.TestCase):
    def test_palettes_have_every_token(self):
        pal = json.loads((THEME / 'palettes.json').read_text())
        for look in ('hearth', 'steam'):
            for mode in ('dark', 'light'):
                for k in ('bg', 'bg2', 'panel', 'panel2', 'panel3', 'line', 'text', 'muted', 'danger', 'ember', 'flame', 'ink'):
                    self.assertRegex(pal[look][mode][k], r'^#[0-9a-f]{6}$', (look, mode, k))
        self.assertTrue(any(a['color'] == '#1a9fff' for a in pal['accents']))
        self.assertTrue(any(a['color'] == '#ff8a3d' for a in pal['accents']))

    def test_hearth_dark_is_hearths_own_files(self):
        with tempfile.TemporaryDirectory() as d:
            env, out = run_theme(d, {'look': 'hearth', 'mode': 'dark', 'accent': ''})
            self.assertEqual(env['HEARTH_SCHEME'], "'dark'")
            self.assertEqual((out / 'gtk4.css').read_text(), (THEME / 'gtk' / 'gtk4.css').read_text())
            self.assertEqual((out / 'hearth-qt.conf').read_text(), (THEME / 'hearth-qt.conf').read_text())

    def test_steam_look(self):
        with tempfile.TemporaryDirectory() as d:
            env, out = run_theme(d, {'look': 'steam', 'mode': 'dark', 'accent': ''})
            self.assertEqual(env['HEARTH_GNOME_ACCENT'], "'blue'")
            gtk = (out / 'gtk4.css').read_text()
            self.assertIn('@define-color window_bg_color #171a21;', gtk)
            self.assertIn('@define-color accent_bg_color #1a9fff;', gtk)
            self.assertNotIn('#ff8a3d', gtk)
            self.assertNotRegex(gtk, r'border-radius:\s*(?:[5-9]|\d\d)px')     # corners capped at 4 px
            rc = (out / 'themerc-override').read_text()
            self.assertIn('window.active.title.bg.color: #1e2633', rc)
            self.assertIn('snapping.overlay.region.bg.color: #1a9fff33', rc)      # #rrggbbaa keeps its alpha
            qt = (out / 'hearth-qt.conf').read_text()
            self.assertIn('#ff1a9fff', qt)
            self.assertIn('Name=Hearth Live', (out / 'Hearth.colors').read_text())
            self.assertEqual(json.loads((out / 'labwc.json').read_text()), {'cornerRadius': 4, 'font': 'Inter'})

    def test_light_mode_and_custom_accent(self):
        with tempfile.TemporaryDirectory() as d:
            env, out = run_theme(d, {'look': 'hearth', 'mode': 'light', 'accent': '#9b7bff'})
            self.assertEqual(env['HEARTH_SCHEME'], "'light'")
            self.assertEqual(env['HEARTH_GNOME_ACCENT'], "'purple'")
            gtk = (out / 'gtk3.css').read_text()
            self.assertIn('@define-color window_bg_color #f6efe8;', gtk)
            self.assertIn('@define-color accent_bg_color #9b7bff;', gtk)

    def test_bad_settings_fall_back_to_hearth(self):
        with tempfile.TemporaryDirectory() as d:
            _, out = run_theme(d, {'look': '_comment', 'mode': 'purple', 'accent': 'red'})
            self.assertEqual((out / 'gtk4.css').read_text(), (THEME / 'gtk' / 'gtk4.css').read_text())

    def test_kde_apps_get_it_through_kdeglobals_unless_its_yours(self):
        with tempfile.TemporaryDirectory() as d:
            run_theme(d, {'look': 'steam', 'mode': 'dark', 'accent': ''})
            kg = pathlib.Path(d, '.config', 'kdeglobals')
            self.assertIn('ColorScheme=HearthLive', kg.read_text())
            self.assertIn('DecorationFocus=26,159,255', kg.read_text())
            kg.write_text('[General]\nColorScheme=Mine\n')
            run_theme(d, {'look': 'steam', 'mode': 'light', 'accent': ''})
            self.assertEqual(kg.read_text(), '[General]\nColorScheme=Mine\n')

    def test_icon_data_matches_the_icons(self):
        r = subprocess.run([sys.executable, str(ROOT / 'tools' / 'make-icon-data.py'), '--check'])
        self.assertEqual(r.returncode, 0, 'shell/icon-data.js is out of date: run tools/make-icon-data.py')

    def test_labwc_folder_gets_the_look(self):
        with tempfile.TemporaryDirectory() as d:
            run_theme(d, {'look': 'steam', 'mode': 'dark', 'accent': ''})
            os.environ['XDG_CONFIG_HOME'] = os.path.join(d, '.config')
            try:
                system = load(DESK / 'shell' / 'hearth-system', 'hearth_system_theme_test')
                dest = system.labwc_config(str(DESK / 'labwc'), os.path.join(d, 'rt'))
            finally:
                os.environ.pop('XDG_CONFIG_HOME', None)
            self.assertEqual(os.readlink(os.path.join(dest, 'themerc-override')), os.path.join(d, '.config', 'nimbus', 'theme', 'themerc-override'))
            rc = pathlib.Path(dest, 'rc.xml').read_text()
            self.assertIn('<cornerRadius>4</cornerRadius>', rc)
            self.assertIn('<font place="ActiveWindow"><name>Inter</name>', rc)


if __name__ == '__main__':
    unittest.main()
