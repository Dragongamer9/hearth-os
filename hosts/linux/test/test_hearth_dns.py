import os, unittest
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SF = os.path.join(ROOT, 'os-bases', 'peppermint', 'system_files')

class DnsTest(unittest.TestCase):
    def test_resolv_conf_link_is_forced_back_at_every_start(self):
        line = [l for l in open(os.path.join(SF, 'usr/lib/tmpfiles.d/hearth-resolv-conf.conf')) if l.strip() and not l.startswith('#')]
        self.assertEqual(len(line), 1)
        f = line[0].split()
        self.assertTrue(f[0].startswith('L+'), 'L+ replaces a plain /etc/resolv.conf something else wrote')
        self.assertEqual(f[1], '/etc/resolv.conf')
        self.assertEqual(f[-1], '/run/systemd/resolve/stub-resolv.conf')

    def test_networkmanager_uses_resolved_directly(self):
        t = open(os.path.join(SF, 'usr/lib/NetworkManager/conf.d/90-hearth-dns.conf')).read()
        self.assertIn('[main]', t)
        self.assertIn('dns=systemd-resolved', t)

if __name__ == '__main__':
    unittest.main()
