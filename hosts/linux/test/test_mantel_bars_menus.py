"""Mantel's bar layouts and five app menus (docs/MANTEL.md): the main bar on any edge with the floating dock on or
off, auto-hide and reserved space on every edge (and the Game Mode return fix), the classic taskbar still the
fallback, the five app menus loaded by file name, settings that are only written when you change them (new handheld
installs start with the Start screen without writing it), and hearth-appmenu (recent files, file search, live
tiles' data)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
STYLES = {'centred': 'MenuCentred.qml', 'tiles': 'MenuTiles.qml', 'start': 'MenuStart.qml', 'classic': 'MenuClassic.qml'}
NEW = {'barLayout': ('string', '""'), 'barEdge': ('string', '""'), 'dockFloat': ('string', '""'), 'appMenu': ('string', '""'),
       'menuTiles': ('var', '({})')}
NEW_FILES = ['TaskStrip.qml', 'BarLayoutPicker.qml', 'AppMenuPicker.qml', 'MenuSearch.qml', 'MenuTile.qml', 'MenuHints.qml',
             'MenuNowPlaying.qml', 'menu-nav.js', 'hearth-appmenu'] + list(STYLES.values())


def read(name):
    return (SHELL / name).read_text()


def code(text):
    """QML / JS without // comments."""
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in text.splitlines())


def block(text, start):
    i = text.index(start)
    j = text.index('{', i)
    depth = 0
    for k in range(j, len(text)):
        depth += {'{': 1, '}': -1}.get(text[k], 0)
        if depth == 0:
            return text[i:k + 1]
    raise AssertionError('unbalanced: ' + start)


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class SettingsOnlyWhenYouChangeThem(unittest.TestCase):
    def test_new_keys_default_to_nothing_and_are_saved_one_at_a_time(self):
        s = read('Settings.qml')
        keys = re.search(r'readonly property var keys: (.*?\]\))\s*\n\s*on', s, re.S).group(1)
        for k, (typ, default) in NEW.items():
            self.assertRegex(s, r'property %s %s: %s(?=\s)' % (typ, k, re.escape(default)), k)
            self.assertIn('"%s"' % k, keys, k)
            self.assertIn('on%sChanged: save("%s")' % (k[0].upper() + k[1:], k), s, k)
        self.assertIn('if (!loaded || applying) return;', s)

    def test_only_the_settings_controls_write_them(self):
        allowed = {'BarLayoutPicker.qml': {'barLayout', 'barEdge', 'dockFloat', 'dockMode'},
                   'AppMenuPicker.qml': {'appMenu', 'barLayout', 'barEdge', 'dockFloat'},
                   'Launcher.qml': {'menuTiles'}}
        for f in sorted(SHELL.glob('*.qml')):
            hits = set(re.findall(r'\bSettings\.(barLayout|barEdge|dockFloat|appMenu|menuTiles)\s*=(?!=)', code(f.read_text())))
            self.assertTrue(hits <= allowed.get(f.name, set()), '%s writes %s' % (f.name, hits))
        # Launcher writes your tiles only from the Resize / Take off / Add as a tile actions.
        la = code(read('Launcher.qml'))
        for m in re.finditer(r'Settings\.\w+\s*=(?!=)', la):
            fn = la.rfind('function ', 0, m.start())
            self.assertRegex(la[fn:fn + 40], r'function (setTileSize|setTileHidden)\(', la[fn:fn + 60])

    def test_picking_tiles_or_classic_asks_and_only_match_changes_the_bar(self):
        a = code(read('AppMenuPicker.qml'))
        choose = block(a, 'function choose(')
        self.assertEqual(re.findall(r'Settings\.(\w+)\s*=(?!=)', choose), ['appMenu'])
        self.assertIn('(v === "tiles" || v === "classic") && !barMatches', choose)
        match = block(a, 'function match(')
        self.assertEqual(sorted(re.findall(r'Settings\.(\w+)\s*=(?!=)', match)), ['barEdge', 'barLayout', 'dockFloat'])
        self.assertIn('Settings.barLayout = "one"', match)
        self.assertIn('Settings.barEdge = "bottom"', match)
        self.assertIn('Settings.dockFloat = "off"', match)
        # match() is called only from the Match the menu button / row, Keep mine changes nothing.
        self.assertEqual(len(re.findall(r'amp\.match\(\)', a)), 2)
        self.assertIn('label: "Keep mine"', a)
        self.assertIn('onPicked: amp.asking = ""', a)

    def test_each_bar_control_writes_only_its_own_key(self):
        b = code(read('BarLayoutPicker.qml'))
        for line in b.splitlines():
            found = re.findall(r'Settings\.(\w+)\s*=(?!=)', line)
            self.assertLessEqual(len(found), 1, line)

    def test_start_screen_is_the_default_only_on_new_handheld_installs_and_is_never_written(self):
        t = read('Theme.qml')
        self.assertIn('readonly property string appMenuDefault: handheld && installedWithMenus ? "start" : "hearth"', t)
        self.assertIn('appMenus.indexOf(Settings.appMenu) >= 0 ? Settings.appMenu : appMenuDefault', t)
        self.assertIn('path: "/etc/nimbus/installed.json"', t)
        self.assertNotRegex(code(t), r'Settings\.\w+\s*=(?!=)')
        # The installer marks a new install; nothing else writes that file.
        inst = (ROOT / 'os' / 'live' / 'src' / 'hearth-install').read_text()
        self.assertIn("'appMenus': 1", inst)
        # The handheld names are hearth-link's (one list of what's a handheld).
        rust = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'discovery.rs').read_text()
        names = re.findall(r'"([^"]+)"', re.search(r'const HANDHELDS: \[&str; \d+\] = \[(.*?)\];', rust).group(1))
        qml = re.findall(r'"([^"]+)"', re.search(r'handheldNames: \[(.*?)\]', t).group(1))
        self.assertEqual(names, qml)

    def test_existing_users_keep_their_layout(self):
        t = read('Theme.qml')
        # Nothing set: the top bar and the floating dock (today's Mantel 2026), the Hearth menu.
        self.assertIn('readonly property bool oneBar: Settings.barLayout === "one"', t)
        self.assertIn(': "bottom") : "top"', t)
        self.assertIn('readonly property bool dockFloats: Settings.dockFloat !== "off"', t)
        # The dock's auto-hide is as before in the default layout; the main bar only hides in One main bar or with your apps.
        self.assertIn('readonly property bool barAutoHide: dockAutoHide && (oneBar || !dockFloats)', t)


