"""1.13 (docs/DESKTOPS.md): KDE Plasma and GNOME downloaded when picked. The root helper (hearth-desktops-helper) with
stand-ins for apt, systemd-run and hearth-unlock: progress from apt's Status-Fd, cancel only while downloading, what
it added recorded for "each desktop's own apps", the switch saved with a .bak and an unreadable file kept aside. The
person's helper (hearth-desktops): pick only what's downloaded, picking again clears the safe start's count, the
login screen's pick, the tray's values, and nothing changes by itself."""
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
HELPER = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-desktops-helper'
USER = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-desktops'
APP_KINDS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'hearth-app-kinds'

FAKE_APT = r'''#!/bin/sh
# apt-get stand-in: notes its arguments, reports progress on the Status-Fd, "installs" KDE on a real install.
fd=""; prev=""
for a in "$@"; do case "$prev" in -o) case "$a" in APT::Status-Fd=*) fd=${a#APT::Status-Fd=} ;; esac ;; esac; prev="$a"; done
echo "$*" >> "$T/apt-calls"
[ -f "$T/apt-fail" ] && { echo "E: Unable to locate package nothing"; exit 100; }
case " $* " in
  *" update "*) [ -n "$fd" ] && eval "echo 'dlstatus:1:50:Lists' >&$fd" ;;
  *" --download-only "*) [ -n "$fd" ] && eval "echo 'dlstatus:1:40:Downloading kwin' >&$fd; echo 'dlstatus:2:100:Done' >&$fd" ;;
  *" install "*) [ -n "$fd" ] && eval "echo 'pmstatus:kwin:50:Installing kwin' >&$fd"
     mkdir -p "$T/root/usr/bin" "$T/apps"
     : > "$T/root/usr/bin/startplasma-wayland"; : > "$T/root/usr/bin/plasmashell"
     printf '[Desktop Entry]\nType=Application\nName=Dolphin\nExec=dolphin\n' > "$T/apps/org.kde.dolphin.desktop" ;;
  *" remove "*) rm -f "$T/root/usr/bin/startplasma-wayland" "$T/root/usr/bin/plasmashell" "$T/apps/org.kde.dolphin.desktop" ;;
esac
exit 0
'''


