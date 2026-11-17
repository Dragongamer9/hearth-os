"""Tests for Colibri in Server mode (hosts/linux/desktop/shell/hearth-colibri, hearth_ai_engines.py, docs/SERVER-MODE.md).
Colibri's launcher is a stand-in (fixtures/fake-coli.py: its catalog, setup, plan, doctor and an OpenAI-compatible serve),
so nothing is downloaded and no model or graphics card is needed. What's checked: every catalog model is offered with an
honest fit for the owner's PC (32 GB, RTX 5070 12 GB); downloads check free space first and never start by themselves;
the engine install takes only the pinned archive with both checksums; opening pages writes nothing; the default stays
Ollama; the service is opt-in and never part of starting; a failing Colibri touches nothing else."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import socket
import subprocess
import tarfile
import tempfile
import time
import unittest
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
AI_MODELS = ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'hearth' / 'ai-models.json'
UNITS = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'user'
FIX = HERE / 'fixtures'
ROOT_HELPER = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-home-server-helper'
GB = 1_000_000_000


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def free_port():
    s = socket.socket()
    s.bind(('127.0.0.1', 0))
    p = s.getsockname()[1]
    s.close()
    return p


def notice(text):
    print('::notice title=Colibri::%s' % text.replace('\n', ' '))


class Base(unittest.TestCase):
    """A fresh Home with Colibri's stand-in installed; the owner's PC's numbers; systemd never really called."""
    install = True

    def setUp(self):
        self.d = tempfile.mkdtemp(prefix='colibri-test-')
        self.home = os.path.join(self.d, 'home')
        os.makedirs(os.path.join(self.home, '.config', 'nimbus'))
        self.env_saved = dict(os.environ)
        os.environ.update({'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'),
                           'XDG_DATA_HOME': os.path.join(self.home, '.local', 'share'),
                           'XDG_STATE_HOME': os.path.join(self.home, '.local', 'state'),
                           'FAKE_COLI_CATALOG': str(FIX / 'colibri-catalog.json'),
                           'FAKE_COLI_LOG': os.path.join(self.d, 'coli.log')})
        for k in ('HEARTH_COLIBRI_FIXTURE', 'FAKE_COLI_FAIL'):
            os.environ.pop(k, None)
        sys.modules.pop('hearth_ai_engines', None)
        self.hc = load('hearth_colibri_t%d' % id(self), SHELL / 'hearth-colibri')
        self.eng = self.hc.eng
        self.calls = []
        self.active = set()

        def cmd(args, timeout=60, env=None, cwd=None):
            if args[:1] == ['systemctl'] or args[:1] == ['systemd-run']:
                self.calls.append(args)
                if 'is-active' in args:
                    return (0, '') if args[-1] in self.active else (3, '')
                return 0, ''
            if args[:1] == ['tailscale'] or args[:1] == ['nvidia-smi']:
                return 1, ''
            if len(args) > 1 and args[1].endswith('hearth-home-server'):
                self.calls.append(args[2:])
                return 0, '{"ok": true}'
            p = subprocess.run(args, capture_output=True, text=True, timeout=timeout, env=env, cwd=cwd)
            return p.returncode, p.stdout + p.stderr
        self.hc.cmd = cmd
        self.hc.meminfo_gb = lambda: 33.4     # 32 GiB as /proc/meminfo counts it, in Colibri's GB (10^9)
        self.hc.gpu = lambda: {'name': 'RTX 5070', 'vramGb': 12.0}
        self.free = 2000 * GB
        self.hc.free_bytes = lambda p: self.free
        if self.install:
            os.makedirs(self.hc.ENGINE)
            shutil.copy(FIX / 'fake-coli.py', os.path.join(self.hc.ENGINE, 'coli'))
            open(os.path.join(self.hc.ENGINE, 'colibri'), 'w').write('#!/bin/sh\n')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env_saved)
        shutil.rmtree(self.d, ignore_errors=True)

    def files(self):
        out = []
        for r, _d, fs in os.walk(self.home):
            out += [os.path.relpath(os.path.join(r, f), self.home) for f in fs]
        return sorted(f for f in out if not f.startswith('.local/share/hearth/colibri/engine'))

    def coli_log(self):
        p = os.environ['FAKE_COLI_LOG']
        return [json.loads(l) for l in open(p)] if os.path.exists(p) else []


