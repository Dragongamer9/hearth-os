"""Smaller updates (1.13): Debian frozen to this week's Monday (tools/debian_snapshot.py, apt-snapshot.sh), every file
marked with its piece so chunkah gives each Debian package and each Hearth app its own layers
(os-bases/peppermint/components.py), a fixed time for the rechunk, the rechunked image published exactly as made,
each build saying how big its update is (tools/update_size.py), and the background download at the lowest priority."""
import sys
sys.dont_write_bytecode = True
import datetime
import importlib.util
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
PEP = ROOT / 'os-bases/peppermint'
WF = (ROOT / '.github/workflows/base-peppermint.yml').read_text()


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, str(path))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


comp = load(PEP / 'components.py', 'hearth_components')
snap = load(ROOT / 'tools/debian_snapshot.py', 'hearth_debian_snapshot')
size = load(ROOT / 'tools/update_size.py', 'hearth_update_size')
UTC = datetime.timezone.utc


class Snapshot(unittest.TestCase):
    def test_this_weeks_monday(self):
        c = snap.choose(datetime.datetime(2026, 11, 14, 22, 0, tzinfo=UTC), '')   # a Saturday
        self.assertEqual(c['snapshot'], '20261109T000000Z')
        self.assertEqual(c['epoch'], int(datetime.datetime(2026, 11, 9, tzinfo=UTC).timestamp()))
        same_week = snap.choose(datetime.datetime(2026, 11, 10, 3, 0, tzinfo=UTC), '')
        self.assertEqual(same_week['snapshot'], c['snapshot'], 'every build in a week gets the same packages')

    def test_a_newer_date_for_a_security_fix(self):
        now = datetime.datetime(2026, 11, 14, 22, 0, tzinfo=UTC)
        self.assertEqual(snap.choose(now, '# note\n20261112\n')['snapshot'], '20261112T000000Z')
        self.assertEqual(snap.choose(now, '20261020')['snapshot'], '20261109T000000Z', 'an old date never holds it back')
        self.assertEqual(snap.choose(now, '20261201')['snapshot'], '20261109T000000Z', 'never a date that hasn\'t come')
        self.assertEqual(snap.choose(now, 'nonsense')['snapshot'], '20261109T000000Z')
        self.assertEqual(snap.choose(now, 'live')['snapshot'], '')

    def test_the_pin_file_starts_without_a_date(self):
        self.assertIsNone(snap.parse((PEP / 'debian-snapshot').read_text()))
        self.assertIn('DEBIAN_SNAPSHOT=', subprocess.run([sys.executable, str(ROOT / 'tools/debian_snapshot.py')], capture_output=True, text=True).stdout)

    def test_apt_snapshot_keeps_the_live_repositories_for_devices(self):
        sh = (PEP / 'apt-snapshot.sh').read_text()
        for want in ('snapshot.debian.org/archive/debian/$D/', 'snapshot.debian.org/archive/debian-security/$D/', 'trixie-backports',
                     'Check-Valid-Until: no', 'HEARTH-SNAPSHOT-FALLBACK', '/usr/share/hearth/apt-live'):
            self.assertIn(want, sh)
        self.assertEqual(subprocess.run(['sh', '-n', str(PEP / 'apt-snapshot.sh')]).returncode, 0)
        cf = (PEP / 'Containerfile').read_text()
        self.assertIn('ARG DEBIAN_SNAPSHOT=', cf)
        self.assertIn('sh /apt-snapshot.sh "${DEBIAN_SNAPSHOT}"', cf)
        b = (PEP / 'build.sh').read_text()
        tidy = b[b.index('apt-get clean'):b.index('system accounts that later images add')]
        self.assertIn('/usr/share/hearth/apt-live/debian.sources', tidy, 'a device\'s apt gets the live repositories back')
        self.assertIn("grep -q 'deb.debian.org'", tidy)


