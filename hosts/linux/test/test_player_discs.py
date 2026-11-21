"""Hearth Player's CDs and DVDs, the parts outside the app (docs/PLAYER-DISCS.md): DVD decoding is only ever added on
request (hearth-dvd-decoding through hearth-admin, an administrator's password every time, never in the image), the
disc pop-up's Plasma actions, and Hearth Files listing CD/DVD drives and sending film and music discs to the player."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import re
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SYS = ROOT / 'os' / 'system_files'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class DvdDecoding(unittest.TestCase):
    def setUp(self):
        self.m = load(SYS / 'usr/libexec/nimbus/hearth-dvd-decoding', 'hearth_dvd_decoding')

    def test_repository_is_only_free_tainted_and_signed(self):
        t = self.m.repo_text('file:///etc/pki/rpm-gpg/RPM-GPG-KEY-rpmfusion-free-fedora-$releasever',
                             'https://mirrors.rpmfusion.org/metalink?repo=free-fedora-$releasever&arch=$basearch')
        self.assertIn('metalink=https://mirrors.rpmfusion.org/metalink?repo=free-fedora-tainted-$releasever&arch=$basearch', t)
        self.assertIn('gpgcheck=1', t)
        self.assertIn('gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-rpmfusion-free-fedora-$releasever', t)
        self.assertEqual(len(re.findall(r'^\[', t, re.M)), 1, 'one repository, free-tainted only')
        self.assertNotIn('nonfree', t)
        self.assertIn('metalink=' + self.m.DEFAULT_METALINK, self.m.repo_text('file:///k', ''))

    def test_reads_the_images_rpmfusion_file(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, 'etc/yum.repos.d'))
            with open(os.path.join(d, 'etc/yum.repos.d/rpmfusion-free.repo'), 'w') as f:
                f.write('[rpmfusion-free]\nname=x\nmetalink=https://mirrors.rpmfusion.org/metalink?repo=free-fedora-$releasever&arch=$basearch\n'
                        'enabled=0\ngpgkey=file:///usr/share/distribution-gpg-keys/rpmfusion/RPM-GPG-KEY-rpmfusion-free-fedora-$releasever\n'
                        '[rpmfusion-free-debuginfo]\ngpgkey=file:///wrong\n')
            key, ml = self.m.rpmfusion_free(d)
            self.assertTrue(key.endswith('rpmfusion-free-fedora-$releasever'), key)
            self.assertIn('repo=free-fedora-$releasever', ml)
            with open(os.path.join(d, 'etc/os-release'), 'w') as f:
                f.write('NAME="Hearth OS"\nID=fedora\n')
            self.assertTrue(self.m.is_fedora(self.m.os_release(d)))
            with open(os.path.join(d, 'etc/os-release'), 'w') as f:
                f.write('ID=debian\n')
            self.assertFalse(self.m.is_fedora(self.m.os_release(d)))
            self.assertFalse(self.m.installed(d))
            os.makedirs(os.path.join(d, 'usr/lib64'))
            open(os.path.join(d, 'usr/lib64/libdvdcss.so.2'), 'w').close()
            self.assertTrue(self.m.installed(d))

    def test_add_needs_root_and_changes_nothing_otherwise(self):
        if os.geteuid() == 0:
            self.skipTest('running as root')
        with self.assertRaises(SystemExit):
            self.m.add()
        self.assertFalse(os.path.exists(self.m.REPO_FILE + '.tmp'))

    def test_hearth_admin_and_polkit(self):
        admin = (SYS / 'usr/libexec/nimbus/hearth-admin').read_text()
        self.assertIn("cmd in ('dvd-decoding-add', 'dvd-decoding-remove') and len(argv) == 2", admin)
        rules = (SYS / 'usr/share/polkit-1/rules.d/49-hearth-admin.rules').read_text()
        m = re.search(r'dvd-decoding-\(add\|remove\)[^\n]*\n\s*return polkit\.Result\.(\w+)', rules)
        self.assertTrue(m and m.group(1) == 'AUTH_ADMIN', 'adding DVD decoding always asks for an administrator\'s password')
        self.assertLess(rules.index('dvd-decoding-'), rules.rindex('polkit.Result.YES'), 'before the rule that lets admins through')

    def test_never_in_the_image(self):
        for base in ('anatase', 'peppermint'):
            b = (ROOT / 'os-bases' / base / 'build.sh').read_text()
            for line in b.splitlines():
                if 'libdvdcss' in line and not line.lstrip().startswith('#'):
                    self.assertNotRegex(line, r'(dnf5?|apt-get|apt|rpm-ostree)\s+(-y\s+)?install[^\n]*libdvdcss', f'{base}: {line}')
        a = (ROOT / 'os-bases/anatase/build.sh').read_text()
        self.assertRegex(a, r'dnf5 -y install [^\n]*libdvdread libdvdnav', 'libdvdread (and libdvdnav) are in the Fedora image')


class PlasmaAndDesktop(unittest.TestCase):
    def test_disc_popup_actions(self):
        d = SYS / 'usr/share/solid/actions'
        dvd = (d / 'hearth-player-disc.desktop').read_text()
        cd = (d / 'hearth-player-audiocd.desktop').read_text()
        for t in (dvd, cd):
            self.assertIn('Exec=hearth-player --disc %d', t)
            self.assertIn('Name=Play disc in Hearth Player', t)
        self.assertIn("'VideoDvd'", dvd)
        self.assertIn("'VideoCd'", dvd)
        self.assertIn("'Audio'", cd)
        entry = (SYS / 'usr/share/applications/hearth-player.desktop').read_text()
        for t in ('x-content/video-dvd', 'x-content/audio-cdda', 'x-content/video-vcd'):
            self.assertIn(t, entry)
        self.assertIn('[Desktop Action disc]', entry)


class DiscNotice(unittest.TestCase):
    """hearth-disc-notify: the "A disc was inserted" notice with Play disc and Eject (udev events, never polling)."""
    def setUp(self):
        self.n = load(SYS / 'usr/libexec/nimbus/hearth-disc-notify', 'hearth_disc_notify')

    def test_events_and_what_it_says(self):
        n = self.n
        k = b'change@/devices/x/block/sr0\0ACTION=change\0DEVNAME=sr0\0SUBSYSTEM=block\0DISK_MEDIA_CHANGE=1\0'
        self.assertEqual(n.drive_of(n.parse_uevent(k)), '/dev/sr0')
        u = bytearray(b'libudev\0') + (0xfeedcafe).to_bytes(4, 'big')
        props = b'ACTION=change\0SUBSYSTEM=block\0DEVNAME=/dev/sr1\0ID_CDROM_MEDIA=1\0ID_FS_LABEL=MY_FILM\0'
        u += (40).to_bytes(4, sys.byteorder) + (40).to_bytes(4, sys.byteorder) + len(props).to_bytes(4, sys.byteorder)
        u += bytes(40 - len(u)) + props
        p = n.parse_uevent(bytes(u))
        self.assertEqual(n.drive_of(p), '/dev/sr1')
        self.assertEqual(n.kind_of(p, '/dev/sr1', read_root=lambda d: (['VIDEO_TS', 'AUDIO_TS'], 'X')), ('dvd', 'MY_FILM'))
        self.assertEqual(n.kind_of({'ID_CDROM_MEDIA': '1', 'ID_CDROM_MEDIA_TRACK_COUNT_AUDIO': '12'}, 'd'), ('audio-cd', ''))
        self.assertEqual(n.kind_of({'ID_CDROM_MEDIA': '1', 'ID_CDROM_MEDIA_BD': '1'}, 'd'), ('bluray', ''))
        self.assertEqual(n.kind_of({'ID_CDROM_MEDIA': '1', 'ID_CDROM_MEDIA_STATE': 'blank'}, 'd'), ('', ''), 'blank: nothing said')
        self.assertEqual(n.kind_of({}, 'd'), ('', ''))
        self.assertEqual(n.message('dvd', 'MY_FILM'), ('A disc was inserted', 'My Film · DVD film', ['play', 'Play disc', 'eject', 'Eject']))
        self.assertEqual(n.message('bluray', '')[2], ['eject', 'Eject'], 'Blu-ray: only Eject')
        self.assertIsNone(n.drive_of({'SUBSYSTEM': 'block', 'DEVNAME': 'sda'}))

    def test_service_and_bridge(self):
        unit = (SYS / 'usr/lib/systemd/user/hearth-disc-notify.service').read_text()
        self.assertIn('ConditionPathExists=/usr/bin/hearth-player', unit)
        self.assertIn('ExecStart=/usr/libexec/nimbus/hearth-disc-notify', unit)
        for base in ('anatase', 'peppermint'):
            self.assertIn('default.target.wants/hearth-disc-notify.service', (ROOT / 'os-bases' / base / 'build.sh').read_text(), base)
        src = (SYS / 'usr/libexec/nimbus/hearth-disc-notify').read_text()
        self.assertNotIn('time.sleep', src, 'no polling: it waits on udev')
        bridge = (ROOT / 'hosts/linux/desktop/shell/hearth-discord').read_text()
        self.assertIn("(['body', 'actions'],)", bridge, 'Steam Game Mode\'s bridge says it takes buttons')
        self.assertIn("'ActionInvoked'", bridge)


class FilesDrives(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.home = tempfile.mkdtemp()
        os.environ['HOME'] = self.home
        self.f = load(ROOT / 'hosts/linux/desktop/shell/hearth-files', 'hearth_files_discs')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)

    def iso(self, names, label='MY_FILM'):
        b = bytearray(2048 * 24)
        pvd = 16 * 2048
        b[pvd] = 1
        b[pvd + 1:pvd + 6] = b'CD001'
        b[pvd + 40:pvd + 72] = label.ljust(32).encode()
        b[pvd + 158:pvd + 162] = (20).to_bytes(4, 'little')
        b[pvd + 166:pvd + 170] = (2048).to_bytes(4, 'little')
        i = 20 * 2048
        for n in [b'\x00', b'\x01'] + [x.encode() for x in names]:
            rec = bytearray(33 + len(n) + (len(n) + 1) % 2)
            rec[0] = len(rec)
            rec[32] = len(n)
            rec[33:33 + len(n)] = n
            b[i:i + len(rec)] = rec
            i += len(rec)
        p = os.path.join(self.home, 'disc.iso')
        with open(p, 'wb') as f:
            f.write(b)
        return p

    def test_reads_a_discs_top_folder(self):
        names, label = self.f.iso_root_names(self.iso(['VIDEO_TS', 'AUDIO_TS']))
        self.assertEqual(names, ['VIDEO_TS', 'AUDIO_TS'])
        self.assertEqual(label, 'MY_FILM')
        self.assertEqual(self.f.iso_root_names(os.path.join(self.home, 'none')), ([], ''))

    def test_optical_drives_listed_and_films_open_in_the_player(self):
        self.f.disc_in = lambda dev: {'/dev/sr0': ('dvd', 'MY_FILM'), '/dev/sr1': ('', '')}[dev]
        flat = [{'name': 'sr0', 'path': '/dev/sr0', 'type': 'rom', 'rm': True}, {'name': 'sr1', 'path': '/dev/sr1', 'type': 'rom', 'rm': True},
                {'name': 'sdb1', 'path': '/dev/sdb1', 'type': 'part', 'rm': True, 'fstype': 'vfat'}]
        o = self.f.optical_drives(flat)
        self.assertEqual([x['device'] for x in o], ['/dev/sr0', '/dev/sr1'])
        self.assertEqual(o[0]['label'], 'My Film')
        self.assertIn('plays in Hearth Player', o[0]['sub'])
        self.assertEqual(o[0]['kind'], 'optical')
        self.assertIn('no disc', o[1]['sub'])
        self.assertEqual([x['device'] for x in self.f.unmounted_drives(flat)], ['/dev/sdb1'], 'CD/DVD drives once, not as USB drives')
        self.assertEqual(self.f.drive_kind('/dev/sr0'), ('optical', 'CD/DVD drive'))
        started = []
        self.f.spawn = lambda argv, cwd=None: started.append(argv)
        from unittest import mock
        with mock.patch.object(self.f.shutil, 'which', lambda n: '/usr/bin/' + n):
            self.assertEqual(self.f.mount('/dev/sr0'), {'ok': True, 'path': '', 'player': True})
            self.assertEqual(started, [['hearth-player', '--disc', '/dev/sr0']])
            with self.assertRaises(self.f.Fail):
                self.f.mount('/dev/sr1')


if __name__ == '__main__':
    unittest.main()
