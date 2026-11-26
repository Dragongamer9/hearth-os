"""Hearth Forge (hosts/hearth-ui/apps/forge; docs/mockups/checklists/hearth-forge.md): its list of every model (the
board's 22, each a real entry), its helper (hearth-forge.py: Get checks every file's SHA-256 and keeps nothing that
doesn't match, the preview of a .glb, the service only answering this PC and Tailscale, its user service guarded so it
can never stop the PC or Game Mode from starting), and that it stays private (never in the Store's public list)."""
import hashlib
import http.server
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import struct
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
APP = ROOT / 'hosts/hearth-ui/apps/forge'
HELPER = APP / 'helper/hearth-forge.py'
MODELS = json.loads((APP / 'helper/models.json').read_text())

# the board's rows (02-models.html), in its order
BOARD = ['Hunyuan3D 2.0', 'Hunyuan3D 2.0 mini', 'Hunyuan3D 2.0 Turbo', 'Hunyuan3D 2.1', 'TRELLIS.2', 'TRELLIS', 'Stable Fast 3D', 'SPAR3D', 'Pixal3D',
         'Step1X-3D', 'TripoSR', 'TripoSG', 'Direct3D-S2', 'Hi3DGen', 'Hi3D 1.0', 'UltraShape', 'SAM 3D Objects', 'SAM 3D Body', 'TripoSplat',
         'HY-World 2.0', 'Lyra 2.0', 'SHARP']


def helper(tmp):
    os.environ['HEARTH_FORGE_DIR'] = os.path.join(tmp, 'Forge')
    os.environ['XDG_STATE_HOME'] = os.path.join(tmp, 'state')
    os.environ['HEARTH_FORGE_DATA'] = os.path.join(tmp, 'data')
    loader = importlib.machinery.SourceFileLoader('hearth_forge_%d' % id(tmp), str(HELPER))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Catalog(unittest.TestCase):
    def test_every_model_on_the_board(self):
        self.assertEqual([m['name'] for m in MODELS['models']], BOARD)

    def test_each_is_a_real_entry(self):
        for m in MODELS['models']:
            with self.subTest(m=m['id']):
                for k in ('maker', 'makes', 'memory', 'fit', 'fitWords', 'licence', 'page'):
                    self.assertTrue(m.get(k), k)
                self.assertIn(m['fit'], ('ok', 'warn', 'bad', 'unknown', 'gated'))
                if m.get('noSource'):
                    self.assertEqual(m['files'], [])
                for f in m['files']:
                    self.assertRegex(f['repo'], r'^[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*$')
                    self.assertTrue(f['include'])
                    self.assertTrue(f['revision'] == '' or len(f['revision']) == 40)
                if m.get('runner'):
                    r = MODELS['runners'][m['runner']]
                    self.assertRegex(r['commit'], r'^[0-9a-f]{40}$')
                    self.assertTrue(r['git'].startswith('https://github.com/'))
                if m.get('code'):
                    self.assertRegex(m['code']['commit'], r'^[0-9a-f]{40}$')

    def test_the_boards_four_and_its_default(self):
        self.assertEqual([m['id'] for m in MODELS['models'] if m.get('default')], ['hunyuan3d-2'])
        self.assertEqual(sorted(m['best'] for m in MODELS['models'] if m.get('best')), [1, 2, 3, 4])


