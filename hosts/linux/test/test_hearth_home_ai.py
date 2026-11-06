"""Tests for Settings › Home server › Services and AI: AI pictures (ComfyUI) and AI voices (Kokoro-FastAPI)
(docs/HOME-SERVER.md). Both are the owner's own user services, off until he turns them on, installed only when he asks
(ComfyUI: its release tag with the commit checked, a venv, PyTorch's CUDA 12.8 builds; Kokoro: its image by a pinned
digest in rootless Podman), reachable only on this PC and through Tailscale (they listen nowhere else, and their ports
are in Hearth's Tailscale-only rule while on), stopped while he plays when he chose that (Light mode; state, never a
setting), his own existing copies left alone until he takes them over (and given back as they were), removing keeps his
models (unless he confirms twice), pictures and workflows. The system's commands are replaced by recorders, and
everything patched here is put back after each test."""
import sys
sys.dont_write_bytecode = True
import json
import os
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_hearth_home_server import hh, hs, USER, ROOT, SHELL, UNITS  # noqa: E402
from test_hearth_home_server_3 import Stage3  # noqa: E402

COMMIT = hs.COMFY_COMMIT
DIGEST = 'sha256:' + 'ab' * 32


# ---------------------------------------------------------------------------------------------------- root's part

class RootPart(Stage3):
    def setUp(self):
        super().setUp()
        for k in ('getpw',):
            self.saved_fns.setdefault(k, getattr(hh, k))
        self.saved['USER_RUN'] = hh.USER_RUN
        self.rt = os.path.join(self.d, 'run-user')
        os.makedirs(os.path.join(self.rt, '1000'))
        open(os.path.join(self.rt, '1000', 'bus'), 'w').close()
        hh.USER_RUN = os.path.join(self.rt, '%d')
        hh.getpw = lambda n: type('P', (), {'pw_uid': 1000, 'pw_name': n, 'pw_dir': '/home/' + n})() if n == USER else None
        self.rec.tools.add('runuser')

    def plan(self, **kw):
        p = {'user': USER, 'lightMode': True, 'ai': {'on': False}, 'media': {'on': False},
             'pictures': {'on': True, 'pause': True}, 'voices': {'on': False}}
        p.update(kw)
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        with open(hh.PLAN, 'w') as f:
            f.write(json.dumps(p))

    def user_calls(self, action, unit):
        return [c for c in self.rec.calls if c[:1] == ['runuser'] and c[-2:] == [action, unit] or c[-3:] == [action, '--no-block', unit]]

    def test_their_ports_are_only_for_tailscale_and_this_pc_whenever_they_are_on(self):
        p = hh.services_plan({'pictures': {'on': True}, 'voices': {'on': True}}, USER)
        self.assertEqual(sorted(p['ports']['tcp']), [8188, 8880])
        self.assertEqual(p['pictures'], {'on': True, 'pause': True}, 'ComfyUI stops while you play by default (it holds VRAM)')
        self.assertEqual(p['voices'], {'on': True, 'pause': False}, 'Kokoro is CPU: keeps running unless you choose')
        self.assertEqual(hh.services_plan({}, USER)['ports'], {'tcp': [], 'udp': []}, 'off: no rule')
        hh.apply({'shares': [], 'pictures': {'on': True}})
        self.assertIn('tcp dport { 8188 } counter drop', open(hh.NFT_FILE).read())
        self.assertIn('iifname { "lo", "tailscale0" } accept', open(hh.NFT_FILE).read())
        plan = json.load(open(hh.PLAN))
        self.assertEqual(plan['pictures'], {'on': True, 'pause': True})
        self.assertEqual(plan['voices'], {'on': False, 'pause': False})
        hh.apply({'shares': []})
        self.assertFalse(os.path.exists(hh.NFT_FILE), 'off: its rule goes')

    def test_the_services_timer_runs_only_when_light_mode_has_something_to_stop(self):
        hh.apply({'shares': [], 'voices': {'on': True}})
        self.assertNotIn(hh.SERVICES_TIMER, self.rec.enabled, 'voices that keep running need no timer')
        hh.apply({'shares': [], 'pictures': {'on': True}})
        self.assertIn(hh.SERVICES_TIMER, self.rec.enabled)
        hh.apply({'shares': [], 'pictures': {'on': True, 'pause': False}})
        self.assertNotIn(hh.SERVICES_TIMER, self.rec.enabled)

    def test_comfyui_stops_when_a_game_starts_and_starts_again_after(self):
        self.plan()
        hh.game_running = lambda: True
        r = hh.services_run(now=1000)
        stop = [c for c in self.rec.calls if c[:1] == ['runuser'] and 'stop' in c]
        self.assertEqual(len(stop), 1)
        self.assertEqual(stop[0][:4], ['runuser', '-u', USER, '--'], 'as you, through your own user manager')
        self.assertIn('XDG_RUNTIME_DIR=' + os.path.join(self.rt, '1000'), stop[0])
        self.assertEqual(stop[0][-4:], ['--user', 'stop', '--no-block', 'hearth-comfyui.service'])
        self.assertEqual(r['paused'], ['comfyui'])
        self.assertEqual(json.load(open(hh.SERVICES_STATE))['paused'], ['comfyui'], 'what was stopped is state')
        self.rec.calls.clear()
        hh.services_run(now=1060)
        self.assertFalse([c for c in self.rec.calls if 'stop' in c], 'stopped once')
        hh.game_running = lambda: False
        r = hh.services_run(now=1120)
        self.assertTrue([c for c in self.rec.calls if c[-3:] == ['start', '--no-block', 'hearth-comfyui.service']])
        self.assertEqual(r['paused'], [])

    def test_nothing_is_stopped_when_you_chose_otherwise_light_mode_is_off_or_jobs_are_let_through(self):
        hh.game_running = lambda: True
        self.plan(pictures={'on': True, 'pause': False}, voices={'on': True})
        hh.services_run(now=1)
        self.assertFalse([c for c in self.rec.calls if c[:1] == ['runuser']], 'pause off, voices default off: left alone')
        self.plan(lightMode=False)
        hh.user_settings = lambda home: {'lightMode': False}
        hh.services_run(now=2)
        self.assertFalse([c for c in self.rec.calls if 'stop' in c])
        hh.user_settings = lambda home: {}
        self.plan()
        hh.light_resumed = lambda now=None: True
        hh.services_run(now=3)
        self.assertFalse([c for c in self.rec.calls if 'stop' in c], 'Let paused jobs run now')

    def test_voices_stop_only_if_you_chose_it_and_turned_off_apps_are_not_started_again(self):
        self.plan(pictures={'on': False}, voices={'on': True, 'pause': True})
        hh.game_running = lambda: True
        hh.services_run(now=1)
        self.assertTrue([c for c in self.rec.calls if c[-3:] == ['stop', '--no-block', 'hearth-kokoro.service']])
        self.plan(pictures={'on': False}, voices={'on': False})
        hh.game_running = lambda: False
        self.rec.calls.clear()
        hh.services_run(now=2)
        self.assertFalse([c for c in self.rec.calls if 'start' in c], 'you turned it off meanwhile: stays off')

    def test_only_hearth_s_two_units_and_real_accounts(self):
        with self.assertRaises(hh.Fail):
            hh.user_systemctl(USER, 'stop', 'sshd.service')
        with self.assertRaises(hh.Fail):
            hh.user_systemctl(USER, 'enable', 'hearth-comfyui.service')
        self.assertEqual(hh.user_systemctl('root; rm -rf /', 'stop', 'hearth-comfyui.service')[0], 1)
        self.assertEqual(hh.user_systemctl('nobody2', 'stop', 'hearth-comfyui.service')[0], 1)
        os.remove(os.path.join(self.rt, '1000', 'bus'))
        self.rec.calls.clear()
        self.assertEqual(hh.user_systemctl(USER, 'stop', 'hearth-comfyui.service')[0], 1, 'no user manager: nothing runs')
        self.assertFalse(self.rec.calls)

    def test_the_command_center_using_ai_voices_is_noted(self):
        self.plan(pictures={'on': False}, voices={'on': True})
        hh.game_running = lambda: False
        orig = self.rec.run
        self.rec.tools.add('tailscale')

        def run(c, stdin=None, timeout=60):
            if c[:1] == ['ss']:
                return (0, 'ESTAB 0 0 100.84.12.7:8880 100.70.1.9:51234\n') if ':8880' in c[-1] else (0, '')
            if c[:3] == ['tailscale', 'status', '--json']:
                return 0, json.dumps({'Peer': {'a': {'HostName': 'Work-PC', 'TailscaleIPs': ['100.70.1.9']}}})
            return orig(c, stdin, timeout)
        hh.run = run
        hh.services_run(now=1_800_000_000)
        self.assertEqual(json.load(open(hh.SEEN_SYS))['commandCenter']['device'], 'Work-PC')


