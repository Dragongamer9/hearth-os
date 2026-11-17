"""Desktop widgets and Activities (1.14, docs/WIDGETS.md): the layout rules (widget-layout.js), the activities rules
(activities.js), the layout store (widgets.json / activities.json through hearth_settings_io.py, as SettingsWriter
runs it: one widget merged at a time, nothing lost, a damaged file put back, never replaced by defaults), the
helper (hearth-widgets: places, weather, feeds, folders, photos, zones) and the QML wiring.
The JavaScript rules run with node, as in test_shell_extras.py."""
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
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
IO = SHELL / 'hearth_settings_io.py'
NODE = shutil.which('node')


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def run_js(lib, body):
    src = (SHELL / lib).read_text().replace('.pragma library', '')
    r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=30)
    if r.returncode != 0:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout.strip().splitlines()[-1])


def code(s):
    return re.sub(r'(?m)(^|\s)//.*$', '', s)


@unittest.skipUnless(NODE, 'node is needed for the widget rules')
class LayoutRules(unittest.TestCase):
    def test_parse_keeps_what_it_cant_show_out_of_its_list(self):
        r = run_js('widget-layout.js', '''console.log(JSON.stringify(parse({
            "w:a": {type: "clock", screen: "DP-1", x: 10, y: 20, w: 50, h: 50, cfg: {style: "analog"}},
            "w:b": {type: "hologram", x: 1},
            "w:c": "not a widget",
            "w:d": {type: "monitor", place: "bar"},
            "w:e": {type: "notes", place: "bar"},
            "other": 1})));''')
        ids = [w['id'] for w in r['widgets']]
        self.assertEqual(sorted(ids), ['a', 'd'])
        self.assertEqual(r['skipped'], 3)          # unknown type, not an object, a note in the bar
        a = next(w for w in r['widgets'] if w['id'] == 'a')
        self.assertEqual((a['w'], a['h']), (180, 130))   # pulled up to the clock's smallest size (only as shown)
        self.assertEqual(a['activity'], 'default')
        self.assertEqual(a['cfg'], {'style': 'analog'})

    def test_entry_keeps_fields_a_newer_hearth_added(self):
        r = run_js('widget-layout.js', '''console.log(JSON.stringify(entryFor({"w:a": {type: "clock", x: 1, future: {k: 1}}}, "a", {x: 50})));''')
        self.assertEqual(r, {'type': 'clock', 'x': 50, 'future': {'k': 1}})

    def test_a_missing_screen_shows_on_the_main_one_without_moving(self):
        r = run_js('widget-layout.js', '''var ws = parse({"w:a": {type: "clock", screen: "HDMI-A-1", x: 3000, y: 10},
            "w:b": {type: "clock", screen: "eDP-1", x: 5, y: 5}, "w:c": {type: "clock", screen: "eDP-1", activity: "work"},
            "w:d": {type: "clock", screen: "eDP-1", activity: "*"}}).widgets;
            console.log(JSON.stringify([forScreen(ws, "eDP-1", ["eDP-1"], "eDP-1", "default").map(w => w.id),
                                        forScreen(ws, "eDP-1", ["eDP-1", "HDMI-A-1"], "eDP-1", "default").map(w => w.id),
                                        forScreen(ws, "eDP-1", ["eDP-1"], "eDP-1", "work").map(w => w.id),
                                        shown(ws[0], 1280, 800, {top: 40}), ws[0].x]));''')
        self.assertEqual(sorted(r[0]), ['a', 'b', 'd'])
        self.assertEqual(sorted(r[1]), ['b', 'd'])
        self.assertEqual(sorted(r[2]), ['c', 'd'])   # Main's widgets aren't on Work
        self.assertEqual(r[3]['x'], 1280 - 300)      # drawn inside the smaller screen
        self.assertEqual(r[4], 3000)                 # the saved place is unchanged

    def test_find_spot_doesnt_overlap(self):
        r = run_js('widget-layout.js', '''var rects = []; for (var i = 0; i < 6; i++) { var s = findSpot(rects, 300, 220, 1920, 1080, {top: 48, bottom: 70}); rects.push({x: s.x, y: s.y, w: 300, h: 220}); }
            var bad = 0; for (var a = 0; a < rects.length; a++) for (var b = a + 1; b < rects.length; b++) if (overlaps(rects[a], rects[b], 0)) bad++;
            console.log(JSON.stringify({bad: bad, first: rects[0], inside: rects.every(r => r.x >= 0 && r.y >= 48 && r.x + r.w <= 1920 && r.y + r.h <= 1080 - 70)}));''')
        self.assertEqual(r['bad'], 0)
        self.assertTrue(r['inside'])
        self.assertGreater(r['first']['x'], 1500)    # down the right side first

    def test_nudge_and_cycle(self):
        r = run_js('widget-layout.js', '''var w = {id: "a", type: "clock", x: 100, y: 100, w: 300, h: 220};
            console.log(JSON.stringify([nudge(w, -500, 0, false, 1920, 1080, {}), nudge(w, -1000, -1000, true, 1920, 1080, {}),
              cycle([{id: "a", x: 500, y: 10}, {id: "b", x: 10, y: 400}, {id: "c", x: 10, y: 10}], "c", 1),
              cycle([{id: "a", x: 500, y: 10}], "", -1), galleryMove(0, 0, 1, 4, 13), galleryMove(12, 1, 0, 4, 13)]));''')
        self.assertEqual(r[0]['x'], 0)
        self.assertEqual((r[1]['w'], r[1]['h']), (180, 130))
        self.assertEqual(r[2], 'a')
        self.assertEqual(r[3], 'a')
        self.assertEqual(r[4], 4)
        self.assertEqual(r[5], 12)

    def test_clocks_timers_and_calendar(self):
        r = run_js('widget-layout.js', '''var t = cityTime(Date.UTC(2026, 10, 16, 23, 30), 540);
            console.log(JSON.stringify([t.h, t.m, fmtTime(t, true), fmtTime({h: 0, m: 5}, false), dayWord(2, 1), fmtDuration(65000), fmtDuration(3723000),
              fmtStopwatch(1234), monthGrid(2026, 10)[0], monthGrid(2026, 10).length, weatherWord(61), weatherKind(0, false), pushSample([1, 2, 3], 4, 3)]));''')
        self.assertEqual(r[:3], [8, 30, '08:30'])
        self.assertEqual(r[3], '12:05 AM')
        self.assertEqual(r[4], 'Tomorrow')
        self.assertEqual(r[5:8], ['1:05', '1:02:03', '0:01.2'])
        self.assertEqual(r[8], {'y': 2026, 'm': 9, 'd': 26, 'inMonth': False})   # 1 Nov 2026 is a Sunday: Monday 26 Oct first
        self.assertEqual(r[9], 42)
        self.assertEqual(r[10:12], ['Light rain', 'moon'])
        self.assertEqual(r[12], [2, 3, 4])

    def test_every_widget_has_its_file_and_icon(self):
        r = run_js('widget-layout.js', 'console.log(JSON.stringify(CATALOG.map(c => [c.type, c.icon, c.bar])));')
        icons = set(p.name.rsplit('-', 1)[0] for p in (SHELL / 'icons').glob('*.svg'))
        self.assertEqual(len(r), 13)
        for t, icon, bar in r:
            f = 'Widget' + t[0].upper() + t[1:].replace('notes', 'Notes') + '.qml'
            if t == 'notes':
                f = 'WidgetNotes.qml'
            self.assertTrue((SHELL / f).exists(), f)
            self.assertIn(icon, icons, t)
            self.assertIn('WidgetBase {', (SHELL / f).read_text(), f)
        self.assertEqual(sorted(t for t, _, b in r if b), ['battery', 'clock', 'monitor', 'timer', 'weather'])


