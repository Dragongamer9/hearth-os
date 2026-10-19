"""Tests for hearth-space (Make Hearth bigger, Beta Experimental): reading the tools' output, the size math, which
partitions may give space, the plan, and running it against a fake partition table, including undoing the
steps in the right order when one fails. No real drives; every command goes through a fake runner."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
PATH = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-space'
ADMIN = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'
INSTALL = ROOT / 'os' / 'live' / 'src' / 'hearth-install'
PREP = ROOT / 'os' / 'live' / 'src' / 'hearth-live-prep'
MIB = 1024 ** 2
GIB = 1024 ** 3
SEC = GIB // 512   # sectors per GiB


def load(path=PATH, name='hearth_space'):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


NTFS_INFO = """ntfsresize v2022.10.3 (libntfs-3g)
Device name        : /dev/nvme0n1p3
NTFS volume version: 3.1
Cluster size       : 4096 bytes
Current volume size: 214748360704 bytes (214749 MB)
Current device size: 214748364800 bytes (214749 MB)
Checking filesystem consistency ...
Accounting clusters ...
Space in use       : 85899 MB (40.0%)
Collecting resizing constraints ...
You might resize at 85899345920 bytes or 85900 MB (freeing 128849 MB).
Please make a test run using both the -n and -s options before real resizing!
"""
NTFS_HIBERNATED = """ntfsresize v2022.10.3 (libntfs-3g)
Device name        : /dev/nvme0n1p3
The NTFS partition is hibernated. Windows must be resumed and turned off properly, so resizing could be done safely.
"""
DUMPE2FS = """dumpe2fs 1.47.0 (5-Feb-2023)
Filesystem volume name:   home
Filesystem state:         clean
Block count:              52428800
Free blocks:              31457280
Block size:               4096
"""
RESIZE2FS_P = "resize2fs 1.47.0 (5-Feb-2023)\nEstimated minimum size of the filesystem: 22020096\n"


def windows_layout(m, extra=None, label='gpt', after_hearth_free=0):
    """EFI, MSR, C: (200 GiB), recovery, /boot, Hearth (198 GiB): the usual install next to Windows."""
    parts = [
        (1, 0.001, 0.1, m.ESP_TYPE, 'vfat', 'SYSTEM', '', ['/boot/efi']),
        (2, 0.101, 0.016, m.MSR_TYPE, '', '', '', []),
        (3, 0.125, 200, '', 'ntfs', 'Windows', 'Basic data partition', []),
        (4, 200.125, 0.875, m.WINRE_TYPE, 'ntfs', 'Recovery', '', []),
        (5, 201, 2, m.XBOOTLDR_TYPE, 'ext4', 'boot', 'boot', ['/boot']),
        (6, 203, 198, m.LINUX_TYPE, 'btrfs', 'hearth', 'hearth', ['/sysroot']),
    ] + (extra or [])
    total = 203 + 198 + after_hearth_free + 1
    return make(parts, total, label)


def make(parts, total_gib, label='gpt', disk='nvme0n1'):
    """parts: (num, start_gib, size_gib, type, fstype, label, partlabel, mounts)"""
    node = lambda n: f'/dev/{disk}p{n}'
    tree = {'blockdevices': [{'name': disk, 'path': f'/dev/{disk}', 'type': 'disk', 'size': total_gib * GIB, 'children': [
        {'name': f'{disk}p{n}', 'path': node(n), 'pkname': disk, 'fstype': fs or None, 'label': lab or None, 'partlabel': pl or None,
         'parttype': t, 'size': int(sz * SEC) * 512, 'mountpoints': mounts or [None], 'type': 'part'}
        for n, st, sz, t, fs, lab, pl, mounts in parts]}]}
    table = {'label': label, 'sectorsize': 512, 'firstlba': 2048, 'lastlba': total_gib * SEC - 34, 'partitions': [
        {'node': node(n), 'start': int(st * SEC) // 2048 * 2048 + 2048, 'size': int(sz * SEC), 'type': t, 'name': pl}
        for n, st, sz, t, fs, lab, pl, mounts in parts]}
    return tree, {f'/dev/{disk}': table}


ROOTINFO = {'mount': '/sysroot', 'rw': '/var/home', 'devices': ['/dev/nvme0n1p6'], 'devids': {'/dev/nvme0n1p6': 1}, 'uuid': '1b2c3d4e-0000-4000-8000-000000000001'}


def ntfs_facts(problem=None, used=86 * GIB, minimum=80 * GIB, size=200 * GIB):
    return {'/dev/nvme0n1p3': {'name': 'Windows', 'size': size, 'used': used, 'min': minimum, 'problem': problem}}


class Parsing(unittest.TestCase):
    def setUp(self):
        self.m = load()

    def test_ntfsresize_info(self):
        n = self.m.parse_ntfsresize_info(NTFS_INFO)
        self.assertEqual(n['size'], 214748360704)
        self.assertEqual(n['min'], 85899345920)
        self.assertNotIn('problem', n)
        self.assertEqual(self.m.parse_ntfsresize_info(NTFS_HIBERNATED)['problem'], 'fast-startup')
        self.assertEqual(self.m.parse_ntfsresize_info('ERROR: volume has bad sectors, run chkdsk /f')['problem'], 'needs-check')

    def test_reads_ntfs_like_the_live_usb(self):
        """The same numbers hearth-live-prep (the installer's Windows shrink) reads."""
        prep = load(PREP, 'hearth_live_prep')
        orig = prep.sh
        prep.sh = lambda cmd, check=True, **kw: type('R', (), {'stdout': NTFS_INFO, 'stderr': '', 'returncode': 0})()
        try:
            a = prep.ntfs_info('/dev/x')
        finally:
            prep.sh = orig
        b = self.m.parse_ntfsresize_info(NTFS_INFO)
        self.assertEqual((a['size'], a['min']), (b['size'], b['min']))

    def test_free_regions_parse_like_the_installer(self):
        inst = load(INSTALL, 'hearth_install')
        text = "Unpartitioned space /dev/nvme0n1: 100 GiB\nSector size: 512\n     Start       End   Sectors  Size\n  419430400 629145566 209715167  100G\n      2048      4095      2048    1M\n"
        self.assertEqual(self.m.parse_free(text), inst.parse_free(text))

    def test_ext4(self):
        e = self.m.parse_dumpe2fs(DUMPE2FS)
        self.assertEqual(e['size'], 52428800 * 4096)
        self.assertEqual(e['used'], (52428800 - 31457280) * 4096)
        self.assertNotIn('problem', e)
        self.assertEqual(self.m.parse_dumpe2fs(DUMPE2FS.replace('clean', 'not clean with errors'))['problem'], 'linux-check')
        self.assertEqual(self.m.parse_resize2fs_min(RESIZE2FS_P, 4096), 22020096 * 4096)

    def test_btrfs(self):
        self.assertEqual(self.m.parse_min_dev_size('123456789 bytes (117.74MiB)\n'), 123456789)
        devs, uuid = self.m.parse_btrfs_show("Label: 'hearth'  uuid: 1b2c3d4e-0000-4000-8000-000000000001\n\tTotal devices 2 FS bytes used 40.00GiB\n"
                                             "\tdevid    1 size 198.00GiB used 50.00GiB path /dev/nvme0n1p6\n"
                                             "\tdevid    2 size 64.00GiB used 0.00B path /dev/nvme0n1p7\n")
        self.assertEqual(devs, [(1, '/dev/nvme0n1p6'), (2, '/dev/nvme0n1p7')])
        self.assertEqual(uuid, '1b2c3d4e-0000-4000-8000-000000000001')

    def test_power(self):
        with tempfile.TemporaryDirectory() as d:
            for name, files in (('AC', {'type': 'Mains', 'online': '0'}), ('BAT0', {'type': 'Battery', 'capacity': '31'})):
                os.makedirs(os.path.join(d, name))
                for k, v in files.items():
                    with open(os.path.join(d, name, k), 'w') as f:
                        f.write(v + '\n')
            self.assertEqual(self.m.power(d), {'plugged': False, 'battery': 31})


class Math(unittest.TestCase):
    def setUp(self):
        self.m = load()

    def test_keeps_minimum_plus_margin_and_20gb_free(self):
        m = self.m
        # 80 GiB minimum, 86 GiB used: keeps max(80+4, 86+20) = 106 GiB of 200
        self.assertEqual(m.keep_size(80 * GIB, 86 * GIB), 106 * GIB)
        self.assertEqual(m.can_give(200 * GIB, 80 * GIB, 86 * GIB), 94 * GIB)
        # A nearly empty drive: the tool's minimum is tiny, 20 GB free still kept
        self.assertEqual(m.can_give(100 * GIB, 1 * GIB, 1 * GIB), 79 * GIB)
        # A full one gives nothing
        self.assertEqual(m.can_give(100 * GIB, 95 * GIB, 95 * GIB), 0)
        # A big minimum gets a 5% margin
        self.assertEqual(m.keep_size(400 * GIB, 300 * GIB), 420 * GIB)

    def test_geometry(self):
        m = self.m
        part = {'start': 264192, 'size': 200 * SEC}
        g = m.shrink_geometry(part, 200 * GIB, 64 * GIB, 106 * GIB, 512)
        self.assertEqual(g['newFs'], 136 * GIB)
        self.assertEqual(g['newStart'] % 2048, 0)                                 # 1 MiB aligned
        self.assertGreaterEqual(g['partSectors'] * 512, g['newFs'] + MIB)         # room for the file system + slack
        self.assertEqual(part['start'] + g['partSectors'], g['newStart'])          # no gap, no overlap
        self.assertLessEqual(g['newStart'] + g['newSectors'], part['start'] + part['size'])
        self.assertGreater(g['newSectors'] * 512, 63 * GIB)
        with self.assertRaises(m.Fail):
            m.shrink_geometry(part, 200 * GIB, 95 * GIB, 106 * GIB, 512)        # more than it can give
        with self.assertRaises(m.Fail):
            m.shrink_geometry(part, 200 * GIB, 100 * MIB, 106 * GIB, 512)       # too little to bother

    def test_4k_sectors(self):
        g = self.m.shrink_geometry({'start': 256, 'size': 200 * GIB // 4096}, 200 * GIB, 10 * GIB, 106 * GIB, 4096)
        self.assertEqual(g['newStart'] % 256, 0)
        self.assertGreaterEqual(g['partSectors'] * 4096, g['newFs'])


class Status(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.m.needed_tools = lambda fs: []

    def status(self, layout=None, facts=None, **kw):
        tree, tables = layout or windows_layout(self.m)
        args = dict(root=ROOTINFO, tree=tree, tables=tables, facts=facts if facts is not None else ntfs_facts(),
                    pw={'plugged': True, 'battery': None}, journal={}, boot=(True, ''), is_root=True, importing=False)
        args.update(kw)
        return self.m.status_data(**args)

    def test_windows_offered_and_efi_msr_recovery_boot_never(self):
        st = self.status()
        self.assertEqual([s['dev'] for s in st['systems']], ['/dev/nvme0n1p3'])
        w = st['systems'][0]
        self.assertEqual((w['name'], w['fs'], w['canGive'], w['blockers']), ('Windows', 'ntfs', 94 * GIB, []))
        kinds = {p['num']: p['kind'] for p in st['parts']}
        self.assertEqual(kinds, {1: 'efi', 2: 'msr', 3: 'windows', 4: 'recovery', 5: 'boot', 6: 'hearth'})
        self.assertEqual(st['hearth']['disks'], ['/dev/nvme0n1'])
        self.assertTrue(st['beta'])

    def test_blockers(self):
        st = self.status(facts=ntfs_facts(problem='fast-startup'))
        self.assertEqual([b['id'] for b in st['systems'][0]['blockers']], ['fast-startup'])
        self.assertIn('Shift', st['systems'][0]['blockers'][0]['text'])
        st = self.status(facts=ntfs_facts(used=190 * GIB, minimum=189 * GIB))
        self.assertEqual([b['id'] for b in st['systems'][0]['blockers']], ['room'])
        st = self.status(pw={'plugged': False, 'battery': 20})
        self.assertIn('power', [b['id'] for b in st['systems'][0]['blockers']])
        st = self.status(pw={'plugged': False, 'battery': 90})
        self.assertEqual(st['systems'][0]['blockers'], [])
        self.assertIn('power', [w['id'] for w in st['systems'][0]['warnings']])
        st = self.status(boot=(False, 'no btrfs scanning'))
        self.assertIn('boot', [b['id'] for b in st['systems'][0]['blockers']])
        st = self.status(importing=True)
        self.assertIn('importing', [b['id'] for b in st['systems'][0]['blockers']])

    def test_bitlocker_and_mbr(self):
        m = self.m
        tree, tables = windows_layout(m)
        tree['blockdevices'][0]['children'][2]['fstype'] = 'BitLocker'
        st = self.status((tree, tables), facts={})
        self.assertEqual([b['id'] for b in st['systems'][0]['blockers']], ['bitlocker'])
        tree, tables = windows_layout(m, label='dos')
        st = self.status((tree, tables))
        self.assertIn('table', [b['id'] for b in st['systems'][0]['blockers']])

    def test_open_in_another_app_vs_importer(self):
        tree, tables = windows_layout(self.m)
        tree['blockdevices'][0]['children'][2]['mountpoints'] = ['/run/media/sam/Windows']
        st = self.status((tree, tables))
        self.assertEqual([b['id'] for b in st['systems'][0]['blockers']], ['busy'])
        tree['blockdevices'][0]['children'][2]['mountpoints'] = ['/run/hearth-import/nvme0n1p3']
        st = self.status((tree, tables))
        self.assertEqual(st['systems'][0]['blockers'], [])

    def test_unfinished_journal_blocks_shrinking_but_not_free_space(self):
        j = {'state': 'running', 'from': '/dev/nvme0n1p3', 'name': 'Windows', 'steps': [
            {'step': 'check', 'changes': False, 'state': 'done'},
            {'step': 'shrink-fs', 'what': "making Windows' file system smaller", 'changes': True, 'state': 'started'}]}
        st = self.status(layout=windows_layout(self.m, after_hearth_free=30), journal=j)
        self.assertIn('unfinished', [b['id'] for b in st['systems'][0]['blockers']])
        self.assertEqual(st['journal']['step'], 'shrink-fs')
        self.assertTrue(st['free'] and st['free'][0]['afterHearth'] and st['free'][0]['blockers'] == [])
        # A run that only checked (nothing changed) doesn't block
        j['steps'] = j['steps'][:1]
        self.assertEqual(self.status(journal=j)['systems'][0]['blockers'], [])

    def test_free_space_after_hearth(self):
        st = self.status(layout=windows_layout(self.m, after_hearth_free=30))
        self.assertEqual(len(st['free']), 1)
        self.assertTrue(st['free'][0]['afterHearth'])
        self.assertGreaterEqual(st['free'][0]['canGive'], 29 * GIB)

    def test_steamos_only_home(self):
        m = self.m
        parts = [(1, 0.001, 0.06, m.ESP_TYPE, 'vfat', '', 'esp', []), (2, 0.07, 0.06, '', 'vfat', '', 'efi-A', []), (3, 0.13, 0.06, '', 'vfat', '', 'efi-B', []),
                 (4, 0.2, 5, '', 'btrfs', 'rootfs', 'rootfs-A', []), (5, 5.2, 5, '', 'btrfs', 'rootfs', 'rootfs-B', []),
                 (6, 10.2, 0.25, '', 'ext4', 'var', 'var-A', []), (7, 10.5, 0.25, '', 'ext4', 'var', 'var-B', []),
                 (8, 11, 400, m.LINUX_TYPE, 'ext4', 'home', 'home', []),
                 (9, 411, 2, m.XBOOTLDR_TYPE, 'ext4', 'boot', 'boot', ['/boot']), (10, 413, 98, m.LINUX_TYPE, 'btrfs', 'hearth', 'hearth', ['/sysroot'])]
        tree, tables = make(parts, 512)
        root = dict(ROOTINFO, devices=['/dev/nvme0n1p10'], devids={'/dev/nvme0n1p10': 1})
        st = self.status((tree, tables), root=root, facts={'/dev/nvme0n1p8': {'name': 'SteamOS', 'size': 400 * GIB, 'used': 100 * GIB, 'min': 98 * GIB}})
        self.assertEqual([(s['dev'], s['name']) for s in st['systems']], [('/dev/nvme0n1p8', 'SteamOS')])
        self.assertEqual(st['systems'][0]['canGive'], 280 * GIB)

    def test_encrypted_linux_cant_shrink(self):
        m = self.m
        tree, tables = windows_layout(m, extra=[(7, 402, 50, m.LINUX_TYPE, 'crypto_LUKS', '', '', [])])
        st = self.status((tree, tables))
        luks = next(s for s in st['systems'] if s['dev'] == '/dev/nvme0n1p7')
        self.assertEqual([b['id'] for b in luks['blockers']], ['fs'])

    def test_boot_check(self):
        m = self.m
        ok = 'drwxr-xr-x usr/lib/udev/rules.d/64-btrfs.rules\n-rwxr-xr-x usr/sbin/btrfs\n'
        cmd = 'BOOT_IMAGE=(hd0,gpt5)/ostree/default-x/vmlinuz root=UUID=1b2c3d4e-0000-4000-8000-000000000001 rootflags=subvol=root rw ostree=/ostree/boot.1/default/x/0'
        self.assertEqual(m.boot_check(ROOTINFO, cmd, ok), (True, ''))
        self.assertFalse(m.boot_check(ROOTINFO, cmd.replace('000000000001', '000000000002'), ok)[0])
        self.assertFalse(m.boot_check(ROOTINFO, cmd.replace('root=UUID=1b2c3d4e-0000-4000-8000-000000000001', 'root=/dev/nvme0n1p6'), ok)[0])
        self.assertFalse(m.boot_check(ROOTINFO, cmd, 'usr/lib/udev/rules.d/60-block.rules\n')[0])
        self.assertFalse(m.boot_check(ROOTINFO, cmd, None)[0])

    def test_installer_kernel_args_stay_valid(self):
        """hearth-install puts root=UUID=<fs uuid> rootflags=subvol=root on the kernel line and UUID= in fstab:
        adding a device keeps the file system's UUID, so both still find it."""
        text = INSTALL.read_text()
        self.assertIn("'--karg', f'root=UUID={root_uuid}', '--karg', 'rootflags=subvol=root'", text)
        inst = load(INSTALL, 'hearth_install')
        self.assertTrue(all(l.startswith('UUID=') for l in inst.fstab_text('r', 'b', 'e').splitlines()[1:]))


class RootFs(unittest.TestCase):
    def test_writable_mount_of_hearths_file_system(self):
        m = load()
        u = '1b2c3d4e-0000-4000-8000-000000000001'
        out = (f'/sysroot ro,relatime,seclabel,compress=zstd:1,ssd,subvolid=5,subvol=/ {u}\n'
               f'/var rw,relatime,seclabel,compress=zstd:1,ssd,subvol=/root/ostree/deploy/default/var {u}\n'
               f'/var/home rw,relatime,compress=zstd:1,subvol=/home {u}\n'
               f'/run/media/x rw,relatime 99999999-0000-4000-8000-000000000000\n')
        m.RUN = lambda cmd, input=None, timeout=None: (0, out, '')
        self.assertEqual(m.rw_mount(u, '/sysroot'), '/var/home')
        m.RUN = lambda cmd, input=None, timeout=None: (0, out.replace('/var/home rw', '/var/home ro').replace('/var rw', '/var ro'), '')
        self.assertEqual(m.rw_mount(u, '/sysroot'), '/sysroot')   # nothing writable found: the fallback


class Plan(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.m.needed_tools = lambda fs: []

    def st(self, layout=None, **kw):
        tree, tables = layout or windows_layout(self.m)
        args = dict(root=ROOTINFO, tree=tree, tables=tables, facts=ntfs_facts(), pw={'plugged': True, 'battery': None},
                    journal={}, boot=(True, ''), is_root=True, importing=False)
        args.update(kw)
        return self.m.status_data(**args), tables

    def test_steps_in_order(self):
        st, tables = self.st()
        p = self.m.plan_grow(st, '/dev/nvme0n1p3', 64, tables, ROOTINFO)
        self.assertEqual([s['step'] for s in p['steps']], ['release', 'check', 'shrink-fs', 'shrink-part', 'new-part', 'device-add', 'verify'])
        self.assertEqual(p['method'], 'device-add')
        sp = p['steps'][3]
        np_ = p['steps'][4]
        self.assertEqual(sp['start'] + sp['newSectors'], np_['start'])
        self.assertEqual(np_['start'] + np_['sectors'], sp['start'] + sp['oldSectors'])
        self.assertTrue(63 * GIB < p['give'] <= 64 * GIB)

    def test_refusals(self):
        m = self.m
        st, tables = self.st()
        for src, gb in (('/dev/nvme0n1p3', 95), ('/dev/nvme0n1p1', 1), ('/dev/nvme0n1p2', 1), ('/dev/nvme0n1p4', 1),
                        ('/dev/nvme0n1p5', 1), ('/dev/nvme0n1p6', 1), ('/dev/sdz9', 1), ('/dev/nvme0n1p3', 0), ('../etc', 1)):
            with self.assertRaises(m.Fail, msg=(src, gb)):
                m.plan_grow(st, src, gb, tables, ROOTINFO)
        st, tables = self.st(facts=ntfs_facts(problem='fast-startup'))
        with self.assertRaisesRegex(m.Fail, 'fully shut down'):
            m.plan_grow(st, '/dev/nvme0n1p3', 10, tables, ROOTINFO)

    def test_estimates_never_planned(self):
        f = ntfs_facts()
        f['/dev/nvme0n1p3']['estimate'] = True
        st, tables = self.st(facts=f)
        with self.assertRaisesRegex(self.m.Fail, 'exactly'):
            self.m.plan_grow(st, '/dev/nvme0n1p3', 10, tables, ROOTINFO)

    def test_free_space_after_hearth_grows_in_place(self):
        st, tables = self.st(windows_layout(self.m, after_hearth_free=30))
        p = self.m.plan_grow(st, 'free', 20, tables, ROOTINFO)
        self.assertEqual(p['method'], 'grow-in-place')
        self.assertEqual([s['step'] for s in p['steps']], ['grow-part', 'grow-fs', 'verify'])
        self.assertEqual(p['steps'][0]['newSectors'] - p['steps'][0]['oldSectors'], 20 * SEC)
        with self.assertRaises(self.m.Fail):
            self.m.plan_grow(st, 'free', 40, tables, ROOTINFO)

    def test_free_space_elsewhere_is_added(self):
        m = self.m
        # Space between recovery and /boot (an earlier shrink that stopped before adding the partition)
        parts = [(1, 0.001, 0.1, m.ESP_TYPE, 'vfat', '', '', ['/boot/efi']), (3, 0.125, 150, '', 'ntfs', 'Windows', '', []),
                 (5, 201, 2, m.XBOOTLDR_TYPE, 'ext4', 'boot', 'boot', ['/boot']), (6, 203, 198, m.LINUX_TYPE, 'btrfs', 'hearth', 'hearth', ['/sysroot'])]
        st, tables = self.st(make(parts, 402), facts=ntfs_facts(size=150 * GIB))
        p = m.plan_grow(st, 'free', 40, tables, ROOTINFO)
        self.assertEqual(p['method'], 'device-add')
        self.assertEqual([s['step'] for s in p['steps']], ['new-part', 'device-add', 'verify'])


class FakeSystem:
    """A partition table and the commands that change it, plus a log of every command."""

    def __init__(self, m, tree, tables, fail=None, fail_undo=None):
        self.m, self.tree, self.tables, self.calls = m, tree, tables, []
        self.fail, self.fail_undo = fail or (), fail_undo or ()
        self.disk = '/dev/nvme0n1'

    def table(self):
        return self.tables[self.disk]

    def run(self, cmd, input=None, timeout=None):
        self.calls.append((cmd, input))
        line = ' '.join(cmd)
        for pat in self.fail:
            if re.search(pat, line):
                return 1, '', 'simulated failure: ' + line
        if cmd[:2] == ['sfdisk', '-J']:
            return 0, json.dumps({'partitiontable': self.table()}), ''
        if cmd[0] == 'lsblk':
            return 0, json.dumps(self.tree), ''
        if cmd[0] == 'sfdisk' and '-N' in cmd:
            n = int(cmd[cmd.index('-N') + 1])
            m = re.match(r'start=(\d+), size=(\d+)', input)
            for p in self.table()['partitions']:
                if p['node'].endswith(f'p{n}'):
                    if len([c for c, _ in self.calls if '-N' in c and c[c.index('-N') + 1] == str(n)]) > 1 and any(re.search(x, line + ' undo') for x in self.fail_undo):
                        return 1, '', 'simulated undo failure'
                    p['start'], p['size'] = int(m.group(1)), int(m.group(2))
            return 0, '', ''
        if cmd[0] == 'sfdisk' and '--append' in cmd:
            m = re.match(r'start=(\d+), size=(\d+)', input)
            nums = [int(re.search(r'(\d+)$', p['node']).group(1)) for p in self.table()['partitions']]
            n = next(i for i in range(1, 129) if i not in nums)
            self.table()['partitions'].append({'node': f'{self.disk}p{n}', 'start': int(m.group(1)), 'size': int(m.group(2)), 'type': self.m.LINUX_TYPE, 'name': 'hearth-more'})
            return 0, '', ''
        if cmd[0] == 'sfdisk' and '--delete' in cmd:
            n = cmd[-1]
            self.table()['partitions'] = [p for p in self.table()['partitions'] if not p['node'].endswith('p' + n)]
            return 0, '', ''
        if cmd[0] == 'ntfsresize' and '--info' in cmd:
            return 0, NTFS_INFO, ''
        if cmd[:3] == ['btrfs', 'filesystem', 'show']:
            devs = [p['node'] for p in self.table()['partitions'] if p.get('name') in ('hearth', 'hearth-more')]
            added = any(c[:3] == ['btrfs', 'device', 'add'] for c, _ in self.calls)
            out = ''.join(f'\tdevid {i + 1} size 1GiB used 0 path {d}\n' for i, d in enumerate(devs if added else devs[:1]))
            return 0, out, ''
        return 0, '', ''

    def stream(self, cmd, input=None, on_line=None):
        code, out, err = self.run(cmd, input)
        if on_line:
            on_line('50.00 percent completed')
        return code, [err or 'ok']

    def commands(self):
        return [' '.join(c) for c, _ in self.calls if c[0] not in ('lsblk', 'udevadm', 'partx') and c[:2] != ['sfdisk', '-J']]


class Running(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.m.needed_tools = lambda fs: []
        self.m.wait_node = lambda dev, seconds=10: True
        self.dir = tempfile.mkdtemp()
        self.m.JOURNAL = os.path.join(self.dir, 'space-grow.json')

    def go(self, fail=(), fail_undo=(), src='/dev/nvme0n1p3', gb=64, layout=None):
        tree, tables = layout or windows_layout(self.m)
        fake = FakeSystem(self.m, tree, tables, fail, fail_undo)
        self.m.RUN, self.m.STREAM = fake.run, fake.stream
        st = self.m.status_data(root=ROOTINFO, tree=tree, tables=tables, facts=ntfs_facts(), pw={'plugged': True, 'battery': None},
                                journal={}, boot=(True, ''), is_root=True, importing=False)
        before = json.loads(json.dumps(tables))
        plan = self.m.plan_grow(st, src, gb, json.loads(json.dumps(tables)), ROOTINFO)
        plan['mount'] = '/sysroot'
        journal = self.m.Journal(plan)
        seen = []
        try:
            res = self.m.execute(plan, journal, emit=lambda pct, what: seen.append((pct, what)))
            err = None
        except self.m.Fail as e:
            res, err = None, str(e)
        with open(self.m.JOURNAL) as f:
            j = json.load(f)
        return fake, res, err, before, j, seen

    def test_success_runs_steps_in_order(self):
        fake, res, err, before, j, seen = self.go()
        self.assertIsNone(err)
        cmds = fake.commands()
        order = [next(i for i, c in enumerate(cmds) if re.search(p, c)) for p in
                 (r'^ntfsresize --info', r'^ntfsfix -n', r'ntfsresize --force --no-action', r'^ntfsresize --force --size',
                  r'^sfdisk .* -N 3', r'^sfdisk .*--append', r'^wipefs -a /dev/nvme0n1p7', r'^btrfs device add -f /dev/nvme0n1p7 /var/home')]
        self.assertEqual(order, sorted(order))
        self.assertEqual(res['newPart'], '/dev/nvme0n1p7')
        self.assertEqual(j['state'], 'done')
        self.assertTrue(all(s['state'] == 'done' for s in j['steps']))
        # Windows' partition kept its start; recovery and everything else untouched
        t = fake.table()['partitions']
        by = {p['node']: p for p in t}
        self.assertEqual(by['/dev/nvme0n1p3']['start'], before['/dev/nvme0n1']['partitions'][2]['start'])
        for n in (1, 2, 4, 5, 6):
            self.assertEqual(by[f'/dev/nvme0n1p{n}'], before['/dev/nvme0n1']['partitions'][n - 1])
        # The file system was shrunk to fit inside the new partition size
        size_arg = next(c for c in cmds if c.startswith('ntfsresize --force --size')).split()[3]
        self.assertLessEqual(int(size_arg), by['/dev/nvme0n1p3']['size'] * 512)
        self.assertTrue(seen[-1][0] == 100)

    def test_check_failure_changes_nothing(self):
        fake, res, err, before, j, _ = self.go(fail=[r'^ntfsfix -n'])
        self.assertRegex(err, 'Nothing was changed')
        self.assertEqual(fake.tables, before)
        self.assertFalse(any(re.search(r'--size \d+ /dev', c) and 'no-action' not in c for c in fake.commands()))
        self.assertEqual(j['state'], 'rolled-back')

    def test_failed_new_partition_puts_everything_back_in_reverse(self):
        fake, res, err, before, j, _ = self.go(fail=[r'--append'])
        self.assertRegex(err, 'put back as it was')
        cmds = fake.commands()
        shrink = next(i for i, c in enumerate(cmds) if re.match(r'ntfsresize --force --size', c))
        restore_part = max(i for i, c in enumerate(cmds) if '-N 3' in c)
        regrow = max(i for i, c in enumerate(cmds) if re.match(r'ntfsresize --force --size', c))
        self.assertLess(shrink, restore_part)
        self.assertLess(restore_part, regrow)                                  # partition back first, then the file system
        self.assertIn(str(200 * GIB), cmds[regrow])                             # grown back to the old size
        self.assertEqual(fake.tables, before)
        self.assertEqual(j['state'], 'rolled-back')
        self.assertEqual(j['undone'], ['shrink-part', 'shrink-fs'])

    def test_failed_device_add_removes_the_new_partition_too(self):
        fake, res, err, before, j, _ = self.go(fail=[r'^btrfs device add'])
        self.assertRegex(err, 'put back')
        cmds = fake.commands()
        self.assertLess(max(i for i, c in enumerate(cmds) if '--delete' in c), max(i for i, c in enumerate(cmds) if '-N 3' in c))
        self.assertEqual(fake.tables, before)
        self.assertEqual(j['undone'], ['new-part', 'shrink-part', 'shrink-fs'])

    def test_failed_undo_stops_and_says_so(self):
        fake, res, err, before, j, _ = self.go(fail=[r'--append'], fail_undo=['-N 3'])
        self.assertRegex(err, "couldn't put everything back")
        self.assertEqual(j['state'], 'stuck')
        self.assertFalse(any(re.match(r'ntfsresize --force --size ' + str(200 * GIB), c) for c in fake.commands()))   # didn't grow into a too-small partition

    def test_grow_in_place(self):
        fake, res, err, before, j, _ = self.go(src='free', gb=20, layout=windows_layout(self.m, after_hearth_free=30))
        self.assertIsNone(err)
        cmds = fake.commands()
        self.assertTrue(any('-N 6' in c for c in cmds))
        self.assertIn('btrfs filesystem resize 1:max /var/home', cmds)
        self.assertFalse(any('device add' in c for c in cmds))
        p6 = next(p for p in fake.table()['partitions'] if p['node'].endswith('p6'))
        self.assertEqual(p6['size'], 218 * SEC)

    def test_grow_in_place_fs_failure_restores_partition(self):
        fake, res, err, before, j, _ = self.go(src='free', gb=20, layout=windows_layout(self.m, after_hearth_free=30), fail=[r'filesystem resize'])
        self.assertRegex(err, 'put back')
        self.assertEqual(fake.tables, before)

    def test_journal_written_before_each_step(self):
        fake, res, err, before, j, _ = self.go(fail=[r'^ntfsresize --force --size'])
        steps = [(s['step'], s['state']) for s in j['steps']]
        self.assertEqual(steps[:2], [('release', 'done'), ('check', 'done')])
        self.assertEqual(steps[2][0], 'shrink-fs')
        self.assertEqual(steps[2][1], 'started')   # it never finished: recorded as started


class Arguments(unittest.TestCase):
    def test_grow_arguments(self):
        m = load()
        self.assertEqual(m.parse_grow_args(['--from', '/dev/nvme0n1p3', '--gb', '64']), ('/dev/nvme0n1p3', 64, False))
        self.assertEqual(m.parse_grow_args(['--from', 'free', '--gb', '5', '--dry-run']), ('free', 5, True))
        for bad in ([], ['--from', '/dev/x'], ['--gb', '5'], ['--from', 'C:', '--gb', '5'], ['--from', '/dev/x;rm', '--gb', '5'],
                    ['--from', '/dev/x', '--gb', '-1'], ['--from', '/dev/x', '--gb', '0'], ['--from', '/dev/x', '--gb', '5', '--yes']):
            with self.assertRaises(m.Fail, msg=bad):
                m.parse_grow_args(bad)

    def test_admin_checks_before_running_as_root(self):
        a = load(ADMIN, 'hearth_admin')
        self.assertEqual(a.space_args(['--from', '/dev/nvme0n1p3', '--gb', '64']), ['--from', '/dev/nvme0n1p3', '--gb', '64'])
        self.assertEqual(a.space_args(['--gb', '8', '--from', 'free', '--dry-run']), ['--gb', '8', '--from', 'free', '--dry-run'])
        for bad in ([], ['--from', '/dev/x'], ['--from', '/dev/../etc/passwd', '--gb', '1'], ['--from', '/etc/passwd', '--gb', '1'],
                    ['--from', '/dev/x', '--gb', '0'], ['--from', '/dev/x', '--gb', '99999999'], ['--from', '/dev/x', '--from', '/dev/y', '--gb', '1'],
                    ['--from', '/dev/x', '--gb', '1', '--force'], ['--from', '/dev/x y', '--gb', '1']):
            self.assertIsNone(a.space_args(bad), bad)


if __name__ == '__main__':
    unittest.main()
