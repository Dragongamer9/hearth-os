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
import sys
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
  {appId: "org.quickshell", title: "Trips – Files"},
  {appId: "org.quickshell", title: ""},
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
        self.assertIn(['window', 'Hearth Settings', ''], r)          # Hearth's own windows are windows too
        self.assertIn(['window', 'Hearth Files', 'Trips'], r)        # Hearth Files: its name, the folder under it
        self.assertNotIn('', [x[1] for x in r])                      # an untitled shell surface isn't

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

    def test_title_bars_only_where_no_window_is_above(self):
        r = run_js('''console.log(JSON.stringify([visibleParts([{x: 0, y: 0, w: 100, h: 30}], [{x: 40, y: -10, w: 20, h: 100}]),
            visibleParts([{x: 0, y: 0, w: 100, h: 30}], [{x: -5, y: -5, w: 200, h: 200}]), visibleParts([{x: 0, y: 0, w: 100, h: 30}], [{x: 200, y: 0, w: 5, h: 5}]),
            stackOrder([{id: "a"}, {id: "b"}, {id: "c"}], ["b"], w => w.id).map(w => w.id),
            gameLine("Ember Trails", true, "60", true), gameLine("Ember Trails", false, "Off", false)]));''')
        self.assertEqual(r[0], [{'x': 0, 'y': 0, 'w': 40, 'h': 30}, {'x': 60, 'y': 0, 'w': 40, 'h': 30}])
        self.assertEqual(r[1], [])
        self.assertEqual(r[2], [{'x': 0, 'y': 0, 'w': 100, 'h': 30}])
        self.assertEqual(r[3], ['b', 'c', 'a'])                 # the one you went to last, then the newest
        self.assertEqual(r[4], 'Ember Trails · paused · 60 fps limit · VRR')
        self.assertEqual(r[5], 'Ember Trails · running')

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

    def test_steam_cloud_saves_from_steams_own_remote_folder(self):
        with tempfile.TemporaryDirectory() as t:
            os.makedirs(os.path.join(t, '7', '1000', 'remote'))
            open(os.path.join(t, '7', '1000', 'remote', 'save.dat'), 'w').close()
            os.makedirs(os.path.join(t, '7', '1001', 'remote'))              # an empty remote folder: nothing in the cloud
            ud = [os.path.join(t, '7')]
            self.assertTrue(self.h.cloud_saves('steam:1000', ud))
            self.assertFalse(self.h.cloud_saves('steam:1001', ud))
            self.assertFalse(self.h.cloud_saves('gog:1000', ud))
            self.assertFalse(self.h.cloud_saves('steam:../1000', ud))
            g = self.h.game_entry({'id': 'steam:1000', 'title': 'E', 'source': 'steam'}, (), ud)
            self.assertTrue(g['cloudSaves'])

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


class Watts(unittest.TestCase):
    """The performance overlay's watts: only from a real source, never made up."""
    def setUp(self):
        self.s = load(SHELL / 'hearth-stats', 'hearth_stats')

    def test_gpu_battery_and_cpu(self):
        with tempfile.TemporaryDirectory() as t:
            hw = os.path.join(t, 'hwmon'); os.makedirs(os.path.join(hw, 'hwmon0')); os.makedirs(os.path.join(hw, 'hwmon1'))
            open(os.path.join(hw, 'hwmon0', 'name'), 'w').write('k10temp\n')
            open(os.path.join(hw, 'hwmon1', 'name'), 'w').write('amdgpu\n')
            self.assertIsNone(self.s.gpu_watts(hw) if not self.s.shutil.which('nvidia-smi') else None)
            open(os.path.join(hw, 'hwmon1', 'power1_average'), 'w').write('18200000\n')
            self.assertEqual(self.s.gpu_watts(hw), 18.2)
            ps = os.path.join(t, 'ps'); os.makedirs(os.path.join(ps, 'BAT0'))
            open(os.path.join(ps, 'BAT0', 'status'), 'w').write('Charging\n')
            open(os.path.join(ps, 'BAT0', 'power_now'), 'w').write('14100000\n')
            self.assertIsNone(self.s.battery_watts(ps))                  # plugged in: not the device's draw
            open(os.path.join(ps, 'BAT0', 'status'), 'w').write('Discharging\n')
            self.assertEqual(self.s.battery_watts(ps), 14.1)
            pc = os.path.join(t, 'pc', 'intel-rapl:0'); os.makedirs(pc)
            open(os.path.join(pc, 'energy_uj'), 'w').write('1000000\n')
            self.assertIsNone(self.s.cpu_watts(os.path.join(t, 'pc'), now=100.0))   # the first reading: nothing yet
            open(os.path.join(pc, 'energy_uj'), 'w').write('20000000\n')
            self.assertEqual(self.s.cpu_watts(os.path.join(t, 'pc'), now=102.0), 9.5)
            self.assertIsNone(self.s.cpu_watts(os.path.join(t, 'nothing'), now=103.0))


