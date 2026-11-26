"""The desktop editor (drag and drop, board 1; docs/mockups/checklists/drag-drop.md): its rules in desktop-edit.js run
with node, and the QML is read for the parts of the board that can only be seen on a screen (the desktop test
pictures them)."""
import json
import pathlib
import re
import shutil
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
NODE = shutil.which('node')


def run(body):
    tools = (SHELL / 'widget-tools.js').read_text().replace('.pragma library', '')
    layout = (SHELL / 'widget-layout.js').read_text().replace('.pragma library', '')
    edit = (SHELL / 'desktop-edit.js').read_text().replace('.pragma library', '')
    src = ('var Tools = (function () {' + tools + '\nreturn {barItem: barItem, panelItem: panelItem, panelOf: panelOf, '
           'barLayout: barLayout, panelClash: panelClash, BAR_ITEMS: BAR_ITEMS, PANEL_ITEMS: PANEL_ITEMS}; })();\n'
           'var L = (function () {' + layout + '\nreturn {info: info, CATALOG: CATALOG}; })();\n' + edit +
           '\nuse(Tools);\n' + body)
    r = subprocess.run([NODE, '-e', src], capture_output=True, text=True, timeout=30)
    if r.returncode != 0:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout.strip().splitlines()[-1])


DRAFT = '''var D = {main: Tools.barLayout(null), panels: {a: {edge: "bottom", items: ["menu", "clock"], size: 44}},
               widgets: {w1: {type: "clock", x: 100, y: 100, w: 300, h: 220}}, own: {}};
           var n = 0; function newId() { n++; return "new" + n; }'''


