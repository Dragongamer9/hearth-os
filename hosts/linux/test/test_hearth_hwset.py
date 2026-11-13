"""Task Manager › Performance's hardware settings (os/system_files/usr/libexec/nimbus/hearth-hwset, 1.11): turbo boost,
the top speed and the battery's charge limit, read and written on a pretend /sys, and remembered in one file."""
import importlib.machinery
import importlib.util
import json
import os
import shutil
import tempfile
import unittest
from unittest import mock

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
loader = importlib.machinery.SourceFileLoader('hearth_hwset', os.path.join(ROOT, 'os/system_files/usr/libexec/nimbus/hearth-hwset'))
spec = importlib.util.spec_from_loader('hearth_hwset', loader)
hw = importlib.util.module_from_spec(spec)
loader.exec_module(hw)


def put(root, rel, text):
    p = os.path.join(root, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    open(p, 'w').write(text)


class Hw(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.sys = os.path.join(self.d, 'sys')
        put(self.sys, 'devices/system/cpu/intel_pstate/no_turbo', '0\n')
        put(self.sys, 'devices/system/cpu/intel_pstate/max_perf_pct', '100\n')
        put(self.sys, 'devices/system/cpu/cpu0/thermal_throttle/core_throttle_count', '3\n')
        b = 'class/power_supply/BAT0/'
        for k, v in (('type', 'Battery'), ('energy_full', '45000000'), ('energy_full_design', '50000000'), ('cycle_count', '120'),
                     ('capacity', '82'), ('status', 'Discharging'), ('charge_control_end_threshold', '100')):
            put(self.sys, b + k, v + '\n')
        self.p = mock.patch.multiple(hw, SYS=self.sys, SETTINGS=os.path.join(self.d, 'etc/hardware.json'))
        self.p.start()

    def tearDown(self):
        self.p.stop()
        shutil.rmtree(self.d)

    def test_status(self):
        s = hw.status()
        self.assertEqual(s['turbo'], {'can': True, 'on': True})
        self.assertEqual(s['maxPct']['value'], 100)
        self.assertEqual(s['throttled'], 3)
        self.assertEqual((s['batteries'][0]['health'], s['batteries'][0]['cycles'], s['batteries'][0]['canLimit']), (90, 120, True))

    def test_set_is_remembered_and_put_back(self):
        hw.set_one('cpu.turbo', 'off')
        hw.set_one('battery.limit', '80')
        self.assertEqual(open(os.path.join(self.sys, 'devices/system/cpu/intel_pstate/no_turbo')).read(), '1')
        self.assertEqual(json.load(open(hw.SETTINGS)), {'cpu.turbo': False, 'battery.limit': 80})
        put(self.sys, 'devices/system/cpu/intel_pstate/no_turbo', '0')      # a restart
        put(self.sys, 'class/power_supply/BAT0/charge_control_end_threshold', '100')
        self.assertEqual(hw.apply()['applied'], {'cpu.turbo': False, 'battery.limit': 80})
        self.assertEqual(open(os.path.join(self.sys, 'class/power_supply/BAT0/charge_control_end_threshold')).read(), '80')

    def test_limits(self):
        for k, v in (('cpu.maxPct', '20'), ('battery.limit', '40'), ('cpu.turbo', 'maybe'), ('fan', '1')):
            with self.assertRaises(hw.Fail):
                hw.set_one(k, v)
        self.assertFalse(os.path.exists(hw.SETTINGS))

    def test_no_file_means_nothing_changes_at_start(self):
        self.assertEqual(hw.apply(), {'ok': True, 'applied': {}})



class Drives(unittest.TestCase):
    def test_kinds(self):
        d = tempfile.mkdtemp()
        try:
            for name, rot, disc in (('nvme0n1', '0', '2199023255040'), ('sda', '1', '0'), ('loop0', '0', '0')):
                put(d, 'block/%s/queue/rotational' % name, rot)
                put(d, 'block/%s/queue/discard_max_bytes' % name, disc)
                put(d, 'block/%s/device/model' % name, 'X')
            with mock.patch.object(hw, 'SYS', d), mock.patch.object(hw, '_timer_times', return_value={'UnitFileState': 'enabled'}):
                r = hw.drives()
            self.assertEqual([(x['name'], x['kind'], x['trim']) for x in r['drives']], [('nvme0n1', 'NVMe', True), ('sda', 'Hard drive', False)])
            self.assertTrue(r['timer']['on'])
        finally:
            shutil.rmtree(d)

if __name__ == '__main__':
    unittest.main()
