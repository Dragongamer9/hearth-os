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
hw = load('hearth_hwset_tmmock', ROOT / 'os/system_files/usr/libexec/nimbus/hearth-hwset')
ni = load('hearth_netinfo_tmmock', SHELL / 'hearth-netinfo')
gt = load('hearth_gputune_tmmock', SHELL / 'hearth-gputune')


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


class DiskSettings(unittest.TestCase):
    def test_smart_nvme(self):
        j = {'smart_status': {'passed': True}, 'model_name': 'SK hynix', 'power_on_time': {'hours': 1412}, 'temperature': {'current': 41},
             'nvme_smart_health_information_log': {'percentage_used': 3, 'data_units_written': 27734375}}
        r = hw.parse_smart(j)
        self.assertEqual((r['ok'], r['lifeLeft'], r['hours'], r['temp']), (True, 97, 1412, 41))
        self.assertEqual(r['written'], 27734375 * 512000)

    def test_smart_sata(self):
        j = {'smart_status': {'passed': True}, 'ata_smart_attributes': {'table': [{'name': 'Total_LBAs_Written', 'raw': {'value': 1000}},
                                                                                   {'name': 'Wear_Leveling_Count', 'value': 95}]}}
        r = hw.parse_smart(j)
        self.assertEqual((r['written'], r['lifeLeft']), (512000, 95))

    def test_snapshots_kept(self):
        names = ['2026-11-%02d_%02d00' % (d, h) for d in range(1, 21) for h in range(0, 24)]
        keep = hw.snapshots_to_keep(names)
        self.assertIn('2026-11-20_2300', keep)
        self.assertIn('2026-11-07_2300', keep)          # the newest of each of the last 14 days
        self.assertNotIn('2026-11-06_2300', keep)
        self.assertEqual(len(keep), 24 + 13)

    def test_choices(self):
        self.assertEqual(hw.parse('disk.commit', '10'), '10')
        self.assertEqual(hw.parse('battery.at5', 'sleep'), 'sleep')
        with self.assertRaises(hw.Fail):
            hw.parse('disk.commit', '7')
        with self.assertRaises(hw.Fail):
            hw.parse('power.plugged', 'turbo')


class Rules(unittest.TestCase):
    def test_parse_rule(self):
        self.assertEqual(hw.parse_rule('cpu', '50'), '50')
        self.assertEqual(hw.parse_rule('cpu', 'low'), 'low')
        self.assertIsNone(hw.parse_rule('cpu', 'remove'))
        self.assertEqual(hw.parse_rule('net', 'limit:5'), 'limit:5')
        for bad in (('cpu', '95'), ('net', 'limit:0'), ('net', 'maybe')):
            with self.assertRaises(hw.Fail):
                hw.parse_rule(*bad)

    def test_nft_only_for_live_boxes(self):
        t = hw.nft_rules({'UaW.exe': 'block', 'steam': 'limit:5', 'gone': 'block'}, {'hearth-app-uaw-exe.scope', 'hearth-app-steam.scope'})
        self.assertIn('socket cgroupv2 level 2 "hearth-apps.slice/hearth-app-uaw-exe.scope" drop', t)
        self.assertIn('limit rate over 5 mbytes/second drop', t)
        self.assertNotIn('gone', t)

    def test_memory_board(self):
        text = ('Memory Device\n\tSize: 8 GB\n\tType: DDR5\n\tConfigured Memory Speed: 4800 MT/s\n\n'
                'Memory Device\n\tSize: No Module Installed\n')
        self.assertEqual(hw.parse_dmidecode_memory(text), {'slots': 2, 'used': 1, 'speed': 4800, 'type': 'DDR5'})


class Network(unittest.TestCase):
    def test_ping(self):
        self.assertEqual(ni.parse_ping('rtt min/avg/max/mdev = 12.1/18.4/30.2/3.2 ms'), (18, 3))
        self.assertEqual(ni.parse_ping(''), (None, None))

    def test_used_today(self):
        st = ni.today_add({}, 1000, 'b1', '2026-11-13')
        st = ni.today_add(st, 1600, 'b1', '2026-11-13')
        self.assertEqual(st['bytes'], 600)
        st = ni.today_add(st, 100, 'b2', '2026-11-13')      # restarted: counters start again
        self.assertEqual(st['bytes'], 600)
        self.assertEqual(ni.today_add(st, 900, 'b2', '2026-11-14')['bytes'], 0)

    def test_totals_skip_loopback_and_tailscale(self):
        dev = ('h\nh\n    lo: 500 0 0 0 0 0 0 0 500 0\n'
               ' wlan0: 1000 0 0 0 0 0 0 0 300 0\n'
               'tailscale0: 99 0 0 0 0 0 0 0 99 0\n')
        self.assertEqual(ni.net_totals(dev), 1300)


class Gpu(unittest.TestCase):
    def test_curve(self):
        self.assertEqual(gt.parse_curve('40:30,60:45,80:80'), {40: 30, 60: 45, 80: 80})
        for bad in ('40:30', '40:60,60:30', '10:5,40:30', 'x'):
            with self.assertRaises(gt.Fail):
                gt.parse_curve(bad)
        self.assertEqual(gt.curve_text({'60': 0.45, '40': 0.3}), '40:30,60:45')


class BatteryAndStartup(unittest.TestCase):
    def test_battery_today(self):
        st = ht.battery_add({}, {'1:1': ['Firefox', 10.0, 0, 0]}, {'1:1': ['Firefox', 25.0, 0, 0]}, 1000.0, '2026-11-13', True, 70)
        st = ht.battery_add(st, {'1:1': ['Firefox', 25.0, 0, 0]}, {'1:1': ['Firefox', 30.0, 0, 0]}, 1300.0, '2026-11-13', True, 70)
        self.assertEqual(st['battery']['apps']['Firefox'], 20.0)
        self.assertEqual(st['battery']['screen'], 300.0)
        st = ht.battery_add(st, {}, {}, 1400.0, '2026-11-13', False, 70)       # plugged in: nothing counted
        self.assertEqual(st['battery']['screen'], 300.0)

    def test_swap_pages_and_vram(self):
        self.assertEqual(ht.vm_swap_pages('pswpin 10\npswpout 20\n'), (10, 20))
        self.assertEqual(ht.fdinfo_vram('drm-memory-vram:\t2048 KiB\n'), 2048 * 1024)
        self.assertEqual(ht.fdinfo_vram(''), 0)

    def test_starts(self):
        self.assertEqual(ht.starts_label('enabled'), 'Automatic')
        self.assertEqual(ht.starts_label('static'), 'When needed')
        self.assertEqual(ht.starts_label('disabled'), 'Off')
        self.assertEqual(ht.plain_service('cups.service', 'CUPS Scheduler'), 'Printing (CUPS): your printers')
        self.assertTrue(ht.MINIMIZED.search('discord --start-minimized'))

    def test_new_task_checks(self):
        for w in ('daily@03:00', 'signin', 'hourly', 'every6h', 'idle', 'battery'):
            self.assertTrue(ht.TASK_WHEN.match(w))
        self.assertFalse(ht.TASK_WHEN.match('daily@25:00'))
        with self.assertRaises(ht.Fail):
            ht.task_new('x', 'sometimes', 'true')


if __name__ == '__main__':
    unittest.main()
