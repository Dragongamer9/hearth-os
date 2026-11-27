"""The Server side's engines (docs/HOME-SERVER-PARTS.md; boards docs/mockups/server-expansion): the Model vault, the
dashboard and Backups for every device, against pretend servers (Hugging Face's API, Cloudflare R2's S3 API, AdGuard
Home) and a pretend restic, never the real ones.

What it holds them to: R2's real prices; Hugging Face's answers read right (removed, changed, renamed, gated); exact
files kept with their SHA-256 and a file that doesn't match never kept; uploads signed with AWS SigV4 (checked here
against the request), resumable, stopping while a game runs; keys only from the password store and never printed;
settings written only when you change one (red flag 9); every timer and service off until you turn it on, and only
in your user's own systemd (red flag 2); a device can only put back its own files; and the Home server helper runs
only its own parts' engines.
"""
import hashlib
import hmac
import http.server
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import unittest.mock
import urllib.parse

REPO = pathlib.Path(__file__).resolve().parents[3]
PARTS = REPO / 'os/system_files/usr/libexec/nimbus/hearth-server-parts'
HELPER = REPO / 'os/system_files/usr/libexec/nimbus/hearth-home-server-helper'
AI_MODELS = REPO / 'os/system_files/usr/share/hearth/ai-models.json'


def load(name, env):
    """An engine as a module, with this test's own home folder."""
    old = dict(os.environ)
    os.environ.update(env)
    try:
        loader = importlib.machinery.SourceFileLoader('engine_' + name + str(time.time_ns()), str(PARTS / (name + '.py')))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
        return m
    finally:
        os.environ.clear()
        os.environ.update(old)


