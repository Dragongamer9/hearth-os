"""1.13: Desktop Mode opens Mantel, KDE Plasma or GNOME (docs/DESKTOPS.md). KDE and GNOME only when picked and their
add-on is here; Mantel whenever one doesn't come up, and after two failed starts in a row until it's picked again. The
login screen's pick is saved without touching the rest of session.conf. The session runs for real with stand-ins."""
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


class Desktops(unittest.TestCase):
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
               'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent', 'HEARTH_DESKTOP_TIMEOUT': '6'}
        r = subprocess.run(['sh', str(SESSION)] + list(args), env=env, capture_output=True, text=True, timeout=60)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        return r, log.read_text() if log.exists() else ''

    def labwc_starts(self):
        p = os.path.join(self.home, 'labwc-ran')
        return open(p).read().splitlines() if os.path.exists(p) else []

    def kde(self, works=True):
        """KDE's add-on "here": startplasma-wayland brings up KWin and the shell (or crashes straight away)."""
        self.write_bin('kwin_wayland', '#!/bin/sh\nsleep 30\n')
        self.write_bin('plasmashell', '#!/bin/sh\nsleep 30\n')
        if works:
            self.write_bin('startplasma-wayland', '#!/bin/sh\necho kde >> "$HOME/kde-ran"\nkwin_wayland & plasmashell &\nsleep 3\nkill %1 %2 2>/dev/null\nexit 0\n')
        else:
            self.write_bin('startplasma-wayland', '#!/bin/sh\necho kde >> "$HOME/kde-ran"\nexit 1\n')

    def notice(self):
        p = os.path.join(self.home, '.local', 'state', 'hearth', 'desktop-notice')
        return open(p).read().split()[:2] if os.path.exists(p) else []

    def test_default_is_mantel(self):
        r, log = self.session('NIMBUS_START_MODE=desktop\n')
        self.assertIn('desktop mode: Hearth Classic', log)
        self.assertEqual(len(self.labwc_starts()), 1, log)

    def test_kde_picked_without_its_add_on_opens_mantel_and_says_why(self):
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n')
        self.assertIn("kde: its add-on isn't here", log)
        self.assertIn('desktop mode: Hearth Classic', log)
        self.assertEqual(self.notice(), ['kde', 'missing'])
        self.assertIn('HEARTH_DESKTOP=kde', pathlib.Path(self.conf).read_text())    # the pick stays saved

    def test_kde_starts_when_its_add_on_is_here(self):
        self.kde(works=True)
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n')
        self.assertIn('desktop mode: KDE Plasma', log)
        self.assertIn('kde: up', log)
        self.assertEqual(self.labwc_starts(), [], log)                           # Mantel didn't start

    def test_kde_that_crashes_falls_back_and_twice_stops_trying(self):
        self.kde(works=False)
        for n in (1, 2):
            r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n')
            self.assertIn("kde: didn't start; Mantel for this sign-in", log)
            os.remove(os.path.join(self.run_dir, 'hearth-kde-failed'))          # a new sign-in
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n')
        self.assertIn("kde: didn't start twice in a row", log)
        self.assertEqual(open(os.path.join(self.home, 'kde-ran')).read().split(), ['kde', 'kde'])   # not a third time
        self.assertEqual(self.notice(), ['kde', 'twice'])
        self.assertTrue(len(self.labwc_starts()) >= 3, log)

    def test_login_screen_pick_is_saved_and_the_rest_kept(self):
        r, log = self.session('NIMBUS_START_MODE=desktop\nFOO=1\n', '--desktop=gnome')
        self.assertEqual(pathlib.Path(self.conf).read_text(), 'NIMBUS_START_MODE=desktop\nFOO=1\nHEARTH_DESKTOP=gnome\n')
        self.assertTrue(os.path.exists(self.conf + '.bak'))
        self.assertIn('desktop: gnome picked on the login screen (saved)', log)

    def test_login_screen_pick_just_that_time(self):
        self.kde(works=True)
        conf = 'NIMBUS_START_MODE=desktop\nHEARTH_LOGIN_PICK=once\n'
        r, log = self.session(conf, '--desktop=kde')
        self.assertEqual(pathlib.Path(self.conf).read_text(), conf)              # not saved
        self.assertIn('desktop: kde picked on the login screen (this sign-in only)', log)
        self.assertIn('desktop mode: KDE Plasma', log)

    def test_kde_starts_its_controller_once_up(self):
        self.kde(works=True)
        shell = os.path.join(self.share, 'desktop', 'shell')
        os.makedirs(shell, exist_ok=True)
        with open(os.path.join(shell, 'hearth-companion'), 'w') as f:
            f.write('#!/bin/sh\necho "$1 $2 HEARTH_DESKTOP=$HEARTH_DESKTOP" >> "$HOME/companion-ran"\n')
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n')
        self.assertIn('kde: up', log)
        import time
        for _ in range(20):
            if os.path.exists(os.path.join(self.home, 'companion-ran')):
                break
            time.sleep(0.2)
        self.assertEqual(open(os.path.join(self.home, 'companion-ran')).read().split(), ['kde', 'session', 'HEARTH_DESKTOP=kde'])

    def test_old_names(self):
        for v, want in (('plasma', 'kde'), ('cosmic', 'classic'), ('deepin', 'classic')):
            with self.subTest(v=v):
                r, log = self.session(f'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP={v}\n')
                self.assertIn('desktop mode: Hearth Classic', log)
                if want == 'kde':
                    self.assertIn("kde: its add-on isn't here", log)

    def test_mode_switch_knows_kde_and_gnome(self):
        m = MODE.read_text()
        for f in ('hearth-kde.pid', 'hearth-gnome.pid', 'org.kde.Shutdown.logout', 'gnome-session-quit --logout --no-prompt'):
            self.assertIn(f, m)


if __name__ == '__main__':
    unittest.main()