class Components(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        r = self.root = self.t.name
        for d in ('usr/bin', 'usr/lib/x86_64-linux-gnu', 'usr/lib/hearth-office', 'usr/share/nimbus/desktop/shell',
                  'usr/share/nimbus/web', 'usr/lib/hhd/bin', 'etc', 'usr/lib/sysimage/dpkg/info', 'usr/lib/opt/steam'):
            os.makedirs(os.path.join(r, d), exist_ok=True)
        os.symlink('usr/bin', os.path.join(r, 'bin'))
        self.files = {
            'usr/bin/bash': 'b', 'usr/lib/x86_64-linux-gnu/libc.so.6': 'c', 'usr/lib/x86_64-linux-gnu/libm.so.6': 'm',
            'usr/lib/opt/steam/steam.sh': 's', 'usr/lib/hearth-office/hearth-office': 'o',
            'usr/share/nimbus/desktop/shell/FileManager.qml': 'f', 'usr/share/nimbus/web/index.html': 'w',
            'usr/lib/hhd/bin/hhd': 'h', 'etc/hearth-thing': 'e', 'usr/bin/hearth-files': 'x'}
        for p, t in self.files.items():
            pathlib.Path(r, p).write_text(t)
        os.symlink('bash', os.path.join(r, 'usr/bin/sh'))
        info = os.path.join(r, 'usr/lib/sysimage/dpkg/info')
        pathlib.Path(info, 'bash.list').write_text('/.\n/bin\n/bin/bash\n/usr/bin/sh\n')
        pathlib.Path(info, 'libc6:amd64.list').write_text('/usr/lib/x86_64-linux-gnu/libc.so.6\n/usr/lib/x86_64-linux-gnu/libm.so.6\n/var/lib/x\n')
        pathlib.Path(info, 'steam-installer.list').write_text('/opt/steam/steam.sh\n')
        self.pkgs = {'bash': 'bash', 'libc6:amd64': 'glibc', 'steam-installer': 'steam'}

    def tearDown(self):
        self.t.cleanup()

    def assigned(self):
        root = os.path.realpath(self.root)
        owned = comp.owners(root, os.path.join(root, 'usr/lib/sysimage/dpkg'), self.pkgs)
        return {os.path.relpath(p, root): c for p, c in comp.assign(root, owned).items()}

    def test_debian_files_by_source_package(self):
        a = self.assigned()
        self.assertEqual(a['usr/bin/bash'], 'deb/bash', 'merged /usr: /bin/bash is /usr/bin/bash')
        self.assertEqual(a['usr/lib/x86_64-linux-gnu/libc.so.6'], 'deb/glibc')
        self.assertEqual(a['usr/lib/x86_64-linux-gnu/libm.so.6'], 'deb/glibc', 'a source\'s packages change together')
        self.assertEqual(a['usr/lib/opt/steam/steam.sh'], 'deb/steam', '/opt moved to /usr/lib/opt')
        self.assertNotIn('usr/bin/sh', a, 'links are never marked')

    def test_each_hearth_app_its_own_piece(self):
        a = self.assigned()
        self.assertEqual(a['usr/lib/hearth-office/hearth-office'], 'hearth/usr/lib/hearth-office')
        self.assertEqual(a['usr/share/nimbus/desktop/shell/FileManager.qml'], 'hearth/usr/share/nimbus/desktop')
        self.assertEqual(a['usr/share/nimbus/web/index.html'], 'hearth/usr/share/nimbus/web')
        self.assertNotEqual(a['usr/share/nimbus/desktop/shell/FileManager.qml'], a['usr/lib/hearth-office/hearth-office'])
        self.assertEqual(a['usr/lib/hhd/bin/hhd'], 'hearth/usr/lib/hhd')
        self.assertEqual(a['etc/hearth-thing'], 'hearth/etc')
        self.assertEqual(a['usr/bin/hearth-files'], 'hearth/usr/bin')

    def test_marked_with_one_update_interval_each(self):
        root = os.path.realpath(self.root)
        owned = comp.owners(root, os.path.join(root, 'usr/lib/sysimage/dpkg'), self.pkgs)
        c = comp.assign(root, owned)
        done, failed = comp.mark(c)
        self.assertEqual((done, failed), (len(self.files) + 3, 0), 'and dpkg\'s own lists')
        self.assertEqual(os.getxattr(os.path.join(root, 'usr/bin/bash'), 'user.component'), b'deb/bash')
        libs = [os.path.join(root, 'usr/lib/x86_64-linux-gnu', f) for f in ('libc.so.6', 'libm.so.6')]
        intervals = [x for x in (os.getxattr(p, 'user.update-interval') if 'user.update-interval' in os.listxattr(p) else None for p in libs) if x]
        self.assertEqual(intervals, [b'monthly'], 'set once per component')
        self.assertEqual(os.getxattr(os.path.join(root, 'usr/lib/opt/steam/steam.sh'), 'user.update-interval'), b'weekly')
        self.assertEqual(comp.interval_for('deb/linux-signed-amd64'), 'weekly')
        self.assertEqual(comp.interval_for('deb/nvidia-graphics-drivers'), 'weekly')
        self.assertEqual(comp.interval_for('deb/coreutils'), 'monthly')
        self.assertEqual(comp.interval_for('hearth/usr/lib/hearth-office'), 'weekly')

    def test_runs_last_in_build_and_never_fails_it(self):
        b = (PEP / 'build.sh').read_text().rstrip()
        self.assertTrue(b.endswith('python3 /ctx/components.py / /usr/lib/sysimage/dpkg || echo "components: marking files failed; updates stay big"'))
        self.assertLess(b.index('rm -rf /boot /home /root'), b.index('/ctx/components.py'), 'after the bootc layout')
        self.assertIn('COPY os-bases/peppermint/components.py /components.py', (PEP / 'Containerfile').read_text())


class UpdateSize(unittest.TestCase):
    def test_only_new_layers_count(self):
        old_man = {'layers': [{'digest': 'sha256:a', 'size': 1000_000_000}, {'digest': 'sha256:b', 'size': 300_000_000}]}
        old_cfg = {'rootfs': {'diff_ids': ['sha256:da', 'sha256:db']}}
        new_man = {'layers': [{'digest': 'sha256:a', 'size': 1000_000_000, 'annotations': {'org.chunkah.component': 'deb/glibc'}},
                              {'digest': 'sha256:c', 'size': 120_000_000, 'annotations': {'org.chunkah.component': 'hearth/usr/lib/hearth-office'}},
                              {'digest': 'sha256:e', 'size': 300_000_000, 'annotations': {'org.chunkah.component': 'deb/mesa'}}]}
        new_cfg = {'rootfs': {'diff_ids': ['sha256:da', 'sha256:dc', 'sha256:db']}}
        r = size.compare(new_man, new_cfg, old_man, old_cfg)
        self.assertEqual((r['updateMb'], r['imageMb'], r['newLayers'], r['layers']), (420, 1420, 2, 3))
        self.assertEqual(r['changed'][0]['component'], 'deb/mesa')
        self.assertEqual(r['recompressedMb'], 300, 'same content, other digest: shown')

    def test_reads_the_rechunk_steps_oci_folder(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, 'blobs', 'sha256'))
            def put(obj, h):
                pathlib.Path(d, 'blobs', 'sha256', h).write_text(json.dumps(obj))
                return 'sha256:' + h
            cfg = put({'rootfs': {'diff_ids': ['sha256:x']}}, 'c1')
            man = put({'config': {'digest': cfg}, 'layers': [{'digest': 'sha256:l1', 'size': 5_000_000}]}, 'm1')
            pathlib.Path(d, 'index.json').write_text(json.dumps({'manifests': [{'digest': man}]}))
            pathlib.Path(d, 'old.json').write_text(json.dumps({'layers': []}))
            out = subprocess.run([sys.executable, str(ROOT / 'tools/update_size.py'), d, os.path.join(d, 'old.json'), '--github'],
                                 capture_output=True, text=True).stdout
            self.assertIn('"updateMb": 5', out)
            self.assertIn('::notice title=update size::', out)


