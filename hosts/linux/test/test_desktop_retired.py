"""1.14: 1.13's KDE Plasma and GNOME choices were retired (the owner, Nov 16: "archive the KDE and GNOME choices, since
we're migrating to KWin"; archive/desktops-kde-gnome). Someone who had one picked still gets a desktop (Mantel), and is
told once why; their session.conf is never rewritten. The session runs for real with stand-ins."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-your-desktop'


class Retired(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-retired-')
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        for d in (self.home, self.run_dir, self.bin, self.share):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        os.makedirs(os.path.dirname(self.conf))
        self.state = os.path.join(self.home, '.local', 'state', 'hearth')
        # Mantel's compositor stand-ins: each notes that it started and ends (no next mode: the session ends).
        # (KWin is the default from 1.14; labwc its fallback. Either one means "Mantel opened".)
        for wm in ('labwc', 'kwin_wayland'):
            self.write_bin(wm, '#!/bin/sh\necho %s >> "$HOME/mantel-ran"\nexit 0\n' % wm)
        # KDE's and GNOME's own programs, as if 1.13's download were still in the writable system: never started.
        for p in ('startplasma-wayland', 'plasmashell', 'gnome-session', 'gnome-shell'):
            self.write_bin(p, '#!/bin/sh\necho %s >> "$HOME/other-desktop-ran"\nsleep 5\n' % p)

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.bin], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def env(self):
        return {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
                'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent', 'HEARTH_KWIN_UP_SECS': '2',
                'HEARTH_KWIN_EARLY_SECS': '1'}

    def session(self, conf, *args):
        with open(self.conf, 'w') as f:
            f.write(conf)
        r = subprocess.run(['sh', str(SESSION)] + list(args), env=self.env(), capture_output=True, text=True, timeout=90)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        return r, log.read_text() if log.exists() else ''

    def helper(self, *args):
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), env=self.env(), capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout.strip().splitlines()[-1])

    def notice(self):
        p = os.path.join(self.state, 'desktop-notice')
        return open(p).read().split()[:2] if os.path.exists(p) else []

    def mantel_started(self):
        p = os.path.join(self.home, 'mantel-ran')
        return open(p).read().split() if os.path.exists(p) else []

    def test_kde_picked_opens_mantel_tells_once_and_keeps_the_file(self):
        conf = 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\nFOO=1\n'
        r, log = self.session(conf)
        self.assertIn('kde: KDE Plasma and GNOME were retired in 1.14; Mantel instead', log)
        self.assertTrue(self.mantel_started(), log)                                     # a desktop came up
        self.assertFalse(os.path.exists(os.path.join(self.home, 'other-desktop-ran')))  # never KDE itself
        self.assertEqual(self.notice(), ['kde', 'retired'])
        self.assertEqual(pathlib.Path(self.conf).read_text(), conf)                      # not rewritten
        self.assertFalse(os.path.exists(self.conf + '.bak'))
        # Mantel's card takes it once...
        n = self.helper('notice')
        self.assertEqual(n['kind'], 'retired')
        self.assertIn('KDE Plasma and GNOME were retired', n['title'])
        self.assertIn('KDE Plasma picked', n['sub'])
        self.assertIn('KWin', n['sub'])
        self.assertEqual(self.helper('notice'), {})
        # ...Settings › Your desktop still shows it, and the next sign-in doesn't bring the card back.
        st = self.helper('status')
        self.assertEqual(st['notice']['kind'], 'retired')
        self.assertTrue(st['noticeShown'])
        before = len(log)
        r, log = self.session(conf)
        self.assertNotIn('were retired', log[before:])
        self.assertEqual(self.notice(), [])
        self.assertEqual(pathlib.Path(self.conf).read_text(), conf)

    def test_gnome_and_old_names(self):
        for v, want in (('gnome', 'gnome'), ('plasma', 'kde'), ('cosmic', None), ('classic', None), ('', None)):
            with self.subTest(v=v):
                shutil.rmtree(self.state, ignore_errors=True)
                r, log = self.session('NIMBUS_START_MODE=desktop\n' + ('HEARTH_DESKTOP=%s\n' % v if v else ''))
                self.assertEqual(self.notice(), [want, 'retired'] if want else [])
                self.assertTrue(self.mantel_started(), log)

    def test_login_screen_entry_from_1_13(self):
        conf = 'NIMBUS_START_MODE=desktop\n'
        r, log = self.session(conf, '--desktop=gnome')
        self.assertEqual(self.notice(), ['gnome', 'retired'])
        self.assertEqual(pathlib.Path(self.conf).read_text(), conf)                      # nothing saved
        self.assertTrue(self.mantel_started(), log)

    def test_no_notice_no_card(self):
        self.assertEqual(self.helper('notice'), {})
        self.assertIsNone(self.helper('status')['notice'])

    def test_retired_pieces_are_gone(self):
        session = SESSION.read_text()
        for gone in ('run_kde', 'run_gnome', 'startplasma-wayland', 'gnome-session', 'hearth-desktop-look', 'save_desktop_choice'):
            self.assertNotIn(gone, session)
        mode = (ROOT / 'hosts/linux/session/nimbus-mode').read_text()
        self.assertNotIn('kde_up', mode)
        self.assertNotIn('gnome_up', mode)
        for f in ('hosts/linux/desktop/shell/hearth-desktops', 'os/system_files/usr/libexec/nimbus/hearth-desktops-helper',
                  'hosts/linux/session/hearth-kde.desktop', 'hosts/linux/session/hearth-gnome.desktop'):
            self.assertFalse((ROOT / f).exists(), f)
            self.assertTrue((ROOT / 'archive/desktops-kde-gnome' / f).exists(), f)
        shell = (ROOT / 'hosts/linux/desktop/shell/shell.qml').read_text()
        self.assertIn('DesktopNotice.qml', shell)
        self.assertNotIn('MantelLookExport', shell)


if __name__ == '__main__':
    unittest.main()
