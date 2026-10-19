"""Tests for hearth-seats (Multi-seat) and the seats-* commands of hearth-admin, with a made-up /sys, made-up
loginctl/systemctl/useradd (a fake runner that records commands) and files written under a temporary folder."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import socket
import struct
import tempfile
import threading
import types
import unittest
from unittest import mock

NIMBUS = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace('-', '_'), str(NIMBUS / name))
    spec = importlib.util.spec_from_loader(name.replace('-', '_'), loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def write(path, text=''):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb' if isinstance(text, bytes) else 'w') as f:
        f.write(text)


def slurp(path):
    with open(path) as f:
        return f.read()


def edid(name):
    e = bytearray(128)
    e[0:8] = b'\x00\xff\xff\xff\xff\xff\xff\x00'
    e[72:77] = b'\x00\x00\x00\xfc\x00'
    e[77:90] = (name.encode() + b'\n').ljust(13, b' ')
    return bytes(e)


class FakeSys:
    """A /sys with PCI graphics cards (and their HDMI sound), screens, controllers and their hidraw nodes."""

    def __init__(self, root):
        self.root = root
        self.n_event = 0
        self.n_hid = 0

    def p(self, *parts):
        return os.path.join(self.root, *parts)

    def gpu(self, card, pci, vendor='0x1002', device='0x73bf', boot_vga=True, outputs=(), audio_card=None):
        dev = self.p('devices/pci0000:00/0000:00:01.0', pci)
        write(os.path.join(dev, 'vendor'), vendor + '\n')
        write(os.path.join(dev, 'device'), device + '\n')
        write(os.path.join(dev, 'boot_vga'), '1\n' if boot_vga else '0\n')
        os.makedirs(os.path.join(dev, 'drm', card), exist_ok=True)
        os.makedirs(self.p('class/drm'), exist_ok=True)
        os.makedirs(self.p('bus/pci/devices'), exist_ok=True)
        os.symlink(dev, self.p('bus/pci/devices', pci))
        cdir = self.p('class/drm', card)
        os.makedirs(cdir)
        os.symlink(dev, os.path.join(cdir, 'device'))
        for conn, connected, mode, monitor in outputs:
            o = self.p('class/drm', '%s-%s' % (card, conn))
            write(os.path.join(o, 'status'), 'connected\n' if connected else 'disconnected\n')
            write(os.path.join(o, 'enabled'), 'enabled\n' if connected else 'disabled\n')
            write(os.path.join(o, 'modes'), (mode + '\n1280x720\n') if mode else '')
            write(os.path.join(o, 'edid'), edid(monitor) if monitor else b'')
        if audio_card:
            apci = pci[:-1] + '1'
            adev = self.p('devices/pci0000:00/0000:00:01.0', apci)
            os.makedirs(os.path.join(adev, 'sound', audio_card), exist_ok=True)
            os.makedirs(self.p('class/sound'), exist_ok=True)
            sdir = self.p('class/sound', audio_card)
            os.makedirs(sdir)
            os.symlink(adev, os.path.join(sdir, 'device'))
            write(os.path.join(sdir, 'id'), 'HDMI\n')

    def pad(self, name, bus='usb', port='1-2', uniq='', virtual=False, gamepad=True, hidraw=True):
        self.n_event += 1
        self.n_hid += 1
        if virtual:
            hid = self.p('devices/virtual/input')
        elif bus == 'usb':
            hid = self.p('devices/pci0000:00/0000:00:14.0/usb1', port, port + ':1.0', '0003:045E:028E.%04X' % self.n_hid)
        else:
            hid = self.p('devices/pci0000:00/0000:00:14.0/usb1/1-10/1-10:1.0/bluetooth/hci0/hci0:11', '0005:045E:0B13.%04X' % self.n_hid)
        inp = os.path.join(hid, 'input', 'input%d' % (self.n_event + 10))
        write(os.path.join(inp, 'name'), name + '\n')
        write(os.path.join(inp, 'uniq'), uniq + '\n')
        write(os.path.join(inp, 'phys'), 'usb-0000:00:14.0-2/input0\n')
        write(os.path.join(inp, 'id/bustype'), ('0003' if bus == 'usb' else '0005') + '\n')
        write(os.path.join(inp, 'id/vendor'), '045e\n')
        write(os.path.join(inp, 'id/product'), '028e\n')
        # key bitmap: BTN_GAMEPAD is bit 0x130 = word 4 (of 64-bit words), bit 48
        key = '7fdb000000000000 0 0 0 0' if gamepad else '0 0 0 0 0'
        write(os.path.join(inp, 'capabilities/key'), key + '\n')
        ev = 'event%d' % self.n_event
        os.makedirs(os.path.join(inp, ev))
        os.makedirs(self.p('class/input'), exist_ok=True)
        os.symlink(os.path.join(inp, ev), self.p('class/input', ev))
        os.symlink(inp, os.path.join(inp, ev, 'device'))
        write(os.path.join(hid, 'uevent'), 'HID_UNIQ=%s\n' % uniq)
        if hidraw and not virtual:
            h = 'hidraw%d' % self.n_hid
            os.makedirs(os.path.join(hid, 'hidraw', h))
            os.makedirs(self.p('class/hidraw'), exist_ok=True)
            os.symlink(os.path.join(hid, 'hidraw', h), self.p('class/hidraw', h))
        return ev


class FakeRunner:
    def __init__(self, answers=None, fail_on=None):
        self.calls = []
        self.answers = answers or {}
        self.fail_on = fail_on

    def run(self, cmd, input=None, timeout=60):
        self.calls.append((list(cmd), input))
        key = ' '.join(cmd)
        for prefix, ans in self.answers.items():
            if key.startswith(prefix):
                return ans
        return 0, '', ''

    def admin_run(self, cmd, input=None):
        self.calls.append((list(cmd), input))
        if self.fail_on and self.fail_on in ' '.join(cmd):
            return 1, 'Failed to enable unit'
        return 0, ''

    def cmds(self):
        return [' '.join(c) for c, _ in self.calls]


def standard_sys(root, two_gpus=False, same_model=False):
    fs = FakeSys(root)
    fs.gpu('card0', '0000:03:00.0', boot_vga=True, audio_card='card1',
           outputs=[('HDMI-A-1', True, '3840x2160', 'LG TV'), ('DP-1', True, '1920x1080', 'DELL U2720'), ('DP-2', False, None, None)])
    if two_gpus:
        fs.gpu('card1', '0000:07:00.0', vendor='0x1002' if same_model else '0x10de', device='0x73bf' if same_model else '0x2684',
               boot_vga=False, audio_card='card2', outputs=[('HDMI-A-2', True, '1920x1080', 'SAMSUNG')])
    fs.pad('Xbox 360 Controller', bus='usb', port='1-2')
    fs.pad('Xbox Wireless Controller', bus='bluetooth', uniq='aa:bb:cc:dd:ee:ff')
    fs.pad('Steam Virtual Gamepad', virtual=True)
    fs.pad('Power Button', bus='usb', port='1-9', gamepad=False)
    return fs


# ============================================================================================ hearth-seats
class SeatsBase(unittest.TestCase):
    def setUp(self):
        self.hs = load('hearth-seats')
        self.tmp = tempfile.TemporaryDirectory()
        self.root = self.tmp.name
        self.hs.SYS = os.path.join(self.root, 'sys')
        self.hs.DEV = '/dev'
        self.hs.CONFIG = os.path.join(self.root, 'etc/hearth/seats.json')
        self.hs.SEAT_RUN = os.path.join(self.root, 'run/hearth-seats')
        self.hs.PASSWD = os.path.join(self.root, 'etc/passwd')
        self.hs.GROUP = os.path.join(self.root, 'etc/group')
        self.hs.HOST_STATE = os.path.join(self.root, 'home/state/seat-host.json')
        write(self.hs.PASSWD, 'root:x:0:0::/root:/bin/bash\nbob:x:1000:1000::/var/home/bob:/bin/bash\n'
                              'hearth-p2:x:1001:1001:Hearth Player 2:/var/home/hearth-p2:/bin/bash\n')
        write(self.hs.GROUP, 'wheel:x:10:bob\ninput:x:104:\nrender:x:105:\n')
        self.runner = FakeRunner({'loginctl list-seats': (0, 'seat0\n', '')})
        self.hs.RUNNER = self.runner
        self.hs.main_user = lambda: {'name': 'bob', 'uid': 1000}

    def tearDown(self):
        self.tmp.cleanup()


class Scan(SeatsBase):
    def test_gpus_outputs_and_monitor_names(self):
        standard_sys(self.hs.SYS)
        gpus = self.hs.scan_gpus()
        self.assertEqual(len(gpus), 1)
        g = gpus[0]
        self.assertEqual((g['id'], g['card'], g['vk_device'], g['boot_vga']), ('0000:03:00.0', 'card0', '1002:73bf', True))
        outs = {o['connector']: o for o in g['outputs']}
        self.assertEqual(set(outs), {'HDMI-A-1', 'DP-1', 'DP-2'})
        self.assertTrue(outs['HDMI-A-1']['connected'])
        self.assertEqual(outs['HDMI-A-1']['mode'], '3840x2160')
        self.assertEqual(outs['HDMI-A-1']['monitor'], 'LG TV')
        self.assertFalse(outs['DP-2']['connected'])

    def test_pads_ids_hidraw_and_skips(self):
        standard_sys(self.hs.SYS)
        pads = {p['name']: p for p in self.hs.scan_pads()}
        self.assertEqual(set(pads), {'Xbox 360 Controller', 'Xbox Wireless Controller'})   # virtual + non-pad skipped
        usb = pads['Xbox 360 Controller']
        self.assertEqual(usb['id'], 'usb:1-2')
        self.assertEqual(usb['bus'], 'usb')
        self.assertEqual(usb['events'], ['/dev/input/event1'])
        self.assertEqual(usb['hidraw'], ['/dev/hidraw1'])
        bt = pads['Xbox Wireless Controller']
        self.assertEqual(bt['id'], 'uniq:aa:bb:cc:dd:ee:ff')
        self.assertIsNone(bt['usb_port'])
        self.assertEqual(bt['seat'], 1)

    def test_pad_seat_from_config(self):
        standard_sys(self.hs.SYS)
        pads = {p['id']: p for p in self.hs.scan_pads({'seats': [{'seat': 1, 'pads': []}, {'seat': 2, 'pads': ['usb:1-2']}]})}
        self.assertEqual(pads['usb:1-2']['seat'], 2)
        self.assertEqual(pads['uniq:aa:bb:cc:dd:ee:ff']['seat'], 1)

    def test_audio_linked_to_gpu(self):
        standard_sys(self.hs.SYS, two_gpus=True)
        audio = {a['card']: a for a in self.hs.scan_audio(self.hs.scan_gpus())}
        self.assertEqual(audio['card1']['gpu'], '0000:03:00.0')
        self.assertEqual(audio['card2']['gpu'], '0000:07:00.0')

    def test_scan_shape(self):
        standard_sys(self.hs.SYS)
        info = self.hs.scan()
        self.assertEqual(set(info), {'gpus', 'pads', 'audio', 'logind_seats', 'users', 'config', 'warnings'})
        self.assertEqual(info['logind_seats'], ['seat0'])
        self.assertEqual(info['users']['main']['name'], 'bob')
        self.assertEqual([p['name'] for p in info['users']['players']], ['hearth-p2'])
        self.assertIsNone(info['config'])
        json.dumps(info)

    def test_input_group_warning(self):
        standard_sys(self.hs.SYS)
        write(self.hs.GROUP, 'input:x:104:bob\n')
        self.assertTrue(any('input' in w for w in self.hs.scan()['warnings']))

    def test_has_bits(self):
        self.assertEqual(self.hs.has_bits('7fdb000000000000 0 0 0 0', 0x130, 0x120), [True, False])
        self.assertEqual(self.hs.has_bits('', 0x130), [False])

    def test_edid_name_missing(self):
        self.assertEqual(self.hs.edid_monitor_name(b''), '')


class Plan(SeatsBase):
    def test_one_card_two_screens_is_nested(self):
        standard_sys(self.hs.SYS)
        plan = self.hs.plan_from(self.hs.scan())
        self.assertEqual(plan['possible'], {'gpu': False, 'nested': True})
        self.assertEqual(plan['recommended'], 'nested')
        seats = plan['layout']['seats']
        self.assertEqual([(s['seat'], s['user']) for s in seats], [(1, 'bob'), (2, 'hearth-p2')])
        self.assertEqual({s['output'] for s in seats}, {'card0-HDMI-A-1', 'card0-DP-1'})
        self.assertEqual(seats[0]['pads'], ['usb:1-2'])
        self.assertEqual(seats[1]['pads'], ['uniq:aa:bb:cc:dd:ee:ff'])

    def test_two_cards_is_gpu(self):
        standard_sys(self.hs.SYS, two_gpus=True)
        plan = self.hs.plan_from(self.hs.scan())
        self.assertEqual(plan['recommended'], 'gpu')
        seats = plan['layout']['seats']
        self.assertEqual([s['gpu'] for s in seats], ['0000:03:00.0', '0000:07:00.0'])   # boot card is player 1's

    def test_same_model_cards_fall_back_to_nested(self):
        standard_sys(self.hs.SYS, two_gpus=True, same_model=True)
        plan = self.hs.plan_from(self.hs.scan())
        self.assertFalse(plan['possible']['gpu'])
        self.assertEqual(plan['recommended'], 'nested')
        self.assertTrue(any('same model' in r for r in plan['reasons']))

    def test_one_screen_not_possible(self):
        FakeSys(self.hs.SYS).gpu('card0', '0000:03:00.0', outputs=[('eDP-1', True, '1920x1080', None)])
        plan = self.hs.plan_from(self.hs.scan())
        self.assertIsNone(plan['recommended'])
        self.assertIsNone(plan['layout'])
        self.assertTrue(any('second TV' in r for r in plan['reasons']))


class Session(SeatsBase):
    def test_parse_env(self):
        env = self.hs.parse_env('# c\nHEARTH_SEAT_MODE=gpu\nPULSE_SINK=\'alsa_output.x y\'\nbad-key=1\n')
        self.assertEqual(env, {'HEARTH_SEAT_MODE': 'gpu', 'PULSE_SINK': 'alsa_output.x y'})

    def test_gamescope_args(self):
        self.assertEqual(self.hs.session_gamescope_args({'HEARTH_SEAT_MODE': 'gpu', 'HEARTH_SEAT_VK_DEVICE': '10de:2684', 'HEARTH_SEAT_OUTPUT': 'HDMI-A-2'}),
                         ['--prefer-vk-device', '10de:2684', '--prefer-output', 'HDMI-A-2'])
        self.assertEqual(self.hs.session_gamescope_args({'HEARTH_SEAT_MODE': 'nested', 'HEARTH_SEAT_SIZE': '1920x1080'}),
                         ['-W', '1920', '-H', '1080', '-w', '1920', '-h', '1080'])
        self.assertEqual(self.hs.session_gamescope_args({'HEARTH_SEAT_MODE': 'nested', 'HEARTH_SEAT_SIZE': '; rm'}), [])

    def test_session_refuses_main_user(self):
        with self.assertRaises(self.hs.Fail):
            self.hs.session()

    def test_main_user_gamescope_args_only_in_gpu_mode(self):
        self.assertEqual(self.hs.gamescope_args(), [])
        cfg = {'enabled': True, 'mode': 'gpu', 'main_user': 'bob', 'seats': [
            {'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1', 'vk_device': '1002:73bf'},
            {'seat': 2, 'user': 'hearth-p2', 'output': 'card1-HDMI-A-2', 'vk_device': '10de:2684'}]}
        write(self.hs.CONFIG, json.dumps(cfg))
        self.assertEqual(self.hs.gamescope_args(), ['--prefer-vk-device', '1002:73bf', '--prefer-output', 'HDMI-A-1'])
        cfg['enabled'] = False
        write(self.hs.CONFIG, json.dumps(cfg))
        self.assertEqual(self.hs.gamescope_args(), [])


class Host(SeatsBase):
    def nested_config(self, **kw):
        cfg = {'enabled': True, 'mode': 'nested', 'main_user': 'bob', 'seats': [
            {'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1', 'pads': []},
            {'seat': 2, 'user': 'hearth-p2', 'output': 'card0-DP-1', 'pads': ['usb:1-2']}]}
        cfg.update(kw)
        write(self.hs.CONFIG, json.dumps(cfg))
        return cfg

    def test_host_check(self):
        standard_sys(self.hs.SYS)
        self.assertFalse(self.hs.host_check())          # not set up
        self.nested_config()
        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop('HEARTH_SEAT', None)
            self.assertTrue(self.hs.host_check())
            self.nested_config(enabled=False)
            self.assertFalse(self.hs.host_check())
            self.nested_config(mode='gpu')
            self.assertFalse(self.hs.host_check())
            self.nested_config()
            os.environ['HEARTH_SEAT'] = '2'              # an extra player never hosts
            self.assertFalse(self.hs.host_check())

    def test_host_check_needs_both_screens(self):
        fs = FakeSys(self.hs.SYS)
        fs.gpu('card0', '0000:03:00.0', outputs=[('HDMI-A-1', True, '1920x1080', None), ('DP-1', False, None, None)])
        self.nested_config()
        os.environ.pop('HEARTH_SEAT', None)
        self.assertFalse(self.hs.host_check())          # the second TV is off: normal Game Mode

    def test_host_paused_after_failures(self):
        standard_sys(self.hs.SYS)
        self.nested_config()
        os.environ.pop('HEARTH_SEAT', None)
        self.hs.save_host_state({'failures': 2})
        self.assertFalse(self.hs.host_check())
        self.hs.reset_host_failures()
        self.assertTrue(self.hs.host_check())

    def test_rc_xml(self):
        xml = self.hs.host_rc_xml([{'seat': 1, 'connector': 'HDMI-A-1'}, {'seat': 2, 'connector': 'DP-1"<x>'}])
        self.assertIn('sandboxEngine="hearth-seat" sandboxAppId="seat1"', xml)
        self.assertIn('output="HDMI-A-1"', xml)
        self.assertIn('DP-1&quot;&lt;x&gt;', xml)
        self.assertNotIn('<x>', xml)

    def test_pick_sink(self):
        sinks = [{'name': 'alsa_output.pci-0000_03_00.1.hdmi-stereo', 'description': 'Navi HDMI', 'ports': [{'description': 'HDMI / DisplayPort (LG TV)'}]},
                 {'name': 'alsa_output.pci-0000_03_00.1.hdmi-stereo-extra1', 'description': 'Navi HDMI 2', 'ports': [{'description': 'HDMI / DisplayPort 2 (DELL U2720)'}]}]
        self.assertEqual(self.hs.pick_sink(sinks, {'monitor': 'DELL U2720'}), 'alsa_output.pci-0000_03_00.1.hdmi-stereo-extra1')
        self.assertEqual(self.hs.pick_sink(sinks, {'monitor': 'LG TV'}), 'alsa_output.pci-0000_03_00.1.hdmi-stereo')
        self.assertEqual(self.hs.pick_sink(sinks, {'monitor': 'LG TV', 'audio': 'custom'}), 'custom')
        self.assertIsNone(self.hs.pick_sink(sinks, {'monitor': None}))

    def test_host_falls_back_without_two_screens(self):
        self.assertEqual(self.hs.host(['gamescope']), self.hs.EXIT_HOST_FALLBACK)


class FakeCompositor(threading.Thread):
    """Answers like a compositor with security-context-v1, and records what the client asked for."""

    def __init__(self, sock, offer=True):
        super().__init__(daemon=True)
        self.sock, self.offer = sock, offer
        self.requests = []
        self.fds = []

    def send(self, obj, op, args=b''):
        self.sock.sendall(struct.pack('<II', obj, ((8 + len(args)) << 16) | op) + args)

    @staticmethod
    def string(s):
        b = s.encode() + b'\0'
        return struct.pack('<I', len(b)) + b + b'\0' * (-len(b) % 4)

    def run(self):
        buf = b''
        import array
        while True:
            try:
                data, anc, _, _ = self.sock.recvmsg(4096, socket.CMSG_SPACE(8 * 4))
            except OSError:
                return
            for _, _, fd in anc:
                self.fds.extend(array.array('i', fd[:len(fd) - len(fd) % 4]))
            if not data:
                return
            buf += data
            while len(buf) >= 8:
                obj, word = struct.unpack('<II', buf[:8])
                size = word >> 16
                if len(buf) < size:
                    break
                payload, buf = buf[8:size], buf[size:]
                op = word & 0xffff
                self.requests.append((obj, op, payload))
                if obj == 1 and op == 1:   # get_registry
                    reg = struct.unpack('<I', payload)[0]
                    self.send(reg, 0, struct.pack('<I', 1) + self.string('wl_compositor') + struct.pack('<I', 6))
                    if self.offer:
                        self.send(reg, 0, struct.pack('<I', 7) + self.string('wp_security_context_manager_v1') + struct.pack('<I', 1))
                elif obj == 1 and op == 0:   # sync
                    self.send(struct.unpack('<I', payload)[0], 0, struct.pack('<I', 0))


class WaylandClient(SeatsBase):
    def client(self, offer=True):
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        srv = FakeCompositor(b, offer)
        srv.start()
        wl = self.hs.Wayland.__new__(self.hs.Wayland)
        wl.sock, wl.next_id, wl.buf, wl.globals, wl.registry = a, 2, b'', {}, None
        self.addCleanup(a.close)
        self.addCleanup(b.close)
        return wl, srv

    def test_string_padding(self):
        self.assertEqual(self.hs.Wayland.string('abc'), struct.pack('<I', 4) + b'abc\0')
        self.assertEqual(self.hs.Wayland.string('abcd'), struct.pack('<I', 5) + b'abcd\0\0\0\0')

    def test_security_context_listeners(self):
        wl, srv = self.client()
        wl.start()
        self.assertIn('wp_security_context_manager_v1', wl.globals)
        os.makedirs(os.path.join(self.hs.SEAT_RUN, 'hearth-p2'))
        with mock.patch.dict(os.environ, {'XDG_RUNTIME_DIR': self.root}):
            opened = self.hs.open_seat_sockets(wl, [{'seat': 1, 'user': 'bob'}, {'seat': 2, 'user': 'hearth-p2'}], 'bob')
        try:
            self.assertEqual([o[0] for o in opened], [os.path.join(self.root, 'hearth-seat1-wayland'),
                                                      os.path.join(self.hs.SEAT_RUN, 'hearth-p2', 'wayland-0')])
            self.assertTrue(all(os.path.exists(o[0]) for o in opened))
            self.assertEqual(len(srv.fds), 4)    # listen fd + close fd, per player
            bind = next(r for r in srv.requests if r[0] == wl.registry and r[1] == 0)
            self.assertIn(b'wp_security_context_manager_v1', bind[2])
            engines = [r[2] for r in srv.requests if r[1] == 1 and b'hearth-seat' in r[2]]
            self.assertEqual(len(engines), 2)
            app_ids = [r[2] for r in srv.requests if r[1] == 2 and r[0] not in (1, wl.registry)]
            self.assertTrue(any(b'seat2' in a for a in app_ids))
        finally:
            for path, ls, w, _ in opened:
                ls.close()
                os.close(w)
            for fd in srv.fds:
                os.close(fd)

    def test_compositor_without_security_context(self):
        wl, srv = self.client(offer=False)
        wl.start()
        with self.assertRaises(self.hs.Fail):
            wl.bind('wp_security_context_manager_v1', 1)


class AdminCalls(SeatsBase):
    def test_progress_passed_on_and_result(self):
        self.runner.answers = {'pkexec': (0, 'progress: 10 Creating\n{"ok": true}\n', '')}
        out = io.StringIO()
        with mock.patch('sys.stdout', out), mock.patch('os.geteuid', return_value=1000):
            r = self.hs.admin(['seats-setup'], {'mode': 'nested'})
        self.assertEqual(r, {'ok': True})
        self.assertIn('progress: 10 Creating', out.getvalue())
        cmd, stdin = self.runner.calls[-1]
        self.assertEqual(cmd, ['pkexec', self.hs.ADMIN, 'seats-setup'])
        self.assertEqual(json.loads(stdin), {'mode': 'nested'})

    def test_error_line(self):
        self.runner.answers = {'pkexec': (1, 'error: player 1 must be you\n', '')}
        with mock.patch('os.geteuid', return_value=1000):
            with self.assertRaises(self.hs.Fail) as e:
                self.hs.admin(['seats-setup'], {})
        self.assertIn('player 1 must be you', str(e.exception))

    def test_assign_pad_sends_full_map(self):
        write(self.hs.CONFIG, json.dumps({'mode': 'nested', 'seats': [{'seat': 1, 'user': 'bob', 'pads': ['usb:1-2']},
                                                                     {'seat': 2, 'user': 'hearth-p2', 'pads': ['uniq:aa:bb:cc:dd:ee:ff']}]}))
        self.runner.answers = {'pkexec': (0, '{"ok": true}\n', '')}
        with mock.patch('os.geteuid', return_value=1000):
            self.hs.assign_pad('usb:1-2', 2)
            with self.assertRaises(self.hs.Fail):
                self.hs.assign_pad('usb:1-2', 3)
        self.assertEqual(json.loads(self.runner.calls[-1][1]), {'pads': {'usb:1-2': 2, 'uniq:aa:bb:cc:dd:ee:ff': 2}})

    def test_check_layout_friendly_errors(self):
        standard_sys(self.hs.SYS)
        info = self.hs.scan()
        with self.assertRaises(self.hs.Fail):
            self.hs.check_layout({'mode': 'nested', 'seats': [{'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1'},
                                                              {'seat': 2, 'user': 'hearth-p2', 'output': 'card0-DP-2'}]}, info)  # disconnected
        self.hs.check_layout({'mode': 'nested', 'seats': [{'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1'},
                                                          {'seat': 2, 'user': 'hearth-p2', 'output': 'card0-DP-1'}]}, info)

    def test_status(self):
        self.assertFalse(self.hs.status()['configured'])
        write(self.hs.CONFIG, json.dumps({'enabled': True, 'mode': 'gpu', 'seats': [{'seat': 1, 'user': 'bob'}, {'seat': 2, 'user': 'hearth-p2'}]}))
        self.runner.answers = {'systemctl is-active hearth-seat@hearth-p2.service': (0, 'active\n', '')}
        st = self.hs.status()
        self.assertEqual([s['state'] for s in st['seats']], ['active', 'running'])


# ============================================================================================ hearth-admin seats-*
class AdminBase(unittest.TestCase):
    def setUp(self):
        self.ha = load('hearth-admin')
        self.tmp = tempfile.TemporaryDirectory()
        r = self.root = self.tmp.name
        self.ha.SEATS_SYS = os.path.join(r, 'sys')
        self.ha.SEATS_DEV = os.path.join(r, 'dev')
        self.ha.SEATS_CONFIG = os.path.join(r, 'etc/hearth/seats.json')
        self.ha.SEATS_ENV_DIR = os.path.join(r, 'etc/hearth/seats')
        self.ha.SEATS_RULES = os.path.join(r, 'etc/udev/rules.d/72-hearth-seats.rules')
        self.ha.SEATS_TMPFILES = os.path.join(r, 'etc/tmpfiles.d/hearth-seats.conf')
        self.ha.SEATS_SDDM = os.path.join(r, 'etc/sddm.conf.d/zzz-hearth-seats.conf')
        self.ha.SEATS_SYSTEMD_ETC = os.path.join(r, 'etc/systemd/system')
        self.ha.SEATS_RUN = os.path.join(r, 'run/hearth-seats')
        self.ha.SEATS_PASSWD_FILE = os.path.join(r, 'etc/passwd')
        self.ha.SEATS_GROUP_FILE = os.path.join(r, 'etc/group')
        write(self.ha.SEATS_PASSWD_FILE, 'root:x:0:0::/root:/bin/bash\nbob:x:1000:1000::/var/home/bob:/bin/bash\n')
        write(self.ha.SEATS_GROUP_FILE, 'wheel:x:10:bob\nrender:x:105:\ninput:x:104:\n')
        standard_sys(self.ha.SEATS_SYS, two_gpus=True)
        self.runner = FakeRunner()
        self.ha.seats_run = self.runner.admin_run
        self.ha.caller = lambda: types.SimpleNamespace(pw_name='bob', pw_uid=1000)
        self.ha.time = types.SimpleNamespace(sleep=lambda s: None)

    def tearDown(self):
        self.tmp.cleanup()

    def nested(self, **over):
        req = {'mode': 'nested', 'seats': [
            {'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1', 'pads': ['usb:1-2']},
            {'seat': 2, 'user': 'hearth-p2', 'output': 'card0-DP-1', 'pads': ['uniq:aa:bb:cc:dd:ee:ff']}]}
        req.update(over)
        return req

    def gpu(self):
        return {'mode': 'gpu', 'seats': [
            {'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1', 'pads': []},
            {'seat': 2, 'user': 'hearth-p2', 'output': 'card1-HDMI-A-2', 'pads': ['usb:1-2']}]}

    def run_cmd(self, fn, stdin='', *args):
        out = io.StringIO()
        code = 0
        with mock.patch('sys.stdin', io.StringIO(stdin)), mock.patch('sys.stdout', out):
            try:
                fn(*args)
            except SystemExit as e:
                code = e.code
        lines = out.getvalue().splitlines()
        return code, lines


class Validate(AdminBase):
    def test_good_nested(self):
        c = self.ha.seats_validate(self.nested(), 'bob')
        s2 = c['seats'][1]
        self.assertEqual((s2['user'], s2['gpu'], s2['card'], s2['size']), ('hearth-p2', '0000:03:00.0', 'card0', '1920x1080'))
        self.assertEqual(s2['vk_device'], '1002:73bf')
        self.assertEqual(s2['audio_pci'], [])   # nested: the sound card stays with player 1

    def test_good_gpu_takes_card_and_its_sound(self):
        c = self.ha.seats_validate(self.gpu(), 'bob')
        self.assertEqual(c['seats'][1]['gpu'], '0000:07:00.0')
        self.assertEqual(c['seats'][1]['audio_pci'], ['0000:07:00.1'])
        self.assertEqual(c['seats'][1]['vk_device'], '10de:2684')

    def bad(self, req, main='bob'):
        with self.assertRaises(self.ha.SeatsError):
            self.ha.seats_validate(req, main)

    def test_rejects(self):
        r = self.nested()
        r['seats'][1]['user'] = 'root'
        self.bad(r)
        r = self.nested()
        r['seats'][1]['user'] = 'hearth-p5'
        self.bad(r)
        r = self.nested()
        r['seats'][1]['user'] = 'hearth-p3'          # must match the player number
        self.bad(r)
        self.bad(self.nested(), main='alice')         # player 1 must be whoever ran it
        r = self.nested()
        r['seats'][1]['output'] = 'card0-HDMI-A-1'    # same screen twice
        self.bad(r)
        r = self.nested()
        r['seats'][1]['output'] = 'card0-../../../etc'
        self.bad(r)
        r = self.nested()
        r['seats'][1]['output'] = 'card9-HDMI-A-9'    # not there
        self.bad(r)
        r = self.nested()
        r['seats'][1]['pads'] = ['usb:1-2", RUN+="/bin/sh -c evil']
        self.bad(r)
        r = self.nested()
        r['seats'][1]['pads'] = ['uniq:aa|bb*']
        self.bad(r)
        r = self.nested()
        r['seats'][1]['pads'] = ['usb:1-2']           # also player 1's
        self.bad(r)
        r = self.nested()
        r['seats'][1]['seat'] = 3
        self.bad(r)
        r = self.nested()
        r['seats'][1]['seat'] = True
        self.bad(r)
        r = self.nested()
        r['seats'][1]['audio'] = 'x"; rm -rf /'
        self.bad(r)
        self.bad(self.nested(mode='both'))
        self.bad(self.nested(seats=[self.nested()['seats'][0]]))
        self.bad([])
        g = self.gpu()
        g['seats'][1]['output'] = 'card0-DP-1'        # gpu mode: player 2 on player 1's card
        self.bad(g)

    def test_rules(self):
        c = self.ha.seats_validate(self.gpu(), 'bob')
        rules = self.ha.seats_rules(c)
        self.assertIn('SUBSYSTEMS=="usb", KERNELS=="1-2", SUBSYSTEM=="input|hidraw", ENV{ID_SEAT}="seat-hearth-p2"', rules)
        self.assertIn('SUBSYSTEM=="drm", KERNEL=="card[0-9]*", KERNELS=="0000:07:00.0"', rules)
        self.assertIn('SUBSYSTEM=="sound", KERNELS=="0000:07:00.1"', rules)
        self.assertIn('TAG-="uaccess"', rules)
        self.assertIn('TAG-="master-of-seat"', rules)
        self.assertIn('RUN+="/usr/bin/setfacl -m u:hearth-p2:rw $devnode"', rules)
        self.assertNotIn('0000:03:00.0', rules)       # player 1's card is never touched
        n = self.ha.seats_rules(self.ha.seats_validate(self.nested(), 'bob'))
        self.assertIn('SUBSYSTEM=="hidraw", IMPORT{parent}="HID_UNIQ"', n)
        self.assertIn('ATTRS{uniq}=="aa:bb:cc:dd:ee:ff"', n)
        self.assertNotIn('SUBSYSTEM=="drm"', n)
        self.assertNotIn('KERNELS=="1-2"', n)          # player 1's controller stays as it is

    def test_tmpfiles_and_env(self):
        c = self.ha.seats_validate(self.nested(), 'bob')
        t = self.ha.seats_tmpfiles(c)
        self.assertIn('d %s/hearth-p2 2750 bob hearth-p2 -' % self.ha.SEATS_RUN, t)
        env = self.ha.seats_env(c, c['seats'][1])
        self.assertIn('HEARTH_SEAT_MODE=nested', env)
        self.assertIn('HEARTH_SEAT_OUTPUT=DP-1', env)
        self.assertIn('HEARTH_SEAT_SIZE=1920x1080', env)


class Setup(AdminBase):
    def test_setup_nested_writes_everything(self):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.nested(start=True)))
        self.assertEqual(code, 0, lines)
        result = json.loads(lines[-1])
        self.assertTrue(result['ok'])
        self.assertEqual(result['created_users'], ['hearth-p2'])
        self.assertFalse(result['restart'])
        self.assertTrue(any(l.startswith('progress: ') for l in lines))
        cmds = self.runner.cmds()
        useradd = next(c for c in cmds if c.startswith('useradd'))
        self.assertIn('--groups render', useradd)
        self.assertNotIn('input', useradd)
        self.assertNotIn('wheel', useradd)
        self.assertTrue(useradd.endswith(' hearth-p2'))
        self.assertIn('systemctl enable --now hearth-seat@hearth-p2.path', cmds)
        for f in (self.ha.SEATS_RULES, self.ha.SEATS_TMPFILES, self.ha.SEATS_SDDM, os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.env')):
            self.assertTrue(os.path.exists(f), f)
        self.assertIn('HideUsers=hearth-p2', slurp(self.ha.SEATS_SDDM))
        cfg = json.loads(slurp(self.ha.SEATS_CONFIG))
        self.assertEqual((cfg['mode'], cfg['enabled'], cfg['main_user']), ('nested', True, 'bob'))
        self.assertEqual(cfg['units'], ['hearth-seat@hearth-p2.path'])

    def test_setup_gpu_enables_service_and_asks_for_restart(self):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.gpu()))
        self.assertEqual(code, 0, lines)
        self.assertTrue(json.loads(lines[-1])['restart'])
        self.assertIn('systemctl enable hearth-seat@hearth-p2.service', self.runner.cmds())

    def test_failure_rolls_back(self):
        self.runner.fail_on = 'systemctl enable'
        write(self.ha.SEATS_SDDM, '# something already here\n')
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.nested()))
        self.assertEqual(code, 1)
        self.assertTrue(lines[-1].startswith('error: multi-seat couldn\'t be set up'), lines[-1])
        self.assertIn('Nothing was changed', lines[-1])
        for f in (self.ha.SEATS_RULES, self.ha.SEATS_TMPFILES, self.ha.SEATS_CONFIG, os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.env')):
            self.assertFalse(os.path.exists(f), f)
        self.assertEqual(slurp(self.ha.SEATS_SDDM), '# something already here\n')   # restored, not deleted
        self.assertIn('userdel -r hearth-p2', self.runner.cmds())

    def test_bad_request_changes_nothing(self):
        r = self.nested()
        r['seats'][1]['user'] = 'root'
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(r))
        self.assertEqual(code, 1)
        self.assertEqual(self.runner.calls, [])
        self.assertFalse(os.path.exists(self.ha.SEATS_CONFIG))

    def test_existing_account_reused_and_not_deleted_on_rollback(self):
        write(self.ha.SEATS_PASSWD_FILE, slurp(self.ha.SEATS_PASSWD_FILE) + 'hearth-p2:x:1001:1001::/var/home/hearth-p2:/bin/bash\n')
        self.runner.fail_on = 'systemctl enable'
        self.run_cmd(self.ha.seats_setup, json.dumps(self.nested()))
        cmds = self.runner.cmds()
        self.assertFalse(any(c.startswith('useradd') for c in cmds))
        self.assertFalse(any(c.startswith('userdel') for c in cmds))


class Remove(AdminBase):
    def setup_first(self, req):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(req))
        self.assertEqual(code, 0, lines)
        write(self.ha.SEATS_PASSWD_FILE, slurp(self.ha.SEATS_PASSWD_FILE) + 'hearth-p2:x:1001:1001::/var/home/hearth-p2:/bin/bash\n')
        self.runner.calls.clear()

    def test_remove_keeps_accounts_by_default(self):
        self.setup_first(self.nested(start=True))
        code, lines = self.run_cmd(self.ha.seats_remove, '', False)
        self.assertEqual(code, 0, lines)
        r = json.loads(lines[-1])
        self.assertEqual((r['removed_users'], r['kept_users']), ([], ['hearth-p2']))
        cmds = self.runner.cmds()
        self.assertIn('systemctl disable --now hearth-seat@hearth-p2.path', cmds)
        self.assertIn('systemctl disable --now hearth-seat@hearth-p2.service', cmds)
        self.assertFalse(any(c.startswith('userdel') for c in cmds))
        for f in (self.ha.SEATS_RULES, self.ha.SEATS_TMPFILES, self.ha.SEATS_CONFIG):
            self.assertFalse(os.path.exists(f), f)
        self.assertTrue(os.path.exists(self.ha.SEATS_SDDM))   # kept accounts stay off the login screen

    def test_remove_delete_users(self):
        self.setup_first(self.gpu())
        code, lines = self.run_cmd(self.ha.seats_remove, '', True)
        r = json.loads(lines[-1])
        self.assertEqual(r['removed_users'], ['hearth-p2'])
        self.assertTrue(r['restart'])
        self.assertIn('userdel -r hearth-p2', self.runner.cmds())
        self.assertFalse(os.path.exists(self.ha.SEATS_SDDM))

    def test_remove_without_config_is_safe(self):
        code, lines = self.run_cmd(self.ha.seats_remove, '', True)
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(lines[-1])['removed_users'], [])
        self.assertFalse(any(c.startswith('userdel') for c in self.runner.cmds()))


class Acl(AdminBase):
    def test_move_pad(self):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.nested()))
        self.assertEqual(code, 0, lines)
        self.runner.calls.clear()
        code, lines = self.run_cmd(self.ha.seats_acl, json.dumps({'pads': {'usb:1-2': 2, 'uniq:aa:bb:cc:dd:ee:ff': 1}}))
        self.assertEqual(code, 0, lines)
        r = json.loads(lines[-1])
        self.assertEqual(sorted(r['changed']), ['uniq:aa:bb:cc:dd:ee:ff', 'usb:1-2'])
        rules = slurp(self.ha.SEATS_RULES)
        self.assertIn('KERNELS=="1-2"', rules)
        self.assertNotIn('aa:bb:cc:dd:ee:ff', rules)
        cmds = self.runner.cmds()
        self.assertTrue(any(c.startswith('setfacl -b') and c.endswith('/dev/input/event1') for c in cmds), cmds)
        self.assertTrue(any(c.startswith('udevadm trigger --action=change') for c in cmds))
        cfg = json.loads(slurp(self.ha.SEATS_CONFIG))
        self.assertEqual(cfg['seats'][1]['pads'], ['usb:1-2'])

    def test_rejects_bad_input(self):
        self.run_cmd(self.ha.seats_setup, json.dumps(self.nested()))
        code, lines = self.run_cmd(self.ha.seats_acl, json.dumps({'pads': {'usb:1-2': 7}}))
        self.assertEqual(code, 1)
        code, lines = self.run_cmd(self.ha.seats_acl, json.dumps({'pads': {'/dev/input/event1': 2}}))
        self.assertEqual(code, 1)
        code, lines = self.run_cmd(self.ha.seats_acl, 'nonsense')
        self.assertEqual(code, 1)

    def test_pad_nodes(self):
        found = self.ha.seats_pad_nodes('uniq:aa:bb:cc:dd:ee:ff')
        nodes = sorted(n for _, n in found)
        self.assertEqual(nodes, [os.path.join(self.ha.SEATS_DEV, 'hidraw2'), os.path.join(self.ha.SEATS_DEV, 'input', 'event2')])
        self.assertEqual(sorted(n for _, n in self.ha.seats_pad_nodes('usb:1-2')),
                         [os.path.join(self.ha.SEATS_DEV, 'hidraw1'), os.path.join(self.ha.SEATS_DEV, 'input', 'event1')])


class Dispatch(AdminBase):
    def test_remove_args_strict(self):
        with mock.patch('os.geteuid', return_value=0):
            code, lines = self.run_cmd(self.ha.main, '', ['hearth-admin', 'seats-remove', '--everything'])
        self.assertEqual(code, 1)
        self.assertTrue(lines[-1].startswith('error: usage'))


if __name__ == '__main__':
    unittest.main()
