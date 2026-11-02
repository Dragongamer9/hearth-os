"""Tests for Hearth Plasma: the desktop choice and the fallback in hosts/linux/session/nimbus-session (a broken or
missing Plasma always ends in Hearth Classic, never without a desktop), and desktop/plasma/hearth-plasma (what Hearth's
Plasma widgets and Settings' desktop choice use). The session runs for real with stand-ins for labwc and Plasma."""
import unittest
# Hearth Plasma is gone: Mantel is the only desktop (nimbus-session never starts it, Settings has no desktop choice,
# the image doesn't install it). These tests are for the removed desktop; the file stays for its dispatch-only
# workflow and history. What replaced it is tested in test_hearth_mantel_only.py.
raise unittest.SkipTest('Hearth Plasma was removed: Mantel is the only desktop')
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import signal
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
MODE = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-mode'
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'plasma' / 'hearth-plasma'
ADMIN_RS = ROOT / 'hosts' / 'hearth-core' / 'src' / 'admin.rs'
SYSTEM_FILES = ROOT / 'os' / 'system_files' / 'usr' / 'share'
LNF = SYSTEM_FILES / 'plasma' / 'look-and-feel' / 'org.hearth.desktop'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_plasma', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_plasma', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


FAKE_PLASMA = {
    # Crashes straight away.
    'crash': '#!/bin/sh\necho "plasma crashed" >&2\nexit 1\n',
    # Starts and never shows a desktop.
    'hang': '#!/bin/sh\nexec sleep 30\n',
    # Starts KWin and the shell, stays a few seconds, then ends normally (signing out).
    'ok': '#!/bin/sh\n"$(dirname "$0")/kwin_wayland" & "$(dirname "$0")/plasmashell" &\n'
          'echo "XDG_CONFIG_DIRS=$XDG_CONFIG_DIRS HEARTH_DESKTOP=$HEARTH_DESKTOP QT=${QT_QPA_PLATFORMTHEME-unset}" > "$HOME/plasma-env"\n'
          'sleep 4\nexit 0\n',
}


