"""Tests for hearth-link-firewall (os/system_files/usr/libexec/nimbus): the firewall commands it runs, with the
system's firewall replaced by a recorder. Run: python3 hosts/hearth-nearby/tests/test_link_firewall.py"""
import importlib.machinery
import importlib.util
import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PATH = os.path.join(HERE, '..', '..', '..', 'os', 'system_files', 'usr', 'libexec', 'nimbus', 'hearth-link-firewall')
loader = importlib.machinery.SourceFileLoader('hlf', PATH)
spec = importlib.util.spec_from_loader('hlf', loader)
hlf = importlib.util.module_from_spec(spec)
loader.exec_module(hlf)


class Fake:
    def __init__(self, fw):
        self.fw = fw
        self.calls = []

    def which(self, name):
        return '/usr/bin/' + name if (name == 'firewall-cmd' and self.fw == 'firewalld') or (name == 'ufw' and self.fw == 'ufw') else None

    def run(self, cmd):
        self.calls.append(cmd)
        if cmd == ['firewall-cmd', '--state']:
            return (0, 'running') if self.fw == 'firewalld' else (252, 'not running')
        if cmd == ['ufw', 'status']:
            return (0, 'Status: active') if self.fw == 'ufw' else (0, 'Status: inactive')
        return 0, ''


class LinkFirewall(unittest.TestCase):
    def use(self, fw):
        f = Fake(fw)
        hlf.run = f.run
        hlf.which = f.which
        return f

    def test_firewalld_opens_only_to_local_networks_and_runtime_only(self):
        f = self.use('firewalld')
        r = hlf.apply('open', 'nearby', 53317)
        self.assertTrue(r['ok'])
        self.assertEqual(r['firewall'], 'firewalld')
        rules = [c[1] for c in f.calls if c[0] == 'firewall-cmd' and c[1].startswith('--add-rich-rule=')]
        self.assertEqual(len(rules), 12)  # TCP and UDP, 4 IPv4 + 2 IPv6 local ranges
        for r in rules:
            self.assertIn('port port="53317"', r)
            self.assertTrue(any(n in r for n in hlf.LOCAL_V4 + hlf.LOCAL_V6), r)
        self.assertFalse(any('--permanent' in ' '.join(c) for c in f.calls))
        f.calls.clear()
        hlf.apply('close', 'nearby', 53317)
        self.assertTrue(all(c[1].startswith('--remove-rich-rule=') for c in f.calls if c[0] == 'firewall-cmd' and c[1] != '--state'))

    def test_connect_ports(self):
        f = self.use('firewalld')
        hlf.apply('open', 'connect', 53317)
        rules = [c[1] for c in f.calls if c[1].startswith('--add-rich-rule=')]
        self.assertTrue(all('port="1714-1764"' in r for r in rules))

    def test_hearth_link_ports(self):
        f = self.use('firewalld')
        self.assertTrue(hlf.apply('open', 'hearth', 53317)['ok'])
        rules = [c[1] for c in f.calls if c[1].startswith('--add-rich-rule=')]
        self.assertEqual(len(rules), 6)  # UDP only, 4 IPv4 + 2 IPv6 local ranges
        self.assertTrue(all('port port="47740-47741" protocol="udp"' in r for r in rules), rules)
        self.assertEqual(hlf.main(['x', 'open', 'hearth', '53317']), 2)

    def test_ufw_rules_are_tagged_and_removed(self):
        f = self.use('ufw')
        hlf.apply('open', 'nearby', 53318)
        adds = [c for c in f.calls if c[:2] == ['ufw', 'allow']]
        self.assertEqual(len(adds), 12)
        self.assertTrue(all(c[-2:] == ['comment', 'hearth-nearby'] and '53318' in c for c in adds))
        f.calls.clear()
        hlf.apply('close', 'connect', 53317)
        dels = [c for c in f.calls if c[:3] == ['ufw', 'delete', 'allow']]
        self.assertTrue(dels and all('1714:1764' in c for c in dels))

    def test_no_firewall_changes_nothing(self):
        f = self.use('none')
        r = hlf.apply('open', 'nearby', 53317)
        self.assertEqual(r['firewall'], 'none')
        self.assertTrue(r['ok'])
        self.assertFalse(any('--add-rich-rule' in ' '.join(c) or c[:2] == ['ufw', 'allow'] for c in f.calls))

    def test_arguments_are_checked(self):
        self.use('none')
        out = []
        real = sys.stdout
        class W:
            def write(self, s): out.append(s)
            def flush(self): pass
        sys.stdout = W()
        try:
            self.assertEqual(hlf.main(['x', 'open', 'nearby', '22']), 2)
            self.assertEqual(hlf.main(['x', 'open', 'ssh']), 2)
            self.assertEqual(hlf.main(['x', 'open', 'connect', '53317']), 2)
            self.assertEqual(hlf.main(['x', 'open', 'nearby', '53320']), 0)
        finally:
            sys.stdout = real
        self.assertTrue(all(json.loads(l) for l in ''.join(out).splitlines() if l.strip()))


if __name__ == '__main__':
    unittest.main()
