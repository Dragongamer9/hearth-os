"""Tests for desktop/shell/hearth-overlay, Hearth's Quick Access as Handheld Daemon's overlay app (installed as hhd-ui
on the HHD editions): what HHD's --version gets, Game Mode starting Hearth Browser's Electron with --hhd-overlay, and
Desktop Mode turning HHD's QAM commands into Mantel's Quick Access. Fake programs stand in for Electron and quickshell."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-overlay'


class HearthOverlay(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = pathlib.Path(self.tmp.name)
        self.log = self.dir / 'log'
        self.bin = self.dir / 'bin'
        self.bin.mkdir()
        self.share = self.dir / 'share'
        (self.share / 'desktop' / 'shell').mkdir(parents=True)
        (self.share / 'version').write_text('0.1.4\n')
        self.electron = self.dir / 'nimbus'
        self.fake(self.electron, 'echo "electron $*" >> "%s"; echo "env XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-unset}" >> "%s"' % (self.log, self.log))
        self.fake(self.bin / 'quickshell', 'echo "quickshell $*" >> "%s"' % self.log)
        self.fake(self.bin / 'id', 'echo "${FAKE_UID:-1000}"')

    def tearDown(self):
        self.tmp.cleanup()

    def fake(self, path, body):
        path.write_text('#!/bin/sh\n' + body + '\n')
        path.chmod(0o755)

    def run_overlay(self, args=(), stdin='', **env):
        e = {'PATH': f'{self.bin}:/usr/bin:/bin', 'HOME': str(self.dir), 'NIMBUS_SHARE': str(self.share),
             'HEARTH_OVERLAY_ELECTRON': str(self.electron)}
        e.update(env)
        return subprocess.run(['sh', str(SCRIPT), *args], input=stdin, env=e, capture_output=True, text=True, timeout=20)

    def logged(self):
        return self.log.read_text().splitlines() if self.log.exists() else []

    def test_is_executable_posix_sh(self):
        self.assertTrue(os.access(SCRIPT, os.X_OK))
        self.assertEqual(subprocess.run(['sh', '-n', str(SCRIPT)]).returncode, 0)

    def test_version_even_as_root_starts_nothing(self):
        r = self.run_overlay(['--version'], FAKE_UID='0')
        self.assertEqual(r.returncode, 0)
        self.assertEqual(r.stdout.strip(), 'Hearth Quick Access 0.1.4')
        self.assertEqual(self.logged(), [])

    def test_game_mode_starts_electron_as_the_overlay(self):
        r = self.run_overlay(STEAM_OVERLAY='1', DISPLAY=':1')
        self.assertEqual(r.returncode, 0, r.stderr)
        lines = self.logged()
        self.assertEqual(lines[0], 'electron --hhd-overlay')
        # HHD gives no XDG_RUNTIME_DIR: it's filled in only when /run/user/<uid> exists.
        expected = '/run/user/1000' if os.path.isdir('/run/user/1000') else 'unset'
        self.assertEqual(lines[1], f'env XDG_RUNTIME_DIR={expected}')
        r = self.run_overlay(STEAM_OVERLAY='1', XDG_RUNTIME_DIR='/somewhere')
        self.assertEqual(self.logged()[-1], 'env XDG_RUNTIME_DIR=/somewhere')

    def test_refuses_root_and_a_missing_electron(self):
        r = self.run_overlay(STEAM_OVERLAY='1', FAKE_UID='0')
        self.assertEqual(r.returncode, 1)
        self.assertEqual(self.logged(), [])
        r = self.run_overlay(STEAM_OVERLAY='1', HEARTH_OVERLAY_ELECTRON=str(self.dir / 'missing'))
        self.assertEqual(r.returncode, 1)
        self.assertIn('missing', r.stderr)

    def test_desktop_mode_opens_mantels_quick_access(self):
        stdin = '\ncmd:mute\ncmd:open_qam_if_closed\n\ncmd:open_qam\naction:a\ncmd:open_expanded\ncmd:close\n'
        r = self.run_overlay(stdin=stdin, HHD_MANAGED='1', DISPLAY=':0')
        self.assertEqual(r.returncode, 0, r.stderr)
        call = f'quickshell ipc -p {self.share}/desktop/shell call quick toggle'
        self.assertEqual(self.logged(), [call, call])   # QAM and expanded; the double press's first half is left out
        self.assertEqual(r.stdout, '')                 # never grabs the controller in Desktop Mode

    def test_desktop_mode_survives_quickshell_missing(self):
        (self.bin / 'quickshell').unlink()
        r = self.run_overlay(stdin='cmd:open_qam\n', HHD_MANAGED='1', PATH=f'{self.bin}:/usr/bin:/bin')
        self.assertEqual(r.returncode, 0)
        self.assertIn("didn't answer", r.stderr)

    def test_steam_overlay_wins_over_hhd_managed(self):
        self.run_overlay(STEAM_OVERLAY='1', HHD_MANAGED='1')
        self.assertEqual(self.logged()[0], 'electron --hhd-overlay')


class Editions(unittest.TestCase):
    """Every HHD edition installs it as HHD's overlay; the Bazzite build (os/) doesn't, so Hearth keeps its own overlay
    window there (Hearth only uses HHD's when /usr/bin/hhd-ui is hearth-overlay)."""
    ROOT = pathlib.Path(__file__).resolve().parents[3]

    def test_hhd_editions_install_it(self):
        for base in ('anatase', 'ubuntu', 'cachyos', 'peppermint'):
            text = (self.ROOT / 'os-bases' / base / 'build.sh').read_text()
            self.assertIn('ln -sfn /usr/share/nimbus/desktop/shell/hearth-overlay /usr/bin/hhd-ui', text, base)
            self.assertIn('Environment=HHD_OVERLAY=/usr/share/nimbus/desktop/shell/hearth-overlay', text, base)
            wf = (self.ROOT / '.github' / 'workflows' / f'base-{base}.yml').read_text()
            self.assertIn('readlink -f /usr/bin/hhd-ui', wf, base)

    def test_bazzite_keeps_its_overlay(self):
        for p in (self.ROOT / 'os').rglob('*'):
            if p.is_file() and p.suffix in ('', '.sh', '.conf', '.yml') and p.stat().st_size < 2_000_000:
                try:
                    text = p.read_text()
                except (UnicodeDecodeError, PermissionError):
                    continue
                self.assertNotIn('hhd-ui', text, str(p))
                self.assertNotIn('HHD_OVERLAY', text, str(p))
        self.assertNotIn('hhd-ui', (self.ROOT / '.github' / 'workflows' / 'os.yml').read_text())


if __name__ == '__main__':
    unittest.main()
