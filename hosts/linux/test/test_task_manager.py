"""Task Manager, the Windows 11 way: the shared sampler in desktop/shell/hearth-tasks (/proc
stat, meminfo, diskstats, net/dev, graphics, the process groups), the heat-map thresholds, start-up apps writing only
your own copies, Part of Hearth staying put, and Task Manager's settings (taskmgr.json) only ever holding what you
changed."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import re
import tempfile
import unittest

TMP = tempfile.mkdtemp()
os.environ['XDG_STATE_HOME'] = os.path.join(TMP, 'state')
os.environ['XDG_RUNTIME_DIR'] = os.path.join(TMP, 'run')
os.environ['XDG_DATA_HOME'] = os.path.join(TMP, 'data')
os.environ['XDG_DATA_DIRS'] = os.path.join(TMP, 'sysdata')
os.environ['XDG_CONFIG_HOME'] = os.path.join(TMP, 'config')
os.makedirs(os.environ['XDG_RUNTIME_DIR'], exist_ok=True)

SHELL = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell'
loader = importlib.machinery.SourceFileLoader('hearth_tasks_tm', str(SHELL / 'hearth-tasks'))
spec = importlib.util.spec_from_loader('hearth_tasks_tm', loader)
ht = importlib.util.module_from_spec(spec)
loader.exec_module(ht)
sys.path.insert(0, str(SHELL))
import hearth_settings_io as hsio  # noqa: E402

ME = os.getuid()


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


def stat_line(pid, comm, ppid, jiffies=0, start=100, state='S', nice=0, threads=1):
    return '%d (%s) %s %d 1 1 0 -1 0 0 0 0 0 %d 0 0 0 20 %d %d 0 %d 1000 100\n' % (
        pid, comm, state, ppid, jiffies, nice, threads, start)


def status_text(uid, anon_kb=1000):
    return 'Name:\tx\nUid:\t%d\t%d\t%d\t%d\nRssAnon:\t%d kB\nRssShmem:\t0 kB\nKthread:\t0\n' % (uid, uid, uid, uid, anon_kb)


class World:
    """A fake /proc and /sys that move on between samples."""

    def __init__(self):
        self.root = tempfile.mkdtemp(dir=TMP)
        self.proc = os.path.join(self.root, 'proc')
        self.sys = os.path.join(self.root, 'sys')
        write(os.path.join(self.proc, 'cpuinfo'), 'processor\t: 0\nmodel name\t: Intel(R) Core(TM) i3-N305\nphysical id\t: 0\ncore id\t: 0\n'
              'flags\t: fpu vmx sse\n\nprocessor\t: 1\nmodel name\t: Intel(R) Core(TM) i3-N305\nphysical id\t: 0\ncore id\t: 1\nflags\t: fpu vmx\n')
        write(os.path.join(self.proc, 'loadavg'), '0.10 0.20 0.30 2/345 999\n')
        write(os.path.join(self.proc, 'uptime'), '3661.5 100.0\n')
        write(os.path.join(self.proc, 'meminfo'), 'MemTotal: 8000000 kB\nMemFree: 1000000 kB\nMemAvailable: 6000000 kB\nBuffers: 100 kB\n'
              'Cached: 2000000 kB\nSReclaimable: 0 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\nCommitted_AS: 3000000 kB\nCommitLimit: 4000000 kB\n')
        # an Intel graphics chip (busy from its idle residency), an NVMe disk with / on it, a Wi-Fi adapter
        card = os.path.join(self.sys, 'class/drm/card0')
        write(os.path.join(card, 'device/vendor'), '0x8086\n')
        write(os.path.join(card, 'gt/gt0/rc6_residency_ms'), '1000\n')
        write(os.path.join(self.sys, 'block/nvme0n1/size'), str(1000215216) + '\n')
        write(os.path.join(self.sys, 'block/nvme0n1/queue/rotational'), '0\n')
        write(os.path.join(self.sys, 'block/nvme0n1/device/model'), 'Fast SSD\n')
        os.makedirs(os.path.join(self.sys, 'block/nvme0n1/nvme0n1p2'))
        write(os.path.join(self.sys, 'block/loop0/size'), '100\n')
        wl = os.path.join(self.sys, 'class/net/wlan0')
        os.makedirs(os.path.join(wl, 'device'))
        os.makedirs(os.path.join(wl, 'wireless'))
        write(os.path.join(wl, 'operstate'), 'up\n')
        self.step(0)

    def step(self, n):
        write(os.path.join(self.proc, 'stat'), 'cpu  %d 0 0 %d 0 0 0 0 0 0\ncpu0 %d 0 0 %d 0 0 0 0 0 0\ncpu1 %d 0 0 %d 0 0 0 0 0 0\n' % (
            100 + 50 * n, 1000 + 50 * n, 50 + 50 * n, 500, 50, 500 + 50 * n))
        write(os.path.join(self.proc, 'diskstats'), '259 0 nvme0n1 %d 0 %d 10 %d 0 %d 10 0 %d 0\n7 0 loop0 1 0 1 1 1 0 1 1 0 1 0\n' % (
            10 + n, 2048 * n, 5 + n, 1024 * n, 250 * n))
        write(os.path.join(self.proc, 'net/dev'), 'Inter-|   Receive\n face |bytes packets\n  wlan0: %d 0 0 0 0 0 0 0 %d 0 0 0 0 0 0 0\n    lo: 5 0 0 0 0 0 0 0 5 0 0 0 0 0 0 0\n' % (
            1000000 * n, 500000 * n))
        write(os.path.join(self.sys, 'class/drm/card0/gt/gt0/rc6_residency_ms'), str(1000 + 250 * n) + '\n')
        self.add(500, 'firefox', 1, ['/usr/lib/firefox/firefox'], jiffies=40 * n, anon_kb=400000)
        self.add(501, 'Web Content', 500, ['/usr/lib/firefox/firefox', '-contentproc'], jiffies=10 * n, anon_kb=200000)
        self.add(600, 'sleepy', 1, ['/usr/bin/sleepy'], state='T')
        self.add(700, 'quickshell', 1, ['quickshell', '-p', '/usr/share/nimbus/desktop/shell'])

    def add(self, pid, comm, ppid, cmd, jiffies=0, anon_kb=1000, state='S', uid=ME):
        d = os.path.join(self.proc, str(pid))
        write(os.path.join(d, 'stat'), stat_line(pid, comm, ppid, jiffies, start=pid * 10, state=state))
        write(os.path.join(d, 'status'), status_text(uid, anon_kb))
        write(os.path.join(d, 'cmdline'), '\0'.join(cmd) + '\0')
        write(os.path.join(d, 'cgroup'), '0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-x-%d.scope\n' % pid)
        write(os.path.join(d, 'io'), 'rchar: 1\nread_bytes: %d\nwrite_bytes: 0\n' % (jiffies * 1048576))


class Parsing(unittest.TestCase):
    def test_proc_stat_cpu(self):
        st = ht.parse_cpu_stat('cpu  100 0 100 700 100 0 0 0 0 0\ncpu0 50 0 50 400 0 0 0 0 0 0\nintr 1 2\n')
        self.assertEqual(st[0], ('cpu', 1000, 800))
        self.assertAlmostEqual(ht.busy_percent((1000, 800), (1100, 850)), 50.0)

    def test_stat_has_nice_and_threads(self):
        s = ht.parse_stat(stat_line(42, 'a b) c', 1, jiffies=7, state='T', nice=19, threads=12))
        self.assertEqual((s['comm'], s['state'], s['nice'], s['threads'], s['jiffies']), ('a b) c', 'T', 19, 12, 7))

    def test_meminfo(self):
        m = ht.parse_meminfo('MemTotal:  2048 kB\nCommitted_AS: 10 kB\nHugePages_Total: 0\n')
        self.assertEqual(m['MemTotal'], 2048 * 1024)
        self.assertEqual(m['Committed_AS'], 10240)

    def test_diskstats(self):
        d = ht.parse_diskstats(' 259       0 nvme0n1 100 5 2000 30 50 2 1000 20 0 40 60 0 0 0 0\n bad line\n')
        self.assertEqual(d['nvme0n1'], {'rd': 100, 'rsec': 2000, 'rms': 30, 'wr': 50, 'wsec': 1000, 'wms': 20, 'ioms': 40})

    def test_net_dev(self):
        n = ht.parse_net_dev('Inter-|   Receive\n face |bytes\n  eth0: 1234 9 0 0 0 0 0 0 5678 3 0 0 0 0 0 0\n')
        self.assertEqual(n, {'eth0': (1234, 5678)})

    def test_small_files(self):
        self.assertEqual(ht.parse_loadavg_threads('0.00 0.01 0.05 1/812 4242\n'), 812)
        self.assertIsNone(ht.parse_loadavg_threads(''))
        self.assertEqual(ht.parse_proc_io('rchar: 9\nread_bytes: 4096\nwrite_bytes: 1024\ncancelled_write_bytes: 7\n'), 5120)
        self.assertEqual(ht.parse_fdinfo('pos: 0\ndrm-driver: i915\ndrm-client-id: 7\ndrm-engine-render: 5000 ns\ndrm-engine-capacity-video: 2\n'),
                         ('7', {'render': 5000}))
        self.assertEqual(ht.parse_fdinfo('pos: 0\nflags: 02\n'), (None, {}))

    def test_boot_time(self):
        self.assertEqual(ht.parse_boot_time('Startup finished in 3.2s (kernel) + 11.0s (userspace) = 14.2s\n'), 14.2)
        self.assertEqual(ht.parse_boot_time('Startup finished in 1min 2.5s (firmware) = 1min 3.512s\n'), 63.5)
        self.assertIsNone(ht.parse_boot_time('Bootup is not yet finished.\n'))

    def test_list_units(self):
        u = ht.parse_list_units('foo.service loaded active running Foo daemon\nbar.service loaded inactive dead\nx.socket loaded active listening X\n')
        self.assertEqual([x['unit'] for x in u], ['foo.service', 'bar.service'])
        self.assertEqual(u[0]['desc'], 'Foo daemon')


class Heat(unittest.TestCase):
    def test_thresholds(self):
        self.assertEqual(ht.heat_level('cpu', 0), 0)
        self.assertEqual(ht.heat_level('cpu', 0.5), 1)
        self.assertEqual(ht.heat_level('cpu', 14.9), 2)
        self.assertEqual(ht.heat_level('cpu', 30), 4)
        self.assertEqual(ht.heat_level('cpu', 99), 5)
        self.assertEqual(ht.heat_level('mem', 3), 2)
        self.assertEqual(ht.heat_level('disk', 200), 5)
        self.assertEqual(ht.heat_level('net', None), 0)

    def test_levels_only_rise(self):
        for kind, steps in ht.HEAT.items():
            self.assertEqual(list(steps), sorted(steps), kind)
            last = 0
            for x in [0, 0.05, 0.3, 1, 3, 8, 20, 40, 80, 300]:
                lv = ht.heat_level(kind, x)
                self.assertGreaterEqual(lv, last)
                self.assertLessEqual(lv, 5)
                last = lv

    def test_startup_impact(self):
        self.assertEqual(ht.startup_impact(None), 'Not measured')
        self.assertEqual(ht.startup_impact(1e8), 'Low')
        self.assertEqual(ht.startup_impact(9e8), 'Medium')
        self.assertEqual(ht.startup_impact(5e9), 'High')
        self.assertEqual(ht.startup_impact(5e9, on=False), 'None')


class Sampling(unittest.TestCase):
    def setUp(self):
        self.w = World()

    def test_two_samples(self):
        s = ht.Sampler(self.w.proc, self.w.sys)
        s.sample(['procs', 'details'])
        s.t -= 1.0                      # one second later
        self.w.step(1)
        d = s.sample(['procs', 'details'])
        self.assertEqual(d['cpu']['name'], 'Intel(R) Core(TM) i3-N305')
        self.assertEqual((d['cpu']['coreCount'], d['cpu']['logical'], d['cpu']['virt']), (2, 2, True))
        self.assertAlmostEqual(d['cpu']['total'], 50.0, places=0)
        self.assertEqual(d['cpu']['threads'], 345)
        self.assertEqual(d['uptime'], 3661)
        self.assertEqual(d['mem']['used'], 2000000 * 1024)
        self.assertEqual(d['mem']['committed'], 3000000 * 1024)
        disk = d['disks'][0]
        self.assertEqual((disk['name'], disk['type'], disk['model']), ('nvme0n1', 'SSD (NVMe)', 'Fast SSD'))
        self.assertEqual(len(d['disks']), 1, 'loop devices are not disks')
        self.assertAlmostEqual(disk['active'], 25.0, delta=3)
        self.assertAlmostEqual(disk['read'], 2048 * 512, delta=2048 * 512 * 0.05)
        net = d['nets'][0]
        self.assertEqual((net['name'], net['kind']), ('wlan0', 'Wi-Fi'))
        self.assertAlmostEqual(net['recv'], 1000000, delta=50000)
        gpu = d['gpus'][0]
        self.assertEqual(gpu['vendor'], 'Intel')
        self.assertAlmostEqual(gpu['busy'], 75.0, delta=3)
        self.assertEqual(d['totals']['gpu'], gpu['busy'])
        # the process groups: Firefox with its child, a suspended app, Hearth's own shell protected
        groups = {g['name']: g for g in d['procs']}
        ff = next(g for g in d['procs'] if g['pid'] == 500)
        self.assertEqual(ff['count'], 2)
        self.assertEqual(len(ff['children']), 2)
        self.assertGreater(ff['cpu'], 0)
        self.assertIn(ff['heat']['cpu'], range(1, 6))
        self.assertGreater(ff['disk'], 0)
        sleepy = next(g for g in d['procs'] if g['pid'] == 600)
        self.assertTrue(sleepy['suspended'])
        qs = next(g for g in d['procs'] if g['pid'] == 700)
        self.assertTrue(qs['protected'])
        self.assertEqual(qs['tag'], 'Part of Hearth')
        self.assertTrue(groups)
        rows = {r['pid']: r for r in d['details']}
        self.assertEqual(rows[600]['status'], 'Suspended')
        self.assertEqual(rows[500]['cmd'], '/usr/lib/firefox/firefox')

    def test_hidden_pages_data_not_read(self):
        s = ht.Sampler(self.w.proc, self.w.sys)
        d = s.sample([])
        self.assertNotIn('procs', d)
        self.assertNotIn('details', d)
        self.assertNotIn('sensors', d)

    def test_letters(self):
        mounts = '/dev/nvme0n1p2 /sysroot btrfs rw 0 0\n/dev/sdb1 /run/media/me/Games ext4 rw 0 0\n'
        write(os.path.join(self.w.sys, 'block/sdb/size'), '2000\n')
        write(os.path.join(self.w.sys, 'block/sdb/queue/rotational'), '1\n')
        os.makedirs(os.path.join(self.w.sys, 'block/sdb/sdb1'))
        plain = ht.disk_layout(self.w.sys, mounts, None)
        self.assertEqual([x['label'] for x in plain], ['Disk 0 (C:)', 'Disk 1'])
        lettered = ht.disk_layout(self.w.sys, mounts, {'u1': {'letter': 'D', 'path': '/run/media/me/Games'}})
        self.assertEqual([x['label'] for x in lettered], ['Disk 0 (C:)', 'Disk 1 (D:)'])
        self.assertTrue(lettered[0]['system'])
        self.assertEqual(lettered[1]['type'], 'HDD')

    def test_letters_file_is_only_read(self):
        f = os.path.join(TMP, 'letters.json')
        self.assertIsNone(ht.read_drive_letters(f))
        write(f, json.dumps({'drives': {'x': {'letter': 'E', 'path': '/mnt/x'}}}))
        before = pathlib.Path(f).read_text()
        self.assertEqual(ht.read_drive_letters(f)['x']['letter'], 'E')
        self.assertEqual(pathlib.Path(f).read_text(), before)

    def test_serve(self):
        r, wfd = os.pipe()
        os.write(wfd, b'{"interval": 0.05, "want": ["procs"]}\n')
        out = io.StringIO()
        n = ht.serve(r, out, self.w.proc, self.w.sys, max_samples=2)
        os.close(wfd)
        os.close(r)
        self.assertEqual(n, 2)
        lines = [json.loads(x) for x in out.getvalue().splitlines()]
        self.assertIn('procs', lines[-1])
        self.assertIn('totals', lines[-1])

    def test_serve_ends_with_stdin(self):
        r, wfd = os.pipe()
        os.write(wfd, b'{"interval": 0}\n')
        os.close(wfd)
        out = io.StringIO()
        n = ht.serve(r, out, self.w.proc, self.w.sys)
        os.close(r)
        self.assertLessEqual(n, 1, 'paused: at most one sample for the change; it ends when the window closes stdin (no hang)')


class Startup(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(dir=TMP)
        self.user = os.path.join(self.d, 'user')
        self.system = [os.path.join(self.d, 'xdg')]
        write(os.path.join(self.system[0], 'syncer.desktop'), '[Desktop Entry]\nType=Application\nName=Syncer\nExec=syncer --tray\n')
        write(os.path.join(self.system[0], 'hearth-thing.desktop'), '[Desktop Entry]\nType=Application\nName=Hearth thing\nExec=hearth-thing\n')

    def digest(self):
        h = hashlib.sha256()
        for root, _, files in sorted(os.walk(self.system[0])):
            for f in sorted(files):
                h.update(f.encode() + pathlib.Path(root, f).read_bytes())
        return h.hexdigest()

    def test_disable_writes_only_your_copy(self):
        before = self.digest()
        ht.set_autostart('syncer.desktop', False, self.user, self.system)
        mine = pathlib.Path(self.user, 'syncer.desktop').read_text()
        self.assertIn('Hidden=true', mine)
        self.assertEqual(self.digest(), before, 'the system copy was changed')
        ht.set_autostart('syncer.desktop', True, self.user, self.system)
        self.assertFalse(os.path.exists(os.path.join(self.user, 'syncer.desktop')), 'turning it back on removes your "off" copy')
        self.assertEqual(self.digest(), before)

    def test_hearth_own_are_part_of_hearth(self):
        items = {i['name']: i for i in ht.autostart_entries(self.user, self.system)}
        self.assertEqual(items['Hearth thing']['kind'], 'hearth-part')
        self.assertFalse(items['Hearth thing']['canChange'])
        self.assertEqual(items['Hearth thing']['publisher'], 'Hearth')
        self.assertEqual(items['Syncer']['kind'], 'autostart')
        self.assertEqual(items['Syncer']['publisher'], 'syncer')
        with self.assertRaises(ht.Fail):
            ht.startup_set('hearth-part:hearth-thing.desktop', False)
        with self.assertRaises(ht.Fail):
            ht.service_act('hearth-core.service', 'stop')
        with self.assertRaises(ht.Fail):
            ht.service_act('pipewire.service', 'stop')


class Settings(unittest.TestCase):
    QML = (SHELL / 'TaskManager.qml').read_text()
    VIEW = (SHELL / 'TaskManagerView.qml').read_text()

    def test_written_only_through_the_safe_helper(self):
        for name, text in (('TaskManager.qml', self.QML), ('TaskManagerView.qml', self.VIEW)):
            self.assertNotIn('setText(', text, name)
            self.assertNotIn('writer.merge(', text.replace('        writer.merge(c);', ''), name + ' merges outside setPref')
        self.assertIn('SettingsWriter { id: writer; path: tm.prefsPath }', self.QML)
        self.assertIn('/nimbus/taskmgr.json', self.QML)

    def test_opening_writes_nothing(self):
        """open() (IPC, the launcher) shows a page without remembering anything; only go() (the person) does."""
        body = re.search(r'function open\(p\) \{(.*?)\n    \}', self.QML, re.S).group(1)
        self.assertNotIn('setPref', body)
        self.assertNotIn('setPref', re.search(r'function resolve\(p\) \{(.*?)\n    \}', self.QML, re.S).group(1))
        # setPref skips a value that's already there, and a damaged file is repaired, never replaced
        self.assertIn('if (JSON.stringify(prefs[k]) === JSON.stringify(v)) return;', self.QML)
        self.assertIn('writer.repair()', self.QML)

    def test_every_choice_is_kept(self):
        for key in ('"speed"', '"startPage"', '"lastPage"', '"onTop"', '"railFolded"'):
            self.assertIn(key, self.QML + self.VIEW)
        for key in ('"cols_procs"', '"sort_" +', 'cols_details', 'perfDevice'):
            self.assertIn(key, self.VIEW)

    def test_merge_keeps_other_keys_and_damaged_file(self):
        f = os.path.join(TMP, 'config', 'nimbus', 'taskmgr.json')
        write(f, json.dumps({'speed': 'low', 'cols_procs': {'cpu': 90}}))
        hsio.merge_json(f, {'onTop': True})
        self.assertEqual(json.loads(pathlib.Path(f).read_text()), {'speed': 'low', 'cols_procs': {'cpu': 90}, 'onTop': True})
        write(f, '{"speed": "lo')            # damaged (cut short)
        hsio.merge_json(f, {'railFolded': True})
        d = json.loads(pathlib.Path(f).read_text())
        self.assertEqual(d.get('speed'), 'low', 'the damaged file came back from .bak, not as defaults')
        self.assertTrue(d['railFolded'])


if __name__ == '__main__':
    unittest.main()
