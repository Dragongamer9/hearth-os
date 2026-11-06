"""Tests for Settings › Home server, stage 3 (docs/HOME-SERVER.md): services (Jellyfin, Ollama) installed only when asked
from their makers' releases with the SHA-256 checked and the archive checked before it's unpacked; each app's reach
(the AI service only through Tailscale or on this PC, never without its firewall rule); Light mode for the AI service
(models unloaded, the service paused and started again) and the media server; the Command Center noted when it uses
the AI service (hide-until-seen, display only); the ransomware guard (read only, told, undone); share links, the
drop-off and Files on the web (never outside a share, secrets only in 0600 state files); smart folders and a clean up
that only moves files to the recycle bin; Safety's fixes and their undo; everything off by default in the settings,
the units and the image. The system's commands are replaced by recorders (test_hearth_home_server.Rec)."""
import sys
sys.dont_write_bytecode = True
import base64
import hashlib
import http.client
import importlib.machinery
import importlib.util
import io
import json
import os
import stat
import subprocess
import tarfile
import tempfile
import threading
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_hearth_home_server import Base, Rec, hh, hs, USER, ROOT, SHELL, LIBEXEC, UNITS  # noqa: E402


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


web = load(SHELL / 'hearth-home-server-web', 'hearth_home_server_web_t')


class Stage3(Base):
    def setUp(self):
        super().setUp()
        for k, v in (('APPS_DIR', 'apps'), ('APP_STATE', 'state/apps.json'), ('SERVICES_STATE', 'run/services.json'),
                     ('SEEN_SYS', 'state/seen.json'), ('GUARD_STATE', 'state/guard.json'), ('JELLYFIN_DATA', 'jellyfin'),
                     ('APP_DROPIN', 'systemd/system/%s.service.d/hearth.conf')):
            self.saved[k] = getattr(hh, k)
            setattr(hh, k, os.path.join(self.etc, v))
        for k in ('http_get', 'download', 'ollama_api', 'notify', 'arch_ok', 'unit_active', 'free_bytes'):
            self.saved_fns[k] = getattr(hh, k)
        hh.arch_ok = lambda: True
        hh.free_bytes = lambda p: 100 << 30
        hh.notify = lambda user, text, urgent=False: self.told.append(text) or True
        self.told = []
        self.active = set()
        hh.unit_active = lambda u: u in self.active
        self.api = []
        hh.ollama_api = lambda m, p, b=None, timeout=10: (self.api.append((m, p, b)) or
                                                          ({'models': [{'name': 'qwen3:14b'}]} if p == '/api/ps' else {}))

    def install(self, name):
        b = os.path.join(hh.APPS_DIR, name, hh.APPS[name]['bin'])
        os.makedirs(os.path.dirname(b), exist_ok=True)
        open(b, 'w').close()


# ---------------------------------------------------------------------------------------------------- reach

class Reach(Stage3):
    def test_the_ai_service_is_only_ever_for_tailscale_and_the_media_server_as_you_choose(self):
        p = hh.services_plan({'ai': {'on': True}, 'media': {'on': True, 'reach': 'home'}}, USER)
        self.assertEqual(p['ports'], {'tcp': [11434], 'udp': []}, 'the AI service always behind the Tailscale-only rule')
        p = hh.services_plan({'media': {'on': True, 'reach': 'tailscale'}}, USER)
        self.assertEqual(p['ports'], {'tcp': [8096], 'udp': [7359]})
        p = hh.services_plan({'media': {'on': True, 'reach': 'home'}, 'tsOnly': True}, USER)
        self.assertEqual(p['ports']['tcp'], [8096], 'Only reachable through Tailscale covers it too')
        self.assertEqual(hh.services_plan({}, USER)['ports'], {'tcp': [], 'udp': []}, 'nothing on: no rule')

    def test_a_service_s_ports_get_the_rule_without_closing_samba(self):
        r = hh.firewall_apply(False, {'tcp': [11434], 'udp': []})
        text = open(hh.NFT_FILE).read()
        self.assertIn('tcp dport { 11434 } counter drop', text)
        self.assertNotIn('445', text, 'Samba stays open to the home network')
        self.assertFalse(os.path.exists(hh.SMBD_DROPIN))
        self.assertEqual(r['services'], 'tailscale-only')
        both = hh.firewall_apply(True, {'tcp': [8096], 'udp': [7359]})
        text = open(hh.NFT_FILE).read()
        self.assertIn('tcp dport { 139, 445, 8096 } counter drop', text)
        self.assertIn('udp dport { 137, 138, 7359 } counter drop', text)
        self.assertEqual(both['firewall'], 'tailscale-only')
        hh.firewall_apply(False, {})
        self.assertFalse(os.path.exists(hh.NFT_FILE), 'nothing left: the table goes')

    def test_the_ai_service_listens_everywhere_only_behind_its_rule(self):
        self.install('ollama')
        r = hh.apply({'shares': [], 'ai': {'on': True}})
        drop = open(hh.APP_DROPIN % 'hearth-ollama').read()
        self.assertIn('OLLAMA_HOST=0.0.0.0:11434', drop)
        self.assertIn('ExecStartPre=+/usr/sbin/nft -f', drop, 'without "-": no rule, no start')
        self.assertIn('hearth-ollama.timer', self.rec.enabled)
        self.assertTrue(self.rec.ran('systemctl', 'restart', 'hearth-ollama.service'))
        self.assertEqual(r['apps']['ollama'], 'on')
        # No nft on this PC: this PC only.
        self.rec.tools.discard('nft')
        r = hh.apply({'shares': [], 'ai': {'on': True}})
        drop = open(hh.APP_DROPIN % 'hearth-ollama').read()
        self.assertIn('OLLAMA_HOST=127.0.0.1:11434', drop)
        self.assertNotIn('ExecStartPre', drop)
        self.assertIn('this PC only', r['apps']['aiNote'])

    def test_a_tailscale_only_media_server_never_starts_without_its_rule(self):
        self.install('jellyfin')
        self.rec.tools.discard('nft')
        r = hh.apply({'shares': [], 'media': {'on': True, 'reach': 'tailscale'}})
        self.assertEqual(r['apps']['jellyfin'], 'not started')
        self.assertNotIn('hearth-jellyfin.timer', self.rec.enabled)
        self.rec.tools.add('nft')
        r = hh.apply({'shares': [], 'media': {'on': True, 'reach': 'tailscale'}})
        drop = open(hh.APP_DROPIN % 'hearth-jellyfin').read()
        self.assertIn('User=%s' % USER, drop, 'it runs as you: it reads your shares')
        self.assertIn('ExecStartPre=+/usr/sbin/nft -f', drop)
        self.assertIn('--ffmpeg /usr/bin/ffmpeg', drop)
        self.assertEqual(r['apps']['jellyfin'], 'on')

    def test_turning_an_app_off_stops_it_and_deletes_nothing(self):
        self.install('ollama')
        hh.apply({'shares': [], 'ai': {'on': True}})
        self.rec.calls.clear()
        r = hh.apply({'shares': [], 'ai': {'on': False}})
        self.assertEqual(r['apps']['ollama'], 'off')
        self.assertTrue(self.rec.ran('systemctl', 'stop', 'hearth-ollama.service'))
        self.assertNotIn('hearth-ollama.timer', self.rec.enabled)
        self.assertTrue(os.path.exists(os.path.join(hh.APPS_DIR, 'ollama', 'bin', 'ollama')))
        self.assertFalse(os.path.exists(hh.NFT_FILE), 'its rule goes with it')

    def test_an_app_that_isn_t_installed_is_never_started(self):
        r = hh.apply({'shares': [], 'ai': {'on': True}})
        self.assertEqual(r['apps']['ollama'], 'not installed')
        self.assertFalse(self.rec.ran('systemctl', 'restart', 'hearth-ollama.service'))
        self.assertFalse(os.path.exists(hh.APP_DROPIN % 'hearth-ollama'))

    def test_the_services_timer_is_on_only_while_something_needs_it(self):
        hh.apply({'shares': []})
        self.assertNotIn(hh.SERVICES_TIMER, self.rec.enabled)
        hh.apply({'shares': [], 'guard': {'on': True}})
        self.assertIn(hh.SERVICES_TIMER, self.rec.enabled)
        hh.apply({'shares': [], 'media': {'on': True, 'pause': False}})
        self.assertNotIn(hh.SERVICES_TIMER, self.rec.enabled, 'a media server that keeps streaming needs no timer')
        hh.apply({'shares': [], 'ai': {'on': True}})
        self.assertIn(hh.SERVICES_TIMER, self.rec.enabled)


