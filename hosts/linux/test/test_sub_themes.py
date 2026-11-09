"""Sub themes (1.10; Settings › Personalisation › Sub theme): each of Hearth, Steam-style and Gaming gets four new
colourways plus today's colours, and the black set of seven. A sub theme changes only colours and the default
wallpaper. Checked here:
- palettes.json has them all, in order, with readable text (WCAG AA, 4.5:1) in dark and light;
- nobody's desktop changes after updating: no subTheme = today's colours, wallpaper and accent;
- a missing or broken sub theme falls back to the look's own colours (Python and the shell's subthemes.js);
- picking a sub theme never replaces a wallpaper you chose; changing the look resets the sub theme, only then;
- every sub theme's wallpaper and list picture is there.
Failures also post ::error annotations with the details."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import struct
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
DESK = ROOT / 'hosts' / 'linux' / 'desktop'
SHELL = DESK / 'shell'
THEME = DESK / 'theme'
PAL = json.loads((THEME / 'palettes.json').read_text())
NODE = shutil.which('node')
TOKENS = ('bg', 'bg2', 'panel', 'panel2', 'panel3', 'line', 'text', 'muted', 'ember', 'flame', 'ink')
SUBS = {
    'hearth': ['firelight', 'rose-quartz', 'evergreen', 'midnight', 'copper-sea'],
    'steam': ['steam-blue', 'crimson', 'emerald', 'royal', 'graphite'],
    'gaming': ['cyber', 'toxic', 'inferno', 'ultraviolet', 'frostbite'],
}
LABELS = {'firelight': 'Firelight', 'rose-quartz': 'Rose Quartz', 'evergreen': 'Evergreen', 'midnight': 'Midnight',
          'copper-sea': 'Copper Sea', 'steam-blue': 'Steam blue', 'crimson': 'Crimson', 'emerald': 'Emerald', 'royal': 'Royal',
          'graphite': 'Graphite', 'cyber': 'Cyber', 'toxic': 'Toxic', 'inferno': 'Inferno', 'ultraviolet': 'Ultraviolet',
          'frostbite': 'Frostbite'}
BLACK = [('black-red', 'Black & Red'), ('black-green', 'Black & Green'), ('black-yellow', 'Black & Yellow'), ('hazard', 'Hazard'),
         ('volt', 'Volt'), ('holly', 'Holly'), ('traffic-light', 'Traffic Light')]
OTHERS = ['zen', 'glass', 'synthwave', 'cabin', 'console']


def annotate(kind, title, msg):
    if os.environ.get('GITHUB_ACTIONS'):
        print('::%s title=%s::%s' % (kind, title, msg.replace('\n', ' | ')[:3000]))


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


HT = load(THEME / 'hearth-theme', 'hearth_theme_sub_test')


def lin(v):
    v /= 255
    return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4


def lum(h):
    h = h.lstrip('#')
    r, g, b = (lin(int(h[i:i + 2], 16)) for i in (0, 2, 4))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b):
    x, y = sorted((lum(a), lum(b)))
    return (y + 0.05) / (x + 0.05)


def code(text):
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in text.splitlines())


def run_theme(home, desktop, theme_dir=THEME):
    os.makedirs(os.path.join(home, '.local', 'share', 'nimbus'), exist_ok=True)
    p = os.path.join(home, '.local', 'share', 'nimbus', 'desktop.json')
    with open(p, 'w') as f:
        json.dump(desktop, f)
    before = pathlib.Path(p).read_text()
    env = dict(os.environ, HOME=home, XDG_RUNTIME_DIR=os.path.join(home, 'rt'))
    for k in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'LABWC_PID'):
        env.pop(k, None)
    r = subprocess.run([sys.executable, str(pathlib.Path(theme_dir) / 'hearth-theme'), '--show'], env=env, capture_output=True, text=True)
    if r.returncode != 0:
        annotate('error', 'Sub themes', 'hearth-theme --show failed for %s: %s' % (desktop, r.stderr[-800:]))
    assert r.returncode == 0, r.stderr
    assert pathlib.Path(p).read_text() == before, 'hearth-theme changed desktop.json'
    return json.loads(r.stdout)


def jpeg_size(path):
    """(width, height) of a JPEG, from its SOF marker (no Pillow needed)."""
    with open(path, 'rb') as f:
        data = f.read()
    assert data[:2] == b'\xff\xd8', path
    i = 2
    while i < len(data):
        if data[i] != 0xff:
            i += 1
            continue
        marker = data[i + 1]
        if marker in (0xc0, 0xc1, 0xc2):
            h, w = struct.unpack('>HH', data[i + 5:i + 9])
            return w, h
        seg = struct.unpack('>H', data[i + 2:i + 4])[0]
        i += 2 + seg
    raise AssertionError('no size in ' + str(path))


class Palettes(unittest.TestCase):
    def test_every_sub_theme_is_there_in_order(self):
        self.assertEqual([b['id'] for b in PAL['blackSet']], [b[0] for b in BLACK])
        self.assertEqual([b['label'] for b in PAL['blackSet']], [b[1] for b in BLACK])
        for look, ids in SUBS.items():
            subs = PAL[look]['subThemes']
            self.assertEqual([s['id'] for s in subs], ids, look)
            self.assertEqual([s['label'] for s in subs], [LABELS[i] for i in ids], look)
            self.assertTrue(subs[0].get('own'), look + ': the first is today\'s colours')
            self.assertNotIn('dark', subs[0], look)                 # today's colours come from the look itself
            self.assertEqual(sorted(PAL[look]['blackWallpapers']), sorted(b[0] for b in BLACK), look)
            listed = [s['id'] for s in HT.sub_list(PAL, look)]
            self.assertEqual(listed, ids + [b[0] for b in BLACK], look)
        for look in OTHERS:
            self.assertNotIn('subThemes', PAL[look], look)
            self.assertEqual(HT.sub_list(PAL, look), [], look)
        self.assertIn('third', PAL['blackSet'][-1]['dark'])          # Traffic Light: red, yellow and green

    def test_colours_are_complete_and_readable_in_both_modes(self):
        entries = [(look, s) for look in SUBS for s in PAL[look]['subThemes'][1:]] + [('black', b) for b in PAL['blackSet']]
        problems = []
        for look, s in entries:
            for mode in ('dark', 'light'):
                p = s[mode]
                for k in TOKENS:
                    self.assertRegex(p.get(k, ''), r'^#[0-9a-f]{6}$', (look, s['id'], mode, k))
                for fg, bgs, need in (('text', ('bg', 'bg2', 'panel', 'panel2', 'panel3'), 4.5),
                                      ('muted', ('bg', 'bg2', 'panel', 'panel2'), 4.5)):
                    for b in bgs:
                        c = contrast(p[fg], p[b])
                        if c < need:
                            problems.append('%s %s %s: %s on %s is %.2f:1' % (look, s['id'], mode, fg, b, c))
                c = contrast(p['ink'], p['ember'])
                if c < 4.5:
                    problems.append('%s %s %s: text on the accent (ink on ember) is %.2f:1' % (look, s['id'], mode, c))
        if problems:
            annotate('error', 'Sub themes contrast', '; '.join(problems))
        else:
            annotate('notice', 'Sub themes contrast', '%d sub themes, dark and light: text, muted and accent text all 4.5:1 or more' % len(entries))
        self.assertEqual(problems, [])

    def test_the_looks_shapes_are_untouched(self):
        # A sub theme carries only colours (and a wallpaper): never corners, bars, frames or fonts.
        allowed = {'id', 'label', 'desc', 'own', 'wallpaper', 'dark', 'light'}
        for look in SUBS:
            for s in PAL[look]['subThemes']:
                self.assertLessEqual(set(s), allowed, (look, s['id']))
        for b in PAL['blackSet']:
            self.assertLessEqual(set(b), allowed - {'own', 'wallpaper'}, b['id'])
        allowed_colours = set(TOKENS) | {'third'}
        for look in SUBS:
            for s in PAL[look]['subThemes'][1:] + PAL['blackSet']:
                for mode in ('dark', 'light'):
                    self.assertLessEqual(set(s[mode]), allowed_colours, (look, s['id'], mode))


class NothingChangesAfterUpdating(unittest.TestCase):
    def test_no_sub_theme_is_todays_colours_accent_and_wallpaper(self):
        for look in list(SUBS) + OTHERS:
            for mode in ('dark', 'light'):
                for desktop in ({'look': look, 'mode': mode}, {'look': look, 'mode': mode, 'subTheme': ''}):
                    with tempfile.TemporaryDirectory() as d:
                        got = run_theme(d, desktop)
                    want = PAL[look][mode]
                    for k in ('bg', 'bg2', 'panel', 'panel2', 'panel3', 'line', 'text', 'muted'):
                        self.assertEqual(got['colors'][k], want[k], (look, mode, k))
                    self.assertEqual(got['accent'], want['ember'], (look, mode))
                    self.assertEqual(got['sub'], '')
        # A new install's Sky accent and the Hearth profile accent stay as they were without a sub theme.
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(run_theme(d, {'defaults': 2})['accent'], '#66c0f4')
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(run_theme(d, {'defaults': 2, 'look': 'steam', 'subTheme': 'crimson'})['accent'], '#e5484d')

    def test_the_new_keys_default_to_today_and_are_never_written_by_themselves(self):
        s = (SHELL / 'Settings.qml').read_text()
        self.assertRegex(s, r'property string subTheme: ""(?=\s)')
        self.assertRegex(s, r'property bool wallMatch: true(?=\s)')
        keys = re.search(r'readonly property var keys: (.*?)\n    on\w+Changed', s, re.S).group(1)
        for k in ('subTheme', 'wallMatch'):
            self.assertIn('"%s"' % k, keys)
            self.assertIn('on%sChanged: save("%s")' % (k[0].upper() + k[1:], k), s)
        self.assertIn('if (!loaded || applying) return;', s)
        # Only Personalisation writes them; Theme.qml and the drawing code only read.
        for f in SHELL.glob('*.qml'):
            t = code(f.read_text())
            hits = re.findall(r'\bSettings\.(subTheme|wallMatch)\s*=(?!=)', t)
            self.assertEqual(hits, [], f.name)
        skel = json.loads((ROOT / 'os' / 'system_files' / 'etc' / 'skel' / '.local' / 'share' / 'nimbus' / 'desktop.json').read_text())
        self.assertNotIn('subTheme', skel)

    def test_the_shell_reapplies_apps_when_the_sub_theme_changes(self):
        t = (SHELL / 'Theme.qml').read_text()
        self.assertIn('readonly property string key: look + "/" + subTheme + "/" + mode + "/" + accent', t)
        self.assertIn('import "subthemes.js" as SubThemes', t)
        self.assertIn('readonly property var p: SubThemes.palette(lookDef[mode] || ({}), palettes, look, Settings.subTheme, mode)', t)


class Fallbacks(unittest.TestCase):
    BROKEN = [
        {'hearth': {'subThemes': 'nope'}},
        {'hearth': {'subThemes': [None, 7, {'id': ''}]}},
        {'hearth': {'subThemes': [{'id': 'firelight', 'own': True}, {'id': 'rose-quartz', 'dark': 'x', 'light': None}]}},
        {'hearth': {'subThemes': [{'id': 'firelight', 'own': True}, {'id': 'rose-quartz', 'dark': {'bg': 'red', 'text': '#12'}}]}},
        {'blackSet': 'nope', 'hearth': {'blackWallpapers': 5}},
        {'blackSet': [None, {'id': 'black-red', 'dark': {'ember': 'rgb(1,2,3)'}}]},
    ]

    @staticmethod
    def broken(change):
        pal = json.loads(json.dumps(PAL))
        for k, v in change.items():
            if isinstance(v, dict) and isinstance(pal.get(k), dict):
                pal[k].update(v)
            else:
                pal[k] = v
        return pal

    def test_python_falls_back_to_the_looks_own(self):
        for change in self.BROKEN:
            pal = self.broken(change)
            for sid in ('', 'rose-quartz', 'black-red', 'nope', None, 7):
                for mode in ('dark', 'light'):
                    got = HT.sub_palette(pal['hearth'][mode], pal, 'hearth', sid, mode)
                    for k in ('bg', 'text', 'ember', 'muted'):
                        self.assertRegex(got[k], r'^#[0-9a-f]{6}$', (change, sid, mode, k))
                    s = HT.sub_find(pal, 'hearth', sid)
                    if not s or s['own']:
                        self.assertEqual(got, pal['hearth'][mode], (change, sid))
        # Unknown ids, another look's id, a sub theme on a look without any: the look's own.
        for look, sid in (('hearth', 'crimson'), ('steam', 'nope'), ('zen', 'crimson'), ('gaming', '')):
            self.assertEqual(HT.sub_palette(PAL[look]['dark'], PAL, look, sid, 'dark'), PAL[look]['dark'])
        self.assertEqual(HT.sub_list(None, 'hearth'), [])
        self.assertEqual(HT.sub_list({}, 'hearth'), [])

    def test_hearth_theme_never_fails_on_a_broken_entry_or_setting(self):
        for desktop in ({'look': 'hearth', 'subTheme': 7}, {'look': 'hearth', 'subTheme': ['x']}, {'look': 'steam', 'subTheme': 'rose-quartz'},
                        {'look': 'zen', 'subTheme': 'crimson'}, {'look': 'gaming', 'subTheme': 'nope', 'mode': 'light'}):
            with tempfile.TemporaryDirectory() as d:
                got = run_theme(d, desktop)
            look = desktop['look']
            self.assertEqual(got['colors']['bg'], PAL[look][got['mode']]['bg'], desktop)
        # A palettes.json whose sub themes are damaged: the look's own colours, not a crash.
        with tempfile.TemporaryDirectory() as d:
            td = os.path.join(d, 'theme')
            shutil.copytree(THEME, td)
            pal = self.broken({'steam': {'subThemes': [{'id': 'steam-blue'}, {'id': 'crimson', 'dark': {'bg': 'not a colour'}}]}, 'blackSet': 'x'})
            pathlib.Path(td, 'palettes.json').write_text(json.dumps(pal))
            got = run_theme(os.path.join(d, 'home'), {'look': 'steam', 'subTheme': 'crimson'}, td)
            self.assertEqual(got['colors']['bg'], PAL['steam']['dark']['bg'])
            got = run_theme(os.path.join(d, 'home2'), {'look': 'steam', 'subTheme': 'black-red'}, td)
            self.assertEqual(got['colors']['bg'], PAL['steam']['dark']['bg'])

    def test_a_sub_theme_changes_the_colours_everywhere(self):
        with tempfile.TemporaryDirectory() as d:
            got = run_theme(d, {'look': 'steam', 'subTheme': 'crimson'})
        crimson = PAL['steam']['subThemes'][2 - 1]
        self.assertEqual(crimson['id'], 'crimson')
        self.assertEqual(got['sub'], 'crimson')
        self.assertEqual(got['colors']['bg'], crimson['dark']['bg'])
        self.assertEqual(got['accent'], crimson['dark']['ember'])
        self.assertEqual(got['colors']['ink'], crimson['dark']['ink'])
        with tempfile.TemporaryDirectory() as d:
            got = run_theme(d, {'look': 'gaming', 'subTheme': 'traffic-light', 'mode': 'light', 'accent': '#1a9fff'})
        self.assertEqual(got['colors']['bg'], PAL['blackSet'][-1]['light']['bg'])
        self.assertEqual(got['accent'], '#1a9fff')                    # your own accent still wins


JS_HARNESS = r'''
const fs = require('fs');
const src = fs.readFileSync(process.argv[2], 'utf8').replace(/^\.pragma library\s*$/m, '');
const S = new Function(src + '\nreturn { list, find, isOwn, palette, ownWallpaper, shownWallpaper, changesForSub, changesForLook, changesForMatch, matchShown, thumb };')();
const cases = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
const out = cases.map(c => {
  try {
    const pal = c.pal;
    const base = pal && pal[c.look] && pal[c.look][c.mode] || {};
    return { list: S.list(pal, c.look).map(s => s.id), palette: S.palette(base, pal, c.look, c.id, c.mode),
             own: S.ownWallpaper(pal, c.look, c.id, c.match), shown: S.shownWallpaper(pal, c.look, c.id, c.chosen, c.match) };
  } catch (e) { return { error: String(e) }; }
});
const actions = {
  sub: ['', 'crimson', 'rose-quartz', 'traffic-light', 7].map(id => S.changesForSub(id)),
  look: ['hearth', 'steam', 'zen'].map(l => S.changesForLook(l)),
  matchOnMine: S.changesForMatch(true, '/home/me/cat.jpg'), matchOn: S.changesForMatch(true, ''),
  matchOff: S.changesForMatch(false, '/home/me/cat.jpg'), matchOffNone: S.changesForMatch(false, ''),
  shownMine: S.matchShown(true, '/home/me/cat.jpg'), shownNone: S.matchShown(true, ''), shownOff: S.matchShown(false, ''),
};
console.log(JSON.stringify({ out, actions }));
'''


@unittest.skipUnless(NODE, 'node is needed to run the shell\'s subthemes.js')
class ShellResolver(unittest.TestCase):
    """subthemes.js (Theme.qml and Personalisation use it) gives the same answers as hearth-theme, and keeps the rules."""

    @classmethod
    def setUpClass(cls):
        cases = []
        broken = Fallbacks.BROKEN
        pals = [PAL] + [Fallbacks.broken(c) for c in broken] + [None, {}]
        for pi, pal in enumerate(pals):
            for look in ('hearth', 'steam', 'gaming', 'zen'):
                for sid in ('', 'rose-quartz', 'crimson', 'cyber', 'black-red', 'traffic-light', 'nope', 7):
                    for mode in ('dark', 'light'):
                        for chosen, match in (('', True), ('', False), ('/home/me/cat.jpg', True), ('builtin:slate-dusk', False)):
                            if pi > 0 and (chosen or not match or mode == 'light'):
                                continue
                            cases.append({'pal': pal, 'look': look, 'id': sid, 'mode': mode, 'chosen': chosen, 'match': match})
        cls.cases = cases
        with tempfile.TemporaryDirectory() as d:
            h, c = os.path.join(d, 'h.js'), os.path.join(d, 'c.json')
            pathlib.Path(h).write_text(JS_HARNESS)
            pathlib.Path(c).write_text(json.dumps(cases))
            r = subprocess.run([NODE, h, str(SHELL / 'subthemes.js'), c], capture_output=True, text=True)
        if r.returncode != 0:
            annotate('error', 'Sub themes (subthemes.js)', r.stderr[-1500:])
        assert r.returncode == 0, r.stderr
        cls.res = json.loads(r.stdout)

    def test_never_throws_and_matches_hearth_theme(self):
        bad = []
        for c, o in zip(self.cases, self.res['out']):
            self.assertNotIn('error', o, c)
            pal = c['pal']
            base = (pal or {}).get(c['look'], {}).get(c['mode'], {}) if isinstance((pal or {}).get(c['look']), dict) else {}
            py = HT.sub_palette(base, pal, c['look'], c['id'], c['mode'])
            if o['palette'] != py:
                bad.append('%s/%s/%s: js %s python %s' % (c['look'], c['id'], c['mode'], o['palette'], py))
            if o['list'] != [s['id'] for s in HT.sub_list(pal, c['look'])]:
                bad.append('%s list: js %s' % (c['look'], o['list']))
        if bad:
            annotate('error', 'Sub themes (subthemes.js)', '; '.join(bad[:10]))
        self.assertEqual(bad, [])

    def test_your_own_wallpaper_is_never_replaced(self):
        for c, o in zip(self.cases, self.res['out']):
            if c['chosen']:
                self.assertEqual(o['shown'], c['chosen'], c['id'])
        a = self.res['actions']
        for ch in a['sub']:
            self.assertEqual(list(ch), ['subTheme'])                 # picking a sub theme writes nothing else
        self.assertEqual(a['sub'][4], {'subTheme': ''})
        self.assertEqual(a['matchOff'], {'wallMatch': False})        # turning Match off keeps your wallpaper
        self.assertEqual(a['matchOn'], {'wallMatch': True})
        self.assertEqual(a['matchOnMine'], {'wallMatch': True, 'wallpaper': ''})   # only when you turn it on yourself
        self.assertFalse(a['shownMine'])                             # with your own wallpaper, Match shows off
        self.assertTrue(a['shownNone'])
        self.assertFalse(a['shownOff'])

    def test_changing_the_look_resets_the_sub_theme(self):
        self.assertEqual(self.res['actions']['look'], [{'look': 'hearth', 'subTheme': ''}, {'look': 'steam', 'subTheme': ''}, {'look': 'zen', 'subTheme': ''}])

    def test_match_shows_the_sub_themes_wallpaper_and_today_keeps_the_looks(self):
        for c, o in zip(self.cases, self.res['out']):
            if c['pal'] is not PAL and c['pal'] != PAL:
                continue
            look, sid = c['look'], c['id']
            s = HT.sub_find(PAL, look, sid)
            look_wall = PAL[look].get('wallpaper', '')
            if not c['match'] or not s or s['own']:
                self.assertEqual(o['own'], look_wall, c)
            else:
                self.assertEqual(o['own'], s['wallpaper'], c)


class Personalisation(unittest.TestCase):
    def test_the_page_is_compact_dropdowns_driven_by_the_rules(self):
        a = code((SHELL / 'SetAppearance.qml').read_text())
        self.assertNotIn('component LookCard', a)
        self.assertIn('onChosen: v => page.apply(SubThemes.changesForLook(v))', a)
        self.assertIn('onChosen: v => page.apply(SubThemes.changesForSub(v))', a)
        self.assertIn('onToggled: on => page.apply(SubThemes.changesForMatch(on, Settings.wallpaper))', a)
        self.assertIn('function apply(changes) { for (const k in changes) Settings[k] = changes[k]; }', a)
        self.assertIn('checked: SubThemes.matchShown(Settings.wallMatch, Settings.wallpaper)', a)
        self.assertEqual(a.count('SetDropdown {'), 2)
        order = [a.index(x) for x in ('title: "Look"', 'title: "Sub theme"', 'title: "Mode"', 'title: "Match sub theme"', 'id: accentRow')]
        self.assertEqual(order, sorted(order))
        self.assertIn('label: "Auto"', a)
        self.assertIn('head: i === 0 ? "Colours" : (s.group === "black"', a)
        # The only other wallpaper write is picking one yourself.
        self.assertEqual(re.findall(r'Settings\.wallpaper\s*=(?!=)', a), ['Settings.wallpaper ='])
        self.assertIn('function chooseWallpaper(f) {\n        Settings.wallpaper = f;', a)

    def test_the_dropdown_works_with_a_controller(self):
        d = (SHELL / 'SetDropdown.qml').read_text()
        self.assertIn('onActivated: dd.open ? dd.close() : dd.openList()', d)           # A opens and closes
        self.assertIn('onEscaped: dd.close()', d)                                       # B closes
        self.assertIn('ownsEscape: true', d)
        self.assertIn('onActivated: dd.pick(entry.modelData.value)', d)                 # A picks
        self.assertIn('usesLeftRight: true', d)
        b = (SHELL / 'SetBase.qml').read_text()
        self.assertIn('case Qt.Key_Escape: if (base.ownsEscape) base.escaped(); else SettingsNav.back(); break;', b)

    def test_wallpapers_and_pictures_are_there(self):
        sizes, total, missing = {}, 0, []
        for look in SUBS:
            for s in HT.sub_list(PAL, look):
                t = SHELL / ('images/wallpapers/sub/thumbs/%s-%s.jpg' % (look, s['id']))
                if not t.exists():
                    missing.append(str(t.relative_to(ROOT)))
                else:
                    self.assertEqual(jpeg_size(t), (320, 180), t.name)
                if s['own']:
                    self.assertEqual(s['wallpaper'], '', look)
                    continue
                w = s['wallpaper']
                self.assertTrue(w.startswith('builtin:sub/'), w)
                f = SHELL / 'images' / 'wallpapers' / (w[8:] + '.jpg')
                if not f.exists():
                    missing.append(str(f.relative_to(ROOT)))
                    continue
                sizes[f.name] = jpeg_size(f)
        for f in sorted((SHELL / 'images' / 'wallpapers' / 'sub').rglob('*.jpg')):
            total += f.stat().st_size
        if missing:
            annotate('error', 'Sub theme wallpapers', 'missing: ' + ', '.join(missing))
        self.assertEqual(missing, [])
        for n, wh in sizes.items():
            self.assertEqual(wh, (2560, 1440), n)          # as the other built-in wallpapers
        annotate('notice', 'Sub theme wallpapers', '%d wallpapers and their pictures, %.1f MB' % (len(sizes), total / 1e6))
        self.assertLess(total, 8e6)


if __name__ == '__main__':
    unittest.main()
