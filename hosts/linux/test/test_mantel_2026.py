"""Mantel 2026 (the desktop's facelift): the eight looks, settings that never change by themselves, Sunset off by
default, the Game Mode button in every look, the safe fallback to the classic taskbar, and hearth-stats (the
dashboard's and the Gaming look's numbers). docs/MANTEL.md."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
DESK = ROOT / 'hosts' / 'linux' / 'desktop'
SHELL = DESK / 'shell'
THEME = DESK / 'theme'
PAL = json.loads((THEME / 'palettes.json').read_text())
LOOKS = ['hearth', 'steam', 'gaming', 'zen', 'glass', 'synthwave', 'cabin', 'console']
TOKENS = ('bg', 'bg2', 'panel', 'panel2', 'panel3', 'line', 'text', 'muted', 'danger', 'ember', 'flame', 'ink')
NEW_KEYS = {'barStyle': '""', 'dockMode': '""', 'deskClock': '""', 'panelGlass': '""', 'sunset': 'false',
            'sunsetLight': '"07:00"', 'sunsetDark': '"19:30"', 'accentFrom': '""', 'wallSwatches': '[]'}
# Files new in Mantel 2026 that only draw things: they never change a setting.
DRAW_ONLY = ['Islands.qml', 'Dock.qml', 'Frame.qml', 'Frost.qml', 'FrostBlur.qml', 'Scene.qml', 'DeskClock.qml', 'HintStrip.qml',
             'PerfIsland.qml', 'Stats.qml', 'StatsCard.qml', 'MediaPill.qml', 'MediaCard.qml', 'WallAccent.qml', 'Theme.qml',
             'Wallpaper.qml', 'ClockPanel.qml', 'Notifications.qml', 'TaskStrip.qml', 'MenuTile.qml', 'MenuSearch.qml',
             'MenuHints.qml', 'MenuNowPlaying.qml', 'MenuCentred.qml', 'MenuTiles.qml', 'MenuStart.qml', 'MenuClassic.qml']
# (Launcher.qml writes only your live tiles' sizes, when you resize one: test_mantel_bars_menus.py checks that.)


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def code(text):
    """QML without // comments (so a comment can't satisfy or fail a check)."""
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in text.splitlines())


def block(text, start):
    """The text of the { } block that starts at the first `start`."""
    i = text.index(start)
    j = text.index('{', i)
    depth = 0
    for k in range(j, len(text)):
        depth += {'{': 1, '}': -1}.get(text[k], 0)
        if depth == 0:
            return text[i:k + 1]
    raise AssertionError('unbalanced: ' + start)


