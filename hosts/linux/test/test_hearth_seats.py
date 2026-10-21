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

    def gpu(self, card, pci, vendor='0x1002', device='0x73bf', boot_vga=True, outputs=(), audio_card=None, render=None):
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
        if render:
            rdir = self.p('class/drm', render)
            os.makedirs(rdir)
            os.symlink(dev, os.path.join(rdir, 'device'))
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
    fs.gpu('card0', '0000:03:00.0', boot_vga=True, audio_card='card1', render='renderD128',
           outputs=[('HDMI-A-1', True, '3840x2160', 'LG TV'), ('DP-1', True, '1920x1080', 'DELL U2720'), ('DP-2', False, None, None)])
    if two_gpus:
        fs.gpu('card1', '0000:07:00.0', vendor='0x1002' if same_model else '0x10de', device='0x73bf' if same_model else '0x2684',
               boot_vga=False, audio_card='card2', render='renderD129', outputs=[('HDMI-A-2', True, '1920x1080', 'SAMSUNG')])
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
        self.assertEqual({k: plan['possible'][k] for k in ('gpu', 'nested', 'split')}, {'gpu': False, 'nested': True, 'split': True})
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

    def test_one_screen_is_split(self):
        FakeSys(self.hs.SYS).gpu('card0', '0000:03:00.0', outputs=[('eDP-1', True, '1920x1080', None)])
        self.hs.stream_support = lambda: (False, 'Streaming a player needs Sunshine, which isn\'t in this edition of Hearth OS yet.')
        plan = self.hs.plan_from(self.hs.scan())
        self.assertEqual(plan['recommended'], 'split')
        self.assertFalse(plan['possible']['nested'])
        self.assertFalse(plan['possible']['stream'])
        self.assertTrue(any('Sunshine' in r for r in plan['reasons']))    # the mode is marked unavailable, with why
        seats = plan['layout']['seats']
        self.assertEqual([(s['seat'], s['mode'], s['output'], s['region']) for s in seats],
                         [(1, 'split', 'card0-eDP-1', 0), (2, 'split', 'card0-eDP-1', 1)])
        self.assertEqual(plan['layout']['split'], {'layout': 'columns'})   # 16:9: side by side

    def test_no_screen_not_possible(self):
        FakeSys(self.hs.SYS).gpu('card0', '0000:03:00.0', outputs=[('HDMI-A-1', False, None, None)])
        self.hs.stream_support = lambda: (False, 'no Sunshine')
        plan = self.hs.plan_from(self.hs.scan())
        self.assertIsNone(plan['recommended'])
        self.assertIsNone(plan['layout'])
        self.assertTrue(any('Connect a TV' in r for r in plan['reasons']))


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


# ============================================================================================ split screen and streamed players
class AdminNewBase(AdminBase):
    def setUp(self):
        super().setUp()
        self.ha.SEATS_STREAM_RULES = os.path.join(self.root, 'etc/udev/rules.d/74-hearth-seats-stream.rules')
        self.ha.SEATS_MODULES = os.path.join(self.root, 'etc/modules-load.d/hearth-seats-stream.conf')
        self.ha.SEATS_INPUT_STATE = os.path.join(self.root, 'run/udev/hearth-seats-input.json')
        self.ha.SEATS_HOSTNAME_FILE = os.path.join(self.root, 'etc/hostname')
        write(self.ha.SEATS_HOSTNAME_FILE, 'bob-pc.local\n')
        self.chowned = []
        self.ha.seats_chown = lambda path, owner, group: self.chowned.append((path, owner, group))

    def split(self, layout='columns', **over):
        req = {'seats': [
            {'seat': 1, 'user': 'bob', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 0, 'pads': ['usb:1-2']},
            {'seat': 2, 'user': 'hearth-p2', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 1, 'pads': ['uniq:aa:bb:cc:dd:ee:ff']}],
            'split': {'layout': layout}}
        req.update(over)
        return req

    def stream(self, **st):
        return {'seats': [
            {'seat': 1, 'user': 'bob', 'mode': 'local', 'output': 'card0-HDMI-A-1', 'pads': []},
            {'seat': 2, 'user': 'hearth-p2', 'mode': 'stream', 'stream': dict({'size': '1920x1080', 'fps': 60}, **st)}]}

    def bad(self, req, main='bob'):
        with self.assertRaises(self.ha.SeatsError):
            self.ha.seats_validate(req, main)


class ValidateSplit(AdminNewBase):
    def test_regions_and_sizes(self):
        c = self.ha.seats_validate(self.split(), 'bob')
        self.assertEqual(c['mode'], 'split')
        self.assertEqual(c['split'], {'output': 'card0-HDMI-A-1', 'layout': 'columns'})
        s1, s2 = c['seats']
        self.assertEqual((s1['mode'], s1['region'], s1['size'], s1['screen_size']), ('split', 0, '1920x2160', '3840x2160'))
        self.assertEqual(s2['rect'], {'x': 1920, 'y': 0, 'w': 1920, 'h': 2160})
        self.assertEqual(self.ha.seats_split_rects(3840, 2160, 'rows'), [(0, 0, 3840, 1080), (0, 1080, 3840, 1080)])
        self.assertEqual(self.ha.seats_split_rects(1921, 1081, 'grid')[3], (960, 540, 960, 540))   # even sizes
        env = self.ha.seats_env(c, s2)
        self.assertIn('HEARTH_SEAT_MODE=split', env)
        self.assertIn('HEARTH_SEAT_SIZE=1920x2160', env)
        self.assertIn('HEARTH_SEAT_WINDOWED=1', env)
        self.assertEqual(self.ha.seats_unit(c, s2), 'hearth-seat@hearth-p2.path')
        self.assertIn('d %s/hearth-p2 2750 bob hearth-p2 -' % self.ha.SEATS_RUN, self.ha.seats_tmpfiles(c))

    def test_grid_with_three_and_a_nested_screen(self):
        r = self.split('grid')
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 3})
        r['seats'].append({'seat': 4, 'user': 'hearth-p4', 'mode': 'nested', 'output': 'card0-DP-1'})
        c = self.ha.seats_validate(r, 'bob')
        self.assertEqual(c['mode'], 'split')
        self.assertEqual(c['seats'][2]['rect'], {'x': 1920, 'y': 1080, 'w': 1920, 'h': 1080})
        self.assertEqual(c['seats'][3]['size'], '1920x1080')
        self.assertNotIn('HEARTH_SEAT_WINDOWED', self.ha.seats_env(c, c['seats'][3]))

    def test_rejects(self):
        self.bad(self.split('diagonal'))                       # presets only
        self.bad(self.split('columns; rm -rf /'))
        r = self.split()
        r['seats'][1]['region'] = 0                            # two players in one part
        self.bad(r)
        r = self.split()
        r['seats'][1]['region'] = 2                            # columns has 2 parts
        self.bad(r)
        r = self.split()
        r['seats'][1]['region'] = '1'
        self.bad(r)
        r = self.split()
        r['seats'][1]['region'] = True
        self.bad(r)
        r = self.split()
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 1})
        self.bad(r)                                            # 3 players don't fit side by side
        r = self.split()
        r['seats'][1]['output'] = 'card0-DP-1'                 # one screen for everyone on it
        self.bad(r)
        r = self.split()
        r['seats'][0]['mode'] = 'nested'                       # player 1 must be part of it
        r['seats'][0].pop('region')
        self.bad(r)
        r = self.split()
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'nested', 'output': 'card0-HDMI-A-1'})
        self.bad(r)                                            # the split screen isn't free for another player
        r = self.split()
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'gpu', 'output': 'card1-HDMI-A-2'})
        self.bad(r)                                            # a card per player doesn't mix with a host
        r = self.split()
        r.pop('split')
        self.bad(r)


