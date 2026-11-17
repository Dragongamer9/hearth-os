"""Mantel on KWin: the window list bridge (desktop/shell/hearth-kwin-bridge), KWin's hearthwindows script
(desktop/kwin/windows) and WinList.qml. Run: python3 -m unittest discover -s hosts/linux/test -p 'test_kwin_windows.py'
The real thing (KWin, the shell, apps) runs in kwin-windows.yml (tools/e2e-kwin-windows.py)."""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
import importlib.util
import json
import pathlib
import re
import shutil
import subprocess
import tempfile
import unittest
import xml.etree.ElementTree as ET

LINUX = pathlib.Path(__file__).resolve().parent.parent
SHELL = LINUX / 'desktop' / 'shell'
KWIN = LINUX / 'desktop' / 'kwin' / 'windows'
loader = importlib.machinery.SourceFileLoader('hearth_kwin_bridge', str(SHELL / 'hearth-kwin-bridge'))
spec = importlib.util.spec_from_loader('hearth_kwin_bridge', loader)
kb = importlib.util.module_from_spec(spec)
loader.exec_module(kb)

MOD = {'W': 'Meta', 'C': 'Ctrl', 'A': 'Alt', 'S': 'Shift'}
KEY = {'space': 'Space', 'Delete': 'Del', 'Escape': 'Esc', 'period': '.', 'equal': '=', 'plus': '+', 'minus': '-',
       'KP_Add': '+', 'KP_Subtract': '-', 'XF86AudioRaiseVolume': 'Volume Up', 'XF86AudioLowerVolume': 'Volume Down',
       'XF86AudioMute': 'Volume Mute', 'XF86PowerOff': 'Power Off', 'XF86MonBrightnessUp': 'Monitor Brightness Up',
       'XF86MonBrightnessDown': 'Monitor Brightness Down', 'XF86AudioMicMute': 'Microphone Mute',
       'XF86AudioPlay': 'Media Play', 'XF86AudioPause': 'Media Pause', 'XF86AudioNext': 'Media Next',
       'XF86AudioPrev': 'Media Previous', 'XF86AudioStop': 'Media Stop', 'XF86Sleep': 'Sleep'}


def labwc_to_qt(k):
    """labwc's "W-C-S-F1" as a set of Qt modifiers and the key, so the order of the modifiers doesn't matter."""
    *mods, key = k.split('-')
    key = KEY.get(key, key.upper() if len(key) == 1 else key)
    return frozenset(MOD[m] for m in mods), key


def qt_parts(seq):
    parts = seq.split('+')
    if seq.endswith('++'):
        parts = seq[:-2].split('+') + ['+']
    return frozenset(parts[:-1]), parts[-1]


class Shortcuts(unittest.TestCase):
    """Every labwc key binding has its KWin counterpart, with the same command."""

    @classmethod
    def setUpClass(cls):
        cls.table = kb.load_table()
        root = ET.parse(LINUX / 'desktop' / 'labwc' / 'rc.xml').getroot()
        cls.binds = {b.get('key'): b.findall('action') for b in root.find('keyboard').findall('keybind')}

    def test_every_labwc_key_is_covered(self):
        covered = {s['labwc'] for s in self.table['shortcuts']} | {s['labwc'] for s in self.table['kwinOwn']} \
            | {s['labwc'] for s in self.table['skipped']}
        missing = sorted(set(self.binds) - covered)
        self.assertEqual(missing, [], 'labwc keys with nothing under KWin (shortcuts.json): %s' % missing)
        extra = sorted(covered - set(self.binds))
        self.assertEqual(extra, [], 'shortcuts.json names labwc keys rc.xml no longer has: %s' % extra)

    def test_same_keys_and_commands(self):
        ids = set()
        for s in self.table['shortcuts']:
            self.assertNotIn(s['id'], ids, 'two shortcuts with the id ' + s['id'])
            ids.add(s['id'])
            self.assertEqual(qt_parts(s['keys']), labwc_to_qt(s['labwc']), s['id'])
            self.assertTrue(('run' in s) != ('win' in s), s['id'] + ': a command or a window action, one of them')
            if 'run' in s:
                act = self.binds[s['labwc']][0]
                self.assertEqual(act.get('name'), 'Execute', s['id'])
                self.assertEqual(s['run'], act.get('command'), s['id'] + ': not the same command as labwc')

    def test_window_actions_are_known(self):
        src = (KWIN / 'hearth-windows.js').read_text()
        for s in self.table['shortcuts']:
            w = s.get('win')
            if not w:
                continue
            base = w.split(':')[0] + (':' if ':' in w else '')
            self.assertIn('"%s"' % base, src, s['id'] + ': the script has no ' + base)

    def test_kwin_defaults_free_mantels_keys(self):
        text = (KWIN / 'xdg' / 'kglobalshortcutsrc').read_text()
        mine = {qt_parts(s['keys']) for s in self.table['shortcuts']}
        for line in text.splitlines():
            m = re.match(r'^([^#\[][^=]*)=none,([^,]+),', line)
            if m and m.group(2) != 'none':
                self.assertIn(qt_parts(m.group(2)), mine, '%s is freed but no Hearth shortcut uses %s' % (m.group(1), m.group(2)))
        self.assertIn('Meta=org.hearth.Windows,/org/hearth/Windows,org.hearth.Windows,Launcher', (KWIN / 'xdg' / 'kwinrc').read_text())

    def test_extra_ids_and_launcher(self):
        self.assertTrue(kb.command_for('launcher', self.table).endswith("call launcher toggle'"))
        self.assertIn('call snap open', kb.command_for('snap-layouts-open', self.table))
        self.assertIsNone(kb.command_for('nope', self.table))


