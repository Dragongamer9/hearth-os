"""Google Chrome as a browser choice (1.12, the owner's request): Google's package checked the way apt checks it (the key's
fingerprint, the signed InRelease, the Packages list's SHA-256, the .deb's SHA-256), your choice saved as one key, and
the three places it's offered (Settings › Web browser, first-time setup, the one-time "Pick your browser"), each with
"Must Have For Xodus Nexus" in red under Chrome."""
import hashlib
import importlib.machinery
import importlib.util
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
sys.path.insert(0, SHELL)
import hearth_chrome as hc  # noqa: E402


def load_helper(env):
    with mock.patch.dict(os.environ, env):
        loader = importlib.machinery.SourceFileLoader('hearth_browser_t', os.path.join(SHELL, 'hearth-browser'))
        spec = importlib.util.spec_from_loader('hearth_browser_t', loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
    return m


PACKAGES = """Package: google-chrome-beta
Version: 132.0.1-1
Filename: pool/main/g/google-chrome-beta/beta.deb
SHA256: %s

Package: google-chrome-stable
Version: 131.0.6778.85-1
Filename: pool/main/g/google-chrome-stable/old.deb
Size: 110000000
SHA256: %s

Package: google-chrome-stable
Version: 131.0.6778.108-1
Filename: pool/main/g/google-chrome-stable/new.deb
Size: 111000000
SHA256: %s
""" % ('aa' * 32, 'bb' * 32, 'cc' * 32)


class Checks(unittest.TestCase):
    def test_the_newest_stable_from_the_packages_list(self):
        p = hc.parse_packages(PACKAGES)
        self.assertEqual((p['version'], p['sha256']), ('131.0.6778.108-1', 'cc' * 32))
        self.assertTrue(p['filename'].endswith('new.deb'))

    def test_the_packages_checksum_from_the_signed_release(self):
        rel = 'Origin: Google LLC\nSHA256:\n %s 1234 main/binary-amd64/Packages\n %s 99 main/binary-amd64/Packages.gz\nOther: x\n' % ('dd' * 32, 'ee' * 32)
        self.assertEqual(hc.parse_release_sha(rel, 'main/binary-amd64/Packages'), 'dd' * 32)
        self.assertEqual(hc.parse_release_sha(rel, 'main/binary-i386/Packages'), '')

    def run_newest(self, fpr, gpgv_ok=True, packages=PACKAGES.encode(), release_sha=None):
        release_sha = release_sha or hashlib.sha256(packages).hexdigest()
        files = {hc.KEY_URL: b'KEY', hc.REPO + '/dists/stable/InRelease': ('SHA256:\n %s 1 main/binary-amd64/Packages\n' % release_sha).encode(),
                 hc.REPO + '/dists/stable/main/binary-amd64/Packages': packages}

        def run(args, timeout=120, inp=None):
            if args[:2] == ['gpg', '--batch'] and '--show-keys' in args:
                return 0, 'pub:-:4096:1:X:1:::-:::scESC::::::23::0:\nfpr:::::::::%s:\nsub:-:4096\nfpr:::::::::%s:\n' % (fpr, '11' * 20)
            if '--dearmor' in args:
                open(args[args.index('-o') + 1], 'wb').write(b'')
                return 0, ''
            if args[0] == 'gpgv':
                return (0 if gpgv_ok else 1), ''
            return 1, ''
        with mock.patch.object(hc, 'run', run):
            return hc.newest(lambda url, limit=0: files[url])

    def test_only_google_s_key_and_a_signed_list(self):
        r = self.run_newest(hc.GOOGLE_FPRS[0])
        self.assertEqual(r['sha256'], 'cc' * 32)
        self.assertTrue(r['url'].startswith('https://dl.google.com/linux/chrome/deb/pool/'))
        with self.assertRaises(hc.Fail):
            self.run_newest('AB' * 20)                                     # someone else's key
        with self.assertRaises(hc.Fail):
            self.run_newest(hc.GOOGLE_FPRS[0], gpgv_ok=False)              # not signed by it
        with self.assertRaises(hc.Fail):
            self.run_newest(hc.GOOGLE_FPRS[0], release_sha='ff' * 32)       # the list was changed

    def test_downloads_only_from_google(self):
        with self.assertRaises(hc.Fail):
            hc.get('https://evil.example.com/chrome.deb')
        with self.assertRaises(hc.Fail):
            hc.get('http://dl.google.com/x')


class Choice(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.m = load_helper({'XDG_CONFIG_HOME': self.t.name + '/config', 'XDG_DATA_HOME': self.t.name + '/data', 'HOME': self.t.name})
        self.m.make_default = lambda desktop_id='x': None

    def tearDown(self):
        self.t.cleanup()

    def test_firefox_until_you_pick_and_one_key_is_written(self):
        c = self.m.choice_read()
        self.assertEqual((c['browser'], c['picked'], c['asked']), ('firefox', False, False))
        self.assertFalse(os.path.exists(self.m.CHOICE), 'nothing written by reading')
        self.m.choice_set(asked=True)
        self.assertEqual(json.load(open(self.m.CHOICE)), {'asked': True})
        self.m.pick('firefox')
        self.assertEqual(json.load(open(self.m.CHOICE)), {'asked': True, 'browser': 'firefox'})

    def test_chrome_is_installed_when_picked(self):
        with mock.patch.object(self.m.chrome, 'installed', return_value=False), mock.patch.object(self.m.chrome, 'install') as inst:
            self.m.pick('chrome')
        inst.assert_called_once()
        self.assertEqual(self.m.choice_read()['browser'], 'chrome')

    def test_a_broken_file_is_never_saved_over(self):
        os.makedirs(os.path.dirname(self.m.CHOICE), exist_ok=True)
        open(self.m.CHOICE, 'w').write('{broken')
        with self.assertRaises(self.m.Fail):
            self.m.choice_set(asked=True)
        self.assertEqual(open(self.m.CHOICE).read(), '{broken')


class Offered(unittest.TestCase):
    def read(self, *p):
        return open(os.path.join(os.path.dirname(__file__), '..', '..', '..', *p)).read()

    def test_everywhere_with_the_red_note(self):
        for f in ('hosts/linux/desktop/shell/SetBrowser.qml', 'hosts/linux/desktop/shell/BrowserPick.qml', 'ui/index.html'):
            self.assertIn('Must Have For Xodus Nexus', self.read(f), f)
        self.assertIn("'wifi','browser','accounts'", self.read('ui/index.html'))
        self.assertIn('Loader { source: "BrowserPick.qml" }', self.read('hosts/linux/desktop/shell/shell.qml'))


if __name__ == '__main__':
    unittest.main()
