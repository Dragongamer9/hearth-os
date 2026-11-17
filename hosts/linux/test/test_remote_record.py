"""Tests for Desktop Mode's screen recording (hearth-record, Recorder.qml) and Settings › Remote desktop (hearth-vnc,
SetRemoteDesktop.qml): a real VNC conversation over this PC's loopback with a stand-in screen (RFB 3.8, VeNCrypt with
TLS, the password, ZRLE and Raw pixels checked byte for byte, only the changed tiles sent again), wrong passwords and
the lockout, who may connect (Tailscale, home network, never the internet), pixel formats, the keyboard map, and the
owner's rules: opening the page or the panel writes nothing, a change writes only its own key, the password is never
in a file, the service and the recorder are off in the image, the bar's dot and the portal config are where the
shell expects them. Nothing here needs a desktop, a portal or GStreamer; patched globals are put back."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import socket
import ssl
import struct
import tempfile
import threading
import time
import unittest
import zlib
from contextlib import redirect_stdout

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


vnc = load(SHELL / 'hearth-vnc', 'hearth_vnc_t')
rec = load(SHELL / 'hearth-record', 'hearth_record_t')
import hearth_screencast as hs  # noqa: E402


def run_cmd(mod, argv):
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = mod.main(argv)
    line = buf.getvalue().strip().splitlines()[-1] if buf.getvalue().strip() else ''
    return rc, line


class Patch(unittest.TestCase):
    def patch(self, mod, name, value):
        old = getattr(mod, name)
        setattr(mod, name, value)
        self.addCleanup(setattr, mod, name, old)


# --------------------------------------------------------------------------------------------- a stand-in screen

class FakeCapture:
    fake = True
    stopped = False
    restart_wanted = False

    def __init__(self, w, h):
        self.size = (w, h)
        self.cond = threading.Condition()
        self.frame = self.make(w, h, 0)
        self.serial = 1

    @staticmethod
    def make(w, h, seed):
        b = bytearray(w * h * 4)
        for y in range(h):
            for x in range(w):
                i = (y * w + x) * 4
                b[i:i + 4] = bytes(((x * 7 + seed) & 255, (y * 5) & 255, ((x + y) * 3) & 255, 0))
        return bytes(b)

    def put(self, frame):
        with self.cond:
            self.frame = frame
            self.serial += 1
            self.cond.notify_all()

    def wait(self, serial, timeout):
        with self.cond:
            if self.serial == serial:
                self.cond.wait(timeout)
            return self.frame, self.serial

    def stop(self):
        pass


def recv_exact(s, n):
    b = b''
    while len(b) < n:
        c = s.recv(n - len(b))
        if not c:
            raise ConnectionError('closed')
        b += c
    return b


class Viewer:
    """Just enough of a VNC viewer: VeNCrypt TLSPlain or X509Plain, then updates."""

    def __init__(self, port, password, sub=vnc.VENC_TLSPLAIN):
        s = socket.create_connection(('127.0.0.1', port), timeout=10)
        assert recv_exact(s, 12) == b'RFB 003.008\n'
        s.sendall(b'RFB 003.008\n')
        n = recv_exact(s, 1)[0]
        if n == 0:
            ln = struct.unpack('!I', recv_exact(s, 4))[0]
            raise PermissionError(recv_exact(s, ln).decode())
        types = recv_exact(s, n)
        assert vnc.SEC_VENCRYPT in types
        s.sendall(bytes([vnc.SEC_VENCRYPT]))
        assert recv_exact(s, 2) == b'\x00\x02'
        s.sendall(b'\x00\x02')
        assert recv_exact(s, 1) == b'\x00'
        k = recv_exact(s, 1)[0]
        self.subs = list(struct.unpack('!%dI' % k, recv_exact(s, 4 * k)))
        s.sendall(struct.pack('!I', sub))
        assert recv_exact(s, 1) == b'\x01'
        c = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        c.check_hostname = False
        c.verify_mode = ssl.CERT_NONE
        if sub == vnc.VENC_TLSPLAIN:
            c.minimum_version = c.maximum_version = ssl.TLSVersion.TLSv1_2
            c.set_ciphers('aNULL:!eNULL:@SECLEVEL=0')
        s = c.wrap_socket(s)
        self.cipher = s.cipher()
        u, p = b'hearth', password.encode()
        s.sendall(struct.pack('!II', len(u), len(p)) + u + p)
        res = struct.unpack('!I', recv_exact(s, 4))[0]
        if res != 0:
            ln = struct.unpack('!I', recv_exact(s, 4))[0]
            raise PermissionError(recv_exact(s, ln).decode())
        s.sendall(b'\x01')
        self.w, self.h = struct.unpack('!HH', recv_exact(s, 4))
        self.pf = recv_exact(s, 16)
        ln = struct.unpack('!I', recv_exact(s, 4))[0]
        self.name = recv_exact(s, ln).decode()
        self.s = s
        self.z = zlib.decompressobj()
        self.fb = bytearray(self.w * self.h * 4)

    def encodings(self, encs):
        self.s.sendall(struct.pack('!BxH', 2, len(encs)) + b''.join(struct.pack('!i', e) for e in encs))

    def request(self, inc, x=0, y=0, w=None, h=None):
        self.s.sendall(struct.pack('!BBHHHH', 3, inc, x, y, w or self.w, h or self.h))

    def update(self):
        """Reads one FramebufferUpdate into self.fb (BGRx); returns its rectangles."""
        s = self.s
        t = recv_exact(s, 1)[0]
        assert t == 0, t
        n = struct.unpack('!xH', recv_exact(s, 3))[0]
        rects = []
        for _ in range(n):
            x, y, w, h, enc = struct.unpack('!HHHHi', recv_exact(s, 12))
            rects.append((x, y, w, h, enc))
            if enc == vnc.ENC_RAW:
                data = recv_exact(s, w * h * 4)
                for row in range(h):
                    i = ((y + row) * self.w + x) * 4
                    self.fb[i:i + w * 4] = data[row * w * 4:(row + 1) * w * 4]
            elif enc == vnc.ENC_ZRLE:
                ln = struct.unpack('!I', recv_exact(s, 4))[0]
                data = self.z.decompress(recv_exact(s, ln))
                pos = 0
                for ty in range(y, y + h, 64):
                    th = min(64, y + h - ty)
                    for tx in range(x, x + w, 64):
                        tw = min(64, x + w - tx)
                        sub = data[pos]
                        pos += 1
                        if sub == 1:
                            px = data[pos:pos + 3] * (tw * th)
                            pos += 3
                        else:
                            assert sub == 0, sub
                            px = data[pos:pos + tw * th * 3]
                            pos += tw * th * 3
                        for row in range(th):
                            for col in range(tw):
                                i = ((ty + row) * self.w + tx + col) * 4
                                j = (row * tw + col) * 3
                                self.fb[i:i + 3] = px[j:j + 3]
                                self.fb[i + 3] = 0
                assert pos == len(data)
        return rects

    def close(self):
        try:
            self.s.close()
        except Exception:
            pass


class VncConversation(Patch):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.patch(vnc, 'RUNTIME', os.path.join(self.d, 'run'))
        self.patch(vnc, 'CLIENTS', os.path.join(self.d, 'run', 'clients.json'))
        self.patch(vnc, 'CERT', os.path.join(self.d, 'none.pem'))
        self.patch(vnc, 'notify', lambda *a: None)
        self.patch(vnc, 'peer_name', lambda ip: '')
        self.cap = FakeCapture(150, 70)        # not a multiple of 64: edge tiles too
        self.srv = vnc.Server(frames=self.cap, password='correct horse', settings={'reach': 'tailscale', 'viewOnly': True})
        self.srv.input = 'none'
        self.srv.listen('127.0.0.1', 0)
        self.port = self.srv.sock.getsockname()[1]
        threading.Thread(target=self.srv.accept_forever, daemon=True).start()

    def tearDown(self):
        self.srv.stopping = True
        self.srv.kick('all')
        self.srv.sock.close()

    def test_zrle_then_only_the_changes(self):
        v = Viewer(self.port, 'correct horse')
        self.addCleanup(v.close)
        self.assertEqual((v.w, v.h), (150, 70))
        self.assertIn('Hearth', v.name)
        self.assertEqual(v.pf, vnc.SERVER_PF)
        self.assertIn(vnc.VENC_TLSPLAIN, v.subs)       # this PC (loopback) may use anonymous TLS
        self.assertNotIn(vnc.VENC_X509PLAIN, v.subs)   # no certificate made in this test
        v.encodings([vnc.ENC_ZRLE, vnc.ENC_RAW, vnc.ENC_DESKTOPSIZE])
        v.request(0)
        v.update()
        self.assertEqual(bytes(v.fb), self.cap.frame)
        # Who's connected, for the bar's dot and Settings.
        with open(vnc.CLIENTS) as f:
            self.assertEqual(json.load(f)['clients'][0]['ip'], '127.0.0.1')
        # One pixel changes: only its tile comes again.
        nf = bytearray(self.cap.frame)
        i = (65 * 150 + 140) * 4
        nf[i:i + 4] = b'\x01\x02\x03\x00'
        v.request(1)
        time.sleep(0.2)
        self.cap.put(bytes(nf))
        rects = v.update()
        self.assertEqual([(r[0], r[1]) for r in rects], [(128, 64)])
        self.assertEqual(bytes(v.fb), bytes(nf))

    def test_raw_and_a_solid_screen(self):
        self.cap.put(b'\x10\x20\x30\x00' * (150 * 70))
        v = Viewer(self.port, 'correct horse')
        self.addCleanup(v.close)
        v.encodings([vnc.ENC_RAW])
        v.request(0)
        rects = v.update()
        self.assertTrue(all(r[4] == vnc.ENC_RAW for r in rects))
        self.assertEqual(bytes(v.fb), self.cap.frame)

    def test_wrong_password_then_locked_out(self):
        self.patch(vnc, 'pause', lambda s: None)
        for _ in range(vnc.FAIL_LIMIT):
            with self.assertRaises(PermissionError) as e:
                Viewer(self.port, 'wrong password')
            self.assertIn('Wrong password', str(e.exception))
        with self.assertRaises(PermissionError) as e:
            Viewer(self.port, 'correct horse')
        self.assertIn('Too many wrong passwords', str(e.exception))

    def test_no_password_yet(self):
        self.srv._password = ''
        with self.assertRaises(PermissionError) as e:
            Viewer(self.port, 'anything at all')
        self.assertIn('no password yet', str(e.exception))

    def test_view_only_input_is_dropped(self):
        got = []
        self.srv.backend = type('B', (), {'key': lambda s, *a: got.append(a), 'pointer': lambda s, *a: got.append(a),
                                          'button': lambda s, *a: got.append(a), 'scroll': lambda s, *a: got.append(a)})()
        v = Viewer(self.port, 'correct horse')
        self.addCleanup(v.close)
        v.s.sendall(struct.pack('!BBxxI', 4, 1, 0x61) + struct.pack('!BBHH', 5, 1, 10, 10))
        v.request(0)
        v.update()
        self.assertEqual(got, [])


class X509(Patch):
    def test_certificate_and_fingerprint(self):
        import shutil
        if not shutil.which('openssl'):
            self.skipTest('no openssl')
        d = tempfile.mkdtemp()
        self.patch(vnc, 'CERT_DIR', d)
        self.patch(vnc, 'CERT', os.path.join(d, 'cert.pem'))
        self.patch(vnc, 'KEY', os.path.join(d, 'key.pem'))
        self.assertTrue(vnc.ensure_cert())
        self.assertEqual(os.stat(vnc.KEY).st_mode & 0o777, 0o600)
        fp = vnc.fingerprint()
        self.assertEqual(len(fp.split(':')), 32)
        self.patch(vnc, 'notify', lambda *a: None)
        cap = FakeCapture(64, 64)
        srv = vnc.Server(frames=cap, password='a good password', settings={'reach': 'tailscale'})
        srv.input = 'none'
        srv.listen('127.0.0.1', 0)
        threading.Thread(target=srv.accept_forever, daemon=True).start()
        self.addCleanup(srv.sock.close)
        v = Viewer(srv.sock.getsockname()[1], 'a good password', sub=vnc.VENC_X509PLAIN)
        self.addCleanup(v.close)
        der = v.s.getpeercert(binary_form=True)
        import hashlib
        self.assertEqual(':'.join('%02X' % b for b in hashlib.sha256(der).digest()), fp)


# --------------------------------------------------------------------------------------------- pieces

class Pieces(unittest.TestCase):
    def test_who_may_connect(self):
        a = vnc.peer_allowed
        self.assertEqual(a('100.101.102.103', 'tailscale'), 'tailscale')
        self.assertEqual(a('fd7a:115c:a1e0::1', 'tailscale'), 'tailscale')
        self.assertEqual(a('192.168.1.20', 'tailscale'), '')
        self.assertEqual(a('192.168.1.20', 'home'), 'home')
        self.assertEqual(a('::ffff:192.168.1.20', 'home'), 'home')
        self.assertEqual(a('10.0.0.5', 'home'), 'home')
        for internet in ('8.8.8.8', '1.1.1.1', '2001:4860:4860::8888', '100.128.0.1'):
            self.assertEqual(a(internet, 'home'), '', internet)
        self.assertEqual(a('127.0.0.1', 'tailscale'), 'local')
        self.assertEqual(a('not an address', 'home'), '')

    def test_pixel_formats(self):
        px = b'\x11\x22\x33\x00'            # B G R x
        self.assertEqual(bytes(vnc.PixelFormat().convert(px)), px)
        self.assertEqual(bytes(vnc.PixelFormat().convert(px, cpixel=True)), b'\x11\x22\x33')
        rgbx = struct.pack('!BBBBHHHBBB3x', 32, 24, 0, 1, 255, 255, 255, 0, 8, 16)
        self.assertEqual(bytes(vnc.PixelFormat(rgbx).convert(px)), b'\x33\x22\x11\x00')
        big = struct.pack('!BBBBHHHBBB3x', 32, 24, 1, 1, 255, 255, 255, 16, 8, 0)
        self.assertEqual(bytes(vnc.PixelFormat(big).convert(px)), b'\x00\x33\x22\x11')
        self.assertEqual(bytes(vnc.PixelFormat(big).convert(px, cpixel=True)), b'\x33\x22\x11')
        rgb565 = struct.pack('!BBBBHHHBBB3x', 16, 16, 0, 1, 31, 63, 31, 11, 5, 0)
        white = vnc.PixelFormat(rgb565).convert(b'\xff\xff\xff\x00')
        self.assertEqual(bytes(white), b'\xff\xff')

    def test_dirty_tiles_and_runs(self):
        w, h = 200, 130
        a = FakeCapture.make(w, h, 0)
        self.assertEqual(len(vnc.dirty_tiles(None, a, w, h)), 4 * 3)
        self.assertEqual(vnc.dirty_tiles(a, a, w, h), [])
        b = bytearray(a)
        b[0] ^= 1
        b[(w * 129 + 199) * 4] ^= 1
        self.assertEqual(vnc.dirty_tiles(a, bytes(b), w, h), [(0, 0, 64, 64), (192, 128, 8, 2)])
        self.assertEqual(vnc.runs([(0, 0, 64, 64), (64, 0, 64, 64), (0, 64, 64, 64)]), [(0, 0, 128, 64), (0, 64, 64, 64)])

    def test_keymap(self):
        self.assertEqual(vnc.KEYMAP[ord('a')], 'KEY_A')
        self.assertEqual(vnc.KEYMAP[ord('A')], 'KEY_A')
        self.assertEqual(vnc.KEYMAP[ord('!')], 'KEY_1')
        self.assertEqual(vnc.KEYMAP[0xff0d], 'KEY_ENTER')
        self.assertEqual(vnc.KEYMAP[0xffc9], 'KEY_F12')
        self.assertEqual(vnc.KEYMAP[0xffe1], 'KEY_LEFTSHIFT')

    def test_caps_line(self):
        line = '/GstPipeline:pipeline0/GstFdSink:fdsink0.GstPad:sink: caps = video/x-raw, format=(string)BGRx, width=(int)2560, height=(int)1440, framerate=(fraction)0/1'
        self.assertEqual(hs.caps_size(line), (2560, 1440))
        self.assertIsNone(hs.caps_size('nothing here'))

    def test_input_kinds(self):
        self.assertEqual(vnc.input_kind(True, 2)[0], 'none')
        self.assertEqual(vnc.input_kind(False, 2)[0], 'portal')


# --------------------------------------------------------------------------------------------- the owner's rules

class Settings(Patch):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.patch(vnc, 'SETTINGS', os.path.join(self.d, 'remote-vnc.json'))
        self.patch(vnc, 'CLIENTS', os.path.join(self.d, 'clients.json'))
        self.patch(vnc, 'PORTAL_STATE', os.path.join(self.d, 'portal.json'))
        self.patch(vnc, 'CERT', os.path.join(self.d, 'cert.pem'))
        self.patch(rec, 'SETTINGS', os.path.join(self.d, 'record.json'))
        self.patch(rec, 'STATE', os.path.join(self.d, 'rec-state.json'))
        self.calls = []

        def fake_run(cmd, timeout=10, inp=None):
            self.calls.append((list(cmd), inp))
            if cmd[:2] == ['secret-tool', 'lookup']:
                return (1, '', '')
            return (0, '', '')
        self.patch(vnc, 'run', fake_run)
        self.patch(vnc, 'control', lambda msg, timeout=5: None)
        self.patch(vnc, 'ensure_cert', lambda: True)

    def files(self):
        return sorted(os.listdir(self.d))

    def test_status_writes_nothing(self):
        rc, line = run_cmd(vnc, ['status'])
        self.assertEqual(rc, 0, line)
        st = json.loads(line)
        self.assertFalse(st['on'])
        self.assertEqual(st['reach'], 'tailscale')
        rc, line = run_cmd(rec, ['status'])
        self.assertEqual(rc, 0, line)
        self.assertEqual(json.loads(line)['choices']['sound'], 'desktop')
        self.assertEqual(self.files(), [])

    def test_a_change_writes_only_its_key(self):
        with open(vnc.SETTINGS, 'w') as f:
            json.dump({'reach': 'home', 'someoneElse': 1}, f)
        rc, line = run_cmd(vnc, ['set', 'viewOnly', 'on'])
        self.assertEqual(rc, 0, line)
        with open(vnc.SETTINGS) as f:
            self.assertEqual(json.load(f), {'reach': 'home', 'someoneElse': 1, 'viewOnly': True})
        rc, line = run_cmd(rec, ['set', 'fps', '60'])
        self.assertEqual(rc, 0, line)
        with open(rec.SETTINGS) as f:
            self.assertEqual(json.load(f), {'fps': 60})
        # The same value again: not written again.
        m = os.path.getmtime(rec.SETTINGS)
        time.sleep(0.02)
        run_cmd(rec, ['set', 'fps', '60'])
        self.assertEqual(os.path.getmtime(rec.SETTINGS), m)
        self.assertEqual(run_cmd(rec, ['set', 'fps', '45'])[0], 1)
        self.assertEqual(run_cmd(vnc, ['set', 'reach', 'internet'])[0], 1)

    def test_on_starts_the_service_and_off_stops_it(self):
        run_cmd(vnc, ['set', 'on', 'on'])
        self.assertIn(['systemctl', '--user', 'start', 'hearth-vnc.service'], [c for c, _ in self.calls])
        run_cmd(vnc, ['set', 'on', 'off'])
        self.assertIn(['systemctl', '--user', 'stop', 'hearth-vnc.service'], [c for c, _ in self.calls])
        with open(vnc.SETTINGS) as f:
            self.assertEqual(json.load(f), {'on': False})

    def test_password_goes_to_the_keyring_only(self):
        os.environ['HEARTH_INPUT'] = 'short'
        self.addCleanup(os.environ.pop, 'HEARTH_INPUT', None)
        self.assertEqual(run_cmd(vnc, ['password'])[0], 1)
        os.environ['HEARTH_INPUT'] = 'a long enough secret'
        self.patch(vnc.shutil, 'which', lambda n: '/usr/bin/' + n)
        rc, line = run_cmd(vnc, ['password'])
        self.assertEqual(rc, 0, line)
        cmd, inp = self.calls[-1]
        self.assertEqual(cmd[:2], ['secret-tool', 'store'])
        self.assertEqual(inp, 'a long enough secret')
        self.assertNotIn('a long enough secret', ' '.join(cmd))
        self.assertEqual(self.files(), [])

    def test_session_does_nothing_when_off(self):
        self.assertEqual(run_cmd(vnc, ['session'])[0], 0)
        self.assertFalse([c for c, _ in self.calls if c[:3] == ['systemctl', '--user', 'start']])


class Wiring(unittest.TestCase):
    def read(self, *p):
        return (ROOT.joinpath(*p)).read_text(encoding='utf-8')

    def test_service_is_off_in_the_image(self):
        unit = self.read('os', 'system_files', 'usr', 'lib', 'systemd', 'user', 'hearth-vnc.service')
        self.assertIn('hearth-vnc serve', unit)
        self.assertNotIn('[Install]', unit)            # never enabled: started only by its switch or the autostart
        build = self.read('os-bases', 'peppermint', 'build.sh')
        self.assertIn('hearth-vnc', build)

    def test_settings_page_and_panel(self):
        pages = self.read('hosts', 'linux', 'desktop', 'shell', 'settings-pages.js')
        self.assertIn('file: "SetRemoteDesktop.qml"', pages)
        self.assertIn('file: "SetRemote.qml"', pages)    # Moonlight's page stays
        page = self.read('hosts', 'linux', 'desktop', 'shell', 'SetRemoteDesktop.qml')
        on_open = page.split('Component.onCompleted:')[1].split('\n')[0]
        self.assertNotIn('"set"', on_open)
        self.assertNotIn('password', on_open)
        shell = self.read('hosts', 'linux', 'desktop', 'shell', 'shell.qml')
        self.assertIn('Recorder.qml', shell)
        self.assertIn('CaptureIndicator.qml', shell)
        opener = self.read('hosts', 'linux', 'desktop', 'open.sh')
        self.assertIn('call record startStop', opener)
        auto = self.read('hosts', 'linux', 'desktop', 'labwc', 'autostart')
        self.assertIn('hearth-vnc" session', auto)

    def test_indicator_watches_the_runtime_files(self):
        ind = self.read('hosts', 'linux', 'desktop', 'shell', 'CaptureIndicator.qml')
        self.assertIn('/hearth-record/state.json', ind)
        self.assertIn('/hearth-vnc/clients.json', ind)
        self.assertTrue(rec.STATE.endswith('/hearth-record/state.json'))
        self.assertTrue(vnc.CLIENTS.endswith('/hearth-vnc/clients.json'))


class RecorderPipeline(unittest.TestCase):
    def test_pipeline_text(self):
        self.addCleanup(setattr, rec, 'gst_has', rec.gst_has)
        rec.gst_has = lambda n: n in ('avenc_aac', 'mp4mux')
        p = rec.pipeline(7, 42, {'fps': 60, 'sound': 'both'}, rec.ENCODERS[3], '/tmp/a b.mp4', True)
        self.assertIn('pipewiresrc fd=7 path=42', p)
        self.assertIn('framerate=60/1', p)
        self.assertIn('location="/tmp/a b.mp4"', p)
        self.assertIn('@DEFAULT_MONITOR@', p)
        self.assertIn('@DEFAULT_SOURCE@', p)
        self.assertIn('audiomixer', p)
        p = rec.pipeline(7, 42, {'fps': 30, 'sound': 'none'}, rec.ENCODERS[3], '/tmp/a.mp4', True)
        self.assertNotIn('pulsesrc', p)

    def test_options(self):
        self.addCleanup(setattr, rec, 'load', rec.load)
        rec.load = lambda: dict(rec.DEFAULTS)
        o = rec.parse_opts(['--what', 'window', '--sound', 'mic', '--fps', '60'])
        self.assertEqual((o['what'], o['sound'], o['fps']), ('window', 'mic', 60))
        with self.assertRaises(ValueError):
            rec.parse_opts(['--sound', 'loud'])


if __name__ == '__main__':
    unittest.main()
