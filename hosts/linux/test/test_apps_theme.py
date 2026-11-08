"""Hearth's own apps follow the desktop's look (Settings › Appearance): theme/hearth-theme writes
~/.config/hearth/theme.css and theme.json for them, every Tauri app puts it in its windows (hosts/hearth-apptheme),
Theater (Electron) and Hearth Browser (Firefox) read the same file, the apps' style sheets use the shared tokens
instead of Hearth's old warm colours (those stay only as the fallback in :root), and KDE apps get the look's
colour scheme from kdeglobals."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
HOSTS = ROOT / 'hosts'
THEME = HOSTS / 'linux' / 'desktop' / 'theme' / 'hearth-theme'
APPTHEME = HOSTS / 'hearth-apptheme' / 'apptheme.rs'

# The Hearth apps in the Debian image with a web page for a window (os-bases/peppermint/build.sh).
APPS = ['calculator', 'camera', 'connect', 'contacts', 'disks', 'documents', 'emoji', 'fonts', 'gaming', 'maps',
        'nearby', 'office', 'paint', 'photos', 'player', 'recorder', 'share', 'terminal', 'text', 'theater', 'time',
        'tools']
TAURI = [a for a in APPS if a != 'theater']   # Theater's windows are Electron's
TOKENS = ('bg', 'bg2', 'panel', 'panel2', 'panel3', 'line', 'text', 'muted', 'faint', 'danger', 'ember', 'accent',
          'flame', 'ink', 'focus', 'glow', 'chrome', 'accentInk')
# Hearth's warm palette (hearth/dark) and the shades the apps made from it: only allowed as a fallback.
OLD = {'#0f0b09', '#17110e', '#1c1512', '#261d18', '#33271f', '#3b2e26', '#f4ece3', '#a69484', '#ff8a3d', '#2a1206',
       '#6f6055', '#2a170d', '#5a3f2c', '#4a372b', '#3a2416', '#c2410c'}


def theme(home, desktop, *args):
    os.makedirs(os.path.join(home, '.local', 'share', 'nimbus'), exist_ok=True)
    with open(os.path.join(home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
        json.dump(desktop, f)
    env = dict(os.environ, HOME=home, XDG_RUNTIME_DIR=os.path.join(home, 'rt'))
    for k in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'LABWC_PID'):
        env.pop(k, None)
    return subprocess.run([sys.executable, str(THEME), *args], env=env, capture_output=True, text=True, check=True).stdout


def ui_files(app):
    for p in sorted((HOSTS / ('hearth-' + app) / 'ui').rglob('*')):
        if p.suffix in ('.css', '.html') and not re.search(r'/(osk|vendor|words)/', str(p)):
            yield p


def outside_fallback(text):
    """The style sheet without its fallback palettes (:root blocks, custom property definitions, var() fallbacks)
    and scripts (their colours are data: chart series, paint swatches, the apps' own named themes)."""
    text = re.sub(r'<script.*?</script>', '', text, flags=re.S)
    text = re.sub(r':root(\.[\w-]+)?\s*\{[^}]*\}', '', text)
    text = re.sub(r'--[\w-]+\s*:[^;}"]*', '', text)
    text = re.sub(r'var\(--[\w-]+,\s*#[0-9a-fA-F]{3,6}\)', '', text)
    return text


class AppsTheme(unittest.TestCase):
    def test_theme_files_for_every_look(self):
        for look, mode, accent in (('steam', 'dark', ''), ('steam', 'light', ''), ('hearth', 'dark', ''),
                                   ('hearth', 'light', '#9b7bff')):
            with tempfile.TemporaryDirectory() as home:
                theme(home, {'look': look, 'mode': mode, 'accent': accent})
                d = pathlib.Path(home, '.config', 'hearth')
                css = (d / 'theme.css').read_text()
                t = json.loads((d / 'theme.json').read_text())
                self.assertEqual((t['look'], t['mode']), (look, mode))
                self.assertTrue(css.lstrip().startswith('/*') and ':root:root{' in css, css)
                self.assertIn('color-scheme:%s' % mode, css)
                for k in TOKENS:
                    self.assertRegex(t['colors'][k], r'^#[0-9a-f]{6}$', k)
                    self.assertIn('--%s:%s;' % (k, t['colors'][k]), css)
                for k in ('--r:', '--corner:', '--font:'):
                    self.assertIn(k, css)
                if accent:
                    self.assertEqual(t['colors']['accent'], accent)
                    self.assertEqual(t['colors']['focus'], accent)
                self.assertEqual('Inter' in t['font'], look == 'steam')   # palettes.json: Steam-style uses Inter
                self.assertEqual(t['radius'], 4 if look == 'steam' else 14)
                if mode == 'light':
                    self.assertGreater(int(t['colors']['bg'][1:3], 16), 0xc0)

    def test_built_in_default_is_steam_dark(self):
        default = subprocess.run([sys.executable, str(THEME), '--app-css', 'steam', 'dark'], capture_output=True,
                                 text=True, check=True).stdout
        rs = re.search(r'r##"(.*?)"##', APPTHEME.read_text(), re.S).group(1)
        self.assertEqual(rs, default, 'apptheme.rs DEFAULT_CSS: run hearth-theme --app-css steam dark')
        js = (HOSTS / 'hearth-theater' / 'electron' / 'main.js').read_text()
        m = re.search(r'const THEME_DEFAULT = (".*?");', js)
        self.assertEqual(json.loads(m.group(1)), default, "Theater's THEME_DEFAULT")

    def test_every_tauri_app_puts_the_look_in_its_windows(self):
        roots = 0
        for app in TAURI:
            for rs in (HOSTS / ('hearth-' + app) / 'src').rglob('*.rs'):
                s = rs.read_text()
                if 'tauri::Builder::default()' not in s:
                    continue
                roots += 1
                self.assertIn('tauri::Builder::default().plugin(apptheme::plugin())', s, rs)
                m = re.search(r'#\[path = "([^"]+)"\]\s*mod apptheme;', s)
                self.assertTrue(m, rs)
                self.assertEqual((rs.parent / m.group(1)).resolve(), APPTHEME, rs)
                self.assertNotIn('Color(15, 11, 9, 255)', s, '%s: use apptheme::bg()' % rs)
        self.assertGreaterEqual(roots, len(TAURI) + 2)   # Gaming has three apps (Controller Lab, Saves, Mods)

    def test_apptheme_script(self):
        s = APPTHEME.read_text()
        for needle in ('js_init_script', 'on_page_load', 'webview_windows', 'hearth-theme', 'window.hearthTheme',
                       'window.__hearthTheme', ':root:root'):
            self.assertIn(needle, s)

    def test_desktop_windows_get_the_bars_text_size(self):
        """In a desktop window the apps' text is the bar's size (desktop.css), not sized by the window's width as
        for Game Mode and TVs; Game Mode (gamescope, --fullscreen) never gets it. desktop-test.yml measures it."""
        s = APPTHEME.read_text()
        css = (HOSTS / 'hearth-apptheme' / 'desktop.css').read_text()
        self.assertIn('include_str!("desktop.css")', s)
        # Every place the look goes into a page carries the size too (the start, a page opened later, a new look).
        self.assertNotRegex(s, r'(js_init_script\(script|apply_js)\(&css\(\)\)')
        self.assertEqual(s.count('&page_css()'), 3)
        for needle in ('"--fullscreen"', 'GAMESCOPE_WAYLAND_DISPLAY', 'LABWC_PID', 'HEARTH_APP_SIZE'):
            self.assertIn(needle, s)
        bar = (HOSTS / 'linux' / 'desktop' / 'shell' / 'Bar.qml').read_text()
        bar_px = int(re.search(r'font\.pixelSize: bar\.portrait \? \d+ : (\d+)', bar).group(1))
        m = re.search(r':root:root body\{font-size:(\d+)px\}', css)
        self.assertTrue(m, 'desktop.css sets the text size on body, above the apps\' own body rules')
        self.assertEqual(int(m.group(1)), bar_px)
        rules = re.sub(r'/\*.*?\*/', '', css, flags=re.S)
        self.assertNotRegex(rules, r'vw|vh|zoom|!important')

    def test_apps_use_the_tokens_not_the_old_palette(self):
        for app in APPS:
            files = list(ui_files(app))
            self.assertTrue(files, app)
            sheets = ''.join(p.read_text() for p in files)
            self.assertRegex(sheets, r'var\(--(ember|accent)\b', app)
            self.assertIn('var(--font', sheets, '%s: its font should follow the look (Inter for Steam-style)' % app)
            for p in files:
                rest = outside_fallback(p.read_text())
                old = sorted({h.lower() for h in re.findall(r'#[0-9a-fA-F]{6}\b', rest)} & OLD)
                old += re.findall(r'rgba\(\s*255\s*,\s*138\s*,\s*61', rest)
                self.assertFalse(old, '%s: Hearth\'s old colours outside the fallback: %s' % (p.relative_to(ROOT), old))

    def test_focus_ring_follows_the_accent(self):
        for app in APPS:
            sheets = ''.join(p.read_text() for p in ui_files(app))
            rings = re.findall(r'box-shadow\s*:\s*0 0 0 3px ([^,;]+)', sheets)
            for r in rings:
                self.assertRegex(r.strip(), r'^var\(--(ember|accent|focus|danger)\)', app)

    def test_terminal_and_text_follow_the_desktop_by_default(self):
        for app in ('terminal', 'text'):
            js = (HOSTS / ('hearth-' + app) / 'ui' / 'app.js').read_text()
            self.assertIn("theme: 'hearth'", js)
            self.assertIn("'Desktop look'", js)
            self.assertIn("addEventListener('hearth-theme'", js)

    def test_browser_reads_the_look(self):
        mjs = (ROOT / 'os/system_files/usr/share/hearth-browser/firefox/hearth/hearth.sys.mjs').read_text()
        self.assertIn('"hearth", "theme.json"', mjs)
        self.assertIn('look();', mjs)

    def test_kde_apps_get_the_colour_scheme(self):
        with tempfile.TemporaryDirectory() as home:
            theme(home, {'look': 'steam', 'mode': 'dark'})
            kg = pathlib.Path(home, '.config', 'kdeglobals').read_text()
            self.assertIn('[UiSettings]\nColorScheme=Hearth\n', kg)
            self.assertIn('[Icons]\nTheme=breeze-dark', kg)
            scheme = pathlib.Path(home, '.local', 'share', 'color-schemes', 'Hearth.colors').read_text()
            self.assertIn('Name=Hearth\n', scheme)
            self.assertIn('BackgroundNormal=23,26,33', scheme.replace(' ', ''))   # Steam-style bg #171a21
            theme(home, {'look': 'steam', 'mode': 'light'})
            self.assertIn('[Icons]\nTheme=breeze\n', pathlib.Path(home, '.config', 'kdeglobals').read_text())


if __name__ == '__main__':
    unittest.main()