# ---------------------------------------------------------------------------------------------------- verified installs

class Installs(Stage3):
    SHA = 'ab' * 32

    def gh(self, digest=True, sums=True, sums_sha=None):
        rel = {'tag_name': 'v0.13.0', 'assets': [{'name': 'ollama-linux-amd64.tgz', 'size': 100,
                                                  'browser_download_url': 'https://github.com/ollama/ollama/releases/download/v0.13.0/ollama-linux-amd64.tgz'}]}
        if digest:
            rel['assets'][0]['digest'] = 'sha256:' + self.SHA
        if sums:
            rel['assets'].append({'name': 'sha256sum.txt', 'browser_download_url': 'https://github.com/ollama/ollama/releases/download/v0.13.0/sha256sum.txt'})

        def get(url, limit=0, timeout=0):
            if url.endswith('/releases/latest'):
                return json.dumps(rel).encode()
            if url.endswith('sha256sum.txt'):
                return ('%s  ./ollama-linux-amd64.tgz\n%s  ./ollama-darwin.tgz\n' % (sums_sha or self.SHA, 'cd' * 32)).encode()
            raise AssertionError(url)
        hh.http_get = get

    def test_ollama_s_checksums_come_from_its_release_and_must_agree(self):
        self.gh()
        r = hh.app_resolve('ollama')
        self.assertEqual((r['sha256'], r['version'], r['file']), (self.SHA, 'v0.13.0', 'ollama-linux-amd64.tgz'))
        self.gh(digest=False)
        self.assertEqual(hh.app_resolve('ollama')['sha256'], self.SHA, 'the checksum file alone is enough')
        self.gh(sums=False)
        self.assertEqual(hh.app_resolve('ollama')['sha256'], self.SHA, "GitHub's own digest alone is enough")
        self.gh(sums_sha='ef' * 32)
        with self.assertRaises(hh.Fail):
            hh.app_resolve('ollama')
        self.gh(digest=False, sums=False)
        with self.assertRaises(hh.Fail):
            hh.app_resolve('ollama')

    def test_jellyfin_s_newest_stable_version_and_its_checksum(self):
        page = '<a href="jellyfin_10.10.7-amd64.tar.xz">x</a><a href="jellyfin_10.11.2-amd64.tar.xz">y</a><a href="jellyfin_10.9.11-amd64.tar.xz">'

        def get(url, limit=0, timeout=0):
            if url.endswith('/amd64/'):
                return page.encode()
            if url.endswith('jellyfin_10.11.2-amd64.tar.xz.sha256sum'):
                return ('%s  jellyfin_10.11.2-amd64.tar.xz\n' % self.SHA).encode()
            raise AssertionError(url)
        hh.http_get = get
        r = hh.app_resolve('jellyfin')
        self.assertEqual(r['version'], '10.11.2', 'by version, not by text')
        self.assertTrue(r['url'].startswith('https://repo.jellyfin.org/'))
        self.assertEqual(r['sha256'], self.SHA)

    def test_downloads_only_over_https_from_the_makers(self):
        for good in ('https://api.github.com/repos/ollama/ollama/releases/latest', 'https://repo.jellyfin.org/files/x',
                     'https://github.com/ollama/ollama/releases/download/v1/a.tgz'):
            self.assertTrue(hh.url_ok(good), good)
        for bad in ('http://github.com/x', 'https://evil.example.com/a.tgz', 'https://github.com.evil.com/x', 'file:///etc/passwd', ''):
            self.assertFalse(hh.url_ok(bad), bad)
            with self.assertRaises(hh.Fail):
                hh.http_get(bad)

    def test_an_archive_is_checked_before_it_is_unpacked(self):
        ok = ('drwxr-xr-x 0/0 0 2025-01-01 00:00 bin/\n-rwxr-xr-x 0/0 9 2025-01-01 00:00 bin/ollama\n'
              'lrwxrwxrwx 0/0 0 2025-01-01 00:00 lib/ollama/libggml.so -> libggml.so.1\n')
        self.assertIsNone(hh.tar_problem(ok))
        for bad in ('-rw-r--r-- 0/0 1 2025-01-01 00:00 /etc/passwd', '-rw-r--r-- 0/0 1 2025-01-01 00:00 ../../etc/x',
                    'crw-r--r-- 0/0 1,3 2025-01-01 00:00 dev/null', 'lrwxrwxrwx 0/0 0 2025-01-01 00:00 bin/x -> /etc/shadow',
                    'lrwxrwxrwx 0/0 0 2025-01-01 00:00 bin/x -> ../../../etc'):
            self.assertIsNotNone(hh.tar_problem(bad), bad)

    def real_tar(self):
        orig = self.rec.run

        def run(cmd, stdin=None, timeout=60):
            if cmd and cmd[0] == 'tar':
                self.rec.calls.append(list(cmd))
                p = subprocess.run(cmd, capture_output=True, text=True)
                return p.returncode, p.stdout + p.stderr
            return orig(cmd, stdin, timeout)
        hh.run = run
        self.rec.tools.add('tar')

    def archive(self, path):
        with tarfile.open(path, 'w:gz') as t:
            data = b'#!/bin/sh\necho ollama\n'
            ti = tarfile.TarInfo('bin/ollama')
            ti.size = len(data)
            ti.mode = 0o755
            t.addfile(ti, io.BytesIO(data))
        return hashlib.sha256(open(path, 'rb').read()).hexdigest()

    def test_install_checks_the_download_then_swaps_it_in(self):
        self.real_tar()
        src = os.path.join(self.d, 'ollama.tgz')
        sha = self.archive(src)
        self.SHA = sha
        self.gh()

        def download(url, dest, limit, progress=None):
            data = open(src, 'rb').read()
            open(dest, 'wb').write(data)
            return hashlib.sha256(data).hexdigest()
        hh.download = download
        r = hh.app_install_run('ollama')
        self.assertEqual(r['version'], 'v0.13.0')
        b = os.path.join(hh.APPS_DIR, 'ollama', 'bin', 'ollama')
        self.assertTrue(os.path.isfile(b))
        self.assertTrue(os.stat(b).st_mode & stat.S_IXUSR)
        st = json.load(open(hh.APP_STATE))['ollama']
        self.assertEqual((st['installing'], st['version'], st['sha256']), (False, 'v0.13.0', sha))
        self.assertEqual([n for n in os.listdir(hh.APPS_DIR) if n.startswith('.')], [], 'nothing left over')

    def test_a_download_that_doesn_t_match_is_never_unpacked(self):
        self.real_tar()
        self.gh()
        hh.download = lambda url, dest, limit, progress=None: open(dest, 'wb').write(b'evil') and 'ff' * 32
        with self.assertRaises(hh.Fail) as e:
            hh.app_install_run('ollama')
        self.assertIn("doesn't match", str(e.exception))
        self.assertFalse(os.path.exists(os.path.join(hh.APPS_DIR, 'ollama')))
        self.assertFalse(any(c[0] == 'tar' for c in self.rec.calls), 'not even listed')
        self.assertIn("doesn't match", json.load(open(hh.APP_STATE))['ollama']['error'])

    def test_install_runs_in_its_own_unit_and_only_when_asked(self):
        r = hh.app_install('ollama')
        self.assertTrue(r['started'])
        run = self.rec.ran('systemd-run')[0]
        self.assertEqual(run[-3:], ['app', 'install-run', 'ollama'])
        with self.assertRaises(hh.Fail):
            hh.app_install('../../bin/sh')
        old_uid = os.environ.get('PKEXEC_UID')
        os.environ['PKEXEC_UID'] = str(os.getuid())
        self.addCleanup(lambda: os.environ.pop('PKEXEC_UID', None) if old_uid is None else os.environ.__setitem__('PKEXEC_UID', old_uid))
        out = io.StringIO()
        saved = sys.stdout
        sys.stdout = out
        try:
            self.assertEqual(hh.main(['x', 'app', 'install-run', 'ollama']), 1, 'never straight through pkexec')
        finally:
            sys.stdout = saved

    def test_removing_an_app_keeps_its_data(self):
        self.install('jellyfin')
        os.makedirs(os.path.join(hh.JELLYFIN_DATA, 'config'))
        r = hh.app_remove('jellyfin')
        self.assertTrue(r['dataKept'])
        self.assertFalse(os.path.exists(os.path.join(hh.APPS_DIR, 'jellyfin')))
        self.assertTrue(os.path.isdir(os.path.join(hh.JELLYFIN_DATA, 'config')))


