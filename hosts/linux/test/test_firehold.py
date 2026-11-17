"""Firehold's Couch (a hidden preview of the 2.0 desktop): the rules its screens follow (firehold.js, with node), the
library helper (hearth-firehold), the input watcher (hearth-posture) and the QML's guards: locked, nothing of it shows
or runs; its settings only through SettingsWriter (opening a page writes nothing); loaded by file name so it can never
stop Desktop Mode; nothing about it in public docs, the site or the release notes."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import types
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
NODE = shutil.which('node')


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def run_js(body):
    src = (SHELL / 'firehold.js').read_text().replace('.pragma library', '')
    r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=30)
    if r.returncode != 0:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout.strip().splitlines()[-1])


def code(s):
    return re.sub(r'(?m)(^|\s)//.*$', '', s)


LIB = '''var games = [
  {id: "steam:1000", title: "Ember Trails", store: "Steam", kind: "steam", installed: true, lastPlayed: 5000, update: false},
  {id: "gog:1", title: "Night Harbor", store: "GOG", kind: "gog", installed: true, lastPlayed: 3000},
  {id: "epic:a", title: "Hollow Deep", store: "Epic", kind: "epic", installed: true, update: true, lastPlayed: 0},
  {id: "manual:q", title: "Quad Mech", store: "Your game", kind: "yours", installed: true, devBuild: true, lastPlayed: 4000},
  {id: "steam:2", title: "Cloud Lanes", store: "Steam", kind: "steam", installed: false, savesBackedUp: true},
  {id: "gog:2", title: "Star Courier", store: "GOG", kind: "gog", installed: false},
  {id: "winapp:o", title: "Old Office", store: "Windows app", kind: "winapp", installed: true}];
var wins = [
  {appId: "steam_app_1000", title: "Ember Trails", activated: true},
  {appId: "firefox", title: "3 tabs — Firefox"},
  {appId: "retroarch", title: "Pixel Rally", minimized: true},
  {appId: "org.quickshell", title: "Hearth Settings"},
  {appId: "discord", title: "Discord"}];
'''


@unittest.skipUnless(NODE, 'node is needed for the Couch rules')
class Rules(unittest.TestCase):
    def test_games_and_windows_are_one_list_games_first(self):
        r = run_js(LIB + 'console.log(JSON.stringify(openItems(wins, games, () => "").map(o => [o.kind, o.name, o.sub])));')
        self.assertEqual(r[0], ['game', 'Ember Trails', 'game · playing'])
        self.assertEqual(r[1], ['game', 'Pixel Rally', 'emulator · paused'])
        self.assertIn(['window', 'Firefox', '3 tabs'], r)
        self.assertIn(['window', 'Discord', ''], r)
        self.assertNotIn('Hearth Settings', [x[1] for x in r])     # the shell's own windows aren't listed

    def test_status_labels(self):
        r = run_js(LIB + 'console.log(JSON.stringify(games.map(g => status(g, ["steam:1000"]))));')
        self.assertEqual(r, ['Playing', 'Installed', 'Update ready', 'Dev build', 'Cloud saves', 'Not installed', 'Installed'])

    def test_continue_and_recent(self):
        r = run_js(LIB + 'console.log(JSON.stringify([continueGame(games, []).id, continueGame(games, ["gog:1"]).id, recent(games, 7, "steam:1000").map(g => g.id)]));')
        self.assertEqual(r[0], 'steam:1000')          # the one played last
        self.assertEqual(r[1], 'gog:1')               # the one running now
        self.assertEqual(r[2][:2], ['manual:q', 'gog:1'])  # newest first, then installed games to fill the row
        self.assertNotIn('steam:1000', r[2])
        self.assertNotIn('gog:2', r[2])               # not installed, never played: not "recent"

    def test_filters_and_search(self):
        r = run_js(LIB + 'console.log(JSON.stringify([filterGames(games, "installed", "").length, filterGames(games, "gog", "").length, filterGames(games, "all", "hollow").map(g => g.id), filterGames(games, "all", "", ["gog:1"]).map(g => g.id), chips({all: 7, steam: 2}).map(c => c.label + " " + c.n)]));')
        self.assertEqual(r[0], 5)
        self.assertEqual(r[1], 2)
        self.assertEqual(r[2], ['epic:a'])
        # playing, installed by last played (never played: A to Z), then the rest A to Z
        self.assertEqual(r[3], ['gog:1', 'steam:1000', 'manual:q', 'epic:a', 'winapp:o', 'steam:2', 'gog:2'])
        self.assertEqual(r[4], ['All 7', 'Installed 0', 'Steam 2', 'GOG 0', 'Epic 0', 'Windows apps 0', 'Emulators 0', 'Your games 0'])

    def test_posture(self):
        r = run_js('''console.log(JSON.stringify([nextPosture("auto", "desk", "pad"), nextPosture("auto", "couch", "mouse"), nextPosture("auto", "couch", "keys"),
            nextPosture("couch", "couch", "mouse"), nextPosture("desk", "desk", "pad"), screenPosture({"HDMI-A-1": "couch"}, "HDMI-A-1", "desk"),
            screenPosture({"HDMI-A-1": "couch"}, "DP-1", "desk"), startPosture("couch", "auto", ""), startPosture("last", "auto", "couch"),
            startPosture("last", "auto", ""), startPosture("couch", "desk", "")]));''')
        self.assertEqual(r, ['couch', 'desk', 'desk', 'couch', 'desk', 'couch', 'desk', 'couch', 'couch', 'desk', 'desk'])

    def test_grid_moves_and_times(self):
        r = run_js('''console.log(JSON.stringify([gridMove(0, 12, 6, "left"), gridMove(5, 12, 6, "right"), gridMove(2, 12, 6, "down"), gridMove(8, 12, 6, "up"),
            gridMove(2, 12, 6, "up"), gridMove(4, 8, 6, "down"), ago(0, 1), ago(1000, 1000 + 41 * 60000), ago(1, 1 + 3 * 3600000), duration(72 * 60000)]));''')
        self.assertEqual(r, [0, 5, 8, 2, -1, 7, '', '41 MIN AGO', '3 H AGO', '1 h 12 min'])


class LibraryHelper(unittest.TestCase):
    def setUp(self):
        self.h = load(SHELL / 'hearth-firehold', 'hearth_firehold')

    def test_entries_badges_and_counts(self):
        raw = [{'id': 'steam:1', 'title': 'B', 'source': 'steam', 'installed': True, 'state': 'installed', 'update': True, 'lastPlayed': 1700000000},
               {'id': 'gog:2', 'title': 'a', 'source': 'gog', 'installed': False, 'state': 'notInstalled'},
               {'id': 'manual:x', 'title': 'Mech', 'source': 'manual', 'exe': '/home/me/projects/mech/build/mech.x86_64'},
               {'id': 'emu:snes:z', 'title': 'Z', 'source': 'emu', 'system': 'Super Nintendo'},
               {'id': 'bad', 'title': 'no colon'}, 'junk']
        apps = [self.h.winapp_entry({'id': 'office', 'name': 'Old Office', 'icon': '/x/o.png'})]
        d = self.h.build(raw, apps, {'a'})
        ids = [g['id'] for g in d['games']]
        self.assertEqual(ids, ['gog:2', 'steam:1', 'manual:x', 'winapp:office', 'emu:snes:z'])
        by = {g['id']: g for g in d['games']}
        self.assertEqual(by['steam:1']['store'], 'Steam')
        self.assertTrue(by['steam:1']['update'])
        self.assertEqual(by['steam:1']['lastPlayed'], 1700000000000)      # seconds → milliseconds
        self.assertFalse(by['gog:2']['installed'])
        self.assertTrue(by['gog:2']['savesBackedUp'])
        self.assertTrue(by['manual:x']['devBuild'])
        self.assertEqual(by['manual:x']['store'], 'Your game')
        self.assertEqual(by['emu:snes:z']['store'], 'Emulator')
        self.assertTrue(by['manual:x']['installed'])                       # the older list has no "installed": installed
        self.assertEqual(by['winapp:office']['icon'], 'file:///x/o.png')
        self.assertEqual(d['counts'], {'all': 5, 'installed': 4, 'steam': 1, 'gog': 1, 'epic': 0, 'winapp': 1, 'emu': 1, 'yours': 1})

    def test_library_from_hearth_then_the_cache(self):
        with tempfile.TemporaryDirectory() as t:
            fake = os.path.join(t, 'nimbus')
            with open(fake, 'w') as f:
                f.write('#!/bin/sh\n[ "$2" = "--all" ] && echo \'[{"id":"steam:9","title":"Nine","source":"steam","installed":false,"state":"notInstalled"}]\' && exit 0\nexit 3\n')
            os.chmod(fake, 0o755)
            env = dict(os.environ, NIMBUS_APP=fake, XDG_CACHE_HOME=os.path.join(t, 'cache'), HOME=t, PATH='/usr/bin:/bin')
            r = subprocess.run(['python3', str(SHELL / 'hearth-firehold'), 'library'], env=env, capture_output=True, text=True, timeout=60)
            d = json.loads(r.stdout.strip().splitlines()[-1])
            self.assertEqual(d['from'], 'hearth')
            self.assertEqual([g['id'] for g in d['games']], ['steam:9'])
            self.assertFalse(d['games'][0]['installed'])
            os.chmod(fake, 0o644)                                            # Hearth gone: the copy from last time
            r = subprocess.run(['python3', str(SHELL / 'hearth-firehold'), 'library'], env=env, capture_output=True, text=True, timeout=60)
            d = json.loads(r.stdout.strip().splitlines()[-1])
            self.assertEqual((d['from'], [g['id'] for g in d['games']]), ('cache', ['steam:9']))

    def test_play_refuses_odd_ids(self):
        r = subprocess.run(['python3', str(SHELL / 'hearth-firehold'), 'play', 'nocolon'], capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 1)
        self.assertTrue(r.stdout.startswith('error: '))


class PostureWatcher(unittest.TestCase):
    def setUp(self):
        self.p = load(SHELL / 'hearth-posture', 'hearth_posture')
        self.e = types.SimpleNamespace(EV_KEY=1, EV_REL=2, EV_ABS=3, BTN_LEFT=0x110, REL_X=0, ABS_X=0, ABS_MT_POSITION_X=0x35,
                                       BTN_SOUTH=0x130, BTN_GAMEPAD=0x130, BTN_JOYSTICK=0x120,
                                       **{'KEY_' + c: i + 16 for i, c in enumerate('QWERTYASDFZX')})

    def test_mice_and_keyboards_but_never_controllers_or_hearths_own(self):
        e = self.e
        keyboard = {e.EV_KEY: list(range(16, 28))}
        mouse = {e.EV_KEY: [e.BTN_LEFT], e.EV_REL: [e.REL_X]}
        pad = {e.EV_KEY: [e.BTN_SOUTH, e.BTN_LEFT], e.EV_ABS: [(e.ABS_X, None)]}
        self.assertEqual(self.p.classify('AT Translated Set 2 keyboard', keyboard, e), 'keys')
        self.assertEqual(self.p.classify('Logitech USB Optical Mouse', mouse, e), 'mouse')
        self.assertIsNone(self.p.classify('Xbox Wireless Controller', pad, e))
        self.assertIsNone(self.p.classify('Hearth controller pointer', mouse, e))
        self.assertIsNone(self.p.classify('Hearth controller keyboard', keyboard, e))
        self.assertIsNone(self.p.classify('Power Button', {e.EV_KEY: [116]}, e))
        self.assertTrue(self.p.counts('keys', e.EV_KEY, 16, 1, e))
        self.assertFalse(self.p.counts('keys', e.EV_KEY, 16, 0, e))       # a key going up isn't a new use
        self.assertTrue(self.p.counts('mouse', e.EV_REL, 0, 3, e))


class Guards(unittest.TestCase):
    def test_loaded_by_file_name_and_locked_shows_nothing(self):
        shell = (SHELL / 'shell.qml').read_text()
        self.assertIn('source: "Firehold.qml"', shell)
        self.assertNotRegex(code(shell), r'(?m)^\s*Firehold\s*\{')       # never as a type: a problem in it can't stop the shell
        f = code((SHELL / 'Firehold.qml').read_text())
        self.assertIn('readonly property bool unlocked: cfg["firehold.unlocked"] === true', f)
        for what in ('model: fhs.unlocked ? Quickshell.screens : []', 'active: fhs.unlocked && fhs.guideShown', 'active: fhs.unlocked && fhs.noticeShown',
                     'active: fhs.unlocked && fhs.settingsShown', 'running: fhs.unlocked && fhs.mode === "auto"'):
            self.assertIn(what, f)
        for fn in ('function begin()', 'function switchTo(', 'function input(', 'function guide(', 'function padButton(', 'function loadLibrary('):
            body = f[f.index(fn):f.index(fn) + 220]
            self.assertIn('unlocked', body, fn)
        couch = code((SHELL / 'FireholdCouch.qml').read_text())
        self.assertIn('readonly property bool shown: fh.unlocked &&', couch)
        row = code((SHELL / 'FireholdAboutRow.qml').read_text())
        self.assertIn('visible: unlocked', row)

    def test_the_desk_only_hides_mantels_bar_and_dock_while_it_is_up(self):
        for name in ('Islands.qml', 'Dock.qml'):
            self.assertIn('visible: settingsReady && !remapping && !SettingsNav.fireholdDesk', (SHELL / name).read_text(), name)
        d = code((SHELL / 'FireholdDesk.qml').read_text())
        self.assertIn('Component.onCompleted: SettingsNav.fireholdDesk = true', d)
        self.assertIn('Component.onDestruction: SettingsNav.fireholdDesk = false', d)
        f = code((SHELL / 'Firehold.qml').read_text())
        self.assertIn('active: fhs.unlocked && fhs.started && fhs.posture === "desk"', f)
        self.assertIn('readonly property bool picturesWanted: unlocked && WinList.kwin && couchUp && page === "switcher"', f)

    def test_window_pictures_only_on_kwin_and_only_real_ids(self):
        h = load(SHELL / 'hearth-firehold', 'hearth_firehold2')
        os.environ.pop('HEARTH_WM', None)
        self.assertEqual(h.pictures('/nonexistent', ['{0b0f3c2e-1111-2222-3333-444455556666}']), {})
        self.assertTrue(h.WIN_ID.match('{0b0f3c2e-1111-2222-3333-444455556666}'))
        self.assertFalse(h.WIN_ID.match('../../etc/passwd'))

    def test_settings_only_through_settings_writer_and_never_on_open(self):
        for name in ('Firehold.qml', 'FireholdCouch.qml', 'FireholdGuide.qml', 'FireholdSettings.qml', 'FireholdAboutRow.qml', 'FireholdNotice.qml', 'FireholdPerf.qml', 'FireholdDesk.qml', 'FireholdGameFrame.qml'):
            src = code((SHELL / name).read_text())
            self.assertNotIn('setText(', src, name)
            self.assertNotRegex(src, r'firehold\.json.*(>|tee|cp )', name)
        f = code((SHELL / 'Firehold.qml').read_text())
        self.assertIn('SettingsWriter { id: writer; path: fhs.cfgPath', f)
        # set() writes only a key that changed, and only from a choice (never in onLoaded or Component.onCompleted).
        self.assertIn('if (JSON.stringify(cfg[key]) === JSON.stringify(value)) return;', f)
        loaded = f[f.index('onLoaded: {'):f.index('onLoadFailed')]
        self.assertNotIn('writer.merge', loaded)
        self.assertNotIn('fhs.set(', loaded)

    def test_hearth_pad_routes_only_while_the_couch_says_so(self):
        p = (SHELL / 'hearth-pad').read_text()
        self.assertIn("return self.keys_ctx and self.keys_why == 'Firehold'", p)
        self.assertIn("if name == 'guide' and self.firehold", p)
        self.assertIn("emit({'t': 'used'})", p)                           # Pad.qml reads m.t
        pad = (SHELL / 'Pad.qml').read_text()
        self.assertIn('if (SettingsNav.fireholdShown) return "Firehold";', pad)
        self.assertIn('send({ cmd: "firehold", on: SettingsNav.fireholdCouch });', pad)

    def test_not_in_public_docs_site_or_release_notes(self):
        public = [ROOT / 'site', ROOT / 'docs' / 'ROADMAP.md', ROOT / 'README.md']
        public += [p for p in (ROOT / 'docs').glob('*.md')]
        hits = []
        for base in public:
            files = [base] if base.is_file() else [p for p in base.rglob('*') if p.is_file() and p.suffix in ('.html', '.md', '.js', '.json', '.txt')]
            for p in files:
                try:
                    t = p.read_text(errors='ignore')
                except OSError:
                    continue
                if re.search(r'\bFirehold\b', t):
                    hits.append(str(p.relative_to(ROOT)))
        self.assertEqual(hits, [], 'Firehold is a hidden preview: keep it out of public docs and the site')
        for name in ('firehold.js', 'Firehold.qml', 'FireholdCouch.qml', 'FireholdGuide.qml', 'FireholdSettings.qml', 'hearth-firehold'):
            self.assertNotRegex((SHELL / name).read_text(), r'(?i)albireo', name)


if __name__ == '__main__':
    unittest.main()