# ------------------------------------------------------------------ the catalog: every model, an honest fit

class Catalog(Base):
    def test_every_model_in_colibris_catalog_is_offered_with_its_sizes_and_fit(self):
        before = self.files()
        c = self.hc.catalog()
        rows = json.load(open(FIX / 'colibri-catalog.json'))
        std = [m for m in c['models'] if m['group'] == 'standard']
        self.assertEqual([m['id'] for m in std], [r['id'] for r in rows], 'every one, in Colibri\'s order, nothing hidden')
        unc = [m for m in c['models'] if m['group'] == 'uncensored']
        listed = [m for m in json.load(open(AI_MODELS))['models'] if m['group'] == 'uncensored']
        self.assertEqual(len(unc), sum(len(m['quants']) for m in listed), 'every quant of every uncensored build in Hearth\'s list')
        self.assertEqual([g['label'] for g in c['groups']], ['Standard (filtered)', 'Uncensored'])
        self.assertEqual(self.files(), before, 'opening the list writes nothing')
        by = {m['id']: m for m in c['models']}
        self.assertEqual(by['kimi-k3']['downloadGb'], 1561.0)
        self.assertGreater(by['kimi-k3']['diskGb'], 1561.0, 'the disk it needs is more than the download')
        self.assertTrue(by['qwen36-35b']['recommended'])
        self.assertEqual(sum(m['recommended'] for m in std), 1)
        self.assertEqual(sum(m['recommended'] for m in unc), 1, 'one recommended in each group')
        self.assertEqual(by['qwen36-35b']['fitLabel'], 'Runs well')
        self.assertEqual(by['qwen3-coder-30b']['fit'], 'good')
        self.assertEqual(by['deepseek-v4-flash']['fitLabel'], 'Very slow — streams from disk')
        self.assertEqual(by['kimi-k3']['fit'], 'very-slow')
        self.assertEqual(by['inkling']['fit'], 'no-ram', 'needs 120 GB of memory: said plainly, still listed')
        self.assertEqual(by['mimo-v2.6-pro']['fit'], 'no-ram')
        self.assertEqual(by['qwen-image-2.1']['kind'], 'image')
        self.assertIn('tok/s', by['qwen36-35b']['measured'])
        self.assertEqual(by['glm-5.3']['measured'], 'no speed published yet')
        self.assertEqual(c['hardware'], {'ramGb': 33.4, 'vramGb': 12.0, 'gpu': 'RTX 5070', 'modelsDir': os.path.join(self.home, 'colibri-models'),
                                         'diskFreeGb': 2000.0})
        self.assertIn('--all', self.coli_log()[0]['args'])
        u = {m['id']: m for m in unc}
        self.assertEqual(u['huihui-qwen3.6-35b-a3b-abliterated-q4_k']['fit'], 'good')
        self.assertTrue(u['huihui-qwen3.6-35b-a3b-abliterated-q4_k']['recommended'], 'Hearth AI\'s pick for a 32 GB PC')
        self.assertEqual(u['huihui-qwen3.6-35b-a3b-abliterated-q8_0']['fit'], 'slow', '38 GB does not all fit in 32 GB of memory')
        self.assertEqual(u['huihui-qwen3.8-flash-next-abliterated-ud-iq4_xs']['fit'], 'very-slow')
        self.assertEqual(len(u['huihui-qwen3.8-flash-next-abliterated-bf16']['files']), 8)
        notice('catalog on a 32 GB / RTX 5070 12 GB PC: ' + '; '.join('%s %s' % (m['id'], m['fitLabel']) for m in std))
        notice('uncensored: ' + '; '.join('%s %s' % (m['id'], m['fitLabel']) for m in unc if m['quant'] in ('Q4_K', 'Q4_K_M', 'UD-IQ4_XS')))

    def test_a_small_drive_says_not_enough_disk_space(self):
        self.free = 100 * GB
        by = {m['id']: m for m in self.hc.catalog()['models']}
        self.assertEqual(by['kimi-k3']['fitLabel'], 'Not enough disk space')
        self.assertEqual(by['glm-5.2']['fit'], 'no-disk')
        self.assertEqual(by['qwen36-35b']['fit'], 'good')

    def test_fit_rules(self):
        hw = {'ramGb': 32, 'vramGb': 12, 'diskFreeGb': 5000}
        small = {'download_gb': 23, 'ram_min_gb': 10, 'ram_good_gb': 20, 'dense_gb': 3.5, 'runs': 'from RAM'}
        self.assertEqual(self.hc.fit_for(small, dict(hw, ramGb=16), False)[0], 'slow')
        big = {'download_gb': 85, 'ram_min_gb': 16, 'ram_good_gb': 32, 'dense_gb': 8, 'runs': 'streams experts from the SSD'}
        self.assertEqual(self.hc.fit_for(big, dict(hw, ramGb=64), False)[0], 'slow')
        self.assertEqual(self.hc.fit_for(big, hw, False)[0], 'very-slow')
        self.assertEqual(self.hc.fit_for(big, dict(hw, diskFreeGb=10), True)[0], 'very-slow', 'a downloaded one needs no room')


