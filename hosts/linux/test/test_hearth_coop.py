"""Tests for os/system_files/usr/libexec/nimbus/hearth-coop (Couch Co-op), with fake /sys, /proc, /dev, library files
and a fake runner: nothing is launched, mounted or downloaded for real."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import signal
import struct
import tarfile
import tempfile
import unittest
import xml.dom.minidom

PATH = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-coop'
loader = importlib.machinery.SourceFileLoader('hearth_coop', str(PATH))
spec = importlib.util.spec_from_loader('hearth_coop', loader)
hc = importlib.util.module_from_spec(spec)
loader.exec_module(hc)


def write(path, text, mode=None):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb' if isinstance(text, bytes) else 'w') as f:
        f.write(text)
    if mode:
        os.chmod(path, mode)


def slurp(path, mode='r'):
    with open(path, mode) as f:
        return f.read()


def elf(bits=64):
    return b'\x7fELF' + bytes([2 if bits == 64 else 1]) + b'\0' * 59


def pe(bits=64, sections=('.text', '.data')):
    """A minimal PE header: enough for pe_info (machine and section names)."""
    off = 0x80
    mz = bytearray(off)
    mz[:2] = b'MZ'
    struct.pack_into('<I', mz, 0x3c, off)
    opt = 0xf0 if bits == 64 else 0xe0
    hdr = b'PE\0\0' + struct.pack('<HHIIIHH', 0x8664 if bits == 64 else 0x14c, len(sections), 0, 0, 0, opt, 0x22)
    secs = b''.join(s.encode().ljust(8, b'\0') + b'\0' * 32 for s in sections)
    return bytes(mz) + hdr + b'\0' * opt + secs


# /proc/bus/input/devices for a handheld with InputPlumber, plus a DualSense it doesn't manage.
DEVICES = """I: Bus=0003 Vendor=0b05 Product=1abe Version=0111
N: Name="ASUSTeK Computer Inc. ROG Ally"
P: Phys=usb-0000:08:00.3-2/input0
S: Sysfs=/devices/pci0000:00/usb3/3-2/3-2:1.0/0003:0B05:1ABE.0001/input/input3
U: Uniq=
H: Handlers=event3 js0
B: PROP=0
B: EV=1b
B: KEY=7fff000000000000 0 0 0 0
B: ABS=3003f

I: Bus=0003 Vendor=045e Product=028e Version=0001
N: Name="Microsoft X-Box 360 pad"
P: Phys=
S: Sysfs=/devices/virtual/input/input20
U: Uniq=
H: Handlers=event20 js1
B: PROP=0
B: EV=20000b
B: KEY=7cdb000000000000 0 0 0 0
B: ABS=3003f

I: Bus=0006 Vendor=0000 Product=0000 Version=0000
N: Name="InputPlumber Keyboard"
P: Phys=
S: Sysfs=/devices/virtual/input/input21
U: Uniq=
H: Handlers=sysrq kbd event21
B: PROP=0
B: EV=120013
B: KEY=1000000000007 ff9f207ac14057ff febeffdfffefffff fffffffffffffffe
B: MSC=10

I: Bus=0005 Vendor=054c Product=0ce6 Version=8100
N: Name="Sony Interactive Entertainment DualSense Wireless Controller"
P: Phys=aa:bb:cc:dd:ee:ff
S: Sysfs=/devices/virtual/misc/uhid/0005:054C:0CE6.0007/input/input30
U: Uniq=11:22:33:44:55:66
H: Handlers=event30 js2
B: PROP=0
B: EV=20000b
B: KEY=7fdb000000000000 0 0 0 0
B: ABS=3003f

I: Bus=0005 Vendor=054c Product=0ce6 Version=8100
N: Name="Sony Interactive Entertainment DualSense Wireless Controller Motion Sensors"
P: Phys=aa:bb:cc:dd:ee:ff
S: Sysfs=/devices/virtual/misc/uhid/0005:054C:0CE6.0007/input/input31
U: Uniq=11:22:33:44:55:66
H: Handlers=event31
B: PROP=40
B: EV=19
B: ABS=3f

