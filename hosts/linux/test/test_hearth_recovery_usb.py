"""Tests for the Hearth Recovery USB's engine (os/recovery/src/hearth-recovery) on fake drives (folders; see
HEARTH_RECOVERY_FAKE in its docstring): send logs, Desktop Mode, reinstall keeping files (every version kept),
copy files off, factory reset (what it may erase: never another system's partitions), and hearth-install's
"reuse" mode it uses. No real drives, no ostree: commands that would change a drive are recorded instead."""
import sys
sys.dont_write_bytecode = True
import contextlib
import http.server
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import threading
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
GIB = 1024 ** 3


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


R = load(ROOT / 'os' / 'recovery' / 'src' / 'hearth-recovery', 'hearth_recovery')
I = load(ROOT / 'os' / 'live' / 'src' / 'hearth-install', 'hearth_install_reuse')

ESP, XBOOT, LINUX, MSR = R.ESP_TYPE, R.XBOOTLDR_TYPE, R.LINUX_TYPE, R.MSR_TYPE
CSUM1, CSUM2 = 'a' * 64, 'b' * 64
# The fake user's number: this one's own when it's a person's (a CI runner), else 1000 (as root, chown works).
UID = os.getuid() if 1000 <= os.getuid() < 60000 else 1000


def write(path, text=''):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


class Fake:
    """A PC with Hearth OS (whole-drive install: EFI, boot, hearth with root and home), two versions, user sam."""

    def __init__(self, extra=()):
        self.dir = tempfile.mkdtemp(prefix='hr-fake-')
        self.run = os.path.join(self.dir, 'run')
        self.fake = os.path.join(self.dir, 'drives')
        uid, gid = os.getuid(), os.getgid()
        devs = [
            {'name': 'esp', 'fstype': 'vfat', 'disk': 'fake:nvme', 'parttype': ESP, 'partlabel': 'EFI System', 'size': 600 * 2 ** 20, 'uuid': 'E-1'},
            {'name': 'boot', 'fstype': 'ext4', 'disk': 'fake:nvme', 'parttype': XBOOT, 'partlabel': 'boot', 'size': 2 * GIB, 'uuid': 'boot-uuid'},
            {'name': 'hearth', 'fstype': 'btrfs', 'disk': 'fake:nvme', 'parttype': LINUX, 'partlabel': 'hearth', 'size': 400 * GIB, 'uuid': 'root-uuid'},
        ] + list(extra)
        write(os.path.join(self.fake, 'devices.json'), json.dumps(devs))
        for d in devs:
            os.makedirs(os.path.join(self.fake, d['name']), exist_ok=True)
        os.makedirs(os.path.join(self.fake, 'esp', 'EFI', 'fedora'))
        os.makedirs(os.path.join(self.fake, 'esp', 'EFI', 'BOOT'))
        self.sysroot = os.path.join(self.fake, 'hearth', 'root')
        boot = os.path.join(self.fake, 'boot')
        for n, (csum, ver) in enumerate(((CSUM1, '0.6.53'), (CSUM2, '0.6.52'))):
            self.add_deployment(csum, 0, ver, version=2 - n)
        os.makedirs(os.path.join(self.sysroot, 'boot'))
        home = os.path.join(self.fake, 'hearth', 'home', 'sam')
        write(os.path.join(home, 'Documents', 'notes.txt'), 'hello from sam@example.com')
        write(os.path.join(home, '.cache', 'big.bin'), 'x' * 1000)
        write(os.path.join(home, '.local', 'share', 'nimbus', 'session.log'), 'session for sam: started\n')
        write(os.path.join(home, '.config', 'nimbus', 'session.conf'),
              'NIMBUS_START_MODE=game\nNIMBUS_GAMESCOPE_ARGS="-W 1280 -H 800 -r 60"\n')
        write(os.path.join(home, '.config', 'nimbus', 'renderer'), 'qt')
        self.boot = boot
        self.uid, self.gid = uid, gid

    def add_deployment(self, csum, serial, ver, version):
        dep = os.path.join(self.sysroot, 'ostree', 'deploy', 'default', 'deploy', f'{csum}.{serial}')
        write(os.path.join(dep, 'usr', 'share', 'nimbus', 'version'), ver + '\n')
        write(os.path.join(dep, 'usr', 'lib', 'os-release'), 'NAME="Hearth OS"\nID=anatase\nID_LIKE=fedora\nVERSION_ID=43\n')
        write(os.path.join(dep, 'etc', 'fstab'), 'UUID=root-uuid / btrfs subvol=root 0 0\nUUID=boot-uuid /boot ext4 defaults 0 2\n'
                                                 'UUID=E-1 /boot/efi vfat umask=0077 0 2\nUUID=root-uuid /var/home btrfs subvol=home 0 0\n')
        write(os.path.join(dep, 'etc', 'passwd'), f'root:x:0:0:root:/root:/bin/bash\nsam:x:{UID}:{UID}:Sam:/var/home/sam:/bin/bash\n')
        write(dep + '.origin', '[origin]\ncontainer-image-reference=ostree-unverified-registry:ghcr.io/dragongamer9/hearth-os:latest\n')
        link = os.path.join(self.sysroot, 'ostree', f'boot.1', 'default', csum, str(serial))
        os.makedirs(os.path.dirname(link), exist_ok=True)
        if not os.path.lexists(link):
            os.symlink(f'../../../deploy/default/deploy/{csum}.{serial}', link)
        write(os.path.join(self.fake, 'boot', 'loader', 'entries', f'ostree-{version}-default.conf'),
              f'title Hearth OS {ver}\nversion {version}\nlinux /ostree/x/vmlinuz\noptions root=UUID=root-uuid rw ostree=/ostree/boot.1/default/{csum}/{serial}\n')
        return dep

    @contextlib.contextmanager
    def active(self, **env):
        with mock.patch.multiple(R, FAKE=self.fake, RUN=self.run, **env):
            yield self

    def close(self):
        shutil.rmtree(self.dir, ignore_errors=True)


