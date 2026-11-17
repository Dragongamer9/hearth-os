"""Mantel on KWin: the window list bridge (desktop/shell/hearth-kwin-bridge), KWin's hearthwindows script
(desktop/kwin/windows) and WinList.qml. Run: python3 -m unittest discover -s hosts/linux/test -p 'test_kwin_windows.py'
The real thing (KWin, the shell, apps) runs in kwin-windows.yml (tools/e2e-kwin-windows.py)."""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
import importlib.util
import json
import math
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

    def test_kwin_only_keys(self):
        mine = {qt_parts(s['keys']) for s in self.table['shortcuts']}
        ids = {s['id'] for s in self.table['shortcuts']}
        for s in self.table.get('kwinOnly', []):
            self.assertNotIn(qt_parts(s['keys']), mine, s['id'] + ' takes a key Mantel already uses')
            self.assertNotIn(s['id'], ids)
            self.assertTrue(('run' in s) != ('win' in s), s['id'] + ': a command or a window action, one of them')
            if 'run' in s:
                self.assertTrue(s['run'].startswith("sh -c 'quickshell ipc"), s['id'])
                self.assertEqual(kb.command_for(s['id'], self.table), s['run'])
            else:
                base = s['win'].split(':')[0] + (':' if ':' in s['win'] else '')
                self.assertIn('"%s"' % base, (KWIN / 'hearth-windows.js').read_text(), s['id'] + ': the script has no ' + base)
        self.assertIn('Hearth activities-next', kb.render().replace('"activities-next"', 'Hearth activities-next'))

    def test_window_actions_are_known(self):
        src = (KWIN / 'hearth-windows.js').read_text()
        for s in self.table['shortcuts']:
            w = s.get('win')
            if not w:
                continue
            base = w.split(':')[0] + (':' if ':' in w else '')
            self.assertIn('"%s"' % base, src, s['id'] + ': the script has no ' + base)

    def test_kde_keys_table(self):
        for e in self.table['kdeKeys']:
            for k in e['keys']:
                kb.qt_key(k)                          # every key has a Qt code
            self.assertTrue(e['why'], e['action'])
        self.assertEqual(kb.qt_key('Meta+Left'), 0x10000000 | 0x01000012)
        self.assertEqual(kb.qt_key('Meta+Shift+Backtab'), 0x12000000 | 0x01000002)
        self.assertEqual(kb.qt_key('Meta+0'), 0x10000030)
        self.assertIn('Meta[$i]=org.hearth.Windows,/org/hearth/Windows,org.hearth.Windows,Launcher', (KWIN / 'xdg' / 'kwinrc').read_text())
        self.assertFalse((KWIN / 'xdg' / 'kglobalshortcutsrc').exists(), 'kglobalaccel never reads a default file')

    def test_kwin_views_and_force_close(self):
        """KWin's Overview, Present windows and Desktop grid, and force closing, each have a Hearth key that KDE's own
        same key is freed for, and the script knows each view's name."""
        only = {s['id']: s for s in self.table['kwinOnly']}
        for sid, keys in (('overview', 'Meta+W'), ('present-windows', 'Ctrl+F10'), ('desktop-grid', 'Ctrl+F8'), ('kill-window', 'Ctrl+Alt+Esc')):
            self.assertEqual(only[sid]['keys'], keys, sid)
        freed = {e['action'] for e in self.table['kdeKeys'] if e['component'] == 'kwin' and e['keys'] == []}
        self.assertTrue({'Overview', 'ExposeAll', 'Kill Window'} <= freed, freed)
        src = (KWIN / 'hearth-windows.js').read_text()
        for name in ('overview', 'windows', 'grid'):
            self.assertIn('%s: "' % name, src, name)
        self.assertIn('call forceclose start', only['kill-window']['run'])
        self.assertTrue((SHELL / 'ForceClose.qml').is_file())

    def test_hearth_switcher_package(self):
        """Hearth's Alt+Tab: a KWin window switcher package, in both of KWin's folders, picked by Hearth's kwinrc."""
        base = LINUX.parent.parent / 'os' / 'system_files' / 'usr' / 'share'
        pkg = base / 'kwin' / 'tabbox' / 'org.hearth.switcher'
        meta = json.loads((pkg / 'metadata.json').read_text())
        self.assertEqual(meta['KPlugin']['Id'], 'org.hearth.switcher')
        self.assertEqual(meta['KPackageStructure'], 'KWin/WindowSwitcher')
        qml = (pkg / 'contents' / 'ui' / meta['X-Plasma-MainScript'].split('/', 1)[1]).read_text()
        for part in ('KWin.TabBoxSwitcher', 'KWin.WindowThumbnail', 'model.activate', 'model.close', 'Kirigami.Theme.highlightColor'):
            self.assertIn(part, qml)
        self.assertEqual((base / 'kwin-wayland' / 'tabbox' / 'org.hearth.switcher').resolve(), pkg.resolve())
        self.assertIn('[TabBox]\nLayoutName=org.hearth.switcher', (KWIN / 'xdg' / 'kwinrc').read_text())

    def test_previews_script(self):
        """The live previews: a declarative KWin script the bridge loads, asking the bridge for work (NextPreview)."""
        qml = (KWIN / 'hearth-previews.qml').read_text()
        for part in ('import org.kde.kwin', 'DBusCall', '"NextPreview"', 'WindowThumbnail', 'closeWindow()', 'Workspace.activeWindow = w'):
            self.assertIn(part, qml)
        self.assertEqual(kb.PREVIEW_SRC, str(KWIN / 'hearth-previews.qml'))
        self.assertIn('NextPreview', kb.XML)
        self.assertIn('HoverMax', kb.XML)

    def test_only_kwin_sends_the_list_and_takes_commands(self):
        self.assertEqual(set(kb.KWIN_ONLY), {'Update', 'Next', 'NextPreview', 'ForceClose'})

        class Owner:
            def unpack(self): return (':1.5',)

        class Bus:
            def call_sync(self, *a): return Owner()

        class Inv:
            err = None
            def return_dbus_error(self, name, msg): self.err = name
            def return_value(self, v): pass

        class GLib:
            @staticmethod
            def Variant(*a): return a
            class VariantType:
                @staticmethod
                def new(s): return s

        class Gio:
            class DBusCallFlags:
                NONE = 0

        b = object.__new__(kb.Bridge)
        b.bus, b.GLib, b.Gio, b.kwin_owner = Bus(), GLib, Gio, None
        self.assertTrue(b.from_kwin(':1.5'))
        self.assertFalse(b.from_kwin(':1.77'))
        self.assertFalse(b.from_kwin(''))
        for m in kb.KWIN_ONLY:
            inv = Inv()
            b.on_call(None, ':1.77', '/org/hearth/Windows', 'org.hearth.Windows', m, None, inv)
            self.assertEqual(inv.err, 'org.freedesktop.DBus.Error.AccessDenied', m + ' from another program is refused')

    def test_killable(self):
        import os
        self.assertFalse(kb.killable(1))
        self.assertFalse(kb.killable(os.getpid()))
        self.assertFalse(kb.killable(0))
        self.assertFalse(kb.killable('12'))
        p = subprocess.Popen(['sleep', '30'])
        try:
            self.assertTrue(kb.killable(p.pid))
        finally:
            p.kill(); p.wait()

    def test_theme_cols(self):
        import os
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, 'hearth'))
            with open(os.path.join(d, 'hearth', 'theme.json'), 'w') as f:
                json.dump({'radius': 14, 'font': '"Inter", system-ui, sans-serif',
                           'colors': {'panel': '#111111', 'text': '#eeeeee', 'accent': '#ff7a3d', 'bogus': 3}}, f)
            old = os.environ.get('XDG_CONFIG_HOME')
            os.environ['XDG_CONFIG_HOME'] = d
            try:
                c = kb.theme_cols()
            finally:
                if old is None:
                    del os.environ['XDG_CONFIG_HOME']
                else:
                    os.environ['XDG_CONFIG_HOME'] = old
        self.assertEqual(c, {'panel': '#111111', 'text': '#eeeeee', 'accent': '#ff7a3d', 'r': 14, 'font': 'Inter'})

    def test_extra_ids_and_launcher(self):
        self.assertTrue(kb.command_for('launcher', self.table).endswith("call launcher toggle'"))
        self.assertIn('call snap open', kb.command_for('snap-layouts-open', self.table))
        self.assertIsNone(kb.command_for('nope', self.table))