I: Bus=0003 Vendor=046d Product=c52b Version=0111
N: Name="Logitech USB Receiver Mouse"
P: Phys=usb-0000:00:14.0-1/input2
S: Sysfs=/devices/pci0000:00/usb1/1-1/input/input40
U: Uniq=
H: Handlers=mouse0 event40
B: PROP=0
B: EV=17
B: KEY=1f0000 0 0 0 0
B: REL=1943
"""


class FakeRunner:
    def __init__(self, tools=('gamescope', 'labwc', 'bwrap', 'umu-run', 'wlr-randr', 'xrandr'), responses=None):
        self.tools = {t: '/usr/bin/' + t for t in tools}
        self.responses = responses or {}
        self.calls, self.spawned, self.killed = [], [], []
        self.live = set()
        self.next_pid = 1000
        self.on_spawn = None

    def which(self, name):
        return self.tools.get(name)

    def run(self, argv, env=None, timeout=20, cwd=None):
        self.calls.append(list(argv))
        name = os.path.basename(argv[0])
        r = self.responses.get(name)
        if callable(r):
            return r(argv)
        if r is not None:
            return r
        if name == 'gamescope':
            return 0, '', 'Usage: gamescope [options...] -- [command...]\n  --backend   select backend\n'
        return 0, '', ''

    def spawn(self, argv, env=None, cwd=None, log=None):
        self.next_pid += 1
        pid = self.next_pid
        self.spawned.append({'argv': list(argv), 'env': dict(env or {}), 'cwd': cwd, 'pid': pid})
        self.live.add(pid)
        if self.on_spawn:
            self.on_spawn(argv, pid)
        return pid

    def alive(self, pid):
        return bool(pid) and pid in self.live

    def killpg(self, pgid, sig):
        self.killed.append(('pg', pgid, sig))
        if sig == signal.SIGKILL:
            self.live.discard(pgid)
        return True

    def kill(self, pid, sig):
        self.killed.append(('pid', pid, sig))
        return True

    def reap(self):
        pass


class Clock:
    def __init__(self):
        self.t = 1000.0

    def now(self):
        return self.t

    def sleep(self, s):
        self.t += s


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = self.tmp.name
        self.home = os.path.join(self.root, 'home', 'sam')
        self.sys = os.path.join(self.root, 'sys')
        self.proc = os.path.join(self.root, 'proc')
        self.dev = os.path.join(self.root, 'dev')
        self.run_dir = os.path.join(self.root, 'run', 'user', '1000')
        for d in (self.home, self.sys, self.proc, self.dev, self.run_dir):
            os.makedirs(d, exist_ok=True)
        self.runner = FakeRunner()
        self.clock = Clock()
        self.out = io.StringIO()
        self.ctx = hc.Ctx(env={'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'PATH': '/usr/bin',
                               'DISPLAY': ':0', 'GAMESCOPE_WAYLAND_DISPLAY': 'gamescope-0',
                               'WAYLAND_DISPLAY': 'gamescope-0', 'ENABLE_GAMESCOPE_WSI': '1'},
                          sys_root=self.sys, proc_root=self.proc, dev_root=self.dev, runner=self.runner,
                          sleep=self.clock.sleep, now=self.clock.now, out=self.out, self_path='/usr/libexec/nimbus/hearth-coop',
                          run_udev=os.path.join(self.root, 'run', 'udev'))

    def tearDown(self):
        self.tmp.cleanup()

    # fake system pieces
    def handheld(self, inputplumber=True):
        write(os.path.join(self.proc, 'bus', 'input', 'devices'), DEVICES)
        if inputplumber:
            write(os.path.join(self.proc, '812', 'comm'), 'inputplumber\n')
        write(os.path.join(self.proc, '1', 'comm'), 'systemd\n')
        # The Ally's own pad is hidden by InputPlumber (no node); the others are there.
        for n in ('event20', 'js1', 'event21', 'event30', 'js2', 'event31', 'event40'):
            write(os.path.join(self.dev, 'input', n), '')
        if not inputplumber:
            write(os.path.join(self.dev, 'input', 'event3'), '')
            write(os.path.join(self.dev, 'input', 'js0'), '')
        # The DualSense's HID device: hidraw5, shared by its pad and motion sensors.
        hid = os.path.join(self.sys, 'devices/virtual/misc/uhid/0005:054C:0CE6.0007')
        os.makedirs(os.path.join(hid, 'hidraw', 'hidraw5'))
        for i in ('input30', 'input31'):
            os.makedirs(os.path.join(hid, 'input', i))
            os.symlink(hid, os.path.join(hid, 'input', i, 'device'))
        for n in ('dri', 'snd', 'ntsync', 'null', 'input'):
            os.makedirs(os.path.join(self.dev, n), exist_ok=True)

    def library(self, data):
        write(os.path.join(self.home, '.local/share/nimbus/library.json'), json.dumps(data))

    def steam(self):
        steam = os.path.join(self.home, '.local/share/Steam')
        other = os.path.join(self.root, 'mnt', 'games', 'SteamLibrary')
        write(os.path.join(steam, 'steamapps/libraryfolders.vdf'),
              '"libraryfolders" { "0" { "path" "%s" } "1" { "path" "%s" } }' % (steam, other))
        write(os.path.join(steam, 'steamapps/appmanifest_400.acf'),
              '"AppState" { "appid" "400" "name" "Portal Co-op" "installdir" "Portal" "StateFlags" "4" }')
        g = os.path.join(steam, 'steamapps/common/Portal')
        write(os.path.join(g, 'portal.exe'), pe(64) + b'\0' * 1000)
        write(os.path.join(g, 'bin/steam_api64.dll'), pe(64))
        write(os.path.join(g, 'bin/unins000.exe'), pe(32))
        write(os.path.join(g, 'config.cfg'), 'volume 1\n')
        write(os.path.join(steam, 'steamapps/appmanifest_500.acf'),
              '"AppState" { "appid" "500" "name" "Shooty" "installdir" "Shooty" "StateFlags" "4" }')
        s = os.path.join(steam, 'steamapps/common/Shooty')
        write(os.path.join(s, 'Shooty.exe'), pe(64))
        write(os.path.join(s, 'EasyAntiCheat/EasyAntiCheat_x64.dll'), b'x')
        write(os.path.join(steam, 'steamapps/appmanifest_1628350.acf'),
              '"AppState" { "appid" "1628350" "name" "Steam Linux Runtime 3.0" "installdir" "SLR" "StateFlags" "4" }')
        os.makedirs(os.path.join(steam, 'steamapps/common/SLR'))
        write(os.path.join(other, 'steamapps/appmanifest_600.acf'),
              '"AppState" { "appid" "600" "name" "Kart Party" "installdir" "Kart" "StateFlags" "4" }')
        k = os.path.join(other, 'steamapps/common/Kart')
        write(os.path.join(k, 'KartParty.x86_64'), elf(64) + b'\0' * 100, 0o755)
        write(os.path.join(k, 'KartParty_Data/Plugins/libsteam_api.so'), elf(64))
        write(os.path.join(k, 'UnityCrashHandler64'), elf(64), 0o755)
        # Not fully installed: skipped.
        write(os.path.join(steam, 'steamapps/appmanifest_700.acf'),
              '"AppState" { "appid" "700" "name" "Later" "installdir" "Later" "StateFlags" "2" }')
        write(os.path.join(steam, 'compatibilitytools.d/GE-Proton10-4/proton'), '#!/bin/sh\n')
        write(os.path.join(steam, 'compatibilitytools.d/GE-Proton9-20/proton'), '#!/bin/sh\n')
        write(os.path.join(steam, 'steamapps/common/Proton 9.0/proton'), '#!/bin/sh\n')
        os.makedirs(os.path.join(steam, 'userdata', '123'))
        return steam

    def gbe_cache(self):
        d = hc.gbe_dir(self.ctx)
        write(os.path.join(d, 'windows/release/regular/x64/steam_api64.dll'), b'GBE64')
        write(os.path.join(d, 'windows/release/regular/x32/steam_api.dll'), b'GBE32')
        write(os.path.join(d, 'windows/release/experimental/x64/steam_api64.dll'), b'EXP')
        write(os.path.join(d, 'windows/.done'), '{}')
        write(os.path.join(d, 'linux/release/regular/x64/libsteam_api.so'), b'GBESO64')
        write(os.path.join(d, 'linux/release/regular/x32/libsteam_api.so'), b'GBESO32')
        write(os.path.join(d, 'linux/.done'), '{}')

    def lines(self):
        return [l for l in self.out.getvalue().splitlines() if l]

    def result(self):
        return json.loads(self.lines()[-1])


# ---------------------------------------------------------------------- controllers

class Pads(Base):
    def test_bits(self):
        self.assertIn(0x130, hc.parse_bits('7cdb000000000000 0 0 0 0'))
        self.assertEqual(hc.parse_bits('3'), {0, 1})
        self.assertIn(64, hc.parse_bits('1 0'))

    def test_inputplumber_heuristic(self):
        self.handheld()
        r = hc.list_pads(self.ctx)
        self.assertTrue(r['inputplumber'])
        ids = [p['id'] for p in r['pads']]
        self.assertEqual(ids, ['event20', 'event30'])
        vp = r['pads'][0]
        self.assertEqual((vp['kind'], vp['event'], vp['js']), ('inputplumber', '/dev/input/event20', '/dev/input/js1'))
        self.assertIn('InputPlumber', vp['label'])
        ds = r['pads'][1]
        # A Bluetooth LE DualSense goes through uhid (under /devices/virtual) but has a Phys: physical, not managed.
        self.assertEqual(ds['kind'], 'physical')
        self.assertEqual(ds['hidraw'], ['/dev/hidraw5'])
        self.assertEqual(ds['extra'], ['/dev/input/event31'])
        self.assertEqual(ds['nodes'], ['/dev/input/event30', '/dev/input/js2'])
        # The Ally's own pad is managed (hidden) by InputPlumber.
        self.assertEqual([(p['id'], p.get('managedBy')) for p in r['hidden']], [('event3', 'inputplumber')])
        self.assertEqual([k['id'] for k in r['keyboards']], ['event21'])
        self.assertEqual([m['id'] for m in r['mice']], ['event40'])

    def test_without_inputplumber(self):
        self.handheld(inputplumber=False)
        r = hc.list_pads(self.ctx)
        self.assertFalse(r['inputplumber'])
        kinds = {p['id']: p['kind'] for p in r['pads']}
        self.assertEqual(kinds, {'event3': 'physical', 'event20': 'virtual', 'event30': 'physical'})
        self.assertEqual(r['hidden'], [])

    def test_unreadable_physical_without_inputplumber_is_hidden_with_reason(self):
        self.handheld(inputplumber=False)
        os.unlink(os.path.join(self.dev, 'input', 'event3'))
        r = hc.list_pads(self.ctx)
        self.assertEqual([p['id'] for p in r['hidden']], ['event3'])
        self.assertIn("can't read", r['hidden'][0]['why'])

    def test_inputplumber_sources_folder_marks_managed(self):
        self.handheld(inputplumber=False)
        write(os.path.join(self.proc, '812', 'comm'), 'inputplumber\n')
        write(os.path.join(self.dev, 'inputplumber', 'sources', 'event3'), '')
        r = hc.list_pads(self.ctx)
        self.assertEqual([p['id'] for p in r['hidden']], ['event3'])

    def test_wait_button(self):
        self.handheld()
        ev = hc.EVENT
        # An axis wiggle on the virtual pad, then A down on the DualSense.
        write(os.path.join(self.dev, 'input', 'event20'), ev.pack(0, 0, 3, 0, 100) + ev.pack(0, 0, 0, 0, 0))
        write(os.path.join(self.dev, 'input', 'event30'), ev.pack(0, 0, 1, 0x130, 1))
        import time
        self.ctx.now = time.monotonic      # regular files are always "ready": let real time run out
        p = hc.wait_button(self.ctx, timeout=2)
        self.assertEqual(p['id'], 'event30')
        self.assertIsNone(hc.wait_button(self.ctx, timeout=0.3, exclude=['event30']))

    def test_wait_button_says_why_nothing_to_listen_to(self):
        self.handheld()
        with self.assertRaises(hc.Fail) as cm:
            hc.wait_button(self.ctx, timeout=0.1, exclude=['event20', 'event30'])
        self.assertIn('already joined', str(cm.exception))
        log = slurp(self.ctx.pads_log)
        self.assertIn('(event30, physical): left out, already joined', log)
        self.assertIn('(event3): left out, InputPlumber manages it', log)

    def test_wait_button_logs_what_it_heard(self):
        self.handheld()
        write(os.path.join(self.dev, 'input', 'event30'), hc.EVENT.pack(0, 0, 1, 0x130, 1))
        import time
        self.ctx.now = time.monotonic
        self.assertEqual(hc.wait_button(self.ctx, timeout=2, exclude=['event20'])['id'], 'event30')
        self.assertIn('(event30) pressed a button', slurp(self.ctx.pads_log))

    def test_pads_command(self):
        self.handheld()
        self.assertEqual(hc.main(['pads'], self.ctx), 0)
        self.assertEqual(len(self.result()['pads']), 2)


# ---------------------------------------------------------------------- games

class Games(Base):
    def test_file_kinds(self):
        p = os.path.join(self.root, 'a.dll')
        write(p, pe(32, ('.text', '.bind')))
        self.assertEqual(hc.pe_info(p), (32, ['.text', '.bind']))
        write(p, elf(32))
        self.assertEqual(hc.elf_bits(p), 32)
        self.assertEqual(hc.pe_info(p), (None, []))

    def test_vdf(self):
        kv = hc.vdf_parse('"AppState" { "appid" "1" "UserConfig" { "language" "english" } "name" "A \\"B\\"" }')
        self.assertEqual(kv['AppState']['UserConfig']['language'], 'english')

    def test_steam_scan(self):
        self.steam()
        games = {g['id']: g for g in hc.steam_games(self.home)}
        self.assertEqual(sorted(games), ['steam:400', 'steam:500', 'steam:600'])
        self.assertTrue(games['steam:600']['installDir'].endswith('SteamLibrary/steamapps/common/Kart'))

    def test_analyze_windows_steam_game(self):
        self.steam()
        g = hc.analyze(next(x for x in hc.steam_games(self.home) if x['id'] == 'steam:400'), home=self.home)
        self.assertTrue(g['canPlay'], g['why'])
        self.assertEqual((g['run'], g['launcher']), ('windows', 'umu'))
        self.assertTrue(g['exe'].endswith('Portal/portal.exe') and g['exeGuessed'])
        self.assertEqual(g['steamApi'], ['bin/steam_api64.dll'])
        self.assertTrue(g['needsSteam'] and g['usesGoldberg'])
        self.assertEqual(g['steamAppId'], '400')

    def test_analyze_native_game_picks_game_not_crash_handler(self):
        self.steam()
        g = hc.analyze(next(x for x in hc.steam_games(self.home) if x['id'] == 'steam:600'), home=self.home)
        self.assertEqual(g['run'], 'native')
        self.assertTrue(g['exe'].endswith('KartParty.x86_64'))
        self.assertEqual(g['steamApi'], ['KartParty_Data/Plugins/libsteam_api.so'])

    def test_anti_cheat_refused_plainly(self):
        self.steam()
        g = hc.analyze(next(x for x in hc.steam_games(self.home) if x['id'] == 'steam:500'), home=self.home)
        self.assertFalse(g['canPlay'])
        self.assertEqual(g['antiCheat'], ['EasyAntiCheat'])
        self.assertIn("uses anti-cheat (EasyAntiCheat)", g['why'])

    def test_steamstub_refused(self):
        d = os.path.join(self.root, 'g')
        write(os.path.join(d, 'Game.exe'), pe(64, ('.text', '.bind')))
        g = hc.analyze({'id': 'manual:1', 'title': 'Game', 'source': 'manual', 'installDir': d,
                        'exe': os.path.join(d, 'Game.exe'), 'args': '', 'steamAppId': None}, home=self.home)
        self.assertEqual(g['drm'], 'steamstub')
        self.assertIn('SteamStub', g['why'])

    def test_unreal_shipping_exe_beats_launcher_stub(self):
        d = os.path.join(self.root, 'Rally')
        write(os.path.join(d, 'Rally.exe'), pe(64))
        write(os.path.join(d, 'Rally/Binaries/Win64/Rally-Win64-Shipping.exe'), pe(64) + b'\0' * 5000)
        write(os.path.join(d, 'Engine/Extras/Redist/en-us/UE4PrereqSetup_x64.exe'), pe(64))
        tree = hc.scan_tree(d)
        exe, kind = hc.guess_exe(d, tree, 'Rally')
        self.assertEqual(kind, 'windows')
        self.assertTrue(exe.endswith('Rally-Win64-Shipping.exe'))

    def test_shared_folder_refused(self):
        write(os.path.join(self.home, 'thing.sh'), '#!/bin/sh\n', 0o755)
        g = hc.analyze({'id': 'manual:2', 'title': 'Thing', 'source': 'manual', 'installDir': self.home,
                        'exe': os.path.join(self.home, 'thing.sh'), 'args': '', 'steamAppId': None}, home=self.home)
        self.assertFalse(g['canPlay'])
        self.assertIn('shared folder', g['why'])

    def test_desktop_shortcut_refused(self):
        d = os.path.join(self.root, 'x')
        write(os.path.join(d, 'x.desktop'), '[Desktop Entry]\n')
        g = hc.analyze({'id': 'manual:3', 'title': 'X', 'source': 'manual', 'installDir': d,
                        'exe': os.path.join(d, 'x.desktop'), 'args': '', 'steamAppId': None}, home=self.home)
        self.assertIn('.desktop', g['why'])

    def test_library_sources_and_games_command(self):
        self.steam()
        epic_dir = os.path.join(self.root, 'Games/Epic/Fall')
        write(os.path.join(epic_dir, 'Fall/Binaries/Win64/Fall.exe'), pe(64))
        write(os.path.join(self.home, '.local/share/nimbus/legendary/installed.json'), json.dumps({
            'Fall': {'app_name': 'Fall', 'title': 'Fall Friends', 'install_path': epic_dir, 'executable': 'Fall/Binaries/Win64/Fall.exe',
                     'launch_parameters': '-nolauncher', 'is_dlc': False},
            'FallDLC': {'title': 'DLC', 'install_path': epic_dir, 'is_dlc': True}}))
        gog_dir = os.path.join(self.root, 'Games/GOG/Duo')
        write(os.path.join(gog_dir, 'goggame-77.info'), json.dumps({'name': 'Duo', 'playTasks': [
            {'isPrimary': True, 'type': 'FileTask', 'path': 'bin\\Duo.exe', 'arguments': '-lan', 'workingDir': 'bin'}]}))
        write(os.path.join(gog_dir, 'bin/Duo.exe'), pe(32))
        man_dir = os.path.join(self.root, 'Games/Tanks')
        write(os.path.join(man_dir, 'tanks'), elf(64), 0o755)
        self.library({'gogInstalls': {'77': gog_dir}, 'manual': [{'id': 'manual:9', 'title': 'Tanks', 'path': os.path.join(man_dir, 'tanks'), 'args': '--lan'}]})
        self.assertEqual(hc.main(['games'], self.ctx), 0)
        games = {g['id']: g for g in self.result()['games']}
        self.assertEqual(sorted(games), ['epic:Fall', 'gog:77', 'manual:9', 'steam:400', 'steam:500', 'steam:600'])
        self.assertEqual(games['epic:Fall']['args'], '-nolauncher')
        self.assertEqual(games['gog:77']['run'], 'windows')
        self.assertTrue(games['gog:77']['workDir'].endswith('Duo/bin'))
        self.assertEqual(games['manual:9']['run'], 'native')
        self.assertFalse(games['steam:500']['canPlay'])
        # Playable first, then by title.
        order = [g['id'] for g in self.result()['games']]
        self.assertEqual(order[-1], 'steam:500')
        self.assertTrue(any(l.startswith('progress: ') for l in self.lines()))


# ---------------------------------------------------------------------- layout, screen, profiles

class Layout(Base):
    def test_two_players_side_by_side_on_16_10(self):
        self.assertEqual(hc.layout_rects(1280, 800, 2), [{'x': 0, 'y': 0, 'w': 640, 'h': 800}, {'x': 640, 'y': 0, 'w': 640, 'h': 800}])

    def test_two_players_stacked_on_4_3(self):
        self.assertEqual(hc.layout_rects(1024, 768, 2), [{'x': 0, 'y': 0, 'w': 1024, 'h': 384}, {'x': 0, 'y': 384, 'w': 1024, 'h': 384}])

    def test_forced_layouts(self):
        self.assertEqual(hc.layout_rects(1920, 1080, 2, 'horizontal')[1], {'x': 0, 'y': 540, 'w': 1920, 'h': 540})
        self.assertEqual(hc.layout_rects(1024, 768, 2, 'vertical')[1]['x'], 512)
        self.assertEqual(len(hc.layout_rects(1920, 1080, 2, 'grid')), 2)

    def test_three_and_four_use_grid(self):
        three = hc.layout_rects(1920, 1080, 3)
        self.assertEqual([(r['x'], r['y']) for r in three], [(0, 0), (960, 0), (0, 540)])
        four = hc.layout_rects(1280, 800, 4)
        self.assertTrue(all(r['w'] == 640 and r['h'] == 400 for r in four))

    def test_sizes_are_even(self):
        for r in hc.layout_rects(1366, 767, 4):
            self.assertEqual((r['w'] % 2, r['h'] % 2), (0, 0))

    def test_bad_layout(self):
        with self.assertRaises(hc.Fail):
            hc.layout_rects(1280, 800, 2, 'diagonal')
        with self.assertRaises(hc.Fail):
            hc.layout_rects(1280, 800, 5)

    def test_screen_size(self):
        self.runner.responses['xrandr'] = (0, 'Screen 0: minimum 16 x 16, current 1920 x 1200, maximum 32767 x 32767\n', '')
        self.assertEqual(hc.screen_size(self.ctx), (1920, 1200))
        self.assertEqual(hc.screen_size(self.ctx, 800, 600), (800, 600))
        self.runner.responses['xrandr'] = (1, '', 'no display')
        self.runner.responses['xdpyinfo'] = (0, '  dimensions:    2560x1440 pixels (677x381 millimeters)\n', '')
        self.assertEqual(hc.screen_size(self.ctx), (2560, 1440))
        self.runner.responses['xdpyinfo'] = (1, '', '')
        self.assertEqual(hc.screen_size(self.ctx), (1280, 800))


class Profiles(Base):
    def test_defaults_use_hearth_profile_name(self):
        self.library({'profile': {'name': 'Sam'}})
        self.assertEqual([p['name'] for p in hc.profiles(self.ctx)], ['Sam', 'Player 2', 'Player 3', 'Player 4'])

    def test_set_and_clean(self):
        self.assertEqual(hc.main(['profile-set', '--seat', '2', '--name', ' Alex\n[x]= '], self.ctx), 0)
        self.assertEqual(self.result()['players'][1]['name'], 'Alexx')
        saved = json.loads(slurp(os.path.join(self.home, '.local/share/nimbus/coop/profiles.json')))
        self.assertEqual(saved['players'][1], {'seat': 2, 'name': 'Alexx'})
        self.assertEqual(hc.main(['profiles'], self.ctx), 0)
        self.assertEqual(self.result()['players'][1]['name'], 'Alexx')

    def test_bad_input(self):
        self.assertEqual(hc.main(['profile-set', '--seat', '5', '--name', 'x'], self.ctx), 1)
        self.assertEqual(self.lines()[-1], 'error: Seats are 1 to 4.')
        self.assertEqual(hc.main(['profile-set', '--seat', '1', '--name', '  '], self.ctx), 1)
        self.assertEqual(hc.main(['profile-set', '--seat', 'one'], self.ctx), 1)


# ---------------------------------------------------------------------- Goldberg

class FakeResponse(io.BytesIO):
    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.close()


class Gbe(Base):
    def linux_tar(self):
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode='w:bz2') as t:
            data = b'GBESO64'
            info = tarfile.TarInfo('release/regular/x64/libsteam_api.so')
            info.size = len(data)
            t.addfile(info, io.BytesIO(data))
        return buf.getvalue()

    def test_fetch_linux_and_windows(self):
        urls = []
        tar = self.linux_tar()

        def urlopen(req, timeout=None):
            urls.append(req.full_url)
            return FakeResponse(tar if req.full_url.endswith('.tar.bz2') else b'7zdata')

        def seven(argv):
            out = next(a[2:] for a in argv if a.startswith('-o'))
            write(os.path.join(out, 'release/regular/x64/steam_api64.dll'), b'GBE64')
            return 0, '', ''
        self.ctx.urlopen = urlopen
        self.runner.tools['7z'] = '/usr/bin/7z'
        self.runner.responses['7z'] = seven
        self.assertEqual(hc.main(['gbe-fetch'], self.ctx), 0)
        self.assertEqual(self.result()['cached'], {'linux': True, 'windows': True})
        self.assertEqual(urls, ['https://github.com/Detanup01/gbe_fork/releases/download/%s/emu-linux-release.tar.bz2' % hc.GBE_TAG,
                                'https://github.com/Detanup01/gbe_fork/releases/download/%s/emu-win-release.7z' % hc.GBE_TAG])
        self.assertTrue(hc.gbe_find(self.ctx, 'libsteam_api.so', 64).endswith('regular/x64/libsteam_api.so'))
        self.assertTrue(hc.gbe_find(self.ctx, 'steam_api64.dll', 64))
        # Cached: no second download.
        urls.clear()
        hc.gbe_fetch(self.ctx, ['linux', 'windows'])
        self.assertEqual(urls, [])

    def test_checksum_mismatch(self):
        self.ctx.urlopen = lambda req, timeout=None: FakeResponse(self.linux_tar())
        old = dict(hc.GBE_SHA256)
        hc.GBE_SHA256['linux'] = '0' * 64
        try:
            with self.assertRaises(hc.Fail) as e:
                hc.gbe_fetch(self.ctx, ['linux'])
            self.assertIn("didn't match", str(e.exception))
        finally:
            hc.GBE_SHA256.clear()
            hc.GBE_SHA256.update(old)

    def test_pinned_release_gone_uses_newest(self):
        import urllib.error
        tar = self.linux_tar()
        urls = []

        def urlopen(req, timeout=None):
            urls.append(req.full_url)
            if '/releases/download/%s/' % hc.GBE_TAG in req.full_url:
                raise urllib.error.HTTPError(req.full_url, 404, 'Not Found', {}, None)
            if req.full_url.endswith('/releases/latest'):
                return FakeResponse(json.dumps({'tag_name': 'release-new', 'assets': [
                    {'name': 'emu-linux-release.tar.bz2', 'browser_download_url': 'https://github.com/x/y/releases/download/release-new/emu-linux-release.tar.bz2'},
                    {'name': 'emu-win-release.7z', 'browser_download_url': 'https://github.com/x/y/releases/download/release-new/emu-win-release.7z'}]}).encode())
            return FakeResponse(tar)
        self.ctx.urlopen = urlopen
        self.assertEqual(hc.gbe_fetch(self.ctx, ['linux']), {'linux': True, 'windows': False})
        self.assertTrue(urls[-1].endswith('release-new/emu-linux-release.tar.bz2'), urls)

    def test_network_error_is_plain(self):
        def boom(req, timeout=None):
            raise OSError('Network is unreachable')
        self.ctx.urlopen = boom
        self.assertEqual(hc.main(['gbe-fetch', '--kind', 'linux'], self.ctx), 1)
        self.assertIn("Couldn't download the Steam emulator", self.lines()[-1])

    def test_windows_needs_7zip(self):
        self.ctx.urlopen = lambda req, timeout=None: FakeResponse(b'')
        with self.assertRaises(hc.Fail) as e:
            hc.gbe_fetch(self.ctx, ['windows'])
        self.assertIn('7-Zip', str(e.exception))

    def test_unsafe_tar_rejected(self):
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode='w:bz2') as t:
            info = tarfile.TarInfo('../../evil')
            info.size = 1
            t.addfile(info, io.BytesIO(b'x'))
        self.ctx.urlopen = lambda req, timeout=None: FakeResponse(buf.getvalue())
        with self.assertRaises(hc.Fail):
            hc.gbe_fetch(self.ctx, ['linux'])

    def test_find_prefers_regular_and_arch(self):
        self.gbe_cache()
        self.assertTrue(hc.gbe_find(self.ctx, 'steam_api64.dll', 64).endswith('regular/x64/steam_api64.dll'))
        self.assertTrue(hc.gbe_find(self.ctx, 'steam_api.dll', 32).endswith('regular/x32/steam_api.dll'))
        self.assertTrue(hc.gbe_find(self.ctx, 'libsteam_api.so', 32).endswith('regular/x32/libsteam_api.so'))

    def test_swap_in_view_never_touches_install(self):
        self.steam()
        self.gbe_cache()
        g = hc.analyze(next(x for x in hc.steam_games(self.home) if x['id'] == 'steam:400'), home=self.home)
        view = os.path.join(self.root, 'view')
        hc.build_farm(g['installDir'], view)
        self.assertTrue(os.path.islink(os.path.join(view, 'bin/steam_api64.dll')))
        hc.swap_steam_api(self.ctx, view, g, 2, 'Alex')
        dll = os.path.join(view, 'bin/steam_api64.dll')
        self.assertFalse(os.path.islink(dll))
        self.assertEqual(slurp(dll, 'rb'), b'GBE64')
        self.assertEqual(slurp(os.path.join(g['installDir'], 'bin/steam_api64.dll'), 'rb'), pe(64))
        st = os.path.join(view, 'bin/steam_settings')
        user = slurp(os.path.join(st, 'configs.user.ini'))
        self.assertIn('account_name=Alex', user)
        self.assertIn('account_steamid=%d' % (hc.STEAM_ID64_BASE + hc.SEAT_ACCOUNT_BASE + 2), user)
        self.assertIn('listen_port=47584', slurp(os.path.join(st, 'configs.main.ini')))
        self.assertEqual(slurp(os.path.join(st, 'steam_appid.txt')), '400\n')
        self.assertEqual(slurp(os.path.join(view, 'steam_appid.txt')), '400\n')
        self.assertFalse(os.path.exists(os.path.join(g['installDir'], 'steam_appid.txt')))
        self.assertFalse(os.path.exists(os.path.join(g['installDir'], 'bin/steam_settings')))


# ---------------------------------------------------------------------- start / container / stop

class Session(Base):
    def setUp(self):
        super().setUp()
        self.handheld()
        self.steam_dir = self.steam()
        self.gbe_cache()
        os.makedirs(os.path.join(self.root, 'run', 'udev'))
        os.makedirs(os.path.join(self.home, '.local/share/umu'))

    def dry(self, *extra):
        rc = hc.main(['start', '--game', 'steam:400', '--players', '2', '--pads', 'event20,event30',
                      '--width', '1280', '--height', '800', '--dry-run'] + list(extra), self.ctx)
        self.assertEqual(rc, 0, self.out.getvalue())
        return self.result()['session']

    def test_plan_isolates_each_controller(self):
        s = self.dry()
        p1, p2 = s['players']
        self.assertEqual((p1['pad'], p2['pad']), ('event20', 'event30'))
        a1, a2 = p1['argv'], p2['argv']
        self.assertEqual(a1[:6], ['bwrap', '--dev-bind', '/', '/', '--dev', '/dev'])

        def binds(a):
            return [a[i + 1] for i, x in enumerate(a) if x == '--dev-bind' and a[i + 1].startswith(('/dev/input', '/dev/hidraw'))]
        self.assertEqual(binds(a1), ['/dev/input/event20', '/dev/input/js1'] + ['/dev/input/event21', '/dev/input/event40'])
        self.assertEqual(binds(a2), ['/dev/input/event30', '/dev/input/js2', '/dev/input/event31', '/dev/hidraw5'])
        for d in ('/dev/dri', '/dev/snd', '/dev/ntsync'):
            self.assertIn(d, a1)
        self.assertIn('/run/udev', a1)
        self.assertIn('--unshare-pid', a1)
        # A /tmp of its own (sticky, writable): the nested gamescope's Xwayland refused the real /tmp/.X11-unix,
        # which shows up owned by "nobody" in bwrap's user namespace, and gamescope crashed at once.
        t = a1.index('/tmp')
        self.assertEqual(a1[t - 3:t + 1], ['--perms', '1777', '--tmpfs', '/tmp'])
        self.assertLess(t, a1.index('--unshare-pid'))
        install = os.path.join(self.steam_dir, 'steamapps/common/Portal')
        i = a1.index('--ro-bind')
        self.assertEqual(a1[i + 1:i + 3], [install, install])
        g = a1.index('gamescope')
        self.assertEqual(a1[g:g + 12], ['gamescope', '--backend', 'sdl', '-W', '640', '-H', '800', '-w', '640', '-h', '800', '--'])
        e = a1.index('env', g)
        self.assertIn('WAYLAND_DISPLAY', a1[e:])
        self.assertEqual(a1[-2], '/usr/bin/umu-run')
        self.assertTrue(a1[-1].endswith('portal.exe'))
        env = p1['env']
        self.assertEqual(env['SDL_JOYSTICK_DISABLE_UDEV'], '1')
        self.assertEqual(env['SDL_APP_ID'], 'hearth-coop-p1')
        self.assertTrue(env['PROTONPATH'].endswith('GE-Proton10-4'))
        self.assertEqual(env['GAMEID'], 'umu-400')
        self.assertNotIn('DISPLAY', env)
        self.assertNotIn('GAMESCOPE_WAYLAND_DISPLAY', env)
        self.assertNotEqual(p1['env']['WINEPREFIX'], p2['env']['WINEPREFIX'])
        self.assertTrue(env['HOME'].endswith('coop/players/1/home'))
        self.assertTrue(os.path.islink(os.path.join(env['HOME'], '.local/share/Steam')))
        self.assertTrue(os.path.islink(os.path.join(env['HOME'], '.local/share/umu')))
        # Keyboard player (seat 1) opens last.
        self.assertEqual(s['order'], [2, 1])

    def test_keyboard_and_mouse_seat(self):
        s = hc.start(self.ctx, {'game': 'steam:400', 'players': 2, 'pads': 'event30,kbm', 'width': 1280, 'height': 800, 'dry_run': True})['session']
        p1, p2 = s['players']
        self.assertEqual((p2['pad'], p2['keyboard'], p1['keyboard']), ('kbm', True, False))
        self.assertIn('/dev/input/event40', p2['argv'])
        self.assertNotIn('/dev/input/event40', p1['argv'])

    def test_no_keyboard(self):
        s = self.dry('--keyboard-seat', '0')
        self.assertFalse(any(p['keyboard'] for p in s['players']))

    def test_refusals(self):
        cases = [
            (['--game', 'steam:400', '--players', '2', '--pads', 'event20'], 'Pick one controller for each'),
            (['--game', 'steam:400', '--players', '2', '--pads', 'event20,event20'], 'their own controller'),
            (['--game', 'steam:400', '--players', '2', '--pads', 'event20,event99'], 'no longer connected'),
            (['--game', 'steam:400', '--players', '2', '--pads', 'event20,event3'], 'no longer connected'),
            (['--game', 'steam:500', '--players', '2', '--pads', 'event20,event30'], 'anti-cheat'),
            (['--game', 'steam:999', '--players', '2', '--pads', 'event20,event30'], "can't find that game"),
            (['--game', 'steam:400', '--players', '5', '--pads', 'a,b,c,d,e'], '2 to 4 players'),
            (['--players', '2'], 'Pick a game'),
        ]
        for args, msg in cases:
            self.out.truncate(0)
            self.out.seek(0)
            self.assertEqual(hc.main(['start'] + args, self.ctx), 1, args)
            self.assertTrue(self.lines()[-1].startswith('error: ') and msg in self.lines()[-1], (args, self.lines()[-1]))

    def test_missing_tools(self):
        del self.runner.tools['bwrap']
        self.assertEqual(hc.main(['start', '--game', 'steam:400', '--pads', 'event20,event30'], self.ctx), 1)
        self.assertIn('bubblewrap', self.lines()[-1])
        self.runner.tools['bwrap'] = '/usr/bin/bwrap'
        del self.runner.tools['umu-run']
        self.assertEqual(hc.main(['start', '--game', 'steam:400', '--pads', 'event20,event30'], self.ctx), 1)
        self.assertIn('umu-launcher', self.lines()[-1])

    def test_no_proton_yet(self):
        import shutil
        shutil.rmtree(os.path.join(self.steam_dir, 'compatibilitytools.d'))
        shutil.rmtree(os.path.join(self.steam_dir, 'steamapps/common/Proton 9.0'))
        self.assertEqual(hc.main(['start', '--game', 'steam:400', '--pads', 'event20,event30'], self.ctx), 1)
        self.assertIn('No Proton yet', self.lines()[-1])

    def test_native_game_runs_directly(self):
        s = hc.start(self.ctx, {'game': 'steam:600', 'players': 2, 'pads': 'event20,event30', 'width': 1920, 'height': 1080, 'dry_run': True})['session']
        a = s['players'][0]['argv']
        self.assertTrue(a[-1].endswith('KartParty.x86_64'))
        self.assertNotIn('/usr/bin/umu-run', a)
        self.assertNotIn('WINEPREFIX', s['players'][0]['env'])

    def full_start(self, overlay=True):
        if overlay:
            self.runner.tools['fuse-overlayfs'] = '/usr/bin/fuse-overlayfs'
            self.runner.tools['fusermount3'] = '/usr/bin/fusermount3'

            def mount(argv):
                # Pretend to mount: make the merged dir look like the game.
                hc.build_farm(os.path.join(self.steam_dir, 'steamapps/common/Portal'), argv[-1])
                return 0, '', ''
            self.runner.responses['fuse-overlayfs'] = mount

        def on_spawn(argv, pid):
            if argv[0] == 'labwc':
                # What the container does once its players are up.
                s = hc.read_json(self.ctx.session_file)
                s['state'] = 'running'
                s['containerPid'] = 4242
                s['labwcPid'] = pid
                for i, p in enumerate(s['players']):
                    p['pid'] = 5000 + i
                    self.runner.live.add(5000 + i)
                hc.write_json(self.ctx.session_file, s)
        self.runner.on_spawn = on_spawn
        rc = hc.main(['start', '--game', 'steam:400', '--players', '2', '--pads', 'event20,event30', '--width', '1280', '--height', '800'], self.ctx)
        self.assertEqual(rc, 0, self.out.getvalue())
        return self.result()

    def test_start_opens_labwc_and_reports(self):
        r = self.full_start()
        self.assertTrue(r['started'])
        self.assertEqual([p['seat'] for p in r['players']], [1, 2])
        self.assertEqual(r['players'][0]['view'], 'overlay')
        self.assertEqual(r['containerPid'], 4242)
        labwc = self.runner.spawned[0]
        self.assertEqual(labwc['argv'][:2], ['labwc', '-C'])
        self.assertEqual(labwc['env']['WLR_BACKENDS'], 'x11')
        self.assertNotIn('WAYLAND_DISPLAY', labwc['env'])
        self.assertNotIn('ENABLE_GAMESCOPE_WSI', labwc['env'])
        run_dir = labwc['argv'][2]
        rc = slurp(os.path.join(run_dir, 'rc.xml'))
        xml.dom.minidom.parseString(rc)
        self.assertIn('identifier="hearth-coop-p2"', rc)
        self.assertIn('<action name="MoveTo" x="640" y="0" />', rc)
        auto = slurp(os.path.join(run_dir, 'autostart'))
        self.assertIn('_container --session', auto)
        mounts = [c for c in self.runner.calls if c[0].endswith('fuse-overlayfs')]
        self.assertEqual(len(mounts), 2)
        self.assertIn('lowerdir=%s' % os.path.join(self.steam_dir, 'steamapps/common/Portal'), mounts[0][2])
        self.assertTrue(mounts[0][3].endswith('coop/players/1/games/steam_400/merged'))
        # Goldberg went into the per-player view only.
        merged = mounts[0][3]
        self.assertEqual(slurp(os.path.join(merged, 'bin/steam_api64.dll'), 'rb'), b'GBE64')
        self.assertEqual(slurp(os.path.join(self.steam_dir, 'steamapps/common/Portal/bin/steam_api64.dll'), 'rb'), pe(64))
        progress = [l for l in self.lines() if l.startswith('progress: ')]
        self.assertTrue(progress and progress[-1] == 'progress: 100 Playing')

        # Status, then Stop.
        self.out.truncate(0)
        self.out.seek(0)
        hc.main(['status'], self.ctx)
        st = self.result()
        self.assertTrue(st['running'])
        self.assertEqual([p['alive'] for p in st['players']], [True, True])
        self.out.truncate(0)
        self.out.seek(0)
        hc.main(['stop'], self.ctx)
        self.assertEqual(self.result(), {'stopped': True, 'forced': 2})
        self.assertIn(('pg', 5000, signal.SIGTERM), self.runner.killed)
        self.assertIn(('pg', 5001, signal.SIGKILL), self.runner.killed)
        self.assertIn(('pid', 4242, signal.SIGTERM), self.runner.killed)
        unmounts = [c for c in self.runner.calls if c[0].endswith('fusermount3')]
        self.assertEqual(len([u for u in unmounts if u[-1].endswith('merged')]), 2 + 2)   # before mounting + at stop
        self.assertFalse(os.path.exists(self.ctx.session_file))

    def test_stop_waits_for_clean_exit(self):
        self.full_start(overlay=False)
        real_sleep = self.clock.sleep

        def sleep(s):
            real_sleep(s)
            self.runner.live.discard(5000)
            self.runner.live.discard(5001)
        self.ctx.sleep = sleep
        self.out.truncate(0)
        self.out.seek(0)
        hc.main(['stop'], self.ctx)
        self.assertEqual(self.result()['forced'], 0)
        self.assertNotIn(('pg', 5000, signal.SIGKILL), self.runner.killed)

    def test_already_running(self):
        self.full_start(overlay=False)
        self.out.truncate(0)
        self.out.seek(0)
        self.assertEqual(hc.main(['start', '--game', 'steam:400', '--pads', 'event20,event30'], self.ctx), 1)
        self.assertIn('already running', self.lines()[-1])

    def test_status_cleans_up_a_dead_session(self):
        self.full_start(overlay=False)
        self.runner.live.clear()
        self.out.truncate(0)
        self.out.seek(0)
        hc.main(['status'], self.ctx)
        self.assertEqual(self.result()['running'], False)
        self.assertFalse(os.path.exists(self.ctx.session_file))

    def test_labwc_dies_early(self):
        def on_spawn(argv, pid):
            self.runner.live.discard(pid)
        self.runner.on_spawn = on_spawn
        rc = hc.main(['start', '--game', 'steam:400', '--pads', 'event20,event30', '--width', '1280', '--height', '800'], self.ctx)
        self.assertEqual(rc, 1)
        self.assertIn("split screen didn't open", self.lines()[-1])
        self.assertFalse(os.path.exists(self.ctx.session_file))

    def test_symlink_view_without_fuse_overlayfs(self):
        r = self.full_start(overlay=False)
        self.assertEqual(r['players'][0]['view'], 'symlinks')
        view = os.path.join(self.home, '.local/share/nimbus/coop/players/1/games/steam_400/view')
        self.assertTrue(os.path.islink(os.path.join(view, 'portal.exe')))
        self.assertEqual(slurp(os.path.join(view, 'bin/steam_api64.dll'), 'rb'), b'GBE64')

    def test_native_symlink_view_copies_the_exe(self):
        self.runner.on_spawn = lambda argv, pid: self.runner.live.discard(pid)
        hc.main(['start', '--game', 'steam:600', '--pads', 'event20,event30', '--width', '1280', '--height', '800'], self.ctx)
        view = os.path.join(self.home, '.local/share/nimbus/coop/players/2/games/steam_600/view')
        self.assertFalse(os.path.islink(os.path.join(view, 'KartParty.x86_64')))
        self.assertTrue(os.path.islink(os.path.join(view, 'UnityCrashHandler64')))
        self.assertEqual(slurp(os.path.join(view, 'KartParty_Data/Plugins/libsteam_api.so'), 'rb'), b'GBESO64')

    def test_container_launches_in_order_and_tidies_up(self):
        s = self.dry()
        run_dir = os.path.join(self.run_dir, 'hearth-coop')
        os.makedirs(run_dir)
        s['runDir'] = run_dir
        s['labwcPid'] = 777
        s['players'][0]['view'] = {'kind': 'overlay', 'path': '/x/merged', 'mounted': True}
        hc.save_session(self.ctx, s)
        self.runner.tools['fusermount3'] = '/usr/bin/fusermount3'
        self.runner.responses['wlr-randr'] = lambda argv: (0, 'X11-1 "X11 output 1"\n  Enabled: yes\n', '') if len(argv) == 1 else (0, '', '')
        inner = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'WAYLAND_DISPLAY': 'wayland-1', 'LABWC_PID': '777'}
        ctx = hc.Ctx(env=inner, sys_root=self.sys, proc_root=self.proc, dev_root=self.dev, runner=self.runner,
                     sleep=self.clock.sleep, now=self.clock.now, out=self.out)
        launched = []

        def on_spawn(argv, pid):
            launched.append(pid)
            # Both players quit as soon as the second one is up.
            if len(launched) == 2:
                self.runner.live.difference_update(launched)
        self.runner.on_spawn = on_spawn
        self.assertEqual(hc.container(ctx, self.ctx.session_file, poll=0.1), 0)
        seats = [sp['env']['HEARTH_COOP_SEAT'] for sp in self.runner.spawned]
        self.assertEqual(seats, ['2', '1'])
        self.assertEqual(self.runner.spawned[0]['env']['WAYLAND_DISPLAY'], 'wayland-1')
        self.assertIn(['wlr-randr', '--output', 'X11-1', '--custom-mode', '1280x800'], self.runner.calls)
        self.assertIn(('pid', 777, signal.SIGHUP), self.runner.killed)
        self.assertIn(('pid', 777, signal.SIGTERM), self.runner.killed)
        self.assertIn(['/usr/bin/fusermount3', '-u', '-z', '/x/merged'], self.runner.calls)
        self.assertFalse(os.path.exists(self.ctx.session_file))
        rc = slurp(os.path.join(run_dir, 'rc.xml'))
        self.assertIn('identifier="gamescope*"', rc)      # the fallback rule for the last launch (seat 1)
        self.assertIn('<action name="MoveTo" x="0" y="0" />', rc.split('gamescope*')[1])


    def test_container_reports_players_that_close_at_once(self):
        s = self.dry()
        run_dir = os.path.join(self.run_dir, 'hearth-coop')
        os.makedirs(run_dir)
        s['runDir'] = run_dir
        s['labwcPid'] = 777
        for p in s['players']:
            write(p['log'], '[gamescope] Info  wlserver: Starting Xwayland on :1\n'
                            '00:00:00.100 [wlr] [xwayland/sockets.c:63] /tmp/.X11-unix not owned by root or us\n')
        hc.save_session(self.ctx, s)
        inner = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'WAYLAND_DISPLAY': 'wayland-1', 'LABWC_PID': '777'}
        ctx = hc.Ctx(env=inner, sys_root=self.sys, proc_root=self.proc, dev_root=self.dev, runner=self.runner,
                     sleep=self.clock.sleep, now=self.clock.now, out=self.out)
        self.runner.on_spawn = lambda argv, pid: self.runner.live.discard(pid)    # gamescope crashes straight away
        self.assertEqual(hc.container(ctx, self.ctx.session_file, poll=0.1), 0)
        last = hc.last_failure(self.ctx)
        self.assertIn("Player 2's game closed right after starting", last['error'])
        self.assertIn('not owned by root or us', last['error'])
        self.assertEqual(sorted(p['seat'] for p in last['players']), [1, 2])
        st = hc.status(self.ctx)
        self.assertFalse(st['running'])
        self.assertIn('closed right after starting', st['lastError'])

    def test_start_fails_with_the_players_reason(self):
        self.runner.tools['fusermount3'] = '/usr/bin/fusermount3'

        def on_spawn(argv, pid):
            if argv[0] == 'labwc':
                # The container (labwc's autostart) found both players gone at once: it leaves the reason, tidies up.
                hc.remember_failure(self.ctx, {'game': None}, "Player 1's game closed right after starting: boom", [])
                os.unlink(self.ctx.session_file)
                self.runner.live.discard(pid)
        self.runner.on_spawn = on_spawn
        rc = hc.main(['start', '--game', 'steam:400', '--pads', 'event20,event30', '--width', '1280', '--height', '800'], self.ctx)
        self.assertEqual(rc, 1)
        self.assertIn("error: Player 1's game closed right after starting: boom", self.out.getvalue())


class CheckAndCli(Base):
    def test_check(self):
        self.steam()
        self.assertEqual(hc.main(['check'], self.ctx), 0)
        r = self.result()
        self.assertTrue(r['ready'])
        self.assertTrue(r['tools']['gamescope'] and r['tools']['bwrap'] and r['tools']['umu-run'])
        self.assertFalse(r['tools']['fuse-overlayfs'])
        self.assertTrue(r['gamescopeBackend'])
        self.assertEqual([p['name'] for p in r['proton']][:2], ['GE-Proton10-4', 'GE-Proton9-20'])
        self.assertEqual(r['gbe']['cached'], {'linux': False, 'windows': False})
        self.assertTrue(any('fuse-overlayfs' in w for w in r['warnings']))

    def test_check_not_ready(self):
        del self.runner.tools['labwc']
        hc.main(['check'], self.ctx)
        r = self.result()
        self.assertFalse(r['ready'])
        self.assertIn('labwc is missing.', r['problems'])

    def test_usage_and_unknown_option(self):
        self.assertEqual(hc.main([], self.ctx), 1)
        self.assertTrue(self.lines()[-1].startswith('error: usage: hearth-coop'))
        self.assertEqual(hc.main(['start', '--frobnicate'], self.ctx), 1)
        self.assertEqual(self.lines()[-1], 'error: Unknown option --frobnicate.')

    def test_status_and_stop_when_idle(self):
        hc.main(['status'], self.ctx)
        self.assertEqual(self.result(), {'running': False})
        hc.main(['stop'], self.ctx)
        self.assertEqual(self.result(), {'stopped': False, 'running': False})

    def test_rc_xml_is_valid(self):
        players = [{'appId': 'hearth-coop-p%d' % i, 'rect': r} for i, r in enumerate(hc.layout_rects(1280, 800, 4), 1)]
        doc = xml.dom.minidom.parseString(hc.rc_xml(players, slot=players[0]['rect']))
        self.assertEqual(len(doc.getElementsByTagName('windowRule')), 5)


if __name__ == '__main__':
    unittest.main()