class Base(unittest.TestCase):
    extra = ()

    def setUp(self):
        self.f = Fake(self.extra)
        self.addCleanup(self.f.close)
        self.out = io.StringIO()
        p = mock.patch('sys.stdout', self.out)
        p.start()
        self.addCleanup(p.stop)

    def sid(self):
        with self.f.active():
            return R.scan()['systems'][0]['id']


class Scan(Base):
    def test_finds_hearth_and_its_versions(self):
        with self.f.active():
            r = R.scan()
        self.assertEqual(len(r['systems']), 1)
        s = r['systems'][0]
        self.assertEqual(s['id'], 'fake:hearth:root')
        self.assertEqual([d['hearthVersion'] for d in s['deployments']], ['0.6.53', '0.6.52'])
        self.assertEqual([u['name'] for u in s['users']], ['sam'])


class DesktopMode(Base):
    def conf(self):
        return open(os.path.join(self.f.fake, 'hearth', 'home', 'sam', '.config', 'nimbus', 'session.conf')).read()

    def test_starts_in_desktop_mode_and_settings_can_read_it(self):
        sid = self.sid()
        with self.f.active():
            r = R.reset_display(sid, desktop=True)
            R.reset_display(sid, desktop=True)   # twice: still one line
        text = self.conf()
        self.assertTrue(r['ok'])
        self.assertEqual(text.count('NIMBUS_START_MODE='), 1)
        self.assertEqual(text.count(R.DESKTOP_NOTE), 1)
        self.assertNotIn('NIMBUS_GAMESCOPE_ARGS', text)
        # The same check Settings › Sign-in (system-admin.js) uses: nothing after the value.
        self.assertRegex(text, re.compile(r'''^\s*NIMBUS_START_MODE=["']?desktop["']?\s*$''', re.M))
        # And the shell reads it.
        out = subprocess.run(['sh', '-c', '. "$1"; echo "$NIMBUS_START_MODE"', 'sh', os.path.join(self.f.fake, 'hearth', 'home', 'sam', '.config', 'nimbus', 'session.conf')],
                             capture_output=True, text=True).stdout.strip()
        self.assertEqual(out, 'desktop')
        cfg = os.path.join(self.f.fake, 'hearth', 'home', 'sam', '.config', 'nimbus')
        self.assertEqual(open(os.path.join(cfg, 'renderer')).read(), 'auto')
        self.assertTrue(any(n.startswith('recovery-backup-') for n in os.listdir(cfg)))

    def test_reset_display_keeps_game_mode(self):
        sid = self.sid()
        with self.f.active():
            R.reset_display(sid, desktop=False)
        self.assertIn('NIMBUS_START_MODE=game', self.conf())
        self.assertNotIn('NIMBUS_GAMESCOPE_ARGS', self.conf())


