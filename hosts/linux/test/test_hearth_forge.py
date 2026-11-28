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
import shutil
import struct
import sys
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
                self.assertFalse(m.get('noSource'), 'every model can be got')
                self.assertTrue(m['files'], 'every model has its own files to get')
                for f in m['files']:
                    self.assertRegex(f['repo'], r'^[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*$')
                    self.assertTrue(f['include'])
                    self.assertTrue(f['revision'] == '' or len(f['revision']) == 40)
                r = MODELS['runners'][m['runner']]
                self.assertRegex(r['commit'], r'^[0-9a-f]{40}$')
                self.assertTrue(r['git'].startswith('https://github.com/'))
                self.assertNotIn('code', m)

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


RUNNERS_PY = APP / 'helper/forge_runners.py'
BUILTIN = ('hunyuan3d2', 'triposr', 'sf3d')


def runners_module():
    loader = importlib.machinery.SourceFileLoader('forge_runners_%d' % len(sys.modules), str(RUNNERS_PY))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class EveryModelMakes(unittest.TestCase):
    """The owner, Nov 26: "Yes every model need to be able to make suff": each of the 22 has its own code to make with."""

    def test_each_has_a_runner_with_its_own_code(self):
        fr = runners_module()
        for m in MODELS['models']:
            with self.subTest(m=m['id']):
                rid = m['runner']
                self.assertIn(rid, MODELS['runners'])
                self.assertTrue(rid in BUILTIN or rid in fr.RUNNERS, 'a make path in forge_runners.py')
                r = MODELS['runners'][rid]
                self.assertIn(r.get('python', ''), ('3.10', '3.11', '3.13'), 'the Python its maker names')

    def test_every_git_requirement_is_a_pinned_commit(self):
        for rid, r in MODELS['runners'].items():
            for line in list(r.get('pip') or []) + list(r.get('late') or []) + list((r.get('replace') or {}).values()):
                with self.subTest(r=rid, line=line):
                    if 'git+' in line:
                        self.assertRegex(line, r'@[0-9a-f]{40}(#|$)')
                    if line.startswith('https://'):
                        self.assertRegex(line, r'#sha256=[0-9a-f]{64}$', 'a download is checked by its hash')
            for h in r.get('hub') or []:
                self.assertRegex(h['commit'], r'^[0-9a-f]{40}$')

    def test_the_makers_own_releases(self):
        by = {m['id']: m for m in MODELS['models']}
        self.assertEqual(by['hi3d']['files'][0]['repo'], 'hi3d-official/Hi3Dv1.0', "Hi3D's own release (its maker's org)")
        self.assertEqual(MODELS['runners']['hi3d']['git'], 'https://github.com/hi3d-official/Hi3Dv1.0.git')
        self.assertEqual(by['hy-world-2']['files'][0]['include'][0], 'HY-WorldMirror-2.0/*', 'only WorldMirror (its world generation is far beyond 12 GB)')
        for mid in ('hi3d', 'ultrashape'):
            for c in by[mid]['coarse']:
                self.assertFalse(by[c].get('coarse'), 'a first shape is made by a model that needs none')
        for m in MODELS['models']:
            if m.get('paint'):
                self.assertEqual(m['paint'], 'hunyuan3d2', "the board's \"2.0 paints them\"")
        self.assertIn('hunyuan3d2', by['hunyuan3d-2.1']['alsoRunners'])
        self.assertEqual(by['sam-3d-objects']['vramMin'], 32)

    def test_the_app_writes_the_runners_beside_its_helper(self):
        src = (APP / 'src/forge.rs').read_text()
        self.assertIn('include_str!("../helper/forge_runners.py")', src)
        self.assertIn('("forge_runners.py", RUNNERS_PY)', src)


class Requirements(unittest.TestCase):
    def setUp(self):
        self.h = helper(tempfile.mkdtemp())

    def test_the_makers_own_list_as_hearth_installs_it(self):
        text = '\n'.join(['--extra-index-url https://mirrors.example/simple', '# a note', 'torch==2.5.1', 'torchvision==0.20.1', 'numpy==1.26.4',
                          'gradio==5.0', 'nvidia-cuda-nvcc-cu12==12.1.105', 'git+https://github.com/openai/CLIP.git', './texture_baker/', 'diso',
                          'MoGe @ git+https://github.com/microsoft/MoGe.git@' + 'a' * 40, 'flash_attn==2.8.3'])
        r = {'skip': ['gradio'], 'replace': {'git+https://github.com/openai/CLIP.git': 'git+https://github.com/openai/CLIP.git@' + 'b' * 40}, 'late': ['diso', 'flash-attn']}
        now, late, dropped = self.h.filter_requirements(text, r)
        self.assertEqual(now, ['numpy==1.26.4', 'git+https://github.com/openai/CLIP.git@' + 'b' * 40, 'MoGe @ git+https://github.com/microsoft/MoGe.git@' + 'a' * 40])
        self.assertEqual(late, ['./texture_baker/', 'diso', 'flash_attn==2.8.3'])
        self.assertIn('torch==2.5.1', dropped, "PyTorch's CUDA 12.8 build stays")
        self.assertIn('--extra-index-url https://mirrors.example/simple', dropped, 'only PyPI and PyTorch\'s own index')
        self.assertIn('nvidia-cuda-nvcc-cu12==12.1.105', dropped)

    def test_an_unpinned_git_requirement_is_refused(self):
        with self.assertRaises(self.h.Fail):
            self.h.filter_requirements('git+https://github.com/NVlabs/nvdiffrast.git', {})

    def test_every_runner_names_what_it_replaces_as_its_maker_wrote_it(self):
        repos = pathlib.Path(os.environ.get('HEARTH_FORGE_REPOS', '/nonexistent'))
        for rid, r in MODELS['runners'].items():
            for k, v in (r.get('replace') or {}).items():
                self.assertTrue(k.split('@')[0].split('#')[0] in v, '%s: %s → %s' % (rid, k, v))

    def test_what_it_builds_for(self):
        self.assertEqual(self.h.cuda_arch('12.8'), '12.0', "the RTX 5070's own sm_120")
        self.assertEqual(self.h.cuda_arch('12.9'), '12.0')
        self.assertEqual(self.h.cuda_arch('12.4'), '8.9+PTX', "an older toolkit: PTX the card's driver compiles")