class ValidateStream(AdminNewBase):
    def test_good(self):
        c = self.ha.seats_validate(self.stream(), 'bob')
        self.assertEqual(c['mode'], 'stream')
        s1, s2 = c['seats']
        self.assertEqual(s1['mode'], 'local')
        self.assertEqual((s2['output'], s2['gpu'], s2['card'], s2['vk_device']), (None, '0000:03:00.0', 'card0', '1002:73bf'))
        self.assertEqual(s2['stream'], {'size': '1920x1080', 'fps': 60, 'port': 48989, 'name': 'bob-pc Player 2', 'render': 'renderD128'})
        self.assertEqual(self.ha.seats_unit(c, s2), 'hearth-seat@hearth-p2.service')
        env = self.ha.seats_env(c, s2)
        for line in ('HEARTH_SEAT_MODE=stream', 'HEARTH_SEAT_STREAM_SIZE=1920x1080', 'HEARTH_SEAT_STREAM_FPS=60',
                     'HEARTH_SEAT_STREAM_PORT=48989', 'HEARTH_SEAT_STREAM_NAME="bob-pc Player 2"', 'HEARTH_SEAT_RENDER=/dev/dri/renderD128'):
            self.assertIn(line, env)
        self.assertNotIn('HEARTH_SEAT_OUTPUT', env)
        self.assertNotIn('hearth-p2', self.ha.seats_tmpfiles(c))   # no window socket folder needed

    def test_ports_fixed_per_player_and_clear_of_remote_play(self):
        r = self.stream()
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'stream', 'stream': {'size': '1280x720', 'fps': 30, 'port': 47989}})
        r['seats'].append({'seat': 4, 'user': 'hearth-p4', 'mode': 'stream', 'stream': {'size': '3840x2160', 'fps': 120, 'gpu': '0000:07:00.0'}})
        c = self.ha.seats_validate(r, 'bob')
        self.assertEqual([s['stream']['port'] for s in c['seats'][1:]], [48989, 49989, 50989])   # a port asked for is ignored
        self.assertEqual(c['seats'][3]['stream']['render'], 'renderD129')
        self.assertEqual(c['seats'][3]['vk_device'], '10de:2684')
        ports = self.ha.seats_stream_ports(c)
        self.assertEqual(ports[:6], ['48984/tcp', '48989/tcp', '49010/tcp', '48998/udp', '48999/udp', '49000/udp'])
        nums = [int(p.split('/')[0]) for p in ports]
        self.assertTrue(all(48984 <= n <= 51010 for n in nums))
        self.assertFalse(any(47984 <= n <= 48010 for n in nums))   # Remote play's (Sunshine's default) family

    def test_mixes(self):
        r = self.split()
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'stream'})
        c = self.ha.seats_validate(r, 'bob')
        self.assertEqual((c['mode'], c['seats'][2]['stream']['port']), ('split', 49989))
        g = self.gpu()
        for s in g['seats']:
            s['mode'] = 'gpu'
        g['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'stream'})
        self.assertEqual(self.ha.seats_validate(g, 'bob')['mode'], 'gpu')
        legacy = self.nested()                                 # older layouts: one mode for everyone
        self.assertEqual([s['mode'] for s in self.ha.seats_validate(legacy, 'bob')['seats']], ['nested', 'nested'])

    def test_rejects(self):
        self.bad(self.stream(size='4k'))
        self.bad(self.stream(size='1920x1080"; touch /x'))
        self.bad(self.stream(fps=59))
        self.bad(self.stream(fps=True))
        self.bad(self.stream(fps='60'))
        self.bad(self.stream(gpu='0000:99:00.0'))              # no such card
        self.bad(self.stream(gpu='../../etc'))
        r = self.stream()
        r['seats'][1]['output'] = 'card0-DP-1'                 # streamed players have no screen here
        self.bad(r)
        r = self.stream()
        r['seats'][1]['pads'] = ['usb:1-2']                    # nor controllers here
        self.bad(r)
        r = self.stream()
        r['seats'][0]['mode'] = 'stream'                       # player 1 isn't streamed: it means "as always"
        self.assertEqual(self.ha.seats_validate(r, 'bob')['seats'][0]['mode'], 'local')
        r = self.stream()
        r['seats'][0]['mode'] = 'nested'                       # nobody else on a screen of this card
        self.bad(r)
        r = self.stream()
        r['seats'][1]['mode'] = 'cloud'
        self.bad(r)
        r = self.stream()
        r['seats'][1]['audio'] = 'alsa_output.x'
        self.bad(r)

    def test_name_from_hostname_is_cleaned(self):
        write(self.ha.SEATS_HOSTNAME_FILE, 'Bob\'s "PC"; rm\n')
        self.assertEqual(self.ha.seats_pc_name(), 'BobsPCrm')
        c = self.ha.seats_validate(self.stream(), 'bob')
        self.assertEqual(c['seats'][1]['stream']['name'], 'BobsPCrm Player 2')

    def test_rules(self):
        c = self.ha.seats_validate(self.stream(), 'bob')
        rules = self.ha.seats_rules(c).splitlines()
        forget = next(i for i, l in enumerate(rules) if 'hearth-seat-input forget' in l)
        goto = next(i for i, l in enumerate(rules) if 'GOTO="hearth_seats_end"' in l)
        claim = next(i for i, l in enumerate(rules) if 'IMPORT{program}="/usr/libexec/nimbus/hearth-seat-input claim"' in l)
        self.assertLess(forget, goto)                          # remove events reach the helper before the jump
        self.assertLess(goto, claim)
        self.assertIn('SUBSYSTEM=="input|hidraw", DEVPATH=="/devices/virtual/*"', rules[claim])
        give = next(l for l in rules if l.startswith('ENV{HEARTH_SEAT_CLAIM}=="hearth-p2"'))
        self.assertIn('TAG-="uaccess"', give)
        self.assertIn('ENV{HEARTH_SEAT_USER}="hearth-p2"', give)
        self.assertTrue(any(l.startswith('ENV{HEARTH_SEAT_CLAIM}=="nobody"') and 'TAG-="uaccess"' in l for l in rules))
        self.assertTrue(any('RUN+="/usr/bin/setfacl -m u:hearth-p2:rw $devnode"' in l for l in rules))
        stream = self.ha.seats_stream_rules(c)
        self.assertIn('KERNEL=="uinput|uhid", SUBSYSTEM=="misc", RUN+="/usr/bin/setfacl -m u:hearth-p2:rw $devnode"', stream)
        self.assertNotIn('bob', stream)
        # Without streamed players, none of it.
        n = self.ha.seats_rules(self.ha.seats_validate(self.nested(), 'bob'))
        self.assertNotIn('hearth-seat-input', n)


class SetupNew(AdminNewBase):
    def test_setup_stream(self):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(dict(self.stream(), start=True)))
        self.assertEqual(code, 0, lines)
        r = json.loads(lines[-1])
        self.assertEqual((r['mode'], r['restart'], r['warnings']), ('stream', False, []))
        creds_file = os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.stream.json')
        creds = json.loads(slurp(creds_file))
        self.assertEqual(creds['user'], 'hearth')
        self.assertRegex(creds['pass'], r'^[0-9a-f]{32}$')
        self.assertEqual(os.stat(creds_file).st_mode & 0o777, 0o640)
        self.assertEqual(self.chowned, [(creds_file, 'bob', 'hearth-p2')])
        self.assertIn('uinput', slurp(self.ha.SEATS_MODULES))
        self.assertIn('u:hearth-p2:rw', slurp(self.ha.SEATS_STREAM_RULES))
        cmds = self.runner.cmds()
        self.assertIn('systemctl enable --now hearth-seat@hearth-p2.service', cmds)
        self.assertIn('modprobe uinput', cmds)
        fw = next(c for c in cmds if c.startswith('firewall-cmd --permanent --add-port'))
        self.assertIn('--add-port=48989/tcp', fw)
        self.assertIn('--add-port=48998/udp', fw)
        cfg = json.loads(slurp(self.ha.SEATS_CONFIG))
        self.assertEqual(cfg['firewall_ports'][1], '48989/tcp')
        self.assertIn(creds_file, cfg['files'])
        self.assertIn(self.ha.SEATS_STREAM_RULES, cfg['files'])

    def test_setup_split(self):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(dict(self.split(), start=True)))
        self.assertEqual(code, 0, lines)
        cfg = json.loads(slurp(self.ha.SEATS_CONFIG))
        self.assertEqual((cfg['mode'], cfg['split']['layout'], cfg['units']), ('split', 'columns', ['hearth-seat@hearth-p2.path']))
        self.assertFalse(os.path.exists(self.ha.SEATS_STREAM_RULES))
        self.assertFalse(any(c.startswith('firewall-cmd') for c in self.runner.cmds()))
        self.assertIn('HEARTH_SEAT_WINDOWED=1', slurp(os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.env')))

    def test_stream_failure_undoes_everything(self):
        self.runner.fail_on = 'systemctl enable'
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.stream()))
        self.assertEqual(code, 1)
        self.assertIn('Nothing was changed', lines[-1])
        for f in (self.ha.SEATS_STREAM_RULES, self.ha.SEATS_MODULES, self.ha.SEATS_RULES, self.ha.SEATS_CONFIG,
                  os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.stream.json'), os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.env')):
            self.assertFalse(os.path.exists(f), f)
        cmds = self.runner.cmds()
        self.assertTrue(any(c.startswith('firewall-cmd --permanent --remove-port=48984/tcp') for c in cmds), cmds)
        self.assertIn('userdel -r hearth-p2', cmds)

    def test_split_failure_undoes_everything(self):
        self.runner.fail_on = 'systemd-tmpfiles'
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.split()))
        self.assertEqual(code, 1)
        for f in (self.ha.SEATS_RULES, self.ha.SEATS_TMPFILES, self.ha.SEATS_CONFIG, os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.env')):
            self.assertFalse(os.path.exists(f), f)

    def test_no_firewall_is_fine(self):
        orig = self.runner.admin_run

        def run(cmd, input=None):
            if cmd[:2] == ['firewall-cmd', '--state']:
                self.runner.calls.append((list(cmd), input))
                return 252, 'not running'
            return orig(cmd, input)
        self.ha.seats_run = run
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.stream()))
        self.assertEqual(code, 0, lines)
        self.assertEqual(json.loads(slurp(self.ha.SEATS_CONFIG))['firewall_ports'], [])
        self.assertFalse(any(c.startswith('firewall-cmd --add') for c in self.runner.cmds()))

    def test_remove_restores_everything(self):
        code, lines = self.run_cmd(self.ha.seats_setup, json.dumps(self.stream()))
        self.assertEqual(code, 0, lines)
        write(self.ha.SEATS_PASSWD_FILE, slurp(self.ha.SEATS_PASSWD_FILE) + 'hearth-p2:x:1001:1001::/var/home/hearth-p2:/bin/bash\n')
        write(os.path.join(self.ha.SEATS_DEV, 'uinput'))
        write(self.ha.SEATS_INPUT_STATE, '{}')
        self.runner.calls.clear()
        code, lines = self.run_cmd(self.ha.seats_remove, '', False)
        self.assertEqual(code, 0, lines)
        for f in (self.ha.SEATS_STREAM_RULES, self.ha.SEATS_MODULES, self.ha.SEATS_INPUT_STATE, self.ha.SEATS_CONFIG,
                  os.path.join(self.ha.SEATS_ENV_DIR, 'hearth-p2.stream.json')):
            self.assertFalse(os.path.exists(f), f)
        cmds = self.runner.cmds()
        self.assertTrue(any(c.startswith('firewall-cmd --permanent --remove-port=48984/tcp') for c in cmds))
        self.assertIn('setfacl -x u:hearth-p2 %s' % os.path.join(self.ha.SEATS_DEV, 'uinput'), cmds)
        self.assertIn('systemctl disable --now hearth-seat@hearth-p2.service', cmds)

    def test_acl_refuses_a_streamed_player(self):
        r = self.stream()
        r['seats'].append({'seat': 3, 'user': 'hearth-p3', 'mode': 'stream'})
        self.run_cmd(self.ha.seats_setup, json.dumps(r))
        code, lines = self.run_cmd(self.ha.seats_acl, json.dumps({'pads': {'usb:1-2': 2}}))
        self.assertEqual(code, 1)
        self.assertIn('streamed', lines[-1])

    def test_old_config_still_removes_and_starts(self):
        write(self.ha.SEATS_CONFIG, json.dumps({'version': 1, 'mode': 'nested', 'main_user': 'bob', 'seats': [
            {'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1', 'pads': []},
            {'seat': 2, 'user': 'hearth-p2', 'output': 'card0-DP-1', 'pads': []}]}))
        code, lines = self.run_cmd(self.ha.seats_start_stop, '', True)
        self.assertEqual(code, 0, lines)
        self.assertIn('systemctl enable --now hearth-seat@hearth-p2.path', self.runner.cmds())
        code, lines = self.run_cmd(self.ha.seats_remove, '', False)
        self.assertEqual(code, 0, lines)
        self.assertFalse(json.loads(lines[-1])['restart'])