class SessionFallback(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-plasma-test-')
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        for d in (self.home, self.run_dir, self.bin, self.share):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        os.makedirs(os.path.dirname(self.conf))
        # labwc stand-in: leaves a mark and ends (no next mode: the session ends).
        self.write_bin('labwc', '#!/bin/sh\ntouch "$HOME/labwc-ran"\nexit 0\n')

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.bin], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def plasma(self, kind):
        self.write_bin('startplasma-wayland', FAKE_PLASMA[kind])
        self.write_bin('kwin_wayland', '#!/bin/sh\nsleep 6\n')
        self.write_bin('plasmashell', '#!/bin/sh\nsleep 6\n')

    def session(self, conf, *args, timeout=60, extra_env=None):
        with open(self.conf, 'w') as f:
            f.write(conf)
        env = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
               'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent', 'HEARTH_PLASMA_TIMEOUT': '20'}
        env.update(extra_env or {})
        r = subprocess.run(['sh', str(SESSION)] + list(args), env=env, capture_output=True, text=True, timeout=timeout)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        return r, log.read_text() if log.exists() else ''

    def labwc_ran(self):
        return os.path.exists(os.path.join(self.home, 'labwc-ran'))

    def test_classic_by_default(self):
        self.plasma('ok')
        _, log = self.session('NIMBUS_START_MODE=desktop\n')
        self.assertTrue(self.labwc_ran(), log)
        self.assertIn('Hearth Classic', log)
        self.assertNotIn('Hearth Plasma', log)

    def test_plasma_crash_falls_back_to_classic(self):
        self.plasma('crash')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n')
        self.assertTrue(self.labwc_ran(), log)
        self.assertIn('falling back to Hearth Classic', log)
        self.assertTrue(os.path.exists(os.path.join(self.run_dir, 'hearth-plasma-failed')))
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth-plasma.pid')))

    def test_plasma_without_a_desktop_in_time_falls_back(self):
        self.plasma('hang')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n', extra_env={'HEARTH_PLASMA_TIMEOUT': '2'})
        self.assertIn('no desktop after 2s', log)
        self.assertTrue(self.labwc_ran(), log)

    def test_plasma_missing_uses_classic(self):
        if any(shutil.which(b, path='/usr/bin:/bin') for b in ('startplasma-wayland', 'kwin_wayland', 'plasmashell')):
            self.skipTest('Plasma is installed on this machine')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n')
        self.assertIn('plasma: not installed', log)
        self.assertTrue(self.labwc_ran(), log)

    def test_plasma_runs_and_ends_normally(self):
        self.plasma('ok')
        _, log = self.session('NIMBUS_START_MODE=desktop\nexport HEARTH_DESKTOP="plasma"\n')
        self.assertIn('desktop mode: Hearth Plasma', log)
        self.assertIn('plasma ended (code 0)', log)
        self.assertFalse(self.labwc_ran(), log)
        env = pathlib.Path(self.home, 'plasma-env').read_text()
        self.assertIn('/usr/share/hearth-plasma/xdg:', env)
        self.assertIn('HEARTH_DESKTOP=plasma', env)
        self.assertIn('QT=unset', env)
        self.assertTrue(os.path.exists(os.path.join(self.home, '.config', 'nimbus', 'plasma-set-up')))

    def test_first_run_puts_other_plasma_layout_aside(self):
        self.plasma('ok')
        cfg = os.path.join(self.home, '.config')
        with open(os.path.join(cfg, 'plasma-org.kde.plasma.desktop-appletsrc'), 'w') as f:
            f.write('[old]\n')
        with open(os.path.join(cfg, 'kdeglobals'), 'w') as f:
            f.write('[KDE]\nLookAndFeelPackage=org.kde.breezedark.desktop\nSingleClick=false\n')
        self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n')
        self.assertTrue(os.path.exists(os.path.join(cfg, 'plasma-org.kde.plasma.desktop-appletsrc.before-hearth')))
        self.assertFalse(os.path.exists(os.path.join(cfg, 'plasma-org.kde.plasma.desktop-appletsrc')))
        kg = pathlib.Path(cfg, 'kdeglobals').read_text()
        self.assertNotIn('LookAndFeelPackage', kg)
        self.assertIn('SingleClick=false', kg)

    def test_login_screen_choice_is_saved(self):
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=classic\n', '--desktop=plasma')
        self.assertIn('HEARTH_DESKTOP=plasma', pathlib.Path(self.conf).read_text())
        self.assertEqual(pathlib.Path(self.conf).read_text().count('HEARTH_DESKTOP='), 1)
        # Plasma isn't here: Classic anyway.
        self.assertTrue(self.labwc_ran(), log)

    def test_nimbus_mode_sees_plasma(self):
        self.plasma('ok')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n', extra_env={'HEARTH_SEAT': ''})
        self.assertIn('Hearth Plasma', log)
        # nimbus-mode reports Desktop Mode while Plasma's pid file is live.
        p = subprocess.Popen(['sleep', '5'])
        try:
            with open(os.path.join(self.run_dir, 'hearth-plasma.pid'), 'w') as f:
                f.write(str(p.pid))
            r = subprocess.run(['sh', str(MODE), 'current'], env={'XDG_RUNTIME_DIR': self.run_dir, 'PATH': '/usr/bin:/bin'},
                               capture_output=True, text=True)
            self.assertEqual(r.stdout.strip(), 'desktop')
        finally:
            p.send_signal(signal.SIGKILL)
            p.wait()


COMPANION = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-companion'


def wait_for(path, seconds=12):
    """True once the file exists (the companion runs in the background, after the session's own steps)."""
    import time
    end = time.time() + seconds
    while time.time() < end:
        if os.path.exists(path):
            time.sleep(0.3)
            return True
        time.sleep(0.2)
    return False


