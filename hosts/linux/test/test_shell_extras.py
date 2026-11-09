"""Hearth 1.10's Windows-style extras in Mantel: the volume mixer (Mixer.qml, mixer.js), snap layouts
(SnapLayouts.qml, snap-layouts.js, labwc's rc.xml) and clipboard history (Clipboard.qml, hearth-clipboard).

Checks each app's remembered level (only when you move its slider, merged per app, put back when it plays again),
the snap zones' geometry and that labwc's regions and hidden keys match them, and the clipboard's storage (about 25
items, pins kept through Clear all, secret copies never kept, off until you turn it on). The JavaScript rules run
with node, as in test_hearth_this_pc.py."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LABWC = ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc'
NODE = shutil.which('node')


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def run_js(lib, body):
    """Runs BODY (which prints JSON with console.log) after the QML JavaScript library LIB, with node."""
    src = (SHELL / lib).read_text().replace('.pragma library', '')
    r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=30)
    if r.returncode != 0:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout.strip().splitlines()[-1])


def code(s):
    """QML/JS without // comments (so a comment can't satisfy or fail a check)."""
    return re.sub(r'(?m)(^|\s)//.*$', '', s)


# ---------------------------------------------------------------------- the volume mixer
@unittest.skipUnless(NODE, 'node is needed for the mixer rules')
class MixerRules(unittest.TestCase):
    def test_app_names(self):
        r = run_js('mixer.js', '''console.log(JSON.stringify([
            appKey({"application.name": "Firefox", "application.process.binary": "firefox", "node.name": "x"}),
            appKey({"application.process.binary": "discord"}),
            appKey({"node.name": "alsa_playback.game"}),
            appKey({}), appKey(null),
            iconName({"application.icon-name": "spotify-client", "application.process.binary": "spotify"}),
            iconName({"application.process.binary": "vlc"})]))''')
        self.assertEqual(r, ['Firefox', 'discord', 'alsa_playback.game', '', '', 'spotify-client', 'vlc'])

    def test_remembered_levels_are_cleaned(self):
        r = run_js('mixer.js', '''console.log(JSON.stringify([
            cleanRemembered({"Firefox": 0.4, "Loud": 9, "Neg": -1, "Text": "0.5", "": 0.3, "Discord": 0.654}),
            cleanRemembered([1, 2]), cleanRemembered(null), cleanRemembered("x")]))''')
        self.assertEqual(r, [{'Firefox': 0.4, 'Discord': 0.65}, {}, {}, {}])

    def test_putting_a_level_back(self):
        r = run_js('mixer.js', '''var m = {"Firefox": 0.4, "Discord": 0.65};
            console.log(JSON.stringify([restoreLevel(m, "Firefox", 1.0), restoreLevel(m, "Firefox", 0.4),
                restoreLevel(m, "Spotify", 0.3), restoreLevel(m, "", 0.3), restoreLevel({"Bad": 7}, "Bad", 1)]))''')
        self.assertEqual(r, [0.4, None, None, None, None])

    def test_what_is_saved_when_you_move_a_slider(self):
        # Just that app's level (merged into the file on its own), clamped; nothing without a name.
        r = run_js('mixer.js', '''console.log(JSON.stringify([rememberChange("Firefox", 0.456), rememberChange("Game", 4),
            rememberChange("", 0.5), step(0.98, 1, 1), step(0.02, -1), step(0.5, 1)]))''')
        self.assertEqual(r, [{'Firefox': 0.46}, {'Game': 1.5}, None, 1, 0, 0.55])

    def test_one_row_per_app(self):
        r = run_js('mixer.js', '''console.log(JSON.stringify(group([{id: 1, key: "Firefox"}, {id: 2, key: "Discord"},
            {id: 3, key: "Firefox"}, {id: 4, key: ""}]).map(g => [g.key, g.streams.map(s => s.id)])))''')
        self.assertEqual(r, [['Firefox', [1, 3]], ['Discord', [2]], ['stream 4', [4]]])

    def test_rows_with_the_keys(self):
        r = run_js('mixer.js', 'console.log(JSON.stringify([moveRow(0, -1, 3), moveRow(1, 1, 3), moveRow(4, 1, 3), moveRow(1, 1, 0)]))')
        self.assertEqual(r, [0, 2, 4, 1])


