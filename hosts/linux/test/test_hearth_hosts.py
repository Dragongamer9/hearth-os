"""Tests for os/system_files/usr/libexec/nimbus/hearth-hosts: /etc/hosts maps the device's name (sudo's "unable to
resolve host chris-hearth"), also after a rename, and leaves the rest of the file alone."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import shutil
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
SYS = REPO / 'os' / 'system_files'


def load():
    path = SYS / 'usr' / 'libexec' / 'nimbus' / 'hearth-hosts'
    loader = importlib.machinery.SourceFileLoader('hearth_hosts', str(path))
    spec = importlib.util.spec_from_loader('hearth_hosts', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hh = load()


def resolves(text, name):
    for line in text.splitlines():
        addr, names = hh.names_on(line)
        if addr and name in names:
            return addr
    return None


class Fix(unittest.TestCase):
    def test_image_hosts_without_the_name(self):
        text = '127.0.0.1\tlocalhost\n::1\t\tlocalhost ip6-localhost ip6-loopback\n'
        new = hh.fix(text, 'chris-hearth')
        self.assertEqual(resolves(new, 'chris-hearth'), '127.0.1.1')
        self.assertTrue(new.startswith('127.0.0.1\tlocalhost\n127.0.1.1\tchris-hearth\n'))
        self.assertEqual(new.count('localhost ip6-localhost'), 1)

    def test_rename_replaces_the_old_name(self):
        text = '127.0.0.1 localhost\n127.0.1.1\thearth\n# mine\n10.0.0.5 nas\n'
        new = hh.fix(text, 'chris-hearth')
        self.assertNotIn('hearth\n', new.replace('chris-hearth', ''))
        self.assertEqual(resolves(new, 'chris-hearth'), '127.0.1.1')
        self.assertIn('# mine\n10.0.0.5 nas\n', new)
        self.assertEqual(sum(1 for l in new.splitlines() if l.startswith('127.0.1.1')), 1)

    def test_already_right_is_unchanged(self):
        text = '127.0.0.1\tlocalhost\n127.0.1.1\tchris-hearth\n::1\t\tlocalhost ip6-localhost ip6-loopback\n'
        self.assertEqual(hh.fix(text, 'chris-hearth'), text)

    def test_name_mapped_elsewhere_is_left_alone(self):
        text = '127.0.0.1 localhost chris-hearth\n::1 localhost\n'
        self.assertEqual(hh.fix(text, 'chris-hearth'), text)

    def test_empty_file(self):
        new = hh.fix('', 'deck')
        self.assertEqual(resolves(new, 'localhost'), '127.0.0.1')
        self.assertEqual(resolves(new, 'deck'), '127.0.1.1')

    def test_fqdn_gets_its_short_name(self):
        new = hh.fix('127.0.0.1 localhost\n::1 localhost\n', 'deck.lan')
        self.assertIn('127.0.1.1\tdeck.lan deck\n', new)


class Run(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-hosts-test-')
        self.hosts = os.path.join(self.tmp, 'hosts')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_writes_and_keeps_mode(self):
        with open(self.hosts, 'w') as f:
            f.write('127.0.0.1 localhost\n')
        os.chmod(self.hosts, 0o644)
        self.assertTrue(hh.run(self.hosts, 'chris-hearth', log=lambda *_: None))
        self.assertEqual(resolves(open(self.hosts).read(), 'chris-hearth'), '127.0.1.1')
        self.assertEqual(os.stat(self.hosts).st_mode & 0o777, 0o644)
        self.assertFalse(hh.run(self.hosts, 'chris-hearth', log=lambda *_: None))
        self.assertFalse(os.path.exists(self.hosts + '.hearth-tmp'))

    def test_name_from_hostname_file(self):
        with open(os.path.join(self.tmp, 'hostname'), 'w') as f:
            f.write('# set by the installer\nsams-deck\n')
        with open(self.hosts, 'w') as f:
            f.write('127.0.0.1 localhost\n')
        hh.run(self.hosts, log=lambda *_: None)
        self.assertEqual(resolves(open(self.hosts).read(), 'sams-deck'), '127.0.1.1')

    def test_refuses_bad_names(self):
        with open(self.hosts, 'w') as f:
            f.write('127.0.0.1 localhost\n')
        for bad in ('localhost', '-x', 'a b', ''):
            self.assertFalse(hh.run(self.hosts, bad or ' ', log=lambda *_: None))
        self.assertEqual(open(self.hosts).read(), '127.0.0.1 localhost\n')


class Wiring(unittest.TestCase):
    def test_units_and_admin(self):
        unit = (SYS / 'usr/lib/systemd/system/hearth-hosts.service').read_text()
        self.assertIn('ExecStart=/usr/libexec/nimbus/hearth-hosts', unit)
        path = (SYS / 'usr/lib/systemd/system/hearth-hosts.path').read_text()
        self.assertIn('PathChanged=/etc/hostname', path)
        admin = (SYS / 'usr/libexec/nimbus/hearth-admin').read_text()
        self.assertIn("HEARTH_HOSTS = '/usr/libexec/nimbus/hearth-hosts'", admin)
        for base in ('peppermint', 'anatase'):
            build = (REPO / 'os-bases' / base / 'build.sh').read_text()
            self.assertIn('systemctl enable hearth-hosts.service hearth-hosts.path', build)
        self.assertTrue(os.access(SYS / 'usr/libexec/nimbus/hearth-hosts', os.X_OK))


if __name__ == '__main__':
    unittest.main()