# ---------------------------------------------------------------------------------------------------- Light mode, Command Center

class LightMode(Stage3):
    def plan(self, **kw):
        p = {'user': USER, 'lightMode': True, 'ai': {'on': True, 'unload': True, 'pause': False}, 'media': {'on': False}}
        p.update(kw)
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        open(hh.PLAN, 'w').write(json.dumps(p))

    def test_models_are_unloaded_when_a_game_starts(self):
        self.install('ollama')
        self.plan()
        self.active.add('hearth-ollama.service')
        hh.game_running = lambda: True
        r = hh.services_run(now=1000)
        self.assertIn(('POST', '/api/generate', {'model': 'qwen3:14b', 'keep_alive': 0}), self.api)
        self.assertTrue(r['playing'])
        self.assertFalse(self.rec.ran('systemctl', 'stop', 'hearth-ollama.service'), 'only unloaded unless you chose to stop it')

    def test_paused_while_you_play_and_started_again_after(self):
        self.install('ollama')
        os.makedirs(os.path.dirname(hh.APP_DROPIN % 'hearth-ollama'))
        open(hh.APP_DROPIN % 'hearth-ollama', 'w').close()
        self.plan(ai={'on': True, 'unload': True, 'pause': True})
        self.active.add('hearth-ollama.service')
        hh.game_running = lambda: True
        hh.services_run(now=1000)
        self.assertTrue(self.rec.ran('systemctl', 'stop', 'hearth-ollama.service'))
        self.assertEqual(json.load(open(hh.SERVICES_STATE))['paused'], ['ollama'])
        self.active.discard('hearth-ollama.service')
        hh.game_running = lambda: False
        r = hh.services_run(now=1100)
        self.assertTrue(self.rec.ran('systemctl', 'start', 'hearth-ollama.service'))
        self.assertEqual(r['paused'], [])

    def test_light_mode_off_or_let_through_leaves_it_alone(self):
        self.install('ollama')
        self.plan(lightMode=False)
        hh.user_settings = lambda home: {'lightMode': False}
        self.active.add('hearth-ollama.service')
        hh.game_running = lambda: True
        hh.services_run(now=1000)
        self.assertFalse([a for a in self.api if a[1] == '/api/generate'])
        hh.user_settings = lambda home: {}
        self.plan()
        hh.light_resumed = lambda now=None: True
        hh.services_run(now=1000)
        self.assertFalse([a for a in self.api if a[1] == '/api/generate'], '"Let paused jobs run now" lets it be')

    def test_the_media_server_pauses_only_if_you_chose_it(self):
        self.install('jellyfin')
        self.plan(ai={'on': False}, media={'on': True, 'pause': False})
        self.active.add('hearth-jellyfin.service')
        hh.game_running = lambda: True
        hh.services_run(now=1000)
        self.assertFalse(self.rec.ran('systemctl', 'stop', 'hearth-jellyfin.service'))
        self.plan(ai={'on': False}, media={'on': True, 'pause': True})
        hh.services_run(now=1000)
        self.assertTrue(self.rec.ran('systemctl', 'stop', 'hearth-jellyfin.service'))

    def test_the_command_center_is_noted_when_it_uses_the_ai_service(self):
        self.install('ollama')
        self.plan()
        self.active.add('hearth-ollama.service')
        hh.game_running = lambda: False
        orig = self.rec.run
        self.rec.tools.add('tailscale')

        def run(cmd, stdin=None, timeout=60):
            if cmd[:1] == ['ss']:
                return 0, 'ESTAB 0 0 100.84.12.7:11434 100.70.1.9:51234\nESTAB 0 0 127.0.0.1:11434 127.0.0.1:40000\n'
            if cmd[:3] == ['tailscale', 'status', '--json']:
                return 0, json.dumps({'Peer': {'a': {'HostName': 'Laptop', 'TailscaleIPs': ['100.70.1.9']}}})
            return orig(cmd, stdin, timeout)
        hh.run = run
        hh.services_run(now=1_800_000_000)
        seen = json.load(open(hh.SEEN_SYS))
        self.assertEqual(seen['commandCenter'], {'at': 1_800_000_000_000, 'device': 'Laptop'})

    def test_the_tailscale_only_rule_is_put_back_if_it_went(self):
        self.plan(ai={'on': False}, guard={'on': False})
        os.makedirs(os.path.dirname(hh.NFT_FILE), exist_ok=True)
        open(hh.NFT_FILE, 'w').write('x')
        orig = self.rec.run
        hh.run = lambda cmd, stdin=None, timeout=60: (1, '') if cmd[:3] == ['nft', 'list', 'table'] else orig(cmd, stdin, timeout)
        r = hh.services_run(now=1)
        self.assertIn('Tailscale-only rule put back', r['did'])
        self.assertTrue(self.rec.ran('nft', '-f', hh.NFT_FILE))


