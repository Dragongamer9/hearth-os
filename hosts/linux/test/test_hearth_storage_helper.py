"""Tests for hearth-storage-helper (os/system_files/usr/libexec/nimbus): Hearth Storage's Windows sharing config, its
password handling and waking from the network, with the system's commands replaced by a recorder."""
import importlib.machinery
import importlib.util
import os
import pwd
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PATH = os.path.join(HERE, '..', '..', '..', 'os', 'system_files', 'usr', 'libexec', 'nimbus', 'hearth-storage-helper')
loader = importlib.machinery.SourceFileLoader('hsh', PATH)
spec = importlib.util.spec_from_loader('hsh', loader)
hsh = importlib.util.module_from_spec(spec)
loader.exec_module(hsh)


class Rec:
    def __init__(self):
        self.calls = []

    def run(self, cmd, stdin=None):
        self.calls.append((cmd, stdin))
        if cmd[:2] == ['systemctl', 'cat']:
            return (0, '') if cmd[2] == 'smb.service' else (1, '')
        if cmd[:2] == ['systemctl', 'is-active']:
            return 3, 'inactive'
        return 0, ''

    def which(self, name):
        return '/usr/bin/' + name if name in ('smbd', 'smbpasswd', 'testparm') else None


class StorageHelper(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.home = os.path.join(self.d, 'home')
        os.makedirs(os.path.join(self.home, 'Videos'))
        os.makedirs(os.path.join(self.home, '.ssh'))
        self.user = pwd.getpwuid(os.getuid()).pw_name
        os.environ['PKEXEC_UID'] = str(os.getuid())
        self.rec = Rec()
        hsh.run, hsh.which = self.rec.run, self.rec.which
        hsh.SMB_CONF = os.path.join(self.d, 'smb.conf')
        hsh.OUR_CONF = os.path.join(self.d, 'hearth-storage.conf')
        hsh.INCLUDE = 'include = %s' % hsh.OUR_CONF
        hsh.STARTED = os.path.join(self.d, '.started')
        hsh.caller = lambda: (self.user, self.home)

    def test_only_the_persons_own_folders_and_nothing_hidden(self):
        self.assertTrue(hsh.share_ok(os.path.join(self.home, 'Videos'), self.user, self.home))
        self.assertIsNone(hsh.share_ok(os.path.join(self.home, '.ssh'), self.user, self.home))
        self.assertIsNone(hsh.share_ok('/etc', self.user, self.home))
        self.assertIsNone(hsh.share_ok('relative/path', self.user, self.home))
        text = hsh.conf_text([{'name': 'Films [x]', 'path': os.path.join(self.home, 'Videos'), 'write': False},
                              {'name': 'Keys', 'path': os.path.join(self.home, '.ssh')},
                              {'name': 'global', 'path': os.path.join(self.home, 'Videos'), 'write': True}], self.user, self.home)
        self.assertIn('[Films -x-]', text)
        self.assertNotIn('.ssh', text)
        self.assertNotIn('[global]', text)
        self.assertIn('valid users = %s' % self.user, text)
        self.assertIn('read only = yes', text)
        self.assertIn('read only = no', text)
        self.assertIn('veto files = /.*/', text)

    def test_apply_and_off_keep_the_rest_of_smb_conf(self):
        with open(hsh.SMB_CONF, 'w') as f:
            f.write('[global]\n   workgroup = HOME\n')
        r = hsh.smb_apply({'shares': [{'name': 'Videos', 'path': os.path.join(self.home, 'Videos')}]})
        self.assertTrue(r['ok'])
        conf = open(hsh.SMB_CONF).read()
        self.assertIn('workgroup = HOME', conf)
        self.assertEqual(conf.count(hsh.INCLUDE), 1)
        hsh.smb_apply({'shares': []})
        self.assertEqual(open(hsh.SMB_CONF).read().count(hsh.INCLUDE), 1, 'once, however often it is applied')
        self.assertIn(['systemctl', 'enable', '--now', 'smb'], [c[0] for c in self.rec.calls])
        hsh.smb_off()
        conf = open(hsh.SMB_CONF).read()
        self.assertNotIn(hsh.INCLUDE, conf)
        self.assertIn('workgroup = HOME', conf)
        self.assertFalse(os.path.exists(hsh.OUR_CONF))
        self.assertIn(['systemctl', 'disable', '--now', 'smb'], [c[0] for c in self.rec.calls], 'stopped: Hearth started it')

    def test_password_goes_to_smbpasswd_on_stdin_only(self):
        hsh.smb_password('correct horse\n')
        cmd, stdin = [c for c in self.rec.calls if c[0][0] == 'smbpasswd'][0]
        self.assertEqual(cmd, ['smbpasswd', '-a', '-s', self.user])
        self.assertEqual(stdin, 'correct horse\ncorrect horse\n')
        self.assertFalse(any('correct horse' in ' '.join(c[0]) for c in self.rec.calls), 'never on a command line')
        with self.assertRaises(hsh.Fail):
            hsh.smb_password('short\n')

    def test_arguments_are_checked(self):
        self.assertEqual(hsh.main(['x', 'smb', 'everything']), 1)
        self.assertEqual(hsh.main(['x', 'wol', '../../etc']), 1)
        self.assertEqual(hsh.main(['x']), 1)


if __name__ == '__main__':
    unittest.main()
