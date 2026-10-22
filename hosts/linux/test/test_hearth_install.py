"""Tests for hearth-install (the live USB's installer): plan checks, parsing sfdisk/ostree output, fstab,
the image's kernel options. No real drives."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'live' / 'src' / 'hearth-install'
GIB = 1024 ** 3


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_install', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_install', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


H = load()
GOOD = {'mode': 'whole', 'disk': '/dev/nvme0n1', 'user': {'name': 'sam', 'full': 'Sam', 'password': 'pw'},
        'hostname': 'Sams-Deck', 'timezone': 'America/Chicago', 'edition': 'anatase'}


class BootEntries(unittest.TestCase):
    def test_reads_efibootmgr(self):
        text = ("BootCurrent: 0001\nBootOrder: 0000,0003,0001\n"
                "Boot0000* Windows Boot Manager\tHD(1,GPT,aaaa-1111,0x800,0x82000)/File(\\EFI\\Microsoft\\Boot\\bootmgfw.efi)\n"
                "Boot0003* Hearth OS\tHD(5,GPT,bbbb-2222,0x1000,0x12c000)/File(\\EFI\\fedora\\shimx64.efi)\n")
        entries, order = H.boot_entries(text)
        self.assertEqual(order, ['0000', '0003', '0001'])
        self.assertIn('bbbb-2222', entries['0003'])
        self.assertIn('Windows', entries['0000'])


class LocalImage(unittest.TestCase):
    """The full ISOs carry Hearth OS (an OCI copy): installs from it when it's the edition being installed."""
    def test_matches_edition(self):
        with tempfile.TemporaryDirectory() as d:
            oci = os.path.join(d, 'image')
            self.assertIsNone(H.local_image('ghcr.io/dragongamer9/hearth-os:latest', oci))     # small installer: none
            os.makedirs(oci)
            open(os.path.join(oci, 'index.json'), 'w').write('{}')
            open(oci + '.ref', 'w').write('ghcr.io/dragongamer9/hearth-os@sha256:abc\n')
            self.assertEqual(H.local_image(H.image('anatase'), oci), f'ostree-unverified-image:oci:{oci}:hearth')
            self.assertIsNone(H.local_image(H.image('ubuntu'), oci))          # another edition: downloaded
            self.assertIsNone(H.local_image('ghcr.io/dragongamer9/hearth:latest', oci))   # the old Bazzite image

    def test_local_progress_text(self):
        self.assertTrue(H.download_text(1e9, 9e9, 0, True).startswith('Copying Hearth OS from the USB stick'))


class EspAfterBootupd(unittest.TestCase):
    """bootupctl --device mounts the EFI partition itself and unmounts it: nothing is at TARGET/boot/efi
    afterwards. The boot entry and the boot menu use the partition partitioning made, never findmnt (which
    failed the whole install on real PCs, after the boot loader was in place)."""

    def setUp(self):
        self.calls = []
        self.orig = (H.sh, H.os.path.isdir, H.efi_loader)

        class R:
            def __init__(self, out=''):
                self.stdout, self.stderr, self.returncode = out, '', 0

        def fake_sh(cmd, check=True, input=None, **kw):
            self.calls.append(cmd)
            if cmd[0] == 'findmnt':
                raise AssertionError('findmnt asked: %s' % cmd)
            if cmd[0] == 'blkid':
                return R('aaaa-1111\n')
            if cmd[:2] == ['efibootmgr', '-v']:
                made = any(c[:3] == ['efibootmgr', '-q', '-c'] for c in self.calls)
                return R('BootOrder: 0001\nBoot0001* UEFI Misc\n' + ('Boot0002* Hearth OS HD(1,GPT,aaaa-1111)\n' if made else ''))
            return R()
        H.sh = fake_sh
        H.os.path.isdir = lambda p: True if p == '/sys/firmware/efi' else self.orig[1](p)
        H.efi_loader = lambda root: '\\EFI\\fedora\\shimx64.efi'

    def tearDown(self):
        H.sh, H.os.path.isdir, H.efi_loader = self.orig

    def test_boot_entry_from_the_known_partition(self):
        H.ensure_boot_entry('/dev/vda', '/dev/vda1')
        made = [c for c in self.calls if c[:3] == ['efibootmgr', '-q', '-c']]
        self.assertEqual(len(made), 1)
        self.assertEqual(made[0][made[0].index('-d') + 1], '/dev/vda')
        self.assertEqual(made[0][made[0].index('-p') + 1], '1')
        self.assertIn(['efibootmgr', '-q', '-o', '0002,0001'], self.calls)

    def test_entry_named_after_the_usb_is_renamed(self):
        made_first = []
        orig_sh = H.sh

        def fake(cmd, check=True, input=None, **kw):
            if cmd[:2] == ['efibootmgr', '-v'] and not any(c[:3] == ['efibootmgr', '-q', '-B'] or c[:4] == ['efibootmgr', '-q', '-b', '0003'] for c in self.calls):
                self.calls.append(cmd)
                class R: stdout, stderr, returncode = 'BootOrder: 0003\nBoot0003* Hearth OS offline installer (peppermint)\tHD(1,GPT,aaaa-1111)/\\EFI\\debian\\shimx64.efi\n', '', 0
                return R()
            return orig_sh(cmd, check, input, **kw)
        H.sh = fake
        try:
            H.ensure_boot_entry('/dev/vda', '/dev/vda1')
        finally:
            H.sh = orig_sh
        self.assertIn(['efibootmgr', '-q', '-b', '0003', '-B'], self.calls)
        made = [c for c in self.calls if c[:3] == ['efibootmgr', '-q', '-c']]
        self.assertEqual(made[0][made[0].index('-L') + 1], 'Hearth OS')

    def test_labels(self):
        self.assertEqual(H.entry_label('Boot0001* Hearth OS\tHD(1,GPT,x)/\\EFI\\fedora\\shimx64.efi'), 'Hearth OS')
        self.assertEqual(H.entry_label('Boot0002* Windows Boot Manager  HD(1,GPT,y)'), 'Windows Boot Manager')
        self.assertEqual(H.entry_label('Boot0003* Hearth installer HD(1,GPT,z)'), 'Hearth installer')
        self.assertEqual(H.entry_label('Boot0004* UEFI Misc'), 'UEFI Misc')

    def test_nvme_partition_number(self):
        H.ensure_boot_entry('/dev/nvme0n1', '/dev/nvme0n1p1')
        made = [c for c in self.calls if c[:3] == ['efibootmgr', '-q', '-c']][0]
        self.assertEqual(made[made.index('-p') + 1], '1')


