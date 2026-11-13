"""Memory and swap (os/system_files/usr/libexec/nimbus/hearth-memory, 1.11): what's recommended per PC, the limits,
Keep memory free as the kernel's watermark, the owner's test files taken over exactly, settings that never change by
themselves, and the memory guard's choice (never the Never close list)."""
import importlib.machinery
import importlib.util
import json
import os
import shutil
import tempfile
import unittest
from unittest import mock

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
TOOL = os.path.join(REPO, 'os/system_files/usr/libexec/nimbus/hearth-memory')
GIB = 1024 ** 3


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_memory', TOOL)
    spec = importlib.util.spec_from_loader('hearth_memory', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hm = load()
LAPTOP = int(7.5 * GIB)          # the owner's i3-N305 laptop
DESKTOP = 32 * GIB


class Recommended(unittest.TestCase):
    def test_zram_scaled_to_each_pc(self):
        self.assertEqual(hm.recommended(LAPTOP)['zram'], {'on': True, 'gb': 11, 'algo': 'zstd'})
        self.assertEqual(hm.recommended(16 * GIB)['zram']['gb'], 16)
        self.assertEqual(hm.recommended(DESKTOP)['zram']['gb'], 16)
        r = hm.recommended(LAPTOP)
        self.assertEqual(r['keep']['mode'], '2gb')
        self.assertEqual(r['guard']['never'], ['desktop', 'steam', 'game', 'aicc'])

    def test_limits(self):
        lim = hm.limits(8 * GIB, 100 * GIB)
        self.assertEqual(lim['zramMaxGb'], 16)                      # up to 2x memory
        self.assertEqual(lim['zramMarkGb'], 12)                     # the green mark at 1.5x
        self.assertEqual(lim['swapfileMaxGb'], 64)                  # never more than 64 GB
        self.assertEqual(hm.limits(8 * GIB, 50 * GIB)['swapfileMaxGb'], 30)   # always 20 GB free on C:
        self.assertEqual(hm.limits(8 * GIB, 10 * GIB)['swapfileMaxGb'], 0)


class KeepFree(unittest.TestCase):
    def test_watermark(self):
        self.assertEqual(hm.watermark_for({'mode': 'off'}, LAPTOP), 10)
        self.assertEqual(hm.watermark_for({'mode': '50%'}, LAPTOP), 2500)
        self.assertEqual(hm.watermark_for({'mode': '2gb'}, 8 * GIB), 1250)
        self.assertEqual(hm.watermark_for({'mode': '2gb'}, DESKTOP), 312)
        self.assertEqual(hm.watermark_for({'mode': 'custom', 'factor': 2300}, LAPTOP), 2300)

    def test_the_owners_2300_is_half_and_kept_exactly(self):
        k = hm.keep_from_factor(2300, LAPTOP)
        self.assertEqual(k, {'mode': '50%', 'factor': 2300})
        vm = hm.wanted_vm({'keep': k, 'swappiness': 200}, LAPTOP, True)
        self.assertEqual(vm['watermark_scale_factor'], 2300)
        self.assertEqual(vm['swappiness'], 200)
        self.assertEqual(vm['page-cluster'], 0)
        self.assertEqual(vm['watermark_boost_factor'], 0)
        self.assertEqual(vm['min_free_kbytes'], 262144 if LAPTOP / 1024 * 0.03 > 262144 else max(67584, int(LAPTOP / 1024 * 0.03)))

    def test_without_zram_swappiness_stays_sensible(self):
        vm = hm.wanted_vm({'swappiness': 200}, LAPTOP, False)
        self.assertEqual(vm['swappiness'], 100)
        self.assertEqual(vm['page-cluster'], 3)


class Validate(unittest.TestCase):
    def test_limits_enforced(self):
        algos = ['zstd', 'lz4', 'lz4hc', 'deflate']
        with self.assertRaises(hm.Fail):
            hm.valid({'zram': {'gb': 17}}, 8 * GIB, 100 * GIB, algos)
        with self.assertRaises(hm.Fail):
            hm.valid({'zram': {'algo': 'lzo-rle'}}, 8 * GIB, 100 * GIB, algos)     # not in this kernel
        with self.assertRaises(hm.Fail):
            hm.valid({'swappiness': 250}, 8 * GIB, 100 * GIB, algos)
        with self.assertRaises(hm.Fail):
            hm.valid({'keep': {'mode': 'lots'}}, 8 * GIB, 100 * GIB, algos)
        self.assertEqual(hm.valid({'zram': {'gb': 16, 'algo': 'lz4'}, 'junk': 1}, 8 * GIB, 100 * GIB, algos),
                         {'zram': {'gb': 16, 'algo': 'lz4'}})
        g = hm.valid({'guard': {'level': 'strong', 'never': ['desktop', 'nope'], 'extra': ['blender', 'rm -rf']}}, 8 * GIB, 1, algos)
        self.assertEqual(g['guard'], {'level': 'strong', 'never': ['desktop'], 'extra': ['blender']})


class Files(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.p = mock.patch.multiple(hm, SETTINGS=os.path.join(self.d, 'etc/hearth/memory.json'),
                                     TEST_SYSCTL=os.path.join(self.d, '99-hearth-memory-test.conf'),
                                     TEST_ZRAM=os.path.join(self.d, 'hearth-zram-test.service'))
        self.p.start()

    def tearDown(self):
        self.p.stop()
        shutil.rmtree(self.d)

    def write_tests(self):
        with open(hm.TEST_SYSCTL, 'w') as f:
            f.write('vm.swappiness = 200\nvm.watermark_scale_factor = 2300\nvm.min_free_kbytes = 262144\nvm.page-cluster = 0\n')
        with open(hm.TEST_ZRAM, 'w') as f:
            f.write('[Service]\nExecStart=/usr/sbin/zramctl /dev/zram0 --size 12G --algorithm zstd\n')

    def test_migration_copies_the_test_files_exactly(self):
        self.write_tests()
        self.assertTrue(hm.migrate_once(LAPTOP))
        s = json.load(open(hm.SETTINGS))
        self.assertEqual(s['zram'], {'on': True, 'gb': 12, 'algo': 'zstd'})
        self.assertEqual(s['keep'], {'mode': '50%', 'factor': 2300})
        self.assertEqual(s['swappiness'], 200)
        self.assertFalse(hm.migrate_once(LAPTOP))            # once: never again over what's saved

    def test_nothing_is_written_without_test_files(self):
        self.assertFalse(hm.migrate_once(LAPTOP))
        self.assertFalse(os.path.exists(hm.SETTINGS))          # recommended values are used without a file

    def test_a_broken_file_is_never_saved_over(self):
        os.makedirs(os.path.dirname(hm.SETTINGS))
        with open(hm.SETTINGS, 'w') as f:
            f.write('{not json')
        with mock.patch.object(hm, 'meminfo', return_value={'MemTotal': LAPTOP}), \
             mock.patch.object(hm, 'free_on', return_value=100 * GIB), \
             mock.patch.object(hm, 'kernel_algos', return_value=['zstd', 'lz4']), \
             mock.patch.object(hm, 'apply', return_value={'ok': True}):
            with self.assertRaises(hm.Fail):
                hm.set_settings({'swappiness': 100})
        self.assertEqual(open(hm.SETTINGS).read(), '{not json')

    def test_set_merges_and_a_new_keep_replaces_the_old(self):
        self.write_tests()
        hm.migrate_once(LAPTOP)
        with mock.patch.object(hm, 'meminfo', return_value={'MemTotal': LAPTOP}), \
             mock.patch.object(hm, 'free_on', return_value=100 * GIB), \
             mock.patch.object(hm, 'kernel_algos', return_value=['zstd', 'lz4']), \
             mock.patch.object(hm, 'apply', return_value={'ok': True}):
            hm.set_settings({'keep': {'mode': '2gb'}, 'zram': {'algo': 'lz4'}})
        s = json.load(open(hm.SETTINGS))
        self.assertEqual(s['keep'], {'mode': '2gb'})            # the old factor doesn't linger
        self.assertEqual(s['zram'], {'on': True, 'gb': 12, 'algo': 'lz4'})
        self.assertEqual(s['swappiness'], 200)                  # untouched
        self.assertNotIn('_from', s)
        self.assertTrue(os.path.exists(hm.SETTINGS + '.bak'))


class Guard(unittest.TestCase):
    def procs(self):
        mb = 1024 * 1024
        return [
            {'pid': 1, 'ppid': 0, 'uid': 0, 'comm': 'systemd', 'rss': 50 * mb, 'swap': 0, 'cgroup': '/init.scope'},
            {'pid': 100, 'ppid': 1, 'uid': 1000, 'comm': 'labwc', 'rss': 300 * mb, 'swap': 0, 'cgroup': '/user.slice/x/session.scope'},
            {'pid': 200, 'ppid': 1, 'uid': 1000, 'comm': 'hearth-ai-command-center', 'rss': 900 * mb, 'swap': 0,
             'cgroup': '/user.slice/user@1000.service/app.slice/app-aicc-1.scope'},
            {'pid': 201, 'ppid': 200, 'uid': 1000, 'comm': 'hearth-ai-comma', 'rss': 2500 * mb, 'swap': 500 * mb,
             'cgroup': '/user.slice/user@1000.service/app.slice/app-aicc-1.scope'},            # its renderer
            {'pid': 300, 'ppid': 1, 'uid': 1000, 'comm': 'firefox', 'rss': 1500 * mb, 'swap': 200 * mb,
             'cgroup': '/user.slice/user@1000.service/app.slice/app-firefox-9.scope'},
            {'pid': 301, 'ppid': 300, 'uid': 1000, 'comm': 'Isolated Web Co', 'rss': 900 * mb, 'swap': 100 * mb,
             'cgroup': '/user.slice/user@1000.service/app.slice/app-firefox-9.scope'},
            {'pid': 400, 'ppid': 1, 'uid': 1000, 'comm': 'steam', 'rss': 600 * mb, 'swap': 0, 'cgroup': '/user.slice/x/app-steam.scope'},
        ]

    def test_never_close_and_whole_apps(self):
        ps = self.procs()
        with mock.patch.object(hm, 'read', side_effect=lambda p, d='': 'hearth-ai-command-center' if p.endswith('/200/cmdline') else ''):
            keep = hm.protected_pids(ps, ['desktop', 'steam', 'game', 'aicc'], [])
        self.assertTrue({1, 100, 200, 201, 400} <= keep)       # root, the desktop, the Command Center and its renderer, Steam
        v = hm.pick_victim(ps, keep)
        self.assertEqual(v['key'], 'app-firefox-9.scope')       # the biggest app that may be closed, all of it
        self.assertEqual(sorted(v['pids']), [300, 301])
        self.assertEqual(v['name'], 'firefox')

    def test_nothing_to_close(self):
        ps = [p for p in self.procs() if p['comm'] not in ('firefox', 'Isolated Web Co')]
        with mock.patch.object(hm, 'read', side_effect=lambda p, d='': 'hearth-ai-command-center' if p.endswith('/200/cmdline') else ''):
            keep = hm.protected_pids(ps, ['desktop', 'steam', 'aicc'], [])
        self.assertIsNone(hm.pick_victim(ps, keep))

    def test_only_when_really_out(self):
        mi = {'MemTotal': 8 * GIB, 'MemAvailable': int(0.01 * 8 * GIB), 'SwapTotal': 48 * GIB, 'SwapFree': GIB}
        self.assertTrue(hm.out_of_memory(mi, {'full': 30.0}, 'gentle'))
        self.assertFalse(hm.out_of_memory(mi, {'full': 5.0}, 'gentle'))         # not stalling
        self.assertFalse(hm.out_of_memory(dict(mi, SwapFree=20 * GIB), {'full': 30.0}, 'gentle'))   # swap has room
        self.assertFalse(hm.out_of_memory(mi, {'full': 99.0}, 'off'))


class Reading(unittest.TestCase):
    def test_in_use_like_windows(self):
        mi = {'MemTotal': 8 * GIB, 'MemFree': GIB, 'Cached': 3 * GIB, 'Buffers': 0, 'SReclaimable': 0, 'Shmem': 0}
        self.assertEqual(hm.in_use(mi), 4 * GIB)

    def test_test_zram_parse(self):
        self.assertEqual(hm.parse_test_zram('zramctl /dev/zram0 --size 12G --algorithm zstd'), {'gb': 12.0, 'algo': 'zstd'})
        self.assertEqual(hm.parse_sysctl_conf('vm.page-cluster = 0\n# x\nvm.swappiness=200'), {'page-cluster': 0, 'swappiness': 200})


if __name__ == '__main__':
    unittest.main()