class LogService:
    def __init__(self):
        got = self.got = []

        class H(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers['Content-Length'])).decode()
                got.append((dict(self.headers), body))
                data = json.dumps({'code': 'ABCD-EFGH-JK'}).encode()
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self):
                self.send_response(404)
                self.end_headers()

            def log_message(self, *a):
                pass

        self.srv = http.server.HTTPServer(('127.0.0.1', 0), H)
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.url = f'http://127.0.0.1:{self.srv.server_port}/v1/logs'

    def close(self):
        self.srv.shutdown()


class SendLogs(Base):
    def test_sends_like_hearth_does(self):
        svc = LogService()
        self.addCleanup(svc.close)
        sid = self.sid()
        with self.f.active(LOGS_URL=svc.url):
            r = R.send_logs(sid, 'black screen')
        self.assertEqual(r['code'], 'ABCD-EFGH-JK')
        self.assertEqual(r['url'], svc.url + '/ABCD-EFGH-JK')
        headers, body = svc.got[0]
        self.assertTrue(body.startswith('Hearth logs'))         # the log service refuses anything else
        self.assertEqual(headers['X-Hearth-Kind'], 'anatase')   # one of the service's kinds
        self.assertEqual(headers['X-Hearth-Version'], '0.6.53')
        self.assertIn('session.log', body)
        self.assertNotIn('sam@example.com', body)
        self.assertNotRegex(body, r'\bsam\b')

    def test_offline_says_so_before_reading_anything(self):
        sid = self.sid()
        with self.f.active(LOGS_URL='http://127.0.0.1:9/v1/logs'):
            with self.assertRaisesRegex(R.Fail, 'internet'):
                R.send_logs(sid)
        self.assertNotIn('collect', self.out.getvalue())


class Reinstall(Base):
    """update: every version already on the drive is pinned while `ostree container image deploy` runs (it
    would otherwise delete all but the new one and the one that starts now), then unpinned."""

    def fake_ostree(self, fail_pin=False):
        calls = []
        f = self.f

        def sh(cmd, check=True, input=None, timeout=None, **kw):
            calls.append(cmd)
            rc = 1 if (fail_pin and cmd[:3] == ['ostree', 'admin', 'pin'] and '--unpin' not in cmd) else 0
            if rc and check:
                raise R.Fail('ostree: failed')
            return subprocess.CompletedProcess(cmd, rc, '', 'error: pin failed' if rc else '')

        class Popen:
            def __init__(self, cmd, **kw):
                calls.append(cmd)
                # What ostree does: a new deployment that starts first.
                f.add_deployment('c' * 64, 0, '0.6.54', version=3)
                self.stdout = iter(['layers already present: 2; layers needed: 3 (1.2 GB)\n', 'Fetching layer 1\n', 'Deploying\n'])

            def wait(self):
                return 0

        return calls, sh, Popen

    def test_keeps_every_version(self):
        sid = self.sid()
        calls, sh, Popen = self.fake_ostree()
        with self.f.active(sh=sh), mock.patch.object(R.subprocess, 'Popen', Popen):
            r = R.update(sid)
        pins = [c for c in calls if c[:3] == ['ostree', 'admin', 'pin']]
        deploy_at = next(i for i, c in enumerate(calls) if c[:4] == ['ostree', 'container', 'image', 'deploy'])
        first_unpin = next(i for i, c in enumerate(calls) if '--unpin' in c)
        # Both versions pinned before the deploy, unpinned after it (they've moved down one: 1 and 2).
        self.assertEqual([c[-1] for c in pins if '--unpin' not in c], ['0', '1'])
        self.assertTrue(all(calls.index(c) < deploy_at for c in pins if '--unpin' not in c))
        self.assertGreater(first_unpin, deploy_at)
        self.assertEqual(sorted(c[-1] for c in pins if '--unpin' in c), ['1', '2'])
        self.assertEqual(r['new']['hearthVersion'], '0.6.54')
        self.assertEqual((r['kept'], r['lost']), (2, 0))
        self.assertEqual(r['stillPinned'], [])

    def test_no_deploy_when_versions_cant_be_kept(self):
        sid = self.sid()
        calls, sh, Popen = self.fake_ostree(fail_pin=True)
        with self.f.active(sh=sh), mock.patch.object(R.subprocess, 'Popen', Popen):
            with self.assertRaisesRegex(R.Fail, 'Nothing was changed'):
                R.update(sid)
        self.assertFalse(any(c[:4] == ['ostree', 'container', 'image', 'deploy'] for c in calls))