class Workflow(unittest.TestCase):
    def test_snapshot_rechunk_size_and_publish_wired(self):
        self.assertIn('python3 tools/debian_snapshot.py >> "$GITHUB_ENV"', WF)
        self.assertIn('--build-arg DEBIAN_SNAPSHOT="${DEBIAN_SNAPSHOT:-}"', WF)
        self.assertIn('-e SOURCE_DATE_EPOCH', WF, 'chunkah stamps a fixed time, not the build\'s')
        self.assertIn('-o oci:/out', WF)
        self.assertIn('tools/update_size.py /mnt/hearth-chunked', WF)
        self.assertIn('skopeo copy -q --preserve-digests', WF, 'published exactly as made: devices compare digests')
        self.assertLess(WF.index('name: Update size'), WF.index('name: Check the image'))
        upd = WF[WF.index('name: Update size'):WF.index('name: Check the image')]
        self.assertIn('continue-on-error: true', upd, 'the size report never stops a release')

    def test_background_download_is_quiet(self):
        d = (PEP / 'system_files/usr/lib/systemd/system/bootc-fetch-apply-updates.service.d/10-hearth-no-reboot.conf').read_text()
        for want in ('Nice=19', 'IOSchedulingClass=idle', 'ExecStart=/usr/bin/bootc upgrade --quiet'):
            self.assertIn(want, d)
        self.assertNotIn('--apply', d, 'never restarts the PC by itself')


if __name__ == '__main__':
    unittest.main()