class Script(unittest.TestCase):
    def test_render_fills_everything(self):
        src = kb.render()
        self.assertIsNone(re.search(r'%%[A-Z]+%%', src))
        self.assertIn('var SHORTCUTS = [{', src)

    def test_zones_match_snap_layouts_and_labwc(self):
        zones = {z['id']: z for z in kb.snap_zones()}
        self.assertIn('left-half', zones)
        self.assertEqual(len(zones), 13)
        root = ET.parse(LINUX / 'desktop' / 'labwc' / 'rc.xml').getroot()
        regions = {r.get('name'): r for r in root.find('regions').findall('region')}
        for zid, z in zones.items():
            if zid in regions:
                r = regions[zid]
                self.assertEqual([int(r.get(k).rstrip('%')) for k in ('x', 'y', 'width', 'height')], [z['x'], z['y'], z['w'], z['h']], zid)
        extra = {z['id']: z for z in kb.load_table()['extraZones']}
        r = regions['center']
        self.assertEqual([int(r.get(k).rstrip('%')) for k in ('x', 'y', 'width', 'height')],
                         [extra['center'][k] for k in ('x', 'y', 'w', 'h')])
        gap = int(root.find('core').find('gap').text)
        self.assertEqual(kb.load_table()['gap'], gap)

    @unittest.skipUnless(shutil.which('node'), 'node is needed to run the script against a pretend KWin')
    def test_script_against_pretend_kwin(self):
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / 'hearth-windows.js'
            p.write_text(kb.render())
            subprocess.run(['node', '--check', str(p)], check=True)
            r = subprocess.run(['node', str(LINUX / 'test' / 'kwin-windows-mock.js'), str(p)], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


class Commands(unittest.TestCase):
    def test_parse_do(self):
        self.assertEqual(kb.parse_do(['snap', '{1}', 'zone=left-half']), {'op': 'snap', 'id': '{1}', 'zone': 'left-half'})
        self.assertEqual(kb.parse_do(['goDesktop', 'n=2']), {'op': 'goDesktop', 'n': 2})
        self.assertEqual(kb.parse_do(['minimize', 'abc', 'on=true']), {'op': 'minimize', 'id': 'abc', 'on': True})


class Shell(unittest.TestCase):
    """Mantel reads its windows through WinList, so it works on labwc and KWin alike."""

    def test_no_direct_toplevel_manager(self):
        bad = []
        for f in sorted(SHELL.glob('*.qml')):
            if f.name in ('WinList.qml', 'desk-cosmic.qml'):
                continue
            if 'ToplevelManager' in f.read_text():
                bad.append(f.name)
        self.assertEqual(bad, [], 'use WinList.toplevels / WinList.activeToplevel: %s' % bad)

    def test_typed_window_keys_have_a_kwin_way(self):
        # Moving a window to a desktop and snap layouts typed labwc's keys with wtype; KWin takes no typed keys.
        for name in ('Bar.qml', 'Dock.qml', 'TaskStrip.qml'):
            src = (SHELL / name).read_text()
            body = src[src.index('function moveToDesktop(win, n) {'):]
            self.assertLess(body.index('WinList.moveToDesktop'), body.index('wtype'), name)
        src = (SHELL / 'SnapLayouts.qml').read_text()
        self.assertLess(src.index('WinList.snap(win, q.id)'), src.index('typer.command ='))
        self.assertIn('WinList.goDesktop', (SHELL / 'DeskSwitcher.qml').read_text())

    def test_winlist_names_match_toplevels(self):
        src = (SHELL / 'WinList.qml').read_text()
        for prop in ('appId', 'title', 'activated', 'minimized', 'maximized', 'fullscreen', 'screens'):
            self.assertRegex(src, r'property (string|bool|var) %s\b' % prop)
        for fn in ('activate()', 'close()'):
            self.assertIn('function ' + fn, src)
        self.assertIn('"hearth-kwin-bridge"', src)
        self.assertIn('"serve", "--stdio"', src)


if __name__ == '__main__':
    unittest.main()