class Runtime(unittest.TestCase):
    """Get installs the model's own code into ~/Hearth/Forge/runtimes/<runner> (no network: every program faked)."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.h = helper(self.tmp)
        self.calls = []
        self.repo_files = {}
        h = self.h

        def fake(args, timeout=60, cwd=None, env=None):
            self.calls.append((list(args), cwd, dict(env or {})))
            a = list(args)
            if a[:2] == ['git', 'init']:
                os.makedirs(os.path.join(a[3], '.git'), exist_ok=True)
            if a[:1] == ['git'] and 'checkout' in a:
                root = a[2]
                for rel, text in self.repo_files.items():
                    os.makedirs(os.path.dirname(os.path.join(root, rel)), exist_ok=True)
                    open(os.path.join(root, rel), 'w').write(text)
            if a[:1] == ['git'] and 'rev-parse' in a:
                return 0, self.commit + '\n'
            if 'venv' in a:
                vp = a[-1]
                os.makedirs(os.path.join(vp, 'bin'), exist_ok=True)
                open(os.path.join(vp, 'bin', 'python'), 'w').close()
                if a[0].endswith('python3') and len(a) > 3:
                    pass
            if a[-1] == self.h.UV_PIN:
                open(os.path.join(os.path.dirname(a[0]), 'uv'), 'w').close()
            if a[-1] == '--version':
                return 0, 'Cuda compilation tools, release 12.8, V12.8.93'
            if 'show' in a:
                return 1, ''
            if '-c' in a and a[0].endswith('python') and 'import json, torch' in a[-1]:
                return 0, '{"torch": "2.7.1+cu128", "cuda": true, "gpu": "NVIDIA GeForce RTX 5070"}'
            return 0, ''
        h.cmd = fake
        self.have_nvcc = True
        from unittest import mock
        real_isfile = os.path.isfile
        for target, fn in (('shutil.which', lambda n: '/usr/bin/' + n if n == 'git' or (n == 'nvcc' and self.have_nvcc) else None),
                           ('os.path.isfile', lambda p: (self.have_nvcc if p.endswith('/nvcc') else real_isfile(p)))):
            pt = mock.patch(target, fn)
            pt.start()
            self.addCleanup(pt.stop)

    def cat(self, rid, r):
        self.commit = r['commit']
        return {'runners': {rid: r}, 'models': []}

    def test_its_own_code_its_python_its_requirements_then_its_parts(self):
        r = {'name': 'SPAR3D', 'git': 'https://github.com/Stability-AI/stable-point-aware-3d.git', 'commit': 'f' * 40, 'python': '3.10',
             'requirements': ['requirements.txt'], 'replace': {'git+https://github.com/openai/CLIP.git': 'git+https://github.com/openai/CLIP.git@' + 'c' * 40},
             'late': ['natten==0.21.0'], 'build': ['uv_unwrapper'], 'env': {'ATTN_BACKEND': 'xformers'}}
        self.repo_files = {'requirements.txt': 'einops==0.7.0\ntorch==2.5.1\ngit+https://github.com/openai/CLIP.git\n./texture_baker/\n'}
        steps = []
        self.h.install_runner('spar3d', self.cat('spar3d', r), steps.append)
        d = os.path.join(self.h.RUNTIMES, 'spar3d')
        self.assertTrue(d.startswith(os.path.join(self.tmp, 'Forge', 'runtimes')), '~/Hearth/Forge/runtimes/<runner>')
        argv = [c[0] for c in self.calls]
        fetch = next(a for a in argv if 'fetch' in a)
        self.assertEqual(fetch[-1], 'f' * 40, 'its pinned commit')
        venv = next(a for a in argv if 'venv' in a and a[0].endswith('/uv'))
        self.assertEqual(venv[1:5], ['venv', '--seed', '--python', '3.10'])
        tor = next(a for a in argv if '--index-url' in a)
        self.assertEqual(tor[tor.index('--index-url') + 1], 'https://download.pytorch.org/whl/cu128')
        self.assertIn('torch==2.7.1', tor)
        pips = [a for a in argv if 'install' in a and a[0] == os.path.join(d, 'venv', 'bin', 'python')]
        self.assertTrue(all('-c' in a for a in pips), 'PyTorch kept by a constraint every time')
        req = open(os.path.join(d, 'requirements-hearth.txt')).read()
        self.assertEqual(req, 'einops==0.7.0\ngit+https://github.com/openai/CLIP.git@' + 'c' * 40 + '\n')
        late = [a for a in pips if '--no-build-isolation' in a]
        self.assertEqual([a[-1] for a in late], ['natten==0.21.0', './texture_baker', './uv_unwrapper'])
        env = next(c[2] for c in self.calls if c[0][-1] == 'natten==0.21.0')
        self.assertEqual((env['TORCH_CUDA_ARCH_LIST'], env['NATTEN_CUDA_ARCH'], env['ATTN_BACKEND']), ('12.0', '12.0', 'xformers'))
        ready = json.load(open(os.path.join(d, 'ready.json')))
        self.assertEqual((ready['commit'], ready['python'], ready['arch']), ('f' * 40, '3.10', '12.0'))
        self.assertTrue(self.h.runner_ready('spar3d', self.cat('spar3d', r)))
        self.assertTrue(any('PyTorch' in s for s in steps) and any('building' in s for s in steps), 'how far it is, shown while it gets')

    def test_a_model_that_builds_needs_the_cuda_compiler(self):
        self.have_nvcc = False
        r = {'name': 'TRELLIS.2', 'git': 'https://github.com/microsoft/TRELLIS.2.git', 'commit': 'e' * 40, 'python': '3.10', 'late': ['git+https://x/y.git@' + 'a' * 40]}
        with self.assertRaises(self.h.Fail) as e:
            self.h.install_runner('trellis2', self.cat('trellis2', r), lambda s: None)
        self.assertIn('nvcc', str(e.exception))

    def test_torch_hub_repos_pinned_in_the_runtime(self):
        r = {'name': 'Hi3DGen', 'git': 'https://github.com/Stable-X/Hi3DGen.git', 'commit': 'd' * 40, 'python': '3.10',
             'hub': [{'repo': 'hugoycj/StableNormal', 'commit': 'd' * 40}]}
        self.h.install_runner('hi3dgen', self.cat('hi3dgen', r), lambda s: None)
        hd = os.path.join(self.h.RUNTIMES, 'hi3dgen', 'torch', 'hub', 'hugoycj_StableNormal_main')
        self.assertTrue(os.path.isdir(hd), "torch.hub finds it there (TORCH_HOME is the runtime's own)")
        self.assertIn(['git', '-C', hd + '.new', 'fetch', '-q', '--depth', '1', 'https://github.com/hugoycj/StableNormal.git', 'd' * 40], [c[0] for c in self.calls])


class FakeCtx:
    """What hearth-forge.py hands a runner, with its command line and its files faked."""

    class Fail(Exception):
        pass

    def __init__(self, tmp, model, vram=12.0, detail='high', **ask):
        self.d = tmp
        self.work = os.path.join(tmp, '.work')
        os.makedirs(self.work, exist_ok=True)
        self.m = next(m for m in MODELS['models'] if m['id'] == model)
        self.run = self.m.get('run') or {}
        self.ask = dict({'mode': 'photo', 'textures': 'pbr', 'detail': detail}, **ask)
        self.src = os.path.join(tmp, 'src')
        self.py = '/rt/venv/bin/python'
        self.bin = '/rt/venv/bin'
        self.seed, self.detail = 71204, detail
        self.res, self.steps = {'draft': (192, 20), 'good': (256, 30), 'high': (320, 40), 'highest': (384, 50)}[detail]
        self.tex, self.tsize, self.vram = self.ask['textures'], 2048, vram
        self.photo = os.path.join(tmp, 'photo.png')
        open(self.photo, 'wb').write(b'png')
        self.photos = [self.photo]
        self.words = self.ask.get('words', '')
        self.out = os.path.join(tmp, 'model.glb')
        self.coarse = os.path.join(tmp, 'coarse.glb') if ask.get('coarse') else ''
        self.timing, self.stages, self.calls, self.outputs = {}, [], [], {}

    def stage(self, s, pct):
        self.stages.append(s)

    def snapshot(self, repo):
        return '/hub/' + repo

    def links(self, mapping, folder):
        base = folder if os.path.isabs(folder) else os.path.join(self.work, folder)
        self.linked = dict(getattr(self, 'linked', {}), **{os.path.join(base, k): v for k, v in mapping.items()})
        return base

    def cli(self, argv, cwd=None, env=None, timeout=0):
        self.calls.append((argv, cwd, env))
        for p, data in self.outputs.items():
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, 'wb').write(data)

    def cutout(self, remover=None):
        open(os.path.join(self.d, 'photo-cut.png'), 'wb').write(b'cut')
        return FakeImage()


class FakeImage:
    def convert(self, mode):
        return self

    def save(self, p):
        open(p, 'wb').write(b'img')


class Rec:
    """Any class or function of a model's own code: it remembers how it was made and called."""
    log = []

    def __init__(self, name, ret=None):
        self.name, self.ret = name, ret

    def __call__(self, *a, **k):
        Rec.log.append((self.name, a, k))
        return self.ret(*a, **k) if callable(self.ret) else self.ret

    def __getattr__(self, attr):
        if attr.startswith('__'):
            raise AttributeError(attr)
        return Rec('%s.%s' % (self.name, attr))


