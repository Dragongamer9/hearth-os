"""1.13: PC backups for the house (UrBackup in the Server apps catalog, docs/SERVER-APPS.md). Off and not installed
until you ask; its page with the admin account (55414) is only ever on this PC and Tailscale, even when the app reaches
your home network; the PCs' client port (55415) reaches the home network; backups go, writable, to the folder you pick;
everything else stays read-only as before."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from test_hearth_server_apps import Base  # noqa: E402


def ports(argv):
    return [argv[i + 1] for i, a in enumerate(argv) if a == '-p']


def vols(argv):
    return [argv[i + 1] for i, a in enumerate(argv) if a == '-v']


class PcBackups(Base):
    def test_in_the_catalog_off_until_asked(self):
        a = self.m.CATALOG['urbackup']
        self.assertEqual(a['kind'], 'app')
        self.assertEqual(a['images'], ['docker.io/uroni/urbackup-server:latest'])
        r = {x['id']: x for x in self.m.list_apps()['apps']}['urbackup']
        self.assertFalse(r['on']); self.assertFalse(r['installed'])
        self.assertIsNone(self.saved())

    def test_its_page_is_never_on_the_home_network(self):
        cfg = self.m.settings()['apps']['urbackup']
        self.assertEqual(cfg['reach'], 'home')
        p = ports(self.m.run_argv('urbackup', cfg, {}, '100.84.12.7'))
        self.assertIn('55415:55415/tcp', p, 'the PCs reach the client port on the home network')
        self.assertNotIn('55414:55414/tcp', p)
        self.assertIn('127.0.0.1:55414:55414/tcp', p)
        self.assertIn('100.84.12.7:55414:55414/tcp', p)
        self.install('urbackup')
        addrs = self.m.address('urbackup', cfg, '100.84.12.7', '192.168.1.20', 'chris-hearth')
        self.assertFalse(any(x['where'] == 'Home network' for x in addrs), 'no home-network link to the admin page')
        self.assertEqual(addrs[-1]['url'], 'http://localhost:55414')
        cfg['reach'] = 'tailscale'
        p = ports(self.m.run_argv('urbackup', cfg, {}, ''))
        self.assertEqual(p, ['127.0.0.1:55414:55414/tcp', '127.0.0.1:55415:55415/tcp'])

    def test_backups_folder_is_writable_others_stay_read_only(self):
        cfg = self.m.settings()['apps']['urbackup']
        v = vols(self.m.run_argv('urbackup', cfg, {}, ''))
        self.assertIn(os.path.expanduser('~/Server apps/UrBackup/Backups') + ':/backups:rw', v)
        self.assertTrue(any(x.endswith(':/var/urbackup:Z') for x in v))
        nv = vols(self.m.run_argv('navidrome', self.m.settings()['apps']['navidrome'], {}, ''))
        self.assertTrue(any(x.endswith(':/music:ro') for x in nv), 'Navidrome still reads your music read-only')

    def test_pick_a_drive_for_backups(self):
        self.m.set_opt('urbackup', 'store', '/mnt/Backups drive/PCs')
        self.assertEqual(self.saved(), {'apps': {'urbackup': {'opts': {'store': '/mnt/Backups drive/PCs'}}}}, 'only that key')
        v = vols(self.m.run_argv('urbackup', self.m.settings()['apps']['urbackup'], {}, ''))
        self.assertIn('/mnt/Backups drive/PCs:/backups:rw', v)
        o = self.m.CATALOG['urbackup']['opts']['store']
        self.assertTrue(self.m.opt_ok(o, '/mnt/Backups drive/PCs'))
        for bad in ('/etc/x', '../x', 'relative', '/boot/efi', '/proc/1'):
            self.assertFalse(self.m.opt_ok(o, bad), bad)


if __name__ == '__main__':
    unittest.main()
