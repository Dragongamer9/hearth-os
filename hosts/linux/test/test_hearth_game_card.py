"""hearth-game-card (docs/GAME-CARD.md): the handheld's microSD card mounted by itself and given to Steam (the owner's
Legion Go S, Nov 28, 2026). Never part of the PC's start (red flag 2)."""
import importlib.machinery
import importlib.util
import pathlib
import pwd
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SYS = ROOT / 'os/system_files/usr'
loader = importlib.machinery.SourceFileLoader('hgc', str(SYS / 'libexec/nimbus/hearth-game-card'))
spec = importlib.util.spec_from_loader(loader.name, loader)
gc = importlib.util.module_from_spec(spec)
loader.exec_module(gc)
U = pwd.struct_passwd(('chris', 'x', 1000, 1000, '', '/home/chris', '/bin/bash'))


class GameCard(unittest.TestCase):
    def test_where_and_how(self):
        self.assertEqual(gc.mount_point('mmcblk0p1', {'LABEL': 'USD00', 'UUID': 'u'}, U), '/run/media/chris/USD00')
        self.assertEqual(gc.mount_point('mmcblk0p1', {'UUID': '1234-AB'}, U), '/run/media/chris/1234-AB')
        self.assertEqual(gc.mount_point('mmcblk0p1', {'LABEL': '../..'}, U), '/run/media/chris/mmcblk0p1')
        self.assertEqual(gc.options('ext4', U), 'noatime,nosuid,nodev')
        self.assertIn('uid=1000,gid=1000', gc.options('exfat', U))

    def test_only_card_partitions(self):
        self.assertEqual(gc.mount('sda1'), 0)
        self.assertIsNone(gc.NAME_RE.fullmatch('mmcblk0'))
        self.assertIsNotNone(gc.NAME_RE.fullmatch('mmcblk0p1'))

    def test_never_part_of_the_start(self):
        unit = (SYS / 'lib/systemd/system/hearth-game-card@.service').read_text()
        self.assertNotRegex(unit, r'(?m)^\[Install\]')
        self.assertIn('BindsTo=dev-%i.device', unit)
        rule = (SYS / 'lib/udev/rules.d/70-hearth-game-card.rules').read_text()
        self.assertIn('KERNEL=="mmcblk[0-9]p[0-9]*"', rule)
        self.assertIn('hearth-game-card@%k.service', rule)


if __name__ == '__main__':
    unittest.main()