@unittest.skipUnless(NODE, 'node is needed for the activities rules')
class ActivityRules(unittest.TestCase):
    def test_main_always_there_and_a_missing_current_is_main(self):
        r = run_js('activities.js', '''console.log(JSON.stringify([parse({}), parse({current: "gone", "act:w": {name: "Work", order: 1, pins: ["a.desktop", 3]}, "app:org.x": "w"})]));''')
        self.assertEqual([a['id'] for a in r[0]['list']], ['default'])
        self.assertEqual(r[0]['current'], 'default')
        self.assertEqual([a['name'] for a in r[1]['list']], ['Main', 'Work'])
        self.assertEqual(r[1]['current'], 'default')
        self.assertEqual(r[1]['list'][1]['pins'], ['a.desktop'])
        self.assertEqual(r[1]['apps'], {'org.x': 'w'})

    def test_switching_hides_and_brings_back_only_what_it_hid(self):
        r = run_js('activities.js', '''var ws = [{key: "1", act: "default", minimized: false}, {key: "2", act: "default", minimized: true},
              {key: "3", act: "work", minimized: true}, {key: "4", act: "work", minimized: true}, {key: "5", act: "*", minimized: false}];
            var p = planSwitch(ws, "default", "work", ["3"]);
            var back = planSwitch([{key: "1", act: "default", minimized: true}, {key: "2", act: "default", minimized: true}, {key: "3", act: "work", minimized: false}], "work", "default", p.hidden);
            console.log(JSON.stringify([p, back]));''')
        p, back = r
        self.assertEqual(p['hide'], ['1'])           # the open one on Main; 2 you minimized yourself; 5 is on every one
        self.assertEqual(p['show'], ['3'])           # 3 Hearth hid; 4 you minimized yourself stays down
        self.assertEqual(sorted(p['hidden']), ['1'])
        self.assertEqual(back['show'], ['1'])        # 2 stays minimized
        self.assertEqual(back['hide'], ['3'])

    def test_pins_names_steps(self):
        r = run_js('activities.js', '''var l = parse({"act:w": {name: "Work", order: 1, pins: ["x"]}, "act:g": {name: "Games", order: 2}}).list;
            console.log(JSON.stringify([pinsFor(l, "w", ["d"]), pinsFor(l, "g", ["d"]), pinsFor(l, "default", ["d"]), togglePin(["x"], "y", true), togglePin(["x", "y"], "x", false),
              newName(l), step(l, "g", 1), step(l, "default", -1), winKey({kwinId: "{abc}"}), winKey({appId: "foot", title: "t"}), prune({a: 1, b: 2}, ["b"])]));''')
        self.assertEqual(r[0:3], [['x'], ['d'], ['d']])
        self.assertEqual(r[3:5], [['x', 'y'], ['y']])
        self.assertEqual(r[5], 'Study')
        self.assertEqual(r[6:8], ['default', 'g'])
        self.assertEqual(r[8:10], ['k:{abc}', 't:foot|t'])
        self.assertEqual(r[10], {'b': 2})


