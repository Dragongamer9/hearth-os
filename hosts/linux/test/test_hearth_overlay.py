"""Tests for desktop/shell/hearth-overlay, Hearth's Quick Access as Handheld Daemon's overlay app (installed as hhd-ui
on the HHD editions): what HHD's --version gets, Game Mode starting Hearth Browser's Electron with --hhd-overlay, and
Desktop Mode turning HHD's QAM commands into Mantel's Quick Access, and the Desktop Mode one ending itself once Desktop
Mode is over (HHD keeps sending everything to it until it ends, hhd/plugins/overlay/base.py OverlayService). Fake
programs stand in for Electron, quickshell and pgrep (gamescope running or not)."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import signal
import subprocess
import tempfile
import time
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
        # gamescope runs while this file is there (pgrep -u <uid> -x gamescope / gamescope-wl).
        self.gamescope = self.dir / 'gamescope-running'
        self.fake(self.bin / 'pgrep', 'case "$*" in *gamescope*) [ -e "%s" ] && exit 0 ;; esac; exit 1' % self.gamescope)
        self.run = self.dir / 'run'
        self.run.mkdir()
        self.x11 = self.dir / 'x11'
        self.x11.mkdir()
        (self.x11 / 'X0').write_text('')

    def tearDown(self):
        self.tmp.cleanup()

    def fake(self, path, body):
        path.write_text('#!/bin/sh\n' + body + '\n')
        path.chmod(0o755)

    def run_overlay(self, args=(), stdin='', **env):
        return subprocess.run(['sh', str(SCRIPT), *args], input=stdin, env=self.env(env), capture_output=True, text=True,
                              timeout=20)

    def env(self, env):
        e = {'PATH': f'{self.bin}:/usr/bin:/bin', 'HOME': str(self.dir), 'NIMBUS_SHARE': str(self.share),
             'HEARTH_OVERLAY_ELECTRON': str(self.electron), 'HEARTH_X11_DIR': str(self.x11), 'HEARTH_OVERLAY_POLL': '0.1'}
        e.update(env)
        return e

    def desktop(self, **env):
        """Started by HHD for Desktop Mode (launch_overlay_de), left running: stdin stays open like HHD's pipe."""
        e = {'HHD_MANAGED': '1', 'DISPLAY': ':0', 'XAUTHORITY': '', 'XDG_RUNTIME_DIR': str(self.run)}
        e.update(env)
        p = subprocess.Popen(['sh', str(SCRIPT)], stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                             stderr=open(self.dir / 'stderr', 'w'), env=self.env(e), text=True, start_new_session=True)
        self.addCleanup(self.stop, p)
        return p

    def stop(self, p):
        try:
            os.killpg(p.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        p.wait()

    def stderr(self):
        return (self.dir / 'stderr').read_text()

    def mode(self, m):
        (self.run / 'nimbus-mode-now').write_text(m + '\n')

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
        r = self.run_overlay(STEAM_OVERLAY='1', DISPLAY=':1', XDG_RUNTIME_DIR='/somewhere')
        self.assertEqual(self.logged()[-1], 'env XDG_RUNTIME_DIR=/somewhere')

    def test_game_mode_gives_chromium_fonts_it_can_read(self):
        """0.6.5 on the Nvidia PC: Quick Access had no text. Chromium's own (older) fontconfig can't read fontconfig 2.17's
        new conf.d files and found no font; the overlay gets a fonts.conf without them, the old generic rules and a
        cache of its own named after the image's fonts."""
        etc = self.dir / 'etc-fonts'
        (etc / 'conf.d').mkdir(parents=True)
        (etc / 'fonts.conf').write_text('<fontconfig>\n  <dir>/usr/share/fonts</dir>\n  <dir prefix="xdg">fonts</dir>\n</fontconfig>\n')
        (etc / 'conf.d' / '10-hinting.conf').write_text('<fontconfig><match/></fontconfig>\n')
        (etc / 'conf.d' / '48-guessfamily.conf').write_text('<fontconfig><test xsi:nil="true"/><const>sans-serif</const></fontconfig>\n')
        (etc / 'conf.d' / '49-sansserif.conf').write_text('<fontconfig><const>monospace</const></fontconfig>\n')
        fonts = self.dir / 'fonts'
        fonts.mkdir()
        (fonts / 'a.ttf').write_text('x')
        el = self.dir / 'nimbus-env'
        self.fake(el, 'echo "FONTCONFIG_FILE=${FONTCONFIG_FILE:-unset}" >> "%s"' % self.log)
        env = dict(STEAM_OVERLAY='1', DISPLAY=':1', XDG_RUNTIME_DIR=str(self.run), HEARTH_FONTS_ETC=str(etc),
                   HEARTH_FONT_DIRS=str(fonts), HEARTH_OVERLAY_ELECTRON=str(el), HEARTH_OVERLAY_NO_SESSION_ENV='1')
        r = self.run_overlay(**env)
        self.assertEqual(r.returncode, 0, r.stderr)
        conf = self.run / 'hearth-overlay-fonts' / 'fonts.conf'
        self.assertEqual(self.logged()[-1], f'FONTCONFIG_FILE={conf}')
        text = conf.read_text()
        self.assertIn(f'<include ignore_missing="yes">{etc}/conf.d/10-hinting.conf</include>', text)
        self.assertNotIn('48-guessfamily.conf</include>', text)
        self.assertNotIn('49-sansserif.conf</include>', text)
        self.assertIn('<dir>/usr/share/fonts</dir>', text)
        self.assertIn('<edit name="family" mode="append_last"><string>sans-serif</string></edit>', text)
        caches = list((self.dir / '.cache' / 'hearth-overlay').glob('fontconfig-*'))
        self.assertEqual(len(caches), 1)
        self.assertIn(f'<cachedir>{caches[0]}</cachedir>', text)
        self.assertIn('left out (fontconfig 2.17 only): 48-guessfamily.conf 49-sansserif.conf', r.stderr)
        log = (self.run / 'hearth-overlay.log').read_text()
        self.assertIn('Quick Access started by Handheld Daemon in Game Mode', log)
        self.assertIn("fonts: Hearth's setup", log)
        # The image's fonts change (an update): a new cache, the old one gone.
        (fonts / 'b.ttf').write_text('y')
        self.run_overlay(**env)
        caches2 = list((self.dir / '.cache' / 'hearth-overlay').glob('fontconfig-*'))
        self.assertEqual(len(caches2), 1)
        self.assertNotEqual(caches2[0], caches[0])
        # Turned off, or a FONTCONFIG_FILE of the user's own: left alone.
        self.run_overlay(**env, HEARTH_OVERLAY_SYSTEM_FONTS='1')
        self.assertEqual(self.logged()[-1], 'FONTCONFIG_FILE=unset')
        self.run_overlay(**env, FONTCONFIG_FILE='/mine.conf')
        self.assertEqual(self.logged()[-1], 'FONTCONFIG_FILE=/mine.conf')

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
        self.run_overlay(STEAM_OVERLAY='1', HHD_MANAGED='1', DISPLAY=':1')
        self.assertEqual(self.logged()[0], 'electron --hhd-overlay')

    def test_game_mode_without_a_display_ends_with_a_reason(self):
        r = self.run_overlay(STEAM_OVERLAY='1')
        self.assertEqual(r.returncode, 1)
        self.assertIn('no DISPLAY', r.stderr)
        self.assertEqual(self.logged(), [])

    def test_mantels_error_is_in_the_log(self):
        self.fake(self.bin / 'quickshell', 'echo "No running instances for $3" >&2; exit 1')
        r = self.run_overlay(stdin='cmd:open_qam\n', HHD_MANAGED='1', DISPLAY=':0', XDG_RUNTIME_DIR=str(self.run))
        self.assertEqual(r.returncode, 0)
        self.assertIn("Mantel's Quick Access didn't answer (No running instances for", r.stderr)

    # ---- the Desktop Mode one never stands in for Game Mode (the ROG Ally report, 2026-10-24: after a trip to
    # Desktop Mode, every QAM press in Game Mode went to it and "Mantel's Quick Access didn't answer" for hours)

    def test_desktop_mode_ends_at_once_when_game_mode_runs(self):
        self.gamescope.write_text('')
        for mode in (None, 'game'):
            if mode:
                self.mode(mode)
            r = self.run_overlay(stdin='cmd:open_qam\n', HHD_MANAGED='1', DISPLAY=':0', XDG_RUNTIME_DIR=str(self.run))
            self.assertEqual(r.returncode, 1, mode)
            self.assertIn("Game Mode (gamescope) is running; ending so HHD starts me in gamescope", r.stderr)
            self.assertNotIn("Mantel", r.stderr)
            self.assertEqual(self.logged(), [])     # Mantel isn't asked

    def test_a_game_in_gamescope_inside_desktop_mode_isnt_game_mode(self):
        self.gamescope.write_text('')
        self.mode('desktop')
        r = self.run_overlay(stdin='cmd:open_qam\n', HHD_MANAGED='1', DISPLAY=':0', XDG_RUNTIME_DIR=str(self.run))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(len(self.logged()), 1)

    def wait_end(self, p, timeout=10):
        try:
            return p.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.fail('the Desktop Mode overlay app kept running: ' + self.stderr())

    def test_ends_itself_when_game_mode_comes_back(self):
        self.mode('desktop')
        p = self.desktop()
        time.sleep(0.5)
        self.assertIsNone(p.poll())                # Desktop Mode: it waits for HHD's presses
        self.mode('game')
        self.gamescope.write_text('')
        self.assertNotEqual(self.wait_end(p), 0)   # no press needed: HHD sees it end and starts it in gamescope
        self.assertIn('Game Mode (gamescope) is running again; ending so Handheld Daemon starts Quick Access in gamescope',
                      self.stderr())

    def test_ends_itself_when_desktop_modes_display_goes(self):
        self.mode('desktop')
        p = self.desktop()
        time.sleep(0.5)
        self.assertIsNone(p.poll())
        (self.x11 / 'X0').unlink()                 # labwc (and its Xwayland) ended: signed out, or between modes
        self.assertNotEqual(self.wait_end(p), 0)
        self.assertIn("Desktop Mode's display (:0) is gone", self.stderr())

    def test_a_press_after_desktop_mode_ends_isnt_sent_to_mantel(self):
        self.mode('desktop')
        p = self.desktop(HEARTH_OVERLAY_POLL='600')    # only the press notices
        p.stdin.write('cmd:open_qam\n'); p.stdin.flush()
        for _ in range(100):
            if self.logged():
                break
            time.sleep(0.05)
        self.assertEqual(len(self.logged()), 1)
        self.mode('game')
        self.gamescope.write_text('')
        p.stdin.write('cmd:open_qam\n'); p.stdin.flush()
        self.assertEqual(self.wait_end(p), 1)
        self.assertEqual(len(self.logged()), 1)
        self.assertIn('ending so Handheld Daemon starts Quick Access in gamescope', self.stderr())

    def test_ends_when_hhd_closes_it(self):
        self.mode('desktop')
        p = self.desktop()
        p.stdin.close()                            # HHD stopped it (closed its pipe)
        self.assertEqual(self.wait_end(p), 0)


class Session(unittest.TestCase):
    """nimbus-session says which mode it's in, where the overlay app reads it."""
    ROOT = pathlib.Path(__file__).resolve().parents[1]

    def test_session_writes_the_mode(self):
        text = (self.ROOT / 'session' / 'nimbus-session').read_text()
        self.assertIn('echo "$mode" >"$RUN/nimbus-mode-now"', text)
        self.assertIn('rm -f "$RUN/nimbus-mode-now"; exit 0', text)
        self.assertIn('nimbus-mode-now', SCRIPT.read_text())


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
