"""1.13: Server mode › Drive health. Each drive's self-check (SMART), temperature, hours and signs of wear (sectors
moved or waiting, NVMe life used and spare blocks), recorded by the root helper for Server mode's tile; a warning on the
dashboard (even with Alerts off) and on your phone (Alerts) before a drive fails."""
import importlib.machinery
import importlib.util
import json
import os
import tempfile
import unittest

HERE = os.path.dirname(__file__)
SHELL = os.path.join(HERE, '..', 'desktop', 'shell')
HELPER = os.path.join(HERE, '..', '..', '..', 'os', 'system_files', 'usr', 'libexec', 'nimbus', 'hearth-home-server-helper')


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, path)
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


helper = load('hhs_helper_drives', HELPER)
sm = load('hsm_drives', os.path.join(SHELL, 'hearth-server-mode'))

HEALTHY_SSD = {'smart_status': {'passed': True}, 'temperature': {'current': 38}, 'power_on_time': {'hours': 9000},
               'ata_smart_attributes': {'table': [{'id': 5, 'raw': {'value': 0}}, {'id': 197, 'raw': {'value': 0}}]}}
WEARING_HDD = {'smart_status': {'passed': True}, 'temperature': {'current': 44}, 'power_on_time': {'hours': 31000},
               'ata_smart_attributes': {'table': [{'id': 5, 'raw': {'value': 24}}, {'id': 197, 'raw': {'value': 3}}]}}
OLD_NVME = {'smart_status': {'passed': True}, 'temperature': {'current': 51}, 'power_on_time': {'hours': 20000},
            'nvme_smart_health_information_log': {'percentage_used': 95, 'available_spare': 12, 'available_spare_threshold': 10, 'media_errors': 0, 'critical_warning': 0}}
FAILING = {'smart_status': {'passed': False}, 'temperature': {'current': 40}}


class Read(unittest.TestCase):
    def test_healthy_wearing_and_failing(self):
        self.assertEqual(helper.smart_read(HEALTHY_SSD)['level'], 'ok')
        w = helper.smart_read(WEARING_HDD)
        self.assertEqual(w['level'], 'watch')
        self.assertEqual(w['signs'], ['24 sectors moved to spares', '3 sectors waiting to be moved'])
        n = helper.smart_read(OLD_NVME)
        self.assertEqual((n['level'], n['lifeUsed']), ('watch', 95))
        self.assertIn('only 12% spare blocks left', n['signs'])
        self.assertEqual(helper.smart_read(FAILING)['level'], 'failing')

    def test_the_record_holds_no_paths_or_files(self):
        with tempfile.TemporaryDirectory() as t:
            p = os.path.join(t, 'drives.json')
            orig = helper.DRIVE_HEALTH
            helper.DRIVE_HEALTH = p
            try:
                helper.health_record([{'model': 'WD Red', 'sizeText': '4 TB', 'kind': 'HDD', 'system': False, 'usb': False, 'smart': helper.smart_read(WEARING_HDD),
                                       'mounts': ['/mnt/data'], 'path': '/dev/sdb', 'token': 'x'}], now=100)
            finally:
                helper.DRIVE_HEALTH = orig
            rec = json.load(open(p))
            self.assertEqual(rec['at'], 100)
            self.assertNotIn('mounts', rec['drives'][0]); self.assertNotIn('path', rec['drives'][0])

    def test_an_early_warning_alert(self):
        orig = helper.drives
        helper.drives = lambda: {'drives': [{'model': 'WD Red', 'token': 't1', 'smart': helper.smart_read(WEARING_HDD)}]}
        try:
            found = helper.alerts_find({'alerts': {'on': True, 'smart': True, 'full': False, 'backup': False, 'mirror': False}}, 0)
        finally:
            helper.drives = orig
        self.assertEqual(found[0]['kind'], 'wear')
        self.assertIn('WD Red is showing early signs of wearing out (24 sectors moved to spares, 3 sectors waiting to be moved): back it up soon', found[0]['text'])


class Tile(unittest.TestCase):
    def test_the_tile(self):
        rec = {'at': 1000, 'drives': [
            {'model': 'Samsung 990 Pro', 'system': True, 'smart': helper.smart_read(HEALTHY_SSD)},
            {'model': 'WD Red', 'system': False, 'smart': helper.smart_read(WEARING_HDD)}]}
        tile, warn = sm.drives_tile(rec, now=2000)
        self.assertEqual((tile['id'], tile['chip'], tile['state'], tile['big']), ('drives', 'Watch', 'warn', '2'))
        self.assertEqual(tile['rows'][0], ['Samsung 990 Pro (system)', 'Good · 38 °C · 1 yr'])
        self.assertTrue(tile['rows'][1][1].startswith('Wearing · 44 °C · 3 yr'))
        self.assertEqual(len(warn), 1); self.assertIn('WD Red is wearing out', warn[0]['text'])
        healthy, none = sm.drives_tile({'drives': [{'model': 'A', 'smart': helper.smart_read(HEALTHY_SSD)}]})
        self.assertEqual((healthy['chip'], none), ('Healthy', []))
        empty, _ = sm.drives_tile({})
        self.assertEqual((empty['chip'], empty['state']), ('Checking', 'off'))

    def test_an_old_record_is_looked_at_again_in_the_background(self):
        looked = []
        sm._drive_look['at'] = 0
        with tempfile.TemporaryDirectory() as t:
            p = os.path.join(t, 'd.json')
            json.dump({'at': 0, 'drives': []}, open(p, 'w'))
            sm.drive_health(now=100000, path=p, look=lambda: looked.append(1))
            sm.drive_health(now=100060, path=p, look=lambda: looked.append(1))   # not again within 30 minutes
        self.assertEqual(looked, [1])

    def test_settings_shows_wear(self):
        q = open(os.path.join(SHELL, 'SetHomeDrives.qml'), encoding='utf-8').read()
        self.assertIn('"Wearing: back it up soon"', q)


if __name__ == '__main__':
    unittest.main()