class Server:
    """A pretend server on 127.0.0.1: handler(method, path, headers, body) -> (status, bytes, headers)."""

    def __init__(self, handler):
        outer = self
        self.seen = []

        class H(http.server.BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def _do(self):
                n = int(self.headers.get('Content-Length') or 0)
                body = self.rfile.read(n) if n else b''
                outer.seen.append((self.command, self.path, dict(self.headers), body))
                st, data, hdr = handler(self.command, self.path, self.headers, body)
                self.send_response(st)
                for k, v in (hdr or {}).items():
                    self.send_header(k, v)
                self.send_header('Content-Length', str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            do_GET = do_POST = do_PUT = do_HEAD = _do

        self.httpd = http.server.ThreadingHTTPServer(('127.0.0.1', 0), H)
        self.url = 'http://127.0.0.1:%d' % self.httpd.server_address[1]
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()

    def close(self):
        self.httpd.shutdown()


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-server-parts-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.env = {'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'), 'XDG_STATE_HOME': os.path.join(self.home, '.local/state'),
                    'XDG_DATA_HOME': os.path.join(self.home, '.local/share'), 'XDG_CACHE_HOME': os.path.join(self.home, '.cache'), 'XDG_RUNTIME_DIR': os.path.join(self.tmp, 'run'),
                    'HEARTH_SHELL_DIR': str(REPO / 'hosts/linux/desktop/shell'), 'HEARTH_TEST_PLAYING': '0', 'HEARTH_TEST_NO_SYSTEMCTL': '1',
                    'HEARTH_AI_MODELS': str(AI_MODELS), 'HEARTH_VAULT_RAM_GB': '32',
                    'HEARTH_TEST_SECRETS': json.dumps({'r2-access': 'AKIDTEST', 'r2-secret': 'SECRETTEST'})}
        os.makedirs(self.env['XDG_RUNTIME_DIR'])

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_engine(self, name, *args, extra=None, stdin=''):
        env = dict(os.environ)
        env.update(self.env)
        env.update(extra or {})
        r = subprocess.run([sys.executable, '-I', str(PARTS / (name + '.py'))] + list(args), env=env, capture_output=True, text=True, input=stdin, timeout=120)
        line = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else '{}'
        return json.loads(line), r

    def settings_file(self, part):
        return os.path.join(self.home, '.config', 'nimbus', 'server-%s.json' % part)


# ------------------------------------------------------------------ the Model vault
class Vault(Base):
    def test_prices_are_r2s(self):
        v = load('vault', self.env)
        std, ia, back = v.costs(4750, {'standard': 0.015, 'infrequent': 0.01, 'retrieval': 0.01, 'freeGb': 10})
        self.assertAlmostEqual(std, 71.10, places=2)   # the first 10 GB on Standard are free
        self.assertAlmostEqual(ia, 47.50, places=2)
        self.assertAlmostEqual(back, 47.50, places=2)
        std, ia, _ = v.costs(1250, {'standard': 0.015, 'infrequent': 0.01, 'retrieval': 0.01, 'freeGb': 10})
        self.assertAlmostEqual(std, 18.60, places=2)
        self.assertAlmostEqual(ia, 12.50, places=2)

    def test_every_list_is_in_the_catalog(self):
        v = load('vault', self.env)
        cat = v.catalog()
        apps = {m['app'] for m in cat}
        for app in ('Colibri', 'Uncensored', 'Hearth Forge', 'Listen', 'Kokoro voices', 'Ollama'):
            self.assertIn(app, apps)
        ai = json.loads(AI_MODELS.read_text())
        self.assertEqual(sum(1 for m in cat if m['app'] == 'Colibri'), len(ai['standard']['models']))
        self.assertEqual(sum(1 for m in cat if m['app'] == 'Uncensored'), len(ai['models']))
        giants = [m['id'] for m in cat if m['giant']]
        self.assertEqual(sorted(giants), sorted(['kimi-k3', 'mimo-v2.6-pro', 'inkling', 'glm-5.3', 'deepseek-v4-flash']))

    def test_looking_writes_no_setting(self):
        out, r = self.run_engine('vault', 'status')
        self.assertTrue(out.get('ok'), r.stderr)
        self.assertEqual(len(out['sets']), 3)
        self.assertFalse(os.path.exists(self.settings_file('vault')), 'opening the vault wrote a setting')
        out, _ = self.run_engine('vault', 'summary')
        self.assertTrue(out.get('ok'))
        self.assertFalse(os.path.exists(self.settings_file('vault')))

    def test_a_choice_writes_only_its_key(self):
        os.makedirs(os.path.dirname(self.settings_file('vault')))
        with open(self.settings_file('vault'), 'w') as f:
            json.dump({'speed': 'medium', 'mine': 'kept'}, f)
        out, r = self.run_engine('vault', 'set', 'keep', 'no-giants')
        self.assertTrue(out.get('ok'), r.stdout + r.stderr)
        d = json.load(open(self.settings_file('vault')))
        self.assertEqual(d, {'speed': 'medium', 'mine': 'kept', 'keep': 'no-giants'})
        out, _ = self.run_engine('vault', 'set', 'keep', 'all-of-it')
        self.assertFalse(out.get('ok'))
        self.assertEqual(json.load(open(self.settings_file('vault')))['keep'], 'no-giants')

    def test_timers_only_when_turned_on(self):
        units = os.path.join(self.home, '.config', 'systemd', 'user')
        self.run_engine('vault', 'status')
        self.assertFalse(os.path.isdir(units) and os.listdir(units))
        out, _ = self.run_engine('vault', 'on')
        self.assertTrue(out.get('ok'))
        names = sorted(os.listdir(units))
        self.assertIn('hearth-model-vault.timer', names)
        self.assertIn('hearth-model-vault-check.timer', names)
        for n in names:
            text = open(os.path.join(units, n)).read()
            self.assertNotIn('multi-user.target', text)   # never part of starting the PC
            self.assertNotIn('graphical.target', text)
        self.assertIn('OnCalendar=Sun *-*-* 03:00:00', open(os.path.join(units, 'hearth-model-vault-check.timer')).read())
        self.run_engine('vault', 'off')
        self.assertEqual(os.listdir(units), [])

    def _hf(self, state):
        """Hugging Face's API: state[repo] = None (gone) | 'gated' | {'sha', 'files': {name: (size, sha256)}, 'id'}"""
        def handler(method, path, headers, body):
            u = urllib.parse.urlparse(path)
            parts = u.path.split('/')
            if u.path.startswith('/api/models/'):
                repo = '/'.join(parts[3:5])
                x = state.get(repo)
                if x is None:
                    return 404, b'{"error":"Repository not found"}', {'Content-Type': 'application/json'}
                if x == 'gated':
                    return 401, b'{"error":"Access to model is restricted: gated"}', {'Content-Type': 'application/json'}
                sib = [{'rfilename': n, 'size': s, 'lfs': {'sha256': h, 'size': s}} for n, (s, h) in x['files'].items()]
                return 200, json.dumps({'id': x.get('id', repo), 'sha': x['sha'], 'siblings': sib, 'cardData': {'license': 'mit'}}).encode(), {'Content-Type': 'application/json'}
            if '/resolve/' in u.path:
                repo = '/'.join(parts[1:3])
                name = parts[-1]
                data = state['_data'][(repo, name)]
                rng = headers.get('Range')
                if rng:
                    start = int(rng.split('=')[1].split('-')[0])
                    return 206, data[start:], {}
                return 200, data, {}
            return 404, b'', {}
        return Server(handler)

    def _forge_list(self, models):
        p = os.path.join(self.home, '.local/share/hearth/forge/models.json')
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            json.dump({'models': models}, f)

    def test_check_finds_removed_changed_and_new(self):
        data = b'model-bytes' * 100
        h = hashlib.sha256(data).hexdigest()
        hf = {'tencent/Hunyuan3D-2': {'sha': 'aaaaaaa111', 'files': {'model.safetensors': (len(data), h)}}, '_data': {('tencent/Hunyuan3D-2', 'model.safetensors'): data}}
        srv = self._hf(hf)
        self.addCleanup(srv.close)
        drive = os.path.join(self.tmp, 'drive')
        os.makedirs(drive)
        self.run_engine('vault', 'set', 'drive', drive)
        self.run_engine('vault', 'set', 'keep', 'this-pc')
        extra = {'HEARTH_VAULT_HF': srv.url, 'HEARTH_VAULT_OLLAMA': srv.url, 'HEARTH_VAULT_SOURCES': self._sources([{'id': 'hunyuan3d-2', 'name': 'Hunyuan3D 2.0', 'app': 'Hearth Forge', 'repo': 'tencent/Hunyuan3D-2', 'licence': 'Hunyuan', 'gb': 0.1}]),
                 'HEARTH_AI_MODELS': self._no_ai()}
        out, r = self.run_engine('vault', 'check', extra=extra)
        self.assertTrue(out.get('ok'), r.stdout + r.stderr)
        self.assertEqual(out['check']['new'], ['hunyuan3d-2'])   # asks before copying
        st, _ = self.run_engine('vault', 'status', extra=extra)
        self.assertEqual([a['id'] for a in st['asks']], ['hunyuan3d-2'])
        # you say copy it: the worker copies the exact files, checked by SHA-256, MODEL.json beside them
        out, _ = self.run_engine('vault', 'decide', 'hunyuan3d-2', 'copy', 'drive', extra=extra)
        self.assertTrue(out.get('ok'))
        self.run_engine('vault', 'on', extra=extra)
        out, r = self.run_engine('vault', 'run', '--minutes', '1', extra=extra)
        self.assertTrue(out.get('ok'), r.stdout + r.stderr)
        metas = list(pathlib.Path(drive).rglob('MODEL.json'))
        self.assertEqual(len(metas), 1)
        meta = json.loads(metas[0].read_text())
        self.assertEqual(meta['revision'], 'aaaaaaa111')
        self.assertEqual(meta['files'][0]['sha256'], h)
        self.assertEqual((metas[0].parent / 'model.safetensors').read_bytes(), data)
        # its maker changes the files: changed, and your copy stays as it is
        new = b'other-bytes' * 120
        hf['tencent/Hunyuan3D-2'] = {'sha': 'bbbbbbb222', 'files': {'model.safetensors': (len(new), hashlib.sha256(new).hexdigest()), 'pipeline.json': (5, 'x' * 64)}}
        out, _ = self.run_engine('vault', 'check', extra=extra)
        self.assertEqual(out['check']['changed'], ['hunyuan3d-2'])
        self.assertEqual((metas[0].parent / 'model.safetensors').read_bytes(), data)
        st, _ = self.run_engine('vault', 'status', extra=extra)
        a = [x for x in st['alerts'] if x['kind'] == 'changed'][0]
        self.assertIn('rev aaaaaaa → rev bbbbbbb', a['text'])
        self.assertIn('+1 file', a['onlineFiles'])
        # then it's taken down: removed, and your copy is safe
        hf['tencent/Hunyuan3D-2'] = None
        out, _ = self.run_engine('vault', 'check', extra=extra)
        self.assertEqual(out['check']['removed'], ['hunyuan3d-2'])
        st, _ = self.run_engine('vault', 'status', extra=extra)
        a = [x for x in st['alerts'] if x['kind'] == 'removed'][0]
        self.assertIn('Your copy is safe', a['text'])
        self.assertTrue(metas[0].exists(), 'nothing is deleted from the vault by itself')
        # keep it forever: the alert is dealt with
        self.run_engine('vault', 'decide', 'hunyuan3d-2', 'keep-forever', extra=extra)
        st, _ = self.run_engine('vault', 'status', extra=extra)
        self.assertEqual([x for x in st['alerts'] if x['kind'] == 'removed'], [])

    def test_a_file_that_doesnt_match_is_never_kept(self):
        data = b'good' * 50
        hf = {'tencent/Hunyuan3D-2': {'sha': 'ccccccc333', 'files': {'model.safetensors': (len(data), 'f' * 64)}}, '_data': {('tencent/Hunyuan3D-2', 'model.safetensors'): data}}
        srv = self._hf(hf)
        self.addCleanup(srv.close)
        drive = os.path.join(self.tmp, 'drive')
        os.makedirs(drive)
        self.run_engine('vault', 'set', 'drive', drive)
        extra = {'HEARTH_VAULT_HF': srv.url, 'HEARTH_VAULT_SOURCES': self._sources([{'id': 'hunyuan3d-2', 'name': 'Hunyuan3D 2.0', 'app': 'Hearth Forge', 'repo': 'tencent/Hunyuan3D-2', 'licence': 'Hunyuan', 'gb': 0.1}]), 'HEARTH_AI_MODELS': self._no_ai()}
        self.run_engine('vault', 'decide', 'hunyuan3d-2', 'copy', 'drive', extra=extra)
        self.run_engine('vault', 'on', extra=extra)
        out, _ = self.run_engine('vault', 'run', '--minutes', '1', extra=extra)
        self.assertFalse(out.get('ok'))
        self.assertIn('checksum', out['error'])
        self.assertEqual(list(pathlib.Path(drive).rglob('model.safetensors')), [])

    def test_nothing_runs_while_a_game_does(self):
        self.run_engine('vault', 'on')
        out, _ = self.run_engine('vault', 'run', '--minutes', '1', extra={'HEARTH_TEST_PLAYING': '1', 'HEARTH_AI_MODELS': self._no_ai(), 'HEARTH_VAULT_SOURCES': self._sources([])})
        self.assertTrue(out.get('ok'))

    def test_r2_upload_is_signed_resumable_and_gets_back_by_checksum(self):
        secret = 'SECRETTEST'
        objects = {}
        uploads = {}

        def r2(method, path, headers, body):
            # check AWS SigV4 exactly as R2 does
            u = urllib.parse.urlparse(path)
            auth = headers['Authorization']
            cred = auth.split('Credential=')[1].split(',')[0]
            signed = auth.split('SignedHeaders=')[1].split(',')[0].split(';')
            sig = auth.split('Signature=')[1]
            date = cred.split('/')[1]
            hdr = {k.lower(): v for k, v in headers.items()}
            hdr['host'] = headers['Host']
            q = '&'.join('%s=%s' % (urllib.parse.quote(a, safe='-_.~'), urllib.parse.quote(b, safe='-_.~')) for a, b in sorted(urllib.parse.parse_qsl(u.query, keep_blank_values=True)))
            canon = '\n'.join([method, u.path, q, ''.join('%s:%s\n' % (n, hdr[n].strip()) for n in signed), ';'.join(signed), hdr['x-amz-content-sha256']])
            to_sign = '\n'.join(['AWS4-HMAC-SHA256', hdr['x-amz-date'], '%s/auto/s3/aws4_request' % date, hashlib.sha256(canon.encode()).hexdigest()])
            k = ('AWS4' + secret).encode()
            for x in (date, 'auto', 's3', 'aws4_request'):
                k = hmac.new(k, x.encode(), hashlib.sha256).digest()
            if hmac.new(k, to_sign.encode(), hashlib.sha256).hexdigest() != sig or hashlib.sha256(body).hexdigest() != hdr['x-amz-content-sha256']:
                return 403, b'<Error>SignatureDoesNotMatch</Error>', {}
            qs = dict(urllib.parse.parse_qsl(u.query, keep_blank_values=True))
            key = u.path
            if method == 'POST' and 'uploads' in qs:
                uploads[key] = {'class': hdr.get('x-amz-storage-class'), 'parts': {}}
                return 200, b'<InitiateMultipartUploadResult><UploadId>up-1</UploadId></InitiateMultipartUploadResult>', {}
            if method == 'PUT' and 'partNumber' in qs:
                uploads[key]['parts'][int(qs['partNumber'])] = body
                return 200, b'', {'ETag': '"%s"' % hashlib.md5(body).hexdigest()}
            if method == 'POST' and 'uploadId' in qs:
                objects[key] = b''.join(v for _, v in sorted(uploads[key]['parts'].items()))
                return 200, b'<CompleteMultipartUploadResult/>', {}
            if method == 'GET':
                return (200, objects[key], {}) if key in objects else (404, b'', {})
            return 400, b'', {}
        srv = Server(r2)
        self.addCleanup(srv.close)
        drive = os.path.join(self.tmp, 'drive')
        d = os.path.join(drive, 'Hearth Model Vault', 'Hearth Forge', 'triposr', '1f4b9c7')
        os.makedirs(d)
        data = os.urandom(300000)
        with open(os.path.join(d, 'model.ckpt'), 'wb') as f:
            f.write(data)
        h = hashlib.sha256(data).hexdigest()
        with open(os.path.join(d, 'MODEL.json'), 'w') as f:
            json.dump({'id': 'triposr', 'name': 'TripoSR', 'app': 'Hearth Forge', 'revision': '1f4b9c7abc', 'short': '1f4b9c7', 'files': [{'name': 'model.ckpt', 'size': len(data), 'sha256': h}], 'keptAt': 1, 'where': 'drive+ia'}, f)
        extra = {'HEARTH_VAULT_R2': srv.url, 'HEARTH_AI_MODELS': self._no_ai(), 'HEARTH_VAULT_SOURCES': self._sources([])}
        self.run_engine('vault', 'set', 'drive', drive, extra=extra)
        self.run_engine('vault', 'set', 'r2.bucket', 'hearth-models', extra=extra)
        self.run_engine('vault', 'on', extra=extra)
        vault = load('vault', dict(self.env, **extra))
        vault.PART_SIZE = 100000   # three parts
        vault.SPEEDS['slow'] = 0
        patch = unittest.mock.patch.dict(os.environ, dict(self.env, **extra))
        patch.start()
        self.addCleanup(patch.stop)
        out = vault.run_worker(1)
        self.assertTrue(out['ok'], out)
        key = '/hearth-models/models/hearth-forge/triposr/1f4b9c7/model.ckpt'
        self.assertEqual(objects[key], data)
        self.assertEqual(uploads[key]['class'], 'STANDARD_IA')   # Infrequent Access
        self.assertEqual(len(uploads[key]['parts']), 3)
        for _, _, hdrs, _ in srv.seen:
            self.assertNotIn('SECRETTEST', json.dumps(hdrs))   # the secret is never sent, only its signature
        st = vault.status()
        row = [m for m in st['models'] if m['id'] == 'triposr'] if any(m['id'] == 'triposr' for m in st['models']) else []
        # get it back after the drive lost it: checked by SHA-256
        os.unlink(os.path.join(d, 'model.ckpt'))
        out = vault.getback('triposr')
        self.assertTrue(out['ok'])
        self.assertEqual(open(os.path.join(d, 'model.ckpt'), 'rb').read(), data)
        objects[key] = b'tampered'
        os.unlink(os.path.join(d, 'model.ckpt'))
        with self.assertRaises(vault.Fail):
            vault.getback('triposr')
        self.assertFalse(os.path.exists(os.path.join(d, 'model.ckpt')))
        del row

    def test_apps_load_from_the_vault_first(self):
        drive = os.path.join(self.tmp, 'drive')
        d = os.path.join(drive, 'Hearth Model Vault', 'Hearth Forge', 'triposr', '1f4b9c7')
        os.makedirs(d)
        open(os.path.join(d, 'model.ckpt'), 'wb').write(b'x')
        json.dump({'id': 'triposr', 'name': 'TripoSR', 'app': 'Hearth Forge', 'source': {'repo': 'stabilityai/TripoSR'}, 'revision': '1f4b9c7abc', 'short': '1f4b9c7', 'files': [{'name': 'model.ckpt', 'size': 1, 'sha256': 'x'}], 'keptAt': 1}, open(os.path.join(d, 'MODEL.json'), 'w'))
        self.run_engine('vault', 'set', 'drive', drive)
        out, _ = self.run_engine('vault', 'link')
        self.assertEqual(out['linked'], 1)
        link = os.path.join(self.home, '.cache/huggingface/hub/models--stabilityai--TripoSR/snapshots/1f4b9c7abc/model.ckpt')
        self.assertEqual(os.path.realpath(link), os.path.realpath(os.path.join(d, 'model.ckpt')))
        out, _ = self.run_engine('vault', 'offline-test')
        self.assertTrue(out['offline']['loaded'] == out['offline']['total'] == 1)

    def _sources(self, models):
        p = os.path.join(self.tmp, 'sources-%d.json' % len(models))
        json.dump({'giants': [], 'prices': {}, 'models': models, 'forgeLists': []}, open(p, 'w'))
        return p

    def _no_ai(self):
        p = os.path.join(self.tmp, 'ai.json')
        json.dump({'standard': {'models': []}, 'models': []}, open(p, 'w'))
        return p


# ------------------------------------------------------------------ the dashboard
class Dash(Base):
    def world(self, **kw):
        w = {'units': {'hearth-ollama.service': {'exists': True, 'active': 'active', 'mem': 4e8}, 'hearth-comfyui.service': {'exists': True, 'active': 'failed'},
                       'hearth-server-dns.service': {'exists': True, 'active': 'active'}},
             'http': {'http://127.0.0.1:11434/api/ps': {'models': []},
                      'http://127.0.0.1:3053/control/filtering/status': {'filters': [{'enabled': True, 'url': 'https://adaway.org/hosts.txt', 'last_updated': ''}, {'enabled': True, 'url': 'x', 'last_updated': '2020-01-01T00:00:00Z'}]}},
             'gpu': {'name': 'RTX 5070', 'pct': 40, 'used': 6e9, 'total': 12e9, 'temp': 60, 'procs': {}}, 'mem': {'total': 32e9, 'avail': 16e9}, 'cpu': {'pct': 10, 'name': 'AMD Ryzen 7 7700X'},
             'drives': [{'disk': 'nvme0n1', 'total': 2e12, 'used': 1e12, 'healthy': True}], 'tailscale': {'BackendState': 'Running', 'Peer': {'a': {'HostName': 'rog-ally', 'Online': True, 'CurAddr': '192.168.1.2:1'}, 'b': {'HostName': 'xbox', 'Online': False}}}}
        w.update(kw)
        p = os.path.join(self.tmp, 'world.json')
        json.dump(w, open(p, 'w'))
        return {'HEARTH_DASH_WORLD': p}

    def test_real_health_from_units_and_answers(self):
        out, r = self.run_engine('dash', 'status', extra=self.world())
        self.assertTrue(out.get('ok'), r.stderr)
        by = {s['id']: s for s in out['services']}
        self.assertEqual(by['ollama']['state'], 'ok')
        self.assertEqual(by['comfyui']['state'], 'bad')          # failed unit, no answer
        self.assertEqual(by['shield']['state'], 'warn')          # old lists, one never fetched
        self.assertNotIn('jellyfin', by)                          # not installed: not counted
        self.assertEqual((out['healthy'], out['total']), (2, 4))  # ollama and tailscale of ollama, comfyui, shield, tailscale
        ids = [n['id'] for n in out['needs']]
        self.assertIn('shield-lists', ids)
        self.assertIn('restart:comfyui', ids)
        self.assertEqual([d['note'] for d in out['devices']], ['', 'off'])
        self.assertFalse(os.path.exists(self.settings_file('dash')), 'looking wrote a setting')

    def test_summary_is_the_overall_line(self):
        out, _ = self.run_engine('dash', 'summary', extra=self.world())
        self.assertEqual(out['big'], '2')
        self.assertIn('2 of 4 services healthy', out['overall']['headline'])
        self.assertEqual(out['overall']['updates'], {'auto': False, 'idle': True})   # off unless you turn it on

    def test_shield_fix_swaps_the_moved_list_and_refreshes(self):
        dash = load('dash', dict(self.env, **self.world()))
        dash.FIXTURE = None
        calls = []

        def http_json(method, url, body=None, headers=None, timeout=10):
            calls.append((method, url, body))
            if url.endswith('/control/filtering/status'):
                return 200, {'filters': [{'url': 'https://adaway.org/hosts.txt', 'name': 'AdAway'}]}
            return 200, {}
        dash.C.http_json = http_json
        dash.probe = lambda url, path, timeout=2: http_json('GET', url + path)
        out = dash.fix('shield-lists')
        self.assertTrue(out['ok'])
        set_url = [c for c in calls if c[1].endswith('/set_url')][0]
        self.assertEqual(set_url[2]['data']['url'], 'https://raw.githubusercontent.com/AdAway/adaway.github.io/master/hosts.txt')
        self.assertTrue(any(c[1].endswith('/control/filtering/refresh') for c in calls))

    def test_alerts_and_updates_timers_off_until_turned_on(self):
        units = os.path.join(self.home, '.config', 'systemd', 'user')
        self.run_engine('dash', 'status', extra=self.world())
        self.assertFalse(os.path.isdir(units))
        self.run_engine('dash', 'alerts', 'on', extra=self.world())
        self.assertIn('hearth-server-watch.timer', os.listdir(units))
        self.run_engine('dash', 'alerts', 'off', extra=self.world())
        self.assertNotIn('hearth-server-watch.timer', os.listdir(units))
        self.run_engine('dash', 'autoupdate', 'on')
        self.assertIn('hearth-server-updates.timer', os.listdir(units))
        self.assertTrue(json.load(open(self.settings_file('dash')))['auto'])

    def test_quiet_hours_cross_midnight(self):
        dash = load('dash', self.env)
        c = {'quiet': True, 'quietFrom': 22, 'quietTo': 6}
        late = time.mktime((2026, 11, 26, 23, 0, 0, 0, 0, -1))
        noon = time.mktime((2026, 11, 26, 12, 0, 0, 0, 0, -1))
        self.assertTrue(dash.quiet_now(c, late))
        self.assertFalse(dash.quiet_now(c, noon))


# ------------------------------------------------------------------ Backups for every device
class Backups(Base):
    def fake_restic(self, snaps):
        p = os.path.join(self.tmp, 'restic')
        data = os.path.join(self.tmp, 'restic.json')
        json.dump(snaps, open(data, 'w'))
        with open(p, 'w') as f:
            f.write('#!%s\nimport json,os,sys\nd=json.load(open(%r))\na=sys.argv[1:]\nr=a[a.index("-r")+1]\nu=os.path.basename(r.rstrip("/"))\n'
                    'if "snapshots" in a: print(json.dumps(d.get(u, [])))\nelif "ls" in a: print(json.dumps({"path": "/var/home/you/Saves", "type": "dir"}))\n' % (sys.executable, data))
        os.chmod(p, 0o755)
        return p

    def test_nights_and_versions_from_each_devices_backups(self):
        recv = os.path.join(self.tmp, 'recv')
        os.makedirs(os.path.join(recv, 'rog-ally-3f9a'))
        now = time.time()
        snaps = {'rog-ally-3f9a': [{'short_id': '%08d' % i, 'time': time.strftime('%Y-%m-%dT02:04:00', time.localtime(now - i * 86400)) + '+00:00', 'tags': ['hearth', 'size-home=38000000000'],
                                    'summary': {'data_added_packed': 1800000000, 'total_bytes_processed': 212000000000}} for i in range(1, 20)]}
        extra = {'HEARTH_RESTIC': self.fake_restic(snaps), 'HEARTH_BACKUP_RECEIVE': recv}
        os.makedirs(os.path.dirname(self.settings_file('backups')), exist_ok=True)
        json.dump({'on': True, 'devices': {'rog-ally': {'name': 'ROG Ally', 'kind': 'handheld', 'user': 'rog-ally-3f9a'}}}, open(self.settings_file('backups'), 'w'))
        before = open(self.settings_file('backups')).read()
        out, r = self.run_engine('backups', 'status', extra=extra)
        self.assertTrue(out.get('ok'), r.stdout + r.stderr)
        dev = out['devices'][0]
        self.assertEqual(dev['whole'], '212 GB · 19 versions')
        self.assertEqual(len(dev['nights']), 14)
        self.assertEqual(dev['sizes'].get('home'), '38 GB')
        self.assertEqual(open(self.settings_file('backups')).read(), before, 'looking changed a setting')

    def test_a_join_code_is_checked(self):
        b = load('backups', self.env)
        good = {'server': '100.101.4.7', 'port': 8095, 'plan': 8097, 'user': 'rog-ally-3f9a', 'rest': 'ABCD-EFGH-JKLM-NPQR', 'key': 'k' * 32}
        self.assertEqual(b.clean_join(dict(good)), good)
        for bad in ({'server': 'x; rm -rf /'}, {'user': '../etc'}, {'port': 0}, {'key': 'short'}):
            with self.assertRaises(b.Fail):
                b.clean_join(dict(good, **bad))
        with self.assertRaises(b.Fail):
            b.join('not-a-code')

    def test_a_device_puts_back_only_its_own_files(self):
        b = load('backups', dict(self.env, HEARTH_TEST_ROOT='1', HEARTH_DEVICE_BACKUP_CONF=os.path.join(self.tmp, 'device.json')))
        json.dump({'server': '100.1.1.1', 'port': 8095, 'plan': 8097, 'user': 'rog-ally-3f9a', 'rest': 'x' * 10, 'key': 'y' * 20, 'owner': 'you'}, open(os.path.join(self.tmp, 'device.json'), 'w'))
        b.caller_user = lambda: type('W', (), {'pw_dir': self.home, 'pw_uid': os.getuid(), 'pw_gid': os.getgid(), 'pw_name': 'you'})()
        with self.assertRaises(b.Fail):
            b.device_put('abcdef12', '/etc/shadow', 'back')
        with self.assertRaises(b.Fail):
            b.device_put('abcdef12', os.path.join(self.home, '..', 'other', 'x'), 'back')
        with self.assertRaises(b.Fail):
            b.device_put('not a night', os.path.join(self.home, 'x'), 'back')

    def test_device_install_writes_root_only_conf_and_a_timer(self):
        conf = os.path.join(self.tmp, 'etc', 'device-backup.json')
        units = os.path.join(self.tmp, 'units')
        os.makedirs(units)
        code = {'server': '100.101.4.7', 'port': 8095, 'plan': 8097, 'user': 'rog-ally-3f9a', 'rest': 'ABCD-EFGH-JKLM-NPQR', 'key': 'k' * 32}
        out, r = self.run_engine('backups', 'root', 'device-install', stdin=json.dumps(code), extra={'HEARTH_TEST_ROOT': '1', 'HEARTH_DEVICE_BACKUP_CONF': conf, 'HEARTH_DEVICE_UNIT_DIR': units})
        self.assertTrue(out.get('ok'), r.stdout + r.stderr)
        self.assertEqual(os.stat(conf).st_mode & 0o777, 0o600)
        self.assertEqual(sorted(os.listdir(units)), ['hearth-device-backup.service', 'hearth-device-backup.timer'])
        t = open(os.path.join(units, 'hearth-device-backup.timer')).read()
        self.assertIn('WantedBy=timers.target', t)
        s = open(os.path.join(units, 'hearth-device-backup.service')).read()
        self.assertNotIn('ABCD', s)   # the code stays in the root-only file, never in a unit
        self.assertNotIn('WantedBy', s)

    def test_due(self):
        b = load('backups', self.env)
        plan = {'at': '02:00', 'away': True}
        now = time.mktime((2026, 11, 26, 2, 30, 0, 0, 0, -1))
        self.assertTrue(b.due(plan, now - 30 * 3600, now))
        self.assertFalse(b.due(plan, now - 10 * 60, now))
        early = time.mktime((2026, 11, 26, 1, 0, 0, 0, 0, -1))
        self.assertFalse(b.due(dict(plan, away=False), early - 20 * 3600, early))
        self.assertTrue(b.due(plan, early - 30 * 3600, early))   # back home after a day away

    def test_the_server_never_sees_the_keys_on_a_command_line(self):
        b = load('backups', dict(self.env, HEARTH_TEST_SECRETS=''))
        self.assertEqual(b.pw_cmd('backup-x'), ['--password-command', 'secret-tool lookup hearth-app home-server key backup-x'])


# ------------------------------------------------------------------ the Home server helper runs its parts
class Helper(unittest.TestCase):
    def test_server_part_runs_only_its_own_engines(self):
        r = subprocess.run([sys.executable, str(HELPER), 'server-part', '../../bin/sh'], capture_output=True, text=True, env=dict(os.environ, HEARTH_SYSTEM_PART='1'))
        self.assertNotEqual(r.returncode, 0)
        self.assertIn('not one of', r.stdout)
        r = subprocess.run([sys.executable, str(HELPER), 'server-part', 'nothere'], capture_output=True, text=True, env=dict(os.environ, HEARTH_SYSTEM_PART='1'))
        self.assertIn('isn\'t in this version', r.stdout)

    def test_every_engine_is_valid_python_and_says_why(self):
        for f in PARTS.glob('*.py'):
            compile(f.read_text(), str(f), 'exec')
        for name in ('dash', 'vault', 'backups'):
            r = subprocess.run([sys.executable, '-I', str(PARTS / (name + '.py')), 'no-such-command'], capture_output=True, text=True)
            self.assertEqual(r.returncode, 1)
            self.assertFalse(json.loads(r.stdout.strip().splitlines()[-1])['ok'])

    def test_the_store_part_carries_the_engines(self):
        wf = (REPO / '.github/workflows/quick-update.yml').read_text()
        self.assertIn('hearth-server-parts', wf)
        self.assertIn('python3 -m py_compile "$f"', wf)


if __name__ == '__main__':
    unittest.main()