class FakeMesh:
    def __init__(self):
        self.exported = []

    def export(self, p, **k):
        self.exported.append(p)
        open(p, 'wb').write(b'glTF')


def fake_modules(**mods):
    """sys.modules entries for a model's own packages (and torch, PIL), each a module whose names are Recs."""
    import types
    out = {}
    for name, attrs in mods.items():
        m = types.ModuleType(name)
        m.__path__ = []
        for k, v in (attrs or {}).items():
            setattr(m, k, v)
        out[name] = m
    torch = types.ModuleType('torch')
    torch.Generator = lambda device='cpu': Rec('Generator', lambda *a, **k: 'gen')(device) and type('G', (), {'manual_seed': lambda self, s: 'gen-%d' % s})()
    torch.float16 = 'fp16'
    torch.cuda = type('C', (), {'is_available': staticmethod(lambda: True), 'empty_cache': staticmethod(lambda: None)})
    torch.hub = type('H', (), {'get_dir': staticmethod(lambda: '/rt/torch/hub'), 'load': staticmethod(Rec('torch.hub.load', lambda *a, **k: Rec('normal', lambda *a, **k: FakeImage())))})
    out.setdefault('torch', torch)
    pil = types.ModuleType('PIL')
    pil.Image = type('I', (), {'open': staticmethod(lambda p: FakeImage())})
    out.setdefault('PIL', pil)
    return out