class MixerWiring(unittest.TestCase):
    def setUp(self):
        self.q = code((SHELL / 'Mixer.qml').read_text())

    def test_saved_only_when_you_move_a_slider(self):
        q = self.q
        # The file is written only through remember(), only from setVol(..., true), only from a slider or Left/Right.
        self.assertEqual(len(re.findall(r'writer\.merge\(', q)), 1)
        self.assertIn('if (save) remember(key, v);', q)
        calls = re.findall(r'setVol\([^;]*?, true\)', q)
        self.assertEqual(len(calls), 2, calls)
        self.assertIn('onMoved: v => { mx.current = app.index + 2; mx.setVol(app.modelData, v, true); }', q)
        self.assertIn('setVol(k, M.step(volOf(k), d), true)', q)
        # Opening, reading the file and putting a level back never save.
        for fn in ('function open', 'function openNear', 'function tryRestore', 'onLoaded'):
            i = q.index(fn)
            body = q[i:q.index('\n    }', i)]
            self.assertNotIn('remember(', body, fn)
            self.assertNotIn('merge(', body, fn)
        self.assertIn('SettingsWriter { id: writer; path: mx.memPath }', q)
        self.assertIn('/.config/nimbus/app-volumes.json', q)
        self.assertIn('writer.repair()', q)          # a damaged file is put back, never replaced

    def test_restore_each_stream_once(self):
        q = self.q
        self.assertIn('if (mx.fix || !memRead || !n || !n.audio || handled[n.id]) return;', q)
        self.assertIn('M.restoreLevel(remembered, M.appKey(n.properties || {}), n.audio.volume)', q)

    def test_controller_and_entry_points(self):
        q = self.q
        for k in ('Qt.Key_Up', 'Qt.Key_Down', 'Qt.Key_Left', 'Qt.Key_Right', 'Qt.Key_Return', 'Qt.Key_Menu', 'Qt.Key_Escape'):
            self.assertIn(k, q)
        self.assertIn('acceptedButtons: Qt.LeftButton | Qt.RightButton', q)     # right-click: Play on…
        self.assertIn('pw-metadata', q)
        self.assertIn('target: "mixer"', q)
        for f in ('Bar.qml', 'Islands.qml'):
            self.assertIn('label: "Volume mixer"', (SHELL / f).read_text(), f)
        self.assertIn('title: "Volume mixer"', (SHELL / 'SetSound.qml').read_text())
        pad = (SHELL / 'Pad.qml').read_text()
        self.assertIn('SettingsNav.mixerShown', pad)
        self.assertIn('SettingsNav.snapShown', pad)