class BarOnEveryEdge(unittest.TestCase):
    def test_anchors_and_reserved_space_for_each_edge(self):
        i = code(read('Islands.qml'))
        self.assertIn('anchors { top: edge !== "bottom"; bottom: edge !== "top"; left: edge !== "right"; right: edge !== "left" }', i)
        self.assertIn('implicitHeight: vertical ? 0 : band', i)
        self.assertIn('implicitWidth: vertical ? band : 0', i)
        self.assertIn('exclusiveZone: reserved', i)
        self.assertIn('readonly property int reserved: autoHide ? 0 : band', i)
        for e, k in (('top', 'barTop'), ('bottom', 'barBottom'), ('left', 'barLeft'), ('right', 'barRight')):
            self.assertIn('DeskState.%s = edge === "%s" ? reserved : 0;' % (k, e), i)

    def test_game_mode_return_fix_on_every_edge(self):
        i = read('Islands.qml')
        self.assertIn('property bool settingsReady: Settings.loaded', i)
        self.assertRegex(i, r'Timer \{ interval: 4000; running: !isl\.settingsReady; onTriggered: isl\.settingsReady = true \}')
        self.assertIn('visible: settingsReady && !remapping', i)
        self.assertIn('onReservedChanged: { claim(); refit(); }', i)
        self.assertEqual(i.count('onReservedChanged'), 1)
        self.assertIn('id: remapTimer; interval: 150', i)
        # A bar being replaced never zeroes what the new one set.
        self.assertIn('if (DeskState.barOwner !== isl) return;', i)
        d = read('Dock.qml')
        self.assertIn('if (DeskState.dockOwner === dock)', d)

    def test_vertical_bars_stack_and_menus_open_toward_the_screen(self):
        i = code(read('Islands.qml'))
        self.assertGreaterEqual(i.count('flow: isl.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight'), 3)
        self.assertIn('if (edge === "left") return Qt.point(isl.band + 2', i)
        self.assertIn('if (edge === "right") return Qt.point(sw - isl.band - 290', i)
        self.assertIn('{ title: title, above: edge === "bottom" }', i)
        self.assertIn('readonly property bool pillCentred: !vertical && !holdsApps && !shelf', i)
        q = code(read('QuickAccess.qml'))
        self.assertIn('DeskState.barEdge === "left"', q)
        self.assertIn('DeskState.barEdge === "bottom"', q)

    def test_apps_in_the_bar_when_the_floating_app_menu_is_off(self):
        i = code(read('Islands.qml'))
        self.assertIn('source: "TaskStrip.qml"', i)
        self.assertIn('active: isl.holdsApps', i)
        ts = code(read('TaskStrip.qml'))
        for fn in ('function activateNth(', 'function toggleShowDesktop(', 'function taskMenu(', 'function finishDrag('):
            self.assertIn(fn, ts)
        self.assertIn('DeskState.mainBar = strip', ts)

    def test_layout_changes_rebuild_live_and_the_classic_taskbar_stays_the_fallback(self):
        sh = code(read('shell.qml'))
        self.assertIn('const wantDock = Theme.dockFloats;', sh)
        self.assertIn('edge: Theme.barEdge, holdsApps: !wantDock', sh)
        self.assertIn('function onBarKeyChanged()', sh)
        self.assertIn('Timer { id: rebuild; interval: 300;', sh)
        self.assertIn('active: perScreen.portrait || perScreen.newFailed', sh)
        self.assertIn('newFailed = true;', sh)
        t = read('Theme.qml')
        self.assertIn('readonly property string barKey: barEdge + "/" + (dockFloats ? "dock" : "nodock")', t)

    def test_the_game_mode_button_on_every_edge(self):
        i = code(read('Islands.qml'))
        comp = block(i, 'component GameModeButton')
        self.assertIn('implicitWidth: isl.vertical ? isl.islandH - 6 :', comp)
        self.assertIn('Quickshell.execDetached(["nimbus-mode", "game"])', comp)