@unittest.skipUnless(NODE, 'node is needed for the editor rules')
class Rules(unittest.TestCase):
    def test_a_block_goes_into_the_main_bar_where_it_is_dropped(self):
        r = run(DRAFT + '''var d = drop(D, "quickaccess", {kind: "lib"}, {kind: "main", zone: "end", index: 1}, L.info, newId);
                           console.log(JSON.stringify(d.main));''')
        self.assertEqual(r['end'][1], 'quickaccess')
        self.assertNotIn('quickaccess', r['hidden'])

    def test_moving_along_the_same_bar_keeps_the_place(self):
        r = run(DRAFT + '''var s = D.main.start.slice(); var id = s[0];
                           var d = drop(D, id, {kind: "main", zone: "start"}, {kind: "main", zone: "start", index: 2}, L.info, newId);
                           console.log(JSON.stringify({before: s, after: d.main.start}));''')
        b, a = r['before'], r['after']
        self.assertEqual(a.index(b[0]), 1, a)
        self.assertEqual(sorted(a), sorted(b))

    def test_a_block_moves_from_a_panel_to_the_main_bar_with_its_bar_name(self):
        r = run(DRAFT + '''D.panels.a.items.push("pager");
                           var d = drop(D, "pager", {kind: "panel", id: "a"}, {kind: "main", zone: "start", index: 0}, L.info, newId);
                           console.log(JSON.stringify({main: d.main.start, panel: d.panels.a.items}));''')
        self.assertEqual(r['main'][0], 'desktops')
        self.assertNotIn('pager', r['panel'])

    def test_what_a_place_cant_hold_is_refused_with_a_reason(self):
        r = run(DRAFT + '''var f = fit("clock", {kind: "main"}, L.info); var g = fit("tray", {kind: "desk"}, L.info);
                           var d = drop(D, "tray", {kind: "main", zone: "end"}, {kind: "desk", x: 5, y: 5}, L.info, newId);
                           console.log(JSON.stringify({f: f, g: g, d: d}));''')
        self.assertEqual(r['f']['id'], '')
        self.assertIn('panel', r['f']['why'])
        self.assertIn('Only widgets', r['g']['why'])
        self.assertIsNone(r['d'])

    def test_a_widget_moves_from_the_desktop_into_a_panel_and_back(self):
        r = run(DRAFT + '''D.widgets.w1.type = "weather";
                           var d = drop(D, "widget:weather", {kind: "desk", id: "w1"}, {kind: "panel", id: "a", index: 1}, L.info, newId);
                           var e = drop(d, "widget:weather", {kind: "panel", id: "a"}, {kind: "desk", x: 640, y: 320}, L.info, newId);
                           console.log(JSON.stringify({d: d, e: e}));''')
        self.assertNotIn('w1', r['d']['widgets'])
        self.assertIn('widget:weather', r['d']['panels']['a']['items'])
        new = [w for k, w in r['e']['widgets'].items() if k.startswith('new')]
        self.assertEqual(new[0]['type'], 'weather')
        self.assertEqual((new[0]['x'], new[0]['y']), (640, 320))

    def test_your_own_block_goes_anywhere_a_bar_or_panel_is(self):
        r = run(DRAFT + '''D.own.k1 = ownBlock("button");
                           var d = drop(D, "own:k1", {kind: "lib"}, {kind: "main", zone: "start", index: 1}, L.info, newId);
                           var e = drop(d, "own:k1", {kind: "main", zone: "start"}, {kind: "panel", id: "a", index: 0}, L.info, newId);
                           var l = Tools.barLayout(d.main);
                           console.log(JSON.stringify({d: d.main.start, kept: l.start, e: e.panels.a.items, hid: e.main.hidden,
                                                       p: Tools.panelOf("a", e.panels.a).items}));''')
        self.assertEqual(r['d'][1], 'own:k1')
        self.assertIn('own:k1', r['kept'], 'the bar keeps your block where you put it')
        self.assertEqual(r['e'][0], 'own:k1')
        self.assertNotIn('own:k1', r['hid'], 'your block is only where you put it')
        self.assertEqual(r['p'][0], 'own:k1')

    def test_panels_go_anywhere_and_an_edge_is_only_a_snap(self):
        r = run('''var free = placePanel({items: ["clock"]}, {x: 700, y: 400, w: 300, h: 44}, 1920, 1080, true);
                   var down = placePanel({items: ["clock"]}, {x: 900, y: 200, w: 56, h: 500}, 1920, 1080, true);
                   var top = placePanel({items: ["clock"]}, {x: 0, y: 10, w: 1920, h: 40}, 1920, 1080, true);
                   var nosnap = placePanel({items: ["clock"]}, {x: 0, y: 10, w: 1920, h: 40}, 1920, 1080, false);
                   var p = Tools.panelOf("x", free); var box = panelBox(p, 1920, 1080, 120);
                   var clash = Tools.panelClash([Tools.panelOf("y", free)], "", "bottom", "M", "top", false);
                   console.log(JSON.stringify({free: free, down: down, top: top, nosnap: nosnap, p: p, box: box, clash: clash}));''')
        f = r['free']
        self.assertEqual(f['place'], 'free')
        self.assertEqual(f['orient'], 'across')
        self.assertEqual(f['length'], 300)
        self.assertEqual(r['down']['orient'], 'down')
        self.assertEqual(r['down']['size'], 56)
        self.assertEqual(r['top']['edge'], 'top')
        self.assertNotEqual(r['top'].get('place'), 'free')
        self.assertEqual(r['nosnap']['place'], 'free')
        self.assertTrue(r['p']['free'])
        self.assertEqual((r['box']['x'], r['box']['y'], r['box']['w']), (700, 400, 300))
        self.assertEqual(r['clash'], '', 'a panel anywhere never takes an edge')

    def test_snapping_to_the_grid_and_lining_up(self):
        r = run('''var a = snap({x: 103, y: 205, w: 50, h: 50}, [], 8, true);
                   var b = snap({x: 1297, y: 405, w: 200, h: 200}, [{x: 1300, y: 120, w: 420, h: 210, name: "the clock"}], 8, true);
                   var c = snap({x: 103, y: 205, w: 50, h: 50}, [], 0, false);
                   console.log(JSON.stringify({a: a, b: b, c: c}));''')
        self.assertEqual((r['a']['x'], r['a']['y']), (104, 208))
        self.assertEqual(r['b']['x'], 1300)
        self.assertEqual(r['b']['note'], 'Lined up with the clock')
        self.assertEqual(r['b']['guides'][0], {'x': 1300})
        self.assertEqual((r['c']['x'], r['c']['y']), (103, 205))

    def test_keep_this_writes_only_what_changed(self):
        r = run(DRAFT + '''var base = JSON.parse(JSON.stringify(D));
                           var none = changes(base, D);
                           var d = drop(D, "widget:weather", {kind: "lib"}, {kind: "desk", x: 10, y: 20}, L.info, newId);
                           d.own.k = ownBlock("text"); delete d.panels.a;
                           var c = changes(base, d);
                           console.log(JSON.stringify({none: nothingChanged(none), c: c}));''')
        self.assertTrue(r['none'])
        c = r['c']
        self.assertNotIn('items', c['bar'])
        self.assertIsNone(c['bar']['p:a'])
        self.assertEqual(c['bar']['b:k']['kind'], 'text')
        self.assertEqual(c['widgets']['add'][0]['rec']['type'], 'weather')
        self.assertEqual(c['widgets']['remove'], [])

    def test_the_blocks_window_lists_every_kind_and_finds_by_name(self):
        r = run('''var l = library(Tools.BAR_ITEMS, Tools.PANEL_ITEMS, L.CATALOG, {k: ownBlock("button")});
                   console.log(JSON.stringify({kinds: l.map(b => b.kind), names: l.map(b => b.name),
                                               found: find(l, "all", "quick").map(b => b.id), w: find(l, "widgets", "").length}));''')
        for k in ('bar', 'widgets', 'launchers', 'info', 'media', 'own'):
            self.assertIn(k, r['kinds'])
        self.assertIn('quickaccess', r['found'])
        self.assertGreater(r['w'], 4)
        self.assertIn('My button', r['names'])


