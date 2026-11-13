"""Task Manager built completely to its approved mockups (1.12, red flag 11; docs/mockups/checklists/task-manager.md)."""
import importlib.machinery
import importlib.util
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ht = load('hearth_tasks_tmmock', SHELL / 'hearth-tasks')


def stat_line(pid, comm, policy):
    rest = ['S', '1'] + ['0'] * 9 + ['5', '6'] + ['0'] * 3 + ['0', '4'] + ['0', '999'] + ['0'] * 18 + [str(policy)] + ['0'] * 5
    return '%d (%s) %s' % (pid, comm, ' '.join(rest))


class Details(unittest.TestCase):
    def test_cores_label(self):
        self.assertEqual(ht.cores_label('0-7', 8), 'All')
        self.assertEqual(ht.cores_label('', 8), 'All')
        self.assertEqual(ht.cores_label('4-7', 8), '4–7')
        self.assertEqual(ht.cores_label('0,2', 8), '0,2')

    def test_realtime_from_stat(self):
        self.assertTrue(ht.parse_stat(stat_line(5, 'game', 1))['rt'])
        self.assertFalse(ht.parse_stat(stat_line(5, 'game', 0))['rt'])

    def test_cpus_allowed(self):
        self.assertEqual(ht.parse_status('Uid:\t1000\t1000\t1000\t1000\nCpus_allowed_list:\t4-7\n')['cpus'], '4-7')

    def test_suspend_needs_a_live_process(self):
        with self.assertRaises(ht.Fail):
            ht.suspend(999999999, True)


class Processes(unittest.TestCase):
    def test_ai_jobs(self):
        self.assertTrue(ht.ai_job(['/usr/bin/claude', '-p']))
        self.assertTrue(ht.ai_job(['node', '/opt/codex/bin/codex']))
        self.assertFalse(ht.ai_job(['/usr/bin/firefox']))
        self.assertFalse(ht.ai_job([]))

    def test_page_parts(self):
        qml = (SHELL / 'TaskManagerView.qml').read_text()
        for s in ('tabs: [["groups", "Apps & groups"], ["all", "All processes"]]', '"Quick Resume ⏸"', '{ id: "where", label: "Open file location"',
                  'label: "Realtime"', 'label: "Create dump file"', '"Columns…"', 'title: "CPU time"', 'title: "Cores"',
                  'every process with its number, the Windows way'):
            self.assertIn(s, qml)


if __name__ == '__main__':
    unittest.main()