class FiveAppMenus(unittest.TestCase):
    def test_loaded_by_file_name_with_the_hearth_menu_as_fallback(self):
        la = code(read('Launcher.qml'))
        for style, f in STYLES.items():
            self.assertIn('%s: "%s"' % (style, f), la)
            self.assertTrue((SHELL / f).exists(), f)
        self.assertIn('readonly property bool styled: style !== "hearth" && styleLoader.status === Loader.Ready', la)
        self.assertIn('visible: !root.styled', la)
        self.assertIn('readonly property string style: Theme.appMenu', la)
        self.assertNotRegex(la, r'(?<![\w.])(MenuCentred|MenuTiles|MenuStart|MenuClassic)\s*\{')

    def test_every_menu_has_search_controller_keys_and_the_contract(self):
        for style, f in STYLES.items():
            t = code(read(f))
            for need in ('property var menu: null', 'readonly property color scrim:', 'readonly property bool wantsLive:', 'function opened()',
                         'MenuSearch {', 'Qt.Key_Return', 'Qt.Key_Escape', 'Qt.Key_Menu', 'Keys.onPressed: e => st.key(e)'):
                self.assertIn(need, t, '%s: %s' % (f, need))
            if style != 'classic':
                self.assertIn('Qt.Key_Tab', t, f)
            self.assertTrue('MenuHints' in t, f)

    def test_search_covers_apps_games_settings_and_files(self):
        la = code(read('Launcher.qml'))
        fn = block(la, 'function searchAll(')
        for part in ('out.apps', 'out.games', 'out.pages', 'out.files'):
            self.assertIn(part, fn)
        self.assertIn('"files", root.fileQuery', la)

    def test_the_flame_and_no_other_companys_names(self):
        for f in NEW_FILES + ['Launcher.qml', 'Islands.qml']:
            t = read(f)
            self.assertNotRegex(t, r'(?i)\b(windows?\s?(7|8|10|11|logo)|microsoft|win(7|8|10|11)\b)', f)
        i = read('Islands.qml')
        self.assertIn('objectName: "appMenuButton"', i)
        self.assertIn('images/hearth-logo.png', block(code(i), 'objectName: "appMenuButton"'))

    def test_live_tiles_sizes_and_plain_fallback(self):
        mt = code(read('MenuTile.qml'))
        self.assertIn('readonly property int cols: size === "small" ? 1 : size === "medium" ? 2 : 4', mt)
        self.assertIn('readonly property int rows: size === "small" ? 1 : size === "large" ? 4 : 2', mt)
        self.assertIn('readonly property bool plain:', mt)
        self.assertIn('source: "MenuNowPlaying.qml"', mt)
        la = code(read('Launcher.qml'))
        self.assertIn('["small", "medium", "wide", "large"].indexOf(s) >= 0 ? s : dflt', la)

    def test_jump_lists_use_recent_files_then_app_actions(self):
        la = code(read('Launcher.qml'))
        jl = block(la, 'function jumpList(')
        self.assertLess(jl.index('recentFor(entry)'), jl.index('entry.actions'))


