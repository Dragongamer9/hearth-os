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
    look = (SHELL / 'block-look.js').read_text().replace('.pragma library', '')
    src = ('var Tools = (function () {' + tools + '\nreturn {barItem: barItem, panelItem: panelItem, panelOf: panelOf, '
           'barLayout: barLayout, panelClash: panelClash, BAR_ITEMS: BAR_ITEMS, PANEL_ITEMS: PANEL_ITEMS, baseId: baseId, '
           'barLook: barLook, barSide: barSide, lengthOf: lengthOf, onScreen: onScreen, lineup: lineup}; })();\n'
           'var L = (function () {' + layout + '\nreturn {info: info, CATALOG: CATALOG}; })();\n'
           'var BL = (function () {' + look + '\nreturn {DEFAULT: DEFAULT, normalize: normalize, sparse: sparse, merge: merge, '
           'copyLook: copyLook, pasteLook: pasteLook, applyToAll: applyToAll, keepPlan: keepPlan, values: values, '
           'fillWords: fillWords, shownHere: shownHere, replaces: replaces, labels: labels, content: content, action: action, '
           'command: command, something: something, argb: argb, takes: takes}; })();\n' + edit +
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


@unittest.skipUnless(NODE, 'node is needed for the editor rules')
class OneBlockPicked(unittest.TestCase):
    """Board 2: a block's look (block-look.js) and the editor's rules for one block picked (desktop-edit.js)."""
    def test_a_look_has_its_defaults_and_a_bad_record_is_the_defaults(self):
        r = run('''var a = BL.normalize(null); var b = BL.normalize({w: "huge", wpx: "x", see: 400, corners: 50, fill: "red",
                   edge: "glow", desktops: [2, "x", 99], hidden: "yes"});
                   console.log(JSON.stringify({a: a, b: b, none: BL.sparse(null), junk: BL.sparse([1, 2]), d: BL.DEFAULT}));''')
        self.assertEqual(r['a'], r['d'])
        b = r['b']
        self.assertEqual((b['w'], b['wpx'], b['see'], b['corners'], b['fill'], b['edge'], b['desktops'], b['hidden']),
                         ('fit', 0, 100, 20, '', 'glow', [2], False))
        self.assertIsNone(r['none'], 'nothing set: nothing kept (the key is taken out)')
        self.assertIsNone(r['junk'])

    def test_merging_keeps_only_what_differs(self):
        r = run('''var a = BL.merge({fill: "#3b2a1d"}, {w: "fixed", wpx: 158, nonsense: 1});
                   var back = BL.merge(a, {w: "fit", wpx: 0, fill: ""});
                   console.log(JSON.stringify({a: BL.sparse(a), back: BL.sparse(back)}));''')
        self.assertEqual(r['a'], {'fill': '#3b2a1d', 'w': 'fixed', 'wpx': 158})
        self.assertIsNone(r['back'])

    def test_copy_and_paste_a_look_take_only_the_look(self):
        r = run('''var from = {fill: "#3b2a1d", edge: "line", corners: 99, words: "Downloads · {count}", doPress: "run",
                              argPress: "firefox", when: "hover", hidden: true};
                   var clip = BL.copyLook(from);
                   var to = BL.pasteLook({doRight: "nothing", see: 40}, clip);
                   console.log(JSON.stringify({clip: clip, to: BL.sparse(to)}));''')
        self.assertNotIn('doPress', r['clip'])
        self.assertNotIn('when', r['clip'])
        self.assertNotIn('hidden', r['clip'])
        t = r['to']
        self.assertEqual((t['fill'], t['edge'], t['corners'], t['words']), ('#3b2a1d', 'line', 99, 'Downloads · {count}'))
        self.assertEqual(t['doRight'], 'nothing', 'what it does stays its own')
        self.assertNotIn('see', t, 'the pasted look is the whole look: its see-through goes back to none')

    def test_apply_to_every_bar_item(self):
        r = run('''var looks = {jobs: {fill: "#3b2a1d", edge: "line", doPress: "run", argPress: "x"}, volume: {doScroll: "nothing"}};
                   var out = BL.applyToAll(looks, ["jobs", "volume", "spacer", "battery"], looks.jobs);
                   console.log(JSON.stringify(out));''')
        self.assertEqual(r['volume']['fill'], '#3b2a1d')
        self.assertEqual(r['volume']['doScroll'], 'nothing')
        self.assertEqual(r['battery'], {'fill': '#3b2a1d', 'edge': 'line'})
        self.assertNotIn('spacer', r)
        self.assertEqual(r['jobs']['argPress'], 'x')

    def test_a_file_that_cant_be_read_is_never_saved_over(self):
        r = run('''console.log(JSON.stringify({bad: BL.keepPlan(false, {"s:jobs": {fill: "#000000"}}),
                   ok: BL.keepPlan(true, {"s:jobs": {fill: "#000000"}}), none: BL.keepPlan(true, {})}));''')
        self.assertFalse(r['bad']['write'])
        self.assertIn("couldn't be read", r['bad']['why'])
        self.assertTrue(r['ok']['write'])
        self.assertFalse(r['none']['write'])
        q = (SHELL / 'DesktopEditor.qml').read_text()
        self.assertIn('BL.keepPlan(BarLayout.readOk, c.bar)', q)
        b = (SHELL / 'BarLayout.qml').read_text()
        self.assertIn('bl.readOk = false; writer.repair()', b)

    def test_keep_this_writes_each_look_under_its_own_key_and_the_bars_settings(self):
        r = run(DRAFT + '''D.looks = {jobs: {fill: "#111111"}, volume: {edge: "line"}}; D.bar = {};
                           var base = JSON.parse(JSON.stringify(D)); var d = JSON.parse(JSON.stringify(D));
                           d.looks.jobs = {fill: "#222222"}; delete d.looks.volume; d.bar = {edge: "free", fx: 0.5};
                           console.log(JSON.stringify(changes(base, d).bar));''')
        self.assertEqual(r['s:jobs'], {'fill': '#222222'})
        self.assertIsNone(r['s:volume'])
        self.assertEqual(r['bar']['edge'], 'free')
        self.assertNotIn('items', r)

    def test_duplicate_move_to_and_take_it_out(self):
        r = run(DRAFT + '''D.looks = {volume: {fill: "#3b2a1d"}};
                           var d = duplicate(D, "volume", {kind: "main", zone: "end"}, "k1");
                           var i = d.main.end.indexOf("volume");
                           var m = moveTo(d, "volume#k1", {kind: "main", zone: "end"}, {kind: "panel", id: "a"}, L.info, newId);
                           var t = takeOut(m, "status#k1", {kind: "panel", id: "a"});
                           var D2 = JSON.parse(JSON.stringify(D)); D2.own = {b1: {kind: "button", label: "Hi"}}; D2.main.start = ["own:b1"];
                           var own = duplicate(D2, "own:b1", {kind: "main", zone: "start"}, "b2");
                           var targets = moveTargets(D, "volume", {kind: "main", zone: "end"}, "top", L.info);
                           console.log(JSON.stringify({next: d.main.end[i + 1], look: d.looks["volume#k1"], kept: Tools.barLayout(d.main).end,
                               panel: m.panels.a.items, mlook: m.looks["status#k1"], t: t.panels.a.items, tl: t.looks["status#k1"] || null,
                               hid: t.main.hidden, own: own.main.start, ownDef: own.own.b2, targets: targets,
                               p: Tools.panelOf("a", m.panels.a).items}));''')
        self.assertEqual(r['next'], 'volume#k1')
        self.assertEqual(r['look'], {'fill': '#3b2a1d'}, 'a copy has the same look')
        self.assertIn('volume#k1', r['kept'], 'the bar keeps a copy where it is')
        self.assertIn('status#k1', r['panel'], 'moved into a panel under the name it has there')
        self.assertIn('status#k1', r['p'])
        self.assertEqual(r['mlook'], {'fill': '#3b2a1d'}, 'its look goes with it')
        self.assertNotIn('status#k1', r['t'])
        self.assertIsNone(r['tl'])
        self.assertNotIn('volume#k1', r['hid'], 'a copy taken out is gone, not kept hidden')
        self.assertEqual(r['own'], ['own:b1', 'own:b2'])
        self.assertEqual(r['ownDef']['label'], 'Hi')
        labels = [t['label'] for t in r['targets']]
        self.assertIn('Bottom panel', labels)
        self.assertIn('Desktop', labels)
        self.assertFalse([t for t in r['targets'] if t['label'] == 'Desktop'][0]['ok'], 'the desktop only takes widgets')

    def test_its_words_values_and_when_it_shows(self):
        r = run('''var v = BL.values("jobs", {jobs: 3, jobsPct: 40, time: "12:16 AM"});
                   var c = BL.content("jobs", {words: "Downloads · {count}"}, v, {icon: "sync", name: "Copies and downloads"});
                   console.log(JSON.stringify({v: v, w: BL.fillWords("Downloads · {count} · {percent} {nope}", v), c: c,
                     rep: BL.replaces("jobs", {words: "x"}), trayRep: BL.replaces("tray", {words: "x"}), trayLab: BL.labels("tray", {words: "x"}),
                     plain: BL.replaces("jobs", null),
                     hid: BL.shownHere({hidden: true}, {}), ed: BL.shownHere({hidden: true}, {editing: true}),
                     scr: BL.shownHere({screen: "HDMI-1"}, {screen: "DP-1", screens: ["DP-1", "HDMI-1"]}),
                     gone: BL.shownHere({screen: "HDMI-9"}, {screen: "DP-1", screens: ["DP-1"]}),
                     desk: BL.shownHere({desktops: [2]}, {desk: 1}), desk2: BL.shownHere({desktops: [2]}, {desk: 2}),
                     apps: BL.shownHere({when: "apps", apps: "firefox, org.kde.dolphin"}, {app: "firefox"}),
                     apps2: BL.shownHere({when: "apps", apps: "firefox"}, {app: "foot"}),
                     some: BL.something("jobs", BL.values("jobs", {jobs: 0})), some2: BL.something("jobs", v)}));''')
        self.assertEqual(r['w'], 'Downloads · 3 · 40% {nope}')
        self.assertEqual(r['c']['words'], 'Downloads · 3')
        self.assertEqual(r['c']['icon'], 'sync')
        self.assertTrue(r['rep'])
        self.assertFalse(r['trayRep'], 'the tray keeps its own icons')
        self.assertTrue(r['trayLab'], 'with your words before them')
        self.assertFalse(r['plain'])
        self.assertFalse(r['hid'])
        self.assertTrue(r['ed'], 'the editor shows a hidden block (faded)')
        self.assertFalse(r['scr'])
        self.assertTrue(r['gone'], "a screen that isn't plugged in never hides it")
        self.assertFalse(r['desk'])
        self.assertTrue(r['desk2'])
        self.assertTrue(r['apps'])
        self.assertFalse(r['apps2'])
        self.assertFalse(r['some'])
        self.assertTrue(r['some2'])

    def test_what_it_does(self):
        r = run('''var l = {doPress: "run", argPress: "notify-send hi", doRight: "app", argRight: "firefox",
                            doMiddle: "panel", argMiddle: "quick", doScroll: "nothing", doHover: "panel", argHover: "bogus"};
                   console.log(JSON.stringify({p: BL.command(BL.action(l, "Press"), "/s"), r: BL.command(BL.action(l, "Right"), "/s"),
                     m: BL.command(BL.action(l, "Middle"), "/s"), s: BL.action(l, "Scroll"), h: BL.command(BL.action(l, "Hover"), "/s"),
                     none: BL.action({}, "Press"), takes: BL.takes(l), nottakes: BL.takes({doHover: "run", argHover: "x"})}));''')
        self.assertEqual(r['p'], ['sh', '-c', 'notify-send hi'])
        self.assertEqual(r['r'], ['gtk-launch', 'firefox'])
        self.assertEqual(r['m'], ['quickshell', 'ipc', '-p', '/s', 'call', 'quick', 'toggle'])
        self.assertEqual(r['s'], {'kind': 'nothing', 'arg': ''})
        self.assertIsNone(r['h'], "only Hearth's own panels")
        self.assertIsNone(r['none'])
        self.assertTrue(r['takes'])
        self.assertFalse(r['nottakes'])

    def test_the_bars_own_settings(self):
        r = run('''var none = Tools.barLook(undefined); var b = Tools.barLook({edge: "free", fx: 0.5, fy: 0.9, orient: "across",
                     len: "pct", lenV: 60, offset: 40, thick: 52, corners: 99, fill: "#102030", see: 20, blur: true, gap: 4,
                     lineup: "centre", islands: "strip", autoHide: "on", over: true, layer: "bottom", screen: "DP-1"});
                   var p = Tools.panelOf("x", {edge: "top", items: ["clock", "panel:in1"], len: "px", lenV: 500, offset: 100, lineup: "end", islands: true, inside: ""});
                   var inner = Tools.panelOf("in1", {inside: "x", items: ["clock"]});
                   var old = Tools.panelOf("y", {edge: "bottom", items: [], style: "centred"});
                   console.log(JSON.stringify({none: none, b: b, side: Tools.barSide(b, "top"), sideL: Tools.barSide({edge: "free", orient: "down", fx: 0.1}, "top"),
                     p: p, inner: inner, old: old.len, len: [Tools.lengthOf("full", 0, 1920, 300), Tools.lengthOf("fit", 0, 1920, 300),
                     Tools.lengthOf("pct", 50, 1920, 300), Tools.lengthOf("px", 700, 1920, 300)],
                     line: [Tools.lineup(["a", "b"], "start"), Tools.lineup(["a", "b"], "centre"), Tools.lineup(["a", "b"], "end"), Tools.lineup(["a", "b"], "spread")],
                     on: [Tools.onScreen("", "A", ["A"]), Tools.onScreen("B", "A", ["A", "B"]), Tools.onScreen("Z", "A", ["A"])]}));''')
        self.assertEqual(r['none']['edge'], '', 'nothing set: the bar as Settings has it')
        b = r['b']
        self.assertEqual((b['edge'], b['len'], b['lenV'], b['thick'], b['corners'], b['fill'], b['see'], b['blur'], b['gap'],
                          b['lineup'], b['islands'], b['autoHide'], b['over'], b['layer'], b['screen']),
                         ('free', 'pct', 60, 52, 99, '#102030', 20, True, 4, 'centre', 'strip', 'on', True, 'bottom', 'DP-1'))
        self.assertEqual(r['side'], 'bottom')
        self.assertEqual(r['sideL'], 'left')
        self.assertEqual((r['p']['len'], r['p']['lenV'], r['p']['offset'], r['p']['lineup'], r['p']['islands']), ('px', 500, 100, 'end', True))
        self.assertIn('panel:in1', r['p']['items'], 'a panel inside a panel')
        self.assertEqual(r['inner']['inside'], 'x')
        self.assertEqual(r['old'], 'fit', 'a centred panel is as long as its blocks, as before')
        self.assertEqual(r['len'], [1920, 300, 960, 700])
        self.assertEqual(r['line'], [['a', 'b', '~'], ['~', 'a', 'b', '~'], ['~', 'a', 'b'], ['a', 'b']])
        self.assertEqual(r['on'], [True, False, True])

    def test_more_than_one_panel_on_an_edge_and_panels_inside(self):
        r = run(DRAFT + '''var d = addInner(D, "a", "in1"); var e = addInner(d, "main", "in2");
                           d.panels.b = {edge: "bottom", items: ["clock"], len: "fit", offset: 300};
                           var both = [Tools.panelOf("a", d.panels.a), Tools.panelOf("b", d.panels.b)].filter(Boolean).length;
                           console.log(JSON.stringify({a: d.panels.a.items, in1: d.panels.in1, main: e.main.end, kept: Tools.barLayout(e.main).end, both: both,
                             box: mainBox({edge: "top", len: "px", lenV: 600, offset: 100}, "top", 1920, 1080, 44, 0),
                             free: mainBox({edge: "free", orient: "across", len: "pct", lenV: 50, fx: 0.5, fy: 0.5}, "top", 1920, 1080, 44, 0)}));''')
        self.assertEqual(r['a'][-1], 'panel:in1')
        self.assertEqual(r['in1']['inside'], 'a')
        self.assertEqual(r['main'][0], 'panel:in2')
        self.assertIn('panel:in2', r['kept'])
        self.assertEqual(r['both'], 2)
        self.assertEqual((r['box']['x'], r['box']['w']), (100, 600))
        self.assertEqual((r['free']['x'], r['free']['y'], r['free']['w']), (480, 518, 960))

    def test_a_copy_is_drawn_as_its_block(self):
        r = run('''console.log(JSON.stringify({b: Tools.baseId("volume#k1"), own: Tools.baseId("own:k1"), item: Tools.barItem("volume#k1").id,
                   p: Tools.panelItem("clock#x2").id, bad: Tools.barItem("volume#BAD!")}));''')
        self.assertEqual((r['b'], r['own'], r['item'], r['p']), ('volume', 'own:k1', 'volume', 'clock'))
        self.assertIsNone(r['bad'])


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

    def test_board_2_is_all_there(self):
        for s in ['"Customize…"', '"Copy its look"', '"Paste a look"', '"Duplicate"', '"Move to ›"', '"Hide (keep it here)"',
                  '"Take it out"', 'Right-click any block · or pick it and use the panel →', '"Reset"', '"Look"', '"Shows"',
                  '"Does"', '"Place"', '"When"', '"Motion"', 'SIZE AND PLACE', '"Width"', '"Fit"', '"Fixed"', '"Fill"',
                  '"Height"', '"Space around it"', '"Space inside"', 'BACKGROUND AND EDGE', '"See-through"', '"Blur behind it"',
                  '"Corners"', '"Edge"', '"None"', '"Line"', '"Glow"', '"Shadow"', '"Soft"', '"Deep"', 'ICON AND WORDS',
                  '"Icon"', '"Words"', '"Font"', '"Light"', '"Medium"', '"Bold"', '"Show"', '"Both"',
                  "WHEN IT'S POINTED AT · PRESSED", '"Hover look"', '"Same"', '"Lighter"', '"Accent"', '"Own…"',
                  '"Pressed look"', '"Darker"', 'THE OTHER TABS', '"Save as my own block…"', '"Apply to every bar item"',
                  'Pick the bar itself (click its empty space) and it has its own:', '"Floating anywhere"',
                  '"Fit to its blocks"', '"Offset from the corner"', '"Thickness"', '"Gap between blocks"', '"Line up blocks"',
                  '"Islands"', '"One strip"', '"Hide until pointed at"', '"Keep their space"', '"Which screen"',
                  '+ Another panel on this edge', '+ A panel inside this one', '"Open an app"', '"Run a command"',
                  '"Show a panel"', '"Nothing"', '"Every desktop"', '"Above windows"', '"Under windows"', '"Only on hover',
                  "Only when there's something to show", '"Only in some apps"', '"Fades in"', '"Bounces when something new comes"']:
            self.assertIn(s, self.q, s)

    def test_every_block_on_the_real_desktop_is_drawn_in_its_look(self):
        for f in ('Islands.qml', 'ExtraPanels.qml', 'NestedPanel.qml'):
            self.assertIn('BlockLook {', (SHELL / f).read_text(), f)
        self.assertIn('BarLayout.lookOf(bid)', (SHELL / 'BlockLook.qml').read_text())
        self.assertIn('lookOverride: c.styled ? c.lk : null', self.q, 'the editor draws them with the same wrapper')
        for f in ('BlockLook.qml', 'DesktopEditor.qml', 'NestedPanel.qml', 'PanelBlocks.qml', 'ExtraPanels.qml', 'Islands.qml'):
            self.assertNotRegex((SHELL / f).read_text(), r'pixelSize: *\d+\.\d', f + ': no fractional font sizes')

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
