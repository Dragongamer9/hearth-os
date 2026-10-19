"""Tests for desktop/shell/hearth-tasks (Mantel's Task Manager helper), with fake /proc and /sys folders."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-tasks'
loader = importlib.machinery.SourceFileLoader('hearth_tasks', str(PATH))
spec = importlib.util.spec_from_loader('hearth_tasks', loader)
ht = importlib.util.module_from_spec(spec)
loader.exec_module(ht)

ME = 1000


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


def stat_line(pid, comm, ppid, utime=0, stime=0, start=100):
    # pid (comm) state ppid pgrp session tty tpgid flags minflt cminflt majflt cmajflt utime stime cutime cstime
    # priority nice threads itrealvalue starttime ...
    return '%d (%s) S %d 1 1 0 -1 0 0 0 0 0 %d %d 0 0 20 0 1 0 %d 1000 100\n' % (pid, comm, ppid, utime, stime, start)


def status_text(uid, anon_kb=1000, shmem_kb=0, kthread=False):
    return 'Name:\tx\nUid:\t%d\t%d\t%d\t%d\nRssAnon:\t%d kB\nRssFile:\t5000 kB\nRssShmem:\t%d kB\nKthread:\t%d\n' % (
        uid, uid, uid, uid, anon_kb, shmem_kb, 1 if kthread else 0)


class FakeProc:
    def __init__(self, root):
        self.root = root

    def add(self, pid, comm, ppid, cmd, uid=ME, anon_kb=1000, jiffies=0, cgroup='0::/user.slice\n', kthread=False):
        d = os.path.join(self.root, str(pid))
        write(os.path.join(d, 'stat'), stat_line(pid, comm, ppid, utime=jiffies, start=pid * 10))
        write(os.path.join(d, 'status'), status_text(uid, anon_kb, kthread=kthread))
        write(os.path.join(d, 'cmdline'), '\0'.join(cmd) + ('\0' if cmd else ''))
        write(os.path.join(d, 'cgroup'), cgroup)


def session(root):
    p = FakeProc(root)
    p.add(1, 'systemd', 0, ['/usr/lib/systemd/systemd'], uid=0)
    p.add(2, 'kthreadd', 0, [], uid=0, kthread=True)
    p.add(3, 'kworker/0:1', 2, [], uid=0)
    p.add(10, 'sh', 1, ['/bin/sh', '/usr/bin/nimbus-session'])
    p.add(11, 'labwc', 10, ['labwc', '-s', '/usr/share/nimbus/desktop/labwc/autostart'])
    p.add(12, 'quickshell', 11, ['quickshell', '-p', '/usr/share/nimbus/desktop/shell'], anon_kb=200000)
    p.add(13, 'python3', 12, ['python3', '/usr/share/nimbus/desktop/shell/hearth-pad'])
    p.add(20, 'firefox', 11, ['/usr/lib64/firefox/firefox'], anon_kb=500000, jiffies=100)
    p.add(21, 'Isolated Web Co', 20, ['/usr/lib64/firefox/firefox', '-contentproc'], anon_kb=300000, jiffies=50)
    p.add(30, 'steam', 1, ['/home/sam/.local/share/Steam/ubuntu12_32/steam', '-silent'], anon_kb=100000)
    p.add(31, 'steamwebhelper', 30, ['./steamwebhelper'], anon_kb=200000)
    p.add(32, 'reaper', 30, ['/home/sam/.local/share/Steam/ubuntu12_32/reaper', 'SteamLaunch', 'AppId=620', '--', 'proton'])
    p.add(33, 'portal2.exe', 32, ['Z:\\games\\Portal 2\\portal2.exe'], anon_kb=2000000, jiffies=400)
    p.add(40, 'nimbus', 11, ['/usr/lib/nimbus/nimbus', '--browser'], anon_kb=1000)
    p.add(41, 'nimbus', 11, ['/usr/lib/nimbus/nimbus'], anon_kb=1000)
    p.add(50, 'bash', 1, ['bash'], uid=1001)                 # someone else's
    p.add(60, 'flatpak-bwrap', 11, ['bwrap', '--args', '40', 'foo'], cgroup='0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-org.foo.Bar-1234.scope\n')
    p.add(70, 'kdeconnectd', 1, ['/usr/libexec/kdeconnectd'])
    return p


class Parsing(unittest.TestCase):
    def test_stat_with_odd_name(self):
        s = ht.parse_stat(stat_line(42, 'Web Content) (x', 7, utime=5, stime=6, start=999))
        self.assertEqual((s['pid'], s['comm'], s['ppid'], s['jiffies'], s['start']), (42, 'Web Content) (x', 7, 11, 999))

    def test_status(self):
        s = ht.parse_status(status_text(1000, anon_kb=10, shmem_kb=2))
        self.assertEqual(s, {'uid': 1000, 'mem': 12 * 1024, 'kthread': False})

    def test_cpu_stat_and_busy(self):
        a = ht.parse_cpu_stat('cpu  100 0 100 700 100 0 0 0 0 0\ncpu0 50 0 50 350 50 0 0 0 0 0\nintr 1 2\n')
        b = ht.parse_cpu_stat('cpu  200 0 200 800 100 0 0 0 0 0\ncpu0 100 0 100 400 50 0 0 0 0 0\n')
        self.assertEqual(a[0], ('cpu', 1000, 800))
        self.assertEqual(len(a), 2)
        self.assertAlmostEqual(ht.busy_percent(a[0][1:], b[0][1:]), 200 / 300 * 100)
        self.assertEqual(ht.busy_percent((10, 5), (10, 5)), 0.0)

    def test_meminfo(self):
        m = ht.parse_meminfo('MemTotal:       16000 kB\nMemAvailable:    6000 kB\nHugePages_Total:       0\n')
        self.assertEqual(m['MemTotal'], 16000 * 1024)
        self.assertEqual(m['HugePages_Total'], 0)

    def test_cgroup_app(self):
        self.assertEqual(ht.parse_cgroup_app('0::/user.slice/app.slice/app-flatpak-com.valvesoftware.Steam-5555.scope'),
                         ('flatpak', 'com.valvesoftware.Steam'))
        self.assertEqual(ht.parse_cgroup_app('0::/user.slice/app.slice/app-gnome-org.gnome.Nautilus-77.scope'),
                         ('unit', 'org.gnome.Nautilus'))
        self.assertEqual(ht.parse_cgroup_app('0::/user.slice/session-2.scope'), (None, None))
        self.assertEqual(ht.parse_cgroup_app(None), (None, None))

    def test_exec_names(self):
        self.assertEqual(ht.exec_names('env FOO=1 /usr/bin/firefox %u'), (['firefox'], None))
        self.assertEqual(ht.exec_names('/usr/bin/flatpak run --branch=stable --arch=x86_64 org.foo.Bar @@u %U @@'), ([], 'org.foo.Bar'))
        self.assertEqual(ht.exec_names('python3 /opt/tool/tool.py'), (['python3', 'tool.py'], None))
        self.assertEqual(ht.exec_names(''), ([], None))

    def test_desktop_entry(self):
        d = ht.parse_desktop('[Desktop Entry]\nName=Files\nName[de]=Dateien\nIcon=folder\n\n[Desktop Action new]\nName=New\n')
        self.assertEqual(d, {'Name': 'Files', 'Icon': 'folder'})

    def test_script_names(self):
        p = ht.Proc(pid=5, ppid=1, comm='sh', exe='dash', cmd=['/bin/sh', '-c', 'exec FOO=1 /usr/bin/thing --x'])
        self.assertEqual(ht.script_name(p), 'thing')
        p = ht.Proc(pid=5, ppid=1, comm='python3', exe='python3.13', cmd=['python3', '-u', '/x/hearth-pad'])
        self.assertEqual(ht.script_name(p), 'hearth-pad')


class Groups(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.proc = os.path.join(self.tmp.name, 'proc')
        session(self.proc)
        apps = os.path.join(self.tmp.name, 'applications')
        write(os.path.join(apps, 'firefox.desktop'), '[Desktop Entry]\nType=Application\nName=Firefox\nIcon=firefox\nExec=firefox %u\n')
        write(os.path.join(apps, 'steam.desktop'), '[Desktop Entry]\nType=Application\nName=Steam\nIcon=steam\nExec=/usr/bin/steam %U\n')
        write(os.path.join(apps, 'org.foo.Bar.desktop'), '[Desktop Entry]\nType=Application\nName=Foo Bar\nIcon=org.foo.Bar\nExec=/usr/bin/flatpak run org.foo.Bar\n')
        self.index = ht.desktop_index([apps])
        self.procs = ht.scan(self.proc)
        self.groups = {g['pid']: g for g in ht.build_groups(self.procs, ME, None, self.index, lambda a: {'620': 'Portal 2'}.get(a))}

    def tearDown(self):
        self.tmp.cleanup()

    def test_scan_skips_nothing_readable(self):
        self.assertIn(33, self.procs)
        self.assertTrue(self.procs[3].kthread)
        self.assertEqual(self.procs[50].uid, 1001)

    def test_apps_grouped_with_what_they_started(self):
        ff = self.groups[20]
        self.assertEqual((ff['name'], ff['icon'], ff['kind'], ff['pids']), ('Firefox', 'firefox', 'app', [20, 21]))
        self.assertEqual(ff['mem'], 800000 * 1024)
        self.assertIn('firefox', ff['appIds'])
        self.assertNotIn(21, self.groups)

    def test_steam_game_is_its_own_group(self):
        self.assertEqual(self.groups[30]['pids'], [30, 31])
        self.assertEqual(self.groups[30]['name'], 'Steam')
        g = self.groups[32]
        self.assertEqual((g['name'], g['kind'], g['pids'], g['icon']), ('Portal 2', 'game', [32, 33], 'steam_icon_620'))

    def test_flatpak_found_by_cgroup(self):
        self.assertEqual(self.groups[60]['name'], 'Foo Bar')

    def test_background_process(self):
        self.assertEqual((self.groups[70]['kind'], self.groups[70]['protected']), ('background', False))

    def test_protected_session_parts(self):
        for pid in (10, 11, 12, 13, 41):
            self.assertTrue(self.groups[pid]['protected'], pid)
            self.assertEqual(self.groups[pid]['pids'], [pid])
        self.assertEqual(self.groups[13]['name'], 'hearth-pad')
        self.assertEqual(self.groups[10]['why'], 'Hearth session')
        self.assertFalse(self.groups[40]['protected'])          # Hearth Browser is a normal app

    def test_only_my_processes(self):
        for pid in (1, 2, 3, 50):
            self.assertNotIn(pid, self.groups)

    def test_self_and_ancestors_protected(self):
        groups = {g['pid']: g for g in ht.build_groups(self.procs, ME, 21, self.index)}
        self.assertTrue(groups[20]['protected'])
        self.assertTrue(groups[21]['protected'])

    def test_cpu_from_two_samples(self):
        groups = list(self.groups.values())
        prev = {self.procs[20].key: 40, self.procs[21].key: 50, self.procs[33].key: 0}
        ht.add_cpu(groups, self.procs, prev, 2000, 1000)
        by = {g['pid']: g for g in groups}
        self.assertEqual(by[20]['cpu'], 6.0)        # (100-40) + (50-50) of 1000
        self.assertEqual(by[32]['cpu'], 40.0)
        self.assertEqual(by[30]['cpu'], 0.0)        # new since the last sample

    def test_refuses_protected_other_users_and_gone(self):
        uid = os.getuid()
        p = FakeProc(os.path.join(self.tmp.name, 'mine'))
        p.add(1, 'systemd', 0, ['systemd'], uid=0)
        p.add(911, 'labwc', 1, ['labwc'], uid=uid)
        p.add(912, 'bash', 1, ['bash'], uid=uid + 1)
        root = os.path.join(self.tmp.name, 'mine')
        with self.assertRaisesRegex(ht.Fail, 'keeps Hearth running'):
            ht.signal_app(911, 15, root)
        with self.assertRaisesRegex(ht.Fail, 'another user'):
            ht.signal_app(912, 15, root)
        with self.assertRaisesRegex(ht.Fail, 'already closed'):
            ht.signal_app(913, 15, root)
        with self.assertRaises(ht.Fail):
            ht.parse_pid('1')

    def test_hearth_running(self):
        uid = os.getuid()
        procs = {41: ht.Proc(pid=41, ppid=1, uid=uid, comm='nimbus', exe='nimbus', cmd=['/usr/lib/nimbus/nimbus'], kthread=False)}
        self.assertTrue(ht.hearth_running(procs))
        procs[41].cmd = ['/usr/lib/nimbus/nimbus', '--browser']
        self.assertFalse(ht.hearth_running(procs))


class Hardware(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.sys = self.tmp.name

    def tearDown(self):
        self.tmp.cleanup()

    def test_nothing_there(self):
        self.assertEqual(ht.sensors(self.sys), ([], []))
        self.assertIsNone(ht.battery(self.sys))
        self.assertEqual(ht.sysfs_gpus(self.sys), [])

    def test_amd_gpu(self):
        dev = os.path.join(self.sys, 'class/drm/card1/device')
        write(os.path.join(dev, 'vendor'), '0x1002\n')
        write(os.path.join(dev, 'gpu_busy_percent'), '37\n')
        write(os.path.join(dev, 'mem_info_vram_used'), '1073741824\n')
        write(os.path.join(dev, 'mem_info_vram_total'), '4294967296\n')
        write(os.path.join(dev, 'pp_dpm_sclk'), '0: 800Mhz\n1: 1600Mhz *\n2: 2700Mhz\n')
        write(os.path.join(self.sys, 'class/drm/card1-eDP-1/status'), 'connected\n')
        g = ht.sysfs_gpus(self.sys)
        self.assertEqual(len(g), 1)
        self.assertEqual((g[0]['vendor'], g[0]['busy'], g[0]['vramUsed'], g[0]['vramTotal'], g[0]['mhz']), ('AMD', 37, 1 << 30, 4 << 30, 1600))
        self.assertIsNone(g[0]['gttUsed'])

    def test_nvidia_smi(self):
        g = ht.parse_nvidia_smi('NVIDIA GeForce RTX 4070, 12, 1024, 12282, 45\nbad line\n')
        self.assertEqual(len(g), 1)
        self.assertEqual((g[0]['name'], g[0]['busy'], g[0]['vramUsed'], g[0]['temp']), ('NVIDIA GeForce RTX 4070', 12.0, 1024 * 1048576, 45.0))
        self.assertIsNone(ht.parse_nvidia_smi('X, [N/A], [N/A], [N/A], [N/A]\n')[0]['busy'])

    def test_sensors(self):
        h0 = os.path.join(self.sys, 'class/hwmon/hwmon0')
        write(os.path.join(h0, 'name'), 'k10temp\n')
        write(os.path.join(h0, 'temp1_input'), '65250\n')
        write(os.path.join(h0, 'temp1_label'), 'Tctl\n')
        h1 = os.path.join(self.sys, 'class/hwmon/hwmon1')
        write(os.path.join(h1, 'name'), 'amdgpu\n')
        write(os.path.join(h1, 'temp1_input'), '58000\n')
        write(os.path.join(h1, 'temp1_label'), 'edge\n')
        h2 = os.path.join(self.sys, 'class/hwmon/hwmon2')
        write(os.path.join(h2, 'name'), 'nvme\n')
        write(os.path.join(h2, 'temp1_input'), '41850\n')
        write(os.path.join(h2, 'temp1_label'), 'Composite\n')
        write(os.path.join(h2, 'temp2_input'), '-273150\n')       # a missing sensor
        h3 = os.path.join(self.sys, 'class/hwmon/hwmon10')
        write(os.path.join(h3, 'name'), 'asus\n')
        write(os.path.join(h3, 'fan1_input'), '3200\n')
        write(os.path.join(h3, 'fan1_label'), 'cpu_fan\n')
        temps, fans = ht.sensors(self.sys)
        self.assertEqual(ht.summary_temp(temps, 'cpu', ('tctl',)), 65.2)
        self.assertEqual(ht.summary_temp(temps, 'gpu', ('edge',)), 58.0)
        self.assertEqual(ht.summary_temp(temps, 'nvme', ('composite',)), 41.9)
        self.assertIsNone(ht.summary_temp(temps, 'other', ()))
        self.assertEqual(len(temps), 3)
        self.assertEqual(fans, [{'label': 'cpu_fan', 'rpm': 3200}])

    def test_battery_discharging(self):
        b = os.path.join(self.sys, 'class/power_supply/BAT0')
        write(os.path.join(b, 'type'), 'Battery\n')
        write(os.path.join(b, 'capacity'), '76\n')
        write(os.path.join(b, 'status'), 'Discharging\n')
        write(os.path.join(b, 'power_now'), '15000000\n')
        write(os.path.join(b, 'energy_now'), '30000000\n')
        ac = os.path.join(self.sys, 'class/power_supply/ACAD')
        write(os.path.join(ac, 'type'), 'Mains\n')
        write(os.path.join(ac, 'online'), '0\n')
        pad = os.path.join(self.sys, 'class/power_supply/ps-controller-battery')
        write(os.path.join(pad, 'type'), 'Battery\n')
        write(os.path.join(pad, 'scope'), 'Device\n')
        write(os.path.join(pad, 'capacity'), '10\n')
        self.assertEqual(ht.battery(self.sys), {'name': 'BAT0', 'percent': 76, 'status': 'Discharging', 'watts': 15.0, 'hours': 2.0, 'plugged': False})

    def test_battery_from_current_and_charge(self):
        b = os.path.join(self.sys, 'class/power_supply/BAT1')
        write(os.path.join(b, 'type'), 'Battery\n')
        write(os.path.join(b, 'capacity'), '50\n')
        write(os.path.join(b, 'status'), 'Charging\n')
        write(os.path.join(b, 'current_now'), '2000000\n')        # 2 A
        write(os.path.join(b, 'voltage_now'), '10000000\n')       # 10 V
        write(os.path.join(b, 'charge_now'), '2000000\n')         # 2 Ah
        write(os.path.join(b, 'charge_full'), '4000000\n')
        r = ht.battery(self.sys)
        self.assertEqual((r['watts'], r['hours'], r['plugged']), (20.0, 1.0, True))

    def test_perf_sample(self):
        proc = os.path.join(self.tmp.name, 'proc')
        write(os.path.join(proc, 'stat'), 'cpu  100 0 100 700 100 0 0 0 0 0\ncpu0 50 0 50 350 50 0 0 0 0 0\ncpu1 50 0 50 350 50 0 0 0 0 0\n')
        write(os.path.join(proc, 'meminfo'), 'MemTotal: 8000 kB\nMemAvailable: 2000 kB\nSwapTotal: 1000 kB\nSwapFree: 250 kB\n')
        write(os.path.join(proc, 'cpuinfo'), 'model name\t: AMD Ryzen Z1 Extreme\ncpu MHz\t\t: 3000.0\n')
        write(os.path.join(proc, 'uptime'), '123.45 100.0\n')
        old = os.environ.get('XDG_RUNTIME_DIR')
        os.environ['XDG_RUNTIME_DIR'] = self.tmp.name
        try:
            ht.save_state('perf', {'t': ht.time.monotonic() - 1, 'cpus': [['cpu', 500, 400], ['cpu0', 250, 200], ['cpu1', 250, 200]]})
            r = ht.perf(proc, self.sys)
        finally:
            if old is None:
                del os.environ['XDG_RUNTIME_DIR']
            else:
                os.environ['XDG_RUNTIME_DIR'] = old
        self.assertEqual(r['cpu']['total'], 20.0)       # 500 more, 400 more idle
        self.assertEqual(r['cpu']['cores'], [20.0, 20.0])
        self.assertEqual((r['cpu']['name'], r['cpu']['mhz']), ('AMD Ryzen Z1 Extreme', 3000))
        self.assertEqual(r['mem']['used'], 6000 * 1024)
        self.assertEqual(r['swap'], {'total': 1000 * 1024, 'used': 750 * 1024})
        self.assertIsNone(r['battery'])
        self.assertEqual((r['fans'], r['uptime']), ([], 123))
        json.dumps(r)


class Startup(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.user = os.path.join(self.tmp.name, 'config/autostart')
        self.system = os.path.join(self.tmp.name, 'xdg/autostart')

    def tearDown(self):
        self.tmp.cleanup()

    def test_unit_files_and_show(self):
        states = ht.parse_unit_files('pipewire.service enabled enabled\nsyncthing.service enabled disabled\nfoo@.service disabled -\nbar.socket enabled -\n')
        self.assertEqual(states, {'pipewire.service': 'enabled', 'syncthing.service': 'enabled', 'foo@.service': 'disabled'})
        info = ht.parse_show('Id=syncthing.service\nDescription=Syncthing - Open Source Continuous File Synchronization\nActiveState=active\n\nId=x.service\nDescription=X\nActiveState=inactive\n')
        self.assertEqual(info['syncthing.service']['ActiveState'], 'active')
        self.assertEqual(info['x.service']['Description'], 'X')

    def test_essential_units(self):
        for u in ('pipewire.service', 'xdg-desktop-portal-gtk.service', 'gvfs-daemon.service', 'foo@.service', 'nimbus-watchdog.service',
                  'systemd-tmpfiles-setup.service', 'app-firefox@autostart.service'):
            self.assertTrue(ht.is_essential_unit(u), u)
        for u in ('syncthing.service', 'kdeconnect.service', 'opentabletdriver.service'):
            self.assertFalse(ht.is_essential_unit(u), u)

    def test_set_key(self):
        self.assertEqual(ht.set_key('[Desktop Entry]\nName=A\nHidden=false\n', 'Hidden', 'true'), '[Desktop Entry]\nName=A\nHidden=true\n')
        self.assertEqual(ht.set_key('[Desktop Entry]\nName=A\n\n[Desktop Action x]\nName=B\n', 'Hidden', 'true'),
                         '[Desktop Entry]\nName=A\n\nHidden=true\n[Desktop Action x]\nName=B\n')
        self.assertEqual(ht.set_key('', 'Hidden', 'true'), '[Desktop Entry]\nHidden=true\n')

    def test_autostart_off_and_back_on(self):
        write(os.path.join(self.system, 'tray.desktop'), '[Desktop Entry]\nType=Application\nName=Tray thing\nComment=Shows a tray\nExec=tray\n')
        write(os.path.join(self.user, 'mine.desktop'), '[Desktop Entry]\nType=Application\nName=Mine\nExec=mine\nX-GNOME-Autostart-enabled=false\n')
        items = {i['id']: i for i in ht.autostart_entries(self.user, [self.system])}
        self.assertEqual(items['autostart:tray.desktop']['on'], True)
        self.assertEqual(items['autostart:tray.desktop']['sub'], 'Shows a tray')
        self.assertEqual(items['autostart:mine.desktop']['on'], False)

        ht.set_autostart('tray.desktop', False, self.user, [self.system])
        override = pathlib.Path(self.user, 'tray.desktop').read_text()
        self.assertIn('Hidden=true', override)
        self.assertIn(ht.OVERRIDE_MARK, override)
        self.assertFalse({i['id']: i for i in ht.autostart_entries(self.user, [self.system])}['autostart:tray.desktop']['on'])
        ht.set_autostart('tray.desktop', True, self.user, [self.system])
        self.assertFalse(os.path.exists(os.path.join(self.user, 'tray.desktop')))

        ht.set_autostart('mine.desktop', True, self.user, [self.system])
        self.assertTrue({i['id']: i for i in ht.autostart_entries(self.user, [self.system])}['autostart:mine.desktop']['on'])
        with self.assertRaises(ht.Fail):
            ht.set_autostart('../evil.desktop', False, self.user, [self.system])
        with self.assertRaises(ht.Fail):
            ht.set_autostart('gone.desktop', False, self.user, [self.system])

    def test_steam_background(self):
        lib = os.path.join(self.tmp.name, 'nimbus', 'library.json')
        self.assertTrue(ht.steam_background(lib))                   # Hearth's default
        write(lib, json.dumps({'favorites': ['steam:1'], 'steamBackground': True}))
        ht.set_steam_background(False, lib)
        data = json.loads(pathlib.Path(lib).read_text())
        self.assertEqual(data, {'favorites': ['steam:1'], 'steamBackground': False})
        self.assertFalse(ht.steam_background(lib))
        write(lib, '{broken')
        self.assertTrue(ht.steam_background(lib))
        with self.assertRaises(ht.Fail):
            ht.set_steam_background(True, lib)

    def test_steam_name(self):
        lib = os.path.join(self.tmp.name, 'steamapps')
        write(os.path.join(lib, 'appmanifest_620.acf'), '"AppState"\n{\n\t"appid"\t\t"620"\n\t"name"\t\t"Portal 2"\n}\n')
        self.assertEqual(ht.steam_name('620', [lib]), 'Portal 2')
        self.assertIsNone(ht.steam_name('1', [lib]))


if __name__ == '__main__':
    unittest.main()