class TargetSystems(unittest.TestCase):
    def test_selinux_only_where_the_new_system_has_it(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(H.selinux_on(d), 'no /etc/selinux (Ubuntu, Peppermint, CachyOS)')
            os.makedirs(f'{d}/etc/selinux')
            open(f'{d}/etc/selinux/config', 'w').write('SELINUX=disabled\n')
            self.assertFalse(H.selinux_on(d))
            open(f'{d}/etc/selinux/config', 'w').write('# x\nSELINUX=enforcing\nSELINUXTYPE=targeted\n')
            self.assertTrue(H.selinux_on(d))

    def test_no_relabel_mark_without_selinux(self):
        with tempfile.TemporaryDirectory() as d:
            H.relabel(d, ['/etc'])
            self.assertFalse(os.path.exists(f'{d}/.autorelabel') or os.path.exists(f'{d}/etc/.autorelabel'))

    def test_relabel_mark_goes_in_etc(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(f'{d}/etc/selinux')
            open(f'{d}/etc/selinux/config', 'w').write('SELINUX=enforcing\n')
            H.relabel(d, ['/etc'])    # no file_contexts: relabel at first start
            self.assertTrue(os.path.exists(f'{d}/etc/.autorelabel'))

    def test_password_hash(self):
        import shutil
        if not shutil.which('openssl'):
            self.skipTest('no openssl here')
        h = H.password_hash('hearth')
        self.assertTrue(h.startswith('$6$') and 'hearth' not in h)


class Account(unittest.TestCase):
    """The account is written into the new system's files directly (useradd --root failed from the small
    installer, chroot found no useradd, chpasswd hit the new system's PAM)."""

    def make(self, d, debian=False):
        os.makedirs(f'{d}/etc/skel')
        open(f'{d}/etc/skel/.bashrc', 'w').write('# hi\n')
        os.makedirs(f'{d}/usr/bin'); open(f'{d}/usr/bin/bash', 'w').close()
        open(f'{d}/etc/passwd', 'w').write('root:x:0:0:root:/root:/bin/bash\nnobody:x:65534:65534::/:/sbin/nologin\nold:x:1000:1000::/var/home/old:/bin/bash\n')
        open(f'{d}/etc/group', 'w').write('root:x:0:\n' + ('sudo:x:27:\n' if debian else 'wheel:x:10:old\n') + 'old:x:1000:\nweird:x:1001:\n')
        open(f'{d}/etc/shadow', 'w').write('root:*:1:0:99999:7:::\n')
        open(f'{d}/etc/gshadow', 'w').write('root:::\n' + ('sudo:!::\n' if debian else 'wheel:!::old\n'))
        open(f'{d}/etc/subuid', 'w').write('old:524288:65536\n')
        os.makedirs(f'{d}/var/home')

    def test_fedora_like(self):
        if os.geteuid() != 0:
            orig = H.os.lchown
            H.os.lchown = lambda *a: None
        try:
            with tempfile.TemporaryDirectory() as d:
                self.make(d)
                uid = H.add_account(d, 'tester', 'Tester: T', '$6$abc$def')
                self.assertEqual(uid, 1002, 'the first id free as a user and as a group')
                self.assertIn('tester:x:1002:1002:Tester  T:/var/home/tester:/bin/bash', open(f'{d}/etc/passwd').read())
                g = open(f'{d}/etc/group').read()
                self.assertIn('wheel:x:10:old,tester', g)
                self.assertIn('tester:x:1002:', g)
                self.assertRegex(open(f'{d}/etc/shadow').read(), r'tester:\$6\$abc\$def:\d+:0:99999:7:::')
                self.assertIn('wheel:!::old,tester', open(f'{d}/etc/gshadow').read())
                self.assertIn('tester:589824:65536', open(f'{d}/etc/subuid').read())
                self.assertTrue(os.path.exists(f'{d}/var/home/tester/.bashrc'))
                with self.assertRaises(H.Fail):
                    H.add_account(d, 'tester', 'Again', '')
        finally:
            if os.geteuid() != 0:
                H.os.lchown = orig

    def test_debian_like_uses_sudo(self):
        orig = H.os.lchown
        H.os.lchown = lambda *a: None
        try:
            with tempfile.TemporaryDirectory() as d:
                self.make(d, debian=True)
                H.add_account(d, 'sam', 'Sam', '')
                self.assertIn('sudo:x:27:sam', open(f'{d}/etc/group').read())
                self.assertIn('sam::', open(f'{d}/etc/shadow').read(), 'no password: empty field')
        finally:
            H.os.lchown = orig


class Plan(unittest.TestCase):
    def test_good_plan(self):
        p = H.check_plan(GOOD)
        self.assertEqual(p['hostname'], 'sams-deck')
        self.assertEqual(p['user']['name'], 'sam')

    def test_bad_user_names(self):
        for name in ('Sam', '1sam', 'root', 'liveuser', 'a b', '', 'x' * 40):
            with self.assertRaises(H.Fail, msg=name):
                H.check_plan({**GOOD, 'user': {'name': name, 'password': 'pw'}})

    def test_system_group_names_refused(self):
        for name in ('video', 'audio', 'input', 'render', 'sudo'):
            with self.assertRaises(H.Fail, msg=name):
                H.check_plan({**GOOD, 'user': {'name': name, 'password': 'pw'}})

    def test_device_name_from_a_user_name_with_underscore(self):
        self.assertEqual(H.check_plan({**GOOD, 'hostname': 'sam_r-hearth'})['hostname'], 'sam-r-hearth')
        self.assertEqual(H.check_plan({**GOOD, 'hostname': '_'})['hostname'], 'hearth')

    def test_reachable_asks_again_then_gives_up(self):
        calls = []
        orig = (H.published, H.time.sleep)
        try:
            H.time.sleep = lambda s: None
            H.published = lambda img: calls.append(img)
            self.assertIsNone(H.reachable('ghcr.io/x/y:latest'))
            self.assertEqual(len(calls), 3)
            answers = [None, True]
            H.published = lambda img: answers.pop(0)
            self.assertTrue(H.reachable('ghcr.io/x/y:latest'))
        finally:
            H.published, H.time.sleep = orig

    def test_needs_password(self):
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'user': {'name': 'sam', 'password': ''}})

    def test_no_password(self):
        p = H.check_plan({**GOOD, 'user': {'name': 'sam', 'password': '', 'nopassword': True}})
        self.assertTrue(p['user']['nopassword'])
        self.assertEqual(p['user']['password'], '')

    def test_signin_choice(self):
        self.assertEqual(H.check_plan(GOOD)['signin'], 'auto')
        self.assertEqual(H.check_plan({**GOOD, 'signin': 'always'})['signin'], 'always')
        self.assertEqual(H.check_plan({**GOOD, 'signin': 'bogus'})['signin'], 'auto')
        nop = {**GOOD, 'signin': 'always', 'user': {'name': 'sam', 'password': '', 'nopassword': True}}
        self.assertEqual(H.check_plan(nop)['signin'], 'auto')

    def test_beside_needs_room(self):
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'mode': 'beside', 'give': 10 * GIB})
        self.assertEqual(H.check_plan({**GOOD, 'mode': 'beside', 'give': 128 * GIB})['give'], 128 * GIB)

    def test_bad_timezone_and_disk(self):
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'timezone': '../../etc/shadow'})
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'disk': '/dev/sda; rm -rf /'})

    def test_default_image(self):
        p = H.check_plan({k: v for k, v in GOOD.items() if k != 'edition'})
        self.assertEqual(p['edition'], 'anatase')
        self.assertEqual(p['image'], 'ghcr.io/dragongamer9/hearth-os:latest')