# ---------------------------------------------------------------------- snap layouts
@unittest.skipUnless(NODE, 'node is needed for the snap layouts')
class SnapLayouts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.layouts = run_js('snap-layouts.js', 'console.log(JSON.stringify(LAYOUTS.map(l => Object.assign({}, l, {zones: l.zones.map(q => Object.assign({}, q, {labwc: labwcKey(q), wtype: wtypeArgs(q)}))}))))')
        cls.rc = ET.parse(LABWC / 'rc.xml').getroot()

    def test_six_layouts_that_fill_the_screen(self):
        self.assertEqual([l['id'] for l in self.layouts], ['halves', 'two-thirds', 'thirds', 'quarters', 'half-quarters', 'narrow-wide'])
        for l in self.layouts:
            zs = l['zones']
            self.assertEqual(sum(q['w'] * q['h'] for q in zs), 100 * 100, l['id'])
            for q in zs:
                self.assertTrue(0 <= q['x'] and q['x'] + q['w'] <= 100 and 0 <= q['y'] and q['y'] + q['h'] <= 100, q)
            for i, a in enumerate(zs):
                for b in zs[i + 1:]:
                    overlap = (min(a['x'] + a['w'], b['x'] + b['w']) > max(a['x'], b['x'])
                               and min(a['y'] + a['h'], b['y'] + b['h']) > max(a['y'], b['y']))
                    self.assertFalse(overlap, (l['id'], a['id'], b['id']))
        # The mockup's shapes: 1:1, 2:1, thirds, 2x2, half + two quarters, 1:2:1.
        widths = {l['id']: [q['w'] for q in l['zones']] for l in self.layouts}
        self.assertEqual(widths['halves'], [50, 50])
        self.assertEqual(widths['two-thirds'], [67, 33])
        self.assertEqual(widths['narrow-wide'], [25, 50, 25])

    def test_same_zone_same_key_and_every_key_its_own_zone(self):
        keys = {}
        for l in self.layouts:
            for q in l['zones']:
                keys.setdefault(q['id'], set()).add(q['labwc'])
        for zid, ks in keys.items():
            self.assertEqual(len(ks), 1, zid)
        self.assertEqual(len({next(iter(k)) for k in keys.values()}), len(keys))

    def test_labwc_has_the_keys_and_regions(self):
        binds = {kb.get('key'): kb for kb in self.rc.iter('keybind')}
        regions = {r.get('name'): r for r in self.rc.iter('region')}
        pct = lambda r, k: int(r.get(k).rstrip('%'))
        for l in self.layouts:
            for q in l['zones']:
                kb = binds.get(q['labwc'])
                self.assertIsNotNone(kb, q['labwc'])
                a = kb.find('action')
                if q['id'] in ('left-half', 'right-half'):
                    self.assertEqual((a.get('name'), a.get('direction')), ('SnapToEdge', q['id'].split('-')[0]))
                    continue
                self.assertEqual((a.get('name'), a.get('region')), ('SnapToRegion', q['id']), q['labwc'])
                r = regions[q['id']]
                self.assertEqual((pct(r, 'x'), pct(r, 'y'), pct(r, 'width'), pct(r, 'height')), (q['x'], q['y'], q['w'], q['h']), q['id'])
        self.assertEqual(self.layouts[1]['zones'][0]['wtype'],
                         ['-M', 'logo', '-M', 'ctrl', '-M', 'shift', '-k', 'F1', '-m', 'shift', '-m', 'ctrl', '-m', 'logo'])

    def test_ways_to_open_it(self):
        binds = {kb.get('key'): kb for kb in self.rc.iter('keybind')}
        self.assertIn('call snap toggle', binds['W-z'].find('action').get('command'))
        # Right-click on Maximize (labwc can't report hovering), and the title bar menu.
        ctx = [c for c in self.rc.iter('context') if c.get('name') == 'Maximize']
        self.assertEqual(len(ctx), 1)
        mb = ctx[0].find('mousebind')
        self.assertEqual((mb.get('button'), mb.get('action')), ('Right', 'Click'))
        self.assertIn('call snap open', ET.tostring(mb, encoding='unicode'))
        self.assertIn('call snap open', (LABWC / 'menu.xml').read_text())
        # Super+R and Super+X stay free for the Run box and the Super+X menu.
        self.assertNotIn('call snap', ET.tostring(binds.get('W-r', ET.Element('x')), encoding='unicode'))
        self.assertNotIn('W-x', [k for k, kb in binds.items() if 'snap' in ET.tostring(kb, encoding='unicode')])

    def test_the_d_pad_moves_between_zones(self):
        r = run_js('snap-layouts.js', '''var f = flat(); var at = id => f.findIndex(q => q.id === id);
            console.log(JSON.stringify([f.length, f[step(0, "right")].id, f[step(0, "down")].layout, step(0, "left"), step(0, "up"),
                f[step(at("right-third"), "right")].layout, f[step(f.length - 1, "down")].id]))''')
        self.assertEqual(r[0], 2 + 2 + 3 + 4 + 3 + 3)
        self.assertEqual(r[1], 'right-half')
        self.assertEqual(r[2], 3)          # the layout under Halves: Quarters
        self.assertEqual((r[3], r[4]), (0, 0))   # the edges stay put
        self.assertEqual(r[5], 2)          # from the two-thirds layout's right third on to Thirds
        self.assertEqual(r[6], 'narrow-right')

    def test_shortcuts_list_names_them(self):
        hs = load(SHELL / 'hearth-system', 'hearth_system_x')
        out = {b['key']: b['action'] for b in hs.parse_keybinds((LABWC / 'rc.xml').read_text())}
        self.assertEqual(out['W-z'], 'Snap layouts')
        self.assertEqual(out['W-C-S-F1'], 'Snap the window to the left two thirds')


