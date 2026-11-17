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
STRONG = {'model': 'AMD Ryzen 7 7800X3D 8-Core Processor', 'threads': 16}
N305 = {'model': 'Intel(R) Core(TM) i3-N305', 'threads': 8}
# The recommendation depends on the processor (1.14): these tests pin it to a desktop one unless they say otherwise.
hm.cpu_facts = lambda root=None: dict(STRONG)


class Recommended(unittest.TestCase):
    def test_zram_scaled_to_each_pc(self):
        self.assertEqual(hm.recommended(LAPTOP)['zram'], {'on': True, 'gb': 11, 'algo': 'zstd'})
        self.assertEqual(hm.recommended(16 * GIB)['zram']['gb'], 16)
        self.assertEqual(hm.recommended(DESKTOP)['zram']['gb'], 16)
        r = hm.recommended(LAPTOP)
        self.assertEqual(r['keep']['mode'], 'auto')
        self.assertEqual(r['swappiness'], 180)
        self.assertEqual(hm.watermark_for({'mode': 'auto'}, LAPTOP), 125)
        self.assertEqual(hm.keep_from_factor(125, LAPTOP), {'mode': 'auto'})
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


class KeepRunning(unittest.TestCase):
    """Keep this running (checkpoint), 1.12 (mockup tm-1): frozen and moved to the drive instead of closed."""
    procs = Guard.procs

    def test_saved_and_checked(self):
        self.assertEqual(hm.recommended(LAPTOP)['guard']['checkpoint'], [])
        self.assertEqual(hm.valid({'guard': {'checkpoint': ['firefox', 'bad name;rm']}}, LAPTOP)['guard']['checkpoint'], ['firefox'])

    def test_names_match_the_kernels_15_letters(self):
        v = {'names': ['hearth-ai-comma']}
        self.assertTrue(hm.on_list(v, ['hearth-ai-command-center']))
        self.assertFalse(hm.on_list(v, ['firefox']))

    def test_the_next_biggest_when_it_cant_be_kept(self):
        ps = self.procs()
        with mock.patch.object(hm, 'read', side_effect=lambda p, d='': ''):
            keep = hm.protected_pids(ps, ['desktop', 'steam'], [])
        v = hm.pick_victim(ps, keep)
        self.assertEqual(v['key'], 'app-aicc-1.scope')
        self.assertEqual(hm.pick_victim(ps, keep, {'app-aicc-1.scope'})['key'], 'app-firefox-9.scope')

    def test_frozen_until_resumed_or_closed(self):
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d)
        proc = os.path.join(d, 'proc')
        for pid, state in ((300, 'T'), (301, 'S')):
            os.makedirs(os.path.join(proc, str(pid)))
            with open(os.path.join(proc, str(pid), 'stat'), 'w') as f:
                f.write('%d (firefox) %s 1 0' % (pid, state))
        with mock.patch.object(hm, 'STATE', d):
            hm.frozen_save([{'at': 1, 'name': 'firefox', 'pids': [300, 301, 999], 'mb': 2600}])
            now = hm.frozen_now([{'pid': 300}, {'pid': 301}], proc)
        self.assertEqual(now, [{'at': 1, 'name': 'firefox', 'pids': [300], 'mb': 2600}])   # 301 resumed, 999 closed

    def test_the_guard_freezes_instead_of_closing(self):
        src = open(TOOL).read()
        g = src[src.index('def guard():'):src.index('# ------------------------------------------------------------------ main')]
        self.assertIn("on_list(v, cfg.get('checkpoint'))", g)
        self.assertIn('freeze(v, procs)', g)
        self.assertIn('keep.update(e[\'pids\'])', g)
        f = src[src.index('def freeze('):src.index('class CheckpointSwap')]
        self.assertIn('signal.SIGSTOP', f)
        self.assertNotIn('SIGKILL', f)
        self.assertNotIn('SIGTERM', f)
        self.assertIn("free_on('/var') - KEEP_FREE_ON_C", src[src.index('def room_for('):src.index('def _pageout(')])