class FactoryMode(unittest.TestCase):
    """What factory reset may erase (pure: factory_mode)."""

    def p(self, path, size, fstype='', parttype='', partlabel='', label=''):
        return {'path': path, 'size': size, 'fstype': fstype, 'parttype': parttype, 'partlabel': partlabel, 'label': label}

    def hearth(self, esp_name='EFI System'):
        return [self.p('/dev/n1', 600 << 20, 'vfat', ESP, esp_name), self.p('/dev/n2', 2 * GIB, 'ext4', XBOOT, 'boot'),
                self.p('/dev/n3', 400 * GIB, 'btrfs', LINUX, 'hearth')]

    def test_whole_drive_with_only_hearth(self):
        m = R.factory_mode(self.hearth(), {'/dev/n1', '/dev/n2', '/dev/n3'}, lambda p: [])
        self.assertEqual(m['mode'], 'whole')

    def test_next_to_windows_only_hearths_partitions(self):
        parts = [self.p('/dev/n1', 100 << 20, 'vfat', ESP, 'EFI system partition'), self.p('/dev/n2', 16 << 20, '', MSR, 'Microsoft reserved'),
                 self.p('/dev/n3', 300 * GIB, 'ntfs', 'ebd0a0a2-b9e5-4433-87c0-68b6b72699c7', 'Basic data', 'Windows'),
                 self.p('/dev/n5', 600 << 20, 'vfat', ESP, 'Hearth EFI'), self.p('/dev/n6', 2 * GIB, 'ext4', XBOOT, 'boot'),
                 self.p('/dev/n7', 200 * GIB, 'btrfs', LINUX, 'hearth')]
        m = R.factory_mode(parts, {'/dev/n5', '/dev/n6', '/dev/n7'}, lambda p: ['Microsoft'] if p == '/dev/n1' else [])
        self.assertEqual(m['mode'], 'hearth')
        self.assertEqual(len(m['erase']), 3)
        self.assertTrue(all('/dev/n' + n not in ' '.join(m['erase']) for n in '123'))
        self.assertTrue(any('Windows' in o for o in m['others']))
        self.assertTrue(any('Microsoft' in o for o in m['others']))

    def test_windows_boot_files_on_hearths_efi_partition_keep_it(self):
        m = R.factory_mode(self.hearth(), {'/dev/n1', '/dev/n2', '/dev/n3'}, lambda p: ['Microsoft'])
        self.assertEqual(m['mode'], 'hearth')   # the EFI partition (not Hearth's by name) stays
        self.assertNotIn('/dev/n1', ' '.join(m['erase']))

    def test_unreadable_efi_partition_counts_as_someone_elses(self):
        m = R.factory_mode(self.hearth(), {'/dev/n1', '/dev/n2', '/dev/n3'}, lambda p: ['(unreadable)'])
        self.assertNotEqual(m['mode'], 'whole')

    def test_not_installed_by_hearths_installer_next_to_something_else(self):
        parts = [self.p('/dev/n1', 600 << 20, 'vfat', ESP, 'EFI-SYSTEM'), self.p('/dev/n2', 1 * GIB, 'ext4', LINUX, 'boot'),
                 self.p('/dev/n3', 200 * GIB, 'xfs', LINUX, 'root'), self.p('/dev/n4', 200 * GIB, 'ext4', LINUX, '', 'data')]
        m = R.factory_mode(parts, {'/dev/n1', '/dev/n2', '/dev/n3'}, lambda p: [])
        self.assertIsNone(m['mode'])
        self.assertIn('data', ' '.join(m['others']))

    def test_too_small_to_install_again(self):
        parts = self.hearth('Hearth EFI') + [self.p('/dev/n4', 300 * GIB, 'ntfs', label='Windows')]
        parts[2]['size'] = 40 * GIB
        self.assertIsNone(R.factory_mode(parts, {'/dev/n1', '/dev/n2', '/dev/n3'}, lambda p: [])['mode'])

    def test_two_hearths(self):
        parts = self.hearth('Hearth EFI') + [self.p('/dev/n4', 2 * GIB, 'ext4', XBOOT, 'boot'), self.p('/dev/n5', 100 * GIB, 'btrfs', LINUX, 'hearth')]
        self.assertIsNone(R.factory_mode(parts, {'/dev/n1', '/dev/n2', '/dev/n3'}, lambda p: [])['mode'])


