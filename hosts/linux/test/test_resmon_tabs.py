"""Resource Monitor's Disk, Network and Graphics tabs (1.11, mockups resmon-4, -5, -6): per-program download and
upload speeds and ping from ss, 3D / video decode / encode per program from the DRM engines, each drive's kind,
space and temperature, and the files the busiest programs have open for writing."""
import importlib.machinery
import importlib.util
import os
import pathlib
import sys
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


rm = load('hearth_resmon_tabs', SHELL / 'hearth-resmon')
ht = load('hearth_tasks_tabs', SHELL / 'hearth-tasks')
QML = (SHELL / 'ResourceMonitorView.qml').read_text()

SS = '''0 0 192.168.1.5:51234 140.82.112.3:443 users:(("steam",pid=4410,fd=55)) cubic rtt:22.1/3.2 bytes_sent:900 bytes_acked:800 bytes_received:5000
0 0 [::ffff:192.168.1.5]:40000 [2606:4700::1]:443 users:(("firefox-bin",pid=3835,fd=9))
\t cubic rtt:31.5/2 bytes_acked:100 bytes_received:200
ESTAB 0 0 100.64.1.2:22 100.84.2.9:41641
'''


class Network(unittest.TestCase):
    def test_parse_ss(self):
        s = rm.parse_ss(SS)
        self.assertEqual([x['pid'] for x in s], [4410, 3835, 0])
        self.assertEqual(s[0]['rtt'], 22.1)
        self.assertEqual((s[0]['recv'], s[0]['sent']), (5000, 800))
        self.assertEqual(s[1]['peer'], '2606:4700::1')       # the indented next line joined
        self.assertEqual(s[1]['rtt'], 31.5)
        self.assertEqual((s[2]['peer'], s[2]['port']), ('100.84.2.9', 41641))   # a State column skipped

    def test_rates(self):
        s = rm.parse_ss(SS)
        prev = {'%s|%s:%s|%s' % (x['local'], x['peer'], x['port'], x['pid']): [x['recv'] - 2000, x['sent'] - 400] for x in s}
        r = rm.tcp_rates(s, prev, 2)
        self.assertEqual(r[4410], {'down': 1000, 'up': 200})
        self.assertNotIn(0, r)
        self.assertEqual(rm.tcp_rates(s, {}, 2), {})
        self.assertEqual(rm.tcp_rates(s, prev, 0), {})

    def test_view(self):
        self.assertIn('network: ["Down", "Up", "Total ↓"]', QML)
        self.assertIn('"Ping"', QML)


class Graphics(unittest.TestCase):
    def test_engine_kinds(self):
        for e, k in (('render', '3d'), ('gfx', '3d'), ('rcs', '3d'), ('video', 'dec'), ('dec', 'dec'), ('vcs', 'dec'),
                     ('enc', 'enc'), ('enc_1', 'enc'), ('video-enhance', 'enc'), ('copy', ''), ('compute', '')):
            self.assertEqual(ht.engine_kind(e), k, e)

    def test_view(self):
        self.assertIn('gpu: ["3D ↓", "Video decode", "Video memory"]', QML)
        self.assertIn('["Video decode", v.pct(last("gdec"))', QML)


class Disk(unittest.TestCase):
    def test_drive_temp(self):
        with tempfile.TemporaryDirectory() as d:
            h = os.path.join(d, 'block/nvme0n1/device/hwmon/hwmon3')
            os.makedirs(h)
            open(os.path.join(h, 'temp1_input'), 'w').write('41850\n')
            self.assertEqual(rm.drive_temp('nvme0n1', d), 42)
            self.assertIsNone(rm.drive_temp('sda', d))

    def test_fs_of(self):
        m = '/dev/nvme0n1p3 /sysroot btrfs rw 0 0\n/dev/sdb1 /run/media/me/My\\040Games ext4 rw 0 0\n'
        self.assertEqual(rm.fs_of('/sysroot', m), 'btrfs')
        self.assertEqual(rm.fs_of('/run/media/me/My Games', m), 'ext4')

    def test_writing_files_mine(self):
        with tempfile.TemporaryDirectory() as d:
            f = os.path.join(d, 'chunk.bin')
            with open(f, 'w') as fh:
                fh.write('x')
                out = rm.writing_files([os.getpid()])
                me = [x for x in out if x['path'] == os.path.realpath(f)]
                self.assertTrue(me and me[0]['writing'])

    def test_view(self):
        self.assertIn('"Files being read and written"', QML)
        for col in ('"Drive", "Type", "Busy", "Response", "Free", "Total", "Full", "Temp"',):
            self.assertIn(col, QML)
        self.assertIn('["python3", v.helper, "drives"]', QML)


if __name__ == '__main__':
    unittest.main()
