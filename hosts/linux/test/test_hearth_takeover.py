"""Tests for hearth-takeover's plan: which partitions on Hearth's drive can go, and how the space is used.
Uses made-up partition tables (no real drives)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import pathlib
import unittest

PATH = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-takeover'
GIB = 1024 ** 3 // 512   # sectors


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_takeover', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_takeover', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def table(parts, total_gib=500):
    """parts: (num, start_gib, size_gib, type, fstype, label, mounts)"""
    lsblk = {'blockdevices': [{'name': 'nvme0n1', 'path': '/dev/nvme0n1', 'type': 'disk', 'children': [
        {'name': f'nvme0n1p{n}', 'path': f'/dev/nvme0n1p{n}', 'pkname': 'nvme0n1', 'fstype': fs, 'label': label,
         'mountpoints': mounts, 'type': 'part'} for n, _, _, _, fs, label, mounts in parts]}]}
    sf = {'partitiontable': {'sectorsize': 512, 'lastlba': total_gib * GIB, 'partitions': [
        {'node': f'/dev/nvme0n1p{n}', 'start': int(st * GIB) + 2048, 'size': int(sz * GIB), 'type': t} for n, st, sz, t, _, _, _ in parts]}}
    return lsblk, sf


class Plan(unittest.TestCase):
    def setUp(self):
        self.t = load()

    def windows_first(self):
        # The usual result of installing Hearth next to Windows: EFI, MSR, C:, recovery, then Hearth.
        return table([
            (1, 0, 0.1, self.t.ESP_TYPE, 'vfat', 'SYSTEM', ['/boot/efi']),
            (2, 0.1, 0.016, self.t.MSR_TYPE, '', '', []),
            (3, 0.12, 200, self.t.BASIC_DATA, 'ntfs', 'Windows', ['/run/hearth-import/nvme0n1p3']),
            (4, 200.2, 0.8, self.t.WINRE_TYPE, 'ntfs', 'Recovery', []),
            (5, 201, 1, self.t.LINUX_FS_TYPE, 'ext4', 'boot', ['/boot']),
            (6, 202, 298, self.t.LINUX_FS_TYPE, 'btrfs', 'fedora', ['/sysroot']),
        ])

    def test_windows_partitions_offered_hearth_and_efi_kept(self):
        lsblk, sf = self.windows_first()
        p = self.t.plan_data(lsblk, sf, root_devs=['/dev/nvme0n1p6'], efi_entries=[{'id': '0000', 'name': 'Windows Boot Manager'}])
        by = {x['num']: x for x in p['parts']}
        self.assertFalse(by[1]['removable'])                      # EFI
        self.assertTrue(by[2]['removable'] and by[2]['chosen'])   # MSR
        self.assertTrue(by[3]['chosen'] and by[3]['kind'] == 'windows')
        self.assertEqual(by[3]['mounted'], [])                    # the importer's read-only mount doesn't count
        self.assertTrue(by[4]['chosen'])                          # recovery
        self.assertFalse(by[5]['removable'])                      # /boot
        self.assertFalse(by[6]['removable'])                      # Hearth
        self.assertEqual(p['disk'], '/dev/nvme0n1')

    def test_space_before_hearth_becomes_a_new_part_of_hearths_file_system(self):
        lsblk, sf = self.windows_first()
        p = self.t.plan_data(lsblk, sf, root_devs=['/dev/nvme0n1p6'], efi_entries=[])
        grow, add = self.t.what_grows(p, ['/dev/nvme0n1p2', '/dev/nvme0n1p3', '/dev/nvme0n1p4'])
        self.assertEqual(grow, [])
        self.assertEqual(len(add), 1)                              # MSR + C: + recovery merge into one stretch
        self.assertGreater(add[0]['bytes'], 200 * 1024 ** 3)
        self.assertEqual(add[0]['after'], '/dev/nvme0n1p1')

    def test_space_after_hearth_grows_it(self):
        lsblk, sf = table([
            (1, 0, 0.5, self.t.ESP_TYPE, 'vfat', '', ['/boot/efi']),
            (2, 0.5, 1, self.t.LINUX_FS_TYPE, 'ext4', '', ['/boot']),
            (3, 1.5, 100, self.t.LINUX_FS_TYPE, 'btrfs', '', ['/sysroot']),
            (4, 101.5, 398, self.t.LINUX_FS_TYPE, 'ext4', 'steamos-home', []),
        ])
        p = self.t.plan_data(lsblk, sf, root_devs=['/dev/nvme0n1p3'], efi_entries=[])
        by = {x['num']: x for x in p['parts']}
        self.assertEqual(by[4]['kind'], 'linux')
        grow, add = self.t.what_grows(p, ['/dev/nvme0n1p4'])
        self.assertEqual([g['after'] for g in grow], ['/dev/nvme0n1p3'])
        self.assertEqual(add, [])

    def test_data_partitions_not_chosen_by_default(self):
        lsblk, sf = table([
            (1, 0, 0.5, self.t.ESP_TYPE, 'vfat', '', ['/boot/efi']),
            (2, 0.5, 100, self.t.LINUX_FS_TYPE, 'btrfs', '', ['/sysroot']),
            (3, 100.5, 300, self.t.BASIC_DATA, 'exfat', 'Games', []),
        ])
        p = self.t.plan_data(lsblk, sf, root_devs=['/dev/nvme0n1p2'], efi_entries=[])
        d = [x for x in p['parts'] if x['num'] == 3][0]
        self.assertTrue(d['removable'])
        self.assertFalse(d['chosen'])


if __name__ == '__main__':
    unittest.main()