class Runners(unittest.TestCase):
    """Each model's own code, called as its README says, with its low-memory settings on 12 GB (and not on a big card),
    its outputs kept the same way (model.glb, splat.ply). Fakes stand in for the models; real making needs the PC."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.fr = runners_module()
        Rec.log = []

    def run_with(self, ctx, mods):
        from unittest import mock
        with mock.patch.dict(sys.modules, mods):
            return self.fr.make(ctx)

    def calls(self, name):
        return [c for c in Rec.log if c[0] == name]

    # -- command-line runners: the exact command, its offload flags, what it made kept

    def test_spar3d(self):
        for vram, low in ((12.0, True), (24.0, False)):
            c = FakeCtx(tempfile.mkdtemp(), 'spar3d', vram=vram)
            c.outputs = {os.path.join(c.work, 'spar3d', '0', 'mesh.glb'): b'glTF'}
            res = self.run_with(c, {})
            argv = c.calls[0][0]
            self.assertEqual(argv[:3], [c.py, os.path.join(c.src, 'run.py'), c.photo])
            self.assertEqual(argv[argv.index('--pretrained-model') + 1], '/hub/stabilityai/stable-point-aware-3d')
            self.assertEqual('--low-vram-mode' in argv, low, "Stability's ~7 GB low-memory mode on 12 GB")
            self.assertTrue(os.path.isfile(c.out) and res['mesh'] and not res['shapeOnly'])

    def test_pixal3d(self):
        c = FakeCtx(self.tmp, 'pixal3d')
        c.outputs = {os.path.join(c.work, 'pixal3d.glb'): b'glTF'}
        self.run_with(c, {})
        argv = c.calls[0][0]
        self.assertEqual(argv[1], os.path.join(c.src, 'inference.py'))
        self.assertEqual(argv[argv.index('--model_path') + 1], '/hub/TencentARC/Pixal3D')
        self.assertIn('--low_vram', argv)
        self.assertEqual(argv[argv.index('--resolution') + 1], '1024')
        self.assertTrue(os.path.isfile(c.out))

    def test_hi3d_refines_another_models_shape(self):
        c = FakeCtx(self.tmp, 'hi3d', coarse=True)
        c.outputs = {os.path.join(c.work, 'hi3d.glb'): b'glTF'}
        res = self.run_with(c, {})
        argv = c.calls[0][0]
        self.assertEqual(argv[1], os.path.join(c.src, 'demo', 'refine_mesh.py'))
        self.assertEqual(argv[argv.index('--coarse-mesh') + 1], c.coarse)
        self.assertEqual(argv[argv.index('--model') + 1], '/hub/hi3d-official/Hi3Dv1.0')
        for f in ('--local-files-only', '--sequential-cpu-offload', '--remove-background'):
            self.assertIn(f, argv)
        self.assertEqual(argv[argv.index('--dino-model') + 1], '/hub/facebook/dinov2-large')
        self.assertTrue(res['shapeOnly'])
        c2 = FakeCtx(tempfile.mkdtemp(), 'hi3d')
        with self.assertRaises(FakeCtx.Fail):
            self.run_with(c2, {})

    def test_ultrashape(self):
        c = FakeCtx(self.tmp, 'ultrashape', coarse=True)
        snap = os.path.join(self.tmp, 'snap')
        os.makedirs(snap)
        open(os.path.join(snap, 'ultrashape_v1.pt'), 'w').close()
        c.snapshot = lambda repo: snap
        c.outputs = {os.path.join(c.work, 'ultrashape', 'photo_refined.glb'): b'glTF'}
        self.run_with(c, {})
        argv = c.calls[0][0]
        self.assertEqual(argv[1], os.path.join(c.src, 'scripts', 'infer_dit_refine.py'))
        self.assertEqual(argv[argv.index('--ckpt') + 1], os.path.join(snap, 'ultrashape_v1.pt'))
        self.assertEqual(argv[argv.index('--mesh') + 1], c.coarse)
        for f, v in (('--num_latents', '8192'), ('--chunk_size', '2048')):
            self.assertEqual(argv[argv.index(f) + 1], v, 'its README\'s low-VRAM settings')
        self.assertIn('--low_vram', argv)
        self.assertTrue(os.path.isfile(c.out))

    def test_lyra2_two_steps_with_its_offload(self):
        c = FakeCtx(self.tmp, 'lyra-2')
        c.outputs = {os.path.join(c.work, 'zoomgs', 'videos', '0.mp4'): b'mp4', os.path.join(c.work, 'recon', 'reconstructed_scene.ply'): b'ply'}
        res = self.run_with(c, {})
        a1, a2 = c.calls[0][0], c.calls[1][0]
        self.assertEqual(a1[1:3], ['-m', 'lyra_2._src.inference.lyra2_zoomgs_inference'])
        for f in ('--offload', '--offload_when_prompt', '--offload_da3_diffusion', '--use_dmd'):
            self.assertIn(f, a1)
        self.assertEqual(a1[a1.index('--checkpoint_dir') + 1], '/hub/nvidia/Lyra-2.0/checkpoints/model')
        self.assertEqual(a2[1:3], ['-m', 'lyra_2._src.inference.vipe_da3_gs_recon'])
        self.assertEqual(c.calls[0][2]['PYTHONPATH'], c.src)
        self.assertTrue(res['splat'] and os.path.isfile(os.path.join(c.d, 'splat.ply')) and not res['mesh'], 'its splat kept; the service makes its mesh')
        c2 = FakeCtx(tempfile.mkdtemp(), 'lyra-2', vram=80.0)
        c2.outputs = c.outputs and {os.path.join(c2.work, 'zoomgs', 'videos', '0.mp4'): b'mp4', os.path.join(c2.work, 'recon', 'reconstructed_scene.ply'): b'ply'}
        self.run_with(c2, {})
        self.assertNotIn('--offload', c2.calls[0][0])

    def test_sharp(self):
        c = FakeCtx(self.tmp, 'sharp')
        snap = os.path.join(self.tmp, 'snap')
        os.makedirs(snap)
        open(os.path.join(snap, 'sharp_2572gikvuh.pt'), 'w').close()
        c.snapshot = lambda repo: snap
        c.outputs = {os.path.join(c.work, 'sharp', 'view-00.ply'): b'ply'}
        res = self.run_with(c, {})
        argv = c.calls[0][0]
        self.assertEqual(argv[:2], ['/rt/venv/bin/sharp', 'predict'])
        self.assertEqual(argv[argv.index('-c') + 1], os.path.join(snap, 'sharp_2572gikvuh.pt'))
        self.assertEqual(argv[argv.index('--device') + 1], 'cuda')
        self.assertTrue(os.path.isfile(os.path.join(c.work, 'input', 'view-00.png')))
        self.assertTrue(res['splat'])

    # -- Python-API runners: the maker's own classes, its low-memory switches

    def test_hunyuan3d_2_1_shape_with_its_offload(self):
        pipe = Rec('pipe', lambda *a, **k: [FakeMesh()])
        P = Rec('Hunyuan3DDiTFlowMatchingPipeline')
        P.from_pretrained = Rec('from_pretrained', lambda *a, **k: pipe)
        pipe.enable_model_cpu_offload = Rec('offload')
        c = FakeCtx(self.tmp, 'hunyuan3d-2.1')
        res = self.run_with(c, fake_modules(**{'hy3dshape': {}, 'hy3dshape.rembg': {'BackgroundRemover': Rec('BR')}, 'hy3dshape.pipelines': {'Hunyuan3DDiTFlowMatchingPipeline': P}}))
        fp = self.calls('from_pretrained')[0]
        self.assertEqual(fp[1], ('tencent/Hunyuan3D-2.1',))
        self.assertEqual(fp[2]['subfolder'], 'hunyuan3d-dit-v2-1')
        self.assertTrue(self.calls('offload'), 'its model offload on a card under 16 GB')
        call = self.calls('pipe')[0][2]
        self.assertEqual((call['octree_resolution'], call['num_inference_steps']), (320, 40))
        self.assertTrue(res['shapeOnly'], 'Hunyuan3D 2.0 paints it')
        self.assertEqual(c.linked[os.path.join(c.work, 'hy3dgen', 'tencent/Hunyuan3D-2.1')], '/hub/tencent/Hunyuan3D-2.1')
        self.assertTrue(os.path.isfile(c.out))

    def test_trellis2_1024_with_its_low_vram(self):
        mesh = Rec('mesh')
        pipe = Rec('pipe')
        pipe.run = Rec('run', lambda *a, **k: [mesh])
        P = Rec('Trellis2ImageTo3DPipeline')
        P.from_pretrained = Rec('from_pretrained', lambda *a, **k: pipe)
        glb = FakeMesh()
        ov = fake_modules(o_voxel={'postprocess': type('PP', (), {'to_glb': staticmethod(Rec('to_glb', lambda *a, **k: glb))})})['o_voxel']
        c = FakeCtx(self.tmp, 'trellis-2')
        self.run_with(c, dict(fake_modules(**{'trellis2': {}, 'trellis2.pipelines': {'Trellis2ImageTo3DPipeline': P}}), o_voxel=ov))
        self.assertEqual(self.calls('run')[0][2]['pipeline_type'], '1024', "the board's \"Fits at 1024 only\"")
        self.assertIs(pipe.__dict__.get('low_vram'), True)
        self.assertEqual(self.calls('to_glb')[0][2]['texture_size'], 2048)
        self.assertEqual(glb.exported, [c.out])

    def test_trellis_from_words_photo_and_its_splat(self):
        out = {'gaussian': [Rec('g')], 'mesh': [Rec('m')]}
        out['gaussian'][0].save_ply = lambda p: open(p, 'wb').write(b'ply')
        img_pipe, txt_pipe = Rec('img'), Rec('txt')
        img_pipe.run = Rec('img.run', lambda *a, **k: out)
        txt_pipe.run = Rec('txt.run', lambda *a, **k: out)
        I, T = Rec('I'), Rec('T')
        I.from_pretrained = Rec('I.from', lambda *a, **k: img_pipe)
        T.from_pretrained = Rec('T.from', lambda *a, **k: txt_pipe)
        glb = FakeMesh()
        mods = fake_modules(**{'trellis': {}, 'trellis.pipelines': {'TrellisImageTo3DPipeline': I, 'TrellisTextTo3DPipeline': T},
                               'trellis.utils': {'postprocessing_utils': type('U', (), {'to_glb': staticmethod(lambda *a, **k: glb)})}})
        c = FakeCtx(self.tmp, 'trellis')
        res = self.run_with(c, mods)
        self.assertEqual(self.calls('I.from')[0][1], ('/hub/microsoft/TRELLIS-image-large',))
        self.assertEqual(self.calls('img.run')[0][2]['formats'], ['gaussian', 'mesh'], 'no radiance field: within 12 GB')
        self.assertTrue(res['splat'] and os.path.isfile(os.path.join(c.d, 'splat.ply')))
        c2 = FakeCtx(tempfile.mkdtemp(), 'trellis', mode='words', words='a dented knight\'s helmet')
        os.unlink(c2.photo)
        self.run_with(c2, mods)
        self.assertEqual(self.calls('txt.run')[0][1], ("a dented knight's helmet",), 'its own words → 3D')

    def test_step1x3d_offload_on_12_gb_its_own_textures_on_27(self):
        geo = Rec('geo', lambda *a, **k: type('O', (), {'mesh': [FakeMesh()]})())
        geo.enable_model_cpu_offload = Rec('geo.offload')
        geo.to = Rec('geo.to', lambda *a: geo)
        G = Rec('G')
        G.from_pretrained = Rec('G.from', lambda *a, **k: geo)
        tex = Rec('tex', lambda *a, **k: FakeMesh())
        T = Rec('T')
        T.from_pretrained = Rec('T.from', lambda *a, **k: tex)
        mods = fake_modules(**{'step1x3d_geometry': {}, 'step1x3d_geometry.models': {}, 'step1x3d_geometry.models.pipelines': {},
                               'step1x3d_geometry.models.pipelines.pipeline': {'Step1X3DGeometryPipeline': G},
                               'step1x3d_geometry.models.pipelines.pipeline_utils': {'reduce_face': lambda m: m, 'remove_degenerate_face': lambda m: m},
                               'step1x3d_texture': {}, 'step1x3d_texture.pipelines': {}, 'step1x3d_texture.pipelines.step1x_3d_texture_synthesis_pipeline': {'Step1X3DTexturePipeline': T}})
        c = FakeCtx(self.tmp, 'step1x-3d', vram=12.0)
        res = self.run_with(c, mods)
        self.assertEqual(self.calls('G.from')[0][2]['subfolder'], 'Step1X-3D-Geometry-1300m')
        self.assertTrue(self.calls('geo.offload'), "its geometry pipeline's own offload on 12 GB")
        self.assertFalse(self.calls('T.from'), 'its 27 GB texturing isn\'t tried on 12 GB')
        self.assertTrue(res['shapeOnly'] and '27 GB' in res['note'])
        Rec.log = []
        c2 = FakeCtx(tempfile.mkdtemp(), 'step1x-3d', vram=48.0)
        res2 = self.run_with(c2, mods)
        self.assertTrue(self.calls('geo.to') and self.calls('T.from') and not res2['shapeOnly'])

    def test_triposg(self):
        pipe = Rec('pipe')
        pipe.to = Rec('to', lambda *a: pipe)
        P = Rec('P')
        P.from_pretrained = Rec('P.from', lambda *a, **k: pipe)
        rmbg = Rec('rmbg')
        rmbg.to = Rec('rmbg.to', lambda *a: rmbg)
        B = Rec('B')
        B.from_pretrained = Rec('B.from', lambda *a, **k: rmbg)
        mods = fake_modules(**{'triposg': {}, 'triposg.pipelines': {}, 'triposg.pipelines.pipeline_triposg': {'TripoSGPipeline': P}, 'briarmbg': {'BriaRMBG': B},
                               'inference_triposg': {'run_triposg': Rec('run_triposg', lambda *a, **k: FakeMesh())}})
        c = FakeCtx(self.tmp, 'triposg')
        res = self.run_with(c, mods)
        self.assertEqual(self.calls('P.from')[0][1], ('/hub/VAST-AI/TripoSG',))
        self.assertEqual(self.calls('B.from')[0][1], ('/hub/briaai/RMBG-1.4',))
        self.assertEqual(self.calls('run_triposg')[0][2]['seed'], 71204)
        self.assertTrue(res['shapeOnly'])

    def test_direct3d_s2_at_512_on_12_gb(self):
        pipe = Rec('pipe', lambda *a, **k: {'mesh': FakeMesh()})
        pipe.to = Rec('to')
        P = Rec('P')
        P.from_pretrained = Rec('P.from', lambda *a, **k: pipe)
        mods = fake_modules(**{'direct3d_s2': {}, 'direct3d_s2.pipeline': {'Direct3DS2Pipeline': P}})
        for vram, sdf in ((12.0, 512), (32.0, 1024)):
            Rec.log = []
            c = FakeCtx(tempfile.mkdtemp(), 'direct3d-s2', vram=vram)
            self.run_with(c, mods)
            self.assertEqual(self.calls('P.from')[0][1], ('/hub/wushuang98/Direct3D-S2/direct3d-s2-v-1-1',))
            self.assertEqual(self.calls('pipe')[0][2]['sdf_resolution'], sdf)

    def test_hi3dgen_normals_then_its_shape(self):
        m = Rec('m')
        m.to_trimesh = lambda **k: FakeMesh()
        pipe = Rec('pipe')
        pipe.run = Rec('run', lambda *a, **k: {'mesh': [m]})
        pipe.preprocess_image = Rec('pre', lambda *a, **k: FakeImage())
        P = Rec('P')
        P.from_pretrained = Rec('P.from', lambda *a, **k: pipe)
        c = FakeCtx(self.tmp, 'hi3dgen')
        res = self.run_with(c, fake_modules(**{'hi3dgen': {}, 'hi3dgen.pipelines': {'Hi3DGenPipeline': P}}))
        hub = self.calls('torch.hub.load')[0]
        self.assertEqual(hub[1][:2], ('/rt/torch/hub/hugoycj_StableNormal_main', 'StableNormal_turbo'))
        self.assertEqual(hub[2]['source'], 'local', 'StableNormal at its pinned commit, never fetched')
        self.assertEqual(self.calls('P.from')[0][1], (os.path.join(c.work, 'weights', 'trellis-normal-v0-1'),))
        self.assertTrue(res['shapeOnly'] and os.path.isfile(c.out))

    def test_sam_3d_objects_mask_from_the_cutout(self):
        class Arr:
            def __init__(self):
                self.shape = (2, 2, 4)

            def __getitem__(self, k):
                return Arr()

            def __gt__(self, o):
                return 'mask'
        np = type('np', (), {'array': staticmethod(lambda x: Arr())})
        gs = Rec('gs')
        gs.save_ply = lambda p: open(p, 'wb').write(b'ply')
        inf = Rec('inf', lambda *a, **k: {'gs': gs, 'glb': FakeMesh()})
        mods = fake_modules(inference={'Inference': Rec('Inference', lambda *a, **k: inf)}, numpy={'array': np.array})
        c = FakeCtx(self.tmp, 'sam-3d-objects')
        res = self.run_with(c, mods)
        self.assertEqual(self.calls('Inference')[0][1], (os.path.join(c.src, 'checkpoints', 'hf', 'pipeline.yaml'),))
        self.assertEqual(c.linked[os.path.join(c.src, 'checkpoints', 'hf')], '/hub/facebook/sam-3d-objects/checkpoints')
        self.assertEqual(self.calls('inf')[0][1][1], 'mask')
        self.assertTrue(res['splat'] and res['mesh'])

    def test_sam_3d_body_people_upright(self):
        import types
        tm = types.ModuleType('trimesh')
        tm.Trimesh = Rec('Trimesh', lambda *a, **k: FakeMesh())
        tm.util = type('U', (), {'concatenate': staticmethod(lambda ms: ms[0])})

        class V(list):
            def __add__(self, o):
                return V(a + b for a, b in zip(self, o))

            def __mul__(self, o):
                return V(a * b for a, b in zip(self, o))
        np = types.ModuleType('numpy')
        np.array = lambda x: V(x)
        np.asarray = lambda x: V(x)
        est = Rec('est')
        est.process_one_image = Rec('process', lambda *a, **k: [{'pred_vertices': [1.0, 2.0, 3.0], 'pred_cam_t': [0.0, 0.0, 1.0]}])
        est.faces = 'faces'
        mods = fake_modules(**{'sam_3d_body': {'load_sam_3d_body': Rec('load', lambda *a, **k: ('model', 'cfg')), 'SAM3DBodyEstimator': Rec('Est', lambda *a, **k: est)},
                               'tools': {}, 'tools.build_detector': {'HumanDetector': Rec('Det')}, 'tools.build_fov_estimator': {'FOVEstimator': Rec('Fov')}})
        mods.update(trimesh=tm, numpy=np)
        c = FakeCtx(self.tmp, 'sam-3d-body')
        res = self.run_with(c, mods)
        self.assertEqual(self.calls('load')[0][1], ('/hub/facebook/sam-3d-body-dinov3/model.ckpt',))
        self.assertEqual(self.calls('Fov')[0][2]['path'], '/hub/Ruicheng/moge-2-vitl-normal')
        self.assertEqual(list(self.calls('Trimesh')[0][1][0]), [1.0, -2.0, -4.0], 'turned upright')
        self.assertEqual(res['note'], '1 person')

    def test_triposplat_its_splat(self):
        g = Rec('g')
        g.save_ply = lambda p: open(p, 'wb').write(b'ply')
        pipe = Rec('pipe')
        pipe.run = Rec('run', lambda *a, **k: (g, FakeImage()))
        c = FakeCtx(self.tmp, 'triposplat')
        res = self.run_with(c, fake_modules(triposplat={'TripoSplatPipeline': Rec('TP', lambda *a, **k: pipe)}))
        self.assertEqual(self.calls('TP')[0][2]['ckpt_path'], '/hub/VAST-AI/TripoSplat/diffusion_models/triposplat_fp16.safetensors')
        self.assertEqual(self.calls('run')[0][2]['num_gaussians'], 131072)
        self.assertTrue(res['splat'] and not res['mesh'])

    def test_hy_world_worldmirror_in_bf16(self):
        world = os.path.join(self.tmp, 'world-out')
        os.makedirs(world)
        open(os.path.join(world, 'gaussians.ply'), 'wb').write(b'ply')
        pipe = Rec('pipe', lambda *a, **k: world)
        W = Rec('W')
        W.from_pretrained = Rec('W.from', lambda *a, **k: pipe)
        c = FakeCtx(self.tmp, 'hy-world-2')
        res = self.run_with(c, fake_modules(**{'hyworld2': {}, 'hyworld2.worldrecon': {}, 'hyworld2.worldrecon.pipeline': {'WorldMirrorPipeline': W}}))
        k = self.calls('W.from')[0][2]
        self.assertEqual((k['subfolder'], k['enable_bf16']), ('HY-WorldMirror-2.0', True))
        self.assertEqual(self.calls('pipe')[0][2]['target_size'], 700, 'a smaller inference size on 12 GB')
        self.assertTrue(res['splat'])

    def test_a_mesh_file_of_another_kind_becomes_a_glb(self):
        import types
        tm = types.ModuleType('trimesh')
        tm.load = Rec('load', lambda *a, **k: FakeMesh())
        c = FakeCtx(self.tmp, 'trellis')
        obj = os.path.join(self.tmp, 'x.obj')
        open(obj, 'w').close()
        from unittest import mock
        with mock.patch.dict(sys.modules, {'trimesh': tm}):
            res = self.fr.finish(c, {'mesh': obj})
        self.assertEqual(self.calls('load')[0][2], {'force': 'mesh'})
        self.assertTrue(os.path.isfile(c.out) and res['mesh'])
        with self.assertRaises(FakeCtx.Fail):
            self.fr.finish(c, {})


def splat_ply(path, pts, ascii=False):
    """A Gaussian splat as the makers save it (x y z, nx ny nz, f_dc_0..2, opacity, scale, rot)."""
    props = ['x', 'y', 'z', 'nx', 'ny', 'nz', 'f_dc_0', 'f_dc_1', 'f_dc_2', 'opacity', 'scale_0', 'scale_1', 'scale_2', 'rot_0', 'rot_1', 'rot_2', 'rot_3']
    head = 'ply\nformat %s 1.0\nelement vertex %d\n%send_header\n' % ('ascii' if ascii else 'binary_little_endian', len(pts), ''.join('property float %s\n' % p for p in props))
    with open(path, 'wb') as f:
        f.write(head.encode())
        for x, y, z, op in pts:
            row = (x, y, z, 0, 0, 0, (1.0 - 0.5) / 0.28209479177387814, (0.2 - 0.5) / 0.28209479177387814, (0.1 - 0.5) / 0.28209479177387814, op, -5, -5, -5, 1, 0, 0, 0)
            f.write((' '.join('%g' % v for v in row) + '\n').encode() if ascii else struct.pack('<17f', *row))


class Splats(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.h = helper(self.tmp)

    def cube(self, n=24, op=4.0):
        return [(i / n, j / n, k / n, op) for i in range(n) for j in range(n) for k in range(n)]

    def test_its_points_and_colours(self):
        for ascii in (False, True):
            p = os.path.join(self.tmp, 'a.ply')
            splat_ply(p, [(1, 2, 3, 4.0), (4, 5, 6, -4.0)], ascii)
            pts = self.h.ply_points(p)
            self.assertEqual([round(v, 3) for v in pts[0][:6]], [1, 2, 3, 1.0, 0.2, 0.1], 'its colour from the SH DC term')
            self.assertGreater(pts[0][6], 0.95)
            self.assertLess(pts[1][6], 0.05, 'opacity from its logit')

    def test_a_mesh_from_a_splat(self):
        p, g = os.path.join(self.tmp, 'splat.ply'), os.path.join(self.tmp, 'model.glb')
        splat_ply(p, self.cube() + [(9, 9, 9, -6.0)] * 50)
        n = self.h.splat_to_mesh(p, g, up='y', detail='draft')
        self.assertGreater(n, 100)
        js, bn = self.h.glb_read(g)
        self.assertIn('COLOR_0', js['meshes'][0]['primitives'][0]['attributes'], 'the splat\'s colours on the mesh')
        os.makedirs(os.path.join(self.tmp, 'm'))
        shutil.copyfile(g, os.path.join(self.tmp, 'm', 'model.glb'))
        shutil.copyfile(p, os.path.join(self.tmp, 'm', 'splat.ply'))
        self.h.make_preview(os.path.join(self.tmp, 'm'))
        st = json.load(open(os.path.join(self.tmp, 'm', 'stats.json')))
        self.assertTrue(st['closed'], 'a closed mesh: it prints solid')
        self.assertTrue(st['splat'])
        self.assertLess(max(st['size']), 1.2, 'the see-through points (opacity under 0.3) left out')
        pv = json.load(open(os.path.join(self.tmp, 'm', 'preview.json')))
        self.assertEqual(len(pv['s']) // 3, len(pv['sc']) // 3)
        self.assertEqual(pv['sc'][:3], [255, 51, 25], 'the Splat view: its own points and colours')

    def test_a_camera_frame_splat_turned_upright(self):
        p, g = os.path.join(self.tmp, 'splat.ply'), os.path.join(self.tmp, 'model.glb')
        splat_ply(p, [(x, y * 3, z, 4.0) for x, y, z, _ in self.cube(12)])
        self.h.splat_to_mesh(p, g, up='-y', detail='draft')
        js, bn = self.h.glb_read(g)
        a = js['accessors'][0]
        self.assertLess(a['max'][1], 0.3, 'y down in the camera frame: turned the right way up')


class MakeFlow(unittest.TestCase):
    """The service's part of making (run_one) with the model's own program faked: a size check, a refiner's first
    shape, a splat's mesh, Hunyuan3D 2.0 painting a shape-only model."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.h = helper(self.tmp)
        self.h.installed = lambda m, lk=None: True
        self.h.ready_to_make = lambda m, cat, lk=None: True
        self.h.picture_from_words = lambda *a, **k: None
        self.spawned = []

    def job(self, model, **ask):
        d = os.path.join(self.h.FORGE, model + '-job')
        os.makedirs(d)
        open(os.path.join(d, 'photo.png'), 'wb').write(b'png')
        json.dump(dict({'model': model, 'mode': 'photo', 'textures': 'pbr', 'detail': 'draft', 'photos': ['photo.png']}, **ask), open(os.path.join(d, 'ask.json'), 'w'))
        return d

    def fake(self, outputs):
        def spawn(d, rid, args, cat, log='log.txt'):
            self.spawned.append((rid, args))
            for name, how in outputs.get(args[0] + (':' + args[args.index('--out') + 1] if '--out' in args else ''), {}).items():
                how(os.path.join(d, name))
            return 0
        self.h.spawn = spawn

    def test_a_card_too_small_says_so(self):
        os.environ['HEARTH_FORGE_VRAM'] = '12.8'
        try:
            d = self.job('sam-3d-objects')
            self.h.run_one(d)
        finally:
            del os.environ['HEARTH_FORGE_VRAM']
        done = json.load(open(os.path.join(d, 'done.json')))
        self.assertFalse(done['ok'])
        self.assertIn('32 GB', done['error'])
        self.assertIn('bigger PC', done['error'], 'made on a bigger PC of yours over Hearth Link')

    def test_a_splat_made_kept_and_given_a_mesh(self):
        os.environ['HEARTH_FORGE_VRAM'] = '12.8'
        try:
            d = self.job('sharp')
            self.fake({'run': {'splat.ply': lambda p: splat_ply(p, [(i / 10, j / 10, k / 10, 4.0) for i in range(10) for j in range(10) for k in range(10)]),
                               'result.json': lambda p: json.dump({'mesh': False, 'splat': True, 'shapeOnly': False}, open(p, 'w'))}})
            self.h.run_one(d)
        finally:
            del os.environ['HEARTH_FORGE_VRAM']
        done = json.load(open(os.path.join(d, 'done.json')))
        self.assertTrue(done['ok'], done)
        self.assertTrue(os.path.isfile(os.path.join(d, 'model.glb')) and os.path.isfile(os.path.join(d, 'splat.ply')))
        self.assertTrue(json.load(open(os.path.join(d, 'stats.json')))['meshFromSplat'])
        self.assertEqual(self.spawned, [('sharp', ['run', d])])

    def test_a_refiner_starts_from_another_models_shape_and_is_painted(self):
        os.environ['HEARTH_FORGE_VRAM'] = '12.8'
        self.h.painter_ready = lambda cat, lk=None: True
        glb = lambda p: tiny_glb(p)
        try:
            d = self.job('hi3d')
            self.fake({'run:coarse.glb': {'coarse.glb': glb}, 'run': {'model.glb': glb, 'result.json': lambda p: json.dump({'mesh': True, 'shapeOnly': True}, open(p, 'w'))},
                       'paint': {'model.glb': glb}})
            self.h.run_one(d)
        finally:
            del os.environ['HEARTH_FORGE_VRAM']
        done = json.load(open(os.path.join(d, 'done.json')))
        self.assertTrue(done['ok'], done)
        self.assertEqual(self.spawned, [('hunyuan3d2', ['run', d, '--as', 'hunyuan3d-2-mini', '--out', 'coarse.glb', '--textures', 'none']), ('hi3d', ['run', d]),
                                        ('hunyuan3d2', ['paint', d])])
        self.assertIn('textures', done['timing'])
        self.assertTrue(os.path.isfile(os.path.join(d, 'model-shape.glb')), 'its shape kept beside the painted one')

    def test_without_the_painter_it_says_so(self):
        os.environ['HEARTH_FORGE_VRAM'] = '12.8'
        self.h.painter_ready = lambda cat, lk=None: False
        try:
            d = self.job('triposg')
            self.fake({'run': {'model.glb': lambda p: tiny_glb(p), 'result.json': lambda p: json.dump({'mesh': True, 'shapeOnly': True}, open(p, 'w'))}})
            self.h.run_one(d)
        finally:
            del os.environ['HEARTH_FORGE_VRAM']
        done = json.load(open(os.path.join(d, 'done.json')))
        self.assertTrue(done['ok'])
        self.assertIn('Get Hunyuan3D 2.0', done['timing']['note'])

    def test_its_program_runs_offline_with_its_own_settings(self):
        cat = json.loads((APP / 'helper/models.json').read_text())
        env, src = self.h.runner_env('trellis2', cat)
        self.assertEqual((env['HF_HUB_OFFLINE'], env['ATTN_BACKEND'], env['SPARSE_ATTN_BACKEND']), ('1', 'xformers', 'xformers'))
        self.assertEqual(env['TORCH_HOME'], os.path.join(self.h.RUNTIMES, 'trellis2', 'torch'))
        env, src = self.h.runner_env('lyra2', cat)
        self.assertTrue(src.endswith(os.path.join('runtimes', 'lyra2', 'src', 'Lyra-2')), 'Lyra 2.0 runs from its Lyra-2 folder')
        self.assertIn("start_new_session=True", HELPER.read_text(), 'Stop ends the model and everything it started')


