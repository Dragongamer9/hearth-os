"""Tests for desktop/shell/hearth-claude (Claude Desktop from Anthropic's apt repository, installed in the home folder).

Everything runs against small fake fixtures: a throwaway OpenPGP key made here signs a fake InRelease, and a tiny
fake .deb stands in for Claude Desktop. The throwaway key is handed to the functions directly; the helper's pinned
fingerprint for Anthropic's real key is never changed (one test checks it's still the published one)."""
import sys
sys.dont_write_bytecode = True
import functools
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import subprocess
import tarfile
import tempfile
import time
import unittest
from email.utils import formatdate

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-claude'
GPG = shutil.which('gpg')
ANTHROPIC_FPR = '31DDDE24DDFAB679F42D7BD2BAA929FF1A7ECACE'


def rb(p):
    with open(p, 'rb') as f:
        return f.read()


def rt(p):
    with open(p) as f:
        return f.read()


def load(home):
    os.environ['HOME'] = home
    os.environ.pop('XDG_DATA_HOME', None)
    os.environ['XDG_CONFIG_HOME'] = os.path.join(home, '.config')
    loader = importlib.machinery.SourceFileLoader('hearth_claude', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_claude', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


# ------------------------------------------------------------------ fixtures

def ar_write(path, members):
    with open(path, 'wb') as f:
        f.write(b'!<arch>\n')
        for name, data in members:
            hdr = '%-16s%-12d%-6d%-6d%-8s%-10d`\n' % (name, 0, 0, 0, '100644', len(data))
            f.write(hdr.encode())
            f.write(data)
            if len(data) % 2:
                f.write(b'\n')


def tar_bytes(entries, mode='w:xz'):
    """entries: (name, kind, payload, perm) with kind file|dir|sym."""
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode=mode) as t:
        for name, kind, payload, perm in entries:
            ti = tarfile.TarInfo(name)
            ti.mode = perm
            if kind == 'dir':
                ti.type = tarfile.DIRTYPE
                t.addfile(ti)
            elif kind == 'sym':
                ti.type = tarfile.SYMTYPE
                ti.linkname = payload
                t.addfile(ti)
            else:
                data = payload.encode() if isinstance(payload, str) else payload
                ti.size = len(data)
                t.addfile(ti, io.BytesIO(data))
    return buf.getvalue()


DESKTOP_FILE = '''[Desktop Entry]
Name=Claude
Exec=claude-desktop %U
Icon=claude-desktop
Type=Application
StartupWMClass=com.anthropic.Claude
MimeType=x-scheme-handler/claude;application/vnd.anthropic.mcpb;
Actions=NewChat;

[Desktop Action NewChat]
Name=New Chat
Exec=claude-desktop "claude://claude.ai/new?surface=chat"
'''


def fake_deb(path, version='1.0.0', extra=None):
    exe = '#!/bin/sh\necho %s\n' % version
    entries = [
        ('./usr', 'dir', None, 0o755),
        ('./usr/lib', 'dir', None, 0o755),
        ('./usr/lib/claude-desktop', 'dir', None, 0o755),
        ('./usr/lib/claude-desktop/claude-desktop', 'file', exe, 0o755),
        ('./usr/lib/claude-desktop/chrome-sandbox', 'file', 'x', 0o4755),
        ('./usr/lib/claude-desktop/resources', 'dir', None, 0o755),
        ('./usr/lib/claude-desktop/resources/linux-mime', 'dir', None, 0o755),
        ('./usr/lib/claude-desktop/resources/linux-mime/com.anthropic.Claude.xml', 'file', '<mime-info/>', 0o644),
        ('./usr/bin', 'dir', None, 0o755),
        ('./usr/bin/claude-desktop', 'sym', '../lib/claude-desktop/claude-desktop', 0o777),
        ('./usr/share', 'dir', None, 0o755),
        ('./usr/share/applications', 'dir', None, 0o755),
        ('./usr/share/applications/com.anthropic.Claude.desktop', 'file', DESKTOP_FILE, 0o644),
        ('./usr/share/icons/hicolor/256x256/apps', 'dir', None, 0o755),
        ('./usr/share/icons/hicolor/256x256/apps/claude-desktop.png', 'file', b'\x89PNG', 0o644),
    ] + (extra or [])
    ar_write(path, [('debian-binary', b'2.0\n'),
                    ('control.tar.xz', tar_bytes([('./control', 'file', 'Package: claude-desktop\n', 0o644)])),
                    ('data.tar.xz', tar_bytes(entries))])