ML, MD, MG, F9 = 0x11000012, 0x10000044, 0x10000047, 0x11000038


class FakeKeys:
    """kglobalaccel, pretend: {action id tuple[:2]: [current, default]}."""
    def __init__(self, keys):
        self.keys = keys
        self.sets = []

    action_id = staticmethod(kb.KdeKeys.action_id)

    def get(self, aid, default=False):
        return [list(k) for k in self.keys.get(tuple(aid[:2]), [[], []])[1 if default else 0]]

    def set(self, aid, keys):
        self.sets.append((aid[1], keys))
        self.keys.setdefault(tuple(aid[:2]), [[], []])[0] = [list(k) for k in keys]


class KdeKeysFix(unittest.TestCase):
    """KDE's keys are freed once, only while on KDE's default; never a key the person set."""
    TABLE = {'kdeKeys': [{'component': 'kwin', 'action': 'Window Quick Tile Left', 'keys': [], 'why': 'Super+Left'},
                         {'component': 'kwin', 'action': 'Show Desktop', 'keys': [], 'why': 'Super+D'},
                         {'component': 'kwin', 'action': 'Grid View', 'keys': [], 'why': 'Super+G'}]}
    IDS = [('snap-left', 'Snap left'), ('game-mode', 'Game Mode'), ('show-desktop', 'Show the desktop')]

    def setUp(self):
        import os
        self.tmp = tempfile.TemporaryDirectory()
        self.old = os.environ.get('XDG_STATE_HOME')
        os.environ['XDG_STATE_HOME'] = self.tmp.name

    def tearDown(self):
        import os
        if self.old is None:
            os.environ.pop('XDG_STATE_HOME', None)
        else:
            os.environ['XDG_STATE_HOME'] = self.old
        self.tmp.cleanup()

    def fake(self):
        return FakeKeys({
            ('kwin', 'Window Quick Tile Left'): [[[ML]], [[ML]]],      # on KDE's default
            ('kwin', 'Show Desktop'): [[[F9]], [[MD]]],                 # the person moved it to Super+F9
            ('kwin', 'Grid View'): [[[MG]], [[MG]]],
            ('kwin', 'Hearth snap-left'): [[], [[ML]]],                 # lost its key to Quick Tile earlier
            ('kwin', 'Hearth game-mode'): [[[MG]], [[MG]]],
            ('kwin', 'Hearth show-desktop'): [[], [[MD]]],              # Super+D: not freed by Hearth, left alone
        })

    def test_frees_defaults_only_and_gives_hearth_its_keys(self):
        k = self.fake()
        kb.fix_kde_keys(k, self.TABLE, self.IDS, say=lambda t: None)
        self.assertEqual(k.get(['kwin', 'Window Quick Tile Left']), [])
        self.assertEqual(k.get(['kwin', 'Show Desktop']), [[F9]], 'the person\'s own key was changed')
        self.assertEqual(k.get(['kwin', 'Grid View']), [])
        self.assertEqual(k.get(['kwin', 'Hearth snap-left']), [[ML]])
        self.assertEqual(k.get(['kwin', 'Hearth show-desktop']), [], 'Hearth took a key it never freed')

    def test_once_only_and_restore(self):
        k = self.fake()
        kb.fix_kde_keys(k, self.TABLE, self.IDS, say=lambda t: None)
        # The person gives Quick Tile Left a key again: the next start leaves it.
        k.keys[('kwin', 'Window Quick Tile Left')][0] = [[ML]]
        k.sets.clear()
        kb.fix_kde_keys(k, self.TABLE, self.IDS, say=lambda t: None)
        self.assertEqual(k.sets, [])
        back = kb.restore_kde_keys(k)
        self.assertIn('kwin/Grid View', back)
        self.assertEqual(k.get(['kwin', 'Grid View']), [[MG]])
        k.sets.clear()
        kb.fix_kde_keys(k, self.TABLE, self.IDS, say=lambda t: None)
        self.assertEqual(k.sets, [], 'restored keys were changed again')


    def test_settings_button_puts_kdes_keys_back(self):
        # Settings › Keyboard › Bring back KDE's own shortcuts (review, Nov 17): what changed is read without D-Bus
        # or writing; a key that can't be put back now is tried again next time, never marked as done.
        self.assertEqual(kb.changed_kde_keys(), {'changed': [], 'count': 0})
        k = self.fake()
        kb.fix_kde_keys(k, self.TABLE, self.IDS, say=lambda t: None)
        ch = kb.changed_kde_keys()
        self.assertEqual(sorted(c['key'] for c in ch['changed']), ['kwin/Grid View', 'kwin/Window Quick Tile Left'])
        real = k.set

        def flaky(aid, keys):
            if aid[1] == 'Grid View':
                raise RuntimeError('kglobalaccel is not there')
            real(aid, keys)
        k.set = flaky
        back = kb.restore_kde_keys(k)
        self.assertIn('kwin/Window Quick Tile Left', back)
        self.assertNotIn('kwin/Grid View', back)
        self.assertEqual([c['key'] for c in kb.changed_kde_keys()['changed']], ['kwin/Grid View'])
        k.set = real
        k.sets.clear()
        self.assertEqual(kb.restore_kde_keys(k), ['kwin/Grid View'], 'only what is still to put back')
        self.assertEqual(k.get(['kwin', 'Grid View']), [[MG]])
        self.assertEqual(kb.changed_kde_keys()['count'], 0)
        page = (LINUX / 'desktop' / 'shell' / 'SetKeyboard.qml').read_text()
        self.assertIn('"Bring back KDE\'s own shortcuts"', page)
        self.assertIn('["keys", "restore", "--json"]', page)
        self.assertIn("Settings › Keyboard › Bring back KDE", (LINUX / 'desktop' / 'shell' / 'hearth-kwin-bridge').read_text())


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

    def test_picture_in_picture_rule(self):
        """Picture-in-picture windows stay on top under KWin (Wayland apps can't ask for that): a default rule for
        Firefox's and Chromium's (and their relatives'), matched by title and app (kwin-windows-mock.js runs it)."""
        rules = kb.load_table()['rules']
        pip = [r for r in rules if r.get('id') == 'picture-in-picture']
        self.assertEqual(len(pip), 1)
        r = pip[0]
        self.assertTrue(r['keepAbove'])
        self.assertEqual({t.lower() for t in r['titles']}, {'picture-in-picture', 'picture in picture'})
        for app in ('firefox', 'firefox-esr', 'org.mozilla.firefox', 'chromium', 'google-chrome', 'brave-browser'):
            self.assertIn(app, r['apps'])
        self.assertIn('"titles": ["Picture-in-Picture", "Picture in picture"]', kb.render())
        self.assertIn('Task Manager (always on top)', [x.get('title') for x in rules])

    @unittest.skipUnless(shutil.which('node'), 'node is needed to run the script against a pretend KWin')
    def test_script_against_pretend_kwin(self):
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / 'hearth-windows.js'
            p.write_text(kb.render())
            subprocess.run(['node', '--check', str(p)], check=True)
            r = subprocess.run(['node', str(LINUX / 'test' / 'kwin-windows-mock.js'), str(p)], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


class Frames(unittest.TestCase):
    """Hearth's window frames on KWin (theme/hearth_aurorae.py), made like labwc's."""

    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(LINUX / 'desktop' / 'theme'))
        import hearth_aurorae
        cls.ha = hearth_aurorae
        cls.cols = hearth_aurorae.themerc_colors((LINUX / 'desktop' / 'labwc' / 'themerc-override').read_text())

    def test_labwcs_colours(self):
        for k in ('window.active.title.bg.color', 'window.inactive.title.bg.color', 'window.active.label.text.color',
                  'window.active.button.hover.bg.color', 'window.active.border.color'):
            self.assertIn(k, self.cols)
        files = self.ha.build(self.cols, 10)
        self.assertIn(self.cols['window.active.title.bg.color'], files['decoration.svg'])
        self.assertIn(self.cols['window.inactive.title.bg.color'], files['decoration.svg'])
        self.assertIn('ActiveTextColor=', files['hearthrc'])

    def test_aurorae_files(self):
        files = self.ha.build(self.cols, 10)
        self.assertEqual(set(files), {'metadata.desktop', 'hearthrc', 'decoration.svg', 'close.svg', 'minimize.svg',
                                      'maximize.svg', 'restore.svg'})
        dec = ET.fromstring(files['decoration.svg'])
        ids = {e.get('id') for e in dec.iter() if e.get('id')}
        for part in ('topleft', 'top', 'topright', 'left', 'center', 'right', 'bottomleft', 'bottom', 'bottomright'):
            self.assertIn('decoration-' + part, ids)
            self.assertIn('decoration-inactive-' + part, ids)
        for b in ('close', 'minimize', 'maximize', 'restore'):
            ids = {e.get('id') for e in ET.fromstring(files[b + '.svg']).iter() if e.get('id')}
            self.assertEqual(ids, {'active-center', 'inactive-center', 'hover-center', 'pressed-center', 'deactivated-center'}, b)

    def _parts(self, svg):
        """{id: (x, y, w, h)} of decoration.svg's parts (each group's first rect is its size, as KSvg reads it)."""
        out = {}
        for g in ET.fromstring(svg).iter('{http://www.w3.org/2000/svg}g'):
            r = g.find('{http://www.w3.org/2000/svg}rect')
            out[g.get('id')] = tuple(float(r.get(k)) for k in ('x', 'y', 'width', 'height'))
        return out

    def _group_text(self, svg, gid):
        m = re.search(r'<g id="%s">(.*?)</g>' % gid, svg)
        return m.group(1) if m else ''

    def test_rounded_bottom_corners_and_outline(self):
        """1.14: all four corners rounded (the bottom ones as far as the window's own square corner fits inside the
        frame), the sides and bottom SIDE px thick, and a light 1 px outline all round, like Plasma's."""
        ha = self.ha
        self.assertEqual(ha.SIDE, 4)
        rc = ha.build(self.cols, 10)['hearthrc']
        for k in ('BorderLeft', 'BorderRight', 'BorderBottom'):
            self.assertIn('%s=%d\n' % (k, ha.SIDE), rc)
        self.assertIn('BorderTop=0\n', rc)
        # The bottom radius: the look's, but never so big that the window's square corner pokes out of the frame.
        for radius in (0, 2, 4, 10, 13, 16, 40):
            rb = ha.bottom_radius(radius)
            self.assertLessEqual(rb, min(radius, 16))
            if rb > ha.SIDE:
                self.assertLessEqual((rb - ha.SIDE) * math.sqrt(2), rb, radius)
        self.assertEqual(ha.bottom_radius(10), 10)
        self.assertEqual(ha.bottom_radius(0), 0)                     # a square look stays square
        # The outline: the border colour lifted towards the title text, more on the window in front; drawn round
        # every part's outside edge (the top, both sides, the bottom and all four corners).
        c = {'active.border': '#3b2e26', 'active.text': '#f4ece3', 'inactive.border': '#261d18', 'inactive.text': '#a69484'}
        lum = lambda h: sum(int(h[i:i + 2], 16) for i in (1, 3, 5))
        a, i = ha.outline_color(c, 'active'), ha.outline_color(c, 'inactive')
        self.assertGreater(lum(a), lum(c['active.border']))
        self.assertGreater(lum(i), lum(c['inactive.border']))
        self.assertGreater(lum(a) - lum(c['active.border']), lum(i) - lum(c['inactive.border']))
        files = ha.build(self.cols, 10)
        svg = files['decoration.svg']
        full = {'active.border': self.cols['window.active.border.color'], 'active.text': self.cols['window.active.label.text.color'],
                'inactive.border': self.cols['window.inactive.border.color'], 'inactive.text': self.cols['window.inactive.label.text.color']}
        for pre, st in (('decoration-', 'active'), ('decoration-inactive-', 'inactive')):
            want = ha.outline_color(full, st)
            for part in ('topleft', 'top', 'topright', 'left', 'right', 'bottomleft', 'bottom', 'bottomright'):
                self.assertIn('fill="%s"' % want, self._group_text(svg, pre + part), (pre, part))
        # A maximized window keeps no outline (flat, edge to edge).
        self.assertNotIn(ha.outline_color(full, 'active'), self._group_text(svg, 'decoration-maximized-top'))

    def test_shadow_like_labwcs(self):
        """A soft drop shadow round the frame, as labwc's (60 px at 38 % in front, 40 px at 25 % behind, no offset),
        in Aurorae's padding; the rounded corners still fit."""
        ha = self.ha
        self.assertEqual(ha.SHADOW['active'], (60, 0x60 / 255.0))
        self.assertEqual(ha.SHADOW['inactive'], (40, 0x40 / 255.0))
        P = ha.PAD
        self.assertEqual(P, 42)
        # Darkest at the frame's edge, fading to nothing by the padding's edge; the window in front's is darker.
        for st in ('active', 'inactive'):
            a = [ha.shadow_alpha(st, d) for d in range(P + 1)]
            self.assertTrue(all(x >= y for x, y in zip(a, a[1:])), st)
            self.assertLess(a[-1], 0.005, st)
        self.assertGreater(ha.shadow_alpha('active', 0), ha.shadow_alpha('inactive', 0))
        self.assertAlmostEqual(ha.shadow_alpha('active', 0), 0.376 * math.exp(-4.5 * 0.09), places=3)
        self.assertEqual(ha.shadow_alpha('inactive', 28), 0.0)            # 40 px, 30 % of it under the window
        for radius, R in ((10, 10), (3, 3), (0, 1), (40, 16)):
            files = ha.build(self.cols, radius)
            parts = self._parts(files['decoration.svg'])
            for pre in ('decoration-', 'decoration-inactive-'):
                self.assertEqual(parts[pre + 'topright'][2:], (max(R, ha.SIDE) + P, P + ha.TITLE_H))
                self.assertEqual(parts[pre + 'top'][3], P + ha.TITLE_H)
                K, RB = max(R, ha.SIDE), min(radius, ha.BOTTOM_R_MAX)
                KB = max(RB, ha.SIDE)
                self.assertEqual(parts[pre + 'topleft'][2:], (P + K, P + ha.TITLE_H), (pre, radius))
                self.assertEqual(parts[pre + 'left'][2], P + K)
                self.assertEqual(parts[pre + 'right'][2], P + K)
                self.assertEqual(parts[pre + 'bottom'][3], P + KB)
                self.assertEqual(parts[pre + 'bottomleft'][2:], (P + K, P + KB))
                self.assertEqual(parts[pre + 'bottomright'][2:], (P + K, P + KB))
            # The rounded corners are still there (an arc of the radius in the title colour), the bottom ones too.
            self.assertIn('A %d %d 0 0 1' % (R, R), files['decoration.svg'])
            if RB:
                self.assertIn('A %d %d 0 0 1' % (RB, RB), self._group_text(files['decoration.svg'], 'decoration-bottomleft'))
                self.assertIn('A %d %d 0 0 1' % (RB, RB), self._group_text(files['decoration.svg'], 'decoration-inactive-bottomright'))
            # Nothing reaches past its part (no strokes, every rect inside its part's size): KSvg would shrink the part
            # and leave a light seam where it meets the next one.
            self.assertNotIn('stroke', files['decoration.svg'])
            for g in ET.fromstring(files['decoration.svg']).iter('{http://www.w3.org/2000/svg}g'):
                rects = g.findall('{http://www.w3.org/2000/svg}rect')
                x, y, w, h = (float(rects[0].get(k)) for k in ('x', 'y', 'width', 'height'))
                for rr in rects[1:]:
                    rx, ry, rw, rh = (float(rr.get(k)) for k in ('x', 'y', 'width', 'height'))
                    self.assertTrue(rx >= x and ry >= y and rx + rw <= x + w and ry + rh <= y + h, (g.get('id'), radius))
        files = ha.build(self.cols, 10)
        svg = files['decoration.svg']
        root = ET.fromstring(svg)
        ns = '{http://www.w3.org/2000/svg}'
        grads = {g.get('id'): g for g in root.iter() if g.tag in (ns + 'linearGradient', ns + 'radialGradient')}
        self.assertTrue(grads)
        first = lambda gid: float(grads[gid].findall(ns + 'stop')[0].get('stop-opacity'))
        last = lambda gid: float(grads[gid].findall(ns + 'stop')[-1].get('stop-opacity'))
        self.assertAlmostEqual(first('s0t'), ha.shadow_alpha('active', 0), places=3)
        self.assertAlmostEqual(first('s1t'), ha.shadow_alpha('inactive', 0), places=3)
        for gid in grads:
            self.assertLess(last(gid), 0.005, gid)
            self.assertTrue(all(s.get('stop-color') == '#000000' for s in grads[gid].findall(ns + 'stop')), gid)
        # Every shadow is the same distance all round: no offset (labwc's sits straight behind the window).
        for side in ('t', 'b', 'l', 'r'):
            stops = [float(s.get('stop-opacity')) for s in grads['s0' + side].findall(ns + 'stop')]
            self.assertEqual(stops, [float(s.get('stop-opacity')) for s in grads['s0t'].findall(ns + 'stop')], side)
        # Aurorae's padding is the shadow's room, and it's switched on.
        rc = files['hearthrc']
        for k in ('PaddingTop', 'PaddingBottom', 'PaddingLeft', 'PaddingRight'):
            self.assertIn('%s=%d\n' % (k, P), rc)
        self.assertIn('Shadow=true\n', rc)
        self.assertNotIn('Shadow=false', rc)
        for k in ('TitleEdgeTopMaximized', 'TitleEdgeBottomMaximized', 'TitleEdgeLeftMaximized', 'TitleEdgeRightMaximized'):
            self.assertIn(k + '=0\n', rc)
        self.assertIn('X-Hearth-Frames=1', files['metadata.desktop'])

    def test_maximized_no_shadow(self):
        """A maximized window: flat title colour, no shadow, no border (KWin leaves the padding out)."""
        files = self.ha.build(self.cols, 10)
        root = ET.fromstring(files['decoration.svg'])
        groups = {g.get('id'): g for g in root.iter('{http://www.w3.org/2000/svg}g')}
        for pre, bg in (('decoration-maximized-', self.cols['window.active.title.bg.color']),
                        ('decoration-maximized-inactive-', self.cols['window.inactive.title.bg.color'])):
            for part in ('topleft', 'top', 'topright', 'left', 'center', 'right', 'bottomleft', 'bottom', 'bottomright'):
                g = groups[pre + part]
                fills = [r.get('fill') for r in g.iter() if r.get('fill') and r.get('fill-opacity') != '0']
                self.assertTrue(fills and all(f.lower() == bg.lower() for f in fills), (pre, part, fills))
                self.assertFalse(any('url(' in f for f in fills), (pre, part))

    def test_look_change_new_name(self):
        """The shadow doesn't stop each look getting its own hearth-<hex> folder, and the same look keeps its name."""
        a = self.ha.version_name(self.cols, 10)
        self.assertEqual(a, self.ha.version_name(self.cols, 10))
        self.assertNotEqual(a, self.ha.version_name(self.cols, 4))
        self.assertNotEqual(a, self.ha.version_name(dict(self.cols, **{'window.active.title.bg.color': '#123456'}), 10))

    def test_writes_only_its_own_folder(self):
        with tempfile.TemporaryDirectory() as d:
            other = pathlib.Path(d) / 'aurorae' / 'themes' / 'someone-elses'
            other.mkdir(parents=True)
            (other / 'decoration.svg').write_text('mine')
            seen = []

            def write(path, text):
                seen.append(path)
                pathlib.Path(path).parent.mkdir(parents=True, exist_ok=True)
                pathlib.Path(path).write_text(text)
                return True
            # An older look's folder of Hearth's (removed) and someone's own folder that only looks like one (kept).
            old = pathlib.Path(d) / 'aurorae' / 'themes' / 'hearth-0badc0de'
            old.mkdir(parents=True)
            (old / 'metadata.desktop').write_text('[Desktop Entry]\nX-Hearth-Frames=1\n')
            alike = pathlib.Path(d) / 'aurorae' / 'themes' / 'hearth-12345678'
            alike.mkdir(parents=True)
            (alike / 'metadata.desktop').write_text('[Desktop Entry]\nName=Mine\n')
            changed, name = self.ha.write_theme(d, self.cols, 6, write)
            self.assertTrue(changed)
            self.assertRegex(name, r'^hearth-[0-9a-f]{8}$')
            root = str(pathlib.Path(d) / 'aurorae' / 'themes')
            self.assertTrue(all(p.startswith(root + '/hearth/') or p.startswith(root + '/' + name + '/') for p in seen))
            self.assertTrue((pathlib.Path(root) / name / (name + 'rc')).is_file())
            self.assertEqual((other / 'decoration.svg').read_text(), 'mine')
            self.assertFalse(old.exists())
            self.assertTrue(alike.exists())
            # Another look: another name.
            cols2 = dict(self.cols, **{'window.active.title.bg.color': '#123456'})
            self.assertNotEqual(self.ha.write_theme(d, cols2, 6, write)[1], name)

    def test_kwinrc_pick_keeps_the_persons_choice(self):
        pick = self.ha.kwinrc_pick
        self.assertEqual(pick('', 'hearth-aa'), '[org.kde.kdecoration2]\ntheme=__aurorae__svg__hearth-aa\n')
        t = '[Windows]\nFocusPolicy=ClickToFocus\n\n[org.kde.kdecoration2]\ntheme=__aurorae__svg__hearth-0011\nButtonsOnLeft=X\n\n[Other]\na=1\n'
        out = pick(t, 'hearth-bb')
        self.assertIn('theme=__aurorae__svg__hearth-bb\n', out)
        self.assertNotIn('hearth-0011', out)
        self.assertIn('ButtonsOnLeft=X', out)
        self.assertIn('[Other]\na=1', out)
        self.assertIn('FocusPolicy=ClickToFocus', out)
        self.assertIsNone(pick('[org.kde.kdecoration2]\nlibrary=org.kde.breeze\ntheme=Breeze\n', 'hearth-bb'))
        self.assertIsNone(pick('[org.kde.kdecoration2]\ntheme=__aurorae__svg__SomeoneElse\n', 'hearth-bb'))
        self.assertIn('theme=__aurorae__svg__hearth-bb', pick('[org.kde.kdecoration2]\nlibrary=org.kde.kwin.aurorae.v2\n', 'hearth-bb'))

    def test_kwin_picks_it(self):
        rc = (KWIN / 'xdg' / 'kwinrc').read_text()
        self.assertIn('library=org.kde.kwin.aurorae', rc)
        self.assertIn('theme=__aurorae__svg__hearth', rc)
        # labwc's title bar order: icon on the left; iconify, max, close on the right.
        root = ET.parse(LINUX / 'desktop' / 'labwc' / 'rc.xml').getroot()
        self.assertEqual(root.find('theme').find('titlebar').find('layout').text, 'icon:iconify,max,close')
        self.assertIn('ButtonsOnLeft=M\n', rc)
        self.assertIn('ButtonsOnRight=IAX\n', rc)
        self.assertIn('TitlebarDoubleClickCommand=Maximize', rc)


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

    def test_desktop_menu_has_labwcs(self):
        # labwc's desktop menu (menu.xml root-menu) in Mantel's under KWin: every entry, in order, the same commands.
        root = ET.parse(LINUX / 'desktop' / 'labwc' / 'menu.xml').getroot()
        want = []
        for el in root.find("menu[@id='root-menu']"):
            if el.tag == 'separator':
                want.append(('-', None))
            else:
                act = el.find('action')
                want.append((el.get('label'), act.get('command') if act.get('name') == 'Execute' else act.get('name')))
        src = (SHELL / 'DesktopMenu.qml').read_text()
        src = src[src.index('property var entries: ['):src.index('function run(')]
        got = []
        for m in re.finditer(r'\{ (separator: true|id: "[a-z-]+", label: "([^"]+)".*?command: "((?:[^"\\]|\\.)*)")', src, re.S):
            got.append(('-', None) if m.group(1).startswith('separator') else (m.group(2), m.group(3)))
        # Terminal and Files: Mantel's own "Open a terminal here" / "Open in Files" (with their own commands).
        d = (SHELL / 'Desktop.qml').read_text()
        dup = {'Terminal': ('Open a terminal here', 'open.sh'), 'Files': ('Open in Files', 'filesApp.open')}
        for label, (theirs, how) in dup.items():
            line = next(l for l in d.splitlines() if 'label: "%s"' % theirs in l)
            self.assertIn(how, line, label)
        want = [w for w in want if w[0] not in dup]
        self.assertEqual([g[0] for g in got], [w[0] for w in want])
        for (label, cmd), (_, mine) in zip(want, got):
            if cmd == 'Reconfigure':
                self.assertIn('org.kde.KWin.reconfigure', mine)
            elif cmd:
                self.assertEqual(mine, cmd, label)
        body = d[d.index('function backgroundMenu('):]
        self.assertLess(body.index('DesktopMenu.items()'), body.index('menu.popup('))

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