class Public(unittest.TestCase):
    """It started private and went public (the owner, Nov 26: "push it to the public")."""
    def test_in_the_public_list(self):
        cat = json.loads((ROOT / 'os/system_files/usr/share/hearth/store/catalog.json').read_text())
        f = [a for a in cat['apps'] if a['id'] == 'forge']
        self.assertEqual(len(f), 1)
        self.assertEqual(f[0]['partId'], 'hearth-forge-app')

    def test_its_store_updates_go_to_the_public_list(self):
        q = (ROOT / '.github/workflows/quick-update.yml').read_text()
        self.assertIn('hearth-forge-app,', q)
        self.assertNotIn('private_apps = ("hearth-forge-app",)', q)

    def test_shown_in_everyones_menu(self):
        d = (ROOT / 'os/system_files/usr/share/applications/hearth-forge.desktop').read_text()
        self.assertNotIn('NoDisplay=true', d)


class PipCommand(unittest.TestCase):
    """The owner, Nov 28: every model failed at "downloading PyTorch for CUDA 12.8" with "pip: no such option:
    --progress-bar" (it stood before 'install'). The real command line is run by pip itself (--dry-run, offline)."""

    def test_progress_bar_after_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            m = helper(tmp)
            c = m.pip_install_cmd('python3', 'c.txt', ['--find-links', 'x'])
            self.assertLess(c.index('install'), c.index('--progress-bar'), ' '.join(c))
            self.assertEqual(c[-2:], ['--find-links', 'x'])

    def test_pip_accepts_the_command_line(self):
        import subprocess
        if subprocess.run([sys.executable, '-m', 'pip', '--version'], capture_output=True).returncode != 0:
            self.skipTest('no pip here')
        with tempfile.TemporaryDirectory() as tmp:
            m = helper(tmp)
            cons = os.path.join(tmp, 'constraints.txt')
            pathlib.Path(cons).write_text('torch==2.7.1\n')
            links = os.path.join(tmp, 'links')
            os.mkdir(links)
            c = m.pip_install_cmd(sys.executable, cons, ['--find-links', links])
            r = subprocess.run(c + ['--dry-run', '--no-index', 'pip'], capture_output=True, text=True, timeout=300)
            out = r.stdout + r.stderr
            self.assertNotIn('no such option', out)
            self.assertEqual(r.returncode, 0, out[-600:])