# ---------------------------------------------------------------------------------------------------- the Mantel helper

class Cmd:
    """Stands in for git, pip, podman and systemctl, and makes what they would have made."""

    def __init__(self, t):
        self.t = t
        self.calls = []
        self.commit = COMMIT
        self.dirty = ''
        self.ancestor = False
        self.images = set()
        self.owner_enabled = False
        self.owner_active = False
        self.kokoro_ps = ''
        self.torch = False

    def __call__(self, args, timeout=60, cwd=None, env=None):
        self.calls.append(list(args))
        a = list(args)
        if a[:2] == ['git', 'clone']:
            d = a[-1]
            os.makedirs(os.path.join(d, 'comfy'))
            open(os.path.join(d, 'main.py'), 'w').write('# ComfyUI\n')
            open(os.path.join(d, 'requirements.txt'), 'w').write('torch\n')
            os.makedirs(os.path.join(d, '.git'))
            os.makedirs(os.path.join(d, 'models', 'checkpoints'))
            return 0, ''
        if a[:1] == ['git'] and 'rev-parse' in a:
            return 0, self.commit + '\n'
        if a[:1] == ['git'] and 'config' in a:
            return 0, 'https://github.com/comfyanonymous/ComfyUI.git\n'
        if a[:1] == ['git'] and 'status' in a:
            return 0, self.dirty
        if a[:1] == ['git'] and 'merge-base' in a:
            return (0 if self.ancestor else 1), ''
        if a[:1] == ['git'] and 'describe' in a:
            return 0, hs.COMFY_TAG + '\n'
        if a[:3] == ['/usr/bin/python3', '-m', 'venv']:
            os.makedirs(os.path.join(a[3], 'bin'))
            open(os.path.join(a[3], 'bin', 'python'), 'w').close()
            return 0, ''
        if len(a) > 3 and a[1:3] == ['-m', 'pip']:
            if 'show' in a:
                return (0 if self.torch else 1), ''
            if '--index-url' in a:
                self.torch = True
            return 0, ''
        if len(a) > 2 and a[1] == '-c' and 'torch' in a[2]:
            return 0, json.dumps({'torch': '2.9.0+cu128', 'cuda': True, 'gpu': 'NVIDIA GeForce RTX 5070'}) + '\n'
        if a[:2] == ['podman', 'pull']:
            self.images.add(a[2])
            return 0, ''
        if a[:3] == ['podman', 'image', 'exists']:
            return (0 if a[3] in self.images else 1), ''
        if a[:2] == ['podman', 'ps']:
            return 0, self.kokoro_ps
        if a[:2] == ['podman', 'rmi']:
            self.images.discard(a[2])
            return 0, ''
        if a[:3] == ['systemctl', '--user', 'is-enabled']:
            return (0 if self.owner_enabled else 1), ''
        if a[:4] == ['systemctl', '--user', 'disable', '--now'] and a[-1] == 'comfyui.service':
            self.owner_enabled = self.owner_active = False
        if a[:4] == ['systemctl', '--user', 'enable', '--now'] and a[-1] == 'comfyui.service':
            self.owner_enabled = self.owner_active = True
        return 0, ''

    def ran(self, *prefix):
        return [c for c in self.calls if c[:len(prefix)] == list(prefix)]


