"""Hearth's panel on Plasma (hosts/linux/desktop/plasma-hearth; docs/mockups/checklists/plasma-hearth-bar.md).

The owner's rules for it, checked without Plasma (its scripting is replaced by a stand-in):
  * Hearth's panel is the default only for a new account's first sign-in (the mark /etc/skel gives); an account that
    already had a panel is never switched by itself, and the first sign-in happens once.
  * A Hearth's panel that doesn't say it's up gives way to KDE's own panel, and says so; one that's up stays.
  * Picking it saves your panels and KDE's settings it changes first; undo puts KDE's settings back as they were.
  * The packages are made from Mantel's own files: transform.py takes out only Quickshell's lines.
"""
import importlib.machinery
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
PH = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'plasma-hearth')


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, path)
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class Panel(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = dict(os.environ)
        os.environ.update(HOME=self.tmp, XDG_RUNTIME_DIR=os.path.join(self.tmp, 'run'), XDG_CONFIG_HOME=os.path.join(self.tmp, '.config'),
                          XDG_DATA_HOME=os.path.join(self.tmp, '.local', 'share'))
        os.makedirs(os.path.join(self.tmp, '.local', 'share', 'nimbus'))
        self.h = load(os.path.join(PH, 'hearth-plasma-panel'), 'hearth_plasma_panel_t')
        self.calls = []
        self.h.plasma_js = lambda js, timeout=20: (self.calls.append(js) or '{"ok": true}')
        self.h.remember = self.fake_remember
        self.h.frames = lambda: 0
        self.h.notify = lambda t: self.calls.append('notify:' + t)
        self.settings = {}
        self.h.kread = lambda f, g, k: self.settings.get((f, g, k))
        self.h.kwrite = self.fake_kwrite
        self.h.detach = lambda argv: True
        self.h.lock_ok = lambda: (False, 'not here')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_remember(self, choice):
        p = os.path.join(self.tmp, '.local', 'share', 'nimbus', 'desktop.json')
        d = json.load(open(p)) if os.path.exists(p) else {}
        d['plasmaPanel'] = choice
        json.dump(d, open(p, 'w'))

    def fake_kwrite(self, f, g, k, v):
        if v is None:
            self.settings.pop((f, g, k), None)
        else:
            self.settings[(f, g, k)] = v
        return True

    def mark(self):
        p = os.path.join(self.tmp, self.h.NEW_MARK)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        open(p, 'w').write('new')
        return p

    def test_an_existing_account_is_never_switched(self):
        self.assertEqual(self.h.first_login(), 'not new')
        self.assertEqual(self.calls, [], 'nothing may be asked of Plasma for an account that already had a panel')
        self.assertFalse(self.h.in_use())

    def test_a_new_account_gets_it_once(self):
        m = self.mark()
        self.assertEqual(self.h.first_login(), 'applied')
        self.assertFalse(os.path.exists(m), 'the first sign-in happens once')
        self.assertTrue(self.h.in_use())
        self.assertTrue(any('new Panel' in c for c in self.calls))
        n = len(self.calls)
        self.assertEqual(self.h.first_login(), 'not new')
        self.assertEqual(len(self.calls), n)

    def test_a_panel_that_does_not_start_gives_way_to_kdes(self):
        self.mark()
        self.h.first_login()
        self.assertEqual(self.h.guard(wait=1), 'fell back')
        self.assertFalse(self.h.in_use())
        self.assertTrue(any('defaultPanel' in c for c in self.calls), "KDE's own panel must come back")
        self.assertTrue(any(c.startswith('notify:') for c in self.calls), 'a switch Hearth makes by itself must say so')

    def test_a_panel_that_is_up_stays(self):
        self.mark()
        self.h.first_login()
        self.h.mark_ready()
        self.assertEqual(self.h.guard(wait=2), 'up')
        self.assertTrue(self.h.in_use())

    def test_undo_puts_kdes_settings_back(self):
        self.settings[('plasmarc', 'OSD', 'Enabled')] = 'true'
        self.h.apply()
        self.assertEqual(self.settings[('plasmanotifyrc', 'Notifications', 'CriticalInDndMode')], 'false')
        self.assertEqual(self.settings[('plasmarc', 'OSD', 'Enabled')], 'false')
        self.h.undo()
        self.assertEqual(self.settings.get(('plasmarc', 'OSD', 'Enabled')), 'true', 'a value you had comes back')
        self.assertNotIn(('plasmanotifyrc', 'Notifications', 'CriticalInDndMode'), self.settings, 'one you never set is taken away again')

    def test_hearths_lock_screen_only_when_the_locker_shows_it(self):
        self.h.lock_ok = lambda: (False, 'not here')
        self.h.apply()
        self.assertFalse(os.path.exists(self.h.lock_dropin()), "a lock screen the locker can't show is never picked")
        self.h.undo()
        self.h.lock_ok = lambda: (True, 'ok')
        self.h.apply()
        t = open(self.h.lock_dropin()).read()
        self.assertIn('Environment=PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock', t)
        self.assertIn('plasma-kwin_wayland.service.d', self.h.lock_dropin(), 'only KWin (which starts the locker) gets it')
        self.h.undo()
        self.assertFalse(os.path.exists(self.h.lock_dropin()), "KDE's own lock screen comes back")

    def test_picking_it_again_keeps_what_undo_puts_back(self):
        self.settings[('plasmarc', 'OSD', 'Enabled')] = 'true'
        self.h.apply()
        self.h.apply()      # Settings › Bar and dock › Hearth's panel again, while it's in use
        self.h.undo()
        self.assertEqual(self.settings.get(('plasmarc', 'OSD', 'Enabled')), 'true', "KDE's settings from before Hearth's panel, not Hearth's")

    def test_apply_saves_your_panels_first(self):
        cfg = os.path.join(self.tmp, '.config')
        os.makedirs(cfg, exist_ok=True)
        open(os.path.join(cfg, 'plasma-org.kde.plasma.desktop-appletsrc'), 'w').write('[Containments][1]\nplugin=org.kde.panel\n')
        self.h.apply()
        saved = os.listdir(self.h.backup_root())
        self.assertTrue(saved)
        self.assertTrue(os.path.isfile(os.path.join(self.h.backup_root(), saved[0], 'plasma-org.kde.plasma.desktop-appletsrc')))


class NotifydBridge(unittest.TestCase):
    """Mantel's hearth-notifyd on Plasma: the go-between answers a wait with what changed, and passes asks on."""
    def test_changes_come_through_and_asks_go_back(self):
        run = tempfile.mkdtemp(prefix='hpn', dir='/tmp')
        fake = os.path.join(run, 'notifyd')
        with open(fake, 'w') as f:
            f.write('import json, sys\nprint(json.dumps({"t": "disks", "disks": [{"name": "USB STICK"}]}), flush=True)\n'
                    'for line in sys.stdin:\n    print(json.dumps({"t": "done", "ok": True, "asked": json.loads(line)}), flush=True)\n')
        env = dict(os.environ, XDG_RUNTIME_DIR=run, HEARTH_NOTIFYD=fake)
        h = os.path.join(PH, 'hearth-plasma-panel')
        try:
            out = subprocess.run([sys.executable, h, 'notifyd-wait', '0'], env=env, capture_output=True, text=True, timeout=30).stdout
            o = json.loads(out.strip().splitlines()[-1])
            self.assertEqual(o['msgs'][0]['t'], 'disks', out)
            self.assertEqual(o['msgs'][0]['disks'][0]['name'], 'USB STICK')
            r = subprocess.run([sys.executable, h, 'notifyd-send', '{"op": "eject", "path": "/dev/sda1"}'], env=env, capture_output=True, text=True, timeout=30).stdout
            self.assertTrue(json.loads(r.strip().splitlines()[-1])['ok'], r)
            out = subprocess.run([sys.executable, h, 'notifyd-wait', str(o['seq'])], env=env, capture_output=True, text=True, timeout=30).stdout
            m = json.loads(out.strip().splitlines()[-1])['msgs']
            self.assertEqual(m[-1]['asked'], {'op': 'eject', 'path': '/dev/sda1'}, out)
        finally:
            subprocess.run(['pkill', '-f', 'hearth-plasma-panel notifyd-bridge'], capture_output=True)
            shutil.rmtree(run, ignore_errors=True)


class Build(unittest.TestCase):
    def test_packages_are_mantels_own_files(self):
        with tempfile.TemporaryDirectory() as root:
            r = subprocess.run(['bash', os.path.join(PH, 'install.sh'), root], capture_output=True, text=True, timeout=120)
            self.assertIn('plasma-hearth: installed', r.stdout, r.stdout + r.stderr)
            ui = os.path.join(root, 'usr/share/plasma/plasmoids/org.hearthos.panel/contents/ui')
            for pkg in ('org.hearthos.panel', 'org.hearthos.appmenu', 'org.hearthos.quicksettings', 'org.hearthos.notifications'):
                self.assertTrue(os.path.isfile(os.path.join(root, 'usr/share/plasma/plasmoids', pkg, 'metadata.json')), pkg)
            for f in os.listdir(ui):
                if f.endswith('.qml'):
                    t = open(os.path.join(ui, f)).read()
                    self.assertNotRegex(t, r'(?m)^\s*import Quickshell', f)
                    self.assertNotRegex(t, r'(?m)^\s*WlrLayershell\.', f)
            # Islands.qml is Mantel's, line for line, apart from what transform.py takes out.
            mine = open(os.path.join(ui, 'Islands.qml')).read().split('\n')
            src = open(os.path.join(ROOT, 'hosts/linux/desktop/shell/Islands.qml')).read().split('\n')
            self.assertEqual(len(mine), len(src))
            changed = [a for a, b in zip(src, mine) if a != b]
            self.assertTrue(all('Quickshell' in a or 'WlrLayershell' in a or 'anchors {' in a or 'mask:' in a or 'resolvedUrl' in a for a in changed), changed)
            self.assertTrue(os.path.isfile(os.path.join(root, 'etc/skel', self.mark_rel())))
            self.assertTrue(os.path.isfile(os.path.join(root, 'etc/xdg/autostart/hearth-plasma-panel-login.desktop')))
            lock = os.path.join(root, 'usr/share/plasma/shells/org.hearth.mantel-lock')
            self.assertEqual(open(os.path.join(lock, 'contents/lockscreen/LockScreen.qml')).read(),
                             open(os.path.join(ROOT, 'hosts/linux/desktop/kwin-lock/contents/lockscreen/LockScreen.qml')).read(),
                             "the lock screen is Mantel's own")
            self.assertEqual(json.load(open(os.path.join(lock, 'metadata.json')))['KPackageStructure'], 'Plasma/Shell')

    def mark_rel(self):
        return os.path.join('.config', 'hearth', 'plasma-panel-new')


if __name__ == '__main__':
    unittest.main()