class FactoryReset(Base):
    extra = ({'name': 'win', 'fstype': 'ntfs', 'disk': 'fake:sata', 'size': 500 * GIB, 'label': 'Windows'},)

    def installer(self):
        path = os.path.join(self.f.dir, 'fake-install')
        self.plan_file = os.path.join(self.f.dir, 'plan.json')
        write(path, '#!/usr/bin/python3\nimport json, sys\n'
                    f'plan = json.load(sys.stdin); open({self.plan_file!r}, "w").write(json.dumps(plan))\n'
                    'print("stage: " + json.dumps({"stage": "disk", "pct": 5, "text": "x"}))\n'
                    'print("result: " + json.dumps({"ok": True}))\n')
        os.chmod(path, 0o755)
        return path

    def test_check_then_reset_the_whole_drive(self):
        sid = self.sid()
        inst = self.installer()
        with self.f.active(INSTALLER=inst):
            c = R.factory_check(sid)
            self.assertEqual(c['mode'], 'whole')   # Windows is on another drive
            self.assertEqual(c['user'], 'sam')
            r = R.factory_reset(sid, 'whole')
        plan = json.load(open(self.plan_file))
        self.assertEqual((plan['mode'], plan['disk'], plan['edition']), ('whole', 'fake:nvme', 'anatase'))
        self.assertTrue(plan['user']['nopassword'])
        self.assertTrue(r['ok'])

    def test_refuses_when_the_drive_looks_different_from_what_was_confirmed(self):
        sid = self.sid()
        inst = self.installer()
        with self.f.active(INSTALLER=inst):
            with self.assertRaisesRegex(R.Fail, 'Nothing was changed'):
                R.factory_reset(sid, 'hearth')
        self.assertFalse(os.path.exists(self.plan_file))

    def test_windows_on_the_same_drive_is_kept(self):
        devs = json.load(open(os.path.join(self.f.fake, 'devices.json')))
        devs[-1]['disk'] = 'fake:nvme'
        write(os.path.join(self.f.fake, 'devices.json'), json.dumps(devs))
        sid = self.sid()
        inst = self.installer()
        with self.f.active(INSTALLER=inst):
            c = R.factory_check(sid)
            self.assertEqual(c['mode'], 'hearth')   # only boot and hearth (hearth-install's "reuse")
            self.assertIn('Windows', ' '.join(c['others']))
            self.assertNotIn('fake:win', ' '.join(c['erase']))
            self.assertNotIn('fake:esp', ' '.join(c['erase']))
            R.factory_reset(sid, 'hearth')
        self.assertEqual(json.load(open(self.plan_file))['mode'], 'reuse')