def sha(path):
    return hashlib.sha256(rb(path)).hexdigest()


def packages_text(debs):
    """debs: (version, arch, filename, path)."""
    out = []
    for ver, arch, fn, p in debs:
        out.append('Package: claude-desktop\nVersion: %s\nArchitecture: %s\nInstalled-Size: 10\nFilename: %s\nSize: %d\n'
                   'SHA256: %s\nDescription: test\n' % (ver, arch, fn, os.path.getsize(p), sha(p)))
    return '\n'.join(out)


class Key:
    """A throwaway signing key in its own temporary GnuPG home (tests only)."""
    def __init__(self, name='Hearth Test'):
        self.home = tempfile.mkdtemp(prefix='hearth-claude-testkey-')
        os.chmod(self.home, 0o700)
        self.run('--passphrase', '', '--pinentry-mode', 'loopback', '--quick-gen-key', name + ' <test@example.invalid>',
                 'ed25519', 'sign', 'never')
        out = self.run('--with-colons', '--fingerprint', '--list-keys').stdout
        self.fpr = next(l.split(':')[9] for l in out.splitlines() if l.startswith('fpr:'))
        self.pub = os.path.join(self.home, 'pub.asc')
        with open(self.pub, 'w') as f:
            f.write(self.run('--armor', '--export', self.fpr).stdout)

    def run(self, *args, input=None):
        return subprocess.run([GPG, '--batch', '--no-tty', '--homedir', self.home] + list(args), input=input,
                              capture_output=True, text=True, check=True)

    def clearsign(self, text, digest='SHA512'):
        return self.run('--passphrase', '', '--pinentry-mode', 'loopback', '--digest-algo', digest,
                        '--local-user', self.fpr, '--clearsign', input=text).stdout

    def close(self):
        subprocess.run(['gpgconf', '--homedir', self.home, '--kill', 'all'], capture_output=True)
        shutil.rmtree(self.home, ignore_errors=True)


def release_text(packages_by_arch, date=None, valid_days=7, suite='stable'):
    date = time.time() if date is None else date
    lines = ['Origin: Anthropic', 'Suite: %s' % suite, 'Codename: %s' % suite, 'Architectures: amd64 arm64',
             'Components: main', 'Date: ' + formatdate(date, usegmt=True),
             'Valid-Until: ' + formatdate(date + valid_days * 86400, usegmt=True), 'SHA256:']
    for arch, text in packages_by_arch.items():
        b = text.encode()
        lines.append(' %s %d main/binary-%s/Packages' % (hashlib.sha256(b).hexdigest(), len(b), arch))
    return '\n'.join(lines) + '\n'


# ------------------------------------------------------------------ tests

class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.home = tempfile.mkdtemp(prefix='hearth-claude-home-')
        self.m = load(self.home)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.home, ignore_errors=True)


class Versions(Base):
    def test_dpkg_order(self):
        c = self.m.version_cmp
        for lo, hi in [('3.1623.4', '3.6556.1'), ('3.9.0', '3.10.0'), ('1.0~beta1', '1.0'), ('1.0', '1.0-1'),
                       ('1.0-1', '1.0-2'), ('1:0.1', '2:0.0'), ('9.9', '1:0.1'), ('1.0a', '1.0b'), ('1.0', '1.0a'),
                       ('1.0~~', '1.0~'), ('1.0+b1', '1.0.1'), ('3.1623.3', '3.1623.4')]:
            self.assertLess(c(lo, hi), 0, (lo, hi))
            self.assertGreater(c(hi, lo), 0, (hi, lo))
        self.assertEqual(c('1.0', '1.0'), 0)
        self.assertEqual(c('01.0', '1.0'), 0)
        self.assertEqual(c('0:1.0', '1.0'), 0)

    def test_arch(self):
        self.assertEqual(self.m.deb_arch('x86_64'), 'amd64')
        self.assertEqual(self.m.deb_arch('aarch64'), 'arm64')
        self.assertIsNone(self.m.deb_arch('riscv64'))

    def test_real_pin_unchanged(self):
        self.assertEqual(self.m.FINGERPRINT, ANTHROPIC_FPR)
        self.assertEqual(self.m.REPO, 'https://downloads.claude.ai/claude-desktop/apt/stable')
        self.assertEqual(self.m.KEY_URL, 'https://downloads.claude.ai/claude-desktop/key.asc')
        self.assertEqual(self.m.CODE_INSTALLER, 'https://claude.ai/install.sh')