# ---------------------------------------------------------------------------------------------- user side
class SplitAndStreamUser(SeatsBase):
    def test_normalize_layout(self):
        n = self.hs.normalize_layout({'mode': 'nested', 'seats': [
            {'seat': 1, 'user': 'bob', 'output': 'card0-HDMI-A-1', 'region': 0},
            {'seat': 2, 'user': 'hearth-p2', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 1},
            {'seat': 3, 'user': 'hearth-p3', 'mode': 'stream', 'output': 'card0-DP-1', 'pads': ['usb:1-2'], 'region': 2}],
            'split': {'layout': 'grid'}})
        self.assertEqual(n['mode'], 'split')
        self.assertEqual([s['mode'] for s in n['seats']], ['split', 'split', 'stream'])
        self.assertEqual((n['seats'][2]['output'], n['seats'][2]['pads'], 'region' in n['seats'][2]), (None, [], False))
        only = self.hs.normalize_layout({'seats': [{'seat': 1, 'user': 'bob', 'mode': 'nested', 'output': 'card0-HDMI-A-1'},
                                                   {'seat': 2, 'user': 'hearth-p2', 'mode': 'stream'}], 'split': {'layout': 'rows'}})
        self.assertEqual((only['mode'], only['seats'][0]['mode'], 'split' in only), ('stream', 'local', False))

    def test_check_layout(self):
        standard_sys(self.hs.SYS)
        info = self.hs.scan()
        lay = {'mode': 'split', 'split': {'layout': 'columns'}, 'seats': [
            {'seat': 1, 'user': 'bob', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 0},
            {'seat': 2, 'user': 'hearth-p2', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 1}]}
        self.hs.check_layout(lay, info)
        lay['seats'][1]['region'] = 0
        with self.assertRaises(self.hs.Fail):
            self.hs.check_layout(lay, info)
        self.hs.stream_support = lambda: (False, 'Streaming a player needs Sunshine')
        st = {'mode': 'stream', 'seats': [{'seat': 1, 'user': 'bob', 'mode': 'local'}, {'seat': 2, 'user': 'hearth-p2', 'mode': 'stream'}]}
        with self.assertRaises(self.hs.Fail) as e:
            self.hs.check_layout(st, info)
        self.assertIn('Sunshine', str(e.exception))
        self.hs.stream_support = lambda: (True, None)
        self.hs.check_layout(st, info)

    def test_split_rects_match_admin(self):
        ha = load('hearth-admin')
        for lay in ('columns', 'rows', 'grid'):
            self.assertEqual([tuple(r.values()) for r in self.hs.split_rects(2560, 1440, lay)], ha.seats_split_rects(2560, 1440, lay))
            self.assertEqual(len(self.hs.SPLIT_REGIONS[lay]), self.hs.SPLIT_LAYOUTS[lay])

    def test_host_rc_xml_split_regions(self):
        seats = [{'seat': 1, 'kind': 'split', 'region': 0, 'connector': 'HDMI-A-1'},
                 {'seat': 2, 'kind': 'split', 'region': 3, 'connector': 'HDMI-A-1'},
                 {'seat': 3, 'kind': 'nested', 'connector': 'DP-1'}]
        xml = self.hs.host_rc_xml(seats, 'grid')
        self.assertIn('<region name="hearth-seat1" x="0%" y="0%" width="50%" height="50%" />', xml)
        self.assertIn('<region name="hearth-seat2" x="50%" y="50%" width="50%" height="50%" />', xml)
        rule2 = xml[xml.index('sandboxAppId="seat2"'):xml.index('sandboxAppId="seat3"')]
        self.assertLess(rule2.index('MoveToOutput'), rule2.index('name="SnapToRegion" region="hearth-seat2"'))
        self.assertIn('serverDecoration="no"', rule2)
        self.assertNotIn('Focus', rule2)                    # player 1 keeps focus on a split screen
        rule3 = xml[xml.index('sandboxAppId="seat3"'):]
        self.assertNotIn('SnapToRegion', rule3)
        self.assertIn('output="DP-1"', rule3)
        self.assertNotIn('<regions>', self.hs.host_rc_xml([{'seat': 1, 'connector': 'HDMI-A-1'}, {'seat': 2, 'connector': 'DP-1'}]))
        # Without labwc's sandbox rules: no title bars on any window, and each placed in the free part.
        self.assertIn('<windowRule identifier="*" serverDecoration="no" skipTaskbar="yes" />', xml)
        self.assertLess(xml.index('identifier="*"'), xml.index('sandboxAppId="seat1"'))
        self.assertIn('<placement><policy>automatic</policy></placement>', xml)

    def test_player1_cmd(self):
        cmd = ['gamescope', '-f', '--mangoapp', '-W', '1280', '--', '/usr/libexec/nimbus/nimbus-watchdog', '/usr/bin/nimbus', '-f']
        out = self.hs.player1_cmd(cmd, {'kind': 'split', 'size': '1920x2160'})
        self.assertEqual(out, ['gamescope', '--mangoapp', '-W', '1280', '-W', '1920', '-H', '2160', '-w', '1920', '-h', '2160',
                               '--', '/usr/libexec/nimbus/nimbus-watchdog', '/usr/bin/nimbus', '-f'])   # the app's own args untouched
        self.assertEqual(self.hs.player1_cmd(cmd, {'kind': 'nested', 'size': '1920x1080'}), cmd)
        self.assertEqual(self.hs.player1_cmd(['gamescope', '-f', '--', 'x'], {'kind': 'split', 'size': '$(evil)'}), ['gamescope', '--', 'x'])

    def split_config(self, **kw):
        cfg = {'version': 2, 'enabled': True, 'mode': 'split', 'main_user': 'bob', 'split': {'output': 'card0-HDMI-A-1', 'layout': 'columns'},
               'seats': [{'seat': 1, 'user': 'bob', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 0, 'size': '1920x2160', 'pads': []},
                         {'seat': 2, 'user': 'hearth-p2', 'mode': 'split', 'output': 'card0-HDMI-A-1', 'region': 1, 'size': '1920x2160', 'pads': []},
                         {'seat': 3, 'user': 'hearth-p3', 'mode': 'stream', 'output': None, 'pads': [],
                          'stream': {'size': '1280x720', 'fps': 60, 'port': 49989, 'name': 'bob-pc Player 3'}}]}
        cfg.update(kw)
        write(self.hs.CONFIG, json.dumps(cfg))
        return cfg

    def test_host_check_split_on_one_screen(self):
        fs = FakeSys(self.hs.SYS)
        fs.gpu('card0', '0000:03:00.0', outputs=[('HDMI-A-1', True, '3840x2160', 'LG TV')])
        cfg = self.split_config()
        os.environ.pop('HEARTH_SEAT', None)
        self.assertTrue(self.hs.host_check())
        seats = self.hs.host_seats(cfg)
        self.assertEqual([(s['seat'], s['kind']) for s in seats], [(1, 'split'), (2, 'split')])   # streamed player isn't hosted
        self.assertTrue(self.hs.hosted(cfg))
        self.assertFalse(self.hs.hosted({'mode': 'stream', 'seats': [{'seat': 1}, {'seat': 2, 'mode': 'stream'}]}))

    def test_status_per_seat_modes(self):
        self.split_config()
        st = self.hs.status()
        self.assertEqual([s['mode'] for s in st['seats']], ['split', 'split', 'stream'])
        self.assertEqual(st['seats'][1]['region'], 1)
        self.assertEqual(st['seats'][2]['stream']['port'], 49989)
        self.assertIsNotNone(st['host'])
        self.assertEqual(st['audio'], {'volumes': {}, 'only_p1': False})

    def test_session_gamescope_args(self):
        self.assertEqual(self.hs.session_gamescope_args({'HEARTH_SEAT_MODE': 'split', 'HEARTH_SEAT_SIZE': '1920x2160'}),
                         ['-W', '1920', '-H', '2160', '-w', '1920', '-h', '2160'])
        env = {'HEARTH_SEAT_MODE': 'stream', 'HEARTH_SEAT_STREAM_SIZE': '1280x720', 'HEARTH_SEAT_STREAM_FPS': '120',
               'HEARTH_SEAT_VK_DEVICE': '1002:73bf'}
        self.assertEqual(self.hs.session_gamescope_args(env),
                         ['-W', '1280', '-H', '720', '-w', '1280', '-h', '720', '-r', '120', '--prefer-vk-device', '1002:73bf'])
        bad = {'HEARTH_SEAT_MODE': 'stream', 'HEARTH_SEAT_STREAM_SIZE': '999x1; rm', 'HEARTH_SEAT_STREAM_FPS': '61',
               'HEARTH_SEAT_VK_DEVICE': '--evil'}
        self.assertEqual(self.hs.session_gamescope_args(bad), ['-W', '1920', '-H', '1080', '-w', '1920', '-h', '1080', '-r', '60'])

    def test_virtual_screen_env(self):
        env = self.hs.stream_labwc_env({'HEARTH_SEAT_RENDER': '/dev/dri/renderD129', 'WAYLAND_DISPLAY': 'wayland-0', 'DISPLAY': ':0'})
        self.assertEqual((env['WLR_BACKENDS'], env['WLR_HEADLESS_OUTPUTS'], env['WLR_RENDER_DRM_DEVICE']), ('headless', '1', '/dev/dri/renderD129'))
        self.assertNotIn('WAYLAND_DISPLAY', env)
        self.assertNotIn('DISPLAY', env)
        self.assertNotIn('WLR_RENDER_DRM_DEVICE', self.hs.stream_labwc_env({'HEARTH_SEAT_RENDER': '/etc/passwd'}))

    def test_stream_run_sizes_screen_and_starts_sunshine(self):
        self.hs.main_user = lambda: {'name': 'hearth-p3', 'uid': 1003}
        envfile = os.path.join(self.root, 'p3.env')
        write(envfile, 'HEARTH_SEAT_MODE=stream\nHEARTH_SEAT_STREAM_SIZE=1280x720\nHEARTH_SEAT_STREAM_FPS=60\n')
        self.hs.seat_env_file = lambda u: envfile
        self.runner.answers = {'wlr-randr': (0, 'HEADLESS-1 "Headless output 1"\n  Enabled: yes\n', '')}
        calls = []
        with mock.patch.dict(os.environ, {'XDG_RUNTIME_DIR': self.root, 'WAYLAND_DISPLAY': 'wayland-1'}), \
                mock.patch.object(self.hs.subprocess, 'call', lambda cmd, env=None: calls.append((cmd, env)) or 0):
            self.assertEqual(self.hs.stream_run(), 0)
        cmds = self.runner.cmds()
        self.assertIn('wlr-randr --output HEADLESS-1 --custom-mode 1280x720@60Hz', cmds)
        self.assertIn('systemctl --user restart hearth-seat-sunshine.service', cmds)
        self.assertEqual(cmds[-2:], ['systemctl --user stop hearth-seat-sunshine.service', 'labwc --exit'])
        self.assertEqual(slurp(os.path.join(self.root, 'hearth-seat-stream', 'stream.env')), 'WAYLAND_DISPLAY=%s\n' % os.path.join(self.root, 'wayland-1'))
        cmd, env = calls[0]
        self.assertEqual(cmd, ['/usr/bin/nimbus-session'])
        self.assertEqual((env['HEARTH_SEAT'], env['NIMBUS_START_MODE']), ('3', 'game'))
        self.assertEqual(env['NIMBUS_GAMESCOPE_ARGS'], '-W 1280 -H 720 -w 1280 -h 720 -r 60')

    def test_sunshine_conf(self):
        env = {'HEARTH_SEAT_STREAM_PORT': '49989', 'HEARTH_SEAT_STREAM_NAME': 'bob-pc Player 3', 'HEARTH_SEAT_RENDER': '/dev/dri/renderD128'}
        conf = self.hs.sunshine_conf(env, '/var/home/hearth-p3/.config/hearth/seat-stream')
        lines = conf.splitlines()
        for want in ('sunshine_name = bob-pc Player 3', 'port = 49989', 'capture = wlr', 'output_name = 0',
                     'origin_web_ui_allowed = pc', 'upnp = disabled', 'keyboard = disabled', 'mouse = disabled',
                     'file_state = /var/home/hearth-p3/.config/hearth/seat-stream/sunshine_state.json',
                     'cert = /var/home/hearth-p3/.config/hearth/seat-stream/credentials/cacert.pem',
                     'adapter_name = /dev/dri/renderD128'):
            self.assertIn(want, lines)
        with self.assertRaises(self.hs.Fail):
            self.hs.sunshine_conf(dict(env, HEARTH_SEAT_STREAM_PORT='47989'), '/x')     # never Remote play's port
        odd = self.hs.sunshine_conf(dict(env, HEARTH_SEAT_STREAM_NAME='x\ncapture = kms', HEARTH_SEAT='3'), '/x')
        self.assertNotIn('capture = kms', odd)
        self.assertIn('sunshine_name = Hearth Player 3', odd)
        apps = json.loads(self.hs.sunshine_apps())
        self.assertEqual([a['name'] for a in apps['apps']], ['Hearth'])

    def pair_setup(self, answers):
        self.split_config()
        self.hs.stream_creds = lambda u: ('hearth', 'secret') if u == 'hearth-p3' else None
        calls = []

        def api(port, method, path, body=None, creds=None, timeout=10):
            calls.append((port, method, path, body, creds))
            return answers[(method, path)]
        self.hs.sunshine_api = api
        return calls

    def test_pair_new_sunshine(self):
        calls = self.pair_setup({('GET', '/api/pin'): (200, {'pairings': [{'id': 'ab' * 16, 'name': 'TV'}]}),
                                 ('POST', '/api/pin'): (200, {'status': True})})
        self.assertEqual(self.hs.stream_pair(3, '1234', 'Living room'), {'ok': True, 'seat': 3})
        self.assertEqual(calls[-1], (49989, 'POST', '/api/pin', {'pin': '1234', 'name': 'Living room', 'pairing_id': 'ab' * 16}, ('hearth', 'secret')))

    def test_pair_old_sunshine_and_failures(self):
        calls = self.pair_setup({('GET', '/api/pin'): (404, {}), ('POST', '/api/pin'): (200, {'status': 'true'})})
        self.hs.stream_pair(3, '0042')
        self.assertEqual(calls[-1][3], {'pin': '0042', 'name': 'Player 3\'s device'})
        for pin in ('123', '12345', 'abcd', None, '12 4'):
            with self.assertRaises(self.hs.Fail):
                self.hs.stream_pair(3, pin)
        with self.assertRaises(self.hs.Fail):
            self.hs.stream_pair(2, '1234')                   # player 2 isn't streamed
        self.pair_setup({('GET', '/api/pin'): (200, {'pairings': []}), ('POST', '/api/pin'): (200, {'status': True})})
        with self.assertRaises(self.hs.Fail) as e:
            self.hs.stream_pair(3, '1234')
        self.assertIn('Moonlight first', str(e.exception))
        self.pair_setup({('GET', '/api/pin'): (404, {}), ('POST', '/api/pin'): (200, {'status': False})})
        with self.assertRaises(self.hs.Fail) as e:
            self.hs.stream_pair(3, '1234')
        self.assertIn('didn\'t work', str(e.exception))
        self.pair_setup({('GET', '/api/pin'): (0, {})})
        with self.assertRaises(self.hs.Fail) as e:
            self.hs.stream_pair(3, '1234')
        self.assertIn('isn\'t running', str(e.exception))

    def test_stream_info(self):
        self.pair_setup({('GET', '/api/clients/list'): (200, {'named_certs': [{'name': 'TV', 'uuid': 'u1', 'cert': 'x'}]}),
                         ('GET', '/api/pin'): (200, {'pairings': [{'id': 'cd' * 16, 'name': 'Phone', 'address': '1.2.3.4'}]})})
        self.runner.answers = {'systemctl is-active hearth-seat@hearth-p3.service': (0, 'active\n', ''),
                               'ip -4 -o addr': (0, '2: enp3s0    inet 192.168.1.20/24 brd 192.168.1.255 scope global enp3s0\n'
                                                    '3: docker0    inet 172.17.0.1/16 scope global docker0\n', '')}
        self.hs.have = lambda c: False
        info = self.hs.stream_info()['seats']
        self.assertEqual(len(info), 1)
        e = info[0]
        self.assertEqual((e['seat'], e['port'], e['running'], e['reachable']), (3, 49989, True, True))
        self.assertEqual(e['clients'], [{'name': 'TV', 'uuid': 'u1'}])
        self.assertEqual(e['pending'], [{'id': 'cd' * 16, 'name': 'Phone'}])
        self.assertEqual(e['addresses'], ['192.168.1.20:49989'])

    def test_qr_is_only_plain_svg(self):
        self.hs.have = lambda c: True
        self.runner.answers = {'qrencode': (0, '<?xml version="1.0"?>\n<svg width="10"><rect/></svg>\n', '')}
        self.assertEqual(self.hs.qr_svg('1.2.3.4:48989'), '<svg width="10"><rect/></svg>\n')
        self.runner.answers = {'qrencode': (0, '<svg onload="x()"></svg>', '')}
        self.assertIsNone(self.hs.qr_svg('x'))

    def test_assign_pad_refuses_streamed(self):
        self.split_config()
        with self.assertRaises(self.hs.Fail):
            self.hs.assign_pad('usb:1-2', 3)

    def test_split_audio(self):
        self.hs.AUDIO_PREFS = os.path.join(self.root, 'audio.json')
        write(self.hs.AUDIO_PREFS, json.dumps({'volumes': {'2': 40, '3': 'loud', '9': 5}, 'only_p1': True}))
        self.assertEqual(self.hs.audio_prefs(), {'volumes': {'2': 40}, 'only_p1': True})
        n = iter(range(41, 50))
        self.runner.answers = {'pactl load-module': None}
        orig = self.runner.run

        def run(cmd, input=None, timeout=60):
            if cmd[:2] == ['pactl', 'load-module']:
                self.runner.calls.append((list(cmd), input))
                return 0, '%d\n' % next(n), ''
            if cmd[:3] == ['pactl', '-f', 'json']:
                return 0, json.dumps([{'name': 'alsa_output.hdmi-stereo', 'description': 'HDMI', 'ports': [{'description': 'HDMI (LG TV)'}]}]), ''
            return orig(cmd, input, timeout)
        self.runner.run = run
        os.makedirs(os.path.join(self.root, 'hearth-seat-host'))
        os.makedirs(os.path.join(self.hs.SEAT_RUN, 'hearth-p2'))
        seats = [{'seat': 1, 'user': 'bob', 'kind': 'split', 'monitor': 'LG TV'},
                 {'seat': 2, 'user': 'hearth-p2', 'kind': 'split', 'monitor': 'LG TV'}]
        with mock.patch.dict(os.environ, {'XDG_RUNTIME_DIR': self.root}):
            self.hs.host_audio(seats, 1000)
            cmds = self.runner.cmds()
            self.assertIn('pactl load-module module-remap-sink sink_name=hearth-seat2 master=alsa_output.hdmi-stereo '
                          'sink_properties=device.description=Hearth-Player-2', cmds)
            self.assertIn('pactl set-sink-volume hearth-seat2 40%', cmds)
            self.assertIn('pactl set-sink-mute hearth-seat2 1', cmds)      # only player 1's sound
            self.assertIn('pactl set-sink-mute hearth-seat1 0', cmds)
            self.assertIn('pactl set-default-sink hearth-seat1', cmds)
            self.assertIn('PULSE_SINK=hearth-seat2', slurp(os.path.join(self.hs.SEAT_RUN, 'hearth-p2', 'audio.env')))
            self.runner.calls.clear()
            self.hs.revoke_audio(seats, 1000)
            cmds = self.runner.cmds()
            self.assertIn('pactl unload-module 41', cmds)
            self.assertIn('pactl unload-module 42', cmds)
            self.assertIn('pactl set-default-sink alsa_output.hdmi-stereo', cmds)

    def test_audio_cmd(self):
        self.hs.AUDIO_PREFS = os.path.join(self.root, 'audio.json')
        self.split_config()
        r = self.hs.audio_cmd(2, 75)
        self.assertEqual(r['volumes'], {'2': 75})
        self.assertTrue(self.hs.audio_cmd(only_p1=True)['only_p1'])
        for bad in ((3, 50), (2, 101), (2, None), (7, 10)):
            with self.assertRaises(self.hs.Fail):
                self.hs.audio_cmd(*bad)

    def test_wait_button(self):
        self.hs.DEV = os.path.join(self.root, 'dev')
        standard_sys(self.hs.SYS)
        os.makedirs(os.path.join(self.hs.DEV, 'input'))
        for ev in ('event1', 'event2'):
            os.mkfifo(os.path.join(self.hs.DEV, 'input', ev))

        def press():
            fd = os.open(os.path.join(self.hs.DEV, 'input', 'event2'), os.O_WRONLY)
            os.write(fd, self.hs.INPUT_EVENT.pack(0, 0, 3, 0, 5) + self.hs.INPUT_EVENT.pack(0, 0, self.hs.EV_KEY, 0x130, 1))
            os.close(fd)
        t = threading.Timer(0.2, press)
        t.start()
        try:
            pad = self.hs.wait_button(5, exclude=['usb:1-2'])
        finally:
            t.join()
        self.assertEqual(pad['id'], 'uniq:aa:bb:cc:dd:ee:ff')
        self.assertIsNone(self.hs.wait_button(0.3, exclude=['usb:1-2']))
        with self.assertRaises(self.hs.Fail):
            self.hs.wait_button(0.1, exclude=['usb:1-2', 'uniq:aa:bb:cc:dd:ee:ff'])