class CopyFiles(Base):
    extra = (
        {'name': 'stick', 'fstype': 'exfat', 'disk': 'fake:usbdisk', 'removable': True, 'tran': 'usb', 'label': 'STICK', 'size': 64 * GIB, 'uuid': 'S-1'},
        {'name': 'win', 'fstype': 'ntfs', 'disk': 'fake:sata', 'size': 500 * GIB, 'label': 'Windows', 'uuid': 'W-1'},
        {'name': 'data', 'fstype': 'ntfs', 'disk': 'fake:sata', 'size': 400 * GIB, 'label': 'Data', 'uuid': 'D-1'},
        {'name': 'mint', 'fstype': 'ext4', 'disk': 'fake:sata2', 'size': 100 * GIB, 'uuid': 'M-1'},
        {'name': 'minthome', 'fstype': 'ext4', 'disk': 'fake:sata2', 'size': 300 * GIB, 'uuid': 'MH-1'},
        {'name': 'msr', 'fstype': 'vfat', 'disk': 'fake:sata', 'parttype': R.ESP_TYPE, 'size': 100 << 20, 'uuid': 'E-2'},
    )

    def setUp(self):
        super().setUp()
        d = self.f.fake
        os.makedirs(os.path.join(d, 'win', 'Windows', 'System32'))
        os.makedirs(os.path.join(d, 'win', 'Users'))
        os.makedirs(os.path.join(d, 'data', 'Photos'))
        write(os.path.join(d, 'mint', 'etc', 'os-release'), 'NAME="Linux Mint"\n')
        write(os.path.join(d, 'mint', 'etc', 'fstab'), 'UUID=M-1 / ext4 defaults 0 1\nUUID=MH-1 /home ext4 defaults 0 2\n')
        os.makedirs(os.path.join(d, 'minthome', 'alex'))

    def test_offers_usb_and_data_drives_only(self):
        sid = self.sid()
        with self.f.active():
            got = R.copy_targets(sid)['drives']
        self.assertEqual([x['part'] for x in got], ['fake:stick', 'fake:data'])
        self.assertEqual([x['kind'] for x in got], ['usb', 'internal'])
        self.assertGreater(got[0]['free'], 0)

    def test_copies_the_home_folder(self):
        sid = self.sid()
        with self.f.active():
            r = R.copy_home(sid, 'sam', 'fake:stick')
        dest = os.path.join(self.f.fake, 'stick', r['folder'])
        self.assertEqual(open(os.path.join(dest, 'Documents', 'notes.txt')).read(), 'hello from sam@example.com')
        self.assertFalse(os.path.exists(os.path.join(dest, '.cache')))
        self.assertEqual(r['drive'], 'STICK')
        self.assertFalse(os.path.islink(os.path.join(self.f.run, 'usb')))   # unmounted afterwards

    def test_not_onto_another_system(self):
        sid = self.sid()
        with self.f.active():
            with self.assertRaisesRegex(R.Fail, 'gone'):
                R.copy_home(sid, 'sam', 'fake:win')

    def test_too_little_room_copies_nothing(self):
        sid = self.sid()
        small = {'drives': [{'part': 'fake:stick', 'label': 'STICK', 'fstype': 'exfat', 'free': 10, 'kind': 'usb'}]}
        with self.f.active(copy_targets=lambda s=None: small):
            with self.assertRaisesRegex(R.Fail, 'Nothing was copied'):
                R.copy_home(sid, 'sam', 'fake:stick')
        self.assertEqual(os.listdir(os.path.join(self.f.fake, 'stick')), [])

    def test_size_leaves_out_what_isnt_copied(self):
        home = os.path.join(self.f.fake, 'hearth', 'home', 'sam')
        write(os.path.join(home, '.var', 'app', 'org.x', 'cache', 'c'), 'y' * 5000)
        n = R.tree_size(home)
        self.assertLess(n, 1000)