class Controller(unittest.TestCase):
    """The controller in Hearth Plasma: on a real device Plasma 6 starts XDG autostart entries through systemd's
    generator, which never sees /usr/share/hearth-plasma/xdg (0.6.59: no controller at all). nimbus-session starts it
    itself once Plasma is up (companion_start → desktop/shell/hearth-companion plasma), once, only in Hearth Plasma,
    and says in pad.log what it decided."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-companion-test-')
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        shell = os.path.join(self.share, 'desktop', 'shell')
        for d in (self.home, self.run_dir, self.bin, shell):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        shutil.copy(COMPANION, shell)
        for f in ('osk-plasma.qml', 'hearth-pad'):
            pathlib.Path(shell, f).write_text('# stand-in\n')
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        os.makedirs(os.path.dirname(self.conf))
        self.write_bin('labwc', '#!/bin/sh\ntouch "$HOME/labwc-ran"\nexit 0\n')
        # quickshell stand-in: says how it was started, then stays a while (as the real one does).
        self.write_bin('quickshell', '#!/bin/sh\necho "ARGS=$* HEARTH_DESKTOP=$HEARTH_DESKTOP WAYLAND_DISPLAY=$WAYLAND_DISPLAY '
                                     'CUR=$XDG_CURRENT_DESKTOP" >> "$HOME/quickshell-ran"\nexec sleep 8\n')
        self.pad_log = os.path.join(self.home, '.local', 'share', 'nimbus', 'pad.log')

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.tmp], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def env(self, **kw):
        e = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
             'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent', 'HEARTH_PLASMA_TIMEOUT': '20',
             'HEARTH_COMPANION_DELAY': '0'}
        e.update(kw)
        return e

    def companion(self, *args, **env):
        return subprocess.run(['sh', str(COMPANION)] + list(args), env=self.env(**env), capture_output=True, text=True, timeout=30)

    def pad_text(self):
        return pathlib.Path(self.pad_log).read_text() if os.path.exists(self.pad_log) else ''

    def test_session_starts_it_once_plasma_is_up(self):
        # Plasma's shell has the desktop's Wayland display in its environment, as on a device.
        self.write_bin('startplasma-wayland', '#!/bin/sh\nexport WAYLAND_DISPLAY=wayland-hearth-test\n'
                                              '"$(dirname "$0")/kwin_wayland" & "$(dirname "$0")/plasmashell" &\nsleep 5\nexit 0\n')
        self.write_bin('kwin_wayland', '#!/bin/sh\nsleep 9\n')
        self.write_bin('plasmashell', '#!/bin/sh\nsleep 9\n')
        with open(self.conf, 'w') as f:
            f.write('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n')
        subprocess.run(['sh', str(SESSION)], env=self.env(), capture_output=True, text=True, timeout=60)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log').read_text()
        self.assertIn('plasma: up; starting the controller', log)
        ran = os.path.join(self.home, 'quickshell-ran')
        self.assertTrue(wait_for(ran), log + self.pad_text())
        text = pathlib.Path(ran).read_text()
        self.assertEqual(text.count('ARGS='), 1, text)
        self.assertIn('ARGS=-p %s/desktop/shell/osk-plasma.qml' % self.share, text)
        self.assertIn('HEARTH_DESKTOP=plasma', text)
        self.assertIn('WAYLAND_DISPLAY=wayland-hearth-test', text)
        self.assertIn('CUR=KDE', text)
        self.assertIn('Hearth Plasma controller (session): starting quickshell osk-plasma.qml', self.pad_text())

    def test_only_in_hearth_plasma(self):
        r = self.companion('plasma', 'test', HEARTH_DESKTOP='')
        self.assertEqual(r.returncode, 0)
        self.assertIn("not started: this isn't Hearth Plasma", self.pad_text())
        self.assertFalse(os.path.exists(os.path.join(self.home, 'quickshell-ran')))

    def test_not_without_kwin(self):
        r = self.companion('plasma', 'test', HEARTH_DESKTOP='plasma')
        self.assertEqual(r.returncode, 0)
        self.assertIn("not started: kwin_wayland isn't running", self.pad_text())

    def test_once(self):
        self.write_bin('kwin_wayland', '#!/bin/sh\nsleep 12\n')
        kwin = subprocess.Popen([os.path.join(self.bin, 'kwin_wayland')])
        try:
            first = subprocess.Popen(['sh', str(COMPANION), 'plasma', 'first'], env=self.env(HEARTH_DESKTOP='plasma'),
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            self.assertTrue(wait_for(os.path.join(self.home, 'quickshell-ran')), self.pad_text())
            r = self.companion('plasma', 'second', HEARTH_DESKTOP='plasma')
            self.assertEqual(r.returncode, 0)
            self.assertIn('Hearth Plasma controller (second): already running', self.pad_text())
            self.assertEqual(pathlib.Path(self.home, 'quickshell-ran').read_text().count('ARGS='), 1)
            first.kill()
            first.wait()
        finally:
            kwin.kill()
            kwin.wait()

    def test_no_quickshell_says_so(self):
        if shutil.which('quickshell', path='/usr/bin:/bin'):
            self.skipTest('quickshell is installed on this machine')
        os.remove(os.path.join(self.bin, 'quickshell'))
        self.write_bin('kwin_wayland', '#!/bin/sh\nsleep 6\n')
        kwin = subprocess.Popen([os.path.join(self.bin, 'kwin_wayland')])
        try:
            self.companion('plasma', 'test', HEARTH_DESKTOP='plasma')
            self.assertIn("not started: quickshell isn't installed", self.pad_text())
        finally:
            kwin.kill()
            kwin.wait()

    def test_start_paths_dont_rely_on_xdg_autostart(self):
        # What a real (systemd-started) session does: nimbus-session starts each desktop's controller itself once the
        # desktop is up, and stops it before the next mode; the old autostart entries only call hearth-companion (which
        # runs it once), guarded by HEARTH_DESKTOP.
        text = SESSION.read_text()
        self.assertIn('COMPANION="$SHARE/desktop/shell/hearth-companion"', text)
        for desk, fn in (('plasma', 'run_plasma'), ('gnome', 'run_gnome'), ('cosmic', 'run_cosmic')):
            body = text.split(fn + '() {', 1)[1].split('\n}\n', 1)[0]
            self.assertIn('companion_start %s ' % desk, body, fn)
            self.assertIn('if [ "$up" = 1 ]; then', body.split('companion_start %s ' % desk)[0].rsplit('\n', 1)[-1], fn)
        for fn in ('settle_plasma', 'settle_gnome', 'settle_cosmic', 'settle'):
            self.assertIn('companion_stop', text.split('\n' + fn + '() {', 1)[1].split('\n}\n', 1)[0], fn)
        for d, f, desk in (('hearth-plasma', 'hearth-osk.desktop', 'plasma'), ('hearth-cosmic', 'hearth-cosmic.desktop', 'cosmic')):
            auto = (SYSTEM_FILES / d / 'xdg' / 'autostart' / f).read_text()
            self.assertIn('[ "$HEARTH_DESKTOP" = %s ]' % desk, auto)
            self.assertIn('/desktop/shell/hearth-companion"', auto)
            self.assertIn('exec sh "$c" %s autostart' % desk, auto)
            self.assertNotIn('quickshell -p', auto)
        comp = COMPANION.read_text()
        self.assertIn('flock -n 9', comp)
        self.assertIn('"$@" 9>&-', comp)
        self.assertTrue(os.access(COMPANION, os.X_OK))


class Helper(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-plasma-helper-')
        os.environ['HOME'] = self.tmp
        os.environ['XDG_CONFIG_HOME'] = os.path.join(self.tmp, 'cfg')
        os.environ['XDG_RUNTIME_DIR'] = self.tmp

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_desktop_line(self):
        m = self.m
        self.assertEqual(m.read_desktop(''), 'classic')
        self.assertEqual(m.read_desktop('A=1\nexport HEARTH_DESKTOP="plasma"\n'), 'plasma')
        self.assertEqual(m.read_desktop('HEARTH_DESKTOP=gnome\n'), 'gnome')
        self.assertEqual(m.read_desktop('HEARTH_DESKTOP=kde\n'), 'classic')
        self.assertEqual(m.write_desktop('A=1', 'plasma'), 'A=1\nHEARTH_DESKTOP=plasma\n')
        self.assertEqual(m.write_desktop('HEARTH_DESKTOP=plasma\nB=2\n', 'classic'), 'HEARTH_DESKTOP=classic\nB=2\n')

    def test_same_rules_as_settings_in_rust(self):
        rs = ADMIN_RS.read_text()
        self.assertIn('pub fn read_desktop_choice', rs)
        self.assertIn('"/usr/share/plasma/look-and-feel/org.hearth.desktop"', rs)
        self.assertEqual(self.m.PLASMA_LNF, '/usr/share/plasma/look-and-feel/org.hearth.desktop')

    def test_desktop_set(self):
        m = self.m
        r = m.desktop_set('classic')
        self.assertEqual(r['desktop'], 'classic')
        with self.assertRaises(m.Fail):
            m.desktop_set('kde')
        if not m.gnome_installed():
            with self.assertRaises(m.Fail):
                m.desktop_set('gnome')
        if not m.plasma_installed():
            with self.assertRaises(m.Fail):
                m.desktop_set('plasma')
        open(os.path.join(self.tmp, 'hearth-plasma-failed'), 'w').close()
        self.assertTrue(m.desktop()['fellBack'])

    def test_parsers(self):
        m = self.m
        self.assertEqual(m.parse_wpctl('Volume: 0.45'), (45, False))
        self.assertEqual(m.parse_wpctl('Volume: 1.20 [MUTED]'), (120, True))
        self.assertEqual(m.parse_wpctl(''), (None, False))
        self.assertEqual(m.parse_brightness('intel_backlight,backlight,400,40%,1000\n'), 40)
        self.assertIsNone(m.parse_brightness(''))

    def test_plasma_visibility(self):
        m = self.m
        self.assertTrue(m.shown_in_plasma({}))
        self.assertFalse(m.shown_in_plasma({'OnlyShowIn': 'Nimbus;'}))
        self.assertTrue(m.shown_in_plasma({'OnlyShowIn': 'KDE;Nimbus;'}))
        self.assertFalse(m.shown_in_plasma({'NotShowIn': 'KDE;'}))
        self.assertFalse(m.shown_in_plasma({'NoDisplay': 'true'}))

    def test_quick_set_checks(self):
        m = self.m
        for k, v in (('mute', 'maybe'), ('wifi', 'x'), ('profile', 'turbo'), ('nope', '1')):
            with self.assertRaises(m.Fail):
                m.quick_set(k, v)

    def test_apps_split_like_mantel(self):
        data = os.path.join(self.tmp, 'data')
        apps = os.path.join(data, 'applications')
        os.makedirs(apps)
        def entry(name, body):
            with open(os.path.join(apps, name), 'w') as f:
                f.write('[Desktop Entry]\nType=Application\n' + body)
        entry('hearth-text.desktop', 'Name=Text\nExec=true\n')
        entry('mine.desktop', 'Name=Mine\nExec=true\n')
        entry('only-mantel.desktop', 'Name=Mantel only\nExec=true\nOnlyShowIn=Nimbus;\n')
        os.environ['XDG_DATA_HOME'] = data
        os.environ['XDG_DATA_DIRS'] = os.path.join(self.tmp, 'none')
        got = {a['id']: a for a in self.m.apps()}
        self.assertIn('hearth-text', got)
        self.assertIn('mine', got)
        self.assertNotIn('only-mantel', got)
        self.assertFalse(got['mine']['system'])   # not under /usr/share/applications: yours


class ThemeFiles(unittest.TestCase):
    def test_look_and_feel_package(self):
        meta = json.loads((LNF / 'metadata.json').read_text())
        self.assertEqual(meta['KPlugin']['Id'], 'org.hearth.desktop')
        self.assertEqual(meta['KPackageStructure'], 'Plasma/LookAndFeel')
        self.assertTrue((LNF / 'contents' / 'defaults').is_file())
        layout = (LNF / 'contents' / 'layouts' / 'org.kde.plasma.desktop-layout.js').read_text()
        for w in ('org.hearth.launcher', 'org.hearth.quicksettings', 'org.hearth.gamemode', 'org.kde.plasma.icontasks'):
            self.assertIn(w, layout)

    def test_plasmoids(self):
        base = SYSTEM_FILES / 'plasma' / 'plasmoids'
        ids = sorted(p.name for p in base.iterdir())
        self.assertEqual(ids, ['org.hearth.gamemode', 'org.hearth.launcher', 'org.hearth.link', 'org.hearth.quicksettings', 'org.hearth.storage'])
        for i in ids:
            meta = json.loads((base / i / 'metadata.json').read_text())
            self.assertEqual(meta['KPlugin']['Id'], i)
            self.assertEqual(meta['KPackageStructure'], 'Plasma/Applet')
            self.assertTrue((base / i / 'contents' / 'ui' / 'main.qml').is_file())
        # The tray ones: Hearth Link and Hearth Storage.
        for i in ('org.hearth.link', 'org.hearth.storage'):
            self.assertEqual(json.loads((base / i / 'metadata.json').read_text()).get('X-Plasma-NotificationArea'), 'true')

    def test_defaults_point_at_hearth(self):
        kg = (SYSTEM_FILES / 'hearth-plasma' / 'xdg' / 'kdeglobals').read_text()
        self.assertIn('LookAndFeelPackage=org.hearth.desktop', kg)
        self.assertIn('ColorScheme=Hearth', kg)
        self.assertIn('name=hearth', (SYSTEM_FILES / 'hearth-plasma' / 'xdg' / 'plasmarc').read_text())
        # The colours themselves are in kdeglobals (KDE reads them there), equal to Hearth.colors.
        colors = (ROOT / 'hosts' / 'linux' / 'desktop' / 'theme' / 'Hearth.colors').read_text()
        body = '\n'.join(l for l in colors.splitlines() if not l.startswith('#')).strip()
        self.assertIn(body, kg)


if __name__ == '__main__':
    unittest.main()