class Base(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp(prefix='hearth-desktops-')
        self.bin = os.path.join(self.t, 'bin')
        os.makedirs(self.bin)
        os.makedirs(os.path.join(self.t, 'apps'))
        self.fake('apt-get', FAKE_APT)
        self.fake('hearth-unlock', '#!/bin/sh\necho \'{"state": "unlocked", "writable": true, "reapply": {}}\'\n')
        self.fake('systemd-run', '#!/bin/sh\necho "$*" >> "$T/systemd-run-calls"\n')
        self.fake('systemctl', '#!/bin/sh\necho "$*" >> "$T/systemctl-calls"\n')
        self.fake('app-kinds', '#!/bin/sh\necho "$*" >> "$T/app-kinds-calls"\n')
        with open(os.path.join(self.t, 'version'), 'w') as f:
            f.write('1.13\n')
        self.env = dict(os.environ, T=self.t, HEARTH_DESKTOPS_TEST='1', HEARTH_DESKTOPS_RUN=os.path.join(self.t, 'run'),
                        HEARTH_DESKTOPS_STATE=os.path.join(self.t, 'state'), HEARTH_DESKTOPS_CONF=os.path.join(self.t, 'etc', 'desktops.json'),
                        HEARTH_DESKTOPS_APPS=os.path.join(self.t, 'apps'), HEARTH_DESKTOPS_UNLOCK=os.path.join(self.bin, 'hearth-unlock'),
                        HEARTH_DESKTOPS_APP_KINDS=os.path.join(self.bin, 'app-kinds'), HEARTH_DESKTOPS_APT=os.path.join(self.bin, 'apt-get'),
                        HEARTH_DESKTOPS_SYSTEMD_RUN=os.path.join(self.bin, 'systemd-run'), HEARTH_DESKTOPS_SYSTEMCTL=os.path.join(self.bin, 'systemctl'),
                        HEARTH_DESKTOPS_VERSION=os.path.join(self.t, 'version'), HEARTH_DESKTOPS_ROOT=os.path.join(self.t, 'root'))

    def tearDown(self):
        shutil.rmtree(self.t, ignore_errors=True)

    def fake(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def root(self, *args):
        r = subprocess.run([sys.executable, str(HELPER), *args], capture_output=True, text=True, env=self.env, timeout=60)
        line = (r.stdout.strip().splitlines() or [''])[-1]
        return r.returncode, (json.loads(line) if line.startswith('{') else line)

    def read(self, *p):
        with open(os.path.join(self.t, *p)) as f:
            return f.read()


class RootHelper(Base):
    def test_nothing_until_asked(self):
        rc, s = self.root('status')
        self.assertEqual(rc, 0)
        self.assertEqual(s['desktops']['kde']['state'], 'none')
        self.assertEqual(s['desktops']['gnome']['state'], 'none')
        self.assertTrue(s['ownApps'])
        self.assertEqual(s['version'], '1.13')
        self.assertFalse(os.path.exists(os.path.join(self.t, 'apt-calls')))

    def test_install_runs_in_the_background_unit(self):
        rc, s = self.root('install', 'kde')
        self.assertEqual(rc, 0, s)
        call = self.read('systemd-run-calls')
        self.assertIn('--unit hearth-desktop-kde', call)
        self.assertIn('run kde', call)

    def test_run_downloads_then_installs_and_records_its_apps(self):
        rc, out = self.root('run', 'kde')
        self.assertEqual(rc, 0, out)
        calls = self.read('apt-calls').splitlines()
        self.assertIn('update', calls[0])
        self.assertIn('--download-only', calls[1])
        self.assertIn('kde-plasma-desktop', calls[2])
        self.assertNotIn('--download-only', calls[2])
        for c in calls:
            self.assertNotIn('sddm-', c)                     # Hearth's login screen is never removed
        rec = json.loads(self.read('state', 'kde.json'))
        self.assertTrue(rec['wanted'])
        self.assertEqual(rec['apps'], ['org.kde.dolphin.desktop'])
        self.assertEqual(rec['version'], '1.13')
        self.assertIn('apply --refresh', self.read('app-kinds-calls'))
        rc, s = self.root('status')
        self.assertEqual(s['desktops']['kde']['state'], 'downloaded')
        prog = json.loads(self.read('run', 'kde.json'))
        self.assertEqual(prog['state'], 'done')

    def test_gnome_never_brings_gdm(self):
        self.root('run', 'gnome')
        self.assertIn('gdm3-', self.read('apt-calls').splitlines()[-1])

    def test_a_failed_download_says_why_and_changes_nothing(self):
        open(os.path.join(self.t, 'apt-fail'), 'w').close()
        rc, out = self.root('run', 'kde')
        self.assertEqual(rc, 1)
        self.assertIn('Unable to locate', out)
        rc, s = self.root('status')
        self.assertEqual(s['desktops']['kde']['state'], 'none')
        self.assertIn('Unable to locate', s['desktops']['kde']['error'])
        self.assertFalse(os.path.exists(os.path.join(self.t, 'state', 'kde.json')))

    def test_locked_system_is_refused(self):
        self.fake('hearth-unlock', '#!/bin/sh\necho \'{"state": "locked", "writable": false}\'\n')
        rc, out = self.root('install', 'gnome')
        self.assertEqual(rc, 1)
        self.assertIn('isn\'t writable', out)
        self.assertFalse(os.path.exists(os.path.join(self.t, 'systemd-run-calls')))

    def test_cancel_only_while_downloading(self):
        os.makedirs(os.path.join(self.t, 'run'))
        p = subprocess.Popen(['sleep', '30'])
        try:
            with open(os.path.join(self.t, 'run', 'kde.json'), 'w') as f:
                json.dump({'state': 'installing', 'pid': p.pid, 'percent': 90}, f)
            rc, out = self.root('cancel', 'kde')
            self.assertEqual(rc, 1)
            self.assertIn('finishes by itself', out)
            self.assertFalse(os.path.exists(os.path.join(self.t, 'systemctl-calls')))
            with open(os.path.join(self.t, 'run', 'kde.json'), 'w') as f:
                json.dump({'state': 'downloading', 'pid': p.pid, 'percent': 40}, f)
            subprocess.Popen(['sh', '-c', 'sleep 1; kill %d' % p.pid])
            rc, s = self.root('cancel', 'kde')
            self.assertEqual(rc, 0, s)
            self.assertIn('kill --signal=SIGTERM hearth-desktop-kde.service', self.read('systemctl-calls'))
        finally:
            p.kill()
            p.wait()

    def test_remove(self):
        self.root('run', 'kde')
        rc, s = self.root('remove', 'kde')
        self.assertEqual(rc, 0, s)
        self.assertIn('remove --autoremove kde-plasma-desktop', self.read('apt-calls'))
        self.assertEqual(s['desktops']['kde']['state'], 'none')
        self.assertFalse(json.loads(self.read('state', 'kde.json'))['wanted'])

    def test_own_apps_switch_is_kept_safely(self):
        rc, s = self.root('own-apps', 'off')
        self.assertFalse(s['ownApps'])
        rc, s = self.root('own-apps', 'on')
        self.assertTrue(s['ownApps'])
        self.assertTrue(os.path.exists(os.path.join(self.t, 'etc', 'desktops.json.bak')))
        with open(os.path.join(self.t, 'etc', 'desktops.json'), 'w') as f:
            f.write('{not json')
        rc, s = self.root('status')
        self.assertTrue(s['ownApps'])                               # unreadable: the default, file untouched
        self.assertEqual(self.read('etc', 'desktops.json'), '{not json')
        self.root('own-apps', 'off')
        kept = [n for n in os.listdir(os.path.join(self.t, 'etc')) if '.unreadable-' in n]
        self.assertEqual(len(kept), 1)                              # kept aside, never just written over

    def test_unknown_desktop_refused(self):
        rc, out = self.root('install', 'deepin')
        self.assertEqual(rc, 1)


class PersonHelper(Base):
    def setUp(self):
        super().setUp()
        self.home = os.path.join(self.t, 'home')
        self.run_dir = os.path.join(self.t, 'xdg-run')
        os.makedirs(os.path.join(self.home, '.config', 'nimbus'))
        os.makedirs(self.run_dir)
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')
        with open(self.conf, 'w') as f:
            f.write('NIMBUS_START_MODE=game\n# mine\n')
        self.fake('pkexec', '#!/bin/sh\nexec "$@"\n')
        self.fake('hearth-desktops-helper', '#!/bin/sh\nexec %s %s "$@"\n' % (sys.executable, HELPER))
        self.env.update(HOME=self.home, XDG_CONFIG_HOME=os.path.join(self.home, '.config'),
                        XDG_STATE_HOME=os.path.join(self.home, '.local', 'state'), XDG_RUNTIME_DIR=self.run_dir,
                        HEARTH_DESKTOPS_HELPER=os.path.join(self.bin, 'hearth-desktops-helper'),
                        HEARTH_DESKTOPS_PKEXEC=os.path.join(self.bin, 'pkexec'), HEARTH_DESKTOPS_ADMIN=os.path.join(self.bin, 'nothing'),
                        HEARTH_DESKTOPS_APP_KINDS=str(APP_KINDS), XDG_CURRENT_DESKTOP='Nimbus')

    def me(self, *args):
        r = subprocess.run([sys.executable, str(USER), *args], capture_output=True, text=True, env=self.env, timeout=60)
        line = (r.stdout.strip().splitlines() or [''])[-1]
        return r.returncode, (json.loads(line) if line.startswith('{') else line)

    def test_status_reads_only(self):
        before = self.read('home', '.config', 'nimbus', 'session.conf')
        rc, s = self.me('status')
        self.assertEqual(rc, 0, s)
        self.assertEqual(s['picked'], 'classic')
        self.assertEqual(s['loginPick'], 'save')
        self.assertEqual(self.read('home', '.config', 'nimbus', 'session.conf'), before)

    def test_pick_only_what_is_downloaded(self):
        rc, out = self.me('pick', 'kde')
        self.assertEqual(rc, 1)
        self.assertIn('isn\'t downloaded', out)
        self.me('install', 'kde')                                   # the unit stand-in does nothing...
        self.root('run', 'kde')                                     # ...so run what it would
        state = os.path.join(self.home, '.local', 'state', 'hearth')
        os.makedirs(state)
        with open(os.path.join(state, 'desktop-fails'), 'w') as f:
            f.write('kde 2\ngnome 1\n')
        with open(os.path.join(state, 'desktop-notice'), 'w') as f:
            f.write('kde twice 1700000000\n')
        rc, s = self.me('status')
        self.assertIn('twice', s['notice']['text'])
        self.assertEqual(s['desktops']['kde']['fails'], 2)
        rc, s = self.me('pick', 'kde')
        self.assertEqual(rc, 0, s)
        self.assertEqual(s['picked'], 'kde')
        conf = self.read('home', '.config', 'nimbus', 'session.conf')
        self.assertIn('HEARTH_DESKTOP=kde', conf)
        self.assertIn('NIMBUS_START_MODE=game', conf)               # the rest stays
        self.assertIn('# mine', conf)
        self.assertEqual(self.read('home', '.local', 'state', 'hearth', 'desktop-fails'), 'gnome 1\n')
        self.assertIsNone(s['notice'])
        self.assertTrue(os.path.exists(self.conf + '.bak'))

    def test_login_pick(self):
        rc, s = self.me('login', 'once')
        self.assertEqual(s['loginPick'], 'once')
        self.assertIn('HEARTH_LOGIN_PICK=once', self.read('home', '.config', 'nimbus', 'session.conf'))

    def test_tray(self):
        rc, t = self.me('tray')
        self.assertEqual(rc, 0, t)
        for k in ('controller', 'keyboard', 'notices', 'server', 'update', 'switch', 'desktops'):
            self.assertIn(k, t)
        self.assertEqual([d['id'] for d in t['desktops']], ['classic'])
        self.assertEqual(t['switch'], 'Mantel')

    def test_switch_needs_a_downloaded_desktop(self):
        rc, out = self.me('act', 'switch', 'gnome')
        self.assertEqual(rc, 1)
        self.assertNotIn('HEARTH_DESKTOP', self.read('home', '.config', 'nimbus', 'session.conf'))


if __name__ == '__main__':
    unittest.main()