class LayoutStore(unittest.TestCase):
    """widgets.json and activities.json as SettingsWriter writes them (hearth_settings_io.py merge, one key a time)."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = dict(os.environ, HOME=self.tmp, XDG_STATE_HOME=os.path.join(self.tmp, 'state'))
        self.env.pop('DBUS_SESSION_BUS_ADDRESS', None)
        self.f = os.path.join(self.tmp, '.config', 'nimbus', 'widgets.json')
        os.makedirs(os.path.dirname(self.f))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def merge(self, changes, path=None):
        r = subprocess.run([sys.executable, str(IO), 'merge', path or self.f, json.dumps(changes)], env=self.env, capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)

    def read(self, path=None):
        with open(path or self.f) as f:
            return json.load(f)

    def test_two_widgets_moved_at_once_both_keep_their_place(self):
        self.merge({'w:a': {'type': 'clock', 'x': 1}, 'w:b': {'type': 'notes', 'x': 2, 'cfg': {'text': 'milk'}}, 'future': True})
        ts = [threading.Thread(target=self.merge, args=({'w:%s' % k: {'type': t, 'x': 100 + i}},)) for i, (k, t) in enumerate((('a', 'clock'), ('b', 'notes')) * 4)]
        for t in ts:
            t.start()
        for t in ts:
            t.join()
        d = self.read()
        self.assertIn('w:a', d)
        self.assertIn('w:b', d)
        self.assertTrue(d['future'])                 # a key the shell doesn't know stays
        self.assertTrue(os.path.exists(self.f + '.bak'))

    def test_removing_one_widget_takes_only_that_one(self):
        self.merge({'w:a': {'type': 'clock'}, 'w:b': {'type': 'notes', 'cfg': {'text': 'keep me'}}})
        self.merge({'w:a': None})
        self.assertEqual(self.read(), {'w:b': {'type': 'notes', 'cfg': {'text': 'keep me'}}})

    def test_a_damaged_file_comes_back_from_bak_and_isnt_replaced(self):
        self.merge({'w:a': {'type': 'clock'}})
        self.merge({'w:b': {'type': 'notes', 'cfg': {'text': 'important'}}})     # .bak now holds w:a
        with open(self.f, 'w') as f:
            f.write('{"w:a": {"type": "clo')                                      # cut short (a power cut)
        self.merge({'w:c': {'type': 'timer'}})
        d = self.read()
        self.assertIn('w:a', d)
        self.assertIn('w:c', d)
        self.assertTrue(any(n.startswith('widgets.json.broken-') for n in os.listdir(os.path.dirname(self.f))))

    def test_activities_file_merges_per_activity(self):
        a = os.path.join(self.tmp, '.config', 'nimbus', 'activities.json')
        self.merge({'act:w': {'name': 'Work', 'order': 1}}, a)
        self.merge({'current': 'w'}, a)
        self.merge({'act:g': {'name': 'Games', 'order': 2, 'wallpaper': 'anim:lava'}}, a)
        self.merge({'act:w': None, 'app:org.x': None}, a)
        self.assertEqual(self.read(a), {'current': 'w', 'act:g': {'name': 'Games', 'order': 2, 'wallpaper': 'anim:lava'}})


class Helper(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.hw = load(SHELL / 'hearth-widgets', 'hearth_widgets')

    def test_places_and_weather(self):
        p = self.hw.parse_places({'results': [{'name': 'Lisbon', 'admin1': 'Lisbon', 'country': 'Portugal', 'latitude': 38.71667, 'longitude': -9.13333, 'timezone': 'Europe/Lisbon'},
                                              {'name': 'Broken'}]})
        self.assertEqual(p, [{'name': 'Lisbon', 'label': 'Lisbon, Portugal', 'lat': 38.7167, 'lon': -9.1333, 'tz': 'Europe/Lisbon'}])
        w = self.hw.parse_weather({'current': {'temperature_2m': 18.4, 'weather_code': 2, 'is_day': 1, 'wind_speed_10m': 12},
                                   'current_units': {'temperature_2m': '°C', 'wind_speed_10m': 'km/h'},
                                   'daily': {'time': ['2026-11-16', '2026-11-17'], 'weather_code': [2, 61], 'temperature_2m_max': [20, 17], 'temperature_2m_min': [12]}})
        self.assertEqual(w['temp'], 18.4)
        self.assertEqual([d['day'] for d in w['days']], ['Mon', 'Tue'])
        self.assertIsNone(w['days'][1]['min'])
        self.assertIn('latitude=38.7167', self.hw.weather_url(38.71667, -9.1, False))
        self.assertIn('temperature_unit=fahrenheit', self.hw.weather_url(1, 2, True))

    def test_feeds(self):
        rss = b'<?xml version="1.0"?><rss><channel><title>News</title><item><title>One &amp; two</title><link>https://a/1</link><pubDate>Mon, 16 Nov 2026 10:00:00 GMT</pubDate><description>&lt;b&gt;Hi&lt;/b&gt; there</description></item></channel></rss>'
        atom = b'<feed xmlns="http://www.w3.org/2005/Atom"><title>Blog</title><entry><title>Post</title><link href="https://b/2"/><updated>2026-11-16T10:00:00Z</updated></entry></feed>'
        r = self.hw.parse_feed(rss)
        self.assertEqual(r['title'], 'News')
        self.assertEqual(r['items'][0]['title'], 'One & two')
        self.assertEqual(r['items'][0]['summary'], 'Hi there')
        a = self.hw.parse_feed(atom)
        self.assertEqual((a['title'], a['items'][0]['link']), ('Blog', 'https://b/2'))
        self.assertIn('error', self.hw.feed('file:///etc/passwd'))

    def test_folder_photos_zones(self):
        d = tempfile.mkdtemp()
        try:
            os.makedirs(os.path.join(d, 'Sub'))
            for n in ('b.jpg', 'a.txt', '.hidden'):
                open(os.path.join(d, n), 'w').close()
            r = self.hw.folder(d)
            self.assertEqual([(i['name'], i['kind']) for i in r['items']], [('Sub', 'folder'), ('a.txt', 'doc'), ('b.jpg', 'image')])
            self.assertIn('error', self.hw.folder(os.path.join(d, 'nope')))
            self.assertEqual(self.hw.walk_pictures(d), [os.path.join(d, 'b.jpg')])
            store = os.path.join(d, 'data', 'nimbus')
            os.makedirs(store)
            with open(os.path.join(store, 'photos.json'), 'w') as f:
                json.dump({'favorites': [os.path.join(d, 'b.jpg'), '/gone.jpg'], 'albums': [{'id': 'a1', 'name': 'Trip', 'items': [os.path.join(d, 'b.jpg')]}]}, f)
            old = os.environ.get('XDG_DATA_HOME')
            os.environ['XDG_DATA_HOME'] = os.path.join(d, 'data')
            try:
                self.assertEqual(self.hw.photos('favorites')['items'], [os.path.join(d, 'b.jpg')])
                self.assertEqual(self.hw.photos('album:a1')['label'], 'Trip')
                self.assertEqual(self.hw.photos('pictures')['albums'], [{'id': 'a1', 'name': 'Trip'}])
            finally:
                if old is None:
                    os.environ.pop('XDG_DATA_HOME', None)
                else:
                    os.environ['XDG_DATA_HOME'] = old
        finally:
            shutil.rmtree(d, ignore_errors=True)
        z = self.hw.zones(['UTC', 'Not/AZone'])
        self.assertEqual(z['zones'][0]['offset'], 0)
        self.assertIsNone(z['zones'][1]['offset'])

    def test_fixtures_answer_without_the_network(self):
        d = tempfile.mkdtemp()
        try:
            with open(os.path.join(d, 'weather.json'), 'w') as f:
                json.dump({'current': {'temperature_2m': 9, 'weather_code': 3}}, f)
            r = subprocess.run([sys.executable, str(SHELL / 'hearth-widgets'), 'weather', '1', '2'], env=dict(os.environ, HEARTH_WIDGETS_FIXTURES=d, HOME=d),
                               capture_output=True, text=True, timeout=30)
            self.assertEqual(json.loads(r.stdout)['temp'], 9)
        finally:
            shutil.rmtree(d, ignore_errors=True)


class Wiring(unittest.TestCase):
    def test_store_writes_only_merges_and_never_saves_over_a_damaged_file(self):
        s = code((SHELL / 'WidgetStore.qml').read_text())
        self.assertIn('SettingsWriter { id: writer; path: st.path }', s)
        self.assertNotIn('setText(', s)
        self.assertEqual(s.count('writer.merge('), 2)        # put() and remove(): only things you did
        self.assertIn('writer.repair()', s)
        bad = s[s.index('if (!o || typeof o'):s.index('const r = Object.assign')]
        self.assertNotIn('st.raw =', bad)                    # the widgets on screen stay; nothing is replaced
        a = code((SHELL / 'Activities.qml').read_text())
        self.assertIn('SettingsWriter { id: writer; path: act.path }', a)
        self.assertNotIn('file.setText(', a)
        self.assertIn('writer.repair()', a)

    def test_loaded_apart_and_hooked_in(self):
        shell = code((SHELL / 'shell.qml').read_text())
        self.assertIn('Loader { source: "Widgets.qml"', shell)
        self.assertNotIn('Widgets {', shell)
        theme = code((SHELL / 'Theme.qml').read_text())
        self.assertNotIn('Activities.', theme)               # Theme never depends on Widgets.qml loading
        for f in ('QuickAccess.qml', 'Desktop.qml', 'DeskState.qml', 'Pad.qml'):
            t = code((SHELL / f).read_text())
            self.assertNotRegex(t, r'\b(Activities|WidgetStore)\.', f)
        self.assertIn('SettingsNav.widgetsShown', code((SHELL / 'Pad.qml').read_text()))
        isl = code((SHELL / 'Islands.qml').read_text())
        self.assertIn('source: "ActivityChip.qml"', isl)
        self.assertIn('source: "BarWidgets.qml"', isl)

    def test_widgets_sit_under_windows(self):
        w = code((SHELL / 'WidgetWindow.qml').read_text())
        self.assertIn('WlrLayershell.layer: WlrLayer.Bottom', w)
        self.assertIn('exclusionMode: ExclusionMode.Ignore', w)
        self.assertIn('WidgetStore.place(', w)
        self.assertNotIn('WidgetStore.place(', w[:w.index('function finish()')].split('onReleased')[0])

    def test_helper_compiles(self):
        import py_compile
        py_compile.compile(str(SHELL / 'hearth-widgets'), doraise=True, cfile=os.path.join(tempfile.mkdtemp(), 'x.pyc'))


if __name__ == '__main__':
    unittest.main()