class CloseVolumes(unittest.TestCase):
    def test_innermost_first_and_only_volumes(self):
        calls = []

        class R:
            def __init__(self, out=''):
                self.stdout, self.returncode = out, 0
        tree = 'sda disk\nsda1 part\nsda2 part\nsda3 part\nluks-1 crypt\nvg-root lvm\nvg-home lvm\n'

        def fake(cmd, check=True, input=None, **kw):
            calls.append(cmd)
            return R(tree if cmd[0] == 'lsblk' else '')
        orig = (H.sh, H.shutil.which)
        try:
            H.sh, H.shutil.which = fake, lambda n: '/usr/sbin/' + n
            H.close_volumes('/dev/sda')
        finally:
            H.sh, H.shutil.which = orig
        removed = [c[-1] for c in calls if c[0] == 'dmsetup']
        self.assertEqual(removed, ['vg-home', 'vg-root', 'luks-1'])


class Parsing(unittest.TestCase):
    def test_part_path(self):
        self.assertEqual(H.part_path('/dev/sda', 3), '/dev/sda3')
        self.assertEqual(H.part_path('/dev/nvme0n1', 3), '/dev/nvme0n1p3')
        self.assertEqual(H.part_path('/dev/loop0', 1), '/dev/loop0p1')

    def test_free_regions(self):
        out = ('Unpartitioned space /dev/nvme0n1: 200 GiB, 214748364800 bytes, 419430400 sectors\n'
               'Units: sectors of 1 * 512 = 512 bytes\n\n'
               '    Start        End   Sectors        Size\n'
               '     2048       4095      2048     1048576\n'
               '600000000 1019430399 419430400 214748364800\n')
        self.assertEqual(H.parse_free(out)[0], (600000000, 419430400))
        self.assertEqual(len(H.parse_free(out)), 2)

    def test_4k_sector_drive(self):
        # An NVMe drive formatted with 4096-byte sectors: sfdisk counts in those, so Hearth's partitions are sized
        # and placed in them (they used to come out 8 times too big, and too little free space was seen).
        from unittest import mock
        sent = []

        def sh(cmd, check=True, input=None, **kw):
            sent.append((cmd, input))

            class R:
                stdout, stderr, returncode = '', '', 0
            return R
        ss = 4096
        start = 256 * 300000   # 1 MiB aligned
        parts = [{'node': '/dev/nvme0n1p1', 'start': 256}, {'node': '/dev/nvme0n1p5', 'start': start},
                 {'node': '/dev/nvme0n1p6', 'start': start + H.ESP_SIZE // ss},
                 {'node': '/dev/nvme0n1p7', 'start': start + (H.ESP_SIZE + H.BOOT_SIZE) // ss}]
        with mock.patch.object(H, 'sh', sh), mock.patch.object(H, 'sector_size', lambda d: ss), \
                mock.patch.object(H, 'free_regions', lambda d: [(start - 100, 100 * GIB // ss)]), \
                mock.patch.object(H, 'release', lambda d: None), mock.patch.object(H, 'settle', lambda d: None), \
                mock.patch.object(H, 'table', lambda d: parts):
            self.assertEqual(H.partition_free('/dev/nvme0n1'), ('/dev/nvme0n1p5', '/dev/nvme0n1p6', '/dev/nvme0n1p7', True))
            layout = sent[-1][1]
            self.assertIn('start=%d, size=%d,' % (start, H.ESP_SIZE // ss), layout)
            self.assertEqual(start % (H.MIB // ss), 0)

    def test_layers_needed(self):
        self.assertEqual(H.parse_needed('layers already present: 3; layers needed: 62 (4.1 GB)'), (62, 4100000000))
        self.assertEqual(H.parse_needed('layers needed: 5 (812 MB)'), (5, 812000000))
        self.assertIsNone(H.parse_needed('Fetching layer sha256:abc'))

    def test_fstab(self):
        t = H.fstab_text('R', 'B', 'E')
        self.assertIn('UUID=R / btrfs subvol=root', t)
        self.assertIn('UUID=R /var/home btrfs subvol=home', t)
        self.assertIn('UUID=B /boot ext4', t)
        self.assertIn('UUID=E /boot/efi vfat', t)

    def test_image_kargs(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(f'{d}/usr/lib/bootc/kargs.d')
            open(f'{d}/usr/lib/bootc/kargs.d/00-nvidia.toml', 'w').write(
                'kargs = ["rd.driver.blacklist=nouveau", "nvidia-drm.modeset=1"]\n')
            open(f'{d}/usr/lib/bootc/kargs.d/10-arm.toml', 'w').write(
                'kargs = ["arm-only"]\nmatch-architectures = ["aarch64"]\n')
            self.assertEqual(H.image_kargs(d), ['rd.driver.blacklist=nouveau', 'nvidia-drm.modeset=1'])



def part(dev, fs, size_gb, partlabel='', typ=''):
    return {'dev': dev, 'fs': fs, 'label': None, 'size': size_gb * GIB, 'type': typ or H.LINUX_TYPE, 'partlabel': partlabel}


class OtherSystems(unittest.TestCase):
    def test_steamos_by_its_partitions(self):
        parts = [part('/dev/nvme0n1p1', 'vfat', 1, 'esp', H.ESP_TYPE), part('/dev/nvme0n1p2', 'vfat', 1, 'efi-A'),
                 part('/dev/nvme0n1p4', 'btrfs', 5, 'rootfs-A'), part('/dev/nvme0n1p5', 'btrfs', 5, 'rootfs-B'),
                 part('/dev/nvme0n1p8', 'ext4', 900, 'home')]
        self.assertEqual(H.systems(parts, peek=lambda *a: self.fail('no need to look inside')),
                         [{'kind': 'linux', 'name': 'SteamOS', 'part': '/dev/nvme0n1p8', 'shrink': '/dev/nvme0n1p8'}])

    def test_every_linux_on_a_drive(self):
        parts = [part('/dev/sda1', 'vfat', 1, '', H.ESP_TYPE), part('/dev/sda2', 'ext4', 50), part('/dev/sda3', 'ext4', 400),
                 part('/dev/sda4', 'btrfs', 120), part('/dev/sda5', 'xfs', 80), part('/dev/sda6', 'crypto_LUKS', 200)]
        parts[2]['uuid'] = 'home-uuid'
        rel = {'/dev/sda2': ({'NAME': 'Ubuntu', 'ID': 'ubuntu'}, 'home-uuid'), '/dev/sda4': ({'NAME': 'Fedora Linux'}, None),
               '/dev/sda5': ({'NAME': 'Rocky Linux'}, None)}
        out = H.systems(parts, peek=lambda dev, fs: rel.get(dev) if dev != '/dev/sda3' else self.fail('/home is part of Ubuntu'))
        self.assertEqual(out, [
            {'kind': 'linux', 'name': 'Ubuntu', 'part': '/dev/sda2', 'shrink': '/dev/sda3'},     # its bigger /home gives the room
            {'kind': 'linux', 'name': 'Fedora Linux', 'part': '/dev/sda4', 'shrink': '/dev/sda4'},
            {'kind': 'linux', 'name': 'Rocky Linux', 'part': '/dev/sda5', 'shrink': None},       # XFS can't shrink
            {'kind': 'linux', 'name': 'Linux (encrypted)', 'part': '/dev/sda6', 'shrink': None}])

    def test_lvm_is_found_but_not_shrunk(self):
        lv = {**part('/dev/mapper/ubuntu--vg-root', 'ext4', 200), 'lvm': True, 'type': ''}
        out = H.systems([lv], peek=lambda *a: ({'NAME': 'Ubuntu'}, None))
        self.assertEqual(out, [{'kind': 'linux', 'name': 'Ubuntu', 'part': lv['dev'], 'shrink': None}])

    def test_windows_and_linux_together_and_hearth_skipped(self):
        parts = [part('/dev/sda1', 'vfat', 1, '', H.ESP_TYPE), part('/dev/sda2', 'ntfs', 300), part('/dev/sdb1', 'ntfs', 900),
                 part('/dev/sda3', 'ext4', 2, 'boot', H.XBOOTLDR_TYPE), part('/dev/sda4', 'btrfs', 100, 'hearth'),
                 part('/dev/sda5', 'btrfs', 200)]
        out = H.systems(parts, peek=lambda dev, fs: ({'ID': 'fedora', 'NAME': 'Fedora Linux'}, None) if dev == '/dev/sda5' else self.fail(dev),
                        peek_win=lambda dev: dev == '/dev/sda2')     # sdb1 is a games drive, not Windows
        self.assertEqual([x['name'] for x in out], ['Windows', 'Fedora Linux'])
        self.assertEqual(H.systems(parts, live=True), [])

    def test_two_of_the_same(self):
        out = H.systems([part('/dev/a', 'ext4', 50), part('/dev/b', 'ext4', 50)], peek=lambda *a: ({'NAME': 'Arch Linux'}, None))
        self.assertEqual([x['name'] for x in out], ['Arch Linux', 'Arch Linux (2)'])

    def test_hibernated_windows_still_counts(self):
        out = H.systems([part('/dev/a', 'ntfs', 300)], peek_win=lambda d: None)
        self.assertEqual(out[0]['kind'], 'windows')

    def test_reading_a_system(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(f'{d}/etc'); os.makedirs(f'{d}/usr/lib')
            open(f'{d}/usr/lib/os-release', 'w').write('NAME="Linux Mint"\n')
            os.symlink('/usr/lib/os-release', f'{d}/etc/os-release')          # absolute: stays inside the system
            self.assertEqual(H.inside(d, 'etc/os-release'), f'{d}/usr/lib/os-release')
            os.symlink('/../../../../etc/passwd', f'{d}/etc/evil')
            self.assertIn(H.inside(d, 'etc/evil'), (None,))
        self.assertEqual(H.home_uuid('UUID=aaa / ext4 defaults 0 1\n# UUID=x /home\nUUID=bbb /home ext4 defaults 0 2\n'), 'bbb')
        self.assertEqual(H.home_uuid('UUID=aaa / btrfs subvol=root 0 0\n'), None)

    def test_os_release(self):
        v = H.parse_os_release('NAME="SteamOS"\nID=steamos\nPRETTY_NAME="SteamOS 3.7"\n')
        self.assertEqual(H.os_name(v), 'SteamOS')
        self.assertEqual(H.os_name(H.parse_os_release("NAME='Arch Linux'\nID=arch")), 'Arch Linux')

    def test_room_math(self):
        self.assertEqual(H.can_give(900 * GIB, 300 * GIB, 900 * GIB), 580 * GIB)
        self.assertEqual(H.can_give(100 * GIB, 95 * GIB, 100 * GIB), 0)
        self.assertEqual(H.new_fs_size(900 * GIB, 200 * GIB, 300 * GIB), 700 * GIB)
        with self.assertRaises(H.Fail):
            H.new_fs_size(900 * GIB, 600 * GIB, 300 * GIB)

    def test_plan_part(self):
        p = H.check_plan({**GOOD, 'mode': 'beside', 'give': 100 * GIB, 'part': '/dev/nvme0n1p8'})
        self.assertEqual(p['part'], '/dev/nvme0n1p8')
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'mode': 'beside', 'give': 100 * GIB, 'part': '/dev/x;rm'})


class Edition(unittest.TestCase):
    """Which Hearth OS: a fixed table of four images, Anatase by default, never a free-form image."""
    def setUp(self):
        self._v3 = H.cpu_v3
        H.cpu_v3 = lambda path='/proc/cpuinfo': True

    def tearDown(self):
        H.cpu_v3 = self._v3

    def test_the_four_images(self):
        self.assertEqual(H.image(), 'ghcr.io/dragongamer9/hearth-os:latest')
        self.assertEqual(H.image('anatase'), 'ghcr.io/dragongamer9/hearth-os:latest')
        self.assertEqual(H.image('ubuntu'), 'ghcr.io/dragongamer9/hearth-ubuntu:testing')
        self.assertEqual(H.image('cachyos'), 'ghcr.io/dragongamer9/hearth-cachyos:testing')
        self.assertEqual(H.image('peppermint'), 'ghcr.io/dragongamer9/hearth-peppermint:testing')
        self.assertEqual(sorted(H.EDITIONS), ['anatase', 'cachyos', 'peppermint', 'ubuntu'])
        for bad in ('hearth', 'hearth-nvidia', 'bazzite', '../x'):
            with self.assertRaises(H.Fail, msg=bad):
                H.image(bad)

    def test_plan_editions(self):
        for ed in H.EDITIONS:
            p = H.check_plan({**GOOD, 'edition': ed})
            self.assertEqual((p['edition'], p['image']), (ed, H.EDITIONS[ed]['image']))
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'edition': 'arch'})
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'edition': ['anatase']})

    def test_never_a_free_form_image(self):
        base = {k: v for k, v in GOOD.items() if k != 'edition'}
        # Exactly one of the table's images is still taken (older callers), as that edition.
        self.assertEqual(H.check_plan({**base, 'image': 'ghcr.io/dragongamer9/hearth-ubuntu:testing'})['edition'], 'ubuntu')
        for bad in ('ghcr.io/dragongamer9/hearth:latest', 'ghcr.io/dragongamer9/hearth-nvidia:latest',
                    'ghcr.io/dragongamer9/hearth-ubuntu:latest', 'docker.io/evil/os:latest', 'ghcr.io/dragongamer9/hearth-os@sha256:abc'):
            with self.assertRaises(H.Fail, msg=bad):
                H.check_plan({**base, 'image': bad})
        with self.assertRaises(H.Fail):   # the edition and the image disagree
            H.check_plan({**GOOD, 'image': 'ghcr.io/dragongamer9/hearth-ubuntu:testing'})

    def test_cachyos_needs_x86_64_v3(self):
        H.cpu_v3 = lambda path='/proc/cpuinfo': False
        with self.assertRaises(H.Fail):
            H.check_plan({**GOOD, 'edition': 'cachyos'})
        self.assertEqual(H.check_plan({**GOOD, 'edition': 'ubuntu'})['edition'], 'ubuntu')
        rows = {r['id']: r for r in H.edition_choices(list(H.EDITIONS), False, False)}
        self.assertFalse(rows['cachyos']['allowed'])
        self.assertTrue(rows['anatase']['allowed'] and rows['ubuntu']['allowed'] and rows['peppermint']['allowed'])

    def test_reads_cpu_flags(self):
        v3 = 'processor\t: 0\nflags\t\t: fpu sse2 avx2 bmi2 fma movbe\nprocessor\t: 1\nflags\t\t: fpu avx2 bmi2 fma movbe\n'
        self.assertTrue(H.x86_64_v3(v3))
        self.assertFalse(H.x86_64_v3(v3 + 'processor\t: 2\nflags\t\t: fpu sse2 avx\n'))
        self.assertFalse(H.x86_64_v3(''))
        with tempfile.TemporaryDirectory() as d:
            open(f'{d}/cpuinfo', 'w').write(v3)
            self.assertTrue(self._v3(f'{d}/cpuinfo'))
            self.assertFalse(self._v3(f'{d}/missing'))

    def test_secure_boot_warns_for_cachyos_only(self):
        self.assertIsNotNone(H.edition_warning('cachyos', True))
        for ed in ('anatase', 'ubuntu', 'peppermint'):
            self.assertIsNone(H.edition_warning(ed, True), ed)
        self.assertIsNone(H.edition_warning('cachyos', False))
        self.assertIsNone(H.edition_warning('cachyos', None))
        rows = {r['id']: r for r in H.edition_choices(list(H.EDITIONS), True, True)}
        self.assertTrue(rows['cachyos']['warning'] and rows['cachyos']['allowed'])
        self.assertIsNone(rows['ubuntu']['warning'])
        with tempfile.TemporaryDirectory() as d:
            open(f'{d}/sb', 'wb').write(bytes([6, 0, 0, 0, 1]))
            self.assertTrue(H.secure_boot(f'{d}/sb'))
            open(f'{d}/sb', 'wb').write(bytes([6, 0, 0, 0, 0]))
            self.assertFalse(H.secure_boot(f'{d}/sb'))

    def test_what_each_usb_offers(self):
        with tempfile.TemporaryDirectory() as d:
            f = f'{d}/hearth-live'
            open(f, 'w').write('installer\n')
            self.assertEqual(H.this_usb(f), 'installer')
            open(f, 'w').write('auto\n')       # older installer USBs
            self.assertEqual(H.this_usb(f), 'installer')
            open(f, 'w').write('anatase\n')
            self.assertEqual(H.this_usb(f), 'anatase')
            self.assertEqual(H.this_usb(f'{d}/missing'), 'anatase')
            open(f, 'w').write('ubuntu\n')        # the Ubuntu offline installer
            self.assertEqual(H.this_usb(f), 'ubuntu')
            open(f, 'w').write('something\n')
            self.assertEqual(H.this_usb(f), 'anatase')
        self.assertEqual(H.editions_offered('ubuntu'), ['ubuntu'])
        self.assertEqual((H.usb_edition('ubuntu'), H.usb_edition('installer'), H.usb_edition('anatase')), ('ubuntu', 'anatase', 'anatase'))
        self.assertEqual(H.published_all(['peppermint']), {}, 'an offline USB has its edition with it')
        self.assertEqual(H.editions_offered('installer'), ['anatase', 'ubuntu', 'cachyos', 'peppermint'])
        self.assertEqual(H.editions_offered('anatase'), ['anatase'])
        rows = H.edition_choices(H.editions_offered('installer'), True, False, {'peppermint': False, 'ubuntu': None})
        self.assertEqual([r['id'] for r in rows if r['recommended']], ['anatase'])
        self.assertEqual([r['id'] for r in rows if r['beta']], ['ubuntu', 'cachyos', 'peppermint'])
        by = {r['id']: r for r in rows}
        self.assertEqual((by['peppermint']['allowed'], by['peppermint']['reason']), (False, 'Not ready yet'))
        self.assertTrue(by['ubuntu']['allowed'])          # couldn't ask (offline): still allowed
        self.assertEqual(H.published_all(['anatase']), {})  # the live USB's own: not asked


class BootMenu(unittest.TestCase):
    def test_finds_other_loaders(self):
        with tempfile.TemporaryDirectory() as d:
            for rel in ('EFI/Microsoft/Boot/bootmgfw.efi', 'EFI/steamos/steamcl.efi', 'EFI/ubuntu/shimx64.efi',
                        'EFI/fedora/shimx64.efi', 'EFI/BOOT/BOOTX64.EFI', 'EFI/empty/readme.txt'):
                os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
                open(os.path.join(d, rel), 'w').close()
            self.assertEqual(H.find_loaders(d), [('Windows', '/EFI/Microsoft/Boot/bootmgfw.efi'),
                                                 ('SteamOS', '/EFI/steamos/steamcl.efi'), ('Ubuntu', '/EFI/ubuntu/shimx64.efi')])

    def test_menu_text(self):
        t = H.menu_text([('SteamOS', 'ABCD-1234', '/EFI/steamos/steamcl.efi'), ("Bad'name", 'X', '/EFI/x/grubx64.efi')])
        self.assertIn("menuentry 'SteamOS' --class os {", t)
        self.assertIn('search --no-floppy --fs-uuid --set=root ABCD-1234', t)
        self.assertIn('chainloader /EFI/steamos/steamcl.efi', t)
        self.assertIn("menuentry 'Badname'", t)


if __name__ == '__main__':
    unittest.main()


class DownloadProgress(unittest.TestCase):
    def test_counts_bytes_received_but_not_loopback(self):
        with tempfile.TemporaryDirectory() as d:
            f = os.path.join(d, 'dev')
            pathlib.Path(f).write_text('Inter-|   Receive\n face |bytes    packets\n    lo: 5000 10 0 0 0 0 0 0 5000 0 0 0 0 0 0 0\n'
                                        '  wlp2s0: 1200000 900 0 0 0 0 0 0 3000 0 0 0 0 0 0 0\n  eth0: 300 2 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n')
            self.assertEqual(H.rx_bytes(f), 1200300)

    def test_says_how_much_and_how_fast(self):
        self.assertEqual(H.download_text(3_200_000_000, 9_600_000_000, 12_400_000), 'Downloading Hearth OS · 3.2 of 9.6 GB · 12.4 MB/s')
        self.assertEqual(H.download_text(0, 0, 0), 'Downloading Hearth OS')


def load_boot_first():
    p = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-boot-first'
    loader = importlib.machinery.SourceFileLoader('hearth_boot_first', str(p))
    m = importlib.util.module_from_spec(importlib.util.spec_from_loader('hearth_boot_first', loader))
    loader.exec_module(m)
    return m


class BootFirst(unittest.TestCase):
    """hearth-boot-first (every start): keeps Hearth first, without piling up boot entries."""
    LOADER = '\\EFI\\fedora\\shimx64.efi'

    def test_found_by_partition(self):
        bf = load_boot_first()
        t = ('BootOrder: 0000,0003\nBoot0000* Windows Boot Manager\tHD(1,GPT,aaaa,0x800,0x32000)/\\EFI\\Microsoft\\Boot\\bootmgfw.efi\n'
             'Boot0003* Hearth OS\tHD(5,GPT,BBBB-1,0x1,0x2)/\\EFI\\fedora\\shimx64.efi\n')
        self.assertEqual(bf.plan(t, 'bbbb-1', self.LOADER), ('0003', ['0003', '0000']))

    def test_rewritten_entry_isnt_made_again(self):
        # The firmware kept the entry but dropped the partition's id: it's still Hearth's (no new one each start).
        bf = load_boot_first()
        t = ('BootOrder: 0003,0000\nBoot0000* Windows Boot Manager\tHD(1,GPT,aaaa)/\\EFI\\Microsoft\\Boot\\bootmgfw.efi\n'
             'Boot0003* Hearth OS\tVenHw(99e275e7-75a0-4b37-a2e6-c5385e6c00cb)/File(\\EFI\\fedora\\shimx64.efi)\n')
        self.assertEqual(bf.plan(t, 'bbbb-1', self.LOADER), ('0003', None))
        # Another system's "Hearth OS"-less entry with the same loader isn't taken for Hearth's.
        t2 = 'BootOrder: 0002\nBoot0002* Fedora\tHD(2,GPT,cccc)/\\EFI\\fedora\\shimx64.efi\n'
        self.assertEqual(bf.plan(t2, 'bbbb-1', self.LOADER), (None, None))

    def test_no_efibootmgr_changes_nothing(self):
        from unittest import mock
        bf = load_boot_first()
        with mock.patch.object(bf.os.path, 'isdir', lambda p: True), mock.patch.object(bf.os.path, 'exists', lambda p: False), \
                mock.patch.object(bf.shutil, 'which', lambda n: None), mock.patch('builtins.print'):
            self.assertEqual(bf.main(), 0)


class GrubFolder(unittest.TestCase):
    """custom.cfg (Windows, SteamOS... in Hearth's boot menu) goes where the edition's GRUB menu reads it."""

    def test_where_custom_cfg_goes(self):
        with tempfile.TemporaryDirectory() as boot:
            # Nothing there yet: bootupd's layout (all editions).
            self.assertEqual(H.grub_dir(boot), os.path.join(boot, 'grub2'))
            os.makedirs(os.path.join(boot, 'grub'))
            open(os.path.join(boot, 'grub', 'grub.cfg'), 'w').close()
            # A distribution-style GRUB (grub/grub.cfg only): there.
            self.assertEqual(H.grub_dir(boot), os.path.join(boot, 'grub'))
            os.makedirs(os.path.join(boot, 'grub2'))
            open(os.path.join(boot, 'grub2', 'grub.cfg'), 'w').close()
            # bootupd's static menu (Fedora's, Debian's, Ubuntu's and Arch's GRUB alike) wins.
            self.assertEqual(H.grub_dir(boot), os.path.join(boot, 'grub2'))
