"""Task Manager's App history and Game cores (1.11, the approved mockups tm-8 and tm-3)."""
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ht = load('hearth_tasks_hist', SHELL / 'hearth-tasks')
hw = load('hearth_hwset_gc', ROOT / 'os/system_files/usr/libexec/nimbus/hearth-hwset')


class AppHistory(unittest.TestCase):
    def test_adds_growth_and_new_processes(self):
        st = ht.history_add({}, {'1:100': ['Firefox', 10.0, 1000, 500]}, 1000)
        self.assertEqual(st['apps']['Firefox'], {'cpu': 10.0, 'read': 1000, 'write': 500, 'last': 1000})
        st = ht.history_add(st, {'1:100': ['Firefox', 12.5, 1500, 500], '2:200': ['Steam', 3.0, 0, 9000]}, 1300)
        self.assertEqual(st['apps']['Firefox']['cpu'], 12.5)
        self.assertEqual(st['apps']['Firefox']['read'], 1500)
        self.assertEqual(st['apps']['Firefox']['last'], 1300)
        self.assertEqual(st['apps']['Steam']['write'], 9000)
        self.assertEqual(st['since'], 1000)
        # nothing new used: last stays
        st = ht.history_add(st, {'1:100': ['Firefox', 12.5, 1500, 500]}, 1600)
        self.assertEqual(st['apps']['Firefox']['last'], 1300)

    def test_written_only_in_state(self):
        with tempfile.TemporaryDirectory() as d:
            os.environ['XDG_STATE_HOME'] = d
            try:
                ht.history_write({'since': 1, 'apps': {'A': {'cpu': 2, 'read': 0, 'write': 0, 'last': 1}}})
                self.assertTrue(os.path.isfile(os.path.join(d, 'hearth', 'app-history.json')))
                self.assertEqual(ht.history_list()['apps'][0]['name'], 'A')
            finally:
                del os.environ['XDG_STATE_HOME']

    def test_page_and_timer(self):
        qml = (SHELL / 'TaskManagerView.qml').read_text()
        self.assertIn('{ id: "history", icon: "undo", label: "App history" }', qml)
        self.assertIn('"Delete usage history"', qml)
        self.assertIn('"history", "startup"', (SHELL / 'TaskManager.qml').read_text())
        unit = (ROOT / 'os/system_files/usr/lib/systemd/user/hearth-app-history.service').read_text()
        self.assertIn('hearth-tasks history-collect', unit)
        self.assertTrue((ROOT / 'os/system_files/usr/lib/systemd/user/timers.target.wants/hearth-app-history.timer').is_symlink())


class GameCores(unittest.TestCase):
    def test_cpu_lists(self):
        self.assertEqual(hw.parse_cpus('4-7', 8), [4, 5, 6, 7])
        self.assertEqual(hw.parse_cpus('0,2,4-5', 8), [0, 2, 4, 5])
        self.assertEqual(hw.cpu_text([0, 1, 2, 3, 6]), '0-3,6')
        with self.assertRaises(hw.Fail):
            hw.parse_cpus('4-9', 8)
        with self.assertRaises(hw.Fail):
            hw.parse_cpus('x', 8)
        self.assertEqual(hw.default_game_cores(8), [4, 5, 6, 7])

    def test_always_leaves_the_system_a_core(self):
        n = hw.logical_cpus()
        if n >= 4:
            with self.assertRaises(hw.Fail):
                hw.parse('game.cores', '0-%d' % (n - 1))
            self.assertEqual(hw.parse('game.cores', 'on'), hw.cpu_text(hw.default_game_cores(n)))
        self.assertEqual(hw.parse('game.cores', 'off'), 'off')

    def test_what_counts_as_a_game(self):
        self.assertTrue(hw.is_game_env(b'HOME=/h\x00SteamGameId=1234\x00', 'eldenring.exe'))
        self.assertTrue(hw.is_game_env(b'HEARTH_PROTON_KEY=abc\x00', 'game.exe'))
        self.assertFalse(hw.is_game_env(b'SteamGameId=1234\x00', 'wineserver'))
        self.assertFalse(hw.is_game_env(b'SteamGameId=0\x00', 'steam-thing'))
        self.assertFalse(hw.is_game_env(b'HOME=/h\x00', 'firefox'))

    def test_service_is_safe(self):
        unit = (ROOT / 'os/system_files/usr/lib/systemd/system/hearth-gamecores.service').read_text()
        self.assertNotIn('Before=', unit)
        self.assertIn('hearth-hwset gamecores', unit)
        src = (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-hwset').read_text()
        self.assertIn("'set-property', '--runtime', 'system.slice'", src)      # never kept past a restart
        self.assertIn("finally:\n        if active:\n            system_cores('')", src)

    def test_settings_page(self):
        qml = (SHELL / 'DeviceSettings.qml').read_text()
        self.assertIn('title: "Game cores"', qml)
        self.assertIn('ds.hwset("game.cores"', qml)


if __name__ == '__main__':
    unittest.main()
