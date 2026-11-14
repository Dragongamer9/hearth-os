"""Horizon's server in Settings › Home server › Server apps (1.12, ROADMAP: Horizon): its signed release checked against
Horizon's key before anything is unpacked, the Nightly signed for its own version, your own prepared game files mounted
read-only, Mojang's EULA and the same port as Minecraft's server."""
import base64
import json
import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
from test_hearth_server_apps import Base  # noqa: E402


class HorizonServer(Base):
    def test_in_the_catalog_with_the_eula_and_minecraft_s_port(self):
        a = self.m.CATALOG['horizon']
        self.assertEqual(a['ports'], [(25565, 'tcp')])
        self.assertEqual(a['eula'], 'https://aka.ms/MinecraftEULA')
        self.assertEqual(self.m.settings()['apps']['horizon']['on'], False)

    def test_needs_the_eula_and_the_game_files(self):
        self.images.add('docker.io/library/debian:bookworm-slim')
        self.m.engine_ok = lambda aid: True
        with self.assertRaises(self.m.Fail) as e:
            self.m.switch('horizon', True)
        self.assertIn('EULA', str(e.exception))
        self.m.save_app('horizon', 'eula', True)
        self.m.horizon_game_files = lambda home=None: (None, None)
        with self.assertRaises(self.m.Fail) as e:
            self.m.switch('horizon', True)
        self.assertIn('open horizon once', str(e.exception).lower())

    def test_runs_the_engine_and_your_game_files_read_only(self):
        self.m.horizon_game_files = lambda home=None: ('/home/p/.local/share/horizon-launcher/versions/v1/.local/horizon-client/26.3', '26.3')
        argv = self.m.run_argv('horizon', ts_ip='100.84.12.7')
        mounts = [argv[i + 1] for i, x in enumerate(argv) if x == '-v']
        self.assertTrue(any(m.endswith(':/opt/horizon:ro') for m in mounts))
        self.assertIn('/home/p/.local/share/horizon-launcher/versions/v1/.local/horizon-client/26.3:/opt/horizon/.local/horizon-client/26.3:ro', mounts)
        self.assertTrue(any(m.endswith(':/data:Z') for m in mounts))
        self.assertEqual(argv[argv.index('--stop-timeout') + 1], '180', 'it saves the world before it stops')
        self.assertIn('horizon-server run', argv[-1])

    def test_finds_the_newest_prepared_game_files(self):
        with tempfile.TemporaryDirectory() as h:
            for v, t in (('v2026.10.1', 100), ('v2026.11.2', 200)):
                d = os.path.join(h, '.local/share/horizon-launcher/versions', v, '.local/horizon-client/26.3')
                os.makedirs(d)
                f = os.path.join(d, 'horizon-resources.json')
                open(f, 'w').write('{}')
                os.utime(f, (t, t))
            os.makedirs(os.path.join(h, '.local/share/horizon-launcher/versions/v-empty/.local/horizon-client/26.3'))
            d, g = self.m.horizon_game_files(h)
            self.assertIn('v2026.11.2', d)
            self.assertEqual(g, '26.3')

    def test_the_nightly_is_checked_for_its_own_version(self):
        rel = {'tag_name': 'nightly', 'name': 'Nightly 2026-11-13 · 0c02067',
               'assets': [{'name': 'horizon-server-linux-x86_64.tar.gz', 'browser_download_url': 'https://github.com/x/a.tgz', 'digest': 'sha256:' + 'ab' * 32},
                          {'name': 'SHA256SUMS', 'browser_download_url': 'https://github.com/x/SHA256SUMS'},
                          {'name': 'SHA256SUMS.sig', 'browser_download_url': 'https://github.com/x/old.sig'},
                          {'name': 'SHA256SUMS.v2.sig', 'browser_download_url': 'https://github.com/x/v2.sig'}]}
        r = self.m.horizon_release(lambda u: json.dumps(rel).encode())
        self.assertEqual(r['tag'], 'nightly-2026-11-13-0c02067')
        self.assertTrue(r['sig_url'].endswith('v2.sig'))
        rel['name'] = 'Nightly'
        with self.assertRaises(self.m.Fail):
            self.m.horizon_release(lambda u: json.dumps(rel).encode())

    @unittest.skipUnless(subprocess.run(['sh', '-c', 'openssl version | grep -q "^OpenSSL [3-9]"']).returncode == 0, 'needs OpenSSL 3')
    def test_only_a_release_signed_with_horizon_s_key_is_installed(self):
        real = lambda args, timeout=60, env=None: (subprocess.run(args, capture_output=True).returncode, '')
        self.m.cmd = real
        self.m.have = lambda n: True
        with tempfile.TemporaryDirectory() as t:
            k = os.path.join(t, 'k.pem')
            subprocess.run(['openssl', 'genpkey', '-algorithm', 'ed25519', '-out', k], check=True, capture_output=True)
            der = subprocess.run(['openssl', 'pkey', '-in', k, '-pubout', '-outform', 'DER'], check=True, capture_output=True).stdout
            pub = base64.b64encode(der[-32:]).decode()
            sums = ('%s  horizon-server-linux-x86_64.tar.gz\n' % ('cd' * 32)).encode()
            msg = os.path.join(t, 'm')
            open(msg, 'wb').write(b'horizon release nightly-2026-11-13-0c02067\n' + sums)
            sig = subprocess.run(['openssl', 'pkeyutl', '-sign', '-rawin', '-inkey', k, '-in', msg], check=True, capture_output=True).stdout
            files = {'https://github.com/x/SHA256SUMS': sums, 'https://github.com/x/v2.sig': base64.b64encode(sig) + b'\n'}
            rel = {'tag': 'nightly-2026-11-13-0c02067', 'digest': '', 'sums_url': 'https://github.com/x/SHA256SUMS',
                   'sig_url': 'https://github.com/x/v2.sig'}
            self.m.HORIZON_KEYS = (pub,)
            self.assertEqual(self.m.horizon_expected_sha(rel, files.get), 'cd' * 32)
            self.m.HORIZON_KEYS = (base64.b64encode(b'\x01' * 32).decode(),)     # someone else's key
            with self.assertRaises(self.m.Fail):
                self.m.horizon_expected_sha(rel, files.get)
            self.m.HORIZON_KEYS = (pub,)
            rel['tag'] = 'nightly-2026-10-01-1111111'                               # an older Nightly passed off as new
            with self.assertRaises(self.m.Fail):
                self.m.horizon_expected_sha(rel, files.get)

    def test_an_archive_climbing_out_is_refused(self):
        self.assertTrue(self.m.tar_safe('-rwxr-xr-x 0/0 9 2026-01-01 00:00 horizon/horizon-server\n'))
        for bad in ('-rw-r--r-- 0/0 1 2026-01-01 00:00 /etc/passwd', '-rw-r--r-- 0/0 1 2026-01-01 00:00 ../x',
                    'lrwxrwxrwx 0/0 0 2026-01-01 00:00 a -> /etc/shadow', 'crw-r--r-- 0/0 1,3 2026-01-01 00:00 null'):
            self.assertFalse(self.m.tar_safe(bad), bad)

    def test_downloads_only_from_github(self):
        with self.assertRaises(self.m.Fail):
            self.m._https_get('https://evil.example.com/a.tgz', 10)
        with self.assertRaises(self.m.Fail):
            self.m._https_get('http://github.com/a.tgz', 10)


if __name__ == '__main__':
    unittest.main()