class Pick(Base):
    def setUp(self):
        super().setUp()
        self.d = tempfile.mkdtemp(dir=self.home)
        self.debs = {}
        for v in ('3.1623.4', '3.6556.1', '3.10.0'):
            p = os.path.join(self.d, v + '.deb')
            fake_deb(p, v)
            self.debs[v] = p

    def test_newest_for_arch(self):
        text = packages_text([(v, 'amd64', 'pool/main/c/claude-desktop/claude-desktop_%s_amd64.deb' % v, p)
                              for v, p in self.debs.items()]
                             + [('9.0.0', 'arm64', 'pool/main/c/claude-desktop/claude-desktop_9.0.0_arm64.deb', self.debs['3.10.0'])])
        pkg = self.m.pick_package(text, 'amd64')
        self.assertEqual(pkg['Version'], '3.6556.1')
        self.assertEqual(pkg['SHA256'], sha(self.debs['3.6556.1']))
        self.assertEqual(self.m.pick_package(text, 'arm64')['Version'], '9.0.0')

    def test_unsafe_entries_skipped(self):
        p = self.debs['3.10.0']
        text = packages_text([('99.0', 'amd64', 'pool/../../etc/evil.deb', p), ('98.0', 'amd64', '/abs/x.deb', p),
                              ('97.0', 'amd64', 'https://evil.example/x.deb', p),
                              ('3.10.0', 'amd64', 'pool/main/c/claude-desktop/ok.deb', p)])
        text += '\nPackage: claude-desktop\nVersion: 96.0\nArchitecture: amd64\nFilename: pool/x.deb\nSize: 1\nSHA256: nothex\n'
        self.assertEqual(self.m.pick_package(text, 'amd64')['Version'], '3.10.0')

    def test_nothing_for_arch(self):
        text = packages_text([('1.0', 'amd64', 'pool/a.deb', self.debs['3.10.0'])])
        with self.assertRaises(self.m.Fail):
            self.m.pick_package(text, 'arm64')

    def test_check_file(self):
        p = self.debs['3.10.0']
        self.m.check_file(p, sha(p), os.path.getsize(p), 'x')
        with self.assertRaises(self.m.Fail):
            self.m.check_file(p, sha(p), os.path.getsize(p) + 1, 'x')
        with self.assertRaises(self.m.Fail):
            self.m.check_file(p, '0' * 64, os.path.getsize(p), 'x')


class Release(Base):
    def test_hash_for_arch(self):
        t = release_text({'amd64': 'A', 'arm64': 'B'})
        h, size, date = self.m.parse_release(t, 'arm64')
        self.assertEqual(h, hashlib.sha256(b'B').hexdigest())
        self.assertEqual(size, 1)
        with self.assertRaises(self.m.Fail):
            self.m.parse_release(t, 'riscv64')

    def test_expired(self):
        t = release_text({'amd64': 'A'}, date=time.time() - 30 * 86400)
        with self.assertRaisesRegex(self.m.Fail, 'expired'):
            self.m.parse_release(t, 'amd64')

    def test_replay_of_older_list(self):
        now = time.time()
        t = release_text({'amd64': 'A'}, date=now - 86400)
        with self.assertRaisesRegex(self.m.Fail, 'older'):
            self.m.parse_release(t, 'amd64', last_date=now)
        self.m.parse_release(t, 'amd64', last_date=now - 2 * 86400)

    def test_other_suite(self):
        with self.assertRaises(self.m.Fail):
            self.m.parse_release(release_text({'amd64': 'A'}, suite='unstable'), 'amd64')


