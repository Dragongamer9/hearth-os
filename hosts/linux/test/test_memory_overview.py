"""Task Manager › Performance › Memory's Overview (1.12, the approved mockup task-manager-memory.png)."""
import importlib.machinery
import importlib.util
import pathlib
import tempfile
import os
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ht = load('hearth_tasks_memov', SHELL / 'hearth-tasks')


class Parsers(unittest.TestCase):
    def test_swaps_split_zram_and_drive(self):
        t = ('Filename Type Size Used Priority\n/dev/zram0 partition 8388604 1024 100\n'
             '/var/swap/swapfile file 4194300 2048 -2\n')
        self.assertEqual(ht.parse_swaps(t), {'zramSize': 8388604 * 1024, 'zramUsed': 1024 * 1024,
                                             'driveSize': 4194300 * 1024, 'driveUsed': 2048 * 1024})
        self.assertEqual(ht.parse_swaps(''), {'zramSize': 0, 'zramUsed': 0, 'driveSize': 0, 'driveUsed': 0})

    def test_pressure(self):
        self.assertEqual(ht.parse_psi('some avg10=1.20 avg60=0.40 avg300=0.10 total=9\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0'),
                         {'psi10': 1.2, 'psi60': 0.4})
        self.assertEqual(ht.parse_psi(''), {'psi10': None, 'psi60': None})

    def test_mm_stat(self):
        self.assertEqual(ht.parse_mm_stat('3000 1000 1100 0 1200 0 0 0 0'), {'zramOrig': 3000, 'zramCompr': 1000, 'zramMem': 1100})
        self.assertEqual(ht.parse_mm_stat(''), {'zramOrig': 0, 'zramCompr': 0, 'zramMem': 0})

    def test_extras_without_zram(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(ht.mem_extras(d, d)['zramMem'], 0)

    def test_shared_memory_per_process(self):
        st = ht.parse_status('Name:\tx\nUid:\t1000\t1000\t1000\t1000\nRssAnon:\t100 kB\nRssFile:\t300 kB\nRssShmem:\t20 kB\nVmSwap:\t50 kB\n')
        self.assertEqual(st['mem'], 120 * 1024)
        self.assertEqual(st['shared'], 320 * 1024)
        self.assertEqual(st['swap'], 50 * 1024)


class Page(unittest.TestCase):
    def test_overview_follows_the_mockup(self):
        qml = (SHELL / 'MemoryOverview.qml').read_text()
        for s in ('Memory in use (programs) · 60 seconds', 'compressed (zram)', 'Memory composition', '"Swap"',
                  '"Memory pressure"', '"Memory setting"', 'BIGGEST USERS', 'hearth-memory", "status"', 'setLineDash'):
            self.assertIn(s, qml)
        view = (SHELL / 'TaskManagerView.qml').read_text()
        self.assertIn('MemoryOverview {', view)
        self.assertIn('push(h, "memz"', view)
        self.assertIn('tm.perfDevice === "memory" ? ["gpu", "temps", "procs"]', view)


if __name__ == '__main__':
    unittest.main()