# ---------------------------------------------------------------------------------------------------- the guard

class Guard(Stage3):
    def test_audit_only_on_shares_that_can_change_and_locked_means_read_only(self):
        good, _ = hh.check_shares([self.share('Games', 'Games', write=True), self.share('Work', 'Work files')], USER, self.home, set())
        plain = hh.conf_text(good, USER, False)
        self.assertNotIn('full_audit', plain, 'off: nothing logged')
        self.assertIn('vfs objects = recycle', plain)
        text = hh.conf_text(good, USER, False, audit=True)
        games = text.split('[Games]')[1].split('[Work]')[0]
        self.assertIn('vfs objects = full_audit recycle', games)
        self.assertIn('full_audit:success = renameat unlinkat', games)
        self.assertNotIn('full_audit', text.split('[Work]')[1], 'a read-only share needs no guard')
        locked = hh.conf_text(good, USER, False, audit=True, locked=True)
        self.assertNotIn('read only = no', locked)

    def test_it_trips_over_the_limit_tells_you_and_never_changes_a_setting(self):
        os.makedirs(os.path.dirname(hh.OUR_CONF), exist_ok=True)
        open(hh.OUR_CONF, 'w').write('[Games]\n   read only = no\n')
        orig = self.rec.run
        lines = {'n': 10}
        hh.run = lambda cmd, stdin=None, timeout=60: (0, 'Games|renameat|ok|a -> a.locked\n' * lines['n']) if cmd[:1] == ['journalctl'] else orig(cmd, stdin, timeout)
        plan = {'user': USER}
        r = hh.guard_check(plan, {'on': True, 'limit': 500}, 1000)
        self.assertTrue(r['watching'])
        self.assertIn('read only = no', open(hh.OUR_CONF).read())
        lines['n'] = 600
        r = hh.guard_check(plan, {'on': True, 'limit': 500}, 1000)
        self.assertEqual(r['tripped'], 1000)
        self.assertIn('read only = yes', open(hh.OUR_CONF).read())
        self.assertTrue(self.told and '600 files' in self.told[0])
        self.assertTrue(hh.guard_tripped())
        # apply() keeps it read only while tripped; the reset makes it writable again.
        hh.apply({'shares': [self.share('Games', 'Games', write=True)], 'guard': {'on': True}})
        self.assertNotIn('read only = no', open(hh.OUR_CONF).read())
        hh.guard_reset()
        hh.apply({'shares': [self.share('Games', 'Games', write=True)], 'guard': {'on': True}})
        self.assertIn('read only = no', open(hh.OUR_CONF).read())


