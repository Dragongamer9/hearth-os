"""First-time setup on a pre-installed PC (Hearth OS put on with `bootc install`, no installer, so no account):
hearth-oem-account makes a stand-in account before the autologin (and never stops a start), nimbus-session opens
first-time setup for it, and hearth-admin oem-finish sets your name, password and time zone, only for that account
and only once. The setup step itself is checked in tools/ui-check.js ("pre-installed PC").
"""
import importlib.machinery
import importlib.util
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
NIMBUS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
UNIT = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system' / 'hearth-oem-account.service'
SESSION = ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session'
BUILD = ROOT / 'os-bases' / 'peppermint' / 'build.sh'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def no_one(uid):
    raise KeyError(uid)


class StandInAccount(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        os.environ['HEARTH_OEM_ROOT'] = self.tmp
        self.path = os.environ['PATH']
        self.m = load(NIMBUS / 'hearth-oem-account', 'hearth_oem_account')

    def tearDown(self):
        os.environ.pop('HEARTH_OEM_ROOT', None)
        os.environ['PATH'] = self.path

    def test_no_account_makes_the_stand_in_with_uid_1000_and_admin_rights(self):
        args = self.m.plan(getpwuid=no_one, groups={'wheel', 'video', 'netdev', 'users'}, getpwall=list)
        self.assertEqual(args[:4], ['useradd', '-m', '-u', '1000'])
        self.assertEqual(args[-1], 'hearth')
        self.assertIn('-G', args)
        self.assertEqual(args[args.index('-G') + 1].split(','), ['wheel', 'video', 'netdev'])

    def test_an_account_already_there_means_nothing_changes(self):
        self.assertIsNone(self.m.plan(getpwuid=lambda uid: object(), groups={'wheel'}, getpwall=list))
        # someone else's account (uid 1001, say uid 1000 was removed): no stand-in either
        other = [type('P', (), {'pw_name': 'alex', 'pw_uid': 1001})()]
        self.assertIsNone(self.m.plan(getpwuid=no_one, groups={'wheel'}, getpwall=lambda: other))
        # system accounts don't count
        sysacc = [type('P', (), {'pw_name': 'sddm', 'pw_uid': 990})(), type('P', (), {'pw_name': 'nobody', 'pw_uid': 65534})()]
        self.assertIsNotNone(self.m.plan(getpwuid=no_one, groups={'wheel'}, getpwall=lambda: sysacc))

    def test_the_live_and_installer_usb_are_left_alone(self):
        for mark in ('etc/hearth-live', 'etc/hearth-installer'):
            os.makedirs(os.path.join(self.tmp, 'etc'), exist_ok=True)
            open(os.path.join(self.tmp, mark), 'w').close()
            self.assertIsNone(self.m.plan(getpwuid=no_one, groups={'wheel'}, getpwall=list), mark)
            os.remove(os.path.join(self.tmp, mark))

    def _fake(self, useradd_rc):
        bindir = os.path.join(self.tmp, 'bin')
        os.makedirs(bindir, exist_ok=True)
        self.calls = os.path.join(self.tmp, 'calls')
        for cmd, rc in (('useradd', useradd_rc), ('passwd', 0)):
            with open(os.path.join(bindir, cmd), 'w') as f:
                f.write('#!/bin/sh\necho "%s $*" >> %s\nexit %d\n' % (cmd, self.calls, rc))
            os.chmod(os.path.join(bindir, cmd), 0o755)
        os.environ['PATH'] = bindir + ':' + os.environ['PATH']
        self.m.plan = lambda: ['useradd', '-m', '-u', '1000', '-c', 'Hearth', '-s', '/bin/bash', 'hearth']

    def test_made_with_no_password_and_the_setup_mark_left(self):
        self._fake(0)
        self.assertEqual(self.m.main(), 0)
        calls = open(self.calls).read()
        self.assertIn('useradd -m -u 1000', calls)
        self.assertIn('passwd -d hearth', calls)
        mark = json.load(open(os.path.join(self.tmp, 'var/lib/hearth/oem-setup.json')))
        self.assertEqual(mark['user'], 'hearth')

    def test_a_failure_never_stops_the_start(self):
        self._fake(9)
        self.assertEqual(self.m.main(), 0)
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'var/lib/hearth/oem-setup.json')))
        r = subprocess.run([sys.executable, str(NIMBUS / 'hearth-oem-account')], capture_output=True, text=True,
                           env=dict(os.environ, HEARTH_OEM_ROOT=self.tmp + '/missing', PATH='/nonexistent'))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_the_unit_runs_before_the_autologin_and_holds_nothing_up(self):
        u = UNIT.read_text()
        self.assertRegex(u, r'Before=.*nimbus-autologin\.service')
        self.assertRegex(u, r'Before=.*display-manager\.service')
        self.assertIn('ExecStart=-/usr/libexec/nimbus/hearth-oem-account', u)
        self.assertNotRegex(u, r'(?m)^(Requires|BindsTo|RequiredBy)=')
        b = BUILD.read_text()
        self.assertIn('systemctl enable hearth-oem-account.service', b)
        self.assertIn('chmod 755 /usr/libexec/nimbus/hearth-oem-account', b)