class AppMenuHelper(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = load(SHELL / 'hearth-appmenu', 'hearth_appmenu_test')

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.home = self.tmp.name
        old = dict(os.environ)
        self.addCleanup(lambda: (os.environ.clear(), os.environ.update(old)))
        os.environ['HOME'] = self.home
        os.environ.pop('XDG_DATA_HOME', None)
        os.environ.pop('XDG_CACHE_HOME', None)

    def write(self, rel, text):
        p = os.path.join(self.home, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def test_recent_files_newest_first_with_their_apps(self):
        a = self.write('Documents/Budget 2026.ods', 'x')
        b = self.write('Pictures/shot.png', 'x')
        xbel = f'''<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0" xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks" xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">
  <bookmark href="file://{a.replace(' ', '%20')}" added="2026-11-01T10:00:00Z" modified="2026-11-04T21:14:00.123Z" visited="2026-11-04T21:14:00Z">
    <info><metadata owner="http://freedesktop.org"><bookmark:applications>
      <bookmark:application name="LibreOffice" exec="&apos;soffice %u&apos;" modified="2026-11-04T21:14:00Z" count="2"/>
    </bookmark:applications></metadata></info></bookmark>
  <bookmark href="file://{b}" added="2026-11-05T09:00:00Z" modified="2026-11-05T09:00:00Z" visited="2026-11-05T09:00:00Z">
    <info><metadata owner="http://freedesktop.org"><bookmark:applications>
      <bookmark:application name="org.gnome.Loupe" exec="&apos;loupe %u&apos;" modified="2026-11-05T09:00:00Z" count="1"/>
    </bookmark:applications></metadata></info></bookmark>
  <bookmark href="file://{self.home}/gone.txt" added="2026-11-05T10:00:00Z" modified="2026-11-05T10:00:00Z" visited="2026-11-05T10:00:00Z"/>
</xbel>'''
        self.write('.local/share/recently-used.xbel', xbel)
        r = self.m.recent()
        self.assertEqual([x['name'] for x in r], ['shot.png', 'Budget 2026.ods'])   # newest first; a file that's gone left out
        self.assertEqual(r[1]['apps'], ['LibreOffice'])
        self.assertEqual(r[1]['execs'], ['soffice'])
        self.assertEqual(r[1]['folder'], 'Documents')
        self.assertGreater(r[0]['when'], r[1]['when'])

    def test_no_or_broken_list_is_just_empty(self):
        self.assertEqual(self.m.recent(), [])
        self.write('.local/share/recently-used.xbel', '<xbel><bookmark')
        self.assertEqual(self.m.recent(), [])

    def test_file_search_skips_hidden_and_build_folders(self):
        self.write('Documents/stream-setup.txt', 'x')
        self.write('Music/Streams/a.mp3', 'x')
        self.write('.cache/stream-cache.bin', 'x')
        self.write('code/node_modules/stream/index.js', 'x')
        names = [h['name'] for h in self.m.find_files('stream')]
        self.assertIn('stream-setup.txt', names)
        self.assertIn('Streams', names)
        self.assertNotIn('stream-cache.bin', names)
        self.assertNotIn('index.js', names)
        self.assertEqual(self.m.find_files('s'), [])      # one letter: nothing (too many)

    def test_next_events_and_game_updates(self):
        now = int(time.mktime((2026, 11, 5, 12, 0, 0, 0, 0, -1)) * 1000)
        ev = [{'title': 'Bins out', 'date': '2026-11-06', 'allDay': True},
              {'title': 'Game night', 'date': '2026-11-05', 'start': now + 8 * 3600000, 'end': now + 10 * 3600000},
              {'title': 'Done already', 'date': '2026-11-05', 'start': now - 7200000, 'end': now - 3600000}]
        self.assertEqual([e['title'] for e in self.m.next_events(ev, now)], ['Game night', 'Bins out'])
        self.assertIsNone(self.m.game_updates())
        self.write('.cache/hearth/desktop-games.json', json.dumps([{'title': 'Veloren', 'update': True}, {'title': 'Wesnoth'}]))
        self.assertEqual(self.m.game_updates(), {'count': 1, 'titles': ['Veloren']})

    def test_tiles_leave_out_what_is_missing(self):
        calls = []

        def fake(cmd, timeout):
            calls.append(cmd)
            if cmd[:2] == ['hearth-time', '--weather-summary']:
                return {'line': '18° Partly cloudy', 'place': 'Leeds'}
            if 'quick' in cmd:
                return {'setupDone': True, 'shares': 2, 'people': 3, 'usage': {'used': 1, 'total': 4}, 'backups': {'failed': ['Photos']}, 'alerts': []}
            return None
        orig = self.m.run_json
        self.m.run_json = fake
        self.addCleanup(setattr, self.m, 'run_json', orig)
        t = self.m.tiles()
        self.assertEqual(t['weather']['line'], '18° Partly cloudy')
        self.assertNotIn('events', t)                       # no Hearth Calendar: no calendar data (a plain tile)
        self.assertFalse(t['home']['ok'])
        self.assertEqual(t['home']['problems'], ['Backup failed: Photos'])
        self.assertNotIn('updates', t)

    def test_it_only_reads(self):
        src = code(read('hearth-appmenu'))
        self.assertNotRegex(src, r"open\([^)]*['\"][wa]['\"]")
        self.assertNotRegex(src, r'\b(os\.remove|os\.unlink|shutil\.|os\.rename|os\.replace)\b')


class DesktopTestCoversIt(unittest.TestCase):
    def test_screenshots_and_the_never_written_check(self):
        wf = (ROOT / '.github' / 'workflows' / 'desktop-test.yml').read_text()
        self.assertIn('test_mantel_bars_menus.py', wf)
        self.assertIn('for e in bottom top left right; do', wf)
        for shot in ('bar-$e-1-desktop', 'bar-bottom-dock-on', 'dock-off-top-bar', 'autohide', 'menu_shot centred', 'menu_shot tiles',
                     'menu_shot start', 'menu_shot classic', 'menu-search-centred', 'menu-picker', 'glass-menu-classic'):
            self.assertIn(shot, wf, shot)
        self.assertIn("'barLayout', 'barEdge', 'dockFloat', 'appMenu', 'menuTiles'", wf)


if __name__ == '__main__':
    unittest.main()