@unittest.skipUnless(GPG, 'gpg not installed')
class Signature(Base):
    @classmethod
    def setUpClass(cls):
        cls.key = Key('Hearth Test')
        cls.other = Key('Someone Else')

    @classmethod
    def tearDownClass(cls):
        cls.key.close()
        cls.other.close()

    def write(self, text):
        p = os.path.join(self.home, 'InRelease')
        with open(p, 'w') as f:
            f.write(text)
        return p

    def test_good(self):
        body = release_text({'amd64': 'A'})
        signed = self.m.verify_inrelease(self.write(self.key.clearsign(body)), self.key.pub, self.key.fpr)
        self.assertEqual(signed.strip(), body.strip())

    def test_real_pin_rejects_test_key(self):
        with self.assertRaisesRegex(self.m.Fail, 'fingerprint'):
            self.m.verify_inrelease(self.write(self.key.clearsign('Suite: stable\n')), self.key.pub)

    def test_tampered(self):
        s = self.key.clearsign(release_text({'amd64': 'A'})).replace('Suite: stable', 'Suite: stablf')
        with self.assertRaises(self.m.Fail):
            self.m.verify_inrelease(self.write(s), self.key.pub, self.key.fpr)

    def test_signed_by_another_key_in_the_same_file(self):
        both = os.path.join(self.home, 'both.asc')
        with open(both, 'w') as f:
            f.write(rt(self.key.pub) + rt(self.other.pub))
        s = self.other.clearsign(release_text({'amd64': 'A'}))
        with self.assertRaises(self.m.Fail):
            self.m.verify_inrelease(self.write(s), both, self.key.fpr)

    def test_text_outside_the_signature(self):
        s = self.key.clearsign(release_text({'amd64': 'A'}))
        with self.assertRaises(self.m.Fail):
            self.m.verify_inrelease(self.write(s + 'SHA256:\n deadbeef 1 main/binary-amd64/Packages\n'), self.key.pub, self.key.fpr)
        with self.assertRaises(self.m.Fail):
            self.m.verify_inrelease(self.write('Suite: stable\n' + s), self.key.pub, self.key.fpr)

    def test_sha1_refused(self):
        try:
            s = self.key.clearsign('Suite: stable\n', digest='SHA1')
        except subprocess.CalledProcessError:
            self.skipTest('this gpg refuses to make SHA-1 signatures')
        with self.assertRaises(self.m.Fail):
            self.m.verify_inrelease(self.write(s), self.key.pub, self.key.fpr)


class Unpack(Base):
    def unpack_both(self, deb):
        outs = []
        modes = [False] + ([True] if shutil.which('dpkg-deb') else [])
        for use in modes:
            dest = tempfile.mkdtemp(dir=self.home)
            self.m.unpack_deb(deb, dest, use_dpkg=use)
            outs.append(dest)
        return outs

    def test_python_and_dpkg_agree(self):
        deb = os.path.join(self.home, 'c.deb')
        fake_deb(deb, '2.0')
        for dest in self.unpack_both(deb):
            exe = os.path.join(dest, 'usr/lib/claude-desktop/claude-desktop')
            self.assertTrue(os.access(exe, os.X_OK))
            self.assertEqual(os.readlink(os.path.join(dest, 'usr/bin/claude-desktop')), '../lib/claude-desktop/claude-desktop')
            self.assertTrue(os.path.isfile(os.path.join(dest, 'usr/share/applications/com.anthropic.Claude.desktop')))

    def test_python_path_drops_setuid(self):
        deb = os.path.join(self.home, 'c.deb')
        fake_deb(deb)
        dest = os.path.join(self.home, 'out')
        self.m.unpack_deb(deb, dest, use_dpkg=False)
        self.assertFalse(os.stat(os.path.join(dest, 'usr/lib/claude-desktop/chrome-sandbox')).st_mode & 0o6000)

    def test_escape_refused(self):
        deb = os.path.join(self.home, 'evil.deb')
        fake_deb(deb, extra=[('./../../evil', 'file', 'x', 0o644)])
        with self.assertRaises((self.m.Fail, tarfile.TarError)):
            self.m.unpack_deb(deb, os.path.join(self.home, 'out'), use_dpkg=False)
        self.assertFalse(os.path.exists(os.path.join(self.home, '..', 'evil')))

    def test_links_out_dropped(self):
        deb = os.path.join(self.home, 'l.deb')
        fake_deb(deb, extra=[('./usr/lib/claude-desktop/passwd', 'sym', '/etc/passwd', 0o777),
                             ('./usr/lib/claude-desktop/up', 'sym', '../../../../../etc', 0o777)])
        dest = os.path.join(self.home, 'out')
        self.m.unpack_deb(deb, dest, use_dpkg=False)
        self.assertFalse(os.path.lexists(os.path.join(dest, 'usr/lib/claude-desktop/passwd')))
        self.assertFalse(os.path.lexists(os.path.join(dest, 'usr/lib/claude-desktop/up')))

    def test_not_a_deb(self):
        p = os.path.join(self.home, 'x.deb')
        with open(p, 'wb') as f:
            f.write(b'<html>not found</html>')
        with self.assertRaises(self.m.Fail):
            self.m.unpack_deb(p, os.path.join(self.home, 'out'), use_dpkg=False)


