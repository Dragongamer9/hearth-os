"""Tests for Hearth GNOME (docs/GNOME.md): the desktop choice and the fallback in hosts/linux/session/nimbus-session (a
broken or missing GNOME always ends in Hearth Classic, never without a desktop), desktop/gnome/hearth-gnome (what the
Shell extensions ask for), hearth-pad's GNOME mode (--gnome), and Hearth's GNOME files (session mode, extensions,
the per-desktop app overrides). The session runs for real with stand-ins for labwc and GNOME."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
MODE = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-mode'
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'gnome' / 'hearth-gnome'
PLASMA_HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'plasma' / 'hearth-plasma'
PAD = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-pad'
ADMIN_RS = ROOT / 'hosts' / 'hearth-core' / 'src' / 'admin.rs'
SHARE = ROOT / 'os' / 'system_files' / 'usr' / 'share'
EXT = SHARE / 'gnome-shell' / 'extensions'
SHELL_MODE = SHARE / 'gnome-shell' / 'modes' / 'hearth.json'
NAMES = ['theme', 'bar', 'launcher', 'quick', 'link', 'storage', 'gamemode', 'pad']


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


FAKE_GNOME = {
    'crash': '#!/bin/sh\necho "gnome crashed" >&2\nexit 1\n',
    'hang': '#!/bin/sh\nexec sleep 30\n',
    'ok': '#!/bin/sh\n"$(dirname "$0")/gnome-shell" &\n'
          'echo "MODE=$GNOME_SHELL_SESSION_MODE DESKTOP=$XDG_CURRENT_DESKTOP HEARTH_DESKTOP=$HEARTH_DESKTOP DATA=$XDG_DATA_DIRS" > "$HOME/gnome-env"\n'
          'sleep 4\nexit 0\n',
}


class SessionFallback(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-gnome-test-')
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        for d in (self.home, self.run_dir, self.bin, self.share):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        os.makedirs(os.path.dirname(self.conf))
        self.mode_file = os.path.join(self.tmp, 'hearth.json')
        with open(self.mode_file, 'w') as f:
            f.write('{}')
        self.write_bin('labwc', '#!/bin/sh\ntouch "$HOME/labwc-ran"\nexit 0\n')

    def tearDown(self):
        subprocess.run(['pkill', '-KILL', '-f', self.bin], capture_output=True)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_bin(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def gnome(self, kind):
        self.write_bin('gnome-session', FAKE_GNOME[kind])
        self.write_bin('gnome-shell', '#!/bin/sh\nsleep 6\n')

    def session(self, conf, *args, timeout=60, extra_env=None):
        with open(self.conf, 'w') as f:
            f.write(conf)
        env = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'NIMBUS_SHARE': self.share,
               'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_APP': '/nonexistent', 'HEARTH_GNOME_TIMEOUT': '20',
               'HEARTH_GNOME_MODE_FILE': self.mode_file}
        env.update(extra_env or {})
        r = subprocess.run(['sh', str(SESSION)] + list(args), env=env, capture_output=True, text=True, timeout=timeout)
        log = pathlib.Path(self.home, '.local', 'share', 'nimbus', 'session.log')
        return r, log.read_text() if log.exists() else ''

    def labwc_ran(self):
        return os.path.exists(os.path.join(self.home, 'labwc-ran'))

    def test_gnome_crash_falls_back_to_classic(self):
        self.gnome('crash')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=gnome\n')
        self.assertTrue(self.labwc_ran(), log)
        self.assertIn('gnome: didn\'t start; falling back to Hearth Classic', log)
        self.assertTrue(os.path.exists(os.path.join(self.run_dir, 'hearth-gnome-failed')))
        self.assertFalse(os.path.exists(os.path.join(self.run_dir, 'hearth-gnome.pid')))

    def test_gnome_without_a_desktop_in_time_falls_back(self):
        self.gnome('hang')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=gnome\n', extra_env={'HEARTH_GNOME_TIMEOUT': '2'})
        self.assertIn('gnome: no desktop after 2s', log)
        self.assertTrue(self.labwc_ran(), log)

    def test_gnome_missing_uses_classic(self):
        self.gnome('ok')
        os.remove(self.mode_file)            # Hearth's session mode isn't there
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=gnome\n')
        self.assertIn('gnome: not installed', log)
        self.assertTrue(self.labwc_ran(), log)

    def test_gnome_runs_and_ends_normally(self):
        self.gnome('ok')
        _, log = self.session('NIMBUS_START_MODE=desktop\nexport HEARTH_DESKTOP="gnome"\n')
        self.assertIn('desktop mode: Hearth GNOME', log)
        self.assertIn('gnome ended (code 0)', log)
        self.assertFalse(self.labwc_ran(), log)
        env = pathlib.Path(self.home, 'gnome-env').read_text()
        self.assertIn('MODE=hearth ', env)
        self.assertNotIn('/usr/share/hearth-gnome', env)
        self.assertIn('DESKTOP=GNOME', env)
        self.assertIn('HEARTH_DESKTOP=gnome', env)
        self.assertTrue(os.path.exists(os.path.join(self.home, '.config', 'gnome-initial-setup-done')))

    def test_safe_graphics_or_seat_never_gnome(self):
        self.gnome('ok')
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=gnome\n', extra_env={'HEARTH_SEAT': 'seat1'})
        self.assertNotIn('Hearth GNOME', log)

    def test_login_screen_choice_is_saved(self):
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n', '--desktop=gnome')
        text = pathlib.Path(self.conf).read_text()
        self.assertIn('HEARTH_DESKTOP=gnome', text)
        self.assertEqual(text.count('HEARTH_DESKTOP='), 1)
        self.assertTrue(self.labwc_ran(), log)     # GNOME isn't here: Classic anyway

    def test_plasma_unchanged(self):
        # Hearth Plasma's choice still goes to Plasma's code (here: not installed → Classic).
        _, log = self.session('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=plasma\n')
        self.assertNotIn('gnome', log.lower().replace('gnome-keyring', ''))
        self.assertTrue(self.labwc_ran(), log)

    def test_nimbus_mode_sees_gnome(self):
        p = subprocess.Popen(['sleep', '5'])
        try:
            with open(os.path.join(self.run_dir, 'hearth-gnome.pid'), 'w') as f:
                f.write(str(p.pid))
            r = subprocess.run(['sh', str(MODE), 'current'], env={'XDG_RUNTIME_DIR': self.run_dir, 'PATH': '/usr/bin:/bin'},
                               capture_output=True, text=True)
            self.assertEqual(r.stdout.strip(), 'desktop')
        finally:
            p.send_signal(signal.SIGKILL)
            p.wait()


class Helper(unittest.TestCase):
    def setUp(self):
        self.m = load(HELPER, 'hearth_gnome')
        self.p = load(PLASMA_HELPER, 'hearth_plasma_for_gnome')
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-gnome-helper-')
        os.environ['HOME'] = self.tmp
        os.environ['XDG_CONFIG_HOME'] = os.path.join(self.tmp, 'cfg')
        os.environ['XDG_RUNTIME_DIR'] = self.tmp

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_gnome_visibility(self):
        s = self.m.shown_in_gnome
        self.assertTrue(s({}))
        self.assertTrue(s({'OnlyShowIn': 'GNOME;'}))
        self.assertFalse(s({'OnlyShowIn': 'Nimbus;'}))
        self.assertFalse(s({'OnlyShowIn': 'KDE;'}))
        self.assertFalse(s({'NotShowIn': 'GNOME;'}))
        self.assertFalse(s({'Hidden': 'true'}))
        self.assertFalse(s({'NoDisplay': 'true'}))

    def test_desktop_choice_knows_gnome(self):
        p = self.p
        self.assertEqual(p.read_desktop('HEARTH_DESKTOP=gnome\n'), 'gnome')
        self.assertEqual(p.read_desktop('HEARTH_DESKTOP="gnome"\nHEARTH_DESKTOP=x\n'), 'classic')
        self.assertIn('gnome', p.DESKTOPS)
        self.assertEqual(p.GNOME_MODE, '/usr/share/gnome-shell/modes/hearth.json')
        d = p.desktop()
        self.assertIn('gnome', d)
        self.assertEqual(d['fellBackFrom'], '')
        open(os.path.join(self.tmp, 'hearth-gnome-failed'), 'w').close()
        d = p.desktop()
        self.assertTrue(d['fellBack'])
        self.assertEqual(d['fellBackFrom'], 'gnome')
        if not p.gnome_installed():
            with self.assertRaises(p.Fail):
                p.desktop_set('gnome')

    def test_apps_through_gnome(self):
        data = os.path.join(self.tmp, 'data')
        apps = os.path.join(data, 'applications')
        os.makedirs(apps)
        def entry(name, body):
            with open(os.path.join(apps, name), 'w') as f:
                f.write('[Desktop Entry]\nType=Application\n' + body)
        entry('hearth-paint.desktop', 'Name=Paint\nExec=true\n')
        entry('org.gnome.Nautilus.desktop', 'Name=Files\nExec=true\nOnlyShowIn=GNOME;\n')
        entry('only-mantel.desktop', 'Name=Mantel only\nExec=true\nOnlyShowIn=Nimbus;\n')
        entry('not-gnome.desktop', 'Name=Not here\nExec=true\nNotShowIn=GNOME;\n')
        os.environ['XDG_DATA_HOME'] = data
        os.environ['XDG_DATA_DIRS'] = os.path.join(self.tmp, 'none')
        r = subprocess.run([sys.executable, str(HELPER), 'apps'], capture_output=True, text=True, env=dict(os.environ))
        got = {a['id'] for a in json.loads(r.stdout.strip().splitlines()[-1])}
        self.assertEqual(got, {'hearth-paint', 'org.gnome.Nautilus'})
        r = subprocess.run([sys.executable, str(HELPER), 'desktop-set', 'kde'], capture_output=True, text=True, env=dict(os.environ))
        self.assertTrue(r.stdout.startswith('error: '))
        self.assertEqual(r.returncode, 1)

    def test_same_rules_as_settings_in_rust(self):
        rs = ADMIN_RS.read_text()
        self.assertIn('pub fn gnome_installed', rs)
        self.assertIn('"/usr/share/gnome-shell/modes/hearth.json"', rs)
        self.assertIn('"gnome" => "gnome"', rs)


class Out:
    def __init__(self):
        self.log = []
        self.sent = []
        self.wheeled = [0, 0]

    def move(self, dx, dy):
        pass

    def wheel(self, v, h):
        self.wheeled[0] += v
        self.wheeled[1] += h

    def press(self, name, down):
        self.log.append((name, down))

    def send(self, obj):
        self.sent.append(obj)

    def rumble(self, strength, ms):
        pass


class PadGnome(unittest.TestCase):
    """hearth-pad --gnome."""

    def setUp(self):
        self.hp = load(PAD, 'hearth_pad_gnome')
        self.out = Out()
        self.pad = self.hp.Pad(self.out, gnome=True)

    def press(self, *names, t=0.0, hold=0.05):
        for n in names:
            self.pad.button(n, True, t)
        for n in reversed(names):
            self.pad.button(n, False, t + hold)

    def downs(self):
        return [k for k, d in self.out.log if d]

    def test_desk(self):
        self.press('a')
        self.assertIn(('BTN_LEFT', True), self.out.log)
        self.press('rt')
        self.press('menu')
        self.assertIn({'t': 'launcher'}, self.out.sent)
        self.press('view')
        self.assertIn({'t': 'overview'}, self.out.sent)
        self.press('view', 'b')
        self.assertIn({'t': 'close'}, self.out.sent)
        self.press('view', 'menu')
        self.assertIn({'t': 'barfocus'}, self.out.sent)
        self.press('guide')
        self.assertEqual(self.pad.mode, 'radial')

    def test_guide_x_keyboard(self):
        self.pad.button('guide', True, 0)
        self.pad.button('x', True, 0.02)
        self.assertIn({'t': 'osk', 'open': True}, self.out.sent)

    def test_shell_has_the_keyboard(self):
        self.pad.command({'cmd': 'keys', 'on': True})
        self.press('down')
        self.press('a')
        self.press('b')
        self.press('rb')
        d = self.downs()
        self.assertIn('KEY_DOWN', d)
        self.assertIn('KEY_ENTER', d)
        self.assertIn('KEY_ESC', d)
        self.assertIn('KEY_TAB', d)
        self.assertNotIn('KEY_LEFTALT', d)
        self.press('rt')             # the triggers still click
        self.assertIn(('BTN_LEFT', True), self.out.log)

    def test_files_keys(self):
        self.pad.command({'cmd': 'app', 'kind': 'files'})
        self.press('a')
        self.press('b')
        self.press('x')
        self.press('y')
        self.press('rb')
        d = self.downs()
        self.assertIn('KEY_ENTER', d)
        self.assertIn(('KEY_LEFTALT', True), self.out.log)
        self.assertIn('KEY_LEFT', d)
        self.assertIn('KEY_COMPOSE', d)
        self.assertIn('KEY_F', d)
        self.assertIn({'t': 'osk', 'open': True}, self.out.sent)
        self.assertIn('KEY_PAGEDOWN', d)
        self.assertNotIn('BTN_LEFT', d)
        # RT held scrolls down.
        self.pad.button('rt', True, 0)
        for i in range(10):
            self.pad.tick(0.1 * i, 0.05)
        self.assertLess(self.out.wheeled[0], 0)

    def test_terminal_and_text(self):
        for kind in ('terminal', 'text'):
            self.out.log.clear()
            self.out.sent.clear()
            self.pad.command({'cmd': 'app', 'kind': kind})
            self.press('b')
            self.press('y')
            self.assertIn('KEY_ESC', self.downs())
            self.assertIn({'t': 'osk', 'open': True}, self.out.sent)
        self.pad.command({'cmd': 'app', 'kind': 'nonsense'})
        self.assertEqual(self.pad.app, '')

    def test_other_modes_unchanged(self):
        out = Out()
        pad = self.hp.Pad(out)
        pad.command({'cmd': 'app', 'kind': 'files'})    # ignored outside GNOME
        self.assertFalse(hasattr(pad, 'app') and pad.app)
        pad.button('menu', True, 0)
        self.assertIn({'t': 'launcher'}, out.sent)
        self.assertIn('KEY_PAGEUP', self.hp.KEYS_USED)


class Files(unittest.TestCase):
    def test_session_mode(self):
        m = json.loads(SHELL_MODE.read_text())
        self.assertEqual(m['parentMode'], 'user')
        self.assertEqual(m['enabledExtensions'], ['hearth-%s@hearthos.fun' % n for n in NAMES])
        self.assertIn('dateMenu', m['panel']['right'])
        self.assertFalse(m['showWelcomeDialog'])

    def test_extensions(self):
        dirs = sorted(p.name for p in EXT.iterdir())
        self.assertEqual(dirs, sorted('hearth-%s@hearthos.fun' % n for n in NAMES))
        for d in dirs:
            meta = json.loads((EXT / d / 'metadata.json').read_text())
            self.assertEqual(meta['uuid'], d)
            for v in ('48', '49', '50'):          # trixie's GNOME, Fedora 43 and 44's
                self.assertIn(v, meta['shell-version'], d)
            js = (EXT / d / 'extension.js').read_text()
            self.assertIn('export default class', js)
            self.assertIn('disable()', js)
            # Only GNOME's own modules, GI, the extension's own files and Hearth's shared helper.
            for spec in re.findall(r"from '([^']+)'", js):
                self.assertTrue(spec.startswith(('gi://', 'resource:///org/gnome/shell/', './', 'file:///usr/share/nimbus/desktop/gnome/')), spec)
                if spec.startswith('file:///usr/share/nimbus/desktop/'):
                    self.assertTrue((ROOT / 'hosts' / 'linux' / 'desktop' / spec[len('file:///usr/share/nimbus/desktop/'):]).is_file(), spec)
                if spec.startswith('./'):
                    self.assertTrue((EXT / d / spec[2:]).is_file(), spec)

    def test_no_downloads(self):
        # In-house only: nothing fetched from the network at run time.
        for p in list(EXT.rglob('*.js')) + [ROOT / 'hosts' / 'linux' / 'desktop' / 'gnome' / 'hearth.js']:
            text = p.read_text()
            self.assertNotIn('Soup', text, p)
            self.assertNotIn('https://', text.replace("'https://hearthos.fun'", ''), p)

    def test_keyboard_uses_mantels_engine(self):
        kb = (EXT / 'hearth-pad@hearthos.fun' / 'keyboard.js').read_text()
        self.assertIn('hearth-osk.js', kb)
        self.assertTrue((ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'osk' / 'hearth-osk.js').is_file())
        hearth_js = (ROOT / 'hosts' / 'linux' / 'desktop' / 'gnome' / 'hearth.js').read_text()
        self.assertIn('/desktop/shell/osk', hearth_js)
        self.assertIn('/desktop/shell/hearth-pad', hearth_js)

    def test_one_app_of_each_kind(self):
        # GNOME's own Files, Console and Text Editor are Hearth GNOME's in the app-kinds table (docs/APP-KINDS.md).
        t = json.loads((SHARE / 'hearth' / 'app-kinds.json').read_text())
        self.assertEqual(t['desktops']['GNOME']['names'], ['GNOME'])          # what run_gnome sets
        self.assertIn('/usr/bin/gnome-shell', t['desktops']['GNOME']['session'])
        for kind, app in (('files', 'org.gnome.Nautilus.desktop'), ('terminal', 'org.gnome.Console.desktop'),
                          ('text-editor', 'org.gnome.TextEditor.desktop')):
            self.assertEqual(t['kinds'][kind]['apps']['GNOME'], [app])
            self.assertIn('GNOME', t['kinds'][kind]['required'])
        self.assertIn('XDG_CURRENT_DESKTOP=GNOME', SESSION.read_text())

    def test_login_screen_session(self):
        text = (ROOT / 'hosts' / 'linux' / 'session' / 'hearth-gnome.desktop').read_text()
        self.assertIn('Exec=/usr/local/bin/nimbus-session --desktop=gnome', text)
        self.assertIn('TryExec=gnome-shell', text)
        inst = (ROOT / 'hosts' / 'linux' / 'session' / 'install-session.sh').read_text()
        self.assertIn('hearth-gnome.desktop', inst)
        for b in ('anatase', 'peppermint'):
            self.assertIn('hearth-classic hearth-plasma hearth-gnome', (ROOT / 'os-bases' / b / 'build.sh').read_text())

    def test_selector_enabled(self):
        self.assertNotIn('coming later', (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'SetSignin.qml').read_text())
        self.assertNotIn('Hearth GNOME (coming later)', (ROOT / 'ui' / 'index.html').read_text())
        self.assertNotIn('coming later', (SHARE / 'sddm' / 'themes' / 'hearth' / 'Main.qml').read_text())


if __name__ == '__main__':
    unittest.main()
