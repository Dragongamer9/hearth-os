"""Stand-ins and the harness for nimbus-session's Desktop Mode tests (test_firehold_session.py, test_desktop_retired.py):
the session runs for real with stand-in window managers. (Moved out of test_kwin_session.py when KWin was archived in
1.15: archive/kwin.)"""
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
        # The share as the image has it: labwc's folder (its environment).
        shutil.copytree(ROOT / 'hosts/linux/desktop/labwc', os.path.join(self.share, 'desktop', 'labwc'))
        # The lock screen's check and the notice helper, as the image has them next to Mantel's shell.
        os.makedirs(os.path.join(self.share, 'desktop', 'shell'))
        for f in ('hearth-your-desktop', 'hearth_settings_io.py'):
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
             'HEARTH_KWIN_UP_SECS': '3', 'HEARTH_KWIN_EARLY_SECS': '2', 'HEARTH_KWIN_STALL_SECS': '2', 'HEARTH_SAFE_FLAG': os.path.join(self.tmp, 'safe'),
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