class Reading(unittest.TestCase):
    def test_in_use_like_windows(self):
        mi = {'MemTotal': 8 * GIB, 'MemFree': GIB, 'Cached': 3 * GIB, 'Buffers': 0, 'SReclaimable': 0, 'Shmem': 0}
        self.assertEqual(hm.in_use(mi), 4 * GIB)

    def test_test_zram_parse(self):
        self.assertEqual(hm.parse_test_zram('zramctl /dev/zram0 --size 12G --algorithm zstd'), {'gb': 12.0, 'algo': 'zstd'})
        self.assertEqual(hm.parse_sysctl_conf('vm.page-cluster = 0\n# x\nvm.swappiness=200'), {'page-cluster': 0, 'swappiness': 200})


class HandOver(unittest.TestCase):
    """1.14 (the owner, Nov 16): an old /etc/sysctl.d/90-hearth-swap.conf held his laptop at ~0.5 GB free while
    Settings said 2 GB. Swap & settings now says so, and Use these settings hands it over, keeping a .bak."""
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.etc = os.path.join(self.d, 'sysctl.d')
        os.makedirs(self.etc)

    def tearDown(self):
        shutil.rmtree(self.d)

    def put(self, name, text):
        p = os.path.join(self.etc, name)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def test_shown_as_not_in_use(self):
        p = self.put('90-hearth-swap.conf', 'vm.swappiness = 150\nvm.watermark_scale_factor = 200\nvm.page-cluster = 0\n')
        owners = hm.sysctl_owners([self.etc, '/nonexistent'])
        owners['max_map_count'] = '/usr/lib/sysctl.d/60-hearth-games.conf'
        over = hm.managed_overrides(owners, self.etc)
        self.assertEqual(over, {'swappiness': p, 'watermark_scale_factor': p, 'page-cluster': p})

    def test_handed_over_with_a_bak(self):
        p = self.put('90-hearth-swap.conf', 'vm.swappiness = 150\nvm.watermark_scale_factor = 200\nvm.page-cluster = 0\n')
        q = self.put('50-mine.conf', '# mine\nvm.swappiness=10\nnet.core.somaxconn = 4096\n')
        with mock.patch.object(hm, 'apply', return_value={'ok': True, 'applied': {}, 'notes': []}) as ap:
            r = hm.handover(self.etc)
        ap.assert_called_once()
        self.assertFalse(os.path.exists(p))
        with open(p + '.bak') as f:
            self.assertIn('vm.swappiness = 150', f.read())
        with open(q) as f:
            left = f.read()
        self.assertNotIn('vm.swappiness', left)
        self.assertIn('net.core.somaxconn = 4096', left)
        with open(q + '.bak') as f:
            self.assertIn('vm.swappiness=10', f.read())
        self.assertEqual(sorted(h['file'] for h in r['handedOver']), sorted([p, q]))
        self.assertEqual(r['keptOtherLines'], [q])

    def test_nothing_else_touched(self):
        p = self.put('70-net.conf', 'net.core.somaxconn = 4096\nvm.max_map_count = 2147483642\n')
        with mock.patch.object(hm, 'apply', return_value={'ok': True, 'applied': {}, 'notes': []}):
            r = hm.handover(self.etc)
        self.assertEqual(r['handedOver'], [])
        self.assertFalse(os.path.exists(p + '.bak'))

    def test_the_page_says_so_and_has_custom_swap(self):
        with open(os.path.join(REPO, 'hosts/linux/desktop/shell/MemorySettings.qml')) as f:
            q = f.read()
        self.assertIn('ms.run(["handover"])', q)
        self.assertIn('Saved, but not all in use', q)
        self.assertIn('["custom", "Custom"]]);', q)
        self.assertIn('title: "Custom size"', q)


if __name__ == '__main__':
    unittest.main()


class OldDefaultsKept(unittest.TestCase):
    """Red flag 9 (start-safety review, Nov 17): 1.14 made Auto the recommended Keep memory free and swappiness 180.
    A memory.json from before that never set them keeps what they were (2 GB, 200); only a PC with no file gets Auto."""

    def test_saved_file_without_keep_or_swappiness_keeps_the_old_values(self):
        mem = 16 * hm.GIB
        old = hm.merged({'zram': {'on': True, 'gb': 8}}, mem)
        self.assertEqual(old['keep'], {'mode': '2gb'})
        self.assertEqual(old['swappiness'], 200)
        self.assertEqual(hm.wanted_vm(old, mem, True)['swappiness'], 200)
        self.assertEqual(hm.wanted_vm(old, mem, True)['watermark_scale_factor'], hm.watermark_for({'mode': '2gb'}, mem))
        new = hm.merged({}, mem)                                  # no file: the new recommendation
        self.assertEqual(new['keep'], {'mode': 'auto'})
        self.assertEqual(new['swappiness'], hm.AUTO_SWAPPINESS)
        mine = hm.merged({'keep': {'mode': 'auto'}, 'swappiness': 150}, mem)   # what you chose stays
        self.assertEqual((mine['keep'], mine['swappiness']), ({'mode': 'auto'}, 150))

    def test_auto_is_a_choice_on_the_page(self):
        import pathlib
        qml = (pathlib.Path(__file__).resolve().parents[1] / 'desktop/shell/MemorySettings.qml').read_text()
        self.assertIn('["auto", "Auto"]', qml)