# ---------------------------------------------------------------------- clipboard history
class ClipboardStore(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-clip-')
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.env = {k: os.environ.get(k) for k in ('XDG_DATA_HOME', 'XDG_RUNTIME_DIR', 'XDG_STATE_HOME', 'CLIPBOARD_STATE')}
        os.environ['XDG_DATA_HOME'] = os.path.join(self.tmp, 'data')
        os.environ['XDG_RUNTIME_DIR'] = os.path.join(self.tmp, 'run')
        os.environ['XDG_STATE_HOME'] = os.path.join(self.tmp, 'state')
        os.environ.pop('CLIPBOARD_STATE', None)
        os.makedirs(os.environ['XDG_RUNTIME_DIR'])
        self.addCleanup(self.restore_env)
        self.c = load(SHELL / 'hearth-clipboard', 'hearth_clipboard_x')
        self.n = 1000.0

    def restore_env(self):
        for k, v in self.env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def put(self, data, types=('text/plain;charset=utf-8',), app='Firefox', state=''):
        self.n += 1
        return self.c.store(data if isinstance(data, bytes) else data.encode(), types=list(types), app=app, now=self.n, state=state)

    def items(self):
        return self.c.list_items()['items']

    def test_kept_with_the_app_and_time_in_its_own_folder(self):
        r = self.put('chris-hearth.tail0e2082.ts.net:8188')
        self.assertTrue(r['stored'])
        it = self.items()[0]
        self.assertEqual((it['kind'], it['text'], it['app'], it['pinned']), ('text', 'chris-hearth.tail0e2082.ts.net:8188', 'Firefox', False))
        d = os.path.join(self.tmp, 'data', 'hearth', 'clipboard')
        self.assertEqual(os.path.dirname(it['path']), d)
        self.assertEqual(os.stat(d).st_mode & 0o777, 0o700)
        with open(it['path'], 'rb') as f:
            self.assertEqual(f.read(), b'chris-hearth.tail0e2082.ts.net:8188')

    def test_about_25_items_and_pins_on_top_of_that(self):
        first = self.put('keep me')['id']
        self.c.set_pin(first, True)
        for i in range(40):
            self.put('item %d' % i)
        items = self.items()
        self.assertEqual(len(items), self.c.LIMIT + 1)
        self.assertEqual(items[0]['id'], first)            # pinned first
        self.assertTrue(items[0]['pinned'])
        self.assertEqual(items[1]['text'], 'item 39')       # then newest first
        self.assertEqual(items[-1]['text'], 'item 15')
        files = os.listdir(os.path.join(self.tmp, 'data', 'hearth', 'clipboard'))
        self.assertEqual(len([f for f in files if f.endswith('.txt')]), self.c.LIMIT + 1)   # dropped items' files gone

    def test_clear_all_keeps_pins(self):
        a = self.put('pinned one')['id']
        self.put('not pinned')
        self.c.set_pin(a, True)
        r = self.c.clear()
        self.assertEqual(r['kept'], 1)
        self.assertEqual([i['text'] for i in self.items()], ['pinned one'])
        self.c.set_pin(a, False)
        self.c.clear()
        self.assertEqual(self.items(), [])

    def test_secrets_are_never_kept(self):
        # A password manager's hint among the offered types, or wl-paste's CLIPBOARD_STATE=sensitive.
        r = self.put('hunter2', types=['text/plain', 'x-kde-passwordManagerHint'])
        self.assertEqual(r, {'stored': False, 'why': 'secret'})
        r = self.put('hunter3', state='sensitive')
        self.assertEqual(r, {'stored': False, 'why': 'secret'})
        os.environ['CLIPBOARD_STATE'] = 'sensitive'
        self.assertFalse(self.c.store(b'hunter4', types=['text/plain'], app='', now=1)['stored'])
        os.environ.pop('CLIPBOARD_STATE')
        self.assertEqual(self.items(), [])
        d = os.path.join(self.tmp, 'data', 'hearth', 'clipboard')
        for f in os.listdir(d) if os.path.isdir(d) else []:
            with open(os.path.join(d, f), 'rb') as fh:
                self.assertNotIn(b'hunter', fh.read(), f)

    def test_copied_again_goes_back_to_the_top(self):
        a = self.put('same', app='Terminal')['id']
        self.put('other')
        b = self.put('same', app='Discord')['id']
        self.assertEqual(a, b)
        items = self.items()
        self.assertEqual([i['text'] for i in items], ['same', 'other'])
        self.assertEqual(items[0]['app'], 'Terminal')      # where it first came from

    def test_pictures(self):
        png = b'\x89PNG\r\n\x1a\n' + b'\x00\x00\x00\rIHDR' + (1920).to_bytes(4, 'big') + (1080).to_bytes(4, 'big') + b'\x08\x02\x00\x00\x00' + b'x' * 50
        self.put(png, types=['image/png'], app='Capture')
        it = self.items()[0]
        self.assertEqual((it['kind'], it['dims'], it['app']), ('image', '1920x1080', 'Capture'))
        self.assertTrue(it['path'].endswith('.png'))

    def test_nothing_odd_is_kept(self):
        for data in (b'', b'   \n', b'\x00\x01binary'):
            self.assertFalse(self.put(data)['stored'])
        self.assertFalse(self.c.store(b'x' * (self.c.MAX_BYTES + 1), types=['text/plain'], app='', now=1)['stored'])

    def test_delete_and_ids(self):
        a = self.put('one')['id']
        self.put('two')
        self.c.delete(a)
        self.assertEqual([i['text'] for i in self.items()], ['two'])
        for bad in ('', 'abc', '1;2', '../x', '1 2'):
            with self.assertRaises(self.c.Fail):
                self.c.delete(bad)
            with self.assertRaises(self.c.Fail):
                self.c.set_pin(bad, True)

    def test_listing_writes_nothing(self):
        self.assertEqual(self.c.list_items()['items'], [])
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'data', 'hearth')))

    def test_a_damaged_list_comes_back_from_its_bak(self):
        self.put('first')
        self.put('second')        # index.json.bak now has "first"
        with open(self.c.index_path(), 'w') as f:
            f.write('{"items": [')
        self.assertEqual([i['text'] for i in self.items()], ['first'])

    def test_the_app_in_front_comes_from_the_shell(self):
        with open(os.path.join(self.tmp, 'run', 'hearth-clipboard-app'), 'w') as f:
            f.write('Hearth Files\n')
        self.c.store(b'hello', types=['text/plain'], now=5, state='')
        self.assertEqual(self.items()[0]['app'], 'Hearth Files')

    def test_store_from_stdin(self):
        env = dict(os.environ, HEARTH_CLIP_TYPES='text/plain\n')
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-clipboard'), 'store'], input=b'from wl-paste', capture_output=True, env=env, timeout=30)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertTrue(json.loads(r.stdout)['stored'])
        env['HEARTH_CLIP_TYPES'] = 'text/plain\nx-kde-passwordManagerHint\n'
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-clipboard'), 'store'], input=b'secret', capture_output=True, env=env, timeout=30)
        self.assertFalse(json.loads(r.stdout)['stored'])
        self.assertEqual([i['text'] for i in self.items()], ['from wl-paste'])


