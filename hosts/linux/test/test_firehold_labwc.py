"""Firehold on labwc draws the approved Couch (1.16, docs/mockups/checklists/firehold-labwc.md): the very pages
Firehold on Plasma draws (desktop/firehold-plasma/couch/contents/ui, reached from Mantel's shell through fhcouch/),
with the desktop's side given by a host (PlasmaHost.qml under Plasma, FhCouchHost.qml under labwc). These checks keep
the two in step and keep KDE-only pieces out of what labwc loads; hearth-firehold-plasma's labwc paths never call KWin
or plasmashell and never write a setting when the modes change (red flag 9).
"""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
UI = ROOT / 'hosts/linux/desktop/firehold-plasma/couch/contents/ui'
HELPER = ROOT / 'hosts/linux/desktop/firehold-plasma/hearth-firehold-plasma'
KDE_ONLY = ('main.qml', 'PlasmaHost.qml', 'PlasmaMedia.qml', 'KIcon.qml', 'Thumb.qml', 'GuideLayer.qml', 'GuideWindow.qml', 'Guide.qml')


def imports(text):
    return re.findall(r'^import\s+(\S+)', text, re.M)


class SharedCouch(unittest.TestCase):
    def test_fhcouch_is_the_plasma_couch(self):
        # (copied when the image is built: Quickshell doesn't follow links into its shell folder)
        import shutil, subprocess
        with tempfile.TemporaryDirectory() as t:
            d = pathlib.Path(t) / 'desktop'
            shutil.copytree(ROOT / 'hosts/linux/desktop/firehold-plasma', d / 'firehold-plasma', symlinks=True)
            shutil.copytree(ROOT / 'hosts/linux/desktop/plasma-hearth', d / 'plasma-hearth', symlinks=True)
            (d / 'shell').mkdir()
            shutil.copy(SHELL / 'firehold.js', d / 'shell')
            subprocess.run(['bash', str(ROOT / 'hosts/linux/desktop/labwc-fedora/install-couch.sh'), str(d)], check=True, capture_output=True)
            out = d / 'shell/fhcouch'
            for f in sorted(UI.iterdir()):
                if f.suffix not in ('.qml', '.js'):
                    continue
                if f.name in KDE_ONLY:
                    self.assertFalse((out / f.name).exists(), '%s is KDE-only: labwc must not load it' % f.name)
                else:
                    self.assertEqual((out / f.name).read_bytes(), f.read_bytes(), f.name)
            self.assertEqual((out / 'firehold.js').read_bytes(), (SHELL / 'firehold.js').read_bytes())
            # Firehold's Quick Settings, cards and list: next to Mantel's files, with Quickshell's imports
            for f in ('FireholdQuick', 'FireholdCards', 'FireholdHistory', 'FhState', 'FhTile'):
                t = (d / 'shell' / (f + '.qml')).read_text()
                self.assertIn('import Quickshell\n', t, f)
                self.assertFalse((SHELL / (f + '.qml')).exists(), '%s.qml is Plasma\'s file, copied in at build time (one copy)' % f)
            self.assertIn('Singleton {', (d / 'shell/FhState.qml').read_text())
            self.assertTrue((d / 'shell/fh.js').exists())
            self.assertIn('Home 1.0 Home.qml', (out / 'qmldir').read_text())
        self.assertFalse((SHELL / 'fhcouch').exists(), 'fhcouch is made when the image is built, never kept in git (one copy)')
        self.assertIn('install-couch.sh', (ROOT / 'os-bases/anatase/build.sh').read_text())

    def test_what_labwc_loads_has_no_kde_import(self):
        for f in UI.glob('*.qml'):
            if f.name in KDE_ONLY:
                continue
            for imp in imports(f.read_text()):
                self.assertFalse(imp.startswith('org.kde'), '%s imports %s (KDE only): through the host instead' % (f.name, imp))

    def test_both_hosts_give_the_same_names(self):
        plasma = (UI / 'PlasmaHost.qml').read_text()
        labwc = (SHELL / 'FhCouchHost.qml').read_text()
        for name in ('hosted', 'mediaFile', 'iconFile', 'now', 'wins'):
            for t, who in ((plasma, 'PlasmaHost'), (labwc, 'FhCouchHost')):
                self.assertRegex(t, r'property \w+ %s\b' % name, '%s lacks %s' % (who, name))
        for fn in ('run', 'activate', 'toggleFull', 'closeWin'):
            for t, who in ((plasma, 'PlasmaHost'), (labwc, 'FhCouchHost')):
                self.assertRegex(t, r'function %s\(' % fn, '%s lacks %s()' % (who, fn))
        for fn in ('front', 'resting'):
            self.assertRegex(labwc, r'property bool %s\b' % fn)
        for fn in ('guide', 'home'):
            self.assertRegex(labwc, r'function %s\(' % fn)
        couch = (UI / 'Couch.qml').read_text()
        for used in re.findall(r'\bhost\.(\w+)', couch):
            self.assertRegex(labwc, r'(property \w+|function) %s\b' % used, 'Couch.qml uses host.%s: FhCouchHost.qml lacks it' % used)

    def test_both_media_sources_give_the_same_names(self):
        a, b = (UI / 'PlasmaMedia.qml').read_text(), (SHELL / 'FhMedia.qml').read_text()
        for name in ('cp', 'has', 'playing'):
            self.assertRegex(a, r'property \w+ %s\b' % name)
            self.assertRegex(b, r'property \w+ %s\b' % name)
        for fn in ('playPause', 'next', 'updatePosition'):
            self.assertIn('function %s(' % fn, a)
            self.assertIn('function %s(' % fn, b)
        card = (UI / 'MediaCard.qml').read_text()
        for f in set(re.findall(r'\bcp\.(\w+)', card)):
            self.assertRegex(b, r'property \w+ %s\b' % f, 'MediaCard reads cp.%s: FhMedia.qml lacks it' % f)

    def test_nothing_runs_by_itself_in_the_labwc_host(self):
        for f in ('FhCouchHost.qml', 'FhpCouch.qml', 'FhMedia.qml', 'FhIcon.qml', 'FhModes.qml', 'FhRun.qml', 'FhPanel.qml', 'Hearth.qml', 'LayerPanel.qml', 'FhpDeskBar.qml', 'FhGlobalMenu.qml'):
            t = (SHELL / f).read_text()
            self.assertNotRegex(t, r'\bTimer\s*\{', '%s: no timers (the owner\'s rule on lag)' % f)
            self.assertNotRegex(t, r'\bloops\s*:', '%s: no animations' % f)

    def test_firehold_falls_back_to_its_own_couch(self):
        t = (SHELL / 'Firehold.qml').read_text()
        self.assertIn('!fhs.sharedCouch ? Quickshell.screens', t)
        self.assertIn('FireholdCouch {', t)
        self.assertIn('sharedCouchFailed = true', (SHELL / 'FhpCouch.qml').read_text())


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_firehold_plasma_lw', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_firehold_plasma_lw', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class LabwcHelper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.env = dict(os.environ)
        os.environ.update({'HOME': self.tmp.name, 'XDG_CONFIG_HOME': os.path.join(self.tmp.name, '.config'),
                           'XDG_RUNTIME_DIR': os.path.join(self.tmp.name, 'run'), 'HEARTH_WM': 'labwc',
                           'HEARTH_FIREHOLD_SCREENS': 'HDMI-A-1:tv:TCL TV,DP-1'})
        os.makedirs(os.environ['XDG_RUNTIME_DIR'])
        self.h = load()
        self.sent = []
        self.h.dbus_send = lambda *a, **k: self.sent.append(a)
        self.h.detach = lambda cmd: self.sent.append(tuple(cmd)) or True
        self.h.quiet = lambda cmd, *a, **k: self.sent.append(tuple(cmd)) or ''
        self.h.have = lambda b: b == 'quickshell'

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env)
        self.tmp.cleanup()

    def test_away_and_back_on_labwc(self):
        self.h.set_modes(['all'], 'away', 'pick')
        modes = {s['name']: s['mode'] for s in self.h.screen_modes()}
        self.assertEqual(modes, {'HDMI-A-1': 'away', 'DP-1': 'away'})
        self.h.come_back('pad')
        modes = {s['name']: s['mode'] for s in self.h.screen_modes()}
        self.assertEqual(modes['HDMI-A-1'], 'couch')
        # never KWin or plasmashell, never a restart of a shell; the Couch is told through Quickshell's IPC
        self.assertFalse([a for a in self.sent if any('plasmashell' in str(x) or 'kglobalaccel' in str(x) or 'KWin' in str(x) for x in a)], self.sent)
        self.assertFalse(any('systemctl' in a for a in self.sent), self.sent)
        self.assertTrue(any(a[:2] == ('quickshell', 'ipc') for a in self.sent), self.sent)
        # switching modes is never a settings change (red flag 9)
        self.assertFalse(os.path.exists(self.h.cfg_path()))

    def test_switch_on_labwc_restarts_nothing(self):
        self.h.restart_shell = lambda pkg: self.fail('labwc has no plasmashell to restart')
        self.h.switch('desk', '', 'pick')
        self.assertEqual(self.h.current_posture(), 'desk')
        self.assertFalse(os.path.exists(self.h.cfg_path()))

    def test_screens_from_wlr_randr(self):
        os.environ.pop('HEARTH_FIREHOLD_SCREENS')
        self.h.have = lambda b: b == 'wlr-randr'
        self.h.quiet = lambda cmd, *a, **k: json.dumps([
            {'name': 'HDMI-A-1', 'enabled': True, 'make': 'TCL', 'model': 'TV', 'physical_size': {'width': 1210, 'height': 680},
             'modes': [{'width': 3840, 'height': 2160, 'refresh': 60.0, 'current': True}]},
            {'name': 'DP-1', 'enabled': True, 'make': 'LG', 'model': '27GP850', 'physical_size': {'width': 600, 'height': 340},
             'modes': [{'width': 2560, 'height': 1440, 'refresh': 165.0, 'current': True}]},
            {'name': 'DP-2', 'enabled': False}])
        out = self.h.kscreen_outputs()
        self.assertEqual([o['name'] for o in out], ['HDMI-A-1', 'DP-1'])
        self.assertTrue(out[0]['tv'])
        self.assertFalse(out[1]['tv'])
        self.assertIn('2560 × 1440 · 165 Hz', out[1]['spec'])