class LaptopFreeze114(unittest.TestCase):
    """1.14 (the owner's i3-N305 laptop froze, log G2W9-WQTH-FK): lz4 and memory-sized zram recommended on small
    processors (a saved file keeps its zstd), the guard sees zram full with a drive swap file behind it, and a
    browser's biggest tab is closed before the whole browser."""
    MB = 1024 * 1024

    def test_small_processors(self):
        self.assertTrue(hm.weak_cpu(N305))
        for m in ('Intel(R) N100', 'Intel(R) Celeron(R) N4020', 'Intel(R) Pentium(R) Silver N6000', 'AMD Athlon Silver 3050U'):
            self.assertTrue(hm.weak_cpu({'model': m, 'threads': 4}), m)
        self.assertTrue(hm.weak_cpu({'model': 'Some CPU', 'threads': 4}))
        self.assertFalse(hm.weak_cpu(STRONG))
        self.assertFalse(hm.weak_cpu({'model': 'Intel(R) Core(TM) i5-1235U', 'threads': 12}))
        self.assertFalse(hm.weak_cpu({'model': 'AMD Ryzen 5 5600X 6-Core Processor', 'threads': 12}))

    def test_lz4_recommended_on_the_laptop_but_a_saved_file_keeps_zstd(self):
        self.assertEqual(hm.recommended(LAPTOP, N305)['zram'], {'on': True, 'gb': 8, 'algo': 'lz4'})
        self.assertEqual(hm.recommended(LAPTOP, STRONG)['zram'], {'on': True, 'gb': 11, 'algo': 'zstd'})
        with mock.patch.object(hm, 'cpu_facts', lambda root=None: dict(N305)):
            self.assertEqual(hm.merged({}, LAPTOP)['zram']['algo'], 'lz4')                     # no file: the new one
            self.assertEqual(hm.merged({'zram': {'on': True, 'gb': 8}}, LAPTOP)['zram'], {'on': True, 'gb': 8, 'algo': 'zstd'})
            self.assertEqual(hm.merged({'swappiness': 150}, LAPTOP)['zram'], {'on': True, 'gb': 11, 'algo': 'zstd'})
            self.assertEqual(hm.merged({'zram': {'algo': 'lz4'}}, LAPTOP)['zram']['algo'], 'lz4')   # yours stays

    def test_cpu_facts_read_from_cpuinfo(self):
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d)
        with open(os.path.join(d, 'cpuinfo'), 'w') as f:
            f.write(''.join('processor\t: %d\nmodel name\t: Intel(R) Core(TM) i3-N305\n\n' % i for i in range(8)))
        self.assertEqual(LaptopFreeze114.real_cpu_facts(d), N305)

    def test_zram_full_with_a_big_drive_swap_file_counts_as_full(self):
        mi = {'MemTotal': 8 * GIB, 'MemAvailable': int(0.01 * 8 * GIB), 'SwapTotal': 45 * GIB, 'SwapFree': 38 * GIB}
        sw = [{'path': '/var/swap/swapfile', 'size': 37 * GIB, 'used': 0, 'prio': 10},
              {'path': '/dev/zram0', 'size': 8 * GIB, 'used': int(7.9 * GIB), 'prio': 100}]
        self.assertFalse(hm.out_of_memory(mi, {'full': 30.0}, 'gentle'))                # before 1.14: never "full"
        self.assertTrue(hm.out_of_memory(mi, {'full': 30.0}, 'gentle', sw))
        self.assertFalse(hm.out_of_memory(mi, {'full': 5.0}, 'gentle', sw))             # not stalling: left alone
        self.assertFalse(hm.out_of_memory(dict(mi, MemAvailable=2 * GIB), {'full': 30.0}, 'gentle', sw))
        roomy = [dict(sw[0]), dict(sw[1], used=4 * GIB)]
        self.assertFalse(hm.out_of_memory(mi, {'full': 30.0}, 'gentle', roomy))         # zram still has room
        only_zram = [dict(sw[1], used=4 * GIB)]
        self.assertFalse(hm.out_of_memory(dict(mi, SwapTotal=8 * GIB, SwapFree=4 * GIB), {'full': 30.0}, 'gentle', only_zram))

    def procs(self):
        mb = self.MB
        return [
            {'pid': 300, 'ppid': 1, 'uid': 1000, 'comm': 'firefox-bin', 'rss': 180 * mb, 'swap': 300 * mb, 'cgroup': '/a/app-firefox-9.scope'},
            {'pid': 301, 'ppid': 300, 'uid': 1000, 'comm': 'Isolated Web Co', 'rss': 1415 * mb, 'swap': 0, 'cgroup': '/a/app-firefox-9.scope'},
            {'pid': 302, 'ppid': 300, 'uid': 1000, 'comm': 'Isolated Web Co', 'rss': 57 * mb, 'swap': 0, 'cgroup': '/a/app-firefox-9.scope'},
            {'pid': 303, 'ppid': 300, 'uid': 1000, 'comm': 'WebExtensions', 'rss': 66 * mb, 'swap': 0, 'cgroup': '/a/app-firefox-9.scope'},
            {'pid': 500, 'ppid': 1, 'uid': 1000, 'comm': 'brave', 'rss': 200 * mb, 'swap': 0, 'cgroup': '/a/app-brave-2.scope'},
            {'pid': 501, 'ppid': 500, 'uid': 1000, 'comm': 'brave', 'rss': 900 * mb, 'swap': 0, 'cgroup': '/a/app-brave-2.scope'},
            {'pid': 502, 'ppid': 500, 'uid': 1000, 'comm': 'brave', 'rss': 700 * mb, 'swap': 0, 'cgroup': '/a/app-brave-2.scope'},
        ]

    def test_the_biggest_tab_goes_first(self):
        ps = self.procs()
        self.assertEqual(hm.pick_victim(ps, set())['key'], 'app-firefox-9.scope')     # still the biggest app
        ff = hm.pick_victim([p for p in ps if p['comm'] != 'brave'], set())
        self.assertEqual(ff['key'], 'app-firefox-9.scope')
        tab = hm.biggest_tab(ff, ps)
        self.assertEqual((tab['pid'], tab['browser']), (301, 'Firefox'))
        cmd = {501: 'brave\0--type=renderer\0', 502: 'brave\0--type=renderer\0--extension-process\0', 500: 'brave\0'}
        with mock.patch.object(hm, 'read', side_effect=lambda p, d='': cmd.get(int(p.split('/')[-2]), '') if p.endswith('cmdline') else d):
            br = hm.pick_victim([p for p in ps if p['comm'] == 'brave'], set())
            tab = hm.biggest_tab(br, ps)
        self.assertEqual((tab['pid'], tab['browser']), (501, 'Brave'))     # a tab, never the extension process

    def test_small_tabs_mean_the_whole_app_as_before(self):
        ps = [p for p in self.procs() if p['comm'] != 'brave']
        ps[1] = dict(ps[1], rss=200 * self.MB)
        v = hm.pick_victim(ps, set())
        self.assertIsNone(hm.biggest_tab(v, ps))
        app = {'pid': 1, 'ppid': 0, 'uid': 1000, 'comm': 'blender', 'rss': 3000 * self.MB, 'swap': 0, 'cgroup': '/a/app-b.scope'}
        v = hm.pick_victim([app], set())
        self.assertIsNone(hm.biggest_tab(v, [app]))

    def test_the_guard_closes_a_tab_and_says_so(self):
        src = open(TOOL, encoding='utf-8').read()
        self.assertIn('out_of_memory(mi, p, level, swaps())', src)
        self.assertIn("tab = biggest_tab(v, procs) if v else None", src)
        self.assertIn('press Reload on that tab', src)


LaptopFreeze114.real_cpu_facts = staticmethod(load().cpu_facts)
