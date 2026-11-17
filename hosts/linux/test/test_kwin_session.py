"""1.14: Mantel on KWin (docs/DESKTOPS.md "Mantel on KWin"). KWin by default; labwc whenever KWin isn't there, ends
right away or Mantel doesn't say it's up, with a notice and a count (twice in a row → labwc until it's turned back on);
HEARTH_DESKTOP_WM=labwc, safe graphics and a second player's seat stay on labwc. nimbus-mode ends KWin through
HEARTH_DESKTOP_PID and never runs labwc's own commands under KWin. The session runs for real with stand-ins."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import shutil
import signal
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
MODE = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-mode'
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-your-desktop'
AUTOSTART = ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc' / 'autostart'

# KWin stand-ins. "works": notes how it was started, says Mantel is up (as shell.qml does) and runs a while.
KWIN_WORKS = '''#!/bin/sh
{ echo "args=$*"; echo "pid=$$ desktop_pid=$HEARTH_DESKTOP_PID wm=$HEARTH_WM labwc_pid=${LABWC_PID-unset}";
  echo "xcd=$XDG_CURRENT_DESKTOP"; echo "xdg=$XDG_CONFIG_DIRS"; echo "qpa=$QT_QPA_PLATFORM"; } >> "$HOME/kwin-ran"
: > "$XDG_RUNTIME_DIR/hearth-shell-up"
sleep 3
exit 0
'''
KWIN_CRASHES = '#!/bin/sh\necho "args=$*" >> "$HOME/kwin-ran"\nexit 1\n'
KWIN_NO_SHELL = '#!/bin/sh\necho "args=$*" >> "$HOME/kwin-ran"\nexec sleep 60\n'


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-kwin-')
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
        # The share as the image has it: labwc's folder (its environment) and KWin's.
        shutil.copytree(ROOT / 'hosts/linux/desktop/labwc', os.path.join(self.share, 'desktop', 'labwc'))
        shutil.copytree(ROOT / 'hosts/linux/desktop/kwin', os.path.join(self.share, 'desktop', 'kwin'))
        self.write_bin('labwc', '#!/bin/sh\necho "labwc $* desktop_pid=$HEARTH_DESKTOP_PID pid=$$ wm=$HEARTH_WM" >> "$HOME/labwc-ran"\nexit 0\n')

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.bin], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def env(self, **kw):
        e = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
             'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent',
             'HEARTH_KWIN_UP_SECS': '3', 'HEARTH_KWIN_EARLY_SECS': '2', 'HEARTH_SAFE_FLAG': os.path.join(self.tmp, 'safe')}
        e.update(kw)
        return e

    def session(self, conf='NIMBUS_START_MODE=desktop\n', **kw):
        with open(self.conf, 'w') as f:
            f.write(conf)
        logp = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        before = len(logp.read_text()) if logp.exists() else 0
        r = subprocess.run(['sh', str(SESSION)], env=self.env(**kw), capture_output=True, text=True, timeout=90)
        return r, logp.read_text()[before:] if logp.exists() else ''

    def lines(self, name):
        p = os.path.join(self.home, name)
        return open(p).read().splitlines() if os.path.exists(p) else []

    def notice(self):
        p = os.path.join(self.state, 'desktop-notice')
        return open(p).read().split()[:2] if os.path.exists(p) else []

    def fails(self):
        p = os.path.join(self.state, 'desktop-fails')
        return open(p).read() if os.path.exists(p) else ''

    def helper(self, *args):
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), env=self.env(), capture_output=True, text=True, timeout=30)
        return json.loads(r.stdout.strip().splitlines()[-1])


class KWinStart(Base):
    def test_kwin_is_the_default(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        r, log = self.session()
        self.assertIn('desktop mode: Mantel on KWin', log)
        self.assertIn('kwin: Mantel up after', log)
        self.assertEqual(self.lines('labwc-ran'), [], log)                    # no fallback
        ran = '\n'.join(self.lines('kwin-ran'))
        self.assertIn('args=--xwayland --no-lockscreen %s/desktop/kwin/autostart' % self.share, ran)
        pid = [l for l in self.lines('kwin-ran') if l.startswith('pid=')][0].split()
        self.assertEqual(pid[0][4:], pid[1][len('desktop_pid='):], 'HEARTH_DESKTOP_PID is KWin itself')
        self.assertIn('wm=kwin', ran)
        self.assertIn('labwc_pid=unset', ran)                               # never LABWC_PID under KWin
        self.assertIn('xcd=Nimbus:KDE', ran)
        self.assertIn('xdg=%s/desktop/kwin/xdg:' % self.share, ran)         # Hearth's defaults under yours
        self.assertIn('qpa=wayland;xcb', ran)                               # labwc's environment file
        self.assertTrue(os.path.exists(os.path.join(self.home, '.local/share/nimbus/kwin.log')))
        self.assertEqual(self.notice(), [])
        self.assertEqual(self.fails(), '')

    def test_kwin_missing_opens_labwc_and_says_why(self):
        r, log = self.session()
        self.assertIn("KWin (kwin_wayland) isn't installed; labwc instead", log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)
        self.assertIn('wm=labwc', self.lines('labwc-ran')[0])
        l = self.lines('labwc-ran')[0].split()
        self.assertEqual(l[-2][4:], l[-3][len('desktop_pid='):], 'HEARTH_DESKTOP_PID is labwc itself')
        self.assertEqual(self.notice(), ['kwin', 'missing'])
        n = self.helper('notice')
        self.assertEqual(n['title'], 'Mantel opened on labwc')

    def test_kwin_dying_fast_opens_labwc_counts_and_twice_stays(self):
        self.write_bin('kwin_wayland', KWIN_CRASHES)
        conf = 'NIMBUS_START_MODE=desktop\n'
        r, log = self.session(conf)
        self.assertIn("kwin ended before Mantel came up (code 1)", log)
        self.assertIn("desktop: KWin didn't start; labwc for this sign-in", log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)
        self.assertEqual(self.notice(), ['kwin', 'failed'])
        self.assertEqual(self.fails(), 'kwin 1\n')
        self.assertTrue(os.path.exists(os.path.join(self.run_dir, 'hearth-kwin-failed')))
        # The same sign-in: labwc straight away, KWin isn't tried again.
        r, log = self.session(conf)
        self.assertIn("KWin didn't start earlier this sign-in", log)
        self.assertEqual(len(self.lines('kwin-ran')), 1)
        # A new sign-in: tried again, fails again → twice.
        os.remove(os.path.join(self.run_dir, 'hearth-kwin-failed'))
        r, log = self.session(conf)
        self.assertIn("KWin didn't start twice in a row", log)
        self.assertEqual(self.notice(), ['kwin', 'twice'])
        self.assertEqual(self.helper('status')['kwinPaused'], True)
        # Then it stays on labwc, sign-in after sign-in, without trying KWin...
        os.remove(os.path.join(self.run_dir, 'hearth-kwin-failed'))
        r, log = self.session(conf)
        self.assertEqual(len(self.lines('kwin-ran')), 2, log)
        self.assertEqual(len(self.lines('labwc-ran')), 4, log)
        self.assertEqual(pathlib.Path(self.conf).read_text(), conf)          # the choice was never rewritten
        # ...until it's turned back on (Settings, or the notice's "Use KWin again").
        st = self.helper('wm', 'kwin')
        self.assertEqual((st['kwinFails'], st['kwinPaused']), (0, False))
        self.assertEqual(self.notice(), [])
        self.write_bin('kwin_wayland', KWIN_WORKS)
        r, log = self.session(pathlib.Path(self.conf).read_text())
        self.assertIn('kwin: Mantel up after', log)

    def test_kde_retired_and_kwin_failing_are_both_told(self):
        self.write_bin('kwin_wayland', KWIN_CRASHES)
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n')
        self.assertEqual(open(os.path.join(self.state, 'desktop-notice')).read().split()[0::3], ['kde', 'kwin'])
        self.assertEqual(self.helper('notice')['kind'], 'retired')
        self.assertEqual(self.helper('notice')['kind'], 'failed')
        self.assertEqual(self.helper('notice'), {})
        self.assertEqual([n['kind'] for n in self.helper('status')['notices']], ['retired', 'failed'])

    def test_mantel_never_up_ends_kwin_and_opens_labwc(self):
        self.write_bin('kwin_wayland', KWIN_NO_SHELL)
        t = time.time()
        r, log = self.session()
        self.assertLess(time.time() - t, 30)
        self.assertIn("kwin: Mantel didn't say it's up after 3s; ending KWin", log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)
        self.assertEqual(self.notice(), ['kwin', 'failed'])
        self.assertEqual(subprocess.run(['pgrep', '-f', os.path.join(self.bin, 'kwin_wayland')]).returncode, 1, 'KWin is gone')

    def test_labwc_picked_is_honoured(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        r, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=labwc\n')
        self.assertIn('desktop: labwc (picked in Settings', log)
        self.assertEqual(self.lines('kwin-ran'), [])
        self.assertEqual(len(self.lines('labwc-ran')), 1)
        self.assertEqual(self.notice(), [])

    def test_safe_graphics_is_labwc(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        open(os.path.join(self.tmp, 'safe'), 'w').close()
        r, log = self.session()
        self.assertIn('safe graphics this time, labwc instead of KWin', log)
        self.assertEqual(self.lines('kwin-ran'), [])
        self.assertEqual(len(self.lines('labwc-ran')), 1)

    def test_a_switch_asked_for_quickly_is_not_a_failure(self):
        self.write_bin('kwin_wayland', '#!/bin/sh\necho logout > "$XDG_RUNTIME_DIR/nimbus-next"\nexit 0\n')
        r, log = self.session()
        self.assertEqual(self.fails(), '')
        self.assertEqual(self.notice(), [])
        self.assertEqual(self.lines('labwc-ran'), [], log)

    def test_your_kwinrc_is_never_written(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        mine = os.path.join(self.home, '.config', 'kwinrc')
        with open(mine, 'w') as f:
            f.write('[Windows]\nFocusPolicy=FocusFollowsMouse\n')
        self.session()
        self.assertEqual(open(mine).read(), '[Windows]\nFocusPolicy=FocusFollowsMouse\n')
        self.assertFalse(os.path.exists(mine + '.bak'))
        for f in ('kwinrc', 'kglobalshortcutsrc'):
            self.assertTrue((ROOT / 'hosts/linux/desktop/kwin/xdg' / f).is_file())


class Choice(Base):
    def test_wm_writes_only_its_line_with_a_backup(self):
        with open(self.conf, 'w') as f:
            f.write('NIMBUS_START_MODE=desktop\nFOO=1\n')
        st = self.helper('wm', 'labwc')
        self.assertEqual(st['wm'], 'labwc')
        self.assertEqual(pathlib.Path(self.conf).read_text(), 'NIMBUS_START_MODE=desktop\nFOO=1\nHEARTH_DESKTOP_WM=labwc\n')
        self.assertEqual(pathlib.Path(self.conf + '.bak').read_text(), 'NIMBUS_START_MODE=desktop\nFOO=1\n')
        self.assertEqual(self.helper('wm', 'kwin')['wm'], 'kwin')
        self.assertIn('HEARTH_DESKTOP_WM=kwin\n', pathlib.Path(self.conf).read_text())
        r = subprocess.run([sys.executable, str(HELPER), 'wm', 'sway'], env=self.env(), capture_output=True, text=True)
        self.assertEqual(r.returncode, 1)
        self.assertIn('HEARTH_DESKTOP_WM=kwin\n', pathlib.Path(self.conf).read_text())

    def test_status_reads_without_writing(self):
        self.assertEqual(self.helper('status')['wm'], 'kwin')
        self.assertFalse(os.path.exists(self.conf))


class Mode(Base):
    def sleeper(self):
        p = subprocess.Popen(['sleep', '60'])
        self.addCleanup(lambda: (p.kill(), p.wait()))
        return p

    def mode(self, *args, **kw):
        e = self.env(HEARTH_SIDE_HELPER='/nonexistent', **kw)
        return subprocess.run(['sh', str(MODE)] + list(args), env=e, capture_output=True, text=True, timeout=20)

    def test_ends_kwin_through_its_pid_and_never_runs_labwc(self):
        self.write_bin('labwc', '#!/bin/sh\necho "labwc $*" >> "$HOME/labwc-ran"\nexit 0\n')
        p = self.sleeper()
        self.assertEqual(self.mode('current', HEARTH_WM='kwin', HEARTH_DESKTOP_PID=str(p.pid)).stdout.strip(), 'desktop')
        self.mode('game', HEARTH_WM='kwin', HEARTH_DESKTOP_PID=str(p.pid))
        p.wait(timeout=10)
        self.assertEqual(p.returncode, -signal.SIGTERM)
        self.assertEqual(self.lines('labwc-ran'), [])
        self.assertEqual(open(os.path.join(self.run_dir, 'nimbus-next')).read().strip(), 'game')

    def test_a_stale_labwc_pid_under_kwin_still_never_runs_labwc(self):
        self.write_bin('labwc', '#!/bin/sh\necho "labwc $*" >> "$HOME/labwc-ran"\nexit 0\n')
        p = self.sleeper()
        self.mode('desktop', HEARTH_WM='kwin', HEARTH_DESKTOP_PID=str(p.pid), LABWC_PID='1')
        p.wait(timeout=10)
        self.assertEqual(self.lines('labwc-ran'), [])

    def test_labwc_still_ends_the_labwc_way(self):
        self.write_bin('labwc', '#!/bin/sh\necho "labwc $*" >> "$HOME/labwc-ran"\nexit 0\n')
        p = self.sleeper()
        self.mode('game', HEARTH_WM='labwc', HEARTH_DESKTOP_PID=str(p.pid), LABWC_PID=str(p.pid))
        self.assertEqual(self.lines('labwc-ran'), ['labwc --exit'])
        p.wait(timeout=10)                                                     # the 4 s guard ends it anyway
        self.assertEqual(p.returncode, -signal.SIGKILL)

    def test_settle_ends_kwin(self):
        text = SESSION.read_text()
        settle = text[text.index('settle() {'):text.index('reset_controllers() {')]
        self.assertIn('pkill -KILL -u "$(id -u)" -x kwin_wayland', settle)


class Helpers(unittest.TestCase):
    def test_labwc_reconfigure_only_under_labwc(self):
        for f in ('hosts/linux/desktop/shell/hearth-access', 'hosts/linux/desktop/shell/hearth-system',
                  'hosts/linux/desktop/theme/hearth-theme'):
            text = (ROOT / f).read_text()
            for i, line in enumerate(text.splitlines()):
                if "'labwc', '--reconfigure'" in line:
                    near = '\n'.join(text.splitlines()[max(0, i - 2):i + 1])
                    self.assertIn("HEARTH_WM", near, '%s:%d runs labwc --reconfigure without checking HEARTH_WM' % (f, i + 1))

    def test_desktop_pid_is_known_everywhere_labwc_pid_was(self):
        self.assertIn('HEARTH_DESKTOP_PID', (ROOT / 'hosts/hearth-apptheme/apptheme.rs').read_text())
        self.assertIn("'HEARTH_DESKTOP_PID'", (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-coop').read_text())
        self.assertIn("env.pop('HEARTH_DESKTOP_PID', None)", (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-seats').read_text())
        self.assertIn("env.pop('HEARTH_DESKTOP_PID', None)", (ROOT / 'hosts/linux/desktop/shell/hearth-server-mode').read_text())
        self.assertIn('HEARTH_DESKTOP_PID', AUTOSTART.read_text())
        self.assertIn('hearth-shell-up', (ROOT / 'hosts/linux/desktop/shell/shell.qml').read_text())


class Autostart(Base):
    """The one autostart under KWin: the bridge and the screens hook start, wlr-randr (labwc's) doesn't."""
    def test_under_kwin(self):
        shell = os.path.join(self.share, 'desktop', 'shell')
        os.makedirs(shell)
        open(os.path.join(shell, 'shell.qml'), 'w').close()
        with open(os.path.join(shell, 'hearth-kwin-bridge'), 'w') as f:
            f.write('import os, sys\nopen(os.path.join(os.environ["HOME"], "bridge-ran"), "w").write(" ".join(sys.argv[1:]))\n')
        self.write_bin('hearth-screens-stub', '#!/bin/sh\necho "$*" > "$HOME/screens-ran"\n')
        os.symlink(os.path.join(self.bin, 'hearth-screens-stub'), os.path.join(shell, 'hearth-screens'))
        self.write_bin('wlr-randr', '#!/bin/sh\necho ran >> "$HOME/wlr-randr-ran"\n')
        self.write_bin('quickshell', '#!/bin/sh\necho "wm=$HEARTH_WM pid=$HEARTH_DESKTOP_PID" >> "$HOME/quickshell-ran"\nsleep 30\n')
        for b in ('dbus-update-activation-environment', 'systemctl', 'notify-send', 'nimbus-mode', 'hearth-tools'):
            self.write_bin(b, '#!/bin/sh\nexit 0\n')
        p = subprocess.Popen(['sleep', '60'])
        self.addCleanup(lambda: (p.kill(), p.wait()))
        e = self.env(HEARTH_WM='kwin', HEARTH_DESKTOP_PID=str(p.pid))
        subprocess.run(['sh', str(AUTOSTART)], env=e, timeout=30)
        for _ in range(150):
            if os.path.exists(os.path.join(self.home, 'bridge-ran')) and self.lines('quickshell-ran'):
                break
            time.sleep(0.2)
        self.assertEqual(open(os.path.join(self.home, 'bridge-ran')).read(), 'serve')
        self.assertEqual(open(os.path.join(self.home, 'screens-ran')).read().strip(), 'restore')
        self.assertFalse(os.path.exists(os.path.join(self.home, 'wlr-randr-ran')))
        self.assertEqual(self.lines('quickshell-ran'), ['wm=kwin pid=%d' % p.pid])


if __name__ == '__main__':
    unittest.main()
