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


SYSTEM_PASSWD = 'root:x:0:0:root:/root:/bin/bash\nsddm:x:990:990::/var/lib/sddm:/usr/sbin/nologin\nnobody:x:65534:65534::/nonexistent:/usr/sbin/nologin\n'
FAKE_USERADD = """#!/bin/sh
echo "useradd $*" >> "$CALLS"
[ "$USERADD_RC" = 0 ] || exit "$USERADD_RC"
echo 'hearth:x:1000:1000:Hearth:/var/home/hearth:/bin/bash' >> "$HEARTH_OEM_ROOT/etc/passwd"
echo 'hearth:!:20000:0:99999:7:::' >> "$HEARTH_OEM_ROOT/etc/shadow"
mkdir -p "$HEARTH_OEM_ROOT/var/home/hearth"
"""
FAKE_PASSWD = """#!/bin/sh
echo "passwd $*" >> "$CALLS"
sed -i 's/^hearth:[^:]*:/hearth::/' "$HEARTH_OEM_ROOT/etc/shadow"
"""


class FakePwd:
    """NSS as tests want it: the accounts it lists and whether uid 1000 is known."""
    def __init__(self, people=(), uid1000=False):
        self.people, self.uid1000 = list(people), uid1000

    def getpwall(self):
        return [type('P', (), {'pw_name': n, 'pw_uid': u})() for n, u in self.people]

    def getpwuid(self, uid):
        if uid == 1000 and self.uid1000:
            return object()
        raise KeyError(uid)


