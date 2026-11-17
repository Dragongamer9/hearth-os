"""Tests for Hearth Settings' KWin pages (docs/KWIN-SETTINGS.md): hearth_kconfig's INI merge (every other key, comment and
group kept; only the changed keys written; a file that can't be read left untouched; .bak kept), and hearth-kwin-settings
(what it may change, desktops, tile layouts, window rules, screens' ids, the mouse settings put on KWin's devices,
hearth-access's colour filters on KWin), all without KWin (no session bus)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import unittest
import uuid
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


import hearth_kconfig as kc  # noqa: E402
K = load(SHELL / 'hearth-kwin-settings', 'hearth_kwin_settings_test')

SAMPLE = """# Written by KWin
[$Version]
update_info=kwin.upd:replace-scalein-with-scale

[Desktops]
Id_1=0b8f7d1e-aaaa-bbbb-cccc-000000000001
Name_1=Main
Number=1
Rows=1

[Plugins]
blurEnabled=false
; another comment
Name[de]=Fenster

[Tiling][1f2e3d4c-0000-1111-2222-333344445555]
tiles={"layoutDirection":"horizontal","tiles":[{"width":0.5},{"width":0.5}]}

[Windows]
ElectricBorderDelay=150
"""


class Env(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = self.tmp.name
        self.home = os.path.join(t, 'home', '.config')
        self.sys = os.path.join(t, 'xdg')
        os.makedirs(self.home)
        os.makedirs(self.sys)
        self.env = mock.patch.dict(os.environ, {'HOME': os.path.join(t, 'home'), 'XDG_CONFIG_HOME': self.home,
                                                'XDG_CONFIG_DIRS': self.sys, 'XDG_RUNTIME_DIR': t,
                                                'XDG_STATE_HOME': os.path.join(t, 'state'),
                                                'DBUS_SESSION_BUS_ADDRESS': 'unix:path=/nonexistent'})
        self.env.start()
        # No session bus here: nothing is sent to KWin.
        K._BUS[:] = [None]

    def tearDown(self):
        self.env.stop()
        K._BUS[:] = []
        self.tmp.cleanup()

    def path(self, name):
        return os.path.join(self.home, name)

    def write(self, name, text):
        with open(self.path(name), 'w') as f:
            f.write(text)

    def read(self, name):
        with open(self.path(name)) as f:
            return f.read()


class Merge(Env):
    def test_only_changed_keys_and_everything_else_kept(self):
        self.write('kwinrc', SAMPLE)
        kc.update(self.path('kwinrc'), {'Plugins': {'blurEnabled': True, 'wobblywindowsEnabled': True}})
        new = self.read('kwinrc')
        old_lines = SAMPLE.splitlines()
        new_lines = new.splitlines()
        # Every line is still there but the one changed, plus the one added.
        self.assertEqual([l for l in old_lines if l != 'blurEnabled=false'],
                         [l for l in new_lines if l not in ('blurEnabled=true', 'wobblywindowsEnabled=true')])
        self.assertIn('blurEnabled=true', new_lines)
        # The new key went into its group, not at the end of the file.
        i = new_lines.index('[Plugins]')
        j = new_lines.index('[Tiling][1f2e3d4c-0000-1111-2222-333344445555]')
        self.assertTrue(i < new_lines.index('wobblywindowsEnabled=true') < j)
        for keep in ('# Written by KWin', '; another comment', 'Name[de]=Fenster', '[$Version]', 'Rows=1'):
            self.assertIn(keep, new_lines)

    def test_bak_kept(self):
        self.write('kwinrc', SAMPLE)
        kc.update(self.path('kwinrc'), {'Windows': {'ElectricBorderDelay': 300}})
        with open(self.path('kwinrc') + '.bak') as f:
            self.assertEqual(f.read(), SAMPLE)
        self.assertIn('ElectricBorderDelay=300', self.read('kwinrc'))

    def test_nothing_written_when_nothing_changes(self):
        self.write('kwinrc', SAMPLE)
        before = os.stat(self.path('kwinrc')).st_mtime_ns
        self.assertFalse(kc.update(self.path('kwinrc'), {'Windows': {'ElectricBorderDelay': 150}}))
        self.assertEqual(os.stat(self.path('kwinrc')).st_mtime_ns, before)
        self.assertFalse(os.path.exists(self.path('kwinrc') + '.bak'))

    def test_failed_parse_left_untouched(self):
        for bad in (b'[Plugins\nblurEnabled=true\n', b'[Plugins]\nx=\xff\xfe\n', b'[Plugins]\nx=1\x00\n'):
            with open(self.path('kwinrc'), 'wb') as f:
                f.write(bad)
            with self.assertRaises(kc.BadFile):
                kc.update(self.path('kwinrc'), {'Plugins': {'blurEnabled': False}})
            with open(self.path('kwinrc'), 'rb') as f:
                self.assertEqual(f.read(), bad)
            self.assertFalse(os.path.exists(self.path('kwinrc') + '.bak'))

    def test_helper_refuses_broken_file(self):
        self.write('kaccessrc', '[Keyboard\nStickyKeys=false\n')
        with self.assertRaises((K.Fail, kc.BadFile)):
            K.set_file('kaccessrc', json.dumps({'Keyboard': {'StickyKeys': True}}))
        self.assertEqual(self.read('kaccessrc'), '[Keyboard\nStickyKeys=false\n')

    def test_null_removes_key_so_the_default_counts(self):
        self.write('kwinrc', SAMPLE)
        with open(os.path.join(self.sys, 'kwinrc'), 'w') as f:
            f.write('[Plugins]\nblurEnabled=true\n')
        vals, mine, _ = kc.effective('kwinrc')
        self.assertEqual(vals['Plugins']['blurEnabled'], 'false')
        self.assertIn('blurEnabled', mine['Plugins'])
        kc.update(self.path('kwinrc'), {'Plugins': {'blurEnabled': None}})
        self.assertNotIn('blurEnabled', self.read('kwinrc'))
        vals, mine, _ = kc.effective('kwinrc')
        self.assertEqual(vals['Plugins']['blurEnabled'], 'true')     # Hearth's system default
        self.assertNotIn('blurEnabled', mine.get('Plugins', []))

    def test_new_file_and_new_nested_group(self):
        kc.update(self.path('kwinrc'), {'Tiling][abc': {'tiles': '{"a":1}'}, 'Tiling': {'padding': 4}})
        text = self.read('kwinrc')
        self.assertIn('[Tiling][abc]\ntiles={"a":1}', text)
        self.assertIn('[Tiling]\npadding=4', text)
        data, _ = kc.parse(text)
        self.assertEqual(data['Tiling][abc']['tiles'], '{"a":1}')

    def test_last_duplicate_wins_and_is_the_one_changed(self):
        self.write('kwinrc', '[Windows]\nRollOverDesktops=false\n[Other]\na=1\n[Windows]\nRollOverDesktops=false\n')
        kc.update(self.path('kwinrc'), {'Windows': {'RollOverDesktops': True}})
        self.assertEqual(self.read('kwinrc'), '[Windows]\nRollOverDesktops=false\n[Other]\na=1\n[Windows]\nRollOverDesktops=true\n')

    def test_escapes(self):
        kc.update(self.path('kwinrc'), {'Desktops': {'Name_1': ' two\nlines\\ '}})
        self.assertIn('Name_1=\\stwo\\nlines\\\\\\s', self.read('kwinrc'))
        vals, _, _ = kc.effective('kwinrc')
        self.assertEqual(vals['Desktops']['Name_1'], ' two\nlines\\ ')

    def test_locked_keys(self):
        with open(os.path.join(self.sys, 'kwinrc'), 'w') as f:
            f.write('[Compositing]\nAllowTearing[$i]=false\n')
        self.write('kwinrc', '[Compositing]\nAllowTearing=true\n')
        vals, _, locked = kc.effective('kwinrc')
        self.assertEqual(vals['Compositing']['AllowTearing'], 'false')
        with self.assertRaises(K.Fail):
            K.set_file('kwinrc', json.dumps({'Compositing': {'AllowTearing': True}}))

    def test_remove_group(self):
        self.write('kwinrulesrc', '[General]\ncount=2\nrules=a1b2c3d4,e5f6a7b8\n\n[a1b2c3d4]\nDescription=One\n\n[e5f6a7b8]\nDescription=Two\n')
        kc.update(self.path('kwinrulesrc'), {'General': {'count': 1, 'rules': 'e5f6a7b8'}}, remove_groups=('a1b2c3d4',))
        self.assertEqual(self.read('kwinrulesrc'), '[General]\ncount=1\nrules=e5f6a7b8\n\n[e5f6a7b8]\nDescription=Two\n')

    def test_bad_names_refused(self):
        with self.assertRaises(ValueError):
            kc.merge_text('', {'Plugins': {'a\nb': 1}})
        with self.assertRaises(ValueError):
            kc.merge_text('', {'Plu\ngins': {'a': 1}})


class Helper(Env):
    def test_only_allowed_groups(self):
        with self.assertRaises(K.Fail):
            K.set_file('kwinrc', json.dumps({'Compositing][x': {'Backend': 'XRender'}}))
        with self.assertRaises(K.Fail):
            K.set_file('kwinrc', json.dumps({'MouseBindings': {'CommandWindow1': 'Nothing'}}))   # clicks in windows stay KWin's
        with self.assertRaises(K.Fail):
            K.set_file('passwd', json.dumps({}))
        out = K.set_file('kwinrc', json.dumps({'Plugins': {'wobblywindowsEnabled': True}}))
        self.assertEqual(out['files']['kwinrc']['Plugins']['wobblywindowsEnabled'], 'true')
        self.assertEqual(self.read('kwinrc'), '[Plugins]\nwobblywindowsEnabled=true\n')

    def test_window_behaviour(self):
        # Settings › Window behaviour (SetKwinWindows.qml): [Windows] focus and placement, [MouseBindings] title bar and
        # Super+drag; only the keys changed are written and a key set back to KWin's default (null) is taken out.
        self.write('kwinrc', SAMPLE)
        K.set_file('kwinrc', json.dumps({'Windows': {'FocusPolicy': 'FocusFollowsMouse', 'AutoRaise': True, 'AutoRaiseInterval': 400,
                                                     'FocusStealingPreventionLevel': 3, 'Placement': 'UnderMouse'},
                                         'MouseBindings': {'CommandActiveTitlebar2': 'Close', 'CommandInactiveTitlebar2': 'Close',
                                                           'CommandAllKey': 'Alt', 'CommandAll3': 'Operations menu',
                                                           'CommandTitlebarWheel': 'Shade/Unshade'}}))
        text = self.read('kwinrc')
        for line in ('FocusPolicy=FocusFollowsMouse', 'AutoRaise=true', 'AutoRaiseInterval=400', 'FocusStealingPreventionLevel=3',
                     'Placement=UnderMouse', 'ElectricBorderDelay=150', '[MouseBindings]', 'CommandActiveTitlebar2=Close',
                     'CommandAllKey=Alt', 'CommandAll3=Operations menu', 'CommandTitlebarWheel=Shade/Unshade', 'blurEnabled=false'):
            self.assertIn(line, text)
        self.assertEqual(text.count('[Windows]'), 1)
        K.set_file('kwinrc', json.dumps({'Windows': {'FocusPolicy': None}, 'MouseBindings': {'CommandAllKey': None}}))
        text = self.read('kwinrc')
        self.assertNotIn('FocusPolicy', text)
        self.assertNotIn('CommandAllKey', text)
        self.assertIn('CommandAll3=Operations menu', text)
        with self.assertRaises(K.Fail):
            K.set_file('kwinrc', json.dumps({'MouseBindings': {'CommandAll1': 'Move', 'CommandWindowWheel': 'Nothing'}}))
        self.assertNotIn('CommandAll1', self.read('kwinrc'), 'a refused change writes none of its keys')

    def test_window_behaviour_page_writes_only_what_it_may(self):
        # Every [Windows] / [MouseBindings] key the page sets is one hearth-kwin-settings lets through, and the page
        # writes nothing when it opens (only from onChosen / onToggled / onMoved).
        qml = (SHELL / 'SetKwinWindows.qml').read_text()
        used = set(re.findall(r'"(Windows|MouseBindings)",\s*"(\w+)"', qml))
        self.assertTrue(used, 'the page names its keys')
        for g, k in used:
            self.assertTrue(K.allowed('kwinrc', g), g)
            only = K.ONLY_KEYS['kwinrc'].get(g)
            if only is not None:
                self.assertIn(k, only)
        for k in ('FocusPolicy', 'AutoRaise', 'AutoRaiseInterval', 'DelayFocusInterval', 'ClickRaise', 'NextFocusPrefersMouse',
                  'FocusStealingPreventionLevel', 'TitlebarDoubleClickCommand', 'Placement'):
            self.assertIn(('Windows', k), used)
        for k in K.ONLY_KEYS['kwinrc']['MouseBindings']:
            self.assertIn(('MouseBindings', k), used)
        self.assertIn('KwinGate', qml)
        self.assertNotIn('Component.onCompleted', qml)
        page = re.search(r'\{ id: "kwin-behaviour", file: "SetKwinWindows.qml", cat: "pers"[^}]*\}', (SHELL / 'settings-pages.js').read_text())
        self.assertTrue(page, 'Personalization › Window behaviour is in settings-pages.js')
        self.assertIn('"kwin-behaviour"', (SHELL / 'settings-index.js').read_text())

    def test_status_under_labwc_is_not_live(self):
        with mock.patch.dict(os.environ, {'HEARTH_WM': '', 'LABWC_PID': '42'}):
            s = K.status()
        self.assertFalse(s['live'])
        self.assertEqual(s['wm'], 'labwc')
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin'}):
            self.assertTrue(K.status()['live'])

    def test_preview_needs_both_fixture_and_marker(self):
        fx = os.path.join(self.tmp.name, 'fx.json')
        with open(fx, 'w') as f:
            json.dump({'effects': ['blur'], 'devices': [{'sysName': 'event3', 'name': 'Pen'}]}, f)
        with mock.patch.dict(os.environ, {'HEARTH_KWIN_FIXTURE': fx, 'HEARTH_WM': ''}):
            self.assertFalse(K.status()['live'])
            open(os.path.join(self.tmp.name, 'hearth-kwin-preview'), 'w').close()
            s = K.status()
            self.assertTrue(s['live'] and s['preview'])
            self.assertEqual(K.input_devices()['devices'][0]['name'], 'Pen')
            self.assertEqual(K.input_set('event3', '{"outputName": "DP-1"}'), {'ok': True, 'preview': True})

    def test_desktops_without_kwin(self):
        self.write('kwinrc', SAMPLE)
        d = K.desktops_set(json.dumps({'count': 3, 'names': {'2': 'Games'}, 'rows': 1}))
        self.assertEqual([x['name'] for x in d['desktops']], ['Main', 'Games', 'Desktop 3'])
        text = self.read('kwinrc')
        self.assertIn('Number=3', text)
        self.assertIn('Id_1=0b8f7d1e-aaaa-bbbb-cccc-000000000001', text)     # kept
        self.assertTrue(re.search(r'Id_3=[0-9a-f-]{36}', text))
        d = K.desktops_set(json.dumps({'count': 2}))
        self.assertEqual(len(d['desktops']), 2)
        self.assertNotIn('Id_3', self.read('kwinrc'))
        for bad in ({'count': 0}, {'count': 99}, {'rows': 9}, {'names': {'1': 'x' * 99}}):
            with self.assertRaises(K.Fail):
                K.desktops_set(json.dumps(bad))

    def test_tiles(self):
        u = '1f2e3d4c-0000-1111-2222-333344445555'
        self.write('kwinrc', SAMPLE)
        K.tiles_set(u, 'DP-1', json.dumps({'columns': [{'width': 1}, {'width': 2, 'rows': [1, 3]}, {'width': 1}]}))
        vals, _, _ = kc.effective('kwinrc')
        kw = json.loads(vals['Tiling][' + u]['tiles'])
        self.assertEqual(kw, {'layoutDirection': 'horizontal', 'tiles': [
            {'width': 0.25}, {'width': 0.5, 'layoutDirection': 'vertical', 'tiles': [{'height': 0.25}, {'height': 0.75}]},
            {'width': 0.25}]})
        self.assertEqual(K.kwin_to_layout(vals['Tiling][' + u]['tiles']),
                         {'columns': [{'width': 0.25, 'rows': []}, {'width': 0.5, 'rows': [0.25, 0.75]}, {'width': 0.25, 'rows': []}]})
        self.assertIn('ElectricBorderDelay=150', self.read('kwinrc'))
        for bad in ({'columns': []}, {'columns': [{'width': 0}]}, {'columns': [{'width': 1, 'rows': [1, 0]}]}):
            with self.assertRaises(K.Fail):
                K.tiles_set(u, 'DP-1', json.dumps(bad))
        with self.assertRaises(K.Fail):
            K.tiles_set('../../etc', 'DP-1', json.dumps({'padding': 4}))
        # A layout deeper than Hearth's editor: left as KWin's, shown as such.
        self.assertIsNone(K.kwin_to_layout('{"layoutDirection":"horizontal","tiles":[{"width":1,"layoutDirection":"vertical","tiles":[{"height":1,"tiles":[{}]}]}]}'))

    def test_output_uuid_like_kwin(self):
        # core/output.cpp: uuid5(uuid5(URL, "https://kwin.kde.org/o/"), "name:eisa:model:serial").
        ns = uuid.uuid5(uuid.NAMESPACE_URL, 'https://kwin.kde.org/o/')
        self.assertEqual(K.output_uuid('DP-1', 'DEL', 'DELL U2720Q', 'ABC123'), str(uuid.uuid5(ns, 'DP-1:DEL:DELL U2720Q:ABC123')))
        edid = bytearray(128)
        edid[0:8] = b'\x00\xff\xff\xff\xff\xff\xff\x00'
        v = ((ord('D') - 64) << 10) | ((ord('E') - 64) << 5) | (ord('L') - 64)
        edid[8], edid[9] = v >> 8, v & 0xff
        edid[10], edid[11] = 0x34, 0x12
        edid[54:72] = b'\x00\x00\x00\xfc\x00' + b'DELL U2720Q\n '
        edid[72:90] = b'\x00\x00\x00\xff\x00' + b'ABC123\n      '
        self.assertEqual(K.edid_ids(bytes(edid)), ('DEL', 'DELL U2720Q', 'ABC123'))
        edid[54:90] = bytes(36)
        edid[12:16] = b'\x01\x00\x00\x00'
        self.assertEqual(K.edid_ids(bytes(edid)), ('DEL', '0x1234', '0x00000001'))

    def test_rules(self):
        r = K.rule_set(json.dumps({'id': '', 'set': {'wmclass': 'steam', 'wmclassmatch': 1, 'above': True, 'aboverule': 2}}))
        rid = r['id']
        self.assertEqual(r['rules'][0]['values']['wmclass'], 'steam')
        self.assertEqual(r['rules'][0]['values']['Description'], 'Window rule')
        K.rule_set(json.dumps({'id': rid, 'set': {'above': None, 'aboverule': None, 'noborder': True, 'noborderrule': 2}}))
        text = self.read('kwinrulesrc')
        self.assertNotIn('above', text)
        self.assertIn('noborder=true', text)
        r2 = K.rule_set(json.dumps({'id': '', 'set': {'Description': 'Two'}}))
        self.assertIn('rules=%s,%s' % (rid, r2['id']), self.read('kwinrulesrc'))
        for bad in ({'id': rid, 'set': {'shortcut': 'Meta+X'}}, {'id': rid, 'set': {'position': 'here'}},
                    {'id': rid, 'set': {'aboverule': 9}}, {'id': 'nope', 'set': {}}, {'id': rid, 'set': {'title': 'a\nb'}}):
            with self.assertRaises(K.Fail):
                K.rule_set(json.dumps(bad))
        K.rule_delete(rid)
        text = self.read('kwinrulesrc')
        self.assertNotIn(rid, text)
        self.assertIn('count=1', text)
        self.assertIn('Description=Two', text)

    def test_pointer_plan(self):
        devices = [
            {'sysName': 'event4', 'pointer': True, 'touchpad': True, 'tapFingerCount': 3, 'tapToClick': False,
             'naturalScroll': False, 'supportsNaturalScroll': True, 'disableWhileTyping': True,
             'supportsDisableWhileTyping': True, 'scrollFactor': 1.0, 'scrollTwoFinger': True, 'supportsScrollTwoFinger': True,
             'scrollEdge': False, 'supportsScrollEdge': True},
            {'sysName': 'event5', 'pointer': True, 'pointerAcceleration': 0.0, 'supportsPointerAcceleration': True,
             'pointerAccelerationProfileFlat': False, 'pointerAccelerationProfileAdaptive': True,
             'supportsPointerAccelerationProfileFlat': True, 'leftHanded': False, 'supportsLeftHanded': True},
            {'sysName': 'event6', 'pointer': True, 'tabletTool': True, 'leftHanded': False, 'supportsLeftHanded': True},
        ]
        plan = K.pointer_plan(devices, {'mouse': {'speed': 0.5, 'accel': 'flat', 'leftHanded': False},
                                        'touchpad': {'tap': True, 'natural': True, 'dwt': True, 'scroll': 'twofinger',
                                                     'scrollSpeed': 2.0}})
        self.assertEqual(sorted(plan), sorted([
            ('event4', 'tapToClick', True), ('event4', 'naturalScroll', True), ('event4', 'scrollFactor', 2.0),
            ('event5', 'pointerAcceleration', 0.5), ('event5', 'pointerAccelerationProfileFlat', True),
            ('event5', 'pointerAccelerationProfileAdaptive', False)]))

    def test_pointer_json_keeps_labwc_unchanged(self):
        S = load(SHELL / 'hearth-system', 'hearth_system_kwin_test')
        plain = S.pointer_clean({'mouse': {'speed': 0.2}, 'touchpad': {'tap': False}})
        extra = S.pointer_clean({'mouse': {'speed': 0.2, 'scrollSpeed': 2.5, 'middle': True},
                                 'touchpad': {'tap': False, 'tapDrag': False, 'click': 'fingers', 'scrollSpeed': 9}})
        self.assertEqual(S.libinput_xml(plain), S.libinput_xml(extra))
        self.assertEqual(extra['touchpad']['scrollSpeed'], 5.0)
        self.assertNotIn('scrollSpeed', plain['mouse'])          # no default: KWin's own counts

    def test_rebind(self):
        K.rebind('Wacom Intuos S Pen', '331', 'MouseButton,2')
        self.assertIn('[ButtonRebinds][TabletTool][Wacom Intuos S Pen]\n331=MouseButton,2', self.read('kcminputrc'))
        K.rebind('Wacom Intuos S Pen', '331', '')
        self.assertNotIn('331=', self.read('kcminputrc'))
        for t, b, a in (('Pen]', '331', ''), ('Pen', 'x', ''), ('Pen', '331', 'Exec,rm -rf')):
            with self.assertRaises(K.Fail):
                K.rebind(t, b, a)


class AccessOnKwin(Env):
    def test_filters_through_kwin(self):
        A = load(SHELL / 'hearth-access', 'hearth_access_kwin_test')
        self.write('kwinrc', SAMPLE)
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin', 'LABWC_PID': ''}), \
                mock.patch.object(A, 'kwin_version_tuple', return_value=(6, 3)):
            self.assertEqual(A.mode(), 'kwin')
            self.assertNotIn('grayscale', A.kwin_filters((6, 3)))
            self.assertIn('grayscale', A.kwin_filters((6, 6)))
            self.assertIsNone(A.apply_filter_kwin('deuteranopia'))
            vals, _, _ = kc.effective('kwinrc')
            self.assertEqual(vals['Plugins']['colorblindnesscorrectionEnabled'], 'true')
            self.assertEqual(vals['Effect-colorblindnesscorrection']['Mode'], '1')
            self.assertIn('Greyscale needs KWin 6.6', A.apply_filter_kwin('grayscale'))
            self.assertIsNone(A.apply_filter_kwin('none'))
            vals, _, _ = kc.effective('kwinrc')
            self.assertEqual(vals['Plugins']['colorblindnesscorrectionEnabled'], 'false')
            self.assertIn('ElectricBorderDelay=150', self.read('kwinrc'))


class Review(Env):
    def test_list_escaped_once(self):
        kc.update(self.path('kwinrc'), {'Effect-overview': {'BorderActivate': [7, 1]}, 'X': {'L': ['a,b', 'c\\d']}})
        text = self.read('kwinrc')
        self.assertIn('BorderActivate=7,1\n', text)
        self.assertIn('L=a\\,b,c\\\\d\n', text)

    def test_crlf_kept(self):
        with open(self.path('kwinrc'), 'w', newline='') as f:
            f.write('[Plugins]\r\nblurEnabled=false\r\n')
        kc.update(self.path('kwinrc'), {'Plugins': {'blurEnabled': True, 'fadeEnabled': True}})
        with open(self.path('kwinrc'), newline='') as f:
            self.assertEqual(f.read(), '[Plugins]\r\nblurEnabled=true\r\nfadeEnabled=true\r\n')

    def test_locked_group_in_your_file_refused(self):
        self.write('kwinrc', '[Plugins][$i]\nblurEnabled=false\n')
        with self.assertRaises(kc.Locked):
            kc.update(self.path('kwinrc'), {'Plugins': {'blurEnabled': True}})
        with self.assertRaises(K.Fail):
            K.set_file('kwinrc', json.dumps({'Plugins': {'blurEnabled': True}}))
        self.assertEqual(self.read('kwinrc'), '[Plugins][$i]\nblurEnabled=false\n')
        with open(os.path.join(self.sys, 'kwinrc'), 'w') as f:
            f.write('[Windows][$i]\nRollOverDesktops=false\n')
        with self.assertRaises(K.Fail):
            K.set_file('kwinrc', json.dumps({'Windows': {'RollOverDesktops': True}}))

    def test_accessibility_owns_its_effects(self):
        for ch in ({'Plugins': {'colorblindnesscorrectionEnabled': True}}, {'Plugins': {'invertEnabled': True}},
                   {'Effect-colorblindnesscorrection': {'Mode': 1}}, {'Effect-zoom': {'ZoomFactor': 2}}):
            with self.assertRaises(K.Fail):
                K.set_file('kwinrc', json.dumps(ch))
        self.assertFalse(os.path.exists(self.path('kwinrc')))
        K.set_file('kwinrc', json.dumps({'Effect-zoom': {'MouseTracking': 1}}))

    def test_kdeglobals_started_with_hearths_mark(self):
        K.set_file('kdeglobals', json.dumps({'KDE': {'AnimationDurationFactor': 0.5}}))
        self.assertTrue(self.read('kdeglobals').startswith(K.KDE_MARK + '\n'))

    def test_theme_keeps_your_animation_speed(self):
        import subprocess
        home = os.path.join(self.tmp.name, 'home')
        os.makedirs(os.path.join(home, '.local', 'share', 'nimbus'), exist_ok=True)
        with open(os.path.join(home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
            json.dump({'look': 'steam', 'mode': 'dark'}, f)
        K.set_file('kdeglobals', json.dumps({'KDE': {'AnimationDurationFactor': 0.25}, 'KScreen': {'XwaylandClientsScale': False}}))
        env = dict(os.environ, HOME=home, NIMBUS_SHARE=str(ROOT / 'hosts' / 'linux'))
        env.pop('LABWC_PID', None)
        for mode in ('dark', 'light'):
            with open(os.path.join(home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
                json.dump({'look': 'steam', 'mode': mode}, f)
            subprocess.run(['sh', str(ROOT / 'hosts' / 'linux' / 'desktop' / 'theme' / 'apply.sh')], env=env,
                           capture_output=True, timeout=120)
            text = self.read('kdeglobals')
            self.assertTrue(text.startswith(K.KDE_MARK))
            self.assertIn('AnimationDurationFactor=0.25', text)
            self.assertIn('XwaylandClientsScale=false', text)
            self.assertIn('[UiSettings]', text)
            self.assertIn('Theme=%s' % ('breeze' if mode == 'light' else 'breeze-dark'), text)
            self.assertIn('ColorScheme=HearthLive', text)
        # And yours (no mark) is still never touched.
        self.write('kdeglobals', '[General]\nColorScheme=Mine\n')
        subprocess.run(['sh', str(ROOT / 'hosts' / 'linux' / 'desktop' / 'theme' / 'apply.sh')], env=env, capture_output=True, timeout=120)
        self.assertEqual(self.read('kdeglobals'), '[General]\nColorScheme=Mine\n')

    def test_access_apply_on_kwin_writes_nothing_twice(self):
        A = load(SHELL / 'hearth-access', 'hearth_access_kwin_idem')
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin', 'LABWC_PID': ''}), \
                mock.patch.object(A, 'kwin_version_tuple', return_value=(6, 3)), \
                mock.patch.object(A, 'gsettings'):
            # 'none' with KWin's defaults: nothing to write, no file made.
            self.assertEqual(A.apply(A.clean({'filter': 'none'}), None, 'kwin'), {})
            self.assertFalse(os.path.exists(self.path('kwinrc')))
            A.apply(A.clean({'filter': 'tritanopia', 'filterStrength': 0.6}), None, 'kwin')
            text = self.read('kwinrc')
            self.assertIn('Intensity=0.6', text)
            t1 = os.stat(self.path('kwinrc')).st_mtime_ns
            A.apply(A.clean({'filter': 'tritanopia', 'filterStrength': 0.6}), None, 'kwin')
            self.assertEqual(os.stat(self.path('kwinrc')).st_mtime_ns, t1)

    def test_stale_invert_mark(self):
        A = load(SHELL / 'hearth-access', 'hearth_access_kwin_mark')
        with open(A.kwin_invert_mark(), 'w') as f:
            f.write('4242')
        with mock.patch.object(A, 'kwin_pid', return_value=4242):
            self.assertTrue(A.kwin_inverted(K))
        with mock.patch.object(A, 'kwin_pid', return_value=999):   # KWin started again: not inverted any more
            self.assertFalse(A.kwin_inverted(K))
        with mock.patch.object(A, 'kwin_pid', return_value=0):
            self.assertFalse(A.kwin_inverted(K))


if __name__ == '__main__':
    unittest.main()
