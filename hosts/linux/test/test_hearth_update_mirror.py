"""Tests for hearth-update-mirror (os/system_files/usr/libexec/nimbus): Hearth OS updates from Hearth Storage. A fake
ghcr.io (with its anonymous token step) and a fake Hearth Link socket stand in for the real ones: the server's
fetch, bootc's mirror (pieces from the server only when they check out, ghcr.io otherwise, 503 with no server),
hold-back's 3-day cap, the registries.conf.d entry, and hearth-boot-guard noting a good start."""
import hashlib
import http.server
import importlib.machinery
import importlib.util
import json
import os
import socket
import tempfile
import threading
import time
import tomllib
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
SYS = os.path.join(HERE, '..', '..', '..', 'os', 'system_files')


def load(name, mod):
    loader = importlib.machinery.SourceFileLoader(mod, os.path.join(SYS, 'usr', 'libexec', 'nimbus', name))
    spec = importlib.util.spec_from_loader(mod, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


M = load('hearth-update-mirror', 'hum')
G = load('hearth-boot-guard', 'hbg_um')


def dg(b):
    return 'sha256:' + hashlib.sha256(b).hexdigest()


LAYER = os.urandom(300000)
CONFIG = json.dumps({'config': {'Labels': {'org.opencontainers.image.version': '0.6.52'}}}).encode()
MANIFEST = json.dumps({'schemaVersion': 2, 'mediaType': 'application/vnd.oci.image.manifest.v1+json',
                       'config': {'digest': dg(CONFIG), 'size': len(CONFIG)},
                       'layers': [{'digest': dg(LAYER), 'size': len(LAYER)}]}).encode()
BLOBS = {dg(LAYER): LAYER, dg(CONFIG): CONFIG}


class FakeGhcr(http.server.BaseHTTPRequestHandler):
    hits = []

    def log_message(self, *a):
        pass

    def do_HEAD(self):
        self.do_GET(head=True)

    def do_GET(self, head=False):
        FakeGhcr.hits.append(self.path)
        if self.path.startswith('/token'):
            body = json.dumps({'token': 'anon'}).encode()
            return self.reply(200, body, {'Content-Type': 'application/json'}, head)
        if self.headers.get('Authorization') != 'Bearer anon':
            port = self.server.server_address[1]
            return self.reply(401, b'', {'WWW-Authenticate': f'Bearer realm="http://127.0.0.1:{port}/token",service="ghcr.io"'}, head)
        p = self.path
        if p in ('/v2/dragongamer9/hearth-os/manifests/latest', f'/v2/dragongamer9/hearth-os/manifests/{dg(MANIFEST)}'):
            return self.reply(200, MANIFEST, {'Content-Type': 'application/vnd.oci.image.manifest.v1+json', 'Docker-Content-Digest': dg(MANIFEST)}, head)
        for d, b in BLOBS.items():
            if p == f'/v2/dragongamer9/hearth-os/blobs/{d}':
                return self.reply(200, b, {'Content-Type': 'application/octet-stream'}, head)
        self.reply(404, b'', {}, head)

    def reply(self, code, body, headers, head):
        self.send_response(code)
        for k, v in headers.items():
            self.send_header(k, v)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        if not head:
            self.wfile.write(body)


class FakeLink(threading.Thread):
    """This user's Hearth Link: answers the mirror's commands from a folder of pieces."""

    def __init__(self, path, folder, use=True):
        super().__init__(daemon=True)
        self.path, self.folder, self.use, self.calls = path, folder, use, []
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.bind(path)
        self.sock.listen(8)

    def run(self):
        while True:
            try:
                c, _ = self.sock.accept()
            except OSError:
                return
            req = json.loads(c.makefile('rb').readline())
            self.calls.append(req['cmd'])
            if req['cmd'] == 'storage.updates.mirror':
                out = {'ok': True, 'data': {'use': self.use}}
            elif req['cmd'] == 'storage.updates.fetch':
                p = os.path.join(self.folder, req['args']['digest'][7:])
                out = {'ok': True, 'data': {'path': p, 'size': os.path.getsize(p)}} if os.path.exists(p) else {'ok': False, 'error': 'Not in the update cache'}
            else:
                out = {'ok': True, 'data': None}
            c.sendall((json.dumps(out) + '\n').encode())
            c.close()


def http_get(sock, path, method='GET', headers=''):
    sock.sendall(f'{method} {path} HTTP/1.1\r\nHost: x\r\n{headers}\r\n'.encode())
    f = sock.makefile('rb')
    status = int(f.readline().split()[1])
    h = {}
    while True:
        line = f.readline().decode().strip()
        if not line:
            break
        k, v = line.split(':', 1)
        h[k.lower()] = v.strip()
    n = int(h.get('content-length', '0'))
    body = f.read(n) if method == 'GET' and n else b''
    return status, h, body


class Base(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ghcr = http.server.ThreadingHTTPServer(('127.0.0.1', 0), FakeGhcr)
        threading.Thread(target=cls.ghcr.serve_forever, daemon=True).start()
        cls.base = f'http://127.0.0.1:{cls.ghcr.server_address[1]}'

    @classmethod
    def tearDownClass(cls):
        cls.ghcr.shutdown()

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = self.tmp.name
        M.RUN = os.path.join(self.d, 'run')
        M.STATE = os.path.join(self.d, 'state')
        FakeGhcr.hits = []

    def tearDown(self):
        self.tmp.cleanup()


class Fetch(Base):
    def test_server_fetches_once_and_checks(self):
        out = M.fetch('ghcr.io/dragongamer9/hearth-os:latest', f'{self.d}/U', M.Upstream(self.base))
        self.assertTrue(out['ok'])
        self.assertEqual(out['digest'], dg(MANIFEST))
        self.assertEqual(out['version'], '0.6.52')
        self.assertEqual(out['downloaded'], len(LAYER) + len(CONFIG))
        with open(M.blob_path(f'{self.d}/U', dg(LAYER)), 'rb') as f:
            self.assertEqual(f.read(), LAYER)
        # Again: nothing downloaded; a piece of no image is cleared away.
        stray = M.blob_path(f'{self.d}/U', 'sha256:' + 'b' * 64)
        open(stray, 'w').close()
        self.assertEqual(M.fetch('ghcr.io/dragongamer9/hearth-os:latest', f'{self.d}/U', M.Upstream(self.base))['downloaded'], 0)
        self.assertFalse(os.path.exists(stray))
        refs = json.load(open(f'{self.d}/U/refs.json'))
        self.assertIn(dg(LAYER), refs['ghcr.io/dragongamer9/hearth-os:latest']['keep'])

    def test_only_hearth_images(self):
        for ref in ('docker.io/library/alpine', 'ghcr.io/someone/hearth-os:latest', 'ghcr.io/dragongamer9/../x'):
            with self.assertRaises(ValueError):
                M.fetch(ref, f'{self.d}/U', M.Upstream(self.base))


class Serve(Base):
    def mirror(self, link_use=None, pieces=None):
        if link_use is not None:
            os.makedirs(f'{self.d}/user', exist_ok=True)
            self.link = FakeLink(f'{self.d}/user/link.sock', pieces or f'{self.d}/pieces', link_use)
            self.link.start()
        M.SOCKETS = f'{self.d}/user/link.sock'
        a, b = socket.socketpair()
        a.settimeout(10)
        t = threading.Thread(target=M.Mirror(b, M.Upstream(self.base)).loop, daemon=True)
        t.start()
        return a

    def test_no_server_bootc_uses_ghcr_itself(self):
        s = self.mirror()
        self.assertEqual(http_get(s, '/v2/')[0], 503)
        s2 = self.mirror(link_use=False)
        self.assertEqual(http_get(s2, '/v2/')[0], 503)

    def test_pieces_from_the_server_when_they_check_out(self):
        os.makedirs(f'{self.d}/pieces')
        with open(f'{self.d}/pieces/{dg(LAYER)[7:]}', 'wb') as f:
            f.write(LAYER)
        s = self.mirror(link_use=True)
        self.assertEqual(http_get(s, '/v2/')[0], 200)
        # The tag: always ghcr.io's.
        st, h, body = http_get(s, '/v2/dragongamer9/hearth-os/manifests/latest', headers='Accept: application/vnd.oci.image.manifest.v1+json\r\n')
        self.assertEqual((st, body, h['docker-content-digest']), (200, MANIFEST, dg(MANIFEST)))
        FakeGhcr.hits = []
        st, h, body = http_get(s, f'/v2/dragongamer9/hearth-os/blobs/{dg(LAYER)}')
        self.assertEqual((st, body), (200, LAYER))
        self.assertFalse(any('blobs' in x for x in FakeGhcr.hits))
        # A range of it.
        st, h, body = http_get(s, f'/v2/dragongamer9/hearth-os/blobs/{dg(LAYER)}', headers='Range: bytes=10-19\r\n')
        self.assertEqual((st, body, h['content-range']), (206, LAYER[10:20], f'bytes 10-19/{len(LAYER)}'))
        # Not on the server: from ghcr.io.
        st, h, body = http_get(s, f'/v2/dragongamer9/hearth-os/blobs/{dg(CONFIG)}')
        self.assertEqual((st, body), (200, CONFIG))
        self.assertIn('storage.updates.drop', self.link.calls)

    def test_a_wrong_piece_is_never_sent(self):
        os.makedirs(f'{self.d}/pieces')
        with open(f'{self.d}/pieces/{dg(LAYER)[7:]}', 'wb') as f:
            f.write(b'x' * len(LAYER))
        s = self.mirror(link_use=True)
        http_get(s, '/v2/')
        st, h, body = http_get(s, f'/v2/dragongamer9/hearth-os/blobs/{dg(LAYER)}')
        self.assertEqual((st, body), (200, LAYER))
        # And the server is left alone for a while.
        self.assertTrue(M.server_bad())
        self.assertEqual(http_get(self.mirror(), '/v2/')[0], 503)

    def test_other_images_and_paths_refused(self):
        s = self.mirror(link_use=True)
        self.assertEqual(http_get(s, '/v2/library/alpine/manifests/latest')[0], 404)
        self.assertEqual(http_get(s, '/v2/dragongamer9/hearth-os/blobs/sha256:..%2f..')[0], 404)
        self.assertEqual(http_get(s, '/v2/dragongamer9/hearth-os/manifests/latest', 'DELETE')[0], 405)

    def test_tls_hello_closes(self):
        a, b = socket.socketpair()
        t = threading.Thread(target=M.Mirror(b, M.Upstream(self.base)).loop)
        t.start()
        a.sendall(b'\x16\x03\x01\x00\x10hello')
        t.join(5)
        self.assertFalse(t.is_alive())


class Hold(Base):
    def test_hold_and_its_cap(self):
        t = 1_000_000.0
        ask = lambda: {'hold': True, 'waitingFor': 'Ally', 'version': '0.6.52'}
        self.assertEqual(M.may_stage(t, ask), 1)
        self.assertEqual(M.may_stage(t + 86400, ask), 1)
        self.assertEqual(json.load(open(M.hold_file()))['first_held'], t)
        self.assertEqual(M.may_stage(t + 3 * 86400, ask), 0)

    def test_fails_open(self):
        self.assertEqual(M.may_stage(1.0, lambda: {'hold': True}), 1)
        self.assertEqual(M.may_stage(2.0, lambda: None), 0)
        self.assertFalse(os.path.exists(M.hold_file()))
        def boom():
            raise RuntimeError('x')
        self.assertEqual(M.may_stage(3.0, boom), 0)
        self.assertEqual(M.may_stage(4.0, lambda: {'hold': 'yes'}), 0)
        # No Hearth Link at all.
        M.SOCKETS = f'{self.d}/none/*.sock'
        self.assertEqual(M.main(['x', 'may-stage']), 0)


class Wiring(unittest.TestCase):
    def test_registries_entry(self):
        with open(os.path.join(SYS, 'etc', 'containers', 'registries.conf.d', '60-hearth-storage.conf'), 'rb') as f:
            conf = tomllib.load(f)
        r = conf['registry'][0]
        self.assertEqual((r['prefix'], r['location']), ('ghcr.io/dragongamer9', 'ghcr.io/dragongamer9'))
        self.assertEqual(r['mirror'], [{'location': '127.0.0.1:479/dragongamer9', 'insecure': True}])
        with open(os.path.join(SYS, 'usr', 'lib', 'systemd', 'system', 'hearth-update-mirror.socket')) as f:
            sock = f.read()
        self.assertIn('ListenStream=127.0.0.1:479', sock)
        self.assertTrue(os.path.islink(os.path.join(SYS, 'usr', 'lib', 'systemd', 'system', 'sockets.target.wants', 'hearth-update-mirror.socket')))
        for unit in ('bootc-fetch-apply-updates', 'uupd'):
            with open(os.path.join(SYS, 'usr', 'lib', 'systemd', 'system', f'{unit}.service.d', '30-hearth-hold-back.conf')) as f:
                drop = f.read()
            self.assertIn('ExecCondition=/usr/libexec/nimbus/hearth-update-mirror may-stage', drop)

    def test_boot_guard_notes_a_good_start(self):
        status = {'status': {'booted': {'image': {'image': {'image': 'ghcr.io/dragongamer9/hearth-os:latest', 'transport': 'registry'},
                                                  'version': '0.6.52', 'imageDigest': 'sha256:' + 'c' * 64}}}}
        with tempfile.TemporaryDirectory() as d:
            G.STATE = d
            ran = mock.Mock(return_value=mock.Mock(returncode=0, stdout=json.dumps(status)))
            with mock.patch.object(G.subprocess, 'run', ran):
                G.record_good()
            v = json.load(open(f'{d}/boot-good.json'))
            self.assertEqual((v['image'], v['version'], v['digest']), ('ghcr.io/dragongamer9/hearth-os:latest', '0.6.52', 'sha256:' + 'c' * 64))
            # bootc missing or failing: nothing written, nothing raised.
            os.remove(f'{d}/boot-good.json')
            with mock.patch.object(G.subprocess, 'run', mock.Mock(side_effect=FileNotFoundError('bootc'))):
                G.record_good()
            self.assertFalse(os.path.exists(f'{d}/boot-good.json'))


if __name__ == '__main__':
    unittest.main()