class InstallerReuse(unittest.TestCase):
    """hearth-install's "reuse" mode (factory reset next to other systems): room checked before anything is
    removed, and Windows is never made smaller."""

    def table(self, hearth_sectors):
        ss = 512
        mib = 2048
        parts = [{'node': '/dev/n1', 'start': mib, 'size': 200 * mib, 'type': ESP.upper(), 'name': 'EFI system partition'},
                 {'node': '/dev/n2', 'start': 201 * mib, 'size': 300 * GIB // ss, 'type': 'EBD0A0A2-B9E5-4433-87C0-68B6B72699C7', 'name': 'Basic data'}]
        at = parts[-1]['start'] + parts[-1]['size']
        for node, size, typ, name in (('/dev/n5', 600 * mib, ESP, 'Hearth EFI'), ('/dev/n6', 2048 * mib, XBOOT, 'boot'), ('/dev/n7', hearth_sectors, LINUX, 'hearth')):
            parts.append({'node': node, 'start': at, 'size': size, 'type': typ.upper(), 'name': name})
            at += size
        parts.append({'node': '/dev/n8', 'start': at, 'size': 1024 * mib, 'type': 'DE94BBA4-06D1-4D40-A16A-BFD50179D6AC', 'name': 'Recovery'})
        return {'label': 'gpt', 'firstlba': 34, 'lastlba': at + 1024 * mib + 100, 'partitions': parts}

    def ours(self, info):
        return [p for p in info['partitions'] if p['name'] in ('Hearth EFI', 'boot', 'hearth')]

    def test_room_is_hearths_space(self):
        info = self.table(100 * GIB // 512)
        room = I.reuse_room(info, self.ours(info), 512)
        self.assertEqual(room, (600 + 2048) * 2 ** 20 + 100 * GIB)

    def test_plan_accepts_reuse(self):
        p = I.check_plan({'mode': 'reuse', 'disk': '/dev/nvme0n1', 'user': {'name': 'sam', 'nopassword': True}})
        self.assertEqual(p['mode'], 'reuse')

    def test_no_room_removes_nothing(self):
        info = self.table(40 * GIB // 512)
        calls = []
        with mock.patch.object(I, 'sh', lambda cmd, check=True, input=None, **kw: subprocess.CompletedProcess(cmd, 0, json.dumps({'partitiontable': info}), '')), \
                mock.patch.object(I, 'sector_size', lambda d: 512), \
                mock.patch.object(I, 'remove_ours', lambda d: calls.append('remove')), \
                mock.patch.object(I, 'partition_beside', lambda *a: calls.append('shrink')), \
                mock.patch.object(I, 'partition_free', lambda d: calls.append('free')):
            with self.assertRaisesRegex(I.Fail, 'Nothing was changed'):
                I.partition_reuse('/dev/nvme0n1')
        self.assertEqual(calls, [])

    def test_room_reinstalls_in_place(self):
        info = self.table(200 * GIB // 512)
        calls = []
        with mock.patch.object(I, 'sh', lambda cmd, check=True, input=None, **kw: subprocess.CompletedProcess(cmd, 0, json.dumps({'partitiontable': info}), '')), \
                mock.patch.object(I, 'sector_size', lambda d: 512), \
                mock.patch.object(I, 'remove_ours', lambda d: calls.append('remove')), \
                mock.patch.object(I, 'partition_free', lambda d: calls.append('free') or ('e', 'b', 'r', True)):
            self.assertEqual(I.partition_reuse('/dev/nvme0n1'), ('e', 'b', 'r', True))
        self.assertEqual(calls, ['remove', 'free'])


class SessionConf(unittest.TestCase):
    def test_other_lines_stay(self):
        old = '# mine\nexport NIMBUS_GAMESCOPE_ARGS="-W 1"\nNIMBUS_START_MODE=game\nFOO=1\n'
        self.assertEqual(R.session_conf_text(old, True), '# mine\nFOO=1\n' + R.DESKTOP_NOTE + '\nNIMBUS_START_MODE=desktop\n')
        self.assertEqual(R.session_conf_text(old, False), '# mine\nNIMBUS_START_MODE=game\nFOO=1\n')


if __name__ == '__main__':
    unittest.main()
