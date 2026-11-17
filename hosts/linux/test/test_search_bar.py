"""The search bar (1.14, like KDE's KRunner: hosts/linux/desktop/shell/SearchBar.qml, search-bar.js): Alt+Space and
Alt+F2 open it on labwc and KWin, the keyboard layout switch keeps Alt+Space when it's set to it (labwc gets a "None"
binding, the bar ignores the key), groups in the order chosen in Settings › Search (hearth-search "pluginOrder",
saved only as it's changed), "?" help, typing on the desktop opens it, the controller drives it (Y on the desktop,
the D-pad and A / B through Pad.qml), and a problem in it never costs the shell (loaded by file name)."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
DESK = ROOT / 'hosts' / 'linux' / 'desktop'
SHELL = DESK / 'shell'
NODE = shutil.which('node')


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def js(*cs):
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False) as f:
        json.dump({'calls': list(cs)}, f)
    r = subprocess.run([NODE, str(HERE / 'search_harness.js'), str(SHELL / 'search-bar.js'), f.name], capture_output=True, text=True, timeout=60)
    pathlib.Path(f.name).unlink()
    if r.returncode:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout)


def read(p):
    return (SHELL / p).read_text(encoding='utf-8')


@unittest.skipUnless(NODE, 'node is not installed')
class Logic(unittest.TestCase):
    RESULT = {
        'best': {'kind': 'app', 'item': {'name': 'Firefox', 'id': 'firefox'}},
        'answers': [{'plugin': 'calc', 'group': 'Calculator', 'title': '4'}, {'plugin': 'web', 'group': 'Web', 'title': 'Open'},
                    {'plugin': 'newone', 'group': 'New plugin', 'title': 'x'}],
        'apps': [{'name': 'Firefox', 'id': 'firefox'}, {'name': 'Files', 'id': 'files'}],
        'games': [], 'pages': [{'name': 'Wi-Fi', 'page': 'wifi'}], 'files': [{'name': 'fire.txt', 'path': '/x/fire.txt'}]}

    def test_order_keeps_yours_then_hearths_then_new(self):
        got = js(['search-bar.js', 'order', ['files', 'calc', 'files', 'Bad Name', 'apps'], ['calc', 'newone']])[0]
        self.assertEqual(got[:3], ['files', 'calc', 'apps'])
        self.assertEqual(got[-1], 'newone')          # a plugin Hearth doesn't know yet comes last
        self.assertEqual(len(got), len(set(got)))
        self.assertNotIn('Bad Name', got)
        self.assertEqual(js(['search-bar.js', 'order', None, None])[0][0], 'apps')

    def test_move(self):
        self.assertEqual(js(['search-bar.js', 'move', ['a', 'b', 'c'], 'b', -1])[0], ['b', 'a', 'c'])
        self.assertEqual(js(['search-bar.js', 'move', ['a', 'b', 'c'], 'a', -1])[0], ['a', 'b', 'c'])
        self.assertEqual(js(['search-bar.js', 'move', ['a', 'b', 'c'], 'c', 1])[0], ['a', 'b', 'c'])

    def test_arrange_best_first_then_your_order(self):
        rows = js(['search-bar.js', 'arrange', self.RESULT, ['files', 'web', 'pages'], 4])[0]
        self.assertEqual(rows[0]['group'], 'Best match')
        self.assertEqual(rows[0]['item']['name'], 'Firefox')
        groups = []
        for r in rows[1:]:
            if not groups or groups[-1] != r['plugin']:
                groups.append(r['plugin'])
        self.assertEqual(groups[:3], ['files', 'web', 'pages'])
        self.assertEqual(groups[-1], 'newone', 'a plugin the bar doesn\'t know yet still shows, last')
        self.assertEqual(sum(1 for r in rows if r['item'].get('name') == 'Firefox'), 1, 'the best match isn\'t shown twice')
        self.assertEqual([r['group'] for r in rows[1:4]], ['Files and folders', 'Web', 'Settings'])

    def test_help(self):
        rows = js(['search-bar.js', 'helpRows', '?', ['web'], {'emoji': True}])[0]
        self.assertEqual(rows[0]['plugin'], 'web')
        self.assertTrue(all(r['kind'] == 'help' for r in rows))
        emoji = [r for r in rows if r['plugin'] == 'emoji'][0]
        self.assertTrue(emoji['off'])
        self.assertIn('off', emoji['title'])
        self.assertEqual(emoji['fill'], ':')
        self.assertEqual([r['plugin'] for r in js(['search-bar.js', 'helpRows', '?currency', [], {}])[0]], ['currency'])
        self.assertEqual(js(['search-bar.js', 'isHelp', ' ?'], ['search-bar.js', 'isHelp', 'a?'])[0:2], [True, False])

    def test_alt_space_and_the_layout_switch(self):
        two = [{'layout': 'us'}, {'layout': 'de'}]
        got = js(['search-bar.js', 'layoutTakesAltSpace', {'switch': 'grp:alt_space_toggle', 'layouts': two}],
                 ['search-bar.js', 'layoutTakesAltSpace', {'switch': 'grp:alt_space_toggle', 'layouts': two[:1]}],
                 ['search-bar.js', 'layoutTakesAltSpace', {'switch': 'grp:alt_shift_toggle', 'layouts': two}],
                 ['search-bar.js', 'layoutTakesAltSpace', None])
        self.assertEqual(got, [True, False, False, False])

    def test_what_starts_a_search_on_the_desktop(self):
        got = js(['search-bar.js', 'startsSearch', 'f', 0, 0, 0], ['search-bar.js', 'startsSearch', 'F', 0, 0, 0],
                 ['search-bar.js', 'startsSearch', '2', 0, 0, 0], ['search-bar.js', 'startsSearch', ' ', 0, 0, 0],
                 ['search-bar.js', 'startsSearch', 'c', 1, 0, 0], ['search-bar.js', 'startsSearch', 'x', 0, 1, 0],
                 ['search-bar.js', 'startsSearch', '', 0, 0, 0], ['search-bar.js', 'startsSearch', '\r', 0, 0, 0])
        self.assertEqual(got, [True, True, True, False, False, False, False, False])


class Prefs(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        h = pathlib.Path(self.tmp.name)
        self.conf = h / '.config' / 'hearth' / 'search.json'
        self.env = dict(os.environ, HOME=str(h), XDG_CONFIG_HOME=str(h / '.config'), XDG_DATA_HOME=str(h / '.local/share'),
                        XDG_CACHE_HOME=str(h / '.cache'))

    def tearDown(self):
        self.tmp.cleanup()

    def run_h(self, *args, stdin=''):
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-search')] + list(args), capture_output=True, text=True,
                           env=dict(self.env, HEARTH_INPUT=stdin), timeout=60)
        last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else ''
        return r.returncode, last

    def test_defaults_and_nothing_written_by_status(self):
        code, out = self.run_h('status')
        self.assertEqual(code, 0)
        pr = json.loads(out)['prefs']
        self.assertEqual(pr['pluginOrder'], [])
        self.assertIs(pr['typeToSearch'], True)
        self.assertFalse(self.conf.exists(), 'reading the settings wrote them')

    def test_only_what_changes_is_saved(self):
        self.assertEqual(self.run_h('prefs-set', stdin='{"currencyOnline": false}')[0], 0)
        self.assertEqual(self.run_h('prefs-set', stdin='{"pluginOrder": ["files", "apps"]}')[0], 0)
        self.assertEqual(self.run_h('prefs-set', stdin='{"typeToSearch": false}')[0], 0)
        saved = json.loads(self.conf.read_text())
        self.assertEqual(saved, {'currencyOnline': False, 'pluginOrder': ['files', 'apps'], 'typeToSearch': False})
        self.assertEqual(self.run_h('prefs-set', stdin='{"pluginOrder": null}')[0], 0)
        self.assertNotIn('pluginOrder', json.loads(self.conf.read_text()))

    def test_bad_orders_are_refused_and_change_nothing(self):
        self.run_h('prefs-set', stdin='{"pluginOrder": ["web"]}')
        before = self.conf.read_text()
        for bad in ('"web"', '["web", "web"]', '["Not ok"]', '[1]', '{"a": 1}'):
            self.assertNotEqual(self.run_h('prefs-set', stdin='{"pluginOrder": %s}' % bad)[0], 0, bad)
        self.assertNotEqual(self.run_h('prefs-set', stdin='{"typeToSearch": "yes"}')[0], 0)
        self.assertEqual(self.conf.read_text(), before)


class Keys(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.hs = load('hearth_system_sb', SHELL / 'hearth-system')
        cls.rc = (DESK / 'labwc' / 'rc.xml').read_text()
        root = ET.fromstring(cls.rc)
        cls.binds = {b.get('key'): b.findall('action') for b in root.find('keyboard').findall('keybind')}

    def test_labwc_binds(self):
        self.assertIn("call searchbar key", self.binds['A-space'][0].get('command'))
        self.assertIn("call searchbar toggle", self.binds['A-F2'][0].get('command'))
        self.assertEqual(self.binds['A-F3'][0].get('name'), 'ShowMenu', 'labwc\'s window menu moved to Alt+F3, as on KDE')

    def test_layout_switch_wins_alt_space_on_labwc(self):
        rc = self.hs.rc_with_search_key(self.rc, {'XKB_DEFAULT_OPTIONS': 'caps:none,grp:alt_space_toggle'})
        b = {x.get('key'): x.findall('action') for x in ET.fromstring(rc).find('keyboard').findall('keybind')}
        self.assertEqual([a.get('name') for a in b['A-space']], ['None'], 'Alt+Space must reach xkb (no labwc default either)')
        self.assertIn('call searchbar toggle', b['A-F2'][0].get('command'), 'Alt+F2 still opens the bar')
        self.assertEqual(self.hs.rc_with_search_key(self.rc, {'XKB_DEFAULT_OPTIONS': 'grp:alt_shift_toggle'}), self.rc)
        self.assertEqual(self.hs.rc_with_search_key(self.rc, {}), self.rc)
        listed = {x['key']: x['action'] for x in self.hs.parse_keybinds(rc)}
        self.assertNotIn('A-space', listed)
        self.assertNotIn('A-Space', listed, 'labwc\'s own Alt+Space window menu isn\'t listed: the key is the layout switch\'s')
        listed = {x['key'].lower(): x['action'] for x in self.hs.parse_keybinds(self.rc)}
        self.assertIn('Search bar', listed['a-space'])

    def test_labwc_config_uses_it(self):
        src = read('hearth-system')
        self.assertIn('rc = rc_with_search_key(rc, parse_env(env))', src)

    def test_kwin_has_the_same_keys(self):
        t = json.loads((DESK / 'kwin' / 'windows' / 'shortcuts.json').read_text())
        s = {x['id']: x for x in t['shortcuts']}
        self.assertEqual(s['searchbar']['keys'], 'Alt+Space')
        self.assertEqual(s['searchbar-2']['keys'], 'Alt+F2')
        self.assertIn({'labwc': 'A-F3', 'kwin': 'Window Operations Menu', 'keys': 'Alt+F3'}, t['kwinOwn'])

    def test_the_bar_ignores_alt_space_when_it_switches_layouts(self):
        src = read('SearchBar.qml')
        self.assertIn('function key() { if (altSpaceIsLayout) return; toggle(); }', src)
        self.assertIn('SB.layoutTakesAltSpace(kb)', src)
        self.assertIn('/nimbus/keyboard.json', src)
        self.assertIn('Alt+F2', read('SetKeyboard.qml'))


class Wiring(unittest.TestCase):
    def test_loaded_by_file_name(self):
        src = read('shell.qml')
        self.assertIn('source: "SearchBar.qml"', src)
        self.assertNotIn('SearchBar {', src, 'a problem in the bar must never cost the shell')

    def test_controller(self):
        src = read('Pad.qml')
        self.assertIn('|| SettingsNav.searchBarShown   // the search bar', src)
        self.assertIn('SettingsNav.winxShown || SettingsNav.searchBarShown', src)
        self.assertIn('!pad.deskY()', src)
        self.assertIn('b.padOpen()', src)
        self.assertIn('property bool searchBarShown: false', read('SettingsNav.qml'))

    def test_type_on_the_desktop(self):
        src = read('Desktop.qml')
        self.assertIn('import "search-bar.js" as SB', src)
        self.assertIn('SettingsNav.searchBar.openWith(event.text)', src)
        self.assertIn('function openWith(t) { if (!typeToSearch) return false;', read('SearchBar.qml'))

    def test_settings_page(self):
        src = read('SetSearch.qml')
        self.assertIn('title: "Search bar (Alt+Space)"', src)
        self.assertIn('page.set({ pluginOrder: l }', src)
        self.assertIn('page.set({ typeToSearch: v })', src)
        self.assertIn('pluginOrder: null', src)
        idx = read('settings-index.js')
        self.assertIn('"t": "Type on the desktop to search"', idx)

    def test_uses_the_search_framework(self):
        src = read('SearchBar.qml')
        for x in ('launcher.searchAll(text, "all")', 'launcher.openRow(r)', 'launcher.findFiles(text)', 'SB.helpRows(', 'target: "searchbar"'):
            self.assertIn(x, src)
        self.assertNotIn('SettingsWriter', src, 'the bar writes nothing')


if __name__ == '__main__':
    unittest.main()