if __name__ == '__main__':
    unittest.main()


class GlobalMenu(unittest.TestCase):
    """hearth-globalmenu: Hearth's own menu source on labwc (mockup firehold-plasma 11)."""
    def setUp(self):
        loader = importlib.machinery.SourceFileLoader('hearth_globalmenu', str(SHELL / 'hearth-globalmenu'))
        spec = importlib.util.spec_from_loader('hearth_globalmenu', loader)
        self.g = importlib.util.module_from_spec(spec)
        loader.exec_module(self.g)

    def test_an_app_id_finds_its_process(self):
        self.assertTrue(self.g.matches('org.kde.dolphin', ['dolphin']))
        self.assertTrue(self.g.matches('hearth-files', ['hearth-files']))
        self.assertFalse(self.g.matches('org.kde.dolphin', ['kate']))
        self.assertFalse(self.g.matches('', ['dolphin']))

    def test_a_menu_as_the_bar_draws_it(self):
        n = (0, {}, [(1, {'label': '_File', 'children-display': 'submenu'}, [
            (2, {'label': 'New _window', 'shortcut': [['Control', 'n']]}, []),
            (3, {'type': 'separator'}, []),
            (4, {'label': 'Hidden', 'visible': False}, []),
            (5, {'label': 'Send to', 'children-display': 'submenu'}, [(6, {'label': "Dad's PC"}, [])]),
            (7, {'label': 'Quit', 'enabled': False}, [])])])
        top = self.g.node(n[2][0])
        self.assertEqual(top['label'], 'File')
        self.assertEqual([i['label'] for i in top['items']], ['New window', '', 'Send to', 'Quit'])
        self.assertEqual(top['items'][0]['shortcut'], 'Ctrl+n')
        self.assertEqual(top['items'][1]['kind'], 'separator')
        self.assertEqual(top['items'][2]['kind'], 'sub')
        self.assertFalse(top['items'][3]['enabled'])

    def test_it_never_writes_a_setting(self):
        t = (SHELL / 'hearth-globalmenu').read_text()
        self.assertNotIn('.config', t)
        self.assertIn('XDG_RUNTIME_DIR', t)