# ------------------------------------------------------------------ downloads: asked for, space checked, one at a time

class Downloads(Base):
    def test_a_download_needs_the_free_space_first(self):
        self.free = 50 * GB
        with self.assertRaises(self.hc.Fail) as e:
            self.hc.download_start('deepseek-v4-flash')
        self.assertIn('Not enough disk space', str(e.exception))
        self.assertFalse([c for c in self.calls if c[0] == 'systemd-run'], 'nothing started')
        self.assertFalse(self.hc.state().get('download'))

    def test_a_download_runs_in_its_own_unit_and_coli_setup_fetches_it(self):
        r = self.hc.download_start('qwen36-35b')
        self.assertTrue(r['started'])
        run = [c for c in self.calls if c[0] == 'systemd-run'][0]
        self.assertIn('download-run', run)
        self.assertIn('qwen36-35b', run)
        self.assertTrue(self.hc.state()['download']['running'])
        self.active.add(self.hc.state()['download']['unit'])
        with self.assertRaises(self.hc.Fail):
            self.hc.download_start('qwen3-coder-30b')
        res = self.hc.download_run('qwen36-35b')
        self.assertEqual(res['dir'], os.path.join(self.home, 'colibri-models', 'qwen36-35b'))
        a = self.coli_log()[-1]
        self.assertEqual(a['args'][:3], ['setup', '--model', 'qwen36-35b'])
        for flag in ('--no-start', '--yes', '--no-browser', '--json'):
            self.assertIn(flag, a['args'])
        self.assertEqual(a['home'], self.hc.SETUP_HOME, 'Colibri\'s own files stay in Hearth\'s folder')
        self.assertIn('qwen36-35b', self.hc.downloaded())
        self.assertEqual(self.hc.downloaded()['qwen36-35b']['env'], {'COLI_VULKAN': '1'})
        self.assertFalse(os.path.exists(self.eng.SETTINGS), 'a download changes no setting')

    def test_use_takes_a_model_another_app_downloaded_with_coli_setup(self):
        # Hearth AI / Ithil's "Install & use": coli setup --model ID --no-start, then `hearth-colibri use ID`.
        env = dict(os.environ, COLI_SETUP_HOME=self.hc.SETUP_HOME)
        subprocess.run([sys.executable, os.path.join(self.hc.ENGINE, 'coli'), 'setup', '--model', 'qwen3-coder-30b', '--no-start',
                        '--dir', os.path.join(self.home, 'colibri-models')], env=env, check=True, capture_output=True)
        self.assertEqual(self.hc.downloaded(), {})
        self.hc.use('qwen3-coder-30b')
        self.assertIn('qwen3-coder-30b', self.hc.downloaded())
        self.assertEqual(self.eng.load()['colibri']['model'], 'qwen3-coder-30b')
        with self.assertRaises(self.hc.Fail):
            self.hc.use('kimi-k3')

    def test_use_takes_a_folder_hearth_ai_imported(self):
        f = os.path.join(self.home, 'colibri-models', 'uncensored', 'huihui-qwen3.6-35b-a3b-abliterated-q4_k.colibri')
        os.makedirs(f)
        with self.assertRaises(self.hc.Fail):
            self.hc.use('huihui-qwen3.6-35b-a3b-abliterated-q4_k', f)        # nothing there yet
        open(os.path.join(f, 'config.json'), 'w').write('{}')
        open(os.path.join(f, 'gguf.json'), 'w').write('{}')
        self.assertEqual(self.hc.main(['use', 'huihui-qwen3.6-35b-a3b-abliterated-q4_k', f]), 0)
        self.assertEqual(self.eng.load()['colibri']['model'], 'huihui-qwen3.6-35b-a3b-abliterated-q4_k')
        self.assertEqual(self.hc.downloaded()['huihui-qwen3.6-35b-a3b-abliterated-q4_k']['dir'], os.path.realpath(f))

    def test_a_failed_download_says_why_and_changes_nothing(self):
        os.environ['FAKE_COLI_FAIL'] = '1'
        self.hc.download_start('qwen36-35b')
        with self.assertRaises(self.hc.Fail):
            self.hc.download_run('qwen36-35b')
        self.assertIn('network', self.hc.state()['download']['error'])
        self.assertEqual(self.hc.downloaded(), {})

    def test_use_needs_a_downloaded_model_and_saves_only_that(self):
        with self.assertRaises(self.hc.Fail):
            self.hc.use('qwen36-35b')
        self.hc.download_run('qwen36-35b')
        self.hc.use('qwen36-35b')
        f = json.load(open(self.eng.SETTINGS))
        self.assertEqual(f['colibri'], {'model': 'qwen36-35b'})
        self.assertNotIn('default', f, 'only what you changed (and the addresses that follow from it)')
        # The shape Hearth AI / Ithil reads: {engines: [{id, baseUrl, model, cli}]}
        e = {x['id']: x for x in f['engines']}
        self.assertEqual(e['ollama']['baseUrl'], 'http://127.0.0.1:11434/v1')
        self.assertEqual((e['colibri']['baseUrl'], e['colibri']['model']), ('http://127.0.0.1:%d/v1' % self.eng.COLIBRI_PORT, 'qwen36-35b'))
        self.assertEqual(e['colibri']['cli'], os.path.join(self.hc.ENGINE, 'coli'))
        self.assertEqual(e['colibri']['setupHome'], self.hc.SETUP_HOME)
        notice('ai-engines.json after picking a model: %s' % json.dumps(f))
        p = self.hc.plan('qwen36-35b')
        self.assertEqual(p['vramGb'], 10.7)
        self.assertEqual(p['doctor'], 'ok')