class StandInAccount(unittest.TestCase):
    """A pretend / (HEARTH_OEM_ROOT) with only system accounts, fake useradd/passwd, and a fake NSS."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = dict(os.environ)
        os.makedirs(self.tmp + '/etc')
        self.write('etc/passwd', SYSTEM_PASSWD)
        self.write('etc/shadow', 'root:*:20000::::::\n')
        self.write('etc/group', 'wheel:x:10:\nvideo:x:44:\nnetdev:x:108:\n')
        bindir = self.tmp + '/bin'
        os.makedirs(bindir)
        for name, text in (('useradd', FAKE_USERADD), ('passwd', FAKE_PASSWD)):
            self.write('bin/' + name, text)
            os.chmod(bindir + '/' + name, 0o755)
        os.environ.update(HEARTH_OEM_ROOT=self.tmp, CALLS=self.tmp + '/calls', USERADD_RC='0', PATH=bindir + ':' + os.environ['PATH'])
        self.m = load(NIMBUS / 'hearth-oem-account', 'hearth_oem_account')
        self.m.OWNER_UID = os.getuid()
        self.nss(FakePwd())

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env)

    def nss(self, fake):
        self.m.pwd = fake

    def write(self, rel, text):
        path = os.path.join(self.tmp, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w') as f:
            f.write(text)

    def read(self, rel):
        with open(os.path.join(self.tmp, rel)) as f:
            return f.read()

    def has(self, rel):
        return os.path.lexists(os.path.join(self.tmp, rel))

    def calls(self):
        return self.read('calls') if self.has('calls') else ''

    def refused(self, why_part):
        self.assertEqual(self.m.main(), 0)
        self.assertNotIn('useradd', self.calls())
        self.assertFalse(self.has('var/lib/hearth/oem-setup.json'))
        self.assertTrue(self.has('var/lib/hearth/oem-account.checked'), 'decided once, whatever it decided')
        self.assertIn(why_part, json.loads(self.read('var/lib/hearth/oem-account.checked'))['why'])

    # ---- makes it only on a PC with no account of any kind
    def test_no_account_makes_the_stand_in_with_no_password_and_the_marker(self):
        self.assertEqual(self.m.main(), 0)
        self.assertIn('useradd -m -u 1000 -c Hearth -s /bin/bash -G wheel,video,netdev hearth', self.calls())
        self.assertIn('passwd -d hearth', self.calls())
        self.assertEqual(json.loads(self.read('var/lib/hearth/oem-setup.json'))['user'], 'hearth')
        self.assertTrue(json.loads(self.read('var/lib/hearth/oem-account.checked'))['made'])
        self.assertIn('hearth::', self.read('etc/shadow'))

    def test_uid_1001_in_etc_passwd_is_refused(self):
        self.write('etc/passwd', SYSTEM_PASSWD + 'alex:x:1001:1001::/var/home/alex:/bin/bash\n')
        self.refused('/etc/passwd')

    def test_a_homed_account_is_refused(self):
        self.nss(FakePwd(people=[('sam', 60123)]))
        self.refused('known to the system')

    def test_homed_storage_alone_is_refused(self):
        self.write('var/lib/systemd/home/sam.home', 'x')
        self.refused('home folders')

    def test_a_high_uid_from_ldap_is_refused_but_nobody_is_not_a_person(self):
        self.nss(FakePwd(people=[('nobody', 65534)]))
        self.assertEqual(self.m.plan()[0], ['stamp', 'mark', 'useradd', 'nopass'])
        self.nss(FakePwd(people=[('ldapuser', 2 ** 31 - 5)]))
        self.assertEqual(self.m.plan()[0], ['stamp'])
        self.nss(FakePwd(uid1000=True))
        self.assertEqual(self.m.plan()[0], ['stamp'])

    def test_an_existing_home_folder_is_refused(self):
        os.makedirs(self.tmp + '/home/alex')
        self.refused('home folders')

    def test_an_unreadable_etc_passwd_is_refused(self):
        os.remove(self.tmp + '/etc/passwd')
        self.refused("can't be read")

    def test_the_stamp_means_it_never_decides_again(self):
        self.write('var/lib/hearth/oem-account.checked', '{}')
        self.assertEqual(self.m.main(), 0)
        self.assertEqual(self.calls(), '')
        self.assertFalse(self.has('var/lib/hearth/oem-setup.json'))

    def test_live_installer_and_recovery_usb_are_left_alone(self):
        for mark in ('etc/hearth-live', 'etc/hearth-installer', 'etc/hearth-recovery'):
            self.write(mark, '')
            self.assertEqual(self.m.plan()[0], [], mark)
            os.remove(os.path.join(self.tmp, mark))
        u = UNIT.read_text()
        for mark in ('hearth-live', 'hearth-installer', 'hearth-recovery'):
            self.assertIn('ConditionPathExists=!/etc/' + mark, u)

    # ---- a crash between steps is finished at the next start, the marker kept
    def test_crash_after_the_marker_before_useradd(self):
        os.environ['USERADD_RC'] = '9'
        self.m.main()
        self.assertTrue(self.has('var/lib/hearth/oem-setup.json'), 'the marker comes first')
        self.assertNotIn('hearth:', self.read('etc/passwd'))
        os.environ['USERADD_RC'] = '0'
        self.m.main()
        self.assertIn('hearth:x:1000', self.read('etc/passwd'))
        self.assertIn('hearth::', self.read('etc/shadow'))
        self.assertTrue(self.has('var/lib/hearth/oem-setup.json'))

    def test_crash_after_useradd_before_passwd(self):
        self.write('var/lib/hearth/oem-account.checked', '{}')
        self.m.write_root_file(self.tmp + '/var/lib/hearth/oem-setup.json', {'user': 'hearth'})
        self.write('etc/passwd', SYSTEM_PASSWD + 'hearth:x:1000:1000:Hearth:/var/home/hearth:/bin/bash\n')
        self.write('etc/shadow', 'hearth:!:20000:0:99999:7:::\n')
        self.assertEqual(self.m.plan()[0], ['nopass'])
        self.m.main()
        self.assertIn('hearth::', self.read('etc/shadow'))
        self.assertTrue(self.has('var/lib/hearth/oem-setup.json'))

    def test_a_password_set_meanwhile_is_never_cleared(self):
        self.m.write_root_file(self.tmp + '/var/lib/hearth/oem-setup.json', {'user': 'hearth'})
        self.write('etc/passwd', SYSTEM_PASSWD + 'hearth:x:1000:1000:Hearth:/var/home/hearth:/bin/bash\n')
        self.write('etc/shadow', 'hearth:$y$j9T$abc$def:20000:0:99999:7:::\n')
        self.assertEqual(self.m.plan()[0], [])

    def test_a_marker_with_a_real_account_and_no_stand_in_does_nothing(self):
        self.m.write_root_file(self.tmp + '/var/lib/hearth/oem-setup.json', {'user': 'hearth'})
        self.write('etc/passwd', SYSTEM_PASSWD + 'alex:x:1001:1001::/var/home/alex:/bin/bash\n')
        self.assertEqual(self.m.plan()[0], [])

    def test_a_planted_marker_is_ignored(self):
        self.write('var/lib/hearth/oem-account.checked', '{}')
        self.write('elsewhere.json', '{"user": "hearth"}')
        os.makedirs(self.tmp + '/var/lib/hearth', exist_ok=True)
        os.symlink(self.tmp + '/elsewhere.json', self.tmp + '/var/lib/hearth/oem-setup.json')
        self.assertEqual(self.m.plan()[0], [], 'a link')
        os.remove(self.tmp + '/var/lib/hearth/oem-setup.json')
        self.write('var/lib/hearth/oem-setup.json', '{"user": "hearth"}')
        os.chmod(self.tmp + '/var/lib/hearth', 0o777)
        self.assertEqual(self.m.plan()[0], [], 'a folder anyone can write to')
        os.chmod(self.tmp + '/var/lib/hearth', 0o755)
        self.m.OWNER_UID = os.getuid() + 1
        self.assertEqual(self.m.plan()[0], [], "someone else's file")

    def test_a_failure_never_stops_the_start(self):
        r = subprocess.run([sys.executable, str(NIMBUS / 'hearth-oem-account')], capture_output=True, text=True,
                           env=dict(os.environ, HEARTH_OEM_ROOT=self.tmp + '/missing', PATH='/nonexistent'))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_the_unit_waits_for_every_account_source_and_holds_nothing_up(self):
        u = UNIT.read_text()
        self.assertRegex(u, r'Before=.*nimbus-autologin\.service')
        self.assertRegex(u, r'Before=.*display-manager\.service')
        for x in ('nss-user-lookup.target', 'systemd-userdbd.service', 'systemd-homed.service'):
            self.assertRegex(u, r'(?m)^After=.*' + re.escape(x))
        self.assertRegex(u, r'(?m)^RequiresMountsFor=.*/home')
        self.assertIn('TimeoutStartSec=30', u)
        self.assertEqual(self.m.STEP_TIMEOUT, 10)
        self.assertIn('ExecStart=-/usr/libexec/nimbus/hearth-oem-account', u)
        self.assertNotRegex(u, r'(?m)^(Requires|BindsTo|RequiredBy)=')
        b = BUILD.read_text()
        self.assertIn('systemctl enable hearth-oem-account.service', b)
        self.assertIn('chmod 755 /usr/libexec/nimbus/hearth-oem-account', b)
        self.assertIn("'d /var/lib/hearth 0755 root root -'", b, '/var/lib/hearth stays root-only (tmpfiles)')


class SessionOpensSetup(unittest.TestCase):
    def pending(self, mark, legacy='0', library=None, asked=False, owner=None):
        text = SESSION.read_text()
        fns = ''.join(re.search(r'(?ms)^%s\(\) \{.*?^\}' % f, text).group(0) + '\n' for f in ('oem_pending', 'setup_pending'))
        d = tempfile.mkdtemp()
        if library is not None:
            os.makedirs(d + '/nimbus')
            with open(d + '/nimbus/library.json', 'w') as f:
                f.write(library)
        if asked:
            open(d + '/nimbus-oem-asked', 'w').close()
        script = 'RUN=%s\ncan_game() { return 0; }\n%s\nsetup_pending && echo yes || echo no\n' % (d, fns)
        env = dict(os.environ, HEARTH_LEGACY_SETUP=legacy, HEARTH_OEM_MARK=mark, XDG_DATA_HOME=d, HEARTH_SEAT='',
                   HEARTH_OEM_OWNER=str(os.getuid() if owner is None else owner))
        return subprocess.run(['bash', '-c', script], env=env, capture_output=True, text=True).stdout.strip()

    def test_the_marker_opens_setup_even_after_skip_once_a_sign_in(self):
        m = tempfile.mktemp()
        self.assertEqual(self.pending(m), 'no')
        with open(m, 'w') as f:
            f.write('{}')
        self.assertEqual(self.pending(m), 'yes')
        self.assertEqual(self.pending(m, library='{"setupDone": 1763370000000}'), 'yes', 'Skip doesn\'t end it')
        self.assertEqual(self.pending(m, asked=True), 'no', 'asked already this sign-in: on to Steam Game Mode')
        self.assertEqual(self.pending(m, owner=os.getuid() + 1), 'no', "someone else's file")
        self.assertEqual(self.pending(tempfile.mktemp(), library='{"setupDone": 1}'), 'no')

    def test_the_session_notes_it_asked(self):
        self.assertIn(': > "$RUN/nimbus-oem-asked"', SESSION.read_text())


class OemFinish(unittest.TestCase):
    def setUp(self):
        self.a = load(NIMBUS / 'hearth-admin', 'hearth_admin_oem')
        self.tmp = tempfile.mkdtemp()
        self.a.OEM_MARK = self.tmp + '/oem-setup.json'
        self.a.OEM_OWNER = os.getuid()
        with open(self.a.OEM_MARK, 'w') as f:
            json.dump({'user': 'hearth'}, f)
        self.ran = []
        self.a.admin_run = lambda cmd, input=None, **k: (self.ran.append((cmd, input)), (0, ''))[1]
        self.a.valid_timezone = lambda z: z in ('America/Chicago', 'UTC')
        self.a.caller = lambda: type('P', (), {'pw_name': self.me, 'pw_uid': self.uid})()
        self.me, self.uid = 'hearth', 1000

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

    def test_another_uid_named_hearth_is_refused(self):
        self.uid = 1001
        code, out = self.run_it({'full': 'Sam', 'password': 'abcd1234'})
        self.assertEqual(code, 1)
        self.assertEqual(self.ran, [])

    def test_a_planted_marker_is_refused(self):
        os.remove(self.a.OEM_MARK)
        with open(self.tmp + '/planted.json', 'w') as f:
            json.dump({'user': 'hearth'}, f)
        os.symlink(self.tmp + '/planted.json', self.a.OEM_MARK)
        self.assertEqual(self.run_it({'full': 'Sam', 'password': 'abcd1234'})[0], 1, 'a link')
        os.remove(self.a.OEM_MARK)
        with open(self.a.OEM_MARK, 'w') as f:
            json.dump({'user': 'hearth'}, f)
        os.chmod(self.tmp, 0o777)
        self.assertEqual(self.run_it({'full': 'Sam', 'password': 'abcd1234'})[0], 1, 'a folder anyone can write to')
        os.chmod(self.tmp, 0o700)
        self.a.OEM_OWNER = os.getuid() + 1
        self.assertEqual(self.run_it({'full': 'Sam', 'password': 'abcd1234'})[0], 1, "not root's")
        self.assertEqual(self.ran, [])

    def test_dispatched_from_main(self):
        self.assertIn("elif cmd == 'oem-finish' and len(argv) == 2:", (NIMBUS / 'hearth-admin').read_text())


if __name__ == '__main__':
    unittest.main()
