"""Mantel is Hearth's only desktop (Hearth Plasma, GNOME and COSMIC are gone): nimbus-session always opens Mantel
(labwc) for Desktop Mode, whatever an older Hearth left in session.conf (HEARTH_DESKTOP=plasma|gnome|cosmic|deepin) or
the login screen asks for (--desktop=...), never fails or loops because of it, and never starts another desktop even
when one happens to be installed. Settings has no desktop choice. The session runs for real with stand-ins."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
MODE = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-mode'


class OldDesktopChoice(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-mantel-only-')
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        for d in (self.home, self.run_dir, self.bin, self.share):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        os.makedirs(os.path.dirname(self.conf))
        # labwc stand-in: counts its starts, notes HEARTH_DESKTOP, ends (no next mode: the session ends).
        self.write_bin('labwc', '#!/bin/sh\necho "start HEARTH_DESKTOP=${HEARTH_DESKTOP-unset}" >> "$HOME/labwc-ran"\nexit 0\n')
        # Other desktops "installed": none of them may ever start.
        for b in ('startplasma-wayland', 'kwin_wayland', 'plasmashell', 'gnome-session', 'gnome-shell',
                  'start-cosmic', 'cosmic-comp', 'cosmic-panel', 'startdde'):
            self.write_bin(b, '#!/bin/sh\necho %s >> "$HOME/other-desktop-ran"\nexit 0\n' % b)

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.bin], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def session(self, conf, *args):
        with open(self.conf, 'w') as f:
            f.write(conf)
        env = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
               'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent'}
        r = subprocess.run(['sh', str(SESSION)] + list(args), env=env, capture_output=True, text=True, timeout=60)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        return r, log.read_text() if log.exists() else ''

    def labwc_starts(self):
        p = os.path.join(self.home, 'labwc-ran')
        return open(p).read().splitlines() if os.path.exists(p) else []

    def assert_mantel(self, r, log):
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertEqual(len(self.labwc_starts()), 1, log)                    # once: no loop
        self.assertFalse(os.path.exists(os.path.join(self.home, 'other-desktop-ran')), log)
        self.assertIn('desktop mode: Hearth Classic', log)
        self.assertIn('desktop mode ended (code 0), leaving the session', log)

    def test_default_is_mantel(self):
        r, log = self.session('NIMBUS_START_MODE=desktop\n')
        self.assert_mantel(r, log)
        self.assertNotIn('only desktop now', log)

    def test_old_values_mean_mantel(self):
        for v in ('plasma', 'gnome', 'cosmic', 'deepin', '"plasma"', "'gnome'", 'kde', ''):
            for line in ('HEARTH_DESKTOP=%s' % v, 'export HEARTH_DESKTOP=%s' % v):
                with self.subTest(line=line):
                    for f in ('labwc-ran', 'other-desktop-ran'):
                        pathlib.Path(self.home, f).unlink(missing_ok=True)
                    r, log = self.session('NIMBUS_START_MODE=desktop\n%s\n' % line)
                    self.assert_mantel(r, log)
                    # Nothing this session starts sees the old value.
                    self.assertEqual(self.labwc_starts(), ['start HEARTH_DESKTOP=unset'])
                    if v.strip('"\'') in ('plasma', 'gnome', 'cosmic', 'deepin', 'kde'):
                        self.assertIn('Mantel is the only desktop now', log)

    def test_old_value_is_left_in_the_file(self):
        self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\nFOO=1\n')
        self.assertEqual(pathlib.Path(self.conf).read_text(), 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\nFOO=1\n')

    def test_old_login_screen_entries_mean_mantel(self):
        for d in ('plasma', 'gnome', 'cosmic', 'deepin', 'classic'):
            with self.subTest(desktop=d):
                pathlib.Path(self.home, 'labwc-ran').unlink(missing_ok=True)
                r, log = self.session('NIMBUS_START_MODE=desktop\n', '--desktop=' + d)
                self.assert_mantel(r, log)
                self.assertNotIn('HEARTH_DESKTOP', pathlib.Path(self.conf).read_text())   # nothing saved

    def test_no_labwc_never_loops(self):
        os.remove(os.path.join(self.bin, 'labwc'))
        if shutil.which('labwc', path='/usr/bin:/bin'):
            self.skipTest('labwc is installed on this machine')
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n')
        self.assertIn('desktop mode unavailable (labwc missing)', log)
        self.assertFalse(os.path.exists(os.path.join(self.home, 'other-desktop-ran')), log)


class Source(unittest.TestCase):
    def test_no_other_desktop_paths(self):
        s = SESSION.read_text()
        for f in ('run_plasma', 'run_gnome', 'run_cosmic', 'startplasma-wayland', 'gnome-session', 'start-cosmic'):
            self.assertNotIn(f, s)
        self.assertIn('desktop_choice() { echo classic; }', s)
        m = MODE.read_text()
        for f in ('plasma_up', 'gnome_up', 'cosmic_up'):
            self.assertNotIn(f, m)

    def test_settings_has_no_desktop_choice(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        for f in ('deskPick', "call('login.setDesktop'", 'd.plasma', 'Hearth GNOME', 'Hearth COSMIC', 'Hearth Deepin'):
            self.assertNotIn(f, ui)
        qml = (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'SetSignin.qml').read_text()
        for f in ('hearth-plasma', 'Hearth Plasma', 'Hearth GNOME', 'Hearth COSMIC', 'desktop-set'):
            self.assertNotIn(f, qml)

    def test_image_has_no_other_desktop_sessions(self):
        b = '\n'.join(l for l in (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text().splitlines()
                      if not l.lstrip().startswith('#'))
        for pkg in ('plasma-workspace', 'kwin-wayland', 'plasma-desktop', 'gnome-shell mutter', 'gnome-session',
                    'xdg-desktop-portal-kde', 'xdg-desktop-portal-gnome', 'hearth-plasma hearth-gnome'):
            self.assertNotIn(pkg, b)
        # What Mantel and Hearth's KDE apps use stays.
        for pkg in ('polkit-kde-agent-1', 'breeze', 'gnome-keyring', 'sddm', 'hearth-kde/install.sh'):
            self.assertIn(pkg, b)


if __name__ == '__main__':
    unittest.main()