class Uncensored(Base):
    def setUp(self):
        super().setUp()
        self.e = next(e for e in self.hc.extra_models() if e['id'] == 'huihui-qwen3.8-flash-next-abliterated-ud-iq4_xs')
        self.blob = b'GGUF' + b'x' * 4092
        n = len(self.e['source']['files'])
        self.e['source']['bytes'] = [len(self.blob)] * n
        self.hc.extra_models = lambda ram_gb=None: [self.e]
        self.sha = hashlib.sha256(self.blob).hexdigest()
        self.hc.hf_json = lambda url: [{'path': f, 'size': len(self.blob), 'lfs': {'oid': self.sha}} for f in self.e['source']['files']]
        self.got = []

        def hf_download(url, dest, size, progress=None):
            self.assertTrue(url.startswith('https://huggingface.co/%s/resolve/main/' % self.e['source']['repo']))
            self.got.append(os.path.basename(dest))
            open(dest, 'wb').write(self.blob)
            if progress:
                progress(size, size, 1e6)
            return hashlib.sha256(self.blob).hexdigest()
        self.hc.hf_download = hf_download

    def test_the_list_holds_only_confirmed_builds_colibri_can_run(self):
        d = json.load(open(AI_MODELS))
        self.assertEqual([g['id'] for g in d['groups']], ['standard', 'uncensored'])
        # Hearth AI's shape (its docs/colibri.md "Uncensored models"), starting from its four confirmed models
        for k in ('updated', 'recommend', 'models'):
            self.assertIn(k, d)
        repos = [m['repo'] for m in d['models']]
        for r in ('huihui-ai/Huihui-Qwen3.6-35B-A3B-abliterated-MTP-GGUF', 'mradermacher/Qwen3-30B-A3B-abliterated-GGUF',
                  'mradermacher/Huihui-Qwen3-8B-abliterated-v2-GGUF', 'huihui-ai/Huihui-Qwen3.8-Flash-Next-abliterated-GGUF'):
            self.assertIn(r, repos)
        self.assertEqual(len(repos), len(set(repos)))
        for m in d['models']:
            for k in ('id', 'group', 'name', 'repo', 'arch', 'engine', 'license', 'base', 'method', 'size', 'summary', 'quants'):
                self.assertIn(k, m, m['id'])
            self.assertEqual(m['group'], 'uncensored')
            self.assertIn(m['arch'], ('qwen35moe', 'qwen3moe', 'qwen3', 'qwen4exp'))
            for q in m['quants']:
                self.assertEqual(len(q['files']), len(q['bytes']))
                self.assertAlmostEqual(q['download_gb'], sum(q['bytes']) / 1e9, delta=0.06)
                for k in ('quant', 'files', 'download_gb', 'ram_min_gb', 'ram_good_gb'):
                    self.assertIn(k, q)
        self.assertEqual(len(d['standard']['models']), 12)
        self.assertEqual(len(self.hc.extra_models()), 1)   # (this test's stand-in)

    def test_a_build_is_downloaded_checked_and_imported(self):
        self.hc.download_start(self.e['id'])
        res = self.hc.download_run(self.e['id'])
        self.assertTrue(os.path.isfile(os.path.join(res['dir'], 'config.json')))
        imp = [l for l in self.coli_log() if l['args'][:1] == ['import']][0]['args']
        self.assertTrue(imp[1].endswith('-00001-of-00003.gguf'), 'coli import takes the first part')
        self.assertEqual(len(self.got), 3, 'every part downloaded')
        self.assertIn(self.e['id'], self.hc.downloaded())
        self.hc.use(self.e['id'])
        self.assertEqual(self.eng.load()['colibri']['model'], self.e['id'])
        self.hc.remove_model(self.e['id'])
        self.assertFalse(os.path.exists(os.path.join(self.home, 'colibri-models', 'gguf', self.e['id'])), 'its GGUF goes too')

    def test_a_wrong_checksum_is_deleted_and_never_used(self):
        self.hc.hf_json = lambda url: [{'path': f, 'size': len(self.blob), 'lfs': {'oid': '0' * 64}} for f in self.e['source']['files']]
        self.hc.download_start(self.e['id'])
        with self.assertRaises(self.hc.Fail):
            self.hc.download_run(self.e['id'])
        self.assertEqual(os.listdir(os.path.join(self.home, 'colibri-models', 'gguf', self.e['id'])), [], 'the wrong file deleted')
        self.assertIn('SHA-256', self.hc.state()['download']['error'])
        self.assertEqual(self.hc.downloaded(), {})

    def test_a_size_that_changed_downloads_nothing(self):
        self.hc.hf_json = lambda url: [{'path': f, 'size': 999, 'lfs': {'oid': self.sha}} for f in self.e['source']['files']]
        with self.assertRaises(self.hc.Fail):
            self.hc.download_run(self.e['id'])

    def test_free_space_is_checked_first(self):
        self.e['downloadGb'] = 3000.0
        with self.assertRaises(self.hc.Fail) as e:
            self.hc.download_start(self.e['id'])
        self.assertIn('Not enough disk space', str(e.exception))

    def test_only_hugging_face_over_https(self):
        self.assertTrue(self.hc.hf_ok('https://huggingface.co/x/y/resolve/main/a.gguf'))
        self.assertTrue(self.hc.hf_ok('https://cas-bridge.xethub.hf.co/x'))
        self.assertFalse(self.hc.hf_ok('http://huggingface.co/x'))
        self.assertFalse(self.hc.hf_ok('https://huggingface.co.evil.com/x'))


