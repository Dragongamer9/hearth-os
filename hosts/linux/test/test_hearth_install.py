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
        'hostname': 'Sams-Deck', 'timezone': 'America/Chicago', 'image': 'ghcr.io/dragongamer9/hearth:latest'}


class Plan(unittest.TestCase):
    def test_good_plan(self):
        p = H.check_plan(GOOD)
        self.assertEqual(p['hostname'], 'sams-deck')
        self.assertEqual(p['user']['name'], 'sam')

    def test_bad_user_names(self):
        for name in ('Sam', '1sam', 'root', 'liveuser', 'a b', '', 'x' * 40):
            with self.assertRaises(H.Fail, msg=name):
                H.check_plan({**GOOD, 'user': {'name': name, 'password': 'pw'}})

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
        p = H.check_plan({k: v for k, v in GOOD.items() if k != 'image'})
        self.assertTrue(p['image'].startswith('ghcr.io/dragongamer9/hearth'))


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
