"""Home server (1.11): night maintenance once a night when nobody uses the PC, and 30 days of power history."""
import importlib.machinery
import importlib.util
import pathlib
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


h = load('hearth_hs_helper_night', ROOT / 'os/system_files/usr/libexec/nimbus/hearth-home-server-helper')


def at(hh, mm):
    t = list(time.localtime())
    t[3], t[4], t[5] = hh, mm, 0
    return time.mktime(tuple(t))


class Night(unittest.TestCase):
    def test_off_by_default_and_kept(self):
        p = h.clean_power({})
        self.assertFalse(p['night'])
        self.assertFalse(p['history'])
        self.assertEqual(h.clean_power({'night': True, 'nightAt': '04:30'})['nightAt'], '04:30')
        self.assertEqual(h.clean_power({'nightAt': '13:00'})['nightAt'], '03:30')

    def test_due_once_in_its_hour_when_idle(self):
        p = h.clean_power({'night': True, 'nightAt': '03:30'})
        self.assertTrue(h.night_due(at(3, 45), p, {}, [], False))
        self.assertFalse(h.night_due(at(5, 0), p, {}, [], False))
        self.assertFalse(h.night_due(at(3, 45), p, {}, ['Game Mode'], False))
        self.assertFalse(h.night_due(at(3, 45), p, {}, [], True))
        today = time.strftime('%Y-%m-%d', time.localtime(at(3, 45)))
        self.assertFalse(h.night_due(at(3, 45), p, {'night': today}, [], False))
        self.assertFalse(h.night_due(at(3, 45), h.clean_power({}), {}, [], False))

    def test_steps_never_touch_booting(self):
        for name, cmd, _ in h.NIGHT_STEPS:
            self.assertNotIn(cmd[0], ('bootc', 'rpm-ostree', 'grub-install', 'bootupctl', 'efibootmgr', 'hearth-admin'))

    def test_timer_runs_while_on(self):
        src = (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-home-server-helper').read_text()
        self.assertIn("or power['night'] or power['history'])", src)


class PowerHistory(unittest.TestCase):
    def test_add_and_trim(self):
        pts = []
        for i in range(10):
            pts = h.history_add(pts, [i, 5.0, None], limit=4)
        self.assertEqual([p[0] for p in pts], [6, 7, 8, 9])
        self.assertEqual(h.history_add(pts, [10, None, None], limit=4), pts)

    def test_watts_from_the_counter(self):
        st = {'rapl': [100.0, 1_000_000]}
        h.rapl_uj = lambda root='/sys/class/powercap': 1_000_000 + 60_000_000      # 60 J in 5 s
        h.gpu_watts = lambda: 30.0
        p = h.history_sample(105.0, st)
        self.assertEqual(p, [105, 12.0, 30.0])

    def test_settings_page(self):
        q = (ROOT / 'hosts/linux/desktop/shell/SetHomeAlerts.qml').read_text()
        self.assertIn('title: "Night maintenance"', q)
        self.assertIn('title: "Power history"', q)
        u = (ROOT / 'hosts/linux/desktop/shell/hearth-home-server').read_text()
        self.assertIn("'powerHistory': ('power', 'history', 'bool')", u)


if __name__ == '__main__':
    unittest.main()