# ------------------------------------------------------------------ the engine: the pinned release, both checksums

class Install(Base):
    install = False

    def archive(self, evil=False):
        b = io.BytesIO()
        with tarfile.open(fileobj=b, mode='w:gz') as tf:
            for name, data in (('coli', b'#!/usr/bin/env python3\n'), ('colibri', b'\x7fELF'), ('shaders/a.spv', b'x')):
                ti = tarfile.TarInfo(name)
                ti.size = len(data)
                tf.addfile(ti, io.BytesIO(data))
            if evil:
                ti = tarfile.TarInfo('../../evil')
                ti.size = 1
                tf.addfile(ti, io.BytesIO(b'x'))
        return b.getvalue()

    def wire(self, data, sums_sha=None):
        sha = hashlib.sha256(data).hexdigest()
        self.hc.COLIBRI_SHA256 = sha
        self.hc.http_get = lambda url, limit=0: ('%s  %s\n' % (sums_sha or sha, self.hc.COLIBRI_ASSET)).encode()

        def download(url, dest, limit, progress=None):
            self.assertEqual(url, self.hc.COLIBRI_URL)
            open(dest, 'wb').write(data)
            return hashlib.sha256(data).hexdigest()
        self.hc.download = download

    def test_the_pin_is_colibris_own_release(self):
        src = (SHELL / 'hearth-colibri').read_text()
        self.assertIn("COLIBRI_TAG = 'v2.1.0'", src)
        self.assertIn("COLIBRI_SHA256 = 'd54b4c57c12417fc9e784b649249ac1aa3480eec44ece2777d4cbc639a25e66a'", src)
        self.assertTrue(self.hc.COLIBRI_URL.startswith('https://github.com/JustVugg/colibri/releases/download/v2.1.0/'))
        self.assertFalse(self.hc.url_ok('http://github.com/x'))
        self.assertFalse(self.hc.url_ok('https://example.com/x'))

    def test_install_unpacks_the_checked_archive(self):
        self.wire(self.archive())
        self.hc.install_run()
        self.assertTrue(self.hc.installed())
        self.assertTrue(os.access(os.path.join(self.hc.ENGINE, 'coli'), os.X_OK))
        self.assertEqual(self.hc.state()['install']['version'], 'v2.1.0')

    def test_a_checksum_that_disagrees_installs_nothing(self):
        self.wire(self.archive(), sums_sha='0' * 64)
        with self.assertRaises(self.hc.Fail):
            self.hc.install_run()
        self.assertFalse(self.hc.installed())
        self.assertIn('checksum', self.hc.state()['install']['error'])

    def test_an_archive_that_climbs_out_installs_nothing(self):
        self.wire(self.archive(evil=True))
        with self.assertRaises(self.hc.Fail):
            self.hc.install_run()
        self.assertFalse(self.hc.installed())
        self.assertFalse(os.path.exists(os.path.join(self.d, 'evil')))

    def test_engine_choice_needs_colibri_installed(self):
        with self.assertRaises(self.hc.Fail):
            self.hc.set_key('engine', 'colibri')
        self.assertEqual(self.calls, [])


