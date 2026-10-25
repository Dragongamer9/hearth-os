"""Tests for hearth-usb, the root half of Hearth Disks' "Make a bootable USB": which drives it will write to (a fake
/sys and /proc/self/mountinfo: USB sticks and SD cards yes; the drive Hearth runs from, internal drives, eMMC,
partitions, swapped or busy drives no), its options, and the dd-style write with read-back against a plain file.
Real writes to disk images run in CI (.github/workflows/disks.yml, hosts/hearth-disks/tests/usb_write.rs)."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import io
import os
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
PATH = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-usb'
ADMIN = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'
GIB = 1024 ** 3


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_usb', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_usb', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class FakeSys:
    """A /sys with: nvme0n1 (Hearth: p1 EFI, p2 /boot, p3 LUKS → dm-0 mounted at /sysroot), sda (internal SATA,
    a data partition), sdb (USB stick, a mounted partition optional), sdc (USB stick, read-only), mmcblk0 (SD card),
    mmcblk1 (soldered eMMC), sdd (a second device of Hearth's btrfs, on USB), loop3 (a disk image)."""

    def __init__(self, root):
        self.root = pathlib.Path(root)
        self.sys = self.root / 'sys'
        self.n = 0
        self.mounts = []

    def disk(self, name, bus, size_gib, removable=False, ro=False, mmc_type='', model='', parts=()):
        dev = self.sys / 'devices' / bus / 'block' / name
        dev.mkdir(parents=True)
        (dev / 'size').write_text(str(size_gib * GIB // 512))
        (dev / 'removable').write_text('1' if removable else '0')
        (dev / 'ro').write_text('1' if ro else '0')
        (dev / 'device').mkdir()
        (dev / 'device' / 'model').write_text(model)
        if mmc_type:
            (dev / 'device' / 'type').write_text(mmc_type)
        (dev / 'holders').mkdir()
        self.link(name, dev)
        for p in parts:
            pd = dev / p
            pd.mkdir()
            (pd / 'partition').write_text('1')
            (pd / 'holders').mkdir()
            self.link(p, pd)
        return dev

    def link(self, name, target):
        cb = self.sys / 'class' / 'block'
        cb.mkdir(parents=True, exist_ok=True)
        (cb / name).symlink_to(target)
        db = self.sys / 'dev' / 'block'
        db.mkdir(parents=True, exist_ok=True)
        self.n += 1
        (db / f'8:{self.n}').symlink_to(target)
        setattr(self, 'num_' + name.replace('-', '_'), f'8:{self.n}')

    def dm(self, name, slaves):
        dev = self.sys / 'devices' / 'virtual' / 'block' / name
        (dev / 'slaves').mkdir(parents=True)
        (dev / 'holders').mkdir()
        for s in slaves:
            (dev / 'slaves' / s).symlink_to(self.sys / 'class' / 'block' / s)
            (self.sys / 'class' / 'block' / s / 'holders').mkdir(exist_ok=True)
            ((self.sys / 'class' / 'block' / s / 'holders') / name).symlink_to(dev)
        self.link(name, dev)

    def mount(self, name, mp, fstype='ext4', src=None):
        self.mounts.append(f'{len(self.mounts) + 20} 1 {getattr(self, "num_" + name.replace("-", "_"))} / {mp} rw - {fstype} {src or "/dev/" + name} rw')

    def write(self):
        (self.root / 'mountinfo').write_text('\n'.join(self.mounts) + '\n')
        (self.root / 'swaps').write_text('Filename Type Size Used Priority\n')


def setup(t, extra=None):
    m = load()
    tmp = tempfile.mkdtemp()
    f = FakeSys(tmp)
    pci = 'pci0000:00/0000:00:1d.0'
    f.disk('nvme0n1', pci + '/nvme/nvme0', 512, parts=('nvme0n1p1', 'nvme0n1p2', 'nvme0n1p3'))
    f.dm('dm-0', ['nvme0n1p3'])
    f.disk('sda', pci + '/ata1/host0/target0:0:0/0:0:0:0', 1000, parts=('sda1',))
    f.disk('sdb', pci + '/usb1/1-1/1-1:1.0/host6/target6:0:0/6:0:0:0', 16, removable=True, model='Cruzer Blade', parts=('sdb1',))
    f.disk('sdc', pci + '/usb1/1-2/1-2:1.0/host7/target7:0:0/7:0:0:0', 8, removable=True, ro=True)
    f.disk('mmcblk0', pci + '/mmc_host/mmc0/mmc0:aaaa', 64, mmc_type='SD', parts=('mmcblk0p1',))
    f.disk('mmcblk1', pci + '/mmc_host/mmc1/mmc1:0001', 128, mmc_type='MMC')
    f.disk('sdd', pci + '/usb2/2-1/2-1:1.0/host8/target8:0:0/8:0:0:0', 32, removable=True, parts=('sdd1',))
    f.disk('loop3', 'virtual', 1)
    f.mount('dm-0', '/sysroot', 'btrfs')
    f.mount('nvme0n1p2', '/boot', 'ext4')
    f.mount('nvme0n1p1', '/boot/efi', 'vfat')
    # Hearth's btrfs also has a device on sdd1 (Make Hearth bigger on a USB drive): listed by the kernel.
    bt = f.sys / 'fs' / 'btrfs' / 'abcd-uuid' / 'devices'
    bt.mkdir(parents=True)
    (bt / 'dm-0').symlink_to(f.sys / 'class' / 'block' / 'dm-0')
    (bt / 'sdd1').symlink_to(f.sys / 'class' / 'block' / 'sdd1')
    if extra:
        extra(f)
    f.write()
    m.SYS = str(f.sys)
    m.MOUNTINFO = str(f.root / 'mountinfo')
    m.SWAPS = str(f.root / 'swaps')
    m.is_block = lambda dev: True
    m.serial = lambda name: {'sdb': ['AA0011', 'SanDisk_Cruzer_Blade_AA0011'], 'mmcblk0': ['0x1234']}.get(name, [])
    return m


class Devices(unittest.TestCase):
    def test_usb_stick_and_sd_card_are_allowed(self):
        m = setup(self)
        info = m.check_device('/dev/sdb', 16 * GIB, 'AA0011')
        self.assertTrue(info['removable'] and info['usb'])
        self.assertEqual(info['model'], 'Cruzer Blade')
        self.assertEqual(info['partitions'], ['/dev/sdb1'])
        self.assertTrue(m.check_device('/dev/sdb', 16 * GIB, 'SanDisk_Cruzer_Blade_AA0011'))
        info = m.check_device('/dev/mmcblk0', 64 * GIB)
        self.assertTrue(info['sd'])

    def test_the_system_drive_is_never_allowed(self):
        m = setup(self)
        self.assertIn('nvme0n1', m.system_disks())
        for allow in (False, True):
            with self.assertRaisesRegex(m.Refuse, 'running from this drive'):
                m.check_device('/dev/nvme0n1', 512 * GIB, allow_internal=allow)

    def test_a_drive_holding_part_of_the_system_btrfs_is_the_system(self):
        m = setup(self)
        self.assertIn('sdd', m.system_disks())
        with self.assertRaisesRegex(m.Refuse, 'running from this drive'):
            m.check_device('/dev/sdd', 32 * GIB, allow_internal=True)

    def test_the_live_medium_is_the_system(self):
        def live(f):
            f.mount('sdb1', '/run/initramfs/live', 'iso9660')
        m = setup(self, live)
        with self.assertRaisesRegex(m.Refuse, 'running from this drive'):
            m.check_device('/dev/sdb', 16 * GIB)

    def test_swap_on_a_usb_drive_is_the_system(self):
        m = setup(self)
        pathlib.Path(m.SWAPS).write_text('Filename Type Size Used Priority\n/dev/mmcblk0p1 partition 1 0 -2\n')
        with self.assertRaisesRegex(m.Refuse, 'running from this drive'):
            m.check_device('/dev/mmcblk0', 64 * GIB)

    def test_internal_drives_need_show_all(self):
        m = setup(self)
        with self.assertRaisesRegex(m.Refuse, 'internal drive'):
            m.check_device('/dev/sda', 1000 * GIB)
        self.assertEqual(m.check_device('/dev/sda', 1000 * GIB, allow_internal=True)['device'], '/dev/sda')
        with self.assertRaisesRegex(m.Refuse, 'internal drive'):
            m.check_device('/dev/mmcblk1', 128 * GIB)   # soldered eMMC isn't an SD card

    def test_nvme_needs_show_all(self):
        m = setup(self)
        with self.assertRaisesRegex(m.Refuse, 'NVMe'):
            m.check_device('/dev/nvme1n1', 1)

    def test_partitions_links_and_odd_names_are_refused(self):
        m = setup(self)
        for dev in ('/dev/sdb1', '/dev/mmcblk0p1', '/dev/disk/by-id/usb-x', '/dev/../dev/sdb', 'sdb', '/dev/sdb ', '/dev/sr0', '/dev/dm-0', '', None):
            with self.assertRaises(m.Refuse, msg=repr(dev)):
                m.check_device(dev, 16 * GIB)
        m.is_block = lambda dev: False
        with self.assertRaisesRegex(m.Refuse, 'isn\'t a drive'):
            m.check_device('/dev/sdb', 16 * GIB)

    def test_a_swapped_drive_is_refused(self):
        m = setup(self)
        with self.assertRaisesRegex(m.Refuse, 'size is different'):
            m.check_device('/dev/sdb', 32 * GIB)
        with self.assertRaisesRegex(m.Refuse, 'serial number'):
            m.check_device('/dev/sdb', 16 * GIB, 'OTHER1')

    def test_read_only_and_unplugged(self):
        m = setup(self)
        with self.assertRaisesRegex(m.Refuse, 'read-only'):
            m.check_device('/dev/sdc', 8 * GIB)
        with self.assertRaisesRegex(m.Refuse, 'isn\'t there'):
            m.check_device('/dev/sdz', 8 * GIB)

    def test_busy_drives_are_refused(self):
        def mounted(f):
            f.mount('sdb1', '/run/media/you/STICK', 'vfat')
        m = setup(self, mounted)
        with self.assertRaisesRegex(m.Refuse, 'in use'):
            m.check_device('/dev/sdb', 16 * GIB)
        # ...except our own mount, for finishing.
        self.assertTrue(m.check_device('/dev/sdb', 16 * GIB, allow_mount='/run/media/you/STICK'))

    def test_held_by_an_open_encrypted_partition(self):
        def held(f):
            f.dm('dm-5', ['sdb1'])
        m = setup(self, held)
        with self.assertRaisesRegex(m.Refuse, 'in use'):
            m.check_device('/dev/sdb', 16 * GIB)

    def test_disk_images_only_in_ci(self):
        m = setup(self)
        with self.assertRaisesRegex(m.Refuse, 'Disk images'):
            m.check_device('/dev/loop3', 1 * GIB, allow_loop=False)
        self.assertTrue(m.check_device('/dev/loop3', 1 * GIB, allow_loop=True)['loop'])


class Options(unittest.TestCase):
    def test_parse(self):
        m = load()
        dev, o = m.parse(['/dev/sdb', '--expect-size', '123', '--size', '45', '--expect-serial', 'AA 00-1'])
        self.assertEqual((dev, o['--expect-size'], o['--size']), ('/dev/sdb', '123', '45'))
        for bad in ([], ['--expect-size', '1'], ['/dev/sdb', '--expect-size', 'x'], ['/dev/sdb', '--rm', '-rf'],
                    ['/dev/sdb', '--expect-size', '1', '--expect-size', '2'], ['/dev/sdb', '--expect-serial', 'a;b'],
                    ['/dev/sdb', '--size']):
            with self.assertRaises(m.Refuse, msg=repr(bad)):
                m.parse(bad)

    def test_labels_and_partition_names(self):
        m = load()
        self.assertTrue(m.LABEL_RE.match('WIN11_2600'))
        for bad in ('', 'win11', 'TOO_LONG_LABEL', 'A B', 'A/B'):
            self.assertFalse(m.LABEL_RE.match(bad), bad)
        self.assertEqual(m.part_path('/dev/sdb'), '/dev/sdb1')
        self.assertEqual(m.part_path('/dev/mmcblk0'), '/dev/mmcblk0p1')
        self.assertEqual(m.part_path('/dev/loop3'), '/dev/loop3p1')

    def test_hearth_admin_hands_usb_commands_to_hearth_usb(self):
        text = ADMIN.read_text()
        self.assertIn("elif cmd in ('usb-check', 'usb-raw', 'usb-prepare', 'usb-finish'):", text)
        self.assertIn("'hearth-usb'", text)
        self.assertTrue(os.access(PATH, os.X_OK), 'hearth-usb must be executable')


class RawWrite(unittest.TestCase):
    def test_writes_and_reads_back(self):
        m = load()
        m.reread = lambda dev: None
        tmp = tempfile.mkdtemp()
        dev = os.path.join(tmp, 'disk')
        with open(dev, 'wb') as f:
            f.truncate(64 * 1024 * 1024)
        img = os.urandom(5 * 1024 * 1024 + 123)
        # A plain file can't be opened O_EXCL without O_CREAT on every system: open it the plain way here.
        real_open = os.open
        m.os.open = lambda p, flags, *a: real_open(p, flags & ~os.O_EXCL, *a)
        try:
            out = m.cmd_raw(dev, {'size': 64 * 1024 * 1024}, len(img), io.BytesIO(img))
        finally:
            m.os.open = real_open
        self.assertTrue(out['ok'])
        self.assertEqual(out['written'], len(img))
        self.assertEqual(out['writtenSha256'], hashlib.sha256(img).hexdigest())
        self.assertEqual(out['readbackSha256'], hashlib.sha256(img).hexdigest())
        with open(dev, 'rb') as f:
            self.assertEqual(f.read(len(img)), img)

    def test_refuses_a_short_or_too_big_image(self):
        m = load()
        m.reread = lambda dev: None
        tmp = tempfile.mkdtemp()
        dev = os.path.join(tmp, 'disk')
        with open(dev, 'wb') as f:
            f.truncate(8 * 1024 * 1024)
        real_open = os.open
        m.os.open = lambda p, flags, *a: real_open(p, flags & ~os.O_EXCL, *a)
        try:
            with self.assertRaisesRegex(m.Refuse, 'stopped after'):
                m.cmd_raw(dev, {'size': 8 * 1024 * 1024}, 4 * 1024 * 1024, io.BytesIO(b'x' * 1000))
            with self.assertRaisesRegex(m.Refuse, 'bigger than the drive'):
                m.cmd_raw(dev, {'size': 8 * 1024 * 1024}, 9 * 1024 * 1024, io.BytesIO(b''))
            with self.assertRaisesRegex(m.Refuse, 'bigger than the drive'):
                m.cmd_raw(dev, {'size': 8 * 1024 * 1024}, 0, io.BytesIO(b'y' * (9 * 1024 * 1024)))
        finally:
            m.os.open = real_open


if __name__ == '__main__':
    unittest.main()
