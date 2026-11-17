"""KDE Plasma and GNOME look like Mantel and follow it (1.14, hearth-desktop-look): only Hearth's keys, the bar laid
out like Mantel's, and nothing touched with the switch off."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import stat
import subprocess
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'


def load(env):
    old = dict(os.environ)
    os.environ.update(env)
    try:
        loader = importlib.machinery.SourceFileLoader('hdl', str(SHELL / 'hearth-desktop-look'))
        spec = importlib.util.spec_from_loader('hdl', loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
        return m
    finally:
        os.environ.clear()
        os.environ.update(old)


class Look(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.env = {'HOME': self.d, 'XDG_CONFIG_HOME': self.d + '/cfg', 'XDG_DATA_HOME': self.d + '/data', 'XDG_STATE_HOME': self.d + '/state'}
        os.makedirs(self.d + '/state/hearth')
        self.look = {'look': 'steam', 'mode': 'dark', 'accent': '#1a9fff', 'colors': {'panel': '#1e2633', 'accent': '#1a9fff'},
                     'font': 'Inter', 'oneBar': False, 'barEdge': 'top', 'dockFloats': True, 'dockAutoHide': False,
                     'wallpaper': self.d + '/state/hearth/mantel-wallpaper.png'}
        json.dump(self.look, open(self.d + '/state/hearth/mantel-look.json', 'w'))
        open(self.d + '/state/hearth/mantel-wallpaper.png', 'wb').write(b'\x89PNG')
        self.bin = pathlib.Path(self.d, 'bin'); self.bin.mkdir()
        g = self.bin / 'gsettings'
        g.write_text('#!/bin/sh\necho "$@" >> "%s/gs.log"\n[ "$1" = get ] && echo "[\'other@x\']"\nexit 0\n' % self.d)
        g.chmod(g.stat().st_mode | stat.S_IEXEC)
        self.env['PATH'] = '%s:/usr/bin:/bin' % self.bin
        self.m = load(self.env)

    def calls(self):
        p = pathlib.Path(self.d, 'gs.log')
        return p.read_text() if p.exists() else ''

    def test_plasma_layout_is_mantels_bar_and_dock(self):
        s = self.m.plasma_script(self.m.mantel())
        self.assertIn('p.location = "top"', s)
        self.assertIn('d.location = "bottom"', s)
        self.assertIn('d.lengthMode = "fit"', s)
        self.assertIn('org.hearth.kickoff', s)
        one = dict(self.m.mantel(), oneBar=True, barEdge='left')
        s = self.m.plasma_script(one)
        self.assertIn('p.location = "left"', s)
        self.assertNotIn('var d = new Panel', s)

    def test_nearest_gnome_accent(self):
        self.assertEqual(self.m.nearest_accent('#1a9fff'), 'blue')
        self.assertEqual(self.m.nearest_accent('#ff8a3d'), 'orange')

    def test_gnome_gets_mantels_look_and_dock_other_extensions_kept(self):
        old = dict(os.environ); os.environ.update(self.env)
        try:
            self.m.THEME = '/bin/true'
            r = self.m.gnome_pre()
        finally:
            os.environ.clear(); os.environ.update(old)
        self.assertTrue(r['ok'])
        c = self.calls()
        self.assertIn("org.gnome.desktop.interface color-scheme 'prefer-dark'", c)
        self.assertIn("org.gnome.desktop.interface accent-color 'blue'", c)
        self.assertIn('picture-uri', c)
        self.assertIn("enabled-extensions ['other@x', 'dash-to-dock@micxgx.gmail.com']", c)
        self.assertIn("dash-to-dock background-color '#1e2633'", c)

    def test_off_touches_nothing(self):
        os.makedirs(self.d + '/cfg/nimbus')
        open(self.d + '/cfg/nimbus/session.conf', 'w').write('HEARTH_DESKTOP_LOOK=0\n')
        old = dict(os.environ); os.environ.update(self.env)
        try:
            self.assertEqual(self.m.gnome_pre().get('skipped'), 'off')
            self.assertEqual(self.m.kde_pre().get('skipped'), 'off')
        finally:
            os.environ.clear(); os.environ.update(old)
        self.assertEqual(self.calls(), '')

    def test_switch_is_one_line(self):
        os.makedirs(self.d + '/cfg/nimbus')
        open(self.d + '/cfg/nimbus/session.conf', 'w').write('# mine\nHEARTH_DESKTOP=kde\n')
        subprocess.run(['python3', str(SHELL / 'hearth-desktop-look'), 'off'], env={**os.environ, **self.env}, check=True, capture_output=True)
        self.assertEqual(open(self.d + '/cfg/nimbus/session.conf').read(), '# mine\nHEARTH_DESKTOP=kde\nHEARTH_DESKTOP_LOOK=0\n')


if __name__ == '__main__':
    unittest.main()