def run_theme(home, desktop):
    os.makedirs(os.path.join(home, '.local', 'share', 'nimbus'), exist_ok=True)
    with open(os.path.join(home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
        json.dump(desktop, f)
    env = dict(os.environ, HOME=home, XDG_RUNTIME_DIR=os.path.join(home, 'rt'))
    for k in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'LABWC_PID'):
        env.pop(k, None)
    out = subprocess.run([sys.executable, str(THEME / 'hearth-theme'), '--show'], env=env, capture_output=True, text=True, check=True).stdout
    return json.loads(out)


class Looks(unittest.TestCase):
    def test_all_eight_looks_are_there_and_complete(self):
        looks = [k for k, v in PAL.items() if not k.startswith('_') and isinstance(v, dict) and 'dark' in v]
        self.assertEqual(sorted(looks), sorted(LOOKS))
        orders = []
        for look in LOOKS:
            d = PAL[look]
            self.assertTrue(d.get('label') and d.get('desc'), look)
            self.assertIn(d.get('islandStyle'), ('floating', 'bar', 'shelf'), look)
            self.assertIn(d.get('frame'), ('plain', 'chamfer', 'neon', 'wood', 'frost'), look)
            self.assertIsInstance(d.get('corner'), int, look)
            orders.append(d.get('order'))
            for mode in ('dark', 'light'):
                for k in TOKENS:
                    self.assertRegex(d[mode][k], r'^#[0-9a-f]{6}$', (look, mode, k))
                self.assertTrue(0.4 < d[mode]['glass'] <= 1, (look, mode))
        self.assertEqual(sorted(orders), list(range(1, 9)))

    def test_the_two_old_looks_keep_what_they_had(self):
        # Existing users keep their look exactly: Hearth and Steam-style's colours, font and corners are unchanged.
        self.assertEqual(PAL['steam']['font'], 'Inter')
        self.assertEqual((PAL['steam']['corner'], PAL['steam']['windowCorner'], PAL['steam']['wallpaper']), (4, 4, 'anim:tide'))
        self.assertEqual(PAL['steam']['dark']['ember'], '#1a9fff')
        self.assertEqual(PAL['steam']['dark']['glass'], 0.97)
        self.assertEqual((PAL['hearth']['corner'], PAL['hearth']['windowCorner'], PAL['hearth']['wallpaper']), (0, 10, ''))
        self.assertEqual(PAL['hearth']['dark']['bg'], '#0f0b09')
        self.assertEqual(PAL['hearth']['dark']['ember'], '#ff8a3d')
        self.assertEqual(PAL['steam']['islandStyle'], 'bar')
        self.assertEqual(PAL['hearth']['islandStyle'], 'floating')

    def test_window_frames_and_apps_follow_every_look(self):
        for look in LOOKS:
            for mode in ('dark', 'light'):
                with tempfile.TemporaryDirectory() as d:
                    got = run_theme(d, {'look': look, 'mode': mode, 'accent': ''})
                    self.assertEqual((got['look'], got['mode']), (look, mode))
                    self.assertEqual(got['accent'], PAL[look][mode]['ember'])

    def test_drawn_wallpapers_exist_for_the_looks_that_use_them(self):
        scene = (SHELL / 'Scene.qml').read_text()
        wall = (SHELL / 'Wallpaper.qml').read_text()
        for look in LOOKS:
            w = PAL[look].get('wallpaper', '')
            if w.startswith('anim:') and w not in ('anim:tide',):
                name = w[5:]
                self.assertIn('root.name === "%s"' % name, scene, look)
                self.assertIn('"%s"' % name, wall, look)
            elif w.startswith('/'):
                self.assertTrue((ROOT / 'os' / 'branding' / 'wallpapers' / os.path.basename(w)).exists(), w)

    def test_theme_reads_the_style_tokens(self):
        t = (SHELL / 'Theme.qml').read_text()
        for tok in ('islandStyle', 'frame', 'dockAutoHide', 'deskClock', 'perfIsland', 'edgeGlow', 'scanlines', 'hints', 'thin'):
            self.assertIn('lookDef.' + tok, t, tok)
        # Your own Bar and dock choice wins over the look's.
        self.assertIn('Settings.barStyle === "floating" || Settings.barStyle === "bar" ? Settings.barStyle : lookIslands', t)
        self.assertIn('Settings.dockMode === "autohide" || (Settings.dockMode !== "shown" && lookDef.dockAutoHide === true)', t)


class SettingsNeverChangeByThemselves(unittest.TestCase):
    def test_new_options_start_at_defaults_without_being_written(self):
        s = (SHELL / 'Settings.qml').read_text()
        keys = re.search(r'readonly property var keys: (.*?\]\))', s, re.S).group(1)
        for k, default in NEW_KEYS.items():
            self.assertRegex(s, r'property (string|bool|var) %s: %s(?=\s)' % (k, re.escape(default)) if default != '[]'
                             else r'property var %s: \[\]' % k, k)
            self.assertIn('"%s"' % k, keys, k)
            self.assertIn('on%sChanged: save("%s")' % (k[0].upper() + k[1:], k), s, k)
        # Saving is a merge of the one key you changed, never while the file is being read in, never before it was.
        self.assertIn('if (!loaded || applying) return;', s)
        self.assertIn('writer.merge(o);', s)
        self.assertNotIn('setText(', code(s))

    def test_drawing_code_never_changes_a_setting(self):
        for f in DRAW_ONLY:
            t = code((SHELL / f).read_text())
            hits = re.findall(r'\bSettings\.\w+\s*=(?!=)', t)
            self.assertEqual(hits, [], f)

    def test_choosing_a_look_changes_only_the_look(self):
        # (Since the sub themes, 1.10: the look and the sub theme back to its first: test_sub_themes.py.)
        a = code((SHELL / 'SetAppearance.qml').read_text())
        self.assertIn('onChosen: v => page.apply(SubThemes.changesForLook(v))', a)
        js = (SHELL / 'subthemes.js').read_text()
        self.assertIn('function changesForLook(look) { return { look: look, subTheme: "" }; }', js)
        # The look's wallpaper, bar, dock, clock and transparency are only used where yours is "" (not set).
        t = (SHELL / 'Theme.qml').read_text()
        self.assertIn('readonly property string wall: Settings.wallpaper !== "" ? Settings.wallpaper : ownWall', t)
        w = (SHELL / 'Wallpaper.qml').read_text()
        self.assertIn('readonly property string wall: Theme.wall', w)

    def test_from_wallpaper_only_runs_when_you_ask(self):
        a = code((SHELL / 'SetAppearance.qml').read_text())
        calls = [m.start() for m in re.finditer(r'fromWallpaper\(', a)]
        # Defined once, called from the From wallpaper row and from picking a wallpaper (with it on): nowhere else.
        self.assertEqual(len(calls), 3, calls)
        self.assertIn('onActivated: page.fromWallpaper(Theme.wall)', a)
        self.assertIn('if (Settings.accentFrom === "wallpaper") fromWallpaper(', a)
        for f in ('SetAppearance.qml', 'Theme.qml', 'WallAccent.qml', 'Wallpaper.qml', 'Settings.qml'):
            self.assertNotIn('onWallpaperChanged: page.fromWallpaper', (SHELL / f).read_text())
            self.assertNotRegex(code((SHELL / f).read_text()), r'onWallpaperChanged:.*compute', f)
        wa = code((SHELL / 'WallAccent.qml').read_text())
        self.assertNotRegex(wa, r'\bSettings\.\w+\s*=(?!=)')
        self.assertNotIn('Timer { id: giveUp; interval: 8000; running: true', wa)

    def test_a_damaged_file_is_never_overwritten_with_the_new_defaults(self):
        # A value of the wrong type in desktop.json (an array where a string goes, a string for an array) is skipped,
        # not "fixed" by writing a default over it.
        s = (SHELL / 'Settings.qml').read_text()
        self.assertIn('typeof o[k] === typeof root[k] && Array.isArray(o[k]) === Array.isArray(root[k])', s)
        self.assertIn('writer.repair();', s)


class Sunset(unittest.TestCase):
    def test_off_by_default(self):
        s = (SHELL / 'Settings.qml').read_text()
        self.assertIn('property bool sunset: false', s)
        t = (SHELL / 'Theme.qml').read_text()
        self.assertIn('if (!Settings.sunset) return "";', t)
        self.assertIn('readonly property string mode: sunsetMode !== "" ? sunsetMode : Settings.mode === "light" ? "light" : "dark"', t)
        # Sunset never writes your Dark / Light choice.
        self.assertNotRegex(code(t), r'Settings\.mode\s*=(?!=)')

    def test_hearth_theme_follows_it_only_when_on(self):
        ht = load(THEME / 'hearth-theme', 'hearth_theme_sunset_test')
        noon = time.mktime((2026, 11, 4, 12, 0, 0, 0, 0, -1))
        night = time.mktime((2026, 11, 4, 22, 0, 0, 0, 0, -1))
        early = time.mktime((2026, 11, 4, 6, 59, 0, 0, 0, -1))
        self.assertIsNone(ht.sunset_mode({}, noon))
        self.assertIsNone(ht.sunset_mode({'sunset': False}, night))
        self.assertIsNone(ht.sunset_mode({'sunset': 'yes'}, night))            # only a real true turns it on
        on = {'sunset': True}
        self.assertEqual(ht.sunset_mode(on, noon), 'light')
        self.assertEqual(ht.sunset_mode(on, night), 'dark')
        self.assertEqual(ht.sunset_mode(on, early), 'dark')
        self.assertEqual(ht.sunset_mode(dict(on, sunsetLight='22:00', sunsetDark='06:00'), night), 'light')   # past midnight
        self.assertEqual(ht.sunset_mode(dict(on, sunsetLight='bad', sunsetDark='19:30'), noon), 'light')
        with tempfile.TemporaryDirectory() as d:
            # Off (the default): the stored mode, whatever the time.
            self.assertEqual(run_theme(d, {'look': 'hearth', 'mode': 'light', 'accent': ''})['mode'], 'light')
            self.assertEqual(run_theme(d, {'look': 'hearth', 'mode': 'dark', 'accent': ''})['mode'], 'dark')


class GameModeEverywhere(unittest.TestCase):
    def test_the_button_is_in_every_look(self):
        isl = code((SHELL / 'Islands.qml').read_text())
        comp = block(isl, 'component GameModeButton')
        self.assertIn('Quickshell.execDetached(["nimbus-mode", "game"])', comp)
        # The component itself is never hidden, and it's used in the band, outside any look-specific Loader.
        head = comp.split('Repeater')[0]
        self.assertNotRegex(head, r'\n\s{8}visible:')
        self.assertEqual(len(re.findall(r'\bGameModeButton \{', isl)), 1)
        band = isl[isl.index('id: bandItem'):isl.index('component Island:')]
        use = band.index('GameModeButton {')
        self.assertNotIn('Loader {', band[max(0, use - 120):use])
        self.assertNotIn('visible:', band[use:band.index('}', use)])
        # Every style a look can have is drawn by Islands.qml (floating, bar, shelf), with the button.
        for look in LOOKS:
            self.assertIn(PAL[look]['islandStyle'], ('floating', 'bar', 'shelf'))
        self.assertIn('readonly property bool shelf: style === "shelf"', isl)

    def test_and_in_the_fallback_quick_settings_and_launcher(self):
        bar = (SHELL / 'Bar.qml').read_text()
        self.assertIn('onClicked: Quickshell.execDetached(["nimbus-mode", "game"])', bar)
        qa = (SHELL / 'QuickAccess.qml').read_text()
        self.assertIn('text: "Return to Game Mode"', qa)
        self.assertIn('case "game": run(["nimbus-mode", "game"]); break;', qa)
        self.assertIn('rows = [["settings", "lock", "power"], ["game"]', qa)
        la = (SHELL / 'Launcher.qml').read_text()
        self.assertIn('ActionButton { label: "Game Mode"; icon: "pad"; primary: true; command: ["nimbus-mode", "game"] }', la)


class AlwaysStarts(unittest.TestCase):
    def test_new_parts_load_by_file_name_with_the_classic_taskbar_as_fallback(self):
        sh = code((SHELL / 'shell.qml').read_text())
        self.assertIn('Qt.createComponent("Islands.qml")', sh)
        self.assertIn('Qt.createComponent("Dock.qml")', sh)
        self.assertIn('ci.status !== Component.Ready || cd && cd.status !== Component.Ready', sh)
        self.assertNotRegex(sh, r'\b(Islands|Dock|Frame|Scene|HintStrip|Frost)\s*\{')
        self.assertIn('active: perScreen.portrait || perScreen.newFailed', sh)
        self.assertIn('Bar {', sh)
        # Files shell.qml uses directly only load the new parts through Loaders (a problem there can't stop the shell).
        for f in ('QuickAccess.qml', 'Launcher.qml', 'ClockPanel.qml', 'Wallpaper.qml', 'Notifications.qml', 'Bar.qml'):
            t = code((SHELL / f).read_text())
            self.assertNotRegex(t, r'(?<![\w.])(Frame|Scene|MediaCard|StatsCard|Stats|DeskClock|WallAccent|PerfIsland|MediaPill)\s*\{', f)

    def test_nothing_here_touches_booting_or_sessions(self):
        out = subprocess.run(['git', 'diff', '--name-only', 'origin/main'], cwd=ROOT, capture_output=True, text=True)
        if out.returncode != 0 or not out.stdout.strip():
            self.skipTest('no origin/main to compare with')
        for f in out.stdout.split():
            self.assertFalse(re.search(r'hosts/linux/session/|nimbus-session|os-bases/|/boot|grub|systemd', f), f)


class Stats(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.hs = load(SHELL / 'hearth-stats', 'hearth_stats_test')

    def test_cpu_and_memory(self):
        a = self.hs.cpu_times('cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 1 2 3 4\n')
        b = self.hs.cpu_times('cpu  150 0 150 900 0 0 0 0 0 0\n')
        self.assertEqual(self.hs.cpu_percent(a, b), 50)
        self.assertIsNone(self.hs.cpu_percent(None, b))
        self.assertEqual(self.hs.memory('MemTotal: 16777216 kB\nMemFree: 1 kB\nMemAvailable: 11429478 kB\nCached: 11429477 kB\n'), (5.1, 16.0))
        self.assertIsNone(self.hs.memory('nothing'))

    def test_fps_only_from_a_fresh_mangohud_log(self):
        self.assertEqual(self.hs.last_fps('os,cpu\nfps,frametime,cpu_load\n143.7,6.9,20\n144.2,6.9,21\n'), 144)
        self.assertIsNone(self.hs.last_fps('fps,frametime\n'))
        with tempfile.TemporaryDirectory() as home:
            logs = pathlib.Path(home, 'logs')
            logs.mkdir()
            conf = pathlib.Path(home, '.config', 'MangoHud')
            conf.mkdir(parents=True)
            (conf / 'MangoHud.conf').write_text('output_folder = %s\n' % logs)
            self.assertIsNone(self.hs.fps(home))                 # no log: no FPS (the island hides it)
            f = logs / 'game_2026.csv'
            f.write_text('fps,frametime\n90.1,11\n')
            self.assertEqual(self.hs.fps(home), 90)
            old = time.time() - 60
            os.utime(f, (old, old))
            self.assertIsNone(self.hs.fps(home))                 # the game ended: no FPS

    def test_temperatures_and_gpu(self):
        with tempfile.TemporaryDirectory() as d:
            for i, (name, t) in enumerate((('acpitz', '40000'), ('k10temp', '58250'), ('amdgpu', '61000'))):
                p = pathlib.Path(d, 'hwmon%d' % i)
                p.mkdir()
                (p / 'name').write_text(name + '\n')
                (p / 'temp1_input').write_text(t + '\n')
            self.assertEqual(self.hs.hwmon_temps(d), {'cpu': 58, 'gpu': 61})
            os.makedirs(os.path.join(d, 'card0', 'device'))
            pathlib.Path(d, 'card0', 'device', 'gpu_busy_percent').write_text('41\n')
            self.assertEqual(self.hs.gpu_busy(d), 41)

    def test_it_only_reads(self):
        t = (SHELL / 'hearth-stats').read_text()
        self.assertNotRegex(t, r"open\([^)]*['\"][wa]")


if __name__ == '__main__':
    unittest.main()


class DockAfterGameMode(unittest.TestCase):
    """Coming back from Game Mode, an auto-hide dock never keeps the bottom of the screen reserved."""

    def test_dock_waits_for_settings_and_refits_when_its_space_changes(self):
        d = (SHELL / 'Dock.qml').read_text()
        self.assertIn('property bool settingsReady: Settings.loaded', d)
        self.assertRegex(d, r'Timer \{ interval: 4000; running: !dock\.settingsReady; onTriggered: dock\.settingsReady = true \}',
                         'the dock always comes up, even if settings never load')
        self.assertIn('visible: settingsReady && !remapping', d)
        self.assertIn('onReservedChanged: { claim(); refit(); }', d)
        self.assertEqual(d.count('onReservedChanged'), 1, 'one handler only (QML refuses two)')
        self.assertRegex(d, r'readonly property int reserved: autoHide \|\| keyboardOpen \? 0 :', 'auto-hide reserves nothing')
        self.assertIn('id: remapTimer; interval: 150', d)