class Guards(unittest.TestCase):
    def test_loaded_by_file_name_and_locked_shows_nothing(self):
        shell = (SHELL / 'shell.qml').read_text()
        self.assertIn('source: "Firehold.qml"', shell)
        self.assertNotRegex(code(shell), r'(?m)^\s*Firehold\s*\{')       # never as a type: a problem in it can't stop the shell
        f = code((SHELL / 'Firehold.qml').read_text())
        self.assertIn('cfg.firehold.unlocked === true) || cfg["firehold.unlocked"] === true', f)
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
            self.assertIn('visible: settingsReady && !remapping && !SettingsNav.fireholdDesk && !SettingsNav.fireholdCouch', (SHELL / name).read_text(), name)
        d = code((SHELL / 'FireholdDesk.qml').read_text())
        self.assertIn('Component.onCompleted: SettingsNav.fireholdDesk = true', d)
        self.assertIn('Component.onDestruction: SettingsNav.fireholdDesk = false', d)
        f = code((SHELL / 'Firehold.qml').read_text())
        self.assertIn('active: fhs.unlocked && fhs.started && fhs.posture === "desk"', f)
        self.assertIn('readonly property bool picturesWanted: unlocked && WinList.pictures && couchUp && page === "switcher"', f)

    def test_guide_frame_limit_and_scaling_go_through_the_firehold_game_helper(self):
        g = code((SHELL / 'FireholdGuide.qml').read_text())
        self.assertIn('hearth-firehold-game', g)
        self.assertIn('gameRun(gameSet, ["step", "frameLimit", d > 0 ? "up" : "down"])', g)
        self.assertIn('gameRun(gameSet, ["step", "scaling", d > 0 ? "up" : "down"])', g)
        self.assertIn('gameRun(gameProbe, ["status"])', g)
        self.assertNotIn('firehold-comp', g)                     # one way in: the helper, which saves per game itself

    def test_window_pictures_only_on_kwin_and_only_real_ids(self):
        h = load(SHELL / 'hearth-firehold', 'hearth_firehold2')
        os.environ.pop('HEARTH_WM', None)
        self.assertEqual(h.pictures('/nonexistent', ['{0b0f3c2e-1111-2222-3333-444455556666}']), {})
        self.assertTrue(h.WIN_ID.match('{0b0f3c2e-1111-2222-3333-444455556666}'))
        self.assertFalse(h.WIN_ID.match('../../etc/passwd'))

    def test_on_fireholds_own_compositor_pictures_desktops_frames_and_game_rows_are_its_own(self):
        # Window pictures: through the compositor's window IPC, numeric ids only, into the runtime folder.
        h = load(SHELL / 'hearth-firehold', 'hearth_firehold3')
        self.assertTrue(h.FH_ID.match('42'))
        self.assertFalse(h.FH_ID.match('../42'))
        old = dict(os.environ)
        try:
            os.environ.update(HEARTH_WM='firehold', XDG_RUNTIME_DIR='/nonexistent-run', WAYLAND_DISPLAY='wayland-x')
            self.assertEqual(h.pictures('/nonexistent-run/pics', ['../../x']), {})
        finally:
            os.environ.clear(); os.environ.update(old)
        w = code((SHELL / 'WinList.qml').read_text())
        for part in ('readonly property bool firehold: wm === "firehold"', 'readonly property bool bridged: kwin || fhReady',
                     'readonly property var toplevels: bridged ? kList : ToplevelManager.toplevels', '"hearth-firehold-bridge"',
                     'running: wl.firehold', 'function addDesktop(name)', 'function shot(w, path, max)'):
            self.assertIn(part, w)
        for name in ('DeskSwitcher.qml', 'WidgetPager.qml'):
            src = code((SHELL / name).read_text())
            self.assertIn('if (WinList.wmDesktops) { WinList.goDesktop(i + 1); return; }', src, name)
        f = code((SHELL / 'Firehold.qml').read_text())
        self.assertIn('active: fhs.unlocked && fhs.started && fhs.posture === "desk" && WinList.firehold && WinList.fhReady', f)
        fc = code((SHELL / 'FireholdFramesComp.qml').read_text())
        self.assertIn('op: "frames", style: "firehold"', fc)
        self.assertIn('Component.onDestruction: WinList.send({ op: "frames", style: "hearth" })', fc)
        self.assertNotIn('setText(', fc)
        g = code((SHELL / 'FireholdGuide.qml').read_text())
        self.assertIn('gameRun(gameSet, ["set", "vrr", fg.vrr ? "off" : "on"])', g)
        self.assertIn('gameRun(gameSet, ["set", "tearing", fg.tearing ? "off" : "on"])', g)
        self.assertIn('readonly property bool fhRows: comp && WinList.firehold', g)

    def test_the_firehold_bridge_says_when_the_compositor_isnt_there(self):
        import subprocess
        env = dict(os.environ, XDG_RUNTIME_DIR=tempfile.mkdtemp(), WAYLAND_DISPLAY='wayland-none')
        p = subprocess.run([sys.executable, str(SHELL / 'hearth-firehold-bridge'), 'serve', '--stdio'], input='',
                           capture_output=True, text=True, env=env, timeout=10)
        first = json.loads(p.stdout.splitlines()[0])
        self.assertEqual(first['t'], 'status')
        self.assertFalse(first['connected'])
        self.assertIn("isn't there", first['problem'])
        q = subprocess.run([sys.executable, str(SHELL / 'hearth-firehold-bridge'), 'list'], capture_output=True, text=True, env=env, timeout=10)
        self.assertEqual(q.returncode, 1)
        self.assertTrue(q.stdout.startswith('error: '))

    def test_settings_only_through_settings_writer_and_never_on_open(self):
        for name in ('Firehold.qml', 'FireholdCouch.qml', 'FireholdGuide.qml', 'FireholdSettings.qml', 'FireholdAboutRow.qml', 'FireholdNotice.qml', 'FireholdPerf.qml', 'FireholdDesk.qml', 'FireholdFramesComp.qml'):
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
        # The website (and the docs built into it). The compositor's own notes (docs/FIREHOLD.md) are its helper's.
        public = [ROOT / 'site', ROOT / 'README.md']
        hits = []
        for base in public:
            files = [base] if base.is_file() else [p for p in base.rglob('*') if p.is_file() and p.suffix in ('.html', '.md', '.js', '.json', '.txt')]
            for p in files:
                try:
                    t = p.read_text(errors='ignore')
                except OSError:
                    continue
                # The 2.0 line may say what Firehold will be; nothing about the preview or how to unlock it (the owner's rule).
                if re.search(r'(?i)firehold \(preview\)|firehold\.json|firehold\.unlocked|tap(ping)? the (hearth os )?version', t):
                    hits.append(str(p.relative_to(ROOT)))
        self.assertEqual(hits, [], 'Firehold is a hidden preview: nothing about the preview or its unlock on the site')
        for name in ('firehold.js', 'Firehold.qml', 'FireholdCouch.qml', 'FireholdGuide.qml', 'FireholdSettings.qml', 'hearth-firehold'):
            self.assertNotRegex((SHELL / name).read_text(), r'(?i)albireo', name)


if __name__ == '__main__':
    unittest.main()