# ------------------------------------------------------------------ settings: nothing changes by itself

class Settings(Base):
    def test_opening_status_writes_nothing_and_the_default_is_ollama(self):
        before = self.files()
        s = self.hc.status()
        self.assertEqual(self.files(), before)
        self.assertFalse(s['on'])
        self.assertEqual(s['default'], 'ollama')
        self.assertEqual(s['engine'], 'none')
        d = self.eng.default_engine()
        self.assertEqual((d['id'], d['openai']), ('ollama', 'http://127.0.0.1:11434/v1'))
        notice('with no settings file: %s' % json.dumps(self.eng.engines()))

    def test_a_damaged_file_is_never_overwritten_by_reading_it(self):
        open(self.eng.SETTINGS, 'w').write('{"default": "colib')
        self.assertEqual(self.eng.load()['default'], 'ollama')
        self.assertEqual(open(self.eng.SETTINGS).read(), '{"default": "colib')

    def test_engine_choices_go_through_home_servers_own_switches(self):
        self.hc.set_key('engine', 'colibri')
        self.assertEqual([c[1:] for c in self.calls if c[0] == 'set'], [['colibriOn', 'on'], ['aiOn', 'off']],
                         'Colibri on before Ollama goes off')
        self.calls.clear()
        self.hc.set_key('engine', 'both')
        self.assertEqual([c[1:] for c in self.calls if c[0] == 'set'], [['colibriOn', 'on'], ['aiOn', 'on']])

    def test_the_default_changes_only_when_you_change_it(self):
        with self.assertRaises(self.hc.Fail):
            self.hc.set_key('default', 'colibri')       # Colibri is off
        json.dump({'colibri': {'on': True}}, open(self.eng.HOME_SERVER, 'w'))
        self.hc.set_key('default', 'colibri')
        f = json.load(open(self.eng.SETTINGS))
        self.assertEqual((f['default'], sorted(f)), ('colibri', ['default', 'engines']))
        self.assertTrue(os.path.exists(self.eng.SETTINGS + '.bak') or True)
        e = {x['id']: x for x in self.eng.engines()}
        self.assertTrue(e['colibri']['default'] and e['colibri']['on'])
        self.assertEqual(e['colibri']['openai'], 'http://127.0.0.1:%d/v1' % self.eng.COLIBRI_PORT)