class MantelAI(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        names = ('SETTINGS', 'APPLIED', 'SEEN', 'SEEN_SYS', 'SERVICES_STATE', 'APP_STATE', 'CONFIG_HOME', 'HOME_AI_STATE',
                 'KOKORO_DIGEST', 'KOKORO_TAG', 'me', 'apply_now', 'root', 'unit_active', 'cmd', 'have', 'free_gb', 'ts_ip4', 'http_json')
        self.saved = {k: getattr(hs, k) for k in names}
        self.addCleanup(lambda: [setattr(hs, k, v) for k, v in self.saved.items()])
        for k, v in (('SETTINGS', 'c/nimbus/home-server.json'), ('APPLIED', 's/applied.json'), ('SEEN', 's/seen.json'),
                     ('SEEN_SYS', 'sys/seen.json'), ('SERVICES_STATE', 'run/services.json'), ('APP_STATE', 'sys/apps.json'),
                     ('CONFIG_HOME', 'c'), ('HOME_AI_STATE', 's/home-ai.json')):
            setattr(hs, k, os.path.join(self.d, v))
        self.home = os.path.join(self.d, 'home')
        os.makedirs(self.home)
        hs.me = lambda: (USER, self.home)
        self.applied = []
        hs.apply_now = lambda data=None: self.applied.append(1) or {}
        hs.root = lambda args, stdin='': {'ok': True}
        self.active = set()
        hs.unit_active = lambda u, user=False: u in self.active
        self.c = Cmd(self)
        hs.cmd = self.c
        self.tools = {'git', 'podman', 'tailscale'}
        hs.have = lambda n: n in self.tools
        hs.free_gb = lambda p: 500
        hs.ts_ip4 = lambda: '100.72.99.97'
        hs.http_json = lambda m, u, b=None, timeout=5: None
        hs.KOKORO_DIGEST = DIGEST
        hs.KOKORO_TAG = 'v0.9.2'
        self.cd = os.path.join(self.home, 'ComfyUI')

    def settings(self):
        return json.load(open(hs.SETTINGS)) if os.path.exists(hs.SETTINGS) else {}

    def owner_unit(self, enabled=True):
        p = os.path.join(hs.CONFIG_HOME, 'systemd', 'user', 'comfyui.service')
        os.makedirs(os.path.dirname(p), exist_ok=True)
        open(p, 'w').write('[Service]\nExecStart=%s/venv/bin/python main.py --listen 100.72.99.97 --port 8188\n' % self.cd)
        self.c.owner_enabled = enabled
        if enabled:
            self.active.add('comfyui.service')

    def make_comfy(self, venv=True):
        os.makedirs(os.path.join(self.cd, 'comfy'))
        os.makedirs(os.path.join(self.cd, '.git'))
        open(os.path.join(self.cd, 'main.py'), 'w').write('x')
        open(os.path.join(self.cd, 'requirements.txt'), 'w').write('torch\n')
        for sub in ('models/checkpoints', 'output', 'input', 'user/default/workflows', 'custom_nodes/x'):
            os.makedirs(os.path.join(self.cd, sub))
        open(os.path.join(self.cd, 'models', 'checkpoints', 'illustrious.safetensors'), 'w').write('m')
        open(os.path.join(self.cd, 'output', 'ComfyUI_00001_.png'), 'w').write('p')
        if venv:
            os.makedirs(os.path.join(self.cd, 'venv', 'bin'))
            open(os.path.join(self.cd, 'venv', 'bin', 'python'), 'w').close()

    # ---- settings

    def test_both_are_off_by_default_and_each_switch_writes_only_its_own_key(self):
        d = hs.load()
        self.assertEqual(d['pictures'], {'on': False, 'pause': True})
        self.assertEqual(d['voices'], {'on': False, 'pause': False})
        self.assertFalse(os.path.exists(hs.SETTINGS), 'reading never writes')
        self.assertFalse(hs.set_key('picturesPause', 'on'), 'the default is what you have')
        self.assertFalse(hs.set_key('voicesOn', 'off'))
        self.assertFalse(os.path.exists(hs.SETTINGS))
        self.assertTrue(hs.set_key('voicesPause', 'on'))
        self.assertEqual(self.settings(), {'voices': {'pause': True}}, 'only that key, never the defaults')
        with self.assertRaises(hs.Fail):
            hs.set_key('picturesOn', 'yes')
        p = hs.apply_payload(hs.load())
        self.assertEqual((p['pictures']['on'], p['voices']['pause']), (False, True), 'root gets them for the rule and Light mode')

    def test_a_damaged_settings_file_never_gets_defaults_written_over_it(self):
        os.makedirs(os.path.dirname(hs.SETTINGS))
        with open(hs.SETTINGS, 'w') as f:
            f.write('{"pictures": {"on": tr')
        self.assertEqual(hs.load()['pictures'], {'on': False, 'pause': True}, 'defaults in memory only')
        self.assertEqual(open(hs.SETTINGS).read(), '{"pictures": {"on": tr', 'reading leaves the damaged file alone')
        hs.set_key('picturesPause', 'off')
        kept = [n for n in os.listdir(os.path.dirname(hs.SETTINGS)) if n.startswith('home-server.json.broken-')]
        self.assertGreaterEqual(len(kept), 1, 'the damaged copy is kept')
        self.assertEqual(json.load(open(hs.SETTINGS)), {'pictures': {'pause': False}}, 'only your change, never the defaults')

    def test_install_and_run_state_never_go_in_the_settings(self):
        hs.ai_state_set('comfyui', installing=True, progress=40)
        self.assertFalse(os.path.exists(hs.SETTINGS))
        self.assertEqual(hs.ai_state()['comfyui']['progress'], 40)

    # ---- where they listen

    def test_comfyui_listens_only_on_this_pc_and_its_tailscale_address(self):
        self.make_comfy()
        got = {}
        saved = hs.os.execv
        self.addCleanup(setattr, hs.os, 'execv', saved)
        cwd = os.getcwd()
        self.addCleanup(os.chdir, cwd)
        hs.os.execv = lambda p, argv: got.update(argv=argv)
        hs.ai_run('comfyui')
        a = got['argv']
        self.assertEqual(a[a.index('--listen') + 1], '127.0.0.1,100.72.99.97')
        self.assertEqual(a[a.index('--port') + 1], '8188')
        self.assertTrue(a[0].endswith('ComfyUI/venv/bin/python'))
        hs.ts_ip4 = lambda: ''
        hs.ai_run('comfyui')
        self.assertEqual(got['argv'][got['argv'].index('--listen') + 1], '127.0.0.1', 'no Tailscale: this PC only')
        self.assertNotIn('0.0.0.0', json.dumps(got))

    def test_kokoro_runs_only_by_its_pinned_digest_on_this_pc_and_tailscale(self):
        got = {}
        saved = hs.os.execvp
        self.addCleanup(setattr, hs.os, 'execvp', saved)
        hs.os.execvp = lambda p, argv: got.update(argv=argv)
        hs.ai_run('kokoro')
        a = got['argv']
        self.assertEqual(a[-1], 'ghcr.io/remsky/kokoro-fastapi-cpu@' + DIGEST)
        self.assertIn('--pull=never', a, 'never downloads at start')
        self.assertIn('--cap-drop=all', a)
        ports = [a[i + 1] for i, x in enumerate(a) if x == '-p']
        self.assertEqual(ports, ['127.0.0.1:8880:8880', '100.72.99.97:8880:8880'])
        self.assertNotIn('0.0.0.0', ' '.join(a))
        hs.KOKORO_DIGEST = 'latest'
        with self.assertRaises(hs.Fail):
            hs.ai_run('kokoro')
        with self.assertRaises(hs.Fail):
            hs.ai_run('anything')

    # ---- ComfyUI: install, adopt, update, remove

    def test_install_clones_the_release_checks_its_commit_then_pytorch_cu128_and_requirements(self):
        r = hs.comfy_job()
        self.assertTrue(r['installed'])
        clone = self.c.ran('git', 'clone')[0]
        self.assertIn('--branch', clone)
        self.assertEqual(clone[clone.index('--branch') + 1], hs.COMFY_TAG)
        self.assertEqual(clone[-2], 'https://github.com/comfyanonymous/ComfyUI.git')
        pips = [c for c in self.c.calls if c[1:3] == ['-m', 'pip'] and 'install' in c]
        self.assertIn('https://download.pytorch.org/whl/cu128', pips[0])
        self.assertEqual(pips[0][-3:], ['torch', 'torchvision', 'torchaudio'])
        self.assertEqual(pips[1][-3:], ['install', '-r', 'requirements.txt'])
        st = hs.ai_state()['comfyui']
        self.assertEqual((st['installing'], st['cuda'], st['gpu']), (False, True, 'NVIDIA GeForce RTX 5070'))
        self.assertTrue(st['hearthMade'])
        self.assertTrue(os.path.isfile(os.path.join(self.cd, 'main.py')))
        self.assertFalse([c for c in self.c.calls if 'huggingface' in ' '.join(c) or 'civitai' in ' '.join(c)], 'never a model')
        self.assertFalse(os.listdir(os.path.join(self.cd, 'models', 'checkpoints')))

    def test_a_clone_that_isn_t_the_pinned_release_is_never_installed(self):
        self.c.commit = 'f' * 40
        with self.assertRaises(hs.Fail):
            hs.comfy_job()
        self.assertFalse(os.path.exists(self.cd))
        self.assertFalse(os.path.exists(self.cd + '.hearth-new'))
        self.assertIn('commit is different', hs.ai_state()['comfyui']['error'])
        self.assertFalse([c for c in self.c.calls if c[1:3] == ['-m', 'pip']], 'nothing installed after it')

    def test_your_own_comfyui_is_adopted_not_installed_twice_and_anything_else_is_left_alone(self):
        self.make_comfy(venv=False)
        hs.comfy_job()
        self.assertFalse(self.c.ran('git', 'clone'), 'adopted: no second copy')
        self.assertTrue(self.c.ran('/usr/bin/python3', '-m', 'venv'))
        self.assertTrue(os.path.exists(os.path.join(self.cd, 'models', 'checkpoints', 'illustrious.safetensors')))
        v = hs.home_ai_view()['pictures']
        self.assertTrue(v['existing'])
        self.assertEqual(v['modelFiles'], ['illustrious.safetensors'])
        # Torch already there (his script installed it): not downloaded again.
        self.c.torch = True
        self.c.calls.clear()
        hs.comfy_job()
        self.assertFalse([c for c in self.c.calls if '--index-url' in c])
        # Something else called ComfyUI, or a link: refused, untouched.
        other = tempfile.mkdtemp()
        hs.me = lambda: (USER, other)
        os.makedirs(os.path.join(other, 'ComfyUI'))
        open(os.path.join(other, 'ComfyUI', 'notes.txt'), 'w').write('mine')
        with self.assertRaises(hs.Fail):
            hs.comfy_job()
        self.assertEqual(os.listdir(os.path.join(other, 'ComfyUI')), ['notes.txt'])
        third = tempfile.mkdtemp()
        os.symlink(self.cd, os.path.join(third, 'ComfyUI'))
        hs.me = lambda: (USER, third)
        with self.assertRaises(hs.Fail):
            hs.comfy_job()

    def test_install_needs_git_venv_and_space_and_says_so(self):
        self.tools.discard('git')
        with self.assertRaises(hs.Fail):
            hs.comfy_job()
        self.assertIn('git', hs.ai_state()['comfyui']['error'])
        self.tools.add('git')
        hs.free_gb = lambda p: 4
        with self.assertRaises(hs.Fail):
            hs.comfy_job()
        self.assertIn('15 GB', hs.ai_state()['comfyui']['error'])

    def test_update_checks_the_tag_never_writes_over_your_changes_and_restarts_it(self):
        self.make_comfy()
        self.c.torch = True
        self.c.dirty = ' M nodes.py\n'
        with self.assertRaises(hs.Fail):
            hs.comfy_job(update=True)
        self.assertFalse(self.c.ran('git', '-C', self.cd, 'checkout'))
        self.c.dirty = ''
        self.active.add('hearth-comfyui.service')
        hs.comfy_job(update=True)
        fetch = self.c.ran('git', '-C', self.cd, 'fetch')[0]
        self.assertIn('refs/tags/%s:refs/tags/%s' % (hs.COMFY_TAG, hs.COMFY_TAG), fetch)
        self.assertTrue(self.c.ran('git', '-C', self.cd, 'checkout', '--detach', COMMIT))
        self.assertTrue(self.c.ran('systemctl', '--user', 'restart', 'hearth-comfyui.service'))
        # Already at or past the release: no checkout back to an older one.
        self.c.ancestor = True
        self.c.calls.clear()
        hs.comfy_job(update=True)
        self.assertFalse(self.c.ran('git', '-C', self.cd, 'checkout'))
        self.c.commit = 'e' * 40
        with self.assertRaises(hs.Fail):
            hs.comfy_job(update=True)

    def test_remove_keeps_your_models_pictures_and_workflows(self):
        self.make_comfy()
        r = hs.comfy_remove()
        self.assertEqual(sorted(r['kept']), ['input', 'models', 'output', 'user'])
        self.assertTrue(os.path.exists(os.path.join(self.cd, 'models', 'checkpoints', 'illustrious.safetensors')))
        self.assertTrue(os.path.exists(os.path.join(self.cd, 'output', 'ComfyUI_00001_.png')))
        for gone in ('venv', 'comfy', 'main.py', 'custom_nodes'):
            self.assertFalse(os.path.exists(os.path.join(self.cd, gone)), gone)
        self.assertTrue(self.c.ran('systemctl', '--user', 'disable', 'hearth-comfyui.timer'))

    def test_models_go_only_with_both_confirmations_and_a_link_is_never_followed(self):
        self.make_comfy()
        elsewhere = tempfile.mkdtemp()
        open(os.path.join(elsewhere, 'big.safetensors'), 'w').write('m')
        os.rename(os.path.join(self.cd, 'models'), os.path.join(self.cd, 'models-real'))
        os.symlink(elsewhere, os.path.join(self.cd, 'models'))
        self.assertEqual(hs.main(['x', 'pictures-remove', 'models', 'sure']), 1, 'one confirmation is not enough')
        self.assertTrue(os.path.islink(os.path.join(self.cd, 'models')))
        hs.comfy_remove(models=True)
        self.assertFalse(os.path.lexists(os.path.join(self.cd, 'models')))
        self.assertTrue(os.path.exists(os.path.join(elsewhere, 'big.safetensors')), 'the link goes, never what it points to')
        self.assertTrue(os.path.exists(os.path.join(self.cd, 'output', 'ComfyUI_00001_.png')))

    def test_remove_turns_its_switch_off_and_refuses_while_your_own_service_uses_it(self):
        self.make_comfy()
        hs.set_key('picturesOn', 'on')
        self.owner_unit(enabled=True)
        with self.assertRaises(hs.Fail):
            hs.comfy_remove()
        self.assertTrue(os.path.exists(os.path.join(self.cd, 'main.py')))
        self.c.owner_enabled = False
        self.active.discard('comfyui.service')
        r = hs.comfy_remove()
        self.assertEqual(r['turnedOff'], 'picturesOn')
        self.assertFalse(hs.load()['pictures']['on'])

    # ---- your own copies: taken over only when you say, given back as they were

    def test_your_own_comfyui_service_blocks_hearth_s_until_you_take_it_over_and_comes_back_after(self):
        self.make_comfy()
        self.owner_unit(enabled=True)
        with self.assertRaises(hs.Fail):
            hs.user_app_precheck('picturesOn', 'on')
        self.assertFalse(self.c.ran('systemctl', '--user', 'disable', '--now', 'comfyui.service'), 'looking never changes it')
        v = hs.home_ai_view()['pictures']
        self.assertEqual((v['owner']['there'], v['owner']['enabled']), (True, True))
        self.active.discard('comfyui.service')
        r = hs.comfy_takeover()
        self.assertTrue(self.c.ran('systemctl', '--user', 'disable', '--now', 'comfyui.service'))
        self.assertTrue(os.path.exists(os.path.join(hs.CONFIG_HOME, 'systemd', 'user', 'comfyui.service')), 'its file stays')
        self.assertTrue(hs.load()['pictures']['on'])
        self.assertEqual(r['applied']['app'], 'on')
        self.assertTrue(self.c.ran('systemctl', '--user', 'enable', 'hearth-comfyui.timer'))
        self.assertTrue(self.applied, 'root put its port behind the Tailscale-only rule')
        hs.comfy_giveback()
        self.assertFalse(hs.load()['pictures']['on'])
        self.assertTrue(self.c.ran('systemctl', '--user', 'stop', 'hearth-comfyui.service'))
        self.assertTrue(self.c.ran('systemctl', '--user', 'enable', '--now', 'comfyui.service'))

    def test_your_own_kokoro_container_is_stopped_only_when_you_take_it_over(self):
        self.c.kokoro_ps = 'kokoro running\n'
        with self.assertRaises(hs.Fail):
            hs.user_app_precheck('voicesOn', 'on')
        self.assertFalse(self.c.ran('podman', 'stop'))
        r = hs.kokoro_takeover()
        self.assertTrue(self.c.ran('podman', 'stop', 'kokoro'))
        self.assertFalse(self.c.ran('podman', 'rm'), 'never deleted')
        self.assertIn('Install', r['note'])
        hs.kokoro_giveback()
        self.assertTrue(self.c.ran('podman', 'start', 'kokoro'))

    # ---- Kokoro

    def test_kokoro_is_pulled_only_by_its_pinned_digest_and_removed_without_touching_yours(self):
        hs.kokoro_job()
        self.assertEqual(self.c.ran('podman', 'pull'), [['podman', 'pull', 'ghcr.io/remsky/kokoro-fastapi-cpu@' + DIGEST]])
        self.assertTrue(hs.kokoro_installed())
        self.assertEqual(hs.ai_state()['kokoro']['digest'], DIGEST)
        r = hs.kokoro_remove()
        self.assertEqual(self.c.ran('podman', 'rmi'), [['podman', 'rmi', 'ghcr.io/remsky/kokoro-fastapi-cpu@' + DIGEST]])
        self.assertFalse([c for c in self.c.calls if c[:2] == ['podman', 'rm'] or c[:2] == ['podman', 'stop'] and 'kokoro' in c],
                         'his own container is never touched')
        self.assertEqual(r['removed'], 'kokoro')
        hs.KOKORO_DIGEST = ''
        with self.assertRaises(hs.Fail):
            hs.kokoro_job()
        self.assertEqual(hs.main(['x', 'voices-install']), 1)

    def test_turning_one_on_starts_your_service_and_its_timer_and_off_deletes_nothing(self):
        hs.kokoro_job()
        self.assertEqual(hs.user_app_switch('kokoro', True), 'on')
        self.assertTrue(self.c.ran('systemctl', '--user', 'enable', 'hearth-kokoro.timer'))
        self.assertTrue(self.c.ran('systemctl', '--user', 'restart', 'hearth-kokoro.service'))
        self.assertEqual(hs.user_app_switch('kokoro', False), 'off')
        self.assertTrue(self.c.ran('systemctl', '--user', 'disable', 'hearth-kokoro.timer'))
        self.assertTrue(hs.kokoro_installed(), 'off keeps it installed')
        self.assertEqual(hs.user_app_switch('comfyui', True), 'not installed', 'never started without ComfyUI')

    def test_the_address_for_the_command_center_and_health(self):
        hs.kokoro_job()
        saved = hs.ts_name_ip
        self.addCleanup(setattr, hs, 'ts_name_ip', saved)
        hs.ts_name_ip = lambda: ('chris-hearth.tail1234.ts.net', '100.72.99.97')
        self.active.add('hearth-kokoro.service')
        hs.http_json = lambda m, u, b=None, timeout=5: {'status': 'healthy'} if u.endswith('/health') else {'voices': ['af_heart', 'am_adam']}
        v = hs.home_ai_view()['voices']
        self.assertEqual(v['url'], 'http://chris-hearth.tail1234.ts.net:8880')
        self.assertEqual(v['local'], 'http://localhost:8880')
        self.assertTrue(v['healthy'])
        self.assertEqual(v['voices'], 2)
        self.assertEqual(hs.home_ai_view()['pictures']['url'], 'http://chris-hearth.tail1234.ts.net:8188')

    # ---- Quick Access

    def test_quick_access_says_what_light_mode_holds_back(self):
        d = hs.load()
        self.assertEqual(hs.ai_app_quick(d, 'pictures', 'comfyui', True), {'on': False, 'state': 'off'})
        d['pictures'] = {'on': True, 'pause': True}
        self.assertEqual(hs.ai_app_quick(d, 'pictures', 'comfyui', False)['state'], 'ready')
        self.assertEqual(hs.ai_app_quick(d, 'pictures', 'comfyui', True)['state'], 'paused')
        d['voices'] = {'on': True, 'pause': False}
        self.assertEqual(hs.ai_app_quick(d, 'voices', 'kokoro', True)['state'], 'ready', 'voices keep talking while you play')


# ---------------------------------------------------------------------------------------------------- off by default, wiring

class OffByDefaultAI(unittest.TestCase):
    def test_their_user_units_are_started_only_by_their_timers_and_never_hold_up_starting(self):
        for n in ('hearth-comfyui', 'hearth-kokoro'):
            svc = (UNITS / 'user' / (n + '.service')).read_text()
            timer = (UNITS / 'user' / (n + '.timer')).read_text()
            self.assertNotIn('[Install]', svc, n)
            self.assertIn('WantedBy=timers.target', timer)
            self.assertIn('hearth-home-server ai-run', svc)
            for text in (svc, timer):
                self.assertNotIn('Requires=', text)
                self.assertNotIn('network-online', text)
                for t in ('default.target', 'graphical-session.target', 'nimbus', 'gamescope', 'steam'):
                    self.assertNotIn('WantedBy=' + t, text)
        self.assertIn('ConditionPathExists=%h/ComfyUI/venv/bin/python', (UNITS / 'user' / 'hearth-comfyui.service').read_text())
        for wants in UNITS.glob('**/*.wants'):
            for n in os.listdir(wants):
                self.assertFalse(n.startswith(('hearth-comfyui', 'hearth-kokoro')), n)

    def test_the_image_bakes_nothing_in_and_checks_they_are_off(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        block = b.split('# ---- Hearth home server')[1].split('# ---- end of Hearth home server')[0]
        self.assertIn('hearth-comfyui.timer hearth-kokoro.timer', block.split('systemctl --global disable')[1].split('\n')[0])
        self.assertIn('ComfyUI or Kokoro is in the image', block)
        for bad in ('git clone', 'pip install', 'podman pull', 'download.pytorch.org', 'kokoro-fastapi', 'systemctl enable'):
            self.assertNotIn(bad, block)
        y = (ROOT / '.github' / 'workflows' / 'base-peppermint.yml').read_text()
        self.assertIn('AI pictures and AI voices are user services that are off', y)
        self.assertIn("ComfyUI and the Kokoro image aren't in the image", y)
        self.assertIn('test_hearth_home_ai.py', (ROOT / '.github' / 'workflows' / 'home-server.yml').read_text())

    def test_the_pages_offer_everything_and_keep_it_visible(self):
        svc = (SHELL / 'SetHomeServices.qml').read_text()
        for want in ('"pictures-install"', '"pictures-update"', '"pictures-takeover"', '"pictures-giveback"', '"picturesOn"', '"picturesPause"',
                     '"pictures-remove", "models", "sure", "really-sure"', '"voices-install"', '"voices-remove"', '"voicesOn"', '"voicesPause"',
                     '"voices-takeover"', 'Models folder', 'Paste this into the Command Center', 'Speech servers › Kokoro-FastAPI', 'wl-copy'):
            self.assertIn(want, svc)
        # Always shown, in the redesign's "AI on this PC" group: no hide-until-seen on Services.
        self.assertIn('SetHeader { text: "AI on this PC" }', svc)
        self.assertIn('// AI pictures and AI voices are always shown here.', svc)
        for row in ('"Install AI pictures (ComfyUI)"', 'title: "Install AI voices (Kokoro)"'):
            self.assertIn(row, svc)
        ai = (SHELL / 'SetHomeAI.qml').read_text()
        self.assertIn('SetHeader { text: "Pictures and voices" }', ai, 'next to the chat models, always shown')
        self.assertIn('"Pictures"', (SHELL / 'SetHomeServer.qml').read_text(), 'the Overview counts them')
        self.assertIn('aiAppsLine', (ROOT / 'hosts' / 'decky-hearth-link' / 'src' / 'index.tsx').read_text(), 'Quick Access says it')

    def test_no_shell_and_no_listening_everywhere(self):
        src = (SHELL / 'hearth-home-server').read_text()
        block = src.split('# ------------------------------------------------------------------ AI pictures (ComfyUI)')[1].split('# ------------------------------------------------------------------ files & links')[0]
        self.assertNotIn('shell=True', block)
        self.assertNotIn('0.0.0.0', block)
        self.assertNotIn("'::'", block)
        self.assertRegex(hs.COMFY_COMMIT, r'^[0-9a-f]{40}$')
        self.assertTrue(hs.TORCH_INDEX.startswith('https://download.pytorch.org/'))
        self.assertTrue(hs.KOKORO_DIGEST == '' or hs.SHA256_DIGEST_RE.fullmatch(hs.KOKORO_DIGEST))


if __name__ == '__main__':
    unittest.main()