class Editor(unittest.TestCase):
    """What the board shows, read from the editor (the desktop test pictures it on a screen)."""
    def setUp(self):
        self.q = (SHELL / 'DesktopEditor.qml').read_text()

    def test_the_board_is_all_there(self):
        for s in ['"Blocks"', '"drag onto anything"', 'Find a block (', '"Bar items"', '"Widgets"', '"Launchers"', '"Info"',
                  '"Media"', '"Your own"', '"Made by your AI"', 'BAR ITEMS', 'WIDGETS · ANYWHERE ON THE DESKTOP', 'MAKE YOUR OWN',
                  'Button · runs anything', 'Text · picture · spacer', '"Drop: "', 'Drag here for a panel', '"Undo"', '"Redo"',
                  'Snap to grid', '"Guides"', '"Layers"', '"Screen: "', '"Panel"', '"Start from: "', '"Cancel"', '"Try it"',
                  '"Keep this"', 'EDITING YOUR DESKTOP · NOTHING IS KEPT UNTIL KEEP THIS · ESC CANCELS']:
            self.assertIn(s, self.q, s)

    def test_it_writes_only_through_the_safe_writers(self):
        self.assertNotRegex(self.q, r'FileView|writeFile|>\s*"\$')
        self.assertIn('BarLayout.put(', self.q)
        self.assertIn('WidgetStore.', self.q)

    def test_it_is_loaded_on_its_own_so_a_problem_only_loses_the_editor(self):
        s = (SHELL / 'shell.qml').read_text()
        self.assertRegex(s, r'source:\s*"DesktopEditor.qml"')
        self.assertIn('Mantel: the desktop editor couldn\'t load', s)


if __name__ == '__main__':
    unittest.main()