class Desktop(Base):
    def test_every_exec_starts_the_home_copy(self):
        out = self.m.rewrite_desktop(DESKTOP_FILE, '/home/a b/.local/bin/claude-desktop')
        execs = [l for l in out.splitlines() if l.startswith('Exec=')]
        self.assertEqual(execs, ['Exec="/home/a b/.local/bin/claude-desktop" %U',
                                 'Exec="/home/a b/.local/bin/claude-desktop" "claude://claude.ai/new?surface=chat"'])
        self.assertEqual(out.count('TryExec='), 1)
        self.assertIn('X-Hearth-Installed-By=hearth-claude', out)
        self.assertEqual(self.m.desktop_mimes(out), ['x-scheme-handler/claude', 'application/vnd.anthropic.mcpb'])


@unittest.skipUnless(GPG, 'gpg not installed')
class EndToEnd(Base):
    """install → update → auto-update → remove, against a fake repository on disk."""
    @classmethod
    def setUpClass(cls):
        cls.key = Key('Hearth Test Repo')

    @classmethod
    def tearDownClass(cls):
        cls.key.close()

    def setUp(self):
        super().setUp()
        # Only gpg on PATH: no dpkg-deb (the Python unpacker, as on Anatase/CachyOS), no xdg-utils, no ldd, no curl.
        b = os.path.join(self.home, 'bin-test')
        os.makedirs(b)
        os.symlink(GPG, os.path.join(b, 'gpg'))
        os.environ['PATH'] = b
        self.m = load(self.home)
        self.repo = os.path.join(self.home, 'repo')
        os.makedirs(os.path.join(self.repo, 'pool'))
        self.publish(['1.0.0'])
        m = self.m

        def fake_curl(url, dest, label=None, lo=0, hi=0, max_bytes=None, timeout=3600):
            if url == m.KEY_URL:
                shutil.copy(self.key.pub, dest)
                return
            self.assertTrue(url.startswith(m.REPO + '/'), url)
            shutil.copy(os.path.join(self.repo, url[len(m.REPO) + 1:]), dest)
        m.curl = fake_curl
        m.fetch_index = functools.partial(m.fetch_index, key_file=self.key.pub, fingerprint=self.key.fpr)
        m.apt_installed = lambda: None
        m.running = lambda: False
        m.is_root = lambda: False
        self.out = io.StringIO()
        self._stdout, sys.stdout = sys.stdout, self.out

    def tearDown(self):
        sys.stdout = self._stdout
        super().tearDown()

    def publish(self, versions, date=None):
        debs = []
        for v in versions:
            fn = 'pool/main/c/claude-desktop/claude-desktop_%s_amd64.deb' % v
            p = os.path.join(self.repo, fn)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            fake_deb(p, v)
            debs.append((v, 'amd64', fn, p))
        text = packages_text(debs)
        d = os.path.join(self.repo, 'dists/stable/main/binary-amd64')
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, 'Packages'), 'w') as f:
            f.write(text)
        with open(os.path.join(self.repo, 'dists/stable/InRelease'), 'w') as f:
            f.write(self.key.clearsign(release_text({'amd64': text}, date=date)))

    def state(self):
        return json.loads(rt(self.m.STATE))

    def test_install_update_remove(self):
        m = self.m
        if m.deb_arch() != 'amd64':
            self.skipTest('fixture repository is amd64')
        m.main(['hearth-claude', 'install'])
        self.assertTrue(os.access(m.LAUNCHER, os.X_OK))
        self.assertIn(os.path.join(m.ROOT, 'claude-desktop'), rt(m.LAUNCHER))
        self.assertEqual(subprocess.run([m.LAUNCHER], capture_output=True, text=True).stdout.strip(), '1.0.0')
        self.assertFalse(os.path.exists(os.path.join(m.ROOT, 'chrome-sandbox')))
        entry = rt(m.DESKTOP)
        self.assertIn('Exec=' + m.LAUNCHER + ' %U', entry)
        self.assertIn('Hidden=true', rt(m.GET_HIDE))
        self.assertTrue(os.path.isfile(os.path.join(m.DATA, 'icons/hicolor/256x256/apps/claude-desktop.png')))
        self.assertTrue(os.path.isfile(m.MIME_XML))
        mimeapps = rt(os.path.join(self.home, '.config', 'mimeapps.list'))
        self.assertIn('x-scheme-handler/claude=com.anthropic.Claude.desktop', mimeapps)
        st = m.status()
        self.assertTrue(st['installed'] and st['ours'])
        self.assertEqual((st['version'], st['latest'], st['updateAvailable']), ('1.0.0', '1.0.0', False))
        last = json.loads(self.out.getvalue().strip().splitlines()[-1])
        self.assertEqual(last['version'], '1.0.0')

        # Nothing newer: update does nothing.
        self.assertFalse(m.install(only_if_newer=True))
        # Anthropic publishes 1.1.0 while Claude is open: the daily check waits, a manual update says to close it.
        time.sleep(1.1)
        self.publish(['1.0.0', '1.1.0'])
        m.running = lambda: True
        m.main(['hearth-claude', 'auto-update'])
        self.assertEqual(self.state()['version'], '1.0.0')
        self.assertEqual(self.state()['pending'], '1.1.0')
        self.assertTrue(m.status()['updateAvailable'])
        with self.assertRaisesRegex(m.Fail, 'Close Claude'):
            m.main(['hearth-claude', 'update'])
        # Closed: the daily check updates it.
        m.running = lambda: False
        m.main(['hearth-claude', 'auto-update'])
        self.assertEqual(self.state()['version'], '1.1.0')
        self.assertNotIn('pending', self.state())
        self.assertEqual(subprocess.run([m.LAUNCHER], capture_output=True, text=True).stdout.strip(), '1.1.0')

        # A replayed older InRelease is refused (and the daily check doesn't fail the unit over it).
        self.publish(['1.0.0'], date=time.time() - 3600)
        with self.assertRaisesRegex(m.Fail, 'older'):
            m.install(only_if_newer=True)
        m.main(['hearth-claude', 'auto-update'])
        self.assertIn('older', self.state()['lastError'])

        m.main(['hearth-claude', 'remove'])
        for p in (m.ROOT, m.LAUNCHER, m.DESKTOP, m.GET_HIDE, m.MIME_XML):
            self.assertFalse(os.path.exists(p), p)
        self.assertFalse(m.status()['installed'])
        # Removed: the daily check doesn't bring it back.
        m.main(['hearth-claude', 'auto-update'])
        self.assertFalse(os.path.exists(m.ROOT))

    def test_tampered_deb_not_installed(self):
        m = self.m
        if m.deb_arch() != 'amd64':
            self.skipTest('fixture repository is amd64')
        p = os.path.join(self.repo, 'pool/main/c/claude-desktop/claude-desktop_1.0.0_amd64.deb')
        data = bytearray(rb(p))
        data[-5] ^= 0xff
        with open(p, 'wb') as f:
            f.write(bytes(data))
        with self.assertRaisesRegex(m.Fail, 'checksum'):
            m.install()
        self.assertFalse(os.path.exists(m.ROOT))

    def test_apt_copy_left_alone(self):
        m = self.m
        m.apt_installed = lambda: '3.6556.1'
        with self.assertRaisesRegex(m.Fail, 'apt'):
            m.install()
        m.main(['hearth-claude', 'auto-update'])     # quietly does nothing
        self.assertFalse(os.path.exists(m.ROOT))


class Helper(unittest.TestCase):
    def test_compiles_and_runs_status(self):
        home = tempfile.mkdtemp(prefix='hearth-claude-cli-')
        try:
            env = dict(os.environ, HOME=home, PYTHONDONTWRITEBYTECODE='1')
            env.pop('XDG_DATA_HOME', None)
            r = subprocess.run([sys.executable, str(PATH), 'status'], capture_output=True, text=True, env=env)
            if os.geteuid() == 0:
                self.assertIn('not as root', r.stdout)
                return
            st = json.loads(r.stdout.strip().splitlines()[-1])
            self.assertFalse(st['ours'])
            self.assertIn('code', st)
        finally:
            shutil.rmtree(home, ignore_errors=True)


if __name__ == '__main__':
    unittest.main()