class Get(unittest.TestCase):
    """Get against a pretend Hugging Face: the files at the pinned commit, each checked against its hash."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.h = helper(self.tmp)
        self.good = b'weights' * 1000
        self.small = b'{"a": 1}\n'
        commit = 'c' * 40
        files = {'dit/model.fp16.safetensors': self.good, 'dit/config.yaml': self.small, 'other/big.bin': b'x'}
        self.bad = False
        outer = self

        class H(http.server.BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def do_GET(self):
                if self.path.startswith('/api/models/'):
                    sib = []
                    for p, b in files.items():
                        e = {'rfilename': p, 'size': len(b), 'blobId': hashlib.sha1(b'blob %d\0' % len(b) + b).hexdigest()}
                        if p.endswith('.safetensors'):
                            e['lfs'] = {'sha256': hashlib.sha256(b).hexdigest(), 'size': len(b)}
                        sib.append(e)
                    body = json.dumps({'sha': commit, 'siblings': sib}).encode()
                else:
                    p = self.path.split('/resolve/%s/' % commit, 1)[1]
                    body = files[p]
                    if outer.bad and p.endswith('.safetensors'):
                        body = b'Weights' + body[7:]
                self.send_response(200)
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)
        self.srv = http.server.ThreadingHTTPServer(('127.0.0.1', 0), H)
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.h.HF = 'http://127.0.0.1:%d' % self.srv.server_address[1]
        self.h.secret = lambda k: ''
        self.h.game_running = lambda: False
        self.h.free_gb = lambda p: 1000.0
        self.cat = {'models': [{'id': 'm', 'name': 'M', 'files': [{'repo': 'org/m', 'revision': '', 'include': ['dit/*']}]}], 'runners': {}}

    def tearDown(self):
        self.srv.shutdown()

    def test_only_the_files_it_needs_each_checked_and_pinned(self):
        self.h.get_model('m', self.cat)
        snap = os.path.join(self.h.HUB, 'models--org--m', 'snapshots', 'c' * 40)
        self.assertEqual(open(os.path.join(snap, 'dit/model.fp16.safetensors'), 'rb').read(), self.good)
        self.assertFalse(os.path.exists(os.path.join(snap, 'other/big.bin')))
        self.assertEqual(open(os.path.join(self.h.HUB, 'models--org--m', 'refs', 'main')).read(), 'c' * 40)
        lk = self.h.lock()
        self.assertEqual(lk['m']['sources'][0]['commit'], 'c' * 40)
        self.assertTrue(self.h.installed(self.cat['models'][0], lk))

    def test_a_file_that_doesnt_match_is_never_kept(self):
        self.bad = True
        with self.assertRaises(self.h.Fail):
            self.h.get_model('m', self.cat)
        snap = os.path.join(self.h.HUB, 'models--org--m', 'snapshots', 'c' * 40, 'dit')
        self.assertFalse(os.path.exists(os.path.join(snap, 'model.fp16.safetensors')))
        self.assertFalse(os.path.exists(os.path.join(snap, 'model.fp16.safetensors.part')))
        self.assertNotIn('m', self.h.lock())

    def test_a_pinned_revision_is_kept_to(self):
        self.cat['models'][0]['files'][0]['revision'] = 'd' * 40
        with self.assertRaises(self.h.Fail):
            self.h.get_model('m', self.cat)


def tiny_glb(path):
    v = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)]
    f = [(0, 2, 1), (0, 1, 3), (0, 3, 2), (1, 2, 3)]
    pos = b''.join(struct.pack('<3f', *x) for x in v)
    idx = b''.join(struct.pack('<3I', *x) for x in f)
    binb = pos + idx
    js = {'asset': {'version': '2.0'}, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0, 'scale': [2, 2, 2]}], 'meshes': [{'primitives': [{'attributes': {'POSITION': 0}, 'indices': 1}]}],
          'buffers': [{'byteLength': len(binb)}], 'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': len(pos)}, {'buffer': 0, 'byteOffset': len(pos), 'byteLength': len(idx)}],
          'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 4, 'type': 'VEC3'}, {'bufferView': 1, 'componentType': 5125, 'count': 12, 'type': 'SCALAR'}]}
    j = json.dumps(js).encode()
    j += b' ' * ((4 - len(j) % 4) % 4)
    with open(path, 'wb') as fh:
        fh.write(struct.pack('<III', 0x46546C67, 2, 28 + len(j) + len(binb)) + struct.pack('<II', len(j), 0x4E4F534A) + j + struct.pack('<II', len(binb), 0x004E4942) + binb)


class Preview(unittest.TestCase):
    def test_a_glb_read_with_python_alone(self):
        tmp = tempfile.mkdtemp()
        h = helper(tmp)
        tiny_glb(os.path.join(tmp, 'model.glb'))
        h.make_preview(tmp)
        st = json.load(open(os.path.join(tmp, 'stats.json')))
        self.assertEqual((st['faces'], st['points']), (4, 4))
        self.assertTrue(st['closed'], 'a closed shape prints solid')
        self.assertEqual([round(x, 3) for x in st['size']], [2.0, 2.0, 2.0], 'the node\'s scale is applied')
        pv = json.load(open(os.path.join(tmp, 'preview.json')))
        self.assertEqual(len(pv['f']), 12)


class Service(unittest.TestCase):
    def test_only_this_pc_and_tailscale(self):
        h = helper(tempfile.mkdtemp())
        self.assertRegex('100.64.1.2', h.TS_IP_RE)
        self.assertIsNone(h.TS_IP_RE.fullmatch('192.168.1.5'))
        src = HELPER.read_text()
        self.assertIn("ip.startswith('127.') or ip == '::1' or TS_IP_RE.fullmatch(ip) is not None", src)

    def test_its_user_service_can_never_stop_starting(self):
        h = helper(tempfile.mkdtemp())
        self.assertNotIn('[Install]', h.UNIT_TEXT, 'the service is started by its timer or the app only')
        self.assertIn('ConditionPathExists=', h.UNIT_TEXT)
        self.assertIn('IOSchedulingClass=idle', h.UNIT_TEXT)
        self.assertIn('WantedBy=timers.target', h.TIMER_TEXT)
        self.assertNotIn('default.target', h.UNIT_TEXT + h.TIMER_TEXT)
        self.assertFalse(list((ROOT / 'os/system_files').rglob('hearth-forge.service')), 'nothing of it starts from the image')

    def test_waits_while_you_play_and_gets_only_when_asked(self):
        src = HELPER.read_text()
        self.assertIn("while game_running():", src)
        self.assertIn("if len(parts) == 3 and parts[0] == 'models' and parts[2] in ('get', 'remove'):", src)
        self.assertNotIn('snapshot_download(', src.split('def job_run')[0], 'Hearth downloads by itself, checked; the models\' own code never does')
        self.assertIn("HF_HUB_OFFLINE='1'", src)

    def test_keys_never_leave_the_password_store(self):
        src = HELPER.read_text()
        self.assertIn("['secret-tool', 'lookup', 'hearth-app', 'forge', 'key', key]", src)
        for bad in ('print(key', 'print(k)', "write(key", 'log(key'):
            self.assertNotIn(bad, src)


class Private(unittest.TestCase):
    def test_never_in_the_public_list(self):
        cat = (ROOT / 'os/system_files/usr/share/hearth/store/catalog.json').read_text().lower()
        self.assertNotIn('forge', cat)

    def test_its_store_updates_go_to_the_private_list(self):
        q = (ROOT / '.github/workflows/quick-update.yml').read_text()
        self.assertIn('hearth-forge-app,', q)
        self.assertIn('private_apps = ("hearth-forge-app",)', q)

    def test_hidden_in_everyones_menu(self):
        d = (ROOT / 'os/system_files/usr/share/applications/hearth-forge.desktop').read_text()
        self.assertIn('NoDisplay=true', d)


if __name__ == '__main__':
    unittest.main()
