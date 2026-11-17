"""Tests for Settings › Window effects' and Virtual desktops' 1.14 extras (KDE-FEATURES-1.14.md): the second switcher
(kwinrc [TabBoxAlternative]), hiding the pointer after a time, show FPS / show paint, KWin's scripts (minimize all,
video wall, switcher follows the taskbar) and the Desktop Cube (kwin-addons, optional in the Debian image). All without
KWin (no session bus): only the keys changed are written, every other key is kept, and the pages say what's missing."""
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
K = load(SHELL / 'hearth-kwin-settings', 'hearth_kwin_settings_extras_test')
EFFECTS_QML = (SHELL / 'SetKwinEffects.qml').read_text()
DESKTOPS_QML = (SHELL / 'SetKwinDesktops.qml').read_text()


class Env(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = self.tmp.name
        self.home = os.path.join(t, 'home', '.config')
        self.data = os.path.join(t, 'share')
        os.makedirs(self.home)
        os.makedirs(os.path.join(t, 'xdg'))
        self.env = mock.patch.dict(os.environ, {'HOME': os.path.join(t, 'home'), 'XDG_CONFIG_HOME': self.home,
                                                'XDG_CONFIG_DIRS': os.path.join(t, 'xdg'), 'XDG_RUNTIME_DIR': t,
                                                'XDG_DATA_HOME': os.path.join(t, 'datahome'), 'XDG_DATA_DIRS': self.data,
                                                'XDG_STATE_HOME': os.path.join(t, 'state'),
                                                'DBUS_SESSION_BUS_ADDRESS': 'unix:path=/nonexistent'})
        self.env.start()
        os.environ.pop('HEARTH_KWIN_FIXTURE', None)
        K._BUS[:] = [None]

    def tearDown(self):
        self.env.stop()
        K._BUS[:] = []
        self.tmp.cleanup()

    def kwinrc(self):
        with open(os.path.join(self.home, 'kwinrc')) as f:
            return f.read()


class SecondSwitcher(Env):
    def test_alternative_group_allowed_and_only_its_keys_written(self):
        with open(os.path.join(self.home, 'kwinrc'), 'w') as f:
            f.write('[TabBox]\nLayoutName=org.hearth.switcher\n\n[Windows]\nRollOverDesktops=false\n')
        K.set_file('kwinrc', json.dumps({'TabBoxAlternative': {'ShowTabBox': True, 'LayoutName': 'thumbnail_grid',
                                                                'ApplicationsMode': 2}}))
        vals, mine, _ = kc.effective('kwinrc')
        self.assertEqual(vals['TabBoxAlternative'], {'ShowTabBox': 'true', 'LayoutName': 'thumbnail_grid',
                                                     'ApplicationsMode': '2'})
        self.assertEqual(vals['TabBox'], {'LayoutName': 'org.hearth.switcher'})     # Alt+Tab's own look kept
        self.assertEqual(vals['Windows'], {'RollOverDesktops': 'false'})
        self.assertTrue(os.path.exists(os.path.join(self.home, 'kwinrc.bak')))

    def test_page_reads_and_writes_the_alternative_group(self):
        self.assertIn('Second switcher (Super+Tab)', EFFECTS_QML)
        for key in ('ApplicationsMode', 'DesktopMode', 'MinimizedMode'):
            self.assertIn('"TabBoxAlternative", "%s"' % key, EFFECTS_QML)
        self.assertIn('page.setLook("TabBoxAlternative", v)', EFFECTS_QML)
        self.assertIn("Alt+` shows only the windows of the app you're on", EFFECTS_QML)
        # Super+Tab really is the alternative switcher in Hearth's KWin shortcuts.
        sc = json.loads((ROOT / 'hosts/linux/desktop/kwin/windows/shortcuts.json').read_text())
        self.assertIn({'labwc': 'W-Tab', 'kwin': 'Walk Through Windows Alternative', 'keys': 'Meta+Tab'}, sc['kwinOwn'])


class HidePointer(Env):
    def test_after_a_time_choice(self):
        self.assertIn('"Effect-hidecursor", "InactivityDuration"', EFFECTS_QML)
        self.assertIn('Hide the pointer when the mouse is still', EFFECTS_QML)
        # Turning it on for "when still" alone doesn't start hiding it while typing (HideOnTyping is on by default).
        self.assertRegex(EFFECTS_QML, r'if \(!page\.hideTyping && kw\.b\("kwinrc", "Effect-hidecursor", "HideOnTyping", true\)\) fx\.HideOnTyping = false')
        self.assertIn('hidecursorEnabled: sec > 0 || page.hideTyping', EFFECTS_QML)

    def test_written_through_the_helper(self):
        K.set_file('kwinrc', json.dumps({'Plugins': {'hidecursorEnabled': True},
                                         'Effect-hidecursor': {'InactivityDuration': 10, 'HideOnTyping': False}}))
        vals, _, _ = kc.effective('kwinrc')
        self.assertEqual(vals['Effect-hidecursor'], {'InactivityDuration': '10', 'HideOnTyping': 'false'})


class Developers(Env):
    def test_fps_and_paint(self):
        self.assertIn('SetHeader { text: "For developers" }', EFFECTS_QML)
        for e in ('showfps', 'showpaint'):
            self.assertIn(e, K.EFFECTS)
            self.assertFalse(K.EFFECTS[e])
            self.assertIn('kw.setEffect("%s", v)' % e, EFFECTS_QML)
        dev = EFFECTS_QML[EFFECTS_QML.index('"For developers"'):]
        self.assertIn('Show FPS', dev)
        self.assertIn('Show paint', dev)


class Scripts(Env):
    def test_installed_scripts_found(self):
        for n in ('minimizeall', 'videowall'):
            os.makedirs(os.path.join(self.data, 'kwin', 'scripts', n))
        os.makedirs(os.path.join(self.data, 'kwin-wayland', 'scripts', 'synchronizeskipswitcher'))
        self.assertEqual(sorted(K.scripts()), ['minimizeall', 'synchronizeskipswitcher', 'videowall'])
        self.assertEqual(sorted(K.status()['scripts']), ['minimizeall', 'synchronizeskipswitcher', 'videowall'])

    def test_no_scripts_folder_means_unknown(self):
        self.assertIsNone(K.scripts())

    def test_switches_and_reload(self):
        for n in ('minimizeall', 'videowall', 'synchronizeskipswitcher'):
            self.assertIn(n, K.SCRIPTS)
            self.assertFalse(K.EFFECTS[n])
            self.assertIn('kw.setEffect("%s", v)' % n, EFFECTS_QML)
            self.assertIn('page.hasScript("%s")' % n, EFFECTS_QML)
        with mock.patch.object(K, 'scripting_start') as start:
            K.set_file('kwinrc', json.dumps({'Plugins': {'videowallEnabled': True}}))
            start.assert_called_once()
        with mock.patch.object(K, 'scripting_start') as start:
            K.set_file('kwinrc', json.dumps({'Plugins': {'blurEnabled': False}}))
            start.assert_not_called()
        vals, _, _ = kc.effective('kwinrc')
        self.assertEqual(vals['Plugins'], {'videowallEnabled': 'true', 'blurEnabled': 'false'})

    def test_scripting_start_without_kwin_does_nothing(self):
        self.assertFalse(K.scripting_start())


class Cube(Env):
    def test_debian_image_installs_it_optionally(self):
        build = (ROOT / 'os-bases/peppermint/build.sh').read_text()
        self.assertRegex(build, r'(?m)^optional kwin-addons$')
        self.assertNotRegex(build, r'(?m)^"\$\{APT\[@\]\}" .*kwin-addons')    # never required

    def test_switch_and_reason(self):
        self.assertIn('cube', K.EFFECTS)
        self.assertEqual(K.SHORTCUTS['cube'], 'Cube')
        self.assertIn('kw.setEffect("cube", v)', DESKTOPS_QML)
        self.assertIn('enabled: kw.hasEffect("cube")', DESKTOPS_QML)
        self.assertIn("the cube comes with the kwin-addons package, which isn't installed", DESKTOPS_QML)
        self.assertIn('kw.shortcut("cube")', DESKTOPS_QML)

    def test_preview_fixture_lists_the_new_effects(self):
        fx = json.loads((ROOT / 'hosts/linux/test/fixtures/kwin-preview.json').read_text())
        for e in ('cube', 'showfps', 'showpaint'):
            self.assertIn(e, fx['effects'])
        self.assertEqual(sorted(fx['scripts']), ['desktopchangeosd', 'minimizeall', 'synchronizeskipswitcher', 'videowall'])


class Index(unittest.TestCase):
    def test_search_finds_the_new_settings(self):
        idx = (SHELL / 'settings-index.js').read_text()
        for t in ("Second switcher's look", 'Hide the pointer when the mouse is still', 'Show FPS', 'Show paint', 'Video wall',
                  'Desktop cube'):
            self.assertIn(t, idx, t)


if __name__ == '__main__':
    unittest.main()
