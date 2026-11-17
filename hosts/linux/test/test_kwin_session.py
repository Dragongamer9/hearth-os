"""1.14: Mantel on KWin (docs/DESKTOPS.md "Mantel on KWin"). KWin by default; labwc whenever KWin isn't there, ends
right away or Mantel doesn't say it's up, with a notice and a count (twice in a row → labwc until it's turned back on);
HEARTH_DESKTOP_WM=labwc, safe graphics and a second player's seat stay on labwc. nimbus-mode ends KWin through
HEARTH_DESKTOP_PID and never runs labwc's own commands under KWin. The session runs for real with stand-ins."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import re
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
  echo "xcd=$XDG_CURRENT_DESKTOP"; echo "xdg=$XDG_CONFIG_DIRS"; echo "qpa=$QT_QPA_PLATFORM"; echo "shell=${PLASMA_DEFAULT_SHELL-unset}"; } >> "$HOME/kwin-ran"
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
        # The lock guard's notice (no lock screen in these tests) already told: the other notices are each test's own.
        os.makedirs(self.state)
        open(os.path.join(self.state, 'kwin-lock-off-told'), 'w').close()
        # The share as the image has it: labwc's folder (its environment) and KWin's.
        shutil.copytree(ROOT / 'hosts/linux/desktop/labwc', os.path.join(self.share, 'desktop', 'labwc'))
        shutil.copytree(ROOT / 'hosts/linux/desktop/kwin', os.path.join(self.share, 'desktop', 'kwin'))
        # The lock screen's check and the notice helper, as the image has them next to Mantel's shell.
        os.makedirs(os.path.join(self.share, 'desktop', 'shell'))
        for f in ('hearth-lockcheck', 'hearth-your-desktop', 'hearth_settings_io.py'):
            shutil.copy(ROOT / 'hosts/linux/desktop/shell' / f, os.path.join(self.share, 'desktop', 'shell', f))
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
             'HEARTH_KWIN_UP_SECS': '3', 'HEARTH_KWIN_EARLY_SECS': '2', 'HEARTH_SAFE_FLAG': os.path.join(self.tmp, 'safe'),
             'HEARTH_LOCK_PKG': os.path.join(self.tmp, 'no-lock-package'), 'HEARTH_LOCK_GREETER': os.path.join(self.tmp, 'no-greeter')}
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

    def lock_screen(self, says='hearth-lock-screen: loaded'):
        """Hearth's lock screen package and a KWin greeter whose test run SAYS that (then stays up, as the real one
        does), with Qt's Wayland plugins: env for session()."""
        pkg = os.path.join(self.tmp, 'lockpkg')
        os.makedirs(os.path.join(pkg, 'contents', 'lockscreen'), exist_ok=True)
        open(os.path.join(pkg, 'contents', 'lockscreen', 'LockScreen.qml'), 'w').close()
        with open(os.path.join(pkg, 'metadata.json'), 'w') as f:
            json.dump({'X-Plasma-APIVersion': '2'}, f)
        qt = os.path.join(self.tmp, 'qt6plugins')
        for f in ('platforms/libqwayland-generic.so', 'wayland-shell-integration/liblayer-shell.so'):
            os.makedirs(os.path.dirname(os.path.join(qt, f)), exist_ok=True)
            open(os.path.join(qt, f), 'w').close()
        self.write_bin('greeter', '#!/bin/sh\necho run >> "$HOME/greeter-runs"\necho "%s" >&2\nexec sleep 30\n' % says)
        return {'HEARTH_LOCK_PKG': pkg, 'HEARTH_LOCK_GREETER': os.path.join(self.bin, 'greeter'), 'HEARTH_LOCK_QT_PLUGINS': qt}

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
        # Hearth's lock screen isn't here: KWin without its locker (never KDE's or a broken lock screen).
        self.assertIn('args=--xwayland --no-lockscreen %s/desktop/kwin/autostart' % self.share, ran)
        self.assertIn('shell=unset', ran)
        pid = [l for l in self.lines('kwin-ran') if l.startswith('pid=')][0].split()
        self.assertEqual(pid[0][4:], pid[1][len('desktop_pid='):], 'HEARTH_DESKTOP_PID is KWin itself')
        self.assertIn('wm=kwin', ran)
        self.assertIn('labwc_pid=unset', ran)                               # never LABWC_PID under KWin
        self.assertIn('xcd=Hearth:Nimbus:KDE', ran)                          # Hearth first (its portals), then Nimbus
        self.assertIn('xdg=%s/desktop/kwin/xdg:' % self.share, ran)         # Hearth's defaults under yours
        self.assertIn('qpa=wayland;xcb', ran)                               # labwc's environment file
        self.assertTrue(os.path.exists(os.path.join(self.home, '.local/share/nimbus/kwin.log')))
        self.assertEqual(self.notice(), [])
        self.assertEqual(self.fails(), '')

    def test_kwin_env_written_before_kwin_starts(self):
        # KWin's stand-in copies kwin.env as it is the moment KWin starts: it's there already, NUL-separated, mode 600 in
        # a 700 folder, with what KWin itself got (its own pid as HEARTH_DESKTOP_PID, Hearth's desktop names).
        self.write_bin('kwin_wayland', '#!/bin/sh\nf="$XDG_RUNTIME_DIR/hearth/kwin.env"\n'
                       '[ -f "$f" ] && cp "$f" "$HOME/env-at-start" && stat -c %a "$f" "$XDG_RUNTIME_DIR/hearth" > "$HOME/env-modes"\n'
                       'echo "pid=$$" > "$HOME/kwin-pid"\n' + KWIN_WORKS.split('\n', 1)[1])
        r, log = self.session()
        got = os.path.join(self.home, 'env-at-start')
        self.assertTrue(os.path.exists(got), log)
        env = dict(x.split('=', 1) for x in open(got, 'rb').read().decode().split('\0') if '=' in x)
        self.assertEqual(env.get('HEARTH_DESKTOP_PID'), self.lines('kwin-pid')[0][4:])
        self.assertEqual(env.get('XDG_CURRENT_DESKTOP'), 'Hearth:Nimbus:KDE')
        self.assertEqual(env.get('HEARTH_WM'), 'kwin')
        self.assertEqual(self.lines('env-modes'), ['600', '700'])
        # gone once KWin has ended (the session ended or switched)
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth', 'kwin.env')))
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth', 'kwin.env.tmp')))
        src = SESSION.read_text()
        body = src[src.index('run_kwin() {'):src.index('\n}\n', src.index('run_kwin() {'))]
        self.assertLess(body.index('env -0 >"$d/kwin.env.tmp" && mv -f "$d/kwin.env.tmp" "$d/kwin.env"'), body.index('exec "$1" --xwayland'))

    def test_look_applied_before_kwin_and_never_in_the_way(self):
        # theme/apply.sh (Hearth's Aurorae frames) runs before KWin starts; failing, it's logged and KWin starts anyway.
        theme = os.path.join(self.share, 'desktop', 'theme')
        os.makedirs(theme)
        with open(os.path.join(theme, 'apply.sh'), 'w') as f:
            f.write('#!/bin/sh\n: > "$HOME/look-applied"\nexit 3\n')
        self.write_bin('kwin_wayland', '#!/bin/sh\n[ -e "$HOME/look-applied" ] && echo yes > "$HOME/look-before-kwin"\n'
                       + KWIN_WORKS.split('\n', 1)[1])
        r, log = self.session()
        self.assertEqual(self.lines('look-before-kwin'), ['yes'], log)
        self.assertIn("theme/apply.sh didn't finish (code 3)", log)
        self.assertIn('kwin: Mantel up after', log)
        rc = (ROOT / 'hosts/linux/desktop/kwin/xdg/kwinrc').read_text()
        self.assertNotRegex(rc, r'(?m)^\[org\.kde\.kdecoration2\]')   # kwin/windows/xdg/kwinrc's Aurorae frames, not overridden

    def test_kwin_env_that_cant_be_written_never_stops_kwin(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        open(os.path.join(self.run_dir, 'hearth'), 'w').close()      # a file where the folder goes: the write fails
        r, log = self.session()
        self.assertIn('kwin: Mantel up after', log)
        self.assertEqual(self.lines('labwc-ran'), [])
        self.assertIn("couldn't write", open(os.path.join(self.home, '.local/share/nimbus/kwin.log')).read())

    def test_lock_guard_tells_once_through_hearths_notice(self):
        os.remove(os.path.join(self.state, 'kwin-lock-off-told'))    # (Base keeps the other tests' notices to their own)
        self.write_bin('kwin_wayland', KWIN_WORKS)
        r, log = self.session()                                       # no lock screen package here
        self.assertIn('--no-lockscreen', '\n'.join(self.lines('kwin-ran')))
        self.assertIn('shell=unset', '\n'.join(self.lines('kwin-ran')))
        p = os.path.join(self.state, 'desktop-notice')
        self.assertEqual([l.split()[:2] for l in open(p).read().splitlines()], [['kwin-lock', 'nopackage']])
        st = self.helper('status')
        self.assertIn("Hearth's lock screen isn't installed correctly", json.dumps(st))
        os.remove(p)
        self.session()                                                # told once
        self.assertFalse(os.path.exists(p))
        # working again, then broken again: told again
        lk = self.lock_screen()
        self.session(**lk)
        self.assertFalse(os.path.exists(p))
        self.session(HEARTH_LOCK_PKG=lk['HEARTH_LOCK_PKG'])           # the package, no locker
        self.assertEqual([l.split()[:2] for l in open(p).read().splitlines()], [['kwin-lock', 'nolocker']])

    def test_kwin_locker_only_with_hearths_lock_screen(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        lk = self.lock_screen()
        started = time.monotonic()
        r, log = self.session(**lk)
        ran = '\n'.join(self.lines('kwin-ran'))
        self.assertIn('args=--xwayland %s/desktop/kwin/autostart' % self.share, ran)
        self.assertIn('shell=org.hearth.mantel-lock', ran)
        self.assertIn("KWin's screen locker on", log)
        self.assertIn('loads in KWin\'s greeter', log)
        self.assertLess(time.monotonic() - started, 25, 'the greeter\'s test run ends as soon as it says it loaded')
        # The package without the greeter: no locker.
        os.remove(os.path.join(self.home, 'kwin-ran'))
        r, log = self.session(HEARTH_LOCK_PKG=lk['HEARTH_LOCK_PKG'])
        self.assertIn('--no-lockscreen', '\n'.join(self.lines('kwin-ran')))
        rc = (ROOT / 'hosts/linux/desktop/kwin/xdg/kscreenlockerrc').read_text()
        for k in ('Autolock[$i]=false', 'LockOnResume[$i]=false', 'Timeout[$i]=0'):
            self.assertIn(k, rc)

    def test_the_lock_screen_must_really_load_before_kwins_locker_is_on(self):
        # Both files there isn't enough (review, Nov 17): the greeter's test run has to show Hearth's lock screen
        # loading, or KWin starts with --no-lockscreen and the notice says so.
        os.remove(os.path.join(self.state, 'kwin-lock-off-told'))
        self.write_bin('kwin_wayland', KWIN_WORKS)
        lk = self.lock_screen('kscreenlocker_greet: Failed to load lockscreen QML, falling back to built-in locker')
        r, log = self.kwin_session(**lk)
        self.assertIn('--no-lockscreen', '\n'.join(self.lines('kwin-ran')))
        self.assertIn('shell=unset', '\n'.join(self.lines('kwin-ran')))
        self.assertIn('didn\'t load in KWin\'s greeter', log)
        with open(os.path.join(self.state, 'desktop-notice')) as f:
            self.assertEqual([l.split()[:2] for l in f], [['kwin-lock', 'notloaded']], log)   # and no "kwin failed"
        self.assertIn('didn\'t show Hearth\'s lock screen loading', json.dumps(self.helper('status')))
        self.assertFalse(os.path.exists(os.path.join(self.state, 'kwin-lock-proven')), 'a failure is never kept')
        # A greeter that says nothing at all: still off.
        os.remove(os.path.join(self.home, 'kwin-ran'))
        self.kwin_session(**self.lock_screen('something else'))
        self.assertIn('--no-lockscreen', '\n'.join(self.lines('kwin-ran')))

    def test_the_lock_check_runs_once_per_image(self):
        self.write_bin('kwin_wayland', KWIN_WORKS)
        lk = self.lock_screen()
        with open(os.path.join(self.share, 'version'), 'w') as f:
            f.write('1.14\n')
        r, log = self.session(**lk)
        self.assertIn('checked once for this image', log)
        self.assertEqual(len(self.lines('greeter-runs')), 1)
        shutil.rmtree(os.path.join(self.run_dir, 'hearth'), ignore_errors=True)     # a new boot: /run is empty
        r, log = self.session(**lk)
        self.assertIn("KWin's screen locker on", log)
        self.assertNotIn('checked once', log)
        self.assertEqual(len(self.lines('greeter-runs')), 1, 'the same image: not checked again')
        with open(os.path.join(self.share, 'version'), 'w') as f:
            f.write('1.15\n')
        shutil.rmtree(os.path.join(self.run_dir, 'hearth'), ignore_errors=True)
        r, log = self.session(**lk)
        self.assertIn('checked once for this image', log)
        self.assertEqual(len(self.lines('greeter-runs')), 2, 'a new image: checked again')

    def kwin_session(self, **kw):
        """A Desktop Mode start that must really be on KWin: a stand-in KWin slow to start under load is never taken
        for a failed one (HEARTH_KWIN_UP_SECS 60; KWIN_WORKS says it's up at once, so this doesn't wait), and a
        fallback to labwc fails the test instead of skipping run_kwin's steps for the rest of the sign-in. The
        stand-in gets a name of its own (HEARTH_KWIN_BIN): a session ending in another test run at the same time
        kills every kwin_wayland of this user by name (settle_kwin's pkill -x), which is what made these tests flaky
        (the log said "kwin ended before Mantel came up (code 137)")."""
        name = 'kwt' + os.path.basename(self.tmp)[-10:].replace('-', '').replace('_', '')
        shutil.copy(os.path.join(self.bin, 'kwin_wayland'), os.path.join(self.bin, name))
        r, log = self.session(HEARTH_KWIN_UP_SECS='60', HEARTH_KWIN_BIN=name, **kw)
        self.assertIn('kwin: Mantel up after', log)
        self.assertEqual(self.lines('labwc-ran'), [], log)
        self.assertNotIn('kwin', [l.split()[0] for l in open(os.path.join(self.state, 'desktop-fails'))]
                         if os.path.exists(os.path.join(self.state, 'desktop-fails')) else [])
        return r, log

    def test_your_kde_settings_that_hearth_overrides_are_told_once(self):
        # Hearth's fixed KWin keys ([$i]) win over a kscreenlockerrc / kwinrc left from KDE Plasma: the person is told
        # once which, and their files are never changed (red flag 9). (The flake, Nov 17: a stand-in KWin judged "not
        # up" after 3 s under load meant labwc for the rest of that sign-in, so the third start never reached
        # run_kwin's check. kwin_session rules that out.)
        self.write_bin('kwin_wayland', KWIN_WORKS)
        cfg = os.path.join(self.home, '.config')
        os.makedirs(cfg, exist_ok=True)
        lockrc = '[Daemon]\nAutolock=true\nLockOnResume=true\nTimeout=5\n'
        kwinrc = '[NightColor]\nActive=true\n\n[Compositing]\nAllowTearing=true\n'
        for name, text in (('kscreenlockerrc', lockrc), ('kwinrc', kwinrc)):
            with open(os.path.join(cfg, name), 'w') as f:
                f.write(text)
        notice = os.path.join(self.state, 'desktop-notice')

        def lines():
            with open(notice) as f:
                return [l.split()[:2] for l in f]
        self.kwin_session()
        self.assertEqual(lines(), [['kwin-mine', 'autolock+lockonresume']])
        st = json.dumps(self.helper('status'))
        self.assertIn('Lock the screen automatically', st)
        self.assertIn('Settings \\u203a Sign-in', st)
        self.assertNotIn('Night', st, 'Night Light and tearing are yours (Settings › Display), never fixed')
        os.remove(notice)
        self.kwin_session()
        self.assertFalse(os.path.exists(notice), 'told once')
        # Another one later: told about that.
        with open(os.path.join(cfg, 'kwinrc'), 'a') as f:
            f.write('\n[Wayland]\nInputMethod[$e]=/usr/share/applications/com.github.maliit.keyboard.desktop\n')
        self.kwin_session()
        self.assertEqual(lines(), [['kwin-mine', 'autolock+lockonresume+inputmethod']])
        with open(os.path.join(cfg, 'kscreenlockerrc')) as f:
            self.assertEqual(f.read(), lockrc)
        self.assertFalse(os.path.exists(os.path.join(cfg, 'kscreenlockerrc.bak')))
        with open(os.path.join(cfg, 'kwinrc')) as f:
            self.assertIn('[NightColor]\nActive=true', f.read())

    def test_told_only_once_the_notice_is_really_queued(self):
        # In-process, no session and no timing: a notice that couldn't be written is never marked as told, so the
        # next start tells again; a notice Mantel is marking as shown at the same moment can't be lost (one lock).
        import importlib.machinery
        import importlib.util
        from unittest import mock
        cfg = os.path.join(self.home, '.config')
        os.makedirs(cfg, exist_ok=True)
        with open(os.path.join(cfg, 'kscreenlockerrc'), 'w') as f:
            f.write('[Daemon]\nAutolock=true\n')
        with mock.patch.dict(os.environ, {'HOME': self.home, 'XDG_STATE_HOME': os.path.join(self.home, '.local', 'state'),
                                          'XDG_CONFIG_HOME': cfg}):
            loader = importlib.machinery.SourceFileLoader('hyd_q', str(HELPER))
            spec = importlib.util.spec_from_loader('hyd_q', loader)
            hyd = importlib.util.module_from_spec(spec)
            loader.exec_module(hyd)
            told = os.path.join(self.state, 'kwin-mine-told')
            with mock.patch.object(hyd, 'write_lines', lambda path, ns: None):        # the write didn't land
                self.assertEqual(hyd.cmd_kwin_overrides(), {'overridden': ['autolock'], 'told': False})
            self.assertFalse(os.path.exists(told))
            with mock.patch.object(hyd, 'write_lines', side_effect=OSError(28, 'No space left on device')):
                with self.assertRaises(OSError):
                    hyd.cmd_kwin_overrides()
            self.assertFalse(os.path.exists(told), 'a failed write is never marked as told')
            self.assertEqual(hyd.cmd_kwin_overrides(), {'overridden': ['autolock'], 'told': True})
            self.assertTrue(os.path.exists(told))
            self.assertEqual([n['what'] for n in hyd.read_notices(hyd.NOTICE)], ['kwin-mine'])
            self.assertEqual(hyd.cmd_kwin_overrides()['told'], False, 'told once')
            # The lock: while someone holds it (the session's desk_notice, Mantel's `notice`), nobody else rewrites.
            got = []
            with hyd.sio.locked(hyd.NOTICE):
                import threading
                th = threading.Thread(target=lambda: got.append(hyd.cmd_notice()))
                th.start()
                th.join(0.3)
                self.assertTrue(th.is_alive(), 'cmd_notice waits for the lock')
            th.join(5)
            self.assertEqual(got[0]['what'], 'kwin-mine')
        sess = SESSION.read_text()
        self.assertIn('9>>"$DESK_STATE/.desktop-notice.lock"', sess)

    def test_hearths_fixed_keys_are_the_ones_it_tells_about(self):
        import importlib.machinery
        import importlib.util
        loader = importlib.machinery.SourceFileLoader('hyd', str(HELPER))
        spec = importlib.util.spec_from_loader('hyd', loader)
        hyd = importlib.util.module_from_spec(spec)
        loader.exec_module(hyd)
        fixed = {}
        for d in ('kwin/xdg', 'kwin/windows/xdg'):
            for name in ('kwinrc', 'kscreenlockerrc'):
                p = ROOT / 'hosts/linux/desktop' / d / name
                if not p.exists():
                    continue
                group = ''
                for line in p.read_text().splitlines():
                    s = line.strip()
                    if s.startswith('['):
                        group = s.strip('[]')
                    elif '=' in s and not s.startswith('#'):
                        k, v = s.split('=', 1)
                        opts = ''.join(re.findall(r'\[\$([a-z]+)\]', k))
                        if 'i' in opts:
                            fixed[(name, group, re.sub(r'\[.*', '', k))] = v
        listed = {(f, g, k): v for _i, f, g, k, v, _w, _h in hyd.LOCKED}
        self.assertEqual(fixed, listed, 'every [$i] key is in LOCKED (and told about), and only those')
        for keep in (('kwinrc', 'NightColor', 'Active'), ('kwinrc', 'Compositing', 'AllowTearing')):
            self.assertNotIn(keep, fixed, 'a setting Hearth exposes is never fixed')
        self.assertEqual(hyd.ini_values('[Daemon][$i]\nAutolock[$i]=true\n[A][B]\nx=1\n'),
                         {('Daemon', 'Autolock'): 'true', ('A/B', 'x'): '1'})

    def test_mantel_closing_under_kwin_opens_labwc(self):
        # Mantel up, then the autostart's give_up: the reason left, KWin ended, no next mode.
        self.write_bin('kwin_wayland', '#!/bin/sh\n: > "$XDG_RUNTIME_DIR/hearth-shell-up"\nsleep 3\n'
                       'echo "Mantel didn\'t keep running under KWin: keeps closing" > "$XDG_RUNTIME_DIR/hearth-kwin-shell-failed"\nexit 0\n')
        r, log = self.session(HEARTH_KWIN_EARLY_SECS='1')
        self.assertIn("didn't keep running under KWin", log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)
        self.assertEqual(self.notice(), ['kwin', 'failed'])
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth-kwin-shell-failed')))

    def test_give_up_under_kwin_ends_kwin_without_a_next_mode(self):
        text = AUTOSTART.read_text()
        g = text[text.index('give_up() {'):text.index('nimbus-mode game', text.index('give_up() {'))]
        self.assertIn('hearth-kwin-shell-failed', g)
        self.assertIn('kill -TERM "$HEARTH_DESKTOP_PID"', g)

    def test_a_kwin_that_hangs_after_a_switch_is_ended(self):
        self.write_bin('kwin_wayland', '#!/bin/sh\ntrap "" TERM\n: > "$XDG_RUNTIME_DIR/hearth-shell-up"\nsleep 3\n'
                       'echo logout > "$XDG_RUNTIME_DIR/nimbus-next"\nwhile :; do sleep 1; done\n')
        t = time.time()
        r, log = self.session()
        self.assertLess(time.time() - t, 40)
        self.assertIn('still running 8s after the switch was asked for; ending it', log)
        self.assertEqual(self.lines('labwc-ran'), [])
        self.assertEqual(self.fails(), '')

    def test_kwin_missing_opens_labwc_and_says_why(self):
        r, log = self.session(HEARTH_KWIN_BIN='hearth-no-kwin-here')
        self.assertIn("KWin (kwin_wayland) isn't installed; labwc instead", log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)
        self.assertIn('wm=labwc', self.lines('labwc-ran')[0])
        l = self.lines('labwc-ran')[0].split()
        self.assertEqual(l[-2][4:], l[-3][len('desktop_pid='):], 'HEARTH_DESKTOP_PID is labwc itself')
        self.assertEqual(self.notice(), ['kwin', 'missing'])
        n = self.helper('notice')
        self.assertEqual(n['title'], 'Mantel opened on labwc')
        self.assertTrue(os.path.exists(os.path.join(self.state, 'kwin-missing-told')))   # told once, across sign-ins

    def test_kwin_crashing_after_a_while_opens_labwc_never_a_loop(self):
        # Red flag 2 (start-safety review, Nov 17): KWin up, then a crash (139, SIGSEGV) after the early window with no
        # switch asked for. Before, the session ended (KWin's code) and Start in: Desktop with Relogin looped for ever.
        # Now: labwc for the rest of this sign-in, counted and told. (20 s on a device; EARLY is 2 s here, so 4 s.)
        crash = '#!/bin/sh\necho "args=$*" >> "$HOME/kwin-ran"\n: > "$XDG_RUNTIME_DIR/hearth-shell-up"\nsleep 4\nexit 139\n'
        self.write_bin('kwin_wayland', crash)
        conf = 'NIMBUS_START_MODE=desktop\n'
        r, log = self.session(conf, HEARTH_KWIN_UP_SECS='30')
        self.assertIn('kwin: Mantel up after', log)
        self.assertIn('kwin crashed after', log)
        self.assertIn('(code 139)', log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)               # labwc took over
        self.assertEqual(self.notice(), ['kwin', 'failed'])
        self.assertEqual(self.fails(), 'kwin 1\n')
        self.assertTrue(os.path.exists(os.path.join(self.run_dir, 'hearth-kwin-failed')))
        # The next start in this sign-in (Relogin): straight to labwc, no loop through KWin.
        r, log = self.session(conf, HEARTH_KWIN_UP_SECS='30')
        self.assertIn("KWin didn't start earlier this sign-in", log)
        self.assertEqual(len(self.lines('kwin-ran')), 1)

    def test_kwin_crash_after_a_long_run_starts_the_count_over(self):
        crash = '#!/bin/sh\n: > "$XDG_RUNTIME_DIR/hearth-shell-up"\nsleep 4\nexit 134\n'
        self.write_bin('kwin_wayland', crash)
        with open(os.path.join(self.state, 'desktop-fails'), 'w') as f:
            f.write('kwin 1\n')                                              # an old failure
        r, log = self.session(HEARTH_KWIN_UP_SECS='30', HEARTH_KWIN_GOOD_SECS='3')
        self.assertIn('kwin crashed after', log)
        self.assertEqual(self.fails(), 'kwin 1\n', log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)
        # A clean end (code 0) after the early window: not a failure, and the count starts over.
        os.remove(os.path.join(self.run_dir, 'hearth-kwin-failed'))
        self.write_bin('kwin_wayland', KWIN_WORKS)
        r, log = self.session()
        self.assertIn('kwin ended (code 0)', log)
        self.assertEqual(self.fails().strip(), '', 'a reset really removes the count')
        self.assertEqual(len(self.lines('labwc-ran')), 1)

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
        open(os.path.join(self.run_dir, 'hearth-kwin-failed'), 'w').close()
        st = self.helper('wm', 'kwin')
        self.assertEqual((st['kwinFails'], st['kwinPaused']), (0, False))
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth-kwin-failed')))   # this sign-in too
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
        for f in ('kwinrc',):
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
    """The one autostart under KWin: the screens hook runs, wlr-randr (labwc's) doesn't, and the bridge is left to
    Mantel (it starts hearth-kwin-bridge itself: feature/kwin-windows)."""
    def test_under_kwin(self):
        shell = os.path.join(self.share, 'desktop', 'shell')
        os.makedirs(shell, exist_ok=True)
        open(os.path.join(shell, 'shell.qml'), 'w').close()
        with open(os.path.join(shell, 'hearth-kwin-bridge'), 'w') as f:
            f.write('import os, sys\nopen(os.path.join(os.environ["HOME"], "bridge-ran"), "w").write(" ".join(sys.argv[1:]))\n')
        self.write_bin('hearth-screens-stub', '#!/bin/sh\necho "$*" > "$HOME/screens-ran"\n')
        os.symlink(os.path.join(self.bin, 'hearth-screens-stub'), os.path.join(shell, 'hearth-screens'))
        self.write_bin('wlr-randr', '#!/bin/sh\necho ran >> "$HOME/wlr-randr-ran"\n')
        self.write_bin('quickshell', '#!/bin/sh\necho "wm=$HEARTH_WM pid=$HEARTH_DESKTOP_PID" >> "$HOME/quickshell-ran"\nsleep 400\n')   # longer than the wait below: never restarted
        for b in ('dbus-update-activation-environment', 'systemctl', 'notify-send', 'nimbus-mode', 'hearth-tools'):
            self.write_bin(b, '#!/bin/sh\nexit 0\n')
        p = subprocess.Popen(['sleep', '60'])
        self.addCleanup(lambda: (p.kill(), p.wait()))
        e = self.env(HEARTH_WM='kwin', HEARTH_DESKTOP_PID=str(p.pid))
        subprocess.run(['sh', str(AUTOSTART)], env=e, timeout=30)
        for _ in range(1500):      # up to 5 minutes: a busy release runner can be slow to start the stand-ins
            if os.path.exists(os.path.join(self.home, 'screens-ran')) and self.lines('quickshell-ran'):
                break
            time.sleep(0.2)
        self.assertFalse(os.path.exists(os.path.join(self.home, 'bridge-ran')))
        self.assertEqual(open(os.path.join(self.home, 'screens-ran')).read().strip(), 'restore')
        self.assertFalse(os.path.exists(os.path.join(self.home, 'wlr-randr-ran')))
        self.assertEqual(self.lines('quickshell-ran'), ['wm=kwin pid=%d' % p.pid])


if __name__ == '__main__':
    unittest.main()