class SessionOpensSetup(unittest.TestCase):
    def pending(self, mark, legacy='0', library=None):
        text = SESSION.read_text()
        fn = re.search(r'(?ms)^setup_pending\(\) \{.*?^\}', text).group(0)
        d = tempfile.mkdtemp()
        if library is not None:
            os.makedirs(d + '/nimbus')
            open(d + '/nimbus/library.json', 'w').write(library)
        script = 'can_game() { return 0; }\n%s\nsetup_pending && echo yes || echo no\n' % fn
        env = dict(os.environ, HEARTH_LEGACY_SETUP=legacy, HEARTH_OEM_MARK=mark, XDG_DATA_HOME=d, HEARTH_SEAT='')
        return subprocess.run(['bash', '-c', script], env=env, capture_output=True, text=True).stdout.strip()

    def test_the_mark_makes_setup_pending(self):
        m = tempfile.mktemp()
        self.assertEqual(self.pending(m), 'no')
        open(m, 'w').write('{}')
        self.assertEqual(self.pending(m), 'yes')
        self.assertEqual(self.pending(m, library='{"setupDone": 1763370000000}'), 'no', 'setup done (or skipped) stays done')


class OemFinish(unittest.TestCase):
    def setUp(self):
        self.a = load(NIMBUS / 'hearth-admin', 'hearth_admin_oem')
        self.tmp = tempfile.mkdtemp()
        self.a.OEM_MARK = self.tmp + '/oem-setup.json'
        json.dump({'user': 'hearth'}, open(self.a.OEM_MARK, 'w'))
        self.ran = []
        self.a.admin_run = lambda cmd, input=None, **k: (self.ran.append((cmd, input)), (0, ''))[1]
        self.a.valid_timezone = lambda z: z in ('America/Chicago', 'UTC')
        self.a.caller = lambda: type('P', (), {'pw_name': self.me})()
        self.me = 'hearth'

    def run_it(self, req):
        out, old_in, old_out = io.StringIO(), sys.stdin, sys.stdout
        sys.stdin, sys.stdout = io.StringIO(json.dumps(req) + '\n'), out
        try:
            self.a.oem_finish()
            code = 0
        except SystemExit as e:
            code = e.code
        finally:
            sys.stdin, sys.stdout = old_in, old_out
        return code, out.getvalue()

    def test_sets_name_password_and_time_zone_then_removes_the_mark(self):
        code, out = self.run_it({'full': 'Sam Smith', 'password': 'hunter22', 'timezone': 'America/Chicago'})
        self.assertEqual(code, 0, out)
        self.assertIn((['timedatectl', 'set-timezone', 'America/Chicago'], None), self.ran)
        self.assertIn((['usermod', '-c', 'Sam Smith', 'hearth'], None), self.ran)
        self.assertIn((['chpasswd'], 'hearth:hunter22\n'), self.ran)
        self.assertFalse(os.path.exists(self.a.OEM_MARK))
        self.assertTrue(json.loads(out.strip().splitlines()[-1])['ok'])

    def test_no_password_leaves_the_password_alone(self):
        code, out = self.run_it({'full': 'Sam', 'nopassword': True, 'timezone': 'UTC'})
        self.assertEqual(code, 0, out)
        self.assertFalse(any(c[0] == ['chpasswd'] for c in self.ran))

    def test_only_the_stand_in_account_and_only_once(self):
        self.me = 'alex'
        code, out = self.run_it({'full': 'Alex', 'password': 'abcd1234'})
        self.assertEqual(code, 1)
        self.assertIn('only for first-time setup', out)
        self.me = 'hearth'
        os.remove(self.a.OEM_MARK)
        code, out = self.run_it({'full': 'Sam', 'password': 'abcd1234'})
        self.assertEqual(code, 1)
        self.assertEqual(self.ran, [], 'nothing changed without the mark')

    def test_bad_input_is_refused_before_anything_changes(self):
        for req in ({'full': '', 'password': 'abcd1234'}, {'full': 'Sam:x', 'password': 'abcd1234'},
                    {'full': 'Sam', 'password': 'ab'}, {'full': 'Sam', 'password': 'abcd\n1234'},
                    {'full': 'Sam', 'password': 'abcd1234', 'timezone': '../../etc/passwd'},
                    {'full': 'Sam', 'password': 'abcd1234', 'timezone': 'Mars/Base'}):
            code, out = self.run_it(req)
            self.assertEqual(code, 1, req)
            self.assertEqual(self.ran, [], req)
            self.assertTrue(os.path.exists(self.a.OEM_MARK))

    def test_dispatched_from_main(self):
        self.assertIn("elif cmd == 'oem-finish' and len(argv) == 2:", (NIMBUS / 'hearth-admin').read_text())


if __name__ == '__main__':
    unittest.main()