class OwnersFixes(unittest.TestCase):
    """The owner's fixes from the Nvidia PC (Nov 28): Hunyuan3D 2's transformers held below 4.50, a changed install
    list installs again, no "people" in the negative words when a character is asked for, out of memory said plainly."""

    def test_hunyuan3d2_transformers_below_4_50(self):
        self.assertIn('transformers>=4.48.0,<4.50', MODELS['runners']['hunyuan3d2']['pip'])

    def test_changed_pip_list_isnt_ready(self):
        with tempfile.TemporaryDirectory() as tmp:
            m = helper(tmp)
            cat = {'runners': {'r': {'commit': 'abc', 'pip': ['a', 'b<2']}}}
            d = m.runner_dir('r')
            py = m.runner_python('r')
            os.makedirs(os.path.dirname(py), exist_ok=True)
            pathlib.Path(py).write_text('')
            m.write_json(os.path.join(d, 'ready.json'), {'commit': 'abc', 'pip': ['a', 'b<2']})
            self.assertTrue(m.runner_ready('r', cat))
            cat['runners']['r']['pip'] = ['a', 'b<3']
            self.assertFalse(m.runner_ready('r', cat))
            m.write_json(os.path.join(d, 'ready.json'), {'commit': 'abc'})
            self.assertTrue(m.runner_ready('r', cat))

    def test_negative_words(self):
        with tempfile.TemporaryDirectory() as tmp:
            m = helper(tmp)
            self.assertNotIn('people', m.negative_words({'words': 'a sci-fi soldier character'}))
            self.assertIn('people', m.negative_words({'words': 'a wooden chair'}))

    def test_out_of_memory_said_plainly(self):
        with tempfile.TemporaryDirectory() as tmp:
            m = helper(tmp)
            m.gpu_users = lambda: ['python3 main.py --listen (9000 MB)']
            self.assertIn('Also using it: python3 main.py', m.out_of_memory_words('torch.OutOfMemoryError: CUDA out of memory.'))
            self.assertIsNone(m.out_of_memory_words('some other error'))


if __name__ == '__main__':
    unittest.main()