# ---------------------------------------------------------------------------------------------- hearth-seat-input
class SeatInput(unittest.TestCase):
    def setUp(self):
        self.si = load('hearth-seat-input')
        self.users = {1002: 'hearth-p2', 1003: 'hearth-p3'}

    def test_device_key(self):
        k = self.si.device_key
        self.assertEqual(k('/devices/virtual/misc/uhid/0003:054C:0CE6.0007/hidraw/hidraw9'), 'uhid:0003:054C:0CE6.0007')
        self.assertEqual(k('/devices/virtual/misc/uhid/0003:054C:0CE6.0007/input/input40/event20'), 'uhid:0003:054C:0CE6.0007')
        self.assertEqual(k('/devices/virtual/input/input31/event18'), 'uinput:input31')
        self.assertEqual(k('/devices/virtual/input/input31'), 'uinput:input31')
        self.assertIsNone(k('/devices/pci0000:00/0000:00:14.0/usb1/1-2/input/input5/event5'))

    def test_new_file_claims_and_shared_key(self):
        st = {}
        cur = [('100:5:7', 1002)]
        self.assertEqual(self.si.decide(st, 'uhid:A', cur, self.users, 10.0), 'hearth-p2')
        # The same controller's other nodes: same answer, no new file needed.
        self.assertEqual(self.si.decide(st, 'uhid:A', cur, self.users, 10.1), 'hearth-p2')
        # Player 1's Steam Input device a minute later: nobody streamed made it.
        self.assertIsNone(self.si.decide(st, 'uinput:input50', cur, self.users, 70.0))
        self.assertNotIn('uinput:input50', st['claims'])

    def test_two_at_once_and_credit(self):
        st = {}
        # Player 3 made two devices before udev got to the first: both files are new at the first event.
        cur = [('200:9:5', 1003), ('200:9:6', 1003)]
        self.assertEqual(self.si.decide(st, 'uinput:input60', cur, self.users, 5.0), 'hearth-p3')
        self.assertEqual(self.si.decide(st, 'uinput:input61', cur, self.users, 5.2), 'hearth-p3')
        self.assertIsNone(self.si.decide(st, 'uinput:input62', cur, self.users, 5.3))   # credit used up
        # Two players at the same moment: nobody gets it.
        cur2 = cur + [('300:1:4', 1002), ('400:1:4', 1003)]
        self.assertEqual(self.si.decide(st, 'uinput:input63', cur2, self.users, 9.0), 'nobody')
        # Credits expire.
        cur3 = cur2 + [('300:1:9', 1002)]
        self.si.decide(st, 'uinput:x', cur3, self.users, 20.0)
        self.assertIsNone(self.si.decide(st, 'uinput:late', cur3, self.users, 40.0))

    def test_forget_and_reused_fd(self):
        st = {}
        self.si.decide(st, 'uinput:input70', [('100:5:7', 1002)], self.users, 1.0)
        self.si.forget(st, 'uinput:input70', [])
        self.assertNotIn('uinput:input70', st['claims'])
        self.assertEqual(st['seen'], [])
        # The same fd number opened again later is new again.
        self.assertEqual(self.si.decide(st, 'uinput:input71', [('100:5:7', 1002)], self.users, 50.0), 'hearth-p2')

    def test_open_files_and_main(self):
        with tempfile.TemporaryDirectory() as root:
            proc = os.path.join(root, 'proc')
            for pid, target in (('100', '/dev/uinput'), ('101', '/dev/null'), ('102', '/dev/uhid')):
                os.makedirs(os.path.join(proc, pid, 'fd'))
                write(os.path.join(proc, pid, 'stat'), '%s (sunshine) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 777 20\n' % pid)
                os.symlink(target, os.path.join(proc, pid, 'fd', '9'))
            os.makedirs(os.path.join(proc, 'self'))
            uid = os.getuid()
            found = self.si.open_files({uid}, proc)
            self.assertEqual(sorted(found), [('100:777:9', uid), ('102:777:9', uid)])
            self.assertEqual(self.si.open_files({uid + 1}, proc), [])
            # The whole thing, as udev runs it.
            self.si.CONFIG = os.path.join(root, 'seats.json')
            self.si.STATE = os.path.join(root, 'udev', 'state.json')
            self.si.PROC = proc
            write(self.si.CONFIG, json.dumps({'mode': 'split', 'seats': [{'seat': 1, 'user': 'bob'}, {'seat': 2, 'user': 'hearth-p2', 'mode': 'stream'}]}))
            self.si.stream_users = lambda c: {uid: 'hearth-p2'}
            self.si.open_files = lambda uids, proc=proc: [('100:777:9', uid)]
            out = io.StringIO()
            with mock.patch.dict(os.environ, {'DEVPATH': '/devices/virtual/input/input80/event30'}), mock.patch('sys.stdout', out):
                self.assertEqual(self.si.main(['hearth-seat-input', 'claim']), 0)
            self.assertEqual(out.getvalue(), 'HEARTH_SEAT_CLAIM=hearth-p2\n')
            out = io.StringIO()
            with mock.patch.dict(os.environ, {'DEVPATH': '/devices/pci0000:00/x/input/input5'}), mock.patch('sys.stdout', out):
                self.si.main(['hearth-seat-input', 'claim'])
            self.assertEqual(out.getvalue(), '')               # not a virtual device: nothing to say
            self.assertEqual(os.stat(self.si.STATE).st_mode & 0o777, 0o600)

    def test_stream_users_only_streamed(self):
        with mock.patch.object(self.si.pwd, 'getpwnam', lambda n: types.SimpleNamespace(pw_uid={'hearth-p2': 1002, 'hearth-p3': 1003}[n])):
            u = self.si.stream_users({'mode': 'nested', 'seats': [{'seat': 1, 'user': 'bob', 'mode': 'stream'},
                                                                  {'seat': 2, 'user': 'hearth-p2', 'mode': 'stream'},
                                                                  {'seat': 3, 'user': 'hearth-p3'}]})
        self.assertEqual(u, {1002: 'hearth-p2'})


if __name__ == '__main__':
    unittest.main()