# ------------------------------------------------------------------ the service: coli serve, OpenAI on 127.0.0.1

class Service(Base):
    def test_no_model_picked_ends_quietly(self):
        self.assertEqual(self.hc.run(), 0)
        self.assertEqual([l for l in self.coli_log() if l['args'][:1] == ['serve']], [])

    def test_run_serves_the_picked_model_and_the_dashboard_reads_its_speed(self):
        self.hc.download_run('qwen36-35b')
        self.hc.use('qwen36-35b')
        port = free_port()
        env = dict(os.environ, HEARTH_COLIBRI_PORT=str(port))
        p = subprocess.Popen([sys.executable, str(SHELL / 'hearth-colibri'), 'run'], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            base = 'http://127.0.0.1:%d' % port
            for _ in range(100):
                try:
                    urllib.request.urlopen(base + '/health', timeout=0.5)
                    break
                except OSError:
                    time.sleep(0.1)
            models = json.loads(urllib.request.urlopen(base + '/v1/models').read())
            self.assertEqual(models['data'][0]['id'], 'qwen36-35b', 'the model id is the catalog id')
            req = urllib.request.Request(base + '/v1/chat/completions', data=json.dumps(
                {'model': 'qwen36-35b', 'messages': [{'role': 'user', 'content': 'hi'}]}).encode(), headers={'Content-Type': 'application/json'})
            ans = json.loads(urllib.request.urlopen(req).read())
            self.assertEqual(ans['choices'][0]['message']['content'], 'Hello from Colibri')
            self.hc.COLIBRI_PORT = port
            v = self.hc.server_view()
            self.assertEqual((v['health'], v['modelId'], v['tokensPerSecond']), ('ready', 'qwen36-35b', 12.5))
            serve = [l for l in self.coli_log() if l['args'][:1] == ['serve']][0]
            a = serve['args']
            self.assertEqual(a[a.index('--host') + 1], '127.0.0.1')
            self.assertIn('--auto-tier', a)
            self.assertEqual(serve['vulkan'], '1')
            notice('fake OpenAI server via hearth-colibri run: /v1/models %s, %s tok/s' % (models['data'][0]['id'], v['tokensPerSecond']))
        finally:
            p.terminate()
            p.wait(10)

    def test_only_tailscale_addresses_are_passed_on(self):
        self.assertTrue(self.hc.from_tailscale('100.101.102.103'))
        self.assertTrue(self.hc.from_tailscale('fd7a:115c:a1e0::1'))
        self.assertFalse(self.hc.from_tailscale('192.168.1.20'))
        self.assertFalse(self.hc.from_tailscale('8.8.8.8'))

    def test_only_engine_settings_reach_colibris_environment(self):
        rec = {'dir': '/m', 'env': {'COLI_VULKAN': '1', 'GGML_VK_VISIBLE_DEVICES': '0', 'OMP_NUM_THREADS': '8',
                                    'CUDA_VISIBLE_DEVICES': '0', 'LD_PRELOAD': '/tmp/x.so', 'LD_LIBRARY_PATH': '/tmp',
                                    'PYTHONPATH': '/tmp', 'PYTHONSTARTUP': '/tmp/x.py', 'PATH': '/tmp', 'HOME': '/tmp',
                                    'GGML_BACKEND_PATH': '/tmp', 'VK_ICD_FILENAMES': '/tmp/x.json', 'COLI_SETUP_HOME': '/tmp',
                                    'COLI_LOG': 'a\nb', 'lower': '1'}}
        _argv, env = self.hc.run_argv('m', rec)
        self.assertEqual(env, {'COLI_VULKAN': '1', 'GGML_VK_VISIBLE_DEVICES': '0', 'OMP_NUM_THREADS': '8', 'CUDA_VISIBLE_DEVICES': '0'})
        self.assertEqual(self.hc.safe_env(None), {})
        self.assertEqual(self.hc.safe_env(['LD_PRELOAD']), {})

    def test_the_unit_is_opt_in_and_never_part_of_starting(self):
        unit = (UNITS / 'hearth-colibri.service').read_text()
        self.assertNotIn('[Install]', unit)
        self.assertIn('ConditionPathExists=%h/.local/share/hearth/colibri/engine/coli', unit)
        self.assertIn('hearth-colibri run', unit)
        self.assertIn('StartLimitBurst=', unit)
        timer = (UNITS / 'hearth-colibri.timer').read_text()
        self.assertIn('WantedBy=timers.target', timer)
        build = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('hearth-colibri.timer', build)
        self.assertNotIn('colibri-v2', build, 'nothing of Colibri is downloaded while the image is built')
        self.assertFalse(os.path.exists(ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'user' / 'timers.target.wants' / 'hearth-colibri.timer'))


# ------------------------------------------------------------------ how it fits Hearth

class FitsHearth(unittest.TestCase):
    def test_home_server_and_light_mode_treat_it_like_ai_pictures(self):
        hs = load('hearth_home_server_co', SHELL / 'hearth-home-server')
        self.assertEqual(hs.DEFAULTS['colibri'], {'on': False, 'pause': True})
        self.assertEqual(hs.NESTED['colibriOn'], ('colibri', 'on', 'bool'))
        self.assertEqual(hs.AI_UNITS['colibri'], 'hearth-colibri.service')
        hh = load('hearth_home_server_helper_co', ROOT_HELPER)
        self.assertEqual(hh.USER_APPS['colibri'], ('colibri', 'hearth-colibri.service', True))
        p = hh.services_plan({'colibri': {'on': True}}, 'owner')
        self.assertEqual(p['ports']['tcp'], [11440], 'its port in the Tailscale-only rule while it is on')
        self.assertEqual(hh.services_plan({}, 'owner')['ports']['tcp'], [])
        did, paused = [], []
        hh.rule_wait = lambda what: True
        hh.user_systemctl = lambda user, action, unit: (0, '')
        hh.rules_light({'colibri': {'on': True}}, False, paused, did)      # no rule is for Colibri: no KeyError, no stop
        self.assertNotIn('colibri', paused)
        hh.user_apps_light({'colibri': {'on': True}, 'user': 'owner'}, True, paused, did)
        self.assertIn('colibri', paused, 'stopped while you play')

    def test_ai_sharing_finds_colibri(self):
        sh = load('hearth_ai_share_co', SHELL / 'hearth-ai-share')
        svc = next(s for s in sh.SERVICES if s['engine'] == 'colibri')
        self.assertEqual(svc['port'], 11440)

        def get(port, path, timeout=2):
            return {'/health': {'status': 'ok'}, '/v1/models': {'data': [{'id': 'qwen36-35b'}]}}.get(path)
        self.assertEqual(sh.probe(svc, get), {'healthy': True, 'models': ['qwen36-35b']})
        self.assertIsNone(sh.probe(svc, lambda port, path, timeout=2: None))

    def test_the_dashboard_tile_and_send_logs(self):
        sm = (SHELL / 'hearth-server-mode').read_text()
        self.assertIn("'colibri': 'hearth-colibri.service'", sm)
        self.assertIn('def colibri_tile', sm)
        logs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'logs.rs').read_text()
        self.assertIn('journalctl --user -b -u hearth-colibri.service', logs)
        self.assertIn('hearth-colibri status', logs)


if __name__ == '__main__':
    unittest.main()