# ---------------------------------------------------------------------------------------------------- the Mantel helper

class Mantel(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.saved = {k: getattr(hs, k) for k in ('SETTINGS', 'APPLIED', 'SEEN', 'SEEN_SYS', 'LINKS_STATE', 'WEB_STATE', 'FIXES', 'CAMERAS',
                                                   'FILES_SCAN', 'RECYCLED', 'CONFIG_HOME', 'GUARD_STATE', 'SERVICES_STATE', 'APP_STATE',
                                                   'me', 'web_apply', 'apply_now', 'side_effect', 'root', 'unit_active')}
        for k, v in (('SETTINGS', 'c/nimbus/home-server.json'), ('APPLIED', 's/applied.json'), ('SEEN', 's/seen.json'), ('SEEN_SYS', 'sys/seen.json'),
                     ('LINKS_STATE', 's/links.json'), ('WEB_STATE', 's/web.json'), ('FIXES', 's/fixes.json'), ('CAMERAS', 's/cams.json'),
                     ('FILES_SCAN', 'cache/files.json'), ('RECYCLED', 's/recycled.json'), ('CONFIG_HOME', 'c'),
                     ('GUARD_STATE', 'sys/guard.json'), ('SERVICES_STATE', 'run/services.json'), ('APP_STATE', 'sys/apps.json')):
            setattr(hs, k, os.path.join(self.d, v))
        self.home = os.path.join(self.d, 'home')
        for p in ('Media/Films', 'Media/.hidden', 'Games'):
            os.makedirs(os.path.join(self.home, p))
        hs.me = lambda: (USER, self.home)
        self.webs = []
        hs.web_apply = lambda data=None: self.webs.append(1) or {'web': 'on'}
        hs.apply_now = lambda data=None: {}
        hs.side_effect = lambda k, on: {}
        hs.root = lambda args, stdin='': {'ok': True}
        hs.unit_active = lambda u, user=False: False
        hs.sio.update_json(hs.SETTINGS, lambda d: d.update({'shares': [
            {'id': 'aaaaaa', 'name': 'Media', 'path': os.path.join(self.home, 'Media'), 'on': True, 'write': True},
            {'id': 'bbbbbb', 'name': 'Old', 'path': os.path.join(self.home, 'Games'), 'on': False}]}))

    def tearDown(self):
        for k, v in self.saved.items():
            setattr(hs, k, v)

    def test_everything_new_is_off_and_nested_keys_write_only_themselves(self):
        d = hs.DEFAULTS
        for g in ('media', 'ai', 'web', 'dropoff', 'guard'):
            self.assertIs(d[g]['on'], False, g)
        self.assertEqual(d['links'], [])
        self.assertEqual(d['web']['reach'], 'tailscale', 'links and files on the web: Tailscale only unless you choose otherwise')
        before = json.load(open(hs.SETTINGS))
        self.assertFalse(hs.set_key('aiUnload', 'on'), 'the default is what you have')
        self.assertTrue(hs.set_key('mediaReach', 'tailscale'))
        self.assertEqual(json.load(open(hs.SETTINGS))['media'], {'reach': 'tailscale'})
        for k, v in (('mediaReach', 'internet'), ('guardLimit', '7'), ('aiOn', 'yes'), ('webReach', '')):
            with self.assertRaises(hs.Fail):
                hs.set_key(k, v)
        self.assertEqual(set(json.load(open(hs.SETTINGS))) - set(before), {'media'})

    def test_the_payload_carries_the_services_and_no_secret(self):
        p = hs.apply_payload(hs.load())
        self.assertEqual((p['media']['on'], p['ai']['on'], p['guard']['on']), (False, False, False))
        self.assertNotIn('web', p, 'the web server is yours, not root\'s')

    def test_the_command_center_seen_by_the_ai_service_or_on_this_pc_shows_its_items(self):
        now = int(time.time() * 1000)
        self.assertTrue(hs.seen_state(hs.load(), now)['commandCenter']['hidden'])
        os.makedirs(os.path.dirname(hs.SEEN_SYS))
        json.dump({'commandCenter': {'at': now - 3600000, 'device': 'Laptop'}}, open(hs.SEEN_SYS, 'w'))
        st = hs.seen_state(hs.load(), now)['commandCenter']
        self.assertFalse(st['hidden'])
        self.assertEqual(st['device'], 'Laptop')
        os.remove(hs.SEEN_SYS)
        os.makedirs(os.path.join(hs.CONFIG_HOME, 'Hearth AI Command Center'))
        self.assertGreater(hs.cc_local_seen(hs.load(), now + 1000), 0, 'its settings folder changed just now')
        old = time.time() - 40 * 86400
        os.utime(os.path.join(hs.CONFIG_HOME, 'Hearth AI Command Center'), (old, old))
        self.assertEqual(hs.cc_local_seen(hs.load(), now), 0, 'not within 30 days')
        before = open(hs.SETTINGS).read()
        hs.seen_state(hs.load(), now)
        self.assertEqual(open(hs.SETTINGS).read(), before, 'display only: no setting changes')

    def test_links_only_inside_a_share_and_their_secrets_never_in_the_settings(self):
        f = os.path.join(self.home, 'Media', 'Films', 'holiday.mp4')
        open(f, 'w').write('x')
        for bad in ('/etc/passwd', os.path.join(self.home, 'Media', '.hidden'), os.path.join(self.home, 'Games'), self.home):
            with self.assertRaises(hs.Fail, msg=bad):
                hs.link_add({'path': bad})
        r = hs.link_add({'path': f, 'for': 'Mom', 'days': 7, 'password': 'secret1'})
        text = open(hs.SETTINGS).read()
        self.assertNotIn(r['token'], text)
        self.assertNotIn('secret1', text)
        self.assertNotIn('pbkdf2', text)
        sec = json.load(open(hs.LINKS_STATE))
        self.assertEqual(sec[r['id']]['token'], r['token'])
        self.assertTrue(hs.pw_check('secret1', sec[r['id']]['hash']))
        self.assertEqual(stat.S_IMODE(os.stat(hs.LINKS_STATE).st_mode), 0o600)
        self.assertEqual(json.load(open(hs.SETTINGS))['links'][0]['for'], 'Mom')
        with self.assertRaises(hs.Fail):
            hs.link_add({'path': f, 'days': 3})
        hs.link_remove(r['id'])
        self.assertNotIn(r['id'], json.load(open(hs.LINKS_STATE)))
        self.assertTrue(os.path.exists(f), 'stopping a link never touches the file')

    def test_the_web_server_runs_only_while_something_needs_it(self):
        d = hs.load()
        self.assertFalse(hs.web_needed(d))
        d['links'] = [{'id': 'x', 'expires': 1}]
        self.assertFalse(hs.web_needed(d), 'an expired link needs nothing')
        d['links'] = [{'id': 'x', 'expires': 0}]
        self.assertTrue(hs.web_needed(d))
        d['links'] = []
        d['dropoff'] = {'on': True}
        self.assertTrue(hs.web_needed(d))

    def test_smart_folders_and_a_clean_up_that_can_be_put_back(self):
        m = os.path.join(self.home, 'Media')
        now = time.time()
        big = os.path.join(m, 'big.iso')
        with open(big, 'wb') as fh:
            fh.truncate(5 << 30)
        a, b = os.path.join(m, 'a.bin'), os.path.join(m, 'Films', 'a copy.bin')
        for p in (a, b):
            open(p, 'wb').write(b'z' * (2 << 20))
        os.utime(b, (now, now - 10))
        old = os.path.join(m, 'old.txt')
        open(old, 'w').write('o')
        os.utime(old, (now - 400 * 86400, now - 400 * 86400))
        shot = os.path.join(m, 'Screenshot 2026-11-01.png')
        open(shot, 'w').write('p')
        open(os.path.join(m, '.hidden', 'secret.png'), 'w').write('h')
        sc = hs.scan_shares(hs.load(), now)
        self.assertEqual([i['path'] for i in sc['big']['items']], [big])
        self.assertEqual(sc['duplicates']['count'], 1)
        self.assertEqual(sc['duplicates']['groups'], 1)
        self.assertEqual([i['path'] for i in sc['old']['items']], [old])
        self.assertEqual([i['path'] for i in sc['screenshots']['items']], [shot])
        self.assertNotIn('secret.png', json.dumps(sc), 'hidden files are never looked at')
        self.assertNotIn('bbbbbb', sc['sizes'], 'a share that is off is left alone')
        r = hs.files_recycle([a, '/etc/passwd', os.path.join(self.home, 'Games', 'x')])
        self.assertEqual(r['moved'], 1, 'only files inside a share that is on')
        self.assertFalse(os.path.exists(a))
        self.assertTrue(os.path.exists(os.path.join(m, '.hearth-recycle', 'a.bin')))
        self.assertEqual(hs.files_putback()['putBack'], 1)
        self.assertTrue(os.path.exists(a))

    def test_search_finds_names_in_shares_that_are_on_and_never_hidden_ones(self):
        open(os.path.join(self.home, 'Media', 'Films', 'BFME replay 1.zip'), 'w').close()
        open(os.path.join(self.home, 'Media', '.hidden', 'bfme replay 2.zip'), 'w').close()
        open(os.path.join(self.home, 'Games', 'bfme replay 3.zip'), 'w').close()
        r = hs.files_search('bfme replay')
        self.assertEqual([x['name'] for x in r['results']], ['BFME replay 1.zip'])

    def test_safety_fixes_say_what_they_change_and_undo_puts_it_back(self):
        v = hs.safety_view()
        ids = [f['id'] for f in v['findings']]
        for want in ('ts-only', 'password', 'snapshots', 'backup', 'guard'):
            self.assertIn(want, ids)
        self.assertTrue(all(f['title'] for f in v['findings']))
        r = hs.fix_apply('ts-only')
        self.assertTrue(r['changed'])
        self.assertTrue(hs.load()['tsOnly'])
        self.assertEqual(hs.safety_view()['done'][0]['steps'], [['set', 'tsOnly', 'on', 'off']])
        self.assertFalse(hs.fix_apply('ts-only')['changed'], 'already so')
        hs.fix_undo('ts-only')
        self.assertFalse(hs.load()['tsOnly'])
        with self.assertRaises(hs.Fail):
            hs.fix_apply('rm-rf')

    def test_the_ready_made_models_are_the_four_for_his_pc_and_only_they_download(self):
        # The four for his PC; the 18+ group (hidden unless he turns it on) and "any model by name" are in
        # test_hearth_server_mode.py (AdultModels). Only model names ever get to Ollama.
        self.assertEqual(tuple(m['id'] for m in hs.MODELS), ('qwen3:30b-a3b', 'qwen3-coder:30b-a3b', 'gpt-oss:20b', 'qwen3:14b'))
        self.assertTrue(hs.MODELS[0]['recommended'])
        self.assertTrue(all(m['note'] and m['size'] for m in hs.MODELS))
        for bad in ('llama3; rm -rf /', 'http://evil/model', '', '--insecure'):
            with self.assertRaises(hs.Fail):
                hs.ai_pull(bad)

    def test_a_camera_answer_is_read_and_seeing_one_only_shows_its_items(self):
        xml = ('<e:Envelope><e:Body><d:ProbeMatches><d:ProbeMatch><d:Scopes>onvif://www.onvif.org/name/Front_Door '
               'onvif://www.onvif.org/hardware/C200</d:Scopes><d:XAddrs>http://192.168.1.40/onvif/device_service</d:XAddrs>'
               '</d:ProbeMatch></d:ProbeMatches></e:Body></e:Envelope>')
        v = hs.parse_wsd(xml)
        self.assertEqual(v, {'urls': ['http://192.168.1.40/onvif/device_service'], 'name': 'Front Door'})
        self.assertEqual(hs.parse_wsd('<x><d:XAddrs>javascript:alert(1)</d:XAddrs></x>')['urls'], [])

    def test_quick_access_gets_the_ai_service_in_a_word(self):
        d = hs.load()
        self.assertEqual(hs.ai_quick(d, True), {'on': False, 'state': 'off'})
        d['ai'] = {'on': True, 'unload': True, 'pause': False}
        self.assertEqual(hs.ai_quick(d, False)['state'], 'ready')
        self.assertEqual(hs.ai_quick(d, True)['state'], 'unloaded')
        os.makedirs(os.path.dirname(hs.SERVICES_STATE))
        json.dump({'paused': ['ollama']}, open(hs.SERVICES_STATE, 'w'))
        self.assertEqual(hs.ai_quick(d, True)['state'], 'paused')


# ---------------------------------------------------------------------------------------------------- the web server

class WebServer(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.saved = {k: getattr(web, k) for k in ('SETTINGS', 'LINKS_STATE', 'WEB_STATE')}
        web.SETTINGS = os.path.join(self.d, 'home-server.json')
        web.LINKS_STATE = os.path.join(self.d, 'links.json')
        web.WEB_STATE = os.path.join(self.d, 'web.json')
        self.share = os.path.join(self.d, 'Media')
        os.makedirs(os.path.join(self.share, 'Films'))
        os.makedirs(os.path.join(self.share, '.secret'))
        open(os.path.join(self.share, 'Films', 'a.mp4'), 'w').write('movie')
        open(os.path.join(self.share, '.secret', 'key'), 'w').write('k')
        open(os.path.join(self.d, 'outside.txt'), 'w').write('no')
        os.symlink(os.path.join(self.d, 'outside.txt'), os.path.join(self.share, 'link-out'))
        self.drop = os.path.join(self.d, 'Drop-off')
        os.makedirs(self.drop)
        self.tok, self.dtok = 'T' * 24, 'D' * 24
        self.settings = {'shares': [{'id': 'aaaaaa', 'name': 'Media', 'path': self.share, 'on': True}],
                         'links': [{'id': 'l1', 'path': os.path.join(self.share, 'Films'), 'label': 'Films', 'expires': 0, 'max': 2, 'on': True}],
                         'dropoff': {'on': True, 'path': self.drop}, 'web': {'on': True}}
        json.dump(self.settings, open(web.SETTINGS, 'w'))
        json.dump({'l1': {'token': self.tok, 'hash': '', 'count': 0}, 'dropoff': {'token': self.dtok}}, open(web.LINKS_STATE, 'w'))
        json.dump({'hash': hs.pw_hash('webpass99')}, open(web.WEB_STATE, 'w'))
        self.srv = web.ThreadingHTTPServer(('127.0.0.1', 0), web.Handler)
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        # web.time is Python's own time module: patched only for this test and put back after it (left patched, every
        # later test's time.sleep returned at once, and the Quick Access overlay tests failed in the full run).
        real_sleep = web.time.sleep
        web.time.sleep = lambda s: None
        self.addCleanup(setattr, web.time, 'sleep', real_sleep)

    def tearDown(self):
        self.srv.shutdown()
        for k, v in self.saved.items():
            setattr(web, k, v)

    def get(self, path, method='GET', body=None, auth=None):
        c = http.client.HTTPConnection('127.0.0.1', self.srv.server_address[1], timeout=10)
        h = {}
        if auth:
            h['Authorization'] = 'Basic ' + base64.b64encode(auth.encode()).decode()
        c.request(method, path, body=body, headers=h)
        r = c.getresponse()
        return r.status, r.read().decode('utf-8', 'replace')

    def test_a_link_opens_its_folder_counts_downloads_and_stops_when_used_up(self):
        code, page = self.get('/l/%s/' % self.tok)
        self.assertEqual(code, 200)
        self.assertIn('a.mp4', page)
        self.assertEqual(self.get('/l/%s/a.mp4' % self.tok), (200, 'movie'))
        self.assertEqual(json.load(open(web.LINKS_STATE))['l1']['count'], 1)
        self.get('/l/%s/a.mp4' % self.tok)
        self.assertEqual(self.get('/l/%s/a.mp4' % self.tok)[0], 410, 'most downloads reached')
        self.assertEqual(self.get('/l/%s/' % ('X' * 24))[0], 410)

    def test_never_outside_the_share_and_never_hidden(self):
        self.settings['links'][0]['path'] = self.share
        self.settings['links'][0]['max'] = 0
        json.dump(self.settings, open(web.SETTINGS, 'w'))
        for p in ('/l/%s/../outside.txt' % self.tok, '/l/%s/%%2e%%2e/outside.txt' % self.tok, '/l/%s/.secret/key' % self.tok,
                  '/l/%s/link-out' % self.tok):
            self.assertEqual(self.get(p)[0], 404, p)
        self.assertNotIn('.secret', self.get('/l/%s/' % self.tok)[1])
        self.assertIsNone(web.inside(self.share, '../outside.txt'))
        self.assertIsNone(web.inside(self.share, 'link-out'), 'links judged by where they point')

    def test_an_expired_or_stopped_link_or_one_whose_share_is_off_doesn_t_work(self):
        self.settings['links'][0]['expires'] = 1
        json.dump(self.settings, open(web.SETTINGS, 'w'))
        self.assertEqual(self.get('/l/%s/' % self.tok)[0], 410)
        self.settings['links'][0].update(expires=0, on=False)
        json.dump(self.settings, open(web.SETTINGS, 'w'))
        self.assertEqual(self.get('/l/%s/' % self.tok)[0], 410)
        self.settings['links'][0]['on'] = True
        self.settings['shares'][0]['on'] = False
        json.dump(self.settings, open(web.SETTINGS, 'w'))
        self.assertEqual(self.get('/l/%s/' % self.tok)[0], 410)

    def test_a_link_with_a_password_asks_for_it(self):
        json.dump({'l1': {'token': self.tok, 'hash': hs.pw_hash('linkpw'), 'count': 0}}, open(web.LINKS_STATE, 'w'))
        self.assertEqual(self.get('/l/%s/' % self.tok)[0], 401)
        self.assertEqual(self.get('/l/%s/' % self.tok, auth='x:wrong')[0], 401)
        self.assertEqual(self.get('/l/%s/' % self.tok, auth='friend:linkpw')[0], 200)

    def test_files_on_the_web_need_your_name_and_web_password(self):
        import getpass
        self.assertEqual(self.get('/')[0], 401)
        self.assertEqual(self.get('/', auth='%s:nope' % getpass.getuser())[0], 401)
        code, page = self.get('/', auth='%s:webpass99' % getpass.getuser())
        self.assertEqual(code, 200)
        self.assertIn('/s/aaaaaa/', page)
        self.assertEqual(self.get('/s/aaaaaa/Films/a.mp4', auth='%s:webpass99' % getpass.getuser()), (200, 'movie'))
        self.settings['web']['on'] = False
        json.dump(self.settings, open(web.SETTINGS, 'w'))
        self.assertEqual(self.get('/', auth='%s:webpass99' % getpass.getuser())[0], 404, 'off: nothing there')

    def test_the_drop_off_takes_files_and_shows_nothing(self):
        self.assertEqual(self.get('/d/%s' % self.dtok)[0], 200)
        self.assertEqual(self.get('/d/%s?name=..%%2F..%%2Fevil.sh' % self.dtok, 'POST', b'hello')[0], 200)
        names = os.listdir(self.drop)
        self.assertEqual(names, ['evil.sh'], 'only the name, inside the drop-off folder')
        self.get('/d/%s?name=evil.sh' % self.dtok, 'POST', b'again')
        self.assertEqual(sorted(os.listdir(self.drop)), ['evil (2).sh', 'evil.sh'], 'never over a file')
        self.assertEqual(self.get('/d/%s' % ('Z' * 24), 'POST', b'x')[0], 410)
        self.assertNotIn('evil', self.get('/d/%s' % self.dtok)[1], 'nothing in it is listed')


# ---------------------------------------------------------------------------------------------------- off by default, wiring

class OffByDefault3(unittest.TestCase):
    def test_units_are_started_only_by_their_timers_and_never_hold_up_starting(self):
        for n in ('hearth-ollama', 'hearth-jellyfin', 'hearth-home-server-services'):
            svc = (UNITS / 'system' / (n + '.service')).read_text()
            timer = (UNITS / 'system' / (n + '.timer')).read_text()
            self.assertNotIn('[Install]', svc, n)
            self.assertIn('WantedBy=timers.target', timer)
            for text in (svc, timer):
                self.assertNotIn('Requires=', text)
                self.assertNotIn('network-online', text)
                for t in ('multi-user.target', 'graphical.target', 'nimbus', 'gamescope', 'steam'):
                    self.assertNotIn('WantedBy=' + t, text)
        for n in ('hearth-ollama', 'hearth-jellyfin'):
            svc = (UNITS / 'system' / (n + '.service')).read_text()
            self.assertIn('ConditionPathExists=/var/lib/hearth/apps/', svc, 'no app: no start')
            self.assertIn('ConditionPathExists=/etc/systemd/system/%s.service.d/hearth.conf' % n, svc, 'no drop-in: no start')
        self.assertIn('DynamicUser=yes', (UNITS / 'system' / 'hearth-ollama.service').read_text())
        w = (UNITS / 'user' / 'hearth-home-server-web.service').read_text()
        self.assertIn('Restart=on-failure', w)
        self.assertNotIn('Requires=', w)
        for wants in UNITS.glob('**/*.wants'):
            for n in os.listdir(wants):
                self.assertFalse(n.startswith(('hearth-ollama', 'hearth-jellyfin', 'hearth-home-server')), n)

    def test_the_image_has_no_app_baked_in_and_checks_everything_is_off(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        block = b.split('# ---- Hearth home server')[1].split('# ---- end of Hearth home server')[0]
        timers = block.split('HS_TIMERS="')[1].split('"')[0]
        for t in ('hearth-home-server-services.timer', 'hearth-ollama.timer', 'hearth-jellyfin.timer'):
            self.assertIn(t, timers)
        self.assertIn('hearth-home-server-web.service', block.split('systemctl --global disable')[1].split('\n')[0])
        self.assertIn('Ollama or Jellyfin is in the image', block)
        for bad in ('ollama.com', 'repo.jellyfin.org', 'install.sh', 'systemctl enable'):
            self.assertNotIn(bad, block)
        y = (ROOT / '.github' / 'workflows' / 'base-peppermint.yml').read_text()
        self.assertIn('Home server, stage 3', y)
        self.assertIn("Ollama and Jellyfin aren't in the image", y)
        hsy = (ROOT / '.github' / 'workflows' / 'home-server.yml').read_text()
        self.assertIn('test_hearth_home_server_3.py', hsy)

    def test_pages_are_registered_and_only_talk_to_the_helper(self):
        s = (SHELL / 'settings-pages.js').read_text()   # Hearth Settings' page registry (the redesign)
        for pid, f in (('hs-files', 'SetHomeFiles.qml'), ('hs-services', 'SetHomeServices.qml'), ('hs-gaming', 'SetHomeGaming.qml'),
                       ('hs-family', 'SetHomeFamily.qml'), ('hs-ai', 'SetHomeAI.qml'), ('hs-safety', 'SetHomeSafety.qml')):
            self.assertIn('{ id: "%s", file: "%s", cat: "hs"' % (pid, f), s)
            qml = (SHELL / f).read_text()
            self.assertIn('helper: "hearth-home-server"', qml)
            self.assertNotIn('FileView', qml)
        self.assertNotIn('Coming next', (SHELL / 'SetHomeServer.qml').read_text())

    def test_no_shell_no_secrets_and_the_steam_account_is_never_touched(self):
        for f in (LIBEXEC / 'hearth-home-server-helper', SHELL / 'hearth-home-server', SHELL / 'hearth-home-server-web'):
            src = f.read_text()
            self.assertNotIn('shell=True', src)
            self.assertNotIn('os.system(', src)
            low = src.lower()
            for bad in ('loginusers.vdf', 'config.vdf', 'steam login', 'steamid', 'refresh_token'):
                self.assertNotIn(bad, low, (f.name, bad))


if __name__ == '__main__':
    unittest.main()