class ClipboardWiring(unittest.TestCase):
    def test_off_until_you_turn_it_on(self):
        skel = json.loads((ROOT / 'os' / 'system_files' / 'etc' / 'skel' / '.local' / 'share' / 'nimbus' / 'desktop.json').read_text())
        self.assertIs(skel['clipboardHistory'], False)
        q = code((SHELL / 'Clipboard.qml').read_text())
        # The watcher runs only while history is on; the switch is only ever changed by you.
        self.assertIn('readonly property bool wanted: tools && isOn', q)
        self.assertIn('onWantedChanged: { watcher.running = wanted; noteApp(); }', q)
        self.assertIn('command: ["wl-paste", "--watch", "python3", cb.helper, "store"]', q)
        self.assertEqual(len(re.findall(r'Settings\.clipboardHistory\s*=', q)), 1)
        self.assertIn('function setOn(v) { Settings.clipboardHistory = v;', q)
        for caller in re.findall(r'[^\n]*setOn\([^)]*\)[^\n]*', q):
            if 'function setOn' in caller:
                continue
            self.assertRegex(caller, r'onClicked|column === 0', caller)
        # Opening it only lists (list writes nothing).
        i = q.index('function open()')
        self.assertNotIn('Settings.', q[i:q.index('\n', i)])

    def test_loaded_so_a_problem_cant_stop_the_shell(self):
        s = code((SHELL / 'shell.qml').read_text())
        for f in ('Clipboard.qml', 'Mixer.qml', 'SnapLayouts.qml'):
            self.assertIn('Loader { source: "%s" }' % f, s)
        self.assertNotRegex(s, r'(?m)^\s*(Clipboard|Mixer|SnapLayouts)\s*\{')

    def test_super_v_and_super_period(self):
        rc = ET.parse(LABWC / 'rc.xml').getroot()
        binds = {kb.get('key'): ET.tostring(kb, encoding='unicode') for kb in rc.iter('keybind')}
        self.assertIn('call clipboard toggle', binds['W-v'])
        self.assertIn('hearth-emoji', binds['W-period'])


if __name__ == '__main__':
    unittest.main()
