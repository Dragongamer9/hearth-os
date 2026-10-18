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


if __name__ == '__main__':
    unittest.main()
